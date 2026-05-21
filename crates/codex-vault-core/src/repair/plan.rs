use crate::models::{
    AppServerCapability, DiagnosticSeverity, GenerateRepairPlanRequest, LocalSession, Precondition,
    RepairOperation, RepairOperationKind, RepairPlan, RepairRisk, RepairSummary, ScanResult,
};
use crate::repair::app_server::detect_app_server_capability;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};
use uuid::Uuid;

pub fn generate_repair_plan(
    scan: &ScanResult,
    request: &GenerateRepairPlanRequest,
) -> Result<RepairPlan> {
    let selected = select_sessions(scan, &request.session_ids)?;
    let capability = detect_app_server_capability();
    let mut operations = Vec::new();

    if request.options.include_official_repair {
        add_official_operations(scan, &selected, &capability, &mut operations);
    }
    if request.options.include_workspace_hint_patch {
        add_workspace_hint_operations(scan, &selected, &mut operations);
    }
    if request.options.include_jsonl_migration {
        add_thread_source_operations(&selected, &mut operations);
    }
    if request.options.include_experimental_index_rebuild {
        add_session_index_rebuild_operations(scan, &selected, &mut operations);
    }

    let risk_level = highest_risk(&operations);
    let summary = RepairSummary {
        affected_sessions: selected.len(),
        operations_total: operations.len(),
        official_operations: operations
            .iter()
            .filter(|operation| operation.official_api)
            .count(),
        local_patch_operations: operations
            .iter()
            .filter(|operation| !operation.official_api)
            .count(),
        critical_operations: operations
            .iter()
            .filter(|operation| operation.risk == RepairRisk::Critical)
            .count(),
        highest_risk: risk_level.clone(),
    };

    Ok(RepairPlan {
        id: Uuid::new_v4().to_string(),
        scan_id: request.scan_id.clone(),
        codex_home: scan.codex_home.clone(),
        created_at: Utc::now(),
        summary,
        session_ids: selected.iter().map(|session| session.id.clone()).collect(),
        operations,
        preconditions: preconditions(&capability),
        required_backups: vec![
            "full_codex_home_backup_zip".to_string(),
            "rollback_package_directory".to_string(),
        ],
        risk_level,
        dry_run_required: true,
        warnings: plan_warnings(&capability),
    })
}

fn select_sessions<'a>(
    scan: &'a ScanResult,
    session_ids: &[String],
) -> Result<Vec<&'a LocalSession>> {
    if session_ids.is_empty() {
        return Err(anyhow!(
            "At least one session must be selected for repair planning."
        ));
    }
    let by_id = scan
        .sessions
        .iter()
        .map(|session| (session.id.as_str(), session))
        .collect::<HashMap<_, _>>();
    let mut selected = Vec::new();
    for id in session_ids {
        selected.push(
            *by_id
                .get(id.as_str())
                .ok_or_else(|| anyhow!("Selected session was not found in scan: {id}"))?,
        );
    }
    Ok(selected)
}

fn add_official_operations(
    scan: &ScanResult,
    selected: &[&LocalSession],
    capability: &AppServerCapability,
    operations: &mut Vec<RepairOperation>,
) {
    let active_count = scan.summary.active_sessions;
    let archived_count = scan.summary.archived_sessions;
    let selected_active = selected
        .iter()
        .filter(|session| !session.archived)
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    if !selected_active.is_empty() {
        operations.push(RepairOperation {
            id: "official-scan-active".to_string(),
            kind: RepairOperationKind::OfficialAppServerScanAndRepair,
            title: "Official app-server scan-and-repair for active threads".to_string(),
            description: "Runs thread/list with useStateDbOnly=false for active threads so Codex can repair stale or missing metadata rows.".to_string(),
            risk: RepairRisk::Medium,
            official_api: true,
            experimental: false,
            affected_session_ids: selected_active,
            affected_files: vec!["state_*.sqlite".to_string(), "session_index.jsonl".to_string()],
            evidence: json!({
                "estimated_threads": active_count,
                "codex_app_server_available": capability.supports_app_server,
            }),
            payload: json!({"archived": false, "limit": 200, "useStateDbOnly": false}),
            diff: None,
        });
    }

    let selected_archived = selected
        .iter()
        .filter(|session| session.archived)
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    if !selected_archived.is_empty() {
        operations.push(RepairOperation {
            id: "official-scan-archived".to_string(),
            kind: RepairOperationKind::OfficialAppServerScanAndRepair,
            title: "Official app-server scan-and-repair for archived threads".to_string(),
            description: "Runs thread/list with useStateDbOnly=false for archived threads so Codex can repair archive metadata.".to_string(),
            risk: RepairRisk::Medium,
            official_api: true,
            experimental: false,
            affected_session_ids: selected_archived.clone(),
            affected_files: vec!["state_*.sqlite".to_string(), "session_index.jsonl".to_string()],
            evidence: json!({
                "estimated_threads": archived_count,
                "codex_app_server_available": capability.supports_app_server,
            }),
            payload: json!({"archived": true, "limit": 200, "useStateDbOnly": false}),
            diff: None,
        });
    }

    for session in selected {
        if session.archived {
            operations.push(RepairOperation {
                id: format!("official-unarchive-{}", session.id),
                kind: RepairOperationKind::OfficialThreadUnarchive,
                title: format!("Unarchive {}", session.title),
                description: "Uses official thread/unarchive if the user wants this archived thread back in active history.".to_string(),
                risk: RepairRisk::Medium,
                official_api: true,
                experimental: false,
                affected_session_ids: vec![session.id.clone()],
                affected_files: session.rollout_path.iter().cloned().collect(),
                evidence: json!({"archived": true, "archived_sources": session.archived_sources}),
                payload: json!({"thread_id": session.id}),
                diff: None,
            });
        }
        if has_diagnostic(session, "R014_TITLE_DRIFT") {
            operations.push(RepairOperation {
                id: format!("official-name-set-{}", session.id),
                kind: RepairOperationKind::OfficialThreadNameSet,
                title: format!("Set official thread name for {}", session.title),
                description: "Uses official thread/name/set to reconcile visible title drift."
                    .to_string(),
                risk: RepairRisk::Medium,
                official_api: true,
                experimental: false,
                affected_session_ids: vec![session.id.clone()],
                affected_files: vec![
                    "state_*.sqlite".to_string(),
                    "session_index.jsonl".to_string(),
                ],
                evidence: json!({"title_sources": session.title_sources}),
                payload: json!({"thread_id": session.id, "new_name": session.title}),
                diff: None,
            });
        }
        operations.push(RepairOperation {
            id: format!("official-fork-copy-{}", session.id),
            kind: RepairOperationKind::OfficialThreadForkCopy,
            title: format!("Restore {} as a copy", session.title),
            description: "Uses official thread/fork to create a new visible copy without editing the original thread.".to_string(),
            risk: RepairRisk::Medium,
            official_api: true,
            experimental: false,
            affected_session_ids: vec![session.id.clone()],
            affected_files: Vec::new(),
            evidence: json!({"original_thread_id": session.id, "non_destructive_copy": true}),
            payload: json!({"thread_id": session.id}),
            diff: None,
        });
    }
}

fn add_workspace_hint_operations(
    scan: &ScanResult,
    selected: &[&LocalSession],
    operations: &mut Vec<RepairOperation>,
) {
    let Some(global_state) = &scan.global_state else {
        return;
    };
    let Some(path) = &global_state.path else {
        return;
    };

    let mut additions = Vec::new();
    for session in selected {
        if session.archived {
            continue;
        }
        if global_state
            .thread_workspace_root_hints
            .contains_key(&session.id)
        {
            continue;
        }
        let Some(root) = session.cwd_raw.clone().or_else(|| {
            global_state
                .writable_roots_by_thread_id
                .get(&session.id)
                .and_then(|roots| roots.first().cloned())
        }) else {
            continue;
        };
        additions.push(json!({
            "thread_id": session.id,
            "workspace_root": root,
            "source": if session.cwd_raw.is_some() { "session.cwd" } else { "writableRoots" }
        }));
    }
    if additions.is_empty() {
        return;
    }

    let diff = additions
        .iter()
        .filter_map(|addition| {
            Some(format!(
                "+ /thread-workspace-root-hints/{} = {}",
                addition.get("thread_id")?.as_str()?,
                addition.get("workspace_root")?.as_str()?
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let affected_session_ids = additions
        .iter()
        .filter_map(|addition| addition.get("thread_id").and_then(|value| value.as_str()))
        .map(|value| value.to_string())
        .collect();

    operations.push(RepairOperation {
        id: "patch-workspace-hints".to_string(),
        kind: RepairOperationKind::PatchWorkspaceHints,
        title: "Patch missing workspace-root hints".to_string(),
        description: "Adds missing thread-workspace-root-hints entries without overwriting existing hints and skips archived threads.".to_string(),
        risk: RepairRisk::High,
        official_api: false,
        experimental: true,
        affected_session_ids,
        affected_files: vec![path.clone()],
        evidence: json!({
            "global_state_path": path,
            "existing_hint_count": global_state.thread_workspace_root_hints.len()
        }),
        payload: json!({"additions": additions}),
        diff: Some(diff),
    });
}

fn add_thread_source_operations(selected: &[&LocalSession], operations: &mut Vec<RepairOperation>) {
    for session in selected {
        let Some(path) = &session.rollout_path else {
            continue;
        };
        let Ok(Some(first_line)) = first_jsonl_line(path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&first_line) else {
            continue;
        };
        if !looks_like_session_meta(&value, &session.id) || has_thread_source(&value) {
            continue;
        }
        operations.push(RepairOperation {
            id: format!("backfill-thread-source-{}", session.id),
            kind: RepairOperationKind::BackfillThreadSource,
            title: format!("Backfill thread_source for {}", session.title),
            description: "Experimental migration for old rollout JSONL files whose session_meta payload is missing thread_source.".to_string(),
            risk: RepairRisk::Critical,
            official_api: false,
            experimental: true,
            affected_session_ids: vec![session.id.clone()],
            affected_files: vec![path.clone()],
            evidence: json!({
                "first_line_type": value.get("type"),
                "thread_id": session.id,
                "thread_source_present": false
            }),
            payload: json!({"thread_id": session.id, "value": "user"}),
            diff: Some("+ payload.thread_source = \"user\"".to_string()),
        });
    }
}

fn add_session_index_rebuild_operations(
    scan: &ScanResult,
    selected: &[&LocalSession],
    operations: &mut Vec<RepairOperation>,
) {
    let Some(index) = &scan.session_index else {
        return;
    };
    let Some(path) = &index.path else {
        return;
    };
    let affected = selected
        .iter()
        .filter(|session| has_diagnostic(session, "R003_INDEX_MISSING"))
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    if affected.is_empty() {
        return;
    }
    operations.push(RepairOperation {
        id: "experimental-rebuild-session-index".to_string(),
        kind: RepairOperationKind::RebuildSessionIndex,
        title: "Experimental session_index.jsonl rebuild".to_string(),
        description: "High-risk fallback only; official app-server repair should be tried first."
            .to_string(),
        risk: RepairRisk::High,
        official_api: false,
        experimental: true,
        affected_session_ids: affected,
        affected_files: vec![path.clone()],
        evidence: json!({"index_path": path, "duplicate_ids": index.duplicate_ids}),
        payload: json!({"implemented": false}),
        diff: Some("Not implemented in MVP. This operation is report-only.".to_string()),
    });
}

fn preconditions(capability: &AppServerCapability) -> Vec<Precondition> {
    vec![
        Precondition {
            code: "codex_app_server_available".to_string(),
            label: "Codex app-server capability detected".to_string(),
            required: true,
            satisfied: capability.supports_app_server,
            evidence: json!(capability),
        },
        Precondition {
            code: "full_backup_required".to_string(),
            label: "Full Codex home backup is required before apply".to_string(),
            required: true,
            satisfied: false,
            evidence: json!({"required": true}),
        },
        Precondition {
            code: "dry_run_required".to_string(),
            label: "Dry-run on copied CODEX_HOME is required before apply".to_string(),
            required: true,
            satisfied: false,
            evidence: json!({"required": true}),
        },
        Precondition {
            code: "codex_desktop_closed".to_string(),
            label: "User must close Codex Desktop before real apply".to_string(),
            required: true,
            satisfied: false,
            evidence: json!({"user_confirmation_required": true}),
        },
    ]
}

fn plan_warnings(capability: &AppServerCapability) -> Vec<String> {
    let mut warnings = capability.warnings.clone();
    if !capability.supports_app_server {
        warnings.push(
            "Official app-server repair cannot run until codex app-server is available."
                .to_string(),
        );
    }
    warnings
}

fn highest_risk(operations: &[RepairOperation]) -> RepairRisk {
    operations
        .iter()
        .map(|operation| operation.risk.clone())
        .max()
        .unwrap_or(RepairRisk::Low)
}

fn has_diagnostic(session: &LocalSession, code: &str) -> bool {
    session.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == code && diagnostic.severity != DiagnosticSeverity::Info
    }) || session
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

fn first_jsonl_line(path: &str) -> Result<Option<String>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let read = reader.read_line(&mut line)?;
    if read == 0 {
        Ok(None)
    } else {
        Ok(Some(line.trim_end_matches(['\r', '\n']).to_string()))
    }
}

fn looks_like_session_meta(value: &Value, thread_id: &str) -> bool {
    let type_matches = value
        .get("type")
        .and_then(|value| value.as_str())
        .map(|value| value == "session_meta")
        .unwrap_or(false);
    let id_matches = value
        .get("payload")
        .and_then(|payload| {
            payload
                .get("id")
                .or_else(|| payload.get("thread_id"))
                .or_else(|| payload.get("threadId"))
        })
        .and_then(|value| value.as_str())
        .map(|value| value == thread_id)
        .unwrap_or(false);
    type_matches && id_matches
}

fn has_thread_source(value: &Value) -> bool {
    value
        .get("payload")
        .and_then(|payload| {
            payload
                .get("thread_source")
                .or_else(|| payload.get("threadSource"))
        })
        .is_some()
}

#[allow(dead_code)]
fn diagnostic_codes(session: &LocalSession) -> BTreeSet<String> {
    session
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Diagnostic, DiagnosticSeverity, GlobalStateSummary, ScanSummary, VisibilityRisk,
    };
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn plan_maps_jsonl_only_hint_and_thread_source_to_operations() {
        let temp = tempdir().unwrap();
        let rollout = temp.path().join("rollout.jsonl");
        std::fs::write(
            &rollout,
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"thread-a\",\"cwd\":\"/repo\"}}\n",
        )
        .unwrap();
        let global = temp.path().join(".codex-global-state.json");
        std::fs::write(&global, "{}").unwrap();
        let session = LocalSession {
            id: "thread-a".to_string(),
            title: "Thread A".to_string(),
            title_sources: vec![],
            created_at: None,
            updated_at: None,
            source: None,
            cwd_raw: Some("/repo".to_string()),
            cwd_canonical: Some("/repo".to_string()),
            workspace_key: Some("/repo".to_string()),
            archived: false,
            archived_sources: vec![],
            rollout_path: Some(rollout.to_string_lossy().to_string()),
            exists_in_jsonl: true,
            exists_in_sqlite: false,
            exists_in_index: false,
            exists_in_global_state: false,
            messages: vec![],
            first_user_message: None,
            sqlite_evidence: None,
            index_evidence: None,
            global_state_evidence: None,
            diagnostics: vec![
                diag("R001_JSONL_EXISTS_SQLITE_MISSING"),
                diag("R003_INDEX_MISSING"),
                diag("R012_WORKSPACE_HINT_MISSING"),
            ],
            visibility_risk: VisibilityRisk::High,
            confidence: 0.5,
            recent_rank: Some(61),
            jsonl_malformed_lines: 0,
            jsonl_raw_events_count: 1,
            jsonl_parse_errors: vec![],
            jsonl_archived_by_path: Some(false),
            path_aliases: vec![],
        };
        let scan = ScanResult {
            scan_id: "scan".to_string(),
            codex_home: temp.path().to_string_lossy().to_string(),
            created_at: Utc::now(),
            sessions: vec![session],
            workspaces: vec![],
            sqlite_databases: vec![],
            session_index: None,
            global_state: Some(GlobalStateSummary {
                path: Some(global.to_string_lossy().to_string()),
                parsed_ok: true,
                ..GlobalStateSummary::default()
            }),
            diagnostics: vec![],
            warnings: vec![],
            errors: vec![],
            summary: ScanSummary {
                total_sessions: 1,
                active_sessions: 1,
                ..ScanSummary::default()
            },
        };

        let plan = generate_repair_plan(
            &scan,
            &GenerateRepairPlanRequest {
                scan_id: "scan".to_string(),
                session_ids: vec!["thread-a".to_string()],
                options: Default::default(),
            },
        )
        .unwrap();

        assert!(plan.operations.iter().any(|operation| {
            operation.kind == RepairOperationKind::OfficialAppServerScanAndRepair
        }));
        assert!(plan
            .operations
            .iter()
            .any(|operation| operation.kind == RepairOperationKind::PatchWorkspaceHints));
        assert!(plan
            .operations
            .iter()
            .any(|operation| operation.kind == RepairOperationKind::BackfillThreadSource));
        assert_eq!(std::fs::read_to_string(rollout).unwrap().lines().count(), 1);
    }

    fn diag(code: &str) -> Diagnostic {
        Diagnostic {
            code: code.to_string(),
            severity: DiagnosticSeverity::Warning,
            message: code.to_string(),
            evidence: json!({}),
            confidence: 1.0,
            suggested_action: None,
            affected_files: vec![],
            safe_to_auto_fix: false,
        }
    }
}

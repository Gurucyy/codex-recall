use crate::models::{
    Diagnostic, DiagnosticSeverity, GlobalStateSummary, LocalSession, SessionIndexSummary,
    SqliteDatabaseSummary, VisibilityRisk,
};
use crate::path_utils::{is_windows_extended_path, maybe_realpath_alias, normalize_path_key};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub fn apply_diagnostics(
    sessions: &mut [LocalSession],
    session_index: &SessionIndexSummary,
    global_state: &GlobalStateSummary,
    sqlite_databases: &[SqliteDatabaseSummary],
) -> Vec<Diagnostic> {
    let mut scan_diagnostics = sqlite_integrity_diagnostics(sqlite_databases);
    add_path_split_diagnostics(sessions);

    for session in sessions.iter_mut() {
        add_session_diagnostics(session, session_index, global_state, sqlite_databases);
        session.visibility_risk = compute_visibility_risk(&session.diagnostics);
    }

    scan_diagnostics.sort_by(|a, b| a.code.cmp(&b.code));
    scan_diagnostics
}

fn add_session_diagnostics(
    session: &mut LocalSession,
    session_index: &SessionIndexSummary,
    global_state: &GlobalStateSummary,
    sqlite_databases: &[SqliteDatabaseSummary],
) {
    let sqlite_paths = sqlite_databases
        .iter()
        .map(|db| db.path.clone())
        .collect::<Vec<_>>();

    if session.exists_in_jsonl && !session.exists_in_sqlite {
        session.diagnostics.push(diag(
            "R001_JSONL_EXISTS_SQLITE_MISSING",
            DiagnosticSeverity::Warning,
            "Session rollout JSONL exists, but no matching SQLite thread row was found.",
            json!({
                "thread_id": session.id,
                "rollout_path": session.rollout_path,
                "sqlite_databases_scanned": sqlite_paths
            }),
            0.90,
            "Use this tool to view/export the session and generate a recovery report.",
            session.rollout_path.iter().cloned().collect(),
        ));
    }

    if session.exists_in_sqlite && !session.exists_in_jsonl {
        let critical = session.messages.is_empty();
        session.diagnostics.push(diag(
            "R002_SQLITE_EXISTS_JSONL_MISSING",
            if critical {
                DiagnosticSeverity::Critical
            } else {
                DiagnosticSeverity::Warning
            },
            "SQLite metadata exists, but no matching rollout JSONL transcript was found.",
            json!({
                "thread_id": session.id,
                "sqlite_database": session.sqlite_evidence.as_ref().and_then(|thread| thread.database_path.clone()),
                "sqlite_rollout_path": session.sqlite_evidence.as_ref().and_then(|thread| thread.rollout_path.clone())
            }),
            0.85,
            "Check archived sessions, backups, and whether the selected Codex home is correct.",
            affected_from_sqlite(session),
        ));
    }

    if (session.exists_in_jsonl || session.exists_in_sqlite) && !session.exists_in_index {
        session.diagnostics.push(diag(
            "R003_INDEX_MISSING",
            DiagnosticSeverity::Warning,
            "Session evidence exists, but no matching session_index.jsonl record was found.",
            json!({
                "thread_id": session.id,
                "rollout_path": session.rollout_path,
                "sqlite_database": session.sqlite_evidence.as_ref().and_then(|thread| thread.database_path.clone()),
                "index_path": session_index.path
            }),
            0.88,
            "View/export this session with the app. Future repair planning may propose an index rebuild.",
            affected_session_files(session, session_index.path.clone()),
        ));
    }

    if let Some(records) = session_index.all_records_by_id.get(&session.id) {
        if records.len() > 1 {
            session.diagnostics.push(diag(
                "R004_DUPLICATE_INDEX",
                DiagnosticSeverity::Warning,
                "session_index.jsonl contains multiple records for this thread ID.",
                json!({
                    "thread_id": session.id,
                    "line_numbers": records.iter().map(|record| record.line_no).collect::<Vec<_>>(),
                    "titles": records.iter().filter_map(|record| record.thread_name.clone()).collect::<Vec<_>>(),
                    "updated_at": records.iter().filter_map(|record| record.updated_at).collect::<Vec<_>>()
                }),
                0.92,
                "Review diagnostics. Duplicate index records may affect stale titles or ordering.",
                session_index.path.iter().cloned().collect(),
            ));
        }
    }

    add_archive_diagnostics(session);

    if session.cwd_raw.as_deref().unwrap_or("").trim().is_empty() {
        session.diagnostics.push(diag(
            "R008_CWD_EMPTY_OR_UNKNOWN",
            DiagnosticSeverity::Warning,
            "The session workspace/cwd could not be determined.",
            json!({
                "thread_id": session.id,
                "missing_sources": ["jsonl.cwd", "sqlite.cwd", "global_state.thread_workspace_root_hints", "global_state.writable_roots"]
            }),
            0.78,
            "The session can still be viewed/exported, but workspace grouping may be unreliable.",
            affected_session_files(session, None),
        ));
    }

    if let Some(rank) = session.recent_rank {
        if rank > 50 {
            session.diagnostics.push(diag(
                "R011_OUTSIDE_RECENT_50",
                DiagnosticSeverity::Info,
                "Session is outside the global top 50 active sessions by updated time.",
                json!({
                    "thread_id": session.id,
                    "updated_at": session.updated_at,
                    "recent_rank": rank
                }),
                0.72,
                "Use this tool to view/export older sessions.",
                affected_session_files(session, None),
            ));
        }
    }

    if session.cwd_raw.is_some()
        && !global_state
            .thread_workspace_root_hints
            .contains_key(&session.id)
    {
        session.diagnostics.push(diag(
            "R012_WORKSPACE_HINT_MISSING",
            DiagnosticSeverity::Warning,
            "Session has cwd evidence, but no thread workspace-root hint was found in global state.",
            json!({
                "thread_id": session.id,
                "cwd_raw": session.cwd_raw,
                "cwd_canonical": session.cwd_canonical,
                "global_state_path": global_state.path,
                "hint_present": false
            }),
            0.82,
            "Generate a recovery report. Phase 1 does not patch global state.",
            affected_session_files(session, global_state.path.clone()),
        ));
    }

    if !global_state.projectless_thread_ids.is_empty()
        && !session.archived
        && !global_state.projectless_thread_ids.contains(&session.id)
        && !global_state
            .thread_workspace_root_hints
            .contains_key(&session.id)
    {
        session.diagnostics.push(diag(
            "R013_PROJECTLESS_THREAD_ID_MISSING",
            DiagnosticSeverity::Info,
            "projectless-thread-ids exists but does not include this active local thread.",
            json!({
                "thread_id": session.id,
                "projectless_list_present": true,
                "workspace_hint_present": false
            }),
            0.55,
            "Treat as a possible sidebar grouping issue. Do not patch in Phase 1.",
            global_state.path.iter().cloned().collect(),
        ));
    }

    add_title_drift_diagnostic(session);

    if session.jsonl_malformed_lines > 0 {
        let critical = session.messages.is_empty()
            || session.jsonl_malformed_lines > session.jsonl_raw_events_count;
        session.diagnostics.push(diag(
            "R015_MALFORMED_JSONL",
            if critical {
                DiagnosticSeverity::Critical
            } else {
                DiagnosticSeverity::Warning
            },
            "Session JSONL contains malformed lines.",
            json!({
                "thread_id": session.id,
                "rollout_path": session.rollout_path,
                "malformed_lines": session.jsonl_malformed_lines,
                "parse_errors": session.jsonl_parse_errors
            }),
            0.95,
            "Export what can be parsed and back up original files.",
            session.rollout_path.iter().cloned().collect(),
        ));
    }

    if looks_like_bogus_status_session(session) {
        session.diagnostics.push(diag(
            "R016_SUSPICIOUS_BOGUS_STATUS_SESSION",
            DiagnosticSeverity::Warning,
            "Session resembles a bogus status/root session that may crowd out real recent sessions.",
            json!({
                "thread_id": session.id,
                "title": session.title,
                "cwd": session.cwd_raw,
                "message_count": session.messages.len(),
                "raw_event_count": session.jsonl_raw_events_count
            }),
            0.64,
            "Do not delete in Phase 1. Generate a report and back up Codex data.",
            affected_session_files(session, None),
        ));
    }

    if session.exists_in_index && !session.exists_in_jsonl && !session.exists_in_sqlite {
        session.diagnostics.push(diag(
            "R018_ORPHAN_INDEX_RECORD",
            DiagnosticSeverity::Info,
            "session_index.jsonl contains this thread ID, but no JSONL or SQLite evidence was found.",
            json!({
                "thread_id": session.id,
                "line_no": session.index_evidence.as_ref().map(|record| record.line_no),
                "index_title": session.index_evidence.as_ref().and_then(|record| record.thread_name.clone()),
                "index_updated_at": session.index_evidence.as_ref().and_then(|record| record.updated_at)
            }),
            0.87,
            "This may be stale metadata. No Phase 1 repair is available.",
            session_index.path.iter().cloned().collect(),
        ));
    }
}

fn add_archive_diagnostics(session: &mut LocalSession) {
    let Some(path_archived) = session.jsonl_archived_by_path else {
        return;
    };
    let Some(db_archived) = session
        .sqlite_evidence
        .as_ref()
        .and_then(|thread| thread.archived)
    else {
        return;
    };

    if path_archived != db_archived {
        session.diagnostics.push(diag(
            "R005_ARCHIVED_FLAG_MISMATCH",
            DiagnosticSeverity::Warning,
            "Archive state differs between path evidence and SQLite metadata.",
            json!({
                "thread_id": session.id,
                "rollout_path": session.rollout_path,
                "archived_by_path": path_archived,
                "sqlite_archived": db_archived
            }),
            0.90,
            "View/export first. Do not manually move files in Phase 1.",
            affected_session_files(session, None),
        ));
    }

    if !path_archived && db_archived {
        session.diagnostics.push(diag(
            "R006_ACTIVE_FILE_BUT_DB_ARCHIVED",
            DiagnosticSeverity::Warning,
            "Rollout file is under active sessions/, but SQLite says archived=true.",
            json!({
                "thread_id": session.id,
                "rollout_path": session.rollout_path,
                "sqlite_archived": true
            }),
            0.90,
            "Generate a report. Future repair should only reconcile with backup and dry-run.",
            affected_session_files(session, None),
        ));
    }

    if path_archived && !db_archived {
        session.diagnostics.push(diag(
            "R007_ARCHIVED_FILE_BUT_DB_ACTIVE",
            DiagnosticSeverity::Warning,
            "Rollout file is under archived_sessions/, but SQLite says archived=false.",
            json!({
                "thread_id": session.id,
                "rollout_path": session.rollout_path,
                "sqlite_archived": false
            }),
            0.90,
            "Generate a report. Future repair should only reconcile with backup and dry-run.",
            affected_session_files(session, None),
        ));
    }
}

fn add_title_drift_diagnostic(session: &mut LocalSession) {
    let mut titles = Vec::new();
    if let Some(index) = &session.index_evidence {
        if let Some(title) = &index.thread_name {
            titles.push(("index", title.clone()));
        }
    }
    if let Some(sqlite) = &session.sqlite_evidence {
        if let Some(title) = &sqlite.title {
            titles.push(("sqlite_title", title.clone()));
        }
        if let Some(first) = &sqlite.first_user_message {
            titles.push(("sqlite_first_user", first.clone()));
        }
    }
    if let Some(first) = &session.first_user_message {
        titles.push(("jsonl_first_user", first.clone()));
    }

    let normalized = titles
        .iter()
        .map(|(_, title)| normalize_title(title))
        .filter(|title| !title.is_empty())
        .collect::<BTreeSet<_>>();
    if normalized.len() > 1 {
        session.diagnostics.push(diag(
            "R014_TITLE_DRIFT",
            DiagnosticSeverity::Info,
            "Session title differs across index, SQLite, and transcript candidates.",
            json!({
                "thread_id": session.id,
                "chosen_title": session.title,
                "title_candidates": titles
            }),
            0.70,
            "Use the chosen title in this tool. Phase 1 does not reconcile titles.",
            affected_session_files(session, None),
        ));
    }
}

fn add_path_split_diagnostics(sessions: &mut [LocalSession]) {
    let mut by_key: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for session in sessions.iter() {
        if let (Some(raw), Some(key)) = (&session.cwd_raw, &session.workspace_key) {
            by_key
                .entry(key.clone())
                .or_default()
                .push((session.id.clone(), raw.clone()));
        }
    }

    let mut diagnostics_by_session: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
    for (key, values) in by_key {
        let raw_paths = values
            .iter()
            .map(|(_, raw)| raw.clone())
            .collect::<BTreeSet<_>>();
        if raw_paths.len() > 1
            && raw_paths
                .iter()
                .any(|path| is_windows_extended_path(path) || path.contains('\\'))
        {
            let affected_ids = values.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
            let diagnostic = diag(
                "R009_WINDOWS_EXTENDED_PATH_SPLIT",
                DiagnosticSeverity::Warning,
                "The same workspace appears with multiple Windows path spellings.",
                json!({
                    "canonical_comparison_key": key,
                    "raw_paths": raw_paths,
                    "affected_session_ids": affected_ids
                }),
                0.86,
                "Use canonical grouping here. Phase 1 does not rewrite paths.",
                Vec::new(),
            );
            for (id, _) in values {
                diagnostics_by_session
                    .entry(id)
                    .or_default()
                    .push(diagnostic.clone());
            }
        }
    }

    for session in sessions.iter_mut() {
        if let Some(raw) = &session.cwd_raw {
            if let Some(realpath) = maybe_realpath_alias(raw) {
                let normalized = normalize_path_key(raw);
                if normalized.comparison_key != raw.replace('\\', "/") {
                    session.diagnostics.push(diag(
                        "R010_SYMLINK_OR_REALPATH_ALIAS",
                        DiagnosticSeverity::Info,
                        "Raw cwd and realpath differ, which may split workspace grouping.",
                        json!({
                            "thread_id": session.id,
                            "raw_path": raw,
                            "realpath": realpath,
                            "canonical_comparison_key": normalized.comparison_key
                        }),
                        0.66,
                        "Compare raw cwd and canonical cwd. Do not rewrite paths in Phase 1.",
                        affected_session_files(session, None),
                    ));
                }
            }
        }

        if let Some(mut diagnostics) = diagnostics_by_session.remove(&session.id) {
            session.diagnostics.append(&mut diagnostics);
        }
    }
}

fn sqlite_integrity_diagnostics(sqlite_databases: &[SqliteDatabaseSummary]) -> Vec<Diagnostic> {
    sqlite_databases
        .iter()
        .filter(|database| !database.integrity_ok || !database.errors.is_empty())
        .map(|database| {
            diag(
                "R017_SQLITE_INTEGRITY_FAILED",
                DiagnosticSeverity::Critical,
                "A state_*.sqlite database failed integrity check or could not be read.",
                json!({
                    "database_path": database.path,
                    "integrity_ok": database.integrity_ok,
                    "integrity_message": database.integrity_message,
                    "errors": database.errors
                }),
                0.96,
                "Back up Codex home and use JSONL files for viewing/export. Do not edit the database.",
                vec![database.path.clone()],
            )
        })
        .collect()
}

fn compute_visibility_risk(diagnostics: &[Diagnostic]) -> VisibilityRisk {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Critical)
    {
        return VisibilityRisk::High;
    }

    let warning_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
        .count();
    if warning_count >= 3 {
        return VisibilityRisk::High;
    }
    if warning_count > 0
        || diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.code.as_str(),
                "R004_DUPLICATE_INDEX"
                    | "R005_ARCHIVED_FLAG_MISMATCH"
                    | "R011_OUTSIDE_RECENT_50"
                    | "R014_TITLE_DRIFT"
            )
        })
    {
        return VisibilityRisk::Medium;
    }

    VisibilityRisk::Low
}

fn looks_like_bogus_status_session(session: &LocalSession) -> bool {
    let title = normalize_title(&session.title);
    let cwd = session.cwd_raw.as_deref().unwrap_or("");
    let cwd_suspicious = cwd.trim().is_empty() || cwd.trim() == "/";
    let title_suspicious = matches!(title.as_str(), "status" | "us" | "atus" | "tus");
    let little_content = session.messages.is_empty() || session.jsonl_raw_events_count <= 2;
    cwd_suspicious && title_suspicious && little_content
}

fn normalize_title(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn diag(
    code: &str,
    severity: DiagnosticSeverity,
    message: &str,
    evidence: Value,
    confidence: f32,
    suggested_action: &str,
    affected_files: Vec<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        message: message.to_string(),
        evidence,
        confidence,
        suggested_action: Some(suggested_action.to_string()),
        affected_files,
        safe_to_auto_fix: false,
    }
}

fn affected_session_files(session: &LocalSession, extra: Option<String>) -> Vec<String> {
    let mut files = Vec::new();
    if let Some(path) = &session.rollout_path {
        files.push(path.clone());
    }
    if let Some(path) = session
        .sqlite_evidence
        .as_ref()
        .and_then(|thread| thread.database_path.clone())
    {
        files.push(path);
    }
    if let Some(path) = extra {
        files.push(path);
    }
    files.sort();
    files.dedup();
    files
}

fn affected_from_sqlite(session: &LocalSession) -> Vec<String> {
    session
        .sqlite_evidence
        .as_ref()
        .and_then(|thread| thread.database_path.clone())
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonl_without_sqlite_gets_r001_and_medium_risk() {
        let mut sessions = vec![LocalSession {
            id: "a".to_string(),
            title: "A".to_string(),
            title_sources: vec![],
            created_at: None,
            updated_at: None,
            source: None,
            cwd_raw: Some("/repo".to_string()),
            cwd_canonical: Some("/repo".to_string()),
            workspace_key: Some("/repo".to_string()),
            archived: false,
            archived_sources: vec![],
            rollout_path: Some("/codex/sessions/a.jsonl".to_string()),
            exists_in_jsonl: true,
            exists_in_sqlite: false,
            exists_in_index: false,
            exists_in_global_state: false,
            messages: vec![],
            first_user_message: None,
            sqlite_evidence: None,
            index_evidence: None,
            global_state_evidence: None,
            diagnostics: vec![],
            visibility_risk: VisibilityRisk::Low,
            confidence: 0.25,
            recent_rank: Some(1),
            jsonl_malformed_lines: 0,
            jsonl_raw_events_count: 1,
            jsonl_parse_errors: vec![],
            jsonl_archived_by_path: Some(false),
            path_aliases: vec![],
        }];
        let scan_diags = apply_diagnostics(
            &mut sessions,
            &SessionIndexSummary::default(),
            &GlobalStateSummary::default(),
            &[],
        );
        assert!(scan_diags.is_empty());
        assert!(sessions[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "R001_JSONL_EXISTS_SQLITE_MISSING"));
        assert!(sessions[0]
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.safe_to_auto_fix));
        assert_eq!(sessions[0].visibility_risk, VisibilityRisk::High);
    }
}

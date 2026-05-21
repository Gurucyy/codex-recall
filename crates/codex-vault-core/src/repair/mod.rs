pub mod app_server;
pub mod jsonl_migration;
pub mod plan;
pub mod rollback;
pub mod workspace_hints;

use crate::backup::backup_codex_home;
use crate::models::{
    AppServerRunSummary, ApplyRepairRequest, ApplyResult, BackupRequest, DryRunResult,
    RepairOperationKind, RepairPlan, RepairScanDiff, RollbackRequest, RollbackResult,
    VerificationResult,
};
use crate::repair::app_server::run_official_scan_and_validate;
use crate::repair::jsonl_migration::apply_thread_source_migrations;
use crate::repair::rollback::{create_rollback_package, restore_rollback_package};
use crate::repair::workspace_hints::apply_workspace_hint_operations;
use crate::scan::run_scan;
use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;
use walkdir::WalkDir;

pub use plan::generate_repair_plan;

pub fn run_repair_dry_run(plan: &RepairPlan) -> Result<DryRunResult> {
    let temp = tempdir().context("create dry-run temporary directory")?;
    let temp_codex_home = temp.path().join("codex-home");
    copy_codex_home(Path::new(&plan.codex_home), &temp_codex_home)?;

    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let before_scan = run_scan(PathBuf::from(&plan.codex_home), Default::default())
        .context("scan original Codex home before dry-run")?;

    let app_server = match run_official_scan_and_validate(&temp_codex_home, &plan.session_ids) {
        Ok(summary) => Some(summary),
        Err(error) => {
            errors.push(error.to_string());
            None
        }
    };

    let after_scan = run_scan(temp_codex_home.clone(), Default::default())
        .context("scan dry-run Codex home after app-server repair")?;
    let diff = Some(scan_diff(plan, &before_scan, &after_scan));
    let succeeded = app_server
        .as_ref()
        .map(|summary| summary.errors.is_empty() && summary.initialized)
        .unwrap_or(false);

    if !succeeded {
        warnings.push(
            "Dry-run did not complete official app-server validation successfully.".to_string(),
        );
    }

    Ok(DryRunResult {
        plan_id: plan.id.clone(),
        temp_codex_home: temp_codex_home.to_string_lossy().to_string(),
        temp_removed_after_run: true,
        app_server,
        diff,
        succeeded,
        warnings,
        errors,
    })
}

pub fn apply_official_appserver_repair(request: &ApplyRepairRequest) -> Result<ApplyResult> {
    validate_apply_request(request)?;
    let operations = selected_operations(request)
        .into_iter()
        .filter(|operation| operation.official_api)
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return Err(anyhow!("No official app-server operations were selected."));
    }

    let backup = backup_codex_home(&BackupRequest {
        codex_home: request.plan.codex_home.clone(),
        output_path: request.backup_output_path.clone(),
    })
    .context("create pre-repair backup")?;

    let rollback = create_rollback_package(
        Path::new(&request.rollback_output_dir),
        Some(backup.output_path.clone()),
        &[],
    )?;

    let mut app_server = run_official_scan_and_validate(
        Path::new(&request.plan.codex_home),
        &request.plan.session_ids,
    )
    .context("run official app-server scan-and-repair")?;

    for operation in &operations {
        match operation.kind {
            RepairOperationKind::OfficialThreadNameSet => {
                let thread_id = operation
                    .payload
                    .get("thread_id")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| anyhow!("thread/name/set operation missing thread_id"))?;
                let name = operation
                    .payload
                    .get("new_name")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| anyhow!("thread/name/set operation missing new_name"))?;
                if let Err(error) = app_server::run_single_official_operation(
                    Path::new(&request.plan.codex_home),
                    "thread/name/set",
                    json!({"threadId": thread_id, "name": name}),
                ) {
                    app_server.errors.push(error.to_string());
                }
            }
            RepairOperationKind::OfficialThreadUnarchive => {
                let thread_id = operation
                    .payload
                    .get("thread_id")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| anyhow!("thread/unarchive operation missing thread_id"))?;
                if let Err(error) = app_server::run_single_official_operation(
                    Path::new(&request.plan.codex_home),
                    "thread/unarchive",
                    json!({"threadId": thread_id}),
                ) {
                    app_server.errors.push(error.to_string());
                }
            }
            RepairOperationKind::OfficialThreadForkCopy => {
                let thread_id = operation
                    .payload
                    .get("thread_id")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| anyhow!("thread/fork operation missing thread_id"))?;
                if let Err(error) = app_server::run_single_official_operation(
                    Path::new(&request.plan.codex_home),
                    "thread/fork",
                    json!({"threadId": thread_id, "excludeTurns": true}),
                ) {
                    app_server.errors.push(error.to_string());
                }
            }
            _ => {}
        }
    }

    let verification = verify_after_apply(&request.plan, Some(app_server))?;
    let succeeded = verification.errors.is_empty();
    Ok(ApplyResult {
        repair_id: verification.repair_id.clone(),
        plan_id: request.plan.id.clone(),
        applied_operation_ids: operations
            .iter()
            .map(|operation| operation.id.clone())
            .collect(),
        backup: Some(backup),
        rollback: Some(rollback),
        verification: Some(verification),
        succeeded,
        warnings: Vec::new(),
        errors: Vec::new(),
    })
}

pub fn apply_workspace_hint_patch(request: &ApplyRepairRequest) -> Result<ApplyResult> {
    validate_apply_request(request)?;
    let operations = selected_operations(request)
        .into_iter()
        .filter(|operation| operation.kind == RepairOperationKind::PatchWorkspaceHints)
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return Err(anyhow!("No workspace hint operations were selected."));
    }

    let backup = backup_codex_home(&BackupRequest {
        codex_home: request.plan.codex_home.clone(),
        output_path: request.backup_output_path.clone(),
    })
    .context("create pre-repair backup")?;
    let affected = operations
        .iter()
        .flat_map(|operation| operation.affected_files.iter().map(PathBuf::from))
        .collect::<Vec<_>>();
    let rollback = create_rollback_package(
        Path::new(&request.rollback_output_dir),
        Some(backup.output_path.clone()),
        &affected,
    )?;
    apply_workspace_hint_operations(&operations).context("apply workspace hint patch")?;
    let verification = verify_after_apply(&request.plan, None)?;

    Ok(ApplyResult {
        repair_id: verification.repair_id.clone(),
        plan_id: request.plan.id.clone(),
        applied_operation_ids: operations
            .iter()
            .map(|operation| operation.id.clone())
            .collect(),
        backup: Some(backup),
        rollback: Some(rollback),
        succeeded: verification.errors.is_empty(),
        verification: Some(verification),
        warnings: Vec::new(),
        errors: Vec::new(),
    })
}

pub fn apply_jsonl_metadata_migration(request: &ApplyRepairRequest) -> Result<ApplyResult> {
    validate_apply_request(request)?;
    let operations = selected_operations(request)
        .into_iter()
        .filter(|operation| operation.kind == RepairOperationKind::BackfillThreadSource)
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return Err(anyhow!(
            "No JSONL metadata migration operations were selected."
        ));
    }

    let backup = backup_codex_home(&BackupRequest {
        codex_home: request.plan.codex_home.clone(),
        output_path: request.backup_output_path.clone(),
    })
    .context("create pre-repair backup")?;
    let affected = operations
        .iter()
        .flat_map(|operation| operation.affected_files.iter().map(PathBuf::from))
        .collect::<Vec<_>>();
    let rollback = create_rollback_package(
        Path::new(&request.rollback_output_dir),
        Some(backup.output_path.clone()),
        &affected,
    )?;
    apply_thread_source_migrations(&operations).context("apply JSONL metadata migration")?;
    let app_server = run_official_scan_and_validate(
        Path::new(&request.plan.codex_home),
        &request.plan.session_ids,
    )
    .ok();
    let verification = verify_after_apply(&request.plan, app_server)?;

    Ok(ApplyResult {
        repair_id: verification.repair_id.clone(),
        plan_id: request.plan.id.clone(),
        applied_operation_ids: operations
            .iter()
            .map(|operation| operation.id.clone())
            .collect(),
        backup: Some(backup),
        rollback: Some(rollback),
        succeeded: verification.errors.is_empty(),
        verification: Some(verification),
        warnings: Vec::new(),
        errors: Vec::new(),
    })
}

pub fn verify_repair(plan: &RepairPlan) -> Result<VerificationResult> {
    let app_server =
        run_official_scan_and_validate(Path::new(&plan.codex_home), &plan.session_ids).ok();
    verify_after_apply(plan, app_server)
}

pub fn rollback_repair(request: &RollbackRequest) -> Result<RollbackResult> {
    restore_rollback_package(&request.package, request.confirm_restore)
}

fn validate_apply_request(request: &ApplyRepairRequest) -> Result<()> {
    if !request.confirm_codex_closed {
        return Err(anyhow!(
            "Codex Desktop close confirmation is required before repair apply."
        ));
    }
    if request.backup_output_path.trim().is_empty() {
        return Err(anyhow!("A backup output path is required."));
    }
    if request.rollback_output_dir.trim().is_empty() {
        return Err(anyhow!("A rollback output directory is required."));
    }
    Ok(())
}

fn selected_operations(request: &ApplyRepairRequest) -> Vec<crate::models::RepairOperation> {
    let selected = request
        .operation_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    request
        .plan
        .operations
        .iter()
        .filter(|operation| selected.is_empty() || selected.contains(&operation.id))
        .cloned()
        .collect()
}

fn verify_after_apply(
    plan: &RepairPlan,
    app_server: Option<AppServerRunSummary>,
) -> Result<VerificationResult> {
    let scan = run_scan(PathBuf::from(&plan.codex_home), Default::default())
        .context("run verification scan")?;
    let mut errors = Vec::new();
    for id in &plan.session_ids {
        if !scan.sessions.iter().any(|session| &session.id == id) {
            errors.push(format!("Selected session was not found after repair: {id}"));
        }
    }
    if let Some(summary) = &app_server {
        errors.extend(summary.errors.clone());
    }
    Ok(VerificationResult {
        repair_id: Uuid::new_v4().to_string(),
        scan_summary: scan.summary,
        app_server,
        warnings: Vec::new(),
        errors,
    })
}

fn scan_diff(
    plan: &RepairPlan,
    before: &crate::models::ScanResult,
    after: &crate::models::ScanResult,
) -> RepairScanDiff {
    let selected_session_changes = plan
        .session_ids
        .iter()
        .map(|id| {
            let before_session = before.sessions.iter().find(|session| &session.id == id);
            let after_session = after.sessions.iter().find(|session| &session.id == id);
            json!({
                "thread_id": id,
                "before": before_session.map(session_status),
                "after": after_session.map(session_status)
            })
        })
        .collect();

    RepairScanDiff {
        before_total_sessions: before.summary.total_sessions,
        after_total_sessions: after.summary.total_sessions,
        before_high_risk_sessions: before.summary.high_risk_sessions,
        after_high_risk_sessions: after.summary.high_risk_sessions,
        selected_session_changes,
    }
}

fn session_status(session: &crate::models::LocalSession) -> serde_json::Value {
    json!({
        "title": session.title,
        "archived": session.archived,
        "exists_in_jsonl": session.exists_in_jsonl,
        "exists_in_sqlite": session.exists_in_sqlite,
        "exists_in_index": session.exists_in_index,
        "exists_in_global_state": session.exists_in_global_state,
        "risk": session.visibility_risk,
        "diagnostics": session.diagnostics.iter().map(|diagnostic| diagnostic.code.clone()).collect::<Vec<_>>()
    })
}

fn copy_codex_home(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination)
            .with_context(|| format!("remove existing {}", destination.display()))?;
    }
    fs::create_dir_all(destination).with_context(|| format!("create {}", destination.display()))?;
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(source)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)
                .with_context(|| format!("copy {} to {}", path.display(), target.display()))?;
        }
    }
    Ok(())
}

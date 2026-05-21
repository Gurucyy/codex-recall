use crate::error::AppError;
use crate::state::AppState;
use codex_vault_core::backup;
use codex_vault_core::discovery::{discover_codex_homes as core_discover, validate_codex_home};
use codex_vault_core::export;
use codex_vault_core::models::{
    AppSettings, ApplyRepairRequest, ApplyResult, BackupRequest, BackupResult, CodexHomeCandidate,
    DryRunResult, ExportRequest, ExportResult, GenerateRepairPlanRequest, LocalSession,
    PagedSessions, RepairPlan, ReportRequest, ReportResult, RollbackRequest, RollbackResult,
    ScanOptions, ScanResult, SearchQuery, SearchResult, SessionQuery, VerificationResult,
    WorkspaceGroup,
};
use codex_vault_core::repair;
use codex_vault_core::scan::{run_scan_with_progress, SCAN_PROGRESS_TOTAL};
use codex_vault_core::search as core_search;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
struct ScanEvent {
    scan_id: String,
    stage: String,
    message: String,
    completed: usize,
    total: Option<usize>,
}

#[tauri::command]
pub async fn discover_codex_homes() -> Result<Vec<CodexHomeCandidate>, AppError> {
    Ok(core_discover())
}

#[tauri::command]
pub async fn choose_codex_home(app: AppHandle) -> Result<Option<String>, AppError> {
    let path = app.dialog().file().blocking_pick_folder();
    Ok(path.map(|path| path.to_string()))
}

#[tauri::command]
pub async fn start_scan(
    state: State<'_, AppState>,
    app: AppHandle,
    codex_home: String,
    options: ScanOptions,
) -> Result<String, AppError> {
    let candidate = validate_codex_home(&PathBuf::from(&codex_home));
    if !candidate.exists {
        return Err(AppError::Message(format!(
            "Selected Codex home does not exist: {codex_home}"
        )));
    }

    let scan_id = Uuid::new_v4().to_string();
    let returned_scan_id = scan_id.clone();
    let app_state = state.inner().clone();
    let app_handle = app.clone();
    emit_scan_event(
        &app,
        "scan-started",
        &scan_id,
        "start",
        "Starting read-only scan",
        0,
        Some(SCAN_PROGRESS_TOTAL),
    );

    tauri::async_runtime::spawn(async move {
        let scan_id_for_result = scan_id.clone();
        let scan_id_for_progress = scan_id.clone();
        let progress_app = app_handle.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            run_scan_with_progress(
                PathBuf::from(codex_home),
                options,
                move |stage, message, completed, total| {
                    emit_scan_event(
                        &progress_app,
                        "scan-progress",
                        &scan_id_for_progress,
                        stage,
                        message,
                        completed,
                        Some(total),
                    );
                },
            )
            .map(|mut result| {
                result.scan_id = scan_id_for_result.clone();
                result
            })
        })
        .await;

        match result {
            Ok(Ok(scan)) => {
                app_state.insert_scan(scan.clone());
                emit_scan_event(
                    &app_handle,
                    "scan-finished",
                    &scan.scan_id,
                    "finished",
                    "Scan finished",
                    SCAN_PROGRESS_TOTAL,
                    Some(SCAN_PROGRESS_TOTAL),
                );
            }
            Ok(Err(error)) => {
                emit_scan_event(
                    &app_handle,
                    "scan-failed",
                    &scan_id,
                    "failed",
                    &error.to_string(),
                    0,
                    None,
                );
            }
            Err(error) => {
                emit_scan_event(
                    &app_handle,
                    "scan-failed",
                    &scan_id,
                    "failed",
                    &error.to_string(),
                    0,
                    None,
                );
            }
        }
    });

    Ok(returned_scan_id)
}

#[tauri::command]
pub async fn get_scan_result(
    state: State<'_, AppState>,
    scan_id: String,
) -> Result<ScanResult, AppError> {
    state
        .get_scan(&scan_id)
        .ok_or_else(|| AppError::Message("Scan is not finished or was not found.".to_string()))
}

#[tauri::command]
pub async fn list_sessions(
    state: State<'_, AppState>,
    scan_id: String,
    query: SessionQuery,
) -> Result<PagedSessions, AppError> {
    let scan = require_scan(&state, &scan_id)?;
    Ok(core_search::list_sessions(&scan.sessions, &query))
}

#[tauri::command]
pub async fn get_session(
    state: State<'_, AppState>,
    scan_id: String,
    session_id: String,
) -> Result<LocalSession, AppError> {
    let scan = require_scan(&state, &scan_id)?;
    scan.sessions
        .into_iter()
        .find(|session| session.id == session_id)
        .ok_or_else(|| AppError::Message("Session not found".to_string()))
}

#[tauri::command]
pub async fn list_workspaces(
    state: State<'_, AppState>,
    scan_id: String,
) -> Result<Vec<WorkspaceGroup>, AppError> {
    let scan = require_scan(&state, &scan_id)?;
    Ok(scan.workspaces)
}

#[tauri::command]
pub async fn search_sessions(
    state: State<'_, AppState>,
    scan_id: String,
    query: SearchQuery,
) -> Result<Vec<SearchResult>, AppError> {
    let scan = require_scan(&state, &scan_id)?;
    Ok(core_search::search_sessions(&scan.sessions, &query))
}

#[tauri::command]
pub async fn choose_output_dir(app: AppHandle) -> Result<Option<String>, AppError> {
    let path = app.dialog().file().blocking_pick_folder();
    Ok(path.map(|path| path.to_string()))
}

#[tauri::command]
pub async fn export_sessions(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResult, AppError> {
    let scan = require_scan(&state, &request.scan_id)?;
    export::export_sessions(&scan, &request).map_err(AppError::from)
}

#[tauri::command]
pub async fn backup_codex_home(request: BackupRequest) -> Result<BackupResult, AppError> {
    backup::backup_codex_home(&request).map_err(AppError::from)
}

#[tauri::command]
pub async fn generate_report(
    state: State<'_, AppState>,
    request: ReportRequest,
) -> Result<ReportResult, AppError> {
    let scan = require_scan(&state, &request.scan_id)?;
    export::generate_report(&scan, &request).map_err(AppError::from)
}

#[tauri::command]
pub async fn generate_repair_plan(
    state: State<'_, AppState>,
    request: GenerateRepairPlanRequest,
) -> Result<RepairPlan, AppError> {
    let scan = require_scan(&state, &request.scan_id)?;
    repair::generate_repair_plan(&scan, &request).map_err(AppError::from)
}

#[tauri::command]
pub async fn run_repair_dry_run(plan: RepairPlan) -> Result<DryRunResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || repair::run_repair_dry_run(&plan))
        .await
        .map_err(|error| AppError::Message(error.to_string()))?
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn apply_official_appserver_repair(
    request: ApplyRepairRequest,
) -> Result<ApplyResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || repair::apply_official_appserver_repair(&request))
        .await
        .map_err(|error| AppError::Message(error.to_string()))?
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn apply_workspace_hint_patch(
    request: ApplyRepairRequest,
) -> Result<ApplyResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || repair::apply_workspace_hint_patch(&request))
        .await
        .map_err(|error| AppError::Message(error.to_string()))?
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn apply_jsonl_metadata_migration(
    request: ApplyRepairRequest,
) -> Result<ApplyResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || repair::apply_jsonl_metadata_migration(&request))
        .await
        .map_err(|error| AppError::Message(error.to_string()))?
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn verify_repair(plan: RepairPlan) -> Result<VerificationResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || repair::verify_repair(&plan))
        .await
        .map_err(|error| AppError::Message(error.to_string()))?
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn rollback_repair(request: RollbackRequest) -> Result<RollbackResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || repair::rollback_repair(&request))
        .await
        .map_err(|error| AppError::Message(error.to_string()))?
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    Ok(state.settings())
}

#[tauri::command]
pub async fn update_app_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    Ok(state.update_settings(settings))
}

fn require_scan(state: &State<'_, AppState>, scan_id: &str) -> Result<ScanResult, AppError> {
    state
        .get_scan(scan_id)
        .ok_or_else(|| AppError::Message("Scan is not finished or was not found.".to_string()))
}

fn emit_scan_event(
    app: &AppHandle,
    event: &str,
    scan_id: &str,
    stage: &str,
    message: &str,
    completed: usize,
    total: Option<usize>,
) {
    let _ = app.emit(
        event,
        ScanEvent {
            scan_id: scan_id.to_string(),
            stage: stage.to_string(),
            message: message.to_string(),
            completed,
            total,
        },
    );
}

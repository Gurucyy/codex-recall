mod commands;
mod error;
mod state;

pub fn run() {
    configure_app_display_name();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::discover_codex_homes,
            commands::choose_codex_home,
            commands::start_scan,
            commands::get_scan_result,
            commands::list_sessions,
            commands::get_session,
            commands::list_workspaces,
            commands::search_sessions,
            commands::choose_output_dir,
            commands::export_sessions,
            commands::backup_codex_home,
            commands::generate_report,
            commands::generate_repair_plan,
            commands::run_repair_dry_run,
            commands::apply_official_appserver_repair,
            commands::apply_workspace_hint_patch,
            commands::apply_jsonl_metadata_migration,
            commands::verify_repair,
            commands::rollback_repair,
            commands::get_app_settings,
            commands::update_app_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running Codex Recall");
}

#[cfg(target_os = "macos")]
fn configure_app_display_name() {
    use objc2_foundation::{NSProcessInfo, NSString};

    NSProcessInfo::processInfo().setProcessName(&NSString::from_str("Codex Recall"));
}

#[cfg(not(target_os = "macos"))]
fn configure_app_display_name() {}

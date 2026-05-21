use anyhow::{anyhow, Result};
use codex_vault_core::backup::backup_codex_home;
use codex_vault_core::discovery::discover_codex_homes;
use codex_vault_core::export::{export_sessions, generate_report};
use codex_vault_core::models::{
    BackupRequest, ExportFormat, ExportRequest, ReportFormat, ReportRequest, ScanOptions,
    SearchQuery, SessionQuery, VisibilityRisk,
};
use codex_vault_core::scan::run_scan;
use codex_vault_core::search::{list_sessions, search_sessions};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        return Ok(());
    }
    let command = args.remove(0);
    match command.as_str() {
        "discover" => {
            println!("{}", serde_json::to_string_pretty(&discover_codex_homes())?);
        }
        "scan" => {
            let home = option_value(&args, "--codex-home")
                .ok_or_else(|| anyhow!("--codex-home is required"))?;
            let out = option_value(&args, "--out");
            let result = run_scan(PathBuf::from(home), ScanOptions::default())?;
            let json = serde_json::to_string_pretty(&result)?;
            if let Some(out) = out {
                fs::write(out, json)?;
            } else {
                println!("{json}");
            }
        }
        "list" => {
            let home = option_value(&args, "--codex-home")
                .ok_or_else(|| anyhow!("--codex-home is required"))?;
            let risk = option_value(&args, "--risk").and_then(|risk| match risk.as_str() {
                "high" => Some(VisibilityRisk::High),
                "medium" => Some(VisibilityRisk::Medium),
                "low" => Some(VisibilityRisk::Low),
                _ => None,
            });
            let result = run_scan(PathBuf::from(home), ScanOptions::default())?;
            let page = list_sessions(
                &result.sessions,
                &SessionQuery {
                    risk,
                    limit: Some(200),
                    ..SessionQuery::default()
                },
            );
            println!("{}", serde_json::to_string_pretty(&page)?);
        }
        "search" => {
            let home = option_value(&args, "--codex-home")
                .ok_or_else(|| anyhow!("--codex-home is required"))?;
            let text = option_value(&args, "--q").unwrap_or_default();
            let result = run_scan(PathBuf::from(home), ScanOptions::default())?;
            let results = search_sessions(
                &result.sessions,
                &SearchQuery {
                    text,
                    limit: Some(50),
                    offset: None,
                },
            );
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        "export" => {
            let home = option_value(&args, "--codex-home")
                .ok_or_else(|| anyhow!("--codex-home is required"))?;
            let out = option_value(&args, "--out").ok_or_else(|| anyhow!("--out is required"))?;
            let format = match option_value(&args, "--format").as_deref() {
                Some("json") => ExportFormat::Json,
                Some("html") => ExportFormat::Html,
                _ => ExportFormat::Markdown,
            };
            let scan = run_scan(PathBuf::from(home), ScanOptions::default())?;
            let result = export_sessions(
                &scan,
                &ExportRequest {
                    scan_id: scan.scan_id.clone(),
                    session_ids: Vec::new(),
                    format,
                    output_path: out,
                    include_raw: false,
                    redact: args.iter().any(|arg| arg == "--redact"),
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "backup" => {
            let home = option_value(&args, "--codex-home")
                .ok_or_else(|| anyhow!("--codex-home is required"))?;
            let out = option_value(&args, "--out").ok_or_else(|| anyhow!("--out is required"))?;
            let result = backup_codex_home(&BackupRequest {
                codex_home: home,
                output_path: out,
            })?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "report" => {
            let home = option_value(&args, "--codex-home")
                .ok_or_else(|| anyhow!("--codex-home is required"))?;
            let out = option_value(&args, "--out").ok_or_else(|| anyhow!("--out is required"))?;
            let scan = run_scan(PathBuf::from(home), ScanOptions::default())?;
            let result = generate_report(
                &scan,
                &ReportRequest {
                    scan_id: scan.scan_id.clone(),
                    format: ReportFormat::Markdown,
                    output_path: out,
                    redact: args.iter().any(|arg| arg == "--redact"),
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "serve" => return Err(anyhow!("serve is forbidden in Phase 1")),
        _ => print_help(),
    }
    Ok(())
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
}

fn print_help() {
    eprintln!(
        "codex-vault commands: discover | scan | list | search | export | backup | report\nNo serve command is available."
    );
}

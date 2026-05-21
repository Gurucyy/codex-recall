use crate::models::{
    ExportFormat, ExportRequest, ExportResult, LocalSession, ReportFormat, ReportRequest,
    ReportResult, ScanResult, VisibilityRisk,
};
use crate::redaction::redact_text;
use crate::utils::ensure_parent_dir;
use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub fn export_sessions(scan: &ScanResult, request: &ExportRequest) -> Result<ExportResult> {
    let sessions = select_sessions(scan, &request.session_ids);
    if sessions.is_empty() {
        return Err(anyhow!("no matching sessions to export"));
    }

    let output = PathBuf::from(&request.output_path);
    let mut output_paths = Vec::new();
    let content = match request.format {
        ExportFormat::Markdown => render_markdown(&sessions, request.redact),
        ExportFormat::Json => render_json(&sessions, request.include_raw, request.redact)?,
        ExportFormat::Html => render_html(&sessions, request.redact),
    };

    if sessions.len() == 1 && output.extension().is_some() {
        ensure_parent_dir(&output)?;
        fs::write(&output, content)
            .with_context(|| format!("write export {}", output.display()))?;
        output_paths.push(output.to_string_lossy().to_string());
    } else {
        fs::create_dir_all(&output)
            .with_context(|| format!("create export directory {}", output.display()))?;
        let file_name = match request.format {
            ExportFormat::Markdown => "codex-sessions.md",
            ExportFormat::Json => "codex-sessions.json",
            ExportFormat::Html => "codex-sessions.html",
        };
        let path = output.join(file_name);
        fs::write(&path, content).with_context(|| format!("write export {}", path.display()))?;
        output_paths.push(path.to_string_lossy().to_string());
    }

    Ok(ExportResult {
        output_paths,
        sessions_exported: sessions.len(),
        warnings: Vec::new(),
    })
}

pub fn generate_report(scan: &ScanResult, request: &ReportRequest) -> Result<ReportResult> {
    let output = PathBuf::from(&request.output_path);
    ensure_parent_dir(&output)?;
    let content = match request.format {
        ReportFormat::Markdown => render_report_markdown(scan, request.redact),
        ReportFormat::Json => render_report_json(scan, request.redact)?,
    };
    fs::write(&output, content).with_context(|| format!("write report {}", output.display()))?;

    Ok(ReportResult {
        output_path: output.to_string_lossy().to_string(),
        diagnostics_count: scan.diagnostics.len()
            + scan
                .sessions
                .iter()
                .map(|session| session.diagnostics.len())
                .sum::<usize>(),
        high_risk_sessions: scan.summary.high_risk_sessions,
        medium_risk_sessions: scan.summary.medium_risk_sessions,
    })
}

fn select_sessions<'a>(scan: &'a ScanResult, session_ids: &[String]) -> Vec<&'a LocalSession> {
    if session_ids.is_empty() {
        return scan.sessions.iter().collect();
    }
    scan.sessions
        .iter()
        .filter(|session| session_ids.iter().any(|id| id == &session.id))
        .collect()
}

fn render_markdown(sessions: &[&LocalSession], redact: bool) -> String {
    let mut out = String::new();
    out.push_str("# Codex Session Export\n\n");
    for session in sessions {
        out.push_str(&format!(
            "## {}\n\n",
            escape_md(&redact_text(&session.title, redact))
        ));
        out.push_str(&format!("- Thread ID: `{}`\n", escape_md(&session.id)));
        out.push_str(&format!("- Risk: `{:?}`\n", session.visibility_risk));
        out.push_str(&format!("- Archived: `{}`\n", session.archived));
        if let Some(cwd) = &session.cwd_raw {
            out.push_str(&format!(
                "- Cwd: `{}`\n",
                escape_md(&redact_text(cwd, redact))
            ));
        }
        if let Some(path) = &session.rollout_path {
            out.push_str(&format!(
                "- Rollout: `{}`\n",
                escape_md(&redact_text(path, redact))
            ));
        }
        if !session.diagnostics.is_empty() {
            out.push_str("\n### Diagnostics\n\n");
            for diagnostic in &session.diagnostics {
                out.push_str(&format!(
                    "- `{}` {:?}: {}\n",
                    diagnostic.code,
                    diagnostic.severity,
                    escape_md(&diagnostic.message)
                ));
            }
        }
        out.push_str("\n### Transcript\n\n");
        for message in &session.messages {
            out.push_str(&format!(
                "#### {:?}\n\n{}\n\n",
                message.role,
                escape_md(&redact_text(&message.content, redact))
            ));
        }
    }
    out
}

fn render_json(sessions: &[&LocalSession], include_raw: bool, redact: bool) -> Result<String> {
    let values = sessions
        .iter()
        .map(|session| {
            let mut value = serde_json::to_value(session)?;
            if redact {
                redact_json_strings(&mut value);
            }
            if !include_raw {
                strip_raw_fields(&mut value);
            }
            Ok(value)
        })
        .collect::<Result<Vec<_>, serde_json::Error>>()?;
    Ok(serde_json::to_string_pretty(&values)?)
}

fn render_html(sessions: &[&LocalSession], redact: bool) -> String {
    let mut body = String::new();
    for session in sessions {
        body.push_str(&format!(
            "<section class=\"session\"><h2>{}</h2><dl><dt>Thread ID</dt><dd><code>{}</code></dd><dt>Risk</dt><dd>{:?}</dd><dt>Archived</dt><dd>{}</dd></dl>",
            html_escape(&redact_text(&session.title, redact)),
            html_escape(&session.id),
            session.visibility_risk,
            session.archived
        ));
        if !session.diagnostics.is_empty() {
            body.push_str("<h3>Diagnostics</h3><ul>");
            for diagnostic in &session.diagnostics {
                body.push_str(&format!(
                    "<li><code>{}</code> <strong>{:?}</strong> {}</li>",
                    html_escape(&diagnostic.code),
                    diagnostic.severity,
                    html_escape(&diagnostic.message)
                ));
            }
            body.push_str("</ul>");
        }
        body.push_str("<h3>Transcript</h3>");
        for message in &session.messages {
            body.push_str(&format!(
                "<article class=\"message\"><header>{:?}</header><pre>{}</pre></article>",
                message.role,
                html_escape(&redact_text(&message.content, redact))
            ));
        }
        body.push_str("</section>");
    }

    format!(
        r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Codex Session Export</title>
<style>
:root {{ color-scheme: light; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; background: #f7f7f4; color: #1e2324; }}
body {{ margin: 0; padding: 32px; }}
h1, h2, h3 {{ font-family: Georgia, 'Times New Roman', serif; letter-spacing: 0; }}
.session {{ border-top: 2px solid #1e2324; padding: 24px 0; }}
dl {{ display: grid; grid-template-columns: max-content 1fr; gap: 6px 16px; }}
dt {{ color: #5d6468; }}
.message {{ border: 1px solid #cfd5d2; background: #fff; margin: 12px 0; }}
.message header {{ padding: 8px 12px; background: #e8ebe5; font-weight: 700; }}
pre {{ white-space: pre-wrap; overflow-wrap: anywhere; padding: 12px; margin: 0; }}
code {{ background: #ecefeb; padding: 1px 4px; }}
</style>
<body><h1>Codex Session Export</h1>{body}</body></html>"#
    )
}

fn render_report_markdown(scan: &ScanResult, redact: bool) -> String {
    let mut out = String::new();
    out.push_str("# Codex Recovery Report\n\n");
    out.push_str("Phase 1 report only. This report does not repair or write back Codex data.\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "- Codex home: `{}`\n",
        escape_md(&redact_text(&scan.codex_home, redact))
    ));
    out.push_str(&format!(
        "- Total sessions: `{}`\n",
        scan.summary.total_sessions
    ));
    out.push_str(&format!(
        "- High risk sessions: `{}`\n",
        scan.summary.high_risk_sessions
    ));
    out.push_str(&format!(
        "- Medium risk sessions: `{}`\n",
        scan.summary.medium_risk_sessions
    ));
    out.push_str(&format!(
        "- Corrupt SQLite databases: `{}`\n",
        scan.summary.corrupt_sqlite_databases
    ));

    if !scan.diagnostics.is_empty() {
        out.push_str("\n## Critical Scan-Level Issues\n\n");
        for diagnostic in &scan.diagnostics {
            out.push_str(&format!(
                "- `{}` {:?}: {}\n",
                diagnostic.code,
                diagnostic.severity,
                escape_md(&diagnostic.message)
            ));
        }
    }

    for risk in [VisibilityRisk::High, VisibilityRisk::Medium] {
        let label = match risk {
            VisibilityRisk::High => "High-Risk Sessions",
            VisibilityRisk::Medium => "Medium-Risk Sessions",
            VisibilityRisk::Low => "Low-Risk Sessions",
        };
        let sessions = scan
            .sessions
            .iter()
            .filter(|session| session.visibility_risk == risk)
            .collect::<Vec<_>>();
        if sessions.is_empty() {
            continue;
        }
        out.push_str(&format!("\n## {label}\n\n"));
        for session in sessions {
            out.push_str(&format!(
                "### {} `{}`\n\n",
                escape_md(&redact_text(&session.title, redact)),
                escape_md(&session.id)
            ));
            for diagnostic in &session.diagnostics {
                out.push_str(&format!(
                    "- `{}` {:?}: {}\n",
                    diagnostic.code,
                    diagnostic.severity,
                    escape_md(&diagnostic.message)
                ));
            }
        }
    }

    out.push_str("\n## Suggested Next Steps\n\n");
    out.push_str("- View and export important sessions with this app.\n");
    out.push_str("- Create a backup before attempting any manual repair outside this app.\n");
    out.push_str("- Keep this report with the backup for later review.\n");
    out
}

fn render_report_json(scan: &ScanResult, redact: bool) -> Result<String> {
    let mut value = json!({
        "summary": scan.summary,
        "diagnostics": scan.diagnostics,
        "sessions": scan.sessions.iter().map(|session| {
            json!({
                "id": session.id,
                "title": session.title,
                "visibility_risk": session.visibility_risk,
                "cwd_raw": session.cwd_raw,
                "rollout_path": session.rollout_path,
                "diagnostics": session.diagnostics
            })
        }).collect::<Vec<_>>()
    });
    if redact {
        redact_json_strings(&mut value);
    }
    Ok(serde_json::to_string_pretty(&value)?)
}

fn redact_json_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
            *text = redact_text(text, true);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_json_strings(item);
            }
        }
        serde_json::Value::Object(map) => {
            for value in map.values_mut() {
                redact_json_strings(value);
            }
        }
        _ => {}
    }
}

fn strip_raw_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("raw");
            for value in map.values_mut() {
                strip_raw_fields(value);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_raw_fields(item);
            }
        }
        _ => {}
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_md(value: &str) -> String {
    value.replace('\\', "\\\\").replace('`', "\\`")
}

#[allow(dead_code)]
fn has_extension(path: &Path) -> bool {
    path.extension().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ScanSummary, VisibilityRisk};
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn html_export_escapes_content() {
        let session = LocalSession {
            id: "a".to_string(),
            title: "<Title>".to_string(),
            title_sources: vec![],
            created_at: None,
            updated_at: None,
            source: None,
            cwd_raw: None,
            cwd_canonical: None,
            workspace_key: None,
            archived: false,
            archived_sources: vec![],
            rollout_path: None,
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
            confidence: 1.0,
            recent_rank: None,
            jsonl_malformed_lines: 0,
            jsonl_raw_events_count: 0,
            jsonl_parse_errors: vec![],
            jsonl_archived_by_path: None,
            path_aliases: vec![],
        };
        let scan = ScanResult {
            scan_id: "scan".to_string(),
            codex_home: "/codex".to_string(),
            created_at: Utc::now(),
            sessions: vec![session],
            workspaces: vec![],
            sqlite_databases: vec![],
            session_index: None,
            global_state: None,
            diagnostics: vec![],
            warnings: vec![],
            errors: vec![],
            summary: ScanSummary::default(),
        };
        let temp = tempdir().unwrap();
        let output = temp.path().join("out.html");
        export_sessions(
            &scan,
            &ExportRequest {
                scan_id: "scan".to_string(),
                session_ids: vec!["a".to_string()],
                format: ExportFormat::Html,
                output_path: output.to_string_lossy().to_string(),
                include_raw: false,
                redact: false,
            },
        )
        .unwrap();
        let html = fs::read_to_string(output).unwrap();
        assert!(html.contains("&lt;Title&gt;"));
        assert!(!html.contains("<Title>"));
    }
}

use crate::diagnostics::apply_diagnostics;
use crate::global_state_reader::read_global_state;
use crate::jsonl_parser::scan_jsonl_sessions_with_progress;
use crate::models::{ScanOptions, ScanResult, ScanSummary, VisibilityRisk};
use crate::normalizer::{build_workspace_groups, normalize_all};
use crate::session_index_reader::read_session_index;
use crate::snapshot::Snapshot;
use crate::sqlite_reader::read_sqlite_databases_with_progress;
use crate::utils::path_to_string;
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

pub const SCAN_PROGRESS_TOTAL: usize = 1000;

const PROGRESS_SNAPSHOT_DONE: usize = 50;
const PROGRESS_JSONL_DONE: usize = 620;
const PROGRESS_INDEX_DONE: usize = 660;
const PROGRESS_SQLITE_DONE: usize = 810;
const PROGRESS_GLOBAL_STATE_DONE: usize = 850;
const PROGRESS_NORMALIZE_DONE: usize = 910;
const PROGRESS_DIAGNOSTICS_DONE: usize = 970;
const PROGRESS_SUMMARY_DONE: usize = 990;

pub fn run_scan(codex_home: PathBuf, options: ScanOptions) -> Result<ScanResult> {
    run_scan_with_progress(
        codex_home,
        options,
        |_stage, _message, _completed, _total| {},
    )
}

pub fn run_scan_with_progress<F>(
    codex_home: PathBuf,
    _options: ScanOptions,
    mut progress: F,
) -> Result<ScanResult>
where
    F: FnMut(&str, &str, usize, usize),
{
    progress(
        "snapshot",
        "Preparing read-only metadata snapshot",
        0,
        SCAN_PROGRESS_TOTAL,
    );
    let snapshot = Snapshot::create(&codex_home).context("create read-only metadata snapshot")?;
    let mut warnings = snapshot.warnings.clone();
    let errors = Vec::new();
    progress(
        "snapshot",
        "Prepared read-only metadata snapshot",
        PROGRESS_SNAPSHOT_DONE,
        SCAN_PROGRESS_TOTAL,
    );

    progress(
        "jsonl",
        "Reading transcript files",
        PROGRESS_SNAPSHOT_DONE,
        SCAN_PROGRESS_TOTAL,
    );
    let mut last_jsonl_progress = PROGRESS_SNAPSHOT_DONE;
    let jsonl_sessions =
        scan_jsonl_sessions_with_progress(&codex_home, |completed_bytes, total_bytes| {
            let next_progress = scaled_progress(
                PROGRESS_SNAPSHOT_DONE,
                PROGRESS_JSONL_DONE,
                completed_bytes,
                total_bytes,
            );
            if next_progress > last_jsonl_progress {
                last_jsonl_progress = next_progress;
                progress(
                    "jsonl",
                    "Reading transcript files",
                    next_progress,
                    SCAN_PROGRESS_TOTAL,
                );
            }
        });
    progress(
        "jsonl",
        "Read transcript files",
        PROGRESS_JSONL_DONE,
        SCAN_PROGRESS_TOTAL,
    );

    progress(
        "index",
        "Reading session index",
        PROGRESS_JSONL_DONE,
        SCAN_PROGRESS_TOTAL,
    );
    let session_index = read_session_index(&snapshot);
    progress(
        "index",
        "Read session index",
        PROGRESS_INDEX_DONE,
        SCAN_PROGRESS_TOTAL,
    );

    progress(
        "sqlite",
        "Reading SQLite state",
        PROGRESS_INDEX_DONE,
        SCAN_PROGRESS_TOTAL,
    );
    let mut last_sqlite_progress = PROGRESS_INDEX_DONE;
    let sqlite_databases = read_sqlite_databases_with_progress(&snapshot, |completed, total| {
        let next_progress = scaled_progress(
            PROGRESS_INDEX_DONE,
            PROGRESS_SQLITE_DONE,
            completed as u64,
            total as u64,
        );
        if next_progress > last_sqlite_progress {
            last_sqlite_progress = next_progress;
            progress(
                "sqlite",
                "Reading SQLite state",
                next_progress,
                SCAN_PROGRESS_TOTAL,
            );
        }
    });
    progress(
        "sqlite",
        "Read SQLite state",
        PROGRESS_SQLITE_DONE,
        SCAN_PROGRESS_TOTAL,
    );

    progress(
        "global-state",
        "Reading global state",
        PROGRESS_SQLITE_DONE,
        SCAN_PROGRESS_TOTAL,
    );
    let global_state = read_global_state(&snapshot);
    progress(
        "global-state",
        "Read global state",
        PROGRESS_GLOBAL_STATE_DONE,
        SCAN_PROGRESS_TOTAL,
    );

    if !global_state.parsed_ok && global_state.path.is_some() {
        warnings.extend(
            global_state
                .errors
                .iter()
                .map(|error| format!("global state parse warning: {error}")),
        );
    }
    if !session_index.malformed_lines.is_empty() {
        warnings.push(format!(
            "session_index.jsonl has {} malformed line(s)",
            session_index.malformed_lines.len()
        ));
    }

    progress(
        "normalize",
        "Normalizing sessions",
        PROGRESS_GLOBAL_STATE_DONE,
        SCAN_PROGRESS_TOTAL,
    );
    let (mut sessions, session_index) = normalize_all(
        &jsonl_sessions,
        &sqlite_databases,
        &session_index,
        &global_state,
    );
    progress(
        "normalize",
        "Normalized sessions",
        PROGRESS_NORMALIZE_DONE,
        SCAN_PROGRESS_TOTAL,
    );

    progress(
        "diagnostics",
        "Applying diagnostics",
        PROGRESS_NORMALIZE_DONE,
        SCAN_PROGRESS_TOTAL,
    );
    let diagnostics = apply_diagnostics(
        &mut sessions,
        &session_index,
        &global_state,
        &sqlite_databases,
    );
    progress(
        "diagnostics",
        "Applied diagnostics",
        PROGRESS_DIAGNOSTICS_DONE,
        SCAN_PROGRESS_TOTAL,
    );

    progress(
        "summary",
        "Preparing result",
        PROGRESS_DIAGNOSTICS_DONE,
        SCAN_PROGRESS_TOTAL,
    );
    let workspaces = build_workspace_groups(&sessions);
    let summary = summarize(
        &sessions,
        jsonl_sessions.len(),
        &sqlite_databases,
        session_index.records_by_id.len(),
    );
    progress(
        "summary",
        "Prepared result",
        PROGRESS_SUMMARY_DONE,
        SCAN_PROGRESS_TOTAL,
    );

    Ok(ScanResult {
        scan_id: Uuid::new_v4().to_string(),
        codex_home: path_to_string(&codex_home),
        created_at: Utc::now(),
        sessions,
        workspaces,
        sqlite_databases,
        session_index: Some(session_index),
        global_state: Some(global_state),
        diagnostics,
        warnings,
        errors,
        summary,
    })
}

fn scaled_progress(start: usize, end: usize, completed: u64, total: u64) -> usize {
    if total == 0 {
        return end;
    }
    let span = end.saturating_sub(start) as u64;
    let completed = completed.min(total);
    start + ((span * completed) / total) as usize
}

fn summarize(
    sessions: &[crate::models::LocalSession],
    jsonl_sessions: usize,
    sqlite_databases: &[crate::models::SqliteDatabaseSummary],
    index_records: usize,
) -> ScanSummary {
    ScanSummary {
        total_sessions: sessions.len(),
        active_sessions: sessions.iter().filter(|session| !session.archived).count(),
        archived_sessions: sessions.iter().filter(|session| session.archived).count(),
        high_risk_sessions: sessions
            .iter()
            .filter(|session| session.visibility_risk == VisibilityRisk::High)
            .count(),
        medium_risk_sessions: sessions
            .iter()
            .filter(|session| session.visibility_risk == VisibilityRisk::Medium)
            .count(),
        jsonl_sessions,
        sqlite_threads: sqlite_databases
            .iter()
            .map(|database| database.threads.len())
            .sum(),
        index_records,
        malformed_jsonl_files: sessions
            .iter()
            .filter(|session| session.jsonl_malformed_lines > 0)
            .count(),
        corrupt_sqlite_databases: sqlite_databases
            .iter()
            .filter(|database| !database.integrity_ok)
            .count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn full_scan_handles_malformed_jsonl() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("sessions")).unwrap();
        fs::write(
            temp.path().join("sessions").join("thread-a.jsonl"),
            r#"{"thread_id":"thread-a","cwd":"/repo","role":"user","content":"hello"}"#.to_string()
                + "\n{bad",
        )
        .unwrap();
        let result = run_scan(temp.path().to_path_buf(), ScanOptions::default()).unwrap();
        assert_eq!(result.summary.total_sessions, 1);
        assert_eq!(result.summary.malformed_jsonl_files, 1);
        assert!(result.sessions[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "R015_MALFORMED_JSONL"));
    }

    #[test]
    fn scan_progress_reports_intermediate_jsonl_steps() {
        let temp = tempdir().unwrap();
        let sessions = temp.path().join("sessions");
        fs::create_dir(&sessions).unwrap();
        for index in 0..5 {
            fs::write(
                sessions.join(format!("thread-{index}.jsonl")),
                format!(
                    "{{\"thread_id\":\"thread-{index}\",\"cwd\":\"/repo\",\"role\":\"user\",\"content\":\"hello {index}\"}}\n"
                ),
            )
            .unwrap();
        }

        let mut events = Vec::new();
        run_scan_with_progress(
            temp.path().to_path_buf(),
            ScanOptions::default(),
            |stage, _message, completed, total| {
                events.push((stage.to_string(), completed, total));
            },
        )
        .unwrap();

        let jsonl_progress = events
            .iter()
            .filter(|(stage, _, _)| stage == "jsonl")
            .map(|(_, completed, _)| *completed)
            .collect::<Vec<_>>();
        assert!(
            jsonl_progress.len() > 3,
            "expected multiple JSONL progress updates, got {jsonl_progress:?}"
        );
        assert!(jsonl_progress.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(
            events.last().map(|(_, _, total)| *total),
            Some(SCAN_PROGRESS_TOTAL)
        );
    }
}

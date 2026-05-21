use crate::models::{IndexRecord, ParseError, SessionIndexSummary};
use crate::snapshot::Snapshot;
use crate::utils::{find_datetime, find_string, path_to_string, safe_snippet};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn read_session_index(snapshot: &Snapshot) -> SessionIndexSummary {
    let path = snapshot.snapshot_path("session_index.jsonl");
    if !path.is_file() {
        return SessionIndexSummary::default();
    }

    let mut summary = SessionIndexSummary {
        path: Some(path_to_string(&snapshot.source_path("session_index.jsonl"))),
        ..SessionIndexSummary::default()
    };

    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            summary.malformed_lines.push(ParseError {
                path: summary.path.clone(),
                line_no: None,
                message: error.to_string(),
                snippet: None,
            });
            return summary;
        }
    };

    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line_no = line_index + 1;
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                summary.malformed_lines.push(ParseError {
                    path: summary.path.clone(),
                    line_no: Some(line_no),
                    message: error.to_string(),
                    snippet: None,
                });
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let raw: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                summary.malformed_lines.push(ParseError {
                    path: summary.path.clone(),
                    line_no: Some(line_no),
                    message: error.to_string(),
                    snippet: Some(safe_snippet(&line, 200)),
                });
                continue;
            }
        };

        let Some(id) = find_string(
            &raw,
            &[
                "id",
                "thread_id",
                "threadId",
                "session_id",
                "sessionId",
                "conversation_id",
                "conversationId",
            ],
        ) else {
            summary.malformed_lines.push(ParseError {
                path: summary.path.clone(),
                line_no: Some(line_no),
                message: "index record has no thread id".to_string(),
                snippet: Some(safe_snippet(&line, 200)),
            });
            continue;
        };

        let record = IndexRecord {
            id: id.clone(),
            thread_name: find_string(&raw, &["thread_name", "threadName", "title", "name"]),
            updated_at: find_datetime(
                &raw,
                &["updated_at", "updatedAt", "last_updated_at", "time"],
            ),
            raw,
            line_no,
        };
        summary
            .all_records_by_id
            .entry(id)
            .or_default()
            .push(record);
    }

    for (id, records) in &summary.all_records_by_id {
        if records.len() > 1 {
            summary.duplicate_ids.push(id.clone());
        }
        let canonical = choose_canonical(records);
        summary.records_by_id.insert(id.clone(), canonical.clone());
    }
    summary.duplicate_ids.sort();

    summary
}

fn choose_canonical(records: &[IndexRecord]) -> &IndexRecord {
    records
        .iter()
        .max_by(|a, b| match (a.updated_at, b.updated_at) {
            (Some(left), Some(right)) => left.cmp(&right).then_with(|| a.line_no.cmp(&b.line_no)),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => a.line_no.cmp(&b.line_no),
        })
        .expect("records is non-empty")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::Snapshot;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn reads_duplicates_and_picks_latest() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("session_index.jsonl"),
            r#"{"id":"a","thread_name":"Old","updated_at":"2026-01-01T00:00:00Z"}"#.to_string()
                + "\n"
                + r#"{"id":"a","thread_name":"New","updated_at":"2026-01-02T00:00:00Z"}"#
                + "\nmalformed",
        )
        .unwrap();
        let snapshot = Snapshot::create(temp.path()).unwrap();
        let summary = read_session_index(&snapshot);

        assert_eq!(summary.duplicate_ids, vec!["a"]);
        assert_eq!(
            summary.records_by_id["a"].thread_name.as_deref(),
            Some("New")
        );
        assert_eq!(summary.malformed_lines.len(), 1);
    }
}

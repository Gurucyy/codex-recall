use crate::models::{JsonlSession, JsonlSessionMeta, Message, MessageRole, ParseError};
use crate::utils::{
    as_non_empty_string, file_mtime, filename_stem, find_datetime, find_string, path_to_string,
    relative_path, safe_snippet,
};
use anyhow::Result;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const JSONL_PROGRESS_CHUNK_BYTES: u64 = 128 * 1024;

pub fn scan_jsonl_sessions(codex_home: &Path) -> Vec<JsonlSession> {
    scan_jsonl_sessions_with_progress(codex_home, |_completed, _total| {})
}

pub fn scan_jsonl_sessions_with_progress<F>(codex_home: &Path, mut progress: F) -> Vec<JsonlSession>
where
    F: FnMut(u64, u64),
{
    let mut sessions = Vec::new();
    let paths = collect_jsonl_paths(codex_home);
    let total_bytes = paths.iter().map(|(_, _, size)| (*size).max(1)).sum();

    if paths.is_empty() {
        progress(0, 0);
        return sessions;
    }

    let mut completed_before_file = 0u64;
    for (path, archived_by_path, file_size) in paths {
        let file_weight = file_size.max(1);
        let mut last_reported_for_file = 0u64;
        let session =
            parse_jsonl_session_with_progress(codex_home, &path, archived_by_path, |bytes_read| {
                let completed_for_file = bytes_read.min(file_weight);
                if completed_for_file.saturating_sub(last_reported_for_file)
                    >= JSONL_PROGRESS_CHUNK_BYTES
                    || completed_for_file >= file_weight
                {
                    last_reported_for_file = completed_for_file;
                    progress(completed_before_file + completed_for_file, total_bytes);
                }
            });
        completed_before_file += file_weight;
        progress(completed_before_file, total_bytes);
        sessions.push(session);
    }

    sessions
}

pub fn parse_jsonl_session(codex_home: &Path, path: &Path, archived_by_path: bool) -> JsonlSession {
    parse_jsonl_session_with_progress(codex_home, path, archived_by_path, |_bytes_read| {})
}

fn parse_jsonl_session_with_progress<F>(
    codex_home: &Path,
    path: &Path,
    archived_by_path: bool,
    mut progress: F,
) -> JsonlSession
where
    F: FnMut(u64),
{
    let metadata = fs::metadata(path).ok();
    let file_size = metadata
        .as_ref()
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let file_mtime = file_mtime(path);
    let relative_path = relative_path(codex_home, path);
    let inferred_id = filename_stem(path);

    let mut session = JsonlSession {
        id: None,
        inferred_id,
        rollout_path: path_to_string(path),
        relative_path,
        archived_by_path,
        file_size,
        file_mtime,
        meta: JsonlSessionMeta::default(),
        messages: Vec::new(),
        raw_events_count: 0,
        malformed_lines: 0,
        parse_errors: Vec::new(),
        first_user_message: None,
        title_candidate: None,
    };

    if let Err(error) = parse_lines(path, &mut session, &mut progress) {
        session.malformed_lines += 1;
        session.parse_errors.push(ParseError {
            path: Some(path_to_string(path)),
            line_no: None,
            message: error.to_string(),
            snippet: None,
        });
    }

    session.first_user_message = session
        .messages
        .iter()
        .find(|message| message.role == MessageRole::User && !message.content.trim().is_empty())
        .map(|message| message.content.clone());
    session.title_candidate = session
        .first_user_message
        .as_ref()
        .map(|text| make_title_candidate(text));

    if session.meta.updated_at.is_none() {
        session.meta.updated_at = session
            .messages
            .iter()
            .filter_map(|message| message.created_at)
            .max()
            .or(session.file_mtime);
    }
    if session.meta.created_at.is_none() {
        session.meta.created_at = session
            .messages
            .iter()
            .filter_map(|message| message.created_at)
            .min()
            .or(session.file_mtime);
    }

    session
}

fn collect_jsonl_paths(codex_home: &Path) -> Vec<(PathBuf, bool, u64)> {
    let mut paths = Vec::new();
    for (relative_root, archived_by_path) in [("sessions", false), ("archived_sessions", true)] {
        let root = codex_home.join(relative_root);
        if !root.exists() {
            continue;
        }

        let mut root_paths = WalkDir::new(&root)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("jsonl"))
            .map(|path| {
                let size = fs::metadata(&path)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                (path, archived_by_path, size)
            })
            .collect::<Vec<_>>();
        root_paths.sort_by(|a, b| a.0.cmp(&b.0));
        paths.extend(root_paths);
    }
    paths
}

fn parse_lines<F>(path: &Path, session: &mut JsonlSession, progress: &mut F) -> Result<()>
where
    F: FnMut(u64),
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut bytes_read = 0u64;

    for (line_index, line) in reader.lines().enumerate() {
        let line_no = line_index + 1;
        let line = match line {
            Ok(line) => {
                bytes_read = bytes_read.saturating_add(line.as_bytes().len() as u64 + 1);
                progress(bytes_read);
                line
            }
            Err(error) => {
                session.malformed_lines += 1;
                session.parse_errors.push(ParseError {
                    path: Some(path_to_string(path)),
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

        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                session.malformed_lines += 1;
                session.parse_errors.push(ParseError {
                    path: Some(path_to_string(path)),
                    line_no: Some(line_no),
                    message: error.to_string(),
                    snippet: Some(safe_snippet(&line, 200)),
                });
                continue;
            }
        };

        session.raw_events_count += 1;
        absorb_metadata(&value, session);
        if session.id.is_none() {
            session.id = infer_thread_id(&value);
        }
        if let Some(message) = extract_message(&value) {
            session.messages.push(message);
        }
    }

    Ok(())
}

fn absorb_metadata(value: &Value, session: &mut JsonlSession) {
    if session.meta.cwd.is_none() {
        session.meta.cwd = find_string(
            value,
            &["cwd", "current_working_directory", "workspace_root"],
        );
    }
    if session.meta.source.is_none() {
        session.meta.source = find_string(value, &["source", "origin", "client"]);
    }
    if session.meta.model.is_none() {
        session.meta.model = find_string(value, &["model", "model_slug", "model_id"]);
    }
    if session.meta.created_at.is_none() {
        session.meta.created_at =
            find_datetime(value, &["created_at", "createdAt", "timestamp", "time"]);
    }
    if let Some(updated_at) =
        find_datetime(value, &["updated_at", "updatedAt", "timestamp", "time"])
    {
        session.meta.updated_at = session
            .meta
            .updated_at
            .map(|current| current.max(updated_at))
            .or(Some(updated_at));
    }

    if session.meta.raw.is_none()
        && (value.get("cwd").is_some()
            || value.get("session").is_some()
            || value.get("metadata").is_some()
            || value.get("type").and_then(Value::as_str) == Some("session_meta"))
    {
        session.meta.raw = Some(value.clone());
    }
}

fn infer_thread_id(value: &Value) -> Option<String> {
    find_string(
        value,
        &[
            "thread_id",
            "threadId",
            "session_id",
            "sessionId",
            "conversation_id",
            "conversationId",
            "rollout_id",
            "rolloutId",
            "id",
        ],
    )
}

fn extract_message(value: &Value) -> Option<Message> {
    let role = extract_role(value)?;
    let content = extract_content(value)?;
    let created_at = find_datetime(value, &["created_at", "createdAt", "timestamp", "time"]);
    let raw_type = value
        .get("type")
        .and_then(as_non_empty_string)
        .or_else(|| value.get("event_type").and_then(as_non_empty_string));

    Some(Message {
        role,
        content,
        created_at,
        raw_type,
        raw: Some(value.clone()),
    })
}

fn extract_role(value: &Value) -> Option<MessageRole> {
    find_string(value, &["role", "author_role", "message_role"])
        .as_deref()
        .map(|role| match role.to_ascii_lowercase().as_str() {
            "user" | "human" => MessageRole::User,
            "assistant" | "ai" => MessageRole::Assistant,
            "system" | "developer" => MessageRole::System,
            "tool" | "function" => MessageRole::Tool,
            _ => MessageRole::Unknown,
        })
}

fn extract_content(value: &Value) -> Option<String> {
    if let Some(content) = direct_content(value) {
        return Some(content);
    }

    match value {
        Value::Object(map) => {
            for key in ["message", "item", "content", "payload", "data"] {
                if let Some(child) = map.get(key).and_then(extract_content) {
                    return Some(child);
                }
            }
            None
        }
        Value::Array(items) => {
            let parts = items.iter().filter_map(extract_content).collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("\n"))
        }
        _ => None,
    }
}

fn direct_content(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.trim().to_string()).filter(|text| !text.is_empty()),
        Value::Object(map) => {
            for key in ["text", "content", "message", "output", "input"] {
                if let Some(found) = map.get(key) {
                    match found {
                        Value::String(text) if !text.trim().is_empty() => {
                            return Some(text.trim().to_string());
                        }
                        Value::Array(items) => {
                            let parts =
                                items.iter().filter_map(extract_content).collect::<Vec<_>>();
                            if !parts.is_empty() {
                                return Some(parts.join("\n"));
                            }
                        }
                        _ => {}
                    }
                }
            }
            None
        }
        Value::Array(items) => {
            let parts = items.iter().filter_map(extract_content).collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("\n"))
        }
        _ => None,
    }
}

fn make_title_candidate(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= 80 {
        normalized
    } else {
        let mut value = normalized.chars().take(77).collect::<String>();
        value.push_str("...");
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_normal_jsonl_and_continues_after_malformed_line() {
        let temp = tempdir().unwrap();
        let sessions = temp.path().join("sessions");
        fs::create_dir(&sessions).unwrap();
        let path = sessions.join("thread-a.jsonl");
        fs::write(
            &path,
            r#"{"type":"session_meta","thread_id":"thread-a","cwd":"/repo","source":"desktop","timestamp":"2026-01-01T00:00:00Z"}"#
                .to_string()
                + "\nnot json\n"
                + r#"{"type":"message","role":"user","content":"Find my session","created_at":"2026-01-01T00:01:00Z"}"#
                + "\n",
        )
        .unwrap();

        let parsed = scan_jsonl_sessions(temp.path());
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id.as_deref(), Some("thread-a"));
        assert_eq!(parsed[0].malformed_lines, 1);
        assert_eq!(parsed[0].messages.len(), 1);
        assert_eq!(
            parsed[0].first_user_message.as_deref(),
            Some("Find my session")
        );
    }

    #[test]
    fn marks_archived_by_path() {
        let temp = tempdir().unwrap();
        let archived = temp.path().join("archived_sessions");
        fs::create_dir(&archived).unwrap();
        fs::write(archived.join("thread-a.jsonl"), "{}\n").unwrap();

        let parsed = scan_jsonl_sessions(temp.path());
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].archived_by_path);
    }
}

use crate::models::RepairOperation;
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn apply_thread_source_migrations(operations: &[RepairOperation]) -> Result<()> {
    for operation in operations {
        let path = operation
            .affected_files
            .first()
            .ok_or_else(|| anyhow!("thread_source operation has no rollout path"))?;
        let thread_id = operation
            .payload
            .get("thread_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("thread_source operation missing thread_id"))?;
        let value = operation
            .payload
            .get("value")
            .and_then(|value| value.as_str())
            .unwrap_or("user");
        apply_one(Path::new(path), thread_id, value)?;
    }
    Ok(())
}

fn apply_one(path: &Path, thread_id: &str, thread_source: &str) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("read rollout JSONL {}", path.display()))?;
    let mut lines = text.split_inclusive('\n');
    let Some(first_with_newline) = lines.next() else {
        return Err(anyhow!("rollout JSONL is empty: {}", path.display()));
    };
    let first = first_with_newline.trim_end_matches(['\r', '\n']);
    let rest = lines.collect::<String>();
    let newline = if first_with_newline.ends_with("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut first_value: Value = serde_json::from_str(first)
        .with_context(|| format!("parse first JSONL line {}", path.display()))?;
    validate_session_meta(&first_value, thread_id)?;
    let payload = first_value
        .get_mut("payload")
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| anyhow!("first JSONL line has no payload object"))?;
    if payload.contains_key("thread_source") || payload.contains_key("threadSource") {
        return Err(anyhow!("thread_source already exists for {thread_id}"));
    }
    payload.insert(
        "thread_source".to_string(),
        Value::String(thread_source.to_string()),
    );

    let mut output = serde_json::to_string(&first_value)?;
    output.push_str(newline);
    output.push_str(&rest);
    atomic_write(path, output.as_bytes())
}

fn validate_session_meta(value: &Value, thread_id: &str) -> Result<()> {
    let type_matches = value
        .get("type")
        .and_then(|value| value.as_str())
        .map(|value| value == "session_meta")
        .unwrap_or(false);
    if !type_matches {
        return Err(anyhow!("first JSONL line is not session_meta"));
    }
    let payload_id = value
        .get("payload")
        .and_then(|payload| {
            payload
                .get("id")
                .or_else(|| payload.get("thread_id"))
                .or_else(|| payload.get("threadId"))
        })
        .and_then(|value| value.as_str());
    if payload_id != Some(thread_id) {
        return Err(anyhow!("session_meta id does not match selected thread id"));
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp_path = temp_path(path);
    {
        let mut file = File::create(&temp_path)
            .with_context(|| format!("create temporary file {}", temp_path.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temp_path, path)
        .with_context(|| format!("replace {} with {}", path.display(), temp_path.display()))?;
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("rollout.jsonl")
        .to_string();
    name.push_str(".repair.tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn backfills_only_first_session_meta_line() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("rollout.jsonl");
        fs::write(
            &path,
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"thread-a\",\"cwd\":\"/repo\"}}\n{\"type\":\"message\",\"payload\":{\"content\":\"hello\"}}\n",
        )
        .unwrap();
        apply_one(&path, "thread-a", "user").unwrap();
        let text = fs::read_to_string(path).unwrap();
        let lines = text.lines().collect::<Vec<_>>();
        let first: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(
            first["payload"]["thread_source"],
            Value::String("user".to_string())
        );
        assert!(lines[1].contains("\"message\""));
    }
}

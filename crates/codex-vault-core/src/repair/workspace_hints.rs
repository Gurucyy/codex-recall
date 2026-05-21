use crate::models::RepairOperation;
use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn apply_workspace_hint_operations(operations: &[RepairOperation]) -> Result<()> {
    for operation in operations {
        let path = operation
            .affected_files
            .first()
            .ok_or_else(|| anyhow!("workspace hint operation has no target file"))?;
        apply_one(Path::new(path), operation)?;
    }
    Ok(())
}

fn apply_one(path: &Path, operation: &RepairOperation) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("read global state {}", path.display()))?;
    let mut root: Value = serde_json::from_str(&text)
        .with_context(|| format!("parse global state {}", path.display()))?;
    let object = root
        .as_object_mut()
        .ok_or_else(|| anyhow!("global state root is not a JSON object"))?;

    let hints = ensure_object(object, "thread-workspace-root-hints")?;
    let additions = operation
        .payload
        .get("additions")
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow!("workspace hint operation missing additions"))?;
    for addition in additions {
        let thread_id = addition
            .get("thread_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("workspace hint addition missing thread_id"))?;
        let workspace_root = addition
            .get("workspace_root")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("workspace hint addition missing workspace_root"))?;
        if !hints.contains_key(thread_id) {
            hints.insert(
                thread_id.to_string(),
                Value::String(workspace_root.to_string()),
            );
        }
    }
    atomic_write_json(path, &root)
}

fn ensure_object<'a>(
    root: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    if !root.contains_key(key) {
        root.insert(key.to_string(), Value::Object(Map::new()));
    }
    root.get_mut(key)
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| anyhow!("{key} exists but is not an object"))
}

fn atomic_write_json(path: &Path, value: &Value) -> Result<()> {
    let temp_path = temp_path(path);
    {
        let mut file = File::create(&temp_path)
            .with_context(|| format!("create temporary file {}", temp_path.display()))?;
        file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
        file.write_all(b"\n")?;
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
        .unwrap_or("global-state")
        .to_string();
    name.push_str(".repair.tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{RepairOperationKind, RepairRisk};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn adds_missing_hint_without_overwriting_existing() {
        let temp = tempdir().unwrap();
        let path = temp.path().join(".codex-global-state.json");
        fs::write(
            &path,
            r#"{"thread-workspace-root-hints":{"existing":"/old"}}"#,
        )
        .unwrap();
        let op = RepairOperation {
            id: "hints".to_string(),
            kind: RepairOperationKind::PatchWorkspaceHints,
            title: String::new(),
            description: String::new(),
            risk: RepairRisk::High,
            official_api: false,
            experimental: true,
            affected_session_ids: vec![],
            affected_files: vec![path.to_string_lossy().to_string()],
            evidence: json!({}),
            payload: json!({
                "additions": [
                    {"thread_id": "existing", "workspace_root": "/new"},
                    {"thread_id": "added", "workspace_root": "/repo"}
                ]
            }),
            diff: None,
        };

        apply_workspace_hint_operations(&[op]).unwrap();
        let parsed: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(
            parsed["thread-workspace-root-hints"]["existing"],
            Value::String("/old".to_string())
        );
        assert_eq!(
            parsed["thread-workspace-root-hints"]["added"],
            Value::String("/repo".to_string())
        );
    }
}

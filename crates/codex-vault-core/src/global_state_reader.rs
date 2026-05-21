use crate::models::GlobalStateSummary;
use crate::snapshot::Snapshot;
use crate::utils::path_to_string;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;

pub fn read_global_state(snapshot: &Snapshot) -> GlobalStateSummary {
    let path = snapshot.snapshot_path(".codex-global-state.json");
    if !path.is_file() {
        return GlobalStateSummary::default();
    }

    let source_path = snapshot.source_path(".codex-global-state.json");
    let mut summary = GlobalStateSummary {
        path: Some(path_to_string(&source_path)),
        parsed_ok: false,
        ..GlobalStateSummary::default()
    };

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            summary.errors.push(error.to_string());
            return summary;
        }
    };

    let raw: Value = match serde_json::from_str(&text) {
        Ok(raw) => raw,
        Err(error) => {
            summary.errors.push(error.to_string());
            return summary;
        }
    };

    summary.parsed_ok = true;
    summary.thread_workspace_root_hints = extract_string_map(
        &raw,
        &["thread-workspace-root-hints", "threadWorkspaceRootHints"],
    );
    summary.projectless_thread_ids =
        extract_string_set(&raw, &["projectless-thread-ids", "projectlessThreadIds"]);
    summary.writable_roots_by_thread_id = extract_string_vec_map(
        &raw,
        &[
            "heartbeat-thread-permissions-by-id",
            "writableRootsByThreadId",
        ],
    );
    summary.workspace_roots = extract_workspace_roots(&raw);
    summary.pinned_thread_ids = extract_string_set(&raw, &["pinned-thread-ids", "pinnedThreadIds"]);
    summary.raw = Some(raw);

    summary
}

fn extract_string_map(value: &Value, keys: &[&str]) -> HashMap<String, String> {
    find_key(value, keys)
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_string_set(value: &Value, keys: &[&str]) -> HashSet<String> {
    find_key(value, keys)
        .and_then(Value::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn extract_string_vec_map(value: &Value, keys: &[&str]) -> HashMap<String, Vec<String>> {
    let mut result = HashMap::new();
    let Some(object) = find_key(value, keys).and_then(Value::as_object) else {
        return result;
    };

    for (thread_id, value) in object {
        let mut roots = Vec::new();
        collect_writable_roots(value, &mut roots);
        if !roots.is_empty() {
            roots.sort();
            roots.dedup();
            result.insert(thread_id.clone(), roots);
        }
    }
    result
}

fn extract_workspace_roots(value: &Value) -> Vec<String> {
    let mut roots = Vec::new();
    collect_workspace_roots(value, &mut roots);
    roots.sort();
    roots.dedup();
    roots
}

fn find_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key) {
                    return Some(found);
                }
            }
            for child in map.values() {
                if let Some(found) = find_key(child, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|item| find_key(item, keys)),
        _ => None,
    }
}

fn collect_writable_roots(value: &Value, roots: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if key == "writableRoots" || key == "writable_roots" || key == "roots" {
                    collect_string_array(value, roots);
                } else {
                    collect_writable_roots(value, roots);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_writable_roots(item, roots);
            }
        }
        _ => {}
    }
}

fn collect_workspace_roots(value: &Value, roots: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                let key_lower = key.to_ascii_lowercase();
                if key_lower.contains("workspace") && key_lower.contains("root") {
                    match value {
                        Value::String(text) => roots.push(text.to_string()),
                        Value::Array(_) => collect_string_array(value, roots),
                        Value::Object(_) => collect_workspace_roots(value, roots),
                        _ => {}
                    }
                } else {
                    collect_workspace_roots(value, roots);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_workspace_roots(item, roots);
            }
        }
        _ => {}
    }
}

fn collect_string_array(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(text.to_string()),
        Value::Array(items) => {
            for item in items {
                collect_string_array(item, out);
            }
        }
        Value::Object(map) => {
            for key in ["path", "root", "cwd"] {
                if let Some(text) = map.get(key).and_then(Value::as_str) {
                    out.push(text.to_string());
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::Snapshot;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn extracts_known_global_state_fields() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join(".codex-global-state.json"),
            r#"{
              "thread-workspace-root-hints":{"a":"/repo"},
              "projectless-thread-ids":["b"],
              "heartbeat-thread-permissions-by-id":{"a":{"writableRoots":["/repo"]}},
              "savedWorkspaceRoots":["/repo"]
            }"#,
        )
        .unwrap();
        let snapshot = Snapshot::create(temp.path()).unwrap();
        let summary = read_global_state(&snapshot);
        assert!(summary.parsed_ok);
        assert_eq!(summary.thread_workspace_root_hints["a"], "/repo");
        assert!(summary.projectless_thread_ids.contains("b"));
        assert_eq!(summary.writable_roots_by_thread_id["a"], vec!["/repo"]);
    }

    #[test]
    fn malformed_global_state_does_not_panic() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join(".codex-global-state.json"), "{bad").unwrap();
        let snapshot = Snapshot::create(temp.path()).unwrap();
        let summary = read_global_state(&snapshot);
        assert!(!summary.parsed_ok);
        assert!(!summary.errors.is_empty());
    }
}

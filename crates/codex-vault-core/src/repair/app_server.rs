use crate::models::{AppServerCapability, AppServerRunSummary, ThreadValidationResult};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

pub fn detect_app_server_capability() -> AppServerCapability {
    let mut warnings = Vec::new();
    let codex_path = Command::new("which")
        .arg("codex")
        .output()
        .ok()
        .and_then(|output| output.status.success().then_some(output.stdout))
        .map(|stdout| String::from_utf8_lossy(&stdout).trim().to_string())
        .filter(|path| !path.is_empty());

    let codex_version = Command::new("codex")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| output.status.success().then_some(output.stdout))
        .map(|stdout| String::from_utf8_lossy(&stdout).trim().to_string())
        .filter(|version| !version.is_empty())
        .or_else(|| {
            warnings.push("Could not read codex --version.".to_string());
            None
        });

    let help = Command::new("codex")
        .args(["app-server", "--help"])
        .output();
    let supports_app_server = help
        .as_ref()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if !supports_app_server {
        warnings.push("codex app-server --help did not succeed.".to_string());
    }
    let supports_generate_schema = help
        .ok()
        .map(|output| {
            let text = String::from_utf8_lossy(&output.stdout);
            text.contains("generate-json-schema") && text.contains("generate-ts")
        })
        .unwrap_or(false);

    AppServerCapability {
        codex_path,
        codex_version,
        supports_app_server,
        supports_generate_schema,
        warnings,
    }
}

pub fn run_official_scan_and_validate(
    codex_home: &Path,
    selected_thread_ids: &[String],
) -> Result<AppServerRunSummary> {
    let capability = detect_app_server_capability();
    if !capability.supports_app_server {
        return Err(anyhow!("Codex app-server is not available."));
    }

    let mut client = AppServerClient::start(codex_home)?;
    let mut warnings = capability.warnings.clone();
    let mut errors = Vec::new();
    let initialized = match client.initialize() {
        Ok(_) => true,
        Err(error) => {
            errors.push(error.to_string());
            false
        }
    };
    let active_threads = if initialized {
        match client.list_all_threads(false) {
            Ok(threads) => threads,
            Err(error) => {
                errors.push(format!("thread/list active failed: {error}"));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    let archived_threads = if initialized {
        match client.list_all_threads(true) {
            Ok(threads) => threads,
            Err(error) => {
                errors.push(format!("thread/list archived failed: {error}"));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let mut listed = BTreeMap::new();
    for id in extract_thread_ids(&active_threads) {
        listed.insert(id, false);
    }
    for id in extract_thread_ids(&archived_threads) {
        listed.insert(id, true);
    }

    let mut thread_validations = Vec::new();
    if initialized {
        for thread_id in selected_thread_ids {
            let read = client.read_thread(thread_id, false);
            let archived = listed.get(thread_id).copied();
            thread_validations.push(ThreadValidationResult {
                thread_id: thread_id.clone(),
                listed: archived.is_some(),
                archived,
                read_ok: read.is_ok(),
                error: read.err().map(|error| error.to_string()),
            });
        }
    }

    warnings.extend(client.take_warnings());
    Ok(AppServerRunSummary {
        capability,
        initialized,
        active_threads_seen: active_threads.len(),
        archived_threads_seen: archived_threads.len(),
        thread_validations,
        warnings,
        errors,
    })
}

pub fn run_single_official_operation(
    codex_home: &Path,
    method: &str,
    params: Value,
) -> Result<Value> {
    let mut client = AppServerClient::start(codex_home)?;
    client.initialize()?;
    client.request(method, params)
}

struct AppServerClient {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: i64,
    warnings: Vec<String>,
}

impl AppServerClient {
    fn start(codex_home: &Path) -> Result<Self> {
        let mut child = Command::new("codex")
            .args(["app-server", "--listen", "stdio://"])
            .env("CODEX_HOME", codex_home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| "spawn codex app-server --listen stdio://")?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to open app-server stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to open app-server stdout"))?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });

        Ok(Self {
            child,
            stdin,
            lines: rx,
            next_id: 1,
            warnings: Vec::new(),
        })
    }

    fn initialize(&mut self) -> Result<Value> {
        let result = self.request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "codex_recall",
                    "title": "Codex Recall",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )?;
        self.notify("initialized", json!({}))?;
        Ok(result)
    }

    fn list_all_threads(&mut self, archived: bool) -> Result<Vec<Value>> {
        let mut cursor: Option<String> = None;
        let mut out = Vec::new();
        for _ in 0..100 {
            let mut params = json!({
                "limit": 200,
                "sortKey": "updated_at",
                "sortDirection": "desc",
                "archived": archived,
                "useStateDbOnly": false
            });
            if let Some(value) = &cursor {
                params["cursor"] = Value::String(value.clone());
            }
            let result = self.request("thread/list", params)?;
            if let Some(items) = result.get("data").and_then(|value| value.as_array()) {
                out.extend(items.iter().cloned());
            }
            cursor = result
                .get("nextCursor")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            if cursor.is_none() {
                break;
            }
        }
        Ok(out)
    }

    fn read_thread(&mut self, thread_id: &str, include_turns: bool) -> Result<Value> {
        self.request(
            "thread/read",
            json!({
                "threadId": thread_id,
                "includeTurns": include_turns
            }),
        )
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({
            "id": id,
            "method": method,
            "params": params
        });
        writeln!(self.stdin, "{}", request)?;
        self.stdin.flush()?;

        loop {
            let line = self
                .lines
                .recv_timeout(REQUEST_TIMEOUT)
                .with_context(|| format!("timeout waiting for app-server response to {method}"))?;
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                self.warnings
                    .push(format!("non-json app-server output: {line}"));
                continue;
            };
            if message.get("id").and_then(|value| value.as_i64()) != Some(id) {
                if message.get("id").is_none() && message.get("method").is_none() {
                    self.warnings.push(line);
                }
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(anyhow!(
                    "{}",
                    error
                        .get("message")
                        .and_then(|value| value.as_str())
                        .unwrap_or("app-server request failed")
                ));
            }
            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let notification = json!({
            "method": method,
            "params": params
        });
        writeln!(self.stdin, "{}", notification)?;
        self.stdin.flush()?;
        Ok(())
    }

    fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
}

impl Drop for AppServerClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn extract_thread_ids(threads: &[Value]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for thread in threads {
        if let Some(id) = thread
            .get("id")
            .or_else(|| thread.get("threadId"))
            .and_then(|value| value.as_str())
        {
            ids.insert(id.to_string());
        }
    }
    ids
}

use crate::models::CodexHomeCandidate;
use crate::utils::path_to_string;
use glob::glob;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

pub fn discover_codex_homes() -> Vec<CodexHomeCandidate> {
    let mut paths = Vec::new();

    if let Ok(value) = env::var("CODEX_HOME") {
        if !value.trim().is_empty() {
            paths.push((PathBuf::from(value), Some("CODEX_HOME".to_string())));
        }
    }

    if let Some(home) = dirs::home_dir() {
        paths.push((home.join(".codex"), Some("home_dot_codex".to_string())));
    }

    #[cfg(windows)]
    {
        if let Ok(value) = env::var("USERPROFILE") {
            paths.push((
                PathBuf::from(value).join(".codex"),
                Some("USERPROFILE".to_string()),
            ));
        }
        if let Ok(value) = env::var("APPDATA") {
            paths.push((
                PathBuf::from(value).join("Codex"),
                Some("APPDATA".to_string()),
            ));
        }
        if let Ok(value) = env::var("LOCALAPPDATA") {
            let local = PathBuf::from(value);
            paths.push((local.join("Codex"), Some("LOCALAPPDATA".to_string())));
            let pattern = local
                .join("Packages")
                .join("OpenAI.Codex_*")
                .join("LocalCache")
                .join("*");
            if let Some(pattern) = pattern.to_str() {
                if let Ok(matches) = glob(pattern) {
                    for entry in matches.flatten() {
                        paths.push((entry, Some("windows_package_local_cache".to_string())));
                    }
                }
            }
        }
    }

    dedupe_score(paths)
}

pub fn validate_codex_home(path: &Path) -> CodexHomeCandidate {
    score_candidate(path.to_path_buf(), Some("manual".to_string()))
}

fn dedupe_score(paths: Vec<(PathBuf, Option<String>)>) -> Vec<CodexHomeCandidate> {
    let mut by_key: BTreeMap<String, (PathBuf, Option<String>)> = BTreeMap::new();
    for (path, hint) in paths {
        let key = path_to_string(&path)
            .trim_end_matches(std::path::MAIN_SEPARATOR)
            .to_string();
        by_key.entry(key).or_insert((path, hint));
    }

    let mut candidates = by_key
        .into_values()
        .map(|(path, hint)| score_candidate(path, hint))
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });
    candidates
}

fn score_candidate(path: PathBuf, platform_hint: Option<String>) -> CodexHomeCandidate {
    let mut confidence = 0.0f32;
    let mut evidence = Vec::new();

    let sessions = path.join("sessions");
    if sessions.is_dir() {
        confidence += 0.25;
        evidence.push("sessions_dir_exists".to_string());
    }

    let archived = path.join("archived_sessions");
    if archived.is_dir() {
        confidence += 0.15;
        evidence.push("archived_sessions_dir_exists".to_string());
    }

    if path.join("session_index.jsonl").is_file() {
        confidence += 0.20;
        evidence.push("session_index_exists".to_string());
    }

    if has_state_sqlite(&path) {
        confidence += 0.25;
        evidence.push("state_sqlite_exists".to_string());
    }

    if path.join(".codex-global-state.json").is_file() {
        confidence += 0.15;
        evidence.push("global_state_exists".to_string());
    }

    CodexHomeCandidate {
        path: path_to_string(&path),
        confidence: confidence.min(1.0),
        evidence,
        platform_hint,
        exists: path.exists(),
    }
}

fn has_state_sqlite(path: &Path) -> bool {
    let pattern = path.join("state_*.sqlite");
    pattern
        .to_str()
        .and_then(|pattern| glob(pattern).ok())
        .map(|mut matches| matches.any(|entry| entry.map(|path| path.is_file()).unwrap_or(false)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scores_candidate_by_evidence() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("sessions")).unwrap();
        fs::write(temp.path().join("session_index.jsonl"), "{}\n").unwrap();

        let candidate = validate_codex_home(temp.path());
        assert!(candidate.confidence >= 0.45);
        assert!(candidate
            .evidence
            .contains(&"sessions_dir_exists".to_string()));
        assert!(candidate
            .evidence
            .contains(&"session_index_exists".to_string()));
    }

    #[test]
    fn missing_candidate_is_valid_low_confidence_result() {
        let candidate = validate_codex_home(Path::new("/definitely/not/a/codex/home"));
        assert!(!candidate.exists);
        assert_eq!(candidate.confidence, 0.0);
    }
}

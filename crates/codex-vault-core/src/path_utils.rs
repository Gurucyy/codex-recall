use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathNormalizationResult {
    pub raw: String,
    pub comparison_key: String,
    pub aliases: Vec<String>,
    pub realpath: Option<String>,
    pub had_windows_extended_prefix: bool,
    pub realpath_differs: bool,
}

pub fn normalize_path_key(raw: &str) -> PathNormalizationResult {
    let raw_trimmed = raw.trim().to_string();
    let had_windows_extended_prefix = is_windows_extended_path(&raw_trimmed);
    let mut normalized = strip_windows_extended_prefix(&raw_trimmed);
    normalized = expand_tilde(&normalized);
    normalized = normalized.replace('\\', "/");
    normalized = collapse_trailing_slashes(&normalized);

    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        let drive = normalized[..1].to_ascii_lowercase();
        normalized = format!("{drive}{}", &normalized[1..]);
    }

    let realpath = maybe_realpath_alias(&raw_trimmed);
    let realpath_key = realpath.as_ref().map(|path| {
        let stripped = strip_windows_extended_prefix(path).replace('\\', "/");
        collapse_trailing_slashes(&stripped)
    });
    let comparison_key = realpath_key.clone().unwrap_or_else(|| normalized.clone());

    let mut aliases = vec![raw_trimmed.clone(), normalized.clone()];
    if let Some(realpath) = &realpath {
        aliases.push(realpath.clone());
    }
    aliases.sort();
    aliases.dedup();

    PathNormalizationResult {
        raw: raw_trimmed,
        comparison_key,
        aliases,
        realpath_differs: realpath
            .as_ref()
            .map(|realpath| collapse_trailing_slashes(&realpath.replace('\\', "/")) != normalized)
            .unwrap_or(false),
        realpath,
        had_windows_extended_prefix,
    }
}

pub fn is_windows_extended_path(raw: &str) -> bool {
    raw.starts_with("\\\\?\\") || raw.starts_with("//?/")
}

pub fn maybe_realpath_alias(raw: &str) -> Option<String> {
    let candidate = PathBuf::from(raw);
    let canonical = candidate.canonicalize().ok()?;
    let canonical = canonical.to_string_lossy().to_string();
    (collapse_trailing_slashes(&canonical.replace('\\', "/"))
        != collapse_trailing_slashes(&raw.replace('\\', "/")))
    .then_some(canonical)
}

fn strip_windows_extended_prefix(raw: &str) -> String {
    raw.strip_prefix("\\\\?\\")
        .or_else(|| raw.strip_prefix("//?/"))
        .unwrap_or(raw)
        .to_string()
}

fn expand_tilde(raw: &str) -> String {
    if raw == "~" {
        dirs::home_dir()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| raw.to_string())
    } else if let Some(rest) = raw.strip_prefix("~/") {
        dirs::home_dir()
            .map(|path| path.join(rest).to_string_lossy().to_string())
            .unwrap_or_else(|| raw.to_string())
    } else if let Some(rest) = raw.strip_prefix("~\\") {
        dirs::home_dir()
            .map(|path| path.join(rest).to_string_lossy().to_string())
            .unwrap_or_else(|| raw.to_string())
    } else {
        raw.to_string()
    }
}

fn collapse_trailing_slashes(raw: &str) -> String {
    if raw == "/" {
        return raw.to_string();
    }
    let mut value = raw.trim().to_string();
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    value
}

pub fn home_dir_string() -> Option<String> {
    dirs::home_dir().map(|path| path.to_string_lossy().to_string())
}

pub fn username_guess() -> Option<String> {
    env::var("USER")
        .ok()
        .or_else(|| env::var("USERNAME").ok())
        .filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_extended_paths_share_comparison_key() {
        let normal = normalize_path_key(r"C:\repo\");
        let extended = normalize_path_key(r"\\?\C:\repo");
        assert_eq!(normal.comparison_key, extended.comparison_key);
        assert!(extended.had_windows_extended_prefix);
    }

    #[test]
    fn removes_posix_trailing_slash() {
        let result = normalize_path_key("/tmp/repo/");
        assert_eq!(result.comparison_key, "/tmp/repo");
    }
}

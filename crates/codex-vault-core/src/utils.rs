use chrono::{DateTime, LocalResult, TimeZone, Utc};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn system_time_to_utc(value: SystemTime) -> DateTime<Utc> {
    DateTime::<Utc>::from(value)
}

pub fn file_mtime(path: &Path) -> Option<DateTime<Utc>> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(system_time_to_utc)
}

pub fn parse_datetime_value(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::String(text) => parse_datetime_str(text),
        Value::Number(number) => number
            .as_i64()
            .and_then(parse_epoch_number)
            .or_else(|| number.as_f64().and_then(parse_epoch_float)),
        _ => None,
    }
}

pub fn parse_datetime_str(value: &str) -> Option<DateTime<Utc>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    DateTime::parse_from_rfc3339(trimmed)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| trimmed.parse::<i64>().ok().and_then(parse_epoch_number))
}

pub fn parse_epoch_number(value: i64) -> Option<DateTime<Utc>> {
    if value <= 0 {
        return None;
    }

    let (secs, nanos) = if value > 10_000_000_000_000 {
        (value / 1_000_000_000, ((value % 1_000_000_000) as u32))
    } else if value > 10_000_000_000 {
        (value / 1_000, ((value % 1_000) as u32) * 1_000_000)
    } else {
        (value, 0)
    };

    match Utc.timestamp_opt(secs, nanos) {
        LocalResult::Single(dt) => Some(dt),
        _ => None,
    }
}

pub fn parse_epoch_float(value: f64) -> Option<DateTime<Utc>> {
    if value <= 0.0 {
        return None;
    }
    let secs = value.trunc() as i64;
    let nanos = ((value.fract()) * 1_000_000_000.0) as u32;
    match Utc.timestamp_opt(secs, nanos) {
        LocalResult::Single(dt) => Some(dt),
        _ => None,
    }
}

pub fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(as_non_empty_string) {
                    return Some(found);
                }
            }
            for child in map.values() {
                if let Some(found) = find_string(child, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|child| find_string(child, keys)),
        _ => None,
    }
}

pub fn find_datetime(value: &Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(parse_datetime_value) {
                    return Some(found);
                }
            }
            for child in map.values() {
                if let Some(found) = find_datetime(child, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|child| find_datetime(child, keys)),
        _ => None,
    }
}

pub fn as_non_empty_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

pub fn safe_snippet(text: &str, limit: usize) -> String {
    let sanitized = text.replace('\n', "\\n").replace('\r', "\\r");
    if sanitized.chars().count() <= limit {
        sanitized
    } else {
        let mut out = sanitized.chars().take(limit).collect::<String>();
        out.push_str("...");
        out
    }
}

pub fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .map(path_to_string)
        .unwrap_or_else(|_| path_to_string(path))
}

pub fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn filename_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
}

pub fn join_if_relative(base: &Path, maybe_relative: &str) -> PathBuf {
    let path = PathBuf::from(maybe_relative);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

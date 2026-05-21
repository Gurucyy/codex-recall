use crate::models::{SqliteDatabaseSummary, SqliteThread};
use crate::snapshot::Snapshot;
use crate::utils::{parse_datetime_value, path_to_string};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn read_sqlite_databases(snapshot: &Snapshot) -> Vec<SqliteDatabaseSummary> {
    read_sqlite_databases_with_progress(snapshot, |_completed, _total| {})
}

pub fn read_sqlite_databases_with_progress<F>(
    snapshot: &Snapshot,
    mut progress: F,
) -> Vec<SqliteDatabaseSummary>
where
    F: FnMut(usize, usize),
{
    let paths = snapshot.find_state_databases();
    let total = paths.len();
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let source_path = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| snapshot.source_path(name))
                .unwrap_or_else(|| path.to_path_buf());
            let summary = read_one_database(path, &source_path);
            progress(index + 1, total);
            summary
        })
        .collect()
}

fn read_one_database(snapshot_path: &Path, source_path: &Path) -> SqliteDatabaseSummary {
    let mut summary = SqliteDatabaseSummary {
        path: path_to_string(source_path),
        schema_fingerprint: None,
        integrity_ok: false,
        integrity_message: None,
        tables: Vec::new(),
        threads: Vec::new(),
        errors: Vec::new(),
    };

    let conn = match Connection::open_with_flags(snapshot_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => conn,
        Err(error) => {
            summary.integrity_message = Some(error.to_string());
            summary.errors.push(format!("open failed: {error}"));
            return summary;
        }
    };

    match conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0)) {
        Ok(message) => {
            summary.integrity_ok = message == "ok";
            summary.integrity_message = Some(message);
        }
        Err(error) => {
            summary.integrity_ok = false;
            summary.integrity_message = Some(error.to_string());
            summary
                .errors
                .push(format!("integrity check failed: {error}"));
        }
    }

    summary.tables = match read_tables(&conn) {
        Ok(tables) => tables,
        Err(error) => {
            summary
                .errors
                .push(format!("read sqlite_master failed: {error}"));
            Vec::new()
        }
    };

    if summary.tables.iter().any(|table| table == "threads") {
        match read_threads(&conn, &summary.path) {
            Ok((columns, threads)) => {
                summary.schema_fingerprint = Some(schema_fingerprint(&summary.tables, &columns));
                summary.threads = threads;
            }
            Err(error) => summary.errors.push(format!("read threads failed: {error}")),
        }
    }

    summary
}

fn read_tables(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

fn read_threads(
    conn: &Connection,
    database_path: &str,
) -> rusqlite::Result<(Vec<String>, Vec<SqliteThread>)> {
    let columns = read_thread_columns(conn)?;
    if !columns.iter().any(|column| column == "id") {
        return Ok((columns, Vec::new()));
    }

    let wanted = [
        "id",
        "title",
        "first_user_message",
        "cwd",
        "source",
        "archived",
        "archived_at",
        "created_at",
        "updated_at",
        "updated_at_ms",
        "rollout_path",
    ];
    let selected = wanted
        .iter()
        .filter(|column| columns.iter().any(|existing| existing == **column))
        .map(|column| column.to_string())
        .collect::<Vec<_>>();
    let select_list = selected
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT {select_list} FROM threads");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut threads = Vec::new();

    while let Some(row) = rows.next()? {
        let mut raw = Map::new();
        for (index, column) in selected.iter().enumerate() {
            let value_ref = row.get_ref(index)?;
            raw.insert(column.clone(), sql_value_to_json(value_ref));
        }

        let raw_value = Value::Object(raw.clone());
        let Some(id) = raw.get("id").and_then(value_to_string) else {
            continue;
        };

        let updated_at_ms = raw.get("updated_at_ms").and_then(value_to_i64);
        let mut updated_at = raw.get("updated_at").and_then(parse_datetime_value);
        if updated_at.is_none() {
            updated_at = updated_at_ms
                .map(|ms| Value::Number(ms.into()))
                .as_ref()
                .and_then(parse_datetime_value);
        }

        threads.push(SqliteThread {
            id,
            title: raw.get("title").and_then(value_to_string),
            first_user_message: raw.get("first_user_message").and_then(value_to_string),
            cwd: raw.get("cwd").and_then(value_to_string),
            source: raw.get("source").and_then(value_to_string),
            archived: raw.get("archived").and_then(value_to_bool),
            archived_at: raw.get("archived_at").and_then(parse_datetime_value),
            created_at: raw.get("created_at").and_then(parse_datetime_value),
            updated_at,
            updated_at_ms,
            rollout_path: raw.get("rollout_path").and_then(value_to_string),
            raw: raw_value,
            database_path: Some(database_path.to_string()),
        });
    }

    Ok((columns, threads))
}

fn read_thread_columns(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("PRAGMA table_info(threads)")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    rows.collect()
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn sql_value_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::Number(number.into()),
        ValueRef::Real(number) => serde_json::Number::from_f64(number)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(bytes) => Value::String(String::from_utf8_lossy(bytes).to_string()),
        ValueRef::Blob(bytes) => Value::String(format!("<blob {} bytes>", bytes.len())),
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.parse::<i64>().ok(),
        _ => None,
    }
}

fn value_to_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(number) => number.as_i64().map(|value| value != 0),
        Value::String(text) => match text.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn schema_fingerprint(tables: &[String], columns: &[String]) -> String {
    let mut hasher = Sha256::new();
    for table in tables {
        hasher.update(table.as_bytes());
        hasher.update([0]);
    }
    for column in columns {
        hasher.update(column.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::Snapshot;
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[test]
    fn reads_threads_with_missing_optional_columns() {
        let temp = tempdir().unwrap();
        let db = temp.path().join("state_5.sqlite");
        let conn = Connection::open(&db).unwrap();
        conn.execute("CREATE TABLE threads (id TEXT PRIMARY KEY, title TEXT)", [])
            .unwrap();
        conn.execute("INSERT INTO threads (id, title) VALUES ('a', 'Alpha')", [])
            .unwrap();
        drop(conn);

        let snapshot = Snapshot::create(temp.path()).unwrap();
        let summaries = read_sqlite_databases(&snapshot);
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].integrity_ok);
        assert_eq!(summaries[0].threads[0].id, "a");
    }

    #[test]
    fn corrupt_sqlite_returns_error_summary() {
        let temp = tempdir().unwrap();
        std::fs::write(temp.path().join("state_5.sqlite"), b"not sqlite").unwrap();
        let snapshot = Snapshot::create(temp.path()).unwrap();
        let summaries = read_sqlite_databases(&snapshot);
        assert_eq!(summaries.len(), 1);
        assert!(!summaries[0].integrity_ok);
        assert!(!summaries[0].errors.is_empty() || summaries[0].integrity_message.is_some());
    }
}

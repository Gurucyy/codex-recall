use crate::models::{
    GlobalStateSummary, IndexRecord, JsonlSession, LocalSession, SessionIndexSummary,
    SqliteDatabaseSummary, SqliteThread, VisibilityRisk, WorkspaceGroup,
};
use crate::path_utils::normalize_path_key;
use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub fn normalize_all(
    jsonl_sessions: &[JsonlSession],
    sqlite_databases: &[SqliteDatabaseSummary],
    session_index: &SessionIndexSummary,
    global_state: &GlobalStateSummary,
) -> (Vec<LocalSession>, SessionIndexSummary) {
    let mut ids = BTreeSet::new();
    let mut jsonl_by_id: HashMap<String, &JsonlSession> = HashMap::new();
    let mut sqlite_by_id: HashMap<String, &SqliteThread> = HashMap::new();

    for session in jsonl_sessions {
        let id = session_id_from_jsonl(session);
        ids.insert(id.clone());
        jsonl_by_id
            .entry(id)
            .and_modify(|existing| {
                if jsonl_updated_at(session) > jsonl_updated_at(*existing) {
                    *existing = session;
                }
            })
            .or_insert(session);
    }

    for database in sqlite_databases {
        for thread in &database.threads {
            ids.insert(thread.id.clone());
            sqlite_by_id
                .entry(thread.id.clone())
                .and_modify(|existing| {
                    if sqlite_updated_at(thread) > sqlite_updated_at(*existing) {
                        *existing = thread;
                    }
                })
                .or_insert(thread);
        }
    }

    for id in session_index.records_by_id.keys() {
        ids.insert(id.clone());
    }
    for id in global_state.thread_workspace_root_hints.keys() {
        ids.insert(id.clone());
    }
    for id in &global_state.projectless_thread_ids {
        ids.insert(id.clone());
    }
    for id in global_state.writable_roots_by_thread_id.keys() {
        ids.insert(id.clone());
    }

    let mut sessions = ids
        .into_iter()
        .map(|id| {
            let jsonl = jsonl_by_id.get(&id).copied();
            let sqlite = sqlite_by_id.get(&id).copied();
            let index = session_index.records_by_id.get(&id);
            normalize_one(id, jsonl, sqlite, index, global_state)
        })
        .collect::<Vec<_>>();

    assign_recent_rank(&mut sessions);

    let mut index = session_index.clone();
    index.orphan_candidates = sessions
        .iter()
        .filter(|session| {
            session.exists_in_index && !session.exists_in_jsonl && !session.exists_in_sqlite
        })
        .map(|session| session.id.clone())
        .collect();
    index.orphan_candidates.sort();

    sessions.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });

    (sessions, index)
}

pub fn build_workspace_groups(sessions: &[LocalSession]) -> Vec<WorkspaceGroup> {
    let mut groups = Vec::new();
    groups.push(special_group(
        "all",
        "All Sessions",
        sessions,
        |_: &LocalSession| true,
    ));
    groups.push(special_group("active", "Active", sessions, |s| !s.archived));
    groups.push(special_group("archived", "Archived", sessions, |s| {
        s.archived
    }));
    groups.push(special_group(
        "possibly-hidden",
        "Possibly Hidden",
        sessions,
        |s| s.visibility_risk != VisibilityRisk::Low || !s.diagnostics.is_empty(),
    ));
    groups.push(special_group("risk-high", "High Risk", sessions, |s| {
        s.visibility_risk == VisibilityRisk::High
    }));
    groups.push(special_group("risk-medium", "Medium Risk", sessions, |s| {
        s.visibility_risk == VisibilityRisk::Medium
    }));
    groups.push(special_group(
        "unknown-workspace",
        "Unknown Workspace",
        sessions,
        |s| s.workspace_key.is_none(),
    ));

    let mut workspace_map: BTreeMap<String, WorkspaceGroup> = BTreeMap::new();
    for session in sessions {
        if let Some(key) = &session.workspace_key {
            let group = workspace_map
                .entry(key.clone())
                .or_insert_with(|| WorkspaceGroup {
                    key: key.clone(),
                    display_path: session.cwd_raw.clone().unwrap_or_else(|| key.clone()),
                    aliases: Vec::new(),
                    session_ids: Vec::new(),
                    suspicious_session_ids: Vec::new(),
                    diagnostics: Vec::new(),
                });
            group.session_ids.push(session.id.clone());
            for alias in &session.path_aliases {
                group.aliases.push(alias.clone());
            }
            if session.visibility_risk != VisibilityRisk::Low || !session.diagnostics.is_empty() {
                group.suspicious_session_ids.push(session.id.clone());
            }
        }
    }

    for mut group in workspace_map.into_values() {
        group.aliases.sort();
        group.aliases.dedup();
        group.session_ids.sort();
        group.suspicious_session_ids.sort();
        groups.push(group);
    }

    groups
}

fn normalize_one(
    id: String,
    jsonl: Option<&JsonlSession>,
    sqlite: Option<&SqliteThread>,
    index: Option<&IndexRecord>,
    global_state: &GlobalStateSummary,
) -> LocalSession {
    let cwd_raw = jsonl
        .and_then(|session| session.meta.cwd.clone())
        .or_else(|| sqlite.and_then(|thread| thread.cwd.clone()))
        .or_else(|| global_state.thread_workspace_root_hints.get(&id).cloned())
        .or_else(|| {
            global_state
                .writable_roots_by_thread_id
                .get(&id)
                .and_then(|roots| roots.first().cloned())
        });
    let path_info = cwd_raw.as_ref().map(|cwd| normalize_path_key(cwd));
    let cwd_canonical = path_info.as_ref().map(|info| info.comparison_key.clone());
    let path_aliases = path_info
        .as_ref()
        .map(|info| info.aliases.clone())
        .unwrap_or_default();

    let mut title_sources = Vec::new();
    let title = first_plausible_title(
        &mut title_sources,
        index.and_then(|record| record.thread_name.as_deref()),
        sqlite.and_then(|thread| thread.title.as_deref()),
        sqlite.and_then(|thread| thread.first_user_message.as_deref()),
        jsonl.and_then(|session| session.title_candidate.as_deref()),
        &id,
    );

    let archived = jsonl
        .map(|session| session.archived_by_path)
        .or_else(|| sqlite.and_then(|thread| thread.archived))
        .unwrap_or(false);
    let mut archived_sources = Vec::new();
    if let Some(session) = jsonl {
        archived_sources.push(format!("jsonl_path:{}", session.archived_by_path));
    }
    if let Some(thread) = sqlite {
        if let Some(archived) = thread.archived {
            archived_sources.push(format!("sqlite:{archived}"));
        }
    }

    let exists_in_global_state = global_state.thread_workspace_root_hints.contains_key(&id)
        || global_state.projectless_thread_ids.contains(&id)
        || global_state.writable_roots_by_thread_id.contains_key(&id)
        || global_state.pinned_thread_ids.contains(&id);

    let global_state_evidence = exists_in_global_state.then(|| {
        json!({
            "workspace_hint": global_state.thread_workspace_root_hints.get(&id),
            "projectless_present": global_state.projectless_thread_ids.contains(&id),
            "writable_roots": global_state.writable_roots_by_thread_id.get(&id).cloned().unwrap_or_default(),
            "pinned": global_state.pinned_thread_ids.contains(&id)
        })
    });

    let exists_in_jsonl = jsonl.is_some();
    let exists_in_sqlite = sqlite.is_some();
    let exists_in_index = index.is_some();
    let evidence_count = [
        exists_in_jsonl,
        exists_in_sqlite,
        exists_in_index,
        exists_in_global_state,
    ]
    .into_iter()
    .filter(|value| *value)
    .count();

    LocalSession {
        id,
        title,
        title_sources,
        created_at: jsonl
            .and_then(|session| session.meta.created_at)
            .or_else(|| sqlite.and_then(|thread| thread.created_at)),
        updated_at: jsonl_updated_at_opt(jsonl)
            .or_else(|| sqlite.and_then(sqlite_updated_at))
            .or_else(|| index.and_then(|record| record.updated_at)),
        source: jsonl
            .and_then(|session| session.meta.source.clone())
            .or_else(|| sqlite.and_then(|thread| thread.source.clone())),
        cwd_raw,
        cwd_canonical: cwd_canonical.clone(),
        workspace_key: cwd_canonical,
        archived,
        archived_sources,
        rollout_path: jsonl
            .map(|session| session.rollout_path.clone())
            .or_else(|| sqlite.and_then(|thread| thread.rollout_path.clone())),
        exists_in_jsonl,
        exists_in_sqlite,
        exists_in_index,
        exists_in_global_state,
        messages: jsonl
            .map(|session| session.messages.clone())
            .unwrap_or_default(),
        first_user_message: jsonl
            .and_then(|session| session.first_user_message.clone())
            .or_else(|| sqlite.and_then(|thread| thread.first_user_message.clone())),
        sqlite_evidence: sqlite.cloned(),
        index_evidence: index.cloned(),
        global_state_evidence,
        diagnostics: Vec::new(),
        visibility_risk: VisibilityRisk::Low,
        confidence: (evidence_count as f32 / 4.0).max(0.25),
        recent_rank: None,
        jsonl_malformed_lines: jsonl.map(|session| session.malformed_lines).unwrap_or(0),
        jsonl_raw_events_count: jsonl.map(|session| session.raw_events_count).unwrap_or(0),
        jsonl_parse_errors: jsonl
            .map(|session| session.parse_errors.clone())
            .unwrap_or_default(),
        jsonl_archived_by_path: jsonl.map(|session| session.archived_by_path),
        path_aliases,
    }
}

fn first_plausible_title(
    sources: &mut Vec<String>,
    index_title: Option<&str>,
    sqlite_title: Option<&str>,
    sqlite_first_user: Option<&str>,
    jsonl_title: Option<&str>,
    fallback_id: &str,
) -> String {
    for (source, value) in [
        ("index", index_title),
        ("sqlite_title", sqlite_title),
        ("sqlite_first_user", sqlite_first_user),
        ("jsonl_first_user", jsonl_title),
    ] {
        if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
            sources.push(format!("{source}:{value}"));
            return truncate_title(value);
        }
    }
    sources.push("thread_id".to_string());
    fallback_id.to_string()
}

fn truncate_title(value: &str) -> String {
    if value.chars().count() <= 90 {
        value.to_string()
    } else {
        let mut out = value.chars().take(87).collect::<String>();
        out.push_str("...");
        out
    }
}

fn session_id_from_jsonl(session: &JsonlSession) -> String {
    session
        .id
        .clone()
        .or_else(|| session.inferred_id.clone())
        .unwrap_or_else(|| session.rollout_path.clone())
}

fn jsonl_updated_at(session: &JsonlSession) -> Option<DateTime<Utc>> {
    session.meta.updated_at.or(session.file_mtime)
}

fn jsonl_updated_at_opt(session: Option<&JsonlSession>) -> Option<DateTime<Utc>> {
    session.and_then(jsonl_updated_at)
}

fn sqlite_updated_at(thread: &SqliteThread) -> Option<DateTime<Utc>> {
    thread.updated_at.or(thread.created_at)
}

fn assign_recent_rank(sessions: &mut [LocalSession]) {
    let mut active = sessions
        .iter()
        .enumerate()
        .filter(|(_, session)| !session.archived)
        .map(|(index, session)| (index, session.updated_at))
        .collect::<Vec<_>>();
    active.sort_by(|a, b| b.1.cmp(&a.1));

    for (rank, (session_index, _)) in active.into_iter().enumerate() {
        sessions[session_index].recent_rank = Some(rank + 1);
    }
}

fn special_group(
    key: &str,
    display_path: &str,
    sessions: &[LocalSession],
    predicate: impl Fn(&LocalSession) -> bool,
) -> WorkspaceGroup {
    let mut session_ids = sessions
        .iter()
        .filter(|session| predicate(session))
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    session_ids.sort();
    let suspicious_session_ids = sessions
        .iter()
        .filter(|session| predicate(session))
        .filter(|session| {
            session.visibility_risk != VisibilityRisk::Low || !session.diagnostics.is_empty()
        })
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();

    WorkspaceGroup {
        key: key.to_string(),
        display_path: display_path.to_string(),
        aliases: Vec::new(),
        session_ids,
        suspicious_session_ids,
        diagnostics: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::JsonlSessionMeta;

    #[test]
    fn title_priority_prefers_index() {
        let jsonl = JsonlSession {
            id: Some("a".to_string()),
            inferred_id: None,
            rollout_path: "/tmp/a.jsonl".to_string(),
            relative_path: "sessions/a.jsonl".to_string(),
            archived_by_path: false,
            file_size: 0,
            file_mtime: None,
            meta: JsonlSessionMeta {
                cwd: Some("/repo".to_string()),
                ..JsonlSessionMeta::default()
            },
            messages: Vec::new(),
            raw_events_count: 0,
            malformed_lines: 0,
            parse_errors: Vec::new(),
            first_user_message: Some("Jsonl title".to_string()),
            title_candidate: Some("Jsonl title".to_string()),
        };
        let mut index = SessionIndexSummary::default();
        index.records_by_id.insert(
            "a".to_string(),
            IndexRecord {
                id: "a".to_string(),
                thread_name: Some("Index title".to_string()),
                updated_at: None,
                raw: json!({}),
                line_no: 1,
            },
        );
        let (sessions, _) = normalize_all(&[jsonl], &[], &index, &GlobalStateSummary::default());
        assert_eq!(sessions[0].title, "Index title");
        assert_eq!(sessions[0].workspace_key.as_deref(), Some("/repo"));
    }
}

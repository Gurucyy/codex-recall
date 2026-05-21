use crate::models::{
    LocalSession, LocalSessionSummary, PagedSessions, SearchQuery, SearchResult, SessionQuery,
    VisibilityRisk,
};
use std::collections::BTreeSet;

pub fn search_sessions(sessions: &[LocalSession], query: &SearchQuery) -> Vec<SearchResult> {
    let parsed = ParsedSearch::parse(&query.text);
    let mut results = sessions
        .iter()
        .filter_map(|session| score_session(session, &parsed))
        .collect::<Vec<_>>();
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
            .then_with(|| a.title.cmp(&b.title))
    });
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);
    results.into_iter().skip(offset).take(limit).collect()
}

pub fn list_sessions(sessions: &[LocalSession], query: &SessionQuery) -> PagedSessions {
    let mut filtered = sessions
        .iter()
        .filter(|session| {
            query
                .archived
                .map(|value| session.archived == value)
                .unwrap_or(true)
        })
        .filter(|session| {
            query
                .workspace
                .as_ref()
                .map(|workspace| {
                    workspace == "all"
                        || session.workspace_key.as_deref() == Some(workspace.as_str())
                        || (workspace == "active" && !session.archived)
                        || (workspace == "archived" && session.archived)
                        || (workspace == "risk-high"
                            && session.visibility_risk == VisibilityRisk::High)
                        || (workspace == "risk-medium"
                            && session.visibility_risk == VisibilityRisk::Medium)
                        || (workspace == "unknown-workspace" && session.workspace_key.is_none())
                        || (workspace == "possibly-hidden"
                            && (session.visibility_risk != VisibilityRisk::Low
                                || !session.diagnostics.is_empty()))
                })
                .unwrap_or(true)
        })
        .filter(|session| {
            query
                .risk
                .as_ref()
                .map(|risk| &session.visibility_risk == risk)
                .unwrap_or(true)
        })
        .filter(|session| {
            query
                .diagnostic
                .as_ref()
                .map(|code| {
                    session
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code.eq_ignore_ascii_case(code))
                })
                .unwrap_or(true)
        })
        .filter(|session| {
            query
                .q
                .as_ref()
                .map(|q| {
                    let parsed = ParsedSearch::parse(q);
                    score_session(session, &parsed).is_some()
                })
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    let sort_desc = query.sort_desc.unwrap_or(true);
    match query.sort_by.as_deref().unwrap_or("updated_at") {
        "title" => filtered.sort_by(|a, b| compare(sort_desc, a.title.cmp(&b.title))),
        "risk" => filtered.sort_by(|a, b| {
            compare(
                sort_desc,
                risk_rank(&a.visibility_risk).cmp(&risk_rank(&b.visibility_risk)),
            )
        }),
        _ => filtered.sort_by(|a, b| compare(sort_desc, a.updated_at.cmp(&b.updated_at))),
    }

    let total = filtered.len();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);
    let items = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(LocalSessionSummary::from)
        .collect();

    PagedSessions {
        items,
        total,
        limit,
        offset,
    }
}

fn score_session(session: &LocalSession, parsed: &ParsedSearch) -> Option<SearchResult> {
    if !parsed.matches_filters(session) {
        return None;
    }

    let haystack = searchable_text(session);
    if !parsed
        .keywords
        .iter()
        .all(|keyword| haystack.contains(&keyword.to_ascii_lowercase()))
    {
        return None;
    }

    let mut score = 0.0;
    let mut matched_fields = BTreeSet::new();
    let text = parsed.original.to_ascii_lowercase();

    if !text.is_empty() && session.id.to_ascii_lowercase() == text {
        score += 100.0;
        matched_fields.insert("thread_id".to_string());
    }
    if !text.is_empty() && session.title.to_ascii_lowercase() == text {
        score += 50.0;
        matched_fields.insert("title".to_string());
    } else if parsed
        .keywords
        .iter()
        .any(|keyword| session.title.to_ascii_lowercase().contains(keyword))
    {
        score += 25.0;
        matched_fields.insert("title".to_string());
    }
    if parsed.keywords.iter().any(|keyword| {
        session
            .first_user_message
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains(keyword)
    }) {
        score += 15.0;
        matched_fields.insert("first_user_message".to_string());
    }
    if parsed.keywords.iter().any(|keyword| {
        session
            .messages
            .iter()
            .any(|message| message.content.to_ascii_lowercase().contains(keyword))
    }) {
        score += 10.0;
        matched_fields.insert("message_content".to_string());
    }
    if parsed.keywords.iter().any(|keyword| {
        session
            .cwd_raw
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains(keyword)
            || session
                .cwd_canonical
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains(keyword)
    }) {
        score += 8.0;
        matched_fields.insert("path".to_string());
    }
    if parsed
        .diagnostic
        .as_ref()
        .map(|code| {
            session
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.eq_ignore_ascii_case(code))
        })
        .unwrap_or(false)
    {
        score += 8.0;
        matched_fields.insert("diagnostic".to_string());
    }
    if parsed
        .risk
        .as_ref()
        .map(|risk| risk == &session.visibility_risk)
        .unwrap_or(false)
    {
        score += 5.0;
        matched_fields.insert("risk".to_string());
    }

    if score == 0.0 {
        score = parsed.keywords.len() as f32;
    }

    Some(SearchResult {
        session_id: session.id.clone(),
        title: session.title.clone(),
        snippet: make_snippet(session, &parsed.keywords),
        matched_fields: matched_fields.into_iter().collect(),
        score,
        diagnostics: session
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect(),
        updated_at: session.updated_at,
    })
}

fn searchable_text(session: &LocalSession) -> String {
    let mut parts = vec![
        session.id.clone(),
        session.title.clone(),
        format!("{:?}", session.visibility_risk),
        session.archived.to_string(),
    ];
    parts.extend(session.cwd_raw.clone());
    parts.extend(session.cwd_canonical.clone());
    parts.extend(session.source.clone());
    parts.extend(session.first_user_message.clone());
    parts.extend(
        session
            .messages
            .iter()
            .map(|message| message.content.clone()),
    );
    parts.extend(
        session
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.clone()),
    );
    parts.join("\n").to_ascii_lowercase()
}

fn make_snippet(session: &LocalSession, keywords: &[String]) -> Option<String> {
    let fields = session
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .chain(session.first_user_message.as_deref())
        .chain(std::iter::once(session.title.as_str()));

    for field in fields {
        let lower = field.to_ascii_lowercase();
        if keywords.is_empty() || keywords.iter().any(|keyword| lower.contains(keyword)) {
            let snippet = field
                .split_whitespace()
                .take(40)
                .collect::<Vec<_>>()
                .join(" ");
            return Some(snippet);
        }
    }
    None
}

#[derive(Debug, Default)]
struct ParsedSearch {
    original: String,
    keywords: Vec<String>,
    workspace: Option<String>,
    path: Option<String>,
    id: Option<String>,
    source: Option<String>,
    archived: Option<bool>,
    diagnostic: Option<String>,
    risk: Option<VisibilityRisk>,
}

impl ParsedSearch {
    fn parse(input: &str) -> Self {
        let mut parsed = Self {
            original: input.trim().to_string(),
            ..Self::default()
        };

        for token in input.split_whitespace() {
            if let Some((key, value)) = token.split_once(':') {
                match key.to_ascii_lowercase().as_str() {
                    "workspace" => parsed.workspace = Some(value.to_string()),
                    "path" => parsed.path = Some(value.to_ascii_lowercase()),
                    "id" => parsed.id = Some(value.to_ascii_lowercase()),
                    "source" => parsed.source = Some(value.to_ascii_lowercase()),
                    "archived" => parsed.archived = parse_bool(value),
                    "diagnostic" => parsed.diagnostic = Some(value.to_string()),
                    "risk" => parsed.risk = parse_risk(value),
                    _ => parsed.keywords.push(token.to_ascii_lowercase()),
                }
            } else {
                parsed.keywords.push(token.to_ascii_lowercase());
            }
        }

        parsed
    }

    fn matches_filters(&self, session: &LocalSession) -> bool {
        if let Some(workspace) = &self.workspace {
            if session.workspace_key.as_deref() != Some(workspace.as_str()) {
                return false;
            }
        }
        if let Some(path) = &self.path {
            let raw = session
                .cwd_raw
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase();
            let canonical = session
                .cwd_canonical
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !raw.contains(path) && !canonical.contains(path) {
                return false;
            }
        }
        if let Some(id) = &self.id {
            if !session.id.to_ascii_lowercase().contains(id) {
                return false;
            }
        }
        if let Some(source) = &self.source {
            if !session
                .source
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains(source)
            {
                return false;
            }
        }
        if let Some(archived) = self.archived {
            if session.archived != archived {
                return false;
            }
        }
        if let Some(diagnostic) = &self.diagnostic {
            if !session
                .diagnostics
                .iter()
                .any(|item| item.code.eq_ignore_ascii_case(diagnostic))
            {
                return false;
            }
        }
        if let Some(risk) = &self.risk {
            if &session.visibility_risk != risk {
                return false;
            }
        }
        true
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

fn parse_risk(value: &str) -> Option<VisibilityRisk> {
    match value.to_ascii_lowercase().as_str() {
        "high" => Some(VisibilityRisk::High),
        "medium" => Some(VisibilityRisk::Medium),
        "low" => Some(VisibilityRisk::Low),
        _ => None,
    }
}

fn risk_rank(risk: &VisibilityRisk) -> usize {
    match risk {
        VisibilityRisk::High => 3,
        VisibilityRisk::Medium => 2,
        VisibilityRisk::Low => 1,
    }
}

fn compare(desc: bool, ordering: std::cmp::Ordering) -> std::cmp::Ordering {
    if desc {
        ordering.reverse()
    } else {
        ordering
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, MessageRole};

    fn sample() -> LocalSession {
        LocalSession {
            id: "thread-a".to_string(),
            title: "Recover hidden session".to_string(),
            title_sources: vec![],
            created_at: None,
            updated_at: None,
            source: Some("desktop".to_string()),
            cwd_raw: Some("/repo/app".to_string()),
            cwd_canonical: Some("/repo/app".to_string()),
            workspace_key: Some("/repo/app".to_string()),
            archived: false,
            archived_sources: vec![],
            rollout_path: None,
            exists_in_jsonl: true,
            exists_in_sqlite: true,
            exists_in_index: true,
            exists_in_global_state: false,
            messages: vec![Message {
                role: MessageRole::User,
                content: "Find the export panel".to_string(),
                created_at: None,
                raw_type: None,
                raw: None,
            }],
            first_user_message: Some("Find the export panel".to_string()),
            sqlite_evidence: None,
            index_evidence: None,
            global_state_evidence: None,
            diagnostics: vec![],
            visibility_risk: VisibilityRisk::Low,
            confidence: 1.0,
            recent_rank: Some(1),
            jsonl_malformed_lines: 0,
            jsonl_raw_events_count: 1,
            jsonl_parse_errors: vec![],
            jsonl_archived_by_path: Some(false),
            path_aliases: vec![],
        }
    }

    #[test]
    fn searches_content_and_filters_path() {
        let sessions = vec![sample()];
        let results = search_sessions(
            &sessions,
            &SearchQuery {
                text: "export path:/repo".to_string(),
                limit: None,
                offset: None,
            },
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "thread-a");
    }
}

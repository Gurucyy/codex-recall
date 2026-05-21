use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodexHomeCandidate {
    pub path: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub platform_hint: Option<String>,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParseError {
    pub path: Option<String>,
    pub line_no: Option<usize>,
    pub message: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub created_at: Option<DateTime<Utc>>,
    pub raw_type: Option<String>,
    pub raw: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct JsonlSessionMeta {
    pub cwd: Option<String>,
    pub source: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub model: Option<String>,
    pub raw: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonlSession {
    pub id: Option<String>,
    pub inferred_id: Option<String>,
    pub rollout_path: String,
    pub relative_path: String,
    pub archived_by_path: bool,
    pub file_size: u64,
    pub file_mtime: Option<DateTime<Utc>>,
    pub meta: JsonlSessionMeta,
    pub messages: Vec<Message>,
    pub raw_events_count: usize,
    pub malformed_lines: usize,
    pub parse_errors: Vec<ParseError>,
    pub first_user_message: Option<String>,
    pub title_candidate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexRecord {
    pub id: String,
    pub thread_name: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub raw: Value,
    pub line_no: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SessionIndexSummary {
    pub path: Option<String>,
    pub records_by_id: HashMap<String, IndexRecord>,
    pub all_records_by_id: HashMap<String, Vec<IndexRecord>>,
    pub duplicate_ids: Vec<String>,
    pub malformed_lines: Vec<ParseError>,
    pub orphan_candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SqliteThread {
    pub id: String,
    pub title: Option<String>,
    pub first_user_message: Option<String>,
    pub cwd: Option<String>,
    pub source: Option<String>,
    pub archived: Option<bool>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub updated_at_ms: Option<i64>,
    pub rollout_path: Option<String>,
    pub raw: Value,
    pub database_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SqliteDatabaseSummary {
    pub path: String,
    pub schema_fingerprint: Option<String>,
    pub integrity_ok: bool,
    pub integrity_message: Option<String>,
    pub tables: Vec<String>,
    pub threads: Vec<SqliteThread>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GlobalStateSummary {
    pub path: Option<String>,
    pub parsed_ok: bool,
    pub thread_workspace_root_hints: HashMap<String, String>,
    pub projectless_thread_ids: HashSet<String>,
    pub writable_roots_by_thread_id: HashMap<String, Vec<String>>,
    pub workspace_roots: Vec<String>,
    pub pinned_thread_ids: HashSet<String>,
    pub raw: Option<Value>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub evidence: Value,
    pub confidence: f32,
    pub suggested_action: Option<String>,
    pub affected_files: Vec<String>,
    pub safe_to_auto_fix: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityRisk {
    Low,
    Medium,
    High,
}

impl Default for VisibilityRisk {
    fn default() -> Self {
        Self::Low
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalSession {
    pub id: String,
    pub title: String,
    pub title_sources: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub cwd_raw: Option<String>,
    pub cwd_canonical: Option<String>,
    pub workspace_key: Option<String>,
    pub archived: bool,
    pub archived_sources: Vec<String>,
    pub rollout_path: Option<String>,
    pub exists_in_jsonl: bool,
    pub exists_in_sqlite: bool,
    pub exists_in_index: bool,
    pub exists_in_global_state: bool,
    pub messages: Vec<Message>,
    pub first_user_message: Option<String>,
    pub sqlite_evidence: Option<SqliteThread>,
    pub index_evidence: Option<IndexRecord>,
    pub global_state_evidence: Option<Value>,
    pub diagnostics: Vec<Diagnostic>,
    pub visibility_risk: VisibilityRisk,
    pub confidence: f32,
    pub recent_rank: Option<usize>,
    pub jsonl_malformed_lines: usize,
    pub jsonl_raw_events_count: usize,
    pub jsonl_parse_errors: Vec<ParseError>,
    pub jsonl_archived_by_path: Option<bool>,
    pub path_aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceGroup {
    pub key: String,
    pub display_path: String,
    pub aliases: Vec<String>,
    pub session_ids: Vec<String>,
    pub suspicious_session_ids: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ScanOptions {
    pub redact: bool,
    pub include_raw: bool,
    pub max_message_preview_chars: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScanResult {
    pub scan_id: String,
    pub codex_home: String,
    pub created_at: DateTime<Utc>,
    pub sessions: Vec<LocalSession>,
    pub workspaces: Vec<WorkspaceGroup>,
    pub sqlite_databases: Vec<SqliteDatabaseSummary>,
    pub session_index: Option<SessionIndexSummary>,
    pub global_state: Option<GlobalStateSummary>,
    pub diagnostics: Vec<Diagnostic>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub summary: ScanSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScanSummary {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub archived_sessions: usize,
    pub high_risk_sessions: usize,
    pub medium_risk_sessions: usize,
    pub jsonl_sessions: usize,
    pub sqlite_threads: usize,
    pub index_records: usize,
    pub malformed_jsonl_files: usize,
    pub corrupt_sqlite_databases: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SessionQuery {
    pub q: Option<String>,
    pub workspace: Option<String>,
    pub archived: Option<bool>,
    pub risk: Option<VisibilityRisk>,
    pub diagnostic: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PagedSessions {
    pub items: Vec<LocalSessionSummary>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalSessionSummary {
    pub id: String,
    pub title: String,
    pub updated_at: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub cwd_raw: Option<String>,
    pub workspace_key: Option<String>,
    pub archived: bool,
    pub exists_in_jsonl: bool,
    pub exists_in_sqlite: bool,
    pub exists_in_index: bool,
    pub exists_in_global_state: bool,
    pub visibility_risk: VisibilityRisk,
    pub diagnostic_codes: Vec<String>,
    pub recent_rank: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub session_id: String,
    pub title: String,
    pub snippet: Option<String>,
    pub matched_fields: Vec<String>,
    pub score: f32,
    pub diagnostics: Vec<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Markdown,
    Json,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportRequest {
    pub scan_id: String,
    pub session_ids: Vec<String>,
    pub format: ExportFormat,
    pub output_path: String,
    pub include_raw: bool,
    pub redact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportResult {
    pub output_paths: Vec<String>,
    pub sessions_exported: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupRequest {
    pub codex_home: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupManifestFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub mtime: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupManifest {
    pub source_codex_home: String,
    pub created_at: DateTime<Utc>,
    pub files: Vec<BackupManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupResult {
    pub output_path: String,
    pub manifest: BackupManifest,
    pub total_files: usize,
    pub total_bytes: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RepairRisk {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for RepairRisk {
    fn default() -> Self {
        Self::Low
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepairOperationKind {
    OfficialAppServerScanAndRepair,
    OfficialThreadNameSet,
    OfficialThreadUnarchive,
    OfficialThreadForkCopy,
    PatchWorkspaceHints,
    RebuildSessionIndex,
    BackfillThreadSource,
    PatchSqliteThreadRow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepairOperation {
    pub id: String,
    pub kind: RepairOperationKind,
    pub title: String,
    pub description: String,
    pub risk: RepairRisk,
    pub official_api: bool,
    pub experimental: bool,
    pub affected_session_ids: Vec<String>,
    pub affected_files: Vec<String>,
    pub evidence: Value,
    pub payload: Value,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepairSummary {
    pub affected_sessions: usize,
    pub operations_total: usize,
    pub official_operations: usize,
    pub local_patch_operations: usize,
    pub critical_operations: usize,
    pub highest_risk: RepairRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Precondition {
    pub code: String,
    pub label: String,
    pub required: bool,
    pub satisfied: bool,
    pub evidence: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepairPlanOptions {
    pub include_official_repair: bool,
    pub include_workspace_hint_patch: bool,
    pub include_jsonl_migration: bool,
    pub include_experimental_index_rebuild: bool,
}

impl Default for RepairPlanOptions {
    fn default() -> Self {
        Self {
            include_official_repair: true,
            include_workspace_hint_patch: true,
            include_jsonl_migration: true,
            include_experimental_index_rebuild: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerateRepairPlanRequest {
    pub scan_id: String,
    pub session_ids: Vec<String>,
    pub options: RepairPlanOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepairPlan {
    pub id: String,
    pub scan_id: String,
    pub codex_home: String,
    pub created_at: DateTime<Utc>,
    pub summary: RepairSummary,
    pub session_ids: Vec<String>,
    pub operations: Vec<RepairOperation>,
    pub preconditions: Vec<Precondition>,
    pub required_backups: Vec<String>,
    pub risk_level: RepairRisk,
    pub dry_run_required: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppServerCapability {
    pub codex_path: Option<String>,
    pub codex_version: Option<String>,
    pub supports_app_server: bool,
    pub supports_generate_schema: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreadValidationResult {
    pub thread_id: String,
    pub listed: bool,
    pub archived: Option<bool>,
    pub read_ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppServerRunSummary {
    pub capability: AppServerCapability,
    pub initialized: bool,
    pub active_threads_seen: usize,
    pub archived_threads_seen: usize,
    pub thread_validations: Vec<ThreadValidationResult>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepairScanDiff {
    pub before_total_sessions: usize,
    pub after_total_sessions: usize,
    pub before_high_risk_sessions: usize,
    pub after_high_risk_sessions: usize,
    pub selected_session_changes: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DryRunResult {
    pub plan_id: String,
    pub temp_codex_home: String,
    pub temp_removed_after_run: bool,
    pub app_server: Option<AppServerRunSummary>,
    pub diff: Option<RepairScanDiff>,
    pub succeeded: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplyRepairRequest {
    pub plan: RepairPlan,
    pub operation_ids: Vec<String>,
    pub backup_output_path: String,
    pub rollback_output_dir: String,
    pub confirm_codex_closed: bool,
    pub allow_process_warnings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollbackFile {
    pub original_path: String,
    pub backup_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollbackPackage {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub root_path: String,
    pub manifest_path: String,
    pub full_backup_path: Option<String>,
    pub files: Vec<RollbackFile>,
    pub auto_restore_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationResult {
    pub repair_id: String,
    pub scan_summary: ScanSummary,
    pub app_server: Option<AppServerRunSummary>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplyResult {
    pub repair_id: String,
    pub plan_id: String,
    pub applied_operation_ids: Vec<String>,
    pub backup: Option<BackupResult>,
    pub rollback: Option<RollbackPackage>,
    pub verification: Option<VerificationResult>,
    pub succeeded: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollbackRequest {
    pub package: RollbackPackage,
    pub confirm_restore: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollbackResult {
    pub rollback_id: String,
    pub restored_files: Vec<String>,
    pub succeeded: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    Markdown,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportRequest {
    pub scan_id: String,
    pub format: ReportFormat,
    pub output_path: String,
    pub redact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportResult {
    pub output_path: String,
    pub diagnostics_count: usize,
    pub high_risk_sessions: usize,
    pub medium_risk_sessions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub last_codex_home: Option<String>,
    pub redact_reports_by_default: bool,
    pub max_message_preview_chars: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_codex_home: None,
            redact_reports_by_default: true,
            max_message_preview_chars: 160,
        }
    }
}

impl From<&LocalSession> for LocalSessionSummary {
    fn from(session: &LocalSession) -> Self {
        Self {
            id: session.id.clone(),
            title: session.title.clone(),
            updated_at: session.updated_at,
            source: session.source.clone(),
            cwd_raw: session.cwd_raw.clone(),
            workspace_key: session.workspace_key.clone(),
            archived: session.archived,
            exists_in_jsonl: session.exists_in_jsonl,
            exists_in_sqlite: session.exists_in_sqlite,
            exists_in_index: session.exists_in_index,
            exists_in_global_state: session.exists_in_global_state,
            visibility_risk: session.visibility_risk.clone(),
            diagnostic_codes: session
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.clone())
                .collect(),
            recent_rank: session.recent_rank,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_session_summary_preserves_list_fields() {
        let session = LocalSession {
            id: "thread-a".to_string(),
            title: "A".to_string(),
            title_sources: vec!["index".to_string()],
            created_at: None,
            updated_at: None,
            source: Some("codex".to_string()),
            cwd_raw: Some("/repo".to_string()),
            cwd_canonical: Some("/repo".to_string()),
            workspace_key: Some("/repo".to_string()),
            archived: false,
            archived_sources: vec![],
            rollout_path: None,
            exists_in_jsonl: true,
            exists_in_sqlite: false,
            exists_in_index: true,
            exists_in_global_state: false,
            messages: vec![],
            first_user_message: None,
            sqlite_evidence: None,
            index_evidence: None,
            global_state_evidence: None,
            diagnostics: vec![],
            visibility_risk: VisibilityRisk::Low,
            confidence: 0.5,
            recent_rank: Some(1),
            jsonl_malformed_lines: 0,
            jsonl_raw_events_count: 0,
            jsonl_parse_errors: vec![],
            jsonl_archived_by_path: Some(false),
            path_aliases: vec![],
        };

        let summary = LocalSessionSummary::from(&session);
        assert_eq!(summary.id, "thread-a");
        assert_eq!(summary.workspace_key.as_deref(), Some("/repo"));
    }
}

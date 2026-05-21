export type DiagnosticSeverity = "info" | "warning" | "critical";
export type VisibilityRisk = "low" | "medium" | "high";
export type MessageRole = "user" | "assistant" | "system" | "tool" | "unknown";
export type ExportFormat = "markdown" | "json" | "html";
export type ReportFormat = "markdown" | "json";
export type RepairRisk = "low" | "medium" | "high" | "critical";
export type RepairOperationKind =
  | "official_app_server_scan_and_repair"
  | "official_thread_name_set"
  | "official_thread_unarchive"
  | "official_thread_fork_copy"
  | "patch_workspace_hints"
  | "rebuild_session_index"
  | "backfill_thread_source"
  | "patch_sqlite_thread_row";

export interface CodexHomeCandidate {
  path: string;
  confidence: number;
  evidence: string[];
  platform_hint?: string | null;
  exists: boolean;
}

export interface ParseError {
  path?: string | null;
  line_no?: number | null;
  message: string;
  snippet?: string | null;
}

export interface Message {
  role: MessageRole;
  content: string;
  created_at?: string | null;
  raw_type?: string | null;
  raw?: unknown;
}

export interface Diagnostic {
  code: string;
  severity: DiagnosticSeverity;
  message: string;
  evidence: unknown;
  confidence: number;
  suggested_action?: string | null;
  affected_files: string[];
  safe_to_auto_fix: boolean;
}

export interface LocalSession {
  id: string;
  title: string;
  title_sources: string[];
  created_at?: string | null;
  updated_at?: string | null;
  source?: string | null;
  cwd_raw?: string | null;
  cwd_canonical?: string | null;
  workspace_key?: string | null;
  archived: boolean;
  archived_sources: string[];
  rollout_path?: string | null;
  exists_in_jsonl: boolean;
  exists_in_sqlite: boolean;
  exists_in_index: boolean;
  exists_in_global_state: boolean;
  messages: Message[];
  first_user_message?: string | null;
  diagnostics: Diagnostic[];
  visibility_risk: VisibilityRisk;
  confidence: number;
  recent_rank?: number | null;
  jsonl_malformed_lines: number;
  jsonl_raw_events_count: number;
  jsonl_parse_errors: ParseError[];
  path_aliases: string[];
}

export interface WorkspaceGroup {
  key: string;
  display_path: string;
  aliases: string[];
  session_ids: string[];
  suspicious_session_ids: string[];
  diagnostics: Diagnostic[];
}

export interface LocalSessionSummary {
  id: string;
  title: string;
  updated_at?: string | null;
  source?: string | null;
  cwd_raw?: string | null;
  workspace_key?: string | null;
  archived: boolean;
  exists_in_jsonl: boolean;
  exists_in_sqlite: boolean;
  exists_in_index: boolean;
  exists_in_global_state: boolean;
  visibility_risk: VisibilityRisk;
  diagnostic_codes: string[];
  recent_rank?: number | null;
}

export interface PagedSessions {
  items: LocalSessionSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface SearchResult {
  session_id: string;
  title: string;
  snippet?: string | null;
  matched_fields: string[];
  score: number;
  diagnostics: string[];
  updated_at?: string | null;
}

export interface ScanSummary {
  total_sessions: number;
  active_sessions: number;
  archived_sessions: number;
  high_risk_sessions: number;
  medium_risk_sessions: number;
  jsonl_sessions: number;
  sqlite_threads: number;
  index_records: number;
  malformed_jsonl_files: number;
  corrupt_sqlite_databases: number;
}

export interface ScanResult {
  scan_id: string;
  codex_home: string;
  created_at: string;
  sessions: LocalSession[];
  workspaces: WorkspaceGroup[];
  diagnostics: Diagnostic[];
  warnings: string[];
  errors: string[];
  summary: ScanSummary;
}

export interface ScanOptions {
  redact: boolean;
  include_raw: boolean;
  max_message_preview_chars?: number | null;
}

export interface SessionQuery {
  q?: string | null;
  workspace?: string | null;
  archived?: boolean | null;
  risk?: VisibilityRisk | null;
  diagnostic?: string | null;
  limit?: number | null;
  offset?: number | null;
  sort_by?: string | null;
  sort_desc?: boolean | null;
}

export interface ScanEvent {
  scan_id: string;
  stage: string;
  message: string;
  completed: number;
  total?: number | null;
}

export interface RepairPlanOptions {
  include_official_repair: boolean;
  include_workspace_hint_patch: boolean;
  include_jsonl_migration: boolean;
  include_experimental_index_rebuild: boolean;
}

export interface GenerateRepairPlanRequest {
  scan_id: string;
  session_ids: string[];
  options: RepairPlanOptions;
}

export interface RepairOperation {
  id: string;
  kind: RepairOperationKind;
  title: string;
  description: string;
  risk: RepairRisk;
  official_api: boolean;
  experimental: boolean;
  affected_session_ids: string[];
  affected_files: string[];
  evidence: unknown;
  payload: unknown;
  diff?: string | null;
}

export interface RepairSummary {
  affected_sessions: number;
  operations_total: number;
  official_operations: number;
  local_patch_operations: number;
  critical_operations: number;
  highest_risk: RepairRisk;
}

export interface Precondition {
  code: string;
  label: string;
  required: boolean;
  satisfied: boolean;
  evidence: unknown;
}

export interface RepairPlan {
  id: string;
  scan_id: string;
  codex_home: string;
  created_at: string;
  summary: RepairSummary;
  session_ids: string[];
  operations: RepairOperation[];
  preconditions: Precondition[];
  required_backups: string[];
  risk_level: RepairRisk;
  dry_run_required: boolean;
  warnings: string[];
}

export interface AppServerCapability {
  codex_path?: string | null;
  codex_version?: string | null;
  supports_app_server: boolean;
  supports_generate_schema: boolean;
  warnings: string[];
}

export interface ThreadValidationResult {
  thread_id: string;
  listed: boolean;
  archived?: boolean | null;
  read_ok: boolean;
  error?: string | null;
}

export interface AppServerRunSummary {
  capability: AppServerCapability;
  initialized: boolean;
  active_threads_seen: number;
  archived_threads_seen: number;
  thread_validations: ThreadValidationResult[];
  warnings: string[];
  errors: string[];
}

export interface RepairScanDiff {
  before_total_sessions: number;
  after_total_sessions: number;
  before_high_risk_sessions: number;
  after_high_risk_sessions: number;
  selected_session_changes: unknown[];
}

export interface DryRunResult {
  plan_id: string;
  temp_codex_home: string;
  temp_removed_after_run: boolean;
  app_server?: AppServerRunSummary | null;
  diff?: RepairScanDiff | null;
  succeeded: boolean;
  warnings: string[];
  errors: string[];
}

export interface RollbackFile {
  original_path: string;
  backup_path: string;
  sha256: string;
}

export interface RollbackPackage {
  id: string;
  created_at: string;
  root_path: string;
  manifest_path: string;
  full_backup_path?: string | null;
  files: RollbackFile[];
  auto_restore_supported: boolean;
}

export interface VerificationResult {
  repair_id: string;
  scan_summary: ScanSummary;
  app_server?: AppServerRunSummary | null;
  warnings: string[];
  errors: string[];
}

export interface ApplyRepairRequest {
  plan: RepairPlan;
  operation_ids: string[];
  backup_output_path: string;
  rollback_output_dir: string;
  confirm_codex_closed: boolean;
  allow_process_warnings: boolean;
}

export interface ApplyResult {
  repair_id: string;
  plan_id: string;
  applied_operation_ids: string[];
  backup?: unknown;
  rollback?: RollbackPackage | null;
  verification?: VerificationResult | null;
  succeeded: boolean;
  warnings: string[];
  errors: string[];
}

export interface RollbackRequest {
  package: RollbackPackage;
  confirm_restore: boolean;
}

export interface RollbackResult {
  rollback_id: string;
  restored_files: string[];
  succeeded: boolean;
  warnings: string[];
  errors: string[];
}

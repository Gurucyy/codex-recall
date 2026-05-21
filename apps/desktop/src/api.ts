import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CodexHomeCandidate,
  ApplyRepairRequest,
  ApplyResult,
  DryRunResult,
  ExportFormat,
  GenerateRepairPlanRequest,
  LocalSession,
  PagedSessions,
  RepairPlan,
  ReportFormat,
  RollbackRequest,
  RollbackResult,
  ScanEvent,
  ScanOptions,
  ScanResult,
  SearchResult,
  SessionQuery,
  VerificationResult,
  WorkspaceGroup
} from "./types";

export const api = {
  discoverCodexHomes: () => invoke<CodexHomeCandidate[]>("discover_codex_homes"),
  chooseCodexHome: () => invoke<string | null>("choose_codex_home"),
  chooseOutputDir: () => invoke<string | null>("choose_output_dir"),
  startScan: (codexHome: string, options: ScanOptions) =>
    invoke<string>("start_scan", { codexHome, options }),
  getScanResult: (scanId: string) => invoke<ScanResult>("get_scan_result", { scanId }),
  listSessions: (scanId: string, query: SessionQuery) =>
    invoke<PagedSessions>("list_sessions", { scanId, query }),
  getSession: (scanId: string, sessionId: string) =>
    invoke<LocalSession>("get_session", { scanId, sessionId }),
  listWorkspaces: (scanId: string) => invoke<WorkspaceGroup[]>("list_workspaces", { scanId }),
  searchSessions: (scanId: string, text: string) =>
    invoke<SearchResult[]>("search_sessions", { scanId, query: { text, limit: 100, offset: 0 } }),
  exportSessions: (
    scanId: string,
    sessionIds: string[],
    format: ExportFormat,
    outputPath: string,
    redact: boolean
  ) =>
    invoke("export_sessions", {
      request: {
        scan_id: scanId,
        session_ids: sessionIds,
        format,
        output_path: outputPath,
        include_raw: false,
        redact
      }
    }),
  backupCodexHome: (codexHome: string, outputPath: string) =>
    invoke("backup_codex_home", {
      request: {
        codex_home: codexHome,
        output_path: outputPath
      }
    }),
  generateReport: (scanId: string, format: ReportFormat, outputPath: string, redact: boolean) =>
    invoke("generate_report", {
      request: {
        scan_id: scanId,
        format,
        output_path: outputPath,
        redact
      }
    }),
  generateRepairPlan: (request: GenerateRepairPlanRequest) =>
    invoke<RepairPlan>("generate_repair_plan", { request }),
  runRepairDryRun: (plan: RepairPlan) =>
    invoke<DryRunResult>("run_repair_dry_run", { plan }),
  applyOfficialAppserverRepair: (request: ApplyRepairRequest) =>
    invoke<ApplyResult>("apply_official_appserver_repair", { request }),
  applyWorkspaceHintPatch: (request: ApplyRepairRequest) =>
    invoke<ApplyResult>("apply_workspace_hint_patch", { request }),
  applyJsonlMetadataMigration: (request: ApplyRepairRequest) =>
    invoke<ApplyResult>("apply_jsonl_metadata_migration", { request }),
  verifyRepair: (plan: RepairPlan) =>
    invoke<VerificationResult>("verify_repair", { plan }),
  rollbackRepair: (request: RollbackRequest) =>
    invoke<RollbackResult>("rollback_repair", { request }),
  onScanEvent: (event: "scan-started" | "scan-progress" | "scan-finished" | "scan-failed", cb: (payload: ScanEvent) => void) =>
    listen<ScanEvent>(event, (message) => cb(message.payload))
};

import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  X,
  FileDiff,
  RotateCcw,
  ShieldCheck,
  Wrench
} from "lucide-react";
import { api } from "../api";
import type { I18n } from "../i18n";
import type {
  ApplyResult,
  DryRunResult,
  LocalSession,
  RepairOperation,
  RepairOperationKind,
  RepairPlan,
  RepairRisk,
  RollbackPackage,
  VerificationResult
} from "../types";

interface RepairCenterProps {
  i18n: I18n;
  scanId: string;
  session: LocalSession | null;
  open: boolean;
  onClose(): void;
}

const defaultOptions = {
  include_official_repair: true,
  include_workspace_hint_patch: true,
  include_jsonl_migration: true,
  include_experimental_index_rebuild: false
};

const riskRank: Record<RepairRisk, number> = {
  low: 0,
  medium: 1,
  high: 2,
  critical: 3
};

export function RepairCenter({ i18n, scanId, session, open, onClose }: RepairCenterProps) {
  const { t } = i18n;
  const [plan, setPlan] = useState<RepairPlan | null>(null);
  const [selectedOperationIds, setSelectedOperationIds] = useState<Set<string>>(new Set());
  const [dryRun, setDryRun] = useState<DryRunResult | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [verification, setVerification] = useState<VerificationResult | null>(null);
  const [rollbackPackage, setRollbackPackage] = useState<RollbackPackage | null>(null);
  const [backupPath, setBackupPath] = useState("");
  const [rollbackDir, setRollbackDir] = useState("");
  const [confirmClosed, setConfirmClosed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setPlan(null);
    setSelectedOperationIds(new Set());
    setDryRun(null);
    setApplyResult(null);
    setVerification(null);
    setRollbackPackage(null);
    setError(null);
  }, [scanId, session?.id]);

  useEffect(() => {
    if (!open || !session || !scanId) return;
    void generatePlan();
  }, [open, session?.id, scanId]);

  const selectedOperations = useMemo(() => {
    if (!plan) return [];
    return plan.operations.filter((operation) => selectedOperationIds.has(operation.id));
  }, [plan, selectedOperationIds]);

  const canApply = Boolean(
    plan && dryRun?.succeeded && backupPath && rollbackDir && confirmClosed && selectedOperations.length > 0
  );
  const canApplyOfficial =
    canApply && selectedOperations.some((operation) => operationMatchesApplyKind(operation.kind, "official"));
  const canApplyWorkspace =
    canApply && selectedOperations.some((operation) => operationMatchesApplyKind(operation.kind, "workspace"));
  const canApplyJsonl =
    canApply && selectedOperations.some((operation) => operationMatchesApplyKind(operation.kind, "jsonl"));

  async function chooseBackupAndRollback() {
    const dir = await api.chooseOutputDir();
    if (!dir) return;
    const separator = dir.includes("\\") ? "\\" : "/";
    const stamp = new Date().toISOString().replace(/[:.]/g, "-");
    setBackupPath(`${dir}${separator}codex-repair-backup-${stamp}.zip`);
    setRollbackDir(`${dir}${separator}codex-repair-rollback`);
  }

  async function generatePlan() {
    if (!session || !scanId) return;
    setBusy(true);
    setError(null);
    try {
      const nextPlan = await api.generateRepairPlan({
        scan_id: scanId,
        session_ids: [session.id],
        options: defaultOptions
      });
      setPlan(nextPlan);
      setDryRun(null);
      setApplyResult(null);
      setVerification(null);
      setRollbackPackage(null);
      setSelectedOperationIds(new Set(defaultSelectedOperations(nextPlan.operations)));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function runDryRun() {
    if (!plan) return;
    setBusy(true);
    setError(null);
    try {
      const result = await api.runRepairDryRun(plan);
      setDryRun(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function apply(kind: "official" | "workspace" | "jsonl") {
    if (!plan || !canApply) return;
    const ids = selectedOperations
      .filter((operation) => operationMatchesApplyKind(operation.kind, kind))
      .map((operation) => operation.id);
    if (ids.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      const request = {
        plan,
        operation_ids: ids,
        backup_output_path: backupPath,
        rollback_output_dir: rollbackDir,
        confirm_codex_closed: confirmClosed,
        allow_process_warnings: false
      };
      const result =
        kind === "official"
          ? await api.applyOfficialAppserverRepair(request)
          : kind === "workspace"
            ? await api.applyWorkspaceHintPatch(request)
            : await api.applyJsonlMetadataMigration(request);
      setApplyResult(result);
      setVerification(result.verification || null);
      setRollbackPackage(result.rollback || null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function verify() {
    if (!plan) return;
    setBusy(true);
    setError(null);
    try {
      setVerification(await api.verifyRepair(plan));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function rollback() {
    if (!rollbackPackage) return;
    setBusy(true);
    setError(null);
    try {
      const result = await api.rollbackRepair({
        package: rollbackPackage,
        confirm_restore: true
      });
      setApplyResult((previous) =>
        previous
          ? {
              ...previous,
              warnings: [...previous.warnings, ...result.warnings],
              errors: [...previous.errors, ...result.errors]
            }
          : previous
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  function toggleOperation(id: string) {
    setSelectedOperationIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  if (!open) return null;

  return (
    <div className="repair-modal-backdrop" role="presentation">
      <section className="repair-modal" role="dialog" aria-modal="true" aria-label={t("repairCenter")}>
        <header className="repair-header">
          <div>
            <span className="repair-kicker">
              <Wrench size={14} /> {t("repairCenter")}
            </span>
            <p>{t("repairCenterSubtitle")}</p>
          </div>
          <button type="button" className="icon-button" onClick={onClose} title={t("close")}>
            <X size={17} />
          </button>
        </header>

        <div className="repair-modal-body">
          {error ? <div className="repair-error">{error}</div> : null}

          {!plan ? (
            <p className="repair-empty">{session ? t("noRepairPlan") : t("noSessionTitle")}</p>
          ) : (
            <>
              <section className="repair-summary">
                <RiskBadge i18n={i18n} risk={plan.risk_level} />
                <span>{t("operationCount", { count: plan.summary.operations_total })}</span>
                <span>{t("threadCount", { count: plan.summary.affected_sessions })}</span>
              </section>

              <section className="repair-section">
                <h3>{t("preconditions")}</h3>
                <div className="precondition-list">
                  {plan.preconditions.map((item) => (
                    <div key={item.code} className={item.satisfied ? "precondition ok" : "precondition"}>
                      {item.satisfied ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}
                      <span>{preconditionLabel(i18n, item.code, item.label)}</span>
                    </div>
                  ))}
                  <label className="repair-check">
                    <input
                      type="checkbox"
                      checked={confirmClosed}
                      onChange={(event) => setConfirmClosed(event.target.checked)}
                    />
                    <span>{t("closeCodexConfirm")}</span>
                  </label>
                </div>
              </section>

              <section className="repair-section">
                <h3>{t("repairOperations")}</h3>
                <div className="operation-list">
                  {plan.operations.length === 0 ? <p className="repair-empty">{t("noOperations")}</p> : null}
                  {plan.operations.map((operation) => (
                    <label key={operation.id} className="operation-item">
                      <input
                        type="checkbox"
                        checked={selectedOperationIds.has(operation.id)}
                        onChange={() => toggleOperation(operation.id)}
                      />
                      <div>
                        <div className="operation-titleline">
                          <strong>{operationTitle(i18n, operation)}</strong>
                          <RiskBadge i18n={i18n} risk={operation.risk} />
                        </div>
                        <p>{operationDescription(i18n, operation)}</p>
                        <div className="operation-meta">
                          <span>{operation.official_api ? t("officialApi") : t("localPatch")}</span>
                          {operation.experimental ? <span>{t("experimental")}</span> : null}
                          <span>{t("fileCount", { count: operation.affected_files.length })}</span>
                        </div>
                        {operation.diff ? <pre className="repair-diff">{operation.diff}</pre> : null}
                      </div>
                    </label>
                  ))}
                </div>
              </section>

              <section className="repair-actions">
                <button type="button" onClick={() => void chooseBackupAndRollback()} disabled={busy}>
                  <ShieldCheck size={15} /> {t("selectBackupAndRollback")}
                </button>
                <button type="button" onClick={() => void runDryRun()} disabled={busy || !plan}>
                  <FileDiff size={15} /> {t("runDryRun")}
                </button>
                <button type="button" onClick={() => void apply("official")} disabled={busy || !canApplyOfficial}>
                  {t("applyOfficialRepair")}
                </button>
                <button type="button" onClick={() => void apply("workspace")} disabled={busy || !canApplyWorkspace}>
                  {t("applyWorkspacePatch")}
                </button>
                <button type="button" onClick={() => void apply("jsonl")} disabled={busy || !canApplyJsonl}>
                  {t("applyJsonlMigration")}
                </button>
                <button type="button" onClick={() => void verify()} disabled={busy || !plan}>
                  {t("verifyRepair")}
                </button>
                <button type="button" onClick={() => void rollback()} disabled={busy || !rollbackPackage}>
                  <RotateCcw size={15} /> {t("rollbackRepair")}
                </button>
              </section>

              <ResultPanel
                title={t("backupRollback")}
                ok={Boolean(backupPath && rollbackDir)}
                body={backupPath && rollbackDir ? `${backupPath}\n${rollbackDir}` : t("backupPathRequired")}
              />

              <ResultPanel
                title={t("dryRun")}
                ok={dryRun?.succeeded}
                body={dryRun ? summarizeDryRun(i18n, dryRun) : t("dryRunNotStarted")}
              />
              <ResultPanel
                title={t("verification")}
                ok={verification ? verification.errors.length === 0 : undefined}
                body={verification ? summarizeVerification(verification) : applyResult ? applyResult.repair_id : ""}
              />
              <ResultPanel
                title={t("rollbackPackage")}
                ok={rollbackPackage ? rollbackPackage.auto_restore_supported : undefined}
                body={rollbackPackage ? rollbackPackage.manifest_path : rollbackDir}
              />
            </>
          )}
        </div>
      </section>
    </div>
  );
}

function defaultSelectedOperations(operations: RepairOperation[]) {
  return operations
    .filter((operation) => operation.kind === "official_app_server_scan_and_repair")
    .map((operation) => operation.id);
}

function operationMatchesApplyKind(kind: RepairOperationKind, applyKind: "official" | "workspace" | "jsonl") {
  if (applyKind === "official") return kind.startsWith("official_");
  if (applyKind === "workspace") return kind === "patch_workspace_hints";
  return kind === "backfill_thread_source";
}

function summarizeDryRun(i18n: I18n, result: DryRunResult) {
  const app = result.app_server;
  const validation = app?.thread_validations || [];
  return [
    result.succeeded ? i18n.t("succeeded") : i18n.t("failed"),
    app ? i18n.t("appServerListed", { active: app.active_threads_seen, archived: app.archived_threads_seen }) : i18n.t("noAppServerResult"),
    validation.length ? i18n.t("readValidationCount", { ok: validation.filter((item) => item.read_ok).length, count: validation.length }) : ""
  ]
    .filter(Boolean)
    .join(" · ");
}

function summarizeVerification(result: VerificationResult) {
  return `${result.scan_summary.total_sessions} sessions · ${result.errors.length} errors`;
}

function RiskBadge({ i18n, risk }: { i18n: I18n; risk: RepairRisk }) {
  const labels: Record<RepairRisk, string> = {
    low: i18n.t("riskLow"),
    medium: i18n.t("riskMedium"),
    high: i18n.t("riskHigh"),
    critical: i18n.t("riskCritical")
  };
  return <span className={`repair-risk repair-risk-${risk}`}>{riskRank[risk] >= 3 ? labels.critical : labels[risk]}</span>;
}

function ResultPanel({ title, ok, body }: { title: string; ok?: boolean; body: string }) {
  if (!body) return null;
  return (
    <section className="repair-result">
      <strong>
        {ok === undefined ? null : ok ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}
        {title}
      </strong>
      <p>{body}</p>
    </section>
  );
}

function preconditionLabel(i18n: I18n, code: string, fallback: string) {
  const labels: Record<string, string> = {
    codex_app_server_available: i18n.t("preconditionAppServer"),
    full_backup_required: i18n.t("preconditionBackup"),
    dry_run_required: i18n.t("preconditionDryRun"),
    codex_desktop_closed: i18n.t("preconditionCodexClosed")
  };
  return labels[code] || fallback;
}

function operationTitle(i18n: I18n, operation: RepairOperation) {
  const title = titleFromPayload(operation) || operation.title;
  const labels: Record<RepairOperationKind, string> = {
    official_app_server_scan_and_repair: i18n.t("opOfficialScan"),
    official_thread_name_set: i18n.t("opNameSet", { title }),
    official_thread_unarchive: i18n.t("opUnarchive", { title }),
    official_thread_fork_copy: i18n.t("opForkCopy", { title }),
    patch_workspace_hints: i18n.t("opWorkspaceHints"),
    rebuild_session_index: i18n.t("opIndexRebuild"),
    backfill_thread_source: i18n.t("opThreadSource", { title }),
    patch_sqlite_thread_row: i18n.t("opSqlitePatch")
  };
  return labels[operation.kind] || operation.title;
}

function operationDescription(i18n: I18n, operation: RepairOperation) {
  const descriptions: Record<RepairOperationKind, string> = {
    official_app_server_scan_and_repair: i18n.t("opOfficialScanDesc"),
    official_thread_name_set: i18n.t("opNameSetDesc"),
    official_thread_unarchive: i18n.t("opUnarchiveDesc"),
    official_thread_fork_copy: i18n.t("opForkCopyDesc"),
    patch_workspace_hints: i18n.t("opWorkspaceHintsDesc"),
    rebuild_session_index: i18n.t("opIndexRebuildDesc"),
    backfill_thread_source: i18n.t("opThreadSourceDesc"),
    patch_sqlite_thread_row: i18n.t("opSqlitePatchDesc")
  };
  return descriptions[operation.kind] || operation.description;
}

function titleFromPayload(operation: RepairOperation) {
  if (operation.kind === "official_thread_name_set" && isRecord(operation.payload)) {
    return String(operation.payload.new_name || operation.title);
  }
  return operation.title.replace(/^Restore /, "").replace(/ as a copy$/, "").replace(/^Backfill thread_source for /, "");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

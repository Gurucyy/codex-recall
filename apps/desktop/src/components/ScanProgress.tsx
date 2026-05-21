import type { I18n } from "../i18n";
import type { ScanEvent } from "../types";

interface ScanProgressProps {
  i18n: I18n;
  event?: ScanEvent | null;
  error?: string | null;
  processing?: boolean;
}

export function ScanProgress({ i18n, event, error, processing = false }: ScanProgressProps) {
  if (!event && !error) return null;
  if (!error && !processing && event?.stage === "finished") return null;
  const measuredPercent = event?.total
    ? Math.min(100, Math.round((event.completed / event.total) * 100))
    : undefined;
  const percent = processing
    ? measuredPercent === undefined
      ? 99
      : Math.min(99, measuredPercent)
    : measuredPercent;
  const message = error || (processing ? i18n.t("scanStageProcessing") : scanStageLabel(i18n, event));
  const progressStyle = percent === undefined ? undefined : { width: `${percent}%` };

  return (
    <div className={error ? "scan-progress error" : "scan-progress"}>
      <div className="scan-progress-copy">
        <strong>{message}</strong>
        {percent !== undefined ? <span>{percent}%</span> : null}
      </div>
      <div className="scan-progress-track" aria-hidden="true">
        <span className={percent === undefined ? "scan-progress-fill indeterminate" : "scan-progress-fill"} style={progressStyle} />
      </div>
    </div>
  );
}

function scanStageLabel(i18n: I18n, event?: ScanEvent | null) {
  switch (event?.stage) {
    case "start":
      return i18n.t("scanStageStart");
    case "snapshot":
      return i18n.t("scanStageSnapshot");
    case "jsonl":
      return i18n.t("scanStageJsonl");
    case "index":
      return i18n.t("scanStageIndex");
    case "sqlite":
      return i18n.t("scanStageSqlite");
    case "global-state":
      return i18n.t("scanStageGlobalState");
    case "normalize":
      return i18n.t("scanStageNormalize");
    case "diagnostics":
      return i18n.t("scanStageDiagnostics");
    case "summary":
      return i18n.t("scanStageSummary");
    case "complete":
    case "finished":
      return i18n.t("scanFinished");
    default:
      return event?.message || i18n.t("scanStarted");
  }
}

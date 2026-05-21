import { Download } from "lucide-react";
import type { I18n } from "../i18n";
import type { ExportFormat } from "../types";

interface ExportPanelProps {
  i18n: I18n;
  format: ExportFormat;
  redact: boolean;
  selectedCount: number;
  onFormat(format: ExportFormat): void;
  onRedact(value: boolean): void;
  onExport(): void;
}

export function ExportPanel({ i18n, format, redact, selectedCount, onFormat, onRedact, onExport }: ExportPanelProps) {
  return (
    <section className="tool-panel">
      <div className="panel-heading">
        <span>{i18n.t("exportPanel")}</span>
        <Download size={16} />
      </div>
      <div className="segmented">
        {(["markdown", "json", "html"] as ExportFormat[]).map((item) => (
          <button key={item} type="button" className={format === item ? "selected" : ""} onClick={() => onFormat(item)}>
            {item.toUpperCase()}
          </button>
        ))}
      </div>
      <label className="checkline">
        <input type="checkbox" checked={redact} onChange={(event) => onRedact(event.target.checked)} />
        {i18n.t("redact")}
      </label>
      <button type="button" className="wide-action" onClick={onExport}>
        {selectedCount ? i18n.t("exportSelected") : i18n.t("exportAll")}
      </button>
    </section>
  );
}

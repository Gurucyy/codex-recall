import { Archive, FileText } from "lucide-react";
import type { I18n } from "../i18n";

interface BackupPanelProps {
  i18n: I18n;
  onBackup(): void;
  onReport(): void;
}

export function BackupPanel({ i18n, onBackup, onReport }: BackupPanelProps) {
  return (
    <section className="tool-panel">
      <div className="panel-heading">
        <span>{i18n.t("backupReport")}</span>
        <Archive size={16} />
      </div>
      <button type="button" className="wide-action" onClick={onBackup}>
        <Archive size={16} /> {i18n.t("createBackup")}
      </button>
      <button type="button" className="wide-action secondary" onClick={onReport}>
        <FileText size={16} /> {i18n.t("generateReport")}
      </button>
    </section>
  );
}

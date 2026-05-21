import { Archive, Download, FileText, FolderOpen, RefreshCcw, ShieldCheck } from "lucide-react";
import type { I18n, Language } from "../i18n";
import type { CodexHomeCandidate } from "../types";
import appIcon from "../assets/app-icon.png";

interface TopBarProps {
  i18n: I18n;
  selectedHome: string;
  candidates: CodexHomeCandidate[];
  busy: boolean;
  language: Language;
  onLanguage(language: Language): void;
  onSelectHome(path: string): void;
  onChooseHome(): void;
  onScan(): void;
  onExport(): void;
  onBackup(): void;
  onReport(): void;
}

export function TopBar({
  i18n,
  selectedHome,
  candidates,
  busy,
  language,
  onLanguage,
  onSelectHome,
  onChooseHome,
  onScan,
  onExport,
  onBackup,
  onReport
}: TopBarProps) {
  const { t } = i18n;
  const selectedCandidate = candidates.find((candidate) => candidate.path === selectedHome);
  const confidenceLabel = selectedCandidate
    ? `${Math.round(selectedCandidate.confidence * 100)}%`
    : selectedHome
      ? t("manualHome")
      : "--";

  return (
    <header className="topbar">
      <div className="brand-block">
        <img src={appIcon} alt="" aria-hidden />
        <div>
          <h1>{t("appName")}</h1>
          <p><ShieldCheck size={13} aria-hidden /> {t("readOnlyBadge")}</p>
        </div>
      </div>
      <div className="home-picker">
        <div className="home-select-shell">
          <span className="home-confidence" title={t("confidence")}>{confidenceLabel}</span>
          <select
            value={selectedHome}
            onChange={(event) => onSelectHome(event.target.value)}
            aria-label={t("homePlaceholder")}
          >
            <option value="">{t("homePlaceholder")}</option>
            {candidates.map((candidate) => (
              <option key={candidate.path} value={candidate.path}>
                {candidate.path}
              </option>
            ))}
            {selectedHome && !candidates.some((candidate) => candidate.path === selectedHome) ? (
              <option value={selectedHome}>{selectedHome}</option>
            ) : null}
          </select>
        </div>
        <button className="icon-button" type="button" title={t("chooseHome")} onClick={onChooseHome}>
          <FolderOpen size={18} />
        </button>
      </div>
      <nav className="action-strip" aria-label={t("actions")}>
        <div className="language-toggle" aria-label={t("language")}>
          <button type="button" className={language === "zh" ? "selected" : ""} onClick={() => onLanguage("zh")}>
            {t("chinese")}
          </button>
          <button type="button" className={language === "en" ? "selected" : ""} onClick={() => onLanguage("en")}>
            {t("english")}
          </button>
        </div>
        <button type="button" onClick={onScan} disabled={!selectedHome || busy} title={t("scan")}>
          <RefreshCcw size={17} /> {t("scan")}
        </button>
        <button type="button" onClick={onExport} disabled={busy} title={t("export")}>
          <Download size={17} /> {t("export")}
        </button>
        <button type="button" onClick={onBackup} disabled={!selectedHome || busy} title={t("backup")}>
          <Archive size={17} /> {t("backup")}
        </button>
        <button type="button" onClick={onReport} disabled={busy} title={t("report")}>
          <FileText size={17} /> {t("report")}
        </button>
      </nav>
    </header>
  );
}

import { useEffect, useState } from "react";
import { PanelLeftOpen } from "lucide-react";
import { api } from "../api";
import { EmptyState } from "../components/EmptyState";
import { ScanProgress } from "../components/ScanProgress";
import { SearchBar } from "../components/SearchBar";
import { RepairCenter } from "../components/RepairCenter";
import { SessionDetail } from "../components/SessionDetail";
import { SessionList } from "../components/SessionList";
import { TopBar } from "../components/TopBar";
import { useI18n } from "../i18n";
import type {
  CodexHomeCandidate,
  LocalSession,
  PagedSessions,
  ScanEvent,
  ScanResult
} from "../types";

const defaultPage: PagedSessions = { items: [], total: 0, limit: 100, offset: 0 };
const allSessionsLimit = 1_000_000;

export function HomePage() {
  const i18n = useI18n();
  const { t } = i18n;
  const [candidates, setCandidates] = useState<CodexHomeCandidate[]>([]);
  const [selectedHome, setSelectedHome] = useState("");
  const [scanId, setScanId] = useState("");
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [sessionsPage, setSessionsPage] = useState<PagedSessions>(defaultPage);
  const [selectedSession, setSelectedSession] = useState<LocalSession | null>(null);
  const [searchText, setSearchText] = useState("");
  const [scanEvent, setScanEvent] = useState<ScanEvent | null>(null);
  const [scanProcessing, setScanProcessing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [repairModalOpen, setRepairModalOpen] = useState(false);

  useEffect(() => {
    api.discoverCodexHomes()
      .then((items) => {
        setCandidates(items);
        const best = items.find((item) => item.exists && item.confidence > 0);
        if (best) setSelectedHome(best.path);
      })
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    const unlisteners = [
      api.onScanEvent("scan-started", (event) => {
        setScanEvent(event);
        setScanProcessing(false);
        setBusy(true);
        setError(null);
      }),
      api.onScanEvent("scan-progress", (event) => {
        setScanEvent(event);
        setScanProcessing(false);
      }),
      api.onScanEvent("scan-failed", (event) => {
        setScanEvent(event);
        setScanProcessing(false);
        setBusy(false);
        setError(event.message);
      }),
      api.onScanEvent("scan-finished", async (event) => {
        setScanEvent(event);
        setScanProcessing(true);
        setBusy(true);
        try {
          const result = await api.getScanResult(event.scan_id);
          setScanId(event.scan_id);
          setScanResult(result);
          await refreshSessions(event.scan_id, "all", "");
        } catch (err) {
          setError(String(err));
        } finally {
          setScanProcessing(false);
          setBusy(false);
        }
      })
    ];
    return () => {
      void Promise.all(unlisteners).then((items) => items.forEach((unlisten) => unlisten()));
    };
  }, [t]);

  useEffect(() => {
    if (!scanId) return;
    void refreshSessions(scanId, "all", searchText);
  }, [scanId, searchText]);

  async function refreshSessions(nextScanId: string, workspace: string, q: string) {
    const page = await api.listSessions(nextScanId, {
      q: q || null,
      workspace,
      limit: allSessionsLimit,
      offset: 0,
      sort_by: "updated_at",
      sort_desc: true
    });
    setSessionsPage(page);
    setSelectedSession((current) => {
      if (!current) return null;
      return page.items.some((item) => item.id === current.id) ? current : null;
    });
  }

  async function selectSession(nextScanId: string, sessionId: string) {
    const session = await api.getSession(nextScanId, sessionId);
    setSelectedSession(session);
  }

  async function chooseHome() {
    setError(null);
    const path = await api.chooseCodexHome();
    if (path) setSelectedHome(path);
  }

  async function scan() {
    if (!selectedHome) return;
    setBusy(true);
    setScanProcessing(false);
    setStatus(null);
    setError(null);
    setSelectedSession(null);
    setSessionsPage(defaultPage);
    setScanResult(null);
    setScanId("");
    try {
      await api.startScan(selectedHome, {
        redact: false,
        include_raw: false,
        max_message_preview_chars: 160
      });
    } catch (err) {
      setBusy(false);
      setError(String(err));
    }
  }

  async function chooseOutputPath(defaultName: string) {
    const dir = await api.chooseOutputDir();
    if (!dir) return null;
    const separator = dir.includes("\\") ? "\\" : "/";
    return `${dir}${separator}${defaultName}`;
  }

  async function exportNow() {
    if (!scanId) return;
    const path = await chooseOutputPath("codex-sessions.md");
    if (!path) return;
    const ids = selectedSession ? [selectedSession.id] : [];
    await api.exportSessions(scanId, ids, "markdown", path, true);
    setStatus(t("exportDone", { path }));
  }

  async function backupNow() {
    if (!selectedHome) return;
    const path = await chooseOutputPath("codex-backup.zip");
    if (!path) return;
    await api.backupCodexHome(selectedHome, path);
    setStatus(t("backupDone", { path }));
  }

  async function reportNow() {
    if (!scanId) return;
    const path = await chooseOutputPath("recovery-report.md");
    if (!path) return;
    await api.generateReport(scanId, "markdown", path, true);
    setStatus(t("reportDone", { path }));
  }

  return (
    <main className="app-shell">
      <TopBar
        i18n={i18n}
        selectedHome={selectedHome}
        candidates={candidates}
        busy={busy}
        language={i18n.language}
        onLanguage={i18n.setLanguage}
        onSelectHome={setSelectedHome}
        onChooseHome={() => void chooseHome()}
        onScan={() => void scan()}
        onExport={() => void exportNow()}
        onBackup={() => void backupNow()}
        onReport={() => void reportNow()}
      />
      <ScanProgress i18n={i18n} event={scanEvent} error={error} processing={scanProcessing} />
      {status ? <div className="status-line">{status}</div> : null}
      {scanResult ? (
        <div className={sidebarCollapsed ? "workbench sidebar-collapsed" : "workbench"}>
          {sidebarCollapsed ? (
            <aside className="sidebar-rail">
              <button
                type="button"
                className="sidebar-toggle"
                onClick={() => setSidebarCollapsed(false)}
                title={t("expandSidebar")}
              >
                <PanelLeftOpen size={18} />
              </button>
            </aside>
          ) : (
            <div className="middle-column">
              <SearchBar
                i18n={i18n}
                value={searchText}
                status={t("scanComplete", { count: scanResult.summary.total_sessions })}
                count={`${sessionsPage.items.length} / ${sessionsPage.total}`}
                onChange={setSearchText}
                onCollapse={() => setSidebarCollapsed(true)}
              />
              <SessionList
                i18n={i18n}
                sessions={sessionsPage.items}
                total={sessionsPage.total}
                selectedId={selectedSession?.id}
                onSelect={(id) => void selectSession(scanId, id)}
              />
            </div>
          )}
          <SessionDetail
            key={selectedSession?.id ?? "empty-detail"}
            i18n={i18n}
            session={selectedSession}
            onRestoreToCodex={() => setRepairModalOpen(true)}
          />
        </div>
      ) : (
        <EmptyState i18n={i18n} />
      )}
      <RepairCenter
        i18n={i18n}
        scanId={scanId}
        session={selectedSession}
        open={repairModalOpen}
        onClose={() => setRepairModalOpen(false)}
      />
    </main>
  );
}

import { ChevronRight, Folder } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { I18n } from "../i18n";
import { getCodexVisibility, type CodexVisibilityStatus } from "../sessionVisibility";
import type { LocalSessionSummary } from "../types";

interface SessionListProps {
  i18n: I18n;
  sessions: LocalSessionSummary[];
  total: number;
  selectedId?: string;
  onSelect(id: string): void;
}

export function SessionList({ i18n, sessions, total, selectedId, onSelect }: SessionListProps) {
  const unknownWorkspace = i18n.t("unknownWorkspace");
  const groups = useMemo(() => groupSessions(sessions, unknownWorkspace), [sessions, unknownWorkspace]);
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    const selectedGroup = groups.find((group) => group.sessions.some((session) => session.id === selectedId));
    setExpandedGroups(selectedGroup ? new Set([selectedGroup.key]) : new Set());
  }, [groups, selectedId]);

  function toggleGroup(groupKey: string) {
    setExpandedGroups((current) => {
      const next = new Set(current);
      if (next.has(groupKey)) {
        next.delete(groupKey);
      } else {
        next.add(groupKey);
      }
      return next;
    });
  }

  return (
    <section className="session-list-panel">
      <div className="panel-heading">
        <span>{i18n.t("sessions")}</span>
        <strong>{sessions.length}/{total}</strong>
      </div>
      <div className="session-list">
        {groups.map((group) => {
          const expanded = expandedGroups.has(group.key);
          return (
            <section className={expanded ? "project-group expanded" : "project-group"} key={group.key}>
              <button
                type="button"
                className="project-heading"
                title={group.path}
                aria-expanded={expanded}
                onClick={() => toggleGroup(group.key)}
              >
                <ChevronRight size={14} className="project-chevron" aria-hidden />
                <Folder size={15} />
                <span>{group.name}</span>
                <strong>{group.sessions.length}</strong>
              </button>
              {expanded ? (
                <div className="project-sessions">
                  {group.sessions.map((session) => (
                    <button
                      key={session.id}
                      type="button"
                      className={session.id === selectedId ? "session-row selected" : "session-row"}
                      onClick={() => onSelect(session.id)}
                    >
                      <div className="session-row-top">
                        <span className="session-title">{session.title}</span>
                        <VisibilityPill i18n={i18n} status={getCodexVisibility(session)} />
                      </div>
                      <div className="session-row-meta">
                        <span>{formatDate(session.updated_at, i18n.language)}</span>
                        <span>{session.archived ? i18n.t("archived") : i18n.t("active")}</span>
                      </div>
                    </button>
                  ))}
                </div>
              ) : null}
            </section>
          );
        })}
      </div>
    </section>
  );
}

interface ProjectGroup {
  key: string;
  name: string;
  path: string;
  sessions: LocalSessionSummary[];
}

function groupSessions(sessions: LocalSessionSummary[], unknownLabel: string): ProjectGroup[] {
  const groups = new Map<string, ProjectGroup>();
  for (const session of sessions) {
    const path = session.cwd_raw || session.workspace_key || unknownLabel;
    const key = session.workspace_key || path;
    const group = groups.get(key) ?? {
      key,
      name: projectName(path, unknownLabel),
      path,
      sessions: []
    };
    group.sessions.push(session);
    groups.set(key, group);
  }
  return Array.from(groups.values());
}

function projectName(path: string, unknownLabel: string) {
  if (!path || path === unknownLabel) return unknownLabel;
  const normalized = path.replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || normalized || unknownLabel;
}

function VisibilityPill({ i18n, status }: { i18n: I18n; status: CodexVisibilityStatus }) {
  const label = status === "shown"
    ? i18n.t("shownInCodex")
    : status === "archived"
      ? i18n.t("archivedInCodex")
    : status === "local-only"
      ? i18n.t("localOnly")
      : i18n.t("metadataOnly");
  return <span className={`visibility-pill visibility-${status}`}>{label}</span>;
}

function formatDate(value: string | null | undefined, language: string) {
  if (!value) return language === "zh" ? "无时间" : "No timestamp";
  try {
    return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit"
    }).format(new Date(value));
  } catch {
    return value;
  }
}

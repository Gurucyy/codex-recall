import { AlertTriangle, Archive, Folder, FolderSearch, Layers3 } from "lucide-react";
import type { I18n } from "../i18n";
import type { WorkspaceGroup } from "../types";

interface WorkspaceSidebarProps {
  i18n: I18n;
  groups: WorkspaceGroup[];
  selectedKey: string;
  onSelect(key: string): void;
}

const iconForKey = (key: string) => {
  if (key === "archived") return <Archive size={16} />;
  if (key === "possibly-hidden" || key === "risk-high" || key === "risk-medium") return <AlertTriangle size={16} />;
  if (key === "unknown-workspace") return <FolderSearch size={16} />;
  if (key === "all" || key === "active") return <Layers3 size={16} />;
  return <Folder size={16} />;
};

export function WorkspaceSidebar({ i18n, groups, selectedKey, onSelect }: WorkspaceSidebarProps) {
  return (
    <aside className="workspace-sidebar">
      <div className="panel-heading">
        <span>{i18n.t("workspaces")}</span>
        <strong>{groups.find((group) => group.key === "all")?.session_ids.length ?? 0}</strong>
      </div>
      <div className="workspace-list">
        {groups.map((group) => (
          <button
            key={group.key}
            type="button"
            className={group.key === selectedKey ? "workspace-item selected" : "workspace-item"}
            onClick={() => onSelect(group.key)}
            title={group.display_path}
          >
            {iconForKey(group.key)}
            <span>{i18n.workspaceLabel(group)}</span>
            <em>{group.session_ids.length}</em>
          </button>
        ))}
      </div>
    </aside>
  );
}

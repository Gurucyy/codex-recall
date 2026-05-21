import { PanelLeftClose, Search, X } from "lucide-react";
import type { I18n } from "../i18n";

interface SearchBarProps {
  i18n: I18n;
  value: string;
  status?: string | null;
  count?: string | null;
  onChange(value: string): void;
  onCollapse?(): void;
}

export function SearchBar({ i18n, value, status, count, onChange, onCollapse }: SearchBarProps) {
  return (
    <div className="searchbar">
      {status ? (
        <div className="sidebar-scan-bar">
          <span><i aria-hidden />{status}</span>
          {count ? <strong>{count}</strong> : null}
        </div>
      ) : null}
      <div className="search-input-row">
        {onCollapse ? (
          <button type="button" className="sidebar-toggle" onClick={onCollapse} title={i18n.t("collapseSidebar")}>
            <PanelLeftClose size={17} />
          </button>
        ) : null}
        <Search size={17} aria-hidden />
        <input
          value={value}
          onChange={(event) => onChange(event.target.value)}
          aria-label={i18n.t("searchPlaceholder")}
          placeholder={i18n.t("searchPlaceholder")}
        />
        {value ? (
          <button type="button" className="ghost-icon" onClick={() => onChange("")} title={i18n.t("clearSearch")}>
            <X size={16} />
          </button>
        ) : null}
      </div>
    </div>
  );
}

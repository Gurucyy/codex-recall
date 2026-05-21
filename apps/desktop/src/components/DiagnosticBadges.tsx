import type { I18n } from "../i18n";
import type { Diagnostic, VisibilityRisk } from "../types";

export function RiskPill({ risk, i18n }: { risk: VisibilityRisk; i18n: I18n }) {
  return <span className={`pill risk-${risk}`}>{i18n.riskLabel(risk)}</span>;
}

export function DiagnosticBadges({ diagnostics, i18n }: { diagnostics: Diagnostic[] | string[]; i18n: I18n }) {
  if (diagnostics.length === 0) {
    return <span className="muted">{i18n.t("noDiagnostics")}</span>;
  }
  return (
    <span className="diagnostic-row">
      {diagnostics.slice(0, 4).map((diagnostic) => {
        const code = typeof diagnostic === "string" ? diagnostic : diagnostic.code;
        const severity = typeof diagnostic === "string" ? "info" : diagnostic.severity;
        const title = typeof diagnostic === "string" ? code : i18n.diagnosticCopy(diagnostic).title;
        return (
          <span className={`diag-badge diag-${severity}`} key={code} title={`${code}: ${title}`}>
            {code.replace(/^R0*/, "R")}
          </span>
        );
      })}
      {diagnostics.length > 4 ? <span className="diag-badge">+{diagnostics.length - 4}</span> : null}
    </span>
  );
}

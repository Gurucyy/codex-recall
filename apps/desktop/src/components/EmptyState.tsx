import { DatabaseZap } from "lucide-react";
import type { I18n } from "../i18n";

export function EmptyState({ i18n }: { i18n: I18n }) {
  return (
    <section className="empty-state">
      <DatabaseZap size={38} />
      <h2>{i18n.t("emptyTitle")}</h2>
      <p>{i18n.t("emptyBody")}</p>
    </section>
  );
}

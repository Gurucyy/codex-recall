export type CodexVisibilityStatus = "shown" | "archived" | "local-only" | "metadata-only";

interface VisibilityEvidence {
  exists_in_jsonl: boolean;
  exists_in_sqlite: boolean;
  exists_in_index: boolean;
  exists_in_global_state: boolean;
  archived: boolean;
  recent_rank?: number | null;
}

export function getCodexVisibility(session: VisibilityEvidence): CodexVisibilityStatus {
  if (session.archived && session.exists_in_sqlite) {
    return "archived";
  }
  const hasCodexRecord = session.exists_in_sqlite && session.exists_in_index;
  const isRecent = typeof session.recent_rank === "number" && session.recent_rank <= 50;
  if (!session.archived && hasCodexRecord && (session.exists_in_global_state || isRecent)) {
    return "shown";
  }
  if (session.exists_in_jsonl) {
    return "local-only";
  }
  return "metadata-only";
}

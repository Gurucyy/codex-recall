import { AlertCircle, CheckCircle2, ChevronDown, ChevronRight, Copy, Database, FileText, FolderTree, UploadCloud } from "lucide-react";
import { Fragment, useMemo, useState, type ReactNode } from "react";
import { DiagnosticBadges } from "./DiagnosticBadges";
import type { I18n } from "../i18n";
import { getCodexVisibility, type CodexVisibilityStatus } from "../sessionVisibility";
import type { LocalSession, Message, MessageRole } from "../types";

interface SessionDetailProps {
  i18n: I18n;
  session?: LocalSession | null;
  onRestoreToCodex?(session: LocalSession): void;
}

export function SessionDetail({ i18n, session, onRestoreToCodex }: SessionDetailProps) {
  if (!session) {
    return (
      <section className="detail-panel detail-blank" aria-label={i18n.t("noSessionTitle")} />
    );
  }

  const visibility = getCodexVisibility(session);
  const canRestoreToCodex = visibility === "local-only";
  const transcriptEntries = session.messages.map((message, index) => ({ message, index }));
  const logEntries = transcriptEntries.filter(({ message }) => !isChatRole(message.role));
  const chatEntries = transcriptEntries.filter(({ message }) => isChatRole(message.role));

  return (
    <section className="detail-panel">
      <section className="detail-summary-card">
        <header className="detail-header">
          <div>
            <div className="detail-kicker">
              <VisibilityPill i18n={i18n} status={visibility} />
              <span>{session.archived ? i18n.t("archived") : i18n.t("active")}</span>
              {session.recent_rank ? <span>{i18n.t("recentRank", { rank: session.recent_rank })}</span> : null}
            </div>
            <h2>{session.title}</h2>
            <p>{session.id}</p>
          </div>
          {canRestoreToCodex ? (
            <button
              type="button"
              className="restore-cta"
              title={i18n.t("restoreToCodexHint")}
              onClick={() => onRestoreToCodex?.(session)}
            >
              <UploadCloud size={18} /> {i18n.t("restoreToCodex")}
            </button>
          ) : null}
        </header>

        <div className="evidence-strip">
          <EvidenceItem ok={session.exists_in_jsonl} label={i18n.t("jsonl")} i18n={i18n} />
          <EvidenceItem ok={session.exists_in_sqlite} label={i18n.t("sqlite")} i18n={i18n} />
          <EvidenceItem ok={session.exists_in_index} label={i18n.t("index")} i18n={i18n} />
          <EvidenceItem ok={session.exists_in_global_state} label={i18n.t("globalState")} i18n={i18n} />
        </div>
      </section>

      <section className="metadata-grid">
        <Meta icon={<FolderTree size={15} />} label={i18n.t("workspace")} value={session.cwd_raw || i18n.t("unknown")} />
        <Meta icon={<Database size={15} />} label={i18n.t("canonicalPath")} value={session.cwd_canonical || i18n.t("unknown")} />
        <Meta icon={<FileText size={15} />} label={i18n.t("rolloutFile")} value={session.rollout_path || i18n.t("missing")} />
        <Meta icon={<Copy size={15} />} label={i18n.t("source")} value={session.source || i18n.t("unknown")} />
      </section>

      <section className="detail-section">
        <div className="panel-heading">
          <span>{i18n.t("diagnostics")}</span>
          <DiagnosticBadges diagnostics={session.diagnostics} i18n={i18n} />
        </div>
        {session.diagnostics.length === 0 ? (
          <p className="quiet-line">{i18n.t("noDiagnosticsForSession")}</p>
        ) : (
          <div className="diagnostic-list">
            {session.diagnostics.map((diagnostic) => {
              const copy = i18n.diagnosticCopy(diagnostic);
              return (
                <article key={`${diagnostic.code}-${diagnostic.message}`} className={`diagnostic-item diag-${diagnostic.severity}`}>
                  <div className="diagnostic-titleline">
                    <strong>{copy.title}</strong>
                    <code>{diagnostic.code}</code>
                  </div>
                  <p>{copy.body}</p>
                  <small><b>{i18n.t("action")}:</b> {copy.action}</small>
                </article>
              );
            })}
          </div>
        )}
      </section>

      <section className="detail-section transcript">
        <div className="panel-heading transcript-heading">
          <span>{i18n.t("transcript")}</span>
          <strong>{session.messages.length}</strong>
        </div>
        {session.messages.length === 0 ? (
          <p className="quiet-line">{i18n.t("noTranscript")}</p>
        ) : (
          <div className="transcript-body">
            {logEntries.length > 0 ? (
              <section className="transcript-log-stack" aria-label={i18n.t("systemLogs")}>
                <div className="transcript-subheading">
                  <span>{i18n.t("systemLogs")}</span>
                  <strong>{logEntries.length}</strong>
                </div>
                {logEntries.map(({ message, index }) => (
                  <TranscriptLogMessage
                    key={`${message.role}-${index}`}
                    message={message}
                    label={i18n.roleLabel(message.role)}
                  />
                ))}
              </section>
            ) : null}

            {chatEntries.length > 0 ? (
              <section className="qa-panel" aria-label={i18n.t("qaRecords")}>
                <div className="qa-heading">
                  <div>
                    <span>{i18n.t("qaRecords")}</span>
                    <small>{i18n.t("qaRecordsHint")}</small>
                  </div>
                  <strong>{chatEntries.length}</strong>
                </div>
                <div className="chat-thread">
                  {chatEntries.map(({ message, index }) => (
                    <ChatMessage
                      key={`${message.role}-${index}`}
                      message={message}
                      label={i18n.roleLabel(message.role)}
                    />
                  ))}
                </div>
              </section>
            ) : null}
          </div>
        )}
      </section>
    </section>
  );
}

function isChatRole(role: MessageRole) {
  return role === "user" || role === "assistant";
}

function ChatMessage({ message, label }: { message: Message; label: string }) {
  const alignment = message.role === "user" ? "chat-message-user" : "chat-message-assistant";
  return (
    <article className={`chat-message ${alignment} role-${message.role}`}>
      <div className="chat-message-shell">
        <header className="chat-message-label">
          <span className="chat-role-dot" aria-hidden="true" />
          {label}
        </header>
        <div className="chat-bubble">
          <MarkdownContent content={message.content} />
        </div>
      </div>
    </article>
  );
}

function TranscriptLogMessage({ message, label }: { message: Message; label: string }) {
  const [expanded, setExpanded] = useState(false);
  const title = message.raw_type ? `${label} / ${message.raw_type}` : label;

  return (
    <article className={`transcript-log-message role-${message.role}${expanded ? " is-expanded" : ""}`}>
      <button
        type="button"
        className="transcript-log-toggle"
        aria-expanded={expanded}
        onClick={() => setExpanded((current) => !current)}
      >
        {expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
        <span>{title}</span>
        <strong>{message.content.length}</strong>
      </button>
      <div className="transcript-log-collapse" aria-hidden={!expanded}>
        <div className="transcript-log-body">
          <MarkdownContent content={message.content} />
        </div>
      </div>
    </article>
  );
}

type MarkdownBlock =
  | { type: "paragraph"; text: string }
  | { type: "heading"; level: 1 | 2 | 3 | 4; text: string }
  | { type: "code"; language: string; code: string }
  | { type: "ul" | "ol"; items: string[] }
  | { type: "quote"; text: string };

function MarkdownContent({ content }: { content: string }) {
  const blocks = useMemo(() => parseMarkdownBlocks(content), [content]);

  return (
    <div className="markdown-content">
      {blocks.map((block, index) => renderMarkdownBlock(block, index))}
    </div>
  );
}

function renderMarkdownBlock(block: MarkdownBlock, index: number) {
  switch (block.type) {
    case "heading":
      if (block.level === 1) return <h1 key={index}>{renderInlineMarkdown(block.text)}</h1>;
      if (block.level === 2) return <h2 key={index}>{renderInlineMarkdown(block.text)}</h2>;
      if (block.level === 3) return <h3 key={index}>{renderInlineMarkdown(block.text)}</h3>;
      return <h4 key={index}>{renderInlineMarkdown(block.text)}</h4>;
    case "code":
      return (
        <div className="markdown-code" key={index}>
          {block.language ? <span>{block.language}</span> : null}
          <pre><code>{block.code}</code></pre>
        </div>
      );
    case "ul":
      return (
        <ul key={index}>
          {block.items.map((item, itemIndex) => <li key={itemIndex}>{renderInlineMarkdown(item)}</li>)}
        </ul>
      );
    case "ol":
      return (
        <ol key={index}>
          {block.items.map((item, itemIndex) => <li key={itemIndex}>{renderInlineMarkdown(item)}</li>)}
        </ol>
      );
    case "quote":
      return <blockquote key={index}>{renderInlineMarkdown(block.text)}</blockquote>;
    default:
      return <p key={index}>{renderInlineMarkdown(block.text)}</p>;
  }
}

function parseMarkdownBlocks(content: string): MarkdownBlock[] {
  const blocks: MarkdownBlock[] = [];
  const lines = content.replace(/\r\n?/g, "\n").split("\n");
  let paragraph: string[] = [];
  let quote: string[] = [];
  let listType: "ul" | "ol" | null = null;
  let listItems: string[] = [];
  let codeLanguage = "";
  let codeLines: string[] | null = null;

  function flushParagraph() {
    if (paragraph.length === 0) return;
    blocks.push({ type: "paragraph", text: paragraph.join("\n").trimEnd() });
    paragraph = [];
  }

  function flushQuote() {
    if (quote.length === 0) return;
    blocks.push({ type: "quote", text: quote.join("\n").trimEnd() });
    quote = [];
  }

  function flushList() {
    if (!listType || listItems.length === 0) return;
    blocks.push({ type: listType, items: listItems });
    listType = null;
    listItems = [];
  }

  function flushTextBlocks() {
    flushParagraph();
    flushQuote();
    flushList();
  }

  for (const line of lines) {
    const fence = line.match(/^```([\w.+-]*)\s*$/);
    if (codeLines) {
      if (fence) {
        blocks.push({ type: "code", language: codeLanguage, code: codeLines.join("\n") });
        codeLines = null;
        codeLanguage = "";
      } else {
        codeLines.push(line);
      }
      continue;
    }

    if (fence) {
      flushTextBlocks();
      codeLanguage = fence[1] || "";
      codeLines = [];
      continue;
    }

    if (line.trim() === "") {
      flushTextBlocks();
      continue;
    }

    const heading = line.match(/^(#{1,4})\s+(.+)$/);
    if (heading) {
      flushTextBlocks();
      blocks.push({ type: "heading", level: heading[1].length as 1 | 2 | 3 | 4, text: heading[2] });
      continue;
    }

    const quoteLine = line.match(/^>\s?(.*)$/);
    if (quoteLine) {
      flushParagraph();
      flushList();
      quote.push(quoteLine[1]);
      continue;
    }

    const unordered = line.match(/^\s*[-*]\s+(.+)$/);
    const ordered = line.match(/^\s*\d+[.)]\s+(.+)$/);
    if (unordered || ordered) {
      flushParagraph();
      flushQuote();
      const nextListType = unordered ? "ul" : "ol";
      if (listType && listType !== nextListType) flushList();
      listType = nextListType;
      listItems.push((unordered || ordered)?.[1] ?? "");
      continue;
    }

    flushQuote();
    flushList();
    paragraph.push(line);
  }

  if (codeLines) blocks.push({ type: "code", language: codeLanguage, code: codeLines.join("\n") });
  flushTextBlocks();
  return blocks.length > 0 ? blocks : [{ type: "paragraph", text: "" }];
}

function renderInlineMarkdown(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  function pushPlain(segment: string) {
    const pieces = segment.split("\n");
    pieces.forEach((piece, index) => {
      if (index > 0) nodes.push(<br key={`br-${nodes.length}`} />);
      if (piece) nodes.push(<Fragment key={`text-${nodes.length}`}>{piece}</Fragment>);
    });
  }

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > cursor) pushPlain(text.slice(cursor, match.index));
    const token = match[0];
    if (token.startsWith("`")) {
      nodes.push(<code key={`code-${nodes.length}`}>{token.slice(1, -1)}</code>);
    } else if (token.startsWith("**")) {
      nodes.push(<strong key={`strong-${nodes.length}`}>{renderInlineMarkdown(token.slice(2, -2))}</strong>);
    } else {
      const link = token.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      const href = link ? safeMarkdownHref(link[2]) : null;
      nodes.push(
        href ? (
          <a key={`link-${nodes.length}`} href={href} target="_blank" rel="noreferrer">
            {link?.[1]}
          </a>
        ) : (
          <Fragment key={`link-text-${nodes.length}`}>{link?.[1] ?? token}</Fragment>
        )
      );
    }
    cursor = match.index + token.length;
  }

  if (cursor < text.length) pushPlain(text.slice(cursor));
  return nodes;
}

function safeMarkdownHref(href: string) {
  const value = href.trim();
  return /^(https?:|mailto:|file:)/i.test(value) ? value : null;
}

function EvidenceItem({ ok, label, i18n }: { ok: boolean; label: string; i18n: I18n }) {
  return (
    <span className={ok ? "evidence-item ok" : "evidence-item missing"} title={ok ? i18n.t("present") : i18n.t("missing")}>
      {ok ? <CheckCircle2 size={15} /> : <AlertCircle size={15} />} {label}
    </span>
  );
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

function Meta({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div className="meta-row">
      <span>{icon}{label}</span>
      <code>{value}</code>
    </div>
  );
}

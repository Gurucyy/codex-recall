# Codex Recall

English | [中文](./README_zh.md)

[![Rust](https://img.shields.io/badge/Rust-stable%2B-000000?logo=rust&logoColor=white)](./Cargo.toml)
[![Node.js](https://img.shields.io/badge/Node.js-20%2B-339933?logo=node.js&logoColor=white)](./package.json)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](./apps/desktop/src-tauri/tauri.conf.json)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

`codex-recall` is an unofficial, local-first desktop application for
viewing, searching, exporting, backing up, diagnosing, and carefully repairing local
OpenAI Codex session data.

It is built for cases where local Codex sessions are missing from the Codex Desktop
sidebar, project view, or search, while the underlying files may still exist on disk.

> [!IMPORTANT]
> This is not an official OpenAI project. Default vault workflows are read-only toward
> original Codex data, transcripts are not uploaded, and Repair Center is experimental,
> local, gated, backed up, and reversible where possible.

## Table of Contents

- [Why This Exists](#why-this-exists)
- [Core Features](#core-features)
- [Quick Start](#quick-start)
- [Development Checks](#development-checks)
- [App Workflow](#app-workflow)
- [Optional CLI](#optional-cli)
- [Safety And Privacy](#safety-and-privacy)
- [Repair Center](#repair-center)
- [Repository Layout](#repository-layout)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [Security](#security)
- [License](#license)

## Why This Exists

Codex Desktop stores useful session evidence locally: rollout JSONL, session index
records, state SQLite files, and global state metadata. When those sources drift, become
partial, or stop matching the sidebar, it can be difficult to understand what still
exists and what is safe to recover.

This project provides a local recovery vault for inspecting that evidence without
turning normal scan, view, search, export, backup, or report flows into write-back
operations.

## Core Features

- Discover Codex home directories automatically or select one manually.
- Scan `sessions/` and `archived_sessions/`.
- Tolerantly parse rollout JSONL, `session_index.jsonl`, all `state_*.sqlite` files,
  and `.codex-global-state.json`.
- Normalize local session evidence and group sessions by workspace.
- Search title, content, path, thread ID, diagnostics, archive state, and risk.
- View transcripts, metadata, raw evidence summaries, and diagnostics.
- Export Markdown, JSON, and self-contained HTML.
- Create full `.codex` backups with SHA-256 manifests.
- Generate local recovery reports with redaction options.
- Use guarded Repair Center workflows for selected repair operations.

## Quick Start

### Prerequisites

- Rust stable and Cargo
- Node.js 20 or newer
- pnpm 10 or newer
- Tauri 2 platform prerequisites for your operating system

### Run The Desktop App

```bash
git clone https://github.com/Gurucyy/codex-recall.git
cd codex-recall
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` opens a native desktop window. This project is not a browser tool and
does not require a local HTTP server or localhost browser workflow.

### One-Line Agent Setup

If you use Codex, Claude Code, Cursor, Windsurf, or another coding agent, you can hand it
the setup intent in one sentence:

```text
Help me clone codex-recall if needed, install dependencies, run the narrow checks, and start the Tauri desktop app by following README.md.
```

## Development Checks

Run frontend checks:

```bash
pnpm typecheck
pnpm build
```

Run Rust checks:

```bash
cargo test --workspace
```

Build desktop packages:

```bash
pnpm tauri build
```

## App Workflow

1. Open the native desktop app.
2. Auto-detect or manually select a Codex home directory.
3. Scan local Codex data.
4. Browse active and archived sessions.
5. Search sessions by title, content, path, thread ID, diagnostics, archive state, or
   risk.
6. Open a session to inspect transcript, metadata, evidence, and diagnostics.
7. Export selected sessions or generate a recovery report.
8. Create a full backup before attempting any Repair Center workflow.

## Optional CLI

The CLI reuses the same local Rust core:

```bash
codex-vault discover
codex-vault scan --codex-home ~/.codex --out scan-report.json
codex-vault list --codex-home ~/.codex --risk high
codex-vault search --codex-home ~/.codex --q "diagnostic:R012 risk:high"
codex-vault export --codex-home ~/.codex --format markdown --out ./exports
codex-vault backup --codex-home ~/.codex --out ./codex-backup.zip
codex-vault report --codex-home ~/.codex --out recovery-report.md
```

There is no `serve` command.

## Safety And Privacy

Default scan, view, search, export, backup, and report workflows are read-only toward
your original Codex data.

The app does not:

- write to original `state_*.sqlite` files during normal vault workflows
- patch `session_index.jsonl`
- patch `.codex-global-state.json`
- move, rename, or delete Codex session files
- upload transcripts, paths, metadata, diagnostics, reports, or backups
- require a browser or localhost workflow
- load remote JavaScript, CSS, fonts, or images in exported HTML

Exports, backups, reports, rollback packages, and app settings are the normal write
targets. Export, backup, report, and rollback outputs go only to user-selected paths or
the app config directory as appropriate.

Exported files and backups may contain private code, transcript content, local paths,
commands, and other sensitive data. Review them before sharing.

## Repair Center

Repair Center is experimental and guarded. It is not a blind one-click repair tool.

Write-capable repair paths require selected sessions, a generated repair plan, dry-run
against a copied `CODEX_HOME`, a full `.codex` backup before real apply, a rollback
package before local patches, explicit confirmation that Codex Desktop is closed, and a
verification scan after apply.

Official `codex app-server --listen stdio://` operations are preferred when available.
Local patches are narrow and currently limited to missing workspace-root hints and
selected rollout JSONL `session_meta.payload.thread_source` backfills.

## Repository Layout

```text
apps/desktop/                 Tauri 2 + React desktop app
crates/codex-vault-core/      scanner, diagnostics, search, export, backup, repair core
crates/codex-vault-cli/       optional local CLI
docs/                         architecture, privacy, and roadmap docs
fixtures/                     synthetic fixture notes and test data only
```

## Documentation

- [Architecture](./docs/architecture.md)
- [Privacy](./docs/privacy.md)
- [Roadmap](./docs/roadmap.md)

## Contributing

Contributions are welcome when they preserve the local-first, desktop-first, read-only
default safety model. See [CONTRIBUTING.md](./CONTRIBUTING.md) and
[CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).

Use synthetic fixtures only. Do not commit real Codex session data, transcripts, local
paths, backups, reports, exports, or repair outputs.

## Security

Please read [SECURITY.md](./SECURITY.md) before reporting vulnerabilities or sharing
diagnostic evidence. Do not post private transcripts, tokens, local paths, or backups in
public issues.

## License

MIT. See [LICENSE](./LICENSE).

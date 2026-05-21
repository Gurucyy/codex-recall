# Architecture

`codex-recall` is a local-first Tauri desktop app.

```text
React + TypeScript UI
        |
        | Tauri commands
        v
Rust Tauri backend
        |
        | codex-vault-core API
        v
Scanner, normalizer, diagnostics, search, export, backup, report, experimental repair planner
        |
        | read-only filesystem and SQLite access
        v
User-selected Codex home
```

## Read-Only Boundary

The core scanner reads from Codex home and never mutates it. SQLite files are copied into a temporary snapshot before read-only access. Session JSONL files are streamed directly from disk in read-only mode.

The only output writers are:

- session export
- diagnostics/recovery report
- zip backup
- app settings stored in the app config directory

Normal vault commands do not write to Codex original data.

Repair Center commands are explicitly outside the normal read-only boundary. They require selected operations, backup, dry-run, rollback packaging, explicit confirmation, and verification before writing local Codex files.

## Core Modules

- `discovery`: finds candidate Codex homes and scores evidence
- `snapshot`: copies metadata files into a temporary snapshot for read-only readers
- `jsonl_parser`: streams `sessions/**/*.jsonl` and `archived_sessions/**/*.jsonl`
- `session_index_reader`: tolerant line-by-line `session_index.jsonl` parser
- `sqlite_reader`: read-only `state_*.sqlite` introspection
- `global_state_reader`: flexible `.codex-global-state.json` extraction
- `normalizer`: merges evidence into `LocalSession`
- `diagnostics`: implements R001-R018
- `search`: local in-memory keyword/filter search
- `export`: Markdown, JSON, and self-contained HTML output
- `backup`: zip backup with manifest and SHA-256 hashes
- `redaction`: report/export masking helpers
- `scan`: high-level orchestration
- `repair`: experimental plan/dry-run/apply/verify/rollback workflows

## Repair Center Architecture

Repair Center is split into official and local layers:

- Official layer: starts `codex app-server --listen stdio://`, initializes JSON-RPC over stdio, runs paged `thread/list useStateDbOnly=false`, and validates selected threads with `thread/read`.
- Local patch layer: applies narrow patches only when official APIs do not cover the issue, currently workspace-root hints and legacy JSONL `thread_source` backfill.
- Rollback layer: creates a full `.codex` backup and file-level rollback package before local patches.

There is no `apply_all_repairs` API. Each repair category has its own command and confirmation path.

## Desktop Commands

The Tauri backend exposes only read-only or user-output commands:

- discovery
- manual directory selection
- scan
- list/get/search sessions
- export
- backup
- report
- app settings

Experimental Repair Center commands add:

- generate repair plan
- run dry-run on copied `CODEX_HOME`
- apply official app-server repair
- apply workspace hint patch
- apply JSONL metadata migration
- verify repair
- rollback file-level patches

SQLite direct patching and durable cwd/path remapping are not implemented in the MVP.

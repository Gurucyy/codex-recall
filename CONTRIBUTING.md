# Contributing

Thank you for helping improve `codex-recall`.

This project is an unofficial, local-first desktop application for Codex session
recovery workflows. Contributions are welcome when they preserve the safety model
described in [README.md](./README.md), [docs/architecture.md](./docs/architecture.md),
and [docs/privacy.md](./docs/privacy.md).

## Project Boundaries

Keep the project:

- Desktop-first with Tauri 2, Rust, React, TypeScript, and Vite.
- Local-first with no transcript upload or remote analysis service.
- Read-only by default toward original Codex data.
- Tolerant of malformed, missing, partial, or corrupt local files.
- Explicitly gated for all write-capable Repair Center operations.

Do not introduce:

- FastAPI, Electron, a Next.js server, or a local HTTP server.
- Browser-only or localhost-required user workflows.
- Telemetry, remote transcript analysis, remote search, or remote repair.
- Generic apply-all repair, destructive cleanup, direct SQLite patching, or broad
  timestamp/path rewrites without a separately reviewed phase.

## Development Setup

Prerequisites:

- Rust stable and Cargo
- Node.js 20 or newer
- pnpm 10 or newer
- Tauri 2 platform prerequisites for your operating system

Install dependencies:

```bash
pnpm install
```

Run the desktop app:

```bash
pnpm tauri dev
```

The app should open as a native desktop window.

## Validation

Run the narrowest relevant checks first.

Frontend:

```bash
pnpm typecheck
pnpm build
```

Rust:

```bash
cargo test --workspace
```

For broad changes, run both frontend and Rust checks before opening a pull request.

## Test Data

Use synthetic fixtures only. Do not depend on real user Codex data in tests, examples,
screenshots, issue attachments, or committed artifacts.

Fixtures and tests should cover malformed JSONL, missing or corrupt SQLite, archived
sessions, duplicate or orphan index records, missing workspace hints, title drift,
unknown cwd, and Repair Center precondition failures when relevant.

## Repair Center Changes

Repair Center is experimental and must remain gated. Write-capable repair code is allowed
only inside explicit Repair Center paths and only when these safeguards remain present:

- selected sessions, not global apply-all behavior
- generated repair plan before apply
- dry-run against a copied `CODEX_HOME`
- full `.codex` backup before real apply
- rollback package before local patches
- explicit user confirmation that Codex Desktop is closed
- operation-specific apply commands
- verification scan after apply

Prefer official `codex app-server --listen stdio://` operations when available. Local
patches must stay narrow and reversible.

## Documentation

Update documentation when behavior, setup, or safety boundaries change:

- [README.md](./README.md)
- [README_zh.md](./README_zh.md)
- [docs/architecture.md](./docs/architecture.md)
- [docs/privacy.md](./docs/privacy.md)
- [docs/roadmap.md](./docs/roadmap.md)

The retired `specs/` directory is not a source of truth and should not be recreated
unless maintainers explicitly request it.

## Pull Request Checklist

Before requesting review, confirm:

- Normal scan, view, search, export, backup, and report flows do not write original
  Codex data.
- Repair Center write paths are explicitly gated and backed up.
- Exported HTML remains self-contained.
- Errors degrade to diagnostics or user-facing warnings where possible.
- Frontend types stay aligned with Rust models.
- Tests use synthetic fixtures only.
- Relevant docs are updated.
- No private transcripts, local paths, backups, reports, exports, or repair outputs are
  committed.

## Security And Conduct

Follow [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md). For vulnerabilities or accidental
exposure of sensitive data, follow [SECURITY.md](./SECURITY.md) and avoid public details.

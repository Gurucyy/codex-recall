# Roadmap

This project is in maintenance and hardening mode. Phase 1 recovery-vault behavior is
complete enough to preserve as the default product shape, and Repair Center remains
experimental and explicitly gated.

## Complete: Read-Only Recovery Vault

- Discover Codex home directories.
- Scan active and archived sessions.
- Read JSONL, session index, SQLite, and global state evidence.
- Normalize sessions and group workspaces.
- Search local sessions.
- Show transcripts, metadata, and diagnostics.
- Export Markdown, JSON, and self-contained HTML.
- Create full `.codex` backups with manifest.
- Generate recovery reports.

Default vault workflows do not write back to original Codex data.

## Current Focus: Open Source Hardening

- Keep README, privacy, architecture, roadmap, contribution, security, and conduct docs
  current.
- Maintain GitHub Actions checks for Rust and frontend validation.
- Keep `.gitignore` strict for local Codex data, generated outputs, credentials, build
  artifacts, editor state, and desktop packaging products.
- Add or improve synthetic fixtures for edge cases instead of using real user data.
- Continue making diagnostics clear, evidence-backed, and safe to share after redaction.

## Current Focus: Repair Center Guardrails

- Generate repair plans for selected sessions only.
- Prefer official `codex app-server --listen stdio://` operations.
- Dry-run all repair plans against a copied `CODEX_HOME`.
- Require a full backup and rollback package before real apply.
- Apply official operations separately from local patches.
- Limit local patches to workspace-root hints and selected rollout JSONL
  `session_meta.payload.thread_source` backfills.
- Verify after apply and keep rollback workflows available.

## Future Candidates

- More diagnostics for malformed or drifting metadata.
- Better process-lock detection and schema fingerprinting.
- Improved redaction previews for reports and exports.
- More cross-platform fixture coverage, including Windows path edge cases.
- Optional release packaging workflows after signing and distribution policy are settled.

## Explicit Non-Goals

- Silent auto-fix behavior.
- Generic apply-all repair.
- Direct SQLite patching in normal workflows.
- Destructive cleanup, file deletion, file moving, or mass timestamp changes.
- Browser-only workflow, local HTTP server, telemetry, or remote transcript analysis.
- Recreating the retired `specs/` directory unless maintainers explicitly request it.

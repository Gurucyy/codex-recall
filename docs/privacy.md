# Privacy

This app is designed for local inspection of sensitive Codex session data.

## Local-Only Processing

Scanning, diagnostics, search, export, backup, and report generation run locally. The app does not upload transcripts, metadata, paths, diagnostics, reports, or backups.

## Sensitive Outputs

Exports, backups, and reports may contain:

- source code
- commands
- user prompts and assistant responses
- local file paths
- workspace names
- environment details
- diagnostic evidence

Use redaction options when sharing reports outside your machine.

## Read-Only Codex Data Access

Default vault workflows do not modify Codex original files. They do not write to SQLite databases, `session_index.jsonl`, `.codex-global-state.json`, or session JSONL files.

## Repair Center Privacy and Safety

Repair Center is an experimental workflow. It still runs locally and does not upload transcripts, metadata, diagnostics, reports, or backups.

Official repair launches the local `codex app-server --listen stdio://` process with the selected `CODEX_HOME`. The app-server may perform Codex-owned local indexing and validation. This tool uses that official path before attempting local patches.

Any real write operation requires:

- a user-selected backup output path
- a user-selected rollback directory
- a dry-run on a copied `CODEX_HOME`
- explicit confirmation that Codex Desktop is closed
- selected operations rather than apply-all repair

Local patches are limited to documented files and are backed up before replacement.

## HTML Export

HTML exports are self-contained. They do not load remote JavaScript, CSS, fonts, or images.

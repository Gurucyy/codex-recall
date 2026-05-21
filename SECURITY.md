# Security Policy

`codex-recall` works near sensitive local Codex data. Please report
security issues carefully and avoid sharing private transcripts, tokens, local paths,
backups, exports, or repair outputs in public.

## Supported Versions

Until the project has tagged releases, security fixes target the `main` branch.

| Version | Supported |
| --- | --- |
| `main` | Yes |
| older snapshots | No |

## Reporting A Vulnerability

Use GitHub private vulnerability reporting from the repository Security tab when it is
enabled.

If private vulnerability reporting is not enabled yet, open a minimal public issue asking
maintainers to provide a private reporting channel. Do not include exploit details,
private transcript content, local file paths, tokens, screenshots with sensitive data, or
backup/export files in the public issue.

Good reports include:

- affected commit, branch, or release
- operating system and app version
- clear reproduction steps using synthetic data
- expected and actual behavior
- impact assessment
- whether original Codex data could be modified, deleted, leaked, or uploaded

## Security Model

Default vault workflows are local and read-only toward original Codex data. The app
should not upload transcripts, metadata, paths, diagnostics, reports, or backups.

Normal workflows must not:

- write original `state_*.sqlite`
- patch `session_index.jsonl`
- patch `.codex-global-state.json`
- delete, move, or rename Codex session files
- load remote JavaScript, CSS, fonts, or images in exported HTML
- expose a runtime port or require a browser workflow

Repair Center is the only write-capable area. It must remain experimental, selected,
planned, dry-run, backed up, confirmed, verified, and reversible where possible.

## Out Of Scope

These are usually not treated as project vulnerabilities:

- Sensitive data exposure caused by intentionally sharing exports, reports, or backups.
- Local machine compromise unrelated to this app.
- Manual edits to Codex files outside this app.
- Issues that require disabling documented safeguards.

If in doubt, report privately.

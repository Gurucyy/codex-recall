## Summary

Describe the change and why it is needed.

## Validation

- [ ] `pnpm typecheck`
- [ ] `pnpm build`
- [ ] `cargo test --workspace`
- [ ] Other:

## Safety Checklist

- [ ] Normal vault flows remain read-only toward original Codex data.
- [ ] Repair Center write paths, if touched, remain selected, planned, dry-run, backed up,
      confirmed, verified, and rollback-aware.
- [ ] No browser-only workflow, local HTTP server, telemetry, or remote transcript analysis
      was added.
- [ ] Exported HTML remains self-contained.
- [ ] Tests and examples use synthetic fixtures only.
- [ ] No private transcripts, local paths, backups, reports, exports, or repair outputs are
      committed.
- [ ] Documentation was updated when behavior, setup, or safety boundaries changed.

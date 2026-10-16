# Frozen-mtime stale views: treat compilation as the authoritative reader

**Rule class:** workflow issue · **First proven:** run e40916d7 cycle 2 implement 755108730 (2026-10-15), on `crates/agenttrace-core/tests/opencode_bounded_admission.rs`; sibling of the transient-revert hazard (run b288fc32, 2026-10-16, memory-verified) · **Repo:** agenttrace

## The failure mode

Mid-implement, a tracked test file began serving **alternating stale views** to read/edit tools: the same path returned different contents on consecutive reads while its mtime stayed frozen at checkout time. Symptoms: the edit tool failed atomically ("oldText not found") on text a fresh read had just shown; a scripted append aborted on its own md5 guard (on-disk bytes ≠ what the script read seconds earlier); `git status` stayed clean throughout, so no porcelain signal appeared. Unlike the b288fc32 incident (a verified edit silently reverted), here edits never landed at all and reads could not be trusted — but the class is the same: **the worktree filesystem view is not always the truth.**

## The rule

1. **Md5-pin before editing any file you will edit repeatedly this session.** Record `md5sum` (and `git show HEAD:<path>` for tracked files) *before* the first edit; re-verify at every phase boundary and before the commit gate.
2. **Treat compilation (or any executed consumer) as the authoritative reader.** When reads disagree, `cargo`/`node`/the test runner is reading the real bytes. Iterate against the compiler, not against the editor's view of the file.
3. **Regenerate, don't patch, on the second inconsistency.** After the first atomic edit failure, stop hand-patching: regenerate the whole file in one guarded pass from the git-verified base (read `git show HEAD:<path>` → apply all intended changes in memory → single write with an md5 assertion) — idempotent by construction, and it defeats stale-view alternation because there is only one write.
4. **Keep spool-side backups of every src edit.** Before each edit round, copy the current file to the delegate spool scratch dir; after any scare, diff the live file against the backup to prove which edits survived. This is what let run e40916d7 prove `parser.rs`/`doctor.rs` edits were never lost while the test file misbehaved.
5. **Disclose the incident in the phase record.** The event sequence, the md5 pins, and the regeneration step belong in the envelope so the commit gate knows the file's provenance chain (base md5 → regenerated md5 → compiled green).

## Escape proof (run e40916d7)

- Base pinned: `git show c4aa3a0:…/opencode_bounded_admission.rs` md5 `d6fef71a…`; final on-disk md5 `4a5662a1…` after one guarded regeneration + fmt; both recorded.
- `parser.rs`/`doctor.rs` verified byte-identical to spool backups after each scare (zero lost src edits).
- Outcome compiled to green in the same phase (core 54 suites ok) and stayed green through targeted (11/0 + 411/0) and full (67 suites / 1162/0) validation — the compiler's view, never the editor's, was the judge.

*Related: cycle-2 compound record `docs/stewardship/2026-10-16-cycle2-compound-record-rune40916d7ef92.md` (L7); transient-revert hazard (run b288fc32) is the sibling mode — there a verified edit silently reverted, here reads/edits served stale views with nothing landing.*

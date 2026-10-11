# Prevention rule: conductor-salvage commits preserve content, not validation claims

- **Class:** workflow / delegate-session infra and validation-evidence lineage
- **Observed:** 2026-10-06/07, run `1ecd791b316c4107aecf4f76522dfc20` cycle 1 (repository-maintenance `224c5cf1`), targeted_tests phase — an attempt died at 13:08:49Z and the conductor salvage path committed the uncommitted implement delta as `ae7277239de9b69dc015b936c684e2c14bed7915` on `conductor/run-1ecd791b316c`
- **Cost:** a salvaged batch arrives as a **committed** tree whose trailers explicitly disclaim validation ("no validation claim — content preservation only"). A later phase that does not read the trailers either (a) treats the commit as already-validated evidence and skips fresh validation — declaring outcomes no one ever produced — or (b) is surprised by unexpected tree state (clean where a dirty delta was expected) and stalls instead of adapting.

## What happened

The implement phase of run `1ecd791b` left the rm-594 batch as a 5-file uncommitted delta (`+188/−4`). The first targeted_tests attempt died before running anything. The conductor salvage path then committed the delta as `ae72772`:

```
conductor-salvage: run=1ecd791b316c4107aecf4f76522dfc20
Salvage-Run: 1ecd791b316c4107aecf4f76522dfc20
Salvage-Status: failed
Salvage-Phase: targeted_tests:targeted_tests
Salvage-Touched: 5 paths
salvage: no validation claim — content preservation only; the integration fleet owns landing
```

The targeted phase that actually completed (`4019084d`) dispatched into a tree that was **clean** — not the dirty delta the work order's narrative described. It (1) recognized the salvage by trailer, (2) verified the commit preserved the batch exactly (`git show --stat ae72772` = the same 5 files, +188/−4, no extra hunks; per-file diffs matched the implement record), and (3) ran its validation **fresh on top of the salvage commit**, declaring only outcomes it produced itself. The subsequent full_tests phase ran the authoritative full command on the same tree. Nothing from the dead attempt was ever transcribed as evidence.

## Prevention rules

1. **Read the trailers before trusting a tree you did not build.** Any `conductor-salvage:` commit carries `Salvage-Status` and a "no validation claim" sentence by construction. Content preservation is the salvage path's only promise.
2. **Verify content, then validate fresh.** Confirm the salvaged commit's file set and diffstat against the phase record that built the delta (and spot-check per-file diffs); then run the phase's own validation against the salvaged tree. Never inherit, transcribe, or "resume" validation claims from attempts that died before the salvage.
3. **Expect the porcelain to change under you.** A phase dispatched against "the uncommitted delta" may find it committed. Adapt: re-derive tree identity (`git rev-parse HEAD`, `git status --porcelain`) at dispatch, and record the discovered state in the phase result instead of failing on the mismatch.
4. **Triage dead attempts with three checks, then redo.** Typed result absent + `events/<attempt>.jsonl` tail `session_reaped:failed` + porcelain at dispatch state ⇒ zero durable work: redo the phase from scratch and declare the redo. (This run had three such deaths — assess `84771adf`, implement `6340fa30`, and the salvaged targeted_tests attempt — each redone, none adopted sight unseen.)
5. **Keep scratch evidence outside the tree.** The salvage can only preserve what is in the tree; probe corpora, logs, and verification notes belong in the delegate spool or `/tmp`, so a salvage commit is always exactly the batch and nothing else.

## Verification

- `git show --stat ae7277239de9b69dc015b936c684e2c14bed7915` → 5 files changed, 188 insertions(+), 4 deletions(-), with the trailers quoted above.
- Fresh validation on the salvaged tree is recorded in typed results `delegate/4019084d40f34fa1bf489a15ee8fdf28.json` (targeted) and `delegate/18b1e33107c84a4684a69fb504d5b2f3.json` (full) — both cite `HEAD = ae72772…`, porcelain clean, outcomes produced by their own commands.
- Full cycle narrative: `docs/stewardship/2026-10-07-cycle1-compound-record-run1ecd791b.md` §2.

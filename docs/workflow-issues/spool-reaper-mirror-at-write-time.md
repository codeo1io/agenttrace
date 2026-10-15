# The spool reaper sweeps current-run scratch — mirror at write time

**Class:** workflow issue (delegate infrastructure) · **Origin:** twice-instantiated in one fleet day (2026-10-15): run f2fccaa9's cycle-3 band and run a3f96656's entire seven-phase chain · **Applies to:** every conductor delegate that banks artifacts "spool-side" and intends them to be applied later

## What happens

The delegate-spool reaper sweeps `delegate/<attempt>-scratch/` directories aggressively — including the **current run's own earlier-phase scratch**, within hours, while the run is still in flight. Observed at run a3f96656 compound: the research, roadmap, prioritize, stewardship ×2, implement, and targeted scratch dirs (plus their `/tmp` mirrors, which age out on their own schedule) were **gone** by the compound phase. One exception proved survival is luck, not contract: the full-tests scratch (its `full-tests-note.md`) and its `/home/agent/tmp` mirror were still present. What survives reliably: the typed result envelopes (`delegate/<attempt>.json`) and event logs.

The expensive variant: a roadmap band that was deliberately delivered **spool-side** ("the commit gate will apply it") loses its patch, postimage, and builder in one sweep — and the gate then has nothing to apply. That cost a full band rebuild at run f2fccaa9 and again here.

## The rules

1. **Mirror at write time, not at end of phase.** Every durable artifact you bank gets copied to a second location the moment it is written (e.g. `/home/agent/.cache/at-<phase>-<attempt>/`). An unmirrored spool artifact should be treated as already lost.
2. **Prefer in-tree for anything a later phase or gate must apply.** When the phase contract permits touching tracked files, land the band/patch in-tree (uncommitted) and bank the spool copy as backup — the worktree is the only store the reaper does not sweep. The commit gate stages from the tree; it cannot stage from reaped history.
3. **Write typed envelopes as if they are the only survivors — because they are.** A roadmap envelope must carry the full band spec (banner inventory, every rider id + gist, every mint id + priority + acceptance gist, postimage md5, next-free numeral). This run proved a complete band is **reconstructible from the envelope alone**: identity lineage, riders, mints, and md5 chain rebuilt the band honestly (rebuilt postimage re-anchored to the live wall; banners marked REBUILT).
4. **Re-anchor on rebuild, never trust the authored wall.** A rebuilt band lands on the **live** wall of the tree that ships, not the wall the envelope was authored against (this run: authored vs origin/master@3e90993 md5 1d59adda/392 rows; the worktree wall was 60d023e3/388 rows — one rider's row didn't even exist on the shipping lineage). Grep every rider anchor on the live wall before inserting; fold orphaned riders into the banner with their destination disclosed.
5. **Bank the preimage and the deterministic builder.** Preimage md5 + an assertion-heavy build script (anchors pinned to line numbers, census assertions after) is what makes a rebuild *verifiable* rather than *remembered*.

## Detection checklist (do this at every phase start)

- `ls /home/agent/.hermes/conductor-delegate-spool/delegate/ | grep <run's attempts>` — census which scratch dirs still exist before planning to consume one.
- If a prior-phase artifact you need is gone: read the typed envelope + event log (both survive), verify identity lineage, and **rebuild from spec** — declaring the rebuild, not re-running the phase.

# Prevention rule: dead-attempt forensics must precede redo, and swept artifacts are recovered from quarantine by md5

- **Class:** process / conductor-delegate phase lifecycle (reaped attempts, reaper-swept evidence, mid-cycle salvage commits)
- **Observed:** 2026-10-14..16, run `7ee18a0dbb6c4dae9ec02a53c373e7ee` (repository-maintenance `d50a9f62a6174a9dbbf73e310c799c67` cycle 3) — **seven of ten phases lost an attempt to a provider-infrastructure reap** (roadmap 829ef1dd @22.8s 429; stewardship 7328d3c4 @0 durable trail; stewardship e52a3094 @delivery-failure; implement 423f79ab @43s 429; targeted_tests 0f8a4be2 @pings-only; full_tests a58d4822 @43s; compound 9d882999 @94s), and **every scratch dir + typed envelope produced before 2026-10-15 was swept** before the compound phase ran.
- **Cost:** absent forensics, each reap would have cost a full phase redo (hours) or — worse — the batch's roadmap seam would have been declared unrecoverable at the commit gate, stranding a validated implementation with no status flip. With forensics, every redo decision took minutes, one reap needed NO redo, and the seam chain was re-proven end-to-end.

## What happened

Three distinct failure shapes, one recipe.

1. **Reap with zero durable trail** (829ef1dd, 7328d3c4, 423f79ab, 0f8a4be2,
   a58d4822, 9d882999): typed result JSON absent; event log = `turn_start` →
   a few progress pings → `session_reaped(failed)`; no scratch dir; worktree
   file mtimes all predate the attempt's window. Verdict: nothing adoptable →
   redo from scratch and declare it.
2. **Reap AFTER durable content landed** (e52a3094, stewardship): the typed
   JSON (18,520 B) and scratch note were written, THEN
   `session_reaped(exit_reason=failed)` + `delegate_turn_completed(status=failed)`
   fired. A "failed" phase status pointed at a complete, valid result. The
   next attempt re-verified the content against the surviving artifacts and
   re-established the request instead of redoing it.
3. **Successful phases, swept evidence** (research 81a17dff, roadmap 5d5d680d,
   prioritize d8c9212f, stewardship e5f05695, implement aa1aa3d9): spool
   scratch dirs and typed envelopes vanished later in the run; `/tmp`
   mirrors were swept within the hour and `/home/agent/.cache` mirrors were
   swept same-day — `.cache` is NOT reaper-proof. Recovery came exclusively
   from the reaper's own quarantine, `quarantine/scratches/<ts>/<attempt>/`,
   plus md5s recorded in the work-order/evidence trail: the roadmap band
   patch (md5 34a4c9b7…) was recovered from
   `quarantine/scratches/20261014T171541Z/5d5d680d…/ROADMAP.patch` and
   round-tripped — wall export (md5 1d59adda) + patch → postimage md5
   25d357453e4bef61229b70e4a9e9dabb, 393 rows — byte-exactly matching the
   value recorded by the phase that produced it.

A fourth trap rode along: **conductor-salvage commits landed mid-cycle**
(c45d5e1/1d51188) carrying 5 of the 10 batch surfaces into HEAD while the
other 5 stayed porcelain. Phases that assumed "implement porcelain == the
batch" would have mis-censused the tree; targeted/full both re-derived the
5+5 split instead of trusting either side alone.

## Prevention rules

1. **Run the forensics quartet before redoing any dead phase**, in order:
   typed envelope (`delegate/<attempt>.json`), event-log tail
   (`events/<attempt>.jsonl` — look for a payload write BEFORE the reap),
   scratch-dir census, and worktree file-mtime window. An absent envelope is
   not evidence no work happened; a present envelope is not proof it is valid.
2. **Adopt only after re-verification against THIS work order** — identity
   (run/action/attempt lineage), tree census, cleanliness at dispatch. e52a3094
   was adopted because its content was re-read and matched the batch record;
   a name-match alone would not suffice.
3. **Record an md5 for every artifact a later phase or the commit gate must
   consume** (patches, postimages, memos). When the reaper strikes, the md5 is
   what turns a quarantine find into proof rather than a guess.
4. **Recover from `quarantine/scratches/<ts>/<attempt>/` first**; treat `/tmp`
   and `/home/agent/.cache` as best-effort only. Mirror anything load-bearing
   to at least two of {spool scratch, quarantine (automatic), `.cache`, `/tmp`}.
5. **Round-trip every recovered artifact before use**: regenerate the expected
   output from pristine inputs + the recovered patch and compare md5 against
   the recorded value. A quarantined file whose md5 does not match the record
   is a different artifact — do not build on it.
6. **Census HEAD vs porcelain at every phase boundary** when salvage commits
   are possible: the batch is "what HEAD + porcelain together carry", never
   what either side alone shows.
7. **Declare the disposition in the result** ("prior attempt X: nothing
   durable, REDONE" / "adopted after verification"), so the next reader never
   re-litigates the same reap.

## Related

- The batch this run carried to completion on top of these rules: rm-1426,
  see `docs/decisions/2026-10-16-cycle-3-batch-selection-run-7ee18a0d.md`.
- Sister rule for the validation side:
  `docs/solutions/workflow-issues/prove-tmpdir-is-outside-any-git-repo-before-trusting-a-battery.md`.
- ROADMAP riders recording the same events: rm-241 (TMPDIR class), the
  cycle-3 compound banner (run 7ee18a0d).

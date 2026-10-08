# Prevention rule: provider-reaped attempts leave adoptable work and live processes

- **Class:** workflow / delegate-attempt forensics (hermes-conductor provider reaps mid-turn)
- **Observed:** 2026-10-07/08, run `32f3b7a1` (repository-maintenance `205f84bb` cycle 3) — implement
  attempt `a6f1e2c7` (named, no-op), full_tests attempt `b66edd79`, compound attempt `7597ef65`;
  the adoptable author was `812b2fd5` on the same run/action lineage
- **Cost if ignored:** re-implementing an already-complete batch from scratch (hours), and/or
  re-running a validation battery that a surviving detached process is still writing into —
  while a second, no-op dead attempt is mistaken for "the work never happened"

## What happened

Four delegate attempts on this run were reaped for provider failures. Three carried no typed
result envelope — and the envelope is the ONLY thing a reaped attempt is guaranteed to lose:

1. The implement work order named dead attempt `a6f1e2c7` (2 messages / 31s — nothing). The real
   author was sibling attempt `812b2fd5` on the same run/action lineage: 22 messages, reaped
   minutes after writing the ENTIRE batch into the worktree (unexplained porcelain). Adopted
   after hunk-by-hunk verification against the stewardship contract — then still needed four
   completion fixes (a test appended outside `mod tests`, two overflowing i32 literals, a fmt
   nit, a schema-version sweep) before any suite ran.
2. The full_tests attempt `b66edd79` launched its 22-lane mirror battery via `setsid` and was
   reaped 11 minutes later. The battery kept running: lanes 07–21 logged mtimes five minutes
   AFTER the reap, all green, machine-sums recomputable from the raw logs. The phase adopted it
   on five verification legs and added a fresh re-execution with identical test-binary hashes.
3. The compound attempt `7597ef65` (2 messages / 31s) was provably a no-op only because the tree
   was checked: the worktree diff sha256 and porcelain were byte-identical to the previous
   phase's records.

## Prevention rules

1. **Sweep the lineage, not the named attempt.** Forensics = every `events/*.jsonl` whose run and
   action match (turn counts + timestamps), every scratch dir under those attempt ids, AND the
   tree itself (unexplained `git status` entries are someone's completed work). A work order
   names the attempt whose envelope is missing — not necessarily the attempt that did the work.
2. **Adopt only against a contract, and always complete the adoption.** Verify the candidate delta
   hunk-by-hunk against the authoritative phase contract (stewardship request / selection
   dossier), then run the focused compile+test pass: adopted work is author-complete, not
   gate-complete (four fixes were needed here). Record both the adoption and the completion as
   separate evidence legs.
3. **Check for surviving detached processes before re-running.** A reaped session does not reap
   its children. `pgrep -af` for the battery command; check lane-results/log mtimes under the
   dead attempt's scratch — mtime-after-reap IS proof of session-independent completion and the
   cheapest adoption evidence there is.
4. **Prove the tree byte-stable before adopting validation evidence** (`git status --porcelain`
   census + `git diff | sha256sum` against the prior phase's recorded values). When the tree is
   byte-stable and the battery's raw logs recompute, adoption + ONE fresh re-execution of the
   authority command (CI step verbatim) with identical binary hashes beats either alone.
5. **Declare a no-op as a no-op, with proof.** "No typed envelope" is not evidence of no work;
   neither is a 2-message event log. The dispositive check for a dead compound/docs phase is the
   tree itself: diff-sha + porcelain identical to the prior phase ⇒ nothing to adopt ⇒ redo and
   say so.

## Related

- `docs/stewardship/2026-10-08-cycle3-compound-record-run32f3b7a1.md` (PR-1/2/3/6 carry these rules
  in the cycle record; the four-attempt forensics table is there)
- Sibling convention (unlanded elsewhere): "provider-reaped attempt census-match" — census MATCH ⇒
  redo from scratch, MISMATCH ⇒ verify-then-adopt; this doc is the complementary angle (the trail
  SURVIVES the reap: tree work and live processes both)
- `docs/solutions/workflow-issues/schema-version-bumps-require-planted-fixture-sweep.md` (the
  schema-sweep completion fix that PR-3's example names)

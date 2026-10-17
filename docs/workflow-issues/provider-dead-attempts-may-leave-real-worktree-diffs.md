# Provider-dead attempts may leave real worktree diffs — adopt by hypothesis, then re-verify

**Class:** workflow-issue (fleet operations) · **Minted:** 2026-10-11, run e4eb22544532 cycle 1 compound (attempt c8d175d0) · **Evidence:** forensics from implement 3b613f9a and full_tests 032021cd this run

## The observed pattern (twice in one cycle)

- **implement (dead attempt 36cb9268):** typed artifact absent; event log =
  dispatch + `exit(provider_error)`. The worktree nonetheless carried a real 4-file
  diff on exactly the batch surfaces. The successor attempt adopted it as a
  hypothesis and completed it — which required fixing 2 cargo-fmt drift spots and
  replacing an under-shaped test fixture the dead attempt had left.
- **full_tests (dead attempt 8b753a36):** no typed result, `session_reaped`
  exit_reason=failed ~22 s in, **zero** tool work — nothing to adopt.

## The rule

A dead attempt's event log says nothing about the worktree. Before either trusting
or discarding a pre-existing diff:

1. **Forensics first:** event log + typed artifacts vs `git status`/`git diff` in
   the worktree. Zero tool calls ⇒ nothing was done; real calls ⇒ real diff possible.
2. **Adopt by hypothesis, never by faith:** treat the found diff as an unverified
   claim. Re-derive every anchor against the live tree, re-check markers, and
   complete the missing pieces (fmt, fixture shape, coverage).
3. **Prove the red arm yourself:** pristine-file swap → focused test fails →
   md5-verified restore → green. This converts "the diff seems to fix it" into
   evidence, and catches dead-attempt half-work (the fmt drift and fixture shape
   above were exactly that).
4. **Record the adoption** in the phase result: which parts were inherited, which
   replaced, and why.

The inverse trap is equally real: reverting a dead attempt's diff on the assumption
that a failed session produced nothing would have destroyed correct work here.

Related fleet case: the marker re-verification discipline (re-check fix markers
before every build/test leg — provider resets can silently revert working-tree
state mid-phase).

## 2026-10-15 extension — five deaths in one cycle + the reaped-scratch class (run 1a4cce3a cycle 3)

Same-day instantiation of the class above, five times inside one maintenance cycle (all
provider family), with one live adoption:

| phase | dead attempt | died at | durable trail | successor action |
|---|---|---|---|---|
| prioritize | 19ea1660 | message_count 3 | event log only (4 lines) | redone from scratch (fcb4b749) |
| stewardship | 81482577 | message_count 2 (~23 s) | event log only | redone from scratch (02059d97) |
| implement | 0fdf4495 | mid-batch | WORKTREE DIRTY DIFF ONLY (5-file delta, 2.1 h mtime-stale, zero untracked) | ADOPTED after line-by-line audit against the stewardship spec (1013d2a8) |
| full_tests | ccc865cc | message_count 2 | none | redone from scratch (b2eace9d) |
| compound | 85c992f3 | message_count 15 | results tombstone only (status=failed, 309 bytes, no payload) | redone from scratch (5b96b1c1) |

New subclass this cycle exposes — **REAPED SCRATCH** (not covered by the 2026-10-13 rules
above): the conductor janitor reaps `*-scratch/` directories mid-cycle, BEFORE the successor
phase runs. This cycle the roadmap phase's spool postimage+patch (feae96d5) were gone by
compound time, and a sibling run's compound scratch was gone the same way. Rules that follow:

1. The phase ENVELOPE (`results/<attempt>.json`, or `delegate/<attempt>.json` when typed) is
   the only surviving authority for a reaped band — rebuild from it, never from memory, and
   DECLARE the rebuild in the band itself (this cycle's roadmap banner says so explicitly).
2. Do not read a present `results/<attempt>.json` as evidence that work happened: dead
   attempts get a FAILED TOMBSTONE there (this cycle: 309 bytes, empty summary, no artifacts).
   A worktree census decides, not a file's existence.
3. A rebuilt band cannot be byte-identical to the reaped postimage; verification is
   subject-by-subject (every mint's track/priority/anchor, every rider's owner row) plus the
   wall invariants (def-row count, zero duplicate ids, base-line survival, managed render
   footer last).
4. Mirrors under `/home/agent/.cache/at-*` are ALSO reaped (two observed gone this cycle) —
   the worktree dirty diff remains the most reaping-resistant channel, which is why this run
   lands its compound band IN-TREE (fleet precedent ff7615be) with a spool patch as the
   secondary seam.

Post-script, same day (re-delivery attempt 31a41503): the cycle's SIXTH death — compound
`5b96b1c1` itself — exposes the INVERSE of rule 2. It was reaped ("This operation was
aborted") AFTER completing every durable write (scratch, in-tree band, docs, typed
phase_result at 06:41Z, results entry 06:42Z), so a COMPLETE, valid typed artifact sat
inside an engine-FAILED attempt and the phase was re-dispatched.

5. When a re-dispatched successor finds a complete typed artifact + worktree products from
   the dead attempt, ADOPT: re-verify every pinned invariant against the dead attempt's own
   records (postimage md5, batch diff md5, def-row census, append-only diffs, porcelain
   census) and re-emit the typed result under the SUCCESSOR's attempt id. A redo discards
   verified work and re-rolls the provider dice for nothing. The adopt-vs-redo decision is
   made on the DURABLE TRAIL (typed artifact / worktree diff / scratch), never on the
   engine's failed mark — that mark only says the session died, not that the work did.

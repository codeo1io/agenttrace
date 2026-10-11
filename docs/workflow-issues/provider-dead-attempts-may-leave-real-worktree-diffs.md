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

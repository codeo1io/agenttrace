---
title: "One tree, two digests: reconcile the base commit and surface coverage before declaring a validation outcome"
date: 2026-10-01
category: process-issues
module: repository-governance
problem_type: process_gap
component: docs/stewardship
symptoms:
  - "Every local gate envelope recorded a digest different from the dispatching work order's digest for the same unchanged tree"
  - "A fix made only of test code under crates/ left the declared digest byte-identical, which reads as a stale declaration"
  - "Copying the envelope digest into a phase result invites a fold-gate rejection for a digest that was never stale"
root_cause: wrong_assumption
resolution_type: doc_change
severity: medium
tags:
  - governance
  - validation
  - truthfulness
  - campaigns
problem_summary: "A validation digest is not one value but a family — the base commit and a surface-coverage allowlist both fold in — so two different digests can both be correct for one tree, and digest inequality alone proves nothing about staleness."
applies_when:
  - "Any validation phase that must copy a dispatch digest verbatim, or re-declare it after edits"
  - "Reconciling local gate-envelope digests against a dispatching work order"
  - "Deciding whether a change is required to move a validation digest"
---

## Problem

During repository-maintenance cycle 1 (2026-10-01), every full- and
targeted-validation run on one unchanged tree produced two different
validation digests: the dispatching work order carried one value and every
local gate envelope carried another — five envelopes observed across the
day, all mutually identical and none matching the work order. Both values
were correct. The digest folds two degrees of freedom that are easy to
conflate:

1. **Base commit.** The digest folds the base commit it is derived against.
   The dispatching engine derives with the dispatch base commit; the local
   gate envelope cannot know that base and records its own derivation
   against an unknown base. Same tree, same surfaces, different fold input,
   different digest — always.
2. **Surface coverage.** The digest hashes only surfaces the validation
   policy classifies as executable — the policy’s classification prefixes
   plus the exact build files. Of those prefixes, `scripts/` and
   `.github/workflows/` are real, digest-visible roots in this repository;
   `src/`, `lib/`, `tests/`, and `bench/` are prefix names from the
   validation policy and do not exist as top-level roots here. Rust
   sources under `crates/` are outside the set entirely, so a change
   confined to `crates/` leaves the digest byte-identical by design. A
   change to a visible surface — for example a CI gate script under
   `scripts/` — DOES move the digest, so a campaign whose later phases
   edit such a script will see different dispatch digests across that
   edit; one whose every delta stays inside `crates/` carries a single
   digest across every phase and every fix.

## Symptoms

- Envelope digest and work-order digest disagree on every run, inviting the
  conclusion that the tree, the envelope, or the declaration is wrong.
- A test-only fix under `crates/` does not move the digest, which reads as
  "the declaration is stale" when it is exactly current.
- The emission-time rule — re-run and declare the digest as printed now, if
  you changed an executable surface — looks unfollowable when nothing you
  changed is able to move the digest.

## What Didn't Work

- Guessing which of the two values to declare, or hedging by quoting both.
- Treating digest inequality between two artifacts as evidence of staleness
  or tampering; here the difference only ever encoded the base commit.
- Re-running the full suite expecting the digest to move after a
  crates-only fix: three green runs in one day, identical digest each time.

## Solution

Reproduce the derivation before declaring anything (this cycle's
full-validation phase):

1. Import the validation policy module from the running conductor release
   and call its digest function directly on the repository root twice —
   once with the dispatch base commit, once with an unknown base:
   `python3 -c "import sys; sys.path.insert(0, '<conductor-release>/src'); from hermes_conductor import validation_policy; print(validation_policy.validation_digest('<base-sha-or-None>', '<repo-root>'))"`.
2. Confirm the base-commit value equals the work order's digest verbatim,
   and the unknown-base value equals the envelope digests — before and
   after this session's changes.
3. Declare the base-commit value, and say which base it was derived against.
   Judge freshness by re-derivation, never by comparison against envelope
   digests: envelopes are bookkeeping for the local run, not the declaration
   authority.
4. When a change must be digest-visible it has to land on a
   policy-executable surface (scripts, workflows, build files); crates-only
   deltas are evidenced by gate envelopes and reconciled test counts, not
   by digest movement.

## Why This Works

- Both digests are reproducible on demand from the same inputs the engine
  and the gate use, so the reconciliation is self-auditing rather than one
  more assumption copied forward.
- Deriving instead of guessing converts a fold-gate rejection risk into a
  two-command check performed once, before the result is written.
- Test-count reconciliation (pre-batch baseline plus batch tests) supplies
  the outcome evidence that digest movement cannot supply for crates-only
  changes.

## Prevention

- Never declare a digest that was not re-derived this session from the
  current tree with a known base commit.
- Treat any unknown-base digest in a local artifact as run bookkeeping,
  never as a declared value.
- Record in the phase result which base the declared digest was derived
  against, so a reviewer or the fold gate can reproduce it.
- Before concluding "stale declaration", diff the base commits and the
  changed-surface classification first; only then spend a re-run.

## Related Issues

- Full-validation outcome of repository-maintenance cycle 1 (2026-10-01):
  verbatim full-suite gate run green at 279 passed / 0 failed, with the
  declared digest re-derived byte-exact before and after a crates-only fix.
- docs/solutions/process-issues/2026-09-30-unverified-assumption-compounded-across-governance-artifacts.md
  — the same rule on its other leg: verify governance claims live instead
  of copying them between artifacts; this document is the digest-specific
  instance of that rule.

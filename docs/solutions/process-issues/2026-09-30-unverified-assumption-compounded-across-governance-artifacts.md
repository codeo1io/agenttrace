---
title: "An unverified privacy assumption compounded across three shipped governance artifacts"
date: 2026-09-30
category: process-issues
module: repository-governance
problem_type: process_gap
component: deny.toml
symptoms:
  - "Three shipped artifacts stated the fork's GitHub remote is private"
  - "A security-hygiene lane (OpenSSF Scorecard) was deferred on that premise"
  - "An unpinned runner in the token-bearing release lane survived review"
root_cause: wrong_assumption
resolution_type: doc_change
severity: medium
tags:
  - governance
  - truthfulness
  - supply-chain
  - scorecard
---

## Problem

During the 2026-09-30 adversarial assessment of this tree, one environmental
claim — "the fork's GitHub remote is private" — was found asserted as fact in
three shipped locations, while the live remote was public
(`gh repo view codeo1io/agenttrace --json visibility,isPrivate` returned
`{"isPrivate":false,"visibility":"PUBLIC"}` on 2026-09-30):

1. the deny.toml header comment explaining why OpenSSF Scorecard is absent (deny.toml:6),
2. the rm-010 roadmap note recording the Scorecard deferral rationale (ROADMAP.md:42),
3. the Prevention section of the RustSec lockfile solutions document (docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md:55).

None of the three cited a verification command or a date, so nothing in the
repository could distinguish a verified fact from a copied assumption.

## Symptoms

- A governance decision (deferring Scorecard) rested on an environmental fact
  that had drifted false with no falsification alarm anywhere in CI.
- The prose carried confidence it had not earned: the claim read as
  justification in each location, so each new artifact that quoted it gained
  apparent authority from the previous one.
- Review effort had to be spent re-deriving ground truth per artifact instead
  of trusting the recorded rationale.

## What Didn't Work

- Copying a rationale between artifacts without its evidence. The claim
  compounded precisely because each copy looked well-sourced.
- Relying on prose at all for properties a machine can check. "The remote is
  private" is one `gh` call; "every workflow cargo invocation is --locked" is
  one grep; neither belonged in comments alone.
- Silent rewrites as the correction path — replacing the false text would have
  erased the decision history that explains why the deferral happened.

## Solution

Treat recorded environmental facts as claims with provenance (this cycle,
repository-maintenance cycle 1):

1. **Verify live before recording.** The remote's visibility was re-derived
   from `gh` at assessment time and treated as the only authority; the three
   prose claims were demoted to corrected records.
2. **Correct by dated append, not rewrite.** Each location now carries a dated
   correction stating the verified reality and the verification method,
   preserving the original decision context. The roadmap's rm-010 note shows
   the pattern: the cycle-1 deferral stays, followed by a cycle-2 correction
   (ROADMAP.md:42).
3. **Move checkable properties into gates.** The claims this incident
   incubated became enforced surfaces: an OpenSSF Scorecard workflow now
   exists (`.github/workflows/scorecard.yml`), and the lockfile posture the
   solutions document described is now mechanically asserted by
   `scripts/ci/check-locked-cargo.sh` in the lint and full CI lanes.

## Why This Works

- A `gh` query is re-runnable by anyone at any time; an unsourced comment is
  not. Citing the command makes the claim self-auditing.
- Dated corrections keep the falsified rationale visible, so the next reader
  learns both the decision and its correction instead of re-deriving them.
- Gates remove the class of drift entirely: a checkable property enforced in
  CI cannot quietly become false, which is the property prose lacks.

## Prevention

- Never record an environmental fact (remote visibility, registry state,
  version availability) without the exact command that verified it and the
  date, inline at the claim site.
- When a deferral rests on an environmental claim, the deferral must state its
  own falsification condition — "revisit when the repository becomes public"
  existed here (docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md:55)
  and is the part of the original record that worked as designed.
- Prefer a gate over a comment whenever the property is mechanically
  checkable; reserve prose for properties only a human can judge.
- Assessment phases should re-verify quoted environmental claims against live
  state rather than treating prior artifacts as evidence.

## Related Issues

- Assessment finding AF-1 of run 30484632: the three-location discovery with
  the live `gh` evidence.
- Roadmap items rm-155 (truth-correction + Scorecard enablement) and rm-157
  (lock-enforcement gate), implemented in repository-maintenance cycle 1 of
  this campaign pending the commit gate.

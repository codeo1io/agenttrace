---
title: Workflow comments must state coverage that grep can prove
date: 2026-09-30
category: ci-hygiene
module: agenttrace-ci
problem_type: stale-automation-comments
component: .github/workflows/dependency-review.yml
severity: low
tags: [ci, documentation, dependency-review, truthfulness]
related_issues:
  - ROADMAP rm-168 (fixed in cycle 1, run c7dca75d30e3)
  - ROADMAP rm-170 (fixed in cycle 1, run c7dca75d30e3)
---

## Problem

A scheduled dependency-review workflow opened with a comment claiming it ran
"cargo audit + cargo deny" over the lockfile. The repository's CI only ever
invoked cargo deny. The comment also carried two pull_request-event guard arms
in a workflow that is workflow_dispatch-only, so the guarded branches were
unreachable by construction.

A reader auditing supply-chain coverage believes two scanners are wired when
one is. That is a truthfulness defect with the same shape as a lying
changelog entry: the artifact states coverage the machine never provides.

## Symptoms

- The claimed command appears in prose but not in the workflow's run steps
  (grep the workflow and its called scripts before believing any comment).
- Guard conditions reference events the workflow cannot receive
  (check the on: block before writing or trusting condition arms).
- Fixing the comment changes no behavior — and that is exactly why nobody
  notices when it drifts.

## What Didn't Work

- Leaving the comment approximate because "everyone knows what it means."
  Eight weeks later nobody knows, and the audit-trust chain silently shrinks.

## Solution

Two edits, both applied uncommitted in worktree run-c7dca75d30e3-c7dca75d
(cycle 1, 2026-09-30):

- The comment now states the actual coverage: ci.yml runs cargo-deny only;
  cargo audit is not wired anywhere in the repository (grep-verified across
  .github/workflows and scripts).
- The dead guard arms were deleted rather than documented — in a
  workflow_dispatch-only workflow the conditions could never vary, so the
  arms were noise claiming to be logic.

## Why This Works

Truthfulness is checkable mechanically: every coverage claim in a comment
must be greppable to the step that executes it. When the claim and the
grep disagree, the claim loses. Deleting unreachable arms removes the
opportunity for the disagreement to exist at all.

## Prevention

- Never write a tool name in an automation comment without pasting the
  command from the actual run step.
- Audit workflow on: blocks before adding event-guarded branches; a guard
  arm for an event the workflow cannot receive is dead code wearing a
  purposeful expression.
- The sibling item in this cycle — the lint tar-cache churn (369 MB re-tarred
  on every run, and a corrupt partial tar previously hard-failed the
  restore) — shares the prevention: CI behavior must be derived from what
  the steps do, not what the comments say they do. The restore now removes
  a corrupt cache and rebuilds cold, and the save step skips when no target
  file is newer than the cached tar.

## Related Issues

- ROADMAP rm-168 and rm-170 — both flipped to done in cycle 1
  (2026-09-30, run c7dca75d30e3), validated by the cycle's targeted and
  full suites.
- docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md
  — same class: a gate believed to cover something it did not.

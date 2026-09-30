---
title: Vacuous parity gates — version gates must pass on all three anchors
date: 2026-09-30
category: quality-gates
module: scripts/ci/check-plugin-version.sh
problem_type: vacuous_validation
component: release-gate
symptoms:
  - CI gate green while the shipped artifact drifts from the release channel
root_cause: gate compares two independently-stale numbers
resolution_type: three-anchor gate plus red-path proof
severity: medium
tags: [release-parity, ci-gates, codex-plugin, changelog]
---

# Vacuous parity gates — version gates must pass on all three anchors

## Problem

`scripts/ci/check-plugin-version.sh` guarded the Codex plugin manifest's version by
comparing it to the CHANGELOG's latest heading — and only that. Over three releases
(v0.8.0 → v0.9.0) neither the manifest (stuck at 0.7.1, last touched e005952 on
2026-08-22) nor the CHANGELOG (latest heading still v0.7.1) had been updated, so the
gate faithfully compared two identical stale numbers and passed on every run. The
released channel — git tags and the actual v0.9.0 artifacts — was invisible to it.

## Symptoms

- `plugin.json` reported 0.7.1 while `git tag` reached v0.9.0 and the npm package
  published 0.9.0.
- CHANGELOG had no entries for v0.8.0, v0.8.1, or v0.9.0.
- The existing CI gate was green throughout.

## What Didn't Work

- Two-anchor comparison (manifest ↔ CHANGELOG): both anchors are hand-maintained, so
  they can rot together. Any gate whose inputs are updated by the same human error is
  a vacuous gate, not a gate.

## Solution

The gate now requires three independent anchors to agree:

1. the manifest version (`plugin.json`),
2. the newest CHANGELOG `## v` heading, and
3. the latest version tag from `git tag --list` matching `^v[0-9]+\.[0-9]+\.[0-9]+$`.

Anchor 3 is filtered to strict semver because this repository also carries non-release
housekeeping tags that sort above version tags. In addition, every `./`-prefixed path
in the manifest must resolve relative to the repository root — composerIcon,
logo, and screenshot references — so dangling asset references fail the gate rather
than shipping.

## Why This Works

The git tag is written by the release act itself, not by the same hand that edits the
manifest and CHANGELOG; tying the gate to it breaks the shared-staleness failure mode.
Path resolution checks make the manifest's declared surface self-verifying.

## Prevention

- Any parity/truthfulness gate must include at least one anchor produced by the
  process being gated (tag, release artifact, published package), never only
  hand-maintained files.
- Prove gates red: before landing a new or hardened gate, run it against the known-bad
  state (here: `git stash` the manifest/CHANGELOG fix, run, observe exit 1 on the tag
  mismatch, unstash). A gate whose red path has never been observed is unverified.
- Prefer many narrow checks over one broad one: the tag check would not have caught
  the dangling asset paths, and the path check would not have caught version drift.

## Related Issues

- ROADMAP `rm-014` (Codex plugin manifest release gate) — implemented in the same
  change; see the ROADMAP cycle-1 annotation.
- Lesson on workflow-comment truthfulness
  (`docs/solutions/ci-hygiene/workflow-comments-must-state-coverage-grep-can-prove.md`) —
  same defect family: claims in channel metadata drifting from reality.
- Upstream #291/#292 (merged 2026-09-30) grew release-channel automation while the
  fork shipped an unverified manifest — the drift risk is live upstream too.

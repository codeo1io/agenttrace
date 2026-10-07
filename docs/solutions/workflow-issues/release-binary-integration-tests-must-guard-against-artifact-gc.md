---
title: "Release-binary integration tests must guard against artifact GC"
date: 2026-10-07
category: workflow-issues
module: "agenttrace-core integration test harness"
problem_type: workflow_issue
component: testing_framework
severity: medium
applies_when:
  - "An integration test locates a compiled binary under target/ (e.g. env var or CARGO_BIN_EXE-style resolution) and spawns it with Command::new"
  - "Validation phases run in separate turns hours apart over the same worktree (targeted tests, then a full-suite mirror)"
  - "A full-suite lane rebuilds artifacts mid-run, so earlier lanes and later lanes see different target/ states"
resolution_type: workflow_improvement
tags: [integration-tests, release-binary, artifact-gc, cargo, ci-mirror, validation-record, flaky-infra]
---

# Release-binary integration tests must guard against artifact GC

## Context

agenttrace's subagent-attribution regression suite (rm-545) verifies the real
CLI end to end: `tests/subagent_attribution.rs` resolves the release binary
under `target/release/` and spawns it over a fixture corpus, so the test
exercises the same parse → attribute → render pipeline a user gets. That makes
the test's availability depend on a build artifact, not just on source.

## What happened

During repository-maintenance cycle 3 (2026-10-07, run c762c2b8), the full-suite
mirror (22 lanes, ci.yml `full`+`deny` jobs lane-for-lane) ran hours after the
targeted-tests phase in the same worktree. Pass 1 aborted in lane 03 with

```
panic: 'spawn release binary: NotFound'  (subagent_attribution.rs:146)
```

while the identical suite had been green at targeted time. Forensics: the
binary existed at 19:30Z (targeted-tests build), `target/release` held zero
entries at 21:00Z, and pass 1's own build lane (lane 04, 836s) had rebuilt it
by the time the failure was inspected. Cargo's artifact garbage collection had
evicted the release binary between phases — an infrastructure absence
masquerading as a code failure inside a validation record.

## Root cause

The test treated `target/release/agenttrace` as always-present because the
suite that compiles tests also builds binaries. That invariant holds inside a
single `cargo test` invocation but NOT across invocations sharing a worktree:
separate cargo runs (different profiles/locks, GC pressure) can evict artifacts
between them, and conductor-style phased validation runs exactly that pattern —
hours-apart turns over one `target/` directory.

## Guidance

- Any test that spawns a located binary resolves the path defensively: check
  existence (and, where cheap, executability) before `Command::new`, and fail
  with an actionable message — or rebuild/skip with an explicit recorded
  reason — instead of letting `Command::new` panic on `NotFound`.
- The landed form (initial guard + review fix a8dfb757): a `bin.exists()` guard before
  `Command::new(bin)` in `tests/subagent_attribution.rs` skips ONLY on an absent binary
  (reason on stderr — visible under `cargo test -- --nocapture`), while a PRESENT binary
  that exits non-zero panics with status + stderr. An artifact gap skips; a regression
  fails — never let one leg cover both.
- When a full-suite mirror fails on ONE lane with a not-found/missing-artifact
  signature while every other lane is green, check `target/` state and recent
  builds BEFORE triaging code: `ls target/release`, artifact mtimes, and which
  lanes build what. Re-run after the failing lane's own build step has run.
- A pass-1 red of this class is a robustness fix, not a band regression: prove
  it by rebuilding and re-running the focused suite (here 4/0), then the whole
  mirror (pass 2: 22/22 rc0) — and record the fix inside the band so the
  evidence contract stays honest.

## When to Apply

- Adding or reviewing any integration test that resolves and spawns a binary
  from `target/` rather than using `CARGO_BIN_EXE_<name>` (which cargo
  guarantees for the invocation's own build).
- Diagnosing a validation record whose failure signature is
  NotFound/No such file on an artifact path, especially across
  phase/turn boundaries or CI lanes that build at different times.

## Related

- docs/stewardship/2026-10-07-cycle3-compound-record-runc762c2b8.md (PR-2; full
  pass-1/pass-2 forensics)
- docs/solutions/workflow-issues/validate-probe-corpus-before-trusting-failure-signals.md
  (same class: distrust a surprising failure signal until the harness side is
  ruled out)
- ROADMAP.md rm-545 EXECUTED bullet (the band the fix rode in on)

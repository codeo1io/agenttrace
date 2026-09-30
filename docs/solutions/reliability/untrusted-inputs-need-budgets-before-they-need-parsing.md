---
title: Untrusted inputs need budgets before they need parsing
date: 2026-09-30
category: reliability
module: agenttrace-core (parser.rs token accounting, pricing.rs catalog fetch)
problem_type: input_validation
component: core
symptoms:
  - "codex_token_count_usage summed output+reasoning token counts with plain + while three sibling sums in the same file already used saturating_add — adversarial i64-scale journal magnitudes panic in debug and wrap negative in release"
  - "download_pricing read the catalog response with unbounded into_string() — a hostile or misbehaving mirror could exhaust memory before parsing even began"
  - "The pricing snapshot carried no fetched-at/source stamp, so a stale or substituted catalog was indistinguishable from a fresh one"
root_cause: missing_bounds
resolution_type: defensive_hardening
severity: high
tags: [untrusted-input, saturating-arithmetic, denial-of-service, pricing, codex, provenance]
---

# Untrusted inputs need budgets before they need parsing

## Problem

Two surfaces accepted externally-controlled magnitudes without budgets. First, token accounting: journal token counts are attacker-influenced integers (an adversarial or corrupted session journal can carry i64-scale numbers), and `codex_token_count_usage` combined them with `+`. In debug builds that panics on overflow; in release it wraps to negative totals, corrupting every downstream cost figure. The codebase already knew the answer — three sibling accumulations in parser.rs used `saturating_add` — but knowing it in three places did not propagate to the fourth.

Second, the pricing fetch: the response body was read into a String with no size bound and no timeout policy at the read boundary, so the failure mode preceded parsing entirely — memory exhaustion from a hostile mirror.

## Symptoms

- Plain `+` at parser.rs codex_token_count_usage while saturating_add sat three functions away.
- Unbounded `into_string()` on an HTTP response body.
- Cost reports derivable from wrapped-negative totals with no error anywhere.

## What Didn't Work

- Convention-by-precedent: the correct pattern existing in the same file did not prevent the new violation; only a check (or a habit of grepping for the bare operator) catches it.
- Trusting the transport: any fetch whose body size is bounded only by the peer's goodwill is a DoS vector regardless of TLS.

## Solution

Implemented in cycle 1 (worktree run-c7dca75d30e3, uncommitted for the commit gate):

1. Token arithmetic: the bare `+` became `saturating_add`, matching the file's own precedent; the upstream #286 `token_usage_high_water` rewind logic was ported so compaction-rewind no longer double-counts. Regression tests feed i64::MAX-scale magnitudes and a rewind fixture.
2. Fetch budget: `download_pricing` streams via `into_reader().take(cap + 1)` with `PRICING_DOWNLOAD_MAX_BYTES = 32 MiB`, rejecting over-cap responses with an explicit error (the +1 makes exactly-at-cap distinguishable from over-cap), UTF-8-validated before parse.
3. Provenance: the snapshot writer stamps a `.meta.json` sidecar (url, fetched_at_unix, bytes), made path-injectable for testing.

## Why This Works

Saturating arithmetic converts corruption into a clamped, visible, testable bound instead of a sign flip. `take(cap+1)` bounds memory before any allocation proportional to the response. Provenance stamps make staleness observable — a cache you cannot date is a cache you cannot trust. Bounding at the read boundary means the parse layer never has to be trusted to be careful.

## Prevention

- When summing externally-derived integers, grep the file for the arithmetic pattern first — the codebase usually already contains the safe form; using it is free.
- Every network read gets a byte cap at the reader, not a hope at the parser; fail closed to the bundled snapshot with a warning.
- Stamp fetched data with source and time at write time; retrofitting provenance onto an existing cache is guesswork.
- Adversarial-magnitude tests (i64::MAX operands) are one-line fixtures; add them wherever totals feed costs.

## Related Issues

- ROADMAP.md rm-012 (arithmetic + rewind) and rm-018 (fetch guardrails) — the change units this learning documents.
- Upstream PR #286 — the rewind fix whose minimal parser.rs hunk was ported.
- docs/solutions/quality-gates/vacuous-parity-gate-passed-on-two-stale-anchors.md — same campaign's gate-side lesson.

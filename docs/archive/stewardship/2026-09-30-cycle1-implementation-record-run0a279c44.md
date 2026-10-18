---
schema: ce-handoff/v1
created_at: '2026-09-30T11:05:00Z'
title: 'Cycle 1 implementation record — campaign 47e4432e, run 0a279c44'
summary: Implementation and pre-review validation record for cycle-1 batch "Trustworthy token accounting on hostile journals" (rm-046 + rm-047).
keywords: [agenttrace, repository-maintenance, cycle-1, token-accounting, rm-046, rm-047]
cwd: /home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-0a279c440c10-0a279c44
repo_root_sha: 9d88b36750a991bd1436dbbc91b4579c39003067
branch: conductor/run-0a279c440c10
---

# Cycle 1 implementation record — campaign 47e4432e (run 0a279c44)

**Note on the filename:** `docs/stewardship/2026-09-30-cycle1-*` (prioritization, stewardship-request, implementation-record) belongs to the concurrent 88feec46 campaign (base `7bb4dcb`) landed via merge `9d88b36`. This record is this campaign's cycle-1 and carries the run id in its filename to stay collision-free (see `docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md` — the same collision class, different artifact).

## Batch

"Trustworthy token accounting on hostile journals" — `rm-046` (saturate token accumulation) + `rm-047` (codex head-classification rescue). Selected as the only open candidates with live in-tree failing evidence at this head; scoring and rejected alternates in `/tmp/at-c1-prioritization/2026-09-30-cycle1-prioritization.md` (session scratch; the durable copy of decisions is ROADMAP.md's selection notes).

## What shipped (pre-review; uncommitted worktree state at `9d88b36`)

| Item | Files | Status |
|---|---|---|
| rm-046 saturating accumulation + `calculated_from_tokens_clamped` provenance marker | `crates/agenttrace-core/src/parser.rs`, `crates/agenttrace-core/src/lib.rs` | implemented; red→green |
| rm-047 key-boundary head probe + whole-line `token_count` rescue + skip accounting (`metrics.line_skips["codex_ignorable_line"]`) | `crates/agenttrace-core/src/parser.rs` | implemented; red→green |
| Roadmap renumber + cycle-1 candidates `rm-046..rm-054` + selection notes | `ROADMAP.md` | bookkeeping |
| Assessment compounding | `docs/reviews/2026-09-30-adversarial-repository-assessment-pass12.md` | compounded (this phase) |
| Research compounding | `docs/research/2026-09-30-extensions-research-pass10.md` | compounded (this phase) |
| Prevention rule | `docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md` | compounded (this phase) |

## Evidence chain (pre-review)

- **Red:** `cargo test -p agenttrace-core --lib` pre-fix → 99 passed / 4 failed, including the live `attempt to add with overflow` panic at the accumulation site.
- **Green (crate):** 103/0 lib; 7 + 72 integration; 0 doc failures; `cargo fmt --check` clean; `cargo check --workspace --tests` clean.
- **Targeted gate (work-order command, verbatim):** `local_validation_gate.py --shell-command 'cargo test'` → GATE_RC=0, **268 passed / 0 failed / 0 ignored** across 10 suites (envelope `result-430870-326193046.json`).
- **Full gate (work-order command, verbatim):** same command via release `4ad38e1a` → GATE_RC=0, **268 passed / 0 failed / 0 ignored** (envelope `result-493778-326216181.json`). Suite lineage: 264-test baseline at `9d88b36` + 4 batch regression tests.
- **Live both-profile parity:** adversarial opencode fixture (`input: 5 + i64::MAX`): debug no longer panics; debug == release totals; input clamped at `i64::MAX`; `provenance.cost = calculated_from_tokens_clamped` observable via `--sessions -f json`. Codex fixture with marker beyond the 160-byte window: usage rescued (old probe dropped it), `line_skips = {"codex_ignorable_line": 3}`.
- **Validation digest:** dispatch token `validation:v1:155d1e3d82ccd7bc7369ace33562cface32567490ee3cde875121137d0a6c738` declared at both gates and **re-derived byte-exact** at base `9d88b36` (the `.rs`-only batch is digest-invisible by policy; rely on gate envelopes for Rust correctness).
- **Method disclosure:** cross-model adversarial review unavailable on the GLM delegate host (router provider enum excludes the family; refusing false attestation) — adversarial lens ran in-thread, disclosed, with direct file/line evidence and live repros.

## Design decisions worth keeping

1. Skip counting reuses the existing `Metrics.line_skips` map rather than adding a field — zero schema/cache churn, additive-only serialization.
2. Saturation over clamping-at-source: per-message values keep their identity; only aggregation saturates, and the provenance marker tells the user *which* runs were clamped (`calculated_from_tokens_clamped`).
3. Rescue over re-parse for the codex head probe: anchored marker scan stays O(head-window); the whole-line `token_count` rescue runs only when the fast path declines — fail-safe direction is "count and disclose," never "drop silently."

## Context for cycle 2

- **Strategic lead:** `rm-053` conformance harness — its first conformance case IS rm-046's adversarial fixture (already in-tree as a regression test).
- **Clean lead:** `rm-051` install.sh pinned fallback (offline gate `scripts/ci/check-install-runtime.sh` exists; remember the dash-not-bash `sh -n` rule).
- **Anchor:** `rm-020` per-model pricing remains the highest-residual-value item (its stale contention rationale was corrected this cycle in ROADMAP.md).
- **Grounding rule for the next campaign in this repo:** campaign trees live under `~/.hermes/conductor-worktrees/`; `/work/projects/agenttrace` is a shared checkout siblings re-branch mid-run — never ground inventory claims there. The research pass in this cycle had three inventory claims read from the wrong lineage and corrected during the roadmap phase; the correction is recorded in ROADMAP.md's `rm-046+` append comment.

# Cycle-7 stewardship request (phase: stewardship)

Run `4e6ff52433d44aff92a85afa14400a58`, attempt
`f30355f9fdd94b4889e2c2204b9cf059`, 2026-09-21. This document is the
human-readable companion to the structured `stewardship_request` in the
phase result; the JSON fields are authoritative. Inputs: the cycle-7
prioritization record (`2026-09-21-cycle7-prioritization.md`, batch
CU-24..CU-28), the roadmap state after this run's update, the cycle-5
independent review disposition, and live git evidence below. Router
note: the compound-engineering router remains an empty stub in this
environment (consistent with every prior phase this run); work
proceeded directly.

## Campaign repository and worktrees (verified live)

- `git worktree list`: `/work/projects/agenttrace` (main worktree, on
  branch `fix/tui-deadline-test-waits` at `df3b621`) and
  `/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-4e6ff52433d4-4e6ff524`
  (this run's conductor worktree, branch `conductor/run-4e6ff52433d4`,
  also at `df3b621`, **clean**) share one object database — one
  repository, two checkouts. Other listed worktrees
  (`run-125bf93302aa` prunable, `/tmp/ci-fast-lane`, `/tmp/wt-agenttrace-ci`
  prunable, `fork-maintenance-worktrees/deploy` detached) are stale or
  unrelated to this cycle.
- Remotes: `origin` and `fork` both point at
  `git@github.com:codeo1io/agenttrace.git` (the fork). Upstream
  (`luoyuctl/agenttrace`) is not a configured remote; drift was
  measured by URL fetch (merge-base `e005952`, new `a34dea2`/`6848aa1`).
- **Dirty state that must be preserved** (main worktree only): the
  in-flight cycle-5-review remediation batch — `crates/agenttrace-cli/src/main.rs`,
  `crates/agenttrace-cli/tests/entrypoints.rs`,
  `crates/agenttrace-core/src/insights.rs`,
  `crates/agenttrace-core/src/session_cache.rs`,
  `crates/agenttrace-core/tests/discovery_contract.rs`,
  `crates/agenttrace-tui/src/tests.rs` — plus this run's `ROADMAP.md`
  edit and the untracked `docs/stewardship/2026-09-21-cycle7-prioritization.md`.
  Suite verified green in this exact state (`cargo test --workspace
  --quiet`, exit 0, 2026-09-21).

## The request

**Title:** Cycle 7 — land the F5 truth-telling debt and hold parser
parity with upstream (CU-24..CU-28).

**Summary:** One repository, five code change-units plus records. CU-24
lands the already-implemented, suite-green F5-1..F5-5 remediation
(including two test-isolation flake fixes) with its missing roadmap
filing and implementation record — absorbing the pre-existing dirty
state as the first unit so attribution is preserved and later units
build on a recorded baseline. CU-25 ports upstream `6848aa1` (skip
leading non-session lines in Oh My Pi JSONL) to restore parser parity.
CU-26 closes the saturating-arithmetic remainder in parser accounting.
CU-27 fixes the Go-flag shim's boolean misclassification with a
clap-pinning contract test. CU-28 repoints publish metadata. Detailed
acceptance and evidence expectations: `2026-09-21-cycle7-prioritization.md`
(CU-24..CU-28) and `ROADMAP.md` hardening-lane entries.

## Change-unit decisions and rationale

| Unit | Surfaces | Rationale |
|---|---|---|
| CU-24 (land F5 batch + records) | `main.rs:161-175,262-267,617-626`; `insights.rs:107-115,326-345`; `session_cache.rs:627-694` + tests `:1461+`; `entrypoints.rs:276-309`; `discovery_contract.rs`; `agenttrace-tui/src/tests.rs` (language-preference isolation); `ROADMAP.md` F5 filing; new implementation record | Impl exists and is green; the only missing work is verification records. Landing it first turns the stalled dirty state into the recorded baseline every later unit builds on. The roadmap filing and implementation record ride WITH this unit (the disposition records the fix they describe). |
| CU-25 (Oh My Pi drift port) | `parser.rs:1325`, `parser.rs:1400` + new fixture/unit test + drift census in the stewardship log | Highest strategic value (parser-superset parity before the next fork PR); upstream reference `6848aa1` exists; S effort. |
| CU-26 (saturating remainder) | `parser.rs:2244-2262,2304-2318,3423-3440` + i64::MAX fixture | MEDIUM robustness; same file as CU-25 but disjoint functions/regions. |
| CU-27 (shim boolean fix) | `main.rs:734-763` (esp. `:753`) + contract test in `entrypoints.rs` | MEDIUM CLI correctness; same file as CU-24 but disjoint regions. |
| CU-28 (publish metadata) | `Cargo.toml:15-16` | XS ride-along; pure metadata, no code coupling. |

## Order and separation constraints (hints, not topology)

This request does **not** choose branches, worktrees, or commit
splitting beyond the following hints; Conductor plans topology:

1. **CU-24 first** — it absorbs the pre-existing dirty state; new edits
   must not be interleaved into the same change-unit (the fixes belong
   to the cycle-5 review findings and must land with their tests and
   filing as one revertable unit).
2. **CU-25 and CU-26 must remain separate units** — both touch
   `parser.rs` but in disjoint concerns (provider-specific skip logic
   vs generic accounting arithmetic); independently revertable.
3. **CU-27 must remain separate from CU-24** — both touch
   `agenttrace-cli/src/main.rs` but in disjoint regions (shim value-flag
   table vs sampling/coverage paths).
4. **CU-28 separate from all code units** — metadata-only.
5. The **cycle-7 implementation record and release-notes ride-along
   land last**, after all five units, as the batch's closing evidence.
6. **Preserve the dirty state**: the six modified code files, the
   `ROADMAP.md` edit, and the untracked prioritization doc live in the
   main worktree; nothing in this request may revert, rebase away, or
   re-checkout over them. Note `fix/tui-deadline-test-waits` is checked
   out in the main worktree (a branch name cannot be reused in a second
   worktree).

## Verification at landing

Per-unit evidence per the prioritization record; batch gate: full
workspace suite (baseline green, exit 0, 2026-09-21), `cargo fmt
--check`, `cargo clippy --workspace --all-targets`. No CI workflow,
JSON-shape, or installer changes are in scope (F5-7 decision preserved;
deferred items stay deferred per the prioritization record).

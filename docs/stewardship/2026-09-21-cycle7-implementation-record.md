---
type: stewardship-record
id: cycle7-implementation
cycle: 7
batch: finish-the-truth-telling-debt-hold-parser-parity
date: 2026-09-21
base_commit: df3b621
record_kind: ce-handoff/v1
status: implemented-uncommitted
---

# Cycle 7 implementation record — CU-24..CU-28

Executed in the working tree on top of HEAD `df3b621` (main worktree,
branch `fix/tui-deadline-test-waits`). Nothing committed, pushed, or
PR'd; the Conductor owns topology per the stewardship contract
(docs/stewardship/2026-09-21-cycle7-stewardship-request.md). The ce-*
compound-engineering router remains unavailable in this environment
(empty stub; only `agent-reach` is installed), so phases were executed
directly, as disclosed in every cycle artifact this run.

The batch also absorbed the pre-existing dirty state as its first unit
(CU-24): the tree entered cycle 7 with a complete, suite-green
cycle-5-review remediation (F5-1..F5-5) plus two test-isolation fixes
that had never been recorded — the cycle-5 review's disposition to file
F5-1/F5-2 as cycle-6 items was never executed (`grep 'F5-' ROADMAP.md`
was empty before this cycle). This record repairs that gap.

## Verification summary (re-run after the last code edit)

- `cargo test --workspace --quiet`: **exit 0**, all test binaries green
  (agenttrace-core lib 82, agenttrace-core bin 7,
  discovery_contract 71, agenttrace bin 15, entrypoints 6,
  demo_contract 2, agenttrace-tui 41 — 224 tests total; corrected
  per review F7-3). New tests listed per CU below.
- `cargo fmt --check`: clean (one fmt drift introduced by CU-26 and
  fixed before the final run).
- `cargo clippy --workspace --all-targets --quiet`: clean, zero
  warnings.
- The full-suite baseline with the in-flight F5 batch (pre-cycle-7
  edits) was also exit 0, confirming CU-24's landing adds no risk.

## CU-24 — land the cycle-5-review remediation F5-1..F5-5 (with riders)

The in-flight work was verified against the review text
(docs/reviews/2026-09-03-cycle5-independent-review.md:136-236) rather
than assumed:

- F5-1: sampling disclosure names the active view — `main.rs:262-267`
  renders `sampled first N ... in the --sort {sort} --order {order}`
  view; the review's unpinned case (non-default `--sort` with
  `--sample`) is closed by the `--sample 2 --sort cost --order asc`
  pin in `governance_sampling_is_explicit_and_disclosed`
  (entrypoints.rs:219-223).
- F5-2: out-of-scope counting is per source unit — `insights.rs`
  `data_health_scoped` counts source units, not sessions;
  contract-pinned in `discovery_contract.rs`.
- F5-3: `--sample` outside governance-class actions is rejected with a
  named error (`main.rs:161-175`); pinned in entrypoints.rs:276-309.
- F5-4: the text compare path carries the same sampled-exclusion
  reason via shared `audit_coverage_line` (`main.rs:617-626`).
- F5-5: headerless session-cache entries are evictable (oldest-first
  uses `map_or(i64::MIN)`), with new unit tests in session_cache.rs.
- Riders: the run-5d025d55 cache-isolation flake fix
  (discovery_contract.rs guards cache state) and the TUI
  language-preference isolation tests (pinned English beats a persisted
  Chinese preference — agenttrace-tui/src/tests.rs).
- F5-6/F5-7 restated as recorded decisions, unchanged: F5-6 (baseline
  gate default) and F5-7 (naming-provenance vocabulary) were accepted
  as-is by the review; no code change owed.

Roadmap filing: the `### Completed in cycle 7` entry records F5-1..F5-5
individually, closing the disposition gap.

## CU-25 — upstream drift port: Oh My Pi leading non-session lines

Upstream commit `6848aa1` (PR #284, merged 2026-09-11) changed
`parse_oh_my_pi_jsonl` to skip leading non-`session` objects before
requiring the session header; upstream reported 203/203 real pi session
files failing with `oh_my_pi: missing session header` because real
files open with a title record. Our tree matched the pre-fix shape
exactly: in-loop `bail!` before the first session object
(previously parser.rs:1323) plus the post-loop guard (:1399).

Port: the in-loop bail becomes a `continue` (skip), preserving the
post-loop `!seen_header` bail. Dispatch is sniffer-gated
(`is_oh_my_pi_jsonl`, parser.rs:143/:1302), so a file with no session
header anywhere never enters the Oh My Pi path at all.

Tests:

- `rust_parses_oh_my_pi_session_with_leading_title_line` — a title-led
  fixture parses to an `oh_my_pi` session (user_messages 1, title
  preserved).
- `rust_headerless_oh_my_pi_style_file_falls_back_to_generic` — pins
  the dispatch boundary: no session header anywhere ⇒ the sniffer
  declines and the generic parser handles the file (source_tool
  "generic"), rather than an oh_my_pi error.

Drift census (this cycle, upstream `e005952..FETCH_HEAD`):

- `6848aa1` parser fix — **ported** (this CU).
- `a34dea2` (PR #283, TUI improvements) — **consciously declined**:
  the fork TUI has diverged (cycle-4/5 TUI hardening, the
  language-preference isolation work, render-path unwrap removal);
  porting upstream TUI changes blind risks regression for no measured
  gain. Revisit at the next deliberate upstream re-sync.

## CU-26 — saturating arithmetic: parser accounting remainder

The audit the roadmap item demanded (re-grepped after the fixes):

saturating: parser.rs:822 (uncached input), :1991 (output+reasoning),
:2278/:2284 (`codex_token_count_usage` uncached input, output sum),
:2334 (`token_usage_delta`), :3442 (`add_usage`), :3449
(`add_usage_value`), :3766 (fold), :3819 (i64::saturating_add fold),
:4157 (output+reasoning). The only remaining bare `+=` in parser.rs
are loop-index increments in the escape scanner (:4053/:4069/:4073/
:4080/:4087) — not token counters. No bare `+=`/`-=` on token or cost
counters survives outside the audited list.

Fixes: `token_usage_delta` subtraction → `saturating_sub` (negative
deltas clamp to 0); `add_usage`/`add_usage_value` → `saturating_add`;
`codex_token_count_usage` output+reasoning sum → `saturating_add`,
uncached-input subtraction → `saturating_sub(...).max(0)`.

Tests: unit tests in parser.rs's test module pin `i64::MAX`/`i64::MIN`
behavior per function (delta saturates instead of overflowing;
accumulators pin at i64::MAX; codex counts saturate), plus the
acceptance fixture `rust_codex_rollout_i64_extreme_token_counts_parse_without_panicking`
(discovery_contract.rs): a rollout whose token_count fields are
`i64::MAX` parses without panic in a debug build, totals saturate
(output pinned at i64::MAX), and nothing wraps negative.

## CU-27 — Go-flag shim boolean misclassification

`flag_takes_value` listed the boolean `--no-baseline-gate` among value
flags, so `agenttrace --no-baseline-gate --overview` swallowed
`--overview` as the flag's value. Fix: removed from the value list
(main.rs). Pinned two ways:

- `go_flag_shim_matches_clap_flag_arity` — the contract test: every
  clap boolean flag (22 enumerated from the Args struct) must be
  arity-0 in the shim, every clap value flag (27) arity-1. Adding a
  new flag without updating the shim now fails this test by name.
- `go_flag_shim_does_not_swallow_the_flag_after_no_baseline_gate` —
  behavioral pin that the flag after `--no-baseline-gate` survives.

## CU-28 — workspace publish metadata (reverted at CI)

`f8f5303` repointed `Cargo.toml:15-16` `repository`/`homepage` to
`https://github.com/codeo1io/agenttrace`. CI (run 35576006780, step
`Validate Cargo manifests`) rejected it: `scripts/ci/check-cargo-manifests.sh`,
inherited from upstream (`e005952`, upstream #281), pins all three crates'
metadata to `luoyuctl/agenttrace`. Reverted rather than editing the guard —
the guard is upstream's crates.io contract, and whether fork crates should
ever carry fork metadata is a steward-level policy question (same family as
the deferred fork dependency-review item), filed for a later cycle.
So the shipped state keeps the upstream URLs.

## Changed files

- crates/agenttrace-core/src/parser.rs — CU-25 skip port; CU-26
  saturating ops; 4 new unit tests.
- crates/agenttrace-core/tests/discovery_contract.rs — 3 new tests
  (CU-25 ×2, CU-26 fixture ×1).
- crates/agenttrace-cli/src/main.rs — CU-27 fix; 2 new tests.
- Cargo.toml — CU-28.
- ROADMAP.md — cycle-7 Completed entry; closed items leave the
  hardening lane; CLI-polish and hygiene amendments struck; tail
  narrative updated.
- CHANGELOG.md — Unreleased entries naming the user-visible changes.
- docs/stewardship/2026-09-21-cycle7-implementation-record.md — this
  record.

Plus the CU-24 in-flight files landed unchanged from their
pre-existing state (main.rs, entrypoints.rs, insights.rs,
session_cache.rs, discovery_contract.rs, agenttrace-tui/src/tests.rs).

## Residuals / handoff

- The commit (single branch or split per the stewardship contract's
  must_remain_separate hints) lands at this run's commit gate; the
  sha is recorded there, not here.
- Cycle-8 headliner (per the prioritization record): the Windows
  HOME/USERPROFILE resolver; then Hermes state.db schema research and
  the fork dependency-review fix at the PR/CI stage.

## Review-fix addendum (2026-09-21, independent-review:fix phase)

Verdict was pass_with_findings (docs/reviews/
2026-09-21-cycle7-independent-review.md); all five findings resolved
this turn:

- **F7-1 (MEDIUM), fixed in code**: `go_flag_shim_matches_clap_flag_arity`
  now derives every flag's arity from `Args::command().get_arguments()`
  (clap `CommandFactory`), asserting shim arity per real clap long and
  short — the previous hand-copied 49-flag snapshot lists are gone. A
  new clap flag the shim misses now fails the test by name, which is
  what the CHANGELOG/ROADMAP/learnings rule 2 claims had already
  promised. Re-verified: the clap-derived test passes with the shim
  unchanged (its lists are complete today).
- **F7-2 (LOW), fixed**: the two paraphrased test names above now
  match `cargo test -- --list` exactly.
- **F7-3 (LOW), fixed**: the verification ledger now carries the true
  224-test per-binary shape.
- **F7-4 (LOW, pre-existing), filed per disposition**: the legacy
  `data_health` files-minus-sessions subtraction (insights.rs:306-314)
  is added to the ROADMAP coverage lane as a unit-hygiene item with
  the TUI reconstructed-denominator exposure named.
- **F7-5 (INFO), fixed as ROADMAP ride-along**: the two chopped
  paragraphs (hygiene lane, CLI-polish lane) re-wrapped and the
  mid-word `installer-`/`checksum` hyphen break joined.

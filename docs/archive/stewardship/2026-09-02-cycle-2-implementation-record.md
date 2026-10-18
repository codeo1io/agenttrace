---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T10:20:00Z"
title: "Cycle 2 implementation record — trustworthy numbers on untrusted databases, and a launch that doesn't crash"
summary: "Implements the amended cycle-2 batch (CU-1..CU-7) from the stewardship request; records red-to-green evidence, the one defect found in the implementation itself, and the full verification matrix."
keywords: ["agenttrace", "cycle-2", "implementation", "sqlite-hardening", "stored-totals", "launch-guard"]
cwd: "/work/projects/agenttrace"
repository: "luoyectl/agenttrace"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
branch: "master"
head: "e005952"
---

# Cycle 2 implementation record

Run `9fcc0661af474e2783a0dee7541f6ddb`, implement attempt
`a1f65ad214b14febbfb36c2edfae6456`. The batch is the one selected by
`docs/stewardship/2026-09-02-cycle2-prioritization-update.md` and
requested by `docs/stewardship/2026-09-02-cycle-2-stewardship-request.md`
(+ `-update.md`). Worked directly in the checkout (no conductor worktree
existed: `git worktree list` shows only this tree); nothing committed or
pushed, per the delegation policy.

Method disclosure (unchanged gap): no ce-* SKILL.md is installed; the
implementation ran in-thread following the batch's own 10-step order.

## Red → green, in order

Regression guards were written **first** and demonstrated red against the
unfixed tree (each in isolation, since a panic inside `with_home` poisons
the shared env lock for the rest of a run — pre-existing infra behavior):

| Guard | Red | Green |
|---|---|---|
| `adversarial_sqlite_overflow_db_neither_panics_nor_wraps` (P5-1) | `panicked at sqlite_sessions.rs:410:5: attempt to add with overflow` | input pins at `i64::MAX`, output 2, exit clean |
| `adversarial_sqlite_u64_max_input_saturates_instead_of_wrapping` (P5-2) | `left: -1, right: i64::MAX` | `tokens_input == i64::MAX` |
| `hermes_negative_token_columns_are_clamped` | `left: -500, right: 0` | clamped to 0 |
| `opencode_stored_session_totals_preferred_with_delta` (CU-2) | `left: 400, right: 1000` (derived won) | stored wins, delta 720, provenance `stored_session_totals` |
| `opencode_unknown_time_session_stays_visible_in_range` (CU-3) | `left: 0, right: 1` (dropped) | visible; `data_health.unknown_time_sessions == 1` |
| `version_wins_over_invalid_lang` (CU-4) | exit 1, `unsupported --lang value 'fr'` | exit 0, banner |
| `tui_launch_fails_cleanly_when_stdout_is_not_a_terminal` (CU-5) | `left: 101` | exit 1 with `not a terminal` + `--overview` message |
| `pricing_snapshot_date_is_pinned_to_the_bundled_payload` (CU-6) | green on arrival (guard, not defect: dates currently match) | green, red on drift by construction |

Plus two companions that encode non-defect behavior:
`opencode_missing_stored_columns_fall_back_to_derived` (older schema keeps
derived path, delta 0) and `opencode_text_valued_stored_columns_do_not_drop_the_session`
(see "Defect found during implementation").

## What changed, per change unit

- **CU-1** — `sqlite_sessions.rs`: local wrapping `number_as_i64` deleted,
  routed through `parser::number_as_i64` (now `pub(crate)`); `:403` and
  the four accumulators → `saturating_add`; the four Hermes token reads →
  `.max(0)`; `waste.rs:180` → `saturating_sub`; `governance.rs`
  `cost_audit` downgrades `confidence` to `low` when any session carries a
  negative token/cost component; `CHANGELOG.md` over-claim at the old
  `:7` corrected (the entry now states the first pass missed the SQLite
  path and points at this one).
- **CU-2** — `sqlite_sessions.rs`: the opencode `session` select sniffs
  `cost` + the five token columns (missing ones become SQL `null`
  placeholders); `apply_opencode_stored_totals` prefers stored values
  (negatives/non-finite cost clamped or ignored), records
  `stored_totals_delta`, and sets `provenance.tokens` /
  `provenance.cost` accordingly; `Metrics.stored_totals_delta` added
  (serialized per session, round-trips through the snapshot cache —
  `GoMetrics.StoredTotalsDelta`, `SQLITE_SNAPSHOT_SCHEMA_VERSION` 4→5,
  schema test updated to pin v5 and reject v4).
- **CU-3** — unknown-time sessions stay visible: both SQL predicates
  (`hermes` `started_at`, `opencode` `time_created`) keep null/≤0 rows;
  `filter_since`, `discovery.rs` retain, and
  `insights::session_matches_time_range` all keep unparseable/empty
  starts; `data_health.unknown_time_sessions` counts them.
- **CU-4** — `main.rs`: `--version` early return moved above
  `report_language`.
- **CU-5** — `app.rs`: `run_with_app` bails with a
  "stdout is not a terminal … use `agenttrace --overview`" error before
  `ratatui::init()` (std `IsTerminal`, MSRV 1.80 ≥ 1.70; no new
  dependency).
- **CU-6** — `pricing.rs` test pins `PRICING_SNAPSHOT_DATE` to the
  bundled payload's `_snapshot.date` via the existing
  `include_str!` constant.
- **CU-7** — `.gitignore`: `.hermes/` added.
- **DataHealth** (`insights.rs`): three additive fields —
  `stored_totals_sessions`, `stored_totals_delta_tokens` (saturating
  |delta| sum), `unknown_time_sessions`.
- **Fixtures**: `scripts/fixtures/make-adversarial-sqlite.py` generates
  `testdata/generated/adversarial/sqlite/{overflow,wrap}.db` (committed
  reproducers of P5-1/P5-2); tests copy them into a temp HOME and load
  through the normal discovery path.

## Defect found during implementation (fixed same-phase)

The first CU-2 cut read stored columns with
`row.get::<Option<i64>>()`. SQLite columns are dynamically typed, so a
TEXT value (`tokens_input = '999'`) failed the row conversion and
**silently dropped the entire session** (reproduced end-to-end:
`Error: No session files found`). Fixed with lenient
`sqlite_value_as_i64`/`sqlite_value_as_f64` reads (Integer/Real/Text-parse;
unparseable → `None` → derived fallback / unknown-time bucket), applied to
the stored columns and both time columns, plus regression test
`opencode_text_valued_stored_columns_do_not_drop_the_session`. This is
the same defect class the batch exists to close, caught by the batch's
own verification plan (DB-mutation fuzzing) rather than by review.

## Verification matrix (all on the dirty tree)

- `cargo test --workspace` (debug): **169 passed, 0 failed** — 159
  baseline + 10 new (7 discovery_contract, 2 CLI launch_guards, 1 pricing
  pin; one existing session-cache schema test updated for v5).
- `cargo test --workspace --release`: **169 passed, 0 failed**.
- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets`
  0 warnings.
- `scripts/ci/check-*.sh` with `AGENTTRACE_BIN=target/release/agenttrace`:
  **8/10 exit 0**; `check-rust-tui-real-smoke.sh` and
  `check-rust-release-local.sh` (which invokes it) exit 1 solely at
  `command -v expect` — `expect` is absent in this environment, the
  pre-existing P4-5 gap, identical before and after this change.
- End-to-end reproducers: overflow DB → exit 0, no panic (was 101);
  `u64::MAX` → `"input": 9223372036854775807` (was `-1`);
  `time_created = 0` under `--range 7d` → `sessions_in_scope: 1`,
  `unknown_time_sessions: 1`; `--demo` piped → exit 1 with the message
  (was 101 + backtrace); `--lang fr --version` → exit 0.
- Fuzz: JSONL mutation harness (`/tmp/at-assess/fuzz_mutate.py`, 30×50×5
  debug runs) → 0 panics; new DB-mutation fuzz (negative / oversized /
  TEXT / null / garbage values in every stored column, message tokens,
  and time columns) → debug 150 runs and release 100 runs, 0 panics, 0
  negative token outputs.

## Behavior changes reviewers should expect (by design)

- OpenCode databases with stored totals now report the stored numbers
  (and stored cost) instead of derived ones; the delta is visible per
  session and in `data_health`. Older schemas are unchanged.
- Sessions with unknown start times now appear in `--range`/`--since`
  views (previously silently dropped), counted in `data_health`.
- The default TUI exits 1 with guidance instead of panicking when stdout
  is not a terminal.
- Cached SQLite snapshots regenerate once (schema 4→5).

## Not done (per batch boundaries)

`parent_id` hierarchy (research-gated), deferred items P3-1/P3-4/P3-5/
P3-2 and candidates 24–27 — untouched, still queued per the
prioritization update. ROADMAP.md was not modified by this phase.

## Re-verification (implement attempt `e0a97ab9fba34e559339842e42e38c7f`)

The phase was re-issued under a new attempt id; the tree was inspected
first — the complete batch was already present (8 `saturating_add` sites,
`IsTerminal` import, `.hermes/` ignore, lenient column readers, fixtures,
`launch_guards.rs`, this record) — so this attempt performed an idempotent
re-verification instead of redoing work, all against the same dirty tree
at `e0059522`:

- debug workspace suite 169/169; release workspace suite 169/169;
  `cargo fmt --check` clean; clippy 0 warnings.
- check scripts with `AGENTTRACE_BIN=release`: 8/10 exit 0; the two
  failures still only at `command -v expect` (absent in this environment).
- Acceptances re-run on the release binary: overflow DB exit 0 with no
  stderr; `u64::MAX` message → `"input": 9223372036854775807`;
  `time_created=0` under `--range 7d` → `sessions_in_scope: 1`,
  `data_health.unknown_time_sessions: 1`; TEXT-valued stored columns →
  `stored_totals_sessions: 1`, `stored_totals_delta_tokens: 729`;
  piped `--demo` → exit 1 with the `--overview` guidance; `--lang fr
  --version` → exit 0 banner.
- Fuzz re-run: JSONL mutation harness 0 panics; DB-mutation fuzz
  (75 release runs) 0 panics, 0 negative token outputs.

No code changed in this attempt; every prior finding stands.

## Targeted tests (attempt `4d97f1eda0624f889f4d040f81bde2bb`, targeted_tests)

Focused suites for the changed surfaces all pass — and the run surfaced
and fixed a **pre-existing flaky test** plus the cascade that amplified
it:

- Targeted (debug): core lib 51/51, discovery_contract 57/57, CLI
  11+1+2 (incl. launch_guards 2/2), TUI 40/40; 21 named module tests for
  the changed units; `cargo fmt --check` clean; clippy on the three
  changed crates (all targets) 0 warnings.
- **Flake found**: `rust_writes_and_refreshes_go_compatible_directory_cache`
  (pre-existing) intermittently failed in release multi-package runs
  (~1 in 15) with `sessions.len(): left 1, right 2`. **Root cause**:
  `bump_dir_mtime` assumed write+remove advances a directory mtime, but
  this `/tmp` is ext2/ext3 — directory mtimes advance once per second, so
  the bump was a no-op 98% of the time (measured: 1958/2000 collisions);
  the test passed only when the sequence straddled a 1-second tick.
  **Fix**: the helper now loops (write/remove/stat, 5 ms sleep, up to 2 s)
  until the mtime actually changes, then fails loudly if it never does.
  Post-fix: 20/20 isolated release runs and 10/10 combined runs green
  (the ~0.2 s runs are the helper correctly waiting for the tick).
- **Cascade amplifier fixed**: a panic inside `with_home*` used to poison
  the shared env-lock mutex (`resume_unwind` fired while the guard was
  still held), turning that single flake into 10 simultaneous failures.
  The helpers now scope the guard to drop before resuming the panic, and
  the lock tolerates poisoning; a deliberate probe test (panicking inside
  `with_home` followed by a follower test) demonstrated exactly one
  failure with the follower passing, then the probe was removed.
- Full workspace re-verified after both fixes: debug 169/169, release
  169/169.

## Full validation (attempt `61620470880e462498e32ddac5e9f503`, full_tests)

Ran the authoritative suite as defined by `.github/workflows/ci.yml`
(ubuntu job), step by step in CI order, with CI's exact environment.
All 16 executable steps pass: fmt `--check` OK; clippy (CI form,
`-D warnings`) clean; `cargo test -p agenttrace-core -p agenttrace-tui
-p agenttrace` 169/169; `cargo build --release -p agenttrace` OK;
entrypoints 1/1; check-output-contract, check-deterministic-output,
check-report-semantics, check-release-surfaces, check-docs-commands,
check-rust-real-cli-smoke (CI env) all exit 0; `ruby -c` on the Homebrew
formula Syntax OK; `npm --prefix npm test` pass 1 fail 0 exit 0;
check-cargo-manifests OK; check-plugin-version OK (plugin v0.7.1 ==
CHANGELOG v0.7.1); `bash -n` on helper scripts OK. The remaining CI step
(Rust TUI real-data smoke) is gated upstream (`if:
env.AGENTTRACE_TUI_REAL_DIR != ''`) and is skipped here exactly as CI
skips it; it also cannot run locally because `expect` is absent (P4-5).
Beyond CI's minimum: `cargo test --workspace --release` 169/169, clippy
`--workspace --all-targets` 0 warnings, and a repeated debug run 169/169
(stability after the targeted_tests flake fixes). No regressions to fix.

## Final validation (attempt `c45d3e3ec1c74671831e08f9891d7d06`, final_validation)

Re-ran the entire authoritative suite plus every per-CU end-to-end
reproducer from a cold shell on the unchanged dirty tree at `e0059522`
(31 status entries, nothing staged/committed). All green, zero fixes
needed.

- **CI suite (`.github/workflows/ci.yml`, all 16 executable steps, in CI
  order, CI's exact env):** `cargo fmt --check` OK; clippy CI form
  (`-D warnings`) clean; `cargo test -p agenttrace-core -p
  agenttrace-tui -p agenttrace` **169 passed / 0 failed**; release build
  OK; entrypoints 1/1; check-output-contract, check-deterministic-output,
  check-report-semantics, check-release-surfaces, check-docs-commands,
  check-rust-real-cli-smoke (CI env, 20 sampled files) all **exit 0**;
  `ruby -c` Syntax OK; `npm --prefix npm test` pass 1 / fail 0;
  check-cargo-manifests aligned; check-plugin-version v0.7.1 ==
  CHANGELOG v0.7.1; `bash -n` on all helper scripts OK. The TUI
  real-data step remains env-gated (`AGENTTRACE_TUI_REAL_DIR` unset) and
  additionally blocked locally by absent `expect` (pre-existing P4-5).
- **Beyond CI:** `cargo test --workspace --release` **169/169**;
  `cargo clippy --workspace --all-targets` **0 warnings**.
- **CU-1 (release binary):** `overflow.db` via `--latest -f json` →
  **exit 0**, `tokens.input = 9223372036854775807` (was debug exit 101);
  `wrap.db` → **exit 0**, `input = 9223372036854775807` (was `-1`).
  Named guards `adversarial_sqlite_overflow_db_neither_panics_nor_wraps`,
  `adversarial_sqlite_u64_max_input_saturates_instead_of_wrapping`,
  `hermes_negative_token_columns_are_clamped` all ok. Only remaining
  `+=` in `sqlite_sessions.rs` is the `usize` per-message counter.
- **CU-2 (release binary):** session row with `tokens_input = 'garbage'`
  (TEXT) + valid `tokens_output = 1000` over messages deriving 400/300 →
  stored totals win (`tokens {input: 0, output: 1000}`),
  `data_health.stored_totals_sessions = 1` (proof that
  `provenance.tokens == "stored_session_totals"`), delta visible.
- **CU-3 (release binary):** `time_created = 0` DB under
  `--overview -f json --range 7d` → **exit 0**,
  `scope.sessions_in_scope = 1`, `data_health.unknown_time_sessions = 1`
  (was silently dropped); guard
  `opencode_unknown_time_session_stays_visible_in_range` ok.
- **CU-4:** `--lang fr --version` → **exit 0**, prints
  `agenttrace v0.0.0-dev`; guard `version_wins_over_invalid_lang` ok.
- **CU-5 (release binary):** default TUI and `--demo` with piped stdout →
  **exit 1** with `stdout is not a terminal; ... use --overview` (was
  exit 101 panic); guard
  `tui_launch_fails_cleanly_when_stdout_is_not_a_terminal` ok.
- **CU-6 (red/green):** sed-diffing `PRICING_SNAPSHOT_DATE` to
  `2025-01-01` → `pricing_snapshot_date_is_pinned_to_the_bundled_payload`
  **FAILED** with the drift message; restoring the const → **ok**.
  File confirmed byte-identical after restore (sha256 unchanged).
- **CU-7:** `.gitignore:13` = `.hermes/`; `git check-ignore .hermes`
  matches; `.hermes/` absent from `git status`.
- **Fuzz re-run:** JSONL mutation harness (debug binary, overflow checks
  on) → `done. panics: 0`; DB-mutation harness (release binary, both DB
  shapes, values incl. `2**63-1`, `-(2**63)`, `2**64-1`, TEXT, REAL
  1.5e19, None) → `db-fuzz: runs 60, bad 0`.
- **Working tree contains only intended changes:** 31 status entries —
  20 modified (cycle-1 set + ROADMAP.md + cycle-2's `sqlite_sessions.rs`,
  `waste.rs`, `app.rs` starting clean at HEAD; `app.rs` diff is exactly
  the IsTerminal guard, `waste.rs` exactly the saturating_sub) + 11
  untracked trees (batch tests/fixtures/scripts +
  `docs/{decisions,ideation,research,reviews,stewardship}` artifacts +
  `pricing_snapshot.json`). Secret/machine-path scan of the full diff
  and all untracked text files: only hits are historical prose in review
  docs naming `/tmp/at-*` reproducer paths. `ci-artifacts/`, `target/`,
  `.hermes/` all properly ignored; nothing staged; HEAD still
  `e0059522`. CHANGELOG "Unreleased" documents every CU including the
  corrected repo-wide-hardening claim (CU-1's rider).

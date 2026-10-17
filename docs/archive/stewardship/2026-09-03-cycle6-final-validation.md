# Cycle 6 Final Validation Record — 2026-09-03

- **run**: `a24bcf084cf049208c75d2cb4f3a3755`
- **phase**: final_validation (`final_validation:final_validation`), attempt `459c4fa3417a48acbb1d1ce6f086adc9`
- **tree under test**: single worktree `/work/projects/agenttrace` at HEAD `696206f` (master, tracking `fork`), carrying the uncommitted cycle-6 batch (CU-17..CU-22), the roadmap update, and this campaign's phase records. Nothing committed, pushed, PR'd, or CI-triggered (all four are prohibited this phase); no stashes (`git stash list` empty; `git worktree list` = one entry).
- **skill routing**: no `ce-*` router is installed on this host (only `agent-reach`); proceeded directly with the same end-to-end + local-verification discipline, disclosed consistently with every prior phase of this run.
- **method**: (1) re-verified the tree is exactly the review-phase state, then found the pass-10 review's one required pre-commit fix (F1) still unfixed — reproduced it live, fixed it, and pinned it with tests (the completion lock forbids stopping at a review finding); (2) re-ran the authoritative suite — the `.github/workflows/ci.yml` `test` job — locally, in workflow order, with the workflow's exact commands/env, fresh release build included; (3) re-verified every CU-17..CU-22 mandatory outcome live against that release binary with reproducers under `/tmp`; (4) audited every working-tree entry for intent and every added line for AGENTS.md rule-3 contamination. Scratch lives in `/tmp`; CI output artifacts go to `/tmp/ci-artifacts`, never the repo.

## 0. F1 (review-required pre-commit fix) — reproduced, fixed, pinned

Pass-10 (`2026-09-03-cycle6-independent-review.md` §F1) required, before any
commit: `parser.rs:513` read the VIEW_FILE path via the wrong-case pointer
`/viewFile/absolutePathURI`, while the daemon serializes the field as
`json:"absolutePathUri"` — so real sidecars silently dropped every VIEW_FILE
step (empty path → `continue`), and a VIEW_FILE-only sidecar failed the
`non_empty(events)` sniff entirely. The review phase deliberately modified no
code; this phase applied the fix.

- **Reproduction (pre-fix, live)**: a sidecar trajectory JSON whose steps are
  USER_INPUT + VIEW_FILE(wire-case `absolutePathUri`, `startLine`/`endLine`)
  parsed the session but produced **zero** view tool events (`grep -c 'view
  file:///x.rs'` → 0; `tool_results` stayed 0) — the silent-drop half of F1.
- **Fix** (`crates/agenttrace-core/src/parser.rs`, `CORTEX_STEP_TYPE_VIEW_FILE`
  arm): wire-case `/viewFile/absolutePathUri` is now primary, with
  `/viewFile/absolutePathURI` kept as a tolerated alias (exactly the remedy
  the review prescribed; the alias protects any producer that copied this
  parser's original wrong-case reading).
- **Tests added**:
  - unit `parser::tests::antigravity_view_file_accepts_wire_case_path_key` —
    asserts both spellings yield a `tool` event with content
    `view file:///work/x.rs lines 1-2`, non-error;
  - the CU-19 contract test's sidecar fixture
    (`rust_discovers_antigravity_conversation_sidecars_but_not_the_store`)
    gains a wire-case VIEW_FILE step and now asserts `tool_results == 1` and
    `events_total == 3` (the suite previously exercised only USER_INPUT +
    PLANNER_RESPONSE, which is why F1 escaped it).
- **Post-fix live check**: wire-case sidecar → `tool_results: 1`; wrong-case
  alias sidecar → `tool_results: 1` (both under
  `target/release/agenttrace -d <dir> --sessions -f json`).
- The review's other findings are unchanged deferrals (§6).

## 1. Authoritative CI suite (`.github/workflows/ci.yml` `test` job), re-run in order — all PASS

Run on the post-F1 tree; `AGENTTRACE_CI_OUT=/tmp/ci-artifacts` (kept out of
the repo, matching prior phases; the workflow's artifact-upload step is
CI-only and has no local equivalent).

| # | Step | Command | Result |
|---|------|---------|--------|
| 1 | Check Rust formatting | `cargo fmt --check` | exit 0 |
| 2 | Portable CI runners only (CU-17 guard) | `scripts/ci/check-no-self-hosted.sh` | exit 0 — "all workflows use portable runners" |
| 3 | Clippy | `cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace -- -D warnings` | exit 0, zero warnings |
| 4 | Rust tests | `cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace` | exit 0 — **213 passed / 0 failed** (13+6+2+78+7+67+40 + 2 empty doc suites; 0 `FAILED` lines) |
| 5 | Build CLI | `cargo build --release -p agenttrace` | exit 0 (Finished, 19.86s) |
| 6 | Single binary entrypoints | `cargo test -p agenttrace --test entrypoints` | ok, 6 passed / 0 failed |
| 7 | Output contract | `AGENTTRACE_BIN=target/release/agenttrace AGENTTRACE_CI_OUT=/tmp/ci-artifacts scripts/ci/check-output-contract.sh` | exit 0 |
| 8 | Deterministic output | same env, `check-deterministic-output.sh` | exit 0 |
| 9 | Report semantics | same env, `check-report-semantics.sh` | exit 0 |
| 10 | Release surface drift | `scripts/ci/check-release-surfaces.sh` | exit 0 |
| 11 | Documented commands | same env, `check-docs-commands.sh` | exit 0 (docs/overview.md + .html written) |
| 12 | Rust CLI fixture smoke | CI env `AGENTTRACE_REAL_CLI_DIR=testdata AGENTTRACE_REAL_CLI_QUERY=internal/ws AGENTTRACE_REAL_CLI_FILE_LIMIT=20` → `check-rust-real-cli-smoke.sh` | exit 0 — "Rust real CLI smoke passed: source_dir=testdata sampled_files=20 query=internal/ws" |
| 13 | Rust TUI real-data smoke | conditional (`if: env.AGENTTRACE_TUI_REAL_DIR != ''`) | env unset in CI — covered by the equivalent PTY harness (§2) |
| 14 | Homebrew formula | `ruby -c homebrew/Formula/agenttrace.rb` | Syntax OK |
| 15 | npm package contract | `npm --prefix npm test` | pass 1 / fail 0 (rc=0) |
| 16 | Cargo manifests | `scripts/ci/check-cargo-manifests.sh` | exit 0 — "Cargo manifest metadata is aligned." |
| 17 | Plugin version sync | `scripts/ci/check-plugin-version.sh` | exit 0 — "plugin.json v0.7.1 matches CHANGELOG v0.7.1" |
| 18 | Helper scripts | `bash -n scripts/record-demo.sh scripts/ci/*.sh` | exit 0 |

## 2. Baseline conventions beyond the workflow

- **Release-mode tests**: `cargo test --workspace --release` → **213 passed /
  0 failed** across 9 `test result: ok` lines — identical to debug mode.
- **Conditional TUI step, PTY-equivalent harness**:
  `python3 /tmp/tui-pty-smoke.py target/release/agenttrace` → all frame
  markers through `Keys`, `TUI exited after scripted q (rc=0)`,
  `PTY TUI smoke passed: source_dir=/home/agent/.pi/agent/sessions
  sampled_files=2` (harness deliberately kept out of the repo, as in cycle 4).
- **Version-claim integrity**: `--overview --version` → `agenttrace
  v0.0.0-dev`, exit 0.
- **Binary freshness**: `find crates -name '*.rs' -newer
  target/release/agenttrace | wc -l` → 0 (the release binary embeds the F1
  fix; all §3 checks ran against it).

## 3. Mandatory outcomes (CU-17..CU-22) re-verified live on the fresh release binary

- **CU-17 (upstream-portable CI, CRITICAL)**: `grep -n runs-on
  .github/workflows/*.yml` → exactly five hits (ci.yml:21,
  dependency-review.yml:16, release.yml:13/40/117), all `ubuntu-latest`; the
  guard is wired as the named step at ci.yml:36 (`Portable CI runners only`);
  `git config branch.master.remote` → `fork` (origin=luoyuctl/agenttrace,
  fork=codeo1io/agenttrace).
- **CU-18 (Gemini CLI `~/.gemini/tmp` root)**: `--doctor` lists `Gemini CLI …
  /home/agent/.gemini/tmp` among the directories (root present even though
  this host has no such corpus); contract tests cover discovery + parse.
- **CU-19 (Antigravity conversations root + sidecar parser)**: `--doctor`
  lists `Antigravity CLI conversations …
  /home/agent/.gemini/antigravity-cli/conversations`; the trajectory sidecar
  parser round-trips USER_INPUT/PLANNER_RESPONSE/RUN_COMMAND/ERROR_MESSAGE/
  VIEW_FILE (VIEW_FILE now with the wire-case key — §0), `.db`/`.pb` stay
  rejected (contract test).
- **CU-20 (thinking tokens as reasoning)**:
  `target/release/agenttrace -d <dir-with-testdata/gemini-thoughts-checkpoint.json>
  --audit -f json` → `tokens {input:80, output:60, reasoning:40, total:140}`,
  `reasoning_share_pct 66.6667`, provider/model `gemini_cli gemini-2.5-flash`.
- **CU-21 (`--sample` names the view)**: `--demo --audit -f json --sample 2
  --sort cost --order asc` → `excluded_reason: "sampled first 2 of 3 sessions
  in the --sort cost --order asc view (--sample 2)"`, audited 2/3.
- **CU-22 (cache byte ceiling)**: `--doctor` text → "cache size 9350092
  bytes, hard bounds: entries<=20000, bytes<=67108864 (oldest-source entries
  evicted first)"; `--doctor -f json` → `cache_size_bytes: 9350092`,
  `cache_limits: entries<=20000, bytes<=67108864`.

## 4. Working-tree intent audit — every entry accounted for

`git status --porcelain` → 31 entries (14 `M` + 17 `??`), diff
**+1151/−41 over 14 files**. Delta vs. the review-phase state
(+1098/−41 over the same 14 files) is exactly the F1 fix: `parser.rs`
381 ins/1 del → 426 ins/1 del (fix comment + alias + unit test),
`discovery_contract.rs` 147 → 155 insertions (fixture step + assertions).
No other file moved.

Modified (14) — all cycle-6/roadmap surfaces:
`.github/workflows/{ci,dependency-review,release}.yml` (CU-17);
`ROADMAP.md` (roadmap phase, +265/−27);
`crates/agenttrace-cli/src/main.rs` + `tests/entrypoints.rs` (CU-21 +
compare-string pin);
`crates/agenttrace-core/src/{discovery,doctor,governance,lib,parser,session_cache}.rs`
(CU-18/19/20/22 + F1);
`crates/agenttrace-core/tests/discovery_contract.rs` (CU-18/19/22 tests +
F1);
`docs/guides/governance-reports.md` (CU-20).

Untracked (17) — three code/test assets plus records:
`scripts/ci/check-no-self-hosted.sh` (CU-17 guard, CI-wired);
`testdata/gemini-thoughts-checkpoint.json` (CU-18/20 fixture);
`AGENTS.md` (operator policy — predates this campaign, must stay
uncommitted per its own rule 3); 13 `docs/stewardship/*` phase records and
roadmap-update `.diff` snapshots from this campaign (cycles 2-6), plus this
record. No stray scratch, no `ci-artifacts` inside the repo, no editor
droppings.

## 5. AGENTS.md rule-3 contamination scans (final tree)

- **Added lines in the diff** (`git diff | grep -aE '^\+' minus `+++`
  headers`, pattern `/home/agent|/work/projects|tailscale|192\.168\.|codeo1io|runs-on:
  self-hosted`): zero hits for every local-path/host/fork-remote pattern;
  the only `self-hosted` matches are three ROADMAP.md narrative sentences
  documenting the 6632014 finding and the guard's acceptance criterion —
  prose, not config. No binary files in the diff (`git diff --numstat`
  shows no `-` entries).
- **Untracked assets** (`scripts/ci/check-no-self-hosted.sh`,
  `testdata/gemini-thoughts-checkpoint.json`): zero local-path/host hits.
- **Untracked stewardship records**: they quote `/work/projects/agenttrace`
  and other operator-local references (F7, Info) — intended as campaign
  records; the commit gate owns their split/exclusion when landing.

## 6. Findings ledger (pass-10 review, disposition after this phase)

| ID | Severity | Disposition |
|----|----------|-------------|
| F1 VIEW_FILE wire-key case mismatch | High-for-CU-19 ("fix before commit") | **RESOLVED this phase** (§0): wire-case primary + alias, unit + contract coverage, live re-verified |
| F2 byte bound under-counts (raw vs. true size) | Medium | Deferred to cycle 7 (review follow-up 2) |
| F3 alias aggregation inconsistency across fold sites | Low | Deferred to cycle 7 |
| F4 unmapped step types dropped without disclosure | Low | Deferred to cycle 7 |
| F5 dispatch double-parses single-object sessions | Low (perf) | Deferred to cycle 7 |
| F6 CHANGELOG not updated for cycle 6 | Info | Disclosed deferral; pairs with the release/commit gate |
| F7 stewardship records carry operator-local refs | Info | Commit-gate split (unchanged; §5) |

## 7. What this phase changed (and nothing else)

1. `crates/agenttrace-core/src/parser.rs` — F1 fix + unit test.
2. `crates/agenttrace-core/tests/discovery_contract.rs` — wire-case VIEW_FILE
   fixture step + assertions.
3. This record. No commits, pushes, PRs, or CI triggers; topology remains
   Conductor's.

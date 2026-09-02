# Cycle 4 Final Validation Record — 2026-09-02

- **run**: `2a15625945fc40419fc4691c59b42a7b`
- **phase**: final_validation (`final_validation:final_validation`), attempt `79ba7711ee5541b0974d50caa481f7c8`
- **tree under test**: single worktree `/work/projects/agenttrace` at HEAD `6632014` (master), carrying the uncommitted cycle-4 batch (CU-6..CU-10) plus this run's phase records. Nothing committed or pushed; no stashes.
- **skill routing**: compound-engineering router → `ce-work` (narrowest match; no dedicated verify/validate skill installed — ce-proof/ce-dogfood/ce-sweep are browser/doc tools). ce-work's end-to-end + local-verification discipline applied; its context fence resolved cwd `/work/projects/agenttrace`, branch `master`, head `6632014`.
- **method**: every CI step re-run this phase in workflow order with the workflow's exact commands/env (fresh `cargo build --release`), then every cycle-4 mandatory outcome re-verified live against that release binary with reproducers rebuilt from scratch in `/tmp/fv4` (byte-compared against the recorded prioritization/review reproducers in `/tmp/pri4` where applicable), then a full working-tree intent audit. All phase scratch lives under `/tmp/fv4` and `/tmp/fv4-artifacts`; the only repo change this phase makes is this record.

## 1. Authoritative CI suite (`.github/workflows/ci.yml`), re-run in order — all PASS

| # | Step | Command | Result |
|---|------|---------|--------|
| 1 | Check Rust formatting | `cargo fmt --check` | exit 0 |
| 2 | Clippy | `cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace -- -D warnings` | exit 0, zero warnings |
| 3 | Rust tests | `cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace` | exit 0 — **189 passed / 0 failed** (12+3+2+64+7+61+40; 2 empty doc-test-style lines) |
| 4 | Build CLI | `cargo build --release -p agenttrace` | exit 0; `--version` → `agenttrace v0.0.0-dev` |
| 5 | Single binary entrypoints | `cargo test -p agenttrace --test entrypoints` | ok, 3 passed |
| 6 | Output contract | `AGENTTRACE_BIN=target/release/agenttrace AGENTTRACE_CI_OUT=/tmp/fv4-artifacts scripts/ci/check-output-contract.sh` | exit 0 (script is silent-on-success; artifacts written) |
| 7 | Deterministic output | same env, `check-deterministic-output.sh` | exit 0 — the cycle-2 second-boundary flake did not recur |
| 8 | Report semantics | same env, `check-report-semantics.sh` | exit 0 |
| 9 | Release surface drift | `scripts/ci/check-release-surfaces.sh` | exit 0 |
| 10 | Documented command smoke | same env, `check-docs-commands.sh` | exit 0 |
| 11 | Rust CLI fixture smoke | CI env (`REAL_CLI_DIR=testdata QUERY=internal/ws LIMIT=20`) `check-rust-real-cli-smoke.sh` | exit 0 — `Rust real CLI smoke passed: source_dir=testdata sampled_files=20 query=internal/ws` |
| 12 | Rust TUI real-data smoke | conditional (`if: env.AGENTTRACE_TUI_REAL_DIR != ''`) | `expect` absent on this host; covered by the equivalent PTY harness (§2) |
| 13 | Homebrew formula | `ruby -c homebrew/Formula/agenttrace.rb` | Syntax OK |
| 14 | npm package contract | `npm --prefix npm test` | pass 1 / fail 0 |
| 15 | Cargo manifests | `scripts/ci/check-cargo-manifests.sh` | exit 0 — "Cargo manifest metadata is aligned." |
| 16 | Plugin version sync | `scripts/ci/check-plugin-version.sh` | exit 0 — "plugin.json v0.7.1 matches CHANGELOG v0.7.1" |
| 17 | Helper scripts | `bash -n scripts/record-demo.sh scripts/ci/*.sh` | exit 0 |

## 2. Repo baseline conventions beyond the workflow

- **Release-mode tests** (`cargo test --workspace --release`): **189 passed / 0 failed** across 9 result lines — identical to debug mode.
- **Conditional TUI step, equivalent PTY harness**: `python3 /tmp/tui-pty-smoke.py target/release/agenttrace` (kept out of the repo deliberately, see full-tests record) → all 12 frame markers in order (`AgentTrace` … `Keys`), `TUI exited after scripted q (rc=0)`, `PTY TUI smoke passed: source_dir=/home/agent/.pi/agent/sessions sampled_files=2`.
- **Version-claim integrity**: `--overview --version` → `agenttrace v0.0.0-dev`, exit 0 (version still wins over action validation).

## 3. Mandatory outcomes (CU-6..CU-10) re-verified live on the fresh release binary

All reproducers rebuilt in `/tmp/fv4`; where a recorded reproducer exists in `/tmp/pri4` the rebuild was `cmp`-verified byte-identical first.

- **CU-6 / P7-1 — no silent line loss in the generic-JSONL fallback**: `mix3.jsonl` (3 lines, one lone-surrogate escape; byte-identical to the prioritization reproducer) → `Messages: 3 user` (pre-fix: 2; the lone-surrogate line is recovered, `data_health.line_skips` absent). The committed fixture `testdata/generated/adversarial/generic-loss.jsonl` (clean + recovered + coerced + genuinely-broken `\uzzzz` lines) → `data_health.line_skips {"unparseable_line":1}` in `-f json`, and the **same "Dropped lines" row in text** (`Dropped lines: unparseable_line=1`), **Markdown** (`| Dropped lines | unparseable_line=1 |`), and **HTML**. Nothing silent in either direction.
- **CU-7 / P7-2 — BOM/encoding at the parse entry**: `bom.jsonl` (UTF-8 BOM + claude-shaped JSONL) → exit 0, parses, and its overview output is **byte-identical** (`cmp`) to the BOM-less variant. A UTF-16LE file with BOM (byte-identical to the recorded reproducer) → `Error: session file utf16bom.jsonl is UTF-16 encoded; convert it to UTF-8 and retry`, exit 1 — a named, actionable error instead of a bare "unsupported session format". (A BOM-less UTF-16 file still reports the generic unsupported-format error; it is not detectable as UTF-16.)
- **CU-8 / P7-3 — baseline thresholds gate the exit code**: real session dir (`dreal`, summary `total_tokens: 11,837,170`), baseline forged to `total_tokens: 0`, run with `--baseline-max-token-delta-pct 1` → **exit 2**, the full report JSON still emitted (stdout parses as JSON *and* the `-o` file is written) carrying `baseline_comparison {token_delta_pct: 100.0, tokens_above_threshold: true}`, stderr naming the breached threshold and the opt-out (`- token delta above --baseline-max-token-delta-pct` … `opt out: --no-baseline-gate`). With `--no-baseline-gate` → **exit 0**, `baseline_comparison` retained in the JSON.
- **CU-9 / P7-5 — durable writes**: sandboxed `AGENTTRACE_SESSION_CACHE_DIR` holding `sessions.json.tmp.99999.0` backdated 2h plus `unrelated.txt` → next cache-loading run (exit 0) **removes only the orphan temp**; `unrelated.txt` survives, a fresh `sessions.json` is written. Torn `{"torn": ` history file under sandboxed `AGENTTRACE_HISTORY_DIR` + `--include-history` → exit 0, visible stderr warning (`history file … was unreadable; quarantined as …history.json.corrupt (new history starts empty)`), quarantined bytes `cmp`-identical to the original.
- **CU-10 — snapshot schema + backslash parity**: `SQLITE_SNAPSHOT_SCHEMA_VERSION: i64 = 6` at `session_cache.rs:13` with the v5-rejection path at `:204`; `sqlite_snapshot_schema_six_round_trips_provenance_and_rejects_older_schemas` passes (re-run filtered: ok, 1 passed; also inside the 64-test lib suite). Backslash parity is pinned by `escaped_backslash_pairs_never_mask_surrogate_repair` inside the same suite.

## 4. Working-tree intent audit — every change accounted for

- **HEAD unchanged**: `git log --oneline -1` → `6632014`; `git worktree list` → the single main worktree; `git stash list` → empty. Nothing committed, pushed, PR'd, or CI'd this run.
- **15 modified tracked files, +883/−110 (`git diff --numstat`)**: 13 are exactly the cycle-4 implement phase's changed-files list (`lib.rs` +110/−4, `session_cache.rs` +95/−9, `parser.rs` +93/−3, `entrypoints.rs` +100, `history.rs` +57/−5, `reports.rs` +57/−2, `main.rs` +33/−1, `insights.rs` +17/−1, `discovery_contract.rs` +29/−3, `demo_contract.rs` +14/−12, `pricing.rs` +6/−1, `presentation.rs` +1/−4 disclosed drive-by, `docs/guides/ci-integration.md` +5) plus `CHANGELOG.md` +11 (implement) and `ROADMAP.md` +255/−65 (this run's roadmap phase).
- **12 untracked files, each mapped**: this run's phase records — `docs/research/2026-09-02-extensions-research-pass6.md` (research), `docs/reviews/2026-09-02-adversarial-repository-assessment-pass7.md` (research/assessment), `docs/stewardship/2026-09-02-cycle4-prioritization.md` (prioritize), `docs/stewardship/2026-09-02-cycle-4-stewardship-request.md` (stewardship), `…cycle-4-implementation-record.md` (implement), `…cycle-4-targeted-tests.md` (targeted_tests), `…cycle-4-full-tests.md` (full_tests), `docs/reviews/2026-09-02-cycle4-independent-review.md` (independent_review); the implement fixture `testdata/generated/adversarial/generic-loss.jsonl`; and carried evidence artifacts — `docs/stewardship/2026-09-02-roadmap-cycle3-update.diff` (this run's roadmap phase), `…roadmap-cycle2-update.diff` (prior run `0a36c541`, untracked carryover), `…2026-09-02-reconciliation-record.md` (prior run `5d025d55`'s post-ship reconcile, written after commit `93aaf05` and therefore necessarily uncommitted). None of the 26 campaign docs committed at `93aaf05` were modified.
- **Ignored paths — correctly excluded**: only `target/`, `ci-artifacts/`, `.hermes/` appear under `git status --ignored`; no stray files anywhere else (all phase scratch is under `/tmp/fv4`, `/tmp/fv4-artifacts`, `/tmp/fv4-*.log`).

## 5. Open items (informational — none block this phase)

- **Independent review verdict `pass_with_findings`** (`docs/reviews/2026-09-02-cycle4-independent-review.md`): 7 non-blocking findings, all reproduced first-hand by the reviewer and re-confirmed unchanged this phase in behavior where observable live (F1: single-JSON-object files with a `usage` key are claimed by `parse_gemini_value` before the generic fallback and still report `Messages: 0 user` — a disclosed residual whose root cause the review pins; F2: `--no-baseline-gate` misregistered as a value flag in the Go-compat shim for flag-order placements; F3–F6: sweep/quarantine edge semantics, format-asymmetric drop signal; F7: scoped fixture evidence deviation). F1 is provisionally filed in the implementation record's future-work section; F2–F7 are inputs for the next cycle's roadmap/prioritize passes — **not silently dropped**.
- **`expect` absent** → the CI-conditional TUI real-data smoke runs only under the PTY-equivalent harness here; re-run on the self-hosted runner when observable (pairs with N5/P4-4/P4-5).
- Generic-fallback sessions (`-d dmix3`) report `confidence: low` even with zero losses — pre-existing unknown-source/model semantics, not a cycle-4 regression.

## Verdict

Final validation is green on every mandatory outcome: 17/17 runnable CI steps pass, debug and release test totals identical at **189/189**, fmt/clippy clean, all five CU-6..CU-10 outcomes reproduce live on a freshly built release binary, and the working tree contains **only** the intended cycle-4 batch plus accounted-for phase records — nothing stray, nothing committed, HEAD `6632014` unchanged.

## Reproduce

```bash
cargo fmt --check && cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace -- -D warnings
cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace        # 189 passed
cargo build --release -p agenttrace && cargo test --workspace --release   # 189 passed
AGENTTRACE_BIN=target/release/agenttrace AGENTTRACE_CI_OUT=/tmp/fv4-artifacts \
  scripts/ci/check-{output-contract,deterministic-output,report-semantics,docs-commands}.sh
AGENTTRACE_BIN=target/release/agenttrace AGENTTRACE_REAL_CLI_DIR=testdata \
  AGENTTRACE_REAL_CLI_QUERY=internal/ws AGENTTRACE_REAL_CLI_FILE_LIMIT=20 \
  AGENTTRACE_REAL_CLI_OUT=/tmp/fv4-artifacts/real-cli-smoke scripts/ci/check-rust-real-cli-smoke.sh
scripts/ci/check-{release-surfaces,cargo-manifests,plugin-version}.sh
ruby -c homebrew/Formula/agenttrace.rb && npm --prefix npm test && bash -n scripts/record-demo.sh scripts/ci/*.sh
python3 /tmp/tui-pty-smoke.py target/release/agenttrace           # conditional TUI step, equivalent
B=target/release/agenttrace
$B /tmp/fv4/mix3.jsonl --overview | grep Messages                 # CU-6: 3 user (was 2)
$B -d /tmp/fv4/dfix --overview | grep -i dropped                  # CU-6: Dropped lines row
$B /tmp/fv4/bom.jsonl --overview && $B /tmp/fv4/utf16bom.jsonl --overview   # CU-7: 0 / named UTF-16 error
$B -d /tmp/fv4/dreal --overview -f json --baseline /tmp/fv4/base-zero.json \
  --baseline-max-token-delta-pct 1 -o /tmp/fv4/reg.json; echo $?  # CU-8: 2; + --no-baseline-gate → 0
# CU-9: sandboxed AGENTTRACE_SESSION_CACHE_DIR / AGENTTRACE_HISTORY_DIR per §3
```

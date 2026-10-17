# Cycle 4 Full-Tests Record — authoritative validation suite on the implement tree

- **Run**: `2a15625945fc40419fc4691c59b42a7b`
- **Phase**: full_tests (`full_tests:full_tests`), intent full_validation
- **Attempt**: `3757cf2fa341403d92b807743f3a7a0d`
- **Date**: 2026-09-02
- **Tree under test**: working tree at HEAD `6632014` carrying the uncommitted cycle-4 batch (CU-6..CU-10) exactly as described in `2026-09-02-cycle-4-implementation-record.md`; no source files changed in this phase (git status identical to implement-phase end + this record).

## Definition of "authoritative full suite"

Enumerated from `.github/workflows/ci.yml` (the repo's CI, now `runs-on: self-hosted`). Every **required** step was run locally with the workflow's exact commands/env; the single **conditional** step (Rust TUI real-data smoke, gated by `if: env.AGENTTRACE_TUI_REAL_DIR != ''`) was additionally covered by an equivalent PTY harness because this host lacks `expect` (no root, no install path).

## Required members — all PASS

| CI step | Command run | Result |
|---|---|---|
| Check Rust formatting | `cargo fmt --check` (also `--all --check`) | clean |
| Clippy | `cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace -- -D warnings` (also the stricter `--all-targets`) | clean, zero warnings |
| Rust tests | `cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace` (and full-workspace `cargo test`) | **189 passed, 0 failed** across 9 targets (12+3+2+64+7+61+40 core/cli/tui + 2 empty bins) |
| Build CLI | `cargo build --release -p agenttrace` | clean; `/tmp/agenttrace` refreshed, `--version` → `agenttrace v0.0.0-dev` |
| Single binary entrypoints | `cargo test -p agenttrace --test entrypoints` | ok, 3 passed |
| Output contract | `AGENTTRACE_BIN=target/release/agenttrace AGENTTRACE_CI_OUT=… scripts/ci/check-output-contract.sh` | PASS |
| Deterministic demo output | same env, `check-deterministic-output.sh` | PASS |
| Report semantic consistency | same env, `check-report-semantics.sh` | PASS |
| Release surface drift | `scripts/ci/check-release-surfaces.sh` | PASS |
| Documented command smoke | same env, `check-docs-commands.sh` | PASS |
| Rust CLI fixture smoke | CI env (`AGENTTRACE_REAL_CLI_DIR=testdata QUERY=internal/ws LIMIT=20`) `check-rust-real-cli-smoke.sh` | PASS — `source_dir=testdata sampled_files=20 query=internal/ws` |
| Homebrew formula syntax | `ruby -c homebrew/Formula/agenttrace.rb` | Syntax OK |
| npm package contract | `npm --prefix npm test` | tests 1, **pass 1, fail 0** |
| Cargo manifests | `scripts/ci/check-cargo-manifests.sh` | PASS |
| Codex plugin version sync | `scripts/ci/check-plugin-version.sh` | PASS |
| Helper script syntax | `bash -n scripts/record-demo.sh scripts/ci/*.sh` | OK |

Aggregator `scripts/ci/check-rust-release-local.sh` (fmt → clippy → tests → release build → manifests → entrypoints → 4 contract scripts → real-cli-smoke → release-surfaces → ruby -c → bash -n → tui-real-smoke): every member it chains **passed**, verified from its `set -x` log; it then exits 1 at the final member because `expect` is absent on this host.

## Conditional member — covered by equivalent PTY harness

`check-rust-tui-real-smoke.sh` requires `expect` (hard `fail "expect is required"`); this host has no `expect`, no `python3-pexpect`, and no root to install. CI itself only runs this step when `AGENTTRACE_TUI_REAL_DIR` is set, so it is not a required member of the authoritative suite. To leave no coverage hole on the TUI path (touched by this cycle via `presentation.rs` and the report surfaces it renders), the same PTY session was reproduced with python3 stdlib (`pty`/`select`):

- 50×196 `xterm-256color` pty, isolated `AGENTTRACE_SESSION_CACHE_DIR`, 2 real session files copied from `~/.pi/agent/sessions` — mirroring the script.
- Identical key sequence and frame markers: `AgentTrace`, `Look here first`, `Why look here`, Enter → `Summary`, → → `What happened`, Esc, `v` → `Switch view`, Esc, `f` → `Filter sessions` + `Context risk`, Esc, Ctrl-K → `Open Look here first`, Esc, `?` → `Keys`, Esc, `q`.
- Result: **all 12 markers rendered in order; TUI exited rc=0 after scripted `q`**; run twice consecutively, both PASS.
- Harness subtleties (documented for reproduction): markers must be matched against a **cumulative** escape-stripped buffer with **whitespace removed** — the TUI renders word gaps via cursor positioning (e.g. `Context risk` arrives as `Contextrisk` in the byte stream).
- Harness lives at `/tmp/tui-pty-smoke.py` (kept out of the repo: adding it to `scripts/ci/` would alter release-surface/baseline expectations and is out of this phase's scope — recorded as provisional future work instead). Full harness source is embedded in the conductor record of this phase.

## Verdict

The authoritative full validation suite passes on the cycle-4 tree: 189/189 tests, fmt/clippy clean, release build current, all 16 required CI steps green, aggregator green through every member it can run, and the conditional TUI real-data smoke demonstrated passing via an equivalent PTY harness (twice). No regressions found; nothing needed fixing; no source files modified this phase.

## Reproduce

```bash
cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace        # 189 passed
cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace -- -D warnings
cargo fmt --check && cargo build --release -p agenttrace
AGENTTRACE_BIN=target/release/agenttrace AGENTTRACE_CI_OUT=/tmp/ci-artifacts \
  scripts/ci/check-{output-contract,deterministic-output,report-semantics,docs-commands}.sh
AGENTTRACE_BIN=target/release/agenttrace AGENTTRACE_REAL_CLI_DIR=testdata \
  AGENTTRACE_REAL_CLI_QUERY=internal/ws AGENTTRACE_REAL_CLI_FILE_LIMIT=20 \
  AGENTTRACE_REAL_CLI_OUT=/tmp/ci-artifacts/real-cli-smoke scripts/ci/check-rust-real-cli-smoke.sh
ruby -c homebrew/Formula/agenttrace.rb && npm --prefix npm test
scripts/ci/check-{release-surfaces,cargo-manifests,plugin-version}.sh
bash -n scripts/record-demo.sh scripts/ci/*.sh
AGENTTRACE_BIN=target/release/agenttrace scripts/ci/check-rust-release-local.sh   # green until expect-gated member
python3 /tmp/tui-pty-smoke.py target/release/agenttrace                          # equivalent TUI smoke
```

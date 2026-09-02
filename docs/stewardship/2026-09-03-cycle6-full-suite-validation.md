---
type: stewardship-record
id: cycle6-full-suite-validation
cycle: 6
phase: full_tests
date: 2026-09-03
base_commit: 696206f
record_kind: ce-handoff/v1
status: full-suite-green
---

# Cycle 6 full-suite validation — CI `test` job replicated locally

Attempt `3df85a63ef5641a8bc201a74bcd207cf` (run
`a24bcf084cf049208c75d2cb4f3a3755`). The repository's authoritative full
validation suite is the `test` job in `.github/workflows/ci.yml`; every
runnable step was executed locally, in workflow order, on the uncommitted
cycle-6 tree (HEAD `696206f`, 14 modified files +1098/−41, 14 untracked
stewardship artifacts, tracking `fork`). Actual GitHub Actions execution is
the prohibited `ci` gate and was not touched.

## Result: 17 executed steps PASS, 0 FAIL, 1 conditional skip

| # | Step (ci.yml) | Command | Result |
|---|---|---|---|
| 1 | Check Rust formatting | `cargo fmt --check` | PASS |
| 2 | Portable CI runners only (new, CU-17) | `scripts/ci/check-no-self-hosted.sh` | PASS |
| 3 | Clippy | `cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace -- -D warnings` | PASS |
| 4 | Rust tests | `cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace` | PASS — 212 passed, 0 failed (incl. 2 empty doc-test suites) |
| 5 | Build CLI | `cargo build --release -p agenttrace` | PASS |
| 6 | Single binary CLI and TUI entrypoints | `cargo test -p agenttrace --test entrypoints` | PASS — 6 passed (incl. the targeted-tests-phase `--compare` twin-string pin) |
| 7 | Demo and report output contract | `check-output-contract.sh` (release bin) | PASS |
| 8 | Deterministic demo output | `check-deterministic-output.sh` | PASS |
| 9 | Report semantic consistency | `check-report-semantics.sh` | PASS |
| 10 | Release surface drift | `check-release-surfaces.sh` | PASS |
| 11 | Documented command smoke tests | `check-docs-commands.sh` | PASS (pins the guide's schema-17 claims) |
| 12 | Rust CLI fixture smoke | `check-rust-real-cli-smoke.sh` (`AGENTTRACE_REAL_CLI_DIR=testdata`, 20 files, query `internal/ws`) | PASS |
| 13 | Rust TUI real-data smoke | env-gated on `AGENTTRACE_TUI_REAL_DIR` | SKIPPED (unset — identical to CI's default branch run) |
| 14 | Validate Homebrew formula syntax | `ruby -c homebrew/Formula/agenttrace.rb` | PASS |
| 15 | Test npm package contract | `npm --prefix npm test` | PASS — 1 test, 0 fail (runs bare; no node_modules needed) |
| 16 | Validate Cargo manifests | `scripts/ci/check-cargo-manifests.sh` | PASS |
| 17 | Codex plugin version sync | `scripts/ci/check-plugin-version.sh` | PASS |
| 18 | Validate helper scripts | `bash -n scripts/record-demo.sh scripts/ci/*.sh` | PASS |

Logs retained at `/tmp/c6-full-ci/*.log` (17 files) during this session.
The only "error" strings in any log are test names that contain the word
(`utf16_files_fail_with_a_named_encoding_error`, etc.), all `ok`.

No regressions were found, so nothing needed fixing this phase. The two
guard/wiring surfaces added by cycle 6 (`check-no-self-hosted.sh` as a CI
step, and the additive doctor/audit JSON fields) are covered by steps 2,
7, 8, 9, and 11 respectively — the suite passed with them in place.

Not locally runnable (GitHub-hosted only, out of scope by design):
`release.yml`/`dependency-review.yml` jobs and the artifact upload.

Tree state after the suite: unchanged (`git status` 28 entries, diff
+1098/−41, HEAD `696206f`, `branch.master.remote=fork`); the suite wrote
only to `/tmp` and `target/`.

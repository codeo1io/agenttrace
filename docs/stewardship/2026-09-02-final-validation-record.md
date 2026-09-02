# Final Validation Record — 2026-09-02

- **run**: 5d025d55b1194dd1a4dd8784146dfeeb
- **phase**: final_validation (attempt a1047671aab54ef9a4f7d87863236051)
- **tree**: single worktree `/work/projects/agenttrace` at HEAD `e005952` (master); nothing committed or pushed — cycles 1–3 remain uncommitted working-tree changes, as required.
- **skill routing**: `ce-work` (narrowest match; no dedicated verify/validate skill — ce-proof/ce-dogfood/ce-sweep are browser/doc tools; ce-work's end-to-end + local-verification discipline applied).

## 1. Full CI suite, run locally in CI order (`.github/workflows/ci.yml`)

| # | Step (command) | Result |
|---|---|---|
| 1 | `cargo fmt --check` | exit 0 |
| 2 | `cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace -- -D warnings` | exit 0, 0 warnings |
| 3 | `cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace` | exit 0 — 180 passed / 0 failed (12+2+2+40+57+60+7; 2 doc-test-style suites 0) |
| 4 | `cargo build --release -p agenttrace` | exit 0 |
| 5 | `cargo test -p agenttrace --test entrypoints` | exit 0 — 2 passed |
| 6 | `AGENTTRACE_BIN=target/release/agenttrace AGENTTRACE_CI_OUT=ci-artifacts scripts/ci/check-output-contract.sh` | exit 0 |
| 7 | `… scripts/ci/check-deterministic-output.sh` (chained, as CI runs it) | exit 0 — the load-dependent flake fixed in the full_tests phase did not recur |
| 8 | `… scripts/ci/check-report-semantics.sh` | exit 0 |
| 9 | `scripts/ci/check-release-surfaces.sh` | exit 0 |
| 10 | `… scripts/ci/check-docs-commands.sh` | exit 0 |
| 11 | `… AGENTTRACE_REAL_CLI_DIR=testdata QUERY=internal/ws FILE_LIMIT=20 scripts/ci/check-rust-real-cli-smoke.sh` | exit 0 |
| 12 | `ruby -c homebrew/Formula/agenttrace.rb` | Syntax OK |
| 13 | `npm --prefix npm test` | exit 0 — pass 1 / fail 0 |
| 14 | `scripts/ci/check-cargo-manifests.sh` | exit 0 |
| 15 | `scripts/ci/check-plugin-version.sh` | exit 0 (plugin.json v0.7.1 == CHANGELOG v0.7.1) |
| 16 | `bash -n scripts/record-demo.sh scripts/ci/*.sh` | exit 0 |
| + | `cargo test --workspace --release` (repo baseline convention: debug **and** release) | exit 0 — 180 passed / 0 failed |

**Conditional step not run:** "Rust TUI real-data smoke" is gated on `env.AGENTTRACE_TUI_REAL_DIR != ''` and requires `expect`, which is absent on this host (`command -v expect` → not found). CI skips it under the same condition; this is a host limitation, not a regression. All 8 non-TUI check scripts pass.

## 2. Headline acceptance criteria re-verified (release binary)

- **P6-1 (UTF-16 escape-repair panic)**: byte-exact reproducers written via quoted heredoc — `{"prompt":"\u中文测试"}` and `{"prompt":"\ud800"}` — through `--overview/--doctor/--waste/--sessions/--latest` positional invocations: all ten runs exit **1** with `Error: unsupported session format: <path>`; `grep -c panicked` = 0 for both files. (Pre-fix behavior: exit 101, `panicked at parser.rs:3785:28`.)
- **CU-3 (version claim integrity)**: `./target/release/agenttrace --overview --version` → `agenttrace v0.0.0-dev`, exit 0.

## 3. Working-tree intent audit — every change accounted for

- **HEAD unchanged**: `git log --oneline -1` → `e005952`; `git worktree list` → the single main worktree; nothing committed/pushed.
- **22 modified tracked files** map to: cycle-3 batch surfaces (parser.rs, lib.rs, sqlite_sessions.rs, session_cache.rs, main.rs, discovery_contract.rs, entrypoints.rs), full_tests determinism fix (insights.rs, reports.rs, demo.rs), cycles 1–2 (pricing.rs, waste.rs, governance.rs, discovery.rs, TUI trio), and campaign docs (ROADMAP.md +610, CHANGELOG.md +29, PRIVACY.md 1-line offline-pricing clarification), plus CI wiring (`.github/workflows/ci.yml` +3 = the Codex plugin-version step) and hygiene (`.gitignore`: dropped stale `apps/desktop` entries, added `.hermes/`).
- **Untracked additions** map to: campaign records (docs/decisions, docs/ideation, docs/research pass-4/5, docs/reviews ×12, docs/stewardship ×10 + one .diff), adversarial corpora (testdata/generated/adversarial/* including cycle-3's unicode-escape.jsonl and the sqlite overflow/wrap fixtures), tooling (scripts/ci/check-plugin-version.sh, scripts/fixtures/make-adversarial-sqlite.py, scripts/pricing/update-snapshot.sh), and new tests/fixtures (crates/agenttrace-cli/tests/launch_guards.rs, crates/agenttrace-core/src/pricing_snapshot.json).
- **Ignored, i.e. correctly excluded**: `target/`, `ci-artifacts/` (.gitignore:3), `.hermes/` (conductor state). No stray files outside these.

## 4. Open items (informational — not blockers for this phase)

- Independent review `docs/reviews/2026-09-02-cycle3-independent-review.md` records informational findings (clap parse precedes the version hoist for `--gate 200 --version`; placeholder-title fallback in the stale-snapshot path; M1/L2 reproduced behaviors) — all documented with reproducers, none regressions.
- `expect` absent → TUI real-data smoke cannot run on this host (conditional in CI).
- ROADMAP hardening/capability lanes carry the remaining open items (N5, P4-5, candidates 33–36) for future cycles.

**Verdict: final validation green on every mandatory outcome; working tree contains only intended, accounted-for changes.**

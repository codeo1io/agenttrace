# Cycle 4 Implementation Record — "Truthful reads, truthful gates, durable records"

- **Run**: `2a15625945fc40419fc4691c59b42a7b`
- **Phase**: implement
- **Attempt**: `211845f7e0cc4d3dbbf4385dc874fbf9`
- **Date**: 2026-09-02
- **Baseline at start**: HEAD `6632014` (workflows-only atop pass-7 baseline `93aaf05`), tree clean except prior-phase docs (`ROADMAP.md` modified + 6 untracked stewardship/research/review artifacts)
- **Batch**: CU-6..CU-10 as selected in `2026-09-02-cycle4-prioritization.md` and scoped by `2026-09-02-cycle-4-stewardship-request.md`
- **Prohibited stages observed**: nothing committed, pushed, PR'd, or CI'd. All changes are working-tree only.

## Scope delivered

| Unit | Finding | Status |
|------|---------|--------|
| CU-6 | P7-1 silent line loss in the generic-JSONL fallback (lead) | Done, verified red→green |
| CU-7 | P7-2 BOM/UTF-16 handling at the parse entry | Done, verified red→green |
| CU-8 | P7-3 baseline thresholds gating the exit code | Done, verified red→green |
| CU-9 | P7-5 atomic writes + orphan temp sweep (pricing/history) | Done, verified live |
| CU-10 | backslash parity in `repair_lone_surrogates` + snapshot schema decision | Done, decision: bump 5→6 |

## Changed files (all uncommitted)

Code:

- `crates/agenttrace-core/src/lib.rs` — CU-6: `Event.usage` gains `alias = "usage"` + `deserialize_with = "deserialize_usage_map"` (lib.rs:190-197, helpers at :194-226) which coerces numeric strings via `parser::number_as_i64` and flattens Event-typed nested objects (`"input":{"tokens":5}` → `input_tokens:5`); `parse_jsonl_session` (lib.rs:423) now routes lines through `parser::parse_jsonl_value_lenient` and counts every loss into `Metrics.line_skips` (field at :347, serde `skip_serializing_if` empty) with reasons `unparseable_line` / `event_schema` / `non_event`, injected into the session after `session_from_events`. Tests at :1585-1630.
- `crates/agenttrace-core/src/insights.rs` — `DataHealth.line_skips` (additive, skip-if-empty) aggregated per reason in `data_health()`; any lost line now also caps `confidence` at `low`.
- `crates/agenttrace-core/src/reports.rs` — `BaselineBreaches` struct (:28, `.any()` helper) returned by `add_baseline_comparison` (:103) so the booleans can gate; `line_skips_cell` (:43) renders `Dropped lines: unparseable_line=1` additively in text (:571), Markdown (:600), and HTML (:622) overviews. Clean corpora produce byte-identical reports.
- `crates/agenttrace-core/src/parser.rs` — CU-7: `parse_session_file` reads bytes, names UTF-16 (`... is UTF-16 encoded; convert it to UTF-8 and retry`, :28) for FF FE / FE FF, keeps the read-error context for other failures; `parse_raw_session` strips one U+FEFF at offset 0 and nowhere else (:79). CU-10: escaped-backslash pairs advance together in `repair_lone_surrogates` so literal `\\uXXXX` text is never rewritten. `parse_jsonl_value_lenient` promoted to `pub(crate)` (:3792). Tests at :4197/:4217/:4245.
- `crates/agenttrace-cli/src/main.rs` — CU-8: `--no-baseline-gate` flag (:99, also added to the Go-flag shim whitelist), breach gate after the existing health gate (:437) exits 2 naming each breached threshold plus the opt-out; report JSON still prints before the gate fires. Test-args literal updated.
- `crates/agenttrace-core/src/session_cache.rs` — CU-9: `sweep_orphaned_temps` (:272, `pub(crate)`, age > `ORPHAN_TEMP_MAX_AGE` = 1h) called from `load_session_cache` (:327); `unique_temp_path` promoted `pub(crate)`. CU-10: `SQLITE_SNAPSHOT_SCHEMA_VERSION` 5 → 6 (:13) with rationale comment. `GoMetrics.line_skips` (Go name `LineSkips`) round-trips the new field. Tests: sweep (:1053-ish), schema-six round-trip + v5 rejection.
- `crates/agenttrace-core/src/pricing.rs` — CU-9: `write_pricing_cache` stages via `unique_temp_path` + rename (:337).
- `crates/agenttrace-core/src/history.rs` — CU-9: `preserve_derived_history` atomic (:50); `records_from_bytes` (:79) quarantines a torn/corrupt `history.json` to `history.json.corrupt` with a visible stderr warning instead of silently wiping it. Test at :217.
- `crates/agenttrace-tui/src/presentation.rs` — 1-line drive-by: `clippy::useless_format` on the provider/model row label fires under the current toolchain (rust-clippy 1.98.0) on code byte-identical to HEAD; fixed to keep `clippy --all-targets -D warnings` green. Disclosed, not part of the batch.
- `crates/agenttrace-core/tests/demo_contract.rs`, `tests/discovery_contract.rs`, `crates/agenttrace-cli/tests/entrypoints.rs` — updated for the new `add_baseline_comparison` return; new contract + CLI gate tests.

Fixtures, docs, changelog:

- `testdata/generated/adversarial/generic-loss.jsonl` — committed P7-1 reproducer: recovered lone-surrogate line, coerced Event-typed usage line, counted unparseable line.
- `docs/guides/ci-integration.md` — baseline step documents exit-2 semantics + `--no-baseline-gate`.
- `CHANGELOG.md` — Unreleased entries for all five units + the drive-by lint fix and the new fixture.

## Verification matrix (all run from the working tree, 2026-09-02)

| Check | Command | Result |
|-------|---------|--------|
| Debug tests | `cargo test` | **189 passed, 0 failed** (baseline 180 + 9 new: 1 lib usage-coercion, 1 lib fallback, 3 parser, 1 contract fixture, 1 CLI gate, 1 sweep, 1 quarantine; demo_contract baseline test extended) |
| Release build | `cargo build --release` | Finished clean |
| fmt | `cargo fmt --all --check` | Clean |
| clippy | `cargo clippy --all-targets -- -D warnings` | Clean |
| check-cargo-manifests.sh | `AGENTTRACE_BIN=/tmp/agenttrace` | PASS |
| check-deterministic-output.sh | " | PASS |
| check-docs-commands.sh | " | PASS |
| check-output-contract.sh | " | PASS |
| check-plugin-version.sh | " | PASS |
| check-release-surfaces.sh | " | PASS |
| check-report-semantics.sh | " | PASS |
| check-rust-real-cli-smoke.sh | " | PASS |
| check-rust-release-local.sh | — | Passes every sub-step; script exit is 1 only because it chains `check-rust-tui-real-smoke.sh`, which requires `expect` — absent on this host (pre-existing host limitation; recorded in the cycle-3 record too) |
| agenttrace-tui lib | `cargo test -p agenttrace-tui --lib` ×3 | 40/40 ×3 (one transient failure observed during a mid-rebuild run; reproducibly green since) |

## Live reproducer evidence (release binary `/tmp/agenttrace`)

- **P7-1** (`/tmp/pri4/mix3.jsonl`, 3 lines incl. one lone-surrogate): before `Messages: 2 user` with data_health byte-identical to the clean file; after: **3 user messages**, zero losses (line recovered). A sibling with an unrecoverable `\uzzzz` line reports `metrics.line_skips {"unparseable_line":1}` and the overview text prints `Dropped lines: unparseable_line=1` (same row in Markdown/HTML/JSON), confidence `low`.
- **P7-2** (`/tmp/pri4/bom.jsonl`, UTF-8 BOM + claude_code-shaped JSONL): before exit 1 "unsupported session format"; after: **parses**, session listed. A UTF-16LE file fails with `Error: session file utf16.jsonl is UTF-16 encoded; convert it to UTF-8 and retry`, exit 1.
- **P7-3** (forged `base-zero.json`, `--baseline-max-token-delta-pct 1`): before exit 0 with `tokens_above_threshold: true`; after: **exit 2**, stderr names all three breached thresholds, the full report JSON still prints, and `--no-baseline-gate` restores exit 0 while keeping `baseline_comparison` in the JSON.
- **P7-5** (sandboxed `AGENTTRACE_SESSION_CACHE_DIR`/`AGENTTRACE_HISTORY_DIR`): an orphaned `sessions.json.tmp.99999.0` backdated 2h is removed by the next cache-loading run while `unrelated.txt` survives; a torn `{"torn": ` history file under `--include-history` prints the quarantine warning and is preserved byte-for-byte as `history.json.corrupt`, with history starting empty instead of silently zeroed.

## Design decisions and discoveries

1. **`Metrics`, not `Session`, carries `line_skips`** — all 25 `Metrics` construction sites use `..Metrics::default()`, so the additive field breaks none of them; `Session` has no `Default` and 34 literal sites. `GoMetrics` gained the field (Go name `LineSkips`) so cache round-trips preserve the counts.
2. **Usage coercion moved into `Event`'s deserializer**, not a retry loop in the fallback — every strict-JSON path (JSON-array ingestion, hermes-style lines) now tolerates the same shapes instead of only the generic fallback, and the strict drop surface disappears rather than being patched per-parser.
3. **Honesty correction to pass 7, recorded**: string-typed usage (`"input_tokens":"5"`) never reached the P7-1 drop surface at all — it was already coerced by the single-JSON-value detector family that claims such files. The verified P7-1 shapes are the multi-line generic-JSONL ones (lone surrogate, Event-typed usage); the fixture pins exactly those.
4. **Usage metrics count from `meta`/`session_meta` events only** (lib.rs:628-651) — a deliberate pre-existing contract (SAMPLE_JSONL encodes it). The fixture's usage line therefore uses `"role":"meta"`; coercion is asserted through the same path real corpora use.
5. **`strusage`/`evusage` single-line probes behave unchanged**: they are claimed before the generic fallback by a single-JSON-value detector that builds a source-less session (source_tool `""`, 0 user messages, strusage tokens coerced). Out of CU-6's named surface; filed below as provisional work.
6. **Quarantine, not delete**: a torn history file is renamed `history.json.corrupt` with a visible warning — the bytes are evidence; deleting them would repeat the silent-loss mistake the unit fixes.
7. **Snapshot schema bumped 5→6, not "compatible"**: cycle 3 changed naming semantics while leaving the version at 5, so v5 snapshots can carry placeholder-derived names under new semantics; the cheap, correct move is regeneration.
8. **`add_baseline_comparison` returns `(String, BaselineBreaches)`**: the report and the gate read the same computation; no re-parsing of report JSON to derive the exit code.
9. **Backdated-mtime sweep test avoids sleeps**; production age is a generous 1h so a live writer is never raced.

## Provisional future work (for the next research/prioritize passes)

- **Single-JSON-value detector honesty** (from discovery 3 above): `strusage`/`evusage`-shaped files produce `source_tool: ""` sessions with no user messages on some paths — same silent-shape-loss family as P7-1, different surface. Needs its own adversarial pass.
- **`--no-baseline-gate` on the Go-flag shim** was added to the whitelist; the TUI/lang surface matrix (check-rust-tui-real-smoke) could not run on this host (`expect` missing) — re-run under the self-hosted CI runner once observable (pairs with N5/P4-4/P4-5).
- **`line_skips` in `--sessions` per-session table**: currently only in JSON metrics and the overview surfaces; a per-row indicator could follow if users ask.
- The clippy toolchain drift (1.98 firing on HEAD code) suggests pinning a clippy version in CI to keep `-D warnings` reproducible across runner images.

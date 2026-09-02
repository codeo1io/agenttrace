---
type: stewardship-record
id: cycle5-implementation
cycle: 5
batch: honest-coverage-honest-cache-honest-math
date: 2026-09-03
base_commit: 998ade8
record_kind: ce-handoff/v1
status: implemented-uncommitted
---

# Cycle 5 implementation record — CU-11..CU-16

Executed in the working tree on top of HEAD `998ade8` (master, PR #282 open);
nothing committed, pushed, or PR'd (Conductor owns topology). All evidence
below was re-run after the last code edit on the rebuilt release binary.

## What shipped

### CU-11 — governance coverage honesty (F8-1, lead)

- Governance-class commands (`--audit`, `--recommend`, `--mcp-governance`,
  `--context-trends`, `--delivery-evidence`, `--compare`) audit **every**
  matching session by default; the `.take(args.limit)` filters at the old
  `main.rs:225` and `main.rs:249` are gone.
- New `--sample N` flag: explicit bounded sampling of the newest N sessions,
  rejected for N=0, always disclosed.
- Every governance report carries coverage keys: `audited_sessions`,
  `total_sessions`, `excluded_reason` (JSON) and a leading
  `(auditing N of M sessions)[; reason]` line (text/Markdown/HTML via
  `audit_coverage_phrase`). `--recommend` (was a bare array) is wrapped as
  `{"recommendations": [...]}`; `--compare` JSON (was a bare array) is wrapped
  as `{"sessions": [...]}` — the CLI-smoke pin was updated to the new
  contract.
- `--limit` is now purely a display cap for list views. The overview's
  `recent_sessions` honors it through `report_overview_json_with_context`'s
  new `display_limit` parameter (`min(limit, 10)`; closes pass-3 P3-5 for
  real). Non-JSON overviews and all aggregates are never limited. A stderr
  note fires whenever `--limit`/sampling semantics could surprise a legacy
  user (`--limit != 20` on governance class; `--limit < len` on overview).
- Live inversion (operator corpus, release binary): default `--audit
  --range all` now reports `audited=1410 total=1410 cost=701.0454` (was the
  silently sampled `3.4427`, a 203× understatement at yesterday's census);
  `--limit 2000` yields the identical totals plus the redirect note;
  `--sample 20` yields `audited=20 … reason="sampled newest 20 of 1410
  sessions (--sample 20)"`.
- Live bug found and fixed during verification: the Go-flag-compat shim did
  not know `--sample` takes a value, so `--sample 20 -f json` silently
  dropped `-f json` (exactly the F8-8 failure class). `--sample` is now in
  `flag_takes_value` with a shim unit test.

### CU-12 — truthful `discovered` and parse/scope split (F8-2)

- `DataHealth` gains `out_of_scope` (loader-discovered sources excluded by
  range/filters) and `non_finite_costs`; `skipped` is loader parse failures
  only.
- New `data_health_scoped(sessions, discovered, parse_failures, cache_hits)`
  builds health from the real `LoadReport`; legacy `data_health` keeps its
  signature for callers without a loader (TUI view, explicit paths) and is
  documented as folding scope into `skipped`.
- The CLI overview branch now passes the loader's `discovered` instead of
  fabricating `sessions.len() + skipped`.
- Renderers share `parse_coverage_phrase` (text/Markdown/HTML): out-of-scope
  sources are disclosed inline, and multi-session (SQLite snapshot) sources
  render as "1410 sessions from 364 sources" instead of a nonsensical
  "1410/364 parsed". When nothing is out of scope the phrase is
  byte-identical to the pre-cycle rendering, so the `--demo` goldens and the
  determinism check are unchanged.
- Live inversion: `--overview --range 1d` now reports `discovered=364,
  parsed=73, out_of_scope=291` (was `discovered=71`); `--range all` reports
  `1410 sessions from 364 sources, 0 skipped`.

### CU-13 — cache eviction (F8-3)

- `prune_dead_entries` runs when the cache loads: entries whose source file
  no longer exists and dir listings whose directory is gone are removed,
  marking the cache dirty so the next save persists the shrink.
- `MAX_SESSION_CACHE_ENTRIES = 20_000`: `enforce_entry_bound` drops
  oldest-source-mtime entries first at save time. Both are unit-tested
  (dead-path prune; deterministic-mtime bound eviction, including
  idempotence).
- `save_session_cache` now takes `&mut SessionCache` (only internal callers).
- Live evidence across one load cycle on the operator cache:
  `737 entries → 721`, `334 dirs → 333`, dead paths `16 → 0`,
  snapshot `9,366,610 → 9,340,395` bytes.

### CU-14 — float hygiene + single percentile (F8-5, F8-6)

- `json_float` renders non-finite values as `null` instead of
  `.expect("float serializes")` panicking; `DataHealth.non_finite_costs`
  counts poisoned sessions and forces confidence `low`.
- `convert_litellm` skips entries whose ×1e6-scaled rates are non-finite
  (f64::MAX inputs), so hostile catalogs fall back to default pricing
  instead of poisoning totals (unit-tested with a hostile fixture).
- The divergent `reports.rs` percentile copy (`(len-1)*p`, rounded — p95 of
  20 values differed from the Go-pinned rule) is deleted; all four call
  sites use `crate::percentile` (`len*p`, truncating, pinned by
  `percentile_matches_go_index_rule`). A source-level test guards against
  re-introduction.

### CU-15 — docs honesty + enforced docs contract (F8-7, F8-8)

- `docs/guides/governance-reports.md`: the "24h automatic refresh" claim is
  replaced by the truth (normal runs are network-free; the cache is served
  as-is and labeled `cache(stale)` past 24h; the only refresh is explicit
  `--update-pricing`); snapshot schema corrected 4 → 6; cache eviction and
  the 20,000-entry bound documented; governance coverage semantics
  (`audited_sessions`/`total_sessions`, `--sample`) documented.
- README (en + zh-CN): "flags go before the session path" note with the
  failing/succeeding examples, `--limit`-is-a-display-cap and `--sample`
  semantics, and audit coverage disclosure.
- `scripts/ci/check-docs-commands.sh` extended into a docs contract: greps
  the real `SESSION_CACHE_SCHEMA_VERSION`/`SQLITE_SNAPSHOT_SCHEMA_VERSION`
  constants out of `session_cache.rs` and requires the guide to state them,
  forbids "refreshed automatically"/background-refresh claims and "schema 4",
  and requires the README flag-order note. Red→green verified: the script
  exits 1 against the pre-edit guide/README (`git show HEAD:`), exits 0 now.

### CU-16 (stretch) — zstd named error (candidate 44 minimum)

- `parse_file` sniffs the zstd magic `28 B5 2F FD` and bails with
  "session file … is zstd-compressed (Codex rollout format); decompress it
  to JSONL first (e.g. `zstd -d … -o ….jsonl`)" instead of the misleading
  "not valid UTF-8". Unit + live probe confirmed.

## Verification (all after the final edit)

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (0 errors) |
| `cargo test --workspace` | **203 passed, 0 failed** (baseline 189; +14) |
| `scripts/ci/check-*.sh` (10 scripts, release binary) | **10/10 exit 0** |
| `cargo build --release` | rebuilt; all live probes on it |

New tests: CLI integration ×3 (audit==overview totals + disclosure;
explicit/disclosed sampling incl. `--sample 0` rejection and text phrase;
overview `--limit` caps `recent_sessions` only), discovery ×3
(range-independent `discovered` + out-of-scope split; dead-path prune
persisted across runs; non-finite costs lower confidence), zstd ×1,
session-cache ×2, pricing ×1, reports ×3 (`json_float` null, percentile
uniqueness/parity, parse-coverage phrase), shim ×1 (`--sample` value flag).

### Check-script pin updates (deliberate)

- `check-rust-real-cli-smoke.sh`: compare-snapshot pin now asserts
  `compare.sessions[...]` **and** `audited_sessions === sessions.length`,
  `total_sessions === sessions.length` (the old pin asserted the bare-array
  shape CU-11 deliberately replaced).
- `check-docs-commands.sh`: +29 lines (docs contract above).
- `check-report-semantics.sh`, `check-deterministic-output.sh`,
  `check-output-contract.sh`: unchanged and passing (demo output
  byte-identical; governance JSON keys are additive).

### Environment notes

- `expect` was missing from this host (present during pass-8); installed via
  `sudo apt-get install -y expect` so `check-rust-tui-real-smoke.sh` (and the
  release-local chain) could run for real rather than being skipped.
- `cargo audit` remains unavailable offline (noted in prior phases; no new
  dependency was added in this batch).

## Files touched (uncommitted)

Code: `crates/agenttrace-cli/src/main.rs`,
`crates/agenttrace-cli/tests/entrypoints.rs`,
`crates/agenttrace-core/src/{insights,lib,parser,pricing,reports,session_cache}.rs`,
`crates/agenttrace-core/tests/discovery_contract.rs`.
Docs/CI: `README.md`, `README.zh-CN.md`, `docs/guides/governance-reports.md`,
`scripts/ci/check-docs-commands.sh`, `scripts/ci/check-rust-real-cli-smoke.sh`.
(Plus this record and the earlier cycle-5 campaign docs already in the tree.)

## Deferred (unchanged from prioritization §6)

- C33+C25 capability lead and C43 pricing tiers → cycle 6 (with F8-4
  dependency lane: rusqlite 0.40.2 / SQLite 3.53.2 / MSRV 1.88, dependabot
  #278/#279 rebase after PR #282 merges).
- N8 Markdown escaping (slack rider; not needed this cycle).
- F8-9 install.sh checksum, F8-10 dangling-diff hygiene → post-merge chores.

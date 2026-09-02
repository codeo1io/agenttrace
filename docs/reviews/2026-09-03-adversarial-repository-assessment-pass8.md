# Adversarial repository assessment — 2026-09-03 (eighth pass)

Run: `dafd34b3940e497f9f1ac234573323ad` · phase `assess` · attempt `33856f75f147406687acc11c09454434`
Head: `998ade8` (worktree clean except 4 pre-existing untracked stewardship files).
Method: fresh build + full test/clippy/fmt/CI-script sweep, live runs of the release binary
against the operator's real 1407-session corpus, hand-built hostile fixtures, targeted code
reading of `main.rs`, `pricing.rs`, `governance.rs`, `insights.rs`, `discovery.rs`,
`session_cache.rs`, `sqlite_sessions.rs`, `reports.rs`, `lib.rs`, plus cross-check against
pass-7 findings so nothing below is a repeat.

Baseline before findings: `cargo build` clean, `cargo test --workspace` 189/189 green,
`cargo clippy --all-targets` silent, `cargo fmt --check` clean, and all 10
`scripts/ci/check-*.sh` scripts exit 0 on the release binary.

---

## F8-1 — HIGH (correctness, silent data-coverage truncation): governance reports audit only the newest 20 sessions by default

`--audit`, `--recommend`, `--mcp-governance`, `--context-trends`, `--delivery-evidence`
and `--compare` all take `args.limit` **after** filtering, and `--limit` defaults to 20
(`crates/agenttrace-cli/src/main.rs:122-123`, applied at `main.rs:224-225` and `main.rs:249`).
`--limit` is documented as a scope control (`docs/guides/governance-reports.md:15`) with no
coverage caveat, and the report output nowhere states how many sessions were excluded.

Live repro on this machine (release binary, `--range all`):

```
agenttrace --audit -f json --range all            -> 20 sessions,   total_estimated_cost 3.9494
agenttrace --audit -f json --range all --limit 2000 -> 1408 sessions, total_estimated_cost 695.4611
```

A user asking for an all-time audit is told the fleet cost **$3.95** when it is **$695.46**
— a 176× understatement with exit code 0 and no warning. `--overview` does *not* apply the
limit, so the tool's own overview and its audit disagree about the same data.

Fix direction: exclude governance/compare paths from the display limit (or default
`--limit` to unbounded there), and always emit an `audited_sessions` / `total_sessions`
pair in the JSON.

## F8-2 — MEDIUM (correctness, report integrity): `data_health.discovered` discards the true discovery count when `--range` is set

`LoadReport.discovered` is the honest file count (`crates/agenttrace-core/src/discovery.rs:33-39`),
but the overview path recomputes it as `sessions.len() + load_report.skipped`
(`crates/agenttrace-cli/src/main.rs:337-341`) — i.e. *post-filter* length — and throws the
field away. Because the `since` filter runs inside the loader
(`discovery.rs:270-283`), every ranged run under-reports discovery:

```
agenttrace --overview -f json --range all -> discovered 1407, parsed 1407, skipped 0   (correct)
agenttrace --overview -f json --range 1d  -> discovered   70, parsed   70, skipped 0   (1407 exist)
```

The HTML/text “Parse coverage N/M parsed; K skipped” denominator is therefore wrong
whenever a range is active, and parse failures in out-of-range files are invisible to the
coverage signal. Fix: pass `load_report.discovered` through.

## F8-3 — MEDIUM (reliability, perf, unbounded growth): the session cache never evicts entries for deleted files

Entries are only invalidated when their file is *visited* and found stale
(`session_cache.rs:597-616`); there is no prune of `entries`, `raw_entries` or `dirs` for
paths that no longer exist, and `save_session_cache` serializes everything
(`session_cache.rs:558-593`). Agent session dirs rotate constantly, so dead entries
accumulate forever, and each dirty save rewrites the whole file while each startup re-parses it.

Measured on this machine (`/home/agent/.cache/agenttrace/sessions.json`):

```
size 10,518,119 bytes · 1487 entries · 761 (51%) point to nonexistent paths · 385 dir listings
--doctor wall time ~0.55 s (dominated by parsing + rewriting this file)
```

Fix direction: drop entries whose path no longer exists during `load_session_cache`
(they cannot be cache hits), or bound the cache (LRU by entry count/bytes).

## F8-4 — MEDIUM (security, supply-chain): bundled SQLite 3.46.0 parses untrusted databases and is ~2 years stale

`rusqlite 0.32.1` / `libsqlite3-sys 0.30.1` vendor SQLite **3.46.0** (verified:
`target/.../libsqlite3-sys-*/out/bindgen.rs` → `SQLITE_VERSION = "3.46.0"`). The DB is used
to read third-party agent stores (`~/.hermes/state.db`, `~/.local/share/opencode/opencode.db`,
and anything reachable via `-d`) — attacker-influenceable file contents. Opening read-only
(`sqlite_sessions.rs:151-154`) is good hygiene but does not backport two years of upstream
parser hardening (3.47→3.5x). `ureq 2.12` (2.x branch, 3.x current) is in the same
“works but old” bucket. Recommend a dependency-refresh cycle with `cargo audit` in CI
(`cargo audit` is not currently installed here, so this pass could not run it).

## F8-5 — LOW (robustness): `json_float` panics on non-finite costs

`crates/agenttrace-core/src/reports.rs:1446-1452`: non-finite values fall through to
`serde_json::to_string(&value).expect("float serializes")`, which **errors** for `inf`/`NaN`
→ panic. Reachable without a code bug: `convert_litellm` multiplies untrusted
`input_cost_per_token` by `1e6` with no finiteness check (`pricing.rs:330-345`), so a
poisoned `AGENTTRACE_PRICING_FILE` or cache file (`input_cost_per_token: 1e304`) yields
`inf` rates → `inf` costs → panic during report rendering. The rest of the float paths are
carefully `total_cmp`/`is_finite`-guarded; this one is not.

## F8-6 — LOW (correctness): two `percentile` implementations disagree

`lib.rs:1309-1320` (index `len*p`, truncate) vs `reports.rs:1777-1782`
(index `(len-1)*p`, round). Same 20-element input, `p=0.95` → lib returns element 20,
reports returns element 19 (verified with a standalone repro). p95 in session detail can
therefore differ from the p95 used by anomaly thresholds in the same run. Consolidate on one.

## F8-7 — LOW (documentation accuracy): governance guide contradicts the code in two places

`docs/guides/governance-reports.md:52-55`:

- “the SQLite snapshot is schema **4**” — code is **6**
  (`session_cache.rs:13`, bumped 5→6 by cycle-4 CU-10).
- “A cache older than 24 hours is refreshed **automatically when a price lookup starts**”
  — the code deliberately never does this: `load_catalog_for_current_env`
  (`pricing.rs:252-259`) is documented “The only network path is the explicit
  `--update-pricing` action”, and `stale_cache_is_served_as_is_without_download_or_rewrite`
  pins it. The doc describes pre-hardening behavior and undercuts the offline-by-default
  privacy story.

## F8-8 — LOW (DX, carry-forward): flags after the positional path are still silently dropped

Re-verified live: `agenttrace testdata/kimi-tool-args.json -f json` prints a **text**
report and exits 0. Intentional Go-flag compatibility (`main.rs:516-540`, pinned by
`go_flag_compatible_args_ignore_flags_after_positional_path`, `main.rs:1138-1153`), but the
one-line README warning pass-7 recommended is still absent, so the trap remains undocumented.

## F8-9 — LOW (supply-chain): installer checksum story is inconsistent and same-origin

`install.sh:44-58` verifies no checksum (≥1 MB size floor only), while
`npm/scripts/install.js:78-104` *does* fetch and compare `<asset>.sha256` — so the two
official install paths make different guarantees. Both fetch the checksum from the same
origin as the artifact, so neither detects a compromised release; only transport corruption.
(Pass-7 recorded “installers never read the .sha256”; the npm side has since gained it —
status updated, gap moved rather than closed.)

## F8-10 — INFO (hygiene): worktree and repo-weight items

- 4 untracked files carried in the worktree (`docs/stewardship/2026-09-02-cycle-4-reconciliation.md`,
  `2026-09-02-reconciliation-record.md`, and two `*.diff` artifacts) — decision records and
  raw diffs should either be committed or gitignored, not left dangling.
- `ROADMAP.md` is 53 KB / 10 top-level sections and doubles as a per-cycle changelog;
  cycle logs already live in `docs/stewardship/`, so the roadmap can shrink to strategy.
- Micro-perf: `builtin_pricing()` rebuilds a ~75-entry `BTreeMap` (String keys) on every
  catalog-miss lookup (`pricing.rs:347-352`) — negligible per call, but trivially hoistable
  into a `OnceLock`.
- `parse_file` reads whole files with no size cap (`parser.rs:20-34`); a multi-hundred-MB
  session file is parsed in full. Pass-7 measured 20 MB fine; a cap or streaming guard
  would bound worst-case memory.

---

## Prior-pass residuals — re-verified status at `998ade8`

Fixed since pass-7 (verified in code this pass, not re-derived): P7-1 dropped lines now
surfaced (`DataHealth.line_skips` + HTML “Dropped lines” row), P7-2 UTF-16 named error
(`parser.rs:22-28`), P7-3 baseline breach exits 2 unless opted out (`main.rs:437-456`),
P7-4 `since` wired into both SQLite loaders, P7-5 pricing cache written via temp+rename,
CU-5 snapshot schema bumped (6), escaped-backslash lookbehind in surrogate repair
(`parser.rs:3822-3831`), orphaned-temp sweep (`session_cache.rs:272-302`).
Still open, unchanged: P3-4 control characters in `--sessions` rows (deferred by campaign
decision), F8-8 above.

## Surfaces probed clean this pass (fresh evidence)

- **Hostile corpus, zero panics:** all `testdata/generated/adversarial/*` plus new
  hand-built fixtures (i64::MAX token counts, year-0 and year-9999 timestamps, negative
  millis, empty session header) exit with graceful errors; token totals saturate instead
  of wrapping; costs render absurd-but-finite values rather than crashing.
- **Escaping:** `html_escape` covers `& < > " '` (`reports.rs:2349-2361`); JSON strings go
  through `serde_json` + Go-style `\u003c` post-escaping (`reports.rs:1435-1444`).
- **Subprocess:** the only `Command::new` outside tests is `git -C <root> log` with an arg
  array, no shell (`governance.rs:759-768`).
- **Filesystem:** discovery is depth-bounded and does not follow symlinked directories
  (`entry.file_type()` is not follow-aware, `discovery.rs:351-430`).
- **TUI:** width-safe truncation via `unicode-width` (`filters.rs:376-414`); ratatui is
  cell-based, so session-text ANSI sequences cannot escape the terminal.
- **CLI:** README/governance-guide commands all run green against the live corpus; the
  health/baseline gates exit 2 with actionable evidence lines.

## Assessment side effects (disclosure)

Running the release binary against the operator corpus refreshed
`/home/agent/.cache/agenttrace/sessions.json` (normal app behavior; it also made the
F8-3 numbers current). `--doctor`, `--overview`, `--audit`, `--search` were executed; no
repository source files were modified beyond this review document; nothing staged,
committed, pushed, or PR'd.

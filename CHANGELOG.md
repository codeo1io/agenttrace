# Changelog

## Unreleased

### Changed

- The generic-JSONL fallback no longer silently drops recoverable lines (pass-7 P7-1): a lone-surrogate line is repaired via the same lenient machinery the format detectors use, an Event-typed usage block (`"usage": {"input": {"tokens": 5}}`) and numeric-string usage coerce instead of failing the whole event, and a lowercase `usage` key is no longer invisible next to `Usage`. Lines that still cannot parse are counted per reason and surfaced everywhere: per-session `metrics.line_skips`, aggregate `data_health.line_skips`, and a `Dropped lines` row in the text, Markdown, and HTML overviews; any lost line also caps `data_health.confidence` at `low`.
- Baseline regression thresholds now gate the exit code (pass-7 P7-3): `--baseline` with any `--baseline-max-*-delta-pct` breach fails the run with exit 2 (mirroring `--fail-under-health`) and names the breached thresholds on stderr, where the breach booleans previously sat unread in the report JSON while the process exited 0. `--no-baseline-gate` keeps the comparison in the report without failing the build.
- Session files written with a UTF-8 BOM now parse (one BOM is stripped at the shared parse entry, and nowhere else — a U+FEFF inside content survives), and UTF-16 files fail with a named, actionable error (`... is UTF-16 encoded; convert it to UTF-8 and retry`) instead of a generic read failure (pass-7 P7-2). PowerShell 5.1's default `>` redirection produces exactly this shape.
- The pricing-catalog cache and the derived-history file now stage through a unique temp sibling and rename into place (pass-7 P7-5), extending the pass-6 atomic-write hardening to the last two raw `fs::write` callers; a torn or corrupt `history.json` is quarantined as `history.json.corrupt` with a visible warning instead of silently wiping the durable record, and orphaned `<name>.json.tmp.<pid>.<seq>` siblings are swept (age > 1 hour) whenever the session cache loads.
- The lone-surrogate repair no longer rewrites literal `\\uXXXX` text: escaped-backslash pairs advance together, so `"\\ud800"` (an escaped backslash followed by `ud800`) keeps its literal bytes while a real lone surrogate in the same line still repairs (cycle-3 residual).
- The SQLite snapshot cache schema was bumped 5 → 6: cycle 3's placeholder-name rewrite shipped while the version stayed at 5, so v5 snapshots could carry stale names under new semantics; they now regenerate (cycle-3 residual).
- Fixed a `clippy::useless_format` finding in the TUI provider/model row label under the current toolchain.

### Added

- Extended the token and cost hardening to the SQLite ingestion paths (OpenCode `opencode.db` and Hermes `state.db`), which the first hardening pass had missed: adversarial token counts now saturate instead of overflowing the per-session accumulator (a crafted database previously crashed debug builds with exit 101) or wrapping negative (`u64::MAX` previously reported `"input": -1`), and negative token columns in Hermes databases are clamped to zero. The earlier entry below claimed repo-wide coverage that this path disproved; it is now true.
- SQLite-backed OpenCode sessions now prefer the authoritative totals recorded on the session row (`cost` and the five token columns) over message-derived aggregation when present, keep the derived path for older schemas, and disclose the choice: `provenance.tokens` reports `stored_session_totals`, the per-session `stored_totals_delta` exposes how far derived aggregation drifted, and `data_health` summarizes both (`stored_totals_sessions`, `stored_totals_delta_tokens`). The SQLite snapshot cache schema was bumped (v5) so cached entries regenerate under the new semantics.
- Sessions with an unknown start time (for example OpenCode rows with `time_created = 0`) are no longer silently dropped from `--range`/`--since` views; they stay visible in an unknown-time bucket and `data_health` reports the count as `unknown_time_sessions`.
- The default TUI now fails with a normal error (`stdout is not a terminal; ... use agenttrace --overview`) instead of panicking with exit 101 when stdout is not a terminal — the README quickstart previously crashed in every piped context (CI, cron, docker without `-t`, IDE consoles).
- `--version` now wins over argument validation, including action validation: `agenttrace --lang fr --version` and `agenttrace --overview --version` print the version instead of rejecting the unsupported language or the action combination first (pass-6 P6-2).
- Fixed a crash every non-interactive surface inherited from format detection: a single log line containing a `\u` escape followed by multi-byte UTF-8 (for example `{"prompt":"\u中文测试"}`) sliced mid-character inside the lone-surrogate repair path and killed `--overview`, `--doctor`, `--waste`, `--latest`, `--sessions`, `--diagnostics`, positional files, and directory scans with exit 101 in debug and release builds. Escape hex is now read from bytes, rejected escapes pass through untouched, and the adversarial corpus gained unicode-escape reproducers plus contract tests asserting the file degrades to "unsupported format" while clean neighbors keep loading (pass-6 P6-1).
- The fallback token estimate (used when a session records no usage block) is now CJK-aware: ASCII text keeps the classic four-characters-per-token rate while every non-ASCII character counts as roughly one token, where the previous bytes-divided-by-four heuristic under-counted CJK by 40-60%. `reasoning_chars` now counts characters rather than bytes, matching the unit its name promises, and a new `Naming` provenance distinguishes `first_user_request`, `file_name`, `provider_title`, `message_derived`, `session_id`, and `provider:placeholder` session names (pass-6 P6-4, research candidate 34).
- OpenCode sessions whose recorded `title` is a `New session - <timestamp>` placeholder (every session the provider does not summarize; 227/227 in the live census) are now named from their first user message text instead of carrying the placeholder as their name; real provider titles still win, and the gate is disclosed through `provider:placeholder` naming provenance (research candidate 34).
- `--demo --overview -f json` now pins `scope.generated_at` to a fixed synthetic epoch (`2026-05-02T10:36:00Z`, just after the newest demo event) instead of the wall clock, so the deterministic-output check no longer flakes whenever two back-to-back runs straddle a second boundary; real (non-demo) reports still stamp the actual generation time.
- Session-cache and SQLite-snapshot writes now stage through a per-writer unique temp file (process id plus an in-process counter) before the same atomic rename, so two concurrent agenttrace processes can no longer race on a shared `<name>.json.tmp` and fail or tear a save (pass-6 P6-3).
- Governance cost audits no longer report `confidence: "high"` when any session in scope carries a negative token or cost component.
- Hardened token and cost aggregation against adversarial or corrupt session logs: token counts now saturate instead of overflowing, negative usage values are clamped, and reports can no longer print negative token totals or panicked and absurd costs.
- Made pricing fully offline by default: report and test paths never download anything. A dated LiteLLM snapshot (2,458 chat models, trimmed from the ~2 MB catalog to 533 KB) is bundled with the binary and used whenever no cached catalog exists; the network is touched only by the explicit `--update-pricing` command.
- Stabilized `pricing_source` labels: they no longer embed fetch or cache timestamps, so identical inputs produce byte-identical reports across runs and cache states.
- `--lang` now rejects unsupported values with an error instead of silently falling back to English.
- Removed stale `.gitignore` entries (`agentwaste`, `apps/desktop/...`) left over from an earlier layout that no longer exists in this tree, and ignore the local `.hermes/` harness-state directory.

### Added

- Committed generic-loss adversarial fixture (`testdata/generated/adversarial/generic-loss.jsonl`) pinning the pass-7 P7-1 shapes: a recovered lone-surrogate line, a coerced Event-typed usage line, and a counted unparseable line, asserted from file through `data_health`.
- A test pinning `PRICING_SNAPSHOT_DATE` to the bundled `pricing_snapshot.json` payload's `_snapshot.date`, so the const and the snapshot can no longer drift apart silently.
- Committed adversarial SQLite repro fixtures (`testdata/generated/adversarial/sqlite/`, regenerable via `scripts/fixtures/make-adversarial-sqlite.py`) with regression tests covering the overflow, wrap, negative-column, stored-totals, and unknown-time paths.

- Added `scripts/pricing/update-snapshot.sh` to regenerate the bundled pricing snapshot.
- Added `scripts/ci/check-plugin-version.sh` tying `.codex-plugin/plugin.json` to the latest CHANGELOG version so release drift is caught locally.

## v0.7.1 - 2026-07-20

### Changed

- Refreshed the redacted real-local-run GIF, TUI screenshots, and static HTML overview evidence for the v0.7.1 release surface.
- Updated release-facing metadata across the CLI, plugin, Homebrew Formula, README, Pages, and sample report.

## v0.7.0 - 2026-07-19

### Added

- Added local governance reports for cost audits, prioritized recommendations, observed MCP usage, cross-session context trends, and read-only Git delivery evidence.
- Added optional local model aliases and per-million-token pricing overrides through `AGENTTRACE_PRICING_FILE`.
- Added report scope, pricing confidence, project-root resolution, cache-aware doctor output, and governance appendices to overview exports.
- Added Action Center, Efficiency, and Delivery workspaces to the TUI, plus bilingual UI copy, live search, paste handling, and report scrollbars.

### Changed

- Updated the CLI, TUI, reports, plugin, Homebrew Formula, Pages assets, and release metadata to v0.7.0.
- Made CLI report actions mutually exclusive and hardened validation for gate thresholds and output formats.
- Made delivery and MCP output explicit about evidence limits: commit correlation is not authorship or merge proof, and invocation logs do not imply complete MCP inventory coverage.

### Validation

- Added governance, TUI interaction, discovery, report, and release-surface coverage to the Rust test and CI paths.

## v0.6.0 - 2026-07-19

### Added

- Replaced the Go implementation with a Rust workspace while preserving one
  `agenttrace` binary for both CLI reports and the terminal TUI.
- Added cached background session loading, Hermes/OpenCode SQLite sources,
  `Detailed`/`Aggregate`/`Limited` data capability labels, coverage reporting,
  privacy-safe tool-step metadata, actionable issue filters, and shared Core
  findings/comparison rules across CLI and TUI.
- Added deterministic generated parser fixtures and CI checks for provider
  coverage, data degradation, step redaction, and single-binary entrypoints.

### Changed

- Split the Rust TUI into state, presentation, filtering, and test modules and
  moved shared data-health/comparison logic into `agenttrace-core`.
- Streamed JSONL object parsing instead of retaining whole-file JSON trees,
  reducing peak memory by about 44% on a 2.53 GiB local session corpus while
  keeping report totals unchanged.
- Added `Ctrl+d`/`Ctrl+u` half-page movement and `G` end navigation to the TUI.
- Updated public docs, Pages, plugin, and Skill surfaces to describe the Rust
  implementation and honest per-source evidence limits.

### Fixed

- Invalidated SQLite session snapshots when the database, WAL, or SHM file
  changes.
- Hardened preserved-history loading against malformed short identifiers and
  stabilized the real-data CLI/TUI release smoke checks.

### Validation

- The Rust workspace, release binary, parser fixtures, CLI/TUI PTY entrypoints,
  report contracts, Homebrew syntax, and Pages artifact are covered by the
  local release gate.

## v0.5.4 - 2026-05-24

### Changed

- Published cross-platform command-line release assets and checksums.

## v0.5.3 - 2026-05-24

### Changed

- Refreshed release-facing artifacts for the v0.5 line.

## v0.5.2 - 2026-05-24

### Fixed

- Fixed Claude Code JSONL metrics for assistant messages that include thinking,
  text, and a parallel `tool_use` batch so the report keeps one assistant turn,
  multiple tool calls, cache token attribution, and failed `tool_result`
  counting aligned. (#243)

## v0.5.1 - 2026-05-19

### Fixed

- Clarified `agenttrace --doctor` cache-state wording so users can distinguish
  parsed session cache entries, entries reusable for the current scan, and
  cached directory listings. (#239)

## v0.5.0 - 2026-05-18

### Added

- Added local baseline comparison for overview reports so a later run can be
  checked against a saved local JSON baseline. (#203)
- Added incident timeline evidence to the TUI and report surfaces. (#204)
- Added tool authority summaries to HTML, Markdown, and text overview reports.
  (#210, #212, #214, #219, #221)

### Changed

- Improved overview report readability for Unicode text, incident rows, and
  terminal-readable authority summaries. (#216, #217, #219, #221)
- Aligned public README, docs, site metadata, and discovery surfaces with the
  current local coding-agent session coverage. (#197, #202, #228, #229, #230,
  #231, #232)
- Removed stale package-channel and launch-kit surfaces so release-facing
  install guidance stays limited to available channels. (#225, #226)

### Validation

- Added and refreshed release-surface, report-semantics, Pages artifact, and
  parser-coverage checks for the v0.5.0 release train. (#178, #182, #184, #205,
  #208)

## v0.4.6 - 2026-05-10

### Fixed

- Show sessions from `~/.pi/agent/sessions` as Pi while keeping
  legacy `~/.omp/agent/sessions` sessions labeled Oh My Pi.

## v0.4.5 - 2026-05-10

### Fixed

- Added PI auto-discovery for `~/.pi/agent/sessions` while keeping the legacy
  Oh My Pi `~/.omp/agent/sessions` path for compatibility.

## v0.4.4 - 2026-05-10

### Added

- Added a real local-data marketing refresh script for README and site assets.

### Changed

- Capped overview JSON anomaly details and added anomaly total/truncation metadata
  so large real histories stay readable for automation and promotional reports.
- Refreshed README and site screenshots from a real local run.

## v0.4.3 - 2026-05-10

### Changed

- Updated release surfaces for the v0.4.3 distribution.

## v0.4.2 - 2026-05-05

### Changed

- Refreshed README GIF and screenshots from a real local run with color enabled.
- Updated release surfaces for the v0.4.2 install paths.

### Fixed

- Kept Session List table values readable when terminal colors are enabled.

## v0.4.1 - 2026-05-04

### Changed

- Refreshed the README's real local-run screenshots and summary metrics from
  the latest TUI against local session logs.
- Updated release surfaces for the v0.4.1 distribution.

## v0.4.0 - 2026-05-04

### Changed

- Polished the first-run TUI demo path with clearer selected-session context,
  scan-friendly status text, refreshed demo assets, and an updated recording
  script. (#91)
- Improved TUI feedback around loading, empty diff states, and command-mode
  results so users get immediate guidance while navigating. (#122)
- Made `--waste` use the same latest-session selection behavior as `--latest`,
  reuse loaded diagnostics, and show clearer waste-report copy. (#95)

### Fixed

- Stabilized overview report ordering for recent sessions and anomaly tie
  breakers across JSON, Markdown, and HTML outputs. (#104)
- Aligned overview aggregate metrics with TUI discovery, including cache
  read/write tokens in the exported totals. (#114)
- Clamped loop waste so reported waste cannot exceed total session cost. (#116)
- Aligned TUI cache status wording with `agenttrace --doctor`. (#117)
- Isolated auto-discovery tests from runner-specific environment configuration.
  (#120)

### Validation

- Added repeatable CI gates for output contracts, deterministic demo output,
  report semantics, release surfaces, and Pages artifacts. (#118)
- Documented the launch-kit validation gates and release consistency checklist
  for public demo and install surfaces. (#115, #121)

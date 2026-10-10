<p align="center">
  <img src="assets/logo-icon.png" alt="agenttrace logo" width="256" height="256">
</p>

<h1 align="center">AgentTrace</h1>

<p align="center">
  Local-first TUI and reports for AI coding-agent session history, cost, tokens, time, and slow-run diagnosis.
</p>

<p align="center">
  English | <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/luoyuctl/agenttrace/actions/workflows/ci.yml"><img src="https://github.com/luoyuctl/agenttrace/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/luoyuctl/agenttrace/releases/latest"><img src="https://img.shields.io/github/v/release/luoyuctl/agenttrace?color=00ADD8" alt="Release"></a>
  <img src="https://img.shields.io/badge/Rust-stable-f74c00.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
  <a href="https://github.com/luoyuctl/homebrew-tap"><img src="https://img.shields.io/badge/Homebrew-tap-2bbc8a.svg" alt="Homebrew tap"></a>
  <a href="https://www.npmjs.com/package/@zack78/agenttrace"><img src="https://img.shields.io/npm/v/@zack78/agenttrace?label=npm" alt="npm"></a>
  <a href="https://github.com/microsoft/winget-pkgs"><img src="https://img.shields.io/badge/WinGet-Luoyuctl.AgentTrace-0078D4.svg" alt="WinGet"></a>
</p>

<p align="center">
  <img src="assets/readme-real-run.gif" alt="agenttrace running locally against real AI coding agent session logs" width="100%">
</p>

---

**agenttrace** is a local-first terminal TUI and report generator for AI coding-agent session history. It reads Claude Code, Codex CLI, Gemini CLI, Qwen Code, Cline, Aider, Cursor exports, Hermes Agent, OpenCode, OpenClaw, Pi and its forks (Senpi, Omo) plus agent-dir variants like `~/.pi/agent-cliproxy-only`, Oh My Pi, Kimi CLI, Copilot-style logs, and generic JSON/JSONL traces, then helps with two daily jobs: see what multiple agents spent across cost, tokens, and time; and diagnose why a task ran slowly.

One `agenttrace` binary provides both interfaces: run it without a report action to open the TUI, or pass flags such as `--sessions` and `--overview` for CLI output.

## Why agenttrace?

AI coding agents now behave like small build systems: they call tools, retry, stall, and spend tokens while you only see the final answer.

**agenttrace** reads the logs your agents already write and puts cost-heavy or slow sessions first.

It helps you answer:

- **What did my agents spend?** Compare historical sessions by agent source, model, input/output/cache tokens, estimated cost, and wall-clock time.
- **Why was this task slow?** Catch long gaps, hanging sessions, retry loops, slow tool calls, large parameters, and context pressure.
- **Did a run regress?** Compare against a local baseline when supplied, then inspect incident timelines and conservative tool authority categories in reports.
- **What should I inspect first?** Rank sessions by cost, duration, turns, health, failures, anomalies, model, source, or text search.
- **Can I inspect this privately?** Everything runs locally; prompts, code, and logs do not need to leave your machine.

## Real local run

```bash
agenttrace
```

| Overview | Critical sessions |
|---|---|
| <img src="assets/readme-real-overview.png" alt="agenttrace overview showing real local AI coding agent sessions, token cost, errors, and health" width="100%"> | <img src="assets/readme-real-critical.png" alt="agenttrace critical session list from real local AI coding agent logs" width="100%"> |

| Session detail | Diagnostics |
|---|---|
| <img src="assets/readme-real-detail.png" alt="agenttrace detail view showing health, cost, tool failures, and next action from a real local session" width="100%"> | <img src="assets/readme-real-diagnostics.png" alt="agenttrace diagnostics view showing latency, context window, and large parameter calls from real local logs" width="100%"> |

## Install

Install the latest public release with your package manager. Check the installed
version with `agenttrace --version`.

```bash
# macOS and Linux
brew install luoyuctl/tap/agenttrace

# macOS, Linux, and Windows (requires Node.js 18+)
npm install -g @zack78/agenttrace
```

Windows:

```powershell
winget install --id Luoyuctl.AgentTrace --exact
```

The package names above become available once the corresponding release has
been published. Manual installs remain available without a package manager:

```bash
curl -fsSL https://raw.githubusercontent.com/luoyuctl/agenttrace/master/install.sh | sh
cargo install --git https://github.com/luoyuctl/agenttrace agenttrace
```

```powershell
iwr -useb https://raw.githubusercontent.com/luoyuctl/agenttrace/master/install.ps1 | iex
```

A prebuilt binary that cannot run on this host (for example, one built
against a newer libc) is never installed silently: the installer reports
the loader's error, and `install.sh` falls back to a source build. That
fallback is pinned, never a floating master tip: it builds the release tag
matching the failed artifact (or whatever `AGENTTRACE_SOURCE_REF` names — a
tag or a full commit id; a commit-id pin is fetched directly and verified
against the pin), and records what it built in an install receipt
(`repo`, `ref`, and the exact `commit`) so the fallback install stays
auditable after the fact.

## Quickstart

```bash
agenttrace
```

### Configuration file

```bash
# agenttrace reads a small set of knobs from a layered config:
#   CLI flags > --config PATH > ./.agenttrace/config.toml
#   > ~/.config/agenttrace/config.toml > AGENTTRACE_* env > defaults
# Every layer only fills keys left unset above it, and --doctor
# discloses which files were found and where each knob came from.
agenttrace --doctor

# ~/.config/agenttrace/config.toml — all keys optional:
#   history_dir = "/data/agenttrace-history"   # where history.json lives
#   pricing_file = "pricing-overrides.json"     # per-model price overrides
#   weekly_budget_usd = 25.0                     # weekly USD spend budget
#   session_cache_entries = 5000                 # parsed-session cache bound (rm-298)
agenttrace --budget                # window-burn view: per-day spend vs budget
agenttrace --statusline-report     # same journal; "Budget: ... remaining" line
```

Unknown keys, tables, arrays, and non-positive budgets are rejected
with the file, line number, and key — a typo never silently does
nothing. The same flags exist on the command line (`--history-dir`,
`--pricing-file`, `--weekly-budget`) and win over every file.

### CSV statement export (rm-409)

`-f csv` emits RFC 4180 (CRLF rows, `""`-doubled quotes) with one table per
section, marker-comment headers, and a leading apostrophe guard on
formula-looking cells (a name or model id like `=SUM(A1)` or
`=cmd|' /C …'!A0` renders as `'=SUM(A1)` — spreadsheet-safe, byte-stable
across runs, announcements on stderr so stdout stays pure):

```bash
agenttrace --sessions -f csv
agenttrace --overview -f csv
```

Session columns: `session,health,data,source,model,cost,tokens,fail,anomalies,zero_usage_events,subagents,subagent_cost,parent_session`.
The `subagents`/`subagent_cost` columns mirror the TSV's SUBAGENTS/SUBAGENT_COST rollup columns and `parent_session` names the parent transcript on subagent rows (empty otherwise), so no format hides the subagent picture another format shows. Rollups are corpus-scope: they reflect every child in the loaded corpus, not only rows surviving the active view filters (`--since`/`--project` etc.). When `--limit` drops rows, the table ends with a `# truncated: showing N of M matching sessions (--limit L)` marker row — a capped statement never reads as the whole corpus. In the same position, `--sessions -f json` wraps the rows as `{matched_sessions, returned_sessions, truncated, limit, sessions: [...]}` instead of a bare array, so a `jq '.sessions | length'` consumer can always tell a capped list from the corpus (and `jq '. | length' == 5` becomes an error, not a silent 5).
`zero_usage_events` carries the rm-408 disclosure: transcripts that
REPORT `{input_tokens: 0, output_tokens: 0, …}` are counted as measured
zeros and flagged `zero_usage_reported` in provenance instead of reading
as clean or silently falling back to the text estimate (absent usage
keeps `estimated_from_text`). `--doctor` aggregates the share over the
discovered session-transcript census (SQLite-backed agent databases are
not part of this census yet).
Overview sections (`--overview` required): `# summary`, `# by_model`,
`# by_provider`, `# by_task_type`. `-f csv` outside this composable set
bails loudly (`csv format requires --overview or --sessions`) instead of
silently rendering the text report.

### Local MCP server (rm-455)

`agenttrace mcp` is a local read-only MCP server: newline-delimited
JSON-RPC 2.0 over stdio, so a coding agent can ask about its own token
usage from inside the session that burned it. Two tools render the same
locally discovered sessions the CLI reports on — `usage_overview` (the
`--overview -f json` document) and `by_model_breakdown` (cost and
sessions per model). The server opens no sockets, performs no network
probes, and writes nothing outside the agenttrace session cache; it
serves until stdin closes.

```json
{
  "mcpServers": {
    "agenttrace": { "command": "agenttrace", "args": ["mcp"] }
  }
}
```

Registration snippets for Claude Code and Codex, the full protocol
surface, error-arm mapping, and a tool-call transcript:
[docs/guides/mcp-server.md](docs/guides/mcp-server.md). (The keyword is
unrelated to the `--mcp-governance` report flag.)

### Shareable usage card (SVG)

`--overview -f svg` renders a single static SVG file — headline cost, sessions,
tool calls, tokens, top models/projects by cost, and a 14-day daily-spend
sparkline, with the same honesty footer as the other overview formats
(window, pricing source, data-confidence). The card is byte-deterministic for
a fixed corpus + range + theme, carries no scripts and no network calls, and
escapes session-derived strings so hostile model or project names render as
text. `--card-theme auto|dark|light` selects the palette (`auto` adapts to the
viewer via `prefers-color-scheme` while degrading gracefully to light).

```shell
agenttrace --overview --range 7d -f svg -o usage-week.svg
agenttrace --overview -f svg --card-theme dark > usage.svg
```

Details and the determinism contract: [docs/guides/usage-card.md](docs/guides/usage-card.md).

### Governance reports

```bash
# Audit raw token components, normalized pricing, and fallback confidence.
# By default this audits every matching session; audited_sessions /
# total_sessions in the JSON (and "(auditing N of M sessions)" in text)
# disclose the coverage. Bound it explicitly with --sample N.
agenttrace --audit --range 30d -f json
# agenttrace --audit --sample 500 -f json

# --range today bounds the report to your local calendar day (sessions
# with activity since your local midnight — a session that started
# before midnight but ran into today counts); 7d/30d are rolling
# windows on the same activity basis, unaffected.

# --demo renders the built-in synthetic sample corpus and is
# side-effect-free: nothing is written to history.json or the session
# cache, and combining it with --preserve-history is refused rather
# than silently banking sample sessions into durable history.

# Optional local model aliases and per-million-token price overrides.
AGENTTRACE_PRICING_FILE=pricing-overrides.json agenttrace --audit -f json
# A requested override file never fails silently: if it is unreadable,
# malformed, carries unknown keys (e.g. a raw LiteLLM snapshot pasted in),
# or holds a negative rate, agenttrace prints a one-line warning to stderr
# naming the path and reason, the pricing label reports
# "user overrides FAILED" (--test-match shows it), and the bundled
# catalog keeps applying. Exit code is unchanged.

# Rank evidence-backed actions by severity and estimated impact.
agenttrace --recommend --range 30d -f json

# Inspect observed MCP invocations. Loaded-server coverage is intentionally
# reported as unavailable unless the source log actually records it.
agenttrace --mcp-governance --range 30d -f json

# Review cross-session context, cache, repeat-read, and read/write trends.
agenttrace --context-trends --range 30d -f json

# Correlate local Git commit timestamps with sessions (heuristic, read-only).
agenttrace --delivery-evidence --range 30d -f json

# Report subscription limit pressure and upstream prompt-cache miss
# causes captured by the statusline hook (see "Statusline capture" below).
agenttrace --statusline-report -f json
```

`pricing-overrides.json` accepts `aliases` plus per-million-token `prices`:

```json
{"aliases":{"provider/raw-model":"my-model"},"prices":{"my-model":{"input":1,"output":2,"cw":0,"cr":0}}}
```

`--overview` now includes scope, parse and pricing confidence, cost audit,
prioritized recommendations, MCP governance, context trends, and delivery
signals in JSON, Markdown, and HTML output. All cost and delivery fields are
explicitly estimates or heuristics; they are not provider billing or proof that
a commit reached `main`.

### Flags go before the session path

`agenttrace` keeps Go-`flag`-compatible argument parsing: option flags are
recognized only **before** the first positional argument. Flags after the
positional are not parsed, so such invocations are rejected with a usage
error naming the dropped arguments:

```bash
agenttrace sessions.jsonl -f json    # rejected: `-f json` follows the path
agenttrace -f json sessions.jsonl    # -f json works
```

`--limit` caps list views only (for example the overview's `recent_sessions`);
it never filters aggregates, audit totals, or recommendations. Governance
reports are unbounded by default — use `--sample N` for an explicitly
disclosed bound.

### `--demo` never silently replaces an explicit session source

`--demo` substitutes the bundled demo corpus for session discovery. It is
therefore a session source itself, and combining it with another one — a
positional path, `-d/--dir`, or a `--range` other than the default `all` —
exits with an error naming the ignored flags instead of quietly rendering
demo numbers as if they were your corpus:

```bash
agenttrace --demo --overview -d ~/.codex/sessions   # rejected: --demo ignores -d/--dir
agenttrace --demo --range 30d --sessions            # rejected: --demo ignores --range
agenttrace --doctor --demo -d ~/sessions            # rejected: same guard covers --doctor
agenttrace --demo --overview                         # works: demo corpus, default range
```

An explicit `--range all` is indistinguishable from the default and stays
legal. The same guard covers `--doctor`: since rm-596, `--doctor --demo`
walks only the bundled demo corpus, so a `-d` beside `--demo` there would
be silently discarded — drop `--demo` to inspect a directory with
`--doctor`.

### `--baseline` and `--compare`

`--baseline <file>` gates an `--overview -f json` run against a previously
saved overview document: thresholds are set with
`--baseline-max-duration-delta-pct` and the token/cost equivalents, and
`--no-baseline-gate` keeps the comparison while skipping the exit-code gate.
`--baseline` requires `--overview -f json`.

`--compare` builds a two-view comparison report over the selected sessions
and never reads a baseline document, so combining `--baseline` with
`--compare` exits 1 with an error naming both flags rather than falling
through to a misleading action error.

### Statusline capture

Subscription limit windows (`5h`/`7d` usage and reset times) and Claude
Code's prompt-cache analytics never appear in session transcripts. To capture
them, register the bundled statusline command once in Claude Code's
`settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "agenttrace statusline"
  }
}
```

Claude Code invokes the command on every prompt with a single-line JSON
payload on stdin; agenttrace renders the status line and appends the raw
payload to a local journal (`~/.cache/agenttrace/statusline.jsonl`, capped at
10 MiB, oldest lines compacted away; set `AGENTTRACE_SESSION_CACHE_DIR` to
relocate it). The parsed-session cache is bounded by entries and
bytes; raise or lower the entry bound with `session_cache_entries` in
the config file or the `AGENTTRACE_SESSION_CACHE_ENTRIES` environment
variable (both clamped to [1, 1,000,000], default 20,000) — `--doctor`
discloses the bound actually in force, which layer supplied it, and how
many session files the last scan re-parsed from source instead of the
cache. Cache files are written owner-only (`0600` on Unix) and
compacted through transient `.tmp.<pid>.<seq>` siblings that the next cache
load reclaims if a crash leaves one behind (see PRIVACY.md). The command
never fails the host — bad or empty input still
exits 0 with a one-line fallback. Review what was captured with
`agenttrace --statusline-report`, the Efficiency panel in the TUI, or
`agenttrace --doctor` ("Statusline capture" line). See
[docs/guides/statusline-capture.md](docs/guides/statusline-capture.md) for the
schema and retention details.

## CLI flag reference

Every option `agenttrace` accepts, with its default and purpose. This
table mirrors `agenttrace --help`; the `check-docs-commands` CI gate
fails when the README table drifts from the binary's actual flag count
(rm-207).

| Flag | Default | Purpose |
| --- | --- | --- |
| `-f, --format <FORMAT>` | `text` | Output format for the requested view: text (default), json, csv, markdown/md, html, svg (the shareable usage card; requires --overview), or otel (the OTLP-JSON export of the whole corpus; requires --overview). Unsupported combinations fail loudly instead of falling back to text |
| `--card-theme <CARD_THEME>` | `auto` | rm-576: color scheme of the SVG usage card (`-f svg`). `auto` ships the light palette plus a `prefers-color-scheme` override so the static file adapts to the viewer; bytes stay deterministic |
| `-d, --dir <DIR>` |  | Session directory to scan instead of auto-discovered agent homes |
| `--compare` |  | Compare the inspected session against the healthy-baseline summary (narrative framed for `--model`) |
| `--audit` |  | Render the governance audit report across matching sessions (tool-authority drift and spend oversight; `--sample` bounds it) |
| `--recommend` |  | Render cost and efficiency recommendations derived from the session corpus |
| `--mcp-governance` |  | Audit MCP server governance across sessions: which servers were reachable, which tools they exposed, and allowlist drift |
| `--context-trends` |  | Render context-utilization trends over the session corpus (window pressure, cache reuse, growth by turn) |
| `--delivery-evidence` |  | Render the delivery-evidence report: verifiable outcome signals per session rather than effort metrics |
| `--overview` |  | Render the corpus overview report: totals, health mix, and model/provider/project breakdowns |
| `--sessions` |  | List sessions as rows (TSV text by default; `--format` json/csv for machine use). The TSV ends with the subagent rollup columns SUBAGENTS and SUBAGENT_COST — attributed spawned work, kept separate from the session's own COST/TOKENS cells; rollups are corpus-scope (every child in the loaded corpus, not just rows surviving the view filters), `--limit` truncation is disclosed in-band for json/csv and on stderr, and csv carries the subagents/subagent_cost/parent_session parity columns |
| `--diagnostics` |  | Render per-session diagnostics: findings, evidence, fix suggestions, and next actions |
| `--inspect <INSPECT>` |  | Inspect a single session by its `--sessions` list index (1-based) instead of the whole corpus |
| `-m <MODEL>` | `default` | Reference model for `--compare` framing (cost-rate attribution in the comparison narrative; not a session filter — see `--model-filter`) |
| `-o <OUTPUT>` |  | Write the report to this path instead of stdout (the report is also kept on stdout — a tee, not a redirect). Regular paths stage atomically through a temp sibling (rm-250); terminal sinks like /dev/null and /dev/stdout write through directly, while fifo/socket/block-device targets are refused with a disclosed reason instead of being replaced (rm-489). Missing parent directories are created recursively (mkdir -p semantics); when a parent cannot be created, the error names the directory and the requested target (rm-784) |
| `--latest` |  | Restrict the view to the single most recent session (works with `--waste` and report actions) |
| `--waste` |  | Render the token-waste report: redundant context, loop cost, and unused tool output, with a per-reason breakdown |
| `--list-models` |  | List the models the pricing catalog knows, with rate coverage; `--test-match` shows the resolution probes |
| `--update-pricing` |  | Refresh the vendored pricing-catalog snapshots from their upstream sources, then report the drift |
| `--test-match` |  | Print the catalog-resolution table for a probe set of model identifiers (alias and rate-match verification) |
| `--statusline-report` |  | Report on the Claude Code statusline capture journal (candidate 53, cycle 7): limit-pressure windows, reset crossings, and per-session prompt-cache miss causes recorded by `agenttrace statusline` |
| `--fetch` |  | `agenttrace upstream`: refresh the remote-tracking refs from the network via `git fetch` (and probe the npm registry) before reporting fork-vs-upstream drift. Without it, `agenttrace upstream` is fully offline (rm-024) |
| `--config <PATH>` |  | Explicit configuration file (rm-384). Layered above the project and user config files; a missing file is an error |
| `--history-dir <PATH>` |  | Override the history directory: highest precedence, then the config files, then `AGENTTRACE_HISTORY_DIR` (rm-384) |
| `--pricing-file <PATH>` |  | Override the pricing override file: highest precedence, then the config files, then `AGENTTRACE_PRICING_FILE` (rm-384) |
| `--weekly-budget <USD>` |  | Weekly spend budget in USD for `--statusline-report` and `--budget`: highest precedence, then config `weekly_budget_usd` (rm-385) |
| `--budget` |  | Show the weekly budget window-burn view: per-day spend from the statusline journal against the resolved weekly budget (rm-385) |
| `--version` |  | Print the version banner and exit 0; wins over action validation |
| `--demo` |  | Use the built-in demo corpus instead of discovered agent homes (stable epoch-anchored sessions) |
| `--doctor` |  | Run environment self-checks (config paths, cache consistency, agent homes) and exit non-zero on failure |
| `--search <SEARCH>` |  | Full-text search across discovered session transcripts for this query |
| `--search-limit <SEARCH_LIMIT>` | `20` | Cap the number of `--search` hits reported (default 20) |
| `--fail-under-health <FAIL_UNDER_HEALTH>` | `0` | CI gate: exit non-zero when corpus health drops below this score (0 disables the gate) |
| `--fail-on-critical` |  | CI gate: exit non-zero when any critical-severity finding exists |
| `--max-tool-fail-rate <MAX_TOOL_FAIL_RATE>` |  | CI gate: exit non-zero when the tool failure rate exceeds this fraction (0.0-1.0) |
| `--baseline <BASELINE>` |  | Load a healthy-baseline summary JSON (path or id) to compare `--compare` runs against |
| `--baseline-max-duration-delta-pct <BASELINE_MAX_DURATION_DELTA_PCT>` | `0` | Baseline gate: maximum allowed session duration drift, in percent |
| `--baseline-max-cost-delta-pct <BASELINE_MAX_COST_DELTA_PCT>` | `0` | Baseline gate: maximum allowed session cost drift, in percent |
| `--baseline-max-token-delta-pct <BASELINE_MAX_TOKEN_DELTA_PCT>` | `0` | Baseline gate: maximum allowed session token drift, in percent |
| `--no-baseline-gate` |  | Opt out of the baseline regression gate: keep the comparison in the report but do not fail the run (exit 2) on a threshold breach (pass-7 P7-3) |
| `--lang <en|zh>` | `en` | Report language for text and TUI surfaces: en (default) or zh |
| `--range <RANGE>` | `all` | Time range filter for session actions: today, 7d, 30d, or all (default all; ignored by corpus-wide reports) |
| `--project <PROJECT>` | `""` | Filter sessions by project slug substring |
| `--source <SOURCE>` | `""` | Filter sessions by source tool substring (e.g. claude-code, codex, pi) |
| `--model-filter <MODEL_FILTER>` | `""` | Filter sessions by model name substring |
| `--query <QUERY>` | `""` | Filter sessions by a free-text query over identity fields |
| `--health <HEALTH>` | `""` | Filter sessions by health tier (healthy/warning/critical) or a numeric comparison in the shared CLI/TUI dialect: an optional `>=`, `<=`, `>`, `<`, or `=` operator followed by a finite number; a bare number means `>=` (e.g. `warn`, `>=80`, `80`) |
| `--cost <COST>` | `""` | Filter sessions by a cost comparison in the shared CLI/TUI dialect: an optional `>=`, `<=`, `>`, `<`, or `=` operator followed by a finite number; a bare number means `>=` (e.g. `>1.5`, `1.5`) |
| `--anomaly <ANOMALY>` | `""` | Filter sessions by anomaly kind |
| `--sort <SORT>` | `recent` | Sort key for list views: recent (default), health, cost, turns, failures, source, or name |
| `--order <ORDER>` | `desc` | Sort direction for list views: desc (default) or asc |
| `--limit <LIMIT>` | `20` | Maximum number of rows a list view renders (default 20) |
| `--sample <SAMPLE>` |  | Explicitly bound governance reports to the newest N sessions. Governance reports audit every matching session by default; sampling is always disclosed via audited_sessions/total_sessions (pass-8 F8-1) |
| `--clear-cache` |  | Delete the session cache before running, forcing a full re-parse |
| `--preserve-history` |  | Persist derived metrics to the history ledger so later `--include-history` runs can see them |
| `--include-history` |  | Merge preserved-history sessions into the view (offline or retained-history analysis) |
| `-h, --help` |  | Print help |

## What you get

| Need | agenttrace gives you |
| --- | --- |
| Historical spend review | Sessions grouped across projects, agents, and models with Today/7d/30d/All ranges |
| Data confidence | Report scope, per-source coverage, parse skips, cache hits, unknown sources/models, pricing fallbacks, and latest observed session |
| Cost audit and action plan | Token component rates, pricing source/status, estimated cost confidence, and prioritized, evidence-backed remediation suggestions |
| Governance trends | Canonical project grouping, observed MCP invocation governance, cross-session context/cache/read-write trends, and read-only Git delivery correlation |
| Honest capability levels | `Detailed`, `Aggregate`, or `Limited` per session so missing event-level evidence is never presented as a complete trace |
| Privacy-safe steps | Tool-step metadata and duration when the source provides call IDs and timestamps; no prompt, response, result, or tool-argument body is stored in steps |
| Slow-task diagnosis | Latency stats, long gaps, hanging sessions, retry loops, slow tools, large params, and context pressure |
| Regression evidence | Local baseline comparison when supplied, incident timelines, and conservative tool authority categories in reports |
| First-session triage | Sort and filter by cost, duration, health, failures, anomalies, model, source, or text search |
| Shareable evidence | JSON, Markdown, and self-contained HTML reports, plus the single-file SVG usage card |
| Local-first inspection | No hosted backend required |

## Docs

- Documentation index: [docs/README.md](docs/README.md)
- CI setup: [docs/guides/ci-integration.md](docs/guides/ci-integration.md)
- Governance reports: [docs/guides/governance-reports.md](docs/guides/governance-reports.md)
- Cursor import: [docs/guides/cursor-import.md](docs/guides/cursor-import.md)
- Parser guide: [docs/guides/parser-guide.md](docs/guides/parser-guide.md)
- Maintainer distribution guide: [docs/maintainers/distribution.md](docs/maintainers/distribution.md)

Listed in these open source projects:

- [awesome-mac](https://github.com/jaywcjlove/awesome-mac)
- [antigravity-awesome-skills](https://github.com/sickn33/antigravity-awesome-skills)
- [awesome-claude-skills](https://github.com/BehiSecc/awesome-claude-skills)

## Contributing

Parser PRs are welcome. A good parser contribution usually includes:

- a tiny redacted fixture or synthetic sample
- format detection in `crates/agenttrace-core/src/parser.rs`
- role, timestamp, model, token usage, tool call, and tool error extraction
- tests for successful parsing and malformed input

Run before sending a PR:

```bash
cargo test
cargo build --release -p agenttrace
target/release/agenttrace --doctor
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contribution flow.

## License

[MIT](LICENSE) © 2026 agenttrace contributors

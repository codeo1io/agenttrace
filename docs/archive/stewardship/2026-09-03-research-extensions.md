# Research: repository extension candidates — run a24bcf08, phase research (attempt 2b3687a)

Executed direct (no ce-* router installed; agent-reach used for GitHub/Exa per
host-local integration rules). All web/GitHub evidence gathered 2026-09-03.

## 1. Upstream state (luoyuctl/agenttrace)

- Upstream `master` tip is still `e005952` (2026-08-22, PR #281) — **zero
  upstream drift since the fork point**; a cleaned rebase is trivial.
  Source: `gh api repos/luoyuctl/agenttrace/commits`.
- Latest release **v0.8.0** (2026-08-22) — the fork is based on post-v0.8.0
  master.
- **PR #282** ("fix: harden untrusted-input handling and make reads truthful
  (cycles 1-4)", +11362/−260, 76 files, from `codeo1io:master`) was opened
  2026-09-02 and **closed unmerged by the fork owner the same day**:
  "this batch includes local-infrastructure CI config (self-hosted runner
  overrides) specific to our deployment, not suitable for upstream… a cleaned,
  upstream-only PR can follow." — independent upstream confirmation of the
  assess-phase CRITICAL finding (6632014 `runs-on: self-hosted`); the cycles-1-4
  hardening work still has no upstream home.
- Open upstream issues (user needs): **#103** "Preserve provider and cost
  provenance" (open since 2026-05-04, 10 comments; cites codeburn,
  TokenTracker, codeledger); radar **#236** Gemini CLI → Antigravity CLI
  transition (7 comments); radar **#237** Qwen Code `/export` + Dual Output
  (2 comments); **#272** npm scoped-package publish broken; dependency PRs
  **#279** (cargo group, 7 updates), **#278** (attest-build-provenance
  4.1.1→4.2.2), **#259** (actions/checkout v7) all open.

## 2. Ecosystem / competitive landscape

- **ccusage (ryoppippi/ccusage): 18,296 ★, pushed 2026-09-02, v20.x.**
  Reads usage files from 16 agent CLIs (Claude Code, Codex, OpenCode, Amp,
  Droid, Codebuff, Hermes Agent `state.db`, pi-agent, Goose, OpenClaw, Kilo,
  Kimi, Qwen, Copilot CLI `~/.copilot/otel/*.jsonl`, Gemini CLI
  `~/.gemini/tmp`). Strengths: daily/weekly/monthly/session views, trends,
  table+JSON output. No TUI diagnostics, no health/anomaly/governance gates.
- **codeburn (getagentseal/codeburn): 10,751 ★, active.** Cross-tool token
  cost + performance attribution across 16 coding tools — the demand signal
  behind upstream #103.
- **claude-cost-tracker-mcp (yuziri-open):** MCP server pattern with
  `track_usage` / `get_current_session` / `get_daily_summary` /
  `get_monthly_summary`, HTMX dashboard, **budget alerts + month-end pace
  projection + per-model breakdown**, SQLite persistence.
- agenttrace itself: 127 ★. Differentiators to defend: single Rust binary,
  TUI, per-session diagnostics (latency, anomalies, health), governance/CI
  exit-code gates, offline-by-default pricing.

## 3. Standards & platform developments

- **OpenTelemetry GenAI semantic conventions** are largely **stable**
  (`gen_ai.*` spans/metrics; registry v1.44; `invoke_agent`/`plan` still
  "development"). Source: open-telemetry/semantic-conventions-genai
  `docs/gen-ai/gen-ai-spans.md`.
- **Claude Code CLI has built-in OTel** (`CLAUDE_CODE_ENABLE_TELEMETRY=1`,
  `OTEL_METRICS_EXPORTER`/`OTEL_LOGS_EXPORTER`/`OTEL_TRACES_EXPORTER`):
  counters for tokens/cost/sessions/LOC/tool decisions, structured log
  events, beta traces. Source: code.claude.com Agent SDK observability docs.
- **Copilot CLI** already writes OTel JSONL locally (`~/.copilot/otel`) —
  agenttrace already scans it (discovery.rs:120).
- **Antigravity CLI (Google)** — official migration doc confirms Gemini CLI →
  `agy`; sessions live in `~/.gemini/antigravity-cli/conversations/`, logs in
  `.../log/cli-*.log`, artifacts in `.../brain/` (also confirmed by the
  NousResearch/hermes-agent antigravity-cli skill reference).
- **Qwen Code**: `/export` writes session history as Markdown/JSONL/HTML
  (default HTML since the 2026-05-14 weekly update); **Dual Output** sidecar
  emits a documented structured-JSON event stream via `--json-file`/`--json-fd`
  plus a reverse control channel. Source: qwenlm.github.io dual-output docs.

## 4. Verified live gap (this host, 2026-09-03)

Fake-HOME experiment (`HOME=/tmp/fakehome agenttrace --doctor`):
a valid Gemini CLI checkpoint placed at `~/.gemini/tmp/session-*.json` and an
Antigravity conversation at `~/.gemini/antigravity-cli/conversations/*.jsonl`
were **both invisible to auto-discovery** ("Session files: 0"), while the same
Gemini file parses fine when passed positionally (`--overview` produced model
`gemini-2.5-flash`, cost, anomalies). Root cause: `known_session_dirs()`
(discovery.rs:60-130) has **no `~/.gemini/tmp` root and no
`.../antigravity-cli/conversations/` root**, even though helper predicates for
exactly those paths already exist (`max_session_dir_depth` line 533 expects
`.gemini/tmp` depth 4; `is_gemini_temp_session_file` lines 584-590 expects
`chats|checkpoints` subdirs). The README lists Gemini CLI as supported — the
format works, only discovery is missing.

## 5. Candidates (evidence → feasibility)

| # | Candidate | Evidence | Feasibility |
|---|-----------|----------|-------------|
| C1 | **Discovery roots: `~/.gemini/tmp` (depth 4, `chats`/`checkpoints`) + `~/.gemini/antigravity-cli/conversations/`** | Live gap test (§4); upstream radar #236; official Antigravity docs; existing helper predicates; README claim | HIGH — two `KnownSessionDir` entries + fixtures/tests. Restores claimed support. |
| C2 | **Budgets & pace projection** (per-day/month/project caps, `--fail-over-budget` gate, month-end pace in overview) | Upstream #103 (10 comments); codeburn 10.7k★; claude-cost-tracker-mcp budget/pace features | MEDIUM — history.json + gate machinery already exist. |
| C3 | **MCP server mode** (`agenttrace mcp`: session/summary/health/cost tools for agents) | MCP cost-tracker pattern; agenttrace already ships a Codex plugin and MCP-governance report; local-first fit | MEDIUM — new subcommand + stdio JSON-RPC; core data all local. |
| C4 | **OTel interop**: (a) ingest OTel/OTLP file-export JSONL as a source; (b) emit `gen_ai.*` metrics/spans from parsed sessions | Stable GenAI conventions; Claude Code native OTel; Copilot OTel JSONL already ingested | MEDIUM-LARGE — (b) is a clean exporter from existing Session structs. |
| C5 | **Pricing snapshot freshness**: report a `pricing snapshot age` line + scheduled snapshot refresh CI | LiteLLM catalog commits 3×/day (last 2026-09-02T18:02 vs bundled 2026-09-02); `scripts/pricing/update-snapshot.sh` already exists; `data_health` already discloses `fallback_pricing`/`unknown_models` | LOW — one report field + workflow. |
| C6 | **Qwen `/export` + Dual Output ingestion** (JSONL export + sidecar event schema detectors + fixtures) | Upstream radar #237 + official Qwen docs (`/export` MD/JSONL/HTML; Dual Output documented event schema) | MEDIUM — needs real fixtures; generic fallback may already parse some shapes. |
| C7 | **Hardening backlog from the assess phase** (session-file size cap; installer SHA-256 parity for install.sh/ps1; `--sample` wording vs `--sort`; Hermes tool-failure extraction; `resolve_project` memoization; cache byte bound) | assess-phase artifact `2026-09-03-cycle5-adversarial-assessment.md`, findings 2-8 | LOW-MEDIUM each; all locally verified. |
| C8 | **Cleaned upstream-only PR** (revert `runs-on` in the PR branch, re-apply cycles-1-4 work as a fork feature branch, never fork `master`) | PR #282 closure reason (fork owner's own statement); upstream master unmoved at `e005952`; open dep PRs #278/#279/#259 as refresh path | HIGH feasibility, operator-gated per AGENTS.md rule 4. |

## 6. Positioning insight

ccusage wins on breadth of usage views; codeburn wins on cross-tool budget
attribution; neither offers TUI per-session diagnosis, health scoring, anomaly
detection, or CI exit-code gates. C1 (restore claimed discovery), C2 (budgets),
and C5 (pricing freshness disclosure) are the cheapest high-signal moves; C3/C4
extend the local-first niche into where the ecosystem (OTel, MCP) is
standardizing.

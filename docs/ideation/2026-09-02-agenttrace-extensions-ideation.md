---
title: "Ideation: agenttrace repository extensions"
date: 2026-09-02
topic: agenttrace-extensions
focus: "upstream changes, ecosystem developments, competing approaches, user needs, dependencies, standards, feasible new capabilities"
mode: repo-grounded
provenance: "run 314df0f829fe49af8de46938c7b579a6, phase research, attempt a07e8b88eef0422b8e37b9a336445bd3; grounded on HEAD e005952 and docs/reviews/2026-09-02-adversarial-repository-assessment.md"
---

# Ideation: agenttrace repository extensions

Evidence-backed candidate capabilities for the agenttrace roadmap. Generated through the `ce-ideate` lens set (pain/friction, inversion-removal-automation, assumption-breaking, leverage, cross-domain analogy, constraint-flipping), grounded in the repository at `e005952`, the prior adversarial assessment, and fresh external research performed 2026-09-02.

**Harness disclosure.** This environment has no subagent-dispatch or per-agent model surface, so every frame, the basis verifier, and arbitration ran in-thread on one context (disclosed per the skill's substitution rule). No candidate is claimed as independently corroborated by a second context; the basis check instead leaned on machine verification — every `direct:` quote below was re-checked against the working tree with `grep`/`sed`, and every `external:` claim was fetched live this session.

## Grounding Context

### Codebase Context

- Rust workspace, one `agenttrace` binary serving CLI reports and a ratatui TUI; local-first post-run observability for AI coding-agent sessions (README, ROADMAP).
- Sixteen parser families live in `crates/agenttrace-core/src/parser.rs` (opencode, qwen, cline, copilot, cursor, aider, gemini, kimi, oh_my_pi, claude, codex, workbuddy, hermes, antigravity, openclaw, plus generic JSON/JSONL), with SQLite-backed sources for Hermes and OpenCode in `crates/agenttrace-core/src/sqlite_sessions.rs`.
- Existing capability surface: overview/sessions/health/waste/search, governance reports (cost audit, MCP usage, context trends at `crates/agenttrace-core/src/governance.rs:571`, delivery evidence via read-only git), CI gate flags at `crates/agenttrace-cli/src/main.rs:81-93` (`--fail-under-health`, `--baseline-max-cost-delta-pct`, …), reports in text/JSON/Markdown/HTML, pricing from LiteLLM with an `AGENTTRACE_PRICING_FILE` override, a session cache, `doctor`, and `--demo`.
- Pricing today: one upstream (`crates/agenttrace-core/src/pricing.rs:10-11`, `BerriAI/litellm … model_prices_and_context_window.json`); `pricing_source()` (`pricing.rs:84-104`) embeds a wall-clock "cached <ts>"/"fetched <now>" string; the report path lazily re-downloads when the cache is stale (`pricing.rs:239-241`: `if catalog.source == "cache(stale)" { if let Ok((raw, entries)) = download_pricing(Duration::from_secs(5))`), while `PRIVACY.md:5` names only `--update-pricing` as a downloader.
- No OpenTelemetry surface exists anywhere in `crates/` (`grep -ri otel` matches nothing relevant).
- `Metrics` (`crates/agenttrace-core/src/lib.rs:262-284`) already carries `tokens_input/output/cache_w/cache_r` plus a `timestamps: Vec<DateTime<Utc>>` — rolling-window analytics are computable from parsed data with no new sources.
- Roadmap non-goals that bound ideation: hosted prompt storage, billing-grade invoice reconciliation, live tracing while a model is streaming, security enforcement.
- Prior adversarial assessment (`docs/reviews/2026-09-02-adversarial-repository-assessment.md`): 19 findings; the ones load-bearing here are F2 (report-path network fetch), F3 (test-path network I/O), F5 (`pricing_source` nondeterminism), F16/F18 (silently lost sessions), F8 (Windows `%TEMP%` caches).
- Deterministic generated fixtures are CI-enforced (`crates/agenttrace-core/tests/discovery_contract.rs:26`).

### External Context

Fetched live this session; URLs and access method per item.

- **OpenTelemetry GenAI semantic conventions** — `open-telemetry/semantic-conventions-genai` (323 stars, pushed 2026-09-01) defines `gen_ai` spans, metrics, events, agent spans, and MCP conventions (`docs/gen-ai/mcp.md`, `anthropic.md`). Most attributes are Development-stability. Via `gh api`.
- **Claude Code ships native OTel export** — `code.claude.com/docs/en/monitoring-usage`: `CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_METRICS_EXPORTER=otlp|prometheus|console`, `OTEL_LOGS_EXPORTER`, OTLP endpoint configuration. Via Exa.
- **Gemini CLI has an OTel telemetry path** — `google-gemini.github.io/gemini-cli/docs/cli/telemetry.html`. Via Exa.
- **Codex session format is documented only in source** — `openai/codex` `codex-rs/rollout/src/recorder.rs`: `~/.codex/sessions/rollout-<ts>-<conversation_id>.jsonl`, and `thread/revert` writes a *new immutable rollout file* while keeping the thread id. Via Exa.
- **ccusage** (`ryoppippi/ccusage`, 18,282 stars, v20.0.20 released 2026-08-15, `ccusage.com`) advertises an explicit **Offline Mode — "Use pre-cached pricing data without network connectivity"** — plus cache support, JSON output, and cost modes. Via `gh api` + `curl`.
- **CodeBurn** (`getagentseal/codeburn`, 10,406 stars, v0.9.23 2026-08-29): "37 tools and agents". Via `gh api`.
- **OpenUsage** (`janekbaraniewski/openusage`, 167 stars, launched ~2026-08-31, `openusage.sh`): local-first terminal dashboard, "36 providers", tracks spend, quotas, and rate limits, and ships a telemetry daemon. Via Exa.
- **models.dev** — `models.dev/api.json` is live (HTTP 200, 4,443,420 bytes; maintained under the anomalyco/opencode umbrella) and carries per-model context windows, provider lists, and release dates (e.g. claude-fable-5-1 updated 2026-09-01). Via `curl`.
- **Dominant user pain is usage-limit drain** — `anthropics/claude-code` issues by reaction count: #16157 "Instantly hitting usage limits with Max subscription" (724 reactions), #38335 "session limits exhausted abnormally fast since March 23, 2026" (545), #9424 "Weekly Usage Limits Making Claude Subscriptions Unusable" (155), #41930 "Widespread abnormal usage limit drain" (97). Via `gh api search`.

## Topic Axes

- Parser and source coverage
- Pricing and cost-data layer
- Diagnosis intelligence
- Interop and integration surface
- Trust, determinism and hygiene

## Ranked Ideas

1. [Offline-first pricing with a vendored snapshot](#1-offline-first-pricing-with-a-vendored-snapshot) · Pricing and cost-data layer
2. [OTel GenAI bridge: ingest agent telemetry, export gen_ai evidence](#2-otel-genai-bridge-ingest-agent-telemetry-export-gen_ai-evidence) · Interop and integration surface
3. [Subscription limit-pressure diagnostics](#3-subscription-limit-pressure-diagnostics) · Diagnosis intelligence
4. [models.dev as a second pricing and model-metadata source](#4-modelsdev-as-a-second-pricing-and-model-metadata-source) · Pricing and cost-data layer
5. [Upstream format canary: contract tests against live samples](#5-upstream-format-canary-contract-tests-against-live-samples) · Parser and source coverage
6. [Token attribution ledger with unexplained-remainder surfacing](#6-token-attribution-ledger-with-unexplained-remainder-surfacing) · Diagnosis intelligence
7. [Shareable project baseline config and multi-machine merge](#7-shareable-project-baseline-config-and-multi-machine-merge) · Interop and integration surface

### 1. Offline-first pricing with a vendored snapshot

**Description.** Bake a dated, trimmed pricing snapshot into the binary at build time, make offline the *default* operating mode, and move every network refresh behind `--update-pricing` or an explicit opt-in. Replace `pricing_source()`'s wall-clock strings ("cached <ts>", "fetched <now>") with a stable enum (`builtin` / `snapshot-2026-09-02` / `user-override`) so identical inputs produce byte-identical JSON. This one move closes three assessment findings at once: the report-path download (F2), the test-path network I/O (F3), and the CI determinism hole (F5).

**Axis.** Pricing and cost-data layer.

**Basis.** `direct:` `crates/agenttrace-core/src/pricing.rs:239-241` — `if catalog.source == "cache(stale)" { if let Ok((raw, entries)) = download_pricing(Duration::from_secs(5))` runs inside `pricing_catalog()` on the ordinary report path, while `PRIVACY.md:5` states "`agenttrace --update-pricing` downloads public model pricing metadata" and nothing else. `external:` ccusage (18,282 stars) makes offline pre-cached pricing a headline feature — "Offline Mode: Use pre-cached pricing data without network connectivity" (ccusage.com, fetched 2026-09-02).

**Rationale.** The assessment proved the current behavior with reproducers (a stale cache makes `--demo --overview -f json` fetch 2,090,796 bytes). Beyond correctness, this is now a competitive table-stakes feature: the most-starred tool in the space advertises exactly the posture agenttrace's own privacy doc promises but its code does not deliver.

**Downsides.** Snapshot staleness between releases (mitigated by the override file and explicit refresh); a small binary-size cost for the embedded subset; the release checklist gains one more surface to date-stamp.

**Confidence.** 92%. **Complexity.** Medium.

### 2. OTel GenAI bridge: ingest agent telemetry, export gen_ai evidence

**Description.** A two-directional OpenTelemetry surface. *Export:* `agenttrace export --format otel` maps sessions to `gen_ai.*` semantic-convention spans and metrics, so evidence computed locally loads into any OTel backend (self-hosted Grafana, Jaeger, Langfuse) without agenttrace growing a server. *Ingest:* accept the telemetry agents already emit — Claude Code with `CLAUDE_CODE_ENABLE_TELEMETRY=1` and `OTEL_METRICS_EXPORTER=console`, Gemini CLI's OTel path — as an additional source, because emitted telemetry carries step timing and tool attributes session logs sometimes lack. Both legs are post-hoc, so the "no live tracing while a model is streaming" non-goal is preserved.

**Axis.** Interop and integration surface.

**Basis.** `external:` Claude Code documents native OTel export (`code.claude.com/docs/en/monitoring-usage`: `CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_METRICS_EXPORTER=otlp|prometheus|console`); Gemini CLI documents an OTel telemetry path (`google-gemini.github.io/gemini-cli/docs/cli/telemetry.html`); `open-telemetry/semantic-conventions-genai` (323 stars, pushed 2026-09-01) publishes the `gen_ai` span/metric/event model including `mcp.md`. `direct:` the repo has zero OTel surface today (`grep -ri otel crates/` — no matches), and the governance module already computes MCP invocation counts, which map directly onto the semconv's MCP conventions.

**Rationale.** The ecosystem standardized while agenttrace was building parsers: the same vendors whose session files agenttrace reads now also emit standard telemetry. Interop is the compounding move — one export adapter makes every backend an integration, and one ingest adapter makes every OTel-emitting agent a potential source without a bespoke parser.

**Downsides.** Many `gen_ai` attributes are still Development-stability, so the mapping needs a versioned pin; span-mapping of post-hoc sessions is lossy; and this is the largest scope item here — it should start as export-only.

**Confidence.** 78%. **Complexity.** High.

### 3. Subscription limit-pressure diagnostics

**Description.** Map parsed sessions onto rolling windows — the 5-hour block, daily, and weekly rhythms subscriptions actually bill and throttle on — and surface burn rate, window-over-window acceleration, and which sessions consumed each window. Framed strictly as diagnosis ("why did my limit drain", "which session burned the block"), never as billing reconciliation, respecting the roadmap non-goal.

**Axis.** Diagnosis intelligence.

**Basis.** `external:` the single most-reacted pain cluster in `anthropics/claude-code` issues is usage-limit drain — #16157 "Instantly hitting usage limits with Max subscription" (724 reactions), #38335 (545), #9424 "Weekly Usage Limits Making Claude Subscriptions Unusable" (155), #41930 "Widespread abnormal usage limit drain" (97). `direct:` `Metrics` already carries the inputs (`crates/agenttrace-core/src/lib.rs:279-284`: `tokens_input/output/cache_w/cache_r` and `timestamps: Vec<DateTime<Utc>>`), so windows are computable from parsed data with no new source.

**Rationale.** agenttrace's stated second job is "diagnose why an agent task ran slowly or regressed" — and for subscription users the dominant regression of 2026 is invisible limit drain. The data is already parsed; the capability is a projection of it. OpenUsage's quota framing shows the demand is current, and agenttrace's diagnosis framing is the differentiated take (explain *which sessions and tools* burned the window, not just display the quota).

**Downsides.** Window semantics are plan-specific and undocumented upstream, so labels must be hedged and configurable; there is a standing temptation to drift toward billing reconciliation, which the roadmap forbids.

**Confidence.** 80%. **Complexity.** Medium.

### 4. models.dev as a second pricing and model-metadata source

**Description.** Extend the pricing catalog to multiple upstreams with per-model provenance, adding `models.dev/api.json` alongside LiteLLM. The near-term payoff is not price accuracy but metadata: models.dev carries context-window size per model, which lets the existing context-trends report (`governance.rs:571`) compare observed context growth against the model's actual ceiling — turning "context is growing" into "context is at 87% of the model window and the last 3 sessions degraded".

**Axis.** Pricing and cost-data layer.

**Basis.** `direct:` `crates/agenttrace-core/src/pricing.rs:10-11` names exactly one upstream (`raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json`), and `context_trends` currently has no per-model ceiling to normalize against. `external:` `models.dev/api.json` is live (fetched 2026-09-02, HTTP 200, 4,443,420 bytes) and maintained under the anomalyco/opencode umbrella — the ecosystem agenttrace already parses most heavily (62 references in `parser.rs`) — with context windows, provider lists, and release dates (claude-fable-5.1, updated 2026-09-01).

**Rationale.** Single-upstream dependency is a fragility the assessment already exposed (the 5s blocking download), and the metadata gap blocks a diagnosis agenttrace is one join away from. Sourcing from the same ecosystem as the best-covered parser also aligns incentives: when opencode adds a model, its pricing and window data arrives in the same feed.

**Downsides.** Two sources disagree on some models, so the catalog needs a documented precedence rule; more cache state to keep deterministic (should ship together with idea 1).

**Confidence.** 82%. **Complexity.** Medium.

### 5. Upstream format canary: contract tests against live samples

**Description.** A separately-scheduled, network-explicit canary job that fetches a small, redacted, synthetic-or-donated sample for each upstream session format, runs the parsers against it, and diffs coverage and capability labels against expectations — filing drift as a visible failure. Kept strictly off the default `cargo test` path so it never reintroduces the hermeticity problem (assessment F3).

**Axis.** Parser and source coverage.

**Basis.** `direct:` agenttrace already has the deterministic-fixture discipline this builds on — `crates/agenttrace-core/tests/discovery_contract.rs:26` (`generated_capability_and_step_fixtures_cover_degradation`) CI-enforces generated fixtures across 16 parser families. `external:` upstream formats drift silently and are documented only in source; `openai/codex` `codex-rs/rollout/src/recorder.rs` shows the rollout filename scheme including the `thread/revert` variant that writes a *new immutable rollout file* — precisely the kind of change a local fixture set cannot notice.

**Rationale.** Parser coverage is agenttrace's moat and its most fragile asset: every upstream release is a silent compatibility lottery. The fixture system proves the team can generate and assert on formats deterministically; extending that machinery from "our snapshot of the format" to "upstream's current format, on a schedule" converts the moat's decay into an alert.

**Downsides.** Introduces network into a deliberately network-free CI unless quarantined in its own workflow; needs a hygiene policy for samples (synthetic or consented-and-redacted only, consistent with the existing redaction posture).

**Confidence.** 70%. **Complexity.** Medium.

### 6. Token attribution ledger with unexplained-remainder surfacing

**Description.** Account for every token twice — once as a total, once as the sum of attributed causes (initial prompt, context growth per turn, cache hit/miss, retries, subagent fan-out) — and surface the unattributed remainder as a first-class health metric alongside the existing `Detailed`/`Aggregate`/`Limited` capability labels. When a parser silently drops fields, the remainder grows; parser gaps become a visible signal instead of an invisible undercount.

**Axis.** Diagnosis intelligence.

**Basis.** `reasoned:` double-entry bookkeeping's core insight is that reconciliation failure is itself information — a ledger that must balance converts missing data into an observable quantity. `direct:` silent loss is a live failure mode today: non-UTF-8 session files vanish into `skipped` with no reason (assessment F16, `crates/agenttrace-core/src/parser.rs:21` → `discovery.rs:225-228`), and `--since` drops sessions with unparseable `session_start` (`discovery.rs:246`, F18).

**Rationale.** The assessment repeatedly found *silent* degradation paths; the product's own honesty labels (`Detailed`/`Aggregate`/`Limited`) stop at per-source capability and never reconcile totals. This generalizes that honesty from "what this source can tell us" to "what we cannot account for", which is the number a reviewer auditing cost governance actually needs.

**Downsides.** Attribution rules are heuristic per format and must not imply false precision; the remainder needs careful naming so users read it as a coverage signal, not an error.

**Confidence.** 68%. **Complexity.** Medium.

### 7. Shareable project baseline config and multi-machine merge

**Description.** Two small interop steps. First, a committed `.agenttrace.toml` holding gate thresholds, pricing overrides, and model aliases — today `--fail-under-health`, `--baseline-max-cost-delta-pct` and friends are per-invocation CLI flags (`main.rs:81-93`), so a team cannot share a cost/health budget through the repo. Second, `agenttrace merge` over exported JSON reports, letting one reviewer aggregate evidence from several machines while staying strictly serverless.

**Axis.** Interop and integration surface.

**Basis.** `direct:` all gate thresholds are CLI arguments (`crates/agenttrace-cli/src/main.rs:81-93`), with only `AGENTTRACE_PRICING_FILE` as a file-based input, and the JSON report has no schema version stamp for a merger to key on. `reasoned:` CI gates are only adoptable by a team if the threshold travels with the code it gates; a config file is the minimal mechanism, and deterministic outputs (idea 1) are its precondition.

**Rationale.** The roadmap promises "repeatable CI gates" and "shareable evidence"; both currently stop at a single machine and a single invocation. This is the smallest change that makes the existing gate machinery a team artifact rather than a personal habit.

**Downsides.** Config precedence rules (file vs flags vs env) need deliberate design; merge requires a stable schema version stamp first; multi-machine merging raises report-merge conflict semantics that must stay boring (append-only, provenance-tagged).

**Confidence.** 72%. **Complexity.** Low.

## Rejection Summary

Every rejection is explicit, with a reason:

- **First-class Windows cache/data directories** — duplicates assessment F8; a fix already queued for the remediation phase, not a new capability (better handled as repair).
- **`doctor --fix` auto-repair of cache and history** — duplicates F4/F6 remediation; below the meeting-test floor as a standalone idea.
- **Publish the fixture corpus as an ecosystem test asset** — interesting, but it is a community/positioning play rather than a repository capability, and lacks a user-need basis; better revisited as a brainstorm variant.
- **Flight-data-recorder timeline replay view** — compelling analogy, but overlaps the existing TUI drill-down and session detail; basis is analogy-only, weaker than the attribution ledger it would depend on. Revisit after idea 6.
- **SQLite-backed incremental index for very large histories** — constraint-flip with no observed user signal at current scale; speculative.
- **Signed evidence bundles for audit** — drifts toward the security-enforcement non-goal and adds key-management burden with no evidence of demand.
- **Team dashboard server** — subject-replacement: the roadmap's first non-goal is hosted services.
- **Live streaming monitor / daemon mode** — contradicts the "no live tracing while a model is streaming" non-goal; OpenUsage's daemon already occupies that space, and agenttrace's differentiated ground is post-run evidence.
- **ML-based anomaly scoring, cost forecasting** — generic-listicle territory with no grounded basis; forecasting additionally abuts the billing non-goal.
- **`--lang` validation, `.gitignore` cleanup, plugin version sync, MSRV job** — tactical fixes already enumerated by the assessment (F12, F14, F19, F15); tactical scope was not requested and they fail the meeting-test as ideas.
- **axis: Trust, determinism and hygiene — zero dedicated survivors** (deliberate gap): that axis is owned by the assessment's fix phase (F1-F8 remediation) rather than by new-capability ideation; idea 1 is the only survivor that crosses into it, by making determinism a *product posture* rather than a bug fix.

---

# Second research pass — 2026-09-02

topic: agenttrace-extensions (continued)
focus: "upstream changes, ecosystem developments, competing approaches, user needs, dependencies, standards, feasible new capabilities"
provenance: "run 0a36c54199de4861b50ddc2dcb26fd8f, phase research, attempt a802dde0d7cf47e0940e7431b8af12ba; grounded on HEAD e005952 plus the uncommitted cycle-1 tree and docs/reviews/2026-09-02-adversarial-repository-assessment-pass2.md"

A second, non-overlapping pass. Ideas 1–7 above are preserved verbatim; everything below is new evidence gathered 2026-09-02 after the cycle-1 hardening landed in the working tree. Idea numbering continues at 8.

**Harness disclosure.** Same as the first pass: this delegate session has no subagent-dispatch surface, so grounding, generation, and critique ran in-thread on one context. No candidate below is claimed as independently corroborated; every `external:` claim was fetched live this session (method noted per item), and every `direct:` claim was re-checked against the working tree with `grep`/`sed`.

## Delta grounding

### Codebase delta since the first pass

- **Idea 1 (offline-first pricing) has landed** in the uncommitted cycle-1 tree: dated vendored snapshot (`crates/agenttrace-core/src/pricing_snapshot.json`), offline default, clock-free `pricing_source`, hermetic tests, `--update-pricing` as the only downloader. Verified this session by running the binary with no cache and a dead proxy. That slot is closed; nothing below re-proposes it.
- **The pass-2 adversarial assessment (N1/N2)** relocated the arithmetic risk: the SQLite ingestion path in `crates/agenttrace-core/src/sqlite_sessions.rs` still re-derives cost and tokens by summing per-message JSON under unguarded `+=`, and debug-panics on adversarial input. This changes where a cost-data-layer idea should aim: the highest-leverage move is now *stop deriving*, not *derive more carefully* (see idea 8).
- Verified clean (no action needed): OpenCode database discovery already globs any `opencode*.db` filename, so upstream's new channel-suffixed databases (`opencode-<channel>.db`) are covered (`sqlite_sessions.rs:118`).

### External context — fetched live 2026-09-02

**Upstream (OpenCode, 203,133 stars, pushed 2026-09-02) — via `gh api`:**

- The `session` table now stores **authoritative, precomputed session-level facts** agenttrace ignores: `cost real not null default 0`, `tokens_input`, `tokens_output`, `tokens_reasoning`, `tokens_cache_read`, `tokens_cache_write`, plus `parent_id` (session/subagent hierarchy), `project_id`/`workspace_id`, `summary_additions`/`summary_deletions`/`summary_files` (per-session delivery diffs), `time_compacting`, `time_archived`, `agent`, `version` (`packages/core/src/session/sql.ts`, fetched via `gh api` contents). agenttrace reads five columns: `select id, title, time_created, time_updated, {directory} from session` (`sqlite_sessions.rs:267`).
- The storage layer is **mid-migration**: `specs/storage/remove-opencode-db.md` is deleting the legacy `opencode/src/storage/db.ts` wrapper in favour of `@opencode-ai/core` + Effect infrastructure; Groups 1 and 2 are marked *Completed*. Every migration entry repeats "Preserve canonical V1 `session`, `message`, and `part` rows", so agenttrace's core reads survive — but the schema is actively reshaping around them.
- Upstream **publishes a schema changelog**: `specs/v2/schema-changelog.md` with dated entries ("2026-06-26: Add Finite Session History", "2026-06-22: Simplify Session Input Promotion", …) plus dated migration files under `packages/core/src/database/migration/`.
- Upstream **records compaction durably**: "2026-06-05: Execute Automatic Session Compaction — trigger automatic compaction before provider turns using the complete estimated request and absolute model-aware headroom … store the completed event with the current checkpoint payload containing stable message identity, reason, summary, and recent context", backed by a `session_context_epoch` table (baseline, snapshot, baseline_seq) and `session.time_compacting`.
- Upstream **ships its own stats now**: `packages/opencode/src/cli/cmd/stats.ts` (`opencode stats`) reporting totalSessions/Messages/Cost, token totals including cache read/write, toolUsage, modelUsage, costPerDay, tokensPerSession, medianTokensPerSession; plus a whole `packages/stats` app with model-compare routes.

**Upstream (Claude Code) — via `gh api repos/anthropics/claude-code/contents/CHANGELOG.md`:**

- "Added a per-session prompt-cache line to `/cost` (**hit ratio, misses, tokens re-cached, warm/cold**) and a matching `prompt_cache` object for **status line scripts**."
- "Added a **Spend limit bar to `/usage` and a `rate_limits.spend_limit` status line field** for developers behind a Claude apps gateway with spend limits."
- The **status line is a first-class output surface**: "runs any shell script you configure. It receives JSON session data on stdin and displays whatever your script prints" (`code.claude.com/docs/en/statusline.md`, fetched via curl).
- `SessionStart` resume hooks now receive "session staleness and the **estimated re-cache cost**" — cache re-warm cost is now upstream vocabulary, not a niche metric.

**Competing approaches — via `gh api search/repositories` (stars, last push):**

| Tool | Stars | Last push | Positioning |
|---|---|---|---|
| ccusage | 18,286 | 2026-09-01 | offline pre-cached pricing; no release since v20.0.20 (2026-08-15) |
| CodeBurn | 10,538 | 2026-09-01 | 37 tools/agents |
| **TokenTracker** | **1,493** | **2026-09-01** | local-first, **31 coding tools**, native apps, **"Never reads prompts"** |
| token-dashboard | 679 | 2026-04-20 | JSONL → cost analytics, **hotspot views** |
| claude-dashboard | 563 | 2026-08-23 | statusline: context, **API rate limits**, cost |
| cship | 417 | 2026-08-04 | statusline: **TOML-configurable cost/context/usage thresholds** |
| aqua5230/usage | 306 | **2026-09-02** | menu-bar/tray: **quota, burn rate**, cost, HTML reports |
| splitrail | 218 | 2026-09-01 | real-time tracker, 11 CLIs incl. OpenCode |
| toktrack | 187 | 2026-09-02 | "ultra-fast" tracker |

OpenUsage has stalled since its launch week (167 → 181 stars). A `gh api` repository search for CI-gate/budget capabilities in this space returns no relevant tool (one unrelated 0-star repo): no surveyed tool ships CI gates, baseline comparison, or shareable threshold configuration — agenttrace's governance lane is uncontested.

**Dependencies and toolchain — via crates.io API and raw READMEs:**

- **ureq 3.4.0** (2026-08-08, MSRV 1.85): "ureq automatically reads proxy configuration from environment variables when creating a default Agent. Proxy variables are checked in order: `ALL_PROXY`, `HTTPS_PROXY`, then `HTTP_PROXY` (with lowercase variants). `NO_PROXY` specifies hosts that bypass the proxy." Repo pins `ureq = "2.12"` (resolves 2.12.1), which does none of this — reproduced in the pass-2 assessment (N4): `--update-pricing` downloaded the full catalog through a dead proxy.
- rusqlite **0.40.2** available vs repo `0.32`; crossterm **0.29.0** vs repo `0.28`; ratatui 0.30.2 == repo (current).
- Rust stable is **1.98.x** (`rust-lang/rust` RELEASES.md); repo declares `rust-version = "1.80"` (Cargo.toml:12) — 18 minor versions behind, and untested in CI (prior assessment F15).

**Standards — via `gh api` + curl:**

- **MCP `2026-07-28` is the stable spec revision** (released 2026-07-28): stateless protocol (no `initialize` handshake), `server/discover`, Multi Round-Trip Requests, required `resultType`, deterministic `tools/list` ordering "to enable client-side caching and improve LLM prompt cache hit rates", `ttlMs`/`cacheScope` cache hints, and documented **OpenTelemetry trace-context propagation in `_meta`** (`traceparent`, `tracestate`, `baggage`).
- **OTel GenAI semconv is still churning**: `open-telemetry/semantic-conventions-genai` (324 stars, pushed 2026-09-01) merged "gen-ai: deprecate per-message finish reason" on 2026-09-01 — reinforcing idea 2's "pin a version" caveat rather than changing its ranking.

## Ranked new ideas

### 8. Trust upstream totals: read OpenCode's authoritative session columns and reconcile

**Description.** Extend the OpenCode reader to select upstream's own session-level `cost`, `tokens_input/output/reasoning/cache_read/cache_write`, `parent_id`, `project_id`, `summary_additions/deletions/files`, `time_compacting` and `agent` columns when present, using them as the primary numbers and the message-derived sums as a cross-check. Surface the delta between stored and derived as a parser-fidelity signal in `data_health`.

**Axis.** Pricing and cost-data layer.

**Basis.** `direct:` agenttrace reads five columns (`sqlite_sessions.rs:267`) and re-derives everything by summing per-message JSON through the unguarded accumulators at `sqlite_sessions.rs:410-413` — the exact path the pass-2 assessment (N1/N2) panics on. `external:` upstream's `session` table now carries `cost real not null default 0` and all five token counters as first-class columns, plus `parent_id` for session hierarchy (`packages/core/src/session/sql.ts`, fetched 2026-09-02 via `gh api`).

**Rationale.** This is the rare change that is simultaneously a hardening fix, a fidelity upgrade, and new capability. Reading numbers the producer already computed removes the overflow class at its root instead of clamping it; the derived-vs-stored delta generalizes idea 6's ledger from an aspiration to a cheap, always-on reconciliation; `parent_id` unlocks subagent fan-out attribution; and `summary_additions/deletions/files` gives delivery evidence without spawning `git log` per root (pass-2 N10). The reader is already schema-tolerant (`sqlite_has_column`), so the pattern exists.

**Downsides.** Two sources of truth means documenting which wins; older databases without the columns need the derived path retained; `parent_id` semantics (subagent vs share vs revert) need verifying against upstream code before being labelled.

**Confidence.** 88%. **Complexity.** Medium.

### 9. Statusline output surface: `agenttrace statusline`

**Description.** A new output mode that reads the host agent's statusline JSON on stdin and prints one or two styled lines — spend-to-date and burn rate for the current session/project from local logs, cache hit ratio, and threshold colouring. Claude Code integration first (`statusLine` setting), since the mechanism is documented and agenttrace already parses its transcripts.

**Axis.** Interop and integration surface.

**Basis.** `external:` Claude Code's status line "runs any shell script you configure. It receives JSON session data on stdin and displays whatever your script prints" (`code.claude.com/docs/en/statusline.md`, fetched 2026-09-02), and upstream now feeds it a `prompt_cache` object and `rate_limits.spend_limit` (CHANGELOG, fetched via `gh api`). Two of the fastest-growing competitors are statusline plugins — claude-dashboard (563 stars) and cship (417 stars) — and aqua5230/usage (306 stars, pushed 2026-09-02) puts quota and burn rate on the desktop. `direct:` agenttrace has text/JSON/Markdown/HTML report formats and no statusline output adapter — `grep -rn "statusline\|statusLine" crates/` matches only the TUI's internal `loading_status_lines` helper (`presentation.rs:3055`).

**Rationale.** Every competitor above is single-agent, mostly Claude-only, and derives cost per-run. agenttrace's differentiator — cross-provider, cached, post-run evidence — is exactly what a statusline wants: a warm session cache makes the invocation a few milliseconds, and the same line can reflect Claude Code, Codex and OpenCode spend in one place. This is the cheapest reach multiplier available: one output adapter for a documented, scriptable surface, with no server and no new data source.

**Downsides.** Statusline scripts run on every render, so latency and cache warmth become user-visible contracts; stdin-JSON shape varies per host agent and needs a defensive parser; must not drift toward live streaming (non-goal) — it stays a post-run read.

**Confidence.** 84%. **Complexity.** Low.

### 10. Compaction and re-cache cost analytics across providers

**Description.** First-class reporting on context compaction: when a session compacted, why (upstream records a reason), what it cost to re-warm the cache afterwards, and how much history was summarized away — surfaced as a per-session line and an aggregate trend, alongside the existing `--context-trends` (`governance.rs:583`) and `--waste` reports.

**Axis.** Diagnosis intelligence.

**Basis.** `external:` OpenCode executes automatic compaction "before provider turns using the complete estimated request and absolute model-aware headroom" and stores durable completed events with "stable message identity, **reason**, summary, and recent context", plus `session_context_epoch` and `session.time_compacting` (`specs/v2/schema-changelog.md`, 2026-06-05 entry, fetched via `gh api`); Claude Code's `/cost` now shows a per-session prompt-cache line with "hit ratio, misses, tokens re-cached, warm/cold", and its resume hooks receive an "estimated re-cache cost" (CHANGELOG). `direct:` agenttrace already computes cache hit rate in `waste.rs` and context growth in `governance.rs:583` but has no compaction concept.

**Rationale.** Both majors independently standardized on the same vocabulary in the same release cycle — cache re-warm cost after compaction. That is upstream telling the ecosystem what users now ask about. agenttrace's `--waste` already has the denominator (cache read/write tokens per session); what is missing is the *event* framing — which sessions paid the re-cache tax, how often, and whether compaction was the cause. Cross-provider compaction cost is something neither upstream surfaces: Claude Code shows its own sessions, OpenCode its own; nobody shows both.

**Downsides.** Compaction events live in different places per provider (durable rows in OpenCode, transcript markers in Claude Code), so coverage will be `Detailed` for some sources and absent for others — the existing capability labels must carry it; reason taxonomies differ upstream and need mapping, hedged labels.

**Confidence.** 78%. **Complexity.** Medium.

### 11. Upstream schema-drift tracker against published changelogs

**Description.** A network-explicit, separately-scheduled job (or release-checklist step) that reads upstream's *published schema contracts* — OpenCode's `specs/v2/schema-changelog.md` and dated `packages/core/src/database/migration/*` files — and diffs them against what agenttrace's readers expect, filing drift as a visible failure. No session samples required.

**Axis.** Parser and source coverage.

**Basis.** `external:` OpenCode maintains a dated, public schema changelog whose entries state "whether consumers or stored data need compatibility handling", plus one migration file per schema change (`specs/v2/schema-changelog.md`; `packages/core/src/database/migration/20260622142730_simplify_session_context_epoch.ts` et al., fetched via `gh api`). The storage layer is actively reshaping (`specs/storage/remove-opencode-db.md`, Groups 1–2 Completed). `direct:` agenttrace's reader is schema-tolerant by column sniffing (`sqlite_sessions.rs:261`), which means it degrades *silently* — a renamed column produces zeros, not an error (the pass-2 N7 finding is this failure mode for timestamps).

**Rationale.** Idea 5 proposed a live-sample canary; this is its cheaper sibling and should land first. Tracking prose contracts instead of data samples avoids the hygiene problem entirely (no donated transcripts), works for formats with no stable public sample, and upgrades the existing "compatible with schema 17/4" claims from point-in-time assertions to continuously checked ones. For the one upstream that publishes contracts, silent drift becomes a diff alert.

**Downsides.** Only as good as upstream's changelog discipline — it covers OpenCode well and Codex/Claude Code not at all (idea 5 remains the answer there); parsing markdown changelogs is brittle; must stay off the default test path.

**Confidence.** 80%. **Complexity.** Low.

### 12. ureq 2→3 upgrade: make the one network call proxy-governable

**Description.** Move `download_pricing` (`pricing.rs:316`) from bare `ureq::get` on 2.12.1 to an explicit `Agent` on ureq 3.x, which reads `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY` (with lowercase variants) and `NO_PROXY` automatically. Ship behind the MSRV bump 1.80 → 1.85 and add an MSRV CI job, closing prior assessment F15 in the same motion.

**Axis.** Trust, determinism and hygiene.

**Basis.** `external:` ureq 3.4.0 (2026-08-08) documents automatic environment-proxy configuration including `NO_PROXY` wildcard/dot-suffix/match-all rules (README, fetched 2026-09-02); its MSRV is 1.85. `direct:` the repo pins `ureq = "2.12"` (Cargo.toml:34) resolving to 2.12.1, and the pass-2 assessment reproduced that `--update-pricing` ignores a dead `https_proxy` and downloads the full 2,090,796-byte catalog directly (finding N4); the repo's `rust-version = "1.80"` (Cargo.toml:12) is 18 minor versions behind stable 1.98 and untested in CI.

**Rationale.** Offline-by-default made the *default* path trustworthy; the *escape hatch* is now the only ungoverned network action, and it is ungovernable by the standard operator mechanism. One dependency bump and one explicit `Agent` converts `PRIVACY.md`'s "one documented network action" into one that respects corporate egress policy — and the required MSRV bump finally forces the MSRV CI job the roadmap already owes.

**Downsides.** Breaking API change (`ureq::get` → `Agent`), an MSRV bump that may constrain downstream packagers, and a dependency refresh ripple (rusqlite 0.32 → 0.40.2 and crossterm 0.28 → 0.29 are also behind) best batched into one maintenance release.

**Confidence.** 90%. **Complexity.** Low.

## Rejection Summary (second pass)

- **Agent loop / repetition detector** — checked the strongest recent signal, claude-code #77136 (541 reactions, 111 comments): it is about *model prose quality* ("repetitive rhetorical tics"), not agent-task looping. No grounded basis survives; analogy-only.
- **Native menu-bar / tray app** — subject replacement: aqua5230/usage (306 stars, pushed 2026-09-02) owns that surface, and a GUI contradicts the single-binary local-first posture. The statusline (idea 9) reaches the same user need without leaving the terminal.
- **`opencode stats` feature parity** — upstream (203k stars) now ships basic cost/token stats natively; chasing parity is a losing race. Noted as moat defense instead: the uncontested lanes are cross-provider breadth, diagnosis, and CI gates.
- **Joining the provider-count race (31/37 tools)** — TokenTracker and CodeBurn compete on volume; agenttrace's deterministic generated-fixture discipline is the opposite trade (every parser needs fixtures and drift tracking). Incremental parser coverage stays roadmap lane 1; a count race is rejected as strategy.
- **Implementing MCP 2026-07-28 client/server semantics** — the stable revision is protocol-level (stateless handshake, `server/discover`, MRTR); agenttrace analyses logs post-run and is not an MCP endpoint. The durable hooks are (a) richer `serverInfo`/`clientInfo` identity likely to appear in future session logs, and (b) OTel `traceparent` in `_meta`, which reinforces idea 2 rather than needing new work.
- **MSRV bump as a standalone item** — tactical; folded into idea 12 where it is load-bearing.

## Next steps

- **Highest leverage now:** idea 8 (reads upstream's own numbers — hardens N1/N2 at the root), then 12 (small, closes N4 and F15 together).
- **Cheapest reach:** idea 9 (statusline adapter).
- **Pair with cycle-2 planning:** idea 10 (compaction analytics) and 11 (schema-drift tracker) both extend lanes the roadmap already names; 11 is the low-risk prefix of idea 5.

---

# Third research pass — 2026-09-02 (upstream, ecosystem, dependencies, standards)

Baseline: HEAD e005952 plus the uncommitted cycle-1 tree (159/159 tests, clippy
clean), same tree assessed by passes 1–4. Candidates 13–23 below are net-new:
each is checked against candidates 1–12 and both rejection summaries. Method
disclosure: no subagent dispatch available in the harness; all lenses ran
in-thread. Evidence types used: live upstream files on this machine, the
crates.io API (fetched 2026-09-02 with a compliant User-Agent), the GitHub API,
and competitor READMEs fetched live.

## Delta grounding — new evidence gathered this pass

### Upstream data sources observed live (this machine, 2026-09-02)

- `~/.claude/stats-cache.json` (861 B): Claude Code's own computed totals —
  `totalSessions`, `totalMessages`, `dailyActivity[]`, `dailyModelTokens[]`,
  `longestSession{sessionId,duration,messageCount}`, `firstSessionDate`,
  `hourCounts`, `lastComputedDate: 2026-08-09`. Stale by weeks relative to
  transcripts written 2026-09-02 (computed on demand, `/stats`), which is itself
  a reconciliation signal.
- `~/.claude/history.jsonl` and `~/.codex/history.jsonl`: per-prompt index rows
  (`{display, timestamp, project, sessionId}` and `{session_id, ts, text}`)
  covering every session at KB scale versus MB-scale transcripts.
- Live Claude Code transcript event fields beyond our fixtures
  (`testdata/claude-code-preamble.jsonl` carries `isSidechain` only):
  `gitBranch` per event, `usage.server_tool_use{web_search_requests,
  web_fetch_requests}`, `usage.service_tier`,
  `usage.cache_creation{ephemeral_1h_input_tokens, ephemeral_5m_input_tokens}`.
  `parser.rs` reads none of `gitBranch`, `isSidechain`, `service_tier`, or
  `server_tool_use` (grep: 0 matches each; `cache_creation` is read as the flat
  counter). So the repo's own fixtures lag the live format — a concrete
  instance of the drift tracked by ideas 5/11.
- `~/.omx/metrics.json` (a third-party tool's state) records
  `five_hour_limit_pct` and `weekly_limit_pct` — billing-window pressure is a
  fact other local tools already surface (reinforces idea 3, not new here).
- Provider breadth gap versus ccusage (18,289 stars, pushed 2026-09-01): it
  parses Amp, Droid, Codebuff, Goose, OpenClaw, Grok Build CLI, and ZCode in
  addition to the sources `--doctor` lists. Recorded as lane-1 input; the
  count-race strategy stays rejected.

### Dependency currency (crates.io API, fetched 2026-09-02)

| crate | pinned (lockfile) | latest stable | note |
|---|---|---|---|
| rusqlite | 0.32.1 | 0.40.2 (2026-08-08) | bundled SQLite 3.46.0 (2024-05) via libsqlite3-sys 0.30.1; ~4 majors behind |
| crossterm | 0.28.1 | 0.29.0 (2025-04-05) | ratatui 0.30.2 exposes `crossterm_0_28` alongside a default crossterm path — a known upgrade route exists |
| clap | 4.5.x | 4.6.6 (2026-08-06) | one minor behind |
| ratatui | 0.30.x | 0.30.2 (2026-06-19) | current |
| ureq | 2.12 | 3.4.0 (2026-08-08) | already idea 12 |
| chrono / serde_json | 0.4.x / 1.0.x | 0.4.45 / 1.0.151 | compatible ranges, current |

## Ranked new ideas

### 13. Absolute-time scoping: `--since`/`--until` and timezone-aware day windows

The CLI exposes only `--range` presets; there is no `--since`/`--until` and no
`--timezone` (grep of `main.rs`: zero hits). ccusage ships both
(`--since`/`--until` date filtering, `--timezone` for date grouping) and lists
them as headline features. Absolute scoping is also the capability version of
assessment P3-2 (`--range today` is a UTC-midnight window; reproduced with a
2026-09-02T01:00+09:00 session invisible to `--range today` in Asia/Tokyo).
Acceptance: `--since 2026-08-01 --until 2026-08-31` windows inclusive of bounds,
`--timezone` accepted by `insights`/TUI range views, unit tests pinning day
boundaries in two timezones, and the P3-2 reproducer visible in the local zone.
Evidence: `crates/agenttrace-cli/src/main.rs` (no since/until/timezone flags),
`insights.rs:50,65-70` (UTC-midnight windows), ccusage README features
(`--since`, `--until`, `--timezone`), pass-3 P3-2 reproducer.

### 14. Sub-agent (sidechain) attribution

Claude Code marks sub-agent turns with `isSidechain` per event; the repo's own
fixture `testdata/claude-code-preamble.jsonl` carries it and `parser.rs` never
reads it (grep: 0). Today `overview.by_agent` groups by provider/source
(demo output: `[{"name":"Hermes Agent (JSONL)",...}]`), not by sub-agent.
ccusage's `--by-agent` groups Claude Code usage by Task-tool agent — a distinct
axis with demonstrated demand. Acceptance: sidechain turns counted per parent
session, `by_sidechain` in JSON overviews, TUI drill-down showing sub-agent
share of cost/tokens, fixtures with both markers. Evidence: fixture grep,
parser grep, demo `by_agent` shape, ccusage README (`--by-agent`).

### 15. Tier- and server-tool-aware cost accuracy

Live Anthropic usage blocks on this machine record
`server_tool_use{web_search_requests, web_fetch_requests}` and `service_tier`;
the vendored pricing snapshot carries per-model token rates only (no
server-tool or tier rates), and `parser.rs` reads neither field. Consequences:
sessions using web search or priority tier are priced as if neither existed —
the core "cost" number is silently incomplete, and the roadmap's own principle
("prefer facts the provider already recorded") is violated by dropping two such
facts. Acceptance: server-tool counters surfaced in `--audit`, tier recorded on
sessions, pricing snapshot gains optional server-tool/tier rate keys with an
explicit "not priced" label when absent rather than a silent zero. Evidence:
live transcript field extract, `parser.rs` greps, `pricing_snapshot.json` keys
(`_snapshot` + model ids only).

### 16. Read provider-recorded git branches (`gitBranch`) for delivery evidence

Every Claude Code event carries `gitBranch` (empty string in our live sample —
absence is itself detectable), and the current delivery-evidence feature
correlates by git timestamps instead. Reading the recorded branch is cheaper,
deterministic, and upstream-authoritative. Acceptance: `--delivery-evidence`
prefers transcript-recorded branches, falls back to the git-timestamp path with
an explicit label, fixtures carry both. Evidence: live transcript extract,
`parser.rs` grep (0), roadmap principle line.

### 17. Reconcile against Claude Code's own totals (`stats-cache.json`)

The Claude analogue of idea 8 (OpenCode's authoritative columns):
`~/.claude/stats-cache.json` publishes `totalSessions`, `dailyModelTokens`, and
`longestSession`, but only recomputed when the user runs `/stats`
(`lastComputedDate` 2026-08-09 here versus transcripts from 2026-09-02).
Acceptance: an overview field that reports agreement/divergence against the
cache when fresh, and a `stale-as-of` note when `lastComputedDate` trails the
newest transcript. Evidence: file walked on this machine (schema above).

### 18. Prompt-history index for fast discovery, naming, and search

`~/.claude/history.jsonl` and `~/.codex/history.jsonl` are KB-scale indexes of
every prompt with `session_id`/`ts`/`project`. Today session names come from the
first user message inside an MB-scale transcript (`lib.rs` session naming), and
discovery must parse every file. Acceptance: history-derived session titles and
a `--search` pass over the prompt index that works before transcripts parse;
explicit degradation when a provider has no history file. Evidence: both files
sampled above; `lib.rs:457` naming path; `--doctor` timings from pass 3
(8.0 s cold parse of 1500 files).

### 19. `agenttrace mcp` — read-only analytics as MCP tools

No usage-analytics MCP server occupies this space (GitHub search 2026-09-02:
39 results, none relevant; ccusage has none). Meanwhile every major coding
agent speaks MCP, and this repo already ships an agent-facing surface
(`.codex-plugin/plugin.json` plus a Pi skill), so "agent self-audits its own
spend and regressions" is one command away for every MCP-capable client.
Scope stays read-only and post-run: `session_overview`, `top_sessions`,
`anomalies`, `compare_baseline`. Acceptance: stdio server answering those
tools over the existing core functions, a fixture-driven integration test, and
no new blocking network dependency. Evidence: GitHub search result set,
`.codex-plugin/plugin.json`, Claude Code plugins surface (anthropics/claude-code,
143,739 stars, README "Plugins" section, pushed 2026-09-01).

### 20. SARIF 2.1.0 export for CI gates

The governance/baseline gate already produces pass/fail findings, but only in
agenttrace's own JSON. SARIF v2.1.0 (OASIS standard) is the interchange format
GitHub code scanning ingests from any CLI (docs.oasis-open.org SARIF v2.1.0,
docs.github.com "SARIF support for code scanning"). A `-f sarif` target turns
anomaly and gate findings into first-class CI results next to linters.
Acceptance: gate failures map to SARIF results with stable rule ids, the file
validates against the published schema, CI job uploads it with
`github/codeql-action/upload-sarif`. Evidence: OASIS spec URL (HTTP 200),
GitHub docs URL, existing gate code path.

### 21. Configuration file with a published JSON Schema

agenttrace is env-var-only today (no config-file code in `main.rs` or core).
ccusage ships config files "complete with IDE autocomplete and validation".
A `~/.config/agenttrace/config.json` (respecting `XDG_CONFIG_HOME`) for
defaults — `range`, `format`, `project`, `sources` — removes repeated flags from
the documented CI and statusline recipes. Acceptance: schema published in-repo
and referenced by `$schema`, unit tests for precedence (flag > env > file >
default), docs updated. Evidence: config grep (zero), ccusage README feature
line.

### 22. Shell completions and man page via `clap_complete`

No completion or man generation exists anywhere in the workspace (grep:
`clap_complete` zero matches). One subcommand (`agenttrace completions
<bash|zsh|fish>` or a build-time generation step) plus Homebrew/npm packaging
wiring covers the standard DX surface, and `--help` already defines the
vocabulary. Acceptance: generated completions exercised in CI (spawn shell,
complete a partial flag), man page shipped in release artifacts. Evidence:
workspace grep, clap 4.6 ecosystem standard, Homebrew formula already in-repo.

### 23. Dependency-currency lane: bundled SQLite 3.46.0 and crossterm 0.28

Generalizes idea 12 (ureq) into one roadmap item: rusqlite 0.32.1 → 0.40.2
moves the bundled SQLite from 3.46.0 (2024-05) to current; the threat model is
precisely "parse untrusted foreign databases" (`opencode.db`, hermes
`state.db`), so an aging bundled engine is a security-posture item, not
cosmetics. crossterm 0.28.1 → 0.29.0 has a documented route through ratatui
0.30.2's `crossterm_0_28`/default feature split. clap 4.5 → 4.6.6 rides along.
Acceptance: lockfile bumps with `cargo test --workspace` green, an
`update-deps` CI job that fails on drift beyond one minor, MSRV statement
re-checked. Evidence: crates.io table above, `libsqlite3-sys-0.30.1/sqlite3/
sqlite3.h` `SQLITE_VERSION "3.46.0"`, `Cargo.lock` pins.

## Rejection Summary (third pass)

- **5-hour billing-window blocks** — ccusage's headline feature and locally
  corroborated by `~/.omx/metrics.json`; still idea 3's scope (subscription
  limit-pressure), so recorded as strengthening evidence, not a new idea.
- **Claude Code plugin/marketplace distribution** — real audience (143k-star
  repo, plugin system), but subsumed by idea 19: one MCP server serves Claude
  Code, Codex, and every other MCP client, while a `.claude-plugin` manifest
  serves one host and adds a second surface to keep in sync.
- **Per-model cost `--breakdown` parity** — already exists
  (`overview.by_model`, `by_agent`, `by_project` keys verified on `--demo`).
- **Joining the provider-count race** (Amp, Droid, Codebuff, Goose, OpenClaw,
  Grok Build CLI, ZCode) — the ccusage provider list above is recorded as
  lane-1 input; the strategy itself was rejected in pass 2 and stays rejected.
- **Live-format fixture refresh as a standalone idea** — the live-field gap
  (`gitBranch`, `server_tool_use`, `service_tier`, `cache_creation` split) is
  folded into ideas 15/16 and the drift tracker (idea 11); a fixtures-only
  refresh would duplicate idea 5's canary.
- **Reading `~/.omx`/`~/.omc` state files as sources** — third-party tool
  state, not agent sessions; `agents_spawned` summaries are interesting but
  idea 14 (sidechains) gets the same signal from first-party transcripts.

## Next steps

- **Highest leverage:** 15 (cost accuracy on facts the provider already
  records), then 13 (absolute scoping; subsumes P3-2's fix), then 23.
- **Cheap, immediate:** 22 (completions), 16 (gitBranch), 17 (stats-cache
  reconciliation).
- **Build-sized, strategic:** 19 (MCP surface) and 20 (SARIF) open reach lanes
  no competitor occupies; pair with the growth lane in the roadmap.
- **Feeds cycle-2 planning:** 14 (sidechain attribution) and 18 (prompt index)
  extend the attribution and discovery lanes respectively.

---

# Fourth research pass — 2026-09-02 (new local sources, outcome analytics)

Same tree and method as the third pass: HEAD e005952 plus the uncommitted
cycle-1 tree, re-verified green before research (`cargo test --workspace`
159/159 passed, 0 failed). Candidates 24–28 are net-new against 1–23 and every
prior rejection. No subagent dispatch available; lenses ran in-thread.
Evidence types: live upstream files on this machine, a full local
transcript event-type census, GitHub issue search, and attempted provider-doc
verification. Skill routing: ce-ideate (same as pass 3).

## Delta grounding — new evidence gathered this pass

### Local transcript event-type census (all `~/.claude/projects` files)

24 `user`, 22 `assistant`, 4 `queue-operation`, 4 `file-history-snapshot`,
2 `summary`. The parser reads none of the three non-message types
(`queue-operation`, `file-history-snapshot`: grep 0 in `parser.rs`; the single
`"summary"` hit at `parser.rs:1203` reads the `summary` *field* of
`branch_summary`/`compaction` events, not `type: "summary"` lines). Sampled
live shapes:

- `{"type":"queue-operation","operation":"dequeue","timestamp":...,"sessionId":...}`
- `{"type":"file-history-snapshot","messageId":...,"snapshot":{"messageId":...,"trackedFileBackups":{},"timestamp":...},"isSnapshotUpdate":false}`
- `~/.claude/todos/<uuid>-agent-<uuid>.json` — per-session TodoWrite state
  (file present, empty array in the local sample; schema is the TodoWrite list).

### New source-class discovery: Hermes conductor spool events

`~/.hermes/sessions` — the path agenttrace's Hermes JSONL source watches —
contains `request_dump_*.json` files, not sessions; `--doctor` reports the
row as `found` with `found=0`. Hermes's actual JSONL event streams live in
`~/.hermes/conductor-delegate-spool-*/events/*.jsonl` — **395 files on this
machine**, none discovered by agenttrace. Census of 60 sampled files:
`delegate_turn_started` 61, `delegate_turn_completed` 58,
`phase_result_repair_started` 1, `phase_result_repair_completed` 1; zero rows
carry token or cost keys (lifecycle timing only). Row shape:
`{action_id, attempt_id, backend, event, phase_id, run_id, written_at}`.

### Codex goals database

`~/.codex/goals_1.sqlite` exists with tables
`thread_goals(thread_id, goal_id, objective, status, token_budget,
tokens_used, time_used_seconds, created_at_ms, updated_at_ms)` and
`thread_goal_continuation_deferrals(thread_id)` — Codex's own per-task
budget-vs-actual accounting. Honest caveat: **0 rows** on this machine
(schema-only evidence; the feature is enabled but unused here).

### Ecosystem checks (2026-09-02)

- GitHub issue search `repo:anthropics/claude-code usage export` (899
  results): no high-signal CSV/spreadsheet ask; top reaction-sorted hits are
  unrelated. Side finds recorded below as radar notes.
- `repo:anthropics/claude-code csv usage` (97 results): #56317 —
  "claude_code_cost_usage_USD_total OTEL counter collides across parallel
  processes" (3 reactions) — direct evidence for idea 2's counter hygiene;
  #84545 — Max-subscription sessions billed to Console — billing-attribution
  confusion that authoritative reconciliation (idea 17) addresses.
- Anthropic Usage API docs could NOT be verified this pass: 404 at
  `docs.claude.com/en/api/usage`, `platform.claude.com/en/api/usage`, and the
  `docs.anthropic.com` redirect — the endpoint is publicly announced but its
  documentation URL churned; candidate rejected below pending a stable
  citation.
- ACP (zed-industries/agent-client-protocol) repo metadata returned no data
  in two API attempts; no ACP artifacts exist on this machine → no grounded
  basis, rejected below.

## Ranked new ideas

### 24. Split wait-time from agent-time using queue-operation events

Claude Code's message queueing writes `queue-operation` enqueue/dequeue rows
with timestamps; agenttrace currently measures only message-to-message gaps,
so a session where the user queued prompts while the agent worked is
indistinguishable from one where the model was slow. Reading the two event
types separates wall-clock into queued vs active — a latency-diagnosis signal
that is exactly job #2 of the roadmap. Acceptance: `--audit` and insights
report queued/active split when queue events exist, omitted (not zero) when
they do not; fixtures with enqueue+dequeue pairs; the P3-3 percentile question
re-checked against the new split. Evidence: 4 live events, shape above;
`parser.rs` grep 0.

### 25. File-change scope evidence from file-history snapshots

Claude Code persists `file-history-snapshot` events (the /rewind substrate)
whose `snapshot.trackedFileBackups` maps the files the agent touched at each
message. That is provider-recorded change scope — today agenttrace infers
file activity only from tool-call names. Acceptance: per-session
files-touched list in reports and the TUI detail view, sourced from snapshot
events when present with explicit absence labeling; fixtures carrying a
non-empty `trackedFileBackups` map. Evidence: 4 live events, structure
confirmed (map empty in the local sample — disclosed); `parser.rs` grep 0.

### 26. Ingest Hermes conductor spool events; deconfuse the doctor row

A new source class: orchestration telemetry. 395 undetected JSONL files on
this machine record delegate turn boundaries per run/phase/attempt, enabling
orchestration-overhead analytics (delegate durations, attempts per phase,
repair ratio) that map to the roadmap's "multi-agent dashboards" adjacent
surface — timing-only, since no row carries tokens. The same change fixes a
doctor lie: `~/.hermes/sessions` holds `request_dump_*.json`, not sessions,
yet is reported as a `found` Hermes source with `found=0`. Acceptance: spool
dirs discovered, orchestration metrics in `--overview -f json`, request dumps
labeled as such or excluded with a reason, fixtures from the sampled shapes.
Evidence: census and row shape above; `--doctor` output from this pass.

### 27. Codex task budgets: read `thread_goals` for budget-vs-actual

The goals DB records `objective`, `status`, `token_budget`, `tokens_used`,
`time_used_seconds` per thread — the provider's own per-task accounting, at
finer grain than idea 8 (which reads OpenCode session columns) and feeding
idea 3 (limit pressure) at task level. Joining `thread_id` to Codex sessions
gives budget overrun rates with zero derivation. Acceptance: goal-level
rows joined to sessions when IDs match, unjoined goals reported as unmatched
with counts, schema-only deployments (0 rows) degrade silently. Evidence:
schema above; 0 local rows disclosed as the current limitation.

### 28. Session outcome analytics from todos and summary events

Job #1 is reviewing agent history, but agenttrace reports no completion
signal: whether the task's todos were finished, or how Claude itself titled
the session (`type: "summary"` lines, and `~/.claude/todos/*.json`
per-session TodoWrite state; the in-transcript `todos` field appears on user
events). Outcome = todo states at last event plus summary presence; feeds
triage (abandoned vs completed vs errored) without touching conversation
bodies — todo *states* and titles only. Acceptance: outcome field on sessions
(completed/partial/none + summary-title source labeled), privacy doc updated
to state todo contents are not retained, fixtures with each terminal state.
Evidence: todos file present locally (empty sample), transcript `todos` field
seen in the pass-3 walk, `type: summary` census (2 live rows), `parser.rs`
todos grep 0.

## Rejection Summary (fourth pass)

- **CSV/spreadsheet export** — searched for demand where it would live
  (anthropics/claude-code, 899 + 97 results): no high-signal ask; finance-team
  reasoning alone is not a grounded basis. Revisit if a competitor ships it
  with traction.
- **Anthropic Usage API reconciliation** — the strongest authoritative-totals
  extension after idea 17, but its documentation URL 404s at every variant on
  2026-09-02; citing an unverifiable endpoint would break this document's
  evidence rule. Re-run when docs.claude.com settles.
- **ACP transcript ingestion** — repo metadata unreachable in two attempts
  and no ACP artifacts on this machine; no grounded basis this pass.
- **Multi-account usage aggregation** — claude-code #18435 (897 reactions) is
  about account *switching* in Claude Code, not analytics; mapping too
  indirect. Radar note only.
- **Parsing `~/.claude/statsig` / `~/.claude/telemetry` / `~/.claude/debug`**
  — vendor telemetry and debug logs, not session evidence; out of posture.
- **Reading the 395 spool files as *token* evidence** — they carry no token
  or cost keys (verified); idea 26 is scoped to timing for that reason.

## Radar notes (no action)

- claude-code #56317: OTEL cost-counter collision across parallel processes —
  input for idea 2's metric hygiene, not new work here.
- claude-code #84545: Max-subscription sessions billed to Console — more
  ammunition for idea 17's reconciliation framing.
- `thread_goal_continuation_deferrals` table exists; if goals see adoption,
  deferral counts are a follow-on metric for idea 27.

## Next steps

- **Highest leverage:** 24 (wait/active split — new diagnosis signal from
  data already on disk), then 26 (new source class + fixes a doctor lie),
  then 25.
- **Cheap once 24/25 land:** 28 (outcome field) and 27 (Codex budgets,
  schema already mapped).
- **Dependency lane unchanged:** idea 23 remains the umbrella; nothing new
  this pass supersedes it.

---

# Fifth research pass — 2026-09-02 (authoritative-cost tables, authority context)

Same tree and method: HEAD e005952 plus the uncommitted cycle-1 tree, re-verified
`cargo test --workspace` 159/159 passed, 0 failed before research. Candidates
29–32 are net-new against 1–28 and every prior rejection. No subagent dispatch
available; lenses in-thread. Skill routing: ce-ideate (as passes 3–4).
Evidence: SQLite schemas and live Codex rollout files on this machine, parser
source reads, live CHANGELOG/release fetches on 2026-09-02.

## Delta grounding — new evidence gathered this pass

### Hermes `state.db` carries an authoritative per-model cost table

`session_model_usage(session_id, model, billing_provider, billing_base_url,
billing_mode, task, api_call_count, input_tokens, output_tokens,
cache_read_tokens, cache_write_tokens, reasoning_tokens, estimated_cost_usd,
actual_cost_usd)` — provider-recorded *actual* and *estimated* cost per session
per model, with billing mode. Repo-wide grep for `session_model_usage`: **zero
matches**; `sqlite_sessions.rs` queries only `message`, `part`, `session`,
`sessions`. Also unread: `async_delegations` (delegation graph:
`delegation_id, parent_session_id, state, dispatched_at, completed_at,
delivery_state, delivery_attempts`), `sessions.display_name` /
`thread_id` / `chat_type` (provider-recorded titles and threading),
`compression_locks` (compaction contention — feeds idea 10), `messages_fts`.

### Codex rollout files: turn_context authority context, token_count detail

Live sample (`~/.codex/sessions/2026/08/10/rollout-*.jsonl`, event types
`event_msg`/`response_item`/`session_meta`/`world_state`/`turn_context`):
`turn_context` records `approval_policy: "never"`, `sandbox_policy:
{type: "read-only"}`, `permission_profile: {type: "managed", file_system:
{type: "restricted", ...}}`, `timezone: "Etc/UTC"`, `workspace_roots`,
`collaboration_mode` (nested model settings), `multi_agent_version: "v1"`,
`realtime_active`, `model`, `summary`. The parser's `turn_context` arm
(`parser.rs:1910-1927`) keeps **only `model`**. `token_count` events are read
(`parser.rs:1931`, `codex_token_count_usage` at `parser.rs:2046`), but
`reasoning_output_tokens` is **added into output** (`parser.rs:2073-2075`),
so the provider's reasoning split is discarded. `world_state` (records the
in-force AGENTS.md, including autonomy directives) and `agent_reasoning`,
`task_started`, `task_complete` events: zero reads.

### Upstream change sweep (fetched live 2026-09-02)

- anthropics/claude-code CHANGELOG (6179 lines, current): permission surface
  is expanding fast — `permissions.blockReadsOutsideWorkingDirectories`,
  `allowManagedPermissionRulesOnly`, per-directory `defaultMode`/agent
  `permissionMode`, teammate permission requests, `/doctor` warnings for stale
  sandbox masks; subagents gained forced-model env (`CLAUDE_CODE_SUBAGENT_MODEL_FORCE`),
  background subagents and teammates with live token counters. Implication:
  both major providers now record authority and sub-agent structure as
  first-class data — candidates 14/30 gain upstream tailwind.
- openai/codex releases: `rust-v0.153.0-alpha.5` published 2026-09-02 —
  rate-limit banners with usage/credit actions, per-turn token budgets
  (Guardian), per-MCP-tool `output_token_limit`, history-compaction-aware
  approval reviews. Implication: budgets (idea 27), limit pressure (idea 3),
  and compaction (idea 10) are all active upstream investment areas.

## Ranked new ideas

### 29. Read Hermes `session_model_usage`: actual vs estimated cost, per model

The single most authoritative cost fact available anywhere in agenttrace's
sources: the orchestration layer's own accounting, including `actual_cost_usd`
alongside `estimated_cost_usd`, `billing_mode`/`billing_provider`, and
`api_call_count`. Two capabilities fall out: (a) idea-8-style trusted totals
for Hermes (replacing derived sums); (b) a **pricing-accuracy signal** — where
actual and estimated diverge, the vendored snapshot or the estimator is wrong,
turning every Hermes-heavy machine into a continuous calibration probe for the
offline pricing snapshot (candidate 1's residual risk). Acceptance: overview
prefers authoritative totals with a source label; a divergence metric
(actual-estimated)/actual with threshold; fixtures from the schema above.
Evidence: schema dump, repo-wide grep 0, `sqlite_sessions.rs` table list.

### 30. Codex `turn_context`: provider-recorded authority and environment context

Today authority evidence is inferred from tool-call shapes; Codex records the
actual policy in force per turn — `approval_policy`, `sandbox_policy`,
`permission_profile` — plus `timezone` (provider-recorded, feeding candidate
13's timezone handling) and `workspace_roots` (scope evidence). The parser
keeps only `model` (`parser.rs:1910-1927`). Acceptance: sessions carry the
last-seen (and changes across) approval/sandbox/permission profile; TUI and
`--audit` show "ran under read-only sandbox, approval never" style lines;
`world_state` AGENTS.md content deliberately NOT retained (posture: metadata
only, consistent with tool-payload policy). Evidence: live turn_context dump,
parser source lines.

### 31. Reasoning-token share as a first-class metric

Codex records `reasoning_output_tokens` separately; Hermes records
`reasoning_tokens` in both `messages` and the unread usage table. The parser
currently folds reasoning into output for Codex (`parser.rs:2073-2075`), so
"how much of the spend was thinking" is invisible even though both providers
publish it. Distinct from idea 10 (compaction) — this is token *composition*.
Acceptance: reasoning share per session/model in `--overview -f json` and the
TUI, priced at the same output rate with an explicit note that providers bill
it as output; fixtures for both providers. Evidence: parser lines above,
`session_model_usage.reasoning_tokens` column.

### 32. Hermes delegation graph and provider-recorded session identity

`async_delegations` links `parent_session_id` → `delegation_id` with state and
dispatch/complete/delivery timestamps — the real parent-child edges that
candidate 26's spool timing lacks — and `sessions.display_name` /
`thread_id` / `chat_type` are provider-recorded titles and threading that
candidate 18's prompt-derived naming approximates. Acceptance: delegation
edges surfaced as multi-agent evidence (fan-out, depth, delivery failures via
`delivery_state`/`delivery_attempts`), display_name preferred over derived
names when present, thread grouping available in `--sessions`. Evidence:
schema dump; `sqlite_sessions.rs` table list (both unread).

## Rejection Summary (fifth pass)

- **Cache-efficiency/savings analytics** — `--waste` already reports cache hit
  rate, waste cost, and enable-caching suggestions (verified on `--demo`);
  a savings counterfactual would extend it, but that is idea-6/idea-1
  territory, not a new candidate.
- **Reading `world_state` AGENTS.md bodies** — provider-recorded, but it is
  instruction text, not evidence metadata; retaining it breaks the
  tool-payload policy. Candidate 30 explicitly excludes it.
- **Codex `agent_reasoning`/`task_started`/`task_complete` events as separate
  ideas** — timing value is subsumed by existing turn parsing and candidate
  24's queue split; no independent user need.
- **`messages_fts` search integration** — agenttrace's `--search` already
  covers prompts across sources; SQLite FTS would duplicate idea 18's index
  with more coupling.
- **`compression_locks` analytics** — one table, contention windows only;
  folded into idea 10 as an additional column, not a candidate.
- **Claude permission-rule expansion as its own parser work item** — the
  changelog evidence feeds candidates 14/30 and idea 11's drift tracker;
  without a verified transcript field shape on this machine it stays radar.

## Radar notes (no action)

- `--waste` ignores `-f json` (text banner printed regardless) — a surface
  inconsistency for the assess lane, not research.
- Codex `rust-v0.153.0-alpha.5` (2026-09-02): rate-limit banners, turn token
  budgets, MCP `output_token_limit` — all reinforce ideas 3/27 and MCP-cost
  visibility generally.
- Claude Code `CLAUDE_CODE_SUBAGENT_MODEL_FORCE` + background subagents with
  live counters: sub-agent token attribution (candidate 14) is becoming
  first-party; moving on it early keeps the diagnosis moat.

## Next steps

- **Highest leverage:** 29 (authoritative costs + pricing self-calibration),
  then 30 (authority context — job #1 evidence), then 32.
- **Cheap once 29 lands:** 31 (reasoning share is two providers' existing
  columns minus one fold-in).
- **Upstream watch:** re-run the changelog sweep monthly; idea 11's tracker
  should ingest the two CHANGELOG sources verified this pass.

# Extensions research — pass 5 (upstream + ecosystem drift)

**Date:** 2026-09-02 · **Run:** 5d025d55b1194dd1a4dd8784146dfeeb · **Phase:** research
**Method:** ce-ideate in-thread (gather → dedupe vs ideas 1–32 / R4-24..27 → ground every claim → rank). Web via curl (GitHub API, raw.githubusercontent, modelcontextprotocol.io, docs.claude.com, raw provider APIs). Numbering continues at **33**.

**Scope of this pass:** upstream moves since pass 4 (Claude Code, ccusage, opencode, codex, gemini-cli, MCP spec, OTel GenAI, LiteLLM, models.dev) and live on-disk verification against this machine's real session stores.

---

## 1. Live evidence log (all fetched or observed 2026-09-02)

### 1.1 Claude Code 2.1.243–2.1.258 (CHANGELOG, anthropics/claude-code)

- **2.1.243 — `modelPricing` managed setting**: "an organization's contracted per-model rates and discount multiplier are used for `/cost`, the status line, and telemetry cost figures **instead of list price**." Enterprises can now have *contracted* rates that differ from every public catalog. → grounds **candidate 35**.
- **PreModelSwitch / PostModelSwitch hook events** (block/confirm/annotate a model switch) — model switching mid-session is now a first-class upstream event. Combined with ccusage's advisor-model fix (§1.2) this confirms mixed-model sessions are common, not exotic. → grounds **candidate 33**.
- **2.1.24x — `/cost` per-session prompt-cache line** (hit ratio, misses, tokens re-cached, warm/cold) and a `/usage` **Loops breakdown** (per-loop run count, tokens/run, last run). First-party analytics now cover agenttrace's cache-efficiency and loop-detection lanes. Competitive pressure; not new input data for us (still derivable from usage blocks).
- **Background sessions (`claude --bg`), agent-team teammates, background subagents with own transcripts and a `state.json`**; "Monitor" tool. New session surfaces exist upstream, but none observed on this machine's `~/.claude/projects` census (18 files: `user/assistant/queue-operation/file-history-snapshot/summary` only) — recorded as radar, not a candidate.
- **`desktopSessionCleanupPeriodDays`** — upstream *deletes* transcripts after 30 days by default and now exempts in-app desktop sessions. Strengthens the preserved-history durability lane (ideas F4/F6/F7): agenttrace's cache is increasingly the only durable record.
- **2.1.257 — Claude Fable 5.1 (`claude-fable-5-1`)**, $10/$50 per Mtok, $0.25 cache reads. Verified present in our vendored snapshot (`pricing_snapshot.json`, 2,459 keys) **and** models.dev — pricing snapshot is current; no action needed beyond the existing freshness lane.
- `~/.claude/history.jsonl` confirmed on this machine (`{display, pastedContents, timestamp, project, sessionId}`) — idea 18's basis holds live.

### 1.2 ccusage — repo moved, now multi-agent (18,293 ★)

`ryoppippi/ccusage` is gone; canonical repo is **`ccusage/ccusage`** (pushed 2026-09-01). v20 now parses **Claude Code, Codex, OpenCode, Hermes Agent, pi-agent, Qwen, Copilot CLI, Gemini CLI, Grok Build CLI, Amp, Droid** — the *same source list agenttrace covers, including our two home-turf sources (Hermes, oh-my-pi)*. Also shipped: `ccusage statusline` (Beta), `--instances`/`--project` grouping, per-agent JSON `modelBreakdowns`.

Release-level evidence:
- **v20.0.17 (2026-07-10): "Count advisor model usage"** — Claude Code sessions with an advisor model were undercounted until ccusage fixed per-model attribution. agenttrace still has the bug class (§3, candidate 33).
- **v20.0.18 (2026-07-20): "Embed Moonshot/Kimi models from models.dev"** — models.dev adopted as pricing source by the largest player. Strengthens idea 4.
- **v20.0.14 (2026-06-15): "Parallelize file/DB reads across all agent loaders"** + unified byte-level prefilter — they invested in load parallelism. agenttrace's discovery is single-threaded; TUI spawns one loader thread. Perf radar for the roadmap's perf lane.
- **v20.0.20 (2026-08-15): Grok Build CLI adapter** — confirms Grok is a real, parseable source (already on our coverage list; no new evidence needed).

**Scope limit (verified from README):** ccusage remains usage/cost aggregation (daily/weekly/monthly/session, instances, statusline). No loop analytics, no tool latency, no health scoring, no diagnostics, no governance reports, no TUI drill-down. agenttrace's differentiation is intact but the *overlap* is now near-total on the cost surface.

### 1.3 better-ccusage — direct multi-agent competitor

`cobra91/better-ccusage` (81 ★, TypeScript, pushed 2026-07-20): "Analyze your Claude Code/Droid/OpenCode/Zcode/codex/**pi/omp** and all providers … incredibly fast and informative". Covers oh-my-pi (`omp`) explicitly. Small, but it is the exact "all local JSONL agents" lane.

### 1.4 opencode — repo moved to `anomalyco/opencode` (203k ★, v1.18.26, 2026-09-01)

- Migration `20260511173437_session-metadata`: `ALTER TABLE session ADD metadata text`. Schema keeps evolving after our snapshot — strengthens idea 11 (drift tracker) and idea 5 (format canary).
- **Live local DB census** (`~/.local/share/opencode/opencode.db`, read-only): 17 tables, **227 sessions**. Key facts:
  - `parent_id` set on **98/227 (43%)** sessions — session delegation is heavily used. (Idea 8 already plans to read it; this upgrades it from "column exists upstream" to "present in ~half of a real corpus".)
  - `title` populated on **227/227** — and *every single one* is the placeholder `"New session - <timestamp>"`. agenttrace today uses `title` verbatim (`sqlite_sessions.rs:672–676`), so **all 227 local opencode sessions currently get junk names**. → grounds **candidate 34**.
  - `summary_additions/deletions/files/diffs`: columns exist but **0/227 non-zero on this machine** — idea 8's delivery-evidence claim is schema-real but data-empty here; acceptance must degrade gracefully (do not promise populated summaries).
  - `event` + `event_sequence` tables: **27,769 events** keyed `(aggregate_id, seq, type)` with JSON `data` (session.info snapshots, etc.). A per-aggregate monotonic change feed. → grounds **candidate 36**.
  - `agent` column: 123× `build`, 4 scout variants, … — per-agent-mode grouping dimension, part of idea 8's column list.
  - Message `data.modelID` still matches our extraction (`opencode_sqlite_message_model`, sqlite_sessions.rs:552) — **no drift bug**; the session-table `model` blob uses `{"id":…}` but we correctly read message-level `modelID` (verified 257 top-level + 43 object occurrences in live `message.data`).

### 1.5 Standards

- **MCP `2026-07-28` is the new STABLE spec revision** (was 2025-11-25). Changelog highlights: elicitation, tasks, sampling, `structuredContent`, `resources/subscribe`. Idea 19 (`agenttrace mcp`) should target 2026-07-28; read-only analytics tools are unaffected by the new optional capabilities, so this is an alignment note, not a redesign.
- **OTel GenAI semconv moved to a dedicated repo**: `open-telemetry/semantic-conventions-genai` (active 2026-09-02; spans/metrics/events for GenAI clients + MCP + provider-specific conventions; Weaver-managed; Python reference + compliance matrix; **no stable release tag yet**). Idea 2 should track *that* repo, not the old `docs/gen-ai` path (which is now a tombstone page).

### 1.6 Pricing catalogs

- **LiteLLM live** (fetched): **3,518 keys / 2,678 chat models** (prior census 3,517). Tier explosion since pass 3: `tiered_pricing` now on **51 models** (was ~3), plus new field families `*_above_272k_tokens_{priority,flex}`, `*_above_1hr_above_200k_tokens`, `input_cost_per_token_batches`, `reasoning_effort_levels`, `supports_{xhigh,minimal,none}_reasoning_effort`, `default_reasoning_effort`, and priority/flex variants of *every* cache field. Idea 15's tier matrix grew ~5× in surface area; the "one price per token kind" model is now untenable for frontier providers.
- **models.dev live**: **212 providers / 7,492 models / 7,056 with cost** `{input, output, cache_read, cache_write}`, plus `reasoning_options`, `limit`, `release_date`, `knowledge`. Independent, freely fetchable, includes claude-fable-5-1. Idea 4's second-source role validated by ccusage adoption (§1.2).

### 1.7 Other upstreams

- **codex** `rust-v0.153.0-alpha.6` (2026-09-02); 0.152.x adds rate-limit banners with usage/credit checks. Local rollout census (64 files): types are still `response_item / event_msg / session_meta / turn_context / world_state` — **no format drift** observed. No action.
- **gemini-cli** v0.58.0 (2026-09-01) / v0.59.0-preview — no usage-format changes in release notes. R4-25 (`thoughtsTokenCount`) unchanged.
- **microsoft/project-telescope** last release v0.15.1 (2026-04-29) — stalled ~4 months. Competitive pressure from that quarter is static.

---

## 2. New candidates

### 33. Per-turn model attribution: stop freezing the session model on the first assistant message

**Description.** `parse_claude_code_jsonl` sets `model` only while it is `"unknown"` (parser.rs:2161–2164); every later assistant turn is priced and labeled with the *first* model. Real sessions are mixed-model: `/model` switches, advisor-model turns (ccusage shipped a fix specifically to "Count advisor model usage", v20.0.17), and sidechain rows — `isSidechain` appears **nowhere** in agenttrace-core, so subagent rows are folded into the parent session under the parent's model. Attach each assistant message's `model` to its own `Event.model_used` (field already exists), aggregate a per-session model mix, and let pricing/waste use the per-turn model. Report a "mixed-model session" disclosure when >1 model is present.

**Basis.** `direct:` parser.rs:2161–2164; `isSidechain` grep-clean across core; local transcripts carry `isSidechain` top-level keys and per-message `model`. `external:` ccusage v20.0.17 advisor fix; CC PreModelSwitch/PostModelSwitch hooks (2.1.24x) making mid-session switches first-class.

**Relationship.** Complements idea 14 (which *separates* sidechain work) and R4-24 (which labels *rate provenance*); this candidate fixes the *model dimension* of the same mispricing. Cheap to land incrementally before 14.

**Confidence:** high · **Complexity:** M · **Payoff:** cost accuracy + honesty of every cost-derived metric.

### 34. Placeholder-title gate for provider-recorded session titles

**Description.** OpenCode's `title` is populated but here is `"New session - <ISO timestamp>"` for 227/227 sessions; `sqlite_sessions.rs:672–676` uses it verbatim, so the TUI/lists show identical junk prefixes. Treat titles matching `^New session - ` (and empty) as absent and fall back to message-derived naming (idea 18's lane), optionally stamping `provider:placeholder` in naming provenance.

**Basis.** `direct:` sqlite_sessions.rs:672; live DB census §1.4. **Confidence:** high (bug-shaped today) · **Complexity:** S.

### 35. Read Claude Code's `modelPricing` managed setting as a local price authority

**Description.** Since 2.1.243, orgs can pin contracted per-model rates and a discount multiplier that Claude Code itself uses for `/cost`. Parse `~/.claude/settings.json` / managed-settings `modelPricing` when present and prefer it over catalog rates for the affected models, surfacing `source: org-contracted` in cost provenance (extends R4-24's labeling; supersedes list price exactly as upstream does).

**Basis.** `external:` CC 2.1.243 changelog line (§1.1). `direct:` agenttrace's pricing resolver already has an override chain (AGENTTRACE_PRICING_FILE) to slot this in front of the catalog. **Confidence:** high — full schema now documented (addendum §7) · **Complexity:** S–M · **Note:** the key is **managed-scope only**; it must be read from `managed-settings.json` / MDM policy paths, *not* `~/.claude/settings.json` (upstream ignores it there).

### 36. Incremental cache sync from OpenCode's event log (`seq` watermark)

**Description.** The DB carries an append-only `event`/`event_sequence` change feed keyed `(aggregate_id, seq, type)` — 27,769 rows locally. The session cache currently re-derives sessions wholesale; store `max(seq)` per cache generation and refresh only aggregates with newer events. Falls back to full scan when the tables are absent (older DBs).

**Basis.** `direct:` live census §1.4; session cache design (session_cache.rs). **Confidence:** medium (semantics of every event type need verification against upstream; single-source win only) · **Complexity:** M.

---

## 3. Strengthening / refinement for existing ideas (no new candidate)

| Idea | New evidence (2026-09-02) | Refinement |
|---|---|---|
| 2 (OTel bridge) | GenAI conventions moved to dedicated repo, no stable tag yet | Track `semantic-conventions-genai`; don't hard-code field names until a tag |
| 4 (models.dev) | 212 providers / 7,492 models / 7,056 costed; ccusage adopted it | Upgrade priority; it is now the ecosystem's default second source |
| 8 (trust opencode totals) | parent_id on 43% of live sessions; summary_* 0/227 non-zero | Add placeholder-title gate (34) and "summary columns may be empty" to acceptance |
| 9 (statusline) | ccusage ships `statusline` Beta; local machine runs a Node HUD with a shell cache wrapper for render-latency reasons | Perf budget must be shell-fast; a Rust single binary is a genuine edge |
| 11 / 5 (drift tracker / canary) | repo move + new migration since our snapshot | Add migration-dir watch for anomalyco/opencode |
| 14 (sidechain attribution) | `isSidechain` unparsed today; CC background subagents/teammates expanding | Land 33 first (model dimension), then split attribution |
| 15 (tier-aware pricing) | LiteLLM tiered_pricing 3→51; above_272k priority/flex; batches fields | Matrix is ~5× bigger; consider deferring full matrix for 4-5 flagship providers |
| 17 (reconcile stats-cache) | CC `/cost` now prints cache hit-ratio/warm-cold itself | Reconciliation surface growing; keep parity fields aligned |
| 18 (prompt-history index) | `~/.claude/history.jsonl` verified live on this machine | Also adopt provider titles *only after* 34's placeholder gate |
| 19 (MCP tools) | MCP 2026-07-28 stable | Target new revision; no tool-model change needed |
| 24 (R4 provenance) | 35 builds directly on it | Sequence 35 after R4-24 |
| 28 (todo/summary outcomes) | opencode `todo` table exists (6 rows local) | Cross-source outcome evidence; low data volume here |
| 32 (Hermes delegation graph) | opencode parent_id 43% live; ccusage per-agent JSON | Delegation graph is a cross-source pattern, not Hermes-only |
| Preserved-history lane (F4/F6/F7) | CC deletes transcripts after 30d by default (`desktopSessionCleanupPeriodDays`) | Durability claim gets stronger; cite upstream retention policy in docs |

---

## 4. Competitive read (positioning, not a feature)

ccusage (18k★) now covers agenttrace's full source list including Hermes and pi, ships a statusline, and parallelized its loaders; better-ccusage covers pi/omp too. Neither does loop analytics, tool latency, health scoring, governance/SARIF, diagnostics, or TUI drill-down. **The defensible center is diagnosis, not accounting** — keep cost surfaces at parity (33/35 are parity+accuracy) and invest depth in the lanes they cannot copy from usage JSONL alone.

## 5. Rejected this pass (with reasons)

- **opencode `summary_*` delivery evidence as a standalone candidate** — covered by idea 8; and 0/227 populated locally, so no evidence of real data yet.
- **Grok Build CLI parser** — already on the coverage list with ccusage's adapter as prior evidence; no local corpus to fixture against.
- **CC background-session / teammates parsing** — changelog-confirmed but zero local samples and no published format; premature until a canary corpus exists (idea 5 would catch it).
- **`/cost` prompt-cache / Loops parity features** — first-party UI pressure, but agenttrace already derives both; no new input.
- **codex / gemini-cli format work** — no drift observed in live rollouts / release notes.
- **Full opencode event-table analytics** — the `seq` watermark (36) captures the value cheaply; parsing all event types is speculative.

## 6. Ranking

| # | Candidate | Confidence | Complexity | Why now |
|---|---|---|---|---|
| 33 | Per-turn model attribution | high | M | Misprices real sessions today; ccusage already fixed their equivalent |
| 34 | Placeholder-title gate | high | S | Bug-shaped on this machine right now (227/227) |
| 35 | `modelPricing` setting ingestion | med-high | S–M | Org-contracted rates are now authoritative upstream |
| 36 | Event-log incremental sync | medium | M | 27k-row change feed unused; cache refresh is the TUI's coldest path |

## 7. Addendum — attempt 415004a5, 2026-09-02: `modelPricing` schema confirmed (candidate 35 → high confidence)

Fetched `docs.claude.com/en/docs/claude-code/costs` + `en/docs/claude-code/settings-reference#modelpricing` (5.0 MB page, section extracted). Facts that change the candidate's design:

```json
{
  "modelPricing": {
    "multiplier": 0.85,
    "overrides": {
      "claude-sonnet-4-6": {
        "input": 2.4, "output": 12, "cacheRead": 0.24, "cacheWrite": 3
      }
    }
  }
}
```

- **Shape:** object with optional `multiplier` (number >0 and ≤1, scales *every* cost) and optional `overrides` (map of model ID → rate object; fields `input`/`output`/`cacheRead`/`cacheWrite`, **USD per million tokens**, all four required; `cacheWrite` covers both 5-min and 1-hr cache writes). Invalid rows/multiplier are **dropped individually**, the rest kept. agenttrace's internal rates are per-token — a ×10⁻⁶ conversion is needed at ingestion.
- **Scope is managed-only:** server-managed settings, MDM policy, `managed-settings.json`, or policy helper. Ignored in user/project/local settings, `--settings`, and HKCU. So candidate 35 must probe the **managed-settings paths** (e.g. `/etc/claude-code/managed-settings.json`, macOS `/Library/Application Support/ClaudeCode/managed-settings.json`), not just `~/.claude/settings.json` — reading the user file alone would silently miss every enterprise deployment. Requires CC **v2.1.242+** (one patch earlier than the changelog's 2.1.243 rollout note).
- **Applies to** `/usage`, status line, Agent SDK `total_cost_usd`, `--max-budget-usd`, **and the OpenTelemetry cost metric and events** — i.e. upstream now emits an OTel cost signal that respects contracted rates. This independently strengthens **idea 2** (OTel bridge): Claude Code is a live GenAI-OTel producer agenttrace could ingest instead of only export.
- Timing nuance for reconciliation (idea 17): with *server*-managed settings, sessions report list price until the session's settings fetch confirms the table — a session-prefix at list price is expected, not drift.

---

**Artifacts:** this file. **Live fetches:** anthropics/claude-code CHANGELOG (6,179 lines, v2.1.258); ccusage/ccusage releases+README; anomalyco/opencode releases+migration+live DB; openai/codex releases; google-gemini/gemini-cli releases; modelcontextprotocol.io 2026-07-28 spec+changelog; open-telemetry/semantic-conventions-genai; LiteLLM `model_prices_and_context_window.json`; models.dev `api.json`; microsoft/project-telescope releases; local `~/.claude/projects`, `~/.claude/history.jsonl`, `~/.codex/sessions`, `~/.local/share/opencode/opencode.db`.

# Extensions research — pass 9 (statusline telemetry surface, tiered-pricing drift, MCP stateless spec → roadmap candidates)

**Date:** 2026-09-14 · **Run:** 125bf93302aa4e308cb0739b67f16f33 · **Phase:** research (attempt b09f497d6a864cb69eae4025489c6f78)
**Method:** in-thread. The installed `ce-*` router exists only under opencode's config tree in this environment, not as loadable pi skills, so this pass follows the ideation doc's substitution rule (disclosed; same precedent as research passes 4–7). Gather live web evidence → dedupe against ideas 1–32 and candidates 33–52 → ground every claim against the working tree (HEAD df3b621) → rank. Upstream inputs re-checked 2026-09-13/14: Claude Code CHANGELOG (raw), codex/gemini-cli/opencode releases APIs, LiteLLM catalog, models.dev `api.json`, OTel semconv repos, MCP spec, crates.io, OSV, GitHub search. Internal-need evidence comes from the pass-11 adversarial assessment (same run, prior phase). Capability numbering continues at **53**.

**Scope of this pass:** (1) a newly documented machine-readable telemetry surface in Claude Code (statusline JSON) that fills agenttrace's two biggest data gaps; (2) pricing-catalog tier drift after 12 more days; (3) the Codex 0.153.0 release, which shipped the pass-7 compression drift *and* new usage records; (4) two standards events (GenAI semconv moved to its own repo; MCP went stateless); (5) an 11-day ecosystem census including the first direct competitor with Hermes+Antigravity coverage; (6) pass-11 assessment findings converted into roadmap candidates.

---

## 1. Live evidence log (fetched or observed 2026-09-13/14, this run)

### 1.1 Upstream heads

| Source | Last known (pass 7/8, ≤2026-09-03) | This pass | Movement |
|---|---|---|---|
| anthropics/claude-code | 2.1.258 | **2.1.270** (2026-09-12) | 12 releases; see §1.2 |
| openai/codex | rust-v0.152.1 | **rust-v0.154.0** (2026-09-09) | 0.153.0 format-relevant; §1.3 |
| google-gemini/gemini-cli | v0.58.0 | **v0.59.0** (2026-09-08) | security/trust fixes only (SSRF in MCP OAuth discovery, fail-closed workspace trust); no session-format change |
| anomalyco/opencode | v1.18.26 | **v1.18.30** (2026-09-09) | provider/model-ID fixes only |
| ccusage/ccusage | v20.0.20 | **unchanged** | |
| microsoft/project-telescope | v0.15.1 (2026-04-29) | unchanged (stalled ~4.5 months) | |

### 1.2 Claude Code 2.1.259–2.1.270 — a new first-party telemetry surface (this pass's biggest find)

The **statusline input JSON** is now a documented, versioned, machine-readable telemetry stream (`https://code.claude.com/docs/en/statusline`, page `dateModified 2026-09-09`; fetched this run). Fields added at **v2.1.251+** include exactly the two things agenttrace's roadmap says it cannot see today:

- **Subscription limit pressure** (roadmap candidate 3's data gap): `rate_limits.five_hour.used_percentage`, `rate_limits.seven_day.used_percentage`, `rate_limits.*.resets_at` (Unix epoch), plus a spend-limit percentage that "runs from 0 to 100, or above 100 once you exceed the limit".
- **Authoritative prompt-cache analytics** (candidate 10 computes these from deltas; upstream now measures them): a `prompt_cache` block — verbatim from the fetched schema — `warm`, `caching_observed`, `ttl` ("1h"), `expires_at`, `requests`, `misses`, `expected_rebuilds`, `hit_ratio`, `cache_write_tokens`, `miss_recache_tokens`, `last_miss_at`, `last_miss_cause: {causes: ["tools_changed"], tools_added, tools_removed}`, `miss_causes: {tools_changed: 2}`, `recache_tokens_if_cold`. Absent until the first API response; documented as debounced at 300 ms.
- Also: `exceeds_200k_tokens`, `context_window.used_percentage`, `usage` token counts, `effort.level`, `thinking.enabled`, `fast_mode`, `workspace.repo` (v2.1.260+), `session_id`/`session_name`/`prompt_id`.
- **`cost.total_cost_usd` is "computed client-side at list price unless a `modelPricing` table is in effect"** — the managed-settings `modelPricing` table that candidate 17/33 target is now a documented public mechanism, and the changelog adds the distribution path: "with `pricing:` set in `gateway.yaml`, signed-in Claude Code clients receive the same rates through managed settings, so `/cost` and telemetry match the spend meter" (2.1.266–268 region).

Working-tree check: agenttrace has **no statusline surface at all** (`grep -rn "statusline\|rate_limit" crates/` — only unrelated TUI "status lines"), and the Claude parser (parser.rs:1266, 1566–1577) reads cache *token counts* but never hit-ratio/miss-cause analytics. The stream is push-based (Claude Code invokes the command), so ingesting it requires being the statusline command — see candidate 53.

Other 2.1.259–2.1.270 items that touch agenttrace's lanes: `claude plugin eval` (scored JSON+HTML plugin eval reports, 2.1.269); `/skill-doctor` ("show which loaded skills go unused and what they cost in context" — first-party competition for agenttrace's context-trends panel); "a likely cause for prompt-cache misses … to `/cost` and the status line's `prompt_cache` field"; OTel expansion (`OTEL_METRICS_INCLUDE_REPOSITORY` tagging metrics/events with `vcs.*` attributes; cloud sessions exporting OTLP directly to a collector; `user.email`/`user.groups` telemetry attributes); auto-compact "shortly before the 1M-token limit" for Opus/Fable (tiered-pricing relevance, §1.4); malformed session IDs now resumed "under a fresh session ID" (dedup identity edge).

### 1.3 Codex 0.153.0 (2026-09-03) — the pass-7 drift event shipped, plus new usage records

From the fetched release body and changelog:

- **"Rollout compression includes shared histories, `codex exec resume` handles compressed rollouts when selecting by working directory, and thread forks work with symlinked session roots" (#42039, #42135).** The zstd-compressed rollout files candidate 44 targets are now a stable-release reality, and compression was *extended* to shared lineages the same day pass 7 filed the candidate.
- **#41912 "Persist response token usage in rollout history"** — rollout files now carry per-response usage records. agenttrace's Codex parser reads usage only from `event_msg`/`token_count` payloads (`codex_token_count_usage`, parser.rs:2131, 2244); whether the new persisted records land in an arm the parser already reads is unverified without a 0.153+ corpus — exactly the format-canary case (candidate 5).
- #41944 "Emit turn cost telemetry for ChatGPT sessions"; #42378 "Route rollout reads through the canonical JSON decoder"; #43494 "Limit archive rollout reads to requested threads" (archived rollouts — already discovered via `.codex/archived_sessions`, discovery.rs:72).
- **Working-tree gap found while grounding #42135:** discovery walks with `fs::read_dir` and tests `entry.file_type().is_dir()` (discovery.rs:379–391), which is false for symlinks — a **symlinked session root is skipped entirely**. Codex now officially supports symlinked session roots, so those sessions silently vanish from reports. Filed as a finding and folded into candidate 54.

### 1.4 Pricing-catalog drift — 12 days, +405 keys, tiering went mainstream

LiteLLM `model_prices_and_context_window.json` fetched this run: **3,923 keys** (pass 7: 3,518 — **+11.5% in 12 days**). Field census (models carrying each field):

| Family | Pass 7 | This pass |
|---|---|---|
| `cache_creation_input_token_cost_above_1hr` | 134 | **147** |
| `input_cost_per_token_above_200k_tokens` (tiered **base** input) | not present | **101** |
| `input_cost_per_token_above_272k_tokens` | — | **81** |
| `input_cost_per_token_priority` (service tier) | (n/a) | **116** |
| `output_cost_per_token_priority` / `cache_read_input_token_cost_priority` | — | **114 / 112** |
| `cache_read_input_token_cost_above_200k/272k_tokens` | 84 / 56 | **95 / 73** |
| `output_cost_per_reasoning_token` | 69 | **72** |
| `*_flex` variants (input/output/cache_read) | 4 | **41–45 each** |

Tiering is no longer a cache-only curiosity: **plain input/output prices are context-tiered on ~100 models and service-tier (priority/flex) on ~115**, which the 4-field `Price` struct (pricing.rs:38-44) and the vendored snapshot generator both discard. Two independent signals corroborate: **models.dev** `api.json` (fetched: **213 providers / 7,784 models / 7,344 costed**; pass 7: 212/7,486/7,051) now models `tiers` on **453** models, `context_over_200k` cost on **397**, `reasoning`-priced on **151**. And Claude Code now compacts "shortly before the 1M-token limit" and exposes `exceeds_200k_tokens` — >200k contexts are routine, so the `above_200k` tiers fire on normal sessions. Candidate 43's scope must widen accordingly (candidate 56).

Snapshot staleness (candidate 51): the vendored snapshot is dated **2026-09-02** (pricing.rs:16, pinned by test) — 12 days old against a catalog that added 405 keys since.

### 1.5 Standards — two events

- **GenAI semantic conventions moved to their own repository**: `open-telemetry/semantic-conventions-genai` (created 2026-05-05; last push 2026-09-10; fetched). README: spans, metrics, and events for GenAI clients **and MCP**, provider-specific conventions (OpenAI/Anthropic/Bedrock/Azure), plus **reference implementations** (`reference/`, Python compliance matrix). Still **zero tags/releases** — so candidate 2's "don't hard-code field names until a tag" stance holds, but the stable home + reference impls make an experimental exporter concrete (candidate 55).
- **MCP spec revision 2026-07-28** (tags list fetched; changelog fetched from the tag): MCP went **stateless** — no `initialize` handshake, every request carries protocol version/capabilities in `_meta`, new `server/discover` RPC, all results carry `resultType`, `subscriptions/listen` replaces GET/subscribe, and `tools/list` results now **MUST** carry `CacheableResult` (`ttlMs`, `cacheScope`) with the stated rationale "to enable client-side caching and improve LLM prompt cache hit rates". Minor change #2 documents **OTel trace-context propagation (`traceparent`/`tracestate`/`baggage`) in `_meta`** — an MCP↔OTel correlation hook. Candidate 19's design target has changed shape underneath it (candidate 57).

### 1.6 Ecosystem census (GitHub search, created > 2026-09-03)

**15+ new repos in 11 days, still all live-limit widgets, none post-run diagnosis**: `vinzdg/codenotch` (macOS notch pinning usage limits for Claude Code/Cursor/Codex/Antigravity — **1,532★ in 9 days**), `asauntung/notch-agent-hud` (10★), `wallacemartinss/cc-cockpit` (GNOME tray + live rate-limit ring), `ruwiss/usage_monitor`, `JonOnTheWeb/claude-usage-bar`, `mrcrapto/toki` (Windows screen-edge notch), `tomaszboloz/FuelSwitch-AI`, `phanvanhuyha-dev/HP-AI-Usage`, plus **three statusline tools** (`k8adev/claude-code-statusline` 17★, `savinofiore/claude-usage-statusline`, `heathdutton/claude-dipstick`) — all consuming the §1.2 statusline payload. `emreay-/adjent` ("spot looping agents, expose quota gates to automation") is the closest to diagnosis.

**The first direct competitor with broader family coverage than agenttrace:** `VasiHemanth/tokentelemetry` (Python, **353★**, created 2026-04-24, active 2026-09-13; README and repo tree fetched) — local-first "token telemetry dashboard" covering **19 families including Hermes Agent (dedicated `/hermes` dashboard across 38 source platforms, reads `$HERMES_HOME`) and Antigravity "fully supported"** (backend ships `test_antigravity_cli.py` — a public second reference for candidate 52's parked SQLite/protobuf decode). Distribution precedent: **`hermes plugins install VasiHemanth/tokentelemetry-hermes-plugin`** — the Hermes plugin manager installs analytics tools from GitHub repos, de-risking candidate 37's channel. Feature frontier visible in their tree: `quotas.py`, `pricing_divergence.py` (multi-source price disagreement), `power_meter.py` + CO₂ estimates, `loops.py` (looping-agent detection). Weight class differs (Docker/web dashboard vs single static binary), but the coverage gap (Hermes platforms, Antigravity store) and the quota/CO₂ features are now competitor-validated user needs. Also new: `wangx1ao2/agentic-usage-hub` ("local-first token usage, cost dashboard **& MCP server** for AI coding agents") — direct precedent for candidate 19; `tomstagl/cctop` (btop-style live dashboard).

### 1.7 Dependencies & advisories

| Crate | Pin | Latest | Note |
|---|---|---|---|
| ureq | 2.12 | **3.4.2** (2026-09-13) | still 2 majors behind (idea 12 / candidate 23) |
| rusqlite | 0.32 (bundled SQLite 3.46) | 0.40.2 | unchanged; candidate 23's SQLite-currency argument stands |
| crossterm | 0.28 | 0.29.0 | unchanged |
| ratatui | 0.30 | 0.30.2 | current |
| clap | 4.5 | 4.6.6 | semver-compatible |

OSV.dev batch query (fetched): **zero known advisories** for the pinned `rusqlite 0.32.0`, `ureq 2.12.0`, `crossterm 0.28.1`, `clap 4.5.0`, `ratatui 0.30.0`, `serde 1.0.200` — the arrears are currency, not vulnerability.

---

## 2. Dedupe against the existing roadmap

Already tracked and *not* re-proposed: candidate 2 (OTel export/ingest — strengthened by §1.5, see 55), 3 (limit-pressure — data gap now closable via 53), 4 (second pricing source — models.dev tiers corroborate 43/56), 5/44 (format canary / Codex zstd — extended by 54), 9 (statusline *output* — 53 supersedes its narrow form), 10 (re-cache analytics — upstream now measures it; 53 ingests it), 17/33 (`modelPricing` — now publicly documented, strengthening both), 19 (`agenttrace mcp` — redesigned by 57), 23 (dependency currency — OSV clean, unchanged), 37 (skill/plugin channel — Hermes precedent de-risks), 51 (snapshot age — staleness now 12 days/405 keys), 52 (Antigravity decode — second reference found, see 59).

## 3. New capability candidates

### 53 — Statusline capture mode: `agenttrace statusline` (confidence high; serves 3, 9, 10, 17)

Become a Claude Code statusLine command: read the documented stdin JSON (§1.2), emit a one-line status, and **tee the raw payload to an append-only local JSONL** (e.g. `~/.cache/agenttrace/statusline.jsonl`) that discovery ingests as a new source keyed by `session_id`. This is the only local, private channel that carries `rate_limits.*` (subscription pressure — candidate 3's missing input), authoritative `prompt_cache` analytics (`hit_ratio`, `miss_causes`, `miss_recache_tokens`, `recache_tokens_if_cold`, `expires_at` — candidate 10's computed-by-us metrics, measured upstream), `exceeds_200k_tokens`, and `modelPricing`-aware `cost.total_cost_usd` (a reconciliation anchor for candidates 17/33/24). Three statusline tools and a dozen limit widgets appeared in 11 days consuming this payload (§1.6) — validated demand, and agenttrace would be the only one that also *persists and post-analyzes* it. Acceptance: a `statusline` subcommand that never fails the host (timeout-safe, errors to stderr only), a capture file with documented retention/bound, ingestion keyed to existing session IDs with overlap dedup, and TUI/report surfaces for limit-pressure windows (`resets_at` crossings) and per-session cache-miss causes. Evidence: fixtures recorded from a real v2.1.251+ session; a test pinning payload→report fields; the doc URL and schema quote above.

### 54 — Codex 0.153+ rollout reader hardening (extends 5/44; includes a live discovery bug)

Three concrete upgrades: (a) zstd magic sniff (`28 B5 2F FD`) with a named "compressed rollout" diagnosis instead of today's misleading "not valid UTF-8" (parser.rs:20-34 path), since compression shipped in stable 0.153.0 and now covers shared lineages (#42039); (b) **symlinked session roots**: discovery's `fs::read_dir` + `file_type().is_dir()` skips symlinks (discovery.rs:379–391) but Codex officially supports symlinked session roots (#42135) — resolve top-level known dirs through symlinks with a loop guard; (c) verify usage extraction against 0.153+'s persisted response token usage (#41912) — the parser currently reads only `event_msg`/`token_count` (parser.rs:2131, 2244). Acceptance: a zstd fixture fails with the named error; a symlinked-root fixture is discovered; a real 0.153+ rollout's token totals match `codex exec`'s own accounting or the delta is explained. Evidence: the three PR links; fixture tests.

### 55 — OTel GenAI exporter pilot against the new semconv repo (extends 2)

The conventions now live in `open-telemetry/semantic-conventions-genai` with reference implementations and an MCP chapter; no tag yet. Pilot an `--export-otel` flag emitting spans/metrics for parsed sessions using the repo's current YAML as the source of truth (generated, not hand-copied), clearly labeled experimental until a tag exists. The MCP 2026-07-28 spec's `_meta` trace-context conventions (§1.5) give a future join path between MCP tool calls and OTel spans. Acceptance: an OTLP file-export round-trip a collector accepts; field names generated from upstream YAML with a drift check; experimental disclosure in docs. Evidence: repo links, collector round-trip artifact.

### 56 — Tier- and service-tier pricing `Price` v2 (extends 43; scope widened by §1.4)

Candidate 43 scoped cache-duration/context tiers; the catalog now also tiers **base** input/output by context (101/81 models) and by service tier (priority ~115, flex ~45), and models.dev independently carries `tiers`/`context_over_200k`/`reasoning` (453/397/151). Design `Price` v2 to represent: duration-tiered cache writes, context-tiered everything, priority/flex service tiers, and reasoning-output pricing — with a fallback price when a session's tier is unknown (today's behavior) and a `pricing_confidence` disclosure of which tiers were applied. Acceptance: snapshot generator preserves the tier families; a >200k-context fixture prices with the `above_200k` tier and discloses it; totals for a tier-free corpus are bit-identical to today. Evidence: the census table; a before/after cost comparison on the operator corpus.

### 57 — `agenttrace mcp` against the stateless 2026-07-28 spec (extends 19)

The MCP surface changed materially (§1.5): stateless requests, `server/discover`, `resultType`, and mandatory `CacheableResult` (`ttlMs`/`cacheScope`) on list endpoints — a natural fit for an analytics server whose answers are cacheable between scans. Precedent: `agentic-usage-hub` ships usage analytics over MCP. Acceptance: a stateless server exposing sessions/overview/waste queries with `ttlMs` hints, specced against 2026-07-28 (with a documented fallback posture for 2025-11-25 clients via `server/discover`); an end-to-end test with an MCP client. Evidence: spec changelog link; working server artifact.

### 58 — Hermes plugin-manager channel for the skill surface (extends 37)

`hermes plugins install VasiHemanth/tokentelemetry-hermes-plugin` (fetched README) proves the Hermes plugin manager installs analytics tools straight from GitHub repos. agenttrace already ships `skills/`; publishing an installable skill/plugin through the same channel (and optionally a `/hermes`-style tab deep-linking into the TUI) reaches Hermes operators where they already live. Acceptance: `hermes plugins install <agenttrace-skill-repo>` works end to end; the skill wraps documented CLI commands only; versioning follows the CHANGELOG gate. Evidence: the working install of the competitor's plugin as the mechanism proof; agenttrace skill artifact.

### 59 — Antigravity store decode via a second public reference (extends 52; relaxes the corpus prerequisite)

Candidate 52 is parked on "prerequisite: a corpus". TokenTelemetry's `backend/test_antigravity_cli.py` plus its README's "Antigravity ✅ fully supported" is a public second reference implementation alongside agy's `internal/daemon/types.go`. Decode work can now be diffed against two independent implementations before a local corpus exists (fixtures can be validated by cross-implementation agreement). Acceptance: unchanged from candidate 52 plus "agrees with tokentelemetry's parsed totals on a shared fixture"; still feature-gated by default. Evidence: both reference links; a cross-check artifact.

## 4. Hardening candidates from the pass-11 assessment (filed for the roadmap's hardening lane)

- **A11-5/A11-1** — `--no-baseline-gate` is misregistered as a value flag (`main.rs:726`, bool at `:99-100`): leading placement silently drops following flags, live-reproduced including a misleading "`--baseline requires --overview -f json`" error; the cycle-4 review F2 that found it never reached the ROADMAP while cycle 5 fixed the same class for `--sample`. One-line fix + shim test + roadmap note.
- **A11-2** — `enforce_byte_bound` double-counts paths present in both cache maps (session_cache.rs:652-695, overlap created at :549-564) → over-eviction near the 64 MiB bound; `enforce_entry_bound` (:617-648) under-drops via duplicate slots. Deduplicate over the union of keys.
- **A11-6** — Windows `HOME`-only discovery (discovery.rs:52-54; roadmap P3-1, HIGH): live `env -u HOME` → zero sessions on a platform with shipped installers. The shared resolver acceptance already written covers it; §1.6 shows Windows widgets (`HP-AI-Usage`, `toki`) are an active demand pocket.
- **A11-3** — install.sh installs mode 0711 (mktemp 0600 + `chmod +x`, install.sh:53-54,66) vs npm's 0755; make it `chmod 0755` (and add the still-missing published-checksum verification, P5-6).
- **A11-4** — TUI Delivery worker has no error path (app.rs:1513-1521, 1536-1553); surface a send-side `Result` in the panel.
- **New this pass (from §1.3)** — symlinked known-session dirs are skipped by discovery (discovery.rs:379-391); fold into candidate 54(b).

## 5. Priority read for the next cycle

1. **Candidate 53** (statusline capture) — newly unlocked, high-demand lane, single mechanism serving three existing candidates; needs no upstream cooperation.
2. **Candidate 54** (Codex 0.153 hardening incl. the live symlink discovery bug) — protects existing coverage on the fastest-moving upstream.
3. **A11-5 + A11-1** (one-line shim fix + traceability) — cheapest user-visible correctness win, already fully diagnosed.
4. **Candidate 56** (Price v2) — biggest cost-accuracy lever; evidence now overwhelming (§1.4).
5. **Candidate 58** (Hermes plugin channel) — distribution win with a proven mechanism, small surface.

No commits, pushes, PRs, or CI runs this pass. All fetched sources are reproducible from the URLs/PR numbers cited above.

# Extensions research — pass 7 (pricing tiers, a format-drift event, and report-integrity needs → roadmap candidates)

**Date:** 2026-09-03 · **Run:** dafd34b3940e497f9f1ac234573323ad · **Phase:** research (attempt 8d44f7a1bb264fae8ef7784dc0819378)
**Method:** in-thread (no ce-* router installed in this environment; disclosed per the ideation doc's substitution rule). Gather → dedupe vs ideas 1–32 / candidates 33–42 → ground every claim → rank. Web via curl (GitHub API, raw.githubusercontent, crates.io, LiteLLM, models.dev). Internal needs evidence from the pass-8 adversarial assessment (same run, prior phase) and live runs of the release binary on the operator corpus. Numbering continues at **43**.

**Scope of this pass:** pricing-catalog field drift (quantified), one real upstream session-format event (Codex zstd rollouts), new ecosystem entrants since 2026-08-20, dependency currency (SQLite pinned exactly), upstream-head re-checks, and the pass-8 assessment findings converted into roadmap candidates.

---

## 1. Live evidence log (all fetched or observed 2026-09-03, this run)

### 1.1 Upstream heads re-checked — one real drift event

| Source | Pass-6 head (2026-09-02) | This pass (2026-09-03) | Note |
|---|---|---|---|
| anthropics/claude-code | 2.1.258 | **2.1.258 (unchanged)** | raw CHANGELOG fetch |
| ccusage/ccusage | v20.0.20 (2026-08-15) | **unchanged** | releases/latest |
| anomalyco/opencode | v1.18.26 (2026-09-01) | **unchanged** | releases/latest |
| openai/codex | rust-v0.152.1 stable / v0.153.0-alpha.6 | **unchanged** | releases list |
| google-gemini/gemini-cli | v0.59.0-preview.0 | stable v0.58.0 (2026-09-01) | no format change |
| microsoft/project-telescope | v0.15.1 (2026-04-29) | **still stalled ~4 months** | |
| cobra91/better-ccusage | v1.8.0 (2026-07-19) | **unchanged** | |
| kelviq/tare | no releases (skill channel) | **unchanged** | |

**The drift event — Codex rollout compression, merged 2026-08-28.** PR openai/codex#41357 "Support compression for shared rollout lineages" (fetched this run via the pulls API): rollout compression previously skipped paginated fork lineages because "lineage readers rely on byte offsets into the original JSONL". The PR adds "a seekable rollout reader that preserves logical JSONL offsets for **plain and zstd-compressed files**" plus the **opt-in `local_thread_store_shared_compression` feature**; "the default mode continues to leave shared lineage [uncompressed]". Consequence for agenttrace: when a user enables that flag (or when it graduates to default), `~/.codex/sessions/rollout-*.jsonl` files can be **zstd streams**, and the current loader (`crates/agenttrace-core/src/parser.rs:20-34`) sniffs only the UTF-16 BOM then requires `String::from_utf8` — a zstd file (magic `28 B5 2F FD`) would be reported as "not valid UTF-8", a misleading diagnosis, and the entire Codex source silently vanishes from reports. This is idea 5 (format canary) firing in the wild; candidate 44 instantiates it.

### 1.2 Pricing-catalog field drift — quantified (this is the pass's biggest find)

LiteLLM `model_prices_and_context_window.json` fetched this run: **3,518 keys** (key count identical to pass 6), but mining *fields* rather than keys exposes a tiered-cache family the 4-field `Price` struct (`crates/agenttrace-core/src/pricing.rs:38-44`) and the snapshot generator both discard:

| LiteLLM field | Models carrying it | Concrete value (fetched) |
|---|---|---|
| `cache_creation_input_token_cost_above_1hr` | **134** | anthropic claude-haiku-4-5 via Bedrock: 1.25e-6 → **2e-6** (1-hour cache TTL costs 2×) |
| `cache_read_input_token_cost_above_200k_tokens` | **84** | context-length-tiered cache reads |
| `cache_creation_input_token_cost_above_272k_tokens` | **28** | azure/gpt-5.6: 6.25e-6 → **1.25e-5** (2×) |
| `cache_creation_input_token_cost_above_200k_tokens` | **35** | |
| `cache_read_input_token_cost_above_272k_tokens` | **56** | |
| `cache_read_input_token_cost_above_512k_tokens` | 1 | |
| `cache_creation_input_token_cost_flex` / `_priority` | 4 | Anthropic service-tier pricing |
| `output_cost_per_reasoning_token` | **69** | separate reasoning-output price |
| `annotation_cost_per_page`, `citation_cost_per_token` | 8 / 1 | tool-priced components |

The vendored snapshot cannot currently represent any of this: `crates/agenttrace-core/src/pricing_snapshot.json` (2,459 keys) carries only `input/output/cache_creation/cache_read` costs + `mode/date/source/litellm_provider/models` — the trimming happens at generation, so even reading a fresh LiteLLM file loses the tiers.

**Why this is the top accuracy lever:** the operator corpus measured live this run (`--audit --limit 2000 --range all`) is **94% cache-read tokens by volume** — input 127,285,772 (5%), output 14,646,366 (<1%), cache_write 96,315 (<1%), cache_read **2,256,794,166 (94%)** of 2.40B total. The single largest cost channel is precisely the one whose catalog pricing just gained duration- and context-tiered variants we discard.

### 1.3 New ecosystem entrants since 2026-08-20 (GitHub repo search, live this run)

| Repo | ★ | Created | What it is |
|---|---|---|---|
| semihtalii/brink | 52 | 2026-08-29 | "Know when you're on the brink — Claude Code, Codex & Cursor usage limits on the edge of your browser" |
| ansonliam/AIUsageMonitor | 27 | 2026-08-21 | Windows widget, real-time Codex/Claude/Antigravity monitoring |
| DevDock-AI/claude-unlimited | 14 | 2026-08-25 | Claude/GPT subscription & API-key rotation inside the CLI |
| jungjoongi/claude-carousel | 8 | 2026-08-28 | several Claude accounts side by side |
| calm032019/claude-kicker | 7 | 2026-08-27 | restarts sessions stopped on a usage limit once the limit resets |
| PsSave/trayt | 7 | 2026-08-28 | Linux system-tray dashboard for Claude Code / Codex usage |
| itssarthak/claudecode-switchboard | 4 | 2026-08-25 | per-session local dashboard: status, live token burn, plan |

Read: six new repos in ~2 weeks, **all live monitoring / limit relief, none post-run diagnosis**. The limit-pressure pain (idea 3) keeps minting entrants; the diagnosis lane remains agenttrace's uncontested center.

### 1.4 Dependency currency — SQLite pinned exactly (crates.io + upstream source, this run)

| Crate | Workspace pin | Latest stable | New pinned evidence |
|---|---|---|---|
| rusqlite | 0.32 (bundled) | **0.40.2** (2026-08-08) | upstream tag `v0.40.2` `libsqlite3-sys/sqlite3/sqlite3.h` → **`SQLITE_VERSION "3.53.2"`** vs our bundled **3.46.0** (verified in-tree last phase via bindgen.rs) — seven minor SQLite releases of parser hardening behind, on a path that parses untrusted third-party DBs |
| libsqlite3-sys | 0.30 | 0.38.2 | master `Cargo.toml`: default `min_sqlite_version_3_45_3`, `rust-version = 1.88.0` (MSRV bump to plan for) |
| ureq | 2.12 | 3.4.0 (2026-08-08) | unchanged since pass 6; idea 12 stands |
| crossterm | 0.28 | 0.29.0 | unchanged; idea 23 |
| ratatui | 0.30 | 0.30.2 | current |
| clap | 4.5 | 4.6.6 | semver-compatible |

The repo's own dependency lane is still visibly in arrears: dependabot **#279** (cargo group, 7 updates) and **#278** (attest-build-provenance 4.1.1→4.2.2) remain open alongside **#282** (the maintenance-campaign PR), per the issues API this run.

### 1.5 Standards & catalogs (re-verified)

- **models.dev** `api.json`: **212 providers / 7,486 models / 7,051 costed** — within noise of pass 6 (7,492/7,056). Idea 4 unchanged.
- **OTel GenAI semconv**: **still zero tags/releases** (tags API empty this run). Idea 2's "don't hard-code field names until a tag" stance holds.
- **MCP spec changelog**: the docs path pass 6 used (`docs/specification/CHANGELOG.md`) now 404s; **spec revision not re-verified this pass** — flagging as unverified rather than assuming no change (idea 19 unaffected either way).
- LiteLLM catalog key count 3,518 (identical to pass 6); chat-mode entries 2,678; tiered 49.

### 1.6 Internal user-need evidence (pass-8 adversarial assessment, same run)

Converted to candidates below: F8-1 (silent 20-session governance sampling, 176× cost understatement), F8-2 (discarded `LoadReport.discovered`), F8-3 (cache never evicts: 10.5 MB, 761/1487 dead paths), F8-5/F8-6 (inf/NaN panic in `json_float`; divergent percentile impls), F8-7 (governance-reports.md claims schema 4 and a 24h auto-refresh that the code forbids).

**Research-loop validation:** pass-6 candidates **41 (Windows-source leniency)** and **42 (baseline regression gating)** are **shipped** at HEAD `998ade8` — UTF-16 sniffing with a named actionable error (`parser.rs:22-28` area) and the baseline exit-2 gate are live, re-verified by the pass-8 assessment. With candidate 34 (cycle-3 CU-5), that makes three research candidates that reached the tree; the loop works.

---

## 2. New candidates

### 43. Tiered cache & reasoning-token pricing (`Price` v2 + snapshot regeneration)

**Description.** Extend `Price` with the catalog fields that now exist and matter: `cache_creation_above_1hr`, cache read/write tiers above 200k/272k/512k context, `_flex`/`_priority` service tiers, and `output_cost_per_reasoning_token`; regenerate the vendored snapshot without stripping them. Costing then picks the tier from each request's observed context size (agenttrace already records per-request token totals) and splits output into standard vs reasoning where the source records reasoning volume. Where a tier applies but the session lacks the discriminating signal, fall back to the flat rate **and say so** (`cost_basis: flat|tiered|estimated` per model row) — cost provenance is also open issue #103.

**Basis.** `external:` LiteLLM catalog fields quantified in §1.2 (134 / 84 / 69 models; concrete 2× deltas). `direct:` `pricing.rs:38-44` reads exactly four cost fields; `pricing_snapshot.json` strips the rest at generation; corpus is 94% cache-read tokens (§1.2, measured this run).

**Confidence:** high (catalog field census + live corpus mix) · **Complexity:** M (pricing struct + snapshot script + tier selection + tests; no new I/O) · **Payoff:** correct costs on the dominant token channel, with disclosed uncertainty.

### 44. Codex zstd rollout support (or a named, actionable error)

**Description.** Sniff the zstd magic (`28 B5 2F FD`) in `parse_file` next to the existing UTF-16 check. Minimum viable: bail with `"codex rollout is zstd-compressed (enable with local_thread_store_shared_compression); not yet supported"` so a whole source never disappears behind a UTF-8 red herring, and surface it in `doctor`/data-health. Full version: decode with a zstd decoder behind a feature flag and keep offsets irrelevant (agenttrace reads whole files).

**Basis.** `external:` openai/codex PR #41357 merged 2026-08-28 (opt-in `local_thread_store_shared_compression`; "seekable rollout reader … for plain and zstd-compressed files"), fetched this run. `direct:` `parser.rs:20-34` handles only plain UTF-8/UTF-16; `doctor` today would show Codex files as unreadable-encoding.

**Confidence:** high (the format now exists upstream; only adoption timing is uncertain) · **Complexity:** S for the named error, M for decode · **Payoff:** no silent source loss on the next Codex default flip; idea 5's canary in action.

### 45. Governance coverage honesty: report audited/total, default unbounded for audit-class commands

**Description.** For `--audit/--recommend/--mcp-governance/--context-trends/--delivery-evidence/--compare`: make the default unbounded (or gate the sample cap behind an explicit `--sample`), and always emit `audited_sessions` / `total_sessions_discovered` / `excluded_reason` in JSON and a "(auditing 20 of 1,408 sessions)" line in text/HTML.

**Basis.** `direct:` F8-1 — measured live: `--audit --range all` → $3.9494 over 20 sessions vs `$695.4611` over 1,408 with `--limit 2000`; exit 0 both times; `main.rs:122-123` default 20, `.take()` at `main.rs:225,249`; `--overview` is unbounded so the tool contradicts itself.

**Confidence:** high · **Complexity:** S · **Payoff:** governance reports stop understating fleet cost by up to 176×.

### 46. Truthful discovery accounting (`discovered` from the loader, not a re-derivation)

**Description.** Thread `load_report.discovered` into `data_health` instead of recomputing `sessions.len() + skipped` (`main.rs:337-341`), so ranged runs report `discovered=1407, parsed=70, out_of_range=1337` rather than `70/70`.

**Basis.** `direct:` F8-2 — live: `--overview --range 1d` reports discovered 70 while 1,407 exist; parse failures in out-of-range files are invisible.

**Confidence:** high · **Complexity:** S · **Payoff:** "Parse coverage N/M" becomes trustworthy for every ranged run.

### 47. Bounded, evicting session cache

**Description.** During `load_session_cache`, prune entries whose path no longer exists (the data is already in hand), then cap entries LRU-style (or by total serialized bytes). Measure `sessions.json` size before/after on the operator corpus as the acceptance number.

**Basis.** `direct:` F8-3 — live: 10,518,119 B, 1,487 entries, 761 (51%) dead paths, full rewrite per dirty save, ~0.55s `doctor` startup; `session_cache.rs:558-616` deletes only on visit-time staleness.

**Confidence:** high · **Complexity:** S–M · **Payoff:** startup and save cost stop growing without bound as session dirs rotate.

### 48. Docs-contract check: pin schema numbers and behavioral claims

**Description.** Extend `scripts/ci/check-docs-commands.sh` (or add a sibling) to assert invariants docs claim: grep `governance-reports.md` for the current `SNAPSHOT_SCHEMA_VERSION` value read from `session_cache.rs:13`, assert the phrase count of "refresh" claims matches the offline-by-default contract in `PRIVACY.md`, and assert README documents flags-before-positional.

**Basis.** `direct:` F8-7 — doc says snapshot schema 4 (code is 6) and "a cache older than 24 hours is refreshed automatically" (the code is network-free outside `--update-pricing`, pinned by test); F8-8 — the flags-after-positional trap is still undocumented one pass later.

**Confidence:** high · **Complexity:** S · **Payoff:** docs stop rotting silently; the two freshest doc lies become structurally impossible.

### 49. Float hygiene in the costing path (`json_float` + percentile unification)

**Description.** Make `json_float` total (render non-finite as `null` with a `data_health` flag instead of `.expect`), add a finiteness check to `convert_litellm`'s ×1e6 multiplication, and collapse the two `percentile` implementations (`lib.rs:1309` vs `reports.rs:1777`) into one used everywhere.

**Basis.** `direct:` F8-5/F8-6 — standalone repro confirmed the percentile divergence (20 vs 19 at p=0.95 on the same input); poisoned pricing reaching `json_float` panics.

**Confidence:** high · **Complexity:** S · **Payoff:** no report path can panic on adversarial catalog data; one percentile definition everywhere.

---

## 3. Refinements for existing ideas (no new candidate)

- **Idea 23 / 12 (dependency refresh):** now carries exact evidence — rusqlite 0.40.2 bundles **SQLite 3.53.2** vs workspace **3.46.0**; MSRV moves to 1.88.0; dependabot #278/#279 still open. Add `cargo audit` to CI while touching the lane (it was not installable in this environment — the gap itself is evidence).
- **Idea 3 (limit-pressure diagnostics):** strengthened — six new entrants since 2026-08-20 (§1.3), including brink at 52★ in days; all live-monitoring, none post-run.
- **Idea 5 (format canary):** validated by event — the Codex zstd merge is the first tracked format drift; canary tests should include one zstd-compressed rollout fixture regardless of whether candidate 44 decodes it.
- **Candidates 41/42:** **shipped** at `998ade8` (§1.6) — close them in ROADMAP.
- **Idea 19 (MCP server):** spec-revision re-verification failed this pass (changelog path 404) — carry the caveat, don't advance the idea.

## 4. Competitive read

ccusage/telescope/better-ccusage/tare: no movement. The entrant wave is uniform: live limit HUDs (brink, AIUsageMonitor, trayt, switchboard) and limit relief (claude-unlimited, carousel, kicker). Rotation tools flirt with ToS; diagnosis of *why* spend happened remains unclaimed, and none of the new entrants touch multi-source corpora or governance/CI surfaces.

## 5. Rejected this pass (with reasons)

- **Live usage HUD / limit-forecast widgets** — non-goal (live tracing while streaming); six fresh competitors confirm a crowded lane without moving our differentiation.
- **Subscription/key rotation integrations** (claude-unlimited, carousel) — security-adjacent enforcement; explicit non-goal.
- **Full Anthropic `_flex`/`_priority` service-tier modeling now** — only 4 models carry the fields; folded into candidate 43's `Price` v2 rather than a standalone lane.
- **Cost-invoice reconciliation** (`annotation_cost_per_page` etc.) — billing-grade promise is a non-goal; the fields are noted for completeness only.

## 6. Ranking

1. **43** — correct pricing on the 94% channel; biggest accuracy lever per unit effort.
2. **45** — removes a 176× understatement from the flagship governance commands.
3. **44 (minimum viable)** — cheap insurance against silent whole-source loss.
4. **47** — unbounded-growth fix with a measurable acceptance number.
5. **46** — small but makes a headline metric truthful.
6. **48 / 49** — hygiene that prevents recurrence; bundle with 46 in one docs/hardening cycle.
7. **Dependency refresh (refinement of 23/12)** — now with pinned versions; pairs with candidate 43's snapshot regeneration.

**Artifacts:** this file. **Live fetches (all 2026-09-03, this run):** anthropics/claude-code CHANGELOG (raw); GitHub API releases for ccusage/ccusage, anomalyco/opencode, openai/codex, google-gemini/gemini-cli (releases/latest), microsoft/project-telescope, cobra91/better-ccusage, kelviq/tare; openai/codex PR #41357 (pulls API); GitHub search/repositories (created >2026-08-20); crates.io for ureq/rusqlite/crossterm/ratatui/clap/libsqlite3-sys; rusqlite tag v0.40.2 `sqlite3.h` + master `libsqlite3-sys/Cargo.toml`; LiteLLM `model_prices_and_context_window.json` (key + field census); models.dev `api.json`; OTel semantic-conventions-genai tags; luoyuctl/agenttrace issues #103/#237 + open-issues list. **Internal:** pass-8 assessment (`docs/reviews/2026-09-03-adversarial-repository-assessment-pass8.md`), `pricing.rs:38-44`, `pricing_snapshot.json` field set, `parser.rs:20-34`, live `--audit`/`--overview` runs on the operator corpus (token mix §1.2).

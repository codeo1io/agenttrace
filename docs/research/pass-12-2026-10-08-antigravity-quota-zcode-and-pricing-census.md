# Research pass 12 — 2026-10-08 — antigravity quota shapes, zcode demand, and pricing-catalog census

Run `ec762a618372` (repository-maintenance `fad1cbf4` cycle 3), research attempt `9a4ad8e0`, base `92149bd` (porcelain 0 in/out). Every probe below was executed first-hand this pass via `gh api` / `git ls-remote` / `curl` / python (OSV + npm + crates.io); morning-wave probes were NOT repeated from their spools. Companion wall rows: `rm-760` (implemented this cycle — usage-truth seam), `rm-761` (this pass's R1), `rm-762` (R3 discipline), `rm-763` (R4). Raw absent-key list: spool `delegate/9a4ad8e0…-scratch/litellm-absent-keys.txt`; live LiteLLM JSON cached at `/tmp/at-research-9a4a/litellm.json`.

## 1) R1 (HIGH) — Antigravity is the rising quota source; the fork's quota lane is two payload versions behind

- codeburn `#1667` "Fix Antigravity 2.16 quota decoding in CLI and desktop" (closed/merged 2026-10-06T22:23Z, fixes `#1613`): Antigravity 2.16 returns quota groups under `response.groups`, puts remaining-fraction + reset time directly on each bucket, and names fallback model rows with `label`. Readers accept both 2.15 and 2.16 shapes; out-of-range reset → unknown-not-discard. The quota payload is a moving per-release target.
- tokscale `#1402` (open, 2026-10-05): fallback to `agy --print /usage` — a programmatic live-quota surface exists (second source for the live-quota lane family `rm-302`/`rm-457`).
- Upstream `luoyuctl/agenttrace` `#236` "[Radar] Track Gemini CLI to Antigravity CLI transition" (open, dormant since 2026-05-19). Upstream `#312` (merged) DELETED the gemini lane entirely; the fork retains it, and this cycle's assess (`984bcea3` F1) showed it is the misclassifying lane — it claims foreign single-object documents and silent-zeros them. `rm-760`'s claiming-probe rejection landed this cycle; full retirement-vs-fix rides `rm-761`'s redirect arm.
- Fork state at `92149bd`: antigravity in-tree at `parser.rs:131-134` (trajectory sidecars) and `discovery.rs:79-86`/`:910` (`~/.gemini/antigravity-cli/brain`). No local install on this host (`~/.gemini/antigravity-cli` absent, no `agy` binary) — ecosystem-sourced evidence only.
- Discipline: fixture-first dual-shape (2.15+2.16) decode with versioned fixtures; shapes must come from the codeburn patch text or a real payload, not from imagination. `@google/gemini-cli` 0.63.0 published 2026-10-06T20:58:58Z (npm time) is the trigger timestamp for the gemini-lane watch.

## 2) R2 (MEDIUM) — ZCode (Z.ai coding CLI) demand is 2-sourced in one ecosystem within 24h; fork has prices but no lane

- ccusage `#1831` (open 2026-10-07T08:16Z): GLM-5.3 / GLM-5.3-Flash price $0.00 on ZCode built-in Z.ai plans although rates exist in embedded data; provider IDs `account:zai-individual-coding-plan`, `account:zai-start-plan`, `builtin:zai-*`.
- ccusage `#1832` (open 2026-10-07T08:35Z): fix = prefix-match the `account:zai-` / `builtin:zai-` families in `is_zai_provider`; opaque provider UUIDs keep zero-cost + missing-pricing warning.
- Fork: zero zcode/zai lane (grep across crates at `92149bd`: only a glm proxy-catalog comment `parser.rs:3106` and a glm-5.2 test; pricing carries the glm-5 family). The bundle ALREADY carries glm-5.3 pricing keys (`pricing_snapshot.json`: `aihubmix/glm-5.3`, `aihubmix/glm-5.3-flash`, `aihubmix/coding-glm-5.3`, `bedrock/*/zai.glm-5`) — a ZCode adapter would price out-of-the-box modulo provider-ID plan semantics (ties into the plan-quota family `rm-457`/`rm-201`/`rm-302`).
- No local ZCode home on this host — demand-only; smallest first step is a fixture from the ccusage issue shapes.

## 3) R3 (MEDIUM) — pricing-catalog census (rm-006/rm-164/rm-176 refresh arms)

- LiteLLM live `model_prices_and_context_window.json`: 4,486 entries (morning 4,479; +7 today) vs bundled 3,099 @2026-10-04 → 1,388 live keys absent from the bundle, 1,233 of them cost-bearing (census method: exact-key set difference, cost-field presence filter).
- Tier-suffix FIELD census (live): `_priority` 937 (flat), `_batches` 1,200 (flat), `_above_1hr` 216 (flat), `_flex` 474 (GREW from ~370 this morning). Still ZERO tier-suffixed model KEYS beyond `:batch` ×75 — the fork's key-space assumption holds; the flex family remains an intake-drop at pricing.rs (`rm-164`).
- `gpt-6-sol` CORRECTION to the morning wall note: root key `gpt-6-sol` now carries 33 cost fields (full `_flex`/`_priority`/`_above_272k` families + nested `search_context_cost_per_query` objects); the `chatgpt/gpt-6-sol` ALIAS is still empty `{}`. On the next snapshot refresh the null-cost skip stops firing only if model-name normalization resolves `chatgpt/` aliases to root keys — otherwise chatgpt-plan sessions still price $0.
- Nested `search_context_cost_per_query` objects now appear on sol-tier entries (×~320 family) — object-valued cost fields are an intake-drop risk class for the refresh (serde struct flattening).
- models.dev `api.json`: 226 providers / 8,418 models — flat since morning.

## 4) R4 (LOW-MEDIUM) — codex per-ChatGPT-account attribution dimension

- ccusage `#1818` "feat(codex): break down usage by ChatGPT account" (closed/implemented 2026-10-02): multi-account codex operators get conflated totals without it.
- Fork codex lane has NO account dimension (grep `account` in parser.rs: only the `rm-436` counter comment + `rm-554` state comments; no `account_id` read anywhere in crates/). Small, fixture-first. → `rm-763`.

## 5) R5 (WATCH) — quiescence + flat negatives

- Upstream `luoyuctl/agenttrace`: tip `15ed07f` unmoved since 2026-10-06T06:49Z; tags ≤ v0.10.1; open set = `#236`/`#237`/`#103`, ALL dormant since 2026-05-19 (`#238` closed). 7th consecutive quiet check class.
- ccusage: latest release v20.0.26 (2026-09-27); v20.0.27 release PR `#1809` open. `#1826` ChatGPT-desktop VOICE usage now CLOSED/implemented (voice source class live in ccusage — matures the earlier VOICE watch). `#1822` `--last` period-unit UX closed (`rm-042` time-bucket arm). `#1823`/`#1824` copilot credits corroborate the landed `rm-485`/`rm-551` family.
- tokscale: `#1405` (pre-Sept-2026 codex rollouts without token_count count zero) still OPEN — external validation of the fork's landed `rm-554` band persists. `#1400` OpenCodex combo-usage niche watch.
- Agent CLIs: `@anthropic-ai/claude-code` 2.1.292 (local journals stale at 2.0.76 / Sep-30 — no local drift evidence; watch arm only); `@openai/codex` 0.160.1 flat; `@google/gemini-cli` 0.63.0 (see R1); kimi-code 1.0.11 flat; `@zack78/agenttrace` 0.10.1 flat; bare `agenttrace` npm still unclaimed (404).
- Dependencies: OSV querybatch CLEAN for all 11 base-lock crates at exact locked versions (ureq 2.12.1, rusqlite 0.32.1, clap 4.6.2, crossterm 0.28.1, ratatui 0.30.2, lru 0.18.5, serde_json 1.0.150, chrono 0.4.45, serde 1.0.229, thiserror 2.0.21, anyhow 1.0.104) — independent of the morning's cargo-deny pass. crates.io maxima flat since morning (clap 4.6.7, crossterm 0.29.0, ratatui 0.30.2, rusqlite 0.40.2, ureq 3.4.2, lru 0.18.5); NOTE the ureq-3 + rusqlite lifts already landed canonically POST-BASE (`d8c27c1e`) — do not re-mint.
- Standards: `open-telemetry/semantic-conventions-genai` still ZERO tags (ls-remote) — `rm-229` first-tag / `rm-493` snapshot triggers unmet.

## Negative / dedupe pins

- Fork pricing overrides: ccusage `#1816` (pi subcommand silently ignoring pricingOverrides, fixed 10-01) has NO fork analog — the fork's override machinery is centralized at pricing.rs:20-46 (`PRICING_OVERRIDE_MODELS`/`STATUS`) and applied at the pricing layer before any per-source attribution; no per-source bypass surface found.
- No local antigravity or ZCode installs on this host — R1/R2 are ecosystem-demand candidates, not operator-demand ones; both stay fixture-first.
- LiteLLM tier census method pinned (suffix-field regex on values; key-suffix split on `:`) — repeat it verbatim on the next refresh to keep numbers comparable.

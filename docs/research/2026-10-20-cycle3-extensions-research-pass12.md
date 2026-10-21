# Research 12: cycle-3 extension research — upstream drift, tool matrix, pricing basis, MCP spec (2026-10-20)

Status: complete (research attempt de2ef3bff96e45d1a75add9df10477fc, run 1c12a0bc32154081bce4cf1d0a52ba96 cycle 3, wall base de3168de). Raw payload mirror: delegate spool `de2ef3bff96e45d1a75add9df10477fc-scratch/` + `/tmp/at-research-de2ef3b1/`. This document is the durable in-repo digest of that dossier (compounded at f14d599b).

## 1. Upstream drift (luoyuctl/agenttrace)

`git fetch upstream` at de3168de: exactly **6 commits uncarried** (`HEAD..upstream/master`), fork 441 ahead.

| upstream | subject | disposition this cycle |
| --- | --- | --- |
| 15ed07f2 | post-#312 follow-up | already in ancestry (predates merge point) |
| b7612692 | #317 install.ps1 env-default pinning (`AGENTTRACE_VERSION` / `AGENTTRACE_INSTALL_DIR`; upstream also carries `AGENTTRACE_NO_MODIFY_PATH`) | **ADOPTED in rm-550 arm 3** — landed at the 2026-10-21 review-fix round (review fd3a37ab H1): the two pinning vars as `install.ps1` parameter defaults; `AGENTTRACE_NO_MODIFY_PATH` unported, no hook (fork installers never modify PATH) |
| 828bf6cb | #314 install.ps1 PSReadLine arch-detection fallback | **ADOPTED in rm-550** |
| d7599f18 | #316 parser cache-count clamp | already carried (our `parser.rs:2230` `subtract_cached_input` + unit clamps at `:11727`) |
| 706bf586 | #315 drop install.cmd | **ADOPTED (dropped)** per the fork decision |
| 9b518ba3 | #313 root slimming | NOT adopted — fork layout is load-bearing |

Upstream v0.10.1 (2026-10-05, PR #312 "drop Gemini CLI") predates our merge point — its token-accounting overhaul is already in our ancestry. Its Gemini-CLI-removal rationale is **refuted by live npm**: `@google/gemini-cli` nightly 0.66.0-nightly.20261019 still building; our keep-both divergence (`.gemini/tmp` + antigravity lanes, `discovery.rs:157/:161-182`) stands.

## 2. Tool version matrix (npm dist-tags, live 2026-10-20)

- `@anthropic-ai/claude-code`: latest **2.1.303** (2026-10-18T21:15Z); stable tag 2.1.292. Changelog 2.1.301-303: fork subagents (`subagent_type: "fork"`) **default-on** in `-p`/SDK/desktop/VS Code; cloud-session cost+usage restart-from-zero after resume (new process); MCP servers omitting `resultType`/`ttlMs`/`cacheScope` now surfaced client-side.
- `@openai/codex`: latest **0.163.0** (2026-10-15); `0.164.0-alpha.4` train in flight.
- `@google/gemini-cli`: latest 0.64.0; **0.65.0-preview.0** published; nightly active.
- `opencode-ai`: 1.19.3 flat.
- `ccusage`: 20.0.36 (2026-10-18T17:52Z); **org moved** `ryoppippi/ccusage` → `ccusage/ccusage`.

## 3. Dependencies

crates.io max_stable vs our lock: `serde_json` 1.0.154 vs **1.0.151** (upstream dependabot PR #320 open); `sha2` 0.11.1 vs **0.10.9**; all other key crates current (`rusqlite`, `ratatui`, `ureq` 3.4.2, `ruzstd` 0.9.1, `clap`, `zstd`). Upstream dependabot #321: `upload-artifact` 7.0.2, `taiki-e/install-action` 2.87.29, `download-artifact` 8.0.2, attest-build-provenance bump; #259: `actions/checkout` 6.0.2→7.0.0. Actions bumps must ride the rm-1944 pin discipline.

## 4. Pricing basis drift

- LiteLLM `model_prices_and_context_window.json` live: **4,222** entries vs bundled 3,131 (**+1,091**; was 4,200 on 10-16).
- models.dev `api.json`: **9,061 models / 232 providers** (+70 models in 3 days).
- Monday 06:00Z pricing cron (10-19) produced **no PR** — fork PR list newest activity #27 @ 2026-10-18T05:47Z. The red/no-PR prediction held.
- **Basis inversion** (sibling cycle-5 research, run 5cc170724abd): live keep-basis 2,785 after 466 upstream retirements — the bundled snapshot's "additive growth" acceptance premise is factually broken; a retired-model ledger + reconciled-basis acceptance must precede the next snapshot refresh (rm-176 amendment).

## 5. MCP spec

`curl -sI https://modelcontextprotocol.io/specification/latest` → 307 → `/specification/2026-07-28` — still the newest revision (the `2026-10-18` string in the index HTML is the page's `dateModified`, not a revision). Our server already conforms incl. SEP-2549 result fields (`crates/agenttrace-cli/src/mcp.rs:469/:552/:553`, rm-455). **Negative register — no row owed.**

## 6. Competitive reference

ccusage auto-commits both pricing snapshots daily and isolates pricing in a leaf crate — the design reference for the rm-176 wave. ccusage 20.0.x adapters cover sources we do not (amp/droid/codebuff/goose/grok/zcode); our hermes-journals/kimi/openclaw lanes are unique to us.

## 7. User-need signal

Upstream issues #103 (provenance), #236 (Antigravity radar), #237 (Qwen radar) dormant since May/July — no new user-need signal this cycle; fork open items are self-filed roadmap trackers.

## Sources consulted

npm registry (dist-tags + publish times, live fetches); api.github.com (upstream issues/PRs/releases, fork PR list, ccusage releases); crates.io API (`/api/v1/crates/<c>` max_stable); raw.githubusercontent.com (anthropics/claude-code CHANGELOG.md @ main; LiteLLM model_prices JSON); models.dev/api.json; modelcontextprotocol.io/specification/ (index + HEAD redirect); local `git fetch upstream` + `git show` of the six drift commits; local tree greps (`install.ps1:5-18`, `parser.rs:2230/:11727`, `mcp.rs:469/:552/:553`, `Cargo.lock:1578/:1592`).

# Extensions research pass 11 — run 7e00d9cbe20a / attempt 71cc9746 (repository-maintenance e74cb714 cycle 1)

Date: 2026-10-07. Tree assessed: be24288 (clean); origin/master now da08338; upstream luoyuctl/agenttrace master = 15ed07f, **30 commits ahead of #293** (pass 10's horizon), v0.10.0 (2026-10-04) + v0.10.1 (2026-10-05) released.
NOTE: written to /tmp this phase (tree must stay clean); the implement phase should land this at docs/research/2026-10-07-research-pass11.md.

Live evidence gathered (all fetched today):
- git fetch upstream; git log cc7d7b8..upstream/master (30 commits, #296–#318)
- GitHub API: releases v0.10.0/v0.10.1 bodies; open issues #103/#236/#237 + PRs #278/#259 (dependabot)
- LiteLLM model_prices_and_context_window.json fetched live, diffed vs vendored snapshot (PRICING_SNAPSHOT_DATE=2026-10-04, 3100 rows)
- crates.io latest vs Cargo.lock (with UA)
- OTel semantic-conventions-genai tags/commits; AgentMeasure commits; codeburn repo
- origin/master (da08338) grep coverage: period flags ABSENT, WorkBuddy present, cache-clamp present, subagents present, gemini_cli still present (5 refs)

## A. Upstream v0.10.0/v0.10.1 — 30 new commits since pass 10

Merged: #296 diagnostics tool-call/timeouts split · #297–#299 rust-i18n locales (reports, governance, doctor, search, CLI) · #300 CodeRabbit assertive · #301 checksum-verified installers + PATH + `agenttrace update` + WinGet DROPPED · #302 TUI 120FPS adaptive · #303 CI split/cache/cross-OS/MSRV 1.85/actionlint · #304 stop classifying CC-with-session_id as Qwen · #305 subagent attribution · **#306 --daily/--weekly/--monthly + --tz + --blocks (5h)** · #307 usage edge cases · #311 WorkBuddy sum-every · #312 token accounting across 7 agents + Gemini CLI DROPPED (antigravity-cli still read) · #313 root slim 33→22 · #314/#317 install.ps1 fixes · #315 CMD installer drop · #318 ureq 3 migration.

### Where each theme lands on OUR wall (dedupe):

| Upstream change | Our wall owner | Status/evidence |
|---|---|---|
| #306 period reporting | **rm-042** (acceptance scope already cites #306) | Fork lacks ALL of it: `git grep` da08338 main.rs for daily/monthly/blocks/tz → only i18n-less nothing; our --budget is journal-weekly only. Upstream flags at their main.rs:195-199, blocks JSON at :1113. Port surface = flags + windows + schema v24 digest. |
| v0.10.1 "expect LOWER totals for Claude Code and Codex" | **rm-232/rm-251** (#312 reconcile design) | Release note confirms over-counting class upstream fixed; fork converged on cache-clamp (rm-600) but audit corrections for CC/Codex not yet reconciled; upstream cache schema v26 vs our 27. |
| #312 Gemini CLI drop | rm-232 rider | Fork keeps gemini_cli (5 refs in parser at be24288/da08338). Divergence decision owed at integration: keep-for-legacy-transcripts vs drop-to-shrink-merge-surface. Upstream rationale: tool discontinued; ~/.gemini/antigravity-cli sessions still read. |
| #311 WorkBuddy | **rm-450** (landed, basis disclosure) | Upstream v0.10.1 sums every usage record vs our keep-last — reconcile "owed at integration" per row text. |
| #305 subagent attribution | **rm-545** (landed c762c2b8 2026-10-06) | CONVERGENT DUPLICATE — fork and upstream solved independently; at integration reconcile by title (our Metrics fields/subagents.rs vs their #305). |
| #301 installers + update + WinGet drop | rm-045/rm-051/rm-043 | Fork: no `update` subcommand (verified --help); upstream advertises `agenttrace update` in install.sh:123; checksum-verified installers are their provenance lane. Pass 10 candidate 60's WinGet leg is now MOOT (upstream dropped WinGet). |
| #313/#314/#315/#317 housekeeping | rm-043 | Root slim 33→22; install.ps1 AGENTTRACE_VERSION + arch detect; CMD installer dropped. |
| #303 CI overhaul | rm-043 | Split jobs, cache, cross-OS tests, MSRV 1.85 (ours already 1.88), actionlint, idempotent release. |
| #297–#299 i18n | **rm-085** (--lang no-op zh, top unimplemented) | Upstream now ships en/zh locale files for reports/governance/doctor/search/CLI — full port reference exists. Fork deliberately uses text() literals (rm-545 precedent: "fork has NO locales/"). Divergence policy owed: adopt rust-i18n keys (max merge compatibility) vs keep literals + mapping doc (minimal deps). |
| #318 ureq 3 | **rm-007** (dep majors) | Locked ureq 2.12.1 vs crates.io 3.4.2 — upstream migration 15ed07f is a port reference (don't rewrite from scratch). rusqlite 0.32.1→0.40.2 still unmigrated anywhere (ours to own). rustls 0.23.45 = current; chrono 0.4.45 = current; serde_json 1.0.150→1.0.151 patch. |
| #304 qwen misclassification | landed both sides | origin/master has tests claude_export_stripped_of_session_id_is_not_qwen_code — converged. |
| #296/#300/#302/#307 | watch only | No fork action identified this pass. |

## B. New candidates (next free numbers after pass 10's 64)

### Candidate 65 — XDG empty-string env guard in user config resolution (fork-first)
- Repo seam: `crates/agenttrace-cli/src/config.rs:72-78` — `user_config_path` maps `XDG_CONFIG_HOME`/`HOME` with no `is_empty` guard → set-but-empty env yields relative `agenttrace/config.toml` resolved against CWD.
- Standards: XDG Base Directory Spec v0.8 — "If $XDG_CONFIG_HOME is either not set or empty, a default equal to $HOME/.config shall be used."
- Live PoC (this run's assess): `XDG_CONFIG_HOME= agenttrace --doctor` → "user config: agenttrace/config.toml (not found)".
- In-repo precedent: `statusline.rs:122-126` user_cache_dir guards empty for XDG_CACHE_HOME.
- Risk: silent config miss + CWD-dependent "user" layer injection (pricing overrides, history_dir, budget).
- Upstream check: `git show upstream/master:crates/agenttrace-cli/src/config.rs` — NO xdg/is_empty handling → fork-first fix + upstream-PR candidate.
- Scope: guard in user_config_path (+ HOME), path-resolution test table (config.rs tests today cover parse/layering/matrix only — the exact gap).

### Candidate 66 — statusline budget-view mechanics (from assess N3/N4)
- `crates/agenttrace-core/src/statusline.rs:798-801` vs `:828-829`: text arm reads/aggregates the ≤10 MiB journal three times per invocation (stats + unconditional json-arm captures/series + re-read). Hoist one read+series.
- `crates/agenttrace-core/src/runtime_config.rs:62-70`: first-writer-wins test vacuous in most run orders (assert_eq!(current,current); assertion gated on installed_first). Rewrite to pin the contract.
- No current wall home (rm-385 rider covers assistant.rs time_lag seam; rm-384 covers interface growth).

### Assess findings fold routing (no new numbers needed)
- N1 budget window (trailing-observed-days, $10 vs $1 PoC) → **rm-402** (parked 5h/7d-semantics lead, PoC recorded) + rm-042 evidence: upstream #306's calendar-window + --tz design is the fix reference.
- N2 (XDG) → candidate 65. N3/N4 → candidate 66.

## C. Pricing/catalog drift (rm-006 refresh, dated 2026-10-07)
Live catalog vs vendored 2026-10-04 snapshot: **+5 rows, 36 price changes**:
- NEW: azure/gpt-6.1-sol, azure/us-east/gpt-6.1-sol, azure/us-west/gpt-6.1-sol (priority processing), zai/glm-5.3, zai/glm-5.3-air.
- CHANGED (material): deepseek-v4-family cuts (deepseek-chat 2.7e-7→1.35e-7 in, 1.09e-6→5.4e-7 out), kimi-k3 (moonshot 2.7e-6→7.2e-7 in — 73% cut), kimi-k2.6 (openrouter 7.1e-7/2.84e-6), llama-3.3-70b-groq, qwen3-30b-a3b dashscope, gemini flash retirement dates/limits.
- Catalog file active 2026-10-05/06 (commits 3689dc0, 77b6c8a).
- Corroboration for in-flight sibling lane rm-610 (flex billing): ccusage #1814 OpenAI Flex already recorded on the wall banner.

## D. External yardsticks (watch updates)
- **OTel semconv-genai**: STILL zero tags as of the 2026-10-05 push (gh_run=387). rm-229 gate stays closed.
- **AgentMeasure**: v0.5.0 released 2026-10-03; 2026-10-05 commit pins iwasinnam-003 session-aggregate receipt vectors (Metrecept) — conformance targets for rm-053 harness are maturing; port vectors.
- **codeburn** (getagentseal): 2026-10-06 v2.0.5 fix/v2.0.3 — active; already wall-owned (rm-041 lane).
- Upstream open issues static: #103 (provenance — fork AHEAD via rm-054+rm-408), #236 (Antigravity radar → pass10 candidates 52/59, now note upstream v0.10.1 reads ~/.gemini/antigravity-cli as reference), #237 (Qwen export radar).

## E. Lockfile hygiene rider (rm-044)
crossterm dual-version in tree: direct dep **0.28.1** (agenttrace-tui) + transitive **0.29.0** (ratatui-crossterm) — two terminal-backend copies in one binary. Align direct dep to 0.29.

## F. crossterm/env verification commands
- `git fetch upstream && git log --oneline cc7d7b8..upstream/master`
- `curl api.github.com/repos/luoyuctl/agenttrace/releases` (v0.10.0/v0.10.1 bodies)
- `curl raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json` → /tmp/at-research-71cc/litellm-live.json, python diff vs crates/agenttrace-core/src/pricing_snapshot.json
- `curl -A UA crates.io/api/v1/crates/{rustls,ureq,rusqlite,clap,serde_json,ratatui,crossterm,chrono,toml}`
- `./target/debug/agenttrace --help` (no update subcommand; --lang en|zh flag exists main.rs:144-145)
- `git grep` da08338 coverage checks (period flags absent, WorkBuddy/gemini_cli/subagents present)

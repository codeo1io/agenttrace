# Extensions research — fourth pass, 2026-09-02

topic: agenttrace-extensions (pass 4)
focus: "upstream changes, ecosystem developments, competing approaches, user needs, dependencies, standards, feasible new capabilities"
provenance: "run 9fcc0661af474e2783a0dee7541f6ddb, phase research, attempt 31c61936dd6248cea9860d8c20eb5fd0, intent research_repository_extensions; grounded on HEAD e005952 plus the uncommitted cycle-1 tree (159/159 tests, clippy clean) and the three prior ideation passes"

A fourth pass over the same focus. Ideas 1–23 and both rejection summaries in
`docs/ideation/2026-09-02-agenttrace-extensions-ideation.md` are treated as
done; everything below is net-new evidence gathered 2026-09-02 after that doc.
Idea numbering continues at 24. Nothing below re-proposes a prior idea; where a
new result strengthens one, it is cited as strengthening evidence only.

**Harness disclosure.** No subagent-dispatch surface in this delegate session;
grounding, generation and critique ran in-thread. No `ce-*` skill library is
installed in this environment; the compound-engineering research methodology
(gather → dedupe against priors → ground every claim → rank with explicit
confidence/complexity → reject with reasons) was applied in-thread, matching
the disclosure in the prior passes. Every `external:` claim was fetched live
this session (GitHub API, raw.githubusercontent.com, ai.google.dev,
developers.googleblog.com); every `direct:` claim was re-verified against the
working tree with grep/sed or a live binary run.

## Delta grounding

### Direct, end-to-end evidence from the working tree

- **Provider-rate collision in the vendored pricing snapshot is real, live,
  and silent.** Reproduced with the debug binary on a synthetic Claude-Code
  transcript whose model is `glm-4.7` — the exact model string present in
  this machine's real `~/.claude/projects/*/*.jsonl` sessions (along with
  `glm-5.2`): 1M input + 1M output prices at **$1.90**, which is exactly
  `openrouter/z-ai/glm-4.7` ($0.4 + $1.5 per M). Z.AI's own entry
  (`zai/glm-4.7`, $0.6/$2.2 per M → $2.80) never applies. Mechanism:
  `convert_litellm` (`pricing.rs:265–289`) normalizes every catalog key with
  `normalize_model`, which strips the provider prefix
  (`pricing.rs:479–487`), collapsing all providers of a model onto the bare
  name; ties are broken by `provider_priority` (`pricing.rs:292–308`) where
  openrouter=8 beats zai=0. Second confirmation with `gpt-oss-120b`: prices at
  $0.98 (1M+1M) = `openrouter/openai/gpt-oss-120b` ($0.18/$0.80 per M); the
  snapshot contains no bare OpenAI-direct key for it at all, so OpenAI's
  first-party rate (priority 10) never even enters the comparison.
  Quantified scale: 237 model suffixes in the bundled snapshot carry
  **conflicting rates across providers** (up to 11 distinct rates for
  `gpt-oss-120b`, 8 for `DeepSeek-V3-0324`); 405 keys are bare vs 2,053
  provider-prefixed. `pricing_source` still reports
  "LiteLLM snapshot … (bundled)" with no per-session provenance, and
  `has_specific_price` (`pricing.rs:59–68`) inherits the same collapse, so
  coverage statistics look better than per-provider reality.
- **Gemini-family thinking tokens are dropped on the floor.** The Gemini
  usage parsers read only `promptTokenCount`/`candidatesTokenCount`/cache
  keys (`parser.rs:1779–1795`, chunk sites `parser.rs:3530`, `3828`); grep
  for `thoughtsTokenCount` matches nothing (the only `thought` hit,
  `parser.rs:2779`, is Anthropic thinking blocks). Google documents
  `thoughtsTokenCount` in `usageMetadata` (ai.google.dev/api/generate-content,
  fetched 2026-09-02), and Gemini thinking-model output is billed per output
  token — so thinking sessions under-report tokens and cost. The bundled
  snapshot already carries gemini-3.x rates (verified: `gemini-3-pro-preview`
  $12/M output, `gemini-3.5-flash` $9/M, …), so the pricing side is ready;
  only the token extraction lags.
- **Verified clean — Codex cumulative usage is handled correctly.** Live
  `~/.codex/sessions/**/rollout-*.jsonl` files carry
  `token_count.info.total_token_usage` (cumulative) and `last_token_usage`;
  `codex_token_count_usage` (`parser.rs:2046–2075`) computes the delta against
  the previous total, maps `cached_input_tokens`, and falls back to
  `last_token_usage`. No candidate here — recorded so the next pass does not
  re-open it.
- **Antigravity CLI is already probed.** `--doctor` lists
  `Antigravity CLI → ~/.gemini/antigravity-cli/brain` (found=0 on this
  machine), and Qwen Code at `~/.qwen/projects`. This kills any
  "add Antigravity discovery" candidate (see rejections).

### External context — fetched live 2026-09-02

**The repository's own issue tracker is the strongest user-needs source, and
it converges on cost provenance** (GitHub API, unauthenticated):

- **#103 "Preserve provider and cost provenance in session reports"** (open
  since 2026-05-04, 10 comments, `lane/radar,status/needs-human`). The
  maintainer's own ecosystem scans repeatedly surface the cross-tool
  cost-attribution category — CodeBurn ("prices calls with LiteLLM, reports
  spending by task type, model, tool, project, and provider"), TokenTracker,
  codeledger, `Dicklesworthstone/coding_agent_usage_tracker`,
  `androidZzT/cc-statistics`, junhoyeo/tokscale, and Anthropic
  claude-code#49588 (phase-level token measurement). The final comment
  (Necmttn, 2026-06-21) states the promotable follow-up precisely: *"not
  another grouping dimension yet; it is **cost provenance/confidence**."* My
  glm-4.7/gpt-oss repro above is the concrete mechanism that issue is about.
- **#236 "[Radar] Track Gemini CLI to Antigravity CLI transition"** (7
  comments): Google's blog "Transitioning Gemini CLI to Antigravity CLI"
  describes Antigravity as "a more robust, agent-first platform"; Gemini CLI
  users are being moved. Parser coverage risk for a listed source.
- **#237 "[Radar] Track Qwen Code export and dual-output session surfaces"**:
  Qwen Code `/export` (Markdown/JSONL/HTML, HTML default since 2026-05-14,
  independently saved forked-session history, session-list metadata loading
  changes) and Dual Output `--json-file` as "a canonical machine-readable
  transcript" (qwenlm.github.io docs).
- **#272 "fix(npm): publish scoped agenttrace package"** (open) — the npm
  package is `@zack78/agenttrace` (`npm/package.json`) while Homebrew and
  WinGet publish as `Luoyuctl.AgentTrace`; release-channel identity is
  split across surfaces. Dependabot PRs #259/#278/#279 sit unmerged.
- Also noted from #103's scan log: Anthropic now ships a Verified
  `session-report` plugin (explorable HTML from `~/.claude/projects` with
  tokens, cache efficiency, subagents, expensive prompts) — first-party
  encroachment on the report lane; and ccusage documents a Codex
  session-grouped report. Neither changes agenttrace's lane, but both raise
  the bar for the report formats agenttrace already ships.

**LiteLLM catalog, live** (`model_prices_and_context_window.json`,
3,517 model keys vs 2,458 chat entries in our snapshot): 86 models carry
`input_cost_per_token_above_200k_tokens` tier rates, 68 carry
`output_cost_per_reasoning_token`, 112 carry `input_cost_per_token_priority`,
41 `…_flex`, 3 dashscope models carry `tiered_pricing` arrays — all dropped
by `convert_litellm` today. These are the ready-made inputs for idea 15
(tier/service-aware pricing), recorded as strengthening evidence, not a new
idea. 456 live models are priced only via non-token fields (image/audio/
per-second) — all non-chat, correctly out of scope (see rejections).

## Ranked new ideas

### 24. Cost provenance: de-collide provider rates and label which rate priced each session

**Description.** Stop collapsing providers onto one bare rate. Keep
provider-scoped keys in the snapshot alongside the bare key; make the bare
key a deterministic, documented choice; and attach provenance to every priced
session — which provider's rate was used, how many providers offered the
model, and the min–max spread of their rates — surfaced in report JSON,
`--audit`, and `data_health` (e.g. a `rate_collisions` count). Document
`AGENTTRACE_PRICING_FILE` aliases as the user's way to pin their actual
provider.

**Axis.** Pricing and cost-data layer (extends idea 1's trust posture; is the
implementation path for issue #103).

**Basis.** `direct:` end-to-end repro above — `glm-4.7` (a model actually used
in this machine's Claude Code transcripts) priced at OpenRouter's rate
($1.90/1M+1M) instead of Z.AI's ($2.80), a 32% underestimate, chosen by
`provider_priority` (`pricing.rs:292–308`) after prefix-collapse in
`convert_litellm` (`pricing.rs:265–289`, normalize at `pricing.rs:479–487`);
237 conflicting suffixes, up to 11 rates each; no per-session rate provenance
anywhere in report output. `external:` issue #103 (10 comments) plus its
final maintainer-adjacent comment naming "cost provenance/confidence" as the
smallest promotable follow-up; CodeBurn/cc-statistics/codeledger compete
explicitly on per-provider cost attribution.

**Rationale.** The vendored snapshot made pricing deterministic but
simultaneously made it *provider-blind*: one provider's rate silently
represents all of them, and the winner is a heuristic, not a fact. This is
the exact gap the repo's own radar issue converged on, it is measurable
(237 collisions), it is user-visible today (any GLM, gpt-oss, DeepSeek or
Llama user), and the fix is mostly additive: keep the keys, add a field. It
also upgrades `has_specific_price`-based coverage stats from optimistic to
honest.

**Downsides.** Choosing the default bare rate stays a judgment (first-party
provider > lowest?); provenance fields grow the JSON contract; older
consumers of `list_pricing` see new keys; needs snapshot regeneration.

**Confidence.** 86%. **Complexity.** Medium.

### 25. Parse Gemini-family thinking tokens (`thoughtsTokenCount`) as billed output

**Description.** Fold `usageMetadata.thoughtsTokenCount` into output tokens
(with a reasoning-token breakdown alongside), add gemini-3.x fixtures, and
show reasoning share in `--audit`. Applies to the shared Gemini-format
family: Gemini CLI, Qwen Code (gemini-cli fork), and the Antigravity probe
path.

**Axis.** Parser and source coverage / cost accuracy (companion to idea 15:
that is missing *price* fields, this is missing *token* fields).

**Basis.** `direct:` the three Gemini usage extraction sites read only
input/output/cache keys (`parser.rs:1779–1795`, `3530`, `3828`); grep for
`thoughtsTokenCount`: 0 matches. `external:` Google documents the field in
`usageMetadata` (ai.google.dev/api/generate-content, fetched 2026-09-02);
thinking tokens bill at the output rate. Snapshot rates for gemini-3.x
already present (verified above), so cost is currently computed on
under-counted tokens.

**Rationale.** Silent token under-count is a correctness bug wearing a
feature request's clothes: every thinking-model session this tool reports is
low, and nobody can tell. The fix is a handful of key lookups plus fixtures,
and it hardens the same parsing family Qwen Code and Antigravity reuse —
both of which the repo's radar (#236/#237) says are the format surfaces in
motion.

**Downsides.** Changes reported token totals (baseline comparisons will
shift — needs a changelog note); qwen/antigravity variants of the field
must be verified against real fixtures; coverage differs per source.

**Confidence.** 80%. **Complexity.** Low.

### 26. Qwen Code dual-output transcripts as a first-class input surface

**Description.** Accept Qwen Code Dual Output `--json-file` transcripts (the
format Qwen itself calls "canonical machine-readable") as an explicit input —
documented `-d` support, fixtures, and a doctor line — and track the
session-list metadata loading changes flagged in radar #237 for drift.

**Axis.** Interop and integration surface (lane 1 parser coverage).

**Basis.** `external:` Qwen Code docs (qwenlm.github.io, dual-output +
weekly-update 2026-05-14) and issue #237, which notes `/export` defaults to
HTML and session-list metadata loading changed. `direct:` `--doctor` probes
`~/.qwen/projects` (found=0 on this machine — no local evidence either way).

**Rationale.** A canonical machine-readable transcript from the tool itself
is exactly the "prefer facts the provider already recorded" lane; dual-output
files are complete, deterministic, and shareable, making them ideal fixtures
even before local sessions exist.

**Downsides.** No local fixture yet (radar explicitly says so); format may
track gemini-cli chunks (in which case most parsing may already work and the
deliverable shrinks to fixtures + docs); forked-session history semantics
need verification.

**Confidence.** 62%. **Complexity.** Low.

### 27. Release-channel identity coherence and an all-channel version-sync guard

**Description.** Resolve the npm scope split (`@zack78/agenttrace` vs
Homebrew/WinGet `Luoyuctl.AgentTrace`, open issue #272), and extend this
cycle's `scripts/ci/check-plugin-version.sh` into a single guard that checks
every release surface (CHANGELOG, plugin.json, npm `package.json`, rendered
Homebrew/WinGet channels) against one version, run in CI.

**Axis.** Trust, determinism and hygiene.

**Basis.** `direct:` `npm/package.json` name `@zack78/agenttrace` with
placeholder version `0.0.0-release` vs `plugin.json` `0.7.1` vs CHANGELOG
`v0.7.1`; `scripts/release/render-channels.sh` already centralizes channel
rendering (so the guard has one source of truth to compare against); the new
CI script covers `plugin.json` only. `external:` open issue #272 and the
unmerged dependabot backlog (#259/#278/#279).

**Rationale.** Four release surfaces with three naming schemes and one
partial guard is how silent version skew ships to users; the release
machinery already exists, so the guard is a natural completion of cycle-1's
work rather than new infrastructure.

**Downsides.** Small, tactical-adjacent (it only made the cut because the
surface count is four and one guard exists); npm scope rename has ecosystem
cost (existing installs); needs a maintainer decision on the canonical name.

**Confidence.** 70%. **Complexity.** Low.

## Rejection Summary

- **Antigravity CLI parser/discovery work** — the probe already exists
  (`--doctor`: `~/.gemini/antigravity-cli/brain`); with zero local traces and
  radar #236 explicitly deferred pending fixtures, any implementation now
  would be ungrounded. Stays radar; revisit when the first real database
  lands on a contributor machine.
- **Per-provider cost *grouping* as the headline feature** — the #103 thread
  itself (Necmttn, 2026-06-21) rejects "another grouping dimension" in favor
  of provenance first; grouping without honest rates would be more confident
  wrongness. Fold into idea 24.
- **Expanding the snapshot to LiteLLM's non-token price fields** (456 models
  priced per image/audio/second/pixel, `search_context_cost_per_query`, etc.)
  — all non-chat modes; coding-agent transcripts never exercise them. The
  token-tier variants (`above_200k`, `_priority`, `_flex`,
  `output_cost_per_reasoning_token`, dashscope `tiered_pricing`) are *not*
  rejected — they are idea 15's inputs, recorded in delta grounding.
- **Bare-key coverage gap as a separate finding** — `glm-4.7`/`glm-5.2`
  missing as bare keys is the same mechanism as idea 24 (collapse + priority
  picks a winner); a "add more aliases" patch would hide, not fix, the
  provenance problem.
- **Adopting Anthropic's `session-report`-style subagent/skills breakdowns**
  — encroachment signal worth watching, but our TUI drill-down and idea 14
  (sidechain attribution) already own that ground; no new evidence of a gap
  in *our* output.
- **Depending on ccusage's Codex session-grouping recipe** — parity framing
  rejected in pass 2; unchanged.

## Next steps

- **Highest leverage:** idea 24 — it is issue #103's named promotion path
  with a live, quantified mechanism behind it, and it makes every other cost
  number more trustworthy.
- **Cheapest correctness win:** idea 25 (three key lookups + fixtures).
- **Radar follow-through:** #236/#237 via ideas 26 and the Antigravity
  fixture-on-arrival note; #272 via idea 27.
- **Sequencing note for the conductor:** idea 24 and 25 both change reported
  cost/token numbers and should land in the same release with a changelog
  line about baseline shifts; idea 24 depends on regenerating the snapshot
  (keep `PRICING_SNAPSHOT_DATE` in sync — see pass-5 assessment P5-3).

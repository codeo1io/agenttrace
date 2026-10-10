# Governance reports

AgentTrace governance reports turn local session metadata into explicit, scoped evidence. They run locally and do not upload prompts, source code, or session logs.

All cost values are estimates. Delivery evidence is a heuristic and does not prove authorship, merge status, or business value.

## Scope first

Use the same scope controls for every report:

```bash
agenttrace --overview --range 30d --project storefront --source claude_code
```

Scope controls must ride a report action (`--overview`, `--sessions`, `--audit`, …).
`--range` on its own is rejected loudly — `--range 30d` without an
action prints an error instead of silently launching the interactive
view with the filter ignored. The other scope flags are not guarded
that way yet: on the interactive view they are accepted and ignored.

Supported controls include `--range today|7d|30d|all`, `--project`, `--source`, `--model-filter`, `--query`, `--health`, `--cost`, `--anomaly`, `--sort`, `--order`, and `--limit`.

`--health` and `--cost` share ONE dialect with the TUI (rm-389 remaining
scope, landed run 24ec00eb cycle 1): an optional `>=`, `<=`, `>`, `<`, or
`=` operator followed by a finite number, where a bare number means `>=`
(e.g. `warn`, `>=80`, `80`, `>1.5`, `1.5`). Non-finite thresholds
(`>=NaN`, `<=inf`, `1e400`) are rejected loudly with a message documenting
the dialect instead of silently matching nothing or everything.
`--project` matches by case-insensitive SUBSTRING against any of the
resolved project identity, display name, and filesystem root (rm-779):
`--project storefront` keeps `/work/storefront` and `storefront-api`
alike, and an encoded Claude session directory matches its
`-work-projects-storefront` spelling. Resolution mirrors the by_project
rollup: git-root grouping when a `.git` is discoverable from the session
cwd, else the normalized cwd itself, else the encoded/unattributed
fallbacks.

By default every governance report audits **every** session in scope, and the
report discloses its coverage: `audited_sessions` / `total_sessions` /
`excluded_reason` in JSON, and a leading `(auditing N of M sessions)` line in
text, Markdown, and HTML. `--limit` caps list views only — it never filters
aggregates. To bound an expensive audit explicitly, use `--sample N`, which
audits the first N sessions of the active `--sort`/`--order` view
(newest-first by default) and names that view in the exclusion reason, in
every format.

Use JSON for automation, Markdown for PR artifacts, HTML for a self-contained visual report, or text for terminal review:

```bash
agenttrace --audit --range 30d -f json
agenttrace --recommend --range 30d -f markdown -o recommendations.md
```

## Cost audit

```bash
agenttrace --audit --range 30d -f json
```

The audit groups sessions by source and normalized model, and reports input, output, cache-write, and cache-read tokens; per-million-token rates; estimated component costs; and pricing confidence. Thinking tokens (Gemini `thoughtsTokenCount` and OpenAI-compatible `reasoning_tokens`/`thinking_tokens` aliases) are billed at the output rate: they are folded into the output count and additionally broken out as `tokens.reasoning` plus a `reasoning_share_pct` of billed output tokens per model row.

Pricing status is intentionally explicit:

- `catalog_estimate`: an exact normalized model match exists in the pricing catalog.
- `fallback_estimate`: no exact catalog match exists, so AgentTrace used its fallback rate.
- `unpriced_or_unknown`: the model name is absent or too generic to price confidently.

To map internal model names or override per-million-token rates locally, set `AGENTTRACE_PRICING_FILE`:

```json
{
  "aliases": {"provider/raw-model": "my-model"},
  "prices": {"my-model": {"input": 1, "output": 2, "cw": 0, "cr": 0}}
}
```

```bash
AGENTTRACE_PRICING_FILE=pricing-overrides.json agenttrace --audit -f json
```

Pricing data is cached locally and normal runs never touch the network: the
cached catalog is served as-is regardless of age, a cache older than 24 hours
is simply labeled `cache(stale)` in the pricing source field, and the only
refresh path is the explicit `--update-pricing` action. If a refresh fails,
the stale cache remains usable and is reported as stale.

The session cache is schema 56 and the SQLite snapshot is schema 9; the
versions move whenever the persisted session model changes (the session
cache to 21 when tool call/result pairing and retry-loop keying were
corrected, then to 22 when per-format usage accounting was corrected —
kimi_cli wire aliases and Codex compaction usage records — to 23 when
present-but-zero usage blocks began counting as disclosed measured zeros
(rm-408), to 24 when pi-journal disclosure counters, upstream
recorded-cost passthrough and per-block multi-model pricing joined the
persisted metrics (rm-436/437/438), to 25 when the workbuddy
input-basis disclosure counters joined Metrics.line_skips (rm-450), to
26 when Copilot sessions began counting session-wide credit totals
(`totalNanoAiu` shutdown reads and `usage_checkpoint` snapshots, rm-485),
to 27 when Codex custom-tools response items began counting as tool
calls, results, and reasoning (custom_tool_call/custom_tool_call_output
pairing and standalone reasoning items, rm-542), to 28 when
WorkBuddy cache counts began clamping to the cache-inclusive input (the
upstream #316 port, rm-529), and to 29 when WorkBuddy usage began
summing across records with reasoning and function_call rows
contributing their usage (upstream #311) and the shared clamp was
adopted for Copilot modelMetrics and spans (rm-600), and to 30 when the
workbuddy input-basis disclosure counters
(`workbuddy_input_basis:cache_subtracted` /
`:zeroed_suspected_mismatch`) moved from `Metrics.line_skips` to
`Metrics.disclosure_counters` — the non-loss disclosure channel — so an
assumption-disclosure no longer renders under "Dropped lines" or
degrade `data_health.confidence` (rm-538; the same channel later took
the codex structural counters `codex_ignorable_line` and
`codex_world_state` — deterministic known-non-loss shapes the rollout
format defines as ignorable or registry-only — so a fully parsed
codex corpus no longer reads `confidence: low` merely for carrying
them; genuinely dropped lines still degrade it, rm-730), and to 31 when Copilot usage
reconciliation became per-model — a checkpoint or shutdown replaces the
per-model entries it names while preserving every omitted model's
last-known values, so rotations across checkpoints and partial shutdowns
stop dropping whole models' tokens (rm-551, landed as 30 → 31 at
integration, re-basing the campaign's own 26 → 27 bump onto the ceiling) —
so cached sessions regenerate under the corrected fold, and to 32 when the
usage-accounting truthfulness batch corrected the class totals — Claude
streaming per-message folding, Qwen alias and cache-inclusive input basis,
Codex reasoning double-add and post-compaction forward counting, and
Copilot per-model shutdown tracking with the shutdown timestamp tail
(campaign numerals rm-551..rm-556 of run 7f9c6d24, rebound at
integration as rm-601..rm-603 with rm-555 folded into the landed
rm-551; schema landed as 31 → 32, re-basing the campaign's own
26 → 27 bump onto the ceiling) —
so cached sessions regenerate under the corrected totals, and to 33 when
generic-lane sessions began reporting their conversation lines' model
identity and usage instead of degrading to text estimation (rm-616, the
Event.model_used snake_case alias plus the counted generic-lane usage
fold; landed at its base as 27 → 28, re-based at integration onto the
advanced ceiling) — so cached sessions regenerate under the corrected
model identity and totals, and to 34 when occurrence-aware usage landed
— Codex post-compaction token_count windows count their fresh `last`
snapshots instead of colliding with values already counted
pre-compaction, Qwen accumulates every turn's result usage, and
pre-rm-526 v32-era entries replayed hidden torn-tail skips at confidence
"high" (the occurrence batch, rm-710/rm-711, one bump covering both
defect classes; landed at its base 1c5edd1 as 32 → 33, re-based at
integration onto the advanced ceiling) — so cached sessions regenerate
under the corrected totals, and to 35 when the agent-lane usage
truthfulness batch folded Antigravity per-generation usage blocks, named
the Copilot agent-host request classes, and summed the agent-host
per-model credit meters instead of a global max (rm-720/rm-721 of run
3ec6cec08fb9 cycle 2, landed with its review fix; landed at its base
1c5edd1 as 32 → 33, re-based at integration onto the advanced ceiling
as 34 → 35) — so cached sessions regenerate under the corrected
agent-lane totals, and to 36 when timestamp parsing was unified onto
the single lenient parser (lib.rs `parse_ts`): naive-ISO timestamp
sessions regenerate with populated tool latencies, trace-step
durations and the naive-stamp count that warm v35 entries would keep
reporting as the pre-fix empties (rm-502 of run 6aaf51aa, landed at its
base ea5c41e as 22 → 23, re-based at integration onto the advanced
ceiling as 35 → 36) — so cached sessions regenerate under the
corrected diagnostics, and to 37 when loop-cost dollar figures became
priced from the session's own model rates × the loop's token mass —
with a disclosed synthetic fallback and a serialized `cost_basis`
field on `LoopCost` — instead of pricing-independent constants
(rm-754 of run bbde21568cd4, landed at its base 1c5edd1 as 32 → 33,
re-based at integration onto the advanced ceiling as 36 → 37) — so
cached sessions regenerate with priced loop costs, and to 38 when the
disclosure-plane honesty batch moved the kimi usage-alias counters
onto the non-loss disclosure channel
(`kimi_usage_alias:<key>` renders under "Disclosed facts" instead of
degrading `data_health.confidence` as parse loss, rm-719), widened the
generic lane's `Event` intake (`model_used` lowercase alias; the
`cachedContentTokenCount` / `cacheReadInputTokens` spellings of the
cache-read count normalize onto the canonical key, canonical-wins,
rm-718), and added the `codex_rollout_no_usage_rows` absence verdict
for pre-Sept-2026 rollouts that carry no usage rows at all (rm-716 —
all three of run 5417681937ae cycle 3, landed at its base 1c5edd1 as
32 → 33, re-based at integration onto the advanced ceiling as 37 → 38)
— so cached sessions regenerate under the corrected confidence,
disclosure census, attribution and cache-read totals, and to 39 when the
report-truthfulness batch made streaming re-emissions count the message
once — Claude stream snapshots fold per message id into their final
form, so turns, tool calls, tool results, `session_end` and duration
describe the message at its last emission instead of once per streamed
block (rm-834 of run f7f81aeaf57b, minted campaign-locally as rm-693;
schema landed as 32 → 33 at the run's base, re-based at integration
onto the advanced ceiling as 38 → 39) — so cached sessions regenerate
under the corrected totals, and to 40 when sessions began carrying the
branch they ran on, captured from the claude-code lane's `gitBranch`
envelope onto the session-meta event — the field deserializes with a
default for older caches, so the bump is the regeneration trigger, not
a decode gate; the same regeneration folds the codex structural
counters onto the non-loss disclosure channel of the rm-538/rm-730
clause above, so a warm v39 entry stops serving a "Dropped lines" row
and degraded confidence for unchanged files (rm-585 + rm-730 of run
4c3ca863, landed 32 → 33 at the run's base e9e8fd9, re-based at
integration onto the advanced ceiling as 39 → 40) — so cached sessions
regenerate under the corrected attribution, and to 41 when Metrics began
persisting which token classes carry upstream-recorded cost
(`upstream_priced_*` — the rm-436 basis split) — the basis the governance
audit's recorded-cost recompute reads, so a warm v40 entry can no longer
hand the audit a recorded-cost session whose priced classes deserialize
as zero and false-trip the drift note on exactly the sessions whose
stored estimate is upstream truth (rm-520 of run ac14e52c, landed 24 → 25
at the run's base 1511547, re-based at integration onto the advanced
ceiling as 40 → 41) — so cached sessions regenerate under the corrected
audit basis, and to 42 when the journal-truth batch joined
`Metrics.wire_metadata` (codex 0.160.1 identity/lineage/quota wire,
rm-880 — minted campaign-locally as rm-776 and rebound at the commit
gate; landed at its base aa5544a as 32 → 33, re-based at the commit
gate onto the ceiling since advanced to 39 as 33 → 40 and at
integration onto the advanced ceiling as 41 → 42) and
`Metrics.model_attribution` (per-model token/cost attribution for
multi-model sessions, rm-406) beside rm-778's parse-time cwd cap and
its `cwd_truncation_disclosure` counter to the persisted metrics — a
warm cache that predates the fields would keep serving sessions whose
hidden wire and advisor split silently vanished on the cache hit — so
cached sessions regenerate under the surfaced wire and advisor
attribution, and to 43 when the hostile-value truthfulness batch made
every parser usage fold hostile-value safe — Antigravity fold
accumulators saturate instead of wrapping, negative token counts are
refused at insert with alias-rescue, and a Copilot per-model credit sum
that overflows to +inf is dropped before the report with a named
copilot_credit_nonfinite disclosure counter instead of serializing a
null credit (rm-831 + the rm-721 rider of run fabd9fb8cf73 cycle 2,
landed with its review fix; landed at its base dc65644 as 35 → 36,
re-based at integration onto the advanced ceiling as 42 → 43; the
review proved a warm v35-era cache kept serving the wrapped-zero and
null-credit shapes for unchanged files), and to 44 when Codex
custom-tool failure attribution became order-independent — outputs are
buffered by (call_id, index) and failure stamps re-applied after the
parse loop, so a warm entry can no longer serve the line-order-dependent
tool-fail verdict (the landed rm-584's ordering arm — run fb1addd5's
campaign-local rm-585; landed 27 → 28 at the run's
base 8991144, re-based at integration onto the advanced ceiling as
43 → 44; the batch's copilot MAX-fold half is retired onto the landed
rm-551 per-model fold with identical numbers, and its empty-type
disclosure half onto the landed codex_missing_type counter, so neither
rides the bump), and to 45 when the truthful-accounting batch stopped
the Codex rollout `token_usage_record` arm from folding
`reasoning_output_tokens` into billed output (it is a breakdown of
output on the Responses wire, so it now rides its own reasoning line
— the port of the landed rm-603 breakdown to the rollout arm, rm-617)
and made aider's DST-ambiguous local session starts resolve
deterministically via `earliest()` instead of degrading to an empty
session_start (rm-899; run 33b7b7b5 cycle 1 — minted 43 above its
base's ceiling 41, re-based twice at review-fix as origin landed 43
(fabd9fb8) and then 44 (fb1addd56503), landing here as 45, the lowest
rung unique above the ceiling); the SQLite snapshot to 7
when Hermes tool outcomes began deriving from message result rows, to 8
when snapshots began carrying their opencode fork-exclusion count so v7
entries cannot silently under-disclose (rm-548 — whose sqlite-lane
exclusion scope was superseded at integration of run 2023f222 by rm-791:
`session.parent_id` rows are subagent children, retained and attributed,
not fork copies to exclude; the JSON-storage-lane `parentID` exclusion
stands and still feeds the disclosed count), and to 9 when sqlite-backed
sessions began carrying their source row key and opencode children their
parent row id, making derived-history identity per-row instead of
per-(database, second) so same-second sessions in one database stop
folding onto a single history record (rm-790, run 2023f222 cycle 1 —
landed at its base 611242d1 as 7 → 8, re-based at integration onto the
advanced ceiling as 8 → 9). Older versions are discarded and
rebuilt on the next load; the migration is read-only and does not modify
source session files. Cache entries whose source file has disappeared are
pruned the next time the cache loads, and the snapshot is bounded at
20,000 entries (oldest source-file mtime drops first).

## Prioritized recommendations

```bash
agenttrace --recommend --range 30d -f json
```

Recommendations rank observed retry loops, failing tool calls, context pressure, and slow or timed-out tools. Each recommendation includes a priority, evidence, estimated impact, confidence, next action, and a validation command.

Treat recommendations as triage prompts, not automatic fixes. Review the referenced local session before changing an agent workflow.

## MCP governance

```bash
agenttrace --mcp-governance --range 30d -f json
```

This report infers MCP server names from observed tool-name prefixes, then summarizes observed sessions, calls, and failures. Most session logs do not contain an inventory of loaded MCP servers or schema-token cost, so `loaded_sessions`, coverage, and schema-token fields remain unavailable when the evidence is absent.

## Context trends

```bash
agenttrace --context-trends --range 30d -f json
```

The report aggregates per-project context pressure, cache effectiveness, repeated file reads, read/write ratios, and output-token cost. Repeated reads are file-surface occurrences, not a claim that every read was unnecessary.

## Delivery evidence

```bash
agenttrace --delivery-evidence --range 30d -f json
```

The command uses a read-only local Git heuristic. It compares commits under a resolved project root with the session time window, with a small lead and tail allowance. It also reports observed file-write, Git-write, publish, and general tool-activity categories.

Evidence levels mean, in the order the report ranks them (strongest
first):

- `strong`: one or more local Git commits overlap the session window.
- `medium`: observed Git-write or publish category.
- `weak`: observed file-write or edit category.
- `non_code`: tool activity exists without code-delivery evidence.
- `none`: no relevant evidence was observed.

The session list is sorted by that strength — `strong` before `medium`
before `weak` before `non_code` before `none`, with the session name as
the tie-break inside a level — not alphabetically by label.
A matching commit is correlation only; it does not establish who made the commit, whether it merged to `main`, or whether it produced user value.

The local `git log` probe is read-only and bounded: it is capped at ten
seconds per repository root, and a probe that overruns the cap is
treated as unavailable — that root degrades to the tool-authority
heuristic and the report says so. A repository that answers slowly
slows the report by at most that cap.

## Waste report

```bash
agenttrace --waste -f json
```

The waste report scores the newest matching session for cache
efficiency, tool-call bloat, retry-loop spend, and stuck patterns. In
JSON it carries the same verdicts the text banner does, as data (schema
`agenttrace.waste.v1`): `waste_score` (0–100), `waste_level`,
`total_wasted_cost`, `loop_waste_percent`, `loop_cost_basis`,
`session_cost`, `cache`
(`rating`, `hit_rate_percent`, `cache_read_tokens`,
`total_input_tokens`, `wasted_cost`, `suggestion`), `tool_bloat`
(`tools_per_turn`, `bloat_score`, `bloat_level`, `top_bloat[]`),
`stuck_patterns[]`, `summary`, and `top_actions[]`.

Per-tool `allocated_cost` figures share the text report's caveat: they
are a share of the session-level estimate, not measured per-tool spend.

`loop_cost_basis` is the loop-dollar honesty key (rm-857): `priced`
when the loop figure is derived from recorded token costs, `synthetic`
when it is a constant-derived estimate — the same contract the
diagnostics surface lands via `LoopCost.cost_basis`. When the basis is
`synthetic`, `summary`, `top_actions[]`, and the text views mark the
loop portion as a synthetic estimate so no constant-derived dollar ever
reads as a measured one.

The format matrix is consistent across report actions: `-f json` is
honored everywhere the format guard admits it (the waste report
included), while `-f markdown` and `-f html` are the overview and
governance surfaces — `--waste -f markdown` is rejected with the guard's
error naming those actions instead of silently rendering text.

Gate flags evaluate for the machine form: since this report became a
versioned json contract, `--fail-under-health`, `--fail-on-critical`
and `--max-tool-fail-rate` judge `--waste -f json` exactly as they
judge `--overview` and `--compare` — write-then-gate (the json is
emitted before the nonzero exit), over the filtered session view the
waste report is drawn from. The waste metrics themselves (`waste_score`,
cache rating, tool bloat) are not gate inputs and carry no gate flags;
the human text view and `--diagnostics` stay ungated by design.

## Overview appendix

`--overview` includes scope, parse/data health, cost audit, prioritized recommendations, MCP governance, context trends, and lightweight delivery evidence in JSON, Markdown, and HTML outputs:

```bash
agenttrace --overview --range 30d -f html -o agenttrace-overview.html
```

The overview also carries attribution dimensions over the same scope:

- `by_provider`: which vendor's models the spend went to. Providers
  come from the pricing-catalog row that prices each model — never
  from the model name's prefix — and models the catalog cannot resolve
  bucket explicitly under `unknown` instead of being dropped.
- `by_branch`: which branch the spend ran on. The branch comes from the
  claude-code lane's `gitBranch` envelope on the session-meta event —
  the only lane that records one today — so every codex, qwen, copilot
  and branchless claude session lands in one explicit `unknown` bucket
  alongside detached-HEAD sessions (the literal `HEAD`) instead of
  being invented or dropped. Extending the branch capture to the other
  lanes is future work as their journals grow the field (rm-585).
- `by_task_type`: a heuristic re-slice of the same parsed aggregates
  into `coding` (write-capable or test/build tool authority observed),
  `debugging` (failure-driven work: at least one failed tool call with
  a ≥25% session fail rate, or a tool-failures anomaly), and
  `planning` (everything else). Token totals ride with each bucket.
  This is correlation over tool-mix and failure evidence, not ground
  truth; the cascade is documented and deterministic so two identical
  sessions always classify identically.
- `top_cost_drivers`: the ranked top sessions by estimated cost, each
  with its share of total spend and its strict possible-driver note
  when one exists. Shares are computed over the full in-scope corpus,
  so the rendered top-3 always sums to less than or equal to 100%.

For CI gates and baseline comparison, see [CI integration](ci-integration.md).

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

The session cache is schema 24 and the SQLite snapshot is schema 7; the
versions move whenever the persisted session model changes (the session
cache to 21 when tool call/result pairing and retry-loop keying were
corrected, to 22 when per-format usage accounting was corrected —
kimi_cli wire aliases and Codex compaction usage records — and to 24 with
the upstream #312/v0.10.1 token-accounting alignment, which renumbered
past upstream's intermediate 23 in a single step so cached sessions
regenerate under the corrected totals; the SQLite snapshot to 7
when Hermes tool outcomes began deriving from message result rows). Older versions are discarded and
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

# Usage Basis and Pricing Lanes

What agenttrace counts, which rate it multiplies the count by, and how the
tool tells you when it had to choose a basis other than the exact one the
provider billed on.

## Usage basis

A session's usage comes from the wire events the agent tool recorded —
OpenAI-compatible usage rows, Anthropic message deltas, Codex `turn_context`
counters, and the claude-code/codex `token_count` repairs. Every usage row
keeps its own class (`input`/`output`/`cache_read`/`cache_write`, plus the
reasoning breakdown where the tool reports one); agenttrace never invents a
class it did not observe and never mutates the recorded numbers. Totals are
sums over those rows — gross input, separate cache columns — and every
non-default decision the pipeline makes (a clamped magnitude, a skipped
unparseable line, an alias netted against its twin, a compaction envelope
counted separately) rides a disclosure counter on the report so a total can
always be decomposed back into what the journals actually said.

## Pricing lanes: standard vs flex

Since the flex-tier integration (ROADMAP rm-927), a pricing row can carry
TWO parallel rate lanes per token class — standard rates and `*_flex` rates
(sourced from LiteLLM `*_cost_per_token_flex`; 106 flex-carrying models in
the vendored snapshot). The lane a request bills on is the service tier the
tool actually ran it at:

- **Standard lane** — the default row rates. A tier-less usage row always
  bills standard.
- **Flex lane** — applies to a turn the journal marks `service_tier: flex`
  (Codex `turn_context.thread_settings.service_tier`, threaded onto events
  additively; tier-less journals are unaffected). A flex turn multiplies by
  the row's flex rates when they exist.

Two rules keep the basis honest:

1. **Fallback is disclosed, never fabricated.** A flex-marked turn against a
   row WITHOUT flex slots bills at the row's standard rates, and the
   provenance note says so (`; service_tier: flex` plus the fallback note).
   The tool never invents a flex rate a provider never published.
2. **Tier resolves per request, not per-bucket.** The tier applied is the
   one the request ran on (tokscale #862 semantics: request-level
   resolution, not marginal per-bucket pricing). A single-model session
   whose turns ran at MIXED tiers therefore prices per usage block: each
   block at its own lane, totals exact across blocks, and the report
   discloses `mixed service tiers (priced per usage block)` — the
   session-level "current rates" column is withheld for such rows rather
   than quoting a single-lane rate that would misdescribe the bill.

## Where to see the applied lane

- `--overview` / report provenance: `; service_tier: flex` on flex-billed
  rows; `mixed service tiers (priced per usage block)` on multi-tier
  sessions.
- Governance audit recompute reprices on the session's tier and withholds
  the current-rate column for mixed-tier rows with the reason stated.
- `--test-match` discloses the tier of a matched pricing row.

The tier-suffix family is still growing (LiteLLM now also carries
`priority`, `ultrafast`, and `above_272k_tokens` lanes; ROADMAP rm-924
tracks representing them in the snapshot). Until a lane lands in the
snapshot, traffic billed on it by the provider is priced at the standard
row and disclosed — the fallback rule above.

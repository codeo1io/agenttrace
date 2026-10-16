---
title: Resolve exact rate rows before any normalization fold
date: 2026-10-15
category: reliability
module: agenttrace-core (pricing.rs rate/provider/source resolution over pricing_snapshot.json)
problem_type: logic_error
component: core
symptoms:
  - "Regional claude-sonnet-4-5 ids (us. / au. / bedrock-us-gov / us-gov-west-1 / anthropic.all prefixes) all billed the fold family's dedupe winner ($3.30/M, au.anthropic) instead of their own rows - the right dollars for the $3.30/M cross-region rows but uniform au.anthropic attribution, a +10% overbill on the $3.00/M rows (standard Bedrock invocation ids, global.anthropic) and a -8.3% underbill on the $3.60/M GovCloud rows"
  - "provider_for attributed the family winner's provider while the rate lookup billed the winner's rate - cost and provenance diverged for the same session"
  - "normalize_model strip_date/strip_version is non-idempotent (pricing.rs:1340-1410 at 3e90993: normalize_model, strip_date_suffix, strip_version_suffix), and two call sites normalizing the same id independently can land on different family keys"
root_cause: fold-before-exact-resolution
resolution_type: exact-first-seam-repair
severity: medium
tags: [pricing, normalization, exact-match, regional-rates, provenance, rate-row]
---

# Resolve exact rate rows before any normalization fold

## Problem

Every pricing surface that resolves a model id to the catalog — the rate lookup,
the provider label, and the pricing-source disclosure — accepted the id, folded
it through aliasing and `normalize_model` (date/version stripping), then matched.
A spelling-exact catalog row (`us.anthropic.claude-sonnet-4-5…` at $3.30/M,
`bedrock/us-gov-west-1/anthropic.claude-…` at $3.60/M) therefore lost to the
family-level row the fold produced: every `-v1:0` spelling folded to
`claude-sonnet-4-5-20250929-v1`, whose first-sorted dedupe winner
(au.anthropic, $3.30/M) billed them all — the right dollars for the
$3.30/M cross-region rows but uniform au.anthropic attribution, +10%
overbill on the $3.00/M rows, −8.3% underbill on GovCloud. The exact
row existed in the bundled snapshot and was simply never consulted. Worse, the
provider attribution path re-normalized the *already-resolved* key, so the rate
row and the provider row could come from different rows of the same catalog —
the session billed one provider's rate while reporting another's identity.

## Symptoms

- `us.anthropic.claude-sonnet-4-5-…`, `au.…`, `anthropic.all.…` sessions all
  resolved to the fold family's dedupe winner — au.anthropic's $3.30/M
  (the assess F-A2 live probe's "$3.30/M-family-collapse across all
  regional spellings"): the right dollars for the $3.30/M cross-region
  rows but every spelling attributed to au.anthropic, a +10% overbill
  on the $3.00/M rows (standard Bedrock invocation ids,
  `global.anthropic…`), and a −8.3% underbill on GovCloud ($3.60/M
  true). (Economics corrected at review fix 6f1290d735734e1b979ff2788a977f85:
  the pre-fix draft of this doc recorded the winner as $3.00/M.)
- The LiteLLM lattice is real and bundled: 19 of the 20 live
  `claude-sonnet-4-5` keys ($3.00 base, $3.30 us./au./eu./jp., $3.60 us-gov ×5)
  were already in `pricing_snapshot.json` — the data was present, the lookup
  order discarded it.
- `--quota`/report provenance could name a provider whose rate row was not the
  one billed (rm-1053's acceptance caught exactly this divergence).

## What Didn't Work

- Trying to make normalization smarter (more aliases, better stripping rules):
  any fold — however good — still destroys the exact spelling before the one
  row that matches it exactly can be found. This is an ordering defect, not a
  rule-quality defect.
- Re-normalizing at the consumer (`provider_for_in` re-normalized the resolved
  key): a second independent fold over already-folded input is where the
  divergence crept in; `normalize_model` is non-idempotent, so re-folding a
  folded id can move it again.
- Pinning regional rates in tests: there were zero regional-rate pins in
  `crates/*/tests`, so no test ever failed while every regional session
  mispriced (the band's fresh census made this absence explicit).

## Solution

Implemented in cycle 5 of run a7e0462f (implement 8443c707 + KTD13 repair
168b98ca, uncommitted for the commit gate), uncommitted surfaces:

- A spell-exact resolution pass runs FIRST at every pricing seam:
  `resolve_spell_exact(model, catalog)` is consulted before
  `resolve_alias`/normalize folds at the `provider_for_in` /
  `pricing_source_for_catalog` heads (`crates/agenttrace-core/src/pricing.rs`,
  the `rm-1053` comment block at `provider_for_in` states the contract: "a
  provider is claimed only where a price is claimed").
- Rate lookup and provider/source attribution now derive from the SAME resolved
  row object — the exact row when one matches, the family row only otherwise.
- The load-time `convert_litellm` key collapse keeps the full regional lattice
  instead of first-sorted-wins discarding the cross-region duplicates
  (`bundled_snapshot_regional_lattice_survives_in`).
- Red-first fixtures pin the behavior:
  `regional_rows_bill_their_own_rates_not_the_family_winner`,
  `rate_provider_and_source_resolve_from_the_same_row`, and (review fix
  6f1290d735734e1b979ff2788a977f85)
  `pricing_source_reads_the_billed_row_for_non_fixed_point_keys`
  (agenttrace-core lib tests, green in targeted 51/0 pricing filter and full
  1124/0/0 battery).
- **Override scope is family-scoped, and that is now disclosed** (review fix
  6f1290d735734e1b979ff2788a977f85, review finding F5): `parse_pricing_overrides`
  normalizes override keys, and an exact catalog id bills its own published row
  before any fold can apply a family override to it — so an override keyed by a
  family id no longer captures regional spellings (their own published rows are
  what should bill). Verbatim-key override reach is minted as roadmap rm-1438,
  and `pricing_source_for_catalog` consumes the spell-exact entries key
  verbatim so the override/deprecation disclosure reads the row the rate
  actually billed.

## Why This Works

The catalog is a table of exact spellings; normalization exists to catch
*approximations* of those spellings. Ordering exact-match before folding makes
the fold a fallback instead of a gate — the exact row can no longer be shadowed
by a row it never claimed to be. Deriving provider and source from the same
`ResolvedRow` object (not from re-resolved strings) removes the second
independent fold, which was the only place cost and provenance could diverge.

## Prevention rules

1. **Exact-match before any fold.** Any lookup over an enumeration of concrete
   keys (pricing, models, providers, registries) must attempt the spelling-exact
   match first; normalization/aliasing is a fallback tier, never a
   pre-processing step.
2. **Resolve once, consume the row.** Consumers (labels, disclosures,
   attribution) take the resolved row object, not a re-normalized string. Two
   call sites folding the same input independently is a divergence bug waiting
   for a lattice to expose it.
3. **Red-first fixtures against real lattices.** When an upstream data source
   enumerates regional/variant keys, pin at least one exact-row-wins test —
   absent pins are how a whole class of mispricing shipped silently.

## Vocabulary

- **Rate row** and **Family winner** — see `CONCEPTS.md`; the terms this fix's
  tests and riders are phrased in.

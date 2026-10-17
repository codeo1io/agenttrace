---
title: Pricing variant candidates must resolve against the catalog before builtin defaults
date: 2026-10-15
category: reliability
module: agenttrace-core (pricing.rs lookup_price_in / match_variants, insights.rs DataHealth)
problem_type: silent_mispricing
component: core
symptoms:
  - "Effort-suffixed model ids (claude-sonnet-5-5-medium, emitted live by Antigravity CLI per tokscale PR #1425) billed at the builtin Claude default (3e-6/1.5e-5, 200k ctx) while the exact base claude-sonnet-5-5 sat in the bundled catalog — PoC: 1000in/500out priced 0.0105 against the base's 0.007, a 1.5x output over-bill"
  - "cost_basis rendered 'priced' and the only disclosure was the aggregate data_health.fallback_pricing counter — nothing said the exact base rate was available"
  - "Pre-batch seam shape (corrected by review fix 1a615b64 — this file's original claim of a has_specific_price/lookup_price_in divergence was wrong): match_variants had no effort arm, so an effort-suffixed id's candidate list was [raw, normalized] only; the catalog-first variant arm had nothing to match and billed the builtin default while has_specific_price — walking the identical list — agreed. One shared resolution, one silently wrong answer"
root_cause: incomplete_variant_derivation
resolution_type: catalog_first_resolution
severity: medium
tags: [pricing, alias-resolution, builtin-default, variant-table, disclosure, effort-suffix, antigravity]
---

# Pricing variant candidates must resolve against the catalog before builtin defaults

## Problem

`lookup_price_in` resolved a model id in two steps: exact catalog match
via `matching_catalog_key` (which offers every `match_variants` candidate
to the catalog — this arm pre-existed the batch, unchanged), else consult
the same candidates against the **builtin** table, falling through to the
builtin Claude default when none hit. The defect (review fix 1a615b64
corrected this file's original asymmetric-tables story): `match_variants`
had **no effort arm**, so an effort-suffixed id's candidate list was
`[raw, normalized]` only — the derivable form that sat in the catalog was
never derived. `has_specific_price` walks the identical candidate list,
so both truth functions agreed on the builtin-default fallback: one
shared resolution, silently pricing the session at default rates while
the exact base sat one arm away in the catalog.

The visible instance: clients append a reasoning-effort tier to model ids
(Antigravity CLI emits `claude-sonnet-5-5-medium`/-low; models.dev carries
`:thinking` variants). No catalog — bundled (3,131 keys) or LiteLLM main
(4,116 keys) — keys effort-suffixed ids, and `match_variants`' minor-suffix
arm covered only digit/mini/nano/flash/lite/pro tails. The exact base was in
the catalog; the session still billed at builtin default rates, silently.

## Resolution (rm-1511, run 0915f736 cycle 4)

1. **Complete the variant derivation**: `match_variants` gains the effort
   arm, so the suffixed id's candidate list contains the in-catalog base and
   the pre-existing catalog-first arm (`matching_catalog_key`) resolves it.
   `lookup_price_in` additionally expands variants-of-variants against the
   catalog before any builtin fallback (pinned as a contract by the
   second-order test). `has_specific_price` shares the basis and moves in
   lockstep — there was no divergence to close (corrected by review fix
   1a615b64).
2. **Variant table gains the effort arm**: `-low/-medium/-high/-thinking/
   :thinking` strip to the base id. The raw id keeps the tier
   (`pricing::effort_tier_of` re-derives it), so tier-aware lanes compose.
3. **Disclose the residual class by name**: `data_health` gains
   `fallback_with_base_available` — models still on fallback whose
   unrecognized tail hides an in-catalog base, bounded to 5 distinct ids.
   Counting alone let a 1.5x over-bill render as `priced`. JSON lane:
   `--overview -f json` (the text/markdown health blocks render the
   aggregate counter only — scoped by review fix 1a615b64).

## The rule

When a resolver consults a fallback table through derivable variants, the
**authoritative table must see the same variants in the same order** — any
resolver that skips it manufactures a silent mis-price class whose only
witness is an aggregate counter. And when a lookup fails, ask whether the
*stripped* form would have succeeded: an available exact base behind an
unknown suffix is a disclosure obligation, not a default-rate bill. The gap
here sat one step upstream of both tables: the derivation itself never
produced the base candidate — completeness of the candidate list is part of
the resolution contract, not an incidental detail.

Red-first pin: `crates/agenttrace-core/tests/pricing_effort_alias_contract.rs`
(12 tests after review fix 1a615b64 — the original 9 grew by the
antigravity-lane end-to-end fixture, the second-order variant pin, and the
`effort_tier_of` SKU-suffix limitation pin; the recorded research PoC is the
flip test — 0.0105 → 0.007 through the real parse path — plus a guard that
fails if a suffixed key ever lands in the catalog, forcing a conscious
contract update).

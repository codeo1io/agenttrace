---
title: First-party ids the upstream catalog omits need a second catalog
date: 2026-10-16
category: reliability
module: agenttrace-core
problem_type: misattribution
component: crates/agenttrace-core/src/pricing.rs
severity: high
tags: [pricing, attribution, liteLLM, models-dev, dated-model-ids]
related_issues:
  - ROADMAP rm-1532 (this fix, run 92ea7949 cycle 4)
  - ROADMAP rm-1053 (the raw-row digest seam this rides on)
  - ROADMAP rm-1511 (the sibling variant-resolution fix)
---

## Problem

A Claude Code session naming `claude-sonnet-4-20250514` reported
`By Provider: openrouter` — and a `claude-3-7-sonnet-20250219`
session reported `vercel_ai_gateway` — with costs priced off those
resellers' rows. The sessions were first-party. (Direction matches the
assess 7fb648a0 PoC and the snapshot itself: the `claude-sonnet-4` base
collapse has both reseller rows, where `openrouter` wins priority 8
over `vercel_ai_gateway`'s 7; the `claude-3-7-sonnet` base has only the
vercel row at 7.)

The bundled LiteLLM snapshot carries NO row in any form for these dated
ids: not bare, not provider-qualified. Only reseller-qualified rows
(`openrouter/…`, `vercel_ai_gateway/…`, `github_copilot/…`) end in
them. The price ladder's last step collapses a dated id to its base
(`claude-sonnet-4-20250514` → `claude-sonnet-4`) and picks a winner by
provider priority among the reseller rows — so the base-model collapse
served a reseller for a first-party session.

A snapshot refresh cannot fix this: live LiteLLM (4,161 keys, checked
2026-10-16) has zero rows for these ids in any form. Upstream absence is
not staleness.

## Symptoms

- `--overview` groups first-party dated-model sessions under resellers.
- `--test-match <dated-id>` reports the base-model row and a reseller
  attribution.
- Per-session provenance shows a reseller snapshot as the source with no
  hint the first-party row is missing upstream.

## Root cause

The catalog has exactly one source of truth. When the single upstream
catalog structurally lacks a class of rows (dated first-party ids),
every consumer — lookup, provider attribution, provenance, tiered
lookup, the `--test-match` disclosure — inherits the closest surviving
row, which for dated ids is a reseller's. No seam existed for "the
primary catalog is silent, but the truth is first-party."

## Fix

A second, embedded catalog (`models_dev_snapshot.json`) — the
first-party pricing extract for exactly the ids the primary catalog
omits, with rates cross-checked against the reseller rows (identical:
resellers forward list pricing). One serving seam
(`serving_catalog`): the LiteLLM ladder runs FIRST; only when it finds
no first-party row does the extract serve. Rows self-retire the day
LiteLLM ships a first-party row, because the ladder never reaches the
extract then.

Four consumers route through the seam (`lookup_price`,
`lookup_price_tiered`, `provider_for_in`, `pricing_source_for_catalog`),
so rate, attribution, and provenance can never disagree about which
catalog served. The provenance line names WHY the extract served
("LiteLLM lacks a first-party row"), and the extract's bytes ride
`catalog_identity` — a changed extract (new binary) reprices warm
cached sessions, honoring the rm-196 warm-cache repricing contract.

A RESELLER-QUALIFIED id (`openrouter/claude-sonnet-4-20250514`) keeps
its reseller row: the extract speaks only for the bare first-party
form, never for an id the session itself scoped to a reseller lane.

## Verification

- `cargo test -p agenttrace-core --test first_party_pricing_extract_contract`
  — 9 pins: list rates, vendor attribution, provenance disclosure,
  specific-price status, reseller-preservation, primary-catalog
  precedence, default fallthrough, identity folding, provenance note.
- End-to-end on the assess fixture corpus (three first-party sessions,
  the three dated ids): `--overview` reports `anthropic 3`; per-session
  provenance reads `models.dev snapshot 2026-10-16 (embedded first-party
  extract; LiteLLM lacks a first-party row)`; `--test-match` shows the
  extract serving with attribution `anthropic`.

## Lesson

When a single upstream catalog is the source of truth for attribution,
check not just whether it is CURRENT but whether it COVERS the ids your
corpus actually records. Dated ids are the canonical gap: upstream
catalogs deprecate old rows faster than CLI agents deprecate old
sessions. The fix is not a refresh — it is a second, narrower catalog
with a precedence rule, consulted only where the first is silent, and
folded into every identity/digest the caches use.

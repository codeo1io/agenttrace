---
title: Cycle 4 batch selection — silent lies in attribution lanes
date: 2026-10-16
run: 92ea794984ae40f4ad8c73836efe21a6
cycle: 4
attempt: 7795001ff7b64a119dd87be25a9e9b77
---

# Decision

Cycle 4 implements two roadmap mints under one theme — "silent lies in
attribution lanes" — both proven live in this run's assess phase
(7fb648a0) and selected by the prioritize phase (eb812bab) against a
fresh cross-run fog sweep:

- **LEAD rm-1532** — second-catalog pricing. The bundled LiteLLM
  snapshot structurally lacks first-party rows for the dated Anthropic
  ids Claude Code records (live upstream confirmed silent, not merely
  stale), so the base-model collapse attributed first-party sessions
  to resellers (`openrouter`, `vercel_ai_gateway`). Fixed with an
  embedded models.dev first-party extract served only where the
  primary ladder finds no first-party row, disclosed at the provenance
  line, folded into `catalog_identity`.
- **PAIR rm-1534** — future admission bound. Range admission was
  floor-only, so far-future timestamps (year-9999 PoC) entered every
  forward window at HEALTH 100 with zero anomalies. Fixed with a
  now+24h upper bound shared by admission, a new `future_timestamp`
  anomaly class, and a counted+disclosed exclusion (never a silent
  drop), with `--range all` keeping the session visible.

## Why these two together

Both are attribution lanes that lie silently: one misattributes WHO
priced the work, the other misattributes WHEN it happened. Neither
errors, neither discloses — the report looks healthy while being wrong.
They share a fix philosophy: bound the trust, disclose the exception.

## What stayed out (fog-adjudicated)

- Antigravity numeric-id pricing: fog-owned by run 6d2a1d8d's unlanded
  rm-1864 banner ("no third mint" rule) — ceded.
- Unbounded-read family (assess F3): fog rm-1871 owns it — ceded.
- rm-1533/rm-1535 (batch riders minted but deprioritized): lower
  severity than the pair above; remain roadmap rows.

## Implementation notes

- Fixture model choice matters downstream: `warm_cache_pricing.rs`
  pins a "bundled" cold-run label, so its fixture model switched to a
  current dated sonnet the primary catalog carries first-party —
  rm-1532 would otherwise have that test asserting on the extract.
- 5-digit years (year 99999) are unparseable ISO-8601 and surface as
  unknown-time sessions (N7 bucket) — they never reach admission; the
  bound covers the parseable-but-impossible space.

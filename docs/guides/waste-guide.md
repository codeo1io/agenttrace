# Waste Score Guide

`agenttrace --waste <session>` prints a 0-100 score, a tier, and a
`Wasted` dollar figure. This guide pins what both numbers mean so a
report can be argued with, not just read (rm-567).

## The score: four capped components, sum 96 (not 100)

| Component | Max | How it is computed |
|---|---:|---|
| Cache | 36 | rating base — `none` 30 / `poor` 24 / `good` 12 / `excellent` 0 — plus 6 when the session wrote cache entries but reads < 30% of input (a paid-for cache that is not being hit) |
| Loops | 25 | loop-waste percent of session cost (from diagnostics), saturating at 50% → `min(percent, 50) × 0.5` |
| Stuck | 20 | 7 per stuck pattern + 5 per critical pattern |
| Tool bloat | 15 | `bloat_score × 15/90` (bloat_score: 90 severe >5 tools/turn, 65 high >3, 35 medium >1.5, 10 low) |

The components are capped individually and the sum is clamped to
0..100, so every tier band is reachable — including `red` (>= 70),
which was dead before this guide existed: the pre-rm-567 arithmetic
capped at 56, so "severe waste" could never print.

Tiers (score band → printed level, emoji, summary):
`green` < 15 → LOW 🟢 "efficient session"; `yellow` 15-39 → MODERATE 🟡
"minor waste …"; `orange` 40-69 → HIGH 🟠 "wasting $X: loops Y%, tools
Z/turn"; `red` >= 70 → SEVERE 🔴 "severe waste $X …". Before rm-567 the
red band and its SEVERE summary were dead code — the arithmetic
ceiling sat at 56, so no session could ever print them.

Guard note (rm-567 review fix): the pre-stuck sum (cache rating base
+ loops + bloat) is clamped at 80 as a defensive guard, but its
arithmetic ceiling is 70 — it can never bind. The paid-cache +6 is
added after the stuck component; the table above lists it with the
cache row (30 + 6 = 36) to keep each component's claim in one place.

## Machine-readable report: `--waste -f json`

The JSON arm prints the same numbers as the text lane plus the
component breakdown: `components.cache / loops / stuck / bloat` carry
each score with its inputs (rating, hit rate, loop %, pattern count,
tools/turn), `components.pre_stuck_sum` and `pre_stuck_clamped`
expose the guard, `wasted` carries `raw_usd` (uncapped), `total_usd`
(capped), `capped_to_session_cost` (bool), and `percent_of_session_cost`,
and `basis` names the session-cost denominator and which dollar bases
the Wasted figure draws from (cache premium on un-cached input, plus
measured loop cost — never both a component score and its dollars).

## Wasted dollars: two disjoint bases, clamped to the session cost

`Wasted` is the sum of exactly two components priced on disjoint bases:

1. **Cache premium** — the *avoidable* part of un-cached input spend:
   un-cached input tokens priced at `(input_rate − cache_read_rate)`,
   i.e. what a working cache would have saved. Models without
   cache-read pricing degrade to the full input rate. The full
   un-cached input spend is deliberately NOT used — it double-counts
   tokens the session legitimately had to send once.
2. **Loop cost** — the measured duplicate-call cost from diagnostics
   (per-duplicate-event heuristic, already capped at the session cost
   at its source).

Tool bloat contributes **no dollars** — it only scores. The pre-rm-567
report added an invented flat 5% of session cost whenever bloat
exceeded 50; that figure is gone.

The sum is **clamped to the session's estimated cost** and the clamp is
always disclosed: when the raw sum exceeds the session cost, the report
prints `Wasted: $X (capped from $R - exceeds session cost $C)`. Every
Wasted line also names its denominator (`N% of session cost $C`, or
`session cost unavailable` when the session's model is unpriced).

Worked example (the rm-567 motivating session): a 200-call session
reported `Wasted: $4.8173` beside an estimated session cost of $2.4450
— 197% of spend called wasted, labeled "minor waste" at score 30. The
score ceiling was unreachable and the dollar sum mixed overlapping
bases with an invented bloat add-on. Both are fixed by the rules
above; the same session now reports the clamped figure with its basis
on the line.

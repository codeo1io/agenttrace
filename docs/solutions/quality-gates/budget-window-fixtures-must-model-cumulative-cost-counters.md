# Budget-window fixtures must model cumulative cost counters

**Rule:** fixtures that feed the statusline budget/window series must model each session's
journal costs as CUMULATIVE counters (a capture carries the running total), not per-day
absolute costs — the series bills the day-over-day RISE (continuous-counter semantics,
landed rm-402), so a fixture whose totals dip day-over-day reads as a counter RESET and
contributes zero rise for that day. Build budget fixtures monotone non-decreasing. And
know which arm you are pinning: `render_budget_view` emits the window fields
(`daily[]`, `excluded_older_days`, `excluded_future_days`, …) at the TOP LEVEL of its
json, while `render_statusline_report` NESTS the same fields under a `budget` object —
assert the shape of the arm under test, not a guessed one.

**Incident (2026-10-10, agenttrace run 6603da8916bd cycle 1, implement 62fb8454):**
the rm-941 window tests (`tests/window_truth_batch.rs`) were drafted from per-day absolute
costs; the series under `statusline_budget_series` disagreed with the live CLI PoCs
because flat/dipping fixtures bill zero rise where the PoC's monotone journal bills the
day's spend. The same suite initially asserted the wrong json shape for one render arm
(top-level vs nested), producing failures that looked like product bugs.

**Why it bites:** the cumulative convention is correct product semantics (a statusline
journal appends running totals), but it is invisible in the fixture bytes — a fixture
"looks like" daily spend either way. Only the RISE interpretation separates them.

**Fix + proof:** fixtures rebuilt monotone (cumulative totals per session), and both
shapes pinned explicitly per arm; the suite's numbers then matched the live PoC outputs
exactly (`daily[7 dates]`, `excluded_older_days 2`, `excluded_future_days 1`,
`spend_7d_usd 1.0` on `--statusline-report -f json`). The semantics are now pinned in the
tests themselves (7/7 green; batch suite 876/0).

**Prevention:**

1. When writing any fixture that feeds `statusline_budget_series`, the daily card series,
   or a window aggregate: totals MONOTONE non-decreasing within a session; put the spend
   growth in the RISE between captures.
2. Pin the json shape per render arm (budget view: top-level; statusline report: nested
   under `budget`) — assert both in one parity test so a future re-nesting fails loudly.
3. Cross-check a new fixture against one live CLI PoC run before trusting its expected
   values; a fixture that contradicts the binary is a fixture-semantics bug first.

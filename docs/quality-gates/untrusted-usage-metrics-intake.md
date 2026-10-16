# Untrusted usage-metrics intake — clamp, disclose, never invent

**Class:** quality gate (parser intake) · **Origin:** rm-1430, cycle 3 of run a3f96656 (2026-10-15); observed live via tokscale #1369 · **Applies to:** every lane parser that consumes externally-recorded usage metrics (Cline/Roo landed; the same rule binds codex, gemini, pi, and future lanes)

## The defect class

A journal records usage metrics in provider-specific shapes, and the values are **untrusted JSON**: numbers or strings, sometimes negative, sometimes absent fields, sometimes an object that looks like usage but carries only a cost. Two symmetric failures hide in intake code:

1. **False zeros** — the parser doesn't recognize the carrier shape, reports `0` tokens / `$0.0000`, and the zero is indistinguishable from a genuinely free session (the rm-1430 finding: the whole 11-function Cline lane had zero token/usage references while real stores carry metrics).
2. **Invented truth** — the parser coerces what it finds into a number it can report: negatives pass through, unparseable fields are defaulted, cost-only objects are treated as usage evidence.

## The gate

Every intake of externally-recorded usage MUST:

- **Clamp at intake, and disclose the clamp.** Negative values are recorded as 0 with a counter (e.g. `cline_metrics_negative_clamped`) disclosed in the report's data-health disclosures — never silently propagated, never a hard failure.
- **Count what you cannot parse.** Present-but-unparseable fields increment a disclosed `unparsed_fields` counter; they are not defaulted, not dropped.
- **Refuse cost-only "carriers".** An object with a cost and no token fields is **not** a usage carrier; treating it as one invents token truth from a price. Require ≥1 token field before the object qualifies.
- **Adjudicate the input basis per provider — and pin the arithmetic.** The provider's `input` figure is either cache-EXCLUSIVE or cache-INCLUSIVE, and the intake must match which: Cline `tokensIn` meters uncached input only (upstream reconstructs the total additively as `tokensIn+cacheReads+cacheWrites`; Anthropic bills the cache legs separately from `input_tokens`) — record it by IDENTITY with the cache legs as separate disjoint counters (rm-1884; netting an exclusive basis mints false zeros on every cache-hit request); a genuinely cache-INCLUSIVE figure (the Gemini `promptTokenCount` class) keeps the rm-600 net-of-cache basis, saturating. Pin the arithmetic in tests on both sides of the line.
- **Flip provenance, suppress estimation.** When a real carrier is absorbed, token provenance becomes `reported_by_agent` and text-stream estimation is suppressed — but the event stream stays additive-untouched otherwise.
- **Pin the metrics-free case byte-identically.** A store without carriers must render exactly as before the feature landed (including preserved text-estimated fallbacks), or the change is a regression, not an addition.

## Test shape (red-first)

Write the failing tests against a banked payload fixture **before** the parser exists (fixture-first — no host corpus needed, per the tokscale-reference payload). Cover: string-typed numerics, negative metrics (clamp + disclosure), cost-only rejection, input-basis arithmetic (identity vs net-of-cache, per the adjudication above — the rm-1884 lesson: the wrong side of that line is the false-zero class), and the metrics-free zero-delta pin.

## Why this is a doc and not just a row

The Cline lane was usage-blind for its entire existence because "the parser has no usage code" looks like a feature gap, not a correctness bug — while users were reading printed `$0` totals as truth. Any future lane that reports usage inherits the same two failure modes; this gate is the checklist.

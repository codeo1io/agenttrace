# Batch selection — 2026-10-16, run e775cac5 cycle 1 (repository-maintenance)

Selected batch: **"cost-lane truth under hostile corpora"** — two rider-owned
items, completable end-to-end in one cycle, chosen from this run's adversarial
assessment (wall 16dcc9a) and live ecosystem research.

## Why these two

1. **rm-487 (lead) — subagent rollup arithmetic escapes the clamp-plus-saturate
   contract.** `attribute_subagents` rolled child totals into parents with six
   plain `+=` (`crates/agenttrace-core/src/subagents.rs:228-230` and
   `:255-257`) over `subagent_count: usize`, `subagent_cost: f64`,
   `subagent_tokens: i64` (`lib.rs:678/680/682`), while `analyze`'s contract
   (lib.rs ~1540-1570) mandates saturating adds for "a new sum of untrusted
   counts". A hostile two-child corpus (`2 × (i64::MAX/2 + 1)` tokens) panicked
   debug builds (rc101, `attempt to add with overflow`) and would silently wrap
   release totals. Because the debug profile is the ordinary test lane, the bug
   was reachable by any fixture-bearing regression test, not just adversarial
   users.
2. **rm-1025 (rider) — refusal message hardcodes the cap numeral.** The zstd
   rollout decode-cap bail at `parser.rs:105` said "64 MiB" as a literal while
   the cap is `ZSTD_ROLLOUT_DECODE_MAX_BYTES` (`parser.rs:71`); a future cap
   change would ship a lying error. Fixed with a derived numeral and an inline
   const-tracking test, following the landed `MCP_INPUT_MAX_BYTES` precedent
   (`mcp.rs:104-112`).

Both are small, independently revertable, and touch no sibling-owned seam (the
sibling 1e4f6b95 zstd batch rewrites the enclosing decode function; by-region
reconciliation declared in the stewardship request ce9f5654).

## What was excluded and why

- **Codex multi-frame zstd truncation** (research C1: identical 51-line corpus
  reports $0.0987 plain vs $0.0001 as two concatenated frames, rc0) — owned by
  sibling run 1e4f6b95's "zstd rollout truth end-to-end" batch; re-verified as
  a next-cycle lead if that batch does not land.
- **Pricing-drift golden exposure** (research C2: live LiteLLM 4,123 keys vs
  bundled 3,131; weekly cron auto-PR stages only 2 files and the SVG goldens
  byte-pin the 2026-10-08 footer) — time-boxed to the 2026-10-19 06:00 UTC
  cron fire; title-twin of unlanded rm-1514, fold by title at mint time.
- **rustls 0.23.45 → 0.23.46** (cures four standing RUSTSEC reds) — one-line
  lock change ceded to the dependency lane to keep this batch rider-clean.

## Outcome at compound time (pre-review)

Implemented at base 16dcc9a (attempt f3d9a239 edits, verified/adopted by
dc9127c6); targeted validation 61/61 across 8 lanes (attempt 0d022ed8) and
full suite 1147/0 over 65 suites with fmt/clippy/changelog gates green
(attempt 2ce4d0af). Review and shipping outcomes are carried forward by the
next cycle's assessment.

# External tier strings must fold case before an exact match prices them

- **discovered**: 2026-10-15, run 96287e100a56469babf0aae93fd0e9ed cycle 1 (assess attempt 0c6db522 F2; research attempt 03fbd4cf R3)
- **fixed by**: rm-164 rider (implemented pre-commit, attempts 394ea0f9/a5c25056; validated 7a3120e2 targeted, ce41e2e9 full)
- **surfaces**: `crates/agenttrace-core/src/pricing.rs` (`lookup_price_tiered`), `crates/agenttrace-core/src/parser.rs` (service_tier capture), `crates/agenttrace-core/tests/pricing_ultrafast_contract.rs`

## Context

The tiered-pricing lane (`lookup_price_tiered`) resolves a model's rates for the service tier a turn actually ran on: `Some("flex")` and `Some("ultrafast")` hit tier slots; everything else bills the standard rates. The tier string enters journals verbatim from user configuration — the parser records it as-written (`next_tier.to_string()`), and the metrics keep the verbatim form (that is deliberate, for disclosure).

## Problem

The tier match was exact-case. A journal carrying `"Flex"` or `"ULTRAFAST"` (case variance from user config) fell to the standard-rate arm: silently repriced, no mismatch disclosure. This is the worst failure class for a cost tool — the module's own comments forbid silent reprice. The exposure went from theoretical to guaranteed within days: opencode v1.19.0 (2026-10-11) enabled the OpenAI `ultrafast` service tier, so real opencode journals began carrying the string in whatever casing their config had.

## Solution

Normalize at the choke point, match on the folded form, keep the verbatim record:

```rust
let service_tier = service_tier.map(str::to_ascii_lowercase);
match service_tier.as_deref() { Some("flex") => …, Some("ultrafast") => …, _ => standard }
```

- Fold at the pricing seam, not the parser: the journal and the metrics keep the verbatim string (disclosure unchanged); only the comparison folds.
- Red-first contract: `tier_matching_is_case_insensitive` pins `"Flex"`/`"ULTRAFAST"`/`"FLEX"` to the flex/ultrafast rows AND the end-to-end lane (mixed-case journal bills the tier's dollars: 1M in + 1M out at flex $5/$25 = $30, not the $60 standard would bill), plus the verbatim-string-preserved assertion.

## Prevention / corollary rules

- A pure-code repricing (a fix that changes derived `cost_estimated` for UNCHANGED files over a byte-identical `pricing_snapshot.json`) is invisible to every freshness input the session cache has — the rm-196 catalog-identity gate keys the catalog bytes. It MUST ship a session-cache schema rung or a warm store replays the old price forever (the rm-485 class); this fix's rung is 72 → 73, landed with `pricing_tier_case_warm_cache.rs` at the integration's independent review.

- Any string sourced from user configuration that gates MONEY (or any exact-match dispatch) must fold case (and trim) before comparison. Config casing is not a contract.
- When a matching table is added (`Some("x") => …`), grep the ingestion path for normalization; absence of folding on the path is the bug, regardless of what today's producers send.
- An upstream enabling a previously-dormant string ("Allow the OpenAI ultrafast service tier") is a live-traffic trigger: treat it as users arriving at your matcher with arbitrary casing, because they arrive with whatever their config has.
- Keep verbatim strings for disclosure, fold copies for matching — do not rewrite the recorded value to make matching work.

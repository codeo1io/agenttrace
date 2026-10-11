//! rm-164 pricing tier/band contract — public-API goldens against the
//! bundled snapshot for the non-flex rate families (priority,
//! ultrafast, balanced, batches) and the long-context bands
//! (`…_above_<N>k_tokens`): every family a row carries bills its own
//! turns, tiers the row does not carry bill at the standard rates,
//! and a prompt past a band threshold reprices the whole request.

use agenttrace_core::lookup_price_banded;
use agenttrace_core::lookup_price_tiered;
use agenttrace_core::render_test_match;
use agenttrace_core::{analyze, Event};
use std::collections::BTreeMap;

fn per_million(price: &agenttrace_core::Price) -> (f64, f64) {
    (price.input, price.output)
}

#[test]
fn every_carried_family_bills_its_own_turn() {
    // gpt-6.1-sol's bundled row (2026-10-12 snapshot) carries every
    // non-flex family LiteLLM publishes for it — including ultrafast,
    // wire-reachable since opencode v1.19.0 allowed the tier:
    //   standard   $2.00/$10.00  (input_cost_per_token 2e-6)
    //   priority   $4.00/$20.00  (2×)
    //   ultrafast  $12.00/$60.00 (6× — the silent under-price rm-164
    //                            fixed: these turns billed $2/$10)
    //   batches    $1.00/$5.00   (½)
    let base = lookup_price_tiered("gpt-6.1-sol", None);
    assert_eq!(per_million(&base), (2.0, 10.0), "standard row");
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-6.1-sol", Some("priority"))),
        (4.0, 20.0)
    );
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-6.1-sol", Some("ultrafast"))),
        (12.0, 60.0)
    );
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-6.1-sol", Some("batches"))),
        (1.0, 5.0)
    );
}

#[test]
fn batch_wire_alias_and_case_normalization_bill_the_family() {
    // OpenAI's response `service_tier` uses the singular "batch" while
    // LiteLLM's field suffix is plural; wire casing is not trusted.
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-6.1-sol", Some("batch"))),
        (1.0, 5.0),
        "the singular wire alias must bill the batches family"
    );
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-6.1-sol", Some("ULTRAFAST"))),
        (12.0, 60.0),
        "wire casing is normalized"
    );
    // Unknown tier names still bill at the standard rates — never
    // zero, never a hostile fallback (rm-610 discipline carried over).
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-6.1-sol", Some("scale"))),
        (2.0, 10.0)
    );
}

#[test]
fn long_context_band_reprices_the_whole_request() {
    // gpt-6.1-sol carries a >272k band: $4/M in, $15/M out (2×/1.5×
    // the standard row). 300k input bills $1.20 at the band rates,
    // while 272k exactly — the band activates ABOVE the threshold —
    // still bills the standard $0.544.
    let banded = lookup_price_banded("gpt-6.1-sol", None, 300_000);
    assert_eq!(per_million(&banded), (4.0, 15.0));
    let unbanded = lookup_price_banded("gpt-6.1-sol", None, 272_000);
    assert_eq!(per_million(&unbanded), (2.0, 10.0));
    // A model without published bands never changes rates at any size
    // (deepseek-chat carries none).
    let plain = lookup_price_banded("deepseek-chat", None, 10_000_000);
    assert_eq!(
        per_million(&plain),
        per_million(&lookup_price_tiered("deepseek-chat", None)),
        "no bands on the row means no band repricing at any prompt size"
    );
}

#[test]
fn test_match_discloses_families_and_bands() {
    let rendered = render_test_match();
    // Same probe-surface discipline as the flex arm (rm-610): the
    // rate an ultrafast turn or a >272k prompt bills at must be
    // answerable from --test-match alone.
    assert!(
        rendered.contains("ultrafast"),
        "--test-match must disclose the ultrafast family; got:\n{rendered}"
    );
    assert!(
        rendered.contains("band >"),
        "--test-match must disclose long-context bands; got:\n{rendered}"
    );
}

fn usage_event(model: &str, tier: Option<&str>, input: i64, output: i64) -> Event {
    let mut usage = BTreeMap::new();
    usage.insert("input_tokens".to_string(), input);
    usage.insert("output_tokens".to_string(), output);
    Event {
        role: "meta".to_string(),
        usage,
        model_used: model.to_string(),
        service_tier: tier.map(str::to_string),
        ..Event::default()
    }
}

#[test]
fn ultrafast_session_bills_the_ultrafast_row_end_to_end() {
    // The wire-reachable under-price rm-164 exists to fix: 1M in +
    // 1M out on gpt-6.1-sol ultrafast billed $12 (standard) where the
    // row says $72 — 6× under. Session totals must bill the family
    // row and disclose the tier they priced on.
    let events = vec![usage_event(
        "gpt-6.1-sol",
        Some("ultrafast"),
        1_000_000,
        1_000_000,
    )];
    let metrics = analyze(&events, "gpt-6.1-sol");
    assert_eq!(metrics.cost_estimated, 72.0);
    assert_eq!(metrics.service_tier.as_deref(), Some("ultrafast"));
    assert!(
        metrics
            .provenance
            .pricing_source
            .contains("service_tier: ultrafast"),
        "the tier billed on must be disclosed: {}",
        metrics.provenance.pricing_source
    );
}

#[test]
fn band_crossing_bills_block_at_band_rates() {
    // A single 300k-input block on the tier-less turn bills at the
    // band rate ($1.20), and the session discloses the repricing.
    let events = vec![usage_event("gpt-6.1-sol", None, 300_000, 0)];
    let metrics = analyze(&events, "gpt-6.1-sol");
    assert_eq!(metrics.cost_estimated, 1.2);
}

#[test]
fn mixed_tier_session_prices_per_block_across_families() {
    // One model, three blocks of 1M in + 1M out each: the tier-less
    // block crosses gpt-6.1-sol's >272k band and bills the band row
    // ($4 in + $15 out = $19), the ultrafast block keeps the tier row
    // ($12 + $60 = $72 — the tier is the larger published factor),
    // and the priority block keeps its row ($4 + $20 = $24).
    let events = vec![
        usage_event("gpt-6.1-sol", None, 1_000_000, 1_000_000),
        usage_event("gpt-6.1-sol", Some("ultrafast"), 1_000_000, 1_000_000),
        usage_event("gpt-6.1-sol", Some("priority"), 1_000_000, 1_000_000),
    ];
    let metrics = analyze(&events, "gpt-6.1-sol");
    assert_eq!(metrics.cost_estimated, 115.0);
    assert_eq!(
        metrics.provenance.pricing_source,
        "mixed service tiers (priced per usage block)"
    );
}

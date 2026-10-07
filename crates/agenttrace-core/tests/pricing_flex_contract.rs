//! rm-610 pricing flex contract — public-API goldens against the
//! bundled snapshot (flex rows actually priced for flex turns, SKU
//! redirects live, `--test-match` discloses the flex tier, and a
//! tier-less world is byte-identical to the old behavior).

use agenttrace_core::lookup_price;
use agenttrace_core::lookup_price_tiered;
use agenttrace_core::render_test_match;
use agenttrace_core::{analyze, Event};
use std::collections::BTreeMap;

fn per_million(price: &agenttrace_core::Price) -> (f64, f64) {
    (price.input, price.output)
}

#[test]
fn flex_turns_bill_at_the_published_flex_rates() {
    // gpt-5.6's bundled row carries the published flex slots
    // ($2/M in, $10/M out vs $4/$20 standard).
    let standard = lookup_price("gpt-5.6");
    assert_eq!(per_million(&standard), (4.0, 20.0));
    let flex = lookup_price_tiered("gpt-5.6", Some("flex"));
    assert_eq!(per_million(&flex), (2.0, 10.0));
    assert_eq!(flex.cw, 2.5);
    assert_eq!(flex.cw, 2.5);
    assert!(
        (flex.cr - 0.2).abs() < 1e-9,
        "4e-6 scaled is 0.2 within f64"
    );
    // Tiers other than "flex" — including the None a tier-less turn
    // prices at — keep the standard rates exactly as before.
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-5.6", None)),
        (4.0, 20.0)
    );
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-5.6", Some("priority"))),
        (4.0, 20.0)
    );
}

#[test]
fn codex_sku_redirects_resolve_to_published_rows() {
    // The codex CLI reports model ids the LiteLLM chat catalog has no
    // row for; each must price at its published equivalent, not at
    // fallback ($1.125/$4.5 would mean fallback gpt-4o-era rates).
    for (sku, target) in [
        ("gpt-5.6-codex", "gpt-5.6"),
        ("gpt-reserve", "gpt-5.6-luna"),
        ("gpt-5.3-spark", "gpt-5.3-codex"),
    ] {
        assert_eq!(
            per_million(&lookup_price(sku)),
            per_million(&lookup_price(target)),
            "{sku} must price as {target}"
        );
    }
    assert_eq!(per_million(&lookup_price("gpt-5.3-codex")), (1.75, 14.0));
}

#[test]
fn test_match_discloses_the_flex_tier() {
    let rendered = render_test_match();
    // The flex line sits under the model it belongs to — same shape
    // the CLI prints for --test-match.
    assert!(
        rendered.contains("gpt-5.6") && rendered.contains("flex:"),
        "--test-match must disclose flex rates; got:\n{rendered}"
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
fn single_model_flex_session_bills_at_the_flex_row() {
    // Live-found gap while completing rm-610 (attempt 20f499a3): the
    // common codex shape — one model, every turn on the flex tier —
    // priced the session totals at the STANDARD row, because the
    // tier-aware arm only ran for multi-model sessions: 1M in + 1M out
    // on gpt-5.6 billed $24 where the flex row says $12.
    let events = vec![usage_event("gpt-5.6", Some("flex"), 1_000_000, 1_000_000)];
    let metrics = analyze(&events, "gpt-5.6");
    assert_eq!(
        metrics.cost_estimated, 12.0,
        "single-model session totals must bill the flex row"
    );
    assert_eq!(
        metrics.service_tier.as_deref(),
        Some("flex"),
        "the tier the totals were billed on is recorded on the metrics"
    );
    assert!(
        metrics
            .provenance
            .pricing_source
            .contains("service_tier: flex"),
        "the tier billed on must be disclosed: {}",
        metrics.provenance.pricing_source
    );
    // The tier-less session keeps the old number and the old label.
    let events = vec![usage_event("gpt-5.6", None, 1_000_000, 1_000_000)];
    let metrics = analyze(&events, "gpt-5.6");
    assert_eq!(metrics.cost_estimated, 24.0);
    assert!(!metrics.provenance.pricing_source.contains("service_tier"));
}

#[test]
fn mixed_tier_session_prices_per_block() {
    // One model, two tiers: each usage block bills on the tier that
    // produced it and the session says so instead of presenting a
    // single-rate estimate.
    let events = vec![
        usage_event("gpt-5.6", Some("flex"), 1_000_000, 1_000_000),
        usage_event("gpt-5.6", None, 1_000_000, 1_000_000),
    ];
    let metrics = analyze(&events, "gpt-5.6");
    assert_eq!(
        metrics.cost_estimated, 36.0,
        "$12 flex block + $24 standard block"
    );
    assert_eq!(
        metrics.provenance.pricing_source,
        "mixed service tiers (priced per usage block)"
    );
    assert_eq!(
        metrics.model_used, "gpt-5.6",
        "one model with several tiers is not 'multiple'"
    );
}

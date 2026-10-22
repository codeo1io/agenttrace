//! Cycle-2 ultrafast pricing contract (run b5d391ec, repository-maintenance
//! 7e0b1c6c cycle 2; roadmap rm-164 2026-10-12 refresh) — public-API
//! goldens against the bundled snapshot. The third tier family went
//! selectable in the wild (opencode >= 1.19.0 lets a user pick the
//! OpenAI `ultrafast` service tier) and LiteLLM publishes `*_ultrafast`
//! rate sets; before this batch the fork silently billed those turns at
//! the standard rates. Mirrors `pricing_flex_contract.rs` (rm-610).

use agenttrace_core::lookup_price;
use agenttrace_core::lookup_price_tiered;
use agenttrace_core::render_test_match;
use agenttrace_core::{analyze, Event};
use std::collections::BTreeMap;

fn per_million(price: &agenttrace_core::Price) -> (f64, f64) {
    (price.input, price.output)
}

#[test]
fn ultrafast_turns_bill_at_the_published_ultrafast_rates() {
    // gpt-6-astra's bundled row (source LiteLLM md5 836ab347…, fetched
    // 2026-10-12): standard $10/$50 per million, flex $5/$25, and the
    // ultrafast set $60 in / $300 out / $75 cw / $6 cr.
    let standard = lookup_price("gpt-6-astra");
    assert_eq!(per_million(&standard), (10.0, 50.0));
    let ultrafast = lookup_price_tiered("gpt-6-astra", Some("ultrafast"));
    assert_eq!(per_million(&ultrafast), (60.0, 300.0));
    // rm-1508: standard cache slots are Option — the ultrafast set
    // publishes both rates, unwrap() keeps the pin strict.
    assert!((ultrafast.cw.unwrap() - 75.0).abs() < 1e-9);
    assert!((ultrafast.cr.unwrap() - 6.0).abs() < 1e-9);
    // The sibling tiers keep their own rows: flex is NOT ultrafast.
    let flex = lookup_price_tiered("gpt-6-astra", Some("flex"));
    assert_eq!(per_million(&flex), (5.0, 25.0));
    // gpt-6.1-sol carries the family too ($12 in / $60 out ultrafast
    // vs $2/$10 standard) — the sibling families stay consistent.
    let sol = lookup_price_tiered("gpt-6.1-sol", Some("ultrafast"));
    assert_eq!(per_million(&sol), (12.0, 60.0));
    // A tier the catalog has no family for keeps the standard row —
    // including the None a tier-less turn prices at — exactly as
    // before this batch.
    for tier in [None, Some("default")] {
        assert_eq!(
            per_million(&lookup_price_tiered("gpt-6-astra", tier)),
            (10.0, 50.0),
            "tier {tier:?} is not a priced tier"
        );
    }
    // rm-1556 (cycle-4, run 1fb751544907): priority left the "not a
    // priced tier" set — gpt-6-astra's 2026-10-17 row publishes it at
    // 2x standard ($20/$100 per million) and a priority turn bills it.
    let priority = lookup_price_tiered("gpt-6-astra", Some("priority"));
    assert_eq!(per_million(&priority), (20.0, 100.0));
    // Post-base composition (rm-1508 landed on the wall after this
    // run's base): `Price::cw/cr` are Option — the priority row
    // publishes both rates, so Some()/unwrap() keeps the pin strict.
    assert_eq!(priority.cw, Some(25.0));
    assert!((priority.cr.unwrap() - 2.0).abs() < 1e-9);
}

#[test]
fn model_without_ultrafast_slots_bills_standard() {
    // Published behavior carried over from the flex decision
    // (codeburn #1642): no ultrafast row means standard rates, never
    // zero and never a guess.
    let ultrafast = lookup_price_tiered("deepseek-chat", Some("ultrafast"));
    let plain = lookup_price("deepseek-chat");
    assert_eq!(per_million(&ultrafast), per_million(&plain));
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
fn single_model_ultrafast_session_bills_at_the_ultrafast_row() {
    // The rm-610 single-model regression, retold for the third tier:
    // 1M in + 1M out on gpt-6-astra ultrafast is $60 + $300 = $360,
    // not the $60 the standard row would bill.
    let events = vec![usage_event(
        "gpt-6-astra",
        Some("ultrafast"),
        1_000_000,
        1_000_000,
    )];
    let metrics = analyze(&events, "gpt-6-astra");
    assert_eq!(
        metrics.cost_estimated, 360.0,
        "session totals must bill the ultrafast row"
    );
    assert_eq!(
        metrics.service_tier.as_deref(),
        Some("ultrafast"),
        "the tier the totals were billed on is recorded on the metrics"
    );
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
fn mixed_ultrafast_session_prices_per_block() {
    // One model, ultrafast + tier-less blocks: each block bills on
    // the tier that produced it ($360 + $60) and the session says so.
    let events = vec![
        usage_event("gpt-6-astra", Some("ultrafast"), 1_000_000, 1_000_000),
        usage_event("gpt-6-astra", None, 1_000_000, 1_000_000),
    ];
    let metrics = analyze(&events, "gpt-6-astra");
    assert_eq!(
        metrics.cost_estimated, 420.0,
        "$360 ultrafast block + $60 standard block"
    );
    assert_eq!(
        metrics.provenance.pricing_source,
        "mixed service tiers (priced per usage block)"
    );
}

#[test]
fn test_match_discloses_the_ultrafast_tier() {
    let rendered = render_test_match();
    assert!(
        rendered.contains("gpt-6-astra") && rendered.contains("ultrafast:"),
        "--test-match must disclose ultrafast rates; got:\n{rendered}"
    );
}

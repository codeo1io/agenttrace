//! rm-610/rm-164 pricing tier contract — public-API goldens against the
//! bundled snapshot (tier rows actually priced for their turns: flex,
//! priority and ultrafast rows from the catalog, fast from the published
//! per-model multipliers; SKU redirects live, `--test-match` discloses
//! the tier rates, and a tier-less world is byte-identical to the old
//! behavior).

use agenttrace_core::lookup_price;
use agenttrace_core::lookup_price_tiered;
use agenttrace_core::render_test_match;
use agenttrace_core::{analyze, Event};
use std::collections::BTreeMap;

fn per_million(price: &agenttrace_core::Price) -> (f64, f64) {
    (price.input, price.output)
}

/// 1e-7 * 1e6 is 0.09999999999999999 in f64, not 0.1 — tiny-tier
/// rows need a tolerance, not ==.
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
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
    // A tier-less turn keeps the standard rates exactly as before.
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-5.6", None)),
        (4.0, 20.0)
    );
    // "priority" used to price at the standard row too — that pin
    // belonged to a world where the importer dropped the `*_priority`
    // slots. Since the 2026-10-13 snapshot refresh (rm-176) the row
    // carries LiteLLM's published priority rates ($8/M in, $40/M out,
    // exactly 2x standard for GPT-5.6) and the tier-aware lookup bills
    // them (rm-164). See priority_turns_bill_at_the_published_priority_rates.
}

#[test]
fn priority_turns_bill_at_the_published_priority_rates() {
    // rm-164: LiteLLM publishes absolute `*_priority` per-token rates on
    // 239 chat rows (2026-10-13 fetch); the 2026-10-13 refresh (rm-176)
    // carries them into the bundle and the tier-aware lookup bills them.
    // GPT-5.6's published priority row is exactly 2x standard.
    let priority = lookup_price_tiered("gpt-5.6", Some("priority"));
    assert_eq!(per_million(&priority), (8.0, 40.0));
    assert_eq!(priority.cw, 10.0);
    assert!(
        (priority.cr - 0.8).abs() < 1e-9,
        "8e-7 scaled is 0.8 in f64"
    );
    // A tier the row has no published rates for still bills standard
    // (published behavior, the same rule as a flex-less flex turn).
    assert_eq!(
        per_million(&lookup_price_tiered("deepseek-chat", Some("priority"))),
        per_million(&lookup_price("deepseek-chat")),
        "deepseek-chat carries no priority slots; a priority turn bills standard"
    );
}

#[test]
fn ultrafast_turns_bill_at_the_published_ultrafast_rates() {
    // rm-164 (2026-10-13 addendum): opencode v1.19.0 puts the OpenAI
    // `ultrafast` service tier on the wire, LiteLLM publishes absolute
    // `*_ultrafast` rates for it (gpt-6-astra / gpt-6.1-sol rows), and a
    // fork that priced every tier at the standard row under-billed an
    // ultrafast gpt-6-astra turn by 6x. Astra: standard $10/$50,
    // priority 2x, ultrafast 6x ($60/$300).
    let standard = lookup_price("gpt-6-astra");
    assert_eq!(per_million(&standard), (10.0, 50.0));
    let ultrafast = lookup_price_tiered("gpt-6-astra", Some("ultrafast"));
    assert_eq!(per_million(&ultrafast), (60.0, 300.0));
    assert_eq!(ultrafast.cw, 75.0);
    assert_eq!(ultrafast.cr, 6.0);
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-6-astra", Some("priority"))),
        (20.0, 100.0),
        "the same row's priority slots bill the priority tier"
    );
    // A model with no ultrafast row bills ultrafast at standard.
    let luna = lookup_price_tiered("gpt-6-luna", Some("ultrafast"));
    assert!(
        near(luna.input, 0.1) && near(luna.output, 0.5),
        "gpt-6-luna carries no ultrafast slots; the turn bills standard ({}, {})",
        luna.input,
        luna.output
    );
}

#[test]
fn fast_turns_bill_at_per_model_fast_multipliers() {
    // rm-164 (2026-10-13 addendum): LiteLLM publishes NO `*_fast`
    // per-token fields anywhere (2026-10-13 fetch, all keys) — fast is
    // published per-model as a multiplier on the standard rates
    // (ccusage PR #1855, merged 2026-10-10: GPT-6 Astra/Sol, GPT-6.1
    // Sol and GPT-6 Luna fast = 2x standard, corroborated by LiteLLM
    // listing those rows' `*_priority` rates at exactly 2x). Per-model
    // on purpose, never a flat table; a model without a published fast
    // multiplier bills fast at the standard rates.
    let fast = lookup_price_tiered("gpt-6-luna", Some("fast"));
    assert!(
        near(fast.input, 0.2) && near(fast.output, 1.0),
        "2x standard on GPT-6 Luna ({}, {})",
        fast.input,
        fast.output
    );
    assert!(near(fast.cw, 0.25) && near(fast.cr, 0.02));
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-6-astra", Some("fast"))),
        (20.0, 100.0),
        "the multiplier scales the standard row even where explicit tier rows exist"
    );
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-5.6", Some("fast"))),
        (4.0, 20.0),
        "GPT-5.6 has no published fast multiplier; the turn bills standard"
    );
}

#[test]
fn end_to_end_ultrafast_session_bills_the_tier_row() {
    // The tier the session actually ran on must reach the final cost
    // through the aggregate path, not just the lookup: 1M in + 1M out
    // on gpt-6-astra ultrafast is $60 + $300 = $360, not the standard
    // row's $60 total.
    let events = vec![usage_event(
        "gpt-6-astra",
        Some("ultrafast"),
        1_000_000,
        1_000_000,
    )];
    let metrics = analyze(&events, "gpt-6-astra");
    assert_eq!(
        metrics.cost_estimated, 360.0,
        "ultrafast session totals must bill the ultrafast row"
    );
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
    // the CLI prints for --test-match. rm-164 (2026-10-13): the other
    // catalog tiers and the fast multiplier line are disclosed the
    // same way, so an operator can see every rate a turn can bill.
    assert!(
        rendered.contains("gpt-5.6") && rendered.contains("flex:"),
        "--test-match must disclose flex rates; got:\n{rendered}"
    );
    assert!(
        rendered.contains("gpt-5.6") && rendered.contains("priority:"),
        "--test-match must disclose priority rates; got:\n{rendered}"
    );
    assert!(
        rendered.contains("gpt-6-astra") && rendered.contains("ultrafast:"),
        "--test-match must disclose ultrafast rates; got:\n{rendered}"
    );
    assert!(
        rendered.contains("fast:"),
        "--test-match must disclose the fast multiplier line; got:\n{rendered}"
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

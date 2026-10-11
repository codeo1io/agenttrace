//! rm-610 pricing flex contract — public-API goldens against the
//! bundled snapshot (flex rows actually priced for flex turns, SKU
//! redirects live, `--test-match` discloses the flex tier, and a
//! tier-less world is byte-identical to the old behavior).

use agenttrace_core::lookup_price;
use agenttrace_core::lookup_price_tiered;
use agenttrace_core::render_test_match;
use agenttrace_core::{analyze, cost_audit, has_specific_tier_price, Event};
use std::collections::BTreeMap;

fn per_million(price: &agenttrace_core::Price) -> (f64, f64) {
    (price.input, price.output)
}

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
    // Tiers other than "flex" — including the None a tier-less turn
    // prices at — keep the standard rates exactly as before.
    // rm-164: "priority" left this list — gpt-5.6 now carries a
    // published priority row (2x standard); the vocabulary test below
    // owns it.
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-5.6", None)),
        (4.0, 20.0)
    );
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-5.6", Some("default"))),
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
fn test_match_discloses_every_tier_family() {
    let rendered = render_test_match();
    // The tier lines sit under the model they belong to — same shape
    // the CLI prints for --test-match. rm-164: priority (gpt-5.6) and
    // ultrafast (gpt-6-astra, added to the probe set) must be as
    // visible as flex, or a tier billing 6x standard hides behind the
    // probe.
    assert!(
        rendered.contains("gpt-5.6") && rendered.contains("flex:"),
        "--test-match must disclose flex rates; got:\n{rendered}"
    );
    assert!(
        rendered.contains("priority:"),
        "--test-match must disclose priority rates; got:\n{rendered}"
    );
    assert!(
        rendered.contains("ultrafast:"),
        "--test-match must disclose ultrafast rates; got:\n{rendered}"
    );
}

#[test]
fn tier_vocabulary_priority_and_ultrafast_bill_their_rows() {
    // rm-164: the 2026-10-12 snapshot carries priority on 214 rows
    // and ultrafast on 6 (gpt-6-astra: 6x in / 6x out — the tier
    // opencode v1.19.0 selects). End-to-end billing over the real
    // corpora lives in tier_vocab_corpora_bill_their_tiers_end_to_end;
    // this is the rate-row contract.
    let priority = lookup_price_tiered("gpt-5.6", Some("priority"));
    assert_eq!(
        per_million(&priority),
        (8.0, 40.0),
        "gpt-5.6 priority is 2x standard"
    );
    let ultrafast = lookup_price_tiered("gpt-6-astra", Some("ultrafast"));
    assert_eq!(
        per_million(&ultrafast),
        (60.0, 300.0),
        "gpt-6-astra ultrafast is 6x standard"
    );
    let flex = lookup_price_tiered("gpt-6-astra", Some("flex"));
    assert_eq!(per_million(&flex), (5.0, 25.0), "astra flex is half");
    let standard = lookup_price("gpt-6-astra");
    assert_eq!(per_million(&standard), (10.0, 50.0));
    // A family the row does not carry bills standard — the flex
    // fallback (codeburn #1642) generalized to every family.
    assert_eq!(
        per_million(&lookup_price_tiered("gpt-5.6", Some("ultrafast"))),
        (4.0, 20.0),
        "gpt-5.6 carries no ultrafast row"
    );
}

#[test]
fn unpriced_tier_bills_standard_but_is_never_exact() {
    // rm-164: the exactness seam. A catalog-matched model on a tier
    // the row cannot price (unknown string, or a family it does not
    // carry) bills at the standard rates — and the coverage audit
    // must classify it fallback, because the vendor's actual rate for
    // that tier is unknown. Before rm-164 the tier was silently
    // dropped BOTH in billing and in the exact-count (the ultrafast
    // PoC: $0.0232 billed at 6x under, footer claiming exact=1).
    assert!(
        has_specific_tier_price("gpt-5.6", None),
        "no tier is exact-standard"
    );
    assert!(has_specific_tier_price("gpt-5.6", Some("default")));
    assert!(
        has_specific_tier_price("gpt-5.6", Some("flex")),
        "carried family is exact"
    );
    assert!(has_specific_tier_price("gpt-5.6", Some("priority")));
    assert!(has_specific_tier_price("gpt-6-astra", Some("ultrafast")));
    assert!(
        !has_specific_tier_price("gpt-5.6", Some("ultrafast")),
        "family the row does not carry is NOT exact"
    );
    assert!(
        !has_specific_tier_price("gpt-5.6", Some("banana")),
        "unknown tier string is NOT exact"
    );
    assert!(
        !has_specific_tier_price("totally-unknown-model-x", Some("flex")),
        "unmatched model is never exact"
    );
}

#[test]
fn tier_vocab_corpora_bill_their_tiers_end_to_end() {
    // rm-164: the end-to-end contract over the vendored research
    // corpora (journal-truth/tier-vocab/) — the same PoC that was RED
    // at dispatch (base $0.0232 == ultrafast $0.0232 with the footer
    // claiming exact=1). Each corpus is the same codex rollout with
    // one recorded service tier; the rate row must follow the tier,
    // and an unknown tier must bill standard WITHOUT claiming exact.
    use std::path::Path;

    let fixture = |tier: &str| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/journal-truth/tier-vocab")
            .join(tier)
            .join("sessions/2026/10/07/rollout-tier.jsonl")
    };
    let cost_of = |tier: &str| {
        let raw = std::fs::read_to_string(fixture(tier)).unwrap();
        let session =
            agenttrace_core::parse_raw_session("rollout-tier.jsonl", "rollout-tier.jsonl", &raw)
                .unwrap();
        assert!(session.metrics.cost_estimated > 0.0);
        (
            session.metrics.cost_estimated,
            session.metrics.service_tier.clone(),
        )
    };

    let (base_cost, base_tier) = cost_of("base");
    assert_eq!(base_tier.as_deref(), Some("default"));
    assert!(
        near(base_cost, 0.0232),
        "base corpus pins the standard bill (got {base_cost})"
    );

    // 6x / 2x / 0.5x of the same token stream — the tier the turn
    // ran on is the rate row that prices it.
    let (ultrafast_cost, ultrafast_tier) = cost_of("ultrafast");
    assert_eq!(ultrafast_tier.as_deref(), Some("ultrafast"));
    assert!(
        near(ultrafast_cost, base_cost * 6.0),
        "ultrafast bills 6x (got {ultrafast_cost})"
    );

    let (priority_cost, priority_tier) = cost_of("priority");
    assert_eq!(priority_tier.as_deref(), Some("priority"));
    assert!(
        near(priority_cost, base_cost * 2.0),
        "priority bills 2x (got {priority_cost})"
    );

    let (flex_cost, flex_tier) = cost_of("flex");
    assert_eq!(flex_tier.as_deref(), Some("flex"));
    assert!(
        near(flex_cost, base_cost * 0.5),
        "flex bills half (got {flex_cost})"
    );

    // banana: no such tier exists — bills the standard rates (the
    // flex fallback generalized), and the audit disclosure lives in
    // tier_audit_discloses_tier_fallback below.
    let (banana_cost, banana_tier) = cost_of("banana");
    assert_eq!(banana_tier.as_deref(), Some("banana"));
    assert!(
        near(banana_cost, base_cost),
        "unknown tier bills standard (got {banana_cost})"
    );
}

#[test]
fn tier_audit_discloses_tier_fallback() {
    // rm-164: the governance seam. A banana-tier session of a
    // catalog-matched model must classify fallback_estimate with a
    // note naming the tier — never catalog_estimate/exact — and a
    // carried tier (priority) must classify exact.
    use std::path::Path;

    let read = |tier: &str| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/journal-truth/tier-vocab")
            .join(tier)
            .join("sessions/2026/10/07/rollout-tier.jsonl");
        agenttrace_core::parse_raw_session(
            "rollout-tier.jsonl",
            "rollout-tier.jsonl",
            &std::fs::read_to_string(path).unwrap(),
        )
        .unwrap()
    };

    let audit = cost_audit(&[read("banana")]);
    assert_eq!(
        audit.pricing_coverage.fallback_priced_sessions, 1,
        "an unpriceable tier is fallback coverage, never exact"
    );
    assert_eq!(audit.pricing_coverage.priced_sessions, 0);
    let row = &audit.by_provider_model[0];
    assert_eq!(row.pricing_status, "fallback_estimate");
    assert!(
        row.pricing_note.contains("banana"),
        "the note must name the tier: {}",
        row.pricing_note
    );

    let audit = cost_audit(&[read("priority")]);
    assert_eq!(
        audit.pricing_coverage.priced_sessions, 1,
        "a carried tier is exact coverage"
    );
    assert_eq!(audit.pricing_coverage.fallback_priced_sessions, 0);
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

    // rm-164: the same single-model path through the priority family
    // — the gap rm-610 fixed for flex held for every other tier too.
    let events = vec![usage_event(
        "gpt-5.6",
        Some("priority"),
        1_000_000,
        1_000_000,
    )];
    let metrics = analyze(&events, "gpt-5.6");
    assert_eq!(
        metrics.cost_estimated, 48.0,
        "single-model session totals must bill the priority row"
    );
    assert_eq!(metrics.service_tier.as_deref(), Some("priority"));
    assert!(metrics
        .provenance
        .pricing_source
        .contains("service_tier: priority"));
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

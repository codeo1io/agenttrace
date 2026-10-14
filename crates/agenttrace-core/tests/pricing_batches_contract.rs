//! Batch-tier pricing contract (run 499a64a9, repository-maintenance
//! 99de1990 cycle 2; roadmap rm-164 2026-10-15 refresh) — public-API
//! goldens against the bundled snapshot. LiteLLM publishes
//! `*_batches` rate sets (the async batch-discount family — exactly
//! 0.5x standard on every carrier at fetch time); before this batch
//! the fork silently billed `service_tier: "batch"` turns at the
//! standard rates. Mirrors `pricing_ultrafast_contract.rs`
//! (rm-610 flex / cycle-2 ultrafast lineage).

use agenttrace_core::lookup_price;
use agenttrace_core::lookup_price_tiered;
use agenttrace_core::render_test_match;
use agenttrace_core::{analyze, Event};
use std::collections::BTreeMap;

fn per_million(price: &agenttrace_core::Price) -> (f64, f64) {
    (price.input, price.output)
}

#[test]
fn batch_turns_bill_at_the_published_batch_rates() {
    // anthropic.claude-haiku-4-5-20251001-v1:0's bundled row (source
    // LiteLLM `_snapshot.date` 2026-10-14, fetched 2026-10-15):
    // standard $1 in / $5 out per million, batch set exactly 0.5x
    // ($0.5 / $2.5); the row carries no batch cache slots, so cw/cr
    // fall back to the standard $1.25 / $0.1 (the documented
    // missing-cache-slot rule).
    let standard = lookup_price("anthropic.claude-haiku-4-5-20251001-v1:0");
    assert_eq!(per_million(&standard), (1.0, 5.0));
    let batch = lookup_price_tiered("anthropic.claude-haiku-4-5-20251001-v1:0", Some("batch"));
    assert_eq!(per_million(&batch), (0.5, 2.5));
    assert!((batch.cw - 1.25).abs() < 1e-9, "cw falls back to standard");
    assert!((batch.cr - 0.1).abs() < 1e-9, "cr falls back to standard");
    // The plural wire spelling (the LiteLLM field suffix) resolves to
    // the same slots — emitters are inconsistent between `batch`
    // (OpenAI service_tier value) and `batches`.
    let plural = lookup_price_tiered("anthropic.claude-haiku-4-5-20251001-v1:0", Some("batches"));
    assert_eq!(per_million(&plural), (0.5, 2.5));
    // The sibling tiers keep their own rows on the same model's
    // family: sonnet-4.5 batch is 1.5/7.5 vs standard 3/15.
    let sonnet = lookup_price_tiered("claude-sonnet-4-5-20250929", Some("batches"));
    assert_eq!(per_million(&sonnet), (1.5, 7.5));
    let sonnet_std = lookup_price("claude-sonnet-4-5-20250929");
    assert_eq!(per_million(&sonnet_std), (3.0, 15.0));
    // A tier the catalog has no family for keeps the standard row —
    // including the None a tier-less turn prices at — exactly as
    // before this batch.
    for tier in [None, Some("default"), Some("priority")] {
        assert_eq!(
            per_million(&lookup_price_tiered(
                "anthropic.claude-haiku-4-5-20251001-v1:0",
                tier
            )),
            (1.0, 5.0),
            "tier {tier:?} is not a priced tier"
        );
    }
}

#[test]
fn model_without_batch_slots_bills_standard() {
    // Published behavior carried over from the flex decision
    // (codeburn #1642): no batch row means standard rates, never
    // zero and never a guess. deepseek-chat carries no `*_batches`
    // fields in the bundle.
    let batch = lookup_price_tiered("deepseek-chat", Some("batch"));
    let plain = lookup_price("deepseek-chat");
    assert_eq!(per_million(&batch), per_million(&plain));
}

#[test]
fn bundle_carries_the_batch_family_at_scale() {
    // The refresh must keep the family (311 input carriers at the
    // 2026-10-14 source date); a trim regression that silently drops
    // the fields trips this census before anything else notices.
    // Parsed straight from the snapshot the binary embeds.
    let raw = include_str!("../src/pricing_snapshot.json");
    let parsed: serde_json::Value = serde_json::from_str(raw).expect("snapshot parses");
    let mut carriers = 0usize;
    for (_key, row) in parsed.as_object().expect("object") {
        if row.get("input_cost_per_token_batches").is_some() {
            carriers += 1;
        }
    }
    assert!(
        carriers >= 280,
        "batch-tier family shrank to {carriers} carriers (expected >=280 at the 2026-10-14 source date; 311 at fetch)"
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
fn single_model_batch_session_bills_at_the_batch_row() {
    // The rm-610 single-model regression, retold for the fourth
    // tier: 1M in + 1M out on haiku-4.5 batch is $0.5 + $2.5 = $3,
    // not the $6 the standard row would bill.
    let events = vec![usage_event(
        "anthropic.claude-haiku-4-5-20251001-v1:0",
        Some("batch"),
        1_000_000,
        1_000_000,
    )];
    let metrics = analyze(&events, "anthropic.claude-haiku-4-5-20251001-v1:0");
    assert_eq!(
        metrics.cost_estimated, 3.0,
        "session totals must bill the batch row"
    );
    assert_eq!(
        metrics.service_tier.as_deref(),
        Some("batch"),
        "the tier the totals were billed on is recorded on the metrics"
    );
    assert!(
        metrics
            .provenance
            .pricing_source
            .contains("service_tier: batch"),
        "the tier billed on must be disclosed: {}",
        metrics.provenance.pricing_source
    );
}

#[test]
fn mixed_batch_session_prices_per_block() {
    // One model, batch + tier-less blocks: each block bills on the
    // tier that produced it ($3 + $6) and the session says so.
    let events = vec![
        usage_event(
            "anthropic.claude-haiku-4-5-20251001-v1:0",
            Some("batch"),
            1_000_000,
            1_000_000,
        ),
        usage_event(
            "anthropic.claude-haiku-4-5-20251001-v1:0",
            None,
            1_000_000,
            1_000_000,
        ),
    ];
    let metrics = analyze(&events, "anthropic.claude-haiku-4-5-20251001-v1:0");
    assert_eq!(
        metrics.cost_estimated, 9.0,
        "$3 batch block + $6 standard block"
    );
    assert_eq!(
        metrics.provenance.pricing_source,
        "mixed service tiers (priced per usage block)"
    );
}

#[test]
fn test_match_discloses_the_batch_tier() {
    let rendered = render_test_match();
    assert!(
        rendered.contains("claude-sonnet-4-5-20250929") && rendered.contains("batch:"),
        "--test-match must disclose batch rates; got:\n{rendered}"
    );
}

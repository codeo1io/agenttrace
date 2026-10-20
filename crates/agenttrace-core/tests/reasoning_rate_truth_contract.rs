//! rm-2018 (run e56e9e18): reasoning-rate truth at billing.
//!
//! A catalog row that publishes `output_cost_per_reasoning_token`
//! bills its reasoning/thinking span at THAT rate — the span rides the
//! output bucket (the parser folds Sibling keys into output and
//! reports Subset keys inside it), so the split moves
//! `min(reasoning, output)` tokens from the plain-output term to the
//! reasoning term. A row that publishes none bills reasoning at the
//! plain output rate, byte-identically to pre-rm-2018 pricing. An
//! explicit `0` is a published free-thinking rate (the rm-1508
//! absence-vs-published discipline, carried into this field). Every
//! application of a distinct published rate fires the
//! `reasoning_rate_applied` disclosure counter on the session's
//! non-loss channel; absence never does.
//!
//! One `#[test]` for the whole contract on purpose, exactly like its
//! rm-1508 sibling (`cache_rate_truth_contract.rs`): the pricing
//! catalog is a process-global `OnceLock` seeded from
//! `AGENTTRACE_PRICING_FILE` on first use, so the override has to be
//! in place before any lookup and must not race sibling tests.

use agenttrace_core::{analyze, lookup_price, Event, Price};
use std::collections::BTreeMap;

/// A pi-shaped usage event: role `meta` carries usage and model
/// attribution, the shape both the session-rate and per-block arms
/// price. `output` is the REPORTED output bucket (reasoning included);
/// `reasoning` is the separately-reported thinking span inside it.
fn usage_event(model: &str, input: i64, output: i64, reasoning: i64) -> Event {
    let mut usage = BTreeMap::new();
    usage.insert("input_tokens".to_string(), input);
    usage.insert("output_tokens".to_string(), output);
    usage.insert("reasoning_tokens".to_string(), reasoning);
    Event {
        role: "meta".to_string(),
        usage,
        model_used: model.to_string(),
        service_tier: None,
        ..Event::default()
    }
}

#[test]
fn distinct_reasoning_rates_bill_the_reasoning_span_and_stay_distinct_from_absence() {
    // The override catalog merges per-entry over the bundled one, so
    // three synthetic rows pin all three states without touching any
    // real model's rates. All three share input $10/M, output $30/M;
    // only the reasoning slot differs (the flagship real row,
    // dashscope/qwen-turbo, is pinned in pricing_snapshot_hygiene.rs).
    let dir = std::env::temp_dir().join(format!("at-reasoning-truth-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let override_path = dir.join("pricing-override.json");
    std::fs::write(
        &override_path,
        concat!(
            "{\n",
            "  \"prices\": {\n",
            "    \"rm2018-none\": {\"input\": 10.0, \"output\": 30.0},\n",
            "    \"rm2018-zero\": {\"input\": 10.0, \"output\": 30.0, \"output_reasoning\": 0.0},\n",
            "    \"rm2018-published\": {\"input\": 10.0, \"output\": 30.0, \"output_reasoning\": 50.0}\n",
            "  }\n",
            "}\n",
        ),
    )
    .expect("write override file");
    // Seed the process catalog from the override BEFORE any lookup.
    std::env::set_var("AGENTTRACE_PRICING_FILE", &override_path);

    // --- ingest: absent stays absent, zero stays zero -------------
    let none = lookup_price("rm2018-none");
    assert_eq!((none.input, none.output), (10.0, 30.0));
    assert_eq!(
        none.output_reasoning, None,
        "absent reasoning rate must not default-fill at ingest"
    );
    let zero = lookup_price("rm2018-zero");
    assert_eq!(
        zero.output_reasoning,
        Some(0.0),
        "explicit 0 is a published free-thinking rate"
    );
    let published = lookup_price("rm2018-published");
    assert_eq!(published.output_reasoning, Some(50.0));

    // --- the split itself ----------------------------------------
    // 1M out with 400k reasoning inside: the span moves out.
    assert_eq!(
        published.reasoning_split(1_000_000, 400_000),
        (600_000, 400_000, Some(50.0))
    );
    // Absent row: identity split, no reasoning term.
    assert_eq!(
        none.reasoning_split(1_000_000, 400_000),
        (1_000_000, 0, None)
    );
    // Zero reasoning tokens on a published row: nothing to reprice.
    assert_eq!(
        published.reasoning_split(1_000_000, 0),
        (1_000_000, 0, None)
    );
    // Span saturates at the bucket: reasoning may exceed the reported
    // output (a vendor reporting the span but not the plain output) —
    // the plain term floors at zero, never negative.
    assert_eq!(
        published.reasoning_split(100_000, 500_000),
        (0, 100_000, Some(50.0))
    );
    // A hostile non-finite/negative stored rate reads as absent —
    // ingest filters these (convert_litellm), the split is the belt to
    // that suspenders, so a hand-crafted override cannot poison the
    // estimate either.
    let hostile: Price = serde_json::from_value(serde_json::json!({
        "input": 10.0, "output": 30.0, "output_reasoning": -1.0
    }))
    .expect("hostile row parses");
    assert_eq!(
        hostile.reasoning_split(1_000_000, 400_000),
        (1_000_000, 0, None)
    );

    // --- session-rate arm: single-model sessions ------------------
    // 1M in + 1M out (400k of it reasoning):
    //   none:      10 + 30×1.0            = 40.00 (pre-rm-2018 identical)
    //   published: 10 + 30×0.6 + 50×0.4   = 48.00
    //   zero:      10 + 30×0.6 + 0×0.4    = 28.00 (thinking is free)
    let events = |model: &str| vec![usage_event(model, 1_000_000, 1_000_000, 400_000)];

    let none_metrics = analyze(&events("rm2018-none"), "rm2018-none");
    assert_eq!(
        none_metrics.cost_estimated, 40.0,
        "absence bills the span at the plain output rate, byte-identically to pre-rm-2018"
    );
    assert!(
        !none_metrics
            .disclosure_counters
            .contains_key("reasoning_rate_applied"),
        "absence never fires the disclosure — nothing was repriced"
    );

    let published_metrics = analyze(&events("rm2018-published"), "rm2018-published");
    assert_eq!(
        published_metrics.cost_estimated, 48.0,
        "the published reasoning rate bills the span at its own rate"
    );
    assert_eq!(
        published_metrics
            .disclosure_counters
            .get("reasoning_rate_applied"),
        Some(&1),
        "applying a distinct published rate must fire the disclosure counter exactly once"
    );

    let zero_metrics = analyze(&events("rm2018-zero"), "rm2018-zero");
    assert_eq!(
        zero_metrics.cost_estimated, 28.0,
        "a published free-thinking rate bills the span as $0"
    );
    assert_eq!(
        zero_metrics
            .disclosure_counters
            .get("reasoning_rate_applied"),
        Some(&1),
        "a published $0 rate was still APPLIED (not fallen back from) — disclosed"
    );

    // A published row with zero reasoning tokens bills like absence:
    // nothing to reprice, no disclosure.
    let no_reasoning = analyze(
        &[usage_event("rm2018-published", 1_000_000, 1_000_000, 0)],
        "rm2018-published",
    );
    assert_eq!(no_reasoning.cost_estimated, 40.0);
    assert!(
        !no_reasoning
            .disclosure_counters
            .contains_key("reasoning_rate_applied"),
        "zero reasoning tokens = nothing repriced = no disclosure"
    );

    // --- per-block arm: multi-model sessions ----------------------
    // One published block (500k in + 500k out with 200k reasoning)
    // plus one absent block (500k in + 500k out with 200k reasoning):
    // only the published block's thinking reprices.
    //   published block: 5 + 30×0.3 + 50×0.2 = 24
    //   absent block:    5 + 30×0.5          = 20
    //   total: 44.00, disclosure fired once (the published block only).
    let multi = analyze(
        &[
            usage_event("rm2018-published", 500_000, 500_000, 200_000),
            usage_event("rm2018-none", 500_000, 500_000, 200_000),
        ],
        "multiple",
    );
    assert_eq!(
        multi.cost_estimated, 44.0,
        "per-block reasoning rates resolve per row"
    );
    assert_eq!(
        multi.disclosure_counters.get("reasoning_rate_applied"),
        Some(&1),
        "exactly the published-rate block fires the disclosure"
    );

    // --- leg (c), made literal: the bundled qwen-turbo golden -----
    // (independent-review F3): 1000 thinking tokens on the REAL
    // dashscope/qwen-turbo row price at the published 0.5/Mtok
    // reasoning rate = $0.0005 — NOT the plain 0.2/Mtok $0.0002 the
    // pre-rm-2018 world billed. The merged catalog serves the bundled
    // entry (the override above names only synthetic keys); the
    // 5e-07-vs-2e-07 value pair itself is pinned in
    // pricing_snapshot_hygiene.rs — this leg executes the golden
    // end-to-end through analyze's session-rate arm.
    let qwen = lookup_price("dashscope/qwen-turbo");
    assert_eq!(
        qwen.output_reasoning,
        Some(0.5),
        "the bundled qwen-turbo row publishes the distinct reasoning rate (per-Mtok)"
    );
    let thoughts = analyze(
        &[usage_event("dashscope/qwen-turbo", 0, 1000, 1000)],
        "dashscope/qwen-turbo",
    );
    assert!(
        (thoughts.cost_estimated - 0.0005).abs() < 1e-9,
        "1000 thoughts on qwen-turbo price at $0.0005 (5e-07/token), got {}",
        thoughts.cost_estimated
    );
    assert!(
        (thoughts.cost_estimated - 0.0002).abs() > 1e-9,
        "…and not the plain-output $0.0002 (2e-07/token) pre-rm-2018 pricing"
    );
    assert_eq!(
        thoughts.disclosure_counters.get("reasoning_rate_applied"),
        Some(&1),
        "the golden application fires the disclosure exactly once"
    );

    // --- suppressed counterfactual reprices WITHOUT billing ------
    // (independent-review F6a: the rm-1052 lane's rm-2018 claim, now
    // tested): a gateway-recorded block on a published-rate row still
    // gets its suppressed catalog estimate at the DISTINCT reasoning
    // rate (the vendor would bill thinking at 50, not 30): 1M in +
    // 1M out with 400k thinking = $48 suppressed, disclosed as 4800
    // cents. But a suppressed estimate is not a billed estimate —
    // reasoning_rate_applied must NOT fire, and the recorded $0
    // passthrough stays authoritative.
    let mut recorded_block = usage_event("rm2018-published", 1_000_000, 1_000_000, 400_000);
    recorded_block.recorded_cost_usd = Some(0.0);
    let gateway = analyze(&[recorded_block], "rm2018-published");
    assert_eq!(
        gateway.cost_estimated, 0.0,
        "the recorded $0 passthrough stays authoritative — no catalog reprice of the billed number"
    );
    assert_eq!(
        gateway
            .disclosure_counters
            .get("recorded_cost_suppressed_estimate_cents"),
        Some(&4800),
        "the counterfactual repriced thinking at the distinct rate: 10 + 30*0.6 + 50*0.4 = $48 -> 4800 cents"
    );
    assert_eq!(
        gateway.disclosure_counters.get("recorded_cost_zero_blocks"),
        Some(&1),
        "a recorded $0 on priced tokens is a gateway-unpriced block"
    );
    assert!(
        !gateway
            .disclosure_counters
            .contains_key("reasoning_rate_applied"),
        "a suppressed counterfactual is not a billed estimate — the reasoning-rate \
         disclosure fires on billed applications only"
    );

    // --- serialization: absence round-trips -----------------------
    let none_json = serde_json::to_value(&none).expect("serialize");
    assert!(
        none_json.get("output_reasoning").is_none(),
        "absent reasoning rate serializes as an omitted key, not 0: {none_json}"
    );
    let zero_json = serde_json::to_value(&zero).expect("serialize");
    assert_eq!(
        zero_json.get("output_reasoning"),
        Some(&serde_json::json!(0.0)),
        "a published free-thinking rate serializes as an explicit 0"
    );
    let round: Price = serde_json::from_value(none_json).expect("deserialize absent row");
    assert_eq!(
        round.output_reasoning, None,
        "absence survives the round-trip"
    );

    // --- hygiene: no scratch left behind --------------------------
    std::env::remove_var("AGENTTRACE_PRICING_FILE");
    std::fs::remove_file(&override_path).ok();
    std::fs::remove_dir(&dir).ok();
}

//! rm-1508 (run 5c144613 cycle 4): cache-rate truth at ingest.
//!
//! A catalog row that publishes no cache rate must stay `None` end to
//! end — at ingest (no zero-fill), at serialization (key omitted, not
//! `0`), and at billing: absent rates bill at the DISCLOSED fallback
//! (cache-read at the input rate, cache-write at 1.25× input), never a
//! silent `$0`. A row that publishes an explicit `0` is a published
//! `$0` and bills as `$0` with no disclosure — the two states are
//! distinct in cost, in the disclosure channel, and in the census.
//!
//! One `#[test]` for the whole contract on purpose: the pricing
//! catalog is a process-global `OnceLock` seeded from
//! `AGENTTRACE_PRICING_FILE` on first use, so the override has to be
//! in place before any lookup and must not race sibling tests.

use agenttrace_core::{analyze, cache_rate_census, lookup_price, CacheRateCensus};
use std::collections::BTreeMap;

/// A pi-shaped usage event: role `meta` (or `session_meta`) carries
/// usage and model attribution, the shape the per-block arm prices.
fn usage_event(
    model: &str,
    input: i64,
    output: i64,
    cache_w: i64,
    cache_r: i64,
) -> agenttrace_core::Event {
    let mut usage = BTreeMap::new();
    usage.insert("input_tokens".to_string(), input);
    usage.insert("output_tokens".to_string(), output);
    usage.insert("cache_creation_input_tokens".to_string(), cache_w);
    usage.insert("cache_read_input_tokens".to_string(), cache_r);
    agenttrace_core::Event {
        role: "meta".to_string(),
        usage,
        model_used: model.to_string(),
        service_tier: None,
        ..agenttrace_core::Event::default()
    }
}

#[test]
fn absent_cache_rates_bill_at_disclosed_fallback_and_stay_distinct_from_published_zero() {
    // The override catalog merges per-entry over the bundled one, so
    // three synthetic rows pin all three states without touching any
    // real model's rates. All three share input $10/M, output $30/M.
    let dir = std::env::temp_dir().join(format!("at-cache-rate-truth-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let override_path = dir.join("pricing-override.json");
    std::fs::write(
        &override_path,
        concat!(
            "{\n",
            "  \"prices\": {\n",
            "    \"rm1508-absent\": {\"input\": 10.0, \"output\": 30.0},\n",
            "    \"rm1508-zero\": {\"input\": 10.0, \"output\": 30.0, \"cw\": 0.0, \"cr\": 0.0},\n",
            "    \"rm1508-published\": {\"input\": 10.0, \"output\": 30.0, \"cw\": 12.5, \"cr\": 1.0}\n",
            "  }\n",
            "}\n",
        ),
    )
    .expect("write override file");
    // Seed the process catalog from the override BEFORE any lookup.
    std::env::set_var("AGENTTRACE_PRICING_FILE", &override_path);

    // --- ingest: absent stays absent, zero stays zero -------------
    let absent = lookup_price("rm1508-absent");
    assert_eq!((absent.input, absent.output), (10.0, 30.0));
    assert_eq!(
        (absent.cw, absent.cr),
        (None, None),
        "absent rates must not zero-fill at ingest"
    );

    let zero = lookup_price("rm1508-zero");
    assert_eq!(
        (zero.cw, zero.cr),
        (Some(0.0), Some(0.0)),
        "explicit 0 is a published $0"
    );

    let published = lookup_price("rm1508-published");
    assert_eq!((published.cw, published.cr), (Some(12.5), Some(1.0)));

    // --- billing: the fallback is disclosed, never a silent $0 ----
    // absent:  cr bills at the input rate (10.0), cw at 1.25× input
    //          (12.5) — the same cache-write premium the published
    //          row happens to carry, which makes the comparison
    //          honest: absent ≡ the vendor convention, not ≡ $0.
    assert_eq!(absent.cache_read_billed(), (10.0, true));
    assert_eq!(absent.cache_write_billed(), (12.5, true));
    assert_eq!(zero.cache_read_billed(), (0.0, false));
    assert_eq!(zero.cache_write_billed(), (0.0, false));
    assert_eq!(published.cache_read_billed(), (1.0, false));
    assert_eq!(published.cache_write_billed(), (12.5, false));

    // --- session-rate arm: single-model sessions ------------------
    // 1M in + 1M out + 100k cache-write + 200k cache-read:
    //   absent:    10 + 30 + 0.1×12.5 + 0.2×10.0 = 43.25 (fallback fired)
    //   zero:      10 + 30 + 0        + 0        = 40.00 (published $0)
    //   published: 10 + 30 + 0.1×12.5 + 0.2×1.0  = 41.45
    let events = |model: &str| vec![usage_event(model, 1_000_000, 1_000_000, 100_000, 200_000)];

    let absent_metrics = analyze(&events("rm1508-absent"), "rm1508-absent");
    assert_eq!(
        absent_metrics.cost_estimated, 43.25,
        "absent rates bill at the fallback"
    );
    assert_eq!(
        absent_metrics
            .disclosure_counters
            .get("cache_rate_fallback"),
        Some(&1),
        "the fallback must fire the disclosure counter exactly once per application"
    );

    let zero_metrics = analyze(&events("rm1508-zero"), "rm1508-zero");
    assert_eq!(
        zero_metrics.cost_estimated, 40.0,
        "a published $0 bills as $0"
    );
    assert!(
        !zero_metrics
            .disclosure_counters
            .contains_key("cache_rate_fallback"),
        "published rates never fire the fallback disclosure"
    );

    let published_metrics = analyze(&events("rm1508-published"), "rm1508-published");
    assert_eq!(published_metrics.cost_estimated, 41.45);
    assert!(
        !published_metrics
            .disclosure_counters
            .contains_key("cache_rate_fallback"),
        "published rates never fire the fallback disclosure"
    );

    // Zero cache tokens never fire the disclosure even on absent rows
    // — there is nothing being repriced.
    let no_cache = analyze(
        &[usage_event("rm1508-absent", 1_000_000, 1_000_000, 0, 0)],
        "rm1508-absent",
    );
    assert_eq!(no_cache.cost_estimated, 40.0);
    assert!(
        !no_cache
            .disclosure_counters
            .contains_key("cache_rate_fallback"),
        "zero cache tokens = nothing repriced = no disclosure"
    );

    // --- per-block arm: multi-model sessions ----------------------
    // One absent-rate block (500k in + 100k cw) + one published block
    // (500k in + 100k cw): only the absent block's cache reprices.
    //   absent block:    5 + 0.1×12.5 = 6.25
    //   published block: 5 + 0.1×12.5 = 6.25 (its own published 12.5)
    //   total: 12.50, disclosure fired once (the absent block only).
    let multi = analyze(
        &[
            usage_event("rm1508-absent", 500_000, 0, 100_000, 0),
            usage_event("rm1508-published", 500_000, 0, 100_000, 0),
        ],
        "multiple",
    );
    assert_eq!(
        multi.cost_estimated, 12.5,
        "per-block cache rates resolve per row"
    );
    assert_eq!(
        multi.disclosure_counters.get("cache_rate_fallback"),
        Some(&1),
        "exactly the absent-rate block fires the disclosure"
    );

    // --- serialization: absence round-trips -----------------------
    let absent_json = serde_json::to_value(&absent).expect("serialize");
    assert!(
        absent_json.get("cw").is_none() && absent_json.get("cr").is_none(),
        "absent rates serialize as omitted keys, not 0: {absent_json}"
    );
    let zero_json = serde_json::to_value(&zero).expect("serialize");
    assert_eq!(
        (zero_json.get("cw"), zero_json.get("cr")),
        (Some(&serde_json::json!(0.0)), Some(&serde_json::json!(0.0))),
        "published zeros serialize as explicit 0"
    );
    // And back: a re-deserialized row keeps each state.
    let round: agenttrace_core::Price =
        serde_json::from_value(absent_json).expect("deserialize absent row");
    assert_eq!(
        (round.cw, round.cr),
        (None, None),
        "absence survives the round-trip"
    );

    // --- census: the population is answerable without code -------
    // The override rows merged into the bundled catalog, so absolute
    // totals are bundle-vintage-dependent (pricing_snapshot_hygiene.rs
    // pins those). What this binary pins: the classification itself —
    // its own override row is absent on BOTH sides and must be counted
    // in the absent class, and the per-class sum invariant absent +
    // zero + nonzero == total. Post rm-176 cure (2026-10-17 refresh,
    // run 190481d0d cycle 1): update-snapshot.sh preserves upstream
    // cache-rate absence (rm-1508), so this process's absent counts =
    // hygiene's process-level counts + this binary's one override row.
    let census: CacheRateCensus = cache_rate_census();
    assert_eq!(
        census.cache_read_absent, 2227,
        "this binary must contribute exactly its own absent-rate row on top of the bundle census: {census:?}"
    );
    assert_eq!(census.cache_write_absent, 3347);
    assert_eq!(
        census.cache_read_absent + census.cache_read_zero + census.cache_read_published,
        census.models,
        "read classes must partition the catalog"
    );
    assert_eq!(
        census.cache_write_absent + census.cache_write_zero + census.cache_write_published,
        census.models,
        "write classes must partition the catalog"
    );

    // --- hygiene: no scratch left behind --------------------------
    std::env::remove_var("AGENTTRACE_PRICING_FILE");
    std::fs::remove_file(&override_path).ok();
    std::fs::remove_dir(&dir).ok();
}

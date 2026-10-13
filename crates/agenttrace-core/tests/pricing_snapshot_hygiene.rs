//! rm-903: the vendored pricing bundle must carry zero wildcard keys —
//! LiteLLM publishes `'*'` glob keys (`bedrock/*/1-month-commitment/…`,
//! 4 of them as of the 2026-10-08 refresh) whose glob semantics our
//! exact-key lookup can never honor, so `scripts/pricing/
//! update-snapshot.sh` skips them at refresh time and this pin holds
//! the line on the bundle itself: any future refresh that re-admits a
//! wildcard row fails here instead of shipping unmatchable dead
//! weight. Concrete-region commitment-tier keys (e.g. `…/1-month-
//! commitment/<model>`) deliberately stay — they are exact-matchable
//! strings and priced rows in their own right.

#[test]
fn vendored_pricing_bundle_carries_no_wildcard_keys() {
    let raw = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/pricing_snapshot.json"
    ));
    let parsed: serde_json::Value =
        serde_json::from_str(raw).expect("vendored pricing snapshot must parse");
    let offenders: Vec<&str> = parsed
        .as_object()
        .expect("pricing snapshot is a JSON object keyed by model name")
        .keys()
        .map(|key| key.as_str())
        .filter(|key| key.contains('*'))
        .collect();
    assert!(
        offenders.is_empty(),
        "vendored pricing snapshot carries wildcard keys the exact-key lookup can never hit \
         (refresh via scripts/pricing/update-snapshot.sh skips '*' keys at the mode filter): \
         {offenders:?}"
    );
}

#[test]
fn vendored_pricing_bundle_carries_the_tier_rate_families() {
    // rm-176/rm-164 (2026-10-13 refresh): the importer's tier ride-along
    // covers `*_flex`, `*_priority` and `*_ultrafast` (fast is a
    // code-side per-model multiplier — LiteLLM publishes no `*_fast`
    // per-token fields anywhere). This census holds the refresh to the
    // coverage it claims: ultrafast rows present with the known keys,
    // priority coverage in the hundreds, flex coverage not regressed.
    // The never-drop union (rm-588) makes these floors stable under
    // upstream churn: a key that leaves the live catalog is retained
    // in the bundle, so the floors only break if the importer stops
    // carrying the fields.
    let raw = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/pricing_snapshot.json"
    ));
    let parsed: serde_json::Value =
        serde_json::from_str(raw).expect("vendored pricing snapshot must parse");
    let rows = parsed
        .as_object()
        .expect("pricing snapshot is a JSON object keyed by model name");
    let carrying = |field: &str| -> usize {
        rows.values()
            .filter(|row| {
                row.get(field)
                    .and_then(serde_json::Value::as_f64)
                    .is_some_and(|rate| rate > 0.0)
            })
            .count()
    };
    let flex = carrying("input_cost_per_token_flex");
    let priority = carrying("input_cost_per_token_priority");
    let ultrafast = carrying("input_cost_per_token_ultrafast");
    assert!(
        flex >= 200,
        "flex coverage regressed: {flex} rows carry input flex rates \
         (the 2026-10-13 refresh carried 219 live rows)"
    );
    assert!(
        priority >= 200,
        "priority coverage regressed: {priority} rows carry input priority rates \
         (the 2026-10-13 refresh carried 239 live rows)"
    );
    assert!(
        ultrafast >= 6,
        "ultrafast coverage regressed: {ultrafast} rows carry input ultrafast rates \
         (the 2026-10-13 refresh carried the 6 live chat rows)"
    );
    for key in [
        "gpt-6-astra",
        "gpt-6.1-sol",
        "global.openai.gpt-6-astra",
        "global.openai.gpt-6.1-sol",
        "us.openai.gpt-6-astra",
        "us.openai.gpt-6.1-sol",
    ] {
        assert!(
            rows.get(key)
                .and_then(|row| row.get("input_cost_per_token_ultrafast"))
                .and_then(serde_json::Value::as_f64)
                .is_some_and(|rate| rate > 0.0),
            "the known ultrafast key {key:?} lost its ultrafast rates"
        );
    }
}

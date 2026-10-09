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

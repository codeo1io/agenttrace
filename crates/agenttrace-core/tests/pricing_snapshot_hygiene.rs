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

/// Review fix F6 (2026-10-13): `scripts/pricing/drift-check.sh` gates
/// on `COST_FIELDS`, `scripts/pricing/update-snapshot.sh` prunes on
/// `TIER_FIELDS`, and the two scripts' comments say "keep the two
/// lists identical" — but nothing ever checked it. This pin parses
/// both scripts and fails when the lists drift apart, so a future
/// field added to one script and forgotten in the other shows up here
/// instead of as a silently wrong drift verdict (the retired-arm
/// versus preserve-arm mismatch class the review flagged).
fn quoted_strings_in_block(script: &str, marker: &str, stop_contains: &str) -> Vec<String> {
    let start = script
        .find(marker)
        .unwrap_or_else(|| panic!("marker {marker:?} not found in script"));
    let mut out = Vec::new();
    for line in script[start..].lines() {
        let stop = line.contains(stop_contains);
        let scan = if stop {
            &line[..line.find(stop_contains).unwrap()]
        } else {
            line
        };
        let mut rest = scan;
        while let Some(open) = rest.find('"') {
            let after = &rest[open + 1..];
            let Some(end) = after.find('"') else {
                break;
            };
            out.push(after[..end].to_string());
            rest = &after[end + 1..];
        }
        if stop {
            break;
        }
    }
    out
}

#[test]
fn drift_gate_field_lists_stay_identical_between_scripts() {
    let drift_check = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/pricing/drift-check.sh"
    ))
    .expect("scripts/pricing/drift-check.sh is part of the repo tree");
    let update_snapshot = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/pricing/update-snapshot.sh"
    ))
    .expect("scripts/pricing/update-snapshot.sh is part of the repo tree");

    let cost_fields = quoted_strings_in_block(&drift_check, "COST_FIELDS = [", "]");
    assert!(
        !cost_fields.is_empty(),
        "COST_FIELDS must parse to a non-empty list"
    );

    // TIER_FIELDS is a tuple comprehension — base fields x tier
    // suffixes — parsed from the block anchored at its assignment so
    // an unrelated earlier tuple can never match.
    let tier_block_start = update_snapshot
        .find("TIER_FIELDS = tuple(")
        .expect("TIER_FIELDS assignment present");
    let tier_block = &update_snapshot[tier_block_start..];
    let bases = quoted_strings_in_block(tier_block, "for base in (", ")");
    let suffixes = quoted_strings_in_block(tier_block, "for suffix in (", ")");
    assert!(!bases.is_empty() && !suffixes.is_empty());

    // COST_FIELDS is the base fields PLUS every tier variant, so the
    // expected set is bases UNION (bases x suffixes).
    let mut expected: std::collections::BTreeSet<String> = bases.clone().into_iter().collect();
    expected.extend(
        bases
            .iter()
            .flat_map(|base| suffixes.iter().map(move |suffix| format!("{base}{suffix}"))),
    );

    assert_eq!(
        cost_fields
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        expected,
        "drift-check.sh COST_FIELDS and update-snapshot.sh TIER_FIELDS must stay \
         identical: the drift gate compares exactly the fields the refresh prunes"
    );
}

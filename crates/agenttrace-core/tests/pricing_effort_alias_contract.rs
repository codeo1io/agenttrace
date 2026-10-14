//! rm-1511 contract: reasoning-effort-suffixed model ids (the class
//! Antigravity CLI emits — `claude-sonnet-5-5-medium`/-low) price at their
//! in-catalog base instead of the builtin Claude default, the effort tier
//! stays derivable from the raw id, and the fallback disclosure names models
//! whose exact base was available instead of only counting them.
//!
//! Recorded pre-fix behavior (research PoC, HEAD 8bc0587): the suffixed id
//! billed $0.0105 at 200k-ctx builtin-default rates for volumes the exact
//! base prices at $0.007 with 1M context, while only the aggregate
//! `fallback_pricing` counter moved and `cost_basis` rendered "priced".

use std::io::Write;

use agenttrace_core::{
    data_health, effort_tier_of, has_specific_price, lookup_price, parse_raw_session,
    specific_base_for_unpriced,
};

fn rates(model: &str) -> (f64, f64) {
    let price = lookup_price(model);
    (price.input, price.output)
}

fn write_fixture(dir: &std::path::Path, name: &str, model: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(
        file,
        r#"{{"type":"assistant","message":{{"id":"m1","model":"{model}","usage":{{"input_tokens":1000,"output_tokens":500}}}},"timestamp":"2026-10-14T10:00:00Z"}}"#
    )
    .unwrap();
    path
}

fn parse_fixture(dir: &std::path::Path, name: &str, model: &str) -> agenttrace_core::Session {
    let path = write_fixture(dir, name, model);
    let raw = std::fs::read_to_string(&path).unwrap();
    parse_raw_session(name, path.to_str().unwrap(), &raw).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn effort_suffixed_ids_price_at_their_in_catalog_base() {
    let base = rates("claude-sonnet-5-5");
    assert!(base.0 > 0.0, "base key must be in the bundled catalog");
    for model in [
        "claude-sonnet-5-5-low",
        "claude-sonnet-5-5-medium",
        "claude-sonnet-5-5-high",
        "claude-sonnet-5-5-thinking",
        "claude-sonnet-5-5:thinking",
    ] {
        assert_eq!(rates(model), base, "expected base rates for {model}");
    }
    // Case-insensitivity rides normalize_model like every other arm.
    assert_eq!(rates("Claude-Sonnet-5-5-Medium"), base);
}

#[test]
fn effort_suffix_arm_depends_on_no_suffixed_catalog_keys() {
    // The alias arm resolves the base; if a suffixed key ever lands in the
    // catalog (exact match wins first), this pin forces a conscious update.
    let snapshot: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("../src/pricing_snapshot.json"))
            .expect("bundled snapshot parses");
    for key in [
        "claude-sonnet-5-5-low",
        "claude-sonnet-5-5-medium",
        "claude-sonnet-5-5-high",
        "claude-sonnet-5-5-thinking",
    ] {
        assert!(
            !snapshot.contains_key(key),
            "suffixed key {key} is now catalogued; update this contract"
        );
    }
}

#[test]
fn effort_tier_stays_derivable_after_resolution() {
    assert_eq!(effort_tier_of("claude-sonnet-5-5-low"), Some("low"));
    assert_eq!(effort_tier_of("claude-sonnet-5-5-medium"), Some("medium"));
    assert_eq!(effort_tier_of("claude-sonnet-5-5-high"), Some("high"));
    assert_eq!(
        effort_tier_of("claude-sonnet-5-5-thinking"),
        Some("thinking")
    );
    assert_eq!(
        effort_tier_of("claude-sonnet-5-5:thinking"),
        Some("thinking")
    );
    assert_eq!(effort_tier_of("claude-sonnet-5-5"), None);
    assert_eq!(effort_tier_of("gpt-5.6"), None);
}

#[test]
fn known_effort_suffix_makes_the_price_specific() {
    // The aggregate fallback counter drops: the session now bills at an
    // exact catalog row instead of the builtin default.
    assert!(has_specific_price("claude-sonnet-5-5-medium"));
    assert!(has_specific_price("claude-opus-5-5:thinking"));
    // And the disclosure helper no longer flags it — it is priced.
    assert_eq!(specific_base_for_unpriced("claude-sonnet-5-5-medium"), None);
}

#[test]
fn unknown_model_with_unknown_suffix_stays_at_default() {
    let default = rates("totally-unknown-model");
    assert_eq!(default, rates("totally-unknown-model-medium"));
    assert!(!has_specific_price("totally-unknown-model-medium"));
    assert_eq!(
        specific_base_for_unpriced("totally-unknown-model-medium"),
        None
    );
}

#[test]
fn disclosure_names_unrecognized_suffixes_with_available_base() {
    // `-ultra` is not in the effort table: the session still prices at the
    // default, but the exact base sits in the catalog — the rm-1511
    // disclosure must say so instead of only counting fallback sessions.
    let model = "claude-sonnet-5-5-ultra";
    assert!(!has_specific_price(model));
    assert_eq!(
        specific_base_for_unpriced(model),
        Some("claude-sonnet-5-5".to_string())
    );
    // A priced model never names a base, whatever its tail.
    assert_eq!(specific_base_for_unpriced("claude-sonnet-5-5"), None);
    // A tail that is not a plain short token stays silent: date-suffixed
    // style ids (`-2026`) are their own model family, not a hidden base.
    assert_eq!(specific_base_for_unpriced("claude-sonnet-5-5-2026"), None);
    assert_eq!(specific_base_for_unpriced("claude-sonnet-5-5-x9"), None);
    // And the short-token rule is bounded: any ≤10-char alpha tail on a
    // priced base discloses (`-ultra`), a longer one does not.
    assert_eq!(
        specific_base_for_unpriced("claude-sonnet-5-5-abcdefghij"),
        Some("claude-sonnet-5-5".to_string())
    );
    assert_eq!(
        specific_base_for_unpriced("claude-sonnet-5-5-abcdefghijk"),
        None
    );
}

#[test]
fn data_health_names_models_with_available_base() {
    let dir = std::env::temp_dir().join("pricing_effort_alias_contract");
    std::fs::create_dir_all(&dir).unwrap();
    let session = parse_fixture(&dir, "ultra.jsonl", "claude-sonnet-5-5-ultra");
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(health.fallback_pricing, 1);
    assert_eq!(
        health.fallback_with_base_available,
        vec!["claude-sonnet-5-5".to_string()]
    );
    // And the priced suffix class no longer appears anywhere in the lane.
    let session = parse_fixture(&dir, "medium2.jsonl", "claude-sonnet-5-5-medium");
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(health.fallback_pricing, 0);
    assert!(health.fallback_with_base_available.is_empty());
}

#[test]
fn recorded_poc_flips_from_default_to_base_rates() {
    // Pre-fix (research PoC at 8bc0587): $0.0105 — the builtin default
    // (3e-6/1.5e-5) applied while the exact base (2e-6/1e-5) was catalogued.
    let dir = std::env::temp_dir().join("pricing_effort_alias_contract");
    std::fs::create_dir_all(&dir).unwrap();
    let session = parse_fixture(&dir, "medium.jsonl", "claude-sonnet-5-5-medium");
    let expected = 1000.0 * 2.0e-6 + 500.0 * 1.0e-5; // $0.007 at base rates
    assert_eq!(session.metrics.cost_estimated, expected);
}

#[test]
fn poc_baseline_unaffected_by_the_alias_arm() {
    let dir = std::env::temp_dir().join("pricing_effort_alias_contract");
    std::fs::create_dir_all(&dir).unwrap();
    let session = parse_fixture(&dir, "base.jsonl", "claude-sonnet-5-5");
    let expected = 1000.0 * 2.0e-6 + 500.0 * 1.0e-5;
    assert_eq!(session.metrics.cost_estimated, expected);
}

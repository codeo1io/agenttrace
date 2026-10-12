//! rm-1059 contract lane: unmodeled cost-field classes must be
//! classified and disclosed, never silently priced as $0.
//!
//! Three arms:
//! 1. keep-list — every cost-bearing field in the bundled snapshot is
//!    either modeled or explicitly classified as unmodeled. A snapshot
//!    refresh that introduces a NEW cost field trips this test so the
//!    class is consciously handled, never silently dropped.
//! 2. classifier — raw entries carrying unmodeled classes produce the
//!    documented family set; entries without them produce none.
//! 3. disclosure plumbing — has_specific_price entries with unmodeled
//!    classes resolve their classes; DataHealth/PricingCoverage expose
//!    the session counter and keep zero-values out of serialized
//!    output (byte-stability for catalogs without such rows).

use agenttrace_core::{
    bundled_unmodeled_census, has_specific_price, unmodeled_class_families_for_entry,
    unmodeled_cost_classes, UNMODELED_COST_FAMILIES,
};
use serde_json::Value;

/// Cost-bearing field names the converter MODELS (these become rates
/// in `Price`). Mirrors the renames on `LiteLlmModel`.
const MODELED_COST_FIELDS: [&str; 8] = [
    "input_cost_per_token",
    "output_cost_per_token",
    "cache_creation_input_token_cost",
    "cache_read_input_token_cost",
    "input_cost_per_token_flex",
    "output_cost_per_token_flex",
    "cache_creation_input_token_cost_flex",
    "cache_read_input_token_cost_flex",
];

/// Cost-bearing field names recognized as UNMODELED classes (the
/// rm-1059 keep-list). A cost key outside BOTH lists is a NEW class:
/// extend this list (or model it) in the same change that refreshes
/// the snapshot — the test below reds until you do.
const KNOWN_UNMODELED_COST_FIELDS: [&str; 16] = [
    "search_context_cost_per_query",
    "input_cost_per_image_token",
    "output_cost_per_image_token",
    "cache_read_input_image_token_cost",
    "input_cost_per_audio_token",
    "output_cost_per_audio_token",
    "input_cost_per_second",
    "output_cost_per_second",
    "cost_per_second",
    "input_cost_per_token_priority",
    "output_cost_per_token_priority",
    "input_cost_per_token_ultrafast",
    "output_cost_per_token_ultrafast",
    "input_cost_per_token_fast",
    "output_cost_per_token_fast",
    "supports_fast_mode",
];

fn bundled_snapshot_json() -> Value {
    let raw = include_str!("../src/pricing_snapshot.json");
    serde_json::from_str(raw).expect("bundled pricing snapshot must parse")
}

#[test]
fn keep_list_covers_every_cost_bearing_field_in_the_bundled_snapshot() {
    let snapshot = bundled_snapshot_json();
    let mut offenders = Vec::new();
    for (model, entry) in snapshot.as_object().expect("snapshot is an object") {
        if model.starts_with('_') || !entry.is_object() {
            continue;
        }
        for field in entry.as_object().unwrap().keys() {
            let is_cost_bearing = field.contains("cost") || field.contains("price");
            if !is_cost_bearing {
                continue;
            }
            let known = MODELED_COST_FIELDS.contains(&field.as_str())
                || KNOWN_UNMODELED_COST_FIELDS.contains(&field.as_str());
            if !known {
                offenders.push(format!("{model} :: {field}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "bundled snapshot carries cost-bearing fields outside the modeled + unmodeled keep-lists \
         (rm-1059): {offenders:#?} — model them, classify them into a family, or extend the \
         keep-lists consciously; never drop them in silence"
    );
}

#[test]
fn classifier_labels_unmodeled_families_from_raw_entries() {
    let base = serde_json::json!({
        "input_cost_per_token": 2e-6,
        "output_cost_per_token": 1e-5,
        "max_input_tokens": 272000
    });
    assert!(
        unmodeled_class_families_for_entry(&base).is_empty(),
        "a plain token entry must classify as fully modeled"
    );

    let search = serde_json::json!({
        "input_cost_per_token": 1.5e-6,
        "output_cost_per_token": 7.5e-6,
        "search_context_cost_per_query": 3e-5
    });
    assert_eq!(
        unmodeled_class_families_for_entry(&search),
        vec!["search-context"]
    );

    let image = serde_json::json!({
        "input_cost_per_token": 1e-6,
        "input_cost_per_image_token": 0.003,
        "output_cost_per_image_token": 0.012,
        "cache_read_input_image_token_cost": 0.001
    });
    assert_eq!(
        unmodeled_class_families_for_entry(&image),
        vec!["image-token"]
    );

    let tiers = serde_json::json!({
        "input_cost_per_token": 2e-6,
        "output_cost_per_token": 1e-5,
        "input_cost_per_token_flex": 1e-6,
        "input_cost_per_token_priority": 4e-6,
        "output_cost_per_token_ultrafast": 6e-5
    });
    assert_eq!(
        unmodeled_class_families_for_entry(&tiers),
        vec!["tier-variants"],
        "flex is modeled — only the non-flex tier arms classify"
    );

    let everything = serde_json::json!({
        "input_cost_per_token": 1e-6,
        "search_context_cost_per_query": 2.5e-5,
        "input_cost_per_audio_token": 1e-5,
        "input_cost_per_second": 0.02,
        "input_cost_per_token_priority": 2e-6
    });
    assert_eq!(
        unmodeled_class_families_for_entry(&everything),
        vec![
            "audio-token",
            "per-second",
            "search-context",
            "tier-variants"
        ],
        "families come back sorted and deduplicated"
    );

    // Zero and absent rates are "no published rate", never a class.
    let zeroed = serde_json::json!({
        "input_cost_per_token": 1e-6,
        "search_context_cost_per_query": 0.0
    });
    assert!(unmodeled_class_families_for_entry(&zeroed).is_empty());
}

#[test]
fn bundled_snapshot_census_is_offline_and_family_named() {
    // The bundled snapshot today carries none of these classes
    // (census 2026-10-08: search-context 0, image-token 0, audio 0,
    // per-second 0, tier 0), so the doctor suffix stays empty — this
    // pins the OFFLINE truth source without touching HOME/XDG state.
    let census = bundled_unmodeled_census();
    for (family, count) in &census {
        assert!(
            UNMODELED_COST_FAMILIES.contains(family),
            "census produced an unknown family {family:?}"
        );
        assert!(*count > 0, "census must not carry empty families");
    }
}

#[test]
fn unmodeled_cost_classes_resolve_like_a_price_lookup() {
    // A model with a specific price (bundled snapshot row) resolves;
    // the bundled snapshot carries no unmodeled classes today, so the
    // resolution must be EMPTY rather than an error — while fallback
    // models stay empty by doctrine (they disclose via
    // fallback_pricing instead).
    let priced = "claude-sonnet-4-20250514";
    assert!(has_specific_price(priced));
    assert!(
        unmodeled_cost_classes(priced).is_empty(),
        "bundled-only catalog rows classify as fully modeled"
    );
    assert!(unmodeled_cost_classes("").is_empty());
    assert!(unmodeled_cost_classes("unknown").is_empty());
    assert!(unmodeled_cost_classes("totally-not-a-model").is_empty());
}

#[test]
fn coverage_counters_serialize_only_when_nonzero() {
    // Zero values must stay out of serialized output so corpora
    // without unmodeled-class sessions keep byte-identical JSON.
    let coverage = agenttrace_core::PricingCoverage::default();
    assert_eq!(coverage.unmodeled_cost_class_sessions, 0);
    let serialized = serde_json::to_value(&coverage).expect("serialize");
    assert!(
        serialized.get("unmodeled_cost_class_sessions").is_none(),
        "zero counter must be skipped: {serialized}"
    );

    let health = agenttrace_core::DataHealth::default();
    assert_eq!(health.unmodeled_cost_class_sessions, 0);
    let serialized = serde_json::to_value(&health).expect("serialize");
    assert!(
        serialized.get("unmodeled_cost_class_sessions").is_none(),
        "zero data-health counter must be skipped: {serialized}"
    );
}

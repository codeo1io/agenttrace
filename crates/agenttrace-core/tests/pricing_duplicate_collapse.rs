//! rm-1419 contract: the bundled LiteLLM snapshot contains families of
//! raw spellings that normalize onto ONE catalog key (all regional
//! twins of `claude-sonnet-4-5-20250929-v1:0` collapse onto
//! `claude-sonnet-4-5-20250929-v1`, because `normalize_model` strips
//! provider prefixes and the `:0` suffix). The loader resolves each
//! family by an explicit rule — provider priority first, then the
//! SHORTEST raw spelling, then lexicographic order — records the
//! resolution in a disclosure ledger, and every losing spelling keeps
//! its own exact-match entry which `lookup_price` tries BEFORE any
//! alias/normalization redirect.
//!
//! Red-first history: `us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0`
//! billed $4.95/M-in (the lexicographically-first twin `au.anthropic…`,
//! 3.3/16.5) instead of its own row (3.6/18.0 → $5.40) at 794b6e5;
//! `resolve_alias` normalizing before matching additionally made raw
//! exact entries unreachable from the public lookup API.

use agenttrace_core::{
    bundled_snapshot_deprecated, bundled_snapshot_duplicate_collapses, lookup_price,
};

/// Rates are per-million; the snapshot stores per-token floats, so
/// compare with a tolerance instead of exact equality.
fn assert_price(model: &str, want_in: f64, want_out: f64) {
    let price = lookup_price(model);
    assert!(
        (price.input - want_in).abs() < 1e-9 && (price.output - want_out).abs() < 1e-9,
        "{model}: input {} output {} (wanted {want_in}/{want_out})",
        price.input,
        price.output
    );
}

#[test]
fn regional_spellings_bill_at_their_own_rows() {
    // The collapse family of `claude-sonnet-4-5-20250929-v1`: every
    // dotted regional twin carries its own exact row and raw-first
    // lookup bills at it.
    assert_price(
        "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0",
        3.6,
        18.0,
    );
    assert_price("au.anthropic.claude-sonnet-4-5-20250929-v1:0", 3.3, 16.5);
    assert_price("us.anthropic.claude-sonnet-4-5-20250929-v1:0", 3.3, 16.5);
    assert_price("eu.anthropic.claude-sonnet-4-5-20250929-v1:0", 3.3, 16.5);
    assert_price("jp.anthropic.claude-sonnet-4-5-20250929-v1:0", 3.3, 16.5);
    assert_price(
        "global.anthropic.claude-sonnet-4-5-20250929-v1:0",
        3.0,
        15.0,
    );
}

#[test]
fn base_keys_resolve_by_explicit_rule_not_source_order() {
    // The unprefixed spelling is the shortest raw in its family, so it
    // owns the base key (bedrock 3.0/15.0) instead of the
    // lexicographically-first regional twin.
    assert_price("claude-sonnet-4-5-20250929-v1:0", 3.0, 15.0);
    // The date-only spelling strips to `claude-sonnet-4-5`, whose only
    // family member is the anthropic row (priority 10).
    assert_price("claude-sonnet-4-5-20250929", 3.0, 15.0);
    assert_price("claude-sonnet-4-5", 3.0, 15.0);
}

#[test]
fn slash_spellings_keep_their_tiered_resolution() {
    // The bedrock slash-spelling arm resolves by region already; the
    // exact-entry additions must not shadow it.
    assert_price(
        "bedrock/us-gov-east-1/claude-sonnet-4-5-20250929-v1:0",
        3.6,
        18.0,
    );
    assert_price(
        "bedrock/us-gov-west-1/anthropic.claude-sonnet-4-5-20250929-v1:0",
        3.6,
        18.0,
    );
}

#[test]
fn collapse_ledger_is_disclosed_and_deterministic() {
    let (resolved, families, ledger) = bundled_snapshot_duplicate_collapses();
    assert!(resolved > 0, "bundled snapshot must disclose collapses");
    assert!(families > 0);
    assert_eq!(ledger.len(), families);

    // The contract family: normalized key without the `:0` suffix,
    // winner (shortest raw spelling) first.
    let family = ledger
        .get("claude-sonnet-4-5-20250929-v1")
        .expect("the sonnet-4-5 regional family must be in the ledger");
    assert_eq!(
        family.first().map(String::as_str),
        Some("claude-sonnet-4-5-20250929-v1:0"),
        "shortest raw spelling owns the base key"
    );
    assert!(family
        .iter()
        .any(|k| k == "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0"));
    assert!(family
        .iter()
        .any(|k| k == "au.anthropic.claude-sonnet-4-5-20250929-v1:0"));

    // Repeat calls must return the identical ledger (determinism).
    let (resolved2, families2, ledger2) = bundled_snapshot_duplicate_collapses();
    assert_eq!((resolved, families), (resolved2, families2));
    assert!(std::ptr::eq(ledger, ledger2));
}

#[test]
fn deprecation_census_counts_distinct_models() {
    // rm-1419 adds exact-match entries for losing spellings; the
    // deprecation census counts distinct MODELS — one per collapse
    // family, the winner's row at the family key (a fixed-point
    // loser shares that key and adds no row; review F1/F2). The
    // explicit precedence rule (priority, then shortest raw
    // spelling, then lexicographic) re-picks some family winners
    // versus the old first-sorted-wins load, moving the bundled
    // census from 61 of 1529 (goldens at 794b6e5) to 65 of 1529 —
    // the svg card goldens were regenerated with
    // UPDATE_CARD_GOLDEN=1 in the same batch.
    let (count, _oldest) = bundled_snapshot_deprecated();
    assert_eq!(
        count, 65,
        "deprecation census must stay at the distinct-model count"
    );
}

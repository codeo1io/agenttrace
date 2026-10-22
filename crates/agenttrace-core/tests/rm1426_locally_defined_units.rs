//! rm-1426 unit contract: the locally-defined predicate goldens and the
//! analyze-lane boundaries (recorded-cost stays in rm-1052's lane; missing
//! identity is a different disclosure).
//!
//! rm-1954 composition (wall, post-base): every arm below is a catalog-lane
//! census assertion — `is_locally_defined` judges the RAW keys of the ACTIVE
//! catalog origin, so a machine pricing cache at the default location flips
//! the goldens in BOTH directions (a cache naming bare `glm-5.2` makes it
//! “named”; a cache missing `zai/glm-5.2` makes that control
//! locally-defined). Each body runs inside `support::with_isolated_pricing_env`
//! (fresh scratch session cache, scratch HOME, `AGENTTRACE_PRICING_FILE`
//! cleared), making the goldens deterministic battery truth.

mod support;

use agenttrace_core::{analyze, is_locally_defined, Event};

#[test]
fn predicate_goldens() {
    support::with_isolated_pricing_env(predicate_goldens_body);
}

fn predicate_goldens_body() {
    // The locally-defined class, arm (b): shares a collapsed name with
    // public carrier rows, so it prices at a foreign carrier's rate.
    assert!(is_locally_defined("glm-5.2"));
    // Arm (a): nothing anywhere backs this slug; builtin default answers.
    assert!(is_locally_defined("glm-9.9-not-a-sku"));
    // Arm (a) as RECORDED WITH a carrier prefix (the private-proxy
    // recording class): no alias redirects it and no raw key names it,
    // so it must stay locally-defined. Pins the rm-1053 composition —
    // the wall's `resolve_alias_detail` returns the un-redirected input
    // for alias-less slugs, which defeats a string-comparison baseline
    // (re-proven red by the cache-catalog arm before the fix).
    assert!(is_locally_defined("myco/gpt-9-private"));
    // Missing identity is NOT the class (a different disclosure owns it).
    assert!(!is_locally_defined(""));
    assert!(!is_locally_defined("default"));
    assert!(!is_locally_defined("unknown"));
    // Rows that NAME the model: exact published keys...
    assert!(!is_locally_defined("claude-sonnet-4-5"));
    assert!(!is_locally_defined("gpt-5.2-codex"));
    assert!(!is_locally_defined("zai/glm-5.2"));
    // ...and the curated builtin table.
    assert!(!is_locally_defined("glm-5"));
}

#[test]
fn recorded_cost_blocks_stay_in_rm1052_lane() {
    support::with_isolated_pricing_env(recorded_cost_blocks_stay_in_rm1052_lane_body);
}

fn recorded_cost_blocks_stay_in_rm1052_lane_body() {
    let mut e = Event {
        model_used: "glm-5.2".to_string(),
        recorded_cost_usd: Some(0.01),
        ..Event::default()
    };
    e.usage.insert("input_tokens".to_string(), 1_000_000);
    e.usage.insert("output_tokens".to_string(), 1_000_000);
    let m = analyze(&[e], "glm-5.2");
    assert!(
        !m.disclosure_counters
            .contains_key("locally_defined_model_pricing:glm-5.2"),
        "recorded-cost usage is disclosed by the rm-1052 family, not here"
    );
    assert!(!m
        .disclosure_counters
        .contains_key("locally_defined_priced_cents"));
}

/// rm-1426 per-block arm: a MULTI-model session (pi model_change shape,
/// rm-438) prices each usage block at its own model — the locally-defined
/// census must attribute fabricated dollars per block, not just for the
/// single-model catalog formula.
#[test]
fn multi_model_session_prices_each_locally_defined_block() {
    support::with_isolated_pricing_env(multi_model_session_prices_each_locally_defined_block_body);
}

fn multi_model_session_prices_each_locally_defined_block_body() {
    let mut local = Event {
        model_used: "glm-9.9-not-a-sku".to_string(),
        role: "session_meta".to_string(),
        ..Event::default()
    };
    local.usage.insert("input_tokens".to_string(), 1_000_000);
    local.usage.insert("output_tokens".to_string(), 1_000_000);
    let mut priced = Event {
        model_used: "claude-sonnet-4-5".to_string(),
        role: "session_meta".to_string(),
        ..Event::default()
    };
    priced.usage.insert("input_tokens".to_string(), 1_000);
    priced.usage.insert("output_tokens".to_string(), 1_000);

    let m = analyze(&[local, priced], "glm-9.9-not-a-sku");
    assert_eq!(
        m.disclosure_counters
            .get("locally_defined_model_pricing:glm-9.9-not-a-sku"),
        Some(&1),
        "the locally-defined BLOCK is counted per slug even in a mixed session"
    );
    // 1M in at $3.00/Mtok + 1M out at $15.00/Mtok = exactly $18.00 of
    // fabricated-default dollars for that one block; the priced block
    // contributes nothing.
    assert_eq!(
        m.disclosure_counters.get("locally_defined_priced_cents"),
        Some(&1800),
        "per-block fabricated dollars; got {:?}",
        m.disclosure_counters
    );
}

#[test]
fn unpriced_usage_without_identity_is_not_locally_defined() {
    support::with_isolated_pricing_env(unpriced_usage_without_identity_is_not_locally_defined_body);
}

fn unpriced_usage_without_identity_is_not_locally_defined_body() {
    let mut e = Event::default();
    e.usage.insert("input_tokens".to_string(), 1_000);
    let m = analyze(&[e], "unknown");
    assert!(!m
        .disclosure_counters
        .contains_key("locally_defined_model_pricing:unknown"));
    assert!(!m
        .disclosure_counters
        .contains_key("locally_defined_priced_cents"));
}

/// Composition with the naming lanes the wall landed after this run's
/// base (independent review, conflict case bcdd1a88): a slug priced
/// through a row that names a real published id of the same model is NOT
/// locally-defined — its dollars are the model's own published rate, not
/// a fabricated one. Without this arm the merge flagged codex 0.163's
/// DEFAULT model set (rm-1570's hand-pinned daybreak rows) as fabricated
/// pricing on every session, contradicting the landed money-truth pins.
#[test]
fn post_base_naming_lanes_are_not_locally_defined() {
    support::with_isolated_pricing_env(post_base_naming_lanes_are_not_locally_defined_body);
}

fn post_base_naming_lanes_are_not_locally_defined_body() {
    // rm-610 / rm-1570 code-side vend rows: exact own-id rates, kept out
    // of the raw envelope by LiteLLM's chat-mode filter.
    assert!(!is_locally_defined("gpt-daybreak-red-latest"));
    assert!(!is_locally_defined("gpt-daybreak-blue-latest"));
    assert!(!is_locally_defined("gpt-5.3-codex"));
    // rm-1532 embedded models.dev extract: a first-party row for an
    // exact dated id the LiteLLM envelope does not carry (the extract
    // keys the id after its provider prefix — the bare dated form is
    // the id it names). The prefixed recording is deliberately NOT
    // extract-named and stays in arm (b): a carrier collapse serving
    // `claude-3-7-sonnet` is not this deployment's own row.
    assert!(!is_locally_defined("claude-3-7-sonnet-20250219"));
    // rm-1511 ladder variants: an effort-tier suffix bills at the base
    // model's own raw row — the base's published rate, not a borrowed
    // carrier collapse.
    assert!(!is_locally_defined("claude-sonnet-5-5-medium"));
    // Arm (b) keeps its disclosure: only foreign CARRIER rows collapsed
    // onto the bare name match, and none of the new lanes names the
    // deployment's own slug.
    assert!(is_locally_defined("glm-5.2"));
    assert!(is_locally_defined("grok-4"));
    // Arm (a) keeps its disclosure.
    assert!(is_locally_defined("glm-9.9-not-a-sku"));
}

/// The rm-1574 basis label mirrors governance's cost-audit
/// classification (its own doc contract), so a locally-defined slug —
/// which the census counts as `fallback_estimate` — must render
/// `fallback` on the JSON/CSV/TUI basis lanes too, never `exact`.
#[test]
fn pricing_basis_label_agrees_with_the_locally_defined_census() {
    support::with_isolated_pricing_env(
        pricing_basis_label_agrees_with_the_locally_defined_census_body,
    );
}

fn pricing_basis_label_agrees_with_the_locally_defined_census_body() {
    use agenttrace_core::pricing_basis_label;
    assert_eq!(pricing_basis_label("glm-5.2"), Some("fallback"));
    assert_eq!(pricing_basis_label("glm-9.9-not-a-sku"), Some("fallback"));
    // Named rows stay exact on every lane.
    assert_eq!(pricing_basis_label("claude-sonnet-4-5"), None);
    assert_eq!(pricing_basis_label("gpt-daybreak-red-latest"), None);
    assert_eq!(pricing_basis_label("claude-sonnet-5-5-medium"), None);
    assert_eq!(pricing_basis_label("default"), Some("unknown"));
}

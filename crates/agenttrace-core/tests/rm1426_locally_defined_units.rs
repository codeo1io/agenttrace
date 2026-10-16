//! rm-1426 unit contract: the locally-defined predicate goldens and the
//! analyze-lane boundaries (recorded-cost stays in rm-1052's lane; missing
//! identity is a different disclosure).

use agenttrace_core::{analyze, is_locally_defined, Event};

#[test]
fn predicate_goldens() {
    // The locally-defined class, arm (b): shares a collapsed name with
    // public carrier rows, so it prices at a foreign carrier's rate.
    assert!(is_locally_defined("glm-5.2"));
    // Arm (a): nothing anywhere backs this slug; builtin default answers.
    assert!(is_locally_defined("glm-9.9-not-a-sku"));
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

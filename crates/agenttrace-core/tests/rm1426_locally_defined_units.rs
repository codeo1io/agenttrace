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

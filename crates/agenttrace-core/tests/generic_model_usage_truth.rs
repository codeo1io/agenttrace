//! rm-616 contract: generic-lane model/usage truth. Foreign sessions
//! that reach the serde passthrough lanes (the parser.rs `Vec<Event>`
//! arm and the `parse_jsonl_session` per-line fallback) must report
//! the model identity and usage their conversation lines actually
//! carry, instead of silently dropping both to text estimation. The
//! contract pins three things:
//!
//! 1. `Event.model_used` accepts the snake_case wire spelling beside
//!    the PascalCase rename (casing parity with the sibling fields
//!    that already carry aliases).
//! 2. Reported usage on generic-classified conversation lines folds
//!    into the session totals and suppresses text estimation, scoped
//!    to `source_tool == "generic"` so native families (whose usage
//!    rides synthetic meta events) cannot double-count.
//! 3. What is NOT counted is disclosed: a rejected line carrying
//!    model/usage keys increments `model_or_usage_dropped`, and a
//!    conversation line whose usage falls outside the counted lanes
//!    while the session estimates increments `usage_present_not_counted`
//!    (both counters riding the pass-7 `line_skips` channel).
//!
//! Red-first fixtures c2.json / c-generic.json are the PoC corpus from
//! the adversarial assessment (pre-fix: model 'default', tokens 1/1
//! and 2/2 against journal truth gpt-5 {7,3}) preserved verbatim.

use agenttrace_core::parse_file;
use std::fs;
use std::path::PathBuf;

const C2: &str = include_str!("fixtures/rm-616-generic/c2.json");
const C_GENERIC: &str = include_str!("fixtures/rm-616-generic/c-generic.json");

fn write(tag: &str, raw: &str) -> PathBuf {
    // Review fix F4: pid-qualify and let the caller remove, so two
    // overlapping invocations of the crate's tests never race on the
    // same scratch paths (fleet precedent 04b1f7f7 U2).
    let path = std::env::temp_dir().join(format!("at-rm616-{}-{tag}.json", std::process::id()));
    fs::write(&path, raw).expect("write fixture");
    path
}

fn session(tag: &str, raw: &str) -> agenttrace_core::Session {
    let path = write(tag, raw);
    let session = parse_file(&path).expect("session parses");
    let _ = fs::remove_file(&path);
    session
}

#[test]
fn vec_lane_reports_true_model_and_usage() {
    let session = session("c2", C2);
    assert_eq!(session.metrics.model_used, "gpt-5");
    assert_eq!(session.metrics.tokens_input, 7);
    assert_eq!(session.metrics.tokens_output, 3);
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(session.metrics.zero_usage_events, 0);
}

#[test]
fn vec_lane_generic_fixture_reports_true_model_and_usage() {
    let session = session("c-generic", C_GENERIC);
    assert_eq!(session.metrics.model_used, "gpt-5");
    assert_eq!(session.metrics.tokens_input, 7);
    assert_eq!(session.metrics.tokens_output, 3);
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(session.metrics.source_tool, "generic");
}

#[test]
fn per_line_fallback_reports_true_model_and_usage() {
    let raw = concat!(
        "{\"role\":\"user\",\"content\":\"hi\"}\n",
        "{\"role\":\"assistant\",\"content\":\"yo\",\"model_used\":\"gpt-5\",",
        "\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}\n",
    );
    let session = session("per-line", raw);
    assert_eq!(session.metrics.model_used, "gpt-5");
    assert_eq!(session.metrics.tokens_input, 7);
    assert_eq!(session.metrics.tokens_output, 3);
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(session.metrics.source_tool, "generic");
}

#[test]
fn casing_parity_both_spellings_populate_model_used() {
    let pascal = session(
        "pascal",
        concat!(
            "[{\"role\":\"assistant\",\"content\":\"x\",\"ModelUsed\":\"pascal-model\",",
            "\"usage\":{\"input_tokens\":2,\"output_tokens\":1}}]"
        ),
    );
    assert_eq!(pascal.metrics.model_used, "pascal-model");
    assert_eq!(pascal.metrics.tokens_input, 2);
    assert_eq!(pascal.metrics.tokens_output, 1);

    let snake = session(
        "snake",
        concat!(
            "[{\"role\":\"assistant\",\"content\":\"x\",\"model_used\":\"snake-model\",",
            "\"usage\":{\"input_tokens\":2,\"output_tokens\":1}}]"
        ),
    );
    assert_eq!(snake.metrics.model_used, "snake-model");
    assert_eq!(snake.metrics.tokens_input, 2);
    assert_eq!(snake.metrics.tokens_output, 1);
}

#[test]
fn native_shaped_session_keeps_meta_gate_and_discloses_unfolded_usage() {
    // Hermes-shaped lines (user/assistant + timestamp) classify as
    // hermes_jsonl, not generic: that family's counted usage lane is
    // the meta arm, so conversation-line usage must NOT fold, text
    // estimation stands, and the unfolded usage is disclosed instead
    // of silently ignored. Model identity is lane-independent.
    let raw = concat!(
        "{\"role\":\"user\",\"content\":\"hi\",\"timestamp\":\"2026-10-04T01:00:00Z\"}\n",
        "{\"role\":\"assistant\",\"content\":\"yo\",\"timestamp\":\"2026-10-04T01:00:01Z\",",
        "\"model_used\":\"gpt-5\",\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}\n",
    );
    let session = session("native-shaped", raw);
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(session.metrics.model_used, "gpt-5");
    assert_eq!(session.metrics.provenance.tokens, "estimated_from_text");
    assert_ne!(session.metrics.tokens_input, 7);
    assert_ne!(session.metrics.tokens_output, 3);
    assert_eq!(
        session.metrics.line_skips.get("usage_present_not_counted"),
        Some(&1)
    );
}

#[test]
fn rejected_lines_carrying_accounting_are_disclosed() {
    let raw = concat!(
        // one healthy line so the session exists
        "{\"role\":\"user\",\"content\":\"keep\"}\n",
        // schema-broken line carrying model + usage
        "{\"role\":\"assistant\",\"content\":\"x\",\"model_used\":\"gpt-5\",",
        "\"usage\":{\"input_tokens\":9,\"output_tokens\":9},\"is_error\":\"not-a-bool\"}\n",
        // parses as an Event but is not a conversation line (no role, no type)
        "{\"model_used\":\"gpt-5\",\"usage\":{\"input_tokens\":4,\"output_tokens\":4}}\n",
        // unparseable line that visibly carries the accounting keys
        "truncated journal line mentioning model_used and usage keys\n",
    );
    let session = session("rejected", raw);
    assert_eq!(session.metrics.user_messages, 1);
    let skips = &session.metrics.line_skips;
    assert_eq!(skips.get("event_schema"), Some(&1));
    assert_eq!(skips.get("non_event"), Some(&1));
    assert_eq!(skips.get("unparseable_line"), Some(&1));
    assert_eq!(skips.get("model_or_usage_dropped"), Some(&3));
}

#[test]
fn parse_side_and_analyze_side_disclosures_merge() {
    // The hermes-shaped line makes analyze() emit
    // usage_present_not_counted; the broken line makes
    // parse_jsonl_session emit event_schema + model_or_usage_dropped.
    // Both must survive on one session (pre-rm-616 the parse-side map
    // overwrote whatever analyze() had put in metrics.line_skips).
    let raw = concat!(
        "{\"role\":\"assistant\",\"content\":\"yo\",\"timestamp\":\"2026-10-04T01:00:01Z\",",
        "\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}\n",
        "{\"role\":\"assistant\",\"content\":\"x\",\"model_used\":\"gpt-5\",",
        "\"usage\":{\"input_tokens\":9},\"is_error\":\"not-a-bool\"}\n",
    );
    let session = session("merge", raw);
    let skips = &session.metrics.line_skips;
    assert_eq!(skips.get("usage_present_not_counted"), Some(&1));
    assert_eq!(skips.get("event_schema"), Some(&1));
    assert_eq!(skips.get("model_or_usage_dropped"), Some(&1));
}

#[test]
fn stray_conversation_usage_disclosed_beside_meta_usage() {
    // Review fix F3 pin: a session whose counted lane (meta events)
    // already reported usage must still DISCLOSE stray usage riding a
    // conversation line — previously the `!has_reported_usage` guard
    // swallowed it silently, narrower than the batch's own prevention
    // rule 2. Totals stay meta-only: the stray line must not fold
    // (double-count) and must not vanish.
    let raw = concat!(
        "{\"role\":\"user\",\"content\":\"hi\",\"timestamp\":\"2026-10-04T01:00:00Z\"}\n",
        "{\"role\":\"session_meta\",\"usage\":{\"input_tokens\":10,\"output_tokens\":5},",
        "\"timestamp\":\"2026-10-04T01:00:02Z\"}\n",
        "{\"role\":\"assistant\",\"content\":\"yo\",\"timestamp\":\"2026-10-04T01:00:03Z\",",
        "\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}\n",
    );
    let session = session("mixed", raw);
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(session.metrics.tokens_input, 10);
    assert_eq!(session.metrics.tokens_output, 5);
    assert_eq!(
        session.metrics.line_skips.get("usage_present_not_counted"),
        Some(&1)
    );
}

#[test]
fn model_switch_joins_usage_models_for_per_block_pricing() {
    // Review fix F4 pin: the generic fold joins usage_models, so a
    // model switch mid-session reaches the per-block multi-model
    // pricing state. Catalog-independent observables: the "multiple"
    // marker and the per-block pricing_source — plus usage from BOTH
    // blocks folding into the totals.
    let raw = concat!(
        "{\"role\":\"assistant\",\"content\":\"a\",\"model_used\":\"model-a\",",
        "\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}\n",
        "{\"role\":\"assistant\",\"content\":\"b\",\"model_used\":\"model-b\",",
        "\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}\n",
    );
    let session = session("two-models", raw);
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(session.metrics.tokens_input, 12);
    assert_eq!(session.metrics.tokens_output, 5);
    assert_eq!(session.metrics.model_used, "multiple");
    assert_eq!(
        session.metrics.provenance.pricing_source,
        "multiple models (priced per usage block)"
    );
}

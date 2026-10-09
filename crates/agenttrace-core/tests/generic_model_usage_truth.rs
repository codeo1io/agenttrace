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
//! rm-856 fixtures rm-856-composition/double{,-reversed}.json are the
//! cycle-2 assessment's double-count PoC shape, reconstructed from the
//! recorded assessment (the /tmp original was swept; stewardship record
//! L4) — meta 100/50 +
//! assistant 7/3 folded to total_tokens 160 with NO disclosure) plus
//! the same shape with the meta line last, pinning the composition
//! policy and its order-independence.

use agenttrace_core::parse_file;
use std::fs;
use std::path::PathBuf;

const C2: &str = include_str!("fixtures/rm-616-generic/c2.json");
const C_GENERIC: &str = include_str!("fixtures/rm-616-generic/c-generic.json");
const DOUBLE: &str = include_str!("fixtures/rm-856-composition/double.json");
const DOUBLE_REVERSED: &str = include_str!("fixtures/rm-856-composition/double-reversed.json");
const META_SWITCH: &str = include_str!("fixtures/rm-856-composition/meta-switch.json");
const META_SWITCH_NO_USAGE: &str =
    include_str!("fixtures/rm-856-composition/meta-switch-nousage.json");

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
fn meta_usage_makes_the_meta_arm_the_counted_lane_not_a_double_count() {
    // rm-856 composition pin (red-first): a generic-lane session that
    // ALSO carries usage on a meta-role line previously folded BOTH
    // lanes into the same totals — the cycle-2 assess PoC (meta 100/50
    // + assistant 7/3 -> total_tokens 160) with no disclosure, because
    // the rm-616 generic fold and the meta arm saturating_add side by
    // side. The native-lane precedent
    // (`stray_conversation_usage_disclosed_beside_meta_usage`) is the
    // policy: when meta usage is present it IS the session's counted
    // lane — the meta block already aggregates the conversation — so
    // the generic fold stands down and the conversation line's usage
    // discloses beside it instead of double-counting.
    let session = session("double", DOUBLE);
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(session.metrics.model_used, "gpt-5");
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(
        session.metrics.tokens_input, 100,
        "meta-only totals: the conversation line's 7 must not add on top"
    );
    assert_eq!(
        session.metrics.tokens_output, 50,
        "meta-only totals: the conversation line's 3 must not add on top"
    );
    assert_eq!(
        session.metrics.line_skips.get("usage_present_not_counted"),
        Some(&1),
        "the stood-down line discloses beside the meta aggregate: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session.metrics.line_skips.len(),
        1,
        "exactly the composition disclosure, nothing else: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn meta_usage_composition_is_order_independent() {
    // Same composition with the meta-role line LAST: the counted-lane
    // decision is a whole-slice pre-scan (`has_meta_usage`), so it
    // cannot depend on where the meta line sits — a per-event gate
    // would fold the earlier conversation line before ever seeing the
    // meta usage and still double-count.
    let session = session("double-reversed", DOUBLE_REVERSED);
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(session.metrics.model_used, "gpt-5");
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 50);
    assert_eq!(
        session.metrics.line_skips.get("usage_present_not_counted"),
        Some(&1)
    );
    assert_eq!(session.metrics.line_skips.len(), 1);
}

#[test]
fn per_block_pricing_stands_down_generic_lines_under_meta_usage() {
    // Review-fix pin (independent review F3, red-first): when a meta-role
    // line carries model attribution different from the session's, the
    // multi-model per-block pricing arm fires — and it used to price the
    // STOOD-DOWN generic conversation block on top of the meta block:
    // tokens disclosed-not-counted but cost counted, the exact class
    // rm-856 closes. Catalog-independent reference: the same journal
    // with NO usage on the conversation line prices the meta block alone,
    // so the composition session's cost must equal it to the cent —
    // pre-fix it is reference + price(gpt-4o, 7, 3).
    let sess = session("meta-switch", META_SWITCH);
    let reference = session("meta-switch-nousage", META_SWITCH_NO_USAGE);
    assert_eq!(sess.metrics.source_tool, "generic");
    assert_eq!(sess.metrics.tokens_input, 100);
    assert_eq!(sess.metrics.tokens_output, 50);
    assert_eq!(
        sess.metrics.line_skips.get("usage_present_not_counted"),
        Some(&1),
        "the stood-down conversation line still discloses: {:?}",
        sess.metrics.line_skips
    );
    assert_eq!(
        sess.metrics.provenance.cost, "calculated_per_message_tokens",
        "post-fix this shape takes the CATALOG arm: usage_models joins only counted lanes, so the stood-down gpt-4o attribution leaves the inventory and the journal prices as the single meta model (pre-fix the multi-model arm fired and priced BOTH blocks — the 0.00087075 red)"
    );
    assert_eq!(
        sess.metrics.cost_estimated, reference.metrics.cost_estimated,
        "the stood-down block must contribute NOTHING to cost: got {:?} vs reference {:?}",
        sess.metrics.cost_estimated, reference.metrics.cost_estimated
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

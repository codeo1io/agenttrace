//! rm-882 (generic-lane role disclosure, journal-to-report truthfulness
//! trio): type-keyed conversation lines on the generic lane deserialized
//! as role-less Events, rode the `has_event_type` exemption past the
//! `non_event` census, and fell into analyze()'s `_ => {}` arm — usage
//! still folded (generic_reported_usage) while user_messages /
//! assistant_turns / title stayed zero with NO census row: silent
//! classification loss (violates the rm-449/rm-616 no-silent-loss
//! doctrine). The fix maps the conversational type keys onto roles so
//! analyze() counts them, and discloses type-bearing lines that map to
//! no known role via a `role_unclassified` census row.

use agenttrace_core::parse_file;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/generic")
        .join(name)
}

#[track_caller]
fn parse_generic(name: &str) -> agenttrace_core::Session {
    let path = fixture(name);
    parse_file(&path).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn type_keyed_user_assistant_lines_now_classify() {
    // hello-lf.jsonl: bare {"type":"user"|"assistant", timestamp, content,
    // usage} lines. Before rm-882 this parsed as generic with activity
    // 0/0/0 while usage folded — red pin: user_messages == 0.
    let session = parse_generic("hello-lf.jsonl");
    assert_eq!(session.metrics.source_tool, "generic");
    assert!(
        session.metrics.user_messages >= 1,
        "user line must classify: {:?}",
        session.metrics
    );
    assert!(
        session.metrics.assistant_turns >= 1,
        "assistant line must classify: {:?}",
        session.metrics
    );
    // Usage folding must keep working (it predates the fix).
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 50);
    // And nothing about this file warrants a classification census row.
    assert!(
        !session.metrics.line_skips.contains_key("role_unclassified"),
        "mapped roles must not disclose: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn one_tool_line_lane_flip_is_unchanged() {
    // hello-tool.jsonl is hello-lf plus ONE top-level tool_use line, which
    // fires the claude_code probe: it must keep parsing as claude_code with
    // the same conversational counts (probe gating must not regress).
    let session = parse_generic("hello-tool.jsonl");
    assert_eq!(session.metrics.source_tool, "claude_code");
    assert!(session.metrics.user_messages >= 1);
    assert!(session.metrics.assistant_turns >= 1);
}

#[test]
fn unclassified_type_lines_disclose_instead_of_vanishing() {
    // generic-unclassified-type.jsonl: a {"type":"system"} line with
    // usage that maps to no conversational role. It must stay an event
    // (usage still folds) AND mint a role_unclassified census row —
    // before rm-882 this line counted usage with zero disclosure. (A
    // tool_use line would fire the claude_code probe; that lane flip is
    // pinned separately by hello-tool above.)
    let session = parse_generic("generic-unclassified-type.jsonl");
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(session.metrics.tokens_input, 40);
    assert_eq!(session.metrics.tokens_output, 20);
    let unclassified = session
        .metrics
        .line_skips
        .get("role_unclassified")
        .copied()
        .unwrap_or(0);
    assert_eq!(
        unclassified, 1,
        "must disclose: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn generic_tool_typed_lines_are_classified_not_unclassified() {
    // Review fix F1 (cycle 2 independent review): a type-bearing generic
    // line whose type names a tool shape is tool activity, not an
    // unclassifiable line — disclosing it as role_unclassified overstated
    // the lane's disclosure census. Camel-case keys keep this journal on
    // the generic lane: exact "tool_use"/"tool_result" markers fire the
    // claude_code probe and that lane flip is pinned separately above.
    // generic-tool-role.jsonl: user 10/5 + toolUse 7/3 + toolResult 5/2.
    let session = parse_generic("generic-tool-role.jsonl");
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(
        session.metrics.line_skips.get("role_unclassified"),
        None,
        "tool-shaped type keys must not disclose as unclassified: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(session.metrics.tool_results, 2);
    // tool_calls_ok is NOT asserted: analyze() clamps ok to
    // tool_calls_total - fail (lib.rs :1379) — call-anchored by design —
    // and this journal has no assistant tool_call blocks, so ok clamps
    // to 0 while tool_results + provenance carry the activity.
    assert_eq!(
        session.metrics.provenance.tool_results,
        "reported_or_inferred"
    );
    // Usage still folds parse-time in the generic lane regardless of role.
    assert_eq!(session.metrics.tokens_input, 22);
    assert_eq!(session.metrics.tokens_output, 10);
}

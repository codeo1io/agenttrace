//! rm-1522 rider (cycle 3, run 6a4166cb): Claude Code v2.1.305
//! transcript-semantics contract fixtures.
//!
//! Three fixture classes from the 2.1.305 changelog (2026-10-21):
//! 1. Subagent fallback-model attribution — the record now carries the
//!    true post-fallback model. Pin: the lane attributes the sticky
//!    first-seen wire model, counts subagent (sidechain) usage exactly
//!    once, and never lets the subagent's distinct model corrupt or
//!    override the session's attribution.
//! 2. Workflow-resume journal serving — completed agents are re-served
//!    from the run journal on resume, re-emitting the SAME message id
//!    with identical usage. Pin: re-emitted turns must not double-count
//!    tokens (the fold is keyed by message id, max-per-class).
//! 3. Tool-search retry-without (2.1.304 regression, fixed in 2.1.305)
//!    — retried requests on some gateways carry no server tools, so
//!    request shape varies between a turn and its retry. Pin:
//!    `server_tool_use` content blocks and unknown `usage` keys are
//!    tolerated (no crash, no disclosure loss), and a retry with a NEW
//!    message id counts as its own turn.
//!
//! Red-first record: all three fixtures were run against the wall
//! parser BEFORE any parser change was considered — verdict recorded
//! in the attempt envelope (no parser arm was needed: the lane's
//! Value-field reads tolerate every shape above).

use agenttrace_core::{parse_file, Session};
use std::fs;
use std::path::Path;

fn parse_fixture(name: &str) -> Session {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/claude-code-2-1-305")
        .join(name);
    let raw = fs::read_to_string(&src).unwrap_or_else(|e| panic!("fixture {name} unreadable: {e}"));
    // tempdir carrier mirrors the 2.1.300 contract tests: the lane is
    // exercised through parse_file, not the in-memory API, so format
    // detection runs for real.
    let root = std::env::temp_dir().join(format!("rm1522-v305-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("tempdir");
    let dst = root.join("session.jsonl");
    fs::write(&dst, raw).expect("write corpus");
    let s = parse_file(&dst).expect("parse claude 2.1.305 fixture");
    let _ = fs::remove_dir_all(&root);
    s
}

#[test]
fn claude_2_1_305_subagent_fallback_model_attribution() {
    let s = parse_fixture("subagent-fallback-model-attribution.jsonl");
    let m = &s.metrics;
    // Sticky attribution: the parent's first-seen model wins; the
    // subagent's fallback model must not override it.
    assert_eq!(m.model_used, "claude-opus-4-6");
    // Every turn counted exactly once — subagent sidechain turns
    // included in usage, none dropped, none doubled.
    assert_eq!(m.tokens_input, 1200 + 500 + 300 + 150);
    assert_eq!(m.tokens_output, 300 + 120 + 60 + 40);
    assert_eq!(m.tokens_cache_r, 4000 + 2000);
    assert_eq!(m.tokens_cache_w, 800);
    assert_eq!(m.user_messages, 1);
    assert_eq!(m.assistant_turns, 4);
    // Never-silent bar: all record kinds are known to the lane.
    assert!(
        m.disclosure_counters.is_empty(),
        "unexpected disclosures: {:?}",
        m.disclosure_counters
    );
}

#[test]
fn claude_2_1_305_workflow_resume_journal_carrier_no_double_count() {
    let s = parse_fixture("workflow-resume-journal-carrier.jsonl");
    let m = &s.metrics;
    // msg_j1 is re-served from the journal (same message id, identical
    // usage): it must count ONCE. msg_j2 is a fresh post-resume turn.
    assert_eq!(m.tokens_input, 100 + 200);
    assert_eq!(m.tokens_output, 50 + 80);
    assert_eq!(m.tokens_cache_r, 900 + 1500);
    assert_eq!(m.tokens_cache_w, 10 + 5);
    assert_eq!(m.assistant_turns, 2);
    assert_eq!(m.user_messages, 2);
    assert!(
        m.disclosure_counters.is_empty(),
        "unexpected disclosures: {:?}",
        m.disclosure_counters
    );
}

#[test]
fn claude_2_1_305_tool_search_retry_without_shape_tolerated() {
    let s = parse_fixture("tool-search-retry-without.jsonl");
    let m = &s.metrics;
    // The retry is a distinct API call with its own message id: both
    // turns count; the server_tool_use blocks and the unknown
    // server_tool_usage key must not crash, lose, or duplicate usage.
    assert_eq!(m.tokens_input, 900 + 700);
    assert_eq!(m.tokens_output, 250 + 180);
    assert_eq!(m.assistant_turns, 2);
    assert_eq!(m.user_messages, 1);
    assert!(
        m.disclosure_counters.is_empty(),
        "unexpected disclosures: {:?}",
        m.disclosure_counters
    );
}

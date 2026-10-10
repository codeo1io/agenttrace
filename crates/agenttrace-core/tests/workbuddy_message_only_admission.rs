//! rm-961 (2026-10-11): workbuddy dialect admission must not require a
//! tool/reasoning marker. The vendor writes `type:"message"` rows with
//! recorded `message.usage` from the FIRST turn of a session — long
//! before any function_call exists — and the old probe gate
//! (`function_call | function_call_result | reasoning` + sessionId + cwd)
//! dropped those whole journals to the generic serde lane, which reads no
//! usage at all: recorded 100/10 rendered as a ~7-token text estimate with
//! `source_tool: "generic"`.
//!
//! The relaxed arm stays tightly workbuddy-shaped: `type:"message"` AND
//! sessionId AND cwd AND a NON-EMPTY usage object on the same record. The
//! claude_code lane (`type: user|assistant`), the array-JSON claude
//! transcript lane, and genuinely unknown producers keep their probes; a
//! `usage:{}` block has nothing to lose and also stays generic (boundary
//! pinned here, not by accident).
//!
//! Live red pair (this run's assessment): the two fixtures below are
//! byte-identical except the usage payload; only the recorded-usage one
//! may claim the workbuddy lane.

use agenttrace_core::parse_file;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workbuddy")
        .join(name)
}

#[test]
fn message_only_journal_with_recorded_usage_claims_workbuddy_lane() {
    let session =
        parse_file(&fixture("message-only-usage.jsonl")).unwrap_or_else(|e| panic!("parse: {e:#}"));
    assert_eq!(
        session.metrics.source_tool, "workbuddy",
        "recorded message.usage IS producer-shape evidence: no tool/reasoning marker needed"
    );
    assert_eq!(
        session.metrics.tokens_input, 100,
        "recorded usage must not degrade to a text estimate (live red: 7)"
    );
    assert_eq!(
        session.metrics.tokens_output, 10,
        "recorded usage must not degrade to a text estimate (live red: 0)"
    );
}

#[test]
fn message_only_journal_with_empty_usage_stays_generic() {
    let session = parse_file(&fixture("message-only-empty-usage.jsonl"))
        .unwrap_or_else(|e| panic!("parse: {e:#}"));
    assert_eq!(
        session.metrics.source_tool, "generic",
        "usage:{{}} has nothing to lose — the relaxed arm must not claim it"
    );
}

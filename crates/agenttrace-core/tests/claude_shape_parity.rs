//! rm-1908 (cycle 1, run 65afcf57): claude-lane disclosure parity.
//! The codex lane names unmatched shapes (`codex_unmatched_type:*`) and
//! the qwen lane `qwen_unmatched_type:*`; the claude lane silently
//! skipped every non-user/assistant row and every unrecognized usage
//! key. The fixtures below pin the four shape classes the 2.1.300 /
//! 2.1.301 changelog wave introduced (compact-boundary metadata,
//! truncated-tail resume summaries, tool_decision OTel rows with
//! classifier_verdict, cache_creation 5m/1h lifetime split) — each
//! must surface as a NAMED disclosure counter in json, never silence.
//!
//! Provenance: the rows are changelog-faithful reconstructions of the
//! pinned upstream lines (2.1.300: compact boundary keeps its kept-
//! messages record after a reply cut at the output limit;
//! cache_creation_5m_tokens/1h_tokens split cache writes by lifetime.
//! 2.1.301: tool_decision events carry classifier_verdict +
//! permission_mode). Private real transcripts are not scrapeable; the
//! shapes here mirror the documented schemas field-for-field.

use agenttrace_core::parse_file;
use std::path::{Path, PathBuf};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude-shapes-2_1_300")
}

fn session_for(name: &str) -> agenttrace_core::Session {
    let path = corpus().join(name);
    parse_file(&path).unwrap_or_else(|err| panic!("{name} parses: {err}"))
}

fn counter<'a>(session: &'a agenttrace_core::Session, key: &str) -> Option<&'a usize> {
    session.metrics.disclosure_counters.get(key)
}

#[test]
fn compact_boundary_rows_are_named_not_silent() {
    // (a) the compaction record shape change: the pre-2.1.300 boundary
    // (isCompactSummary content row) and the 2.1.300+ boundary (same
    // row + compactMetadata) both surface, distinguished by shape so
    // an operator sees WHICH compaction record a journal carries.
    let session = session_for("compact-boundary.jsonl");
    assert_eq!(
        counter(&session, "claude_system_row:compact"),
        Some(&1),
        "legacy compact boundary row is disclosed"
    );
    assert_eq!(
        counter(&session, "claude_system_row:compact+metadata"),
        Some(&1),
        "2.1.300+ compactMetadata shape is disclosed distinctly"
    );
    assert_eq!(session.metrics.tokens_input, 300, "usage unaffected");
    assert_eq!(session.metrics.tokens_output, 50);
}

#[test]
fn truncated_tail_resume_summary_is_disclosed() {
    // (b) truncated-tail resume semantics: a summary row with leafUuid
    // marks a tail resumed after a cut — the session shown is a tail,
    // earlier history lives in another file. That is a truthfulness
    // datum, not a dropped line: disclose it.
    let session = session_for("truncated-tail-resume.jsonl");
    assert_eq!(
        counter(&session, "claude_resume_summary:leaf"),
        Some(&1),
        "leaf-anchored resume summary is disclosed"
    );
    assert_eq!(session.metrics.tokens_input, 80, "usage still counted");
}

#[test]
fn tool_decision_otel_row_names_its_type() {
    // (c) 2.1.301 tool_decision rows (classifier_verdict +
    // permission_mode attrs) are not transcript rows — when one lands
    // in a scanned jsonl it must be named as an unmatched type, not
    // silently skipped.
    let session = session_for("otel-tool-decision.jsonl");
    assert_eq!(
        counter(&session, "claude_unmatched_type:tool_decision"),
        Some(&1),
        "unknown top-level type rides the unmatched-type counter (codex/qwen parity)"
    );
    assert_eq!(session.metrics.tokens_input, 50, "known rows still fold");
}

#[test]
fn cache_write_lifetime_split_is_disclosed_not_dropped() {
    // (d) cache_creation_5m_tokens/1h_tokens (2.1.300) and the
    // transcript cache_creation {ephemeral_5m/1h} object: both are a
    // SPLIT of cache_creation_input_tokens — folding them again would
    // double-count, dropping them silently loses the fact. The split
    // is disclosed per lifetime; benign non-token keys (service_tier)
    // do not fire.
    let session = session_for("cache-write-split.jsonl");
    assert_eq!(
        counter(&session, "claude_cache_write_split:ephemeral_5m"),
        Some(&1)
    );
    assert_eq!(
        counter(&session, "claude_cache_write_split:ephemeral_1h"),
        Some(&1)
    );
    assert_eq!(
        counter(&session, "claude_cache_write_split:5m_tokens"),
        Some(&1),
        "the OTel-named attr surfaces under the same family"
    );
    assert_eq!(
        counter(&session, "claude_cache_write_split:1h_tokens"),
        Some(&1)
    );
    assert_eq!(
        counter(&session, "claude_usage_unmatched_key:service_tier"),
        None,
        "benign non-token usage keys stay quiet"
    );
    assert_eq!(
        session.metrics.tokens_cache_w, 50,
        "cache writes counted ONCE at the total (no double fold of the split)"
    );
    assert_eq!(session.metrics.tokens_input, 110);
}

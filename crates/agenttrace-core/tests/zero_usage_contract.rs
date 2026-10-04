//! rm-408 contract: present-but-zero usage blocks are counted as
//! measured zeros and disclosed — never silently clean, never rerouted
//! to the text-estimate fallback. The contrast pair below pins the
//! absent-vs-present-zero distinction the acceptance names, plus the
//! mixed case (zeros counted alongside real usage) and the JSON
//! surface downstream automation reads.

use agenttrace_core::{parse_file, Session};
use std::fs;
use std::path::PathBuf;

fn fixture(tag: &str, lines: &[String]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("at-zero-usage-{tag}.jsonl"));
    fs::write(&path, lines.join("\n") + "\n").expect("write fixture");
    path
}

fn user_line() -> String {
    r#"{"type":"user","timestamp":"2026-10-04T01:00:00Z","cwd":"/tmp/zero","message":{"role":"user","content":"check the billing please"}}"#.to_string()
}

fn assistant_line(id: &str, usage: Option<&str>) -> String {
    let usage = match usage {
        Some(raw) => format!(r#","usage":{raw}"#),
        None => String::new(),
    };
    format!(
        r#"{{"type":"assistant","timestamp":"2026-10-04T01:00:0{id}Z","message":{{"id":"msg_{id}","model":"claude-sonnet-4-5-20250929","role":"assistant","content":[{{"type":"text","text":"on it"}}]{usage}}}}}"#
    )
}

const ALL_ZERO: &str = r#"{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}"#;
const REAL_USAGE: &str = r#"{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":2,"cache_read_input_tokens":3}"#;

#[test]
fn present_zero_usage_is_counted_and_flagged_not_estimated() {
    let path = fixture(
        "present",
        &[user_line(), assistant_line("5", Some(ALL_ZERO))],
    );
    let session: Session = parse_file(&path).expect("claude transcript parses");
    assert_eq!(
        session.metrics.zero_usage_events, 1,
        "one reported all-zero usage block"
    );
    assert_eq!(
        session.metrics.provenance.tokens, "reported_by_agent+zero_usage_reported:1",
        "present usage — even all-zero — is measured and the flag is appended"
    );
    // The flag names the affected-event count wherever the string lands.
    let json = serde_json::to_string(&session).expect("session serializes");
    assert!(
        json.contains("zero_usage_reported:1"),
        "provenance flag names the count, got: {}",
        session.metrics.provenance.tokens
    );
    assert!(
        json.contains("\"zero_usage_events\":1"),
        "structured field serializes for automation"
    );
    assert_eq!(
        session.metrics.tokens_input + session.metrics.tokens_output,
        0,
        "zeros count as measured zeros (no invented tokens)"
    );
    fs::remove_file(path).ok();
}

#[test]
fn absent_usage_still_falls_back_to_the_text_estimate() {
    let path = fixture("absent", &[user_line(), assistant_line("5", None)]);
    let session = parse_file(&path).expect("claude transcript parses");
    assert_eq!(
        session.metrics.zero_usage_events, 0,
        "no usage block reported, nothing to count"
    );
    assert_eq!(
        session.metrics.provenance.tokens, "estimated_from_text",
        "absent usage keeps the estimate path — the contrast half of the rule"
    );
    let json = serde_json::to_string(&session).expect("session serializes");
    assert!(
        !json.contains("zero_usage_events"),
        "zero count stays out of the JSON"
    );
    fs::remove_file(path).ok();
}

#[test]
fn mixed_session_counts_only_the_zero_blocks() {
    let path = fixture(
        "mixed",
        &[
            user_line(),
            assistant_line("5", Some(ALL_ZERO)),
            assistant_line("6", Some(REAL_USAGE)),
        ],
    );
    let session = parse_file(&path).expect("claude transcript parses");
    assert_eq!(session.metrics.zero_usage_events, 1);
    assert!(session
        .metrics
        .provenance
        .tokens
        .contains("zero_usage_reported:1"));
    // The real block is still counted; the zero block adds nothing.
    // (input stays gross; cache-read is its own column: 10/5/3.)
    assert_eq!(session.metrics.tokens_input, 10);
    assert_eq!(session.metrics.tokens_output, 5);
    assert_eq!(session.metrics.tokens_cache_r, 3);
    fs::remove_file(path).ok();
}

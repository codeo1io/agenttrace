//! rm-538 — workbuddy input-basis disclosures are not parse loss.
//!
//! The corpus below is the run-cb38b958 assess F1 PoC shape (wb-priced
//! session: a cache-inclusive input basis — `input_tokens` already has
//! `cache_read_input_tokens` subtracted upstream, so the two together
//! disclose `workbuddy_input_basis:cache_subtracted`). rm-450 first
//! surfaced that disclosure by minting it into `Metrics.line_skips`,
//! where it rode the "Dropped lines" parse-loss channel and tanked
//! `data_health.confidence` to "low" even though the parse was exact
//! (1/1 parsed, 0 unparseable, 0 schema skips). rm-538 moves the
//! counters to `Metrics.disclosure_counters` — the non-loss channel
//! the pi-family journal facts already use (rm-436/rm-437) — so:
//!
//! 1. `data_health.confidence` stays high (it keys on real parse loss);
//! 2. `data_health.line_skips` is empty and json carries the counters
//!    under `data_health.disclosures` (the named disclosure map);
//! 3. report renders show a "Disclosed facts" row (with the basis
//!    provenance note) and no "Dropped lines" row for this corpus.
//!
//! The fixture is synthetic (mirrors the live PoC at
//! /tmp/at-assess-a1a0/corpus3, wb-priced session), not a recording.

use std::fs;

use agenttrace_core::{
    compute_overview, data_health, parse_file, report_overview_markdown_with_context, Session,
    TimeRange,
};

/// Cache-inclusive basis: 2,000 reported input with 1,500 cache-read —
/// the subtraction assumption fires and is disclosed, but nothing is
/// lost (mirrors testdata/workbuddy/usage-basis-inclusive.jsonl shape
/// with the model carried on the message, as the wb-priced PoC does).
const WB_PRICED_INCLUSIVE: &str = concat!(
    r#"{"type": "function_call", "name": "bash", "arguments": "{}", "sessionId": "wb-priced", "cwd": "/tmp/proj", "callId": "c1", "providerData": {"model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "message", "role": "user", "content": [{"type": "text", "text": "priced"}], "sessionId": "wb-priced", "cwd": "/tmp/proj", "message": {"role": "user", "usage": {"input_tokens": 2000, "output_tokens": 50, "cache_read_input_tokens": 1500}}}"#,
    "\n",
    r#"{"type": "function_call_result", "output": "done", "callId": "c1", "sessionId": "wb-priced", "cwd": "/tmp/proj"}"#,
);

fn parse_fixture(name: &str, body: &str) -> Session {
    let dir = std::env::temp_dir().join(format!("rm538-{}-{}", name, std::process::id()));
    fs::create_dir_all(&dir).expect("tempdir");
    let file = dir.join(format!("{name}.jsonl"));
    fs::write(&file, body).expect("fixture write");
    let session = parse_file(&file).expect("parses as workbuddy");
    let _ = fs::remove_dir_all(&dir);
    session
}

#[test]
fn workbuddy_basis_disclosure_rides_the_disclosure_channel_not_parse_loss() {
    let session = parse_fixture("inclusive", WB_PRICED_INCLUSIVE);
    assert_eq!(session.metrics.source_tool, "workbuddy");

    // The counters moved channels (rm-538): present under
    // disclosure_counters, absent from line_skips.
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("workbuddy_input_basis:cache_subtracted"),
        Some(&1),
        "the basis note must still be disclosed somewhere: {:?}",
        session.metrics.disclosure_counters
    );
    assert!(
        session.metrics.line_skips.is_empty(),
        "an assumption disclosure is not a dropped line: {:?}",
        session.metrics.line_skips
    );

    // And nothing was actually lost: the usage block is counted.
    assert_eq!(session.metrics.tokens_input, 500);
    assert_eq!(session.metrics.tokens_cache_r, 1500);
}

#[test]
fn exact_parse_confidence_is_not_degraded_by_a_basis_disclosure() {
    let session = parse_fixture("confidence", WB_PRICED_INCLUSIVE);
    let health = data_health(&[session], 1, 0);

    assert_eq!(health.confidence, "high");
    assert!(
        health.line_skips.is_empty(),
        "Dropped-lines map must not carry the basis note: {:?}",
        health.line_skips
    );
    assert_eq!(
        health
            .disclosures
            .get("workbuddy_input_basis:cache_subtracted"),
        Some(&1),
        "json data_health.disclosures carries the named disclosure: {:?}",
        health.disclosures
    );
}

#[test]
fn report_renders_disclosed_facts_without_a_dropped_lines_row() {
    let session = parse_fixture("render", WB_PRICED_INCLUSIVE);
    let session_ref = &session;
    let health = data_health(std::slice::from_ref(session_ref), 1, 0);
    let markdown = report_overview_markdown_with_context(
        &compute_overview(std::slice::from_ref(session_ref)),
        std::slice::from_ref(session_ref),
        &health,
        TimeRange::All,
        false,
    );
    let text = markdown;

    assert!(
        text.contains("Disclosed facts"),
        "the disclosure row must stay visible (rm-450's goal survives the channel move):\n{text}"
    );
    assert!(
        text.contains("workbuddy_input_basis:cache_subtracted=1"),
        "the counter renders with its provenance note:\n{text}"
    );
    assert!(
        !text.contains("Dropped lines"),
        "an otherwise-exact parse must not present a parse-loss row:\n{text}"
    );
    assert!(
        text.contains("| Confidence | high |"),
        "confidence stays high through the report arm:\n{text}"
    );
}

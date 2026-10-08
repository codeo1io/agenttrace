//! rm-873: negative usage values are corrupt or hostile data. They were
//! already zeroed at the token fold (lib.rs `usage_tokens`); this file pins
//! that the zeroing is now DISCLOSED — via `disclosure_counters`
//! (`negative_usage_zeroed:<field>`, per-event counts) AND a
//! `negative_usage_zeroed` anomaly — so a corrupt journal is never
//! indistinguishable from a genuine zero.

use agenttrace_core::parse_file;
use std::fs;
use std::path::PathBuf;

const NEG_CACHE_READ: &str = include_str!("fixtures/rm-871-truth/negative-cache-read.jsonl");
const NEG_OUTPUT: &str = include_str!("fixtures/rm-871-truth/negative-output.jsonl");
const CLEAN: &str = include_str!("fixtures/rm-871-truth/usage-heavy.jsonl");

fn write(tag: &str, raw: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("at-rm873-{}-{tag}.jsonl", std::process::id()));
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
fn negative_cache_read_is_zeroed_and_disclosed() {
    let session = session("negcr", NEG_CACHE_READ);
    let m = &session.metrics;

    // historic safety: metrics stay zeroed for the negative field, and
    // positive fields on the same event still fold.
    assert_eq!(m.tokens_input, 1_000);
    assert_eq!(m.tokens_output, 200);
    assert_eq!(m.tokens_cache_r, 0, "negative cache_read clamps to 0");

    // disclosure: per-field counter
    assert_eq!(
        m.disclosure_counters
            .get("negative_usage_zeroed:cache_read_input_tokens"),
        Some(&1),
        "clamp must be counted in disclosure_counters: {:?}",
        m.disclosure_counters
    );

    // disclosure: anomaly row for table/JSON consumers
    let row = session
        .anomalies
        .iter()
        .find(|a| a.kind == "negative_usage_zeroed")
        .expect("anomaly must exist");
    assert_eq!(row.severity, "medium");
    assert!(
        row.detail.contains("cache_read_input_tokens"),
        "detail names the clamped field: {}",
        row.detail
    );
}

#[test]
fn negative_output_clamps_and_discloses_both_fields() {
    let session = session("negout", NEG_OUTPUT);
    let m = &session.metrics;

    assert_eq!(m.tokens_input, 1_000);
    assert_eq!(m.tokens_output, 0, "negative output clamps to 0");
    assert_eq!(m.tokens_cache_r, 0, "negative cache_read clamps to 0");

    assert_eq!(
        m.disclosure_counters
            .get("negative_usage_zeroed:output_tokens"),
        Some(&1)
    );
    assert_eq!(
        m.disclosure_counters
            .get("negative_usage_zeroed:cache_read_input_tokens"),
        Some(&1)
    );
    let row = session
        .anomalies
        .iter()
        .find(|a| a.kind == "negative_usage_zeroed")
        .expect("anomaly must exist");
    assert!(
        row.detail.contains("output_tokens") && row.detail.contains("cache_read_input_tokens"),
        "detail names every clamped field: {}",
        row.detail
    );
}

#[test]
fn clean_journal_produces_no_negative_disclosure() {
    let session = session("clean", CLEAN);
    let m = &session.metrics;

    assert_eq!(m.tokens_cache_r, 400_000, "positive value folds untouched");
    assert!(
        !m.disclosure_counters
            .keys()
            .any(|k| k.starts_with("negative_usage_zeroed:")),
        "no clamp, no counter: {:?}",
        m.disclosure_counters
    );
    assert!(!session
        .anomalies
        .iter()
        .any(|a| a.kind == "negative_usage_zeroed"));
}

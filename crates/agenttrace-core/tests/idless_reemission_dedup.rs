//! rm-880 (id-less re-emission dedup, journal-to-report truthfulness
//! trio): streaming claude journals can re-emit an id-less assistant row
//! with growing usage (the class ccusage #1837/#1838 saw and fixed
//! upstream as "counted 2-3x when requestId absent"). The fork's fold map
//! `usage_by_message` only engages when `message.id` is non-empty, so
//! every id-less emission pushed its own meta Event and session totals
//! SUMMED re-emissions — 2-3x usage overcounts. The fix discriminates:
//! with NO intervening user line, the SAME model, every token class
//! non-decreasing, and a sub-2s gap between emissions, the rows are one
//! response and max-fold (the with-id semantics); anything else keeps
//! legacy summing. Every fold is disclosed via the
//! `claude_idless_reemission_folded` disclosure counter so the
//! correction is observable.

use agenttrace_core::parse_file;
use std::path::PathBuf;

fn parse_claude(name: &str) -> agenttrace_core::Session {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/usage-accounting")
        .join(name);
    parse_file(&path).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn folds(session: &agenttrace_core::Session) -> usize {
    session
        .metrics
        .disclosure_counters
        .get("claude_idless_reemission_folded")
        .copied()
        .unwrap_or(0)
}

#[test]
fn idless_reemissions_fold_when_the_gate_holds() {
    // claude-idless-reemission.jsonl: one user line, then two id-less
    // assistant rows 300ms apart, same model, usage growing
    // 100/50 -> 120/60. One response re-emitted: 120/60, NOT the legacy
    // sum 220/110 (the research-phase live PoC of the bug).
    let session = parse_claude("claude-idless-reemission.jsonl");
    assert_eq!(session.metrics.tokens_input, 120);
    assert_eq!(session.metrics.tokens_output, 60);
    assert_eq!(folds(&session), 1, "the fold must be disclosed");
}

#[test]
fn idless_distinct_responses_stay_summed() {
    // claude-idless-distinct.jsonl: two q/a pairs with id-less assistant
    // rows — an intervening user line and a five-minute gap break the
    // gate, so these are two responses: summed 300/130, no fold.
    let session = parse_claude("claude-idless-distinct.jsonl");
    assert_eq!(session.metrics.tokens_input, 300);
    assert_eq!(session.metrics.tokens_output, 130);
    assert_eq!(folds(&session), 0, "distinct responses must not fold");
}

#[test]
fn with_id_stream_fold_is_unchanged() {
    // rm-616 regression guard: the with-id growing stream (emissions
    // 1000/50 -> 1000/120 -> 1000/200 under one message id) still
    // max-folds into one counted event and reports no id-less folds.
    let session = parse_claude("claude-stream-growing.jsonl");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(session.metrics.tokens_output, 200);
    assert_eq!(folds(&session), 0);
}

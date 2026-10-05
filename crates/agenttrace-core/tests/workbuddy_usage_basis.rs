//! rm-497 verify-first harness: workbuddy usage-basis netting contract.
//!
//! Mechanism under test (research run 97480e45, live PoC at ea5c41e): the
//! workbuddy parser nets `input_tokens` against `cache_read_input_tokens`
//! (`saturating_sub().max(0)` in `parser.rs` `workbuddy_usage`) — correct
//! when the vendor reports input GROSS of cache reads. When the reported
//! basis disagrees (input smaller than cache reads) the subtraction clamps
//! input to zero, and at the fork point that clamp was completely silent:
//! the recorded live PoC (`wb-mismatch`: input 50 / cache_read 5000 /
//! output 40) reported `tokens 5040 (input silently 0)`, cost priced on
//! cache reads alone, and zero parse diagnostics.
//!
//! The fix landed as rm-497 (run 1f12309adc31, cycle 3): the netting
//! semantics are deliberately UNCHANGED (metrics-neutral for every journal
//! that parsed before), but every firing of the clamp now surfaces as the
//! per-session parse diagnostic `workbuddy_usage_basis_clamped` — the same
//! disclosure channel rm-400 (kimi usage aliases) and rm-401 (codex
//! token_usage_records) use. Usage blocks riding `reasoning` /
//! `function_call_result` lines (never consulted by the netting) are
//! likewise disclosed as `workbuddy_usage_dropped:{type}` instead of
//! disappearing quietly; they stay uncounted.
//!
//! The `basis-clamped.jsonl` test is the regression gate: flipping the
//! counter off restores the silent zero from the live PoC. The
//! `basis-includes.jsonl` control pins the healthy net (175 gross − 40
//! cached = 135 net input) with EMPTY diagnostics, so clean journals keep
//! byte-identical reports.

use agenttrace_core::parse_raw_session;
use std::fs;
use std::path::PathBuf;

fn parse_fixture(name: &str) -> agenttrace_core::Session {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workbuddy")
        .join(name);
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    parse_raw_session("workbuddy", name, &raw).unwrap_or_else(|e| panic!("parse {}: {}", name, e))
}

#[test]
fn gross_basis_journal_nets_cache_and_stays_silent() {
    // The contract's healthy side: input reported gross of cache reads.
    let session = parse_fixture("basis-includes.jsonl");
    assert_eq!(session.metrics.tokens_input, 135, "175 gross - 40 cached");
    assert_eq!(session.metrics.tokens_cache_r, 40);
    assert_eq!(session.metrics.tokens_output, 40);
    assert!(
        session.metrics.line_skips.is_empty(),
        "a well-formed gross-basis journal must not fire any disclosure — \
         clean corpora keep byte-identical report output"
    );
}

#[test]
fn basis_mismatch_clamps_input_and_discloses_it() {
    // The live PoC numbers: input 50 / cache_read 5000 / output 40.
    // The netting is unchanged (input 0, cost on cache reads alone) but the
    // clamp is now a visible per-session parse diagnostic instead of a
    // silent zero.
    let session = parse_fixture("basis-clamped.jsonl");
    assert_eq!(session.metrics.tokens_input, 0, "clamp semantics unchanged");
    assert_eq!(session.metrics.tokens_cache_r, 5000);
    assert_eq!(session.metrics.tokens_output, 40);
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("workbuddy_usage_basis_clamped"),
        Some(&1),
        "silent-zero regression gate: the recorded PoC at ea5c41e fired zero counters"
    );
}

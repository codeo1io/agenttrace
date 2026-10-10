//! rm-497 verify-first harness: workbuddy usage-basis netting contract.
//!
//! Mechanism under test (research run 97480e45, live PoC at ea5c41e): the
//! workbuddy parser nets `input_tokens` against `cache_read_input_tokens` —
//! correct when the vendor reports input GROSS of cache reads. When the
//! reported basis disagrees (input smaller than cache reads) the netting
//! clamps, and at the fork point that clamp was completely silent: the
//! recorded live PoC (`wb-mismatch`: input 50 / cache_read 5000 / output
//! 40) reported `tokens 5040 (input silently 0)`, cost priced on cache
//! reads alone, and zero parse diagnostics.
//!
//! Merged semantics (conflict case 739e7bc4, integration of run
//! 1f12309adc31): rm-497's candidate mechanics were superseded by the
//! landed family — upstream #311's sum riders (rm-600 landed them for
//! message/reasoning/function_call records), upstream #316's clamp
//! (`subtract_cached_input` clamps the CACHE count to the remaining
//! input, so cache_read 5000 against input 50 reads as cache_r 50), and
//! rm-538's channel move (the whole `workbuddy_input_basis:*` family
//! lands on `Metrics.disclosure_counters`, the non-loss channel rendered
//! under "Disclosed facts", never in `line_skips`). The counter key is
//! `workbuddy_input_basis:cache_clamped`. The reimplemented rm-497
//! residual on this tree is the `function_call_result` drop disclosure
//! (`workbuddy_usage_dropped:function_call_result`, same non-loss
//! channel), pinned in the parser lib tests; reasoning-record usage is
//! READ into the sum (upstream #311), not dropped.
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
        "a well-formed gross-basis journal must not fire any parse loss — \
         clean corpora keep byte-identical report output"
    );
    // rm-450's landed aggregator discloses the plain subtraction itself
    // (cache_subtracted) — the healthy net is a disclosed fact on the
    // non-loss channel, never a skipped line. No clamp/mismatch keys.
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("workbuddy_input_basis:cache_subtracted"),
        Some(&1)
    );
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("workbuddy_input_basis:cache_clamped")
            && !session
                .metrics
                .disclosure_counters
                .contains_key("workbuddy_input_basis:zeroed_suspected_mismatch"),
        "the clean control fires no clamp or mismatch disclosure"
    );
}

#[test]
fn basis_mismatch_clamps_cache_and_discloses_it() {
    // The live PoC numbers: input 50 / cache_read 5000 / output 40. Under
    // the landed upstream #316 clamp (rm-600) the CACHE count is clamped
    // to the remaining input (cache_r 50, input 0 — the session total
    // never exceeds the source-recorded input), and the firing is a
    // visible non-loss disclosure (rm-538 channel) instead of a silent
    // zero.
    let session = parse_fixture("basis-clamped.jsonl");
    assert_eq!(
        session.metrics.tokens_input, 0,
        "net input floors at zero under the clamp"
    );
    assert_eq!(
        session.metrics.tokens_cache_r, 50,
        "upstream #316: the cached count is clamped to the source-recorded input, not reported raw at 5000"
    );
    assert_eq!(session.metrics.tokens_output, 40);
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("workbuddy_input_basis:cache_clamped"),
        Some(&1),
        "silent-zero regression gate: the recorded PoC at ea5c41e fired zero counters"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("workbuddy_input_basis:zeroed_suspected_mismatch"),
        Some(&1),
        "the landed rm-450 aggregator arm also flags the zeroed input"
    );
    assert!(
        session.metrics.line_skips.is_empty(),
        "the clamp fact is a disclosure, never parse loss (rm-526/rm-538)"
    );
}

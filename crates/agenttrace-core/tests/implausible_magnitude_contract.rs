//! rm-726 contract: implausible token magnitudes are disclosed, never
//! silently priced as clean truth.
//!
//! The corpus below is the run-32f3b7a1 assess F1 PoC pair, pinned
//! verbatim under `tests/fixtures/implausible-magnitude/`:
//!
//! - `big.jsonl`: `input_tokens: 1e19` (above i64::MAX — the parser's
//!   `number_as_i64` clamps it onto the i64::MAX bound) beside
//!   `output_tokens: 1e18` (in-range but far above any plausible
//!   per-message magnitude). Priced as parsed this renders a $42.7T
//!   total that used to sail through with HEALTH 100 and
//!   "No anomalies detected".
//! - `neg.jsonl`: negative usage values, zeroed by the aggregator's
//!   `.max(0)` clamps with no disclosure at all.
//!
//! After rm-726 every clamp fact rides `Metrics.disclosure_counters`
//! under the `magnitude_clamped:*` family, `detect_anomalies` grows the
//! `implausible_magnitude` class (severity high → health drops below
//! 100), and the cost surfaces (`--overview` text, `--audit` JSON)
//! disclose the clamp count beside the total while still pricing what
//! was parsed.

use agenttrace_core::{
    compute_overview, cost_audit, fix_suggestions, parse_file, report_overview_text,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/implausible-magnitude");
    path.push(name);
    assert!(path.exists(), "fixture must be in-tree: {}", path.display());
    path
}

#[test]
fn above_i64_magnitudes_are_clamped_counted_and_flagged() {
    let session = parse_file(&fixture("big.jsonl")).expect("parses as claude-code");

    // The clamps are disclosed on the dedicated channel, not treated as
    // parse loss (rm-538 channel semantics).
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("magnitude_clamped:above_i64"),
        Some(&1),
        "the 1e19 input clamp must be counted: {:?}",
        session.metrics.disclosure_counters
    );
    // Both surviving values (i64::MAX and 1e18) sit at/above the
    // documented plausibility bound (1e12 tokens per value).
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("magnitude_clamped:above_plausibility_bound"),
        Some(&2),
        "both implausible magnitudes must be counted: {:?}",
        session.metrics.disclosure_counters
    );

    // The anomaly class fires and health drops below 100.
    let anomaly = session
        .anomalies
        .iter()
        .find(|anomaly| anomaly.kind == "implausible_magnitude")
        .expect("implausible_magnitude anomaly must be present");
    assert_eq!(anomaly.severity, "high");
    assert!(
        session.health < 100,
        "health must drop below 100, got {}",
        session.health
    );

    // Pricing still prices what was parsed (no silent drop): the
    // $42.7T total survives, now with its clamp count beside it.
    assert_eq!(session.metrics.tokens_input, i64::MAX);
    assert_eq!(session.metrics.tokens_output, 1_000_000_000_000_000_000);
    assert!(
        session.metrics.cost_estimated > 40_000_000_000_000.0,
        "cost must stay priced as parsed: {}",
        session.metrics.cost_estimated
    );
}

#[test]
fn negative_usage_clamp_to_zero_is_counted_and_flagged() {
    let session = parse_file(&fixture("neg.jsonl")).expect("parses as claude-code");

    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("magnitude_clamped:negative_to_zero"),
        Some(&2),
        "both negative values must be counted: {:?}",
        session.metrics.disclosure_counters
    );
    assert!(
        session
            .anomalies
            .iter()
            .any(|anomaly| anomaly.kind == "implausible_magnitude"),
        "negative usage is an implausible magnitude, not silent zeroing"
    );
    assert!(
        session.health < 100,
        "health must drop below 100, got {}",
        session.health
    );
    // Conflict 606aefa4 rebase: rm-831 (landed on master after rm-726
    // was authored) refuses negative classes at intake — "corrupt
    // journal data, never a real measurement" — so the values never
    // ride the fold for the old `.max(0)` zeroing. With no usage left
    // the session prices from the absent-usage text-estimate fallback
    // (user "n" and assistant "ok" each estimate at 1 token), and
    // the corrupt-journal fact rides the disclosure counters + the
    // implausible_magnitude anomaly above instead of a zeroed total.
    assert_eq!(session.metrics.tokens_input, 1);
    assert_eq!(session.metrics.tokens_output, 1);
}

#[test]
fn overview_and_audit_disclose_the_clamp_count_beside_the_total() {
    let big = parse_file(&fixture("big.jsonl")).expect("big parses");
    let neg = parse_file(&fixture("neg.jsonl")).expect("neg parses");
    let sessions = [big, neg];

    // 1 clamp above i64 + 2 bound trips (big) + 2 negative clamps (neg).
    let overview = compute_overview(&sessions);
    assert_eq!(overview.clamped_values, 5);

    let text = report_overview_text(&overview, &sessions);
    assert!(
        text.contains("Total estimated cost"),
        "the total stays priced: {text}"
    );
    assert!(
        text.contains("Clamped values"),
        "the clamp count rides beside the total: {text}"
    );

    let audit = cost_audit(&sessions);
    assert_eq!(audit.clamped_values, 5);
    assert!(
        audit.total_estimated_cost > 40_000_000_000_000.0,
        "audit total stays priced as parsed: {}",
        audit.total_estimated_cost
    );
}

#[test]
fn fix_suggestions_cover_the_implausible_magnitude_class() {
    let session = parse_file(&fixture("big.jsonl")).expect("parses");
    let fixes = fix_suggestions(&session);
    assert!(
        fixes
            .iter()
            .any(|fix| fix.title.contains("token counts") && fix.category == "data_quality"),
        "the new anomaly class must have a fix suggestion: {fixes:?}"
    );
}

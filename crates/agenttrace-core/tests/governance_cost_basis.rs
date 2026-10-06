//! rm-578 — governance cost-basis truthfulness.
//!
//! The assess PoC pair that isolated the false-drift defect, kept as
//! the regression fixture the row asks for: the same 31.5k tokens on
//! `claude-sonnet-4-20250514`, once with the cost recorded by the
//! journal (a pi v3 `type:"usage"` entry carrying
//! `usage.cost.total`, recorded-cost basis) and once without it
//! (usage inside the assistant message only, pure catalog basis).
//! The stored estimate of the recorded-cost session legitimately
//! differs from a full catalog re-price, and the audit's drift note
//! ("current rates recalculate a different total than the stored
//! estimate") used to fire on every such session; it must re-price
//! on the stored estimate's own basis instead.

use std::fs;
use std::path::PathBuf;

use agenttrace_core::{cost_audit, lookup_price, parse_file, session_cost_audit, Session};

const RECORDED: &str = concat!(
    r#"{"id": "b1", "timestamp": "2026-10-04T07:16:46.634Z", "type": "session", "version": 3, "cwd": "/redacted/project"}"#,
    "\n",
    r#"{"id": "b2", "parentId": "b1", "timestamp": "2026-10-04T07:16:46.700Z", "type": "session_info", "name": "recorded-cost-usage-entry"}"#,
    "\n",
    r#"{"id": "b3", "timestamp": "2026-10-04T07:16:47.000Z", "message": {"role": "user", "content": [{"type": "text", "text": "hello"}], "timestamp": "2026-10-04T07:16:47.000Z"}, "type": "message"}"#,
    "\n",
    r#"{"id": "b4", "timestamp": "2026-10-04T07:16:48.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "hi"}], "timestamp": "2026-10-04T07:16:48.000Z", "model": "claude-sonnet-4-20250514"}, "type": "message"}"#,
    "\n",
    r#"{"id": "b5", "timestamp": "2026-10-04T07:16:48.100Z", "type": "usage", "kind": "message", "provider": "anthropic", "model": "claude-sonnet-4-20250514", "usage": {"input": 500, "output": 1000, "cacheRead": 25000, "cacheWrite": 5000, "cost": {"total": 0.0075}}}"#,
);

const CATALOG_ONLY: &str = concat!(
    r#"{"id": "a1", "timestamp": "2026-10-04T07:16:46.634Z", "type": "session", "version": 3, "cwd": "/redacted/project"}"#,
    "\n",
    r#"{"id": "a2", "parentId": "a1", "timestamp": "2026-10-04T07:16:46.700Z", "type": "session_info", "name": "single-model-recorded"}"#,
    "\n",
    r#"{"id": "a3", "timestamp": "2026-10-04T07:16:47.000Z", "message": {"role": "user", "content": [{"type": "text", "text": "hello"}], "timestamp": "2026-10-04T07:16:47.000Z"}, "type": "message"}"#,
    "\n",
    r#"{"id": "a4", "timestamp": "2026-10-04T07:16:48.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "hi there"}], "timestamp": "2026-10-04T07:16:48.000Z", "usage": {"input": 500, "output": 1000, "cacheRead": 25000, "cacheWrite": 5000, "totalTokens": 31500, "cost": {"input": 0.0015, "output": 0.006, "cacheRead": 0.0, "cacheWrite": 0.0, "total": 0.0075}}, "model": "claude-sonnet-4-20250514"}, "type": "message"}"#,
);

/// Scratch dir per call (atomic sequence, not `line!()`, which is
/// macro-site invariant).
fn scratch_dir() -> PathBuf {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("at-governance-basis-{}-{seq}", std::process::id()));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn parsed(name: &str, body: &str) -> Session {
    let path = scratch_dir().join(name);
    fs::write(&path, body).expect("write fixture");
    parse_file(&path).unwrap_or_else(|error| panic!("{name} must parse as pi journal: {error}"))
}

fn catalog_total() -> f64 {
    let price = lookup_price("claude-sonnet-4-20250514");
    (500.0 / 1e6 * price.input)
        + (1000.0 / 1e6 * price.output)
        + (5000.0 / 1e6 * price.cw)
        + (25000.0 / 1e6 * price.cr)
}

const DRIFT_NOTE: &str = "current rates recalculate";

#[test]
fn recorded_cost_sessions_carry_no_drift_note() {
    // The rm-578 defect: the stored estimate rests on a recorded-cost
    // basis (0.0075 at face value), which legitimately differs from
    // the 0.0428 catalog re-price — that is a basis difference, not
    // rate drift, and must not be reported as drift.
    let session = parsed("recorded.jsonl", RECORDED);
    assert_eq!(session.metrics.tokens_input, 500);
    assert_eq!(session.metrics.tokens_output, 1_000);
    assert_eq!(session.metrics.tokens_cache_w, 5_000);
    assert_eq!(session.metrics.tokens_cache_r, 25_000);
    assert!((session.metrics.upstream_cost_usd - 0.0075).abs() < 1e-9);
    let split = session.metrics.upstream_priced_input
        + session.metrics.upstream_priced_output
        + session.metrics.upstream_priced_cache_w
        + session.metrics.upstream_priced_cache_r;
    assert_eq!(split, 31_500, "the token side of the recorded cost");
    assert!((session.metrics.cost_estimated - 0.0075).abs() < 1e-6);

    let audit = session_cost_audit(&session);
    assert!(
        !audit.pricing_note.contains(DRIFT_NOTE),
        "recorded-cost basis must not read as drift, note was: {}",
        audit.pricing_note
    );
    let current = audit.estimated_cost_usd.expect("same-basis re-price");
    assert!(
        (current - session.metrics.cost_estimated).abs() < 1e-6,
        "same-basis current total {current} must match stored {}",
        session.metrics.cost_estimated
    );

    let report = cost_audit(&[session]);
    let row = &report.by_provider_model[0];
    assert!(!row.pricing_note.contains(DRIFT_NOTE));
    let row_current = row.estimated_cost_usd.expect("row re-price");
    assert!((row_current - row.stored_estimated_cost_usd).abs() < 1e-6);
}

#[test]
fn the_same_tokens_without_recorded_cost_still_reprice() {
    // The control: identical tokens, no `type:"usage"` cost entry, so
    // the stored estimate is pure catalog pricing. The current-rate
    // column still re-prices every token, and with unchanged rates
    // there is no drift note.
    let session = parsed("catalog-only.jsonl", CATALOG_ONLY);
    assert!(session.metrics.upstream_cost_usd <= 0.0);
    assert_eq!(session.metrics.upstream_priced_input, 0);

    let expected = catalog_total();
    let audit = session_cost_audit(&session);
    let current = audit.estimated_cost_usd.expect("catalog re-price");
    assert!(
        (current - expected).abs() < 1e-4,
        "catalog-only session must re-price at current rates: {current} vs {expected}"
    );
    assert!(
        (current - session.metrics.cost_estimated).abs() < 1e-4,
        "unchanged rates mean no drift: {current} vs {}",
        session.metrics.cost_estimated
    );
    assert!(!audit.pricing_note.contains(DRIFT_NOTE));
}

#[test]
fn genuine_rate_drift_is_still_reported() {
    // The note's honest meaning survives the fix: a stored estimate
    // priced under different catalog rates (recorded dollars absent)
    // still re-prices to a different total and says so.
    let mut session = parsed("catalog-only.jsonl", CATALOG_ONLY);
    let expected = catalog_total();
    // Simulate pricing under older, cheaper rates.
    session.metrics.cost_estimated = expected * 0.5;
    let audit = session_cost_audit(&session);
    assert!(
        audit.pricing_note.contains(DRIFT_NOTE),
        "real drift must keep the note, note was: {}",
        audit.pricing_note
    );
    assert!((audit.estimated_cost_usd.expect("re-price") - expected).abs() < 1e-4);
}

#[test]
fn a_split_lost_to_a_cache_round_trip_stays_silent() {
    // The in-memory upstream-priced split does not survive a session
    // cache round-trip (`GoMetrics::into_metrics` zero-defaults it),
    // while the recorded dollars do. That basis cannot be reproduced,
    // so the audit must not report it as drift either.
    let mut session = parsed("recorded.jsonl", RECORDED);
    session.metrics.upstream_priced_input = 0;
    session.metrics.upstream_priced_output = 0;
    session.metrics.upstream_priced_cache_w = 0;
    session.metrics.upstream_priced_cache_r = 0;
    let audit = session_cost_audit(&session);
    assert!(
        !audit.pricing_note.contains(DRIFT_NOTE),
        "an unreproducible basis must stay silent, note was: {}",
        audit.pricing_note
    );
    assert!(
        audit.pricing_note.contains("no same-basis re-price"),
        "the withheld basis must be disclosed, note was: {}",
        audit.pricing_note
    );
    assert_eq!(
        audit.estimated_cost_usd, None,
        "a split-lost basis must not publish a double-counted current-rate total"
    );
    assert!(
        audit.rates_per_million_usd.is_none(),
        "no rates alongside a withheld estimate"
    );
    assert!(audit.pricing_source.contains("session cache"));

    // The row aggregate inherits the same withholding.
    let report = cost_audit(&[session]);
    let row = &report.by_provider_model[0];
    assert!(!row.pricing_note.contains(DRIFT_NOTE));
    assert!(row.pricing_note.contains("no same-basis re-price"));
    assert_eq!(row.estimated_cost_usd, None);
}

//! rm-1965 contract: timestamp-aware rate selection — a session is priced
//! at the rate live at session time, and the basis is disclosed
//! (`rate_date` + `rate_source` + `rate_basis`).
//!
//! RED-FIRST PROOF: every assertion below fails on the pre-rm-1965 tree —
//! `lookup_priced_at` did not exist (compile-time red for the selection
//! arms) and `Metrics` carried no rate-basis disclosure (the serialized
//! disclosure arms fail against the pre-change struct). The pins use the
//! bundled 2026-10-08 LiteLLM snapshot and the models.dev captures the
//! cycle-3 research runs recorded (runs f97d974e 10-07T23:27Z, ad6d83cb
//! 10-12T20:47Z, 939c4681/020b341d/1e51bf14 on 10-17 — dossier C1: eight
//! in-place cost edits within six hours, catalog row count unchanged).
//!
//! Units: every rate below is USD per million tokens, matching the
//! in-memory `Price` basis (the LiteLLM loader scales its per-token
//! fields ×1e6; models.dev publishes per-million directly).

use std::collections::BTreeMap;

use agenttrace_core::{analyze, rate_history_rows, Event, Metrics};

/// Catalog-served churn key (LiteLLM snapshot carries the `~` form).
const KEY: &str = "openrouter/~deepseek/deepseek-v4-flash-latest";
/// Raw-id key the LiteLLM snapshot does NOT carry (no kilo rows at all).
const RAW_KEY: &str = "kilo/tencent/hy3";

fn ts(s: &str) -> chrono::DateTime<chrono::Utc> {
    s.parse().expect("test timestamp parses")
}

#[test]
fn history_rows_are_deduped_dated_and_evidence_backed() {
    let rows = rate_history_rows();
    assert!(!rows.is_empty(), "rm-1965 table must not be empty");

    // Bounded storage: rows exist only where an edit was observed, never
    // one per catalog key (the snapshot carries thousands).
    assert!(
        rows.len() < 200,
        "rate-history table stays bounded: {} rows",
        rows.len()
    );

    // Per-key: dates ascending in table order, no consecutive
    // duplicate states, every row carries its evidence trail.
    let mut last: Option<(&str, &str, f64, f64)> = None;
    for row in rows {
        assert_eq!(row.effective.len(), 10, "effective is YYYY-MM-DD");
        assert!(
            row.observed.starts_with(row.effective),
            "observed on its effective date"
        );
        assert!(!row.note.is_empty(), "note carries provenance");
        assert!(row.input.is_finite() && row.input > 0.0, "sane input rate");
        assert!(
            row.output.is_finite() && row.output > 0.0,
            "sane output rate"
        );
        if let Some((key, date, input, output)) = last {
            if key == row.key {
                assert!(
                    date.as_bytes() <= row.effective.as_bytes(),
                    "rows for {key} ascend by date"
                );
                assert!(
                    (input, output) != (row.input, row.output) || date != row.effective,
                    "no consecutive duplicate state for {key}"
                );
            }
        }
        last = Some((row.key, row.effective, row.input, row.output));
    }

    // The research-attested churn key is present with its multi-state
    // timeline (dossier C1: 0.018/1.28 on 10-07, 0.0128/0.768 on 10-12,
    // 0.0079/0.2829 then back to 1.28 on 10-17).
    let states: Vec<_> = rows.iter().filter(|row| row.key == KEY).collect();
    assert!(states.len() >= 5, "headline key has a multi-state timeline");
    assert_eq!(states[0].effective, "2026-10-07");
    assert!((states[0].output - 1.28).abs() < 1e-9);
    let mid = states
        .iter()
        .find(|row| row.effective == "2026-10-12")
        .expect("10-12 observed state recorded");
    assert!((mid.output - 0.768).abs() < 1e-9);
    let last = states.last().expect("newest state");
    assert_eq!(last.effective, "2026-10-17");
    assert!((last.output - 1.28).abs() < 1e-9);
}

#[test]
fn session_between_edits_prices_at_the_state_live_then() {
    // Session on 2026-10-14: the ladder serves the 10-12 observed state
    // (0.0128/0.768), NOT the stale bundled 2026-10-08 row (0.04/0.64)
    // and NOT the newest 10-17 state.
    let priced = agenttrace_core::lookup_priced_at(KEY, None, Some(ts("2026-10-14T12:00:00Z")));
    assert!((priced.price.input - 0.0128).abs() < 1e-12);
    assert!((priced.price.output - 0.768).abs() < 1e-12);
    assert!((priced.price.cr.expect("10-12 state publishes cache_read") - 0.0128).abs() < 1e-12);
    assert_eq!(priced.basis.rate_date, "2026-10-12");
    assert_eq!(priced.basis.basis, agenttrace_core::RateBasisArm::AtSession);
    assert!(
        priced.basis.rate_source.starts_with("rate history"),
        "history-served arm discloses its source: {}",
        priced.basis.rate_source
    );
}

#[test]
fn session_before_every_state_gets_earliest_state_disclosed() {
    // Session on 2026-10-05 predates every recorded state AND the
    // bundled snapshot: the earliest provable state (10-07) serves and
    // the basis admits it postdates the session.
    let priced = agenttrace_core::lookup_priced_at(KEY, None, Some(ts("2026-10-05T12:00:00Z")));
    assert!((priced.price.output - 1.28).abs() < 1e-9);
    assert!((priced.price.input - 0.018).abs() < 1e-9);
    assert_eq!(priced.basis.rate_date, "2026-10-07");
    assert_eq!(
        priced.basis.basis,
        agenttrace_core::RateBasisArm::NewerThanSession
    );
}

#[test]
fn bundled_row_is_a_state_on_the_same_ladder() {
    // Session on 2026-10-10 (between the 10-08 snapshot and the 10-12
    // observed edit): the BUNDLED state is the newest at session time and
    // serves as-is, disclosed at its own date. This is the arm that keeps
    // every catalog-served key numerically identical whenever no edit
    // intervened.
    let priced = agenttrace_core::lookup_priced_at(KEY, None, Some(ts("2026-10-10T12:00:00Z")));
    assert!(
        (priced.price.input - 0.04).abs() < 1e-12,
        "bundled input 0.04/M"
    );
    assert!(
        (priced.price.output - 0.64).abs() < 1e-12,
        "bundled output 0.64/M"
    );
    assert_eq!(priced.basis.rate_date, "2026-10-08");
    assert_eq!(priced.basis.basis, agenttrace_core::RateBasisArm::AtSession);
    assert!(
        !priced.basis.rate_source.starts_with("rate history"),
        "bundled arm keeps the catalog source label"
    );

    // A session predating the snapshot AND every observed state serves
    // the EARLIEST provable state (10-07: 0.018/1.28), disclosed as
    // newer than the session — closer to truth than the bundle, and
    // honest about the gap. This is the surface that priced silently
    // pre-change.
    let priced = agenttrace_core::lookup_priced_at(KEY, None, Some(ts("2026-09-01T12:00:00Z")));
    assert!((priced.price.input - 0.018).abs() < 1e-9);
    assert!((priced.price.output - 1.28).abs() < 1e-9);
    assert_eq!(priced.basis.rate_date, "2026-10-07");
    assert_eq!(
        priced.basis.basis,
        agenttrace_core::RateBasisArm::NewerThanSession
    );
    assert!(priced.basis.rate_source.starts_with("rate history"));
}

#[test]
fn no_session_time_serves_the_newest_known_state() {
    let priced = agenttrace_core::lookup_priced_at(KEY, None, None);
    assert!((priced.price.input - 0.0119).abs() < 1e-12);
    assert!((priced.price.output - 1.28).abs() < 1e-9);
    assert_eq!(priced.basis.rate_date, "2026-10-17");
}

#[test]
fn raw_id_keys_not_in_the_catalog_price_from_history() {
    // kilo/* has no LiteLLM rows: pre-rm-1965 these sessions silently
    // billed at the built-in fallback. Now the observed states serve,
    // with the basis disclosed.
    let priced = agenttrace_core::lookup_priced_at(RAW_KEY, None, Some(ts("2026-10-14T09:00:00Z")));
    assert!((priced.price.input - 0.0825).abs() < 1e-12);
    assert!((priced.price.output - 0.33).abs() < 1e-12);
    assert_eq!(priced.basis.rate_date, "2026-10-07");
    assert_eq!(priced.basis.basis, agenttrace_core::RateBasisArm::AtSession);
}

#[test]
fn audit_lane_prices_at_newest_and_discloses_against_session() {
    // The cost-audit drift recompute prices at CURRENT rates by design;
    // rm-1965 keeps that but discloses the arm honestly.
    let priced =
        agenttrace_core::rate_basis_against_session(KEY, None, Some(ts("2026-10-10T12:00:00Z")));
    assert!((priced.price.output - 1.28).abs() < 1e-9, "newest state");
    assert_eq!(
        priced.basis.basis,
        agenttrace_core::RateBasisArm::NewerThanSession,
        "audit admits the reprice postdates the session"
    );
    assert_eq!(priced.basis.rate_date, "2026-10-17");
}

#[test]
fn analyze_prices_at_session_time_and_serializes_the_basis() {
    // 1000 in + 1000 out tokens at 0.04/0.64 USD per million (the
    // bundled 10-08 state — the newest at the 10-10 session time):
    // 1000/1e6 * 0.04 + 1000/1e6 * 0.64 = 0.00068 → round4 = 0.0007.
    let events = vec![Event {
        timestamp: "2026-10-10T12:00:00Z".to_string(),
        usage: BTreeMap::from([
            ("input_tokens".to_string(), 1000),
            ("output_tokens".to_string(), 1000),
        ]),
        source_tool: "generic".to_string(),
        ..Event::default()
    }];
    let metrics: Metrics = analyze(&events, KEY);
    assert!(
        (metrics.cost_estimated - 0.0007).abs() < 1e-9,
        "{}",
        metrics.cost_estimated
    );
    assert_eq!(metrics.rate_date.as_deref(), Some("2026-10-08"));
    assert_eq!(metrics.rate_basis.as_deref(), Some("at_session"));
    assert!(metrics.rate_source.as_deref().unwrap().contains("snapshot"));

    // The pre-edit red from the selection test, end to end: a session on
    // the 14th prices at the 10-12 observed state (0.0128 + 0.768 per
    // million → 1000/1e6 * (0.0128 + 0.768) = 0.0007808 → round4 0.0008),
    // and the disclosure names the observed-state source.
    let events = vec![Event {
        timestamp: "2026-10-14T12:00:00Z".to_string(),
        usage: BTreeMap::from([
            ("input_tokens".to_string(), 1000),
            ("output_tokens".to_string(), 1000),
        ]),
        source_tool: "generic".to_string(),
        ..Event::default()
    }];
    let metrics: Metrics = analyze(&events, KEY);
    assert!(
        (metrics.cost_estimated - 0.0008).abs() < 1e-9,
        "{}",
        metrics.cost_estimated
    );
    assert_eq!(metrics.rate_date.as_deref(), Some("2026-10-12"));
    assert_eq!(metrics.rate_basis.as_deref(), Some("at_session"));
    assert!(metrics
        .rate_source
        .as_deref()
        .unwrap()
        .starts_with("rate history"));

    // The disclosure serializes additively — `rate_date` first appears as
    // a serialized string on exactly the rm-1965 fields.
    let value = serde_json::to_value(&metrics).expect("Metrics serializes");
    assert_eq!(value["rate_date"], serde_json::json!("2026-10-12"));
    assert_eq!(value["rate_basis"], serde_json::json!("at_session"));
    assert!(value["rate_source"]
        .as_str()
        .unwrap()
        .starts_with("rate history"));
}

#[test]
fn keys_without_history_keep_the_snapshot_date_disclosure() {
    // gpt-5 is catalog-served with no recorded edit: rate_date is the
    // bundled snapshot date and the arm tells the truth about sessions
    // that predate it.
    let priced = agenttrace_core::lookup_priced_at("gpt-5", None, Some(ts("2026-10-15T00:00:00Z")));
    assert_eq!(priced.basis.rate_date, "2026-10-08");
    assert_eq!(priced.basis.basis, agenttrace_core::RateBasisArm::AtSession);
    let priced = agenttrace_core::lookup_priced_at("gpt-5", None, Some(ts("2026-09-01T00:00:00Z")));
    assert_eq!(
        priced.basis.basis,
        agenttrace_core::RateBasisArm::NewerThanSession
    );
    // Unknown keys (no catalog row, no history) disclose the snapshot
    // date too — single-date rows span PRICING_SNAPSHOT_DATE.
    let priced = agenttrace_core::lookup_priced_at(
        "totally-unknown-model",
        None,
        Some(ts("2026-10-15T00:00:00Z")),
    );
    assert_eq!(priced.basis.rate_date, "2026-10-08");
    assert_eq!(priced.basis.basis, agenttrace_core::RateBasisArm::AtSession);
}

//! rm-2013 — hostile naive-extreme-year timestamps must not panic the
//! process. Chrono 0.4.45's parsers accept expanded-year strings
//! (`-262143-01-01T00:00:00` parses via the naive `%Y` arm; chrono's
//! RFC-3339 arm accepts `+262143-…`), and downstream window arithmetic
//! (`quota_window`'s `cutoff - span`, governance's commit-window
//! bounds) panics with `DateTime - TimeDelta overflowed` on those
//! edges. Every real journal emits 4-digit ISO-profile years, so
//! `parse_ts` rejects anything outside `0000..=9999` as unplaceable —
//! the corpus-disclosure counters (quota attribution census) already
//! own the "row carries no placeable time" story.
//!
//! Red-first pins: the pre-fix tree returned `Some` for the MIN edge
//! and `quota_report` panicked (research PoC 0b22f8d0, /tmp/quota-probe
//! lineage — rebuilt here as an in-tree test so it never depends on a
//! /tmp artifact again).

use agenttrace_core::{parse_raw_session, parse_ts, quota_report};

fn parse_one(line: &str) -> agenttrace_core::Session {
    parse_raw_session("t.jsonl", "t.jsonl", line).unwrap()
}

// ------------------------------------------------------------- parse bound

#[test]
fn naive_negative_expanded_year_is_rejected() {
    // The chrono-MIN edge: this string PARSES under chrono's naive
    // `%Y` (one-sided hole — positive expanded years are rejected by
    // `%Y` without a sign). Pre-fix it returned Some and panicked the
    // quota window arithmetic.
    assert!(parse_ts("-262143-01-01T00:00:00").is_none());
}

#[test]
fn rfc3339_positive_expanded_year_is_rejected() {
    // chrono's RFC-3339 arm is the opposite side: it accepts the
    // signed positive form.
    assert!(parse_ts("+262143-12-31T23:59:59+00:00").is_none());
    assert!(parse_ts("+99999-01-01T00:00:00Z").is_none());
}

#[test]
fn four_digit_year_boundaries_still_parse() {
    // The bound is inclusive of the whole 4-digit range: nothing a
    // real journal emits changes meaning.
    assert!(parse_ts("0000-01-01T00:00:00").is_some());
    assert!(parse_ts("9999-12-31T23:59:59").is_some());
    assert!(parse_ts("2026-10-18T09:00:00Z").is_some());
    assert!(parse_ts("2026-10-18T09:00:00.123+02:00").is_some());
    // Unchanged rejections (shape, not range): empty stays None.
    assert!(parse_ts("").is_none());
}

// ----------------------------------------------------- end-to-end no-panic

#[test]
fn quota_report_survives_hostile_journal_timestamp() {
    // Research PoC shape: a claude-lane journal whose event carries the
    // chrono-MIN naive timestamp. Pre-fix, `quota_report` panicked at
    // the `cutoff - span` window bound; post-fix the row is simply
    // unplaceable and the corpus lane discloses it.
    let line = serde_json::json!({
        "type": "user",
        "cwd": "/tmp",
        "sessionId": "sess-hostile",
        "timestamp": "-262143-01-01T00:00:00",
        "message": { "role": "user", "content": "hello" },
    })
    .to_string();
    let session = parse_one(&line);
    let report = std::panic::catch_unwind(|| quota_report(std::slice::from_ref(&session)))
        .expect("quota_report must not panic on hostile timestamps");
    let unplaced = report["corpus"]["attribution"]["unplaced"]
        .as_u64()
        .unwrap_or(0);
    assert_eq!(
        unplaced, 1,
        "hostile-timestamp session is disclosed as unplaced, not billed"
    );
    assert!(
        report["corpus"]["cutoff_utc"].is_null(),
        "a hostile-only corpus must not mint a cutoff"
    );
    assert!(report["five_hour"].is_null() && report["weekly"].is_null());
}

#[test]
fn quota_report_places_valid_events_alongside_hostile_ones() {
    // Mixed corpus: the hostile row is disclosed unplaced, the valid
    // row still carries the cutoff and the windows — the guard must
    // not eat real time placement.
    let hostile = serde_json::json!({
        "type": "user",
        "cwd": "/tmp",
        "sessionId": "sess-hostile",
        "timestamp": "-262143-01-01T00:00:00",
        "message": { "role": "user", "content": "hostile" },
    })
    .to_string();
    let valid = serde_json::json!({
        "type": "user",
        "cwd": "/tmp",
        "sessionId": "sess-valid",
        "timestamp": "2026-10-18T09:00:00Z",
        "message": { "role": "user", "content": "valid" },
    })
    .to_string();
    let sessions = [parse_one(&valid), parse_one(&hostile)];
    let report = std::panic::catch_unwind(|| quota_report(&sessions))
        .expect("quota_report must not panic on a mixed corpus");
    assert!(report["corpus"]["cutoff_utc"].is_string());
    assert_eq!(
        report["corpus"]["attribution"]["unplaced"].as_u64(),
        Some(1)
    );
    assert_eq!(
        report["corpus"]["attribution"]["event_timestamps"].as_u64(),
        Some(1)
    );
}

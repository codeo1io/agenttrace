//! rm-2031 contract (run e40a66d3 cycle 6): a hostile past-extreme
//! event timestamp must never panic a quota lane. This is the
//! integration twin of the `quota_extreme_event_timestamps_never_panic`
//! unit pin — it drives the real parse path with the preserved 94-byte
//! PoC corpus (see `fixtures/quota-past-extreme-timestamp.jsonl`).
//!
//! Red-first at wall f0649493: `agenttrace --quota -f json` on this
//! fixture panicked `called Result::unwrap() on an Err value: DateTime -
//! TimeDelta overflowed` (lib.rs:3191). The cure saturates the window
//! start at the chrono floor instead.
//!
//! COMPOSED (2026-10-21, conflict case 76618454) with rm-2013 (run
//! 91066ba0 cycle 2, landed on the wall in parallel): the wall's
//! 4-digit-year admission bound now refuses this corpus's expanded-year
//! anchor at `parse_ts`, so the panic is closed BY CONSTRUCTION before
//! the window math ever runs — the row is disclosed unplaced, the
//! corpus mints no cutoff, and both windows render null. The rm-2031
//! saturation stays as the same victim's second belt and is exercised
//! directly by the unit pin; this contract keeps owning the end-to-end
//! parse-path truth over the byte-preserved PoC.

use agenttrace_core::{parse_raw_session, quota_report};

#[test]
fn quota_lane_survives_past_extreme_event_timestamp_from_disk() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/quota-past-extreme-timestamp.jsonl"
    );
    let raw = std::fs::read_to_string(fixture).expect("fixture readable");
    let session = parse_raw_session("quota-past-extreme", "hostile.jsonl", &raw)
        .expect("hostile-but-parseable journal parses (lane must not crash downstream)");
    let sessions = vec![session];
    // rm-2013 admission (composed): the past-extreme anchor is
    // unplaceable, so no window exists — the corpus lane discloses the
    // row unplaced instead of billing it, and never panics. Finite
    // money (rm-2031) is pinned by the unit pin's saturation arm.
    let report = quota_report(&sessions);
    assert!(
        report["corpus"]["cutoff_utc"].is_null(),
        "an extreme-only corpus must not mint a cutoff"
    );
    assert_eq!(
        report["corpus"]["attribution"]["unplaced"].as_u64(),
        Some(1),
        "the hostile row is disclosed unplaced, not billed"
    );
    assert!(report["five_hour"].is_null() && report["weekly"].is_null());
    // And the full JSON lane round-trips.
    let text = serde_json::to_string(&report).expect("serializable");
    assert!(!text.contains("null payload"));
}

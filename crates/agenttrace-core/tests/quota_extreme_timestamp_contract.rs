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
    // The event timestamp is placeable (floor), so a window exists; it
    // must render finite money — never NaN/inf, never a panic.
    let report = quota_report(&sessions);
    for window in ["five_hour", "weekly"] {
        if let Some(node) = report.get(window) {
            if let Some(spend) = node.get("spend_usd").and_then(|v| v.as_f64()) {
                assert!(spend.is_finite(), "{window} spend finite, got {spend}");
            }
        }
    }
    // And the full JSON lane round-trips.
    let text = serde_json::to_string(&report).expect("serializable");
    assert!(!text.contains("null payload"));
}

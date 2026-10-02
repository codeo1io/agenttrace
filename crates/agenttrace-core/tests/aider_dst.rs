//! rm-356 (cycle-3, run 27fdc908): DST fold/gap determinism for aider
//! timestamps. Integration test on its own test binary so the TZ env
//! mutation below cannot race any other test process; this file must keep
//! exactly ONE `#[test]`.
//!
//! Before the fix, `aider_time` used `.and_local_timezone(Local).single()`
//! and `unwrap_or_default()`: the fold instant 2026-11-01 01:30:00
//! (America/New_York, ambiguous) and the gap instant 2026-03-08 02:30:00
//! (nonexistent) both returned None, emitting an EMPTY timestamp -- blank
//! temporal columns and zero durations, silently.
//!
//! Policy under test (documented in aider_time): fold maps to the EARLIEST
//! occurrence (EDT), gap shifts forward one hour to the first representable
//! instant. The metrics path re-renders timestamps in UTC (Z suffix), so the
//! policy is asserted as exact UTC instants: the fold earliest (EDT) is
//! 05:30Z while the latest (EST) would be 06:30Z, and the empty-string
//! collapse of the defect would render neither.

use agenttrace_core::parse_raw_session;

fn session_start_for(header_time: &str) -> String {
    let raw = format!("# aider chat started at {header_time}\n#### hello\nhi\n");
    let session =
        parse_raw_session("chat.txt", "chat.txt", &raw).expect("aider fixture must parse");
    session.metrics.session_start
}

#[test]
fn aider_dst_fold_and_gap_render_deterministic_offsets() {
    // Pinned zone: transition semantics are the whole point. Set TZ before
    // the first chrono::Local use in this process.
    std::env::set_var("TZ", "America/New_York");

    let control = session_start_for("2026-06-15 12:00:00");
    assert_eq!(
        control, "2026-06-15T16:00:00Z",
        "June control must render 12:00 EDT == 16:00Z (proves TZ pinning); got {control:?}"
    );

    let fold = session_start_for("2026-11-01 01:30:00");
    assert!(
        !fold.is_empty(),
        "fold instant must not collapse to an empty timestamp (the rm-356 defect)"
    );
    assert_eq!(
        fold, "2026-11-01T05:30:00Z",
        "fold must map to the EARLIEST occurrence (01:30 EDT == 05:30Z; the latest EST would be 06:30Z); got {fold:?}"
    );

    let gap = session_start_for("2026-03-08 02:30:00");
    assert!(
        !gap.is_empty(),
        "gap instant must not collapse to an empty timestamp (the rm-356 defect)"
    );
    assert_eq!(
        gap, "2026-03-08T07:30:00Z",
        "gap must shift forward one hour to the first representable instant (03:30 EDT == 07:30Z); got {gap:?}"
    );

    // Unparseable input keeps the historical empty-string fallback.
    assert_eq!(session_start_for("not a timestamp"), "");
}

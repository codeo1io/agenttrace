//! rm-883 (fmt_duration minute-carry half of the journal-to-report
//! truthfulness trio): the minutes arm rounded `s/60.0` at one decimal with
//! no carry, so durations in `[3599.5s, 3600s)` rendered as `60.0m` — a
//! minutes figure of 60 in a field that promises `< 1h` — instead of
//! carrying into the hours arm. These boundary pins hold the carry.

use agenttrace_core::fmt_duration;

#[test]
fn minute_rounding_carries_into_the_hour_arm() {
    // 3599.4s = 59.99m rounds to 60.0m at one decimal: must carry.
    assert_eq!(fmt_duration(3599.4), "1h 0m");
    // 3599.8s = 59.9967m: the assess PoC's live 60.0m case.
    assert_eq!(fmt_duration(3599.8), "1h 0m");
    // Exactly one hour rides the native hours arm.
    assert_eq!(fmt_duration(3600.0), "1h 0m");
}

#[test]
fn non_carrying_boundaries_keep_their_arm() {
    // 3593.0s = 59.883m -> 59.9m: no carry, minutes arm stays.
    assert_eq!(fmt_duration(3593.0), "59.9m");
    // 125s = 2.0833m -> 2.1m (existing shape, regression guard).
    assert_eq!(fmt_duration(125.0), "2.1m");
    // 3660s -> 1h 1m (existing shape).
    assert_eq!(fmt_duration(3660.0), "1h 1m");
    // 25h boundary math (existing shape).
    assert_eq!(fmt_duration(25.0 * 3600.0), "25h 0m");
}

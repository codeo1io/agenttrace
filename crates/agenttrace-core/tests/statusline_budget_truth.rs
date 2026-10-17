//! Statusline budget truth — red-first battery for the cycle-2 batch
//! "Statusline budget truth: clamp-and-disclose hostile capture costs and
//! unparsable journal lines" (run 80336dfb, campaign mint rm-1503,
//! repository-maintenance 34a54fc8; title-keyed per the stewardship
//! request — the numeral reconciles at the commit gate).
//!
//! Stage A (behavioral, was red on the pre-fix tree — 4/4 failures,
//! log /tmp/at-impl-d99c1f27/red-stageA2.log): a hostile negative
//! `cost.total_cost_usd` capture must never drive the 7d total negative
//! nor buy budget headroom.
//!
//! Stage B (disclosure, compiles only against the fixed API): the
//! discard is counted (`negative_capture_cost_samples` /
//! `invalid_capture_cost_samples`), surfaced in both render arms, and
//! an unparsable journal line (serde rejects a literal `1e400`) is
//! counted in the `unparsable_lines` stats census. Note the rm-771
//! discipline the render honors: a session first observed with a
//! nonzero cumulative contributes only its intra-day delta, so a
//! healthy corpus here uses two same-day samples to make the rise
//! observable.
//!
//! `captured_at` is epoch SECONDS (`utc_day` feeds `timestamp_opt(epoch, 0)`)
//! — the assess PoC shape; millis land in year 57779 and miss the window.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

use agenttrace_core::{
    render_budget_view, render_statusline_report, statusline_budget_series,
    statusline_journal_stats, CapturedStatusline,
};
use chrono::Utc;
use serde_json::{json, Value};

/// Serializes the tests that mutate process-global environment state
/// (`AGENTTRACE_SESSION_CACHE_DIR`): cargo runs #[test] fns on threads
/// inside one process, so the env-touching tests must never overlap.
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Restores `AGENTTRACE_SESSION_CACHE_DIR` even when the guarded test
/// panics, so a failure cannot leak the override into later tests.
struct EnvRestore(Option<std::ffi::OsString>);
impl EnvRestore {
    fn set(dir: &Path) -> Self {
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", dir);
        EnvRestore(prior)
    }
}
impl Drop for EnvRestore {
    fn drop(&mut self) {
        match self.0.take() {
            Some(v) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", v),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
    }
}

/// The assess PoC capture: a hostile editor hook / corrupted plugin
/// payload with a negative `total_cost_usd` (pre-fix live render:
/// `7d total $-50.00` on a $10 budget, `$60.00 remaining`).
fn hostile_negative_capture(now_s: i64) -> CapturedStatusline {
    CapturedStatusline {
        captured_at: now_s,
        payload: json!({
            "session_id": "neg-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": -50.0 },
        }),
    }
}

/// A benign cumulative capture so a corpus still has a foldable day.
fn healthy_capture(now_s: i64, session: &str, cost: f64) -> CapturedStatusline {
    CapturedStatusline {
        captured_at: now_s,
        payload: json!({
            "session_id": session,
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": cost },
        }),
    }
}

/// The assess F3 PoC capture: `1e400` cannot be represented as a JSON
/// number, so the cost node arrives as `null` in the value tree and the
/// capture contributed nothing to any budget surface with zero
/// disclosure (`"lines": 1, "daily": [], captures_unknown_payload: 0`).
fn non_representable_cost_capture(now_s: i64) -> CapturedStatusline {
    // serde_json itself rejects a literal 1e400 at line-parse time (it
    // is not representable as f64); such lines are disclosed by the
    // journal-stats `unparsable_lines` census. For the SERIES-level
    // invalid-sample counter the reachable shape is a cost node that
    // parses but is not a finite number — JSON `null`.
    CapturedStatusline {
        captured_at: now_s,
        payload: json!({
            "session_id": "inf-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": null },
        }),
    }
}

// ---------------------------------------------------------------- Stage A

#[test]
fn hostile_negative_total_is_never_negative() {
    let now = Utc::now();
    let captures = [hostile_negative_capture(now.timestamp())];
    let series = statusline_budget_series(&captures, 7, now);
    assert!(
        series.total >= 0.0,
        "a negative capture cost must not drive the 7d total negative: total = {}",
        series.total
    );
}

#[test]
fn hostile_negative_day_samples_never_negative() {
    let now = Utc::now();
    let captures = [hostile_negative_capture(now.timestamp())];
    let series = statusline_budget_series(&captures, 7, now);
    let negative_days: Vec<_> = series
        .daily
        .iter()
        .filter(|(_, sample)| *sample < 0.0)
        .collect();
    assert!(
        negative_days.is_empty(),
        "negative capture cost must not appear as a negative day sample: {negative_days:?}"
    );
}

#[test]
fn hostile_negative_never_buys_budget_headroom() {
    let now = Utc::now();
    let captures = [hostile_negative_capture(now.timestamp())];
    let series = statusline_budget_series(&captures, 7, now);
    let budget = 10.0;
    let remaining = budget - series.total;
    assert!(
        remaining <= budget + 1e-9,
        "remaining must never exceed the budget through a negative total: remaining = {remaining}"
    );
}

// ---------------------------------------------------------------- Stage B

#[test]
fn non_representable_cost_is_disclosed_not_silent() {
    let now = Utc::now();
    let captures = [non_representable_cost_capture(now.timestamp())];
    let series = statusline_budget_series(&captures, 7, now);
    // The 1e400 capture is discarded from the math (correct), but the
    // discard must be VISIBLE: the series exposes it as an
    // invalid-cost sample count. Pre-fix this battery could not even
    // compile; post-fix it must be exactly one.
    assert_eq!(series.invalid_capture_cost_samples, 1);
    assert_eq!(series.negative_capture_cost_samples, 0);
    assert_eq!(series.total, 0.0);
}

#[test]
fn negative_and_invalid_samples_are_counted_separately() {
    let now = Utc::now();
    let t = now.timestamp();
    let captures = [
        hostile_negative_capture(t),
        hostile_negative_capture(t),
        non_representable_cost_capture(t),
        healthy_capture(t, "ok-sess", 2.0),
        healthy_capture(t, "ok-sess", 2.5),
    ];
    let series = statusline_budget_series(&captures, 7, now);
    assert_eq!(series.negative_capture_cost_samples, 2);
    assert_eq!(series.invalid_capture_cost_samples, 1);
    // The healthy session still folds: two same-day cumulatives make
    // the intra-day rise (2.0 -> 2.5) observable under rm-771's
    // first-seen discipline.
    assert!(
        (series.total - 0.5).abs() < 1e-9,
        "total = {}",
        series.total
    );
}

#[test]
fn budget_text_arm_discloses_and_never_prints_headroom() {
    let _env = env_lock();
    let dir = std::env::temp_dir().join(format!("at-budget-truth-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp cache dir");
    let journal = dir.join("statusline.jsonl");
    let t = Utc::now().timestamp();
    let neg = json!({
        "captured_at": t,
        "payload": {
            "session_id": "neg-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": -50.0 },
        },
    });
    let ok_a = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.0 },
        },
    });
    let ok_b = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.5 },
        },
    });
    std::fs::write(&journal, format!("{neg}\n{ok_a}\n{ok_b}\n")).expect("plant journal");
    let _restore = EnvRestore::set(&dir);

    let out = render_budget_view("text", Some(10.0)).expect("budget view renders");
    let parsed: Value =
        serde_json::from_str(&render_budget_view("json", Some(10.0)).expect("json arm renders"))
            .expect("json arm is valid JSON");
    std::fs::remove_dir_all(&dir).ok();

    // Text arm: the PoC bug must be gone …
    assert!(
        !out.contains("$-50.00"),
        "a discarded hostile sample must not appear as a negative 7d total\n{out}"
    );
    assert!(
        !out.contains("$60.00 remaining"),
        "a negative capture must never render headroom above the budget\n{out}"
    );
    // … the healthy session's intra-day rise ($2.00 -> $2.50) still
    // folds: $0.50 spent, $9.50 of $10 remaining …
    assert!(
        out.contains("budget $10.00 — $9.50 remaining"),
        "expected the healthy-sample budget line\n{out}"
    );
    // … and the discard is disclosed, not silent.
    assert!(
        out.contains("1 hostile cost sample discarded (1 negative, 0 not a finite number)"),
        "the text arm must disclose the discarded sample\n{out}"
    );
    // JSON arm carries the counters too.
    assert_eq!(parsed["negative_capture_cost_samples"], 1);
    assert_eq!(parsed["invalid_capture_cost_samples"], 0);
}

#[test]
fn journal_stats_count_unparsable_lines() {
    let _env = env_lock();
    let dir = std::env::temp_dir().join(format!("at-budget-truth-stats-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp cache dir");
    let journal = dir.join("statusline.jsonl");
    let t = Utc::now().timestamp();
    let good = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 1.0 },
        },
    });
    // One torn line, one literal 1e400 line (serde_json rejects it at
    // parse time — the assess F3 shape): both must land in the census.
    std::fs::write(
        &journal,
        format!("{good}\n{{this is not json\n{{\"captured_at\":{t},\"payload\":{{\"session_id\":\"inf-sess\",\"cost\":{{\"total_cost_usd\":1e400}}}}}}\n"),
    )
    .expect("plant journal");
    let _restore = EnvRestore::set(&dir);

    let stats = statusline_journal_stats(&journal);
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(
        stats.unparsable_lines, 2,
        "the torn line and the 1e400 line must both be counted"
    );
    assert_eq!(stats.lines, 3, "the census covers every non-empty line");
}

// ---------------------------------------------------------------- Stage C
// Completion turn (attempt 9a1b7012, adopting reaped attempt d99c1f27's
// batch): the `remaining_usd` floor and the --statusline-report budget
// disclosures (stewardship surfaces 5 and 7's floor half).

/// Over-budget corpus: a healthy $2.00 -> $2.50 same-day rise against a
/// $1.00 budget — `remaining_usd` must floor to 0, never render the
/// pre-fix negative (and never exceed the budget).
#[test]
fn remaining_usd_is_floored_into_the_budget_range() {
    let _env = env_lock();
    let dir = std::env::temp_dir().join(format!("at-budget-truth-floor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp cache dir");
    let journal = dir.join("statusline.jsonl");
    let t = Utc::now().timestamp();
    let ok_a = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 1.0 },
        },
    });
    let ok_b = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.0 },
        },
    });
    std::fs::write(&journal, format!("{ok_a}\n{ok_b}\n")).expect("plant journal");
    let _restore = EnvRestore::set(&dir);

    let budget: Value = serde_json::from_str(
        &render_budget_view("json", Some(1.0)).expect("budget json arm renders"),
    )
    .expect("budget arm is valid JSON");
    assert_eq!(
        budget["remaining_usd"], 0.0,
        "over-spend must floor remaining_usd to 0, not a negative: {budget}"
    );

    let report: Value = serde_json::from_str(
        &render_statusline_report("json", Some(1.0)).expect("report json arm renders"),
    )
    .expect("report arm is valid JSON");
    assert_eq!(
        report["budget"]["remaining_usd"], 0.0,
        "the report budget block must floor remaining_usd too: {report}"
    );
    // A hostile `--weekly-budget -5` must not panic the floor either.
    let hostile_budget: Value = serde_json::from_str(
        &render_budget_view("json", Some(-5.0)).expect("negative budget still renders"),
    )
    .expect("negative budget arm is valid JSON");
    assert_eq!(
        hostile_budget["remaining_usd"], 0.0,
        "a negative budget must floor to 0, not panic or mirror: {hostile_budget}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The --statusline-report arms carry the same hostile-sample and
/// unparsable-line disclosures the --budget view carries (stewardship
/// surface 5), so no budget surface can drop a capture silently.
#[test]
fn report_arms_disclose_hostile_samples_and_unparsable_lines() {
    let _env = env_lock();
    let dir = std::env::temp_dir().join(format!("at-budget-truth-report-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp cache dir");
    let journal = dir.join("statusline.jsonl");
    let t = Utc::now().timestamp();
    let neg = json!({
        "captured_at": t,
        "payload": {
            "session_id": "neg-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": -50.0 },
        },
    });
    let ok_a = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.0 },
        },
    });
    let ok_b = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.5 },
        },
    });
    std::fs::write(&journal, format!("{neg}\n{ok_a}\n{ok_b}\n{{torn\n")).expect("plant journal");
    let _restore = EnvRestore::set(&dir);

    let report_json: Value = serde_json::from_str(
        &render_statusline_report("json", Some(10.0)).expect("report json arm renders"),
    )
    .expect("report arm is valid JSON");
    let report_text =
        render_statusline_report("text", Some(10.0)).expect("report text arm renders");
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(
        report_json["budget"]["negative_capture_cost_samples"], 1,
        "the report budget block must carry the hostile-sample counter: {report_json}"
    );
    assert_eq!(
        report_json["budget"]["invalid_capture_cost_samples"], 0,
        "no non-finite sample exists in this corpus: {report_json}"
    );
    assert_eq!(
        report_json["budget"]["spend_7d_usd"], 0.5,
        "the healthy intra-day rise still folds: {report_json}"
    );
    assert_eq!(
        report_json["journal"]["unparsable_lines"], 1,
        "the torn line must be visible in the journal stats: {report_json}"
    );
    assert!(
        report_text.contains("1 unparsable journal line —"),
        "the report text arm must disclose the torn line\n{report_text}"
    );
    assert!(
        report_text.contains(
            "Budget: 1 hostile cost sample discarded (1 negative, 0 not a finite number)"
        ),
        "the report text arm must disclose the hostile sample\n{report_text}"
    );
    assert!(
        !report_text.contains("$-50.00"),
        "a discarded hostile sample must not appear as a negative spend\n{report_text}"
    );
}

// ---------------------------------------------------------------- Stage D
// Adoption-turn additions (attempt 8a9b0344, adopting the combined
// d99c1f27 + 9a1b7012 delta): the two remaining stewardship
// surface-8 pins — non-numeric cost nodes (the reachable "NaN-like"
// shapes: JSON cannot carry a bare NaN or ±inf literal, so a string or
// bool node is how one arrives) and the zero-noise healthy corpus.

#[test]
fn non_numeric_cost_nodes_are_invalid_samples() {
    let now = Utc::now();
    let t = now.timestamp();
    let string_cost = CapturedStatusline {
        captured_at: t,
        payload: json!({
            "session_id": "str-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": "50.00" },
        }),
    };
    let bool_cost = CapturedStatusline {
        captured_at: t,
        payload: json!({
            "session_id": "bool-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": true },
        }),
    };
    let series = statusline_budget_series(&[string_cost, bool_cost], 7, now);
    assert_eq!(
        series.invalid_capture_cost_samples, 2,
        "a string/bool cost node is a present-but-unusable sample: disclosed, not silent"
    );
    assert_eq!(series.negative_capture_cost_samples, 0);
    assert_eq!(series.total, 0.0);
}

/// Zero-noise: a corpus with no hostile samples and no torn lines
/// renders with none of the new disclosure markers. The additions are
/// strictly conditional, so healthy output stays byte-identical to the
/// pre-batch render — full-string identity is pinned by the pre-existing
/// in-file tests (`budget_view_renders_spend_budget_and_unconfigured_state`
/// et al.), which pass unmodified on this tree.
#[test]
fn healthy_corpus_renders_with_zero_disclosure_noise() {
    let _env = env_lock();
    let dir = std::env::temp_dir().join(format!("at-budget-truth-clean-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp cache dir");
    let journal = dir.join("statusline.jsonl");
    let t = Utc::now().timestamp();
    let ok_a = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.0 },
        },
    });
    let ok_b = json!({
        "captured_at": t,
        "payload": {
            "session_id": "ok-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.5 },
        },
    });
    std::fs::write(&journal, format!("{ok_a}\n{ok_b}\n")).expect("plant journal");
    let _restore = EnvRestore::set(&dir);

    let budget_text = render_budget_view("text", Some(10.0)).expect("budget text renders");
    let report_text = render_statusline_report("text", Some(10.0)).expect("report text renders");
    let budget_json: Value =
        serde_json::from_str(&render_budget_view("json", Some(10.0)).expect("budget json renders"))
            .expect("budget arm is valid JSON");
    let report_json: Value = serde_json::from_str(
        &render_statusline_report("json", Some(10.0)).expect("report json renders"),
    )
    .expect("report arm is valid JSON");
    std::fs::remove_dir_all(&dir).ok();

    for (label, out) in [
        ("--budget", &budget_text),
        ("--statusline-report", &report_text),
    ] {
        assert!(
            !out.contains("discarded"),
            "no hostile sample exists — {label} must carry no discard line\n{out}"
        );
        assert!(
            !out.contains("unparsable"),
            "no torn line exists — {label} must carry no unparsable line\n{out}"
        );
        assert!(
            !out.contains("$-"),
            "a healthy corpus never renders a negative dollar figure in {label}\n{out}"
        );
    }
    assert_eq!(budget_json["negative_capture_cost_samples"], 0);
    assert_eq!(budget_json["invalid_capture_cost_samples"], 0);
    assert_eq!(report_json["journal"]["unparsable_lines"], 0);
    assert_eq!(report_json["budget"]["negative_capture_cost_samples"], 0);
    assert_eq!(report_json["budget"]["invalid_capture_cost_samples"], 0);
    // The healthy intra-day rise ($2.00 -> $2.50) still folds exactly.
    assert!(
        budget_text.contains("budget $10.00 — $9.50 remaining"),
        "the healthy budget line must be unchanged\n{budget_text}"
    );
}

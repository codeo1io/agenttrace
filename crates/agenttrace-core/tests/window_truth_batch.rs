//! Window-truth batch (run 6603da8916bd, repository-maintenance cycle 1).
//!
//! Contract tests for the three arms of the selected implementation
//! batch ("truthful tokens, bounded windows"):
//!
//! * rm-619 — opencode message-level `tokens.reasoning` must fold on
//!   the reasoning channel AND must never count as "the message
//!   carried usage": a reasoning-only tokens object leaves the
//!   part-level step-finish fallback live instead of suppressing it.
//! * rm-941 — future-dated samples (beyond now+1h) are excluded from
//!   every time-windowed series and the exclusion is disclosed
//!   (statusline budget window, overview card/svg/json/text).
//! * rm-917 residual — the `--statusline-report` render surface
//!   carries the same calendar-window truth (daily +
//!   excluded_older_days + excluded_future_days) as `--budget`.

use agenttrace_core::{
    parse_file, render_budget_view, render_statusline_report, report_overview_json_with_context,
    report_overview_text_with_context, statusline_budget_series, CapturedStatusline, DataHealth,
    Overview, TimeRange,
};
use serde_json::Value;

// ---------- shared fixture scaffolding ----------

fn scratch_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("at-window-truth-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Writes an opencode storage session triple (session + message +
/// parts) and returns the session-file path. `message_tokens` is the
/// raw JSON for the message-level `tokens` object (None omits it).
fn opencode_session(
    root: &std::path::Path,
    message_tokens: Option<&str>,
    with_step_finish: bool,
) -> std::path::PathBuf {
    let session_dir = root.join("opencode/storage/session/proj1");
    let message_dir = root.join("opencode/storage/message/sess-r19");
    let part_dir = root.join("opencode/storage/part/msg_1");
    std::fs::create_dir_all(&session_dir).expect("dirs");
    std::fs::create_dir_all(&message_dir).expect("dirs");
    std::fs::create_dir_all(&part_dir).expect("dirs");

    std::fs::write(
        session_dir.join("sess-r19.json"),
        r#"{"id":"sess-r19","projectID":"proj1","title":"r19","time":{"created":"2026-10-09T10:00:00.000Z"}}"#,
    )
    .expect("session");

    let tokens_json = match message_tokens {
        Some(raw) => format!("\"tokens\":{raw},"),
        None => String::new(),
    };
    std::fs::write(
        message_dir.join("msg_1.json"),
        format!(r#"{{"id":"msg_1","role":"assistant","time":{{"created":"2026-10-09T10:00:05.000Z"}},{tokens_json}"modelID":"claude-sonnet-4-5"}}"#),
    )
    .expect("message");

    std::fs::write(
        part_dir.join("part_0.json"),
        r#"{"id":"part_0","type":"text","text":"assistant answer","time":{"created":"2026-10-09T10:00:05.500Z"}}"#,
    )
    .expect("text part");

    if with_step_finish {
        std::fs::write(
            part_dir.join("part_1.json"),
            r#"{"id":"part_1","type":"step-finish","time":{"created":"2026-10-09T10:00:06.000Z"},"tokens":{"input":100,"output":50}}"#,
        )
        .expect("step-finish part");
    }

    session_dir.join("sess-r19.json")
}

/// A claude-format session jsonl with a single usage record at
/// `timestamp` — used to build `Session`s for the reports arms.
fn claude_session(root: &std::path::Path, timestamp: &str) -> std::path::PathBuf {
    std::fs::create_dir_all(root).expect("dirs");
    let path = root.join(format!(
        "s-{}.jsonl",
        timestamp
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
    ));
    let record = serde_json::json!({
        "type": "assistant",
        "sessionId": "future-card",
        "timestamp": timestamp,
        "message": {
            "model": "claude-sonnet-4-5",
            "usage": {"input_tokens": 10, "output_tokens": 5},
        },
    });
    std::fs::write(&path, format!("{}\n", record)).expect("jsonl");
    path
}

// ---------- rm-619: opencode reasoning + fallback ----------

#[test]
fn reasoning_only_message_tokens_do_not_suppress_step_finish_usage() {
    let root = scratch_dir("rm619-twin");
    let path = opencode_session(&root, Some(r#"{"reasoning":500}"#), true);
    let session = parse_file(&path).expect("parses");

    // The twin oracle: the step-finish part's usage must survive the
    // reasoning-only message-level tokens object (0 in / estimate out
    // was the suppressed behavior this test pins away).
    assert_eq!(
        session.metrics.tokens_input, 100,
        "input must come from the step-finish part"
    );
    assert_eq!(
        session.metrics.tokens_output, 50,
        "output must come from the step-finish part"
    );
    // And the reasoning itself is folded on its own channel.
    assert_eq!(
        session.metrics.tokens_reasoning, 500,
        "reasoning must be disclosed"
    );
}

#[test]
fn control_without_message_tokens_is_unchanged() {
    let root = scratch_dir("rm619-control");
    let path = opencode_session(&root, None, true);
    let session = parse_file(&path).expect("parses");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 50);
    assert_eq!(session.metrics.tokens_reasoning, 0);
}

#[test]
fn message_with_full_usage_still_owns_usage_no_double_count() {
    let root = scratch_dir("rm619-owner");
    let path = opencode_session(
        &root,
        Some(r#"{"input":10,"output":5,"reasoning":7}"#),
        true,
    );
    let session = parse_file(&path).expect("parses");
    // Message-level usage owns the record; the part fallback must NOT
    // stack on top of it.
    assert_eq!(session.metrics.tokens_input, 10);
    assert_eq!(session.metrics.tokens_output, 5);
    assert_eq!(session.metrics.tokens_reasoning, 7);
}

#[test]
fn reasoning_without_step_finish_still_folds() {
    let root = scratch_dir("rm619-fold");
    let path = opencode_session(&root, Some(r#"{"reasoning":500}"#), false);
    let session = parse_file(&path).expect("parses");
    assert_eq!(
        session.metrics.tokens_reasoning, 500,
        "reasoning-only tokens must still be accounted"
    );
}

// ---------- rm-941: future-dated samples are excluded + disclosed ----------

fn captures(now: chrono::DateTime<chrono::Utc>) -> Vec<CapturedStatusline> {
    // Cumulative per-session cost snapshots (the series bills the
    // day-over-day rise), monotone by construction.
    let day = |offset: i64| -> i64 { (now + chrono::Duration::days(offset)).timestamp() };
    [(-10i64, 4.0f64), (-9, 10.0), (-1, 11.0), (4000, 20.0)]
        .iter()
        .map(|(offset, cost)| CapturedStatusline {
            captured_at: day(*offset),
            payload: serde_json::json!({
                "session_id": "A",
                "cost": {"total_cost_usd": cost},
            }),
        })
        .collect()
}

#[test]
fn budget_series_excludes_and_discloses_future_days() {
    let now = chrono::Utc::now();
    let series = statusline_budget_series(&captures(now), 7, now);
    assert_eq!(series.excluded_older_days, 2, "stale days stay excluded");
    assert_eq!(
        series.excluded_future_days, 1,
        "the 4000-days-out capture is future-dated"
    );
    assert!(
        !series.daily.iter().any(|(day, _)| {
            let d = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").expect("day");
            d > (now + chrono::Duration::hours(1)).date_naive()
        }),
        "no day beyond now+1h may appear in the window: {:?}",
        series.daily
    );
    assert_eq!(series.total, 1.0, "only the in-window day bills");
}

#[test]
fn overview_card_and_arms_disclose_future_dated_sessions() {
    let root = scratch_dir("rm941-overview");
    // One real session (in the past) + one 2030 session.
    let past = parse_file(&claude_session(&root, "2026-10-09T10:00:00Z")).expect("past parses");
    let future = parse_file(&claude_session(&root, "2030-06-15T12:00:00Z")).expect("future parses");
    let sessions = vec![past, future];
    let health = DataHealth::default();

    let json = report_overview_json_with_context(
        &Overview::default(),
        &sessions,
        Some(&health),
        TimeRange::All,
        false,
        None,
        0,
    );
    let payload: Value = serde_json::from_str(&json).expect("valid json");
    assert_eq!(
        payload["future_dated_sessions_excluded"].as_u64(),
        Some(1),
        "json arm must disclose the exclusion: {}",
        json
    );

    let text = report_overview_text_with_context(
        &Overview::default(),
        &sessions,
        &health,
        TimeRange::All,
        false,
    );
    assert!(
        text.contains("Future-dated sessions excluded: 1"),
        "text arm must disclose the exclusion: {text}"
    );
}

// ---------- rm-917 residual: statusline-report parity ----------

fn write_journal(dir: &std::path::Path, now: chrono::DateTime<chrono::Utc>) {
    let day = |offset: i64| (now + chrono::Duration::days(offset)).timestamp();
    // Cumulative per-session cost snapshots (the series bills the
    // day-over-day rise), monotone by construction.
    let lines: Vec<String> = [(-10i64, 4.0f64), (-9, 10.0), (-1, 11.0), (4000, 20.0)]
        .iter()
        .map(|(offset, cost)| {
            serde_json::json!({
                "captured_at": day(*offset),
                "payload": {
                    "session_id": "A",
                    "cost": {"total_cost_usd": cost},
                },
            })
            .to_string()
        })
        .collect();
    std::fs::create_dir_all(dir).expect("cache dir");
    std::fs::write(dir.join("statusline.jsonl"), lines.join("\n") + "\n").expect("journal");
}

#[test]
fn statusline_report_carries_budget_window_truth_in_both_arms() {
    let cache = scratch_dir("rm917-parity");
    write_journal(&cache, chrono::Utc::now());

    // Scoped env: this integration-test binary owns its environment.
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache);

    let json = render_statusline_report("json", Some(5.0)).expect("json renders");
    let payload: Value = serde_json::from_str(&json).expect("valid json");
    let budget = &payload["budget"];
    assert!(
        budget.get("daily").is_some(),
        "json arm must carry daily: {json}"
    );
    assert_eq!(
        budget["excluded_older_days"].as_u64(),
        Some(2),
        "json arm must carry excluded_older_days: {json}"
    );
    assert_eq!(
        budget["excluded_future_days"].as_u64(),
        Some(1),
        "json arm must carry excluded_future_days: {json}"
    );

    let text = render_statusline_report("text", Some(5.0)).expect("text renders");
    assert!(
        text.contains("2 older days outside the window not counted"),
        "text arm must disclose stale days: {text}"
    );
    assert!(
        text.contains("1 future day beyond now+1h not counted"),
        "text arm must disclose future days: {text}"
    );

    // The --budget arm itself stays truthful on the new counter
    // (its json shape keeps the window fields at the top level).
    let budget_json = render_budget_view("json", Some(5.0)).expect("budget json");
    let budget_payload: Value = serde_json::from_str(&budget_json).expect("valid json");
    assert_eq!(
        budget_payload["excluded_future_days"].as_u64(),
        Some(1),
        "budget arm must carry excluded_future_days: {budget_json}"
    );
    let budget_text = render_budget_view("text", Some(5.0)).expect("budget text");
    assert!(
        budget_text.contains("1 future day beyond now+1h not counted"),
        "budget text must disclose: {budget_text}"
    );

    std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
}

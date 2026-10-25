//! rm-2033 (cycle 2, run a356ccb78fc1): `--quota --forecast` contract tests.
//!
//! Contract (ROADMAP rm-2033 acceptance):
//! 1. per-window burn rate + projected window-end spend at that rate, with the
//!    estimate basis disclosed;
//! 2. partial-TTL windows (corpus covers less than the full window span) are
//!    EXCLUDED from the rate and disclosed, never extrapolated;
//! 3. `-f json` parity via a stable `forecast` schema member beside the
//!    existing agenttrace.quota/1 members;
//! 4. empty/starved corpora refuse loudly (insufficient-signal refusal naming
//!    the flag pair) instead of projecting from nothing;
//! 5. zero change to non-forecast `--quota` output (no forecast member in
//!    quota_report, existing render paths untouched).

use agenttrace_core::{
    parse_raw_session, quota_forecast, quota_report, render_quota_view, render_quota_view_forecast,
    QuotaForecastArm, QuotaForecastError, Session,
};
use chrono::{DateTime, TimeZone, Utc};

fn ts(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().expect("valid rfc3339 timestamp")
}

fn mk_session(
    name: &str,
    start: &str,
    end: &str,
    cost: f64,
    timestamps: Vec<DateTime<Utc>>,
) -> Session {
    let line = r#"{"type":"user","message":{"role":"user","content":"go"}}"#;
    let path = format!("/tmp/{}.jsonl", name);
    let mut s = parse_raw_session(&path, &path, line).expect("fixture session parses");
    s.metrics.session_start = start.to_string();
    s.metrics.session_end = end.to_string();
    s.metrics.cost_estimated = cost;

    s.metrics.timestamps = timestamps;
    s
}

/// Corpus observed for ~9.2 days ending 2026-10-12T10:00Z: both windows full.
///
/// * `old`   (10-03, cost 0.25): pushes coverage back past the weekly span but
///   contributes to neither window (its instants predate both windows).
/// * `s1`    (10-12 05:30-08:00, cost 1.0): fully inside the five-hour window.
/// * `warm`  (10-12 06:00-07:00, cost 0.5, no timestamps): bills via span
///   share inside the five-hour window (warm-cache attribution class).
fn full_corpus() -> Vec<Session> {
    vec![
        mk_session(
            "old",
            "2026-10-03T09:00:00Z",
            "2026-10-03T13:00:00Z",
            0.25,
            vec![ts("2026-10-03T10:00:00Z"), ts("2026-10-03T12:00:00Z")],
        ),
        mk_session(
            "s1",
            "2026-10-12T05:30:00Z",
            "2026-10-12T08:00:00Z",
            1.0,
            vec![ts("2026-10-12T05:30:00Z"), ts("2026-10-12T08:00:00Z")],
        ),
        mk_session(
            "warm",
            "2026-10-12T06:00:00Z",
            "2026-10-12T07:00:00Z",
            0.5,
            vec![],
        ),
    ]
}

#[test]
fn full_corpus_projects_both_windows_with_disclosed_basis() {
    let sessions = full_corpus();
    // cutoff = newest at-or-before-now instant = 2026-10-12T08:00Z (s1 event).
    // observed = 10-03T10:00Z .. 10-12T08:00Z = 214h >= 168h -> both windows full.
    // five_hour window 10-12T03:00Z..08:00Z: s1 fully inside (1.0) + warm span
    // share (0.5) -> spend 1.5, rate 1.5/5 = 0.3/h, projected 0.3*5 = 1.5.
    // weekly window 10-05T08:00Z..10-12T08:00Z: s1 + warm inside, old predates
    // -> spend 1.5, rate 1.5/168.
    let out = render_quota_view_forecast("json", &sessions).expect("forecast renders");
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid json");
    let f = &v["forecast"];
    assert!(f.is_object(), "forecast member present");
    assert_eq!(
        f["basis"].as_str().expect("basis disclosed"),
        agenttrace_core::QUOTA_FORECAST_BASIS
    );

    let five = &f["five_hour"];
    assert!((five["burn_rate_usd_per_hour"].as_f64().unwrap() - 0.3).abs() < 1e-12);
    assert!((five["projected_next_window_usd"].as_f64().unwrap() - 1.5).abs() < 1e-12);
    assert!((five["window_spend_usd"].as_f64().unwrap() - 1.5).abs() < 1e-12);
    assert!((five["window_hours"].as_f64().unwrap() - 5.0).abs() < 1e-9);
    assert!(five["observed_hours"].as_f64().unwrap() >= 168.0);
    assert_eq!(
        five["attribution"]["event_basis"].as_u64(),
        Some(1),
        "s1 is event-timed"
    );
    assert_eq!(
        five["attribution"]["span_basis"].as_u64(),
        Some(1),
        "warm bills via span share"
    );

    let weekly = &f["weekly"];
    assert!((weekly["burn_rate_usd_per_hour"].as_f64().unwrap() - 1.5 / 168.0).abs() < 1e-12);
    assert!((weekly["projected_next_window_usd"].as_f64().unwrap() - 1.5).abs() < 1e-12);
    assert_eq!(weekly["attribution"]["event_basis"].as_u64(), Some(1));
    assert_eq!(weekly["attribution"]["span_basis"].as_u64(), Some(1));

    // The rest of the report is the untouched agenttrace.quota/1 payload.
    assert_eq!(v["schema"].as_str(), Some("agenttrace.quota/1"));
    assert!(v["five_hour"]["spend_usd"].is_number());

    // Text arm: basis + both windows rendered.
    let text = render_quota_view_forecast("text", &sessions).expect("text renders");
    assert!(
        text.contains("forecast (linear, local-first):"),
        "text forecast section"
    );
    assert!(text.contains("burn $0.3000/h"), "five-hour burn line");
    assert!(text.contains("basis:"), "basis disclosure line");
    assert!(text.contains("span-shared 1"), "warm attribution disclosed");
}

#[test]
fn young_corpus_refuses_loudly_instead_of_projecting() {
    // 1.5h of placeable history: less than one full five-hour window.
    let sessions = vec![mk_session(
        "young",
        "2026-10-12T08:00:00Z",
        "2026-10-12T09:30:00Z",
        1.0,
        vec![ts("2026-10-12T08:00:00Z"), ts("2026-10-12T09:30:00Z")],
    )];
    let err = render_quota_view_forecast("json", &sessions).expect_err("starved corpus refuses");
    match err {
        QuotaForecastError::InsufficientSignal(msg) => {
            assert!(
                msg.contains("--quota --forecast"),
                "refusal names the flag pair"
            );
            assert!(msg.contains("insufficient signal"), "loud refusal");
            assert!(msg.contains("1.5h"), "observed coverage disclosed");
        }
        QuotaForecastError::Render(m) => panic!("expected refusal, got render error: {}", m),
    }
    // The typed API reports the same starvation.
    assert!(matches!(
        quota_forecast(&sessions),
        Err(QuotaForecastError::InsufficientSignal(_))
    ));
}

#[test]
fn six_hour_corpus_forecasts_five_hour_and_excludes_weekly() {
    // cutoff 2026-10-12T09:30Z; oldest 2026-10-12T03:00Z -> observed 6.5h.
    let sessions = vec![
        mk_session(
            "push",
            "2026-10-12T03:00:00Z",
            "2026-10-12T03:30:00Z",
            0.25,
            vec![ts("2026-10-12T03:00:00Z")],
        ),
        mk_session(
            "s1",
            "2026-10-12T05:30:00Z",
            "2026-10-12T09:30:00Z",
            1.0,
            vec![ts("2026-10-12T05:30:00Z"), ts("2026-10-12T09:30:00Z")],
        ),
    ];
    let out =
        render_quota_view_forecast("json", &sessions).expect("partial corpus still forecasts");
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid json");
    let five = &v["forecast"]["five_hour"];
    assert!((five["burn_rate_usd_per_hour"].as_f64().unwrap() - 0.2).abs() < 1e-12);
    let weekly = &v["forecast"]["weekly"];
    assert_eq!(weekly["excluded"].as_bool(), Some(true), "weekly excluded");
    assert_eq!(weekly["reason"].as_str(), Some("partial_ttl"));
    assert!((weekly["window_hours"].as_f64().unwrap() - 168.0).abs() < 1e-9);
    assert!((weekly["observed_hours"].as_f64().unwrap() - 6.5).abs() < 1e-9);

    let text = render_quota_view_forecast("text", &sessions).expect("text renders");
    assert!(
        text.contains("weekly: excluded (partial_ttl)"),
        "exclusion disclosed in text"
    );
    assert!(text.contains("no rate extrapolated"));
}

#[test]
fn warm_only_corpus_bills_via_span_share_and_session_start_fallback() {
    // No timestamps anywhere: coverage comes from the session_start fallback,
    // and spend bills through the span share (warm-cache class).
    let sessions = vec![
        mk_session(
            "old-warm",
            "2026-10-03T09:00:00Z",
            "2026-10-03T13:00:00Z",
            0.0,
            vec![],
        ),
        mk_session(
            "warm",
            "2026-10-12T06:00:00Z",
            "2026-10-12T07:00:00Z",
            0.5,
            vec![],
        ),
    ];
    let forecast = quota_forecast(&sessions).expect("warm corpus forecasts");
    let five = match &forecast.five_hour {
        QuotaForecastArm::Full(w) => w,
        QuotaForecastArm::Excluded { reason, .. } => {
            panic!("five_hour unexpectedly excluded: {}", reason)
        }
    };
    assert!(
        (five.window_spend_usd - 0.5).abs() < 1e-12,
        "warm span share billed"
    );
    assert!((five.burn_rate_usd_per_hour - 0.1).abs() < 1e-12);
    assert_eq!(
        five.attribution_span_basis, 1,
        "warm session counted on span basis"
    );
    assert_eq!(five.attribution_event_basis, 0);
    assert!(
        five.observed_hours >= 168.0,
        "session_start fallback carried coverage"
    );
}

#[test]
fn non_finite_cost_poison_refuses_with_finite_message() {
    // rm-1500 discipline: quota_window skips poisoned costs at share
    // distribution, so a NaN cost can never poison the window spend. The
    // forecast therefore still renders - with finite numbers and no NaN
    // anywhere in the serialized output. (The non-finite-spend refusal arm
    // stays in the lib as defense-in-depth; the honest wall behavior for a
    // poisoned cost record is disclosure, not refusal.)
    let mut sessions = full_corpus();
    sessions.push(mk_session(
        "poison",
        "2026-10-12T07:00:00Z",
        "2026-10-12T07:30:00Z",
        f64::NAN,
        vec![ts("2026-10-12T07:00:00Z")],
    ));
    let out = render_quota_view_forecast("json", &sessions)
        .expect("poison cost does not stop the forecast");
    assert!(
        !out.contains("NaN"),
        "no NaN in serialized forecast: {}",
        out
    );
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid json");
    assert!(
        v["forecast"]["five_hour"]["burn_rate_usd_per_hour"].is_f64(),
        "five-hour arm still renders finite: {}",
        v["forecast"]
    );
    assert!(
        (v["forecast"]["five_hour"]["window_spend_usd"]
            .as_f64()
            .unwrap()
            - 1.5)
            .abs()
            < 1e-12,
        "poisoned cost skipped at share distribution: {}",
        v["forecast"]["five_hour"]
    );
    assert!(
        v["forecast"]["weekly"]["burn_rate_usd_per_hour"].is_f64(),
        "weekly arm still renders finite: {}",
        v["forecast"]["weekly"]
    );
}

#[test]
fn empty_corpus_refuses_rather_than_projecting_nothing() {
    let err = render_quota_view_forecast("json", &[]).expect_err("empty corpus refuses");
    match err {
        QuotaForecastError::InsufficientSignal(msg) => {
            assert!(msg.contains("--quota --forecast"));
            assert!(msg.contains("placeable"));
        }
        QuotaForecastError::Render(m) => panic!("expected refusal, got render error: {}", m),
    }
}

#[test]
fn non_forecast_quota_output_gains_no_forecast_member() {
    let sessions = full_corpus();
    let report = quota_report(&sessions);
    let s = report.to_string();
    assert!(
        !s.contains("forecast"),
        "quota_report stays forecast-free: {}",
        s
    );
    let text = render_quota_view("text", &sessions).expect("non-forecast text renders");
    assert!(
        !text.contains("forecast"),
        "non-forecast text stays forecast-free"
    );
    let json = render_quota_view("json", &sessions).expect("non-forecast json renders");
    assert!(
        !json.contains("forecast"),
        "non-forecast json stays forecast-free"
    );
    // Unsupported-format discipline unchanged for the non-forecast path.
    assert!(render_quota_view("yaml", &sessions).is_err());
}

#[test]
fn future_only_corpus_refuses() {
    // Every instant is in the future: quota_cutoff finds nothing placeable.
    let sessions = vec![mk_session(
        "future",
        "2026-12-01T00:00:00Z",
        "2026-12-01T01:00:00Z",
        1.0,
        vec![Utc.with_ymd_and_hms(2026, 12, 1, 0, 30, 0).unwrap()],
    )];
    let err =
        render_quota_view_forecast("json", &sessions).expect_err("future-only corpus refuses");
    assert!(matches!(err, QuotaForecastError::InsufficientSignal(_)));
}

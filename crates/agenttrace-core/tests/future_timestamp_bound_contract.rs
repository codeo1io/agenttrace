//! rm-1534 (run 92ea7949 cycle 4, assess 7fb648a0 F1): the future
//! admission bound. Far-future timestamps used to sail through every
//! lane: HEALTH 100, zero anomalies, and — because
//! `session_admitted_since` was a LOWER bound only — admission into
//! every forward `--range` window (the assess PoC: a single year-9999
//! session in `--range today`, with `latest_session_at` reporting the
//! year 9999). A clock corrupted by eight millennia should not be able
//! to launder a session into "today".
//!
//! These pins hold the fixed contract:
//! 1. admission is bounded above by now + 24h when a range is active
//!    (the rm-1402 clamp pattern generalized to corpus admission);
//! 2. the exclusion is COUNTED and DISCLOSED (`future_excluded_sessions`
//!    → `future_timestamp_sessions` in data_health) — never a silent
//!    drop, twin discipline with rm-727's fork-copy accounting;
//! 3. `--range all` / no range keeps far-future sessions VISIBLE —
//!    the disclosure there is the `future_timestamp` anomaly class and
//!    the health drop, not a vanishing session;
//! 4. a new `future_timestamp` anomaly fires on the session itself,
//!    on the same last-activity anchor the admission predicate uses;
//! 5. grace tolerance is exactly 24h: an in-grace session (a small
//!    clock skew, now+12h) is admitted unchanged, anomaly-free;
//! 6. unknown-time sessions keep the N7 bucket contract — visible,
//!    never "future".

use agenttrace_core::{
    detect_anomalies, future_admission_bound, load_sessions_with_options, session_admitted_since,
    session_beyond_future_grace, LoadOptions, Metrics, Session, FUTURE_ADMISSION_GRACE_SECS,
};
use chrono::{Duration, TimeZone, Utc};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const YEAR_9999_STAMP: &str = "9999-12-31T23:59:59Z";

fn far_future_metrics() -> Metrics {
    Metrics {
        session_start: YEAR_9999_STAMP.to_string(),
        session_end: YEAR_9999_STAMP.to_string(),
        ..Metrics::default()
    }
}

fn in_grace_metrics(now: chrono::DateTime<Utc>) -> Metrics {
    let stamp = (now + Duration::hours(12)).to_rfc3339();
    Metrics {
        session_start: stamp.clone(),
        session_end: stamp,
        ..Metrics::default()
    }
}

fn session_with(metrics: Metrics) -> Session {
    Session {
        name: "probe".to_string(),
        path: "probe.jsonl".to_string(),
        cwd: String::new(),
        branch: String::new(),
        org: String::new(),
        org_email: String::new(),
        metrics,
        anomalies: Vec::new(),
        health: 100,
        tool_warnings: Vec::new(),
        diagnostics: agenttrace_core::Diagnostics::default(),
    }
}

#[test]
fn grace_is_24_hours() {
    // Pinned by contract: tight enough to exclude far-future
    // fabrications, loose enough for a timezone-offset mistake.
    assert_eq!(
        FUTURE_ADMISSION_GRACE_SECS,
        24 * 60 * 60,
        "the bound is now+24h — changing it changes what every ranged view admits"
    );
    let now = Utc::now();
    assert_eq!(
        future_admission_bound(now),
        now + Duration::hours(24),
        "the bound is exactly now+24h"
    );
}

#[test]
fn far_future_sessions_are_excluded_from_ranged_views() {
    let now = Utc::now();
    let since = now - Duration::hours(24);
    let session = session_with(far_future_metrics());
    assert!(
        !session_admitted_since(&session, Some(since), now),
        "a year-9999 session must NOT be admitted into a forward window"
    );
}

#[test]
fn range_all_keeps_far_future_sessions_visible() {
    let now = Utc::now();
    let session = session_with(far_future_metrics());
    assert!(
        session_admitted_since(&session, None, now),
        "`--range all` keeps far-future sessions visible — the anomaly is the disclosure there"
    );
}

#[test]
fn in_grace_sessions_are_admitted_unchanged() {
    let now = Utc::now();
    let since = now - Duration::hours(24);
    let session = session_with(in_grace_metrics(now));
    assert!(
        session_admitted_since(&session, Some(since), now),
        "now+12h sits inside the 24h grace: a small clock skew stays in scope"
    );
}

#[test]
fn past_and_unknown_time_sessions_keep_their_contracts() {
    let now = Utc::now();
    let since = now - Duration::hours(24);
    // Ended before the window: excluded (the rm-694 overlap rule).
    let past = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
    let ended = session_with(Metrics {
        session_start: past.to_rfc3339(),
        session_end: past.to_rfc3339(),
        ..Metrics::default()
    });
    assert!(!session_admitted_since(&ended, Some(since), now));
    // Unknown time: visible everywhere (the N7 unknown-time bucket).
    let unknown = session_with(Metrics::default());
    assert!(session_admitted_since(&unknown, Some(since), now));
    assert!(
        !session_beyond_future_grace(&unknown, now),
        "an unknown-time session is never \"future\""
    );
}

#[test]
fn future_timestamp_anomaly_fires_and_drops_health() {
    let anomalies = detect_anomalies(&far_future_metrics());
    assert!(
        anomalies
            .iter()
            .any(|anomaly| anomaly.kind == "future_timestamp" && anomaly.severity == "high"),
        "the year-9999 session must carry a high-severity future_timestamp anomaly: {anomalies:?}"
    );
    // Unknown-time sessions carry no anchor — the class must stay
    // silent for them (they disclose through unknown_time_sessions).
    assert!(
        !detect_anomalies(&Metrics::default())
            .iter()
            .any(|anomaly| anomaly.kind == "future_timestamp"),
        "no timestamps parsed means no future_timestamp anomaly"
    );
    // In-grace sessions are anomaly-free (acceptance 4).
    assert!(
        !detect_anomalies(&in_grace_metrics(Utc::now()))
            .iter()
            .any(|anomaly| anomaly.kind == "future_timestamp"),
        "now+12h sits inside the grace: no anomaly"
    );
}

/// Claude-Code JSONL fixture whose only timestamps sit in the year
/// 9999 (the assess PoC shape) plus an in-grace twin.
fn far_future_corpus(root: &std::path::Path) -> PathBuf {
    let dir = root.join("farfuture");
    fs::create_dir_all(&dir).expect("create corpus dir");
    fs::write(
        dir.join("future.jsonl"),
        format!(
            "{{\"type\":\"user\",\"timestamp\":\"{YEAR_9999_STAMP}\",\"message\":{{\"role\":\"user\",\"content\":\"from the future\"}}}}\n\
             {{\"type\":\"assistant\",\"timestamp\":\"{YEAR_9999_STAMP}\",\"message\":{{\"id\":\"msg_ff\",\"model\":\"claude-sonnet-4-20250514\",\"content\":[{{\"type\":\"text\",\"text\":\"ok\"}}],\"usage\":{{\"input_tokens\":1000,\"output_tokens\":200,\"cache_read_input_tokens\":500}}}}}}\n"
        ),
    )
    .expect("write future fixture");
    dir
}

fn with_sandboxed_env<F: FnOnce(&std::path::Path)>(f: F) {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let root = std::env::temp_dir().join(format!(
        "at-rm1534-future-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create scratch root");
    let previous_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", root.join("cache"));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&root)));
    match previous_cache {
        Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
        None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
    }
    let _ = fs::remove_dir_all(&root);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn loader_excludes_and_discloses_far_future_sessions() {
    with_sandboxed_env(|root| {
        let dir = far_future_corpus(root);
        let since = Some(Utc::now() - Duration::hours(24));
        let report = load_sessions_with_options(
            Some(&dir),
            &LoadOptions {
                since,
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            report.future_excluded_sessions, 1,
            "the year-9999 session is excluded AND counted, never silently dropped"
        );
        assert!(
            report
                .sessions
                .iter()
                .all(|session| session.metrics.session_end != YEAR_9999_STAMP),
            "the excluded session must not render in the ranged view"
        );
        // No range: still visible, carrying the anomaly (acceptance 3).
        let all = load_sessions_with_options(Some(&dir), &LoadOptions::default());
        assert_eq!(
            all.future_excluded_sessions, 0,
            "count only applies to ranged loads"
        );
        assert_eq!(all.sessions.len(), 1);
        assert!(
            all.sessions[0]
                .anomalies
                .iter()
                .any(|anomaly| anomaly.kind == "future_timestamp"),
            "under --range all the far-future session is visible WITH its anomaly"
        );
    });
}

#[test]
fn in_grace_session_survives_a_ranged_load() {
    with_sandboxed_env(|root| {
        let dir = root.join("ingrace");
        fs::create_dir_all(&dir).expect("create corpus dir");
        let stamp = (Utc::now() + Duration::hours(12)).to_rfc3339();
        fs::write(
            dir.join("ingrace.jsonl"),
            format!(
                "{{\"type\":\"user\",\"timestamp\":\"{stamp}\",\"message\":{{\"role\":\"user\",\"content\":\"now+12h\"}}}}\n\
                 {{\"type\":\"assistant\",\"timestamp\":\"{stamp}\",\"message\":{{\"id\":\"msg_ig\",\"model\":\"claude-sonnet-4-5-20250929\",\"content\":[{{\"type\":\"text\",\"text\":\"ok\"}}],\"usage\":{{\"input_tokens\":1000,\"output_tokens\":200,\"cache_read_input_tokens\":500}}}}}}\n"
            ),
        )
        .expect("write in-grace fixture");
        let report = load_sessions_with_options(
            Some(&dir),
            &LoadOptions {
                since: Some(Utc::now() - Duration::hours(24)),
                ..LoadOptions::default()
            },
        );
        assert_eq!(report.future_excluded_sessions, 0);
        assert_eq!(
            report.sessions.len(),
            1,
            "an in-grace session stays in scope (grace tolerance, acceptance 4)"
        );
    });
}

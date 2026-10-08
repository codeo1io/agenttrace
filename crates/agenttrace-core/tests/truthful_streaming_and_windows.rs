//! Batch "Report truthfulness & honest surfacing" (cycle 1, truthfulness
//! round 3 — continues the landed rm-554/601-603 arc). Red-first battery
//! for rm-834 (Claude streaming re-emissions must not multiply turns,
//! tool calls, or truncate session_end/duration), rm-694 (range
//! membership and the report scope window must follow last activity, not
//! session start), and rm-835 (discovery must report parse failures in
//! LoadReport instead of swallowing them at `.ok()`).
//!
//! PoC corpora: /tmp/at-assess-24a3/corpus-stream (rm-834),
//! /tmp/at-assess-3541/corpus2 (rm-694 membership),
//! /tmp/at-assess-bd9d/zst (rm-835). The fixtures here are the committed
//! minimal forms of those corpora (see each test).

use agenttrace_core::{
    data_health, load_sessions_with_options, parse_file, report_scope, session_matches_time_range,
    LoadOptions, TimeRange,
};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn fixture(rel: &[&str]) -> PathBuf {
    let mut p: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for part in rel {
        p = p.join(part);
    }
    p
}

fn parse_fixture(rel: &[&str]) -> agenttrace_core::Session {
    parse_file(&fixture(rel)).unwrap_or_else(|e| panic!("parse {rel:?}: {e:#}"))
}

fn parse_at(rel: &[&str]) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rel[0])
        .unwrap()
        .with_timezone(&Utc)
}

/// Scoped env override so discovery-lane tests never touch (or write) the
/// ambient session cache. The guard mutex serializes this binary's tests
/// that swap env vars, mirroring discovery_contract.rs's lock_env().
fn with_isolated_cache(f: impl FnOnce()) {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    // The result is computed and the guard dropped in an inner block, so
    // a resumed panic never unwinds through a held lock — the poisoning
    // cascade discovery_contract.rs documents (one flaky failure failing
    // every later env-isolated test) must not reproduce here either.
    let result = {
        // unwrap_or_else(into_inner): a prior poisoning must not fail
        // every subsequent test; the vars are restored below regardless.
        let _guard = GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let tmp = std::env::temp_dir().join(format!("at-truthful-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("cache dir");
        let key = "AGENTTRACE_SESSION_CACHE_DIR";
        let previous = std::env::var_os(key);
        // Safety: same-thread set/remove of a shared env var, serialized
        // by the guard mutex across this test binary's threads.
        unsafe { std::env::set_var(key, &tmp) };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match previous {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
        let _ = std::fs::remove_dir_all(&tmp);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

// ---------------------------------------------------------------- rm-834

/// rm-834: Claude streaming rows re-emit the same message id with a
/// growing snapshot. The rm-601 fold kept usage at max per class but let
/// each re-emission count as another assistant event, so turns, tool
/// calls, and tool usage inflated (dossier PoC A: with-tools reported
/// turns 3 / tools 3 / Bash:3 where the conversation had exactly one
/// turn with one Bash call), and manufactured [P1] slow-tool / [P2]
/// retry anomalies from the phantom repeats.
#[test]
fn claude_stream_reemission_counts_the_message_once() {
    let session = parse_fixture(&["usage-accounting", "claude-stream-with-tools.jsonl"]);
    let m = &session.metrics;
    assert_eq!(m.assistant_turns, 1, "one message id = one turn");
    assert_eq!(m.tool_calls_total, 1, "one tool_use block = one call");
    assert_eq!(
        m.tool_usage.get("Bash"),
        Some(&1),
        "Bash usage must not triple via re-emissions"
    );
    // tokens keep the landed rm-601 max-per-class semantics
    assert_eq!(m.tokens_input, 1000);
    assert_eq!(m.tokens_output, 200);
    assert_eq!(m.tokens_cache_r, 200);
    // the last re-emission carries the session's true end and duration
    assert_eq!(m.session_end, "2026-10-01T11:00:07Z");
    assert_eq!(m.duration_sec, 7.0);
    assert!(
        session.anomalies.is_empty(),
        "phantom re-emissions must not manufacture anomalies: {:?}",
        session.anomalies
    );
}

/// rm-834 review fix (0772a0ca, MEDIUM finding): the tool_result dedup
/// arm (parser.rs "tool_result" => (message id, tool_use_id) BTreeSet)
/// executed zero times under the committed fixtures — with-tools only
/// re-emits tool_use blocks. This corpus re-emits a tool_result under a
/// repeated message id (same tool_use_id, enriched second snapshot)
/// plus a second, distinct result: deleting the dedup would count
/// toolu_tr_1 twice. Counts are the observable surface — Metrics
/// aggregates events, so the surviving snapshot's content is pinned
/// structurally (the first snapshot inserts the tool event;
/// re-emissions skip it).
#[test]
fn tool_result_reemission_counts_once_per_message_and_use() {
    let session = parse_fixture(&["usage-accounting", "claude-stream-toolresult-reemit.jsonl"]);
    let m = &session.metrics;
    assert_eq!(
        m.assistant_turns, 2,
        "two distinct message ids = two turns, a re-emission is not a turn"
    );
    assert_eq!(
        m.tool_results, 2,
        "re-emitted tool_result must count once (toolu_tr_1 once + toolu_tr_2 once), \
         not once per snapshot"
    );
    assert_eq!(
        m.session_end, "2026-10-01T10:00:08Z",
        "end must cover the last emission"
    );
}

/// rm-834 arm B: usage-only re-emissions (no content blocks) used to
/// leave session_end/duration frozen at the FIRST snapshot, so the
/// landed claude-stream-growing fixture reported duration 5.0 /
/// end 10:00:05Z while the stream ran to 10:00:07Z (dossier PoC B).
#[test]
fn claude_stream_usage_only_reemissions_advance_session_end() {
    for name in [
        "claude-stream-growing.jsonl",
        "claude-stream-with-content.jsonl",
    ] {
        let session = parse_fixture(&["usage-accounting", name]);
        assert_eq!(
            session.metrics.session_end, "2026-10-01T10:00:07Z",
            "{name}: end must cover the last emission"
        );
        assert_eq!(
            session.metrics.duration_sec, 7.0,
            "{name}: duration must span first user message to last emission"
        );
        if name.contains("with-content") {
            assert_eq!(
                session.metrics.assistant_turns, 1,
                "{name}: one message id = one turn"
            );
        }
    }
}

// ---------------------------------------------------------------- rm-694

fn overnight_session() -> agenttrace_core::Session {
    parse_fixture(&["window-membership", "codex-overnight.jsonl"])
}

fn old_start_old_end() -> agenttrace_core::Session {
    parse_fixture(&["window-membership", "claude-old-start-old-end.jsonl"])
}

fn old_start_recent_end() -> agenttrace_core::Session {
    parse_fixture(&["window-membership", "claude-old-start-recent-end.jsonl"])
}

/// rm-694: `--range today` (and rolling windows) must admit a session
/// whose ACTIVITY overlaps the window — a 23:30Z session that ran to
/// 01:00Z is today's spend even though it started yesterday. The old
/// start-only basis dropped it (dossier PoC N2: "No sessions match"
/// vs $0.0158 under --range all), silently hiding exactly the overnight
/// runs the flag exists to surface.
#[test]
fn range_membership_admits_sessions_active_in_window() {
    // midday 2026-10-06: the overnight session (23:30Z start, 01:00Z
    // end) is inside today; a session that ended 2026-10-03 is not.
    let now = parse_at(&["2026-10-06T12:00:00Z"]);
    assert!(
        session_matches_time_range(&overnight_session(), TimeRange::Today, now),
        "overnight session active after midnight must be a today session"
    );
    assert!(
        !session_matches_time_range(&old_start_old_end(), TimeRange::Today, now),
        "session entirely before today must stay excluded"
    );
    assert!(
        !session_matches_time_range(&old_start_recent_end(), TimeRange::Today, now),
        "ended 10-03 is not in today"
    );
    assert!(
        session_matches_time_range(&old_start_recent_end(), TimeRange::Days7, now),
        "ended 10-03 is inside the rolling 7d window"
    );
    assert!(
        !session_matches_time_range(&old_start_old_end(), TimeRange::Days7, now),
        "ended 09-01 is outside the rolling 7d window"
    );
}

/// rm-694, discovery side: the `since` retain filter shares the overlap
/// basis (discovery.rs used session_start directly).
#[test]
fn discovery_since_filter_admits_by_overlap() {
    with_isolated_cache(|| {
        let dir = fixture(&["window-membership"]);
        let load = |since: DateTime<Utc>| {
            load_sessions_with_options(
                Some(dir.as_path()),
                &LoadOptions {
                    since: Some(since),
                    ..LoadOptions::default()
                },
            )
        };
        let midnight = parse_at(&["2026-10-06T00:00:00Z"]);
        let report = load(midnight);
        let names: Vec<&str> = report.sessions.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["codex-overnight"],
            "the overnight session must survive a midnight-based since filter"
        );
        // end 2026-10-03T09:00Z: inside a window since 10-03 midnight,
        // outside a window since 10-04 midnight.
        let since_oct3 = parse_at(&["2026-10-03T00:00:00Z"]);
        let report = load(since_oct3);
        assert_eq!(report.sessions.len(), 2, "old-start/recent-end joins");
        let since_oct4 = parse_at(&["2026-10-04T00:00:00Z"]);
        let report = load(since_oct4);
        assert_eq!(
            report.sessions.len(),
            1,
            "session that ENDED before the since bound stays excluded"
        );
    });
}

/// rm-694 arm C: the overview scope line "Session window … → X" used to
/// end at the latest session START, so a report could end its own
/// window before the activity it was reporting (dossier PoC C:
/// "to 11:00:00Z" while the stream ran to 11:00:07Z).
#[test]
fn report_scope_window_ends_at_last_activity() {
    let sessions = vec![
        old_start_old_end(),
        old_start_recent_end(),
        overnight_session(),
    ];
    let scope = report_scope(&sessions, TimeRange::All, false);
    assert_eq!(
        scope.earliest_session_at, "2026-09-01T10:00:00Z",
        "earliest keeps first activity"
    );
    assert_eq!(
        scope.latest_session_at, "2026-10-06T01:00:00Z",
        "latest must be the last ACTIVITY (01:00Z), not the 23:30Z start"
    );
}

/// rm-694 review fix (0772a0ca, HIGH finding): data_health's
/// `latest_session_at` is user-visible (the `--overview -f json`
/// `data_health` block and the TUI health row) and used to be computed
/// from session STARTS while the same report's scope window ended at
/// last activity — a report could claim a "latest" older than its own
/// window (overnight corpus: 23:30Z vs the 01:00Z window end). Same
/// corpus, same basis now.
#[test]
fn data_health_latest_session_at_uses_last_activity() {
    let sessions = vec![
        old_start_old_end(),
        old_start_recent_end(),
        overnight_session(),
    ];
    let health = data_health(&sessions, sessions.len(), 0);
    assert_eq!(
        health.latest_session_at, "2026-10-06T01:00:00Z",
        "data_health latest must match the scope window end (last activity), not a session start"
    );
}

// ---------------------------------------------------------------- rm-835

/// rm-694 guard: a fresh session (start and end inside the window) must
/// still be admitted — overlap basis must not regress the plain case.
#[test]
fn range_membership_keeps_fresh_sessions() {
    let now = parse_at(&["2026-10-06T12:00:00Z"]);
    let fresh = overnight_session();
    let session = agenttrace_core::Session {
        metrics: agenttrace_core::Metrics {
            session_start: "2026-10-06T09:00:00Z".into(),
            session_end: "2026-10-06T09:30:00Z".into(),
            ..fresh.metrics
        },
        ..fresh
    };
    assert!(session_matches_time_range(&session, TimeRange::Today, now));
}

/// rm-835: discovery used to drop parse failures at `.ok()`, so an
/// all-corrupt directory surfaced as "No sessions match the requested
/// filters" — hiding both the failure and the parser's own hint (here:
/// the zstd-compressed rollout hint the explicit-path lane already
/// prints). The LoadReport must now carry the failure count and the
/// first failure message.
#[test]
fn discovery_report_carries_parse_failures() {
    with_isolated_cache(|| {
        let dir = fixture(&["discovery-failures"]);
        let report =
            load_sessions_with_options(Some(Path::new(dir.as_os_str())), &LoadOptions::default());
        assert_eq!(report.discovered, 1, "the zst fixture is discovered");
        assert_eq!(
            report.parse_failures, 1,
            "its parse failure must be counted, not swallowed"
        );
        assert!(report.sessions.is_empty());
        let first = report
            .first_parse_failure
            .as_deref()
            .expect("first failure message");
        assert!(
            first.contains("zstd"),
            "the parser's zstd hint must ride the report: {first}"
        );
        assert!(
            first.contains("zstd-rollout.jsonl"),
            "the failure must name the offending file: {first}"
        );
    });
}

/// rm-835 mixed arm: when some files parse and the empty set comes from
/// filters, the parse failures must still be disclosed alongside.
#[test]
fn discovery_report_counts_partial_parse_failures() {
    with_isolated_cache(|| {
        let tmp = std::env::temp_dir().join(format!("at-truthful-mixed-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("tmp");
        std::fs::copy(
            fixture(&["discovery-failures", "zstd-rollout.jsonl"]),
            tmp.join("rollout-0000.jsonl"),
        )
        .expect("copy zst");
        std::fs::copy(
            fixture(&["window-membership", "claude-old-start-old-end.jsonl"]),
            tmp.join("old.jsonl"),
        )
        .expect("copy good");
        let since = parse_at(&["2026-10-06T00:00:00Z"]);
        let report = load_sessions_with_options(
            Some(&tmp),
            &LoadOptions {
                since: Some(since),
                ..LoadOptions::default()
            },
        );
        assert_eq!(report.discovered, 2);
        assert_eq!(report.parse_failures, 1, "one file failed to parse");
        assert!(
            report.sessions.is_empty(),
            "the good file ends 2026-09-01, so the since filter drops it"
        );
        assert!(report.first_parse_failure.is_some());
        let _ = std::fs::remove_dir_all(&tmp);
    });
}

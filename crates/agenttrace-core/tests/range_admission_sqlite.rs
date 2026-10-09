//! rm-890 contract tests (run ac3ac300 cycle 2): the sqlite lanes (hermes
//! and opencode) must admit `--range`/`--since` sessions by LAST ACTIVITY
//! (the rm-694 overlap basis), not by session start — README.md:192-196
//! promises "a session that started before midnight but ran into today
//! counts; 7d/30d are rolling windows on the same activity basis", but
//! `session_within_since` filtered by `metrics.session_start`, so an
//! overnight session vanished from every ranged sqlite-lane report while
//! the TUI kept it (assess F1 HIGH, run c92f079f, live PoC:
//! `--range 7d --sessions` dropped the overnight session that
//! `--range all` showed).
//!
//! Fixtures are the committed minimal PoC corpora — see
//! `fixtures/range-overnight/README.md` for rows, timestamps, and sha256
//! pins (rm-542 convention). The cutoff is pinned to
//! 2026-10-02T00:00:00Z so the fixtures stay deterministic forever.

use std::path::{Path, PathBuf};

use agenttrace_core::load_sqlite_backed_sessions_reported;
use chrono::{TimeZone, Utc};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm890-contract-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir_all(&home).expect("temp home");
    home
}

/// Scoped env override (HOME + XDG + session-cache dir) so the sqlite
/// discovery lanes read only the fixture home and never touch the ambient
/// session cache. Mirrors sqlite_hostile_disclosure.rs's `with_home`,
/// including the guard mutex that serializes env-swapping tests and the
/// catch_unwind so a resumed panic never unwinds through a held lock.
fn with_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
    let result = {
        let _guard = lock_env();
        let saved: Vec<(&str, Option<std::ffi::OsString>)> = [
            "HOME",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "XDG_DATA_HOME",
            "AGENTTRACE_SESSION_CACHE_DIR",
        ]
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect();
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        for (key, value) in saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        result
    };
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    match LOCK.get_or_init(|| std::sync::Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// 2026-10-02T00:00:00Z — the pinned 7d-style cutoff. Relative to it:
/// ses-overnight started 2026-09-19 (outside) but ended 2026-10-08
/// (inside); ses-fresh is entirely inside; ses-stale entirely outside.
fn cutoff() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 2, 0, 0, 0).unwrap()
}

#[test]
fn hermes_lane_admits_overnight_sessions_by_last_activity() {
    let home = temp_home("hm");
    let target = home.join(".hermes/state.db");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(fixture("range-overnight/state.db"), &target).unwrap();

    let sessions = with_home(&home, || {
        let (sessions, report) = load_sqlite_backed_sessions_reported(Some(cutoff()));
        assert!(
            report.unreadable.is_empty(),
            "the fixture database is readable: {:?}",
            report.unreadable
        );
        sessions
    });
    std::fs::remove_dir_all(&home).ok();

    let mut keys: Vec<&str> = sessions
        .iter()
        .map(|s| s.metrics.session_key.as_str())
        .collect();
    keys.sort_unstable();
    // The acceptance golden: admission by last activity keeps BOTH the
    // fresh session and the overnight one (started before the cutoff,
    // activity inside it) and still excludes the stale one. Pre-fix
    // (start-basis) this was ["ses-fresh"] — the overnight session
    // vanished from the ranged report.
    assert_eq!(
        keys,
        vec!["ses-fresh", "ses-overnight"],
        "rm-890: overnight session must count by last activity, not start"
    );
}

#[test]
fn opencode_lane_admits_overnight_sessions_by_last_activity() {
    let home = temp_home("oc");
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(fixture("range-overnight/opencode.db"), &target).unwrap();

    let sessions = with_home(&home, || {
        let (sessions, report) = load_sqlite_backed_sessions_reported(Some(cutoff()));
        assert!(
            report.unreadable.is_empty(),
            "the fixture database is readable: {:?}",
            report.unreadable
        );
        sessions
    });
    std::fs::remove_dir_all(&home).ok();

    let mut keys: Vec<&str> = sessions
        .iter()
        .map(|s| s.metrics.session_key.as_str())
        .collect();
    keys.sort_unstable();
    // The assess PoC corpus (byte-identical opencode.db): pre-fix
    // (start-basis) only ["ses-fresh"] survived `--range 7d`.
    assert_eq!(
        keys,
        vec!["ses-fresh", "ses-overnight"],
        "rm-890: overnight session must count by last activity, not start"
    );
}

#[test]
fn unfiltered_load_keeps_all_rows_including_stale() {
    // The basis swap must not disable filtering: since=None loads every
    // row (overnight + fresh + stale), pinning that the predicate is the
    // only gate and the lanes' SQL pre-filter stays unused (since is
    // threaded as None into both queries today).
    let home = temp_home("all");
    let target = home.join(".hermes/state.db");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(fixture("range-overnight/state.db"), &target).unwrap();

    let mut keys: Vec<String> = with_home(&home, || {
        load_sqlite_backed_sessions_reported(None)
            .0
            .iter()
            .map(|s| s.metrics.session_key.clone())
            .collect()
    });
    std::fs::remove_dir_all(&home).ok();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["ses-fresh", "ses-overnight", "ses-stale"],
        "since=None admits every row"
    );
}

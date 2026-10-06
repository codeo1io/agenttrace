//! rm-596 contract tests: SQLite-backed ingestion must be honest about
//! hostile databases. The three corpora below are the assess-phase
//! probes (P11/P13/P12/P14 of run ff0068ca), committed as fixtures so
//! the disclosure contract is pinned against the exact bytes that
//! motivated it:
//!
//! - `opencode-hostile/opencode.db` — five session rows, one with a
//!   NULL id, one with TEXT in time_created, negative and
//!   i64::MAX token values. Pre-fix: 4 of 5 sessions "reported", the
//!   fifth vanished (P11).
//! - `hermes-hostile/state.db` — two session rows, one NULL id.
//!   Pre-fix: 1 of 2 sessions reported (P13).
//! - `random-bytes/opencode.db` — 4096 bytes of /dev/urandom in the
//!   discovered opencode slot. Pre-fix: "No session files found",
//!   doctor printed `OpenCode (DB) found parsed=0 failed=0` and
//!   recommended `--demo` (P12/P14).
//!
//! sha256 pins live in each fixture's README.md (rm-542 convention).

use std::path::{Path, PathBuf};

use agenttrace_core::{
    build_doctor_report, load_sqlite_backed_sessions_reported, SqliteIngestReport,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm596-contract-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir_all(&home).expect("temp home");
    home
}

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

#[test]
fn hostile_opencode_rows_are_counted_not_erased() {
    let home = temp_home("oc");
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(fixture("opencode-hostile/opencode.db"), &target).unwrap();

    let (sessions, report) = with_home(&home, load_sqlite_backed_sessions_reported_once);
    std::fs::remove_dir_all(&home).ok();

    // The acceptance golden: every hostile row either loads or is
    // counted — five rows in, four sessions + one counted drop out.
    assert_eq!(sessions.len(), 4, "P11: 4 of 5 sessions load");
    assert_eq!(
        report.dropped_rows.len(),
        1,
        "the NULL-id ghost row is disclosed, not erased: {:?}",
        report.dropped_rows
    );
    let dropped = &report.dropped_rows[0];
    assert_eq!(dropped.dropped, 1);
    assert_eq!(dropped.source, "opencode");
    assert!(!dropped.sample.is_empty(), "the drop carries a sample");
    assert_eq!(
        sessions.len() + dropped.dropped,
        5,
        "row conservation: loaded + dropped == rows in the database"
    );
}

#[test]
fn hostile_hermes_rows_are_counted_not_erased() {
    let home = temp_home("hm");
    let target = home.join(".hermes/state.db");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(fixture("hermes-hostile/state.db"), &target).unwrap();

    let (sessions, report) = with_home(&home, load_sqlite_backed_sessions_reported_once);
    std::fs::remove_dir_all(&home).ok();

    // P13: 2 rows, 1 NULL-id ghost.
    assert_eq!(sessions.len(), 1, "P13: 1 of 2 sessions loads");
    assert_eq!(report.dropped_rows.len(), 1);
    assert_eq!(report.dropped_rows[0].dropped, 1);
    assert_eq!(report.dropped_rows[0].source, "hermes");
    assert_eq!(sessions.len() + report.dropped_rows[0].dropped, 2);
}

#[test]
fn random_bytes_db_is_unreadable_not_absent() {
    let home = temp_home("rnd");
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(fixture("random-bytes/opencode.db"), &target).unwrap();

    let (sessions, report) = with_home(&home, load_sqlite_backed_sessions_reported_once);
    std::fs::remove_dir_all(&home).ok();

    // P12/P14: a discovered-but-unreadable database must surface as a
    // failure, never as an empty corpus that silently "parsed" to 0.
    assert!(sessions.is_empty(), "nothing can parse from random bytes");
    assert_eq!(report.unreadable.len(), 1, "the failure is disclosed");
    let unreadable = &report.unreadable[0];
    assert_eq!(unreadable.source, "opencode");
    assert!(
        !unreadable.reason.is_empty(),
        "the failure carries a reason"
    );
}

/// Runs the reported load twice: the first call seeds the snapshot
/// cache, the second proves the seeded state cannot hide the failure
/// (no poisoned empty snapshot from a failed load).
fn load_sqlite_backed_sessions_reported_once() -> (Vec<agenttrace_core::Session>, SqliteIngestReport)
{
    let first = load_sqlite_backed_sessions_reported(None);
    let second = load_sqlite_backed_sessions_reported(None);
    assert_eq!(
        first.0.len(),
        second.0.len(),
        "cache round-trips the sessions"
    );
    second
}

#[test]
fn doctor_reports_unreadable_and_dropped_rows_as_failures() {
    // P12/P14 acceptance: --doctor over the random-bytes database must
    // show the OpenCode (DB) row with failed >= 1 (and a sample),
    // not `found 1 parsed=0 failed=0`.
    let home = temp_home("doc-rnd");
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(fixture("random-bytes/opencode.db"), &target).unwrap();

    let report = with_home(&home, || build_doctor_report(None, false));
    std::fs::remove_dir_all(&home).ok();

    let row = report
        .directories
        .iter()
        .find(|dir| dir.name == "OpenCode (DB)")
        .expect("the discovered database appears in doctor output");
    assert!(row.exists, "the database is found");
    assert_eq!(row.parsed, 0, "nothing parses from random bytes");
    assert_eq!(row.failed, 1, "unreadable counts as a failure");
    assert_eq!(row.files, 1);
    assert!(
        row.failure_samples
            .iter()
            .any(|sample| sample.contains("unreadable")),
        "the failure is explained: {:?}",
        row.failure_samples
    );
}

#[test]
fn doctor_counts_dropped_session_rows_as_failures() {
    // P11 acceptance: dropped session rows surface in doctor counters.
    let home = temp_home("doc-hostile");
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(fixture("opencode-hostile/opencode.db"), &target).unwrap();

    let report = with_home(&home, || build_doctor_report(None, false));
    std::fs::remove_dir_all(&home).ok();

    let row = report
        .directories
        .iter()
        .find(|dir| dir.name == "OpenCode (DB)")
        .expect("the discovered database appears in doctor output");
    assert_eq!(row.parsed, 4, "the decodable rows parse");
    assert_eq!(row.failed, 1, "the NULL-id ghost counts as a failure");
    assert_eq!(row.files, 5, "row conservation in the counters");
    assert!(
        row.failure_samples
            .iter()
            .any(|sample| sample.contains("failed to decode")),
        "the drop is explained: {:?}",
        row.failure_samples
    );
}

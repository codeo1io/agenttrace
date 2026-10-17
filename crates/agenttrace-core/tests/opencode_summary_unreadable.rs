//! OpenCode summary-column readability contract (rm-1926, run 9d3b927c
//! cycle 1, repository-maintenance 14f74994).
//!
//! rm-1509 landed the `summary_diffs` cross-check: when the per-file
//! payload sums DISAGREE with the row's `summary_additions` /
//! `summary_deletions` integers, the divergence is disclosed. But when
//! the integers are PRESENT and UNREADABLE — stored as SQL NULL, as
//! BLOB storage, or as non-numeric TEXT — `sqlite_value_as_i64`
//! returns None and the cross-check silently skips the row
//! (sqlite_sessions.rs `summary_diffs_divergence` early-`?` seam): a
//! writer reporting `summary_diffs` beside broken integers classifies
//! as "nothing to compare", hiding the defect the cross-check exists
//! to catch.
//!
//! Fixture `opencode-summary-diffs/opencode-summary-unreadable.db`
//! (v1 `session` schema, cloned from the rm-1509 oracle corpus):
//! - `ses-nullsum` — summary_* integers stored as SQL NULL beside a
//!   valid 2-file payload;
//! - `ses-textsum` — integers stored as non-numeric TEXT
//!   ("many"/"few"/"several") beside a valid 1-file payload;
//! - `ses-blobsum` — integers stored as BLOB storage beside a valid
//!   1-file payload;
//! - `ses-healthy-control` — readable integers matching the payload:
//!   must stay byte-quiet (no unreadable disclosure, no divergence).
//!
//! The healthy-row oracle suite `opencode_summary_diffs.rs` stays the
//! authority for the matched/divergent/malformed/empty classes and
//! must keep passing byte-identically.

use agenttrace_core::{delivery_evidence, load_sqlite_backed_sessions_reported, Session};
use std::path::{Path, PathBuf};

fn temp_home(tag: &str) -> PathBuf {
    static HOMES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm1926-contract-{}-{tag}-{}",
        std::process::id(),
        HOMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&home).expect("temp home");
    home
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    match LOCK.get_or_init(|| std::sync::Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
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

fn plant(home: &Path, fixture: &str) {
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).expect("mkdir opencode home");
    std::fs::copy(
        format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), fixture),
        &target,
    )
    .expect("copy fixture db");
}

fn loaded(fixture: &str) -> Vec<Session> {
    let home = temp_home(fixture);
    let sessions = with_home(&home, || {
        plant(&home, fixture);
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        assert!(
            report.unreadable.is_empty(),
            "fixture database must be readable: {:?}",
            report.unreadable
        );
        sessions
    });
    std::fs::remove_dir_all(&home).ok();
    sessions
}

// Delivery records key on the derived display name (fixture titles
// are non-placeholder), not on the row id.
fn evidence_for(sessions: &[Session], title: &str) -> Vec<String> {
    let report = delivery_evidence(sessions);
    let record = report
        .sessions
        .iter()
        .find(|record| record.session.contains(title))
        .unwrap_or_else(|| panic!("no delivery-evidence record for {title:?}"));
    record.evidence.clone()
}

#[test]
fn null_summary_integers_disclose_cross_check_not_runnable() {
    let sessions = loaded("opencode-summary-diffs/opencode-summary-unreadable.db");
    let lines = evidence_for(&sessions, "null summary integers");
    assert!(
        lines.iter().any(|line| {
            line.contains("cross-check not runnable")
                && line.contains("summary column unreadable")
                && line.contains("summary_additions")
        }),
        "NULL summary integers beside a summary_diffs payload must disclose an unreadable cross-check; got {lines:?}"
    );
}

#[test]
fn non_numeric_text_summary_integers_disclose() {
    let sessions = loaded("opencode-summary-diffs/opencode-summary-unreadable.db");
    let lines = evidence_for(&sessions, "text summary integers");
    assert!(
        lines.iter().any(|line| {
            line.contains("cross-check not runnable")
                && line.contains("summary column unreadable")
                && line.contains("summary_deletions")
        }),
        "non-numeric TEXT summary integers must disclose an unreadable cross-check; got {lines:?}"
    );
}

#[test]
fn blob_summary_integers_disclose() {
    let sessions = loaded("opencode-summary-diffs/opencode-summary-unreadable.db");
    let lines = evidence_for(&sessions, "blob summary integers");
    assert!(
        lines.iter().any(|line| {
            line.contains("cross-check not runnable")
                && line.contains("summary column unreadable")
                && line.contains("summary_additions")
                && line.contains("stored as BLOB")
        }),
        "BLOB-stored summary integers must disclose with the BLOB storage reason; got {lines:?}"
    );
}

#[test]
fn healthy_control_row_stays_quiet() {
    let sessions = loaded("opencode-summary-diffs/opencode-summary-unreadable.db");
    let lines = evidence_for(&sessions, "healthy control");
    assert!(
        lines
            .iter()
            .all(|line| !line.contains("cross-check not runnable")),
        "readable integers matching the payload must stay byte-quiet; got {lines:?}"
    );
}

#[test]
fn per_file_paths_surface_in_delivery_evidence() {
    // rm-1926 (c) / rm-1509 residual: the governance comment promises
    // the writer-reported list is "surfaced as counts and paths", but
    // the render carried counts only. The paths must appear beside
    // the counts line for the healthy oracle row.
    let sessions = loaded("opencode-summary-diffs/opencode.db");
    let lines = evidence_for(&sessions, "builder run");
    assert!(
        lines.iter().any(|line| {
            line.contains("writer-reported per-file")
                && line.contains("src/a.rs")
                && line.contains("README.md")
        }),
        "per-file paths must surface in delivery evidence; got {lines:?}"
    );
}

/// rm-1926 (d): the disclosure must survive the snapshot cache — a
/// warm read (unchanged size/mtime fingerprint, schema-14 snapshot)
/// replays the same counter and the same evidence line, never a
/// re-silenced v13 replay. This is the snapshot-DTO regression class
/// proven on run e30a8801: a Metrics field the snapshot DTO does not
/// carry makes the fix invisible to every warm-path user while the
/// cold tests stay green.
#[test]
fn warm_snapshot_keeps_disclosing_the_unreadable_columns() {
    let home = temp_home("warm-round-trip");
    let outcome = with_home(&home, || {
        plant(
            &home,
            "opencode-summary-diffs/opencode-summary-unreadable.db",
        );
        let (first, report_first) = load_sqlite_backed_sessions_reported(None);
        assert!(report_first.unreadable.is_empty());
        let (second, report_second) = load_sqlite_backed_sessions_reported(None);
        assert!(report_second.unreadable.is_empty());
        let disclosing = |sessions: &[agenttrace_core::Session]| {
            sessions
                .iter()
                .filter(|session| {
                    session
                        .metrics
                        .disclosure_counters
                        .contains_key("opencode_summary_cross_check_unreadable")
                })
                .count()
        };
        (
            first.len(),
            second.len(),
            disclosing(&first),
            disclosing(&second),
            evidence_for(&second, "null summary integers")
                .iter()
                .any(|line| line.contains("cross-check not runnable")),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (first, second, cold_disclosing, warm_disclosing, warm_line) = outcome;
    assert_eq!(first, 4, "cold parse counts all four rows");
    assert_eq!(second, 4, "warm read replays the same sessions");
    assert_eq!(cold_disclosing, 3, "cold: all three poisoned rows disclose");
    assert_eq!(
        warm_disclosing, 3,
        "warm: all three poisoned rows STILL disclose"
    );
    assert!(
        warm_line,
        "warm read keeps the 'cross-check not runnable' evidence line"
    );
}

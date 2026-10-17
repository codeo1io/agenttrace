//! OpenCode v2-only store disclosure contract (rm-1927, run
//! 9d3b927c cycle 1, repository-maintenance 14f74994).
//!
//! rm-507 discipline: key on the DB SCHEMA, not the package layout.
//! The opencode store probe reads the v1 `session` table by name; a
//! v2-only store (the public anomalyco/opencode@dev
//! `packages/core/src/database/schema.gen.ts` DDL — `session_v2`,
//! shipped to the dev dist-tag while npm latest is still v1 1.19.x)
//! has no `session` table, so the probe's prepare fails with "no such
//! table" and the store used to classify as session-less: ZERO
//! sessions, ZERO disclosures — a silent zero that reads as "nothing
//! happened here" instead of "format not handled".
//!
//! Fixture `opencode-summary-diffs/opencode-v2-store.db` carries the
//! public dev-branch `session_v2` DDL verbatim with two realistic
//! rows and NO v1 `session` table.
//!
//! Contract: a v2-shaped store is disclosed as a NAMED unhandled
//! format through the ingest report (and the doctor census counts it
//! as its own class) — never a silent zero, and never folded into
//! `unreadable` (a v2 store is a healthy known format, not
//! corruption).

use agenttrace_core::{doctor_sqlite_directories, load_sqlite_backed_sessions_reported};
use std::path::{Path, PathBuf};

fn temp_home(tag: &str) -> PathBuf {
    static HOMES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm1927-contract-{}-{tag}-{}",
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

fn plant(home: &Path) -> PathBuf {
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).expect("mkdir opencode home");
    std::fs::copy(
        format!(
            "{}/tests/fixtures/opencode-summary-diffs/opencode-v2-store.db",
            env!("CARGO_MANIFEST_DIR")
        ),
        &target,
    )
    .expect("copy fixture db");
    target
}

/// Red-first lineage: on the pristine reader this probe asserted
/// `sessions > 0 || unreadable > 0 || dropped_rows > 0` and failed —
/// the store read as a session-less foreign database (zero sessions,
/// zero unreadable, zero drops): the silent zero this contract
/// forbids.
#[test]
fn v2_only_store_is_never_silent_zero() {
    let home = temp_home("v2-only");
    let outcome = with_home(&home, || {
        let path = plant(&home);
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        (
            path,
            sessions.len(),
            report.unreadable.len(),
            report.dropped_rows.len(),
            report
                .unhandled_v2_stores
                .iter()
                .map(|store| {
                    (
                        store.path.clone(),
                        store.table.clone(),
                        store.reason.clone(),
                    )
                })
                .collect::<Vec<_>>(),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (path, sessions, unreadable, dropped, unhandled) = outcome;
    assert_eq!(sessions, 0, "v2 rows must not be fabricated into sessions");
    assert_eq!(unreadable, 0, "a v2 store is not corruption");
    assert_eq!(dropped, 0, "nothing was decoded, so nothing was dropped");
    assert!(
        sessions > 0 || unreadable > 0 || dropped > 0 || !unhandled.is_empty(),
        "v2-only opencode store classified as silent zero"
    );
    assert_eq!(unhandled.len(), 1, "exactly one disclosure for the store");
    let (store_path, table, reason) = &unhandled[0];
    assert_eq!(store_path, &path, "disclosure names the store path");
    assert_eq!(table, "session_v2", "shape probe matched by columns");
    assert!(
        reason.contains("opencode v2 store") && reason.contains("ingest not implemented"),
        "reason names the unhandled format: {reason}"
    );
}

/// The disclosure survives the snapshot-cache path: an unhandled
/// store must never bank a session-less snapshot that would mask it
/// on the warm read (`is_clean` stays false).
#[test]
fn v2_disclosure_survives_a_second_load() {
    let home = temp_home("warm");
    let outcome = with_home(&home, || {
        plant(&home);
        let (first, report_first) = load_sqlite_backed_sessions_reported(None);
        let (_second, report_second) = load_sqlite_backed_sessions_reported(None);
        (
            first.len(),
            report_first.unhandled_v2_stores.len(),
            report_second.unhandled_v2_stores.len(),
            report_second.unreadable.len(),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (first, unhandled_first, unhandled_second, unreadable_second) = outcome;
    assert_eq!(first, 0);
    assert_eq!(unhandled_first, 1, "cold read discloses");
    assert_eq!(unhandled_second, 1, "warm read discloses identically");
    assert_eq!(unreadable_second, 0);
}

/// The doctor census counts the v2 store as its own class: found
/// (exists), failed=1 with a named sample — never parsed=0 failed=0
/// (silence) and never under the unreadable/corrupt sample.
#[test]
fn doctor_census_counts_the_v2_store_as_its_own_class() {
    let home = temp_home("doctor");
    let outcome = with_home(&home, || {
        plant(&home);
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        let dirs = doctor_sqlite_directories(&sessions, &report);
        let opencode = dirs
            .iter()
            .find(|dir| dir.name == "OpenCode (DB)")
            .expect("opencode census row");
        (
            opencode.exists,
            opencode.parsed,
            opencode.failed,
            opencode.failure_samples.join(" | "),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (exists, parsed, failed, samples) = outcome;
    assert!(exists, "the v2 store is found on disk");
    assert_eq!(parsed, 0, "nothing ingested");
    assert_eq!(failed, 1, "counted as its own failure class");
    assert!(
        samples.contains("v2-shaped store not ingested")
            && samples.contains("ingest not implemented"),
        "census sample names the unhandled format: {samples}"
    );
}

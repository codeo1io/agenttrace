//! opencode v2 `session_v2` store contract: counted sessions, and a
//! named residual class for shape-without-readability (rm-1927 + the
//! rm-2290 read arm, run e9021a4b cycle 7, repository-maintenance
//! d3046844, batch "opencode v2 session-ingest truth").
//!
//! rm-1927 (run 9d3b927c cycle 1) first made the v2 SHAPE visible: a
//! store with no v1 `session` table but a session-store column
//! signature under `session_v2` classified as a silent zero and
//! became a named `unhandled_v2_stores` disclosure ("ingest not
//! implemented"). opencode then shipped v2.1.2 (2026-10-21) and
//! fixed its V1→V2 migration the same day (c3b613ca) — migrated
//! homes are the common case and their sessions live in
//! `session_v2`, so a disclosure-only posture would leave every
//! post-migration session invisible. The rm-2290 read arm ingests
//! them; this suite pins the v2-only store (a fresh v2 install), the
//! warm-snapshot replay of the read, the doctor census, and the
//! residual class that must STAY disclosed: a table whose shape the
//! probe matched but whose guarded SELECT cannot run (`collate
//! nosuch` names a collation the file does not define — prepare
//! fails even though `pragma table_info` happily lists the columns).
//!
//! The migrated-home twins and drifted-schema contracts live in
//! `opencode_v2_read.rs`; the CLI stderr lane in
//! `agenttrace-cli/tests/opencode_v2_disclosure.rs`.

use agenttrace_core::{doctor_sqlite_directories, load_sqlite_backed_sessions_reported};
use std::path::{Path, PathBuf};

fn temp_home(tag: &str) -> PathBuf {
    // The suite's tests run in parallel and several load the SAME
    // fixture, so the path must be unique per CALL, not per fixture —
    // a shared path lets one test's `remove_dir_all` teardown race
    // another's plant/load.
    static HOMES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm2290-v2store-{}-{tag}-{}",
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

fn plant(home: &Path, fixture: &str) -> PathBuf {
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).expect("mkdir opencode home");
    std::fs::copy(
        format!(
            "{}/tests/fixtures/opencode-summary-diffs/{fixture}",
            env!("CARGO_MANIFEST_DIR")
        ),
        &target,
    )
    .expect("copy fixture db");
    target
}

/// Red-first lineage: on the rm-1927 reader this asserted
/// `sessions == 0` with an "ingest not implemented" disclosure and
/// PASSED; under the read arm it fails with `2 == 0` — the contract
/// flip this suite re-pins. A fresh v2 install must COUNT: both
/// `session_v2` rows become sessions carrying their stored totals,
/// and the read discloses nothing (no twins, no absent columns, no
/// reason) — success is silent, never a warning-shaped apology.
#[test]
fn v2_only_store_is_counted_not_a_silent_zero() {
    let home = temp_home("v2-only");
    let outcome = with_home(&home, || {
        let path = plant(&home, "opencode-v2-store.db");
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        (
            path,
            sessions
                .iter()
                .map(|session| session.metrics.session_key.clone())
                .collect::<Vec<_>>(),
            report.unreadable.len(),
            report.dropped_rows.len(),
            report.unhandled_v2_stores.len(),
            report.v2_reads.clone(),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (path, mut keys, unreadable, dropped, unhandled, v2_reads) = outcome;
    keys.sort();
    assert_eq!(
        keys,
        vec![String::from("sesv2-alpha"), String::from("sesv2-beta")],
        "both v2 rows are counted sessions"
    );
    assert_eq!(unreadable, 0, "a v2 store is not corruption");
    assert_eq!(dropped, 0, "nothing was decoded, so nothing was dropped");
    assert_eq!(
        unhandled, 0,
        "a readable v2 store no longer classifies as unhandled"
    );
    assert_eq!(v2_reads.len(), 1, "exactly one read entry for the store");
    let read = &v2_reads[0];
    assert_eq!(read.path, path, "read entry names the store path");
    assert_eq!(read.table, "session_v2", "matched by column signature");
    assert_eq!(read.rows_ingested, 2, "both rows ingested");
    assert_eq!(read.twin_rows, 0, "no v1 lane, so no twins");
    assert!(
        read.absent_columns.is_empty(),
        "full upstream DDL has every read column: {:?}",
        read.absent_columns
    );
}

/// Warm-path pin: the sqlite snapshot rung moved 14 -> 15 so a
/// snapshot banked by the pre-read build (which would answer "zero
/// sessions" for this store) cannot be trusted — the bump forces
/// regeneration, and the fresh snapshot persists the v2 read meta so
/// the replay discloses identically.
#[test]
fn v2_read_replays_on_the_warm_path() {
    let home = temp_home("warm");
    let outcome = with_home(&home, || {
        plant(&home, "opencode-v2-store.db");
        let (cold, report_cold) = load_sqlite_backed_sessions_reported(None);
        let (warm, report_warm) = load_sqlite_backed_sessions_reported(None);
        (
            cold.len(),
            warm.len(),
            report_cold.v2_reads.clone(),
            report_warm.v2_reads.clone(),
            report_warm.unhandled_v2_stores.len(),
            report_warm.unreadable.len(),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (cold, warm, v2_reads_cold, v2_reads_warm, unhandled, unreadable) = outcome;
    assert_eq!(cold, 2, "cold read counts both rows");
    assert_eq!(warm, 2, "warm read replays the count");
    assert_eq!(v2_reads_cold.len(), 1);
    assert_eq!(v2_reads_warm.len(), 1, "read meta persists in the snapshot");
    assert_eq!(
        v2_reads_warm[0].rows_ingested, 2,
        "warm replay keeps the ingested count"
    );
    assert_eq!(v2_reads_warm[0].twin_rows, 0);
    assert_eq!(unhandled, 0, "no disclosure regression on the warm path");
    assert_eq!(unreadable, 0);
}

/// The doctor census counts the v2 store as parsed sessions now —
/// found, parsed=2, failed=0. The named failure class is reserved
/// for the shape-without-readability residual below.
#[test]
fn doctor_census_counts_ingested_v2_rows() {
    let home = temp_home("doctor");
    let outcome = with_home(&home, || {
        plant(&home, "opencode-v2-store.db");
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
    assert_eq!(parsed, 2, "both v2 rows are parsed sessions");
    assert_eq!(failed, 0, "a healthy v2 store is not a failure class");
    assert!(samples.is_empty(), "no failure samples: {samples}");
}

/// The residual rm-1927 class: the shape probe matched (pragma
/// table_info lists every column) but the guarded SELECT cannot even
/// prepare — `time_created text collate nosuch` names a collation
/// the file does not define. Zero sessions, named disclosure (never
/// "unreadable" = corrupt, never silent), no snapshot banked, and
/// the doctor census carries the failure with a sample.
#[test]
fn shape_without_readability_stays_a_named_class() {
    let home = temp_home("unreadable");
    let outcome = with_home(&home, || {
        let path = plant(&home, "opencode-v2-unreadable.db");
        let (cold, report_cold) = load_sqlite_backed_sessions_reported(None);
        let unhandled_cold = report_cold
            .unhandled_v2_stores
            .iter()
            .map(|store| {
                (
                    store.path.clone(),
                    store.table.clone(),
                    store.reason.clone(),
                )
            })
            .collect::<Vec<_>>();
        let (_warm, report_warm) = load_sqlite_backed_sessions_reported(None);
        let dirs = doctor_sqlite_directories(&cold, &report_cold);
        let opencode = dirs
            .iter()
            .find(|dir| dir.name == "OpenCode (DB)")
            .expect("opencode census row");
        (
            path,
            cold.len(),
            unhandled_cold,
            report_cold.v2_reads.len(),
            report_cold.dropped_rows.len(),
            report_warm.unhandled_v2_stores.len(),
            opencode.parsed,
            opencode.failed,
            opencode.failure_samples.join(" | "),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (path, sessions, unhandled, v2_reads, dropped, unhandled_warm, parsed, failed, samples) =
        outcome;
    assert_eq!(sessions, 0, "unreadable rows are never fabricated");
    assert_eq!(dropped, 0, "nothing was decoded, so nothing was dropped");
    assert_eq!(
        v2_reads, 0,
        "no read entry for a store that could not be read"
    );
    assert_eq!(unhandled.len(), 1, "exactly one disclosure for the store");
    let (store_path, table, reason) = &unhandled[0];
    assert_eq!(store_path, &path, "disclosure names the store path");
    assert_eq!(table, "session_v2", "shape probe matched by columns");
    assert!(
        reason.contains("shape detected but unreadable")
            && reason.contains("no such collation sequence: nosuch"),
        "reason names the residual class and the engine error: {reason}"
    );
    assert_eq!(
        unhandled_warm, 1,
        "no session-less snapshot was banked to mask the disclosure"
    );
    assert_eq!(parsed, 0);
    assert_eq!(failed, 1, "counted as its own failure class");
    assert!(
        samples.contains("v2-shaped store not ingested"),
        "census sample names the unhandled format: {samples}"
    );
}

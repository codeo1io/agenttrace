//! opencode v2 read-arm contract: migrated homes count every session
//! exactly once, and a drifted v2 schema counts with named absences
//! (rm-2290, run e9021a4b cycle 7, repository-maintenance d3046844,
//! batch "opencode v2 session-ingest truth").
//!
//! Upstream provenance (anomalyco/opencode, dev
//! `packages/core/src/session/sql.ts`): the `session_v2` table —
//! `id` PK, `project_id`/`workspace_id`/`parent_id`/`fork_session_id`
//! refs, `slug`/`directory`/`path`/`title`/`version`,
//! `summary_additions`/`summary_deletions`/`summary_files`/
//! `summary_diffs`, `metadata`, `cost` REAL default 0,
//! `tokens_input`/`tokens_output`/`tokens_reasoning`/
//! `tokens_cache_read`/`tokens_cache_write` INTEGER default 0,
//! `revert`/`permission`/`agent`/`model`, `time_*` INTEGERs. v2.1.2
//! shipped 2026-10-21 and its V1→V2 migration was patched the same
//! day (c3b613ca): a MIGRATED home carries the legacy `session`
//! table AND `session_v2` holding a copy of every legacy row — the
//! common case this contract exists for.
//!
//! Fixtures (`tests/fixtures/opencode-summary-diffs/`):
//! - `opencode-migrated-v1v2.db` — v1 `session` with the four
//!   `ses-diff-*` rows (the rm-1509 oracle set) beside `session_v2`
//!   carrying the SAME four ids as twins with POISONED totals
//!   (`cost 9.99`, `tokens 888888`/`777777` — if the read arm ever
//!   double-counts a twin or lets the thinner v2 copy win, these
//!   numbers surface and the pin breaks) plus `ses-v2-post-migration`,
//!   a row only v2 knows (cost 0.25, 1200/240).
//! - `opencode-v2-drift.db` — `session_v2` reduced to 9 columns
//!   (`id`, `project_id`, `slug`, `directory`, `version`,
//!   `time_created`, `time_updated`, `tokens_input`,
//!   `tokens_output`): 12 of the 18 read columns are absent, read as
//!   NULL, and named per read.
//!
//! The v2-only store and the shape-without-readability residual are
//! pinned in `opencode_v2_store.rs`; the CLI stderr lane in
//! `agenttrace-cli/tests/opencode_v2_disclosure.rs`.

use agenttrace_core::{load_sqlite_backed_sessions_reported, Session};
use std::path::{Path, PathBuf};

fn temp_home(tag: &str) -> PathBuf {
    static HOMES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm2290-v2read-{}-{tag}-{}",
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
        format!(
            "{}/tests/fixtures/opencode-summary-diffs/{fixture}",
            env!("CARGO_MANIFEST_DIR")
        ),
        &target,
    )
    .expect("copy fixture db");
}

fn session_named<'a>(sessions: &'a [Session], key: &str) -> &'a Session {
    sessions
        .iter()
        .find(|session| session.metrics.session_key.contains(key))
        .unwrap_or_else(|| {
            let mut keys: Vec<String> = sessions
                .iter()
                .map(|session| session.metrics.session_key.clone())
                .collect();
            keys.sort();
            panic!("no session matching {key:?}; got keys {keys:?}")
        })
}

/// A migrated home counts every session exactly once: the four
/// legacy rows through the v1 `session` lane with THEIR totals (the
/// poisoned twin copies must never win), the post-migration row
/// through the v2 read arm with ITS stored totals, and the read
/// discloses 1 ingested + 4 twins. Red-first lineage: on the
/// rm-1927 reader the migrated home reported 4 sessions and an
/// unhandled store — `ses-v2-post-migration` was invisible.
#[test]
fn migrated_home_counts_v2_only_rows_and_keeps_twins_singular() {
    let home = temp_home("migrated");
    let outcome = with_home(&home, || {
        plant(&home, "opencode-migrated-v1v2.db");
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        (
            sessions.clone(),
            report.v2_reads.clone(),
            report.unhandled_v2_stores.len(),
            report.unreadable.len(),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (sessions, v2_reads, unhandled, unreadable) = outcome;
    assert_eq!(
        sessions.len(),
        5,
        "four v1 rows + the v2-only row — twins never add a sixth"
    );
    let matched = session_named(&sessions, "ses-diff-matched");
    assert!(
        (matched.metrics.cost_estimated - 0.12).abs() < 1e-9,
        "twin keeps the v1 stored cost, not the poisoned 9.99: {}",
        matched.metrics.cost_estimated
    );
    assert_eq!(matched.metrics.tokens_input, 100, "v1 tokens_input wins");
    assert_eq!(matched.metrics.tokens_output, 50, "v1 tokens_output wins");
    assert!(
        sessions
            .iter()
            .all(|session| session.metrics.tokens_input != 888_888),
        "a leaked twin row would surface 888888 input tokens"
    );
    let fresh = session_named(&sessions, "ses-v2-post-migration");
    assert!(
        (fresh.metrics.cost_estimated - 0.25).abs() < 1e-9,
        "the v2-only row carries its stored cost: {}",
        fresh.metrics.cost_estimated
    );
    assert_eq!(fresh.metrics.tokens_input, 1200);
    assert_eq!(fresh.metrics.tokens_output, 240);
    assert_eq!(unhandled, 0, "a migrated home is not an unhandled store");
    assert_eq!(unreadable, 0);
    assert_eq!(v2_reads.len(), 1, "one read entry for the v2 table");
    let read = &v2_reads[0];
    assert_eq!(read.table, "session_v2");
    assert_eq!(read.rows_ingested, 1, "only the v2-only row ingests");
    assert_eq!(read.twin_rows, 4, "the four copied legacy ids are twins");
    assert!(
        read.absent_columns.is_empty(),
        "full upstream DDL: {:?}",
        read.absent_columns
    );
}

/// The twin accounting must replay from the warm snapshot: the
/// sqlite snapshot rung moved 14 -> 15 so the pre-read build's stale
/// snapshot (which would answer 4 sessions and an unhandled store)
/// is regenerated, and the v2 read meta persists with it.
#[test]
fn migrated_home_replays_twin_accounting_on_the_warm_path() {
    let home = temp_home("migrated-warm");
    let outcome = with_home(&home, || {
        plant(&home, "opencode-migrated-v1v2.db");
        let (cold, _) = load_sqlite_backed_sessions_reported(None);
        let (warm, report_warm) = load_sqlite_backed_sessions_reported(None);
        (
            cold.len(),
            warm.len(),
            report_warm.v2_reads.clone(),
            report_warm.unhandled_v2_stores.len(),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (cold, warm, v2_reads_warm, unhandled_warm) = outcome;
    assert_eq!(cold, 5, "cold read counts all five");
    assert_eq!(warm, 5, "warm read replays the same count");
    assert_eq!(v2_reads_warm.len(), 1, "read meta persists in the snapshot");
    assert_eq!(v2_reads_warm[0].rows_ingested, 1);
    assert_eq!(v2_reads_warm[0].twin_rows, 4, "twin accounting replays");
    assert_eq!(unhandled_warm, 0);
}

/// A drifted v2 schema (upstream ships, a mirror trims, a fork
/// renames) still counts: both rows ingest with the columns that
/// survive, every absent optional column is read as NULL — not an
/// unreadable store, not a dropped row — and the absences are named
/// per read in read-column order.
#[test]
fn drifted_v2_schema_counts_rows_and_names_absent_columns() {
    let home = temp_home("drift");
    let outcome = with_home(&home, || {
        plant(&home, "opencode-v2-drift.db");
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        (
            sessions.clone(),
            report.v2_reads.clone(),
            report.unhandled_v2_stores.len(),
            report.dropped_rows.len(),
        )
    });
    std::fs::remove_dir_all(&home).ok();
    let (sessions, v2_reads, unhandled, dropped) = outcome;
    assert_eq!(sessions.len(), 2, "both drifted rows count");
    let one = session_named(&sessions, "sesv2-drift-one");
    assert_eq!(one.metrics.tokens_input, 4242, "surviving columns carry");
    assert_eq!(one.metrics.tokens_output, 1717);
    assert!(
        one.metrics.agent_name.is_empty(),
        "an absent agent column reads as absent, never as a stringified null"
    );
    assert!(
        one.metrics.parent_session.is_empty(),
        "absent parent_id degrades to no linkage"
    );
    assert_eq!(unhandled, 0, "drift is not the unreadable class");
    assert_eq!(dropped, 0, "absent columns never drop the row");
    assert_eq!(v2_reads.len(), 1);
    let read = &v2_reads[0];
    assert_eq!(read.rows_ingested, 2);
    assert_eq!(read.twin_rows, 0, "no v1 table, so every row is new");
    assert_eq!(
        read.absent_columns,
        vec![
            "title",
            "cost",
            "tokens_reasoning",
            "tokens_cache_read",
            "tokens_cache_write",
            "parent_id",
            "agent",
            "model",
            "summary_additions",
            "summary_deletions",
            "summary_files",
            "summary_diffs",
        ],
        "twelve absent read columns, named in read-column order"
    );
}

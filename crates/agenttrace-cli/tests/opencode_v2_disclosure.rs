//! CLI stderr disclosure for the opencode v2 read arm (rm-2290, run
//! e9021a4b cycle 7, repository-maintenance d3046844, batch
//! "opencode v2 session-ingest truth").
//!
//! The lib-side contracts live in
//! `agenttrace-core/tests/opencode_v2_store.rs` and
//! `agenttrace-core/tests/opencode_v2_read.rs`; this suite pins the
//! operator-facing lane (`disclose_sqlite_ingest` in main.rs): one
//! stderr NOTE per v2 read that had anything to disclose (twin rows
//! skipped, absent columns, or a non-default reason) and one WARNING
//! per shape-without-readability store — while a plain successful
//! read of a fresh v2 install prints NOTHING (the rm-753 discipline:
//! healthy corpora keep their historical stderr bytes).
//!
//! Each test runs the real binary in a throwaway HOME sandbox with
//! the opencode store planted at `.local/share/opencode/opencode.db`
//! — no in-process env mutation, so the tests are parallel-safe
//! (same posture as csv_export.rs).

use std::path::PathBuf;
use std::process::Command;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../agenttrace-core/tests/fixtures/opencode-summary-diffs")
        .join(name)
}

struct Planted {
    home: PathBuf,
    stderr: String,
    success: bool,
}

fn run_with_store(tag: &str, fixture: &str, args: &[&str]) -> Planted {
    let home = std::env::temp_dir().join(format!(
        "at-v2-disclosure-{}-{tag}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let store = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(store.parent().unwrap()).expect("mkdir opencode home");
    std::fs::copy(fixture_path(fixture), &store).expect("plant fixture db");
    let cache = home.join("cache");
    std::fs::create_dir_all(&cache).expect("mkdir sandbox cache");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .args(args)
        .output()
        .expect("run agenttrace CLI");
    Planted {
        home,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    }
}

/// A migrated home discloses the v2 read on stderr: rows counted,
/// twin rows skipped and WHY (upstream's V1→V2 migration copies every
/// legacy row into `session_v2`; the v1 `session` lane already
/// counted them).
#[test]
fn migrated_home_prints_the_twin_note() {
    let planted = run_with_store("migrated", "opencode-migrated-v1v2.db", &["--latest"]);
    std::fs::remove_dir_all(&planted.home).ok();
    assert!(
        planted.stderr.contains("opencode v2 store read"),
        "the note names the lane: {}",
        planted.stderr
    );
    assert!(
        planted
            .stderr
            .contains("1 row(s) counted, 4 twin row(s) skipped"),
        "the note carries the twin accounting: {}",
        planted.stderr
    );
    assert!(
        planted
            .stderr
            .contains("already counted through the v1 `session` lane"),
        "the note says why twins are skipped: {}",
        planted.stderr
    );
}

/// A fresh v2 install (no twins, no absent columns, no reason) is a
/// plain successful read: NO note — silence is the healthy state,
/// never a warning-shaped apology.
#[test]
fn fresh_v2_store_prints_nothing() {
    let planted = run_with_store("fresh", "opencode-v2-store.db", &["--latest"]);
    std::fs::remove_dir_all(&planted.home).ok();
    assert!(
        planted.success,
        "two counted sessions render: {}",
        planted.stderr
    );
    assert!(
        !planted.stderr.contains("opencode v2 store read"),
        "a clean read prints no note: {}",
        planted.stderr
    );
    assert!(
        !planted.stderr.contains("not ingested"),
        "a readable store is never an unhandled store: {}",
        planted.stderr
    );
}

/// The residual class stays a WARNING: a store whose shape the probe
/// matched but whose guarded SELECT cannot run is named on stderr
/// with the engine error. The command itself may still bail (zero
/// sessions anywhere) — the disclosure must print regardless, and it
/// does, before the empty-corpus check.
#[test]
fn unreadable_v2_store_warns() {
    let planted = run_with_store("unreadable", "opencode-v2-unreadable.db", &["--latest"]);
    std::fs::remove_dir_all(&planted.home).ok();
    assert!(
        planted
            .stderr
            .contains("agent database found but not ingested"),
        "the warning names the store: {}",
        planted.stderr
    );
    assert!(
        planted.stderr.contains("shape detected but unreadable"),
        "the warning names the residual class: {}",
        planted.stderr
    );
}

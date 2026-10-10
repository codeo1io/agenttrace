//! rm-700 bounded ingestion acceptance: a session file larger than the
//! admission cap is SKIPPED (never read, never allocated) and DISCLOSED
//! (dedicated counter + message lane), never a silent drop and never an
//! unbounded whole-file read.
//!
//! Three legs:
//! 1. `parse_file_capped` unit boundary (explicit cap, no env).
//! 2. discovery classification: an over-cap skip counts as
//!    `oversize_skipped` — NOT a `parse_failure` — and its message rides
//!    the first-failure lane (regression arm for the prefix mismatch
//!    between discovery's classifier and the parser's message).
//! 3. the data-health lane discloses `oversize_session_files_skipped`.
//!
//! Legs 2–3 mutate process env (`AGENTTRACE_MAX_SESSION_FILE_BYTES`,
//! `AGENTTRACE_SESSION_CACHE_DIR`, `HOME`) and are serialized behind one
//! mutex so parallel tests in this binary never observe a foreign cap.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use agenttrace_core::{
    data_health_scoped, load_sessions_with_options, parse_file_capped, LoadOptions,
    DEFAULT_MAX_SESSION_FILE_BYTES, OVERSIZE_SESSION_PREFIX,
};

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn unique_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agenttrace-rm700-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create scratch root");
    root
}

/// A session-shaped journal (opencode storage-doc grammar) whose header
/// line alone exceeds a 64-byte cap, so the file is DISCOVERED and then
/// rejected by admission — exactly the boundary rm-700 guards.
fn oversized_journal() -> String {
    concat!(
        "{\"id\":\"h1\",\"timestamp\":\"2026-10-10T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp/rm700\"}\n",
        "{\"id\":\"e1\",\"parentId\":\"h1\",\"timestamp\":\"2026-10-10T10:00:01.000Z\",\"type\":\"message\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"ok\"}],\"model\":\"gpt-image-1.5\",\"usage\":{\"input\":1000,\"output\":1000,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2000}}}\n"
    )
    .to_string()
}

#[test]
fn parse_file_capped_rejects_over_cap_before_reading() {
    let root = unique_root("unit");
    let journal = root.join("journal.jsonl");
    fs::write(&journal, oversized_journal()).expect("write journal");
    let len = fs::metadata(&journal).expect("stat").len();
    assert!(len > 64, "fixture must exceed the tiny cap ({} bytes)", len);

    // Over cap: the error IS the oversize skip, names the numbers and the
    // escape hatch, and never reads the file body.
    let err = parse_file_capped(&journal, 64).expect_err("over-cap file is rejected");
    let message = format!("{err:#}");
    assert!(
        message.starts_with(OVERSIZE_SESSION_PREFIX),
        "message must start with the stable classification prefix: {message}"
    );
    assert!(
        message.contains(&format!("({len} bytes > 64 bytes)")),
        "message states the measured size and cap: {message}"
    );
    assert!(
        message.contains("raise AGENTTRACE_MAX_SESSION_FILE_BYTES"),
        "message names the deliberate-raise escape hatch: {message}"
    );
    assert!(
        message.contains("journal.jsonl"),
        "message names the skipped file: {message}"
    );

    // Under cap: the same file proceeds past admission (it parses here —
    // proving admission is the only gate, not a blanket rejection).
    let under = parse_file_capped(&journal, DEFAULT_MAX_SESSION_FILE_BYTES);
    let note = under
        .as_ref()
        .err()
        .map(|e| format!("{e:#}"))
        .unwrap_or_default();
    assert!(under.is_ok(), "under-cap journal parses: {note}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn discovery_counts_over_cap_skips_as_oversize_not_parse_failures() {
    let _guard = env_lock();
    let root = unique_root("discovery");
    let corpus = root.join("corpus");
    let cache = root.join("cache");
    fs::create_dir_all(&corpus).expect("corpus dir");
    fs::create_dir_all(&cache).expect("cache dir");
    fs::write(corpus.join("journal.jsonl"), oversized_journal()).expect("journal");
    let prior_cap = std::env::var("AGENTTRACE_MAX_SESSION_FILE_BYTES").ok();
    let prior_cache = std::env::var("AGENTTRACE_SESSION_CACHE_DIR").ok();
    let prior_home = std::env::var("HOME").ok();
    std::env::set_var("AGENTTRACE_MAX_SESSION_FILE_BYTES", "64");
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache);
    std::env::set_var("HOME", &root);

    let report = load_sessions_with_options(Some(&corpus), &LoadOptions::default());

    std::env::remove_var("AGENTTRACE_MAX_SESSION_FILE_BYTES");
    if let Some(v) = prior_cap {
        std::env::set_var("AGENTTRACE_MAX_SESSION_FILE_BYTES", v);
    }
    std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
    if let Some(v) = prior_cache {
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", v);
    }
    std::env::remove_var("HOME");
    if let Some(v) = prior_home {
        std::env::set_var("HOME", v);
    }

    assert_eq!(report.discovered, 1, "the journal is discovered");
    assert!(
        report.sessions.is_empty(),
        "the over-cap file yields no session"
    );
    // The classification this phase fixed: a never-read oversize file is
    // an oversize skip, NOT a parse failure.
    assert_eq!(
        report.oversize_skipped, 1,
        "over-cap skip counted in the dedicated lane"
    );
    assert_eq!(
        report.parse_failures, 0,
        "a bounded-ingestion decision must not masquerade as a parse failure"
    );
    // Disclose, never silent: the message rides the first-failure lane.
    let first = report
        .first_parse_failure
        .as_deref()
        .expect("oversize skip keeps its message");
    assert!(
        first.starts_with(OVERSIZE_SESSION_PREFIX),
        "message lane carries the skip: {first}"
    );
    // And the data-health channel carries the dedicated counter.
    let health = data_health_scoped(
        &report.sessions,
        report.discovered,
        report.skipped,
        report.cache_hits,
        report.opencode_fork_excluded,
        report.sqlite.unreadable.clone(),
        report.oversize_skipped,
    );
    assert_eq!(
        health
            .disclosures
            .get("oversize_session_files_skipped")
            .copied()
            .unwrap_or(0),
        1,
        "data health discloses the oversize skip count"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn data_health_discloses_oversize_skips_only_when_present() {
    // Zero oversize: no disclosure key is added (guards the fold-silently
    // direction as well as the noise direction).
    let health = data_health_scoped(&[], 0, 0, 0, 0, Vec::new(), 0);
    assert!(
        !health
            .disclosures
            .contains_key("oversize_session_files_skipped"),
        "zero skips stay silent: {:?}",
        health.disclosures
    );
    let health = data_health_scoped(&[], 2, 0, 0, 0, Vec::new(), 2);
    assert_eq!(
        health
            .disclosures
            .get("oversize_session_files_skipped")
            .copied()
            .unwrap_or(0),
        2
    );
}

#[test]
fn parse_file_capped_leaves_directories_untouched() {
    // The cline task-dir lane parses a directory; admission only guards
    // single files, and a directory path must not hit the cap check.
    let root = unique_root("dir");
    let dir = root.join("taskdir");
    fs::create_dir_all(&dir).expect("task dir");
    // An empty dir fails to parse as a cline task (no files) but must NOT
    // fail with the oversize message.
    let err = parse_file_capped(&dir, 1).expect_err("empty task dir is a parse error");
    let message = format!("{err:#}");
    assert!(
        !message.starts_with(OVERSIZE_SESSION_PREFIX),
        "directories never hit the file cap: {message}"
    );
    fs::remove_dir_all(&root).ok();
}

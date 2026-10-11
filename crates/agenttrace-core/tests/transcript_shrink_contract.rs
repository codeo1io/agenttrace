//! rm-996 (run cebf20d5 cycle 2, batch "trusted instruments"):
//! a transcript rewritten IN PLACE as a strict prefix — the
//! CLAUDE_CODE_TRANSCRIPT_LOCAL_GC shape, Claude Code's local
//! garbage collector truncating old turns out of the live journal —
//! changes under the session cache's size+mtime fingerprint, the stale
//! entry is dropped, the file re-parses, and every total silently
//! drops: the warm report says the session used fewer events, tokens,
//! and dollars than the same report said a run earlier, with nothing
//! on any surface saying why. This suite pins the disclosure contract
//! the fix lands: the re-parsed session carries a `transcript_shrunk`
//! anomaly, the journal gains a bounded `transcript_shrunk` disclosure
//! ledger (mirroring `schema_invalidations`), `--doctor` surfaces it
//! beside the cache-invalidation note, and the totals themselves
//! follow the on-disk truth (annotated, never silently lowered).
//!
//! Growth (the normal append path) and first-parse (no baseline) are
//! pinned as no-false-fire controls: the disclosure fires only when
//! the re-parse comes back with FEWER events than the cached parse it
//! replaced. The check lives at the cache refresh seam — capture the
//! stale entry's totals at the fingerprint miss, compare at the store
//! — so it is layout-independent (no entry-layout or schema change;
//! `SESSION_CACHE_SCHEMA_VERSION` stays 60) and lane-agnostic even
//! though the motivating rewrite is claude-code's.
//!
//! The doctor arm pins HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR to a
//! sandbox home first (the doctor_honesty_contract pattern) because
//! `build_doctor_report` loads the real session-cache journal and
//! persists what it parses.

use agenttrace_core::{
    build_doctor_report, cached_session, load_session_cache, parse_file, render_doctor_report,
    save_session_cache, store_session, SessionCache,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within
/// this test binary (doctor_honesty_contract pattern; catch-unwind
/// keeps a failing assertion from poisoning the lock).
fn lock_env() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

fn with_home(home: &Path, f: impl FnOnce()) {
    let result = {
        let _guard = lock_env();
        let previous_home = std::env::var_os("HOME");
        let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
        let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
        let previous_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore_env("HOME", previous_home);
        restore_env("XDG_CONFIG_HOME", previous_xdg_config);
        restore_env("XDG_CACHE_HOME", previous_xdg_cache);
        restore_env("XDG_DATA_HOME", previous_xdg_data);
        restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_cache);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn temp_root(marker: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("at-rm996-shrink-{marker}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create sandbox root");
    root
}

/// One claude-code assistant turn (the claude_streamed_usage_rows
/// shape: distinct message id + claude model string is lane evidence).
fn assistant_line(i: usize) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": "2026-10-01T10:00:00Z",
        "message": {
            "id": format!("msg_{i}"),
            "model": "claude-sonnet-4-5",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_read_input_tokens": 10,
                "cache_creation_input_tokens": 5
            },
            "content": [{"type": "text", "text": format!("turn {i}")}]
        }
    })
    .to_string()
}

fn user_line(i: usize) -> String {
    serde_json::json!({
        "type": "user",
        "timestamp": "2026-10-01T09:59:00Z",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": format!("prompt {i}")}]
        }
    })
    .to_string()
}

fn journal_text(turns: usize) -> String {
    let mut lines = Vec::new();
    for i in 0..turns {
        lines.push(user_line(i));
        lines.push(assistant_line(i));
    }
    lines.join("\n") + "\n"
}

fn must_parse(path: &Path) -> agenttrace_core::Session {
    parse_file(path).expect("journal parses")
}

/// The contract: warm cache → strict-prefix rewrite (GC simulation) →
/// re-parse stores LOWER totals WITH a disclosure, never silently.
#[test]
fn strict_prefix_rewrite_is_disclosed_not_silent() {
    let root = temp_root("prefix");
    let journal = root.join("session.jsonl");

    // Warm: six turns, cached clean (no anomaly).
    fs::write(&journal, journal_text(6)).expect("write full journal");
    let warm = must_parse(&journal);
    let mut cache = SessionCache::default();
    store_session(&journal, &warm, &mut cache).expect("store warm entry");
    let served = cached_session(&journal, &mut cache).expect("warm hit");
    assert!(
        served
            .anomalies
            .iter()
            .all(|anomaly| anomaly.kind != "transcript_shrunk"),
        "a warm, unchanged transcript must carry no shrink disclosure"
    );
    assert!(cache.transcript_shrunk_records().is_empty());
    let warm_events = served.metrics.events_total;

    // GC: the journal is rewritten in place as a strict prefix (three
    // turns). Size and mtime change, so the fingerprint misses.
    fs::write(&journal, journal_text(3)).expect("rewrite as prefix");
    assert!(
        cached_session(&journal, &mut cache).is_none(),
        "a rewritten transcript must fail the fingerprint check"
    );

    // Re-parse + store: the shrink fires at the refresh seam.
    let shrunk = must_parse(&journal);
    assert!(
        shrunk.metrics.events_total < warm_events,
        "fixture premise: the prefix parses to fewer events"
    );
    store_session(&journal, &shrunk, &mut cache).expect("store shrunk entry");

    // The disclosure, on the session row itself.
    let served = cached_session(&journal, &mut cache).expect("warm hit after refresh");
    let anomaly = served
        .anomalies
        .iter()
        .find(|anomaly| anomaly.kind == "transcript_shrunk")
        .expect("the shrunk session carries a transcript_shrunk anomaly");
    assert!(
        anomaly
            .detail
            .contains(&shrunk.metrics.events_total.to_string()),
        "the anomaly names the current totals: {}",
        anomaly.detail
    );
    // The totals follow the on-disk truth — annotated, never inflated
    // back and never silently lowered.
    assert_eq!(served.metrics.events_total, shrunk.metrics.events_total);
    assert!(served.metrics.events_total < warm_events);

    // The bounded journal ledger records prior and current totals.
    let records = cache.transcript_shrunk_records();
    assert_eq!(records.len(), 1, "one shrink observation recorded");
    let record = &records[0];
    assert_eq!(
        record.get("prior_events_total").and_then(|v| v.as_u64()),
        Some(warm_events as u64),
        "the ledger keeps the prior totals: {record}"
    );
    assert_eq!(
        record.get("current_events_total").and_then(|v| v.as_u64()),
        Some(shrunk.metrics.events_total as u64),
        "the ledger names the current totals: {record}"
    );
    assert!(
        record.get("file").is_some_and(|v| v.as_str().is_some()),
        "the ledger names the file: {record}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Growth is the normal append path: fingerprint misses, re-parse
/// stores MORE events, and no disclosure may fire. First parse (no
/// cached entry at all) is the other no-false-fire control.
#[test]
fn growth_and_first_parse_never_disclose() {
    let root = temp_root("growth");
    let journal = root.join("session.jsonl");

    // Growth on a warm entry.
    fs::write(&journal, journal_text(2)).expect("write small journal");
    let small = must_parse(&journal);
    let mut cache = SessionCache::default();
    store_session(&journal, &small, &mut cache).expect("store small entry");
    fs::write(&journal, journal_text(5)).expect("append to journal");
    assert!(cached_session(&journal, &mut cache).is_none());
    let grown = must_parse(&journal);
    store_session(&journal, &grown, &mut cache).expect("store grown entry");
    let served = cached_session(&journal, &mut cache).expect("warm hit");
    assert!(
        served
            .anomalies
            .iter()
            .all(|anomaly| anomaly.kind != "transcript_shrunk"),
        "an appended transcript must not disclose a shrink"
    );
    assert!(
        cache.transcript_shrunk_records().is_empty(),
        "no growth observation may reach the ledger"
    );

    // First parse on a cold cache: no baseline, no disclosure.
    let fresh_path = root.join("session-cold.jsonl");
    fs::write(&fresh_path, journal_text(4)).expect("write cold journal");
    let cold = must_parse(&fresh_path);
    let mut cold_cache = SessionCache::default();
    store_session(&fresh_path, &cold, &mut cold_cache).expect("store cold entry");
    assert!(cold_cache.transcript_shrunk_records().is_empty());
    let _ = fs::remove_dir_all(&root);
}

/// The doctor surface: after the refresh seam persists the ledger,
/// `--doctor` names it (text and JSON) beside the cache-invalidation
/// note, and the walked session still parses (annotated, not dropped).
#[test]
fn doctor_surfaces_the_shrink_ledger() {
    let home = temp_root("doctor-home");
    with_home(&home, || {
        let corpus = home.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join("session.jsonl");

        // Warm run through the REAL cache file: store, then persist
        // through the env-pinned journal (the library embeds and the
        // loader pipeline both drive the save explicitly).
        fs::write(&journal, journal_text(6)).expect("write full journal");
        let warm = must_parse(&journal);
        let mut cache = load_session_cache();
        store_session(&journal, &warm, &mut cache).expect("store warm entry");
        save_session_cache(&mut cache).expect("persist warm journal");
        // rm-996 acceptance: a journal that never observed a shrink
        // stays byte-identical — the ledger member is absent, not
        // empty (no schema rung rides the disclosure).
        let journal_text_before =
            fs::read_to_string(home.join("cache/sessions.json")).expect("journal persisted");
        assert!(
            !journal_text_before.contains("transcript_shrunk"),
            "an untouched journal must not gain the ledger member"
        );

        // GC rewrite, then a second real run: load from disk, refresh
        // the stale entry, persist the annotated session + ledger.
        fs::write(&journal, journal_text(3)).expect("rewrite as prefix");
        let mut cache = load_session_cache();
        assert!(cached_session(&journal, &mut cache).is_none());
        let shrunk = must_parse(&journal);
        store_session(&journal, &shrunk, &mut cache).expect("store shrunk entry");
        save_session_cache(&mut cache).expect("persist shrunk journal");
        assert_eq!(cache.transcript_shrunk_records().len(), 1);
        let journal_text_after =
            fs::read_to_string(home.join("cache/sessions.json")).expect("journal persisted");
        assert!(journal_text_after.contains("transcript_shrunk"));

        let report = build_doctor_report(Some(&corpus), false);
        let note = report
            .transcript_shrunk_note
            .as_deref()
            .expect("doctor report carries the shrink disclosure");
        assert!(
            note.contains("transcript shrink"),
            "the note names the shrink: {note}"
        );
        assert_eq!(report.session_files, 1, "the walked corpus is one file");

        let text = render_doctor_report(Some(&corpus), false, "text").expect("text render");
        assert!(
            text.contains("cache shrink disclosure"),
            "doctor text carries the shrink disclosure row:\n{text}"
        );
        assert!(
            text.contains("transcript_shrunk") || note.contains("prior_events_total"),
            "the text names the observation: {text}"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

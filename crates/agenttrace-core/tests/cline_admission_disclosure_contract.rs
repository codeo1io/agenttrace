//! rm-1989 (cycle 1, 2026-10-18): Cline admission-loss disclosure
//! contract.
//!
//! The rm-1881 cline session gate refuses transcript-shaped files under
//! the Cline CLI canonical root that lack their `session.json` manifest
//! sibling (or fail the basename rule). Before rm-1989 the refusal was a
//! bare `continue` (discovery.rs): the transcript appeared in NO counter
//! — not `files`, not `failed`, not `read_cap_refused`, no skip key, no
//! stderr — so a store could silently lose spend (assess c954461c PoC
//! F3: two sessions, one manifest-less, doctor reported `files:1
//! parsed:1`).
//!
//! The contract (ROADMAP row rm-1989): on the acceptance fixture (one
//! admitted + one manifest-less session) the doctor shows files 2 /
//! parsed 1 / admission-skipped 1 — in the directory row, in the
//! report-level `cline_admission_skipped` counter, in the json
//! disclosures map, in the rendered plain report, and in the
//! empty-store explain arm. The counter is TRANSCRIPT-SHAPED: the
//! `session.json`/`hooks.jsonl` siblings the gate deliberately excludes
//! never ride it (a healthy session contributes 0, not 2, refusals),
//! and a store with zero refusals mints NO key anywhere
//! (0-count-clean, the rm-1866 opencode loss-disclosure law).

use agenttrace_core::{build_doctor_report, render_doctor_report};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Same serialization pattern as doctor_honesty_contract.rs — pins
/// HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR (no fixture reaches the real
/// cache) plus CLINE_SESSION_DATA_DIR (this contract's root).
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

fn with_env(store: &Path, f: impl FnOnce()) {
    let result = {
        let _guard = lock_env();
        let home = store.join("home");
        fs::create_dir_all(&home).expect("sandbox home");
        let previous = [
            ("HOME", std::env::var_os("HOME")),
            ("XDG_CONFIG_HOME", std::env::var_os("XDG_CONFIG_HOME")),
            ("XDG_CACHE_HOME", std::env::var_os("XDG_CACHE_HOME")),
            ("XDG_DATA_HOME", std::env::var_os("XDG_DATA_HOME")),
            (
                "AGENTTRACE_SESSION_CACHE_DIR",
                std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR"),
            ),
            (
                "CLINE_SESSION_DATA_DIR",
                std::env::var_os("CLINE_SESSION_DATA_DIR"),
            ),
        ];
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", store.join("cache"));
        // Absolute override: a relative CLINE_SESSION_DATA_DIR leaves the
        // gate lexical checks inert (discovery.rs root note).
        std::env::set_var("CLINE_SESSION_DATA_DIR", store);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        for (key, value) in previous {
            restore_env(key, value);
        }
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn temp_store(marker: &str) -> PathBuf {
    let store = std::env::temp_dir().join(format!(
        "at-rm1989-{marker}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&store);
    fs::create_dir_all(&store).expect("create cline store root");
    store
}

const ADMITTED_TRANSCRIPT: &str = concat!(
    "{\"messages\":[",
    "{\"role\":\"user\",\"content\":\"query\",\"timestamp\":\"2026-10-17T09:00:00Z\"},",
    "{\"role\":\"assistant\",\"content\":\"reply\",\"timestamp\":\"2026-10-17T09:00:05Z\",",
    "\"modelInfo\":{\"id\":\"claude-sonnet-4-5\",\"provider\":\"anthropic\"},",
    "\"metrics\":{\"tokensIn\":100,\"tokensOut\":20}}]}",
);

/// One admitted session (`sess-a` with its `session.json` manifest) plus
/// one manifest-less session (`sess-b`) — the acceptance fixture.
fn acceptance_store() -> PathBuf {
    let store = temp_store("acceptance");
    let a = store.join("sess-a");
    fs::create_dir_all(&a).unwrap();
    fs::write(a.join("sess-a.messages.json"), ADMITTED_TRANSCRIPT).unwrap();
    fs::write(a.join("session.json"), "{\"id\":\"sess-a\"}").unwrap();
    let b = store.join("sess-b");
    fs::create_dir_all(&b).unwrap();
    fs::write(b.join("sess-b.messages.json"), ADMITTED_TRANSCRIPT).unwrap();
    store
}

/// The acceptance line: files 2 / parsed 1 / admission-skipped 1, on the
/// row, the report counter, the json disclosures map, and the rendered
/// report — with the manifest sibling NOT counted as a second refusal.
#[test]
fn manifest_less_transcript_is_counted_not_silently_dropped() {
    let store = acceptance_store();
    with_env(&store, || {
        let report = build_doctor_report(None, false);
        let row = report
            .directories
            .iter()
            .find(|d| d.name.contains("Cline"))
            .expect("a Cline row renders");
        assert_eq!(row.parsed, 1, "exactly the manifest-bearing session parses");
        assert_eq!(
            row.admission_skipped, 1,
            "the manifest-less transcript is a counted refusal"
        );
        assert_eq!(
            row.files, 2,
            "found = admitted + refused (the refusal is visible in files)"
        );
        assert_eq!(report.cline_admission_skipped, 1);
        assert_eq!(
            report.disclosures.get("cline_admission_skipped_sessions"),
            Some(&1),
            "the json disclosures map carries the loss class"
        );
        let text = render_doctor_report(None, false, "text").expect("render text");
        assert!(
            text.contains("admission-skipped: 1"),
            "plain report names the refusal count: {text}"
        );
        assert!(
            text.contains("spend NOT counted"),
            "plain report states the spend consequence: {text}"
        );
    });
}

/// Transcript-shaped bound: a HEALTHY store (transcript + manifest per
/// session) reports zero refusals — the deliberately-excluded manifest
/// sibling never rides the loss counter — and mints no key anywhere
/// (0-count-clean).
#[test]
fn healthy_store_reports_zero_refusals_and_mints_no_key() {
    let store = temp_store("healthy");
    let c = store.join("sess-c");
    fs::create_dir_all(&c).unwrap();
    fs::write(c.join("sess-c.messages.json"), ADMITTED_TRANSCRIPT).unwrap();
    fs::write(c.join("session.json"), "{\"id\":\"sess-c\"}").unwrap();
    // A hooks sibling must not count either.
    fs::write(c.join("hooks.jsonl"), "{}\n").unwrap();
    with_env(&store, || {
        let report = build_doctor_report(None, false);
        let row = report
            .directories
            .iter()
            .find(|d| d.name.contains("Cline"))
            .expect("a Cline row renders");
        assert_eq!(row.parsed, 1);
        assert_eq!(row.admission_skipped, 0, "manifest/hooks are not losses");
        assert_eq!(row.files, 1, "found counts the transcript only");
        assert_eq!(report.cline_admission_skipped, 0);
        assert!(
            !report.disclosures.contains_key("cline_admission_skipped_sessions"),
            "0-count-clean: no disclosures key without a loss"
        );
        let text = render_doctor_report(None, false, "text").expect("render text");
        assert!(
            !text.contains("admission-skipped"),
            "no refusal line without a refusal: {text}"
        );
        let json = serde_json::to_string(&report).expect("report serializes");
        assert!(
            !json.contains("cline_admission_skipped"),
            "0-count-clean json (field skips at zero): {json}"
        );
    });
}

/// Empty-store explain arm: a store whose EVERY transcript was refused
/// says so (sessions == 0 + cline_admission_skipped > 0) instead of the
/// generic "point HOME elsewhere" advice.
#[test]
fn all_refused_store_explains_the_admission_loss() {
    let store = temp_store("allrefused");
    let b = store.join("sess-d");
    fs::create_dir_all(&b).unwrap();
    fs::write(b.join("sess-d.messages.json"), ADMITTED_TRANSCRIPT).unwrap();
    with_env(&store, || {
        let report = build_doctor_report(None, false);
        assert_eq!(report.sessions, 0, "nothing was admitted");
        assert_eq!(report.cline_admission_skipped, 1);
        let text = render_doctor_report(None, false, "text").expect("render text");
        assert!(
            text.contains("cline session gate"),
            "empty-store explain names the gate: {text}"
        );
        assert!(
            text.contains("restore the session.json manifests"),
            "empty-store explain gives the repair path: {text}"
        );
    });
}

//! rm-1010 dir-scope cache-isolation contract. The parsed-session
//! cache indexes the HOME-rooted corpus; a `-d`/`--dir` run analyses
//! a directory that corpus was never scoped to, so it must consult
//! and rewrite NO cache:
//!
//! 1. no consult — a poisoned marker entry keyed at a path inside
//!    `-d` (valid schema + fingerprint, fabricated session) must be
//!    ignored: the source parse wins;
//! 2. no rewrite — the cache file stays byte-identical across
//!    dir-scoped doctor and load runs (and is never created when
//!    absent);
//! 3. honest disclosure — the dir-scoped doctor report names the
//!    cache as out of scope instead of rendering zeroed counts as if
//!    the operator's cache were empty.
//!
//! The global lane (dir None) keeps the rm-367 stage (a)
//! persist-and-reuse behavior — pinned by doctor_honesty_contract
//! (global persistence) and discovery_contract (Go-compatible cache
//! round-trip). The control arm at the bottom of the poison test
//! proves the ignored marker was servable, so the isolation is the
//! fix, not an accidental fingerprint invalidation.

use agenttrace_core::{
    build_doctor_report, load_sessions_from_dir, load_sessions_with_options, LoadOptions,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const CACHE_SCOPE_NOTE: &str =
    "cache out of scope: -d/--dir scans consult and rewrite no session cache";

const PLANTED_SESSION: &str = concat!(
    "{\"role\":\"session_meta\",\"timestamp\":\"2026-05-01T09:00:00Z\",\"ModelUsed\":\"claude-sonnet-4-5\"}\n",
    "{\"role\":\"meta\",\"ModelUsed\":\"claude-sonnet-4-5\",\"Usage\":{\"input_tokens\":120,\"output_tokens\":30}}\n",
);

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serialize + panic-safe pin of every env var the cache scoping
/// reads (the HOME/XDG bases and the cache overrides themselves),
/// restoring the PREVIOUS values on the way out. doctor_honesty_
/// contract's `with_home` established the pattern; rm-1010 applies it
/// to every fixture that touches the session cache so the lanes can
/// never assert against real operator state.
fn with_pinned_home(home: &Path, f: impl FnOnce()) {
    let result = {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut previous: Vec<(&'static str, Option<std::ffi::OsString>)> = vec![
            ("HOME", std::env::var_os("HOME")),
            ("XDG_CONFIG_HOME", std::env::var_os("XDG_CONFIG_HOME")),
            ("XDG_CACHE_HOME", std::env::var_os("XDG_CACHE_HOME")),
            ("XDG_DATA_HOME", std::env::var_os("XDG_DATA_HOME")),
            (
                "AGENTTRACE_SESSION_CACHE_DIR",
                std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR"),
            ),
            (
                "AGENTTRACE_SESSION_CACHE_ENTRIES",
                std::env::var_os("AGENTTRACE_SESSION_CACHE_ENTRIES"),
            ),
        ];
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        std::env::remove_var("AGENTTRACE_SESSION_CACHE_ENTRIES");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        for (key, value) in previous.drain(..) {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

/// Sandbox home whose ONLY session corpus lives under
/// `~/.hermes/sessions` — a known session dir, so global discovery
/// (dir None) finds exactly the planted journal.
fn temp_home(marker: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("at-rm1010-{marker}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let sessions = root.join(".hermes").join("sessions");
    fs::create_dir_all(&sessions).expect("create sandbox sessions dir");
    (root, sessions)
}

fn escape_json_pointer(path: &Path) -> String {
    path.to_string_lossy().replace('~', "~0").replace('/', "~1")
}

#[test]
fn dir_scoped_runs_ignore_a_poisoned_marker_and_rewrite_nothing() {
    let (root, sessions_dir) = temp_home("poison");
    with_pinned_home(&root, || {
        let journal = sessions_dir.join("session.jsonl");
        fs::write(&journal, PLANTED_SESSION).expect("plant journal");

        // Bank a real entry through the global lane — the only lane
        // the cache is scoped to — then poison it in place: same
        // schema version, same mod_time/size fingerprint, fabricated
        // session name. This is exactly the marker class the assess
        // F3 PoC planted in the operator cache.
        let banked = load_sessions_from_dir(None);
        assert_eq!(banked.len(), 1, "global lane must bank the planted session");
        let cache_path = root.join("cache").join("sessions.json");
        assert!(cache_path.is_file(), "global lane must create the cache");
        let raw = fs::read_to_string(&cache_path).expect("read banked cache");
        let mut doc: Value = serde_json::from_str(&raw).expect("banked cache json");
        let entry = doc
            .pointer_mut(&format!("/entries/{}", escape_json_pointer(&journal)))
            .expect("banked entry for the journal path");
        *entry
            .pointer_mut("/session/Name")
            .expect("session name in the banked entry") =
            Value::String("POISONED-rm-1010".to_string());
        fs::write(&cache_path, serde_json::to_string(&doc).unwrap()).expect("poison cache");
        let poisoned_bytes = fs::read(&cache_path).expect("snapshot poisoned cache");

        // Dir-scoped load over the same directory: the poisoned marker
        // must be ignored (source parse wins, zero consults) and the
        // cache file must stay byte-identical.
        let scoped = load_sessions_with_options(Some(&sessions_dir), &LoadOptions::default());
        assert_eq!(
            scoped.cache_hits, 0,
            "rm-1010: a dir-scoped load must not consult the session cache"
        );
        assert_eq!(scoped.sessions.len(), 1);
        assert_ne!(
            scoped.sessions[0].name, "POISONED-rm-1010",
            "a poisoned marker keyed inside -d must not be served"
        );
        assert_eq!(
            fs::read(&cache_path).expect("cache after scoped load"),
            poisoned_bytes,
            "rm-1010: a dir-scoped load must not rewrite the cache"
        );

        // Dir-scoped doctor: the same isolation, plus zeroed counts
        // that NAME the out-of-scope cache instead of implying an
        // empty operator index.
        let doctor = build_doctor_report(Some(&sessions_dir), false);
        assert_eq!(doctor.cache_entries, 0, "scoped doctor consults no cache");
        assert_eq!(doctor.cached_valid, 0);
        assert_eq!(doctor.cache_size_bytes, 0);
        assert_eq!(doctor.cache_scope_note.as_deref(), Some(CACHE_SCOPE_NOTE));
        assert_eq!(
            fs::read(&cache_path).expect("cache after scoped doctor"),
            poisoned_bytes,
            "rm-1010: a dir-scoped doctor run must not rewrite the cache"
        );

        // CONTROL: the marker is still servable — the global lane
        // serves it (the cache is scoped to that corpus). This proves
        // the scoped runs ignored a VALID entry rather than tripping
        // fingerprint/schema validation.
        let served = load_sessions_with_options(None, &LoadOptions::default());
        assert_eq!(
            served.cache_hits, 1,
            "control: the poisoned marker must be servable"
        );
        assert_eq!(
            served.sessions.into_iter().next().unwrap().name,
            "POISONED-rm-1010",
            "control: the global lane serves the poisoned entry"
        );
        assert_eq!(
            fs::read(&cache_path).expect("cache after control"),
            poisoned_bytes,
            "control: serving from a clean cache rewrites nothing"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn dir_scoped_runs_never_create_the_cache_and_disclose_the_scope() {
    let (root, sessions_dir) = temp_home("fresh");
    with_pinned_home(&root, || {
        let journal = sessions_dir.join("session.jsonl");
        fs::write(&journal, PLANTED_SESSION).expect("plant journal");

        // From a cold start (no cache file at all), dir-scoped runs
        // must never create one — twice over: load, then doctor.
        for _ in 0..2 {
            let scoped = load_sessions_with_options(Some(&sessions_dir), &LoadOptions::default());
            assert_eq!(scoped.cache_hits, 0);
            assert_eq!(scoped.sessions.len(), 1);
        }
        let cache_path = root.join("cache").join("sessions.json");
        assert!(
            !cache_path.exists(),
            "rm-1010: dir-scoped loads must never create the session cache"
        );
        let _ = build_doctor_report(Some(&sessions_dir), false);
        assert!(
            !cache_path.exists(),
            "rm-1010: dir-scoped doctor runs must never create the session cache"
        );

        // The scope disclosure renders in both doctor formats: the
        // text block names the out-of-scope cache; the JSON carries
        // the additive cache_scope_note field (absent on global runs).
        let text = agenttrace_core::render_doctor_report(Some(&sessions_dir), false, "text")
            .expect("render scoped doctor text");
        assert!(
            text.contains(&format!("cache scope: {CACHE_SCOPE_NOTE}")),
            "text doctor must disclose the scope note:\n{text}"
        );
        assert!(
            text.contains("0 parsed session cache entries"),
            "scoped doctor renders zeroed counts with the note"
        );
        let json = agenttrace_core::render_doctor_report(Some(&sessions_dir), false, "json")
            .expect("render scoped doctor json");
        let doc: Value = serde_json::from_str(&json).expect("scoped doctor json");
        assert_eq!(
            doc.pointer("/cache_scope_note").and_then(Value::as_str),
            Some(CACHE_SCOPE_NOTE)
        );
        assert_eq!(
            doc.pointer("/cache_entries").and_then(Value::as_i64),
            Some(0)
        );

        // Global runs carry no scope note (the field is additive and
        // skipped when the cache IS in scope).
        let global_json = agenttrace_core::render_doctor_report(None, false, "json")
            .expect("render global doctor json");
        let global_doc: Value = serde_json::from_str(&global_json).expect("global doctor json");
        assert!(
            global_doc.pointer("/cache_scope_note").is_none(),
            "global runs keep their JSON shape (no scope note)"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

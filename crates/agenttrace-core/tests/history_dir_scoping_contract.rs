//! rm-1047 + rm-249 + rm-089 contract: a resolved history directory
//! (`--history-dir` > config file > `AGENTTRACE_HISTORY_DIR`) scopes
//! EVERY corpus-facing read to exactly that directory — discovery, the
//! report loaders, and the doctor inventory — even when the discovery
//! cache already holds rows from the real agent homes (the assess PoC:
//! pinning an empty history dir used to report the entire 30,769-session
//! home corpus); the discovery walk discloses directories it cannot read
//! instead of narrowing silently (rm-249); `-m <model>` prices the
//! `--compare` corpus at the named model's rates and targets
//! `--test-match` at one lookup (rm-089).
//!
//! Environment-mutating tests serialize on one lock per binary
//! (subagent_attribution / doctor_honesty_contract patterns) and
//! restore every variable they touch.

use agenttrace_core::{
    build_doctor_report, cache_root_disabled_reason, cache_root_warning_text,
    discover_session_dirs, find_session_files, find_session_files_reported,
    load_sessions_with_progress_from_cache, render_doctor_report, render_test_match,
    render_test_match_for_model, report_compare_json_with_model, session_cache_path, Session,
    SessionCache, LoadOptions,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Serializes process-global environment mutation (HOME/XDG cache pins,
/// AGENTTRACE_HISTORY_DIR) across this binary's lanes.
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

/// One minimal parseable session transcript (Claude-style jsonl).
const SAMPLE_JSONL: &str = r#"{"role":"session_meta","timestamp":"2026-10-12T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"meta","ModelUsed":"claude-sonnet-4","Usage":{"input_tokens":1000,"output_tokens":500}}
{"role":"user","content":"scope check","timestamp":"2026-10-12T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"assistant","content":"ok","timestamp":"2026-10-12T10:00:01Z","ModelUsed":"claude-sonnet-4"}
"#;

fn seed_session(dir: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("seed dir");
    let path = dir.join(name);
    fs::write(&path, SAMPLE_JSONL).expect("seed session");
    path
}

/// Pins AGENTTRACE_HISTORY_DIR for the closure and restores it after
/// (catch-unwind so a failing assertion cannot poison the env).
/// Acquires the env lock; `std::sync::Mutex` is NOT reentrant, so code
/// already inside `with_home` must use [`with_history_dir_locked`]
/// instead (nesting these two was the prior attempt's deadlock).
fn with_history_dir(pin: &Path, f: impl FnOnce()) {
    let _guard = lock_env();
    with_history_dir_locked(pin, f);
}

/// [`with_history_dir`] without acquiring the env lock — for closures
/// already running inside [`with_home`], which holds it.
fn with_history_dir_locked(pin: &Path, f: impl FnOnce()) {
    let previous = std::env::var_os("AGENTTRACE_HISTORY_DIR");
    std::env::set_var("AGENTTRACE_HISTORY_DIR", pin);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    restore_env("AGENTTRACE_HISTORY_DIR", previous);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

/// The doctor_honesty_contract isolation: pin HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR
/// to a temp home so no doctor test can read or write the operator's
/// real cache or agent homes.
fn with_home(home: &Path, f: impl FnOnce()) {
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
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn history_dir_scopes_discovery_to_the_pinned_root() {
    let tmp = tempdir();
    let elsewhere = tmp.path.join("elsewhere");
    let pinned = tmp.path.join("pinned");
    seed_session(&elsewhere, "other-session.jsonl");
    seed_session(&pinned, "pinned-session.jsonl");

    with_history_dir(&pinned, || {
        let dirs = discover_session_dirs();
        assert_eq!(
            dirs,
            vec![pinned.clone()],
            "a resolved history_dir must be the ONLY discovery root"
        );
        let files = find_session_files(None);
        assert_eq!(
            files.len(),
            1,
            "scoped discovery must not leak the unscoped corpus"
        );
        assert!(
            files[0].ends_with("pinned-session.jsonl"),
            "the scoped corpus is the pinned root's files, got {:?}",
            files
        );
    });
}

#[test]
fn history_dir_scoping_survives_a_populated_session_cache() {
    // The assess PoC as a regression test: a cache warmed by the
    // UNSCOPED walk (here: the explicit elsewhere dir) must not re-leak
    // those sessions once a history_dir pins a different corpus.
    let tmp = tempdir();
    let elsewhere = tmp.path.join("elsewhere");
    let pinned = tmp.path.join("pinned");
    seed_session(&elsewhere, "other-session.jsonl");
    seed_session(&pinned, "pinned-session.jsonl");

    // Warm the cache with the unscoped-corpus directory.
    let mut cache = SessionCache::default();
    let unscoped = load_sessions_with_progress_from_cache(
        Some(&elsewhere),
        &LoadOptions::default(),
        &mut cache,
        |_| {},
    );
    assert_eq!(unscoped.sessions.len(), 1, "fixture premise");

    with_history_dir(&pinned, || {
        let report = load_sessions_with_progress_from_cache(
            None,
            &LoadOptions::default(),
            &mut cache,
            |_| {},
        );
        assert_eq!(
            report.sessions.len(),
            1,
            "the pinned corpus is exactly the pinned root's session — cached rows from other roots must not render"
        );
        assert!(
            report
                .sessions
                .iter()
                .all(|session: &Session| session.path.ends_with("pinned-session.jsonl")),
            "scoped load leaked an unscoped session: {:?}",
            report.sessions.iter().map(|s| &s.path).collect::<Vec<_>>()
        );
    });
}

#[test]
fn history_dir_pin_to_a_missing_directory_reads_as_scoped_empty_not_home() {
    let tmp = tempdir();
    let pin = tmp.path.join("does-not-exist");
    with_history_dir(&pin, || {
        assert_eq!(
            discover_session_dirs(),
            vec![pin.clone()],
            "a bad pin is the scoped corpus, never a fallthrough to the home roots"
        );
        let (files, unreadable) = find_session_files_reported(None);
        assert!(files.is_empty(), "missing pin discovers nothing");
        assert!(
            !unreadable.is_empty(),
            "the walk discloses that the pinned root itself could not be read"
        );
        assert_eq!(
            unreadable[0].path, pin,
            "the disclosure names the pinned root"
        );
    });
}

#[cfg(unix)]
#[test]
fn unreadable_subdirectory_is_reported_without_failing_the_walk() {
    let tmp = tempdir();
    let root = tmp.path.join("root");
    seed_session(&root, "readable.jsonl");
    let locked = root.join("locked");
    fs::create_dir_all(&locked).expect("locked dir");
    seed_session(&locked, "hidden.jsonl");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod 000");

    let (files, unreadable) = find_session_files_reported(Some(&root));
    // Restore readable state regardless of assertions below.
    let _ = fs::set_permissions(&locked, fs::Permissions::from_mode(0o755));

    assert!(
        files.iter().any(|path| path.ends_with("readable.jsonl")),
        "the walk stays non-fatal: readable siblings still report"
    );
    assert!(
        !files.iter().any(|path| path.ends_with("hidden.jsonl")),
        "the locked subtree's files stay out of the corpus"
    );
    let entry = unreadable
        .iter()
        .find(|dir| dir.path == locked)
        .unwrap_or_else(|| panic!("locked dir must be disclosed, got {unreadable:?}"));
    assert!(
        entry.reason.contains("denied") || entry.reason.contains("permission"),
        "the disclosure carries the io story, got {:?}",
        entry.reason
    );
}

#[test]
fn doctor_discloses_scope_unreadable_dirs_and_cache_disability() {
    let tmp = tempdir();
    let home = tmp.path.join("home");
    let pinned = tmp.path.join("pinned");
    fs::create_dir_all(&home).expect("home");
    seed_session(&pinned, "pinned-session.jsonl");

    with_home(&home, || {
        // rm-1047 arm 2, seeded-integration half: the temp HOME carries a
        // REAL agent-home corpus (a discoverable .hermes/sessions
        // journal) while the history_dir pin points elsewhere — the
        // doctor inventory must be exactly the pinned corpus, never
        // the populated home.
        seed_session(&home.join(".hermes").join("sessions"), "home-session.jsonl");
        // Plant a FILE where the cache ROOT directory belongs, so the
        // cache is unusable and the doctor must say so.
        let cache_root = session_cache_path()
            .parent()
            .expect("cache root")
            .to_path_buf();
        fs::create_dir_all(cache_root.parent().expect("grandparent"))
            .expect("grandparent");
        fs::write(&cache_root, "not a directory").expect("file-as-root");

        with_history_dir_locked(&pinned, || {
            let report = build_doctor_report(None, false);
            let scope = report
                .history_dir_scope
                .as_deref()
                .expect("scoped doctor discloses the scope");
            assert!(
                scope.contains("pinned"),
                "scope row names the pinned root: {scope}"
            );
            assert!(
                scope.contains("scoped to exactly this directory"),
                "scope row states the scoping semantics: {scope}"
            );
            let disabled = report
                .cache_disabled_reason
                .as_deref()
                .expect("file-as-cache-root must read as disabled");
            assert!(
                disabled.contains("not a directory"),
                "disabled reason states the story: {disabled}"
            );

            let text = render_doctor_report(None, false, "text").expect("render text");
            assert!(
                text.contains("History dir scoping:"),
                "text lane carries the scope row before any counts:\n{text}"
            );
            assert!(
                text.contains("disabled:"),
                "text lane carries the cache-disabled row:\n{text}"
            );
            assert!(
                text.contains("Session files: 1"),
                "the scoped corpus is the pinned session, not the populated home:\n{text}"
            );
            assert!(
                !text.contains("home-session.jsonl"),
                "the home corpus must not leak into the scoped inventory:\n{text}"
            );
        });
    });
}

#[test]
fn cache_root_warning_and_disabled_reason_formats() {
    // Pure formatters — pin the exact stderr/doctor text so the two
    // surfaces cannot drift apart.
    let warning = cache_root_warning_text("permission denied", Path::new("/x/y"));
    assert!(
        warning.starts_with("agenttrace: warning: session cache disabled (permission denied)"),
        "warning names the condition first: {warning}"
    );
    assert!(
        warning.contains("/x/y"),
        "warning names the unusable root: {warning}"
    );
    assert!(
        warning.contains("every run re-parses sessions from source"),
        "warning states the consequence: {warning}"
    );

    let tmp = tempdir();
    let home = tmp.path.join("home");
    fs::create_dir_all(&home).expect("home");
    with_home(&home, || {
        // Order matters: the disabled-probe's NotFound arm CREATES the
        // root, so plant the file-as-root FIRST, then unplant it to
        // exercise the creatable-root (None) story on the same fixture.
        let cache_root = session_cache_path()
            .parent()
            .expect("cache root")
            .to_path_buf();
        fs::create_dir_all(cache_root.parent().expect("grandparent"))
            .expect("grandparent");
        fs::write(&cache_root, "file").expect("file-as-root");
        let reason = cache_root_disabled_reason().expect("file root is disabled");
        assert!(reason.contains("not a directory"), "story: {reason}");
        fs::remove_file(&cache_root).expect("unplant file-as-root");
        assert_eq!(
            cache_root_disabled_reason(),
            None,
            "a creatable cache root under the temp home is not disabled"
        );
    });
}

#[test]
fn test_match_targets_the_named_model() {
    // rm-089: -m <model> --test-match prints the named model's lookup,
    // not the fixed probe set.
    let out = render_test_match_for_model("claude-sonnet-4-5-20250929");
    assert!(
        out.contains("claude-sonnet-4-5-20250929"),
        "the named model renders: {out}"
    );
    assert!(
        !out.contains("unknown-model-xyz"),
        "the fixed probe set stays on the default lane: {out}"
    );
    // The default lane is unchanged: all probes, one line each.
    let baseline = render_test_match();
    for probe in [
        "claude-sonnet-4-5-20250929",
        "unknown-model-xyz",
        "gemini-2.5-pro",
    ] {
        assert!(baseline.contains(probe), "default lane keeps {probe}");
    }
}

fn parse_session(dir: &Path) -> Vec<Session> {
    let path = seed_session(dir, "session.jsonl");
    let session = agenttrace_core::parse_file(&path).expect("parse fixture");
    vec![session]
}

#[test]
fn compare_json_envelope_prices_the_named_reference_model() {
    let tmp = tempdir();
    let sessions = parse_session(&tmp.path.join("corpus-a"));
    let mut sessions = sessions;
    let more = parse_session(&tmp.path.join("corpus-b"));
    sessions.extend(more);

    // Catalog model: real rates, fallback flag false.
    let json = report_compare_json_with_model(&sessions, "claude-sonnet-4-5-20250929");
    let value: serde_json::Value = serde_json::from_str(&json).expect("envelope parses");
    assert_eq!(value["reference_model"], "claude-sonnet-4-5-20250929");
    assert_eq!(
        value["sessions"]
            .as_array()
            .expect("sessions array")
            .len(),
        2
    );
    assert_eq!(value["default_rate_fallback"], false);
    let reference = value["reference_cost_estimated"].as_f64().expect("f64");
    assert!(reference > 0.0, "catalog rates price above zero: {reference}");

    // Unknown model: the fallback flag must be honest.
    let json = report_compare_json_with_model(&sessions, "no-such-model-anywhere");
    let value: serde_json::Value = serde_json::from_str(&json).expect("envelope parses");
    assert_eq!(value["default_rate_fallback"], true);
    let fallback = value["reference_cost_estimated"].as_f64().expect("f64");
    assert!(fallback > 0.0, "default rates still price: {fallback}");
}

/// Minimal tempdir helper (std-only; avoids a dev-dependency for one call site).
fn tempdir() -> TempDir {
    TempDir::new()
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir();
        let unique = format!(
            "agenttrace-rm1047-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let path = base.join(unique);
        fs::create_dir_all(&path).expect("tempdir");
        TempDir { path }
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

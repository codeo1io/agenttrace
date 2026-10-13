//! rm-440 acceptance: a 0-byte session file next to a parseable one is
//! ONE disclosed class in BOTH views — doctor (`empty`, excluded from
//! `failed`) and the data_health lane (`empty_files`, excluded from
//! `skipped`) — and never a parse failure or failure sample. The live
//! 21,186-session corpus that minted the row showed exactly this shape:
//! its only "failure" was a 0-byte file the parser never should have
//! been counted against.

use agenttrace_core::{
    build_doctor_report, data_health_scoped, load_sessions_with_options, render_doctor_report,
    LoadOptions,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_env() -> MutexGuard<'static, ()> {
    let lock = LOCK.get_or_init(|| Mutex::new(()));
    match lock.lock() {
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

/// HOME/XDG-isolated sandbox: the loader must not touch this host's
/// real cache, and the doctor must not scan the real session roots.
fn with_isolated_env(home: &Path, f: impl FnOnce()) {
    let _guard = lock_env();
    let previous_home = std::env::var_os("HOME");
    let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
    let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
    let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
    let previous_session_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
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
    restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_session_cache);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn sandbox(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agenttrace-rm440-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create sandbox");
    root
}

const PARSEABLE: &str = r#"{"role":"session_meta","timestamp":"2026-05-02T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"user","content":"Inspect billing export.","timestamp":"2026-05-02T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"assistant","content":"I will inspect the route.","timestamp":"2026-05-02T10:00:01Z","ModelUsed":"claude-sonnet-4"}
"#;

#[test]
fn zero_byte_file_is_one_disclosed_class_in_both_views() {
    let root = sandbox("both-views");
    let sessions_dir = root.join("sessions");
    fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    fs::write(sessions_dir.join("live.jsonl"), PARSEABLE).expect("write parseable journal");
    // The fixture the row was minted from: a 0-byte file sitting next to
    // perfectly good journals, doctor's only "failure" on the corpus.
    fs::write(sessions_dir.join("truncated.jsonl"), "").expect("write zero-byte journal");

    with_isolated_env(&root, || {
        let report = load_sessions_with_options(Some(&sessions_dir), &LoadOptions::default());
        assert_eq!(report.discovered, 2, "both files are discovered");
        assert_eq!(
            report.sessions.len(),
            1,
            "only the parseable journal yields a session"
        );
        assert_eq!(
            report.empty, 1,
            "loader discloses the 0-byte file as `empty`"
        );
        assert_eq!(report.skipped, 0, "the 0-byte file is NOT a skip/failure");
        assert_eq!(report.parse_failures, 0, "the parser never ran on it");

        // data_health: same class, same count, one name.
        let health = data_health_scoped(
            &report.sessions,
            report.discovered,
            report.skipped,
            report.empty,
            report.cache_hits,
            report.opencode_fork_excluded,
            report.sqlite.unreadable.clone(),
        );
        assert_eq!(health.empty_files, 1, "health reports the same empty class");
        assert_eq!(health.skipped, 0, "health no longer counts it as skipped");
        assert_eq!(
            health.out_of_scope, 0,
            "and it is not smuggled into out_of_scope either"
        );
        let json = serde_json::to_string(&health).expect("serialize health");
        assert!(
            json.contains("\"empty_files\":1"),
            "disclosed in JSON: {json}"
        );

        // Doctor: `failed` excludes it; `empty` carries it with a sample.
        let doctor = build_doctor_report(Some(&sessions_dir), false);
        let dir = doctor
            .directories
            .iter()
            .find(|dir| Path::new(&dir.path) == sessions_dir)
            .expect("doctor row for the explicit dir");
        assert_eq!(dir.files, 2);
        assert_eq!(dir.parsed, 1);
        assert_eq!(
            dir.failed, 0,
            "doctor no longer counts the 0-byte file as failed"
        );
        assert_eq!(dir.empty, 1, "doctor reports the same empty class");
        assert!(
            dir.empty_samples
                .iter()
                .any(|sample| sample.contains("truncated.jsonl")),
            "empty sample names the file: {:?}",
            dir.empty_samples
        );
        assert!(
            dir.failure_samples.is_empty(),
            "no failure sample for the 0-byte file"
        );

        // Rendered text: the row carries empty=1, the sample line says
        // `empty:`, and no `failed:` line points at the 0-byte file.
        let text =
            render_doctor_report(Some(&sessions_dir), false, "text").expect("render doctor report");
        assert!(
            text.contains("empty=1"),
            "row discloses empty count:\n{text}"
        );
        assert!(
            text.contains("empty: ") && text.contains("truncated.jsonl"),
            "sample line rendered:\n{text}"
        );
        assert!(
            !text
                .lines()
                .any(|line| line.starts_with("    failed: ") && line.contains("truncated.jsonl")),
            "no failure line for the 0-byte file:\n{text}"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn clean_corpus_serializes_without_the_empty_key() {
    // 0-count-clean: when the class is absent the disclosure key is
    // absent, so healthy corpora serialize identically to pre-rm-440
    // output (the gate-visible JSON contract does not churn).
    let root = sandbox("clean-corpus");
    let sessions_dir = root.join("sessions");
    fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    fs::write(sessions_dir.join("live.jsonl"), PARSEABLE).expect("write parseable journal");

    with_isolated_env(&root, || {
        let report = load_sessions_with_options(Some(&sessions_dir), &LoadOptions::default());
        assert_eq!(report.discovered, 1);
        assert_eq!(report.empty, 0);
        let health = data_health_scoped(
            &report.sessions,
            report.discovered,
            report.skipped,
            report.empty,
            report.cache_hits,
            report.opencode_fork_excluded,
            report.sqlite.unreadable.clone(),
        );
        let json = serde_json::to_string(&health).expect("serialize health");
        assert!(!json.contains("empty_files"), "key absent at zero: {json}");

        let doctor = build_doctor_report(Some(&sessions_dir), false);
        let dir = doctor
            .directories
            .iter()
            .find(|dir| Path::new(&dir.path) == sessions_dir)
            .expect("doctor row for the explicit dir");
        assert_eq!(dir.failed, 0);
        assert_eq!(dir.empty, 0);
        let rendered =
            render_doctor_report(Some(&sessions_dir), false, "text").expect("render doctor report");
        assert!(
            !rendered.contains("empty="),
            "no empty note on a clean row:\n{rendered}"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

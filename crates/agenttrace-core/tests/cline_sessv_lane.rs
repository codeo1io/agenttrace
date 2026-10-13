//! rm-1032: the Cline sess-v storage generation and the standalone
//! CLI homes, on the documented shapes of ccusage PR #1642 (closed by
//! the legacy contributor gate 2026-08-31 — agenttrace implements the
//! format on its own evidence; see tests/fixtures/cline-sessv/README.md).
//!
//! Covered:
//! - the sess-v transcript body (`<dir>/<dir>.messages.json`) parses
//!   with per-message usage counting on meta events, generation
//!   disclosure, recorded-cost deferral, and no same-timestamp dedup
//!   loss;
//! - the legacy task-dir generation keeps parsing and carries its own
//!   generation counter (doctor census separates the two);
//! - discovery admits the sess-v SESSION DIR (never the body file),
//!   under the VSCode tasks root, `~/.config/cline`, `~/.cline`, and
//!   `CLINE_HOME` (comma-separated like the reference collector);
//! - the doctor's disclosure census folds both generation counters.

use agenttrace_core::{
    build_doctor_report, collect_session_files, find_session_files, known_session_dirs, parse_file,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cline-sessv");

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        None => std::env::remove_var(key),
        Some(value) => std::env::set_var(key, value),
    }
}

fn with_env(env: &[(&str, Option<std::ffi::OsString>)], f: impl FnOnce()) {
    let result = {
        let _guard = lock_env();
        let saved: Vec<(&str, Option<std::ffi::OsString>)> = env
            .iter()
            .map(|(key, _)| (*key, std::env::var_os(key)))
            .collect();
        for (key, value) in env {
            match value {
                None => std::env::remove_var(key),
                Some(value) => std::env::set_var(key, value),
            }
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        for (key, previous) in saved {
            restore_env(key, previous);
        }
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn copy_fixture(from: &Path, to: &Path) {
    fs::create_dir_all(to.parent().expect("fixture target parent")).expect("create target dir");
    fs::copy(from, to).expect("copy fixture");
}

/// `~/.config/cline` holding one sess-v session under `data/sessions`
/// (the CLI layout) and the fixture pair under `tasks/`.
fn write_cline_home(home: &Path) -> PathBuf {
    let root = home.join(".config").join("cline");
    copy_fixture(
        &Path::new(FIXTURES)
            .join("sess-v")
            .join("sess-v.messages.json"),
        &root
            .join("data")
            .join("sessions")
            .join("sess-a")
            .join("sess-a.messages.json"),
    );
    copy_fixture(
        &Path::new(FIXTURES)
            .join("sess-v")
            .join("sess-v.messages.json"),
        &root
            .join("tasks")
            .join("sess-v")
            .join("sess-v.messages.json"),
    );
    for name in ["api_conversation_history.json", "task_metadata.json"] {
        copy_fixture(
            &Path::new(FIXTURES).join("legacy-task").join(name),
            &root.join("tasks").join("legacy-task").join(name),
        );
    }
    root
}

#[test]
fn sessv_dir_parses_with_usage_and_generation_disclosure() {
    let dir = Path::new(FIXTURES).join("sess-v");
    let session = parse_file(&dir).expect("parse sess-v fixture dir");
    let metrics = &session.metrics;
    assert_eq!(metrics.source_tool, "cline");
    // Three usage-bearing assistant messages (the user message and the
    // empty-metrics message are skipped like the reference loader).
    assert_eq!(metrics.events_total, 3);
    assert_eq!(metrics.tokens_input, 1200 + 950 + 500);
    assert_eq!(metrics.tokens_output, 300 + 210 + 90);
    assert_eq!(metrics.tokens_cache_r, 800 + 640);
    assert_eq!(metrics.tokens_cache_w, 250 + 60);
    // Multi-model corpus: attribution machinery sees both models.
    assert_eq!(metrics.model_used, "multiple");
    // Generation disclosed; recorded cost deferred (rm-253 family).
    assert_eq!(
        metrics.disclosure_counters.get("cline_generation:sess-v"),
        Some(&3)
    );
    assert_eq!(
        metrics
            .disclosure_counters
            .get("cline_recorded_cost_deferred"),
        Some(&1)
    );
    // Estimated, not recorded: the deferral stays visible.
    assert!(metrics.cost_estimated > 0.0);
    assert_eq!(metrics.provenance.cost, "calculated_per_message_tokens");
    // Millisecond timestamps resolved to real RFC 3339 bounds.
    assert!(metrics.session_start.starts_with("2025-10-"));
    assert!(metrics.session_end.starts_with("2025-10-"));
}

#[test]
fn sessv_same_timestamp_messages_both_count() {
    // The two 1760123415000 messages must both land: the sess-v lane
    // counts each metrics-bearing message directly instead of the
    // legacy key dedup (empty content + same ts would erase usage).
    let dir = Path::new(FIXTURES).join("sess-v");
    let session = parse_file(&dir).expect("parse");
    assert_eq!(session.metrics.events_total, 3);
}

#[test]
fn legacy_task_dir_carries_its_generation_counter() {
    let dir = Path::new(FIXTURES).join("legacy-task");
    let session = parse_file(&dir).expect("parse legacy fixture dir");
    assert_eq!(session.metrics.source_tool, "cline");
    assert_eq!(session.metrics.model_used, "claude-sonnet-4-5");
    assert!(session.metrics.events_total >= 3);
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("cline_generation:task-dir"),
        Some(&(session.metrics.events_total)),
        "one counter per legacy message, for the census"
    );
    // Text-only corpus: tokens are estimated, never fabricated usage.
    assert_eq!(session.metrics.provenance.tokens, "estimated_from_text");
}

#[test]
fn transcript_body_is_not_a_session_file() {
    // The body is the session unit's DIR, never a standalone file —
    // and passing the body path directly still parses the parent dir.
    let body = Path::new(FIXTURES)
        .join("sess-v")
        .join("sess-v.messages.json");
    let session = parse_file(&body).expect("body path routes through parent dir");
    assert_eq!(session.metrics.events_total, 3);
    // The body itself is never returned as a session file — the walk
    // must yield its directory instead (verified end-to-end below).
    assert!(!body.is_dir());
}

#[test]
fn discovery_admits_dirs_not_bodies_under_standalone_homes() {
    let home = std::env::temp_dir().join(format!("at-cline-sessv-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let root = write_cline_home(&home);
    let home = home.canonicalize().expect("canonical home");
    let root = root.canonicalize().expect("canonical root");
    with_env(
        &[
            ("HOME", Some(home.as_os_str().to_os_string())),
            (
                "XDG_CONFIG_HOME",
                Some(home.join(".config").into_os_string()),
            ),
            (
                "XDG_DATA_HOME",
                Some(home.join(".local").join("share").into_os_string()),
            ),
            (
                "AGENTTRACE_SESSION_CACHE_DIR",
                Some(home.join("cache").into_os_string()),
            ),
            ("CLINE_HOME", None),
        ],
        || {
            // The registry names the standalone home beside the VSCode
            // root, and the walk returns DIRS for both generations.
            let names: Vec<String> = known_session_dirs()
                .iter()
                .map(|dir| dir.name.clone())
                .collect();
            assert!(
                names.contains(&"Cline XDG".to_string()),
                "standalone XDG home registered: {names:?}"
            );
            let found = collect_session_files(&root);
            let paths: Vec<String> = found
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect();
            assert_eq!(found.len(), 3, "three session dirs, no bodies: {paths:?}");
            for path in &found {
                assert!(path.is_dir(), "session unit is a dir: {}", path.display());
            }
            // find_session_files on the sess-v dir itself round-trips.
            let sess_a = root.join("data").join("sessions").join("sess-a");
            assert_eq!(find_session_files(Some(&sess_a)), vec![sess_a.clone()]);
            // And the legacy task dir still parses from the walk.
            let legacy = root.join("tasks").join("legacy-task");
            let session = parse_file(&legacy).expect("legacy dir parses");
            assert!(session
                .metrics
                .disclosure_counters
                .contains_key("cline_generation:task-dir"));
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn cline_home_env_override_registers_roots() {
    let home = std::env::temp_dir().join(format!("at-cline-env-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let cli_root = home.join("cli-cline");
    copy_fixture(
        &Path::new(FIXTURES)
            .join("sess-v")
            .join("sess-v.messages.json"),
        &cli_root
            .join("sessions")
            .join("sess-e")
            .join("sess-e.messages.json"),
    );
    let other_root = home.join("other-cline");
    copy_fixture(
        &Path::new(FIXTURES)
            .join("sess-v")
            .join("sess-v.messages.json"),
        &other_root.join("sess-f").join("sess-f.messages.json"),
    );
    let home = home.canonicalize().expect("canonical home");
    let cli_root = cli_root.canonicalize().expect("canonical cli root");
    let other_root = other_root.canonicalize().expect("canonical other root");
    with_env(
        &[
            ("HOME", Some(home.as_os_str().to_os_string())),
            (
                "XDG_CONFIG_HOME",
                Some(home.join(".config").into_os_string()),
            ),
            (
                "CLINE_HOME",
                Some(format!("{},{}", cli_root.display(), other_root.display()).into()),
            ),
        ],
        || {
            let all = known_session_dirs();
            let dirs: Vec<&agenttrace_core::KnownSessionDir> =
                all.iter().filter(|dir| dir.name == "Cline CLI").collect();
            assert_eq!(
                dirs.len(),
                2,
                "comma-separated override registers both: {dirs:?}"
            );
            assert_eq!(dirs[0].path, cli_root);
            assert_eq!(dirs[1].path, other_root);
            // Default ~/.cline is replaced by the override.
            assert!(!dirs.iter().any(|dir| dir.path == home.join(".cline")));
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn doctor_census_counts_both_generations() {
    let home = std::env::temp_dir().join(format!("at-cline-doctor-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    write_cline_home(&home);
    let home = home.canonicalize().expect("canonical home");
    with_env(
        &[
            ("HOME", Some(home.as_os_str().to_os_string())),
            (
                "XDG_CONFIG_HOME",
                Some(home.join(".config").into_os_string()),
            ),
            (
                "XDG_DATA_HOME",
                Some(home.join(".local").join("share").into_os_string()),
            ),
            (
                "AGENTTRACE_SESSION_CACHE_DIR",
                Some(home.join("cache").into_os_string()),
            ),
            ("CLINE_HOME", None),
        ],
        || {
            // The doctor runs its own discovery over the temp home;
            // the census must carry both storage generations (and the
            // corpus report must have parsed all three sessions).
            let report = build_doctor_report(None, false);
            assert_eq!(
                report.disclosures.get("cline_generation:sess-v"),
                Some(&6),
                "two sess-v sessions x three messages each"
            );
            assert_eq!(
                report
                    .disclosures
                    .get("cline_generation:task-dir")
                    .copied()
                    .unwrap_or(0),
                legacy_fixture_event_count(),
                "legacy generation visible beside the new one"
            );
            assert_eq!(
                report.disclosures.get("cline_recorded_cost_deferred"),
                Some(&2)
            );
        },
    );
    let _ = fs::remove_dir_all(&home);
}

/// Event count of the on-disk legacy fixture (single source of truth
/// for the doctor census expectation).
fn legacy_fixture_event_count() -> usize {
    parse_file(&Path::new(FIXTURES).join("legacy-task"))
        .expect("parse legacy fixture")
        .metrics
        .events_total
}

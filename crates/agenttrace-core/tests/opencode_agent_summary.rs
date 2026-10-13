//! opencode `session.agent` + `summary_*` consumption contract
//! (rm-991, cycle 2 of run b5affead).
//!
//! The live opencode session DB (this host, 2026-10-12: 302/310 rows
//! carry `agent`, 297/310 carry `summary_*`) records WHO ran a
//! session (`agent`) and the writer-reported diff summary
//! (`summary_additions` / `summary_deletions` / `summary_files`).
//! The loader's column-presence SELECT previously read NEITHER —
//! `agent` never surfaced anywhere (the by_agent spend family keyed
//! on the lane's source tool, so every opencode session sat in one
//! `opencode_db` bucket) and the reported diff numbers were invisible
//! while `files` provenance claimed "unavailable".
//!
//! This contract pins both arms against two fixture databases:
//! - `opencode-agent-summary/opencode.db` — upstream schema WITH the
//!   columns (2 named roots, 1 named child with NULL summary, 1
//!   unnamed root);
//! - `range-overnight/opencode.db` — the older schema WITHOUT them
//!   (the lenient column-probe arm: loads clean, stays empty).

use agenttrace_core::{compute_overview, load_sqlite_backed_sessions_reported};
use std::path::{Path, PathBuf};

fn temp_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm991-contract-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir_all(&home).expect("temp home");
    home
}

/// Scoped env override (HOME + XDG + session-cache dir) so the sqlite
/// discovery lanes read only the fixture home and never touch the
/// ambient session cache. Mirrors range_admission_sqlite.rs's
/// `with_home`, including the guard mutex that serializes
/// env-swapping tests and the catch_unwind so a resumed panic never
/// unwinds through a held lock.
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

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    match LOCK.get_or_init(|| std::sync::Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn plant(home: &Path, fixture: &str) {
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).expect("mkdir opencode home");
    std::fs::copy(
        format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), fixture),
        &target,
    )
    .expect("copy fixture db");
}

fn metrics_for<'a>(
    sessions: &'a [agenttrace_core::Session],
    key: &str,
) -> &'a agenttrace_core::Metrics {
    match sessions
        .iter()
        .find(|session| session.metrics.session_key == key)
    {
        Some(session) => &session.metrics,
        None => {
            let mut keys: Vec<String> = sessions
                .iter()
                .map(|session| session.metrics.session_key.clone())
                .collect();
            keys.sort();
            panic!("session {key} must load; got keys {keys:?}");
        }
    }
}

/// The main arm: a db carrying `agent` + `summary_*` surfaces both.
#[test]
fn agent_names_and_summary_diffs_surface_from_the_session_table() {
    let home = temp_home("full");
    plant(&home, "opencode-agent-summary/opencode.db");
    let sessions = with_home(&home, || {
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        assert!(
            report.unreadable.is_empty(),
            "fixture database must be readable: {:?}",
            report.unreadable
        );
        sessions
    });
    std::fs::remove_dir_all(&home).ok();
    assert_eq!(sessions.len(), 4, "four fixture sessions must load");

    let build = metrics_for(&sessions, "ses-build");
    assert_eq!(build.agent_name, "build");
    assert_eq!(build.summary_additions, Some(120));
    assert_eq!(build.summary_deletions, Some(8));
    assert_eq!(build.summary_files, Some(4));
    assert_eq!(
        build.provenance.files, "reported_by_agent",
        "a writer-reported summary_files upgrades the files provenance"
    );

    // Named child with NULL summary columns: named, but no diff
    // numbers — each column is read independently (lenient).
    let scout = metrics_for(&sessions, "ses-scout");
    assert_eq!(scout.agent_name, "repo-scout");
    assert_eq!(scout.summary_additions, None);
    assert_eq!(scout.summary_files, None);

    // Unnamed row: agent_name empty, so by_agent falls back to the
    // source-tool bucket — the pre-rm-991 behavior.
    let unnamed = metrics_for(&sessions, "ses-unnamed");
    assert!(unnamed.agent_name.is_empty());
    assert_eq!(unnamed.summary_files, None);
}

#[test]
fn by_agent_splits_on_the_upstream_agent_column() {
    let home = temp_home("byagent");
    plant(&home, "opencode-agent-summary/opencode.db");
    let sessions = with_home(&home, || {
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        assert!(report.unreadable.is_empty());
        sessions
    });
    std::fs::remove_dir_all(&home).ok();

    let overview = compute_overview(&sessions);
    let mut agents: Vec<(String, usize)> = overview
        .by_agent
        .iter()
        .map(|(name, group)| (name.clone(), group.sessions))
        .collect();
    agents.sort();
    assert_eq!(
        agents,
        vec![
            ("build".to_string(), 1),
            ("opencode_db".to_string(), 1),
            ("repo-scout".to_string(), 1),
            ("worker".to_string(), 1),
        ],
        "named agents get their own bucket; only the unnamed row falls back to the source-tool bucket"
    );
}

/// The lenient arm: an older db WITHOUT the columns loads clean and
/// surfaces nothing new (agent_name empty, no summary numbers, files
/// provenance stays "unavailable").
#[test]
fn older_schema_without_the_columns_stays_empty_and_readable() {
    let home = temp_home("legacy");
    plant(&home, "range-overnight/opencode.db");
    let sessions = with_home(&home, || {
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        assert!(
            report.unreadable.is_empty(),
            "the legacy fixture database is readable: {:?}",
            report.unreadable
        );
        sessions
    });
    std::fs::remove_dir_all(&home).ok();
    assert!(
        !sessions.is_empty(),
        "the legacy fixture loads its sessions"
    );
    for session in &sessions {
        assert!(
            session.metrics.agent_name.is_empty(),
            "{} must carry no agent_name against a schema without the column",
            session.metrics.session_key
        );
        assert!(
            session.metrics.summary_additions.is_none()
                && session.metrics.summary_deletions.is_none()
                && session.metrics.summary_files.is_none(),
            "{} must carry no summary numbers against a schema without the columns",
            session.metrics.session_key
        );
        assert_ne!(
            session.metrics.provenance.files, "reported_by_agent",
            "files provenance cannot claim a writer report that does not exist"
        );
    }
    let overview = compute_overview(&sessions);
    assert!(
        overview.by_agent.contains_key("opencode_db"),
        "legacy schema keeps the source-tool bucket: {:?}",
        overview.by_agent.keys().collect::<Vec<_>>()
    );
}

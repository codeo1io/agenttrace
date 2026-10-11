//! rm-1031: the Claude Science (beta) corpus lane — per-frame token
//! usage from the desktop app's `metadata.db` (and the switch-proxy
//! daemon's per-org `operon-cli.db`), on the schema documented by
//! ccusage PR #1703 (closed unmerged 2026-09-14 after full review
//! engagement; agenttrace implements the same SELECT-only read on its
//! own evidence). Databases are synthesized at test time (rm-542:
//! sqlite fixtures stay textual by being generated, not committed).
//!
//! Covered:
//! - frames roll up per `COALESCE(root_frame_id, id)` (sub-agent
//!   frames join their root's session);
//! - usage classes map to the four token counters; timestamps are
//!   epoch milliseconds; projects provide display names;
//! - recorded `total_cost` is deferred to the rm-253 family and
//!   disclosed, never estimated-as-recorded;
//! - the org daemon database joins the same corpus;
//! - the rm-947 env override replaces the default root set;
//! - column-gated recognition: a foreign database yields zero rows
//!   quietly; a drifted `frames` table is disclosed as dropped rows;
//! - the doctor's directory census lists the lane beside hermes/opencode;
//! - the file walk stays out of the SQLite-backed homes.

use agenttrace_core::{
    build_doctor_report, known_session_dirs, load_sqlite_backed_sessions_reported,
    skip_sqlite_backed_file_dir, Session,
};
use rusqlite::OpenFlags;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

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

/// The #1703 schema, verbatim shape:
/// `frames(id, root_frame_id, model, project_id, input_tokens,
/// output_tokens, cache_read_tokens, cache_write_tokens, total_cost,
/// updated_at)` with epoch-millisecond `updated_at`, plus a
/// `projects(id, name)` table.
fn create_metadata_db(path: &Path) -> rusqlite::Connection {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create db dir");
    }
    let db = rusqlite::Connection::open(path).expect("create metadata.db");
    db.execute_batch(
        "create table projects (id text primary key, name text);
         create table frames (
             id text primary key,
             root_frame_id text,
             model text,
             project_id text,
             input_tokens integer,
             output_tokens integer,
             cache_read_tokens integer,
             cache_write_tokens integer,
             total_cost real,
             updated_at integer
         );",
    )
    .expect("create schema");
    db
}

/// A test-data row writer; the frame schema itself is wide, and the
/// test corpus reads clearest with one call per frame.
#[allow(clippy::too_many_arguments)]
fn insert_frame(
    db: &rusqlite::Connection,
    id: &str,
    root: Option<&str>,
    model: &str,
    project: Option<&str>,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_write: i64,
    cost: Option<f64>,
    updated_at_ms: i64,
) {
    db.execute(
        "insert into frames (id, root_frame_id, model, project_id, input_tokens, output_tokens, \
         cache_read_tokens, cache_write_tokens, total_cost, updated_at) \
         values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            id,
            root,
            model,
            project,
            input,
            output,
            cache_read,
            cache_write,
            cost,
            updated_at_ms
        ],
    )
    .expect("insert frame");
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as i64
}

/// A temp home with a populated `.claude-science/metadata.db`:
/// - root frame `root-1` (2 frames, one sub-agent frame rolling in,
///   recorded cost on the first);
/// - standalone root frame `root-2` on another model, another project.
fn write_science_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("at-science-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let db = create_metadata_db(&home.join(".claude-science").join("metadata.db"));
    db.execute(
        "insert into projects (id, name) values ('p1', 'atlas'), ('p2', 'bokeh')",
        [],
    )
    .expect("insert projects");
    let t = now_ms();
    insert_frame(
        &db,
        "root-1",
        None,
        "claude-sonnet-4-5",
        Some("p1"),
        1_000,
        200,
        400,
        100,
        Some(0.021),
        t - 60_000,
    );
    // Sub-agent frame: rolls into root-1's session.
    insert_frame(
        &db,
        "sub-1",
        Some("root-1"),
        "claude-sonnet-4-5",
        Some("p1"),
        500,
        50,
        250,
        0,
        None,
        t - 30_000,
    );
    insert_frame(
        &db,
        "root-2",
        None,
        "claude-opus-4-6",
        Some("p2"),
        300,
        30,
        0,
        20,
        None,
        t,
    );
    home.canonicalize().expect("canonical home")
}

fn science_session<'a>(sessions: &'a [Session], key: &str) -> &'a Session {
    let keys: Vec<String> = sessions
        .iter()
        .map(|session| session.metrics.session_key.clone())
        .collect();
    sessions
        .iter()
        .find(|session| session.metrics.session_key == key)
        .unwrap_or_else(|| panic!("session {key} in corpus: {keys:?}"))
}

#[test]
fn frames_roll_up_per_root_with_token_classes_and_project_names() {
    let home = write_science_home("rollup");
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            let (sessions, report) = load_sqlite_backed_sessions_reported(None);
            assert!(report.unreadable.is_empty(), "{:?}", report.unreadable);
            assert!(report.dropped_rows.is_empty(), "{:?}", report.dropped_rows);
            assert_eq!(sessions.len(), 2, "one session per root frame id");

            let root1 = science_session(&sessions, "root-1");
            let metrics = &root1.metrics;
            assert_eq!(metrics.source_tool, "claude_science_db");
            assert_eq!(root1.name, "atlas", "project name is the display name");
            assert_eq!(metrics.model_used, "claude-sonnet-4-5");
            assert_eq!(metrics.tokens_input, 1_500);
            assert_eq!(metrics.tokens_output, 250);
            assert_eq!(metrics.tokens_cache_r, 650);
            assert_eq!(metrics.tokens_cache_w, 100);
            assert_eq!(metrics.events_total, 2, "one counted event per frame");
            assert!(metrics.cost_estimated > 0.0);
            assert_eq!(metrics.provenance.tokens, "reported_by_agent");
            assert_eq!(metrics.provenance.cost, "calculated_from_tokens");
            // The span between first and last frame is a real duration.
            assert!(
                (metrics.duration_sec - 30.0).abs() < 1.0,
                "60s-30s before now = 30s: {}",
                metrics.duration_sec
            );
            // Recorded cost deferred (rm-253 family), disclosed per frame.
            assert_eq!(
                metrics
                    .disclosure_counters
                    .get("claude_science_recorded_cost_deferred"),
                Some(&1)
            );

            let root2 = science_session(&sessions, "root-2");
            assert_eq!(root2.name, "bokeh");
            assert_eq!(root2.metrics.model_used, "claude-opus-4-6");
            assert_eq!(root2.metrics.tokens_input, 300);
            assert!(!root2
                .metrics
                .disclosure_counters
                .contains_key("claude_science_recorded_cost_deferred"));
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn multi_model_root_folds_to_multiple() {
    let home = std::env::temp_dir().join(format!("at-science-multi-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let db = create_metadata_db(&home.join(".claude-science").join("metadata.db"));
    let t = now_ms();
    insert_frame(
        &db,
        "root-m",
        None,
        "claude-sonnet-4-5",
        None,
        100,
        10,
        0,
        0,
        None,
        t,
    );
    insert_frame(
        &db,
        "sub-m",
        Some("root-m"),
        "claude-opus-4-6",
        None,
        100,
        10,
        0,
        0,
        None,
        t + 1_000,
    );
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            let (sessions, _) = load_sqlite_backed_sessions_reported(None);
            let root = science_session(&sessions, "root-m");
            assert_eq!(root.metrics.model_used, "multiple");
            assert_eq!(root.metrics.tokens_input, 200);
            assert_eq!(root.metrics.events_total, 2);
            // No project table rows: the session id names the session.
            assert_eq!(root.name, "root-m");
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn org_daemon_database_joins_the_corpus() {
    let home = std::env::temp_dir().join(format!("at-science-org-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    // Primary root WITHOUT a metadata.db: the org db alone must load.
    let db = create_metadata_db(
        &home
            .join(".claude-science")
            .join("cs-switch-proxy")
            .join("orgs")
            .join("acme")
            .join("operon-cli.db"),
    );
    let t = now_ms();
    insert_frame(
        &db,
        "org-root",
        None,
        "claude-sonnet-4-5",
        None,
        700,
        70,
        0,
        0,
        None,
        t,
    );
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            let (sessions, report) = load_sqlite_backed_sessions_reported(None);
            assert!(report.unreadable.is_empty(), "{:?}", report.unreadable);
            let org = science_session(&sessions, "org-root");
            assert_eq!(org.metrics.source_tool, "claude_science_db");
            assert_eq!(org.metrics.tokens_input, 700);
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn env_override_replaces_the_default_roots() {
    let home = write_science_home("override-home");
    let other = std::env::temp_dir().join(format!("at-science-override-db-{}", std::process::id()));
    let _ = fs::remove_dir_all(&other);
    let explicit_db = other.join("explicit").join("metadata.db");
    let db = create_metadata_db(&explicit_db);
    let t = now_ms();
    insert_frame(
        &db,
        "explicit-root",
        None,
        "claude-sonnet-4-5",
        None,
        42,
        4,
        0,
        0,
        None,
        t,
    );
    let other = other.canonicalize().expect("canonical explicit root");
    let explicit_db = explicit_db.canonicalize().expect("canonical explicit db");
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
            (
                "CLAUDE_SCIENCE_DB",
                Some(explicit_db.as_os_str().to_os_string()),
            ),
        ],
        || {
            let (sessions, _) = load_sqlite_backed_sessions_reported(None);
            // Only the explicit root's session: the default home's
            // root-1/root-2 corpus is replaced, not unioned.
            assert_eq!(sessions.len(), 1, "{sessions:?}");
            assert_eq!(sessions[0].metrics.session_key, "explicit-root");
            assert_eq!(sessions[0].metrics.tokens_input, 42);
        },
    );
    let _ = fs::remove_dir_all(&other);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn foreign_database_yields_nothing_and_drifted_frames_is_disclosed() {
    let home = std::env::temp_dir().join(format!("at-science-foreign-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    // A foreign sqlite database (no frames table): legitimately zero.
    let science_dir = home.join(".claude-science");
    fs::create_dir_all(&science_dir).expect("create science dir");
    let foreign =
        rusqlite::Connection::open(science_dir.join("metadata.db")).expect("create foreign db");
    foreign
        .execute_batch("create table unrelated (x);")
        .expect("foreign schema");
    drop(foreign);
    // A drifted frames table (loader columns missing): disclosed drop.
    let drift_dir = home.join(".config").join("Claude Science");
    fs::create_dir_all(&drift_dir).expect("create drift dir");
    let drifted =
        rusqlite::Connection::open(drift_dir.join("metadata.db")).expect("create drifted db");
    drifted
        .execute_batch("create table frames (id text primary key, whatever integer);")
        .expect("drifted schema");
    drop(drifted);
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            let (sessions, report) = load_sqlite_backed_sessions_reported(None);
            assert!(sessions.is_empty(), "no frames, no sessions");
            assert!(report.unreadable.is_empty(), "{:?}", report.unreadable);
            assert_eq!(
                report.dropped_rows.len(),
                1,
                "drifted frames table disclosed, foreign db not: {report:?}"
            );
            assert_eq!(report.dropped_rows[0].source, "claude_science_db");
            assert!(
                report.dropped_rows[0].sample.contains("schema drift"),
                "{}",
                report.dropped_rows[0].sample
            );
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn legacy_column_set_degrades_gracefully() {
    let home = std::env::temp_dir().join(format!("at-science-legacy-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let science_dir = home.join(".claude-science");
    fs::create_dir_all(&science_dir).expect("create science dir");
    let db_path = science_dir.join("metadata.db");
    let db = rusqlite::Connection::open(&db_path).expect("create legacy db");
    // Pre-cache-tokens schema: no cache_read/cache_write, no projects
    // table, no root_frame_id (every frame its own session).
    db.execute_batch(
        "create table frames (
             id text primary key,
             model text,
             input_tokens integer,
             output_tokens integer,
             total_cost real,
             updated_at integer
         );",
    )
    .expect("legacy schema");
    let t = now_ms();
    db.execute(
        "insert into frames values ('legacy-1', 'claude-sonnet-4-5', 111, 11, null, ?1)",
        [t],
    )
    .expect("insert");
    drop(db);
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            let (sessions, report) = load_sqlite_backed_sessions_reported(None);
            assert!(report.unreadable.is_empty(), "{:?}", report.unreadable);
            assert!(report.dropped_rows.is_empty(), "{:?}", report.dropped_rows);
            assert_eq!(sessions.len(), 1);
            let metrics = &sessions[0].metrics;
            assert_eq!(metrics.session_key, "legacy-1");
            assert_eq!(metrics.tokens_input, 111);
            assert_eq!(metrics.tokens_cache_r, 0);
            assert_eq!(metrics.tokens_cache_w, 0);
            assert_eq!(
                sessions[0].name, "legacy-1",
                "no projects table: id names it"
            );
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn read_only_never_writes_or_creates() {
    let home = write_science_home("readonly");
    let db_path = home.join(".claude-science").join("metadata.db");
    let before = fs::metadata(&db_path).expect("db exists");
    let bytes_before = before.len();
    let mtime_before = before.modified().expect("mtime");
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            let (sessions, _) = load_sqlite_backed_sessions_reported(None);
            assert_eq!(sessions.len(), 2);
        },
    );
    // No journal/wal side files, no size or mtime change: the lane is
    // SELECT-only over the operator's own app database.
    for side in ["metadata.db-journal", "metadata.db-wal", "metadata.db-shm"] {
        assert!(!db_path.with_file_name(side).exists(), "{side} created");
    }
    let after = fs::metadata(&db_path).expect("db still exists");
    assert_eq!(after.len(), bytes_before);
    // mtime granularity on coarse filesystems can hide a rewrite, but
    // a SELECT-only connection never opens a journal, so size + no
    // side files is the strong proof; mtime equality where supported.
    assert!(after
        .modified()
        .map(|time| time == mtime_before)
        .unwrap_or(true));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn doctor_census_lists_the_claude_science_lane() {
    let home = write_science_home("doctor");
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            // The lane's home is registered like any transcript home…
            let names: Vec<String> = known_session_dirs()
                .iter()
                .map(|dir| dir.name.clone())
                .collect();
            assert!(names.contains(&"Claude Science".to_string()), "{names:?}");
            // …and the doctor's directory census names the database.
            let rendered = agenttrace_core::render_doctor_report(None, false, "text")
                .expect("render doctor report");
            assert!(
                rendered.contains("Claude Science (DB)"),
                "census row present:\n{rendered}"
            );
            // …and its disclosure folds into the census channels.
            let report = build_doctor_report(None, false);
            assert_eq!(
                report
                    .disclosures
                    .get("claude_science_recorded_cost_deferred"),
                Some(&1)
            );
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn file_walk_skips_the_sqlite_backed_home() {
    let home = write_science_home("walk");
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            assert!(skip_sqlite_backed_file_dir(&home.join(".claude-science")));
            // The whole lane is SQLite-backed territory (the opencode
            // storage precedent): every claude-science home is skipped
            // once the lane has a database, and unrelated dirs are not.
            let share_root = home.join(".local").join("share").join("claude-science");
            assert!(skip_sqlite_backed_file_dir(&share_root));
            assert!(!skip_sqlite_backed_file_dir(&home.join("notes")));
        },
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn connection_is_opened_read_only() {
    // Defense-in-depth for the "never writes" claim: a file with no
    // write permission still loads (a writer would fail to create its
    // journal). POSIX chmod is the cheap harness; skipped elsewhere.
    let home = std::env::temp_dir().join(format!("at-science-rdonly-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let db = create_metadata_db(&home.join(".claude-science").join("metadata.db"));
    let t = now_ms();
    insert_frame(
        &db,
        "rd-root",
        None,
        "claude-sonnet-4-5",
        None,
        10,
        1,
        0,
        0,
        None,
        t,
    );
    drop(db);
    let db_path = home.join(".claude-science").join("metadata.db");
    let mut perms = fs::metadata(&db_path).expect("meta").permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o444);
    fs::set_permissions(&db_path, perms).expect("read-only perms");
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
            ("CLAUDE_SCIENCE_DB", None),
        ],
        || {
            let (sessions, report) = load_sqlite_backed_sessions_reported(None);
            assert!(report.unreadable.is_empty(), "{:?}", report.unreadable);
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].metrics.session_key, "rd-root");
        },
    );
    let _ = fs::remove_dir_all(&home);
}

/// Silence the unused-import lint for OpenFlags (kept to document the
/// lane's own open discipline mirrors the test harness).
#[allow(dead_code)]
fn _flags_documentation() -> OpenFlags {
    OpenFlags::SQLITE_OPEN_READ_ONLY
}

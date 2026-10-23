//! Zed agent-panel threads.db lane contract (rm-2370, run fcbdf5ba cycle 5).
//!
//! Acceptance (from roadmap band bf7894ca):
//! (1) discovery admits threads.db from the documented per-OS paths;
//! (2) per-thread usage = Σ request_token_usage rows PLUS one disclosed
//!     remainder entry reconciling to cumulative_token_usage — never
//!     silently under- or over-counting;
//! (3) unknown models price via the standard engine with disclosure;
//! (4) red-first: this test failed before the lane existed.
//!
//! Fixture strategy: build a real threads.db in-test with rusqlite from the
//! DOCUMENTED schema (cross-checked against Zed crates/agent/src/db.rs and
//! the codeburn provider doc mirrored at
//! .cache/at-research-74ee8947/cb-zed.md).

#![test_serialized::serialized]
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn pin_home(root: &std::path::Path) {
    std::env::set_var("HOME", root);
    std::env::set_var("USERPROFILE", root);
    std::env::remove_var("ZED_THREADS_DB"); // reserved future override
}

fn build_zed_db(dir: &std::path::Path) -> PathBuf {
    use rusqlite::Connection;
    let db = dir.join("threads.db");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE threads (
            rowid INTEGER PRIMARY KEY,
            updated_at REAL,
            cumulative_token_usage TEXT,
            request_token_usage TEXT,
            serialized_language_model TEXT
        );
        "#,
    )
    .unwrap();
    // Thread 1: two request entries summing BELOW cumulative -> remainder
    // must reconcile (the ~3x under-coverage trap, shape from cb-zed.md).
    conn.execute(
        "INSERT INTO threads (rowid, updated_at, cumulative_token_usage, request_token_usage, serialized_language_model) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            1i64,
            1762900000.0f64,
            r#"{"input_tokens": 1000, "output_tokens": 200, "cache_read_tokens": 50, "cache_creation_tokens": 10}"#,
            r#"{"msg-a": {"input_tokens": 300, "output_tokens": 100, "cache_read_tokens": 20, "cache_creation_tokens": 10}, "msg-b": {"input_tokens": 200, "output_tokens": 50, "cache_read_tokens": 30, "cache_creation_tokens": 0}}"#,
            r#"{"name": "claude-sonnet-4-5"}"#
        ],
    )
    .unwrap();
    // Thread 2: NO request entries at all — cumulative must still land.
    conn.execute(
        "INSERT INTO threads (rowid, updated_at, cumulative_token_usage, request_token_usage, serialized_language_model) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            2i64,
            1762900600.0f64,
            r#"{"input_tokens": 500, "output_tokens": 50, "cache_read_tokens": 0, "cache_creation_tokens": 0}"#,
            r#"{}"#,
            r#"{"name": "claude-sonnet-4-5"}"#
        ],
    )
    .unwrap();
    drop(conn);
    db
}

fn events_total_usage(events: &[agenttrace_core::Event], key: &str) -> i64 {
    events.iter().map(|e| e.usage.get(key).copied().unwrap_or(0)).sum()
}

#[test]
fn zed_threads_db_is_admitted_and_reconciles_to_cumulative() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = tempfile::TempDir::new().unwrap();
    let (zed_dir, root) = {
        let h = home.path().to_path_buf();
        let z = h.join(".local/share/zed/threads");
        (z.clone(), h)
    };
    std::fs::create_dir_all(&zed_dir).unwrap();
    let db = build_zed_db(&zed_dir);
    pin_home(root);

    // (1) discovery admits threads.db from the documented linux path
    let dirs = agenttrace_core::known_session_dirs();
    assert!(
        dirs.iter().any(|d| d.name == "zed"),
        "zed known-session dir must be registered: {:?}",
        dirs.iter().map(|d| d.name.clone()).collect::<Vec<_>>()
    );

    // (2) parse: request entries + disclosed remainder == cumulative, per thread
    let raw = std::fs::read(&db).unwrap();
    assert!(raw.starts_with(b"SQLite format 3\0"));
    let session = agenttrace_core::parse_raw_bytes("threads.db", "zed", &raw);
    let events = session.events.as_ref().expect("zed session must carry events");
    assert!(events.len() >= 3, "expected >=3 attributed calls, got {}", events.len());
    assert_eq!(events_total_usage(events, "input_tokens"), 1500);
    assert_eq!(events_total_usage(events, "output_tokens"), 250);
    assert_eq!(events_total_usage(events, "cache_read_tokens"), 50);
    assert_eq!(events_total_usage(events, "cache_creation_tokens"), 10);
    // remainder entries are DISCLOSED, not silent
    assert!(
        session
            .metrics
            .disclosure_counters
            .keys()
            .any(|k| k.contains("zed") && k.contains("remainder")),
        "zed remainder reconciliation must be a named disclosure, got {:?}",
        session.metrics.disclosure_counters.keys().collect::<Vec<_>>()
    );
}

#[test]
fn zed_threads_db_without_threads_table_is_not_zed() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = tempfile::TempDir::new().unwrap();
    let db = {
        use rusqlite::Connection;
        let p = home.path().join("other.db");
        let c = Connection::open(&p).unwrap();
        c.execute_batch("CREATE TABLE t(x);").unwrap();
        drop(c);
        p
    };
    let raw = std::fs::read(&db).unwrap();
    // Not our shape: the parser must NOT claim it as a zed session.
    let session = agenttrace_core::parse_raw_bytes("other.db", "zed", &raw);
    assert!(session.events.as_ref().map(|e| e.is_empty()).unwrap_or(true));
}

#[test]
fn zed_thread_below_cumulative_never_reports_negative() {
    // Vendor counter reset: cumulative BELOW summed requests -> zero
    // remainder, never negative tokens.
    let _g = ENV_LOCK.lock().unwrap();
    let home = tempfile::TempDir::new().unwrap();
    let dir = home.path().join("z");
    std::fs::create_dir_all(&dir).unwrap();
    use rusqlite::Connection;
    let db = dir.join("threads.db");
    let c = Connection::open(&db).unwrap();
    c.execute_batch("CREATE TABLE threads (rowid INTEGER PRIMARY KEY, updated_at REAL, cumulative_token_usage TEXT, request_token_usage TEXT, serialized_language_model TEXT);").unwrap();
    c.execute(
        "INSERT INTO threads VALUES (1, 1762900000.0, ?1, ?2, ?3)",
        rusqlite::params![
            r#"{"input_tokens": 100, "output_tokens": 10, "cache_read_tokens": 0, "cache_creation_tokens": 0}"#,
            r#"{"a": {"input_tokens": 400, "output_tokens": 40, "cache_read_tokens": 0, "cache_creation_tokens": 0}}"#,
            r#"{"name": "claude-sonnet-4-5"}"#
        ],
    )
    .unwrap();
    drop(c);
    let raw = std::fs::read(&db).unwrap();
    let session = agenttrace_core::parse_raw_bytes("threads.db", "zed", &raw);
    let events = session.events.as_ref().unwrap();
    assert_eq!(events_total_usage(events, "input_tokens"), 400);
    for e in events {
        for v in e.usage.values() {
            assert!(*v >= 0, "negative token count leaked: {:?}", e.usage);
        }
    }
}

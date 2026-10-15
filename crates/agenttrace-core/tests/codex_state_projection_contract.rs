//! rm-1504 contract: the Codex state-store family projection is a
//! strictly read-only, disclosed census.
//!
//! Fixtures are built at test time from the schema pinned against the
//! live codex state family (host-observed 2026-10-14, codex-rs `state`
//! sqlite.rs @ e35a074d pinning the `state_5`/`goals_1` filenames),
//! so the contract stays reviewable and no binary database ships with
//! the tree. Arms bind:
//! 1. an absent family is a zero-delta census (nothing found,
//!    nothing rendered, nothing serialized);
//! 2. the pinned fixture projects per-thread `tokens_used` totals,
//!    source counts, sanitized+truncated sample titles, and per-goal
//!    budget-vs-used with an over-budget count;
//! 3. schema drift is disclosed both ways (missing core tables stop
//!    that class's projection; extra tables are informational —
//!    upstream migrations such as `0034_drop_thread_goals` and newer
//!    `state` migrations evolve this family);
//! 4. a discovered-but-unreadable database is named with its error
//!    (found + failed), never a silent zero;
//! 5. the census is read-only: main-file bytes are unchanged
//!    everywhere; rollback-mode members get no sidecars at all;
//!    WAL-mode members may carry sqlite's read-coordination
//!    `-shm`/empty `-wal` (pinned), and a live writer's non-empty
//!    `-wal` is never checkpointed or truncated;
//! 6. rendering is pure and only speaks when the family exists;
//! 7. the doctor `--demo` and `--dir` lanes skip the census (their
//!    output stays byte-identical to pre-rm-1504 behavior).

use agenttrace_core::{codex_state_census_at, codex_state_render, render_doctor_report};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Serialize env-touching doctor arms (the `--demo` render reads the
/// environment; the census itself never does — it takes a path).
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn fixture_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("at-rm1504-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create fixture root");
    root
}

/// The `threads` DDL pinned from the live `state_5.sqlite` (column
/// names exactly as observed; the projection reads
/// `tokens_used`/`source`/`title`, the rest pins the census shape).
const THREADS_DDL: &str = r#"
CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    created_at_ms INTEGER NOT NULL DEFAULT 0,
    updated_at_ms INTEGER NOT NULL DEFAULT 0,
    archived BOOLEAN NOT NULL DEFAULT 0,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    origin_session_id TEXT,
    rollback_id TEXT,
    noop_count INTEGER NOT NULL DEFAULT 0,
    forward_id TEXT,
    draft_id TEXT,
    mode TEXT,
    reasoning_allowed BOOLEAN,
    auto_applies_enabled BOOLEAN,
    run_state TEXT,
    include_plan_tool BOOLEAN,
    include_apply_tool BOOLEAN,
    include_review_tool BOOLEAN,
    model_reasoning_effort TEXT,
    model_reasoning_summary TEXT,
    model_verbosity TEXT,
    shell_environment_policy TEXT,
    sandbox_policy TEXT,
    approval_policy TEXT,
    model TEXT,
    summary TEXT,
    context_window INTEGER,
    last_user_message TEXT,
    last_agent_message TEXT,
    compacted_message_ids TEXT,
    item_flags TEXT,
    tags TEXT,
    pinned_signals TEXT,
    squash_point TEXT,
    web_search_let_all_domains_through_query_string_check BOOLEAN,
    rules_sources TEXT
);
"#;

/// The `thread_goals` DDL pinned from the live `goals_1.sqlite`.
const THREAD_GOALS_DDL: &str = r#"
CREATE TABLE thread_goals (
    thread_id TEXT NOT NULL,
    goal_id TEXT NOT NULL,
    objective TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT '',
    token_budget INTEGER NOT NULL DEFAULT 0,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    updated_at_ms INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL DEFAULT 0,
    continuation_id TEXT,
    PRIMARY KEY (thread_id, goal_id)
);
"#;

fn build_state_db(path: &Path) {
    let conn = rusqlite::Connection::open(path).expect("create state fixture");
    conn.execute_batch(THREADS_DDL).expect("threads ddl");
    // One hostile title: leading `=` (csv/formula injection), control
    // characters, and 200+ chars — the projection must sanitize and
    // hard-truncate before the doctor ever relays it.
    let hostile_title = format!("=2+5_cmd\x01inj\x7f{}", "A".repeat(200));
    let rows: [(&str, i64, &str); 3] = [
        ("thread-largest", 100, "cli"),
        ("thread-middle", 50, "vscode"),
        ("thread-smallest", 25, "cli"),
    ];
    for (id, tokens, source) in rows {
        conn.execute(
            "INSERT INTO threads (id, tokens_used, source, title) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                id,
                tokens,
                source,
                if id == "thread-largest" {
                    hostile_title.as_str()
                } else {
                    "ordinary title"
                }
            ],
        )
        .expect("insert thread");
    }
}

fn build_goals_db(path: &Path, with_core: bool) {
    let conn = rusqlite::Connection::open(path).expect("create goals fixture");
    if with_core {
        conn.execute_batch(THREAD_GOALS_DDL)
            .expect("thread_goals ddl");
        conn.execute(
            "INSERT INTO thread_goals (thread_id, goal_id, token_budget, tokens_used) VALUES ('t1', 'g1', 1000, 1500)",
            [],
        )
        .expect("insert over-budget goal");
        conn.execute(
            "INSERT INTO thread_goals (thread_id, goal_id, token_budget, tokens_used) VALUES ('t2', 'g2', 500, 100)",
            [],
        )
        .expect("insert in-budget goal");
    } else {
        // A post-0034 goals database: the table was dropped upstream.
        conn.execute_batch("CREATE TABLE continuation_deferrals (id TEXT PRIMARY KEY);")
            .expect("drifted goals ddl");
    }
}

fn md5_of(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("read fixture for md5");
    // Tests avoid external crates: FNV-1a 64 is enough to prove the
    // bytes are untouched (the read-only arm re-reads the same file).
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Rollback-mode fixtures must exit the census exactly as they
/// entered it — no `-wal`/`-shm`/`-journal` files at all. WAL-mode
/// members are held to the read-coordination contract instead (see
/// the two `wal_mode_*` tests).
fn assert_no_sidecars(path: &Path) {
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        assert!(
            !Path::new(&sidecar).exists(),
            "census wrote a sidecar: {sidecar:?}"
        );
    }
}

#[test]
fn absent_family_is_zero_delta() {
    let root = fixture_root("absent");
    let census = codex_state_census_at(&root);
    assert_eq!(census.found, 0);
    assert_eq!(census.failed, 0);
    assert!(census.dbs.is_empty());
    assert!(census.is_empty());
    assert_eq!(codex_state_render(&census), "");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn pinned_fixture_projection_and_readonly() {
    let root = fixture_root("pinned");
    build_state_db(&root.join("state_5.sqlite"));
    build_goals_db(&root.join("goals_1.sqlite"), true);
    let state_path = root.join("state_5.sqlite");
    let goals_path = root.join("goals_1.sqlite");
    let state_before = md5_of(&state_path);
    let goals_before = md5_of(&goals_path);

    let census = codex_state_census_at(&root);
    assert_eq!(census.found, 2, "both fixtures discovered");
    assert_eq!(census.failed, 0);

    let state = census
        .dbs
        .iter()
        .find(|db| db.class == "state")
        .expect("state db censused");
    assert_eq!(state.file_name, "state_5.sqlite");
    assert_eq!(state.version, Some(5));
    assert!(state.missing_tables.is_empty(), "core present");
    assert!(state.extra_tables.is_empty(), "no extras in fixture");
    assert!(state.tables.contains(&"threads".to_string()));
    let threads = state.threads.as_ref().expect("threads projection");
    assert_eq!(threads.threads, 3);
    assert_eq!(threads.tokens_used_total, 175);
    let cli = threads
        .sources
        .iter()
        .find(|s| s.source == "cli")
        .expect("cli counted");
    assert_eq!(cli.count, 2);
    assert_eq!(
        threads.top_threads.len(),
        3,
        "sample leaders bounded by TOP_THREADS_SAMPLE"
    );
    // Top sample carries the hostile title bounded for display: no
    // control bytes and hard-truncated to 60 chars + ellipsis. (A
    // leading `=` survives `sanitize_line_segment` by design — it
    // guards terminal/ANSI and control payloads, not CSV cells; the
    // doctor renders the title quoted, and truncation is the proof
    // the raw 200-char corpus bytes never reach the report.)
    let top = &threads.top_threads[0];
    assert_eq!(top.tokens_used, 100);
    assert_eq!(top.source, "cli");
    assert!(
        !top.title.chars().any(|c| c.is_control()),
        "control chars stripped"
    );
    assert!(top.title.chars().count() <= 61, "title truncated");
    assert!(top.title.ends_with('…'));

    let goals = census
        .dbs
        .iter()
        .find(|db| db.class == "goals")
        .expect("goals db censused");
    assert_eq!(goals.version, Some(1));
    let goal_census = goals.goals.as_ref().expect("goals projection");
    assert_eq!(goal_census.goals, 2);
    assert_eq!(goal_census.token_budget_total, 1500);
    assert_eq!(goal_census.tokens_used_total, 1600);
    assert_eq!(goal_census.over_budget, 1);

    // Read-only proof: bytes unchanged, no sidecars.
    assert_eq!(md5_of(&state_path), state_before, "state fixture mutated");
    assert_eq!(md5_of(&goals_path), goals_before, "goals fixture mutated");
    assert_no_sidecars(&state_path);
    assert_no_sidecars(&goals_path);

    let rendered = codex_state_render(&census);
    assert!(rendered.contains("Codex state stores (rm-1504): 2 found, 0 unreadable"));
    assert!(rendered.contains("state_5.sqlite (state v5)"));
    assert!(rendered.contains("threads 3 · tokens_used 175"));
    assert!(rendered.contains("cli×2"));
    assert!(rendered.contains("goals 2 · budget 1500 · used 1600 · over budget 1"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn schema_drift_is_disclosed_both_ways() {
    let root = fixture_root("drift");
    // A newer state store: extra table (newer migration state) and a
    // missing informational table — core intact, so the projection
    // still runs and the drift is informational.
    {
        let conn = rusqlite::Connection::open(root.join("state_6.sqlite")).unwrap();
        conn.execute_batch(THREADS_DDL).unwrap();
        conn.execute_batch("CREATE TABLE projects (id TEXT PRIMARY KEY);")
            .unwrap();
    }
    // A post-0034 goals store: core table dropped upstream — the
    // projection is skipped and DISCLOSED, never a panic.
    build_goals_db(&root.join("goals_2.sqlite"), false);

    let census = codex_state_census_at(&root);
    assert_eq!(census.found, 2);
    let state = census.dbs.iter().find(|db| db.class == "state").unwrap();
    assert_eq!(state.version, Some(6));
    assert!(state.missing_tables.is_empty(), "core intact");
    assert_eq!(state.extra_tables, vec!["projects".to_string()]);
    assert!(state.threads.is_some(), "projection still runs");
    let goals = census.dbs.iter().find(|db| db.class == "goals").unwrap();
    assert_eq!(goals.missing_tables, vec!["thread_goals".to_string()]);
    assert!(goals.goals.is_none(), "dropped core stops the projection");
    assert_eq!(census.failed, 0, "drift is disclosure, not failure");

    let rendered = codex_state_render(&census);
    assert!(rendered.contains("extra projects"));
    assert!(rendered.contains("missing thread_goals"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn unreadable_database_is_named_not_silent() {
    let root = fixture_root("unreadable");
    std::fs::write(
        root.join("state_5.sqlite"),
        b"this is not a sqlite database at all",
    )
    .expect("garbage fixture");
    let before = md5_of(&root.join("state_5.sqlite"));

    let census = codex_state_census_at(&root);
    assert_eq!(census.found, 1);
    assert_eq!(census.failed, 1);
    let db = &census.dbs[0];
    assert_eq!(db.file_name, "state_5.sqlite");
    let error = db.error.as_ref().expect("error disclosed");
    assert!(!error.is_empty());
    assert!(db.threads.is_none());

    let rendered = codex_state_render(&census);
    assert!(rendered.contains("1 found, 1 unreadable"));
    assert!(rendered.contains("unreadable —"));
    // Even the garbage file is only read.
    assert_eq!(md5_of(&root.join("state_5.sqlite")), before);
    assert_no_sidecars(&root.join("state_5.sqlite"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn unrecognized_sqlite_is_still_disclosed() {
    let root = fixture_root("unrecognized");
    {
        let conn = rusqlite::Connection::open(root.join("checkbook.sqlite")).unwrap();
        conn.execute_batch("CREATE TABLE ledger (id TEXT PRIMARY KEY);")
            .unwrap();
    }
    let census = codex_state_census_at(&root);
    assert_eq!(census.found, 1);
    let db = &census.dbs[0];
    assert_eq!(db.class, "unrecognized");
    assert!(db.error.is_none());
    assert!(db.threads.is_none() && db.goals.is_none());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn wal_mode_member_with_live_writer_reads_through_the_wal() {
    let root = fixture_root("wal-live-writer");
    let path = root.join("state_5.sqlite");
    // A live writer holding a committed, un-checkpointed WAL frame —
    // the state a running codex leaves the store in.
    let writer = rusqlite::Connection::open(&path).expect("open writer");
    writer
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
        .expect("wal pragmas");
    writer.execute_batch(THREADS_DDL).expect("threads ddl");
    writer
        .execute(
            "INSERT INTO threads (id, tokens_used, source, title) VALUES ('wal-live', 7, 'cli', 'committed in wal')",
            [],
        )
        .expect("insert");
    let main_before = md5_of(&path);
    let wal_path = root.join("state_5.sqlite-wal");
    let wal_before = std::fs::metadata(&wal_path).expect("writer's wal").len();
    assert!(
        wal_before > 0,
        "fixture precondition: wal carries the frame"
    );

    let census = codex_state_census_at(&root);
    assert_eq!(census.found, 1);
    assert_eq!(census.failed, 0);
    let threads = census.dbs[0]
        .threads
        .as_ref()
        .expect("read through the wal");
    assert_eq!(threads.threads, 1);
    assert_eq!(threads.tokens_used_total, 7);

    // The census must not absorb, truncate, or checkpoint the
    // writer's wal, and must not touch the main file.
    assert_eq!(
        std::fs::metadata(&wal_path).expect("wal still there").len(),
        wal_before,
        "writer's wal byte-length preserved"
    );
    assert_eq!(md5_of(&path), main_before, "main file untouched");
    drop(writer);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn sidecar_free_wal_member_leaves_only_read_coordination_files() {
    let root = fixture_root("wal-sidecar-free");
    let path = root.join("state_5.sqlite");
    {
        let conn = rusqlite::Connection::open(&path).expect("open fixture writer");
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .expect("wal mode");
        conn.execute_batch(THREADS_DDL).expect("threads ddl");
        conn.execute(
            "INSERT INTO threads (id, tokens_used, source, title) VALUES ('t', 3, 'cli', 'x')",
            [],
        )
        .expect("insert");
    }
    // Clean writer close checkpoints and removes its own wal/shm —
    // the state five of the six live host DBs sit in.
    let wal_path = root.join("state_5.sqlite-wal");
    assert!(!wal_path.exists(), "fixture precondition: sidecar-free");
    let main_before = md5_of(&path);

    let census = codex_state_census_at(&root);
    assert_eq!(census.found, 1);
    assert_eq!(census.failed, 0);
    assert!(census.dbs[0].threads.is_some(), "wal member readable");
    assert_eq!(md5_of(&path), main_before, "main file untouched");
    // Reading a WAL-mode member coordinates through sqlite's
    // WAL-index: an -shm and an EMPTY -wal may remain after the
    // reader closes (a readonly close never cleans them). A
    // non-empty -wal would mean the census wrote frames — forbidden.
    if wal_path.exists() {
        assert_eq!(
            std::fs::metadata(&wal_path).unwrap().len(),
            0,
            "census never writes wal frames"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn doctor_demo_and_dir_lanes_skip_the_census() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Demo arm: hermetic by construction (bundled corpus; no host
    // walk) — must not mention the state stores.
    let demo_text = render_doctor_report(None, true, "text").expect("demo render");
    assert!(
        !demo_text.contains("Codex state stores"),
        "demo arm is zero-delta"
    );
    let demo_json = render_doctor_report(None, true, "json").expect("demo json");
    let parsed: serde_json::Value = serde_json::from_str(&demo_json).expect("demo json parses");
    assert!(
        parsed.get("codex_state").is_none(),
        "empty census is not serialized"
    );
    // `--dir` arm: the census is a real-home fact; a custom corpus
    // directory must stay byte-identical (no state-store lines).
    let root = fixture_root("dirlane");
    let corpus = root.join("corpus");
    std::fs::create_dir_all(&corpus).unwrap();
    std::fs::write(corpus.join("s.jsonl"), b"{}\n").unwrap();
    let dir_text = render_doctor_report(Some(&corpus), false, "text").expect("dir render");
    assert!(
        !dir_text.contains("Codex state stores"),
        "--dir lane is zero-delta"
    );
    let _ = std::fs::remove_dir_all(&root);
}

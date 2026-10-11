//! rm-1004 contract tests: Cursor's live sqlite stores ingest with
//! the same honesty the file lanes hold — read-only opens, recorded
//! cost precedence over estimates, ccusage #1771's prompt-token
//! fallback disclosed by counter, cross-store dedup, and unreadable
//! stores disclosed through the report instead of rendering as an
//! absent corpus.
//!
//! Stores are synthesized at test time with rusqlite (the shape
//! ccusage #1771 documents live and codeburn validates against real
//! installs): a `state.vscdb` with a `cursorDiskKV` table carrying
//! `composerData:<id>` / `bubbleId:<composerId>:<bubbleId>` rows, and
//! a `~/.cursor/chats/**/store.db` variant with dedicated
//! `composerData` / `bubbles` tables.

use std::path::{Path, PathBuf};

use agenttrace_core::{
    load_sqlite_backed_sessions_reported, skip_sqlite_backed_file_dir, SqliteIngestReport,
};

fn temp_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm1004-contract-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("temp home");
    home
}

/// Same env-scoped harness as `sqlite_hostile_disclosure.rs`, plus
/// `CURSOR_HOME` so a relocated host install never leaks in and the
/// test host's real `~/.config/Cursor` is never read.
fn with_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
    let result = {
        let _guard = lock_env();
        let saved: Vec<(&str, Option<std::ffi::OsString>)> = [
            "HOME",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "XDG_DATA_HOME",
            "AGENTTRACE_SESSION_CACHE_DIR",
            "CURSOR_HOME",
        ]
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect();
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        std::env::remove_var("CURSOR_HOME");
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

/// The `state.vscdb` default candidate on this OS, computed WITHOUT
/// ambient env — `with_home` mutates the process env concurrently,
/// and a store path built while a sibling test is mid-load must not
/// depend on it. Mirrors `cursor_state_vscdb_paths`' default arm,
/// which `with_home` scopes to this same home.
fn state_db(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library")
            .join("Application Support")
            .join("Cursor")
            .join("User/globalStorage/state.vscdb")
    } else if cfg!(target_os = "windows") {
        home.join("AppData/Roaming/Cursor/User/globalStorage/state.vscdb")
    } else {
        home.join(".config/Cursor/User/globalStorage/state.vscdb")
    }
}

fn kv_store(path: &Path, rows: &[(&str, &str)]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("store parent");
    }
    let conn = rusqlite::Connection::open(path).expect("create store");
    conn.execute_batch(
        "create table if not exists cursorDiskKV (key text primary key, value text);",
    )
    .expect("table");
    for (key, value) in rows {
        conn.execute(
            "insert or replace into cursorDiskKV (key, value) values (?1, ?2)",
            [key, value],
        )
        .expect("row");
    }
}

fn composer_row(_id: &str, name: &str, created_at: &str, total_used: Option<i64>) -> String {
    let breakdown = match total_used {
        Some(total) => {
            format!(r#"{{"totalUsedTokens":{total}}}"#)
        }
        None => "{}".to_string(),
    };
    format!(r#"{{"name":"{name}","createdAt":"{created_at}","promptTokenBreakdown":{breakdown}}}"#)
}

fn bubble_row(
    _bubble_id: &str,
    conversation_id: &str,
    bubble_type: i64,
    text: &str,
    created_at: &str,
    token_count: Option<&str>,
    model: Option<&str>,
) -> String {
    let mut row = format!(
        r#"{{"conversationId":"{conversation_id}","type":{bubble_type},"text":"{text}","createdAt":"{created_at}""#
    );
    if let Some(token_count) = token_count {
        row.push_str(&format!(r#","tokenCount":{token_count}"#));
    }
    if let Some(model) = model {
        row.push_str(&format!(r#","modelInfo":{{"modelName":"{model}"}}"#));
    }
    row.push('}');
    row
}

fn load(home: &Path) -> (Vec<agenttrace_core::Session>, SqliteIngestReport) {
    let scoped = home.to_path_buf();
    with_home(&scoped, load_sqlite_backed_sessions_reported_once)
}

fn load_sqlite_backed_sessions_reported_once() -> (Vec<agenttrace_core::Session>, SqliteIngestReport)
{
    load_sqlite_backed_sessions_reported(None)
}

fn cursor_sessions(sessions: &[agenttrace_core::Session]) -> Vec<&agenttrace_core::Session> {
    sessions
        .iter()
        .filter(|session| session.metrics.source_tool == "cursor")
        .collect()
}

// ---------------------------------------------------------------------
// Ingest + recorded-cost precedence
// ---------------------------------------------------------------------

#[test]
fn state_vscdb_bubbles_ingest_with_recorded_cost_precedence() {
    let home = temp_home("precedence");
    let store = state_db(&home);
    kv_store(
        &store,
        &[
            ("uiState:lastWindow", r#"{"theme":"dark"}"#),
            (
                "composerData:cA",
                &composer_row(
                    "cA",
                    "Fix the login bug",
                    "2026-10-01T10:00:00Z",
                    Some(999_999),
                ),
            ),
            (
                "bubbleId:cA:b1",
                &bubble_row(
                    "b1",
                    "convA",
                    1,
                    "Please fix the login bug",
                    "2026-10-01T10:00:05Z",
                    None,
                    None,
                ),
            ),
            (
                "bubbleId:cA:b2",
                &bubble_row(
                    "b2",
                    "convA",
                    2,
                    "Done — redirect after auth.",
                    "2026-10-01T10:00:30Z",
                    Some(
                        r#"{"inputTokens":1200,"outputTokens":340,"cacheReadTokens":500,"costUSD":0.0123}"#,
                    ),
                    Some("claude-sonnet-4-5"),
                ),
            ),
        ],
    );
    let (sessions, report) = load(&home);
    assert!(
        report.unreadable.is_empty() && report.dropped_rows.is_empty(),
        "report: {report:?}"
    );
    let cursor = cursor_sessions(&sessions);
    assert_eq!(cursor.len(), 1, "sessions: {sessions:?}");
    let session = cursor[0];
    assert_eq!(session.metrics.source_tool, "cursor");
    assert_eq!(session.metrics.session_key, "convA");
    assert_eq!(session.name, "Fix the login bug");
    assert_eq!(session.metrics.provenance.naming, "provider_title");
    assert_eq!(session.metrics.tokens_input, 1200);
    assert_eq!(session.metrics.tokens_output, 340);
    assert_eq!(session.metrics.tokens_cache_r, 500);
    assert_eq!(session.metrics.user_messages, 1);
    assert_eq!(session.metrics.assistant_turns, 1);
    assert_eq!(session.metrics.model_used, "claude-sonnet-4-5");
    // rm-253 precedence: the recorded per-bubble cost prices exactly
    // its own tokens — the catalog estimate never replaces it.
    assert!(
        (session.metrics.cost_estimated - 0.0123).abs() < 1e-9,
        "cost_estimated {}",
        session.metrics.cost_estimated
    );
    assert_eq!(
        session.metrics.provenance.cost,
        "calculated_from_tokens_with_recorded_cost"
    );
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("cursor_usage_basis:per_bubble_token_count"),
        Some(&1),
        "counters: {:?}",
        session.metrics.disclosure_counters
    );
    // App state KV rows are ignored, not ingested or dropped.
    assert_eq!(session.metrics.events_total, 3, "meta + 2 messages");
}

#[test]
fn usage_without_text_still_counts() {
    let home = temp_home("meta-only");
    kv_store(
        &state_db(&home),
        &[
            (
                "composerData:cM",
                &composer_row("cM", "", "2026-10-02T10:00:00Z", None),
            ),
            (
                "bubbleId:cM:b1",
                &bubble_row(
                    "b1",
                    "convM",
                    2,
                    "",
                    "2026-10-02T10:00:10Z",
                    Some(r#"{"inputTokens":50,"outputTokens":7}"#),
                    None,
                ),
            ),
        ],
    );
    let (sessions, report) = load(&home);
    assert!(report.unreadable.is_empty(), "report: {report:?}");
    let cursor = cursor_sessions(&sessions);
    assert_eq!(cursor.len(), 1);
    assert_eq!(cursor[0].metrics.tokens_input, 50);
    assert_eq!(cursor[0].metrics.tokens_output, 7);
}

// ---------------------------------------------------------------------
// ccusage #1771 fallback
// ---------------------------------------------------------------------

#[test]
fn prompt_token_breakdown_fallback_applies_once_with_disclosure() {
    let home = temp_home("fallback");
    kv_store(
        &state_db(&home),
        &[
            (
                "composerData:cF",
                &composer_row("cF", "Refactor", "2026-10-01T09:00:00Z", Some(9_999)),
            ),
            (
                "bubbleId:cF:b1",
                &bubble_row(
                    "b1",
                    "convF",
                    1,
                    "Refactor the module",
                    "2026-10-01T09:00:05Z",
                    None,
                    None,
                ),
            ),
            (
                "bubbleId:cF:b2",
                &bubble_row(
                    "b2",
                    "convF",
                    2,
                    "Done.",
                    "2026-10-01T09:00:25Z",
                    None,
                    Some("gpt-5-codex"),
                ),
            ),
        ],
    );
    let (sessions, report) = load(&home);
    assert!(report.unreadable.is_empty(), "report: {report:?}");
    let cursor = cursor_sessions(&sessions);
    assert_eq!(cursor.len(), 1);
    let session = cursor[0];
    // ONCE per conversation, from the composer row, disclosed.
    assert_eq!(session.metrics.tokens_input, 9_999);
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("cursor_prompt_token_breakdown_fallback"),
        Some(&1),
        "counters: {:?}",
        session.metrics.disclosure_counters
    );
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    // No recorded cost anywhere → catalog estimates and says so.
    assert_eq!(session.metrics.provenance.cost, "calculated_from_tokens");
    assert!(
        session.metrics.cost_estimated > 0.0,
        "gpt-5-codex is catalog-priced: {}",
        session.metrics.cost_estimated
    );
}

#[test]
fn composer_embedded_messages_fold_when_bubbles_never_persisted() {
    let home = temp_home("embedded");
    let payload = concat!(
        r#"{"name":"Chat E","createdAt":"2026-10-03T08:00:00Z","conversation":{"messages":["#,
        r#"{"role":"user","text":"hi","createdAt":"2026-10-03T08:00:05Z"},"#,
        r#"{"role":"assistant","text":"hello","createdAt":"2026-10-03T08:00:09Z"}]}}"#
    );
    kv_store(&state_db(&home), &[("composerData:cE", payload)]);
    let (sessions, report) = load(&home);
    assert!(report.unreadable.is_empty(), "report: {report:?}");
    let cursor = cursor_sessions(&sessions);
    assert_eq!(cursor.len(), 1);
    let session = cursor[0];
    assert_eq!(session.name, "Chat E");
    assert_eq!(session.metrics.user_messages, 1);
    assert_eq!(session.metrics.assistant_turns, 1);
    // No usage anywhere: tokens come from text, disclosed as such.
    assert_eq!(session.metrics.provenance.tokens, "estimated_from_text");
}

// ---------------------------------------------------------------------
// store.db variant + cross-store dedup
// ---------------------------------------------------------------------

fn chats_store(home: &Path, rel: &str, rows: &[(&str, &str)]) -> PathBuf {
    let path = home.join(".cursor").join("chats").join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("chats parent");
    }
    let conn = rusqlite::Connection::open(&path).expect("create chats store");
    conn.execute_batch(
        "create table composerData (key text primary key, value text);
         create table bubbles (id text primary key, value text);",
    )
    .expect("tables");
    for (composer, bubble) in rows.iter().filter(|(k, _)| k.starts_with("composerData:")) {
        conn.execute(
            "insert into composerData (key, value) values (?1, ?2)",
            [composer.trim_start_matches("composerData:"), bubble],
        )
        .expect("composer row");
    }
    for (bubble_key, bubble) in rows.iter().filter(|(k, _)| k.starts_with("bubbleId:")) {
        let suffix = bubble_key.trim_start_matches("bubbleId:");
        // Keep the `<composerId>:<bubbleId>` key chain — the adapter
        // derives composer linkage from it.
        conn.execute(
            "insert into bubbles (id, value) values (?1, ?2)",
            [suffix, bubble],
        )
        .expect("bubble row");
    }
    path
}

#[test]
fn store_db_variant_ingests_via_probed_tables() {
    let home = temp_home("store-db");
    chats_store(
        &home,
        "proj-1/chat-2/store.db",
        &[
            (
                "composerData:cS",
                &composer_row("cS", "Store-dot-db chat", "2026-10-04T10:00:00Z", None),
            ),
            (
                "bubbleId:cS:b1",
                &bubble_row(
                    "b1",
                    "convS",
                    1,
                    "hello from store.db",
                    "2026-10-04T10:00:05Z",
                    Some(r#"{"inputTokens":10,"outputTokens":3}"#),
                    Some("gpt-5-codex"),
                ),
            ),
        ],
    );
    let (sessions, report) = load(&home);
    assert!(report.unreadable.is_empty(), "report: {report:?}");
    let cursor = cursor_sessions(&sessions);
    assert_eq!(cursor.len(), 1);
    assert_eq!(cursor[0].metrics.session_key, "convS");
    assert_eq!(cursor[0].metrics.tokens_input, 10);
    assert_eq!(cursor[0].name, "Store-dot-db chat");
}

#[test]
fn same_conversation_in_two_stores_counts_once() {
    let home = temp_home("dedup");
    kv_store(
        &state_db(&home),
        &[
            (
                "composerData:cD",
                &composer_row("cD", "Shared chat", "2026-10-05T10:00:00Z", None),
            ),
            (
                "bubbleId:cD:b1",
                &bubble_row(
                    "b1",
                    "convD",
                    1,
                    "same chat, two stores",
                    "2026-10-05T10:00:05Z",
                    Some(r#"{"inputTokens":100,"outputTokens":20}"#),
                    None,
                ),
            ),
        ],
    );
    chats_store(
        &home,
        "proj/chat/store.db",
        &[
            (
                "composerData:cD",
                &composer_row("cD", "Shared chat", "2026-10-05T10:00:00Z", None),
            ),
            (
                "bubbleId:cD:b1",
                &bubble_row(
                    "b1",
                    "convD",
                    1,
                    "same chat, two stores",
                    "2026-10-05T10:00:05Z",
                    Some(r#"{"inputTokens":100,"outputTokens":20}"#),
                    None,
                ),
            ),
        ],
    );
    let (sessions, report) = load(&home);
    let cursor = cursor_sessions(&sessions);
    assert_eq!(
        cursor.len(),
        1,
        "the (source_tool, session_key) dedup keeps one copy"
    );
    assert_eq!(cursor[0].metrics.tokens_input, 100);
    // The duplicate copy is disclosed, not silent.
    assert_eq!(report.duplicate_dbs.len(), 1, "report: {report:?}");
    assert_eq!(report.duplicate_dbs[0].duplicate_sessions, 1);
}

// ---------------------------------------------------------------------
// Hostile stores disclose
// ---------------------------------------------------------------------

#[test]
fn corrupt_state_db_discloses_through_report() {
    let home = temp_home("corrupt");
    let store = state_db(&home);
    std::fs::create_dir_all(store.parent().unwrap()).expect("parent");
    std::fs::write(&store, b"definitely not a sqlite database").expect("write");
    let (sessions, report) = load(&home);
    assert!(cursor_sessions(&sessions).is_empty());
    assert_eq!(report.unreadable.len(), 1, "report: {report:?}");
    assert_eq!(report.unreadable[0].source, "cursor");
    assert!(
        report.unreadable[0].reason.contains("cursor"),
        "reason names the store family: {}",
        report.unreadable[0].reason
    );
}

#[test]
fn foreign_sqlite_store_db_discloses_as_unrecognized_schema() {
    let home = temp_home("foreign");
    let path = home.join(".cursor").join("chats").join("a/b/store.db");
    std::fs::create_dir_all(path.parent().unwrap()).expect("parent");
    let conn = rusqlite::Connection::open(&path).expect("foreign store");
    conn.execute_batch("create table blobs (payload blob); create table meta (k text);")
        .expect("foreign tables");
    drop(conn);
    let (sessions, report) = load(&home);
    assert!(cursor_sessions(&sessions).is_empty());
    assert_eq!(report.unreadable.len(), 1, "report: {report:?}");
    assert!(
        report.unreadable[0]
            .reason
            .contains("unrecognized cursor store schema"),
        "reason: {}",
        report.unreadable[0].reason
    );
}

#[test]
fn malformed_payload_rows_are_counted_not_erased() {
    let home = temp_home("dropped");
    kv_store(
        &state_db(&home),
        &[
            (
                "composerData:cX",
                &composer_row("cX", "", "2026-10-06T10:00:00Z", None),
            ),
            (
                "bubbleId:cX:b1",
                r#"{"type":1,"text":"ok msg","createdAt":"2026-10-06T10:00:05Z"}"#,
            ),
            ("bubbleId:cX:b2", "{not json at all"),
        ],
    );
    let (sessions, report) = load(&home);
    let cursor = cursor_sessions(&sessions);
    assert_eq!(cursor.len(), 1);
    assert_eq!(cursor[0].metrics.user_messages, 1);
    assert_eq!(report.dropped_rows.len(), 1, "report: {report:?}");
    assert_eq!(report.dropped_rows[0].dropped, 1);
    assert_eq!(report.dropped_rows[0].source, "cursor");
    assert_eq!(report.dropped_rows[0].sample, "bubbleId:cX:b2");
}

// ---------------------------------------------------------------------
// Read-only discipline
// ---------------------------------------------------------------------

#[test]
fn store_reads_never_write_and_survive_read_only_permissions() {
    let home = temp_home("readonly");
    let store = state_db(&home);
    kv_store(
        &store,
        &[
            (
                "composerData:cR",
                &composer_row("cR", "", "2026-10-07T10:00:00Z", None),
            ),
            (
                "bubbleId:cR:b1",
                &bubble_row(
                    "b1",
                    "convR",
                    1,
                    "read only please",
                    "2026-10-07T10:00:05Z",
                    Some(r#"{"inputTokens":11,"outputTokens":2}"#),
                    None,
                ),
            ),
        ],
    );
    let before = std::fs::read(&store).expect("read store");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o444))
            .expect("chmod 444");
    }
    let (sessions, report) = load(&home);
    assert!(report.unreadable.is_empty(), "report: {report:?}");
    let cursor = cursor_sessions(&sessions);
    assert_eq!(cursor.len(), 1, "a 0444 store still ingests read-only");
    assert_eq!(cursor[0].metrics.tokens_input, 11);
    let after = std::fs::read(&store).expect("re-read store");
    assert_eq!(before, after, "the store bytes never change");
    let dir = store.parent().unwrap();
    let sidecars: Vec<_> = std::fs::read_dir(dir)
        .expect("dir")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| {
            name.ends_with("-wal") || name.ends_with("-shm") || name.ends_with("-journal")
        })
        .collect();
    assert!(
        sidecars.is_empty(),
        "no journal sidecars created: {sidecars:?}"
    );
}

// ---------------------------------------------------------------------
// Transcript skip gate (rm-1004 coverage truth)
// ---------------------------------------------------------------------

#[test]
fn transcript_skip_is_gated_on_real_store_coverage() {
    let home = temp_home("skip-gate");
    let projects = home.join(".cursor").join("projects");
    std::fs::create_dir_all(&projects).expect("projects dir");
    let skip_in = |home: &Path, dir: &Path| {
        let scoped = home.to_path_buf();
        let dir = dir.to_path_buf();
        with_home(&scoped, move || skip_sqlite_backed_file_dir(&dir))
    };
    // No store at all: transcripts are the only coverage → counted.
    assert!(!skip_in(&home, &projects));
    // Every Cursor install ships a state.vscdb of app state; chat-free
    // is NOT coverage → transcripts still counted.
    kv_store(&state_db(&home), &[("uiState:theme", r#"{"v":"dark"}"#)]);
    assert!(
        !skip_in(&home, &projects),
        "an app-state-only store must not skip transcripts"
    );
    // Real chat rows → the store is authoritative, transcripts skip.
    kv_store(
        &state_db(&home),
        &[
            ("uiState:theme", r#"{"v":"dark"}"#),
            (
                "composerData:cG",
                &composer_row("cG", "", "2026-10-08T10:00:00Z", None),
            ),
            (
                "bubbleId:cG:b1",
                &bubble_row("b1", "convG", 1, "hi", "2026-10-08T10:00:05Z", None, None),
            ),
        ],
    );
    assert!(skip_in(&home, &projects));
    // Unrelated directories never ride this arm — the coverage gate
    // is scoped to `~/.cursor/projects` only.
    assert!(!skip_in(&home, &home.join(".hermes")));
    assert!(!skip_in(&home, &home.join(".cursor")));
}

// ---------------------------------------------------------------------
// Relocated installs + snapshot cache
// ---------------------------------------------------------------------

#[test]
fn cursor_home_env_relocates_the_state_db() {
    let home = temp_home("relocated");
    let relocated = home.join("cursor-root");
    kv_store(
        &relocated
            .join("User")
            .join("globalStorage")
            .join("state.vscdb"),
        &[
            (
                "composerData:cH",
                &composer_row("cH", "Relocated", "2026-10-09T10:00:00Z", None),
            ),
            (
                "bubbleId:cH:b1",
                &bubble_row(
                    "b1",
                    "convH",
                    1,
                    "from CURSOR_HOME",
                    "2026-10-09T10:00:05Z",
                    Some(r#"{"inputTokens":21,"outputTokens":4}"#),
                    None,
                ),
            ),
        ],
    );
    let scoped = home.to_path_buf();
    let (sessions, report) = with_home(&scoped, || {
        std::env::set_var("CURSOR_HOME", &relocated);
        load_sqlite_backed_sessions_reported_once()
    });
    assert!(report.unreadable.is_empty(), "report: {report:?}");
    let cursor = cursor_sessions(&sessions);
    assert_eq!(cursor.len(), 1);
    assert_eq!(cursor[0].metrics.tokens_input, 21);
}

#[test]
fn second_load_serves_the_snapshot_cache_identically() {
    let home = temp_home("snapshot");
    kv_store(
        &state_db(&home),
        &[
            (
                "composerData:cN",
                &composer_row("cN", "Warm", "2026-10-10T10:00:00Z", None),
            ),
            (
                "bubbleId:cN:b1",
                &bubble_row(
                    "b1",
                    "convN",
                    2,
                    "warm hit",
                    "2026-10-10T10:00:05Z",
                    Some(r#"{"inputTokens":31,"outputTokens":5,"costUSD":0.0077}"#),
                    Some("claude-sonnet-4-5"),
                ),
            ),
        ],
    );
    let (first, _) = load(&home);
    let (second, report) = load(&home);
    assert!(report.unreadable.is_empty(), "report: {report:?}");
    assert_eq!(cursor_sessions(&first).len(), 1);
    assert_eq!(cursor_sessions(&second).len(), 1);
    let a = &cursor_sessions(&first)[0].metrics;
    let b = &cursor_sessions(&second)[0].metrics;
    assert_eq!(a.tokens_input, b.tokens_input);
    assert_eq!(a.tokens_output, b.tokens_output);
    assert!((a.cost_estimated - b.cost_estimated).abs() < 1e-12);
    assert_eq!(a.provenance.cost, b.provenance.cost);
}

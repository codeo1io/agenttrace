//! rm-1004: read-only adapter for Cursor's live sqlite stores.
//!
//! Cursor persists IDE agent chats OUTSIDE the `~/.cursor/projects`
//! transcript walk, in sqlite the file lane never sees:
//!
//! * the global app-state database `state.vscdb`
//!   (`…/Cursor/User/globalStorage/state.vscdb`), whose
//!   `cursorDiskKV` table carries `composerData:<id>` conversation
//!   rows and `bubbleId:<composerId>:<bubbleId>` message rows —
//!   per-bubble `tokenCount` (`inputTokens`/`outputTokens`), optional
//!   `modelInfo.modelName`, `createdAt`, `conversationId` — the
//!   schema ccusage #1771 documents live and the codeburn reader
//!   validates against real installs;
//! * per-chat `store.db` files under `~/.cursor/chats/`, which
//!   persist the same composerData/bubbles payloads on some installs
//!   (same issue, same spellings).
//!
//! Contract (roadmap rm-1004):
//! * every open is READ-ONLY — the stores belong to the editor; this
//!   reader never writes, never creates WAL files, never blocks past
//!   the shared busy timeout;
//! * a recorded cost (`tokenCount.costUSD`-class spellings) is
//!   preferred per usage block and prices exactly its own tokens;
//!   blocks without one estimate from the pricing catalog and are
//!   disclosed as `calculated_from_tokens` (rm-253 precedence);
//! * per-bubble `tokenCount` is preferred; when a conversation has
//!   none, the composer's `promptTokenBreakdown.totalUsedTokens` is
//!   applied ONCE per conversation and disclosed by counter
//!   (ccusage #1771's documented fallback);
//! * sessions dedup by `(source_tool, session_key)` across stores,
//!   so a chat visible in both `state.vscdb` and a `chats/store.db`
//!   counts once (rm-607 union discipline);
//! * an unreadable or unrecognized store discloses through the
//!   sqlite ingest report (`--doctor`), never silently as an absent
//!   corpus.

use crate::parser::{cursor_composer_message_events, cursor_timestamp};
use crate::sqlite_sessions::{
    filter_since, open_sqlite_read_only, sqlite_file_exists, SqliteDroppedRows, SqliteIngestReport,
    SqliteUnreadableDb,
};
use crate::{session_from_events, Event, Session};
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// `source_tool` for every lane this module emits. Same identifier as
/// the Cursor transcript/export file lanes, so aggregates group all
/// Cursor sessions under one agent.
const CURSOR_SOURCE: &str = "cursor";

/// Candidate `state.vscdb` paths, primary first: `$CURSOR_HOME` when
/// Cursor's relocation env is set, then the OS-default user-data dir
/// (XDG_CONFIG_HOME-aware on Linux). Order decides dedup precedence
/// (rm-607: first occurrence wins).
pub fn cursor_state_vscdb_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(root) = std::env::var_os("CURSOR_HOME").map(PathBuf::from) {
        if !root.as_os_str().is_empty() {
            paths.push(cursor_global_storage(&root));
        }
    }
    let default_root = if cfg!(target_os = "macos") {
        home.join("Library")
            .join("Application Support")
            .join("Cursor")
    } else if cfg!(target_os = "windows") {
        home.join("AppData").join("Roaming").join("Cursor")
    } else {
        env_dir("XDG_CONFIG_HOME", &home.join(".config")).join("Cursor")
    };
    paths.push(cursor_global_storage(&default_root));
    paths
}

fn cursor_global_storage(root: &Path) -> PathBuf {
    root.join("User").join("globalStorage").join("state.vscdb")
}

fn env_dir(name: &str, fallback: &Path) -> PathBuf {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| fallback.to_path_buf())
}

/// `~/.cursor/chats` — Cursor's per-chat `store.db` files live below
/// this root (layout `chats/<project>/<chat>/store.db`).
pub fn cursor_chats_root(home: &Path) -> PathBuf {
    home.join(".cursor").join("chats")
}

fn cursor_chats_store_dbs(home: &Path) -> Vec<PathBuf> {
    let root = cursor_chats_root(home);
    let mut found = Vec::new();
    walk_for_store_dbs(&root, 0, &mut found);
    found.sort();
    found
}

/// Depth-capped walk for `store.db` leaves. Depth keeps pathological
/// homes bounded; sorting keeps the walk deterministic so the dedup
/// precedence is stable run to run.
fn walk_for_store_dbs(dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if depth > 2 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<_> = entries.filter_map(Result::ok).collect();
    children.sort_by_key(|entry| entry.file_name());
    for entry in children {
        let path = entry.path();
        if path.is_dir() {
            walk_for_store_dbs(&path, depth + 1, found);
        } else if entry.file_name() == "store.db" {
            found.push(path);
        }
    }
}

/// Every live-store database the adapter reads, precedence-ordered:
/// the `state.vscdb` candidates, then the `chats/**/store.db` files.
pub fn cursor_store_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = cursor_state_vscdb_paths(home);
    paths.extend(cursor_chats_store_dbs(home));
    paths
}

/// One store's sessions, snapshot-cached like the hermes/opencode
/// lanes (fingerprint-invalidated on db/wal/shm/pricing changes;
/// rm-753: an EMPTY snapshot is never trusted as final — a genuinely
/// empty database re-queries cheaply, a corrupt one re-detects).
pub(crate) fn load_cursor_store_sessions(
    path: &Path,
    since: Option<chrono::DateTime<chrono::Utc>>,
    report: &mut SqliteIngestReport,
) -> Vec<Session> {
    if !sqlite_file_exists(path) {
        return Vec::new();
    }
    if let Some(sessions) = crate::session_cache::load_sqlite_snapshot(path, CURSOR_SOURCE) {
        if !sessions.is_empty() {
            return filter_since(sessions, since);
        }
    }
    let (sessions, clean) = read_store_sessions(path, report);
    if clean {
        let _ = crate::session_cache::store_sqlite_snapshot(path, CURSOR_SOURCE, &sessions);
    }
    filter_since(sessions, since)
}

fn read_store_sessions(path: &Path, report: &mut SqliteIngestReport) -> (Vec<Session>, bool) {
    match read_cursor_store(path) {
        Ok(Some(rows)) => sessions_from_rows(path, &rows, report),
        // Absent is a legitimate empty corpus, not a failure.
        Ok(None) => (Vec::new(), true),
        Err(reason) => {
            report.unreadable.push(SqliteUnreadableDb {
                path: path.to_path_buf(),
                source: CURSOR_SOURCE,
                reason,
            });
            (Vec::new(), false)
        }
    }
}

struct CursorStoreRows {
    /// `(key, payload)` pairs; keys are normalized to the
    /// `composerData:<id>` / `bubbleId:<…>` spellings regardless of
    /// which store shape produced them.
    rows: Vec<(String, String)>,
}

fn read_cursor_store(path: &Path) -> Result<Option<CursorStoreRows>, String> {
    if !sqlite_file_exists(path) {
        return Ok(None);
    }
    let conn = open_sqlite_read_only(path).map_err(|error| format!("open failed: {error}"))?;
    if table_exists(&conn, "cursorDiskKV") {
        let rows = read_kv_rows(&conn).map_err(|error| format!("read cursorDiskKV: {error}"))?;
        return Ok(Some(CursorStoreRows { rows }));
    }
    // The `chats/store.db` variant (ccusage #1771): dedicated
    // `composerData` / `bubbles` tables carrying the same payloads.
    // The documentation does not pin their column names, so the
    // id/payload pair is probed from the declared schema.
    const PAYLOAD_COLUMNS: [&str; 5] = ["value", "data", "json", "payload", "blob"];
    let mut rows = Vec::new();
    let mut matched = false;
    for (table, prefix, id_columns) in [
        (
            "composerData",
            "composerData:",
            ["key", "composerId", "id"].as_slice(),
        ),
        (
            "bubbles",
            "bubbleId:",
            ["id", "bubbleId", "key", "composerId"].as_slice(),
        ),
    ] {
        if !table_exists(&conn, table) {
            continue;
        }
        let Some((id_col, payload_col)) = probe_columns(&conn, table, id_columns, &PAYLOAD_COLUMNS)
        else {
            continue;
        };
        matched = true;
        let mut table_rows = read_table_rows(&conn, table, &id_col, &payload_col, prefix)
            .map_err(|error| format!("read {table}: {error}"))?;
        rows.append(&mut table_rows);
    }
    if !matched {
        return Err(
            "unrecognized cursor store schema (no cursorDiskKV, composerData, or bubbles table)"
                .to_string(),
        );
    }
    Ok(Some(CursorStoreRows { rows }))
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "select 1 from sqlite_master where type = 'table' and name = ?1",
        [table],
        |_| Ok(()),
    )
    .is_ok()
}

fn column_names(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("pragma table_info({table})"))
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
            Ok(rows.filter_map(Result::ok).collect::<Vec<_>>())
        })
        .unwrap_or_default()
}

fn probe_columns(
    conn: &Connection,
    table: &str,
    id_candidates: &[&str],
    payload_candidates: &[&str],
) -> Option<(String, String)> {
    let columns = column_names(conn, table);
    let id = id_candidates
        .iter()
        .find(|name| columns.iter().any(|column| column == *name))?;
    let payload = payload_candidates
        .iter()
        .find(|name| columns.iter().any(|column| column == *name))?;
    Some(((*id).to_string(), (*payload).to_string()))
}

fn read_kv_rows(conn: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("select key, value from cursorDiskKV")?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        Ok((key, payload_text(row.get_ref(1)?)))
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

/// Table and column names in this query are literals or values read
/// back from `pragma table_info` on a table that passed the
/// sqlite_master literal-name check — no user input reaches the SQL.
#[allow(clippy::needless_pass_by_value)]
fn read_table_rows(
    conn: &Connection,
    table: &str,
    id_column: &str,
    payload_column: &str,
    prefix: &str,
) -> rusqlite::Result<Vec<(String, String)>> {
    let sql = format!("select {id_column}, {payload_column} from {table}");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        let id: Option<String> = row.get(0)?;
        Ok((
            format!("{prefix}{}", id.unwrap_or_default()),
            payload_text(row.get_ref(1)?),
        ))
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

/// SQLite payload cells arrive as TEXT or BLOB depending on the
/// writer; both decode lossily to the JSON document this reader
/// expects.
fn payload_text(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Text(text) => String::from_utf8_lossy(text).into_owned(),
        ValueRef::Blob(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        _ => String::new(),
    }
}

/// rm-1004 coverage gate for the transcript walk (see
/// `skip_sqlite_backed_file_dir`): true when the IDE's live store
/// both opens read-only and actually carries chat rows. Existence
/// alone is NOT enough — every Cursor install ships a `state.vscdb`
/// full of app state, so the gate must prove chat coverage before
/// `~/.cursor/projects` transcripts may be skipped as the store's
/// re-emitted copies (codeburn's `ide-composer-skip-v1` rationale).
pub(crate) fn cursor_store_covers_transcripts(home: &Path) -> bool {
    for path in cursor_state_vscdb_paths(home) {
        if !sqlite_file_exists(&path) {
            continue;
        }
        let Ok(conn) = open_sqlite_read_only(&path) else {
            continue;
        };
        let covered = table_exists(&conn, "cursorDiskKV")
            && conn
                .query_row(
                    "select count(*) from cursorDiskKV where key like 'composerData:%' or key like 'bubbleId:%'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map(|count| count > 0)
                .unwrap_or(false);
        if covered {
            return true;
        }
    }
    // A chats-only install (no chat rows in state.vscdb) still covers
    // its transcripts when any `~/.cursor/chats/**/store.db` carries
    // composer rows — the gate must follow the coverage, not the
    // store spelling (live-probe leg C: a chats-only home would else
    // double-count every conversation).
    for path in cursor_chats_store_dbs(home) {
        let Ok(conn) = open_sqlite_read_only(&path) else {
            continue;
        };
        let covered = table_exists(&conn, "composerData")
            && conn
                .query_row("select count(*) from composerData", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map(|count| count > 0)
                .unwrap_or(false);
        if covered {
            return true;
        }
    }
    false
}

struct CursorComposer {
    name: String,
    created_at: String,
    prompt_breakdown_total: Option<i64>,
    /// The raw composer object, kept only when it embeds message
    /// history (`conversation.messages` / `messages`) so the
    /// export-lane extractor can fold it for conversations whose
    /// bubbles were never persisted.
    embedded: Option<Map<String, Value>>,
}

#[derive(Default)]
struct CursorConversation {
    title: Option<String>,
    composer_created_at: String,
    prompt_breakdown_total: Option<i64>,
    embedded_events: Vec<Event>,
    bubbles: Vec<CursorBubble>,
}

#[derive(Clone)]
struct CursorBubble {
    /// Insertion sequence — a stable tiebreak for equal timestamps.
    seq: usize,
    conversation_id: String,
    /// The composer this bubble chains to (from the key), even when
    /// the payload omits conversationId — used to attach composer
    /// metadata to the conversation it actually belongs to.
    composer_id: String,
    role: &'static str,
    text: String,
    created_at: String,
    model: String,
    usage: Option<BTreeMap<String, i64>>,
    recorded_cost_usd: Option<f64>,
}

fn json_string(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn json_i64(map: &Map<String, Value>, key: &str) -> Option<i64> {
    map.get(key)
        .and_then(crate::parser::number_as_i64)
        .map(|value| value.max(0))
}

/// Token classes a bubble's `tokenCount` carries, normalized to the
/// canonical usage keys. Alternate spellings come from the observed
/// Cursor wire formats; unrecognized keys are ignored (not invented).
fn bubble_usage(token_count: &Map<String, Value>) -> BTreeMap<String, i64> {
    let mut usage = BTreeMap::new();
    for (target, keys) in [
        ("input_tokens", ["inputTokens", "promptTokens"].as_slice()),
        (
            "output_tokens",
            ["outputTokens", "completionTokens"].as_slice(),
        ),
        (
            "cache_read_input_tokens",
            ["cacheReadTokens", "cacheReadInputTokens"].as_slice(),
        ),
        (
            "cache_creation_input_tokens",
            ["cacheWriteTokens", "cacheCreationInputTokens"].as_slice(),
        ),
    ] {
        for key in keys {
            if let Some(value) = json_i64(token_count, key) {
                usage.insert(target.to_string(), value);
                break;
            }
        }
    }
    usage
}

/// A recorded cost spelling on a usage block or its bubble. Mirrors
/// the pi `usage.cost.total` discipline (rm-436): PRESENT beats
/// estimated, and a present-but-unusable value (negative, NaN) is
/// absent rather than trusted.
fn recorded_cost_usd(
    token_count: Option<&Map<String, Value>>,
    bubble: &Map<String, Value>,
) -> Option<f64> {
    let mut candidates: Vec<Option<f64>> = Vec::new();
    if let Some(token_count) = token_count {
        for key in ["costUSD", "costUsd", "totalCostUsd"] {
            candidates.push(token_count.get(key).and_then(Value::as_f64));
        }
    }
    for key in ["costUSD", "costUsd"] {
        candidates.push(bubble.get(key).and_then(Value::as_f64));
    }
    candidates
        .into_iter()
        .flatten()
        .find(|value| value.is_finite() && *value >= 0.0)
}

fn parse_cursor_composer(_key_suffix: &str, payload: &str) -> Option<CursorComposer> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let obj = value.as_object()?;
    let prompt_breakdown_total = obj
        .get("promptTokenBreakdown")
        .and_then(|breakdown| breakdown.get("totalUsedTokens"))
        .and_then(crate::parser::number_as_i64)
        .map(|value| value.max(0));
    let created_at = if cursor_timestamp(obj.get("createdAt")).is_empty() {
        cursor_timestamp(obj.get("lastUpdatedAt"))
    } else {
        cursor_timestamp(obj.get("createdAt"))
    };
    let embeds_messages = obj
        .get("conversation")
        .and_then(|conversation| conversation.get("messages"))
        .is_some()
        || obj.get("messages").is_some();
    Some(CursorComposer {
        name: json_string(obj, "name").unwrap_or_default(),
        created_at,
        prompt_breakdown_total,
        embedded: embeds_messages.then(|| obj.clone()),
    })
}

fn parse_cursor_bubble(seq: usize, key_suffix: &str, payload: &str) -> Option<CursorBubble> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let obj = value.as_object()?;
    // `bubbleId:<composerId>:<bubbleId>` keys chain a bubble to its
    // composer even when the bubble payload omits conversationId.
    let composer_id = composer_id_of(key_suffix);
    let conversation_id = json_string(obj, "conversationId").unwrap_or_else(|| composer_id.clone());
    let text = json_string(obj, "text").unwrap_or_default();
    let model = obj
        .get("modelInfo")
        .and_then(|model| model.get("modelName"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|model| !model.is_empty())
        .unwrap_or_default();
    let token_count = obj.get("tokenCount").and_then(Value::as_object);
    let usage = token_count.map(bubble_usage);
    let recorded_cost_usd = recorded_cost_usd(token_count, obj);
    let role = match json_i64(obj, "type") {
        Some(1) => "user",
        Some(2) => "assistant",
        _ => match json_string(obj, "role").as_deref() {
            Some("user" | "human") => "user",
            Some("assistant" | "ai") => "assistant",
            _ if !text.is_empty() => "tool",
            _ => return None,
        },
    };
    Some(CursorBubble {
        seq,
        conversation_id,
        composer_id,
        role,
        text,
        created_at: cursor_timestamp(obj.get("createdAt")),
        model,
        usage,
        recorded_cost_usd,
    })
}

fn sessions_from_rows(
    path: &Path,
    store: &CursorStoreRows,
    report: &mut SqliteIngestReport,
) -> (Vec<Session>, bool) {
    // Pass one: decode rows. Composers and bubbles interleave in the
    // KV, so bubbles carry their composer id and metadata attaches
    // in the grouping pass below.
    let mut composers: HashMap<String, CursorComposer> = HashMap::new();
    let mut bubbles: Vec<CursorBubble> = Vec::new();
    let mut dropped = 0usize;
    let mut dropped_sample = String::new();
    for (key, payload) in &store.rows {
        if let Some(composer_id) = key.strip_prefix("composerData:") {
            match parse_cursor_composer(composer_id, payload) {
                Some(composer) => {
                    composers.insert(composer_id.to_string(), composer);
                }
                None => {
                    dropped += 1;
                    if dropped_sample.is_empty() {
                        dropped_sample = key.clone();
                    }
                }
            }
        } else if let Some(key_suffix) = key.strip_prefix("bubbleId:") {
            match parse_cursor_bubble(bubbles.len(), key_suffix, payload) {
                Some(bubble) => bubbles.push(bubble),
                None => {
                    dropped += 1;
                    if dropped_sample.is_empty() {
                        dropped_sample = key.clone();
                    }
                }
            }
        }
        // Other cursorDiskKV rows are Cursor app state, not chats —
        // deliberately ignored, not dropped.
    }
    let clean = dropped == 0;
    if dropped > 0 {
        report.dropped_rows.push(SqliteDroppedRows {
            path: path.to_path_buf(),
            source: CURSOR_SOURCE,
            dropped,
            sample: dropped_sample,
        });
    }
    // Pass two: group bubbles into conversations, attaching the
    // composer metadata each conversation's bubbles chain to (first
    // composer wins the title; prompt fallbacks fill only gaps).
    let mut conversations: BTreeMap<String, CursorConversation> = BTreeMap::new();
    let mut referenced_composers: BTreeSet<String> = BTreeSet::new();
    for bubble in bubbles {
        let composer = composers.get(&bubble.composer_id);
        referenced_composers.insert(bubble.composer_id.clone());
        let conversation = conversations
            .entry(bubble.conversation_id.clone())
            .or_default();
        if let Some(composer) = composer {
            if conversation.title.is_none() && !composer.name.is_empty() {
                conversation.title = Some(composer.name.clone());
                conversation.composer_created_at = composer.created_at.clone();
            }
            conversation.prompt_breakdown_total = conversation
                .prompt_breakdown_total
                .or(composer.prompt_breakdown_total);
        }
        conversation.bubbles.push(bubble);
    }
    // Composers whose bubbles never persisted still count when they
    // embed message history — keyed apart from bubble conversation
    // ids so a store carrying both shapes cannot collide.
    for (composer_id, composer) in &composers {
        if referenced_composers.contains(composer_id) {
            continue;
        }
        let Some(embedded) = &composer.embedded else {
            continue;
        };
        let fallback_ts = composer.created_at.clone();
        let events = cursor_composer_message_events(embedded, &fallback_ts);
        if events.is_empty() {
            continue;
        }
        let conversation = conversations
            .entry(format!("composer:{composer_id}"))
            .or_default();
        conversation.title = Some(composer.name.clone()).filter(|name| !name.is_empty());
        conversation.composer_created_at = composer.created_at.clone();
        conversation.prompt_breakdown_total = composer.prompt_breakdown_total;
        conversation.embedded_events = events;
    }
    let mut sessions = Vec::new();
    for (key, conversation) in &conversations {
        if let Some(mut session) = cursor_session(conversation, path) {
            session.metrics.session_key = key.clone();
            sessions.push(session);
        }
    }
    (sessions, clean)
}

/// The composer id of a `bubbleId:<composerId>:<bubbleId>` key (the
/// segment before the first colon; a colon-less suffix is itself the
/// composer id).
fn composer_id_of(key_suffix: &str) -> String {
    match key_suffix.split_once(':') {
        Some((composer_id, _)) => composer_id.to_string(),
        None => key_suffix.to_string(),
    }
}

fn cursor_session(conversation: &CursorConversation, path: &Path) -> Option<Session> {
    let mut bubbles = conversation.bubbles.clone();
    bubbles.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.seq.cmp(&b.seq)));
    let mut events: Vec<Event> = Vec::new();
    let mut metas: Vec<Event> = Vec::new();
    for bubble in &bubbles {
        if !bubble.text.is_empty() {
            events.push(Event {
                role: bubble.role.to_string(),
                content: bubble.text.clone(),
                timestamp: bubble.created_at.clone(),
                model_used: if bubble.role == "assistant" {
                    bubble.model.clone()
                } else {
                    String::new()
                },
                source_tool: CURSOR_SOURCE.to_string(),
                ..Event::default()
            });
        }
        // Usage rides ONE meta event per usage block (rm-720 fold
        // discipline): lib.rs counts usage on meta rows, a recorded
        // cost prices exactly its own block's tokens, and assistant
        // conversation rows stay content-only so the
        // `usage_present_not_counted` skip channel never fires.
        if let Some(usage) = bubble.usage.as_ref().filter(|usage| !usage.is_empty()) {
            metas.push(Event {
                role: "session_meta".to_string(),
                timestamp: bubble.created_at.clone(),
                model_used: bubble.model.clone(),
                usage: usage.clone(),
                recorded_cost_usd: bubble.recorded_cost_usd,
                source_tool: CURSOR_SOURCE.to_string(),
                ..Event::default()
            });
        }
    }
    if metas.is_empty() {
        // ccusage #1771 fallback: no bubble carried tokenCount —
        // apply the composer's promptTokenBreakdown.totalUsedTokens
        // ONCE per conversation, disclosed by counter.
        if let Some(total) = conversation.prompt_breakdown_total {
            let timestamp = if conversation.composer_created_at.is_empty() {
                bubbles
                    .first()
                    .map(|bubble| bubble.created_at.clone())
                    .unwrap_or_default()
            } else {
                conversation.composer_created_at.clone()
            };
            let mut counters = BTreeMap::new();
            counters.insert("cursor_prompt_token_breakdown_fallback".to_string(), 1);
            let mut usage = BTreeMap::new();
            usage.insert("input_tokens".to_string(), total);
            metas.push(Event {
                role: "session_meta".to_string(),
                timestamp,
                usage,
                disclosure_counters: counters,
                source_tool: CURSOR_SOURCE.to_string(),
                ..Event::default()
            });
        }
    } else {
        // Basis disclosure: these totals are per-bubble tokenCount
        // blocks, not whole-session figures.
        let basis_blocks = metas.len() as i64;
        if let Some(first) = metas.first_mut() {
            first.disclosure_counters.insert(
                "cursor_usage_basis:per_bubble_token_count".to_string(),
                basis_blocks,
            );
        }
    }
    if events.is_empty() && metas.is_empty() && conversation.embedded_events.is_empty() {
        return None;
    }
    events.extend(conversation.embedded_events.iter().cloned());
    for meta in metas.into_iter().rev() {
        events.insert(0, meta);
    }
    let title = conversation.title.clone().unwrap_or_default();
    // An empty name lets session_from_events derive the display name
    // from the first user request — provenance stays honest instead
    // of claiming a title the store never carried.
    let mut session = session_from_events(&title, &path.to_string_lossy(), events).ok()?;
    if !title.is_empty() {
        session.name = title;
        session.metrics.provenance.naming = "provider_title".to_string();
    }
    Some(session)
}

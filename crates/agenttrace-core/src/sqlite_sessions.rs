use crate::{detect_anomalies, health_score, token_cost, Metrics, Session};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct RoleCounts {
    user: usize,
    assistant: usize,
    tool: usize,
}

#[derive(Debug, Default)]
struct SqliteSessionAgg {
    id: String,
    title: String,
    model: String,
    models: BTreeSet<String>,
    start_unix: f64,
    end_unix: f64,
    events: usize,
    user_messages: usize,
    assistant_turns: usize,
    tool_results: usize,
    tool_calls_total: usize,
    tool_calls_ok: usize,
    tool_calls_fail: usize,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
    message_tokens: usize,
    usage_cost: f64,
    usage_cost_set: bool,
    /// Authoritative per-session totals recorded on the upstream session
    /// row (opencode schema); present only when the columns exist and are
    /// non-null. When set they displace message-derived aggregation.
    stored_input: Option<i64>,
    stored_output: Option<i64>,
    stored_reasoning: Option<i64>,
    stored_cache_read: Option<i64>,
    stored_cache_write: Option<i64>,
    stored_cost: Option<f64>,
    stored_totals_applied: bool,
    stored_cost_applied: bool,
    /// stored total minus derived total (saturating), exposed so
    /// data_health can surface derived-aggregation drift.
    stored_totals_delta: i64,
    source_tool: String,
    path: String,
    cwd: String,
    /// Text of the first user message (opencode: from the `part` table),
    /// used for message-derived naming when the provider title is empty
    /// or a placeholder (research candidate 34).
    first_user_text: String,
}

pub fn load_sqlite_backed_sessions() -> Vec<Session> {
    load_sqlite_backed_sessions_since(None)
}

pub(crate) fn load_sqlite_backed_sessions_since(since: Option<DateTime<Utc>>) -> Vec<Session> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let mut sessions = Vec::new();
    for path in hermes_state_db_paths(&home) {
        sessions.extend(load_hermes_sqlite_sessions(&path, since));
    }
    for path in opencode_db_paths(&home) {
        sessions.extend(load_opencode_sqlite_sessions(&path, since));
    }
    sessions
}

pub fn skip_sqlite_backed_file_dir(dir: &Path) -> bool {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return false;
    };
    if hermes_state_db_paths(&home)
        .iter()
        .any(|path| sqlite_file_exists(path))
        && clean_path(dir) == clean_path(&home.join(".hermes").join("sessions"))
    {
        return true;
    }
    if opencode_db_paths(&home)
        .iter()
        .any(|path| sqlite_file_exists(path))
        && is_opencode_storage_root(dir)
    {
        return true;
    }
    false
}

fn hermes_state_db_path(home: &Path) -> PathBuf {
    home.join(".hermes").join("state.db")
}

fn hermes_state_db_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![hermes_state_db_path(home)];
    if let Ok(entries) = std::fs::read_dir(home.join(".hermes").join("profiles")) {
        paths.extend(
            entries
                .flatten()
                .map(|entry| entry.path().join("state.db"))
                .filter(|path| path.is_file()),
        );
    }
    paths
}

fn opencode_db_path(home: &Path) -> PathBuf {
    home.join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db")
}

fn opencode_db_paths(home: &Path) -> Vec<PathBuf> {
    let primary = opencode_db_path(home);
    let Some(dir) = primary.parent() else {
        return vec![primary];
    };
    let mut paths = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("opencode") && name.ends_with(".db"))
        })
        .collect::<Vec<_>>();
    if paths.is_empty() {
        paths.push(primary);
    }
    paths.sort();
    paths
}

fn sqlite_file_exists(path: &Path) -> bool {
    path.is_file()
}

fn open_sqlite_read_only(path: &Path) -> rusqlite::Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
}

fn load_hermes_sqlite_sessions(path: &Path, since: Option<DateTime<Utc>>) -> Vec<Session> {
    if !sqlite_file_exists(path) {
        return Vec::new();
    }
    if let Some(sessions) = crate::session_cache::load_sqlite_snapshot(path, "hermes") {
        return filter_since(sessions, since);
    }
    let sessions = query_hermes_sqlite_sessions(path, None);
    let _ = crate::session_cache::store_sqlite_snapshot(path, "hermes", &sessions);
    filter_since(sessions, since)
}

fn query_hermes_sqlite_sessions(path: &Path, since: Option<DateTime<Utc>>) -> Vec<Session> {
    let Ok(db) = open_sqlite_read_only(path) else {
        return Vec::new();
    };
    let roles = sqlite_role_counts(&db, "messages", "session_id", "role");
    let cwd = if sqlite_has_column(&db, "sessions", "cwd") {
        "cwd"
    } else {
        "''"
    };
    let sql = format!(
        "select id, model, started_at, ended_at, message_count, tool_call_count, \
         input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, {cwd} from sessions \
         where (?1 is null or started_at >= ?1 or started_at is null or started_at <= 0)"
    );
    let Ok(mut stmt) = db.prepare(&sql) else {
        return Vec::new();
    };
    let since_unix = since.map(|value| value.timestamp() as f64);
    let Ok(rows) = stmt.query_map([since_unix], |row| {
        Ok(SqliteSessionAgg {
            id: row.get::<_, String>(0)?,
            model: string_or(row.get::<_, Option<String>>(1)?, "default"),
            start_unix: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            end_unix: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            events: row.get::<_, Option<i64>>(4)?.unwrap_or(0).max(0) as usize,
            tool_calls_total: row.get::<_, Option<i64>>(5)?.unwrap_or(0).max(0) as usize,
            tool_calls_ok: row.get::<_, Option<i64>>(5)?.unwrap_or(0).max(0) as usize,
            input_tokens: row.get::<_, Option<i64>>(6)?.unwrap_or(0).max(0),
            output_tokens: row.get::<_, Option<i64>>(7)?.unwrap_or(0).max(0),
            cache_read_tokens: row.get::<_, Option<i64>>(8)?.unwrap_or(0).max(0),
            cache_write_tokens: row.get::<_, Option<i64>>(9)?.unwrap_or(0).max(0),
            cwd: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
            source_tool: "hermes_db".to_string(),
            path: path.to_string_lossy().to_string(),
            ..SqliteSessionAgg::default()
        })
    }) else {
        return Vec::new();
    };

    rows.filter_map(Result::ok)
        .map(|mut agg| {
            if !agg.model.is_empty() {
                agg.models.insert(agg.model.clone());
            }
            if let Some(counts) = roles.get(&agg.id) {
                agg.user_messages = counts.user;
                agg.assistant_turns = counts.assistant;
                agg.tool_results = counts.tool;
            }
            session_from_sqlite_agg(agg)
        })
        .collect()
}

fn load_opencode_sqlite_sessions(path: &Path, since: Option<DateTime<Utc>>) -> Vec<Session> {
    if !sqlite_file_exists(path) {
        return Vec::new();
    }
    if let Some(sessions) = crate::session_cache::load_sqlite_snapshot(path, "opencode") {
        return filter_since(sessions, since);
    }
    let sessions = query_opencode_sqlite_sessions(path, None);
    let _ = crate::session_cache::store_sqlite_snapshot(path, "opencode", &sessions);
    filter_since(sessions, since)
}

fn query_opencode_sqlite_sessions(path: &Path, since: Option<DateTime<Utc>>) -> Vec<Session> {
    let Ok(db) = open_sqlite_read_only(path) else {
        return Vec::new();
    };
    let mut aggs = opencode_sqlite_session_rows(&db, path, since);
    if aggs.is_empty() {
        return Vec::new();
    }
    add_opencode_sqlite_messages(&db, &mut aggs);
    add_opencode_sqlite_parts(&db, &mut aggs);
    capture_opencode_user_text(&db, &mut aggs);

    aggs.into_values()
        .map(|mut agg| {
            if agg.model.is_empty() {
                agg.model = "default".to_string();
            }
            if agg.events == 0 {
                agg.events = agg.user_messages + agg.assistant_turns + agg.tool_calls_total;
            }
            apply_opencode_stored_totals(&mut agg);
            session_from_sqlite_agg(agg)
        })
        .collect()
}

/// Prefer the authoritative totals recorded on the session row over
/// message-derived aggregation (candidate 8, totals scope): stored
/// values displace the derived ones, the provenance discloses it, and
/// the delta between the two is exposed for data_health reporting.
/// Corrupt stored values (negatives, non-finite cost) are clamped away
/// rather than trusted blindly.
fn apply_opencode_stored_totals(agg: &mut SqliteSessionAgg) {
    let has_stored_tokens = agg.stored_input.is_some()
        || agg.stored_output.is_some()
        || agg.stored_reasoning.is_some()
        || agg.stored_cache_read.is_some()
        || agg.stored_cache_write.is_some();
    if !has_stored_tokens {
        return;
    }
    let derived_total = [
        agg.input_tokens,
        agg.output_tokens,
        agg.cache_read_tokens,
        agg.cache_write_tokens,
    ]
    .iter()
    .fold(0i64, |acc, value| acc.saturating_add(*value));
    let stored_input = agg.stored_input.unwrap_or(0).max(0);
    let stored_output = agg
        .stored_output
        .unwrap_or(0)
        .max(0)
        .saturating_add(agg.stored_reasoning.unwrap_or(0).max(0));
    let stored_cache_read = agg.stored_cache_read.unwrap_or(0).max(0);
    let stored_cache_write = agg.stored_cache_write.unwrap_or(0).max(0);
    let stored_total = [
        stored_input,
        stored_output,
        stored_cache_read,
        stored_cache_write,
    ]
    .iter()
    .fold(0i64, |acc, value| acc.saturating_add(*value));
    agg.stored_totals_delta = stored_total.saturating_sub(derived_total);
    agg.input_tokens = stored_input;
    agg.output_tokens = stored_output;
    agg.cache_read_tokens = stored_cache_read;
    agg.cache_write_tokens = stored_cache_write;
    agg.stored_totals_applied = true;
    if let Some(cost) = agg
        .stored_cost
        .filter(|cost| cost.is_finite() && *cost >= 0.0)
    {
        agg.usage_cost = cost;
        agg.usage_cost_set = true;
        agg.stored_cost_applied = true;
    }
}

fn filter_since(sessions: Vec<Session>, since: Option<DateTime<Utc>>) -> Vec<Session> {
    sessions
        .into_iter()
        .filter(|session| session_within_since(session, since))
        .collect()
}

/// A session whose start time is unknown (empty or unparseable) stays in
/// the unknown-time bucket instead of being silently dropped from every
/// time-ranged view (N7); only sessions with a known start before the
/// cutoff are filtered out.
fn session_within_since(session: &Session, since: Option<DateTime<Utc>>) -> bool {
    since.map_or(true, |since| {
        DateTime::parse_from_rfc3339(&session.metrics.session_start)
            .map(|time| time.with_timezone(&Utc) >= since)
            .unwrap_or(true)
    })
}

fn opencode_sqlite_session_rows(
    db: &Connection,
    path: &Path,
    since: Option<DateTime<Utc>>,
) -> HashMap<String, SqliteSessionAgg> {
    let directory = if sqlite_has_column(db, "session", "directory") {
        "directory"
    } else {
        "''"
    };
    // Authoritative per-session totals (upstream schema): present columns
    // are selected directly, missing ones become null so the row indices
    // stay stable across schemas.
    let stored_columns = [
        ("cost", sqlite_has_column(db, "session", "cost")),
        (
            "tokens_input",
            sqlite_has_column(db, "session", "tokens_input"),
        ),
        (
            "tokens_output",
            sqlite_has_column(db, "session", "tokens_output"),
        ),
        (
            "tokens_reasoning",
            sqlite_has_column(db, "session", "tokens_reasoning"),
        ),
        (
            "tokens_cache_read",
            sqlite_has_column(db, "session", "tokens_cache_read"),
        ),
        (
            "tokens_cache_write",
            sqlite_has_column(db, "session", "tokens_cache_write"),
        ),
    ];
    let stored_select = stored_columns
        .iter()
        .map(|(column, present)| {
            if *present {
                (*column).to_string()
            } else {
                "null".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "select id, title, time_created, time_updated, {directory}, {stored_select} from session \
         where (?1 is null or time_created >= ?1 or time_created is null or time_created <= 0)"
    );
    let Ok(mut stmt) = db.prepare(&sql) else {
        return HashMap::new();
    };
    let since_millis = since.map(|value| value.timestamp_millis());
    let Ok(rows) = stmt.query_map([since_millis], |row| {
        let id = row.get::<_, String>(0)?;
        Ok((
            id.clone(),
            SqliteSessionAgg {
                id,
                title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                // Lenient reads: SQLite columns are dynamically typed, so a
                // corrupt TEXT/REAL value must degrade (unknown time,
                // stored-total fallback) instead of failing the row and
                // silently dropping the whole session.
                start_unix: sqlite_value_as_i64(row.get(2)?).unwrap_or(0).max(0) as f64 / 1000.0,
                end_unix: sqlite_value_as_i64(row.get(3)?).unwrap_or(0).max(0) as f64 / 1000.0,
                cwd: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                stored_cost: sqlite_value_as_f64(row.get(5)?),
                stored_input: sqlite_value_as_i64(row.get(6)?),
                stored_output: sqlite_value_as_i64(row.get(7)?),
                stored_reasoning: sqlite_value_as_i64(row.get(8)?),
                stored_cache_read: sqlite_value_as_i64(row.get(9)?),
                stored_cache_write: sqlite_value_as_i64(row.get(10)?),
                source_tool: "opencode_db".to_string(),
                path: path.to_string_lossy().to_string(),
                ..SqliteSessionAgg::default()
            },
        ))
    }) else {
        return HashMap::new();
    };
    rows.filter_map(Result::ok).collect()
}

fn add_opencode_sqlite_messages(db: &Connection, aggs: &mut HashMap<String, SqliteSessionAgg>) {
    let Ok(mut stmt) = db.prepare("select session_id, data from message") else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return;
    };
    for (session_id, raw) in rows.filter_map(Result::ok) {
        let Some(agg) = aggs.get_mut(&session_id) else {
            continue;
        };
        let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        agg.events += 1;
        match string(doc.get("role")) {
            "user" => agg.user_messages += 1,
            "assistant" => agg.assistant_turns += 1,
            "tool" => agg.tool_results += 1,
            _ => {}
        }
        let model = opencode_sqlite_message_model(&doc);
        if !model.is_empty() {
            agg.models.insert(model.clone());
            agg.model = model;
        }
        if add_opencode_sqlite_message_tokens(agg, &doc) {
            agg.message_tokens += 1;
        }
    }
}

/// Captures the earliest user-message text per session for
/// message-derived naming (research candidate 34): opencode stores user
/// prose in `part` rows of type `text`, not on the `message` row itself,
/// so the join recovers the prompt behind placeholder titles like
/// `New session - <timestamp>`.
fn capture_opencode_user_text(db: &Connection, aggs: &mut HashMap<String, SqliteSessionAgg>) {
    let Ok(mut stmt) = db.prepare(
        "select p.session_id, p.data, m.data from part p \
         join message m on p.message_id = m.id order by p.time_created",
    ) else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    }) else {
        return;
    };
    for (session_id, part_raw, message_raw) in rows.filter_map(Result::ok) {
        let Some(agg) = aggs.get_mut(&session_id) else {
            continue;
        };
        if !agg.first_user_text.is_empty() {
            continue;
        }
        let Ok(serde_json::Value::Object(message)) = serde_json::from_str::<Value>(&message_raw)
        else {
            continue;
        };
        if string(message.get("role")) != "user" {
            continue;
        }
        let Ok(serde_json::Value::Object(part)) = serde_json::from_str::<Value>(&part_raw) else {
            continue;
        };
        if string(part.get("type")) != "text" {
            continue;
        }
        let text = string(part.get("text"));
        if !text.trim().is_empty() {
            agg.first_user_text = text.to_string();
        }
    }
}

fn add_opencode_sqlite_parts(db: &Connection, aggs: &mut HashMap<String, SqliteSessionAgg>) {
    let Ok(mut stmt) = db.prepare("select session_id, data from part") else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return;
    };
    for (session_id, raw) in rows.filter_map(Result::ok) {
        let Some(agg) = aggs.get_mut(&session_id) else {
            continue;
        };
        let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        match string(doc.get("type")) {
            "step-finish" => {
                if agg.message_tokens == 0 {
                    add_opencode_step_finish_tokens(agg, &doc);
                }
            }
            "tool" => {
                agg.tool_calls_total += 1;
                if opencode_tool_failed(&doc) {
                    agg.tool_calls_fail += 1;
                } else {
                    agg.tool_calls_ok += 1;
                }
            }
            _ => {}
        }
    }
}

fn add_opencode_sqlite_message_tokens(
    agg: &mut SqliteSessionAgg,
    doc: &serde_json::Map<String, Value>,
) -> bool {
    let Some(tokens) = doc.get("tokens").and_then(Value::as_object) else {
        return false;
    };
    let (input, output, cache_read, cache_write) = add_opencode_tokens_from_map(agg, tokens);
    let mut model = opencode_sqlite_message_model(doc);
    if model.is_empty() {
        model = agg.model.clone();
    }
    if !model.is_empty() {
        agg.usage_cost += token_cost_raw(input, output, cache_write, cache_read, &model);
        agg.usage_cost_set = true;
    }
    true
}

fn add_opencode_step_finish_tokens(
    agg: &mut SqliteSessionAgg,
    doc: &serde_json::Map<String, Value>,
) {
    let Some(tokens) = doc.get("tokens").and_then(Value::as_object) else {
        return;
    };
    let (input, output, cache_read, cache_write) = add_opencode_tokens_from_map(agg, tokens);
    if !agg.model.is_empty() {
        agg.usage_cost += token_cost_raw(input, output, cache_write, cache_read, &agg.model);
        agg.usage_cost_set = true;
    }
}

fn add_opencode_tokens_from_map(
    agg: &mut SqliteSessionAgg,
    tokens: &serde_json::Map<String, Value>,
) -> (i64, i64, i64, i64) {
    let cache = tokens.get("cache").and_then(Value::as_object);
    let input = number_as_i64(tokens.get("input"));
    let output =
        number_as_i64(tokens.get("output")).saturating_add(number_as_i64(tokens.get("reasoning")));
    let cache_read = cache
        .map(|cache| number_as_i64(cache.get("read")))
        .unwrap_or(0);
    let cache_write = cache
        .map(|cache| number_as_i64(cache.get("write")))
        .unwrap_or(0);
    // Saturate instead of overflowing: two adversarial i64::MAX token
    // fields previously overflowed here in debug (exit 101) and wrapped
    // negative in release.
    agg.input_tokens = agg.input_tokens.saturating_add(input);
    agg.output_tokens = agg.output_tokens.saturating_add(output);
    agg.cache_read_tokens = agg.cache_read_tokens.saturating_add(cache_read);
    agg.cache_write_tokens = agg.cache_write_tokens.saturating_add(cache_write);
    (input, output, cache_read, cache_write)
}

fn opencode_tool_failed(doc: &serde_json::Map<String, Value>) -> bool {
    doc.get("state")
        .and_then(Value::as_object)
        .map(|state| string(state.get("status")).to_ascii_lowercase())
        .map(|status| matches!(status.as_str(), "error" | "failed" | "cancelled"))
        .unwrap_or(false)
}

fn opencode_sqlite_message_model(doc: &serde_json::Map<String, Value>) -> String {
    let direct = string(doc.get("modelID"));
    if !direct.is_empty() {
        return direct.to_string();
    }
    doc.get("model")
        .and_then(Value::as_object)
        .map(|model| string(model.get("modelID")).to_string())
        .unwrap_or_default()
}

fn sqlite_role_counts(
    db: &Connection,
    table: &str,
    session_column: &str,
    role_column: &str,
) -> HashMap<String, RoleCounts> {
    let sql = format!(
        "select {session_column}, {role_column}, count(*) from {table} group by {session_column}, {role_column}"
    );
    let Ok(mut stmt) = db.prepare(&sql) else {
        return HashMap::new();
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    }) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for (session_id, role, count) in rows.filter_map(Result::ok) {
        let entry = out.entry(session_id).or_insert_with(RoleCounts::default);
        match role.as_str() {
            "user" => entry.user = count.max(0) as usize,
            "assistant" => entry.assistant = count.max(0) as usize,
            "tool" => entry.tool = count.max(0) as usize,
            _ => {}
        }
    }
    out
}

fn session_from_sqlite_agg(agg: SqliteSessionAgg) -> Session {
    let mut models = agg.models;
    if models.is_empty() && !agg.model.is_empty() {
        models.insert(agg.model.clone());
    }
    let multiple_models = models.len() > 1;
    let model = if multiple_models {
        "multiple".to_string()
    } else if agg.model.is_empty() {
        "default".to_string()
    } else {
        agg.model
    };
    let mut cost_estimated = token_cost(
        agg.input_tokens,
        agg.output_tokens,
        agg.cache_write_tokens,
        agg.cache_read_tokens,
        &model,
    );
    if agg.usage_cost_set {
        cost_estimated = crate::round4(agg.usage_cost);
    }
    let pricing_source = if multiple_models {
        "SQLite aggregate: multiple models".to_string()
    } else {
        crate::pricing::pricing_source_for(&model)
    };
    let mut metrics = Metrics {
        events_total: agg.events,
        user_messages: agg.user_messages,
        assistant_turns: agg.assistant_turns,
        tool_results: agg.tool_results,
        tool_calls_total: agg.tool_calls_total,
        tool_calls_ok: agg.tool_calls_ok,
        tool_calls_fail: agg.tool_calls_fail,
        tokens_input: agg.input_tokens,
        tokens_output: agg.output_tokens,
        tokens_cache_w: agg.cache_write_tokens,
        tokens_cache_r: agg.cache_read_tokens,
        model_used: model,
        source_tool: agg.source_tool,
        session_start: unix_seconds_rfc3339(agg.start_unix),
        session_end: unix_seconds_rfc3339(agg.end_unix),
        cost_estimated,
        stored_totals_delta: agg.stored_totals_delta,
        provenance: crate::MetricProvenance {
            naming: String::new(),
            tokens: if agg.stored_totals_applied {
                "stored_session_totals".to_string()
            } else {
                "reported_by_agent".to_string()
            },
            duration: "unavailable".to_string(),
            tool_results: if agg.tool_results > 0 {
                "reported_by_agent".to_string()
            } else {
                "unavailable".to_string()
            },
            files: "unavailable".to_string(),
            cost: if agg.stored_cost_applied {
                "reported_by_agent".to_string()
            } else if agg.usage_cost_set {
                "calculated_per_message_tokens".to_string()
            } else {
                "calculated_from_tokens".to_string()
            },
            pricing_source,
        },
        ..Metrics::default()
    };
    if agg.end_unix > agg.start_unix {
        metrics.duration_sec = agg.end_unix - agg.start_unix;
        metrics.provenance.duration = "timestamp_span".to_string();
    }
    let anomalies = detect_anomalies(&metrics);
    // Research candidate 34: OpenCode fills `title` with a
    // `New session - <timestamp>` placeholder on every session it does
    // not summarize, and that placeholder must not become the session
    // name. Placeholder titles are treated as absent so message-derived
    // naming wins, and the naming provenance discloses the gate.
    let placeholder_title = agg.title.starts_with("New session - ");
    let derived_name = if placeholder_title || agg.title.is_empty() {
        crate::display_title_from_text(&agg.first_user_text)
    } else {
        None
    };
    let name = match &derived_name {
        Some(derived) => derived.clone(),
        None if placeholder_title || agg.title.is_empty() => agg.id.clone(),
        None => agg.title.clone(),
    };
    metrics.provenance.naming = if placeholder_title {
        "provider:placeholder"
    } else if !agg.title.is_empty() {
        "provider_title"
    } else if derived_name.is_some() {
        "message_derived"
    } else {
        "session_id"
    }
    .to_string();
    let health = health_score(&anomalies);
    Session {
        name,
        path: agg.path,
        cwd: agg.cwd,
        metrics,
        anomalies,
        health,
        tool_warnings: Vec::new(),
        diagnostics: crate::Diagnostics::default(),
    }
}

fn token_cost_raw(input: i64, output: i64, cache_write: i64, cache_read: i64, model: &str) -> f64 {
    token_cost(input, output, cache_write, cache_read, model)
}

fn unix_seconds_rfc3339(value: f64) -> String {
    if value <= 0.0 {
        return String::new();
    }
    let secs = value as i64;
    let nsecs = ((value - secs as f64) * 1e9) as u32;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nsecs)
        .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

fn string_or(value: Option<String>, fallback: &str) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn string(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or("")
}

fn sqlite_has_column(db: &Connection, table: &str, column: &str) -> bool {
    db.prepare(&format!("pragma table_info({table})"))
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(1))
                .map(|rows| rows.filter_map(Result::ok).any(|name| name == column))
        })
        .unwrap_or(false)
}

/// SQLite columns are dynamically typed: a value written as TEXT ("999")
/// or REAL must still yield a usable integer instead of failing the row
/// conversion — a failed `get::<Option<i64>>` previously dropped the
/// entire session silently. Unparseable values become None so callers
/// fall back (derived tokens, unknown time) rather than disappearing.
fn sqlite_value_as_i64(value: Option<rusqlite::types::Value>) -> Option<i64> {
    use rusqlite::types::Value as SqliteValue;
    match value {
        Some(SqliteValue::Integer(number)) => Some(number),
        Some(SqliteValue::Real(number)) => Some(number as i64),
        Some(SqliteValue::Text(text)) => text.parse::<i64>().ok(),
        _ => None,
    }
}

fn sqlite_value_as_f64(value: Option<rusqlite::types::Value>) -> Option<f64> {
    use rusqlite::types::Value as SqliteValue;
    match value {
        Some(SqliteValue::Integer(number)) => Some(number as f64),
        Some(SqliteValue::Real(number)) => Some(number),
        Some(SqliteValue::Text(text)) => text.parse::<f64>().ok(),
        _ => None,
    }
}

fn number_as_i64(value: Option<&Value>) -> i64 {
    // Reuse the hardened parser converter: u64 values above i64::MAX
    // previously wrapped negative through `n as i64` here (P5-2
    // reported `"input": -1` for a u64::MAX token count); strings that
    // parse as integers are accepted, everything else is 0.
    value.and_then(crate::parser::number_as_i64).unwrap_or(0)
}

fn is_opencode_storage_root(path: &Path) -> bool {
    path.to_string_lossy()
        .replace('\\', "/")
        .ends_with("/opencode/storage")
}

fn clean_path(path: &Path) -> PathBuf {
    path.components().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_session_preserves_workspace() {
        let session = session_from_sqlite_agg(SqliteSessionAgg {
            id: "session".to_string(),
            cwd: "/work/sqlite".to_string(),
            ..SqliteSessionAgg::default()
        });
        assert_eq!(session.cwd, "/work/sqlite");
    }

    #[test]
    fn sqlite_multi_model_aggregate_is_not_exactly_priced_as_one_model() {
        let session = session_from_sqlite_agg(SqliteSessionAgg {
            model: "gpt-5".to_string(),
            models: BTreeSet::from(["gpt-5".to_string(), "claude-sonnet-4".to_string()]),
            usage_cost: 1.25,
            usage_cost_set: true,
            ..SqliteSessionAgg::default()
        });
        assert_eq!(session.metrics.model_used, "multiple");
        assert_eq!(
            session.metrics.provenance.pricing_source,
            "SQLite aggregate: multiple models"
        );
        assert_eq!(session.metrics.cost_estimated, 1.25);
    }
}

//! Claude Science (beta) corpus lane (rm-1031): reads per-frame token
//! usage from the Claude Science desktop app's metadata databases.
//!
//! Upstream shape (mirrored from ccusage PR #1703, closed unmerged
//! 2026-09-14 after full review engagement — agenttrace implements the
//! same SELECT-only read against the documented on-disk schema):
//!
//! - Storage roots: `~/.claude-science`, `~/.config/claude-science` /
//!   `~/.config/Claude Science`, `~/.local/share/claude-science` /
//!   `~/.local/share/Claude Science`, and the macOS
//!   `~/Library/Application Support/Claude Science`; each holds a
//!   `metadata.db`. The switch-proxy daemon additionally keeps one
//!   `operon-cli.db` per org under
//!   `~/.claude-science/cs-switch-proxy/orgs/<org>/`.
//! - `CLAUDE_SCIENCE_DB` (rm-947 env-override convention) points at an
//!   explicit database file and replaces the default root set.
//! - Recognition is column-gated: a database counts only when it has a
//!   `frames` table and the loader's SELECT prepares against it. A
//!   `frames` table that exists but has drifted columns is disclosed
//!   as a dropped-row entry (never a silent empty corpus); a database
//!   without `frames` is legitimately foreign and yields zero rows.
//! - Frames→sessions mapping: one session per
//!   `COALESCE(root_frame_id, id)` — sub-agent frames carry their
//!   root's id and roll up into the root's session. Timestamps are
//!   epoch milliseconds in `updated_at`.
//! - The connection is opened `SQLITE_OPEN_READ_ONLY` and only SELECT
//!   statements run: this lane never writes, never journals, and never
//!   creates a database file.
//! - `frames.total_cost` is READ but deliberately not consumed — the
//!   recorded-cost arm is deferred to the rm-253 recorded-vs-estimate
//!   family (native cost columns win when that lands). Sessions whose
//!   frames carry a non-NULL recorded cost disclose the deferral via
//!   the `claude_science_recorded_cost_deferred` counter so the corpus
//!   stays honest about what it estimated instead.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};

use crate::{detect_anomalies, health_score, token_cost, Metrics, Session};

use crate::sqlite_sessions::{
    sqlite_file_exists, sqlite_has_column, unix_seconds_rfc3339, SqliteFileFailures,
    SqliteIngestReport,
};

/// rm-947-convention env override: an explicit database file (or a
/// directory holding `metadata.db`) that replaces the default roots.
pub const CLAUDE_SCIENCE_DB_ENV: &str = "CLAUDE_SCIENCE_DB";

/// `source_tool` for this lane (the `*_db` convention of the other
/// SQLite lanes: `hermes_db`, `opencode_db`).
pub const CLAUDE_SCIENCE_SOURCE_TOOL: &str = "claude_science_db";

/// The daemon's per-org database layout under the primary root.
const ORG_DB_LAYOUT: [&str; 3] = ["cs-switch-proxy", "orgs", "operon-cli.db"];

/// Known storage roots (display names + paths), in probe order. The
/// env override, when set, replaces the whole set with one root.
fn claude_science_roots(home: &Path) -> Vec<(String, PathBuf)> {
    if let Some(explicit) = std::env::var_os(CLAUDE_SCIENCE_DB_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return vec![(explicit.to_string_lossy().to_string(), explicit)];
    }
    let config = user_config_dir(home);
    let data = xdg_data_dir(home);
    vec![
        ("Claude Science".to_string(), home.join(".claude-science")),
        (
            "Claude Science config".to_string(),
            config.join("claude-science"),
        ),
        (
            "Claude Science config (Claude Science)".to_string(),
            config.join("Claude Science"),
        ),
        (
            "Claude Science share".to_string(),
            data.join("claude-science"),
        ),
        (
            "Claude Science share (Claude Science)".to_string(),
            data.join("Claude Science"),
        ),
    ]
}

fn user_config_dir(home: &Path) -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        if !dir.as_os_str().is_empty() {
            return dir;
        }
    }
    if cfg!(target_os = "macos") {
        return home.join("Library").join("Application Support");
    }
    home.join(".config")
}

fn xdg_data_dir(home: &Path) -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        if !dir.as_os_str().is_empty() {
            return dir;
        }
    }
    home.join(".local").join("share")
}

/// Discovery registry entries for the Claude Science homes (rm-1031
/// acceptance a): each root that exists on disk becomes one
/// KnownSessionDir, the same pattern as the rm-947 sibling set. The
/// roots hold SQLite databases, not transcript files — the file walk
/// is skipped over them when a database is present (see
/// `sqlite_sessions::skip_sqlite_backed_file_dir`) and the corpus is
/// loaded by this module's SQLite lane instead.
pub fn claude_science_known_session_dirs(home: &Path) -> Vec<crate::KnownSessionDir> {
    claude_science_roots(home)
        .into_iter()
        .filter(|(_, path)| path.is_dir())
        .map(|(name, path)| crate::KnownSessionDir { name, path })
        .collect()
}

/// True when `dir` is one of the Claude Science homes — used to keep
/// the file walk out of SQLite-backed territory.
pub fn is_claude_science_root(dir: &Path, home: &Path) -> bool {
    claude_science_roots(home)
        .iter()
        .any(|(_, root)| root == dir)
}

/// Candidate database files, canonical root order first, then sorted
/// org databases (primary-first, matching the dedup-aware union's
/// canonical-before-sibling expectation in `sqlite_sessions`).
pub fn claude_science_db_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for (_, root) in claude_science_roots(home) {
        // An env override may point at the database file itself; the
        // default roots hold it under `metadata.db`.
        if root.is_file() {
            paths.push(root);
            continue;
        }
        let db = root.join("metadata.db");
        if sqlite_file_exists(&db) {
            paths.push(db);
        }
    }
    let mut org_dbs = org_db_paths(&home.join(".claude-science"));
    org_dbs.sort();
    paths.extend(org_dbs);
    paths
}

fn org_db_paths(primary_root: &Path) -> Vec<PathBuf> {
    let orgs = primary_root.join(ORG_DB_LAYOUT[0]).join(ORG_DB_LAYOUT[1]);
    let Ok(entries) = std::fs::read_dir(&orgs) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for entry in entries.flatten() {
        let db = entry.path().join(ORG_DB_LAYOUT[2]);
        if sqlite_file_exists(&db) {
            paths.push(db);
        }
    }
    paths
}

/// The database the doctor's directory census names for this lane
/// (the canonical home's `metadata.db`, whatever the host layout).
pub fn claude_science_primary_db(home: &Path) -> PathBuf {
    home.join(".claude-science").join("metadata.db")
}

/// One frame row as loaded from `frames`.
struct FrameRow {
    session_id: String,
    model: String,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
    /// Present-but-unconsumed (rm-253 deferral); only its presence is
    /// recorded, through the disclosure counter.
    has_recorded_cost: bool,
    updated_at_ms: i64,
    project: Option<String>,
}

/// Per-session aggregate across the frames of one root id.
struct FrameSession {
    models: BTreeSet<String>,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
    frames: usize,
    recorded_cost_frames: usize,
    first_ms: i64,
    last_ms: i64,
    project: Option<String>,
}

impl FrameSession {
    fn new() -> Self {
        Self {
            models: BTreeSet::new(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            frames: 0,
            recorded_cost_frames: 0,
            first_ms: i64::MAX,
            last_ms: i64::MIN,
            project: None,
        }
    }

    fn absorb(&mut self, row: FrameRow) {
        if !row.model.is_empty() {
            self.models.insert(row.model);
        }
        self.input_tokens = self.input_tokens.saturating_add(row.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(row.output_tokens);
        self.cache_read_tokens = self.cache_read_tokens.saturating_add(row.cache_read_tokens);
        self.cache_write_tokens = self
            .cache_write_tokens
            .saturating_add(row.cache_write_tokens);
        self.frames += 1;
        if row.has_recorded_cost {
            self.recorded_cost_frames += 1;
        }
        if row.updated_at_ms > 0 {
            self.first_ms = self.first_ms.min(row.updated_at_ms);
            self.last_ms = self.last_ms.max(row.updated_at_ms);
        }
        if self.project.is_none() {
            self.project = row.project;
        }
    }
}

/// Load every Claude Science database under `home`. Failures absorb
/// into the shared ingest report so the doctor discloses them beside
/// the hermes/opencode lanes.
pub fn load_claude_science_sessions_reported(
    home: &Path,
    since: Option<DateTime<Utc>>,
    report: &mut SqliteIngestReport,
) -> Vec<Session> {
    let mut sessions = Vec::new();
    for path in claude_science_db_paths(home) {
        let loaded = load_claude_science_db(&path, since, report);
        sessions.extend(loaded);
    }
    sessions
}

fn load_claude_science_db(
    path: &Path,
    since: Option<DateTime<Utc>>,
    report: &mut SqliteIngestReport,
) -> Vec<Session> {
    let mut failures = SqliteFileFailures::default();
    let sessions = query_claude_science_db(path, &mut failures);
    report.absorb(path, CLAUDE_SCIENCE_SOURCE_TOOL, failures);
    sessions
        .into_iter()
        .filter(|session| crate::insights::session_admitted_since(session, since))
        .collect()
}

fn query_claude_science_db(path: &Path, failures: &mut SqliteFileFailures) -> Vec<Session> {
    let db = match open_read_only(path) {
        Ok(db) => db,
        // rm-753 discipline: found-but-unopenable (permissions, lock
        // policy, corrupt bytes) is disclosed, not nulled.
        Err(error) => {
            failures.unreadable = Some(error.to_string());
            return Vec::new();
        }
    };
    if !has_frames_table(&db) {
        // A database without `frames` is legitimately foreign — the
        // same precedent as the hermes lane's no-`sessions` case.
        return Vec::new();
    }
    // Column-gated recognition: the loader SELECT itself is the gate.
    // A `frames` table that exists but has drifted columns fails the
    // probe; that is schema drift over a real Claude Science database
    // and is disclosed as a dropped row rather than skipped silently.
    if let Err(error) = db.prepare(&frames_select(&db, false)) {
        failures.dropped += 1;
        failures.dropped_sample = Some(format!(
            "frames table present but loader columns missing (schema drift): {error}"
        ));
        return Vec::new();
    }
    let sql = frames_select(&db, true);
    let mut stmt = match db.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(error) => {
            failures.unreadable = Some(error.to_string());
            return Vec::new();
        }
    };
    let columns = frames_select_columns(&db);
    let rows = match stmt.query_map([], |row| {
        Ok(FrameRow {
            session_id: row.get::<_, String>(columns.session_id)?,
            model: row
                .get::<_, Option<String>>(columns.model)?
                .unwrap_or_default(),
            input_tokens: row
                .get::<_, Option<i64>>(columns.input)?
                .unwrap_or(0)
                .max(0),
            output_tokens: row
                .get::<_, Option<i64>>(columns.output)?
                .unwrap_or(0)
                .max(0),
            cache_read_tokens: row
                .get::<_, Option<i64>>(columns.cache_read)?
                .unwrap_or(0)
                .max(0),
            cache_write_tokens: row
                .get::<_, Option<i64>>(columns.cache_write)?
                .unwrap_or(0)
                .max(0),
            has_recorded_cost: row.get::<_, Option<f64>>(columns.total_cost)?.is_some(),
            updated_at_ms: row.get::<_, Option<i64>>(columns.updated_at)?.unwrap_or(0),
            project: columns
                .project
                .and_then(|index| row.get::<_, Option<String>>(index).ok().flatten()),
        })
    }) {
        Ok(rows) => rows,
        Err(error) => {
            failures.unreadable = Some(error.to_string());
            return Vec::new();
        }
    };
    // rm-753 discipline: a row that fails to decode is counted and
    // sampled, never dropped silently.
    let mut by_session: BTreeMap<String, FrameSession> = BTreeMap::new();
    for row in rows {
        match row {
            Ok(frame) => {
                // The reference lane skips frames without token usage
                // (they are not usage-bearing turns).
                if frame.input_tokens == 0 && frame.output_tokens == 0 {
                    continue;
                }
                let session = by_session
                    .entry(frame.session_id.clone())
                    .or_insert_with(FrameSession::new);
                session.absorb(frame);
            }
            Err(error) => failures.record_dropped(&error),
        }
    }
    by_session
        .into_iter()
        .map(|(session_id, agg)| session_from_frames(path, session_id, agg))
        .collect()
}

/// Result columns of the loader SELECT, resolved from the gate probe.
#[derive(Clone, Copy)]
struct FramesSelectColumns {
    session_id: usize,
    model: usize,
    input: usize,
    output: usize,
    cache_read: usize,
    cache_write: usize,
    total_cost: usize,
    updated_at: usize,
    project: Option<usize>,
}

fn has_frames_table(db: &Connection) -> bool {
    let Ok(mut stmt) =
        db.prepare("select 1 from sqlite_master where type = 'table' and name = 'frames'")
    else {
        return false;
    };
    stmt.exists([]).unwrap_or(false)
}

/// The loader SELECT. Optional columns degrade to literals when the
/// schema predates them, so a missing `cache_*_tokens` or `model`
/// column yields zeros/`multiple`-free aggregates instead of a failed
/// query; `id`, `input_tokens`, `output_tokens` and `updated_at` are
/// required and gate recognition (via the prepare probe).
fn frames_select(db: &Connection, with_project: bool) -> String {
    let root = if sqlite_has_column(db, "frames", "root_frame_id") {
        "coalesce(frames.root_frame_id, frames.id)"
    } else {
        "frames.id"
    };
    let model = if sqlite_has_column(db, "frames", "model") {
        "frames.model"
    } else {
        "''"
    };
    let cache_read = if sqlite_has_column(db, "frames", "cache_read_tokens") {
        "frames.cache_read_tokens"
    } else {
        "0"
    };
    let cache_write = if sqlite_has_column(db, "frames", "cache_write_tokens") {
        "frames.cache_write_tokens"
    } else {
        "0"
    };
    let total_cost = if sqlite_has_column(db, "frames", "total_cost") {
        "frames.total_cost"
    } else {
        "null"
    };
    let (project_column, join) = if with_project && projects_table_usable(db) {
        (
            ", projects.name",
            " left join projects on projects.id = frames.project_id",
        )
    } else {
        (", null", "")
    };
    // The WHERE mirrors the reference loader: frames without token
    // usage are not usage-bearing turns.
    format!(
        "select frames.id, {root} as session_id, {model}, frames.input_tokens, \
         frames.output_tokens, {cache_read}, {cache_write}, {total_cost}, \
         frames.updated_at{project_column} from frames{join} \
         where frames.input_tokens is not null and frames.output_tokens is not null"
    )
}

fn projects_table_usable(db: &Connection) -> bool {
    sqlite_has_column(db, "frames", "project_id")
        && db.prepare("select id, name from projects limit 1").is_ok()
}

fn frames_select_columns(db: &Connection) -> FramesSelectColumns {
    FramesSelectColumns {
        session_id: 1,
        model: 2,
        input: 3,
        output: 4,
        cache_read: 5,
        cache_write: 6,
        total_cost: 7,
        updated_at: 8,
        project: if projects_table_usable(db) {
            Some(9)
        } else {
            None
        },
    }
}

fn open_read_only(path: &Path) -> rusqlite::Result<Connection> {
    // rm-626 discipline: a short busy timeout lets the app's writer
    // release before the read fails as busy.
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(std::time::Duration::from_millis(250))?;
    Ok(connection)
}

fn session_from_frames(path: &Path, session_id: String, agg: FrameSession) -> Session {
    let multiple_models = agg.models.len() > 1;
    let model = if multiple_models {
        "multiple".to_string()
    } else {
        agg.models
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string())
    };
    let cost_estimated = token_cost(
        agg.input_tokens,
        agg.output_tokens,
        agg.cache_write_tokens,
        agg.cache_read_tokens,
        &model,
        // rm-610 precedent: aggregate sqlite rows carry no per-turn
        // service tier; estimates stay at standard rates.
        None,
    );
    let mut metrics = Metrics {
        events_total: agg.frames,
        tokens_input: agg.input_tokens,
        tokens_output: agg.output_tokens,
        tokens_cache_w: agg.cache_write_tokens,
        tokens_cache_r: agg.cache_read_tokens,
        model_used: model.clone(),
        source_tool: CLAUDE_SCIENCE_SOURCE_TOOL.to_string(),
        cost_estimated,
        // rm-790: the source row id makes the derived-history identity
        // per-row — here the root frame id, the lane's session key.
        session_key: session_id.clone(),
        ..Metrics::default()
    };
    metrics.session_start = unix_seconds_rfc3339(agg.first_ms as f64 / 1000.0);
    metrics.session_end = unix_seconds_rfc3339(agg.last_ms as f64 / 1000.0);
    if agg.last_ms > agg.first_ms {
        metrics.duration_sec = (agg.last_ms - agg.first_ms) as f64 / 1000.0;
        metrics.provenance.duration = "timestamp_span".to_string();
    }
    metrics.provenance.tokens = "reported_by_agent".to_string();
    metrics.provenance.cost = "calculated_from_tokens".to_string();
    metrics.provenance.tool_results = "unavailable".to_string();
    metrics.provenance.files = "unavailable".to_string();
    metrics.provenance.naming = if agg.project.is_some() {
        "provider_title"
    } else {
        "session_id"
    }
    .to_string();
    metrics.provenance.pricing_source = if multiple_models {
        "SQLite aggregate: multiple models".to_string()
    } else {
        crate::pricing::pricing_source_for(&model)
    };
    // Acceptance d: the recorded-cost arm is deferred to the rm-253
    // family — disclose every frame whose stored cost was read but
    // not consumed, so an estimate never masquerades as the app's own
    // recorded number.
    if agg.recorded_cost_frames > 0 {
        metrics.disclosure_counters.insert(
            "claude_science_recorded_cost_deferred".to_string(),
            agg.recorded_cost_frames,
        );
    }
    let anomalies = detect_anomalies(&metrics);
    let health = health_score(&anomalies);
    Session {
        name: agg.project.unwrap_or_else(|| session_id.clone()),
        path: path.to_string_lossy().to_string(),
        cwd: String::new(),
        branch: String::new(),
        metrics,
        anomalies,
        health,
        tool_warnings: Vec::new(),
        diagnostics: crate::Diagnostics::default(),
    }
}

use crate::{
    detect_anomalies, health_score, token_cost, Metrics, Session, SessionFileDiff,
    SummaryColumnUnreadable, SummaryDiffDivergence,
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, ErrorCode, OpenFlags};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct RoleCounts {
    user: usize,
    assistant: usize,
    tool: usize,
}

#[derive(Debug, Default)]
struct SqliteSessionAgg {
    id: String,
    /// rm-791: opencode `session.parent_id` — links a subagent child row
    /// to its parent row within the same database. `None` for hermes rows
    /// and for opencode schemas without the column.
    parent_id: Option<String>,
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
    /// rm-991 (cycle 2, run b5affead): opencode `session.agent` — the
    /// upstream-recorded name of the agent that ran the session (live
    /// host census 2026-10-12: 302/310 rows carry one). Empty string
    /// when the column or value is absent; the `by_agent` rollup then
    /// falls back to `source_tool` exactly as before.
    agent_name: String,
    /// rm-991: the writer-reported diff summary on the same row
    /// (`summary_additions`/`summary_deletions`/`summary_files`),
    /// read leniently per column — `None` when absent.
    summary_additions: Option<i64>,
    summary_deletions: Option<i64>,
    summary_files: Option<i64>,
    /// rm-1509 (cycle 2, run b01c2b6a): the OpenCode v2
    /// `session.summary_diffs` json column — the writer's per-file
    /// diff list (`FileDiff.LegacyInfo[]` upstream), decoded
    /// leniently. `None` when the column or value is absent (1.x
    /// stores), and `None` plus a counted drop below when the payload
    /// exists but fails to decode. Patch bodies are deliberately not
    /// carried: delivery evidence reports counts and paths.
    summary_diffs: Option<Vec<SessionFileDiff>>,
    /// rm-1509: 1 when this row's `summary_diffs` payload failed to
    /// decode — disclosed on Metrics, never a hard error and never a
    /// silently re-derived value.
    summary_diffs_dropped: u32,
    /// rm-1926 (run 9d3b927c cycle 1): `summary_additions` /
    /// `summary_deletions` columns that are schema-present but
    /// unreadable on this row (NULL / BLOB / non-numeric TEXT
    /// storage) — the cross-check against `summary_diffs` discloses
    /// them instead of silently skipping the row.
    summary_unreadable: Vec<SummaryColumnUnreadable>,
    source_tool: String,
    path: String,
    cwd: String,
    /// Text of the first user message (opencode: from the `part` table),
    /// used for message-derived naming when the provider title is empty
    /// or a placeholder (research candidate 34).
    first_user_text: String,
}

/// Per-file honesty report for the SQLite-backed ingestion lanes
/// (rm-753): a discovered database that exists but cannot be read, and
/// session rows that fail to decode, are disclosed here instead of
/// silently counting as absent/empty.
#[derive(Debug, Default, Clone)]
pub struct SqliteIngestReport {
    /// Discovered database files that could not be opened or queried
    /// (corrupt bytes, unusable schema, permissions) — distinct from an
    /// absent file, which legitimately yields no sessions.
    pub unreadable: Vec<SqliteUnreadableDb>,
    /// Per-file counts of session rows dropped because the row failed
    /// to decode (NULL ids, wrong-typed columns outside the lenient
    /// readers). A dropped row is a lost session, not an empty corpus.
    pub dropped_rows: Vec<SqliteDroppedRows>,
    /// rm-548: opencode fork copies excluded from aggregation — a fork
    /// re-emits its parent's history, so counting both double-counts
    /// usage. The count is carried here so the cached and fresh paths
    /// disclose identically.
    ///
    /// Superseded in the SQLITE lane at integration (run 2023f222,
    /// rm-791, ranked above rm-548): live host measurement showed
    /// `session.parent_id` rows are subagent children with their own
    /// message ids — retained and attributed, not excluded — so only
    /// the JSON storage lane (`parentID` marker, counted during file
    /// discovery) feeds this counter now; the sqlite lane reports 0.
    pub fork_excluded: usize,
    /// rm-607: sibling databases (backups, copies, sync artifacts)
    /// whose rows repeated sessions an earlier — canonical — database
    /// already contributed. Suppressed duplicates are disclosed (via
    /// `--doctor`), never silently skipped: the numbers are honest but
    /// the user should learn their backup lives inside the glob.
    pub duplicate_dbs: Vec<SqliteDuplicateDb>,
    /// rm-367(b): databases whose ingest pass was cut short by the
    /// doctor's scan budget. A budget-truncated pass keeps the
    /// sessions it loaded and is disclosed here — partial counts are
    /// never presented as complete ones, and the truncated pass never
    /// writes a snapshot (see `SqliteFileFailures::is_clean`).
    pub partial_dbs: Vec<SqlitePartialDb>,
    /// rm-1927 (read arm rm-2290): stores carrying the v2 schema
    /// shape are ingested — see `v2_reads`. This disclosure now
    /// fires ONLY when a v2-shaped table was detected but could not
    /// be READ (shape-detected-but-unreadable: the guarded SELECT
    /// itself failed), counted separately from `unreadable` (corrupt)
    /// and from absent files (legitimately session-less).
    pub unhandled_v2_stores: Vec<SqliteV2Store>,
    /// rm-2290: v2-shaped tables this pass actually READ (the
    /// `session_v2` lane beside the v1 `session` lane). Each entry
    /// is a success disclosure, not a failure: `rows_ingested` rows
    /// became counted sessions, `twin_rows` rows were skipped
    /// because the v1 lane already counted the same id (upstream's
    /// V1→V2 migration is a COPY — it inserts every v1 `session`
    /// row into `session_v2` verbatim and leaves the v1 rows in
    /// place, so a migrated home double-carries each legacy session;
    /// counting once, through the richer message-enriched v1 lane,
    /// is the honest read), and `absent_columns` names the columns
    /// upstream drifted away (leniently read as NULL, never a hard
    /// failure).
    pub v2_reads: Vec<SqliteV2Read>,
}

/// rm-2290: accounting for one opencode v2-shaped session table this
/// pass read (per-database; the primary and every sibling store get
/// their own entry).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SqliteV2Read {
    pub path: PathBuf,
    /// The session-store-shaped table the rows came from (usually
    /// `session_v2`; matched by column signature, not name).
    pub table: String,
    pub rows_ingested: usize,
    pub twin_rows: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub absent_columns: Vec<String>,
}

/// rm-607: a sibling sqlite database whose sessions were fully or
/// partly suppressed as cross-file duplicates of an earlier database.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SqliteDuplicateDb {
    pub path: String,
    pub duplicate_sessions: usize,
}

/// rm-367(b): a sqlite database whose ingest pass was truncated by
/// the doctor's per-database wall-clock budget; `sessions_kept` is
/// what the pass loaded before the bound hit.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SqlitePartialDb {
    pub path: String,
    pub sessions_kept: usize,
}

/// rm-1927 (run 9d3b927c cycle 1): one discovered opencode store that
/// carries the v2 schema SHAPE — a session-store column signature
/// under a table other than the v1 `session` (the `session_v2`
/// today) — which this reader could not READ (see `SqliteV2Read` for
/// the ingested arm). Distinct from `SqliteUnreadableDb`: a shape
/// the store probe knows, not corruption; disclosing it under
/// "unreadable" would misreport a clean upstream store as broken.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SqliteV2Store {
    pub path: PathBuf,
    /// The table the shape probe matched — matched by column
    /// signature, not by name (rm-507 discipline), so an upstream
    /// rename keeps the disclosure truthful.
    pub table: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SqliteUnreadableDb {
    pub path: PathBuf,
    pub source: &'static str,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SqliteDroppedRows {
    pub path: PathBuf,
    pub source: &'static str,
    pub dropped: usize,
    pub sample: String,
}

impl SqliteIngestReport {
    fn absorb(
        &mut self,
        path: &Path,
        source: &'static str,
        failures: SqliteFileFailures,
        loaded: usize,
    ) {
        // rm-367(b): a budget-truncated pass is disclosed as partial —
        // the sessions it kept are returned, the rest is visibly
        // uncounted, and no snapshot is written for the pass.
        if failures.budget_exhausted {
            self.partial_dbs.push(SqlitePartialDb {
                path: path.to_string_lossy().to_string(),
                sessions_kept: loaded,
            });
        }
        if let Some(reason) = failures.unreadable {
            self.unreadable.push(SqliteUnreadableDb {
                path: path.to_path_buf(),
                source,
                reason,
            });
        }
        // rm-1927: the unhandled-format class reaches the report
        // through the same absorb — doctor and the CLI read it off
        // `unhandled_v2_stores`, never off `unreadable`.
        if let Some(reason) = failures.unhandled_format {
            self.unhandled_v2_stores.push(SqliteV2Store {
                path: path.to_path_buf(),
                table: failures.unhandled_table.unwrap_or_default(),
                reason,
            });
        }
        // rm-2290: the v2 READ arm — a success disclosure, carried
        // beside the sessions it produced so doctor and the CLI can
        // show "counted once" twin and drift accounting even when
        // the pass was otherwise clean (and therefore snapshot-banked
        // on the warm path, where the entry replays from the snapshot
        // meta — schema 15, rm-548 precedent).
        if let Some(read) = failures.v2_read {
            self.v2_reads.push(read);
        }
        if failures.dropped > 0 {
            self.dropped_rows.push(SqliteDroppedRows {
                path: path.to_path_buf(),
                source,
                dropped: failures.dropped,
                sample: failures
                    .dropped_sample
                    .unwrap_or_else(|| "undecodable session row".to_string()),
            });
        }
    }
}

/// Failures of one database file during a load pass.
#[derive(Debug, Default, Clone)]
struct SqliteFileFailures {
    unreadable: Option<String>,
    /// rm-1927: the file is a healthy store in a format this reader
    /// does not ingest (v2 schema shape). Carried separately from
    /// `unreadable` so the doctor census counts its own class, and so
    /// `is_clean` stays false — an unhandled store must never bank a
    /// session-less snapshot that would mask the disclosure on the
    /// warm path.
    unhandled_format: Option<String>,
    unhandled_table: Option<String>,
    /// rm-2290: set when this pass READ a v2-shaped session table —
    /// the ingested arm of the rm-1927 disclosure. Carries the
    /// per-table accounting (rows, twins, drifted columns) to the
    /// report through `absorb`; does NOT affect `is_clean`, because
    /// a successful read banks an honest snapshot (its twin/drift
    /// notes ride the snapshot meta on the warm path).
    v2_read: Option<SqliteV2Read>,
    dropped: usize,
    dropped_sample: Option<String>,
    /// rm-367(b): set when the pass was cut short by the scan budget
    /// (progress-handler interrupt or phase-boundary deadline), which
    /// is a bound, not a decode failure.
    budget_exhausted: bool,
}

impl SqliteFileFailures {
    fn record_dropped(&mut self, error: &rusqlite::Error) {
        // rm-367(b): the doctor's budget interrupt surfaces as
        // OperationInterrupted — that is a scan bound being enforced,
        // not a row that failed to decode, so it must not inflate the
        // dropped-row disclosure.
        if matches!(
            error,
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: ErrorCode::OperationInterrupted,
                    ..
                },
                _,
            )
        ) {
            self.budget_exhausted = true;
            return;
        }
        self.dropped += 1;
        if self.dropped_sample.is_none() {
            self.dropped_sample = Some(error.to_string());
        }
    }

    fn merge(&mut self, other: SqliteFileFailures) {
        if self.unreadable.is_none() {
            self.unreadable = other.unreadable;
        }
        if self.unhandled_format.is_none() {
            self.unhandled_format = other.unhandled_format;
            self.unhandled_table = other.unhandled_table;
        }
        if self.v2_read.is_none() {
            self.v2_read = other.v2_read;
        }
        self.dropped += other.dropped;
        if self.dropped_sample.is_none() {
            self.dropped_sample = other.dropped_sample;
        }
        self.budget_exhausted |= other.budget_exhausted;
    }

    fn is_clean(&self) -> bool {
        self.unreadable.is_none()
            && self.unhandled_format.is_none()
            && self.dropped == 0
            && !self.budget_exhausted
    }
}

pub fn load_sqlite_backed_sessions() -> Vec<Session> {
    load_sqlite_backed_sessions_since(None)
}

pub(crate) fn load_sqlite_backed_sessions_since(since: Option<DateTime<Utc>>) -> Vec<Session> {
    load_sqlite_backed_sessions_reported(since).0
}

/// rm-753: the SQLite-backed sessions alongside their per-file failure
/// report. Callers that only need sessions keep the Vec-returning
/// wrappers; doctor and the CLI report paths consume the report so a
/// discovered-but-unreadable database and dropped session rows are
/// disclosed instead of rendering as an empty corpus.
///
/// rm-607 rider: this is also the dedup-aware union. The `opencode*.db`
/// glob (and the hermes state.db set) can pick up sibling databases
/// containing the same sessions as the canonical file — a
/// byte-identical backup used to silently double every aggregate (live
/// PoC: 309 -> 618 sessions on the real corpus). Sessions are deduped
/// by `(source_tool, metrics.session_key)` — the rm-790 source row
/// key, not a dedicated column: at this merge the candidate's own
/// `sqlite_session_id` field is superseded by the landed per-row
/// identity — keeping the first occurrence, so the canonical
/// databases load first (the opencode glob is primary-first with
/// sorted siblings; hermes is primary then sorted profiles), and
/// files that contributed duplicates are recorded in
/// `report.duplicate_dbs` for the doctor disclosure. The dedup runs at
/// union time, after each database's snapshot cache resolves, and the
/// key already survives v9+ snapshots, so no snapshot schema bump is
/// required for it.
pub fn load_sqlite_backed_sessions_reported(
    since: Option<DateTime<Utc>>,
) -> (Vec<Session>, SqliteIngestReport) {
    load_sqlite_backed_sessions_reported_bounded(since, None)
}

/// rm-367(b): the doctor's wall-clock bound for the SQLite lane.
/// `per_db` caps any single database (installed as a SQLite progress
/// handler, so even one multi-minute aggregate statement is
/// interruptible), `total` caps the lane, and `progress` emits
/// per-database stderr lines so a slow run is diagnosable live.
#[derive(Debug, Clone, Copy)]
pub struct SqliteScanBudget {
    pub per_db: Duration,
    pub total: Duration,
    pub progress: bool,
}

fn db_ingest_deadline(
    budget: Option<SqliteScanBudget>,
    total_deadline: Option<Instant>,
) -> Option<Instant> {
    let per_db = budget.map(|b| Instant::now() + b.per_db)?;
    Some(match total_deadline {
        Some(total) if total < per_db => total,
        _ => per_db,
    })
}

/// rm-367(b): `SqliteFileFailures::budget_exhausted` discriminator —
/// true when the deadline has passed or an interrupt already landed.
fn budget_deadline_hit(deadline: Option<Instant>, failures: &mut SqliteFileFailures) -> bool {
    if deadline.is_some_and(|bound| Instant::now() >= bound) {
        failures.budget_exhausted = true;
    }
    failures.budget_exhausted
}

pub fn load_sqlite_backed_sessions_reported_bounded(
    since: Option<DateTime<Utc>>,
    budget: Option<SqliteScanBudget>,
) -> (Vec<Session>, SqliteIngestReport) {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return (Vec::new(), SqliteIngestReport::default());
    };
    let total_deadline = budget.map(|b| Instant::now() + b.total);
    let mut sessions = Vec::new();
    let mut report = SqliteIngestReport::default();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for path in hermes_state_db_paths(&home) {
        // rm-367(b): an exhausted lane budget skips the database
        // before it is opened — disclosed as a partial pass with zero
        // sessions kept, never as an absent store.
        if total_deadline.is_some_and(|bound| Instant::now() >= bound) {
            report.partial_dbs.push(SqlitePartialDb {
                path: path.to_string_lossy().to_string(),
                sessions_kept: 0,
            });
            continue;
        }
        let deadline = db_ingest_deadline(budget, total_deadline);
        if budget.is_some_and(|b| b.progress) {
            eprintln!(
                "agenttrace: doctor: sqlite hermes {} (budget {}s)",
                path.display(),
                budget.map(|b| b.per_db.as_secs()).unwrap_or(0)
            );
        }
        let loaded = load_hermes_sqlite_sessions(&path, since, &mut report, deadline);
        let loaded_count = loaded.len();
        extend_deduped(&mut sessions, &mut report, &mut seen, loaded, &path);
        if deadline.is_some_and(|bound| Instant::now() >= bound)
            && budget.is_some_and(|b| b.progress)
        {
            eprintln!("agenttrace: doctor: sqlite hermes {} — budget reached, {} sessions kept, disclosed partial", path.display(), loaded_count);
        }
    }
    // rm-548 (JSON-lane scope after the rm-791 supersession): fork
    // copies re-emit their parent's history, so counting them would
    // double-count usage; they are excluded in file discovery and
    // counted into the report for disclosure. The sqlite lane's
    // `parent_id` rows are subagent children (rm-791) and are retained
    // and attributed instead — see `opencode_sqlite_session_rows`.
    for path in opencode_db_paths(&home) {
        if total_deadline.is_some_and(|bound| Instant::now() >= bound) {
            report.partial_dbs.push(SqlitePartialDb {
                path: path.to_string_lossy().to_string(),
                sessions_kept: 0,
            });
            continue;
        }
        let deadline = db_ingest_deadline(budget, total_deadline);
        if budget.is_some_and(|b| b.progress) {
            eprintln!(
                "agenttrace: doctor: sqlite opencode {} (budget {}s)",
                path.display(),
                budget.map(|b| b.per_db.as_secs()).unwrap_or(0)
            );
        }
        let loaded = load_opencode_sqlite_sessions(&path, since, &mut report, deadline);
        let loaded_count = loaded.len();
        extend_deduped(&mut sessions, &mut report, &mut seen, loaded, &path);
        if deadline.is_some_and(|bound| Instant::now() >= bound)
            && budget.is_some_and(|b| b.progress)
        {
            eprintln!("agenttrace: doctor: sqlite opencode {} — budget reached, {} sessions kept, disclosed partial", path.display(), loaded_count);
        }
    }
    (sessions, report)
}

/// Merge one database's sessions into the union (rm-607), counting
/// rows whose `(source_tool, metrics.session_key)` identity was
/// already carried by an earlier file as duplicates of that file.
/// The key is the rm-790 per-row source key — globally unique per
/// session row within a family — so a byte-identical backup db
/// contributes zero new identities and is recorded whole.
fn extend_deduped(
    sessions: &mut Vec<Session>,
    report: &mut SqliteIngestReport,
    seen: &mut HashSet<(String, String)>,
    loaded: Vec<Session>,
    path: &Path,
) {
    let mut duplicates = 0usize;
    for session in loaded {
        let key = (
            session.metrics.source_tool.clone(),
            session.metrics.session_key.clone(),
        );
        if seen.insert(key) {
            sessions.push(session);
        } else {
            duplicates += 1;
        }
    }
    if duplicates > 0 {
        report.duplicate_dbs.push(SqliteDuplicateDb {
            path: path.to_string_lossy().to_string(),
            duplicate_sessions: duplicates,
        });
    }
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
        let mut profiles = entries
            .flatten()
            .map(|entry| entry.path().join("state.db"))
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        // rm-607 (review F3): read_dir order is host-nondeterministic,
        // so a duplicate shared by two profiles would otherwise keep a
        // random copy; sorted, the cross-profile winner is stable —
        // and the primary above always wins against either.
        profiles.sort();
        paths.extend(profiles);
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
    // rm-607: the canonical db loads FIRST so the cross-file session
    // dedup keeps the primary copy; sibling databases (backups, copies)
    // sort after it and surrender their duplicate sessions.
    let mut siblings = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("opencode") && name.ends_with(".db"))
        })
        .filter(|path| *path != primary)
        .collect::<Vec<_>>();
    siblings.sort();
    let mut ordered = Vec::with_capacity(siblings.len() + 1);
    ordered.push(primary);
    ordered.extend(siblings);
    ordered
}

fn sqlite_file_exists(path: &Path) -> bool {
    path.is_file()
}

/// rm-626: without a busy timeout, a database locked by its writer fails
/// the read immediately (SQLITE_BUSY), and that transient failure used
/// to be persisted as an empty snapshot that masked the database on
/// every later run. Waiting briefly lets the normal writer-release race
/// resolve; reads that still time out fail the query — which the caller
/// no longer caches — instead of caching "no data".
const SQLITE_BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(250);

/// rm-367(b): keeps the deadline watcher thread alive for the
/// connection's ingest pass; dropped (stopping the watcher) as soon
/// as the queries finish.
struct SqliteBudgetWatch {
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for SqliteBudgetWatch {
    fn drop(&mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// rm-367(b): `deadline` spawns a watcher thread that interrupts any
/// statement still running past the bound — the only way to bound a
/// single multi-minute aggregate over a huge store without pulling
/// in rusqlite's `hooks` feature.
fn open_sqlite_read_only_bounded(
    path: &Path,
    deadline: Option<Instant>,
) -> rusqlite::Result<(Connection, Option<SqliteBudgetWatch>)> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
    let watch = deadline.map(|deadline| {
        let handle = connection.get_interrupt_handle();
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = std::sync::Arc::clone(&done);
        std::thread::spawn(move || {
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                if Instant::now() >= deadline {
                    handle.interrupt();
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        SqliteBudgetWatch { done }
    });
    Ok((connection, watch))
}

fn load_hermes_sqlite_sessions(
    path: &Path,
    since: Option<DateTime<Utc>>,
    report: &mut SqliteIngestReport,
    deadline: Option<Instant>,
) -> Vec<Session> {
    if !sqlite_file_exists(path) {
        return Vec::new();
    }
    if let Some(sessions) = crate::session_cache::load_sqlite_snapshot(path, "hermes") {
        // rm-753: an empty snapshot is not trusted as final — a corrupt
        // database cached an empty result under pre-fix binaries and
        // the unreadable disclosure would stay hidden behind it
        // forever. Re-query instead (cheap on a genuinely empty DB) so
        // failures are re-detected every run.
        if !sessions.is_empty() {
            return filter_since(sessions, since);
        }
    }
    let (sessions, failures) = query_hermes_sqlite_sessions(path, None, deadline);
    let clean = failures.is_clean();
    let loaded = sessions.len();
    report.absorb(path, "hermes", failures, loaded);
    // A failed pass must not poison the snapshot cache with an empty
    // result: the failure would replay as "absent" on every later run.
    if clean {
        let _ = crate::session_cache::store_sqlite_snapshot(path, "hermes", &sessions);
    }
    filter_since(sessions, since)
}

fn query_hermes_sqlite_sessions(
    path: &Path,
    since: Option<DateTime<Utc>>,
    deadline: Option<Instant>,
) -> (Vec<Session>, SqliteFileFailures) {
    let mut failures = SqliteFileFailures::default();
    let (db, _budget_watch) = match open_sqlite_read_only_bounded(path, deadline) {
        Ok(opened) => opened,
        // rm-753: found-but-unopenable is a distinct failure from
        // absent (permissions, lock policy) — disclose, don't null.
        Err(error) => {
            failures.unreadable = Some(error.to_string());
            return (Vec::new(), failures);
        }
    };
    // rm-893: the enrichment lanes share the per-file failure
    // accumulator, so a row that fails column decode is counted into
    // the same disclosure as session-row drops (one record per
    // database file).
    let roles = sqlite_role_counts(&db, "messages", "session_id", "role", &mut failures);
    let tool_outcomes = hermes_tool_outcome_counts(&db, &mut failures);
    // rm-367(b): an aggregate interrupted mid-scan can return a
    // partial map with no row-level error — the deadline check turns
    // that into a disclosed partial pass instead of a silently
    // undercounted ingest.
    if budget_deadline_hit(deadline, &mut failures) {
        return (Vec::new(), failures);
    }
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
    let mut stmt = match db.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(error) => {
            // A valid-but-foreign database with no `sessions` table is
            // legitimately session-less; anything else (corrupt bytes,
            // "file is not a database") is found-but-unreadable.
            if !error.to_string().contains("no such table") {
                failures.unreadable = Some(error.to_string());
            }
            return (Vec::new(), failures);
        }
    };
    let since_unix = since.map(|value| value.timestamp() as f64);
    let rows = match stmt.query_map([since_unix], |row| {
        Ok(SqliteSessionAgg {
            id: row.get::<_, String>(0)?,
            model: string_or(row.get::<_, Option<String>>(1)?, "default"),
            start_unix: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            end_unix: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            events: row.get::<_, Option<i64>>(4)?.unwrap_or(0).max(0) as usize,
            tool_calls_total: row.get::<_, Option<i64>>(5)?.unwrap_or(0).max(0) as usize,
            input_tokens: row.get::<_, Option<i64>>(6)?.unwrap_or(0).max(0),
            output_tokens: row.get::<_, Option<i64>>(7)?.unwrap_or(0).max(0),
            cache_read_tokens: row.get::<_, Option<i64>>(8)?.unwrap_or(0).max(0),
            cache_write_tokens: row.get::<_, Option<i64>>(9)?.unwrap_or(0).max(0),
            cwd: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
            source_tool: "hermes_db".to_string(),
            path: path.to_string_lossy().to_string(),
            ..SqliteSessionAgg::default()
        })
    }) {
        Ok(rows) => rows,
        Err(error) => {
            failures.unreadable = Some(error.to_string());
            return (Vec::new(), failures);
        }
    };

    // rm-753: a row that fails to decode is counted and sampled, never
    // dropped by filter_map — a NULL id used to erase the whole session
    // silently (P13: 1 of 2 sessions "reported").
    let mut sessions = Vec::new();
    for row in rows {
        match row {
            Ok(mut agg) => {
                enrich_hermes_agg(&mut agg, &roles, &tool_outcomes);
                sessions.push(session_from_sqlite_agg(agg));
            }
            Err(error) => failures.record_dropped(&error),
        }
    }
    (sessions, failures)
}

fn enrich_hermes_agg(
    agg: &mut SqliteSessionAgg,
    roles: &HashMap<String, RoleCounts>,
    tool_outcomes: &HashMap<String, HermesToolOutcomes>,
) {
    if !agg.model.is_empty() {
        agg.models.insert(agg.model.clone());
    }
    if let Some(counts) = roles.get(&agg.id) {
        agg.user_messages = counts.user;
        agg.assistant_turns = counts.assistant;
    }
    if let Some(outcomes) = tool_outcomes.get(&agg.id) {
        // rm-198: the hermes session row records only the number
        // of tool calls, never their outcome. The outcome split
        // comes from the messages table instead (observed result
        // rows, minus the ones hermes renders as tool errors),
        // matching the parser convention that ok+fail counts
        // observed results while tool_calls_total counts calls.
        agg.tool_results = outcomes.results;
        agg.tool_calls_fail = outcomes.failures;
        agg.tool_calls_ok = outcomes.results.saturating_sub(outcomes.failures);
        agg.tool_calls_total = agg.tool_calls_total.max(outcomes.results);
    } else if let Some(counts) = roles.get(&agg.id) {
        // No content column (or no tool rows): an outcome split
        // cannot be derived, so report the observed result count
        // without fabricating a success split.
        agg.tool_results = counts.tool;
    }
}

fn load_opencode_sqlite_sessions(
    path: &Path,
    since: Option<DateTime<Utc>>,
    report: &mut SqliteIngestReport,
    deadline: Option<Instant>,
) -> Vec<Session> {
    if !sqlite_file_exists(path) {
        return Vec::new();
    }
    if let Some((sessions, fork_excluded, v2_reads)) =
        crate::session_cache::load_sqlite_snapshot_with_meta(path, "opencode")
    {
        // rm-548: the warm snapshot (schema 9) carries the stored
        // exclusion count, so the disclosure cannot go silent on the
        // cached path. v9 snapshots bank 0 for the sqlite lane (the
        // rm-791 supersession); the count survives for the JSON lane's
        // benefit through the shared field.
        report.fork_excluded += fork_excluded;
        // rm-2290: the warm snapshot (schema 15) carries the v2 read
        // accounting beside the sessions, so the counted-once twin
        // and column-drift disclosures survive the cached path
        // exactly like the exclusion count above.
        report.v2_reads.extend(v2_reads);
        // rm-753: see load_hermes_sqlite_sessions — an empty snapshot
        // is re-verified, not trusted (pre-fix corrupt databases cached
        // one; the P14 probe left exactly such a file behind); the
        // fresh query recounts the exclusions, so the stored count is
        // only folded in on the warm-return path.
        if !sessions.is_empty() {
            return filter_since(sessions, since);
        }
    }
    let (sessions, failures, fork_excluded) = query_opencode_sqlite_sessions(path, None, deadline);
    let clean = failures.is_clean();
    let loaded = sessions.len();
    // rm-2290: the v2 accounting banks with the snapshot (schema 15)
    // so it can replay on warm loads — captured before `absorb`
    // consumes the per-file failures.
    let v2_meta: Vec<SqliteV2Read> = failures.v2_read.clone().into_iter().collect();
    report.absorb(path, "opencode", failures, loaded);
    report.fork_excluded += fork_excluded;
    if clean {
        let _ = crate::session_cache::store_sqlite_snapshot_with_meta(
            path,
            "opencode",
            &sessions,
            fork_excluded,
            &v2_meta,
        );
    }
    filter_since(sessions, since)
}

fn query_opencode_sqlite_sessions(
    path: &Path,
    since: Option<DateTime<Utc>>,
    deadline: Option<Instant>,
) -> (Vec<Session>, SqliteFileFailures, usize) {
    let mut failures = SqliteFileFailures::default();
    let (db, _budget_watch) = match open_sqlite_read_only_bounded(path, deadline) {
        Ok(opened) => opened,
        Err(error) => {
            failures.unreadable = Some(error.to_string());
            return (Vec::new(), failures, 0);
        }
    };
    let (mut aggs, row_failures, fork_excluded) = opencode_sqlite_session_rows(&db, path, since);
    failures.merge(row_failures);
    if budget_deadline_hit(deadline, &mut failures) {
        return (Vec::new(), failures, fork_excluded);
    }
    if aggs.is_empty() {
        return (Vec::new(), failures, fork_excluded);
    }
    // rm-893: enrichment decode failures count into the same per-file
    // disclosure as session-row drops.
    add_opencode_sqlite_messages(&db, &mut aggs, &mut failures);
    if budget_deadline_hit(deadline, &mut failures) {
        return (Vec::new(), failures, fork_excluded);
    }
    add_opencode_sqlite_parts(&db, &mut aggs, &mut failures);
    if budget_deadline_hit(deadline, &mut failures) {
        return (Vec::new(), failures, fork_excluded);
    }
    capture_opencode_user_text(&db, &mut aggs, &mut failures);

    let sessions = aggs
        .into_values()
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
        .collect();
    (sessions, failures, fork_excluded)
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

/// rm-890: admission delegates to the ONE shared predicate
/// (insights::session_admitted_since, the rm-694 overlap basis) — a
/// session counts when it had activity at or after the cutoff, so an
/// overnight session (started before the window, last activity inside
/// it) stays visible, matching the file lanes and the TUI. A session
/// with unknown time (empty or unparseable) stays in the unknown-time
/// bucket instead of being silently dropped (N7).
fn session_within_since(session: &Session, since: Option<DateTime<Utc>>) -> bool {
    // rm-890: same basis as every other lane — last known activity at
    // or after the cutoff (overnight sessions stay in ranged views).
    // rm-1534: this is a PRE-filter only, so it deliberately applies
    // the LOWER bound alone: the future bound (now+grace) is applied
    // once, by the merged-corpus retain in discovery.rs, so its
    // exclusion count (`future_excluded_sessions`) sees every lane —
    // sqlite-backed future sessions too. The pre-filter stays a
    // strict superset of the ONE predicate's final view, so no lane
    // can disagree about which sessions exist in a ranged view.
    since.is_none_or(|since| {
        crate::insights::session_last_activity(session)
            .map(|time| time >= since)
            .unwrap_or(true)
    })
}

fn opencode_sqlite_session_rows(
    db: &Connection,
    path: &Path,
    since: Option<DateTime<Utc>>,
) -> (HashMap<String, SqliteSessionAgg>, SqliteFileFailures, usize) {
    let mut failures = SqliteFileFailures::default();
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
    // rm-791 (superseding rm-548's sqlite-lane fork reading of the same
    // column at integration): opencode's `parent_id` links subagent
    // child sessions to their parent row; guarded like the stored
    // columns so an older database without the column still loads
    // (parentless).
    let parent_select = if sqlite_has_column(db, "session", "parent_id") {
        "parent_id"
    } else {
        "null"
    };
    // rm-991 (cycle 2, run b5affead — the landed lane this integration
    // kept over run b5d391ec's stranded parallel implementation of
    // the same unit): `session.agent` names WHO ran the session and
    // `summary_*` carries the writer-reported diff summary. Guarded
    // like the stored columns so an older schema without them still
    // loads (unnamed, no summary) — the live host census found
    // migration rows predating the columns.
    let agent_select = if sqlite_has_column(db, "session", "agent") {
        "agent"
    } else {
        "null"
    };
    let summary_columns = [
        (
            "summary_additions",
            sqlite_has_column(db, "session", "summary_additions"),
        ),
        (
            "summary_deletions",
            sqlite_has_column(db, "session", "summary_deletions"),
        ),
        (
            "summary_files",
            sqlite_has_column(db, "session", "summary_files"),
        ),
        // rm-1509 (run b01c2b6a): the OpenCode v2 per-file diff
        // list — MUST stay last: it extends the summary block so the
        // fixed read indices for the integer columns are unchanged
        // (summary_diffs reads index 16).
        (
            "summary_diffs",
            sqlite_has_column(db, "session", "summary_diffs"),
        ),
    ];
    let summary_select = summary_columns
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
        "select id, title, time_created, time_updated, {directory}, {stored_select}, {parent_select}, {agent_select}, {summary_select} \
         from session where (?1 is null or time_created >= ?1 or time_created is null or time_created <= 0)"
    );
    let mut stmt = match db.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(error) => {
            // Same class split as the hermes lane: a database with no
            // `session` table is session-less, anything else is
            // found-but-unreadable (random bytes → "file is not a
            // database", P12/P14).
            if !error.to_string().contains("no such table") {
                failures.unreadable = Some(error.to_string());
                return (HashMap::new(), failures, 0);
            }
            // rm-1927 (read arm rm-2290): session-less by NAME is
            // not session-less by SCHEMA. A store whose tables carry
            // the opencode session-store column signature under
            // another name (the `session_v2` DDL today) used to
            // classify as session-less — zero sessions, zero
            // disclosures, a silent zero that reads as "nothing
            // happened here". The v2 arm now READS the shaped table
            // into counted sessions; the shape probe inside it still
            // keys on the column signature, not the table name
            // (rm-507), and a table that matches the shape but
            // cannot be read stays the named unhandled format. A
            // database with neither shape stays legitimately
            // session-less (a foreign sqlite file inside the glob).
            let mut aggs = HashMap::new();
            opencode_v2_session_rows(db, path, since, &mut aggs, &mut failures);
            return (aggs, failures, 0);
        }
    };
    let since_millis = since.map(|value| value.timestamp_millis());
    let rows = match stmt.query_map([since_millis], |row| {
        let id = row.get::<_, String>(0)?;
        // rm-1926: computed before the struct literal so the reasons
        // can read the raw storage class. Only schema-present
        // columns participate: an absent column is the 1.x schema
        // (the null-padded SELECT reads None), not a broken v2 write.
        let summary_unreadable = {
            let additions: Option<rusqlite::types::Value> = row.get(13)?;
            let deletions: Option<rusqlite::types::Value> = row.get(14)?;
            let mut unreadable = Vec::new();
            for (column, present, value) in [
                ("summary_additions", summary_columns[0].1, &additions),
                ("summary_deletions", summary_columns[1].1, &deletions),
            ] {
                if let Some(reason) = sqlite_i64_unreadable_reason(value.as_ref(), present) {
                    unreadable.push(SummaryColumnUnreadable {
                        column: column.to_string(),
                        reason: reason.to_string(),
                    });
                }
            }
            unreadable
        };
        // rm-1509: decode the v2 per-file list before building the
        // aggregate — one session row maps to one aggregate, so the
        // drop count lands per-session by construction. A payload
        // that fails to decode is COUNTED, never a hard error and
        // never silently re-derived. Review fix F5 (attempt
        // 03ec687a): SQLite columns are dynamically typed, so a
        // PRESENT-but-non-TEXT payload (BLOB/INTEGER/REAL storage)
        // is counted as dropped too — reading it as absent would
        // neither surface it nor count it, against this lane's
        // posture. Only SQL NULL reads as absent.
        let (summary_diffs, summary_diffs_dropped) = {
            let payload: Option<rusqlite::types::Value> = row.get(16)?;
            match payload {
                None => (None, 0),
                Some(rusqlite::types::Value::Text(text)) => {
                    match serde_json::from_str::<Vec<SessionFileDiff>>(&text) {
                        Ok(list) => (Some(list), 0),
                        Err(_) => (None, 1),
                    }
                }
                Some(_) => (None, 1),
            }
        };
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
                // rm-791: lenient like the stored totals — a non-text
                // parent_id degrades to "no linkage" instead of failing
                // (and thereby dropping) the whole session row.
                parent_id: sqlite_value_as_text(row.get(11)?),
                // rm-991: same leniency — a non-text agent name degrades
                // to unnamed (by_agent falls back to the source-tool
                // bucket) instead of dropping the session.
                agent_name: sqlite_value_as_text(row.get(12)?).unwrap_or_default(),
                summary_additions: sqlite_value_as_i64(row.get(13)?),
                summary_deletions: sqlite_value_as_i64(row.get(14)?),
                summary_files: sqlite_value_as_i64(row.get(15)?),
                // rm-1926: present-but-unreadable `summary_*` integers
                // are disclosed, not silently skipped — the cross-check
                // comparing per-file sums against these columns must
                // say WHY it cannot run.
                summary_unreadable,
                // rm-1509: `summary_diffs` — index 16 is always
                // valid: the SELECT null-pads when the column is
                // absent, so 1.x stores read `None` and parse
                // byte-identically.
                summary_diffs,
                summary_diffs_dropped,
                source_tool: "opencode_db".to_string(),
                path: path.to_string_lossy().to_string(),
                ..SqliteSessionAgg::default()
            },
        ))
    }) {
        Ok(rows) => rows,
        Err(error) => {
            failures.unreadable = Some(error.to_string());
            return (HashMap::new(), failures, 0);
        }
    };
    // rm-753: decode failures are counted and sampled, not
    // filter_map-dropped — the NULL-id ghost row erased a whole
    // session silently (P11: 4 of 5 sessions "reported").
    // rm-791: a row carrying `parent_id` is a subagent child, NOT a
    // fork copy — it stays in the aggregate with its parent marker
    // parked on the agg (`SqliteSessionAgg::parent_id`) for
    // `attribute_subagents` to resolve and roll up; the third return
    // element (the rm-548 fork-exclusion count) is 0 for this lane
    // after the integration supersession of rm-548's sqlite arm.
    let mut aggs = HashMap::new();
    for row in rows {
        match row {
            Ok((id, agg)) => {
                aggs.insert(id, agg);
            }
            Err(error) => failures.record_dropped(&error),
        }
    }
    // rm-2290: the v2 lane rides beside the v1 lane on every pass —
    // a MIGRATED home carries BOTH tables (upstream's V1→V2
    // migration copies every `session` row into `session_v2` verbatim
    // and leaves the v1 rows in place, so the common post-migration
    // shape double-carries each legacy session), so the v2 read runs
    // with the v1 aggregates already keyed: a v2 row whose id the v1
    // lane counted is a TWIN — counted once (through the richer,
    // message-enriched v1 aggregate) and disclosed — while rows that
    // exist only in `session_v2` (sessions started after the
    // migration) load through the v2 arm. Fresh v2 installs have no
    // `session` table and take this same arm off the no-such-table
    // path above with zero twins by construction.
    opencode_v2_session_rows(db, path, since, &mut aggs, &mut failures);
    (aggs, failures, 0)
}

/// rm-2290: the columns the v2 read consumes, in SELECT order. The
/// four-column shape signature (id, time_created, time_updated,
/// directory) is REQUIRED — [`opencode_v2_shaped_table`] verified it
/// before this read — and every other column is guarded and
/// null-padded, exactly like the v1 lane's guarded SELECT, so
/// upstream drift (a dropped or renamed column) degrades to an
/// absent value plus a DISCLOSED column name instead of failing the
/// pass: never a hard error, never silent.
const OPENCODE_V2_READ_COLUMNS: [(&str, bool); 18] = [
    ("id", true),
    ("title", false),
    ("time_created", true),
    ("time_updated", true),
    ("directory", true),
    ("cost", false),
    ("tokens_input", false),
    ("tokens_output", false),
    ("tokens_reasoning", false),
    ("tokens_cache_read", false),
    ("tokens_cache_write", false),
    ("parent_id", false),
    ("agent", false),
    // v2 stores the model as a JSON column — the writer serializes
    // `{"id": …, "providerID": …}` — parsed leniently in
    // [`opencode_v2_model_id`]; never a failed row.
    ("model", false),
    ("summary_additions", false),
    ("summary_deletions", false),
    ("summary_files", false),
    ("summary_diffs", false),
];

/// rm-2290: read the v2-shaped session table and merge its rows into
/// `aggs` — the ingest arm of the rm-1927 disclosure. Sessions this
/// arm contributes carry the writer-stored aggregates (cost/token
/// columns are NOT NULL DEFAULT 0 in the v2 DDL) and the same
/// `opencode_db` source tool as the v1 lane; per-message enrichment
/// (`session_message`) stays deliberately out of scope — the v2 read
/// is session-level aggregates only, the same posture as the v1
/// lane's stored-totals fallback for stores whose `message` table is
/// absent.
fn opencode_v2_session_rows(
    db: &Connection,
    path: &Path,
    since: Option<DateTime<Utc>>,
    aggs: &mut HashMap<String, SqliteSessionAgg>,
    failures: &mut SqliteFileFailures,
) {
    let Some(table) = opencode_v2_shaped_table(db) else {
        return;
    };
    let quoted = format!("\"{}\"", table.replace('"', "\"\""));
    let presence = OPENCODE_V2_READ_COLUMNS
        .iter()
        .map(|(column, _)| sqlite_has_column(db, &quoted, column))
        .collect::<Vec<_>>();
    let absent_columns = OPENCODE_V2_READ_COLUMNS
        .iter()
        .zip(presence.iter())
        .filter(|(_, present)| !**present)
        .map(|((column, _), _)| (*column).to_string())
        .collect::<Vec<_>>();
    let select_list = presence
        .iter()
        .enumerate()
        .map(|(index, present)| {
            if *present {
                OPENCODE_V2_READ_COLUMNS[index].0.to_string()
            } else {
                "null".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "select {select_list} from {quoted} \
         where (?1 is null or time_created >= ?1 or time_created is null or time_created <= 0)"
    );
    let mut stmt = match db.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(error) => {
            // The one remaining unhandled class: shape-detected but
            // UNREADABLE (the guarded SELECT itself failed). It keeps
            // rm-1927's census bucket and `is_clean` stays false, so
            // no session-less snapshot can mask the disclosure on
            // the warm path.
            failures.unhandled_format = Some(format!(
                "opencode v2 store: shape detected but unreadable (found `{table}`; v1 `session` table absent; read failed: {error})"
            ));
            failures.unhandled_table = Some(table);
            return;
        }
    };
    let since_millis = since.map(|value| value.timestamp_millis());
    let rows = match stmt.query_map([since_millis], |row| {
        let summary_unreadable = {
            let additions: Option<rusqlite::types::Value> = row.get(14)?;
            let deletions: Option<rusqlite::types::Value> = row.get(15)?;
            let mut unreadable = Vec::new();
            for (column, present, value) in [
                ("summary_additions", presence[14], &additions),
                ("summary_deletions", presence[15], &deletions),
            ] {
                if let Some(reason) = sqlite_i64_unreadable_reason(value.as_ref(), present) {
                    unreadable.push(SummaryColumnUnreadable {
                        column: column.to_string(),
                        reason: reason.to_string(),
                    });
                }
            }
            unreadable
        };
        // rm-1509 discipline, v2 arm: a `summary_diffs` payload that
        // fails to decode (or carries a non-TEXT storage class) is
        // counted, never a hard error and never re-derived.
        let (summary_diffs, summary_diffs_dropped) = {
            let payload: Option<rusqlite::types::Value> = row.get(17)?;
            match payload {
                None => (None, 0),
                Some(rusqlite::types::Value::Text(text)) => {
                    match serde_json::from_str::<Vec<SessionFileDiff>>(&text) {
                        Ok(list) => (Some(list), 0),
                        Err(_) => (None, 1),
                    }
                }
                Some(_) => (None, 1),
            }
        };
        let model = sqlite_value_as_text(row.get(13)?)
            .map(|raw| opencode_v2_model_id(&raw))
            .unwrap_or_default();
        let id = row.get::<_, String>(0)?;
        Ok((
            id.clone(),
            SqliteSessionAgg {
                id,
                title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                // Lenient like the v1 lane: SQLite columns are
                // dynamically typed, so a corrupt value degrades
                // (unknown time, stored-total fallback) instead of
                // dropping the session row.
                start_unix: sqlite_value_as_i64(row.get(2)?).unwrap_or(0).max(0) as f64 / 1000.0,
                end_unix: sqlite_value_as_i64(row.get(3)?).unwrap_or(0).max(0) as f64 / 1000.0,
                cwd: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                stored_cost: sqlite_value_as_f64(row.get(5)?),
                stored_input: sqlite_value_as_i64(row.get(6)?),
                stored_output: sqlite_value_as_i64(row.get(7)?),
                stored_reasoning: sqlite_value_as_i64(row.get(8)?),
                stored_cache_read: sqlite_value_as_i64(row.get(9)?),
                stored_cache_write: sqlite_value_as_i64(row.get(10)?),
                // Lenient like rm-791's v1 read — a non-text parent id
                // degrades to "no linkage", a non-text agent name to
                // unnamed.
                parent_id: sqlite_value_as_text(row.get(11)?),
                agent_name: sqlite_value_as_text(row.get(12)?).unwrap_or_default(),
                model,
                summary_additions: sqlite_value_as_i64(row.get(14)?),
                summary_deletions: sqlite_value_as_i64(row.get(15)?),
                summary_files: sqlite_value_as_i64(row.get(16)?),
                summary_unreadable,
                summary_diffs,
                summary_diffs_dropped,
                source_tool: "opencode_db".to_string(),
                path: path.to_string_lossy().to_string(),
                ..SqliteSessionAgg::default()
            },
        ))
    }) {
        Ok(rows) => rows,
        Err(error) => {
            failures.unhandled_format = Some(format!(
                "opencode v2 store: shape detected but unreadable (found `{table}`; v1 `session` table absent; read failed: {error})"
            ));
            failures.unhandled_table = Some(table);
            return;
        }
    };
    // rm-753 discipline, v2 arm: decode failures are counted and
    // sampled, never filter_map-dropped — and twin bookkeeping runs
    // per row so an upstream migration cannot double-count a session
    // through its copied row.
    let mut read = SqliteV2Read {
        path: path.to_path_buf(),
        table,
        absent_columns,
        ..SqliteV2Read::default()
    };
    for row in rows {
        match row {
            Ok((id, agg)) => match aggs.entry(id) {
                std::collections::hash_map::Entry::Occupied(_) => read.twin_rows += 1,
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(agg);
                    read.rows_ingested += 1;
                }
            },
            Err(error) => failures.record_dropped(&error),
        }
    }
    failures.v2_read = Some(read);
}

/// rm-2290: extract the model name from the v2 `model` column. The
/// writer serializes a JSON object (`{"id": …, "providerID": …}`),
/// but SQLite columns are dynamically typed and hostile or hand-edited
/// rows carry arbitrary text: an object with a string `id` yields that
/// id, a bare JSON string yields the string, and anything else
/// (malformed JSON, plain text) degrades to that text trimmed — model
/// attribution, never a failed row.
fn opencode_v2_model_id(raw: &str) -> String {
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(serde_json::Value::String(id)) = map.get("id") {
            return id.clone();
        }
    }
    if let Ok(serde_json::Value::String(text)) = serde_json::from_str::<serde_json::Value>(raw) {
        return text;
    }
    raw.trim().to_string()
}

/// rm-893: message rows whose columns fail to decode (a hostile or
/// hand-edited database can carry a non-TEXT storage class where the
/// lane reads TEXT — a BLOB bypasses column affinity and survives
/// storage) are counted into `failures` instead of silently skipped —
/// the drop class rm-753 closed for session rows, extended to the
/// enrichment lanes reading the same databases.
fn add_opencode_sqlite_messages(
    db: &Connection,
    aggs: &mut HashMap<String, SqliteSessionAgg>,
    failures: &mut SqliteFileFailures,
) {
    let Ok(mut stmt) = db.prepare("select session_id, data from message") else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return;
    };
    for row in rows {
        let (session_id, raw) = match row {
            Ok(pair) => pair,
            // rm-893: counted and sampled, never filter_map-dropped.
            Err(error) => {
                failures.record_dropped(&error);
                continue;
            }
        };
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
/// rm-893: rows of the part x message join that fail column decode
/// are counted into `failures` — the join reads the same message.data
/// and part.data columns the dedicated lanes read, so a mistyped
/// value trips every lane that reaches it and each trip is disclosed.
fn capture_opencode_user_text(
    db: &Connection,
    aggs: &mut HashMap<String, SqliteSessionAgg>,
    failures: &mut SqliteFileFailures,
) {
    // rm-753 perf rider: this used to `order by p.time_created`,
    // sorting the ENTIRE part table on every load to find each
    // session's earliest user text. One unordered pass keeping the
    // per-session minimum selects the same part without the
    // database-side sort. rm-954: ordering now honors SQLite's ASC
    // storage-class semantics — NULL first, then numeric values, then
    // wrong-typed TEXT/BLOB (sqlite_value_asc_rank) — where the lenient
    // i64 converter used to degrade wrong-typed values into the NULL
    // bucket; ties break by rowid.
    let Ok(mut stmt) = db.prepare(
        "select p.session_id, p.data, m.data, p.time_created, p.rowid from part p \
         join message m on p.message_id = m.id",
    ) else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            sqlite_value_asc_rank(row.get(3)?),
            row.get::<_, i64>(4)?,
        ))
    }) else {
        return;
    };
    let mut earliest: HashMap<String, ((i32, i64), i64, String)> = HashMap::new();
    for row in rows {
        let (session_id, part_raw, message_raw, time_created, rowid) = match row {
            Ok(quint) => quint,
            // rm-893: counted and sampled, never filter_map-dropped.
            Err(error) => {
                failures.record_dropped(&error);
                continue;
            }
        };
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
        if text.trim().is_empty() {
            continue;
        }
        let better = match earliest.get(&session_id) {
            None => true,
            Some((best_time, best_rowid, _)) => (time_created, rowid) < (*best_time, *best_rowid),
        };
        if better {
            earliest.insert(session_id, (time_created, rowid, text.to_string()));
        }
    }
    for (session_id, (_, _, text)) in earliest {
        if let Some(agg) = aggs.get_mut(&session_id) {
            if agg.first_user_text.is_empty() {
                agg.first_user_text = text;
            }
        }
    }
}

/// rm-893: part rows whose columns fail to decode are counted into
/// `failures` instead of silently skipped (same disclosure class as
/// the message lane above).
fn add_opencode_sqlite_parts(
    db: &Connection,
    aggs: &mut HashMap<String, SqliteSessionAgg>,
    failures: &mut SqliteFileFailures,
) {
    let Ok(mut stmt) = db.prepare("select session_id, data from part") else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return;
    };
    for row in rows {
        let (session_id, raw) = match row {
            Ok(pair) => pair,
            // rm-893: counted and sampled, never filter_map-dropped.
            Err(error) => {
                failures.record_dropped(&error);
                continue;
            }
        };
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

/// rm-893: grouped role rows that fail column decode (e.g. a non-TEXT
/// storage class where the lane reads TEXT — BLOBs bypass column
/// affinity and survive storage) are counted into `failures`
/// instead of silently skipped — role counts feed session messaging
/// metrics, and a vanished group is a silent mis-report.
fn sqlite_role_counts(
    db: &Connection,
    table: &str,
    session_column: &str,
    role_column: &str,
    failures: &mut SqliteFileFailures,
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
    for row in rows {
        let (session_id, role, count) = match row {
            Ok(triple) => triple,
            // rm-893: counted and sampled, never filter_map-dropped.
            Err(error) => {
                failures.record_dropped(&error);
                continue;
            }
        };
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

/// Per-session hermes tool outcome counts, derived from the `messages`
/// table. `results` counts observed `role='tool'` rows; `failures`
/// counts the rows hermes renders as tool errors. The hermes executor
/// writes a failing result as `§<id>§ Error executing tool '<name>':
/// <reason>` (an untagged leading phrase is tolerated for older
/// renders) — verified against a live state.db where the marker matches
/// 317 tool rows and zero rows of any other role, while
/// `messages.effect_disposition` stays NULL for ~all rows and cannot
/// serve as the outcome signal. Sessions row `tool_call_count` counts
/// calls, never outcomes (rm-198).
#[derive(Debug, Default)]
struct HermesToolOutcomes {
    results: usize,
    failures: usize,
}

/// rm-893: grouped tool-outcome rows that fail column decode are
/// counted into `failures` instead of silently skipped — the failure
/// tallies drive the ok/fail split, and a dropped group silently
/// flips a session to all-ok.
fn hermes_tool_outcome_counts(
    db: &Connection,
    failures: &mut SqliteFileFailures,
) -> HashMap<String, HermesToolOutcomes> {
    if !sqlite_has_column(db, "messages", "content") {
        return HashMap::new();
    }
    let sql = "select session_id, count(*), \
               sum(case when (content like '§_%§ Error executing tool %' \
                            or content like 'Error executing tool %') \
                        then 1 else 0 end) \
               from messages where role = 'tool' group by session_id";
    let Ok(mut stmt) = db.prepare(sql) else {
        return HashMap::new();
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<i64>>(2)?.unwrap_or(0),
        ))
    }) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for row in rows {
        let (session_id, results, failed) = match row {
            Ok(triple) => triple,
            // rm-893: counted and sampled, never filter_map-dropped.
            Err(error) => {
                failures.record_dropped(&error);
                continue;
            }
        };
        out.insert(
            session_id,
            HermesToolOutcomes {
                results: results.max(0) as usize,
                failures: failed.max(0) as usize,
            },
        );
    }
    out
}

/// rm-1509 (cycle 2, run b01c2b6a): the OpenCode v2 divergence check
/// — the `summary_diffs` per-file sums against the integer `summary_*`
/// columns of the same session row. `Some` only when both sides are
/// known, the per-file list carries at least one entry, and the sums
/// disagree: a writer that recorded both shapes disagreed with
/// itself, and the divergence is disclosed on Metrics (and in
/// delivery evidence), never silently reconciled into either side —
/// the same recorded-vs-derived failure class ccusage #1883 closed
/// with a float tolerance. An explicitly empty list (`[]`, sums 0/0)
/// never flags: the writer marked no files, and absence is not a
/// disagreement. Sums are saturating — a hostile payload of
/// i64::MAX entries cannot overflow the disclosure arithmetic.
fn summary_diffs_divergence(agg: &SqliteSessionAgg) -> Option<SummaryDiffDivergence> {
    let diffs = agg.summary_diffs.as_ref()?;
    if diffs.is_empty() {
        return None;
    }
    let (additions, deletions) = diffs
        .iter()
        .fold((0i64, 0i64), |(additions, deletions), diff| {
            (
                additions.saturating_add(diff.additions),
                deletions.saturating_add(diff.deletions),
            )
        });
    let summary_additions = agg.summary_additions?;
    let summary_deletions = agg.summary_deletions?;
    if additions == summary_additions && deletions == summary_deletions {
        return None;
    }
    Some(SummaryDiffDivergence {
        per_file_additions: additions,
        per_file_deletions: deletions,
        summary_additions,
        summary_deletions,
    })
}

fn session_from_sqlite_agg(agg: SqliteSessionAgg) -> Session {
    // rm-1509: per-file sums vs the integer columns, computed before
    // the aggregate is partially moved into the Metrics literal below.
    let summary_diffs_divergence = summary_diffs_divergence(&agg);
    // rm-1926: a row with a decoded payload but unreadable integers
    // discloses WHY the cross-check cannot run instead of silently
    // skipping it — `summary_diffs_divergence`'s early-`?` seam read
    // missing integers as "nothing to compare", hiding exactly the
    // writer defect the cross-check exists to catch.
    let summary_cross_check_unreadable =
        if agg.summary_diffs.is_some() && !agg.summary_unreadable.is_empty() {
            Some(agg.summary_unreadable.clone())
        } else {
            None
        };
    // rm-1926: the unreadable-column count, hoisted before the
    // Metrics literal takes ownership — see the counter bump below.
    let summary_cross_check_unreadable_count =
        summary_cross_check_unreadable.as_ref().map_or(0, Vec::len);
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
        // rm-610: stored sqlite sessions carry no per-turn tier;
        // aggregate estimates stay at standard rates.
        None,
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
        // rm-991 (cycle 2, run b5affead): the upstream-recorded agent
        // name and diff summary ride along on the metrics; the
        // `by_agent` rollup splits on the name (falling back to
        // `source_tool` when empty) and `files` provenance upgrades
        // only when the writer actually reported a summary_files.
        agent_name: agg.agent_name,
        summary_additions: agg.summary_additions,
        summary_deletions: agg.summary_deletions,
        summary_files: agg.summary_files,
        summary_diffs: agg.summary_diffs,
        summary_diffs_dropped: agg.summary_diffs_dropped,
        summary_diffs_divergence,
        summary_cross_check_unreadable,
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
            files: if agg.summary_files.is_some() {
                "reported_by_agent".to_string()
            } else {
                "unavailable".to_string()
            },
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
    // rm-790: the source row id makes the derived-history identity per-row
    // (two same-second sessions from one database no longer hash to one
    // history record). rm-791: the raw parent row id is the pre-link marker
    // that `attribute_subagents` resolves into linkage + rollups after
    // load (subagents.rs) — never a stale link on its own.
    metrics.session_key = agg.id.clone();
    metrics.parent_session = agg.parent_id.unwrap_or_default();
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
    // rm-1926: the unreadable cross-check is a disclosure-counter
    // event like `opencode_summary_dropped` — the doctor census and
    // diagnostics fold read it off the counters, so warm and cold
    // paths disclose identically (snapshot schema 15 round-trips it).
    if summary_cross_check_unreadable_count > 0 {
        metrics
            .disclosure_counters
            .entry("opencode_summary_cross_check_unreadable".to_string())
            .and_modify(|count| *count += summary_cross_check_unreadable_count)
            .or_insert(summary_cross_check_unreadable_count);
    }
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
        branch: String::new(),
        // rm-1405 reap (run 4f2af62e5b7e): the sqlite aggregate lane
        // has no identity dimension; org stays empty and the by_org
        // facet leaves the session out (zero-noise contract).
        org: String::new(),
        org_email: String::new(),
        metrics,
        anomalies,
        health,
        tool_warnings: Vec::new(),
        diagnostics: crate::Diagnostics::default(),
    }
}

fn token_cost_raw(input: i64, output: i64, cache_write: i64, cache_read: i64, model: &str) -> f64 {
    // rm-610: sqlite stores carry no per-turn service tier; standard rates.
    token_cost(input, output, cache_write, cache_read, model, None)
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

/// rm-954: SQLite ASC orders NULL first, then numeric values by
/// magnitude, then TEXT/BLOB. The lenient i64 reader folds wrong-typed
/// values into the NULL bucket they never shared, so the earliest-part
/// selection ranks them explicitly: NULL (0) < numeric (1) < wrong-typed
/// (2), ties by rowid — matching what `order by time_created` would have
/// picked instead of inverting it.
fn sqlite_value_asc_rank(value: Option<rusqlite::types::Value>) -> (i32, i64) {
    use rusqlite::types::Value as SqliteValue;
    match value {
        None => (0, 0),
        Some(SqliteValue::Integer(number)) => (1, number),
        Some(SqliteValue::Real(number)) => (1, number as i64),
        Some(SqliteValue::Text(text)) => match text.parse::<i64>() {
            Ok(number) => (1, number),
            Err(_) => (2, 0),
        },
        Some(_) => (2, 0),
    }
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

/// rm-1926 (run 9d3b927c cycle 1): WHY a schema-present `summary_*`
/// integer failed to read — SQL NULL storage, BLOB storage, or
/// non-numeric TEXT (SQLite columns are dynamically typed; INTEGER
/// affinity coerces what it can, the rest survives in its written
/// storage class). `None` when the value reads fine or the column is
/// absent: an absent column is the 1.x schema behind the
/// null-padding probe, not a broken v2 write, and 1.x rows keep
/// parsing byte-identically.
fn sqlite_i64_unreadable_reason(
    value: Option<&rusqlite::types::Value>,
    column_present: bool,
) -> Option<&'static str> {
    if !column_present {
        return None;
    }
    let value = match value {
        // SQL NULL: `row.get::<_, Option<Value>>` yields None —
        // present (the schema has the column) but unreadable.
        None => return Some("stored as NULL"),
        Some(value) => value,
    };
    if sqlite_value_as_i64(Some(value.clone())).is_some() {
        return None;
    }
    Some(match value {
        rusqlite::types::Value::Blob(_) => "stored as BLOB",
        rusqlite::types::Value::Text(_) => "non-numeric text",
        _ => "unreadable storage class",
    })
}

/// rm-1927 (run 9d3b927c cycle 1): does this database contain an
/// opencode session-store-shaped table under a non-v1 name? Keys on
/// the column SHAPE — `id`, `time_created`, `time_updated`,
/// `directory`, the signature shared by the v1 `session` table and
/// the dev-branch `session_v2` DDL
/// (anomalyco/opencode@dev packages/core/src/database/schema.gen.ts)
/// — not on the table name (rm-507 discipline), so an upstream rename
/// keeps the disclosure truthful. Reached only on the
/// no-`session`-table path, so the pragma walk costs nothing on
/// healthy v1 stores.
/// rm-1927 (run 9d3b927c cycle 1; read arm rm-2290): does this
/// database contain an opencode session-store-shaped table under a
/// non-v1 name? Keys on the column SHAPE — `id`, `time_created`,
/// `time_updated`, `directory`, the signature shared by the v1
/// `session` table and the `session_v2` DDL
/// (sst/opencode@dev packages/core/src/database/schema.gen.ts) — not
/// on the table name (rm-507 discipline), so an upstream rename keeps
/// the disclosure truthful. The v1 `session` table itself is EXCLUDED
/// BY NAME from this probe — that name is the one table the v1 lane
/// legitimately owns by name, so the v2 read arm (rm-2290, which now
/// runs on every fresh pass, not only the no-`session`-table path)
/// never re-reads v1 rows as v2 twins. Since the read arm landed, a
/// probe hit is INGESTED; only a table that matches the shape but
/// fails the guarded read stays the unhandled class.
fn opencode_v2_shaped_table(db: &Connection) -> Option<String> {
    let names = db
        .prepare("select name from sqlite_master where type = 'table' and name not like 'sqlite_%' and name != 'session'")
        .and_then(|mut stmt| {
            let names = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(names)
        })
        .ok()?;
    for name in names {
        let quoted = format!("\"{}\"", name.replace('"', "\"\""));
        let columns = db
            .prepare(&format!("pragma table_info({quoted})"))
            .and_then(|mut stmt| {
                let columns = stmt
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(columns)
            })
            .unwrap_or_default();
        if ["id", "time_created", "time_updated", "directory"]
            .iter()
            .all(|needed| columns.iter().any(|column| column == needed))
        {
            return Some(name);
        }
    }
    None
}

/// rm-791: lenient TEXT reader for the opencode `parent_id` column —
/// only a real TEXT row id yields linkage; NULL, wrong-typed or blob
/// values degrade to "no parent" instead of failing the row.
fn sqlite_value_as_text(value: Option<rusqlite::types::Value>) -> Option<String> {
    use rusqlite::types::Value as SqliteValue;
    match value {
        Some(SqliteValue::Text(text)) => Some(text),
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

    /// rm-991: two golden schemas over the SAME read-only query —
    ///
    /// - legacy (pre-agent) opencode `session` table: the guarded
    ///   columns are absent; every session still loads, attribution
    ///   stays empty (`agent_name`), summary counters stay absent,
    ///   and the overview `by_agent` key is the plain source tool —
    ///   byte-identical behavior to before this row.
    /// - current live schema (verified on the fleet host db,
    ///   2026-10-12: 305/310 sessions carry `agent`, 297/310 the
    ///   summary triple): `agent` splits the by_agent rollup into
    ///   bare agent-name buckets (the landed rm-991 lane keys on the
    ///   upstream name directly — this integration kept run
    ///   b5affead's mechanics over run b5d391ec's stranded
    ///   `opencode_db:<agent>` variant), and the summarizer counters
    ///   land on Metrics for the inspect-class surfaces.
    ///
    /// The wrong-typed-value leg pins the lenient readers: a corrupt
    /// agent cell degrades to unnamed instead of dropping the
    /// session row (the drop class rm-753 closed for session rows).
    #[test]
    fn opencode_agent_and_summary_columns_load_across_schemas() {
        let dir = std::env::temp_dir().join(format!(
            "agenttrace-rm991-agent-columns-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("tempdir");
        let path = dir.join("opencode.db");
        let db = Connection::open(&path).expect("open fixture db");
        // Legacy schema: cost/token columns only — no parent_id, no
        // agent, no summary triple. This is the pre-1.18-era shape the
        // guarded SELECT must keep loading.
        db.execute_batch(
            r#"
            create table session (
                id text primary key,
                title text,
                time_created integer,
                time_updated integer,
                directory text,
                cost real,
                tokens_input integer,
                tokens_output integer,
                tokens_reasoning integer,
                tokens_cache_read integer,
                tokens_cache_write integer
            );
            insert into session values
                ('legacy-1', 'Legacy session', 1700000000000, 1700000600000, '/w/legacy', null, 10, 20, 0, 0, 0);
            "#,
        )
        .expect("seed legacy schema");
        let (aggs, failures, _) = opencode_sqlite_session_rows(&db, &path, None);
        assert!(failures.is_clean(), "legacy schema must load clean");
        let legacy = aggs.get("legacy-1").expect("legacy row loads");
        assert_eq!(legacy.agent_name, "");
        assert_eq!(legacy.summary_additions, None);
        let legacy_session = session_from_sqlite_agg(SqliteSessionAgg {
            id: "legacy-1".to_string(),
            ..SqliteSessionAgg::default()
        });
        assert_eq!(legacy_session.metrics.agent_name, "");
        assert_eq!(legacy_session.metrics.summary_additions, None);

        // Current schema: agent + parent_id + summary triple, one
        // clean attributed row, one unattributed row, one WRONG-TYPED
        // agent cell (a BLOB bypasses column affinity — the leniency
        // leg rm-893 pinned for messages, here for the agent column).
        let path2 = dir.join("opencode2.db");
        let db2 = Connection::open(&path2).expect("open fixture db 2");
        db2.execute_batch(
            r#"
            create table session (
                id text primary key,
                title text,
                time_created integer,
                time_updated integer,
                directory text,
                cost real,
                tokens_input integer,
                tokens_output integer,
                tokens_reasoning integer,
                tokens_cache_read integer,
                tokens_cache_write integer,
                parent_id text,
                agent text,
                summary_additions integer,
                summary_deletions integer,
                summary_files integer
            );
            insert into session (id, title, time_created, time_updated, directory, agent, summary_additions, summary_deletions, summary_files) values
                ('cur-1', 'Build session', 1700000000000, 1700000600000, '/w/cur', 'build', 120, 30, 7),
                ('cur-2', 'Anonymous session', 1700000700000, 1700001200000, '/w/cur', null, null, null, null),
                ('cur-3', 'Corrupt agent cell', 1700001400000, 1700002000000, '/w/cur', x'00ff', 1, 0, 1);
            "#,
        )
        .expect("seed current schema");
        let (aggs2, failures2, _) = opencode_sqlite_session_rows(&db2, &path2, None);
        assert!(failures2.is_clean(), "current schema must load clean");
        let attributed = aggs2.get("cur-1").expect("attributed row loads");
        assert_eq!(attributed.agent_name, "build");
        assert_eq!(attributed.summary_additions, Some(120));
        assert_eq!(attributed.summary_deletions, Some(30));
        assert_eq!(attributed.summary_files, Some(7));
        assert_eq!(
            aggs2.get("cur-2").map(|row| row.agent_name.clone()),
            Some("".to_string()),
            "NULL agent stays unnamed"
        );
        assert_eq!(
            aggs2.get("cur-3").map(|row| row.agent_name.clone()),
            Some("".to_string()),
            "wrong-typed agent degrades to unnamed, row survives"
        );

        // The rollup split: attributed sessions fold under the bare
        // upstream agent name, unattributed keep the plain source
        // tool, and the sum of bucket session counts still equals the
        // total (additive split, no double counting).
        let sessions: Vec<crate::Session> = aggs2
            .into_values()
            .map(|mut agg| {
                agg.source_tool = "opencode_db".to_string();
                agg.path = path2.to_string_lossy().to_string();
                session_from_sqlite_agg(agg)
            })
            .collect();
        let overview = crate::compute_overview(&sessions);
        assert_eq!(overview.total_sessions, 3);
        assert_eq!(overview.by_agent.get("build").unwrap().sessions, 1);
        assert_eq!(overview.by_agent.get("opencode_db").unwrap().sessions, 2);
        let bucket_total: usize = overview.by_agent.values().map(|group| group.sessions).sum();
        assert_eq!(bucket_total, 3, "split must stay additive");
        let build_session = sessions
            .iter()
            .find(|session| session.metrics.agent_name == "build")
            .expect("build session carries agent_name on Metrics");
        assert_eq!(build_session.metrics.summary_additions, Some(120));
        assert_eq!(build_session.metrics.summary_files, Some(7));
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

    /// Minimal hermes `state.db` fixture: two sessions. `s1` recorded two
    /// tool calls on the session row and has three messages rows — one
    /// clean tool result and one hermes-rendered tool error. `s2` recorded
    /// two tool calls but retained no message rows.
    fn hermes_state_db_fixture(dir: &Path) -> PathBuf {
        let path = dir.join("state.db");
        let db = Connection::open(&path).expect("open fixture db");
        db.execute_batch(
            r#"
            create table sessions (
                id text primary key,
                model text,
                started_at real,
                ended_at real,
                message_count integer,
                tool_call_count integer,
                input_tokens integer,
                output_tokens integer,
                cache_read_tokens integer,
                cache_write_tokens integer,
                cwd text
            );
            create table messages (
                id integer primary key,
                session_id text,
                role text,
                content text
            );
            insert into sessions values
                ('s1', 'claude-sonnet-4', 1700000000.0, 1700000600.0, 5, 2, 10, 20, 0, 0, '/work/x'),
                ('s2', 'claude-sonnet-4', 1700000700.0, 1700001200.0, 3, 2, 10, 20, 0, 0, '/work/x');
            insert into messages (session_id, role, content) values
                ('s1', 'user', 'go'),
                ('s1', 'assistant', 'doing'),
                ('s1', 'tool', '§7§ {"output": "ok"}'),
                ('s1', 'tool', '§8§ Error executing tool ''terminal'': timed out after 420.0'),
                ('s2', 'user', 'go');
            "#,
        )
        .expect("seed fixture");
        path
    }

    #[test]
    fn hermes_tool_outcomes_come_from_result_rows_not_the_call_count() {
        // rm-198 (golden boundary): pre-fix, tool_calls_total and
        // tool_calls_ok both read sessions.tool_call_count, so hermes
        // sessions claimed every tool call succeeded and
        // tool_calls_fail was structurally 0. Post-fix, the split comes
        // from the messages table: observed results, minus the rows
        // hermes renders as tool errors.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-rm198-metrics-{}-{}",
            std::process::id(),
            1981
        ));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = hermes_state_db_fixture(&root);
        let (sessions, failures) = query_hermes_sqlite_sessions(&path, None, None);
        std::fs::remove_dir_all(&root).ok();
        assert_eq!(sessions.len(), 2, "both fixture sessions must load");
        assert!(
            failures.is_clean(),
            "healthy fixture must report zero failures: {failures:?}"
        );

        let s1 = sessions
            .iter()
            .find(|session| session.name == "s1")
            .expect("s1");
        assert_eq!(s1.metrics.tool_calls_total, 2, "calls from session row");
        assert_eq!(s1.metrics.tool_results, 2, "observed tool result rows");
        assert_eq!(s1.metrics.tool_calls_ok, 1, "clean result rows");
        assert_eq!(
            s1.metrics.tool_calls_fail, 1,
            "hermes-rendered error rows must count as failures"
        );

        let s2 = sessions
            .iter()
            .find(|session| session.name == "s2")
            .expect("s2");
        assert_eq!(s2.metrics.tool_calls_total, 2, "calls from session row");
        assert_eq!(s2.metrics.tool_results, 0, "no retained message rows");
        assert_eq!(
            s2.metrics.tool_calls_ok, 0,
            "no fabricated success split when no results were observed"
        );
        assert_eq!(s2.metrics.tool_calls_fail, 0);
    }

    #[test]
    #[cfg(unix)]
    fn failed_hermes_read_never_persists_or_serves_an_empty_snapshot() {
        // rm-626 red-first (the home1 poison recipe): a database that
        // cannot be read must not persist a snapshot. Pre-fix, the
        // failed query returned Vec::new() and load stored it as the
        // database's snapshot with a live fingerprint — and because the
        // chmod 000 -> 644 recovery keeps mtime/size, that empty
        // snapshot kept matching the fingerprint and masked the
        // database on every later run until the cache was wiped.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-rm626-poison-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("tempdir");
        let database = root.join("state.db");
        let hermes_snapshot = root.join("hermes-sqlite.json");
        let opencode_database = root.join("storage.db");
        let opencode_snapshot = root.join("opencode-sqlite.json");

        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Leg 1 — a garbage database file: a deterministic query
        // failure everywhere (including containers where chmod cannot
        // stop a root reader). Nothing may be stored for the failure,
        // on either the hermes or the opencode lane; the wall's
        // disclosure shape (rm-753) must also carry the failure — the
        // store gate is `failures.is_clean()`.
        std::fs::write(&database, b"not a sqlite database at all").expect("write garbage db");
        let mut report = SqliteIngestReport::default();
        assert!(load_hermes_sqlite_sessions(&database, None, &mut report, None).is_empty());
        assert!(
            !hermes_snapshot.exists(),
            "a failed hermes read must not persist any snapshot"
        );
        std::fs::write(&opencode_database, b"not a sqlite database either")
            .expect("write garbage opencode db");
        let mut opencode_report = SqliteIngestReport::default();
        assert!(load_opencode_sqlite_sessions(
            &opencode_database,
            None,
            &mut opencode_report,
            None
        )
        .is_empty());
        assert!(
            !opencode_snapshot.exists(),
            "a failed opencode read must not persist any snapshot"
        );
        assert!(
            !opencode_report.unreadable.is_empty() && !report.unreadable.is_empty(),
            "the unreadable databases must be disclosed, not nulled"
        );

        // Leg 2 — the original poison sequence: chmod 000 -> chmod 644
        // leaves mtime/size untouched, so a stored empty snapshot would
        // keep matching the fingerprint forever. Skipped when the
        // runner reads through 000-mode files (root).
        {
            use std::os::unix::fs::PermissionsExt;
            let fixture_dir = root.join("fixture");
            std::fs::create_dir_all(&fixture_dir).expect("fixture tempdir");
            let fixture = hermes_state_db_fixture(&fixture_dir);
            std::fs::set_permissions(&fixture, std::fs::Permissions::from_mode(0o000))
                .expect("chmod 000");
            let blocked = std::fs::read(&fixture).is_err();
            std::fs::set_permissions(&fixture, std::fs::Permissions::from_mode(0o644))
                .expect("chmod 644");
            if blocked {
                std::fs::rename(&fixture, &database).expect("move fixture into place");
                std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o000))
                    .expect("chmod 000");
                assert!(load_hermes_sqlite_sessions(
                    &database,
                    None,
                    &mut SqliteIngestReport::default(),
                    None
                )
                .is_empty());
                assert!(
                    !hermes_snapshot.exists(),
                    "the chmod-000 failure must not persist either"
                );
                std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o644))
                    .expect("chmod 644");
                let recovered = load_hermes_sqlite_sessions(
                    &database,
                    None,
                    &mut SqliteIngestReport::default(),
                    None,
                );
                assert_eq!(
                    recovered.len(),
                    2,
                    "the healthy database must parse again without wiping the cache"
                );
                assert!(
                    hermes_snapshot.exists(),
                    "the successful re-read must populate the snapshot"
                );
            } else {
                eprintln!("skipping chmod legs: runner reads through 000-mode files");
            }
        }

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn busy_timeout_waits_out_a_short_lock_and_fails_closed_without_storing_on_a_long_one() {
        // rm-626 acceptance pin (independent_review 7d51a912 M1, landed at
        // review-fix d48c5af7): the busy_timeout on open_sqlite_read_only
        // is part of the fail-closed contract and must be pinned by a real
        // concurrent-writer contention test, not only by the corruption
        // legs. A SHORT exclusive lock (released inside SQLITE_BUSY_TIMEOUT)
        // must be waited out; a LONG lock (held across the load) must fail
        // closed AND persist nothing — the same no-store contract as the
        // corruption legs, reached through the contention door. Adapted to
        // this wall at integration: the load carries the rm-753
        // SqliteIngestReport (3-arg call) and the no-store gate is
        // `failures.is_clean()` on the disclosed failure.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-rm626-contention-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("tempdir");
        let database = hermes_state_db_fixture(&root);
        let snapshot = root.join("hermes-sqlite.json");

        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        use std::sync::mpsc;

        // Leg 0 — the configured timeout itself. rusqlite installs a
        // 5,000ms default busy timeout on every connection it opens
        // (rusqlite 0.32.1 sets sqlite3_busy_timeout(db, 5000) in
        // inner_connection.rs), so deleting our busy_timeout call does
        // NOT make locked reads fail fast — it silently degrades every
        // contended statement from a 250ms wait to a 5s wait. The
        // behavioral legs below cannot see that regression (a short
        // lock is waited out either way; the long-lock leg just stalls
        // ~20s instead of ~1s), so the constant is pinned by reading it
        // back off the connection the loader actually opens.
        let pinned_timeout = open_sqlite_read_only_bounded(&database, None)
            .map(|(connection, _watch)| connection)
            .expect("open read-only connection for the pragma probe")
            .query_row("pragma busy_timeout", [], |row| row.get::<_, i64>(0))
            .expect("read busy_timeout pragma");
        assert_eq!(
            pinned_timeout,
            SQLITE_BUSY_TIMEOUT.as_millis() as i64,
            "open_sqlite_read_only_bounded must keep SQLITE_BUSY_TIMEOUT (250ms); \
             rusqlite's silent 5s default is the regression this pins out"
        );

        // Leg 1 — SHORT lock: released after 100ms (< SQLITE_BUSY_TIMEOUT
        // 250ms), the loader must wait it out and the load must succeed.
        // Together with leg 0 this pins busy_timeout: leg 0 pins the
        // configured wait budget, this leg pins that a short lock is
        // actually waited out rather than failing the read.
        let (locked_tx, locked_rx) = mpsc::channel::<()>();
        let short_lock = {
            let database = database.clone();
            std::thread::spawn(move || {
                let writer = rusqlite::Connection::open(&database).expect("open writer");
                writer
                    .execute_batch("BEGIN EXCLUSIVE")
                    .expect("take exclusive lock");
                locked_tx.send(()).expect("signal lock held");
                std::thread::sleep(std::time::Duration::from_millis(100));
                writer
                    .execute_batch("ROLLBACK")
                    .expect("release short lock");
            })
        };
        locked_rx.recv().expect("wait for the short lock");
        let short_sessions =
            load_hermes_sqlite_sessions(&database, None, &mut SqliteIngestReport::default(), None);
        short_lock.join().expect("short writer thread");
        assert_eq!(
            short_sessions.len(),
            2,
            "a short exclusive lock must be waited out and the load succeed"
        );
        assert!(
            snapshot.exists(),
            "the waited-out load must persist its snapshot"
        );

        // Leg 2 — LONG lock: held across the whole load; the loader must
        // fail closed and must NOT persist any snapshot — contention is a
        // MISS, never a store.
        std::fs::remove_file(&snapshot).expect("wipe snapshot between legs");
        let (held_tx, held_rx) = mpsc::channel::<()>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let long_lock = {
            let database = database.clone();
            std::thread::spawn(move || {
                let writer = rusqlite::Connection::open(&database).expect("open writer");
                writer
                    .execute_batch("BEGIN EXCLUSIVE")
                    .expect("take exclusive lock");
                held_tx.send(()).expect("signal lock held");
                release_rx.recv().expect("wait for release");
                let _ = writer.execute_batch("ROLLBACK");
            })
        };
        held_rx.recv().expect("wait for the long lock");
        let long_sessions =
            load_hermes_sqlite_sessions(&database, None, &mut SqliteIngestReport::default(), None);
        assert!(
            long_sessions.is_empty(),
            "a permanently locked database must fail closed to zero sessions"
        );
        assert!(
            !snapshot.exists(),
            "a contended hermes read must not persist any snapshot"
        );
        release_tx.send(()).expect("release the long lock");
        long_lock.join().expect("long writer thread");

        // Leg 3 — recovery: once the lock is gone the same database loads
        // and stores normally again.
        let recovered =
            load_hermes_sqlite_sessions(&database, None, &mut SqliteIngestReport::default(), None);
        assert_eq!(
            recovered.len(),
            2,
            "after the lock releases the database must load normally"
        );
        assert!(
            snapshot.exists(),
            "the recovered load must persist its snapshot"
        );

        match prior_cache {
            Some(v) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", v),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn hermes_tool_failures_trip_the_overview_gate() {
        // rm-198 end-to-end leg: fixture metrics -> tool_fail_rate ->
        // evaluate_overview_gate (--max-tool-fail-rate). Pre-fix the rate
        // is always 0 for hermes-sourced sessions, so the shipped gate
        // could never trip on them.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-rm198-gate-{}-{}",
            std::process::id(),
            1982
        ));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = hermes_state_db_fixture(&root);
        let (sessions, _failures) = query_hermes_sqlite_sessions(&path, None, None);
        std::fs::remove_dir_all(&root).ok();

        // s1: ok 1 / fail 1 -> 50% across the corpus (s2 contributes no
        // observed outcomes, matching the parser convention).
        let rate = crate::tool_fail_rate(&sessions);
        assert!((rate - 50.0).abs() < 1e-9, "tool_fail_rate was {rate}");
        let failures = crate::evaluate_overview_gate(
            &crate::Overview::default(),
            &sessions,
            0,
            false,
            Some(40.0),
        );
        assert!(
            failures
                .iter()
                .any(|message| message.contains("tool failure rate")),
            "--max-tool-fail-rate must trip on hermes-sourced failures: {failures:?}"
        );
    }

    /// Minimal opencode `opencode.db` with parts deliberately out of
    /// chronological insertion order and a NULL time_created — the
    /// rm-753 perf-rider corpus (the load must select the same
    /// earliest user text without the removed `order by`).
    fn opencode_user_text_fixture(dir: &Path, with_null_time: bool) -> PathBuf {
        let path = dir.join("opencode.db");
        let db = Connection::open(&path).expect("open fixture db");
        db.execute_batch(
            r#"
            create table session (
                id text primary key,
                title text,
                time_created integer,
                time_updated integer
            );
            create table message (
                id text primary key,
                session_id text,
                data text
            );
            create table part (
                id text primary key,
                message_id text,
                session_id text,
                data text,
                time_created integer
            );
            insert into session (id, title, time_created, time_updated) values
                ('s1', '', 1762000000000, 1762000000001);
            insert into message (id, session_id, data) values
                ('m1', 's1', '{"role":"user"}'),
                ('m2', 's1', '{"role":"user"}'),
                ('m3', 's1', '{"role":"user"}'),
                ('m4', 's1', '{"role":"user"}');
            "#,
        )
        .expect("seed session/message");
        let parts: &[(&str, &str, &str)] = if with_null_time {
            // NULL time sorts FIRST under SQLite ASC semantics, so the
            // untimestamped part wins the earliest-user-text slot.
            &[
                ("p1", "third by time", "300000000"),
                ("p2", "first by time", "100000000"),
                ("p3", "no timestamp", "NULL"),
                ("p4", "tie, later rowid", "100000000"),
            ]
        } else {
            &[
                ("p1", "third by time", "300000000"),
                ("p2", "first by time", "100000000"),
                ("p4", "tie, later rowid", "100000000"),
            ]
        };
        let mut parts_sql = String::new();
        for (part, text, time_created) in parts {
            let data = format!("{{\"type\":\"text\",\"text\":\"{text}\"}}");
            parts_sql.push_str(&format!(
                "insert into part (id, message_id, session_id, data, time_created) \
                 values ('{part}', 'm1', 's1', '{data}', {time_created});\n"
            ));
        }
        db.execute_batch(&parts_sql).expect("seed parts");
        path
    }

    #[test]
    fn opencode_user_text_selection_survives_without_the_order_by() {
        // rm-753 perf rider: the earliest-user-text pass used to sort
        // the whole part table via `order by p.time_created`. The
        // unordered pass keeps the (time_created, rowid) minimum, so
        // the selected text must be identical: NULL time first, then
        // ascending value, ties by rowid.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-rm753-usertext-{}-a",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = opencode_user_text_fixture(&root, true);
        let (sessions, failures, _fork_excluded) =
            query_opencode_sqlite_sessions(&path, None, None);
        std::fs::remove_dir_all(&root).ok();
        assert!(failures.is_clean(), "corpus must load clean: {failures:?}");
        assert_eq!(sessions.len(), 1, "one session in the fixture");
        // NULL time_created sorts before every timestamp under SQLite
        // ASC semantics — the untimestamped part wins.
        assert_eq!(
            sessions[0].name, "no timestamp",
            "empty title means message-derived naming from the earliest part"
        );

        let root = std::env::temp_dir().join(format!(
            "agenttrace-rm753-usertext-{}-b",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = opencode_user_text_fixture(&root, false);
        let (sessions, failures, _fork_excluded) =
            query_opencode_sqlite_sessions(&path, None, None);
        std::fs::remove_dir_all(&root).ok();
        assert!(failures.is_clean(), "corpus must load clean: {failures:?}");
        // Same minimum time (100000000) on two parts: the lower rowid
        // ("first by time", inserted before "tie, later rowid") wins,
        // exactly as the ordered scan resolved ties before.
        assert_eq!(
            sessions[0].name, "first by time",
            "ties resolve by rowid, matching the previous ordered scan"
        );
    }

    #[test]
    fn failed_loads_neither_store_nor_trust_an_empty_snapshot() {
        // rm-753 poison class: a load that dropped rows used to cache
        // its PARTIAL result and an unreadable database used to cache
        // an EMPTY one — after which every later run hit the snapshot
        // and the failure never re-appeared. Two pins: (a) a failed
        // load stores nothing; (b) even a pre-existing empty snapshot
        // (written by a pre-fix binary) is distrusted and re-verified.
        // The cache location is env-derived, so the test isolates it
        // (and restores on exit). It takes the SHARED test_env lock,
        // not a private static: per-module locks let any other
        // env-mutating test (pricing/session-cache/statusline) re-point
        // or remove AGENTTRACE_SESSION_CACHE_DIR mid-window, resolving
        // session_cache_path() to the REAL host cache (where a real
        // opencode-sqlite.json snapshot lives) — a latent full-suite
        // flake caught while stress-revalidating review fix b45f8690 —
        // the exact re-pointing class `test_env` documents and closes.
        struct EnvRestore(Vec<(&'static str, Option<std::ffi::OsString>)>);
        impl Drop for EnvRestore {
            fn drop(&mut self) {
                for (key, value) in self.0.iter() {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
        // Declared before EnvRestore so the env is restored (Drop)
        // while the lock is still held.
        let _guard = crate::test_env::lock_env();
        let _restore = EnvRestore(
            [
                (
                    "AGENTTRACE_SESSION_CACHE_DIR",
                    std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR"),
                ),
                ("XDG_CACHE_HOME", std::env::var_os("XDG_CACHE_HOME")),
                ("HOME", std::env::var_os("HOME")),
            ]
            .to_vec(),
        );
        let root =
            std::env::temp_dir().join(format!("agenttrace-rm753-poison-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("tempdir");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let path = opencode_hostile_fixture(&root);

        // (a) hostile load through the snapshot-caching entry point:
        // 4 decodable rows, 1 dropped row counted, no snapshot stored.
        let mut report = SqliteIngestReport::default();
        let sessions = load_opencode_sqlite_sessions(&path, None, &mut report, None);
        assert_eq!(sessions.len(), 4, "the four decodable rows load");
        assert_eq!(
            report.dropped_rows.len(),
            1,
            "the NULL-id ghost row is counted"
        );
        assert_eq!(report.dropped_rows[0].dropped, 1);
        let snapshot =
            crate::session_cache::session_cache_path().with_file_name("opencode-sqlite.json");
        assert!(
            !snapshot.exists(),
            "a failed load must not store a snapshot at {}",
            snapshot.display()
        );

        // (b) a pre-fix empty snapshot for the same database is
        // ignored instead of trusted: the row drop is re-detected.
        crate::session_cache::store_sqlite_snapshot(&path, "opencode", &[])
            .expect("hand-poison an empty snapshot");
        assert!(snapshot.exists(), "poisoned snapshot is in place");
        let mut report = SqliteIngestReport::default();
        let sessions = load_opencode_sqlite_sessions(&path, None, &mut report, None);
        assert_eq!(sessions.len(), 4, "decodable rows still load");
        assert_eq!(
            report.dropped_rows.len(),
            1,
            "empty snapshot must be distrusted so failures re-surface"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn corrupt_hermes_header_is_disclosed_and_never_banks_a_snapshot() {
        // rm-734 fixture class adopted onto the landed rm-753 report
        // channel: a hermes `state.db` carrying a sqlite magic header
        // but corrupt bytes behind it must ride `unreadable` (never
        // `Ok(empty)`), must bank no snapshot, and — after repair —
        // must be re-discovered on the very next read.
        let _env = crate::test_env::lock_env();
        let previous_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        let root =
            std::env::temp_dir().join(format!("agenttrace-rm734-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("tempdir");
        let cache = root.join("cache");
        std::fs::create_dir_all(&cache).expect("cache dir");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache);
        let snapshot = cache.join("hermes-sqlite.json");
        let path = root.join("state.db");
        std::fs::write(&path, b"SQLite format 3\x00corrupted-bytes").expect("write corrupt db");

        let mut report = SqliteIngestReport::default();
        let sessions = load_hermes_sqlite_sessions(&path, None, &mut report, None);
        assert!(
            sessions.is_empty(),
            "no sessions can come from a corrupt db"
        );
        assert_eq!(
            report.unreadable.len(),
            1,
            "the corrupt database must be disclosed as exactly one unreadable"
        );
        let unreadable = &report.unreadable[0];
        assert_eq!(unreadable.source, "hermes");
        assert_eq!(unreadable.path, path);
        assert!(
            !unreadable.reason.is_empty(),
            "the open error must be carried"
        );
        assert!(
            !snapshot.exists(),
            "the poison gate must not bank a snapshot for a failed read — \
             a banked empty would hide the database even after repair"
        );

        // Repair: the same path now holds the healthy fixture. The
        // failed read banked nothing, so the lane must re-probe it.
        std::fs::remove_file(&path).expect("drop corrupt db");
        let healthy = hermes_state_db_fixture(&root);
        assert_eq!(healthy, path, "fixture must land on the same path");
        let mut repaired_report = SqliteIngestReport::default();
        let repaired = load_hermes_sqlite_sessions(&path, None, &mut repaired_report, None);
        match previous_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            repaired_report.unreadable.is_empty() && repaired_report.dropped_rows.is_empty(),
            "a repaired database must read cleanly: {repaired_report:?}"
        );
        assert_eq!(
            repaired.len(),
            2,
            "the repair must be discovered on the very next read — \
             no banked poison can mask it"
        );
    }

    // E0433 on windows-latest: the chmod-000 probe needs
    // PermissionsExt, so the whole leg is unix-only (salvage 9b2a7a1
    // U1; sibling c63084be rm-993).
    #[cfg(unix)]
    #[test]
    fn unreadable_permissions_are_disclosed_as_unreadable() {
        // rm-734 fixture class adopted onto the landed rm-753 report
        // channel: a chmod-000 database is found-but-unreadable, not
        // absent. Root can open anything, so the leg is skipped (the
        // mode is restored) when the probe itself opens cleanly.
        let root =
            std::env::temp_dir().join(format!("agenttrace-rm734-chmod-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = hermes_state_db_fixture(&root);
        let original = std::fs::metadata(&path)
            .expect("fixture metadata")
            .permissions();
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o000))
            .expect("chmod 000");
        let unreadable = Connection::open(&path).is_err();
        let mut report = SqliteIngestReport::default();
        let sessions = if unreadable {
            load_hermes_sqlite_sessions(&path, None, &mut report, None)
        } else {
            // Running as root: the disclosure assertion would be
            // vacuous, and the mode must be restored either way.
            Vec::new()
        };
        std::fs::set_permissions(&path, original).ok();
        let _ = std::fs::remove_dir_all(&root);
        if !unreadable {
            return; // running as root: nothing to prove here
        }
        assert!(sessions.is_empty());
        assert_eq!(
            report.unreadable.len(),
            1,
            "the chmod-000 database must surface as unreadable"
        );
        assert_eq!(report.unreadable[0].source, "hermes");
    }

    /// The P11 hostile corpus, re-crafted at runtime: five session rows
    /// of which the NULL-id ghost must drop counted, not silently.
    fn opencode_hostile_fixture(dir: &Path) -> PathBuf {
        let path = dir.join("opencode.db");
        let db = Connection::open(&path).expect("open hostile db");
        db.execute_batch(
            r#"
            create table session (
                id text primary key,
                title text,
                time_created integer,
                time_updated integer,
                tokens_input integer,
                tokens_output integer
            );
            insert into session (id, title, time_created, time_updated, tokens_input, tokens_output) values
                ('s-good', 'Good session', 1762000000000, 1762000000001, 100, 200),
                (NULL, 'Ghost session', 1762001000000, 1762001000001, 7, 8),
                ('s-texty', 'Type-abuse session', 'not-a-number', 1762002000001, 1, 2),
                ('s-neg', 'Negative tokens', 1762003000000, 1762003000001, -100, -5),
                ('s-huge', 'Huge tokens', 1762005000000, 1762005000001, 9223372036854775807, 9223372036854775807);
            "#,
        )
        .expect("seed hostile rows");
        path
    }

    #[test]
    fn hostile_opencode_rows_are_reported_or_counted_never_silently_dropped() {
        // The acceptance golden: every hostile session row either
        // loads or is counted in the failure report — none vanishes.
        let root =
            std::env::temp_dir().join(format!("agenttrace-rm753-golden-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = opencode_hostile_fixture(&root);
        let (sessions, failures, _fork_excluded) =
            query_opencode_sqlite_sessions(&path, None, None);
        std::fs::remove_dir_all(&root).ok();
        assert_eq!(sessions.len(), 4, "the four decodable rows load");
        assert_eq!(failures.dropped, 1, "the NULL-id ghost is counted");
        assert!(
            failures
                .dropped_sample
                .as_deref()
                .is_some_and(|sample| !sample.is_empty()),
            "the count carries a sample: {:?}",
            failures.dropped_sample
        );
        assert_eq!(sessions.len() + failures.dropped, 5, "row conservation");
        assert!(
            sessions
                .iter()
                .all(|session| session.metrics.tokens_input >= 0
                    && session.metrics.tokens_output >= 0),
            "lenient readers keep negative token values clamped at 0"
        );
    }

    /// rm-790/rm-791 lineage fixture: one opencode database where a parent
    /// row and three children all start inside the SAME second
    /// (1762000000123 / 1762000000200 ms — the assess PoC shape), two
    /// children point at the parent via `parent_id`, one at an absent row,
    /// and one parentless sibling exists. `with_parent_column = false`
    /// reproduces an older opencode schema without `parent_id`.
    fn opencode_lineage_fixture(dir: &Path, with_parent_column: bool) -> PathBuf {
        let path = dir.join("opencode.db");
        let db = Connection::open(&path).expect("open fixture db");
        let parent_column = if with_parent_column {
            ", parent_id text"
        } else {
            ""
        };
        let parent_values = if with_parent_column {
            ", parent_id"
        } else {
            ""
        };
        // Every row lands in second 1762000000 except ses-solo.
        let rows = if with_parent_column {
            r#"
                ('ses-parent', 'proj-a', 1762000000123, 1762000000900, '/work/a', 1.0, 100, 0, 0, 0, 0, NULL),
                ('ses-c1', 'proj-b', 1762000000123, 1762000000200, '/work/a', 0.5, 40, 0, 0, 0, 0, 'ses-parent'),
                ('ses-c2', 'proj-b', 1762000000123, 1762000000200, '/work/a', 0.25, 10, 0, 0, 0, 0, 'ses-parent'),
                ('ses-orphan', 'proj-c', 1762000000123, 1762000000200, '/work/a', 0.1, 5, 0, 0, 0, 0, 'ses-gone'),
                ('ses-solo', 'proj-d', 1762000005000, 1762000005100, '/work/a', 9.0, 900, 0, 0, 0, 0, NULL)
            "#
        } else {
            r#"
                ('ses-parent', 'proj-a', 1762000000123, 1762000000900, '/work/a', 1.0, 100, 0, 0, 0, 0),
                ('ses-c1', 'proj-b', 1762000000123, 1762000000200, '/work/a', 0.5, 40, 0, 0, 0, 0),
                ('ses-c2', 'proj-b', 1762000000123, 1762000000200, '/work/a', 0.25, 10, 0, 0, 0, 0),
                ('ses-orphan', 'proj-c', 1762000000123, 1762000000200, '/work/a', 0.1, 5, 0, 0, 0, 0),
                ('ses-solo', 'proj-d', 1762000005000, 1762000005100, '/work/a', 9.0, 900, 0, 0, 0, 0)
            "#
        };
        db.execute_batch(&format!(
            r#"
            create table session (
                id text primary key,
                title text,
                time_created integer,
                time_updated integer,
                directory text,
                cost real,
                tokens_input integer,
                tokens_output integer,
                tokens_reasoning integer,
                tokens_cache_read integer,
                tokens_cache_write integer{parent_column}
            );
            insert into session (id, title, time_created, time_updated, directory, cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write{parent_values}) values
                {rows};
            "#,
        ))
        .expect("seed lineage fixture");
        path
    }

    #[test]
    fn opencode_rows_carry_source_key_and_distinct_identity_within_one_second() {
        // rm-790: same-second rows from one database previously shared one
        // derived-history id (FNV-1a over path|start — the DB path is the
        // session path for sqlite lanes). Post-fix every row carries its
        // source `session_key`, so identities stay distinct while the
        // per-second session_start is identical.
        let root =
            std::env::temp_dir().join(format!("agenttrace-rm790-key-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = opencode_lineage_fixture(&root, true);
        let (sessions, failures, _fork_excluded) =
            query_opencode_sqlite_sessions(&path, None, None);
        std::fs::remove_dir_all(&root).ok();
        assert!(failures.is_clean(), "fixture must load clean: {failures:?}");
        assert_eq!(sessions.len(), 5);
        for row in ["ses-parent", "ses-c1", "ses-c2", "ses-orphan", "ses-solo"] {
            assert!(
                sessions
                    .iter()
                    .any(|session| session.metrics.session_key == row),
                "every row carries its source session id ({row} missing)"
            );
        }
        let starts: std::collections::HashSet<&str> = sessions
            .iter()
            .filter(|session| session.metrics.session_key != "ses-solo")
            .map(|session| session.metrics.session_start.as_str())
            .collect();
        assert_eq!(
            starts.len(),
            1,
            "the four same-second rows share one start second (the collision shape): {starts:?}"
        );
        let keys: std::collections::HashSet<&str> = sessions
            .iter()
            .map(|session| session.metrics.session_key.as_str())
            .collect();
        assert_eq!(keys.len(), 5, "keys are all distinct");
    }

    #[test]
    fn opencode_parent_id_rolls_children_into_the_parent_session() {
        // rm-791: the loader parks the raw `parent_id` marker in
        // `parent_session`; `attribute_subagents` resolves it by
        // `session_key` within the same database and rolls each child's
        // cost/tokens into the parent's subagent_* rollups — never into
        // the parent's own metrics.
        let root =
            std::env::temp_dir().join(format!("agenttrace-rm791-rollup-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = opencode_lineage_fixture(&root, true);
        let (mut sessions, failures, _fork_excluded) =
            query_opencode_sqlite_sessions(&path, None, None);
        std::fs::remove_dir_all(&root).ok();
        assert!(failures.is_clean(), "fixture must load clean: {failures:?}");
        // Pre-attribution: children carry the raw parent row marker.
        let raw = sessions
            .iter()
            .map(|session| {
                (
                    session.metrics.session_key.as_str(),
                    session.metrics.parent_session.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert!(raw.contains(&("ses-c1", "ses-parent")));
        crate::subagents::attribute_subagents(&mut sessions);
        let parent = sessions
            .iter()
            .find(|session| session.metrics.session_key == "ses-parent")
            .expect("parent row");
        assert_eq!(parent.metrics.subagent_count, 2);
        assert!((parent.metrics.subagent_cost - 0.75).abs() < 1e-9);
        assert_eq!(parent.metrics.subagent_tokens, 50);
        assert_eq!(
            parent.metrics.tokens_input, 100,
            "parent's own metrics never absorb the children"
        );
        assert_eq!(parent.metrics.cost_estimated, 1.0);
        let children: Vec<&Session> = sessions
            .iter()
            .filter(|session| {
                session.metrics.session_key == "ses-c1" || session.metrics.session_key == "ses-c2"
            })
            .collect();
        assert_eq!(children.len(), 2);
        assert!(children
            .iter()
            .all(|child| child.metrics.parent_session == "ses-parent"));
        // Children stay in the fleet (counted once each).
        assert_eq!(sessions.len(), 5);
        let orphan = sessions
            .iter()
            .find(|session| session.metrics.session_key == "ses-orphan")
            .expect("orphan row");
        assert!(
            orphan.metrics.parent_session.is_empty(),
            "an absent parent leaves no stale link"
        );
        let solo = sessions
            .iter()
            .find(|session| session.metrics.session_key == "ses-solo")
            .expect("solo row");
        assert_eq!(solo.metrics.subagent_count, 0);
        assert!(solo.metrics.parent_session.is_empty());
        // Idempotent re-run keeps the lineage without double rollups.
        crate::subagents::attribute_subagents(&mut sessions);
        let parent = sessions
            .iter()
            .find(|session| session.metrics.session_key == "ses-parent")
            .expect("parent row");
        assert_eq!(parent.metrics.subagent_count, 2);
        assert_eq!(parent.metrics.subagent_tokens, 50);
    }

    #[test]
    fn opencode_schema_without_parent_column_still_loads() {
        // rm-791 rider: an older opencode database without `parent_id`
        // (the column is guarded like the stored token columns) must
        // still load every row, parentless, clean.
        let root =
            std::env::temp_dir().join(format!("agenttrace-rm791-nocol-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("tempdir");
        let path = opencode_lineage_fixture(&root, false);
        let (mut sessions, failures, _fork_excluded) =
            query_opencode_sqlite_sessions(&path, None, None);
        std::fs::remove_dir_all(&root).ok();
        assert!(failures.is_clean(), "fixture must load clean: {failures:?}");
        assert_eq!(sessions.len(), 5, "every row loads");
        assert!(sessions
            .iter()
            .all(|session| session.metrics.parent_session.is_empty()));
        crate::subagents::attribute_subagents(&mut sessions);
        assert!(sessions
            .iter()
            .all(|session| session.metrics.subagent_count == 0));
    }
}

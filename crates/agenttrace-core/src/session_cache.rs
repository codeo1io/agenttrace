use crate::{Anomaly, Diagnostics, Metrics, Session, ToolWarning};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const SESSION_CACHE_SCHEMA_VERSION: i64 = 22;
// Bumped 21 -> 22 (integration of run 2c2db6f5, rm-400/rm-401): the
// kimi_cli usage-alias fix and the Codex compaction pairing fix change
// reported totals for unchanged files, so warm v21 entries carry the
// pre-fix under-counts this batch exists to correct (the rm-230
// convention: parser-semantics changes bump the schema so cached
// sessions regenerate under corrected accounting). Entries regenerate
// once on next scan.
// Fork-merge bump (cycle-1 rm-009, v0.9.0 port): upstream #286 moved the
// schema 17 -> 19 (loop_fingerprints diagnostics model); the fork had
// independently evolved 17 with its own fields (byte bound, walk v2).
// The merged schema is neither lineage, so 20 invalidates every legacy
// snapshot family once: fork-17 snapshots lack the new diagnostics
// model; upstream-19 snapshots lack the fork-only cache semantics.
// Bumped 20 -> 21 (run cbe30a9c integration, rm-230/rm-233): the
// flat-transcript parser now preserves the tool_use_id -> tool_call_id
// join (explicit ids verbatim, id-less entries paired positionally per
// tool) and loop detection keys on (tool name, argument identity)
// instead of the name alone, so v20 entries carry stale
// tool_latencies.unmatched counts, trace-step result pairing, and
// loop_cost groups under the old name-only key. Entries regenerate
// once on next scan.
// Bumped 5 → 6 (cycle-4 CU-10): cycle 3's naming change (placeholder
// titles replaced by first-user-message names) shipped while this stayed
// at 5, so v5 snapshots can carry stale names under new semantics. The
// version check regenerates them on next use.
// Bumped 6 → 7 (cycle-1 rm-198): hermes tool outcome semantics changed —
// tool_calls_ok/fail are now derived from the messages table instead of
// fabricating ok == sessions.tool_call_count, so v6 snapshots carry stale
// tool outcome splits and must regenerate once.
const SQLITE_SNAPSHOT_SCHEMA_VERSION: i64 = 7;

/// Orphaned temp files (crashed writers) are swept when the cache loads.
/// Live writers finish quickly; one hour is generous enough that a sweep
/// never races an in-flight write (pass-7 P7-5).
const ORPHAN_TEMP_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// Hard entry bound for the session cache (pass-8 F8-3). Beyond it the
/// entries with the oldest source-file fingerprint (mtime) are dropped
/// at save time, so an ever-growing corpus can no longer grow the
/// snapshot without limit.
pub const MAX_SESSION_CACHE_ENTRIES: usize = 20_000;

/// Hard bound on the serialized `sessions.json` size, enforced over
/// EVERY byte the writer emits — entry values, per-path keys, JSON
/// punctuation, the top-level fields, and the `dirs` map (rm-298: the
/// values-only estimate let a real 5,293-entry corpus write 108.9% of
/// this cap while the estimator called it in-bounds). The entry-count
/// bound alone cannot stop unbounded growth: entries carry full tool-arg
/// maps and directory listings, so a corpus of large sessions can grow
/// the cache to hundreds of MB at 20,000 entries. Eviction follows the
/// same policy as the count bound: oldest source-file fingerprint first
/// (pass-9 CU-22).
pub const MAX_SESSION_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Hard count bound for cached directory listings (rm-298). Before it,
/// only entries had bounds: the `dirs` map grew with directory-tree
/// breadth (2,783 listings / 3.6 MB on the operator corpus) with
/// nothing to stop it. Eviction is oldest directory mtime first, the
/// same policy as the entry bounds.
pub const MAX_SESSION_CACHE_DIRS: usize = 20_000;

/// Hard byte bound for the serialized `dirs` map — one eighth of the
/// overall cache cap (rm-298), so listings can never crowd session
/// entries out of the byte bound: evicting entries alone can always
/// reach the cap because the dirs block already sits inside this
/// budget.
pub const MAX_SESSION_CACHE_DIR_BYTES: usize = MAX_SESSION_CACHE_BYTES / 8;

/// Walk-semantics version for cached directory listings. Bumped when the
/// discovery walk's directory set changes so stale listings are dropped
/// once at load (see `load_session_cache`). v2: symlinked child
/// directories are followed (Codex `#42135`, cycle 7). v3: npm/package
/// manifests are no longer admitted (rm-370, cycle 4) — the cached replay
/// extends `listing.files` verbatim instead of re-running the admission
/// predicates, and a v2 listing still names `package-lock.json`,
/// `models-store.json`, and `*.lock` as files plus `npm` roots as child
/// directories, so a warm journal keeps feeding the walk the manifest
/// noise the blocklist exists to remove. The session-cache schema
/// version cannot cover this one: schema 22 already shipped (rm-400/401)
/// with the pre-blocklist walker, so a schema-22 journal can still carry
/// stale v2 listings.
const DIR_LISTING_WALK_VERSION: i64 = 3;

fn dirs_were_empty(doc: &Map<String, Value>) -> bool {
    doc.get("dirs")
        .and_then(Value::as_object)
        .map(|dirs| dirs.is_empty())
        .unwrap_or(true)
}

#[derive(Debug, Clone, Default)]
pub struct SessionCache {
    path: PathBuf,
    entries: BTreeMap<String, CacheEntry>,
    raw_entries: BTreeMap<String, Value>,
    dirs: BTreeMap<String, DirCacheEntry>,
    dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    mod_time: i64,
    size: i64,
    session: GoSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntryHeader {
    mod_time: i64,
    size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileFingerprint {
    mod_time: i64,
    size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SqliteSnapshot {
    schema_version: i64,
    database: FileFingerprint,
    wal: Option<FileFingerprint>,
    shm: Option<FileFingerprint>,
    sessions: Vec<GoSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirCacheEntry {
    mod_time: i64,
    files: Vec<String>,
    dirs: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CachedDirListing {
    pub files: Vec<PathBuf>,
    pub dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoSession {
    #[serde(default, rename = "Name")]
    name: String,
    #[serde(default, rename = "Path")]
    path: String,
    #[serde(default, rename = "CWD")]
    cwd: String,
    #[serde(default, rename = "Metrics")]
    metrics: GoMetrics,
    #[serde(default, rename = "Anomalies")]
    anomalies: Vec<GoAnomaly>,
    #[serde(default, rename = "Health")]
    health: i32,
    #[serde(default, rename = "ToolWarnings")]
    tool_warnings: Vec<GoToolWarning>,
    #[serde(default, rename = "Diagnostics")]
    diagnostics: Diagnostics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoMetrics {
    #[serde(default, rename = "EventsTotal")]
    events_total: usize,
    #[serde(default, rename = "UserMessages")]
    user_messages: usize,
    #[serde(default, rename = "AssistantTurns")]
    assistant_turns: usize,
    #[serde(default, rename = "ToolResults")]
    tool_results: usize,
    #[serde(default, rename = "ToolCallsTotal")]
    tool_calls_total: usize,
    #[serde(default, rename = "ToolCallsOK")]
    tool_calls_ok: usize,
    #[serde(default, rename = "ToolCallsFail")]
    tool_calls_fail: usize,
    #[serde(default, rename = "ToolUsage")]
    tool_usage: BTreeMap<String, usize>,
    #[serde(default, rename = "FileUsage")]
    file_usage: BTreeMap<String, usize>,
    #[serde(default, rename = "ToolArgUsage")]
    tool_arg_usage: BTreeMap<String, usize>,
    #[serde(default, rename = "ToolAuthority")]
    tool_authority: BTreeMap<String, usize>,
    #[serde(default, rename = "HighestAuthority")]
    highest_authority: String,
    #[serde(default, rename = "ReasoningBlocks")]
    reasoning_blocks: usize,
    #[serde(default, rename = "ReasoningChars")]
    reasoning_chars: usize,
    #[serde(default, rename = "ReasoningLens")]
    reasoning_lens: Vec<usize>,
    #[serde(default, rename = "ReasoningRedact")]
    reasoning_redact: usize,
    #[serde(default, rename = "TokensInput")]
    tokens_input: i64,
    #[serde(default, rename = "TokensOutput")]
    tokens_output: i64,
    #[serde(default, rename = "TokensReasoning")]
    tokens_reasoning: i64,
    #[serde(default, rename = "TokensCacheW")]
    tokens_cache_w: i64,
    #[serde(default, rename = "TokensCacheR")]
    tokens_cache_r: i64,
    #[serde(default, rename = "GapsSec")]
    gaps_sec: Vec<f64>,
    #[serde(default, rename = "ModelUsed")]
    model_used: String,
    #[serde(default, rename = "SourceTool")]
    source_tool: String,
    #[serde(default, rename = "SessionStart")]
    session_start: String,
    #[serde(default, rename = "SessionEnd")]
    session_end: String,
    #[serde(default, rename = "DurationSec")]
    duration_sec: f64,
    #[serde(default, rename = "CostEstimated")]
    cost_estimated: f64,
    #[serde(default, rename = "StoredTotalsDelta")]
    stored_totals_delta: i64,
    /// Parse lines lost inside the session source, by reason (pass-7
    /// P7-1). Preserved across cache round-trips; absent on clean parses.
    #[serde(
        default,
        rename = "LineSkips",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    line_skips: BTreeMap<String, usize>,
    /// rm-408: present-but-zero usage events, preserved across cache
    /// round-trips so the disclosure survives a warm cache.
    #[serde(
        default,
        rename = "ZeroUsageEvents",
        skip_serializing_if = "crate::usize_is_zero"
    )]
    zero_usage_events: usize,
    #[serde(default, rename = "Provenance")]
    provenance: crate::MetricProvenance,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoAnomaly {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    emoji: String,
    #[serde(default)]
    detail: String,
}

pub fn session_cache_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR").map(PathBuf::from) {
        if !dir.as_os_str().is_empty() {
            return dir.join("sessions.json");
        }
    }
    user_cache_dir().join("agenttrace").join("sessions.json")
}

/// Every at-rest artifact `--clear-cache` removes (rm-086), built
/// from the same constructors that write the files: the parsed-metrics
/// journal, the Hermes and OpenCode SQLite snapshots, the statusline
/// capture journal, and the LiteLLM pricing catalog. The statusline
/// and pricing files resolve through their own env-aware path
/// constructors, so each artifact is cleared where it actually lives
/// instead of assuming it sits beside the session cache.
/// `privacy_disclosure_lists_every_artifact` (tests) pins PRIVACY.md's
/// disclosure table against these file names, so a new store cannot
/// ship undisclosed.
pub(crate) fn cache_artifact_paths() -> Vec<PathBuf> {
    vec![
        session_cache_path(),
        sqlite_snapshot_path("hermes"),
        sqlite_snapshot_path("opencode"),
        crate::statusline::statusline_capture_path(),
        crate::pricing::pricing_cache_path(),
    ]
}

pub fn clear_session_cache() -> anyhow::Result<()> {
    let mut paths = cache_artifact_paths();
    paths.extend(legacy_cache_artifact_paths());
    remove_cache_artifacts(&paths)
}

/// Superseded-version leftovers (rm-086 review follow-up): older builds
/// wrote versioned SQLite snapshots (`hermes-sqlite-v7-*.json`,
/// `opencode-sqlite-v7-*.json`); no live code names those files today,
/// so on an upgraded host they would otherwise survive `--clear-cache`
/// forever — the most privacy-sensitive class (full parsed metrics)
/// among the least visible files. Swept by pattern from the same
/// env-aware cache root the registry uses; prefixes are pinned to the
/// two stores that ever wrote them so nothing else in the directory is
/// touched.
fn legacy_cache_artifact_paths() -> Vec<PathBuf> {
    // Both snapshot constructors share one env-aware cache root.
    let snapshot_path = sqlite_snapshot_path("hermes");
    let Some(cache_dir) = snapshot_path.parent() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                return false;
            };
            (file_name.starts_with("hermes-sqlite-v") || file_name.starts_with("opencode-sqlite-v"))
                && file_name.ends_with(".json")
        })
        .map(|entry| entry.path())
        .collect()
}

fn remove_cache_artifacts(paths: &[PathBuf]) -> anyhow::Result<()> {
    // rm-086 review follow-up: clear as much as we can, then report —
    // a first failure must not strand the artifacts after it (a partial
    // clear is the worst outcome for a privacy-motivated purge).
    let mut failures: Vec<String> = Vec::new();
    for path in paths {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => failures.push(format!("{}: {}", path.display(), err)),
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "failed to remove some cache artifacts: {}",
            failures.join("; ")
        )
    }
}

pub(crate) fn load_sqlite_snapshot(database: &Path, name: &str) -> Option<Vec<Session>> {
    load_sqlite_snapshot_from(database, &sqlite_snapshot_path(name))
}

fn load_sqlite_snapshot_from(database: &Path, snapshot_path: &Path) -> Option<Vec<Session>> {
    let raw = fs::read(snapshot_path).ok()?;
    let snapshot = serde_json::from_slice::<SqliteSnapshot>(&raw).ok()?;
    if snapshot.schema_version != SQLITE_SNAPSHOT_SCHEMA_VERSION
        || snapshot.database != file_fingerprint(database)?
        || snapshot.wal != file_fingerprint(&sqlite_wal_path(database))
        || snapshot.shm != file_fingerprint(&sqlite_shm_path(database))
    {
        return None;
    }
    Some(
        snapshot
            .sessions
            .into_iter()
            .map(|session| session.into_session(&database.to_string_lossy()))
            .collect(),
    )
}

pub(crate) fn store_sqlite_snapshot(
    database: &Path,
    name: &str,
    sessions: &[Session],
) -> anyhow::Result<()> {
    store_sqlite_snapshot_at(database, &sqlite_snapshot_path(name), sessions)
}

fn store_sqlite_snapshot_at(
    database: &Path,
    path: &Path,
    sessions: &[Session],
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let snapshot = SqliteSnapshot {
        schema_version: SQLITE_SNAPSHOT_SCHEMA_VERSION,
        database: file_fingerprint(database).ok_or_else(|| anyhow::anyhow!("database missing"))?,
        wal: file_fingerprint(&sqlite_wal_path(database)),
        shm: file_fingerprint(&sqlite_shm_path(database)),
        sessions: sessions.iter().map(GoSession::from_session).collect(),
    };
    let tmp = unique_temp_path(path);
    write_private(&tmp, &serde_json::to_vec(&snapshot)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

/// Per-writer temp path for atomic cache writes (pass-6 P6-3): the fixed
/// `<name>.json.tmp` sibling made two concurrent agenttrace processes race
/// on the same temp file, failing or tearing the save. The suffix is unique
/// per process and per write within the process; the atomic rename is
/// unchanged. Shared by the pricing-catalog and derived-history writes
/// (pass-7 P7-5).
pub(crate) fn unique_temp_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let mut name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sessions.json")
        .to_string();
    name.push_str(&format!(".tmp.{}.{}", std::process::id(), seq));
    path.with_file_name(name)
}

/// Owner-only creation for conversation-derived artifacts (cycle-1
/// rm-208): the session cache, the SQLite snapshots, the statusline
/// journal, and derived history all carry session names, tool args,
/// project paths, and transcript-derived metrics, so a group/world-
/// readable copy hands them to every local account. `fs::write`/
/// `File::create` default to 0644 (0664 under a common umask); this
/// helper creates 0o600 on Unix — umask can only tighten it — and
/// keeps the platform default elsewhere. Callers keep their own
/// temp-then-rename atomicity.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(bytes)
    }
    #[cfg(not(unix))]
    {
        fs::write(path, bytes)
    }
}

/// Append-mode counterpart of [`write_private`] for the statusline
/// journal (cycle-1 rm-208): creates the journal owner-only the first
/// time it is written; an existing file's permissions are left alone.
pub(crate) fn open_private_append(path: &Path) -> std::io::Result<fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)
    }
    #[cfg(not(unix))]
    {
        fs::OpenOptions::new().create(true).append(true).open(path)
    }
}

/// Remove temp files left behind by crashed writers (pass-7 P7-5):
/// `<name>.json.tmp.<pid>.<seq>` siblings in the cache directory older
/// than `max_age`. Returns how many were removed. Live writers finish
/// quickly, so a generous `max_age` keeps the sweep from racing one.
pub(crate) fn sweep_orphaned_temps(path: &Path, max_age: std::time::Duration) -> usize {
    let Some(dir) = path.parent() else {
        return 0;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let now = std::time::SystemTime::now();
    let mut removed = 0;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if !name.contains(".tmp.") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let modified = metadata.modified().unwrap_or(now);
        if now
            .duration_since(modified)
            .unwrap_or(std::time::Duration::ZERO)
            >= max_age
            && fs::remove_file(entry.path()).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

fn sqlite_snapshot_path(name: &str) -> PathBuf {
    session_cache_path().with_file_name(format!("{name}-sqlite.json"))
}

fn sqlite_wal_path(database: &Path) -> PathBuf {
    PathBuf::from(format!("{}-wal", database.to_string_lossy()))
}

fn sqlite_shm_path(database: &Path) -> PathBuf {
    PathBuf::from(format!("{}-shm", database.to_string_lossy()))
}

fn file_fingerprint(path: &Path) -> Option<FileFingerprint> {
    let metadata = fs::metadata(path).ok()?;
    Some(FileFingerprint {
        mod_time: file_mod_time_nanos(&metadata),
        size: metadata.len() as i64,
    })
}

pub fn load_session_cache() -> SessionCache {
    let path = session_cache_path();
    sweep_orphaned_temps(&path, ORPHAN_TEMP_MAX_AGE);
    let Ok(raw) = fs::read_to_string(&path) else {
        return SessionCache {
            path,
            ..SessionCache::default()
        };
    };
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&raw) else {
        return SessionCache {
            path,
            ..SessionCache::default()
        };
    };
    if doc.get("schema_version").and_then(Value::as_i64) != Some(SESSION_CACHE_SCHEMA_VERSION) {
        return SessionCache {
            path,
            dirty: true,
            ..SessionCache::default()
        };
    }
    let raw_entries = doc
        .get("entries")
        .and_then(Value::as_object)
        .map(|entries| {
            entries
                .iter()
                .filter(|(_, value)| decode_cache_entry_header(value).is_some())
                .map(|(path, value)| (path.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default();
    // Directory listings are versioned by walk semantics: v2 follows
    // symlinked child directories (Codex `#42135`, cycle 7), so listings
    // written by the older walker — which silently omitted them — are
    // dropped once at load instead of hiding files until each parent
    // directory's mtime happens to change.
    let listings_current =
        doc.get("dir_listing_version").and_then(Value::as_i64) == Some(DIR_LISTING_WALK_VERSION);
    let dirs = if listings_current {
        doc.get("dirs")
            .and_then(Value::as_object)
            .map(|dirs| {
                dirs.iter()
                    .filter_map(|(path, value)| {
                        serde_json::from_value::<DirCacheEntry>(value.clone())
                            .ok()
                            .map(|entry| (path.clone(), entry))
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let listing_stale = !listings_current && !dirs_were_empty(&doc);
    let mut cache = SessionCache {
        path,
        raw_entries,
        dirs,
        dirty: listing_stale,
        ..SessionCache::default()
    };
    // Dead-path eviction (pass-8 F8-3): entries whose source file no
    // longer exists are pruned at load time instead of accumulating
    // forever. A pruned entry costs only a re-parse if the file ever
    // reappears, so eviction never loses data.
    prune_dead_entries(&mut cache);
    cache
}

/// Removes cache entries whose source path no longer exists (and dir
/// listings whose directory is gone), marking the cache dirty so the
/// next save persists the smaller snapshot. Returns how many entries
/// were pruned (pass-8 F8-3).
fn prune_dead_entries(cache: &mut SessionCache) -> usize {
    let mut pruned = 0;
    let dead_paths: Vec<String> = cache
        .raw_entries
        .keys()
        .chain(cache.entries.keys())
        .filter(|path| !Path::new(path).exists())
        .cloned()
        .collect();
    for path in dead_paths {
        if cache.entries.remove(&path).is_some() | cache.raw_entries.remove(&path).is_some() {
            pruned += 1;
        }
    }
    let dead_dirs: Vec<String> = cache
        .dirs
        .keys()
        .filter(|dir| !Path::new(dir).exists())
        .cloned()
        .collect();
    for dir in dead_dirs {
        cache.dirs.remove(&dir);
        pruned += 1;
    }
    if pruned > 0 {
        cache.dirty = true;
    }
    pruned
}

pub fn load_cached_sessions(dir: Option<&Path>) -> Vec<Session> {
    let mut cache = load_session_cache();
    load_cached_sessions_from_cache(dir, &mut cache)
}

pub fn load_cached_sessions_from_cache(
    dir: Option<&Path>,
    cache: &mut SessionCache,
) -> Vec<Session> {
    let paths = cache
        .raw_entries
        .keys()
        .chain(cache.entries.keys())
        .map(PathBuf::from)
        .collect::<BTreeSet<_>>();
    let sessions = paths
        .into_iter()
        .filter(|path| dir.map_or(true, |dir| path.starts_with(dir)))
        .filter_map(|path| cached_session(&path, cache))
        .collect();
    if cache.is_dirty() {
        let _ = save_session_cache(cache);
    }
    sessions
}

impl SessionCache {
    pub fn entry_count(&self) -> usize {
        let mut count = self.raw_entries.len();
        for path in self.entries.keys() {
            if !self.raw_entries.contains_key(path) {
                count += 1;
            }
        }
        count
    }

    pub fn dir_count(&self) -> usize {
        self.dirs.len()
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }
}

pub(crate) fn cached_dir_listing(dir: &Path, cache: &mut SessionCache) -> Option<CachedDirListing> {
    let key = cache_key(dir);
    let metadata = fs::metadata(dir).ok()?;
    if !metadata.is_dir() {
        return None;
    }
    let entry = cache.dirs.get(&key)?;
    if entry.mod_time != file_mod_time_nanos(&metadata) {
        cache.dirs.remove(&key);
        cache.dirty = true;
        return None;
    }
    Some(CachedDirListing {
        files: entry.files.iter().map(PathBuf::from).collect(),
        dirs: entry.dirs.iter().map(PathBuf::from).collect(),
    })
}

pub(crate) fn store_dir_listing(
    dir: &Path,
    files: &[PathBuf],
    dirs: &[PathBuf],
    cache: &mut SessionCache,
) -> anyhow::Result<()> {
    let metadata = fs::metadata(dir)?;
    cache.dirs.insert(
        cache_key(dir),
        DirCacheEntry {
            mod_time: file_mod_time_nanos(&metadata),
            files: files.iter().map(|path| cache_key(path)).collect(),
            dirs: dirs.iter().map(|path| cache_key(path)).collect(),
        },
    );
    cache.dirty = true;
    Ok(())
}

pub(crate) fn cached_file_mod_time_if_fresh(
    path: &Path,
    metadata: &fs::Metadata,
    cache: &mut SessionCache,
) -> Option<i64> {
    let key = cache_key(path);
    let header = cached_entry_header(&key, cache)?;
    if header.size == metadata.len() as i64 && header.mod_time == file_mod_time_nanos(metadata) {
        return Some(header.mod_time);
    }
    delete_cached_session_key(&key, cache);
    None
}

fn decode_cache_entry_header(value: &Value) -> Option<CacheEntryHeader> {
    Some(CacheEntryHeader {
        mod_time: value.get("mod_time")?.as_i64()?,
        size: value.get("size")?.as_i64()?,
    })
}

fn cached_entry_header(path: &str, cache: &mut SessionCache) -> Option<CacheEntryHeader> {
    if let Some(entry) = cache.entries.get(path) {
        return Some(CacheEntryHeader {
            mod_time: entry.mod_time,
            size: entry.size,
        });
    }
    cache
        .raw_entries
        .get(path)
        .and_then(decode_cache_entry_header)
}

fn cached_entry(path: &str, cache: &mut SessionCache) -> Option<CacheEntry> {
    if let Some(entry) = cache.entries.get(path) {
        return Some(entry.clone());
    }
    let raw = cache.raw_entries.get(path)?.clone();
    let Ok(entry) = serde_json::from_value::<CacheEntry>(raw) else {
        delete_cached_session_key(path, cache);
        return None;
    };
    cache.entries.insert(path.to_string(), entry.clone());
    Some(entry)
}

fn cached_entry_missing_tool_warnings(path: &str, cache: &SessionCache) -> bool {
    cache
        .raw_entries
        .get(path)
        .and_then(|entry| entry.get("session").or_else(|| entry.get("Session")))
        .and_then(Value::as_object)
        .is_some_and(|session| {
            !session.contains_key("ToolWarnings") && !session.contains_key("tool_warnings")
        })
}

fn cached_entry_missing_tool_arg_usage(path: &str, cache: &SessionCache) -> bool {
    cache
        .raw_entries
        .get(path)
        .and_then(|entry| entry.get("session").or_else(|| entry.get("Session")))
        .and_then(|session| session.get("Metrics").or_else(|| session.get("metrics")))
        .and_then(Value::as_object)
        .is_some_and(|metrics| {
            !metrics.contains_key("ToolArgUsage") && !metrics.contains_key("tool_arg_usage")
        })
}

fn cached_entry_empty_source_tool(path: &str, cache: &SessionCache) -> bool {
    cache
        .raw_entries
        .get(path)
        .and_then(|entry| entry.get("session").or_else(|| entry.get("Session")))
        .and_then(|session| session.get("Metrics").or_else(|| session.get("metrics")))
        .and_then(Value::as_object)
        .is_some_and(|metrics| {
            metrics
                .get("SourceTool")
                .or_else(|| metrics.get("source_tool"))
                .and_then(Value::as_str)
                .is_some_and(str::is_empty)
        })
}

pub(crate) fn delete_cached_session(path: &Path, cache: &mut SessionCache) {
    delete_cached_session_key(&cache_key(path), cache);
}

fn delete_cached_session_key(path: &str, cache: &mut SessionCache) {
    if cache.entries.remove(path).is_some() || cache.raw_entries.remove(path).is_some() {
        cache.dirty = true;
    }
}

/// Deduplicated cache paths, each sized as the writer will emit it: a
/// path decoded from `raw_entries` also lives in `entries` (see
/// `cached_entry`), so chaining the two maps' keys counts it twice — the
/// byte bound then over-evicts (up to ~2x near the ceiling) and the
/// entry bound wastes drop slots and under-drops (pass-11 A11-2, cycle
/// 7). At save time the decoded copy OVERWRITES the raw copy for the
/// same path, so when both maps hold a path its written-form size is
/// the decoded entry's — exactly what the byte-true bound must count
/// (rm-298).
fn cache_paths_sized_once(cache: &SessionCache) -> Vec<(String, usize)> {
    let mut sized: BTreeMap<String, usize> = BTreeMap::new();
    for (path, value) in &cache.raw_entries {
        let bytes = serde_json::to_string(value)
            .map(|text| text.len())
            .unwrap_or(0);
        sized.insert(path.clone(), bytes);
    }
    for (path, entry) in &cache.entries {
        let bytes = serde_json::to_string(entry)
            .map(|text| text.len())
            .unwrap_or(0);
        sized.insert(path.clone(), bytes);
    }
    sized.into_iter().collect()
}

/// Serialized length of a JSON object key as `save_session_cache`
/// writes it: the surrounding quotes plus any escape sequences
/// (rm-298 — the keys were the invisible ~80 bytes per entry).
fn json_key_len(key: &str) -> usize {
    serde_json::to_string(key)
        .map(|text| text.len())
        .unwrap_or(0)
}

/// Serialized length of a JSON object from its members' total bytes
/// (`key` + `:` + value each) and the member count: two braces plus a
/// comma between adjacent members (rm-298).
fn json_object_len(member_bytes: usize, count: usize) -> usize {
    if count == 0 {
        2
    } else {
        2 + member_bytes + (count - 1)
    }
}

/// Total member bytes of the `dirs` map as written: per listing, the
/// path key, one colon, and the serialized value (rm-298).
fn dirs_member_bytes(cache: &SessionCache) -> usize {
    cache
        .dirs
        .iter()
        .map(|(path, entry)| {
            json_key_len(path)
                + 1
                + serde_json::to_string(entry)
                    .map(|text| text.len())
                    .unwrap_or(0)
        })
        .sum()
}

/// Length of the document `save_session_cache` writes for these
/// blocks: the fixed top-level fields, the entries map, the `dirs`
/// member (when non-empty), and the closing brace (rm-298).
fn doc_frame_len(
    entries_member_bytes: usize,
    entries_count: usize,
    dirs_bytes: usize,
    dirs_count: usize,
) -> usize {
    let mut total = format!(
        "{{\"schema_version\":{},\"dir_listing_version\":{},\"entries\":",
        SESSION_CACHE_SCHEMA_VERSION, DIR_LISTING_WALK_VERSION
    )
    .len();
    total += json_object_len(entries_member_bytes, entries_count);
    if dirs_count > 0 {
        total += 1 + "\"dirs\":".len() + json_object_len(dirs_bytes, dirs_count);
    }
    total + 1
}

/// Byte-true projection of the cache file: exactly the number of
/// bytes `save_session_cache` writes for this cache state — entry
/// values, per-path keys, JSON punctuation, top-level fields, and the
/// `dirs` map. The byte bound enforces the cap over THIS number, so
/// the written file cannot exceed the bound through uncounted
/// overhead (rm-298).
fn serialized_doc_size(cache: &SessionCache) -> usize {
    let sized = cache_paths_sized_once(cache);
    let entries_member_bytes: usize = sized
        .iter()
        .map(|(path, bytes)| json_key_len(path) + 1 + bytes)
        .sum();
    doc_frame_len(
        entries_member_bytes,
        sized.len(),
        dirs_member_bytes(cache),
        cache.dirs.len(),
    )
}

/// Enforces the entry bound by dropping the entries with the oldest
/// source-file fingerprint (mtime) first; keeps at most `max` entries.
/// Returns how many entries were dropped (pass-8 F8-3). The bound walks
/// the deduplicated path union so a path present in both maps costs one
/// drop slot, not two (pass-11 A11-2); an entry without a decodable
/// header is undatable, not unevictable — it counts as the oldest age so
/// the bound is total, not best-effort (F5-5, cycle-5 review).
fn enforce_entry_bound(cache: &mut SessionCache, max: usize) -> usize {
    let total = cache.entry_count();
    if total <= max {
        return 0;
    }
    let mut by_age: Vec<(i64, String)> = cache_paths_sized_once(cache)
        .into_iter()
        .map(|(path, _bytes)| {
            (
                cached_entry_header(&path, cache).map_or(i64::MIN, |header| header.mod_time),
                path,
            )
        })
        .collect();
    by_age.sort_unstable();
    let drop = total - max;
    let mut dropped = 0;
    for (_, path) in by_age.into_iter().take(drop) {
        // `|`, not `||`: a path can live in both maps and both copies
        // must go in the same drop.
        if cache.entries.remove(&path).is_some() | cache.raw_entries.remove(&path).is_some() {
            dropped += 1;
        }
    }
    if dropped > 0 {
        cache.dirty = true;
    }
    dropped
}

/// Enforces the dirs-map bounds (rm-298): listings drop oldest
/// directory mtime first — the same eviction policy as the entry
/// bounds — until the count is at most `max_count` and the serialized
/// `{...}` block is at most `max_bytes`. Before this, `dirs` was
/// bounded by nothing at all, so a broad directory tree could grow
/// the snapshot without limit and then crowd entries out of the byte
/// bound. Returns how many listings were dropped.
fn enforce_dirs_bound(cache: &mut SessionCache, max_count: usize, max_bytes: usize) -> usize {
    let mut by_age: Vec<(i64, String, usize)> = cache
        .dirs
        .iter()
        .map(|(path, entry)| {
            (
                entry.mod_time,
                path.clone(),
                json_key_len(path)
                    + 1
                    + serde_json::to_string(entry)
                        .map(|text| text.len())
                        .unwrap_or(0),
            )
        })
        .collect();
    // (mod_time, path): deterministic even on mtime ties.
    by_age.sort();
    let mut member_bytes: usize = by_age.iter().map(|(_, _, member)| *member).sum();
    let mut remaining = by_age.len();
    // Oldest first; drop listings until both budgets hold.
    let mut dropped = 0;
    for (_, path, member) in by_age {
        if remaining <= max_count && json_object_len(member_bytes, remaining) <= max_bytes {
            break;
        }
        if cache.dirs.remove(&path).is_some() {
            dropped += 1;
            member_bytes -= member;
            remaining -= 1;
        }
    }
    if dropped > 0 {
        cache.dirty = true;
    }
    dropped
}

/// Enforces the serialized-size bound over the byte-true projection of
/// the WRITTEN document (rm-298): entry values, per-path keys, JSON
/// punctuation, top-level fields, and the dirs map — exactly what
/// `save_session_cache` emits. Entries drop oldest source-file
/// fingerprint (mtime) first, the same eviction order as
/// `enforce_entry_bound`; headerless entries count as the oldest age
/// (F5-5); each path of the deduplicated union counts once (pass-11
/// A11-2). The dirs block is bounded separately by `enforce_dirs_bound`,
/// so evicting entries alone always reaches the cap. Returns how many
/// entries were dropped (pass-9 CU-22; byte-true since rm-298).
fn enforce_byte_bound(cache: &mut SessionCache, max: usize) -> usize {
    // Fast path (the common case): one projection over the whole cache,
    // no member bookkeeping. This is also the production consumer of
    // `serialized_doc_size` — the same projection the tests pin to the
    // written file byte for byte.
    if serialized_doc_size(cache) <= max {
        return 0;
    }
    let mut by_age: Vec<(i64, String, usize)> = cache_paths_sized_once(cache)
        .into_iter()
        .map(|(path, bytes)| {
            let member = json_key_len(&path) + 1 + bytes;
            (
                cached_entry_header(&path, cache).map_or(i64::MIN, |header| header.mod_time),
                path,
                // `key`:value as one member; the comma between members
                // is accounted for in the decrement below.
                member,
            )
        })
        .collect();
    let entries_member_bytes: usize = by_age.iter().map(|(_, _, member)| *member).sum();
    let mut entries_count = by_age.len();
    let dirs_bytes = dirs_member_bytes(cache);
    let mut total = doc_frame_len(
        entries_member_bytes,
        entries_count,
        dirs_bytes,
        cache.dirs.len(),
    );
    if total <= max {
        return 0;
    }
    // Oldest first; drop entries until the written document fits.
    by_age.sort_by_key(|(mod_time, _, _)| *mod_time);
    let mut dropped = 0;
    for (_, path, member) in by_age {
        if total <= max {
            break;
        }
        if cache.entries.remove(&path).is_some() | cache.raw_entries.remove(&path).is_some() {
            dropped += 1;
            // Dropping one member frees its bytes plus one comma —
            // unless it was the only member, when the map collapses
            // to `{}` and only the member bytes go.
            total -= member + usize::from(entries_count >= 2);
            entries_count -= 1;
        }
    }
    if dropped > 0 {
        cache.dirty = true;
    }
    dropped
}

pub fn save_session_cache(cache: &mut SessionCache) -> anyhow::Result<()> {
    // Hard bounds before serializing: beyond MAX_SESSION_CACHE_ENTRIES
    // the oldest-fingerprint entries are dropped (pass-8 F8-3); the
    // dirs map keeps its own count and byte budgets (rm-298); and the
    // serialized document is capped at MAX_SESSION_CACHE_BYTES over
    // every byte this function writes — keys, punctuation, top-level
    // fields, and dirs included (pass-9 CU-22; byte-true rm-298).
    enforce_entry_bound(cache, MAX_SESSION_CACHE_ENTRIES);
    enforce_dirs_bound(cache, MAX_SESSION_CACHE_DIRS, MAX_SESSION_CACHE_DIR_BYTES);
    enforce_byte_bound(cache, MAX_SESSION_CACHE_BYTES);
    if let Some(parent) = cache.path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut doc = Map::new();
    doc.insert(
        "schema_version".to_string(),
        Value::Number(SESSION_CACHE_SCHEMA_VERSION.into()),
    );
    doc.insert(
        "dir_listing_version".to_string(),
        Value::Number(DIR_LISTING_WALK_VERSION.into()),
    );
    let mut entries = Map::new();
    for (path, value) in &cache.raw_entries {
        entries.insert(path.clone(), value.clone());
    }
    for (path, entry) in &cache.entries {
        entries.insert(
            path.clone(),
            serde_json::to_value(entry).expect("cache entry serialize"),
        );
    }
    doc.insert("entries".to_string(), Value::Object(entries));
    if !cache.dirs.is_empty() {
        let dirs = cache
            .dirs
            .iter()
            .map(|(path, entry)| {
                (
                    path.clone(),
                    serde_json::to_value(entry).expect("dir cache entry serialize"),
                )
            })
            .collect();
        doc.insert("dirs".to_string(), Value::Object(dirs));
    }
    let tmp = unique_temp_path(&cache.path);
    write_private(&tmp, &serde_json::to_vec(&Value::Object(doc))?)?;
    fs::rename(tmp, &cache.path)?;
    Ok(())
}

pub fn cached_session(path: &Path, cache: &mut SessionCache) -> Option<Session> {
    let key = cache_key(path);
    let header = cached_entry_header(&key, cache)?;
    if !is_fresh(path, &header) {
        delete_cached_session_key(&key, cache);
        return None;
    }
    if cached_entry_missing_tool_warnings(&key, cache) {
        delete_cached_session_key(&key, cache);
        return None;
    }
    if cached_entry_missing_tool_arg_usage(&key, cache) {
        delete_cached_session_key(&key, cache);
        return None;
    }
    if cached_entry_empty_source_tool(&key, cache) {
        delete_cached_session_key(&key, cache);
        return None;
    }
    let entry = cached_entry(&key, cache)?;
    Some(entry.session.clone().into_session(&key))
}

pub fn store_session(
    path: &Path,
    session: &Session,
    cache: &mut SessionCache,
) -> anyhow::Result<()> {
    let metadata = fs::metadata(path)?;
    let key = cache_key(path);
    cache.entries.insert(
        key.clone(),
        CacheEntry {
            mod_time: file_mod_time_nanos(&metadata),
            size: metadata.len() as i64,
            session: GoSession::from_session(session),
        },
    );
    cache.raw_entries.remove(&key);
    cache.dirty = true;
    Ok(())
}

impl GoSession {
    fn from_session(session: &Session) -> Self {
        Self {
            name: session.name.clone(),
            path: session.path.clone(),
            cwd: session.cwd.clone(),
            metrics: GoMetrics::from_metrics(&session.metrics),
            anomalies: session
                .anomalies
                .iter()
                .map(GoAnomaly::from_anomaly)
                .collect(),
            health: session.health,
            tool_warnings: session
                .tool_warnings
                .iter()
                .map(GoToolWarning::from_tool_warning)
                .collect(),
            diagnostics: session.diagnostics.clone(),
        }
    }

    fn into_session(self, fallback_path: &str) -> Session {
        Session {
            name: self.name,
            path: if self.path.is_empty() {
                fallback_path.to_string()
            } else {
                self.path
            },
            cwd: self.cwd,
            metrics: self.metrics.into_metrics(),
            anomalies: self
                .anomalies
                .into_iter()
                .map(GoAnomaly::into_anomaly)
                .collect(),
            health: self.health,
            tool_warnings: self
                .tool_warnings
                .into_iter()
                .map(GoToolWarning::into_tool_warning)
                .collect(),
            diagnostics: self.diagnostics,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoToolWarning {
    #[serde(default, rename = "ToolName")]
    tool_name: String,
    #[serde(default, rename = "Pattern")]
    pattern: String,
    #[serde(default, rename = "Count")]
    count: usize,
    #[serde(default, rename = "Detail")]
    detail: String,
    #[serde(default, rename = "Severity")]
    severity: String,
}

impl GoToolWarning {
    fn from_tool_warning(warning: &ToolWarning) -> Self {
        Self {
            tool_name: warning.tool_name.clone(),
            pattern: warning.pattern.clone(),
            count: warning.count,
            detail: warning.detail.clone(),
            severity: warning.severity.clone(),
        }
    }

    fn into_tool_warning(self) -> ToolWarning {
        ToolWarning {
            tool_name: self.tool_name,
            pattern: self.pattern,
            count: self.count,
            detail: self.detail,
            severity: self.severity,
        }
    }
}

impl GoMetrics {
    fn from_metrics(metrics: &Metrics) -> Self {
        Self {
            events_total: metrics.events_total,
            user_messages: metrics.user_messages,
            assistant_turns: metrics.assistant_turns,
            tool_results: metrics.tool_results,
            tool_calls_total: metrics.tool_calls_total,
            tool_calls_ok: metrics.tool_calls_ok,
            tool_calls_fail: metrics.tool_calls_fail,
            tool_usage: metrics.tool_usage.clone(),
            file_usage: metrics.file_usage.clone(),
            tool_arg_usage: metrics.tool_arg_usage.clone(),
            tool_authority: metrics.tool_authority.clone(),
            highest_authority: metrics.highest_authority.clone(),
            reasoning_blocks: metrics.reasoning_blocks,
            reasoning_chars: metrics.reasoning_chars,
            reasoning_lens: metrics.reasoning_lens.clone(),
            reasoning_redact: metrics.reasoning_redact,
            tokens_input: metrics.tokens_input,
            tokens_output: metrics.tokens_output,
            tokens_reasoning: metrics.tokens_reasoning,
            tokens_cache_w: metrics.tokens_cache_w,
            tokens_cache_r: metrics.tokens_cache_r,
            gaps_sec: metrics.gaps_sec.clone(),
            model_used: metrics.model_used.clone(),
            source_tool: metrics.source_tool.clone(),
            session_start: metrics.session_start.clone(),
            session_end: metrics.session_end.clone(),
            duration_sec: metrics.duration_sec,
            cost_estimated: metrics.cost_estimated,
            stored_totals_delta: metrics.stored_totals_delta,
            line_skips: metrics.line_skips.clone(),
            zero_usage_events: metrics.zero_usage_events,
            provenance: metrics.provenance.clone(),
        }
    }

    fn into_metrics(self) -> Metrics {
        Metrics {
            events_total: self.events_total,
            user_messages: self.user_messages,
            assistant_turns: self.assistant_turns,
            tool_results: self.tool_results,
            tool_calls_total: self.tool_calls_total,
            tool_calls_ok: self.tool_calls_ok,
            tool_calls_fail: self.tool_calls_fail,
            tool_usage: self.tool_usage,
            file_usage: self.file_usage,
            tool_arg_usage: self.tool_arg_usage,
            tool_authority: self.tool_authority,
            highest_authority: self.highest_authority,
            reasoning_blocks: self.reasoning_blocks,
            reasoning_chars: self.reasoning_chars,
            reasoning_lens: self.reasoning_lens,
            reasoning_redact: self.reasoning_redact,
            tokens_input: self.tokens_input,
            tokens_output: self.tokens_output,
            tokens_reasoning: self.tokens_reasoning,
            tokens_cache_w: self.tokens_cache_w,
            tokens_cache_r: self.tokens_cache_r,
            timestamps: Vec::new(),
            gaps_sec: self.gaps_sec,
            model_used: self.model_used,
            source_tool: self.source_tool,
            session_start: self.session_start,
            session_end: self.session_end,
            duration_sec: self.duration_sec,
            cost_estimated: self.cost_estimated,
            stored_totals_delta: self.stored_totals_delta,
            line_skips: self.line_skips.clone(),
            zero_usage_events: self.zero_usage_events,
            provenance: self.provenance,
        }
    }
}

impl GoAnomaly {
    fn from_anomaly(anomaly: &Anomaly) -> Self {
        Self {
            kind: anomaly.kind.clone(),
            severity: anomaly.severity.clone(),
            emoji: anomaly_emoji(&anomaly.severity).to_string(),
            detail: anomaly.detail.clone(),
        }
    }

    fn into_anomaly(self) -> Anomaly {
        Anomaly {
            kind: self.kind,
            severity: self.severity,
            detail: self.detail,
        }
    }
}

fn user_cache_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return home.join("Library").join("Caches");
        }
    }
    if let Some(cache) = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from) {
        if !cache.as_os_str().is_empty() {
            return cache;
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        return home.join(".cache");
    }
    std::env::temp_dir()
}

fn cache_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn is_fresh(path: &Path, entry: &CacheEntryHeader) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    entry.size == metadata.len() as i64 && entry.mod_time == file_mod_time_nanos(&metadata)
}

#[cfg(unix)]
fn file_mod_time_nanos(metadata: &fs::Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    metadata.mtime() * 1_000_000_000 + metadata.mtime_nsec()
}

#[cfg(not(unix))]
fn file_mod_time_nanos(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn anomaly_emoji(severity: &str) -> &'static str {
    match severity {
        "high" => "🔴",
        "medium" => "🟡",
        "low" => "🟢",
        _ => "",
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_persist_never_races_on_a_shared_temp_file() {
        // Pass-6 P6-3: cache persist used a fixed `<name>.json.tmp`
        // sibling, so two concurrent writers raced on the same temp file
        // (failed or torn save). With the per-writer suffix every persist
        // must succeed, the final snapshot must load, and no temp files
        // may survive the atomic renames.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-temp-race-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let database = root.join("state.db");
        let snapshot = root.join("snapshot.json");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&database, b"db").expect("write database");
        let session = Session {
            name: "cached".to_string(),
            path: database.to_string_lossy().to_string(),
            cwd: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        let writers: Vec<_> = (0..8)
            .map(|i| {
                std::thread::spawn({
                    let database = database.clone();
                    let snapshot = snapshot.clone();
                    let session = session.clone();
                    move || store_sqlite_snapshot_at(&database, &snapshot, &[session]).map(|_| i)
                })
            })
            .collect();
        for writer in writers {
            writer
                .join()
                .expect("writer thread must not panic")
                .expect("concurrent persist must not fail");
        }
        assert_eq!(
            load_sqlite_snapshot_from(&database, &snapshot)
                .expect("snapshot must survive the race")
                .len(),
            1
        );
        let leftovers: Vec<_> = std::fs::read_dir(&root)
            .expect("read temp dir")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files must not survive: {leftovers:?}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sqlite_snapshot_is_invalidated_by_database_wal_or_shm_changes() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-sqlite-cache-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let database = root.join("state.db");
        let snapshot = root.join("snapshot.json");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&database, b"db").expect("write database");
        let session = Session {
            name: "cached".to_string(),
            path: database.to_string_lossy().to_string(),
            cwd: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };

        store_sqlite_snapshot_at(&database, &snapshot, &[session]).expect("store snapshot");
        assert_eq!(
            load_sqlite_snapshot_from(&database, &snapshot)
                .expect("cache hit")
                .len(),
            1
        );

        fs::write(sqlite_wal_path(&database), b"wal").expect("write wal");
        assert!(load_sqlite_snapshot_from(&database, &snapshot).is_none());

        store_sqlite_snapshot_at(
            &database,
            &snapshot,
            &[Session {
                name: "cached".to_string(),
                path: database.to_string_lossy().to_string(),
                cwd: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            }],
        )
        .expect("store snapshot with wal");
        fs::write(sqlite_shm_path(&database), b"shm").expect("write shm");
        assert!(load_sqlite_snapshot_from(&database, &snapshot).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sqlite_snapshot_schema_seven_round_trips_provenance_and_rejects_older_schemas() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-sqlite-schema-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let database = root.join("state.db");
        let snapshot = root.join("snapshot.json");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&database, b"db").expect("write database");
        let session = Session {
            name: "cached".to_string(),
            path: database.to_string_lossy().to_string(),
            cwd: String::new(),
            metrics: Metrics {
                stored_totals_delta: 720,
                provenance: crate::MetricProvenance {
                    tokens: "stored_session_totals".to_string(),
                    duration: "timestamp_span".to_string(),
                    tool_results: "reported_by_agent".to_string(),
                    pricing_source: "LiteLLM (cached)".to_string(),
                    ..crate::MetricProvenance::default()
                },
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_sqlite_snapshot_at(&database, &snapshot, &[session]).expect("store snapshot");
        let raw = fs::read_to_string(&snapshot).expect("read snapshot");
        let doc: serde_json::Value = serde_json::from_str(&raw).expect("snapshot json");
        // Version seven (cycle-1 rm-198): hermes tool outcome semantics
        // changed (ok/fail now derive from the messages table instead of
        // fabricating ok == tool_call_count), so v6 snapshots carry stale
        // tool outcome splits and must regenerate.
        assert_eq!(doc["schema_version"], 7);
        assert_eq!(
            doc.pointer("/sessions/0/Metrics/Provenance/Tokens")
                .and_then(serde_json::Value::as_str),
            Some("stored_session_totals")
        );
        assert_eq!(
            doc.pointer("/sessions/0/Metrics/StoredTotalsDelta"),
            Some(&serde_json::Value::from(720)),
            "the stored-versus-derived delta must survive the snapshot cache"
        );
        let loaded =
            load_sqlite_snapshot_from(&database, &snapshot).expect("schema seven cache hit");
        assert_eq!(loaded[0].metrics.provenance.duration, "timestamp_span");
        assert_eq!(loaded[0].metrics.stored_totals_delta, 720);
        assert_eq!(loaded[0].metrics.provenance.tokens, "stored_session_totals");
        let mut old = doc;
        old["schema_version"] = serde_json::Value::from(6);
        fs::write(
            &snapshot,
            serde_json::to_vec(&old).expect("schema six json"),
        )
        .expect("write old snapshot");
        assert!(load_sqlite_snapshot_from(&database, &snapshot).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn orphaned_temp_siblings_are_swept_on_cache_load() {
        // Pass-7 P7-5: a crashed writer leaves `<name>.json.tmp.<pid>.<seq>`
        // siblings behind forever. The sweep (run when the cache loads,
        // production age one hour) removes only temp siblings past the
        // age; real cache files are never touched.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-sweep-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let cache = root.join("sessions.json");
        fs::write(&cache, b"{}").expect("write cache");
        let orphan = unique_temp_path(&cache);
        fs::write(&orphan, b"torn").expect("write orphan temp");
        let neighbor = root.join("unrelated.txt");
        fs::write(&neighbor, b"keep").expect("write neighbor");
        let removed = sweep_orphaned_temps(&cache, std::time::Duration::ZERO);
        assert_eq!(removed, 1, "the orphaned temp is removed");
        assert!(!orphan.exists(), "orphan temp is gone");
        assert!(cache.exists(), "the cache itself is untouched");
        assert!(neighbor.exists(), "non-temp neighbors are untouched");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dead_paths_are_pruned_on_load_and_do_not_resurrect() {
        // Pass-8 F8-3: cache entries whose source file no longer exists
        // used to accumulate forever (761 of 1,487 entries dead on the
        // operator snapshot). Pruning at load marks the cache dirty so
        // the next save persists the smaller snapshot; live entries and
        // dir listings are untouched.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-prune-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let live = root.join("live.jsonl");
        let dead = root.join("dead.jsonl");
        let listed = root.join("listed");
        let ghost = root.join("ghost");
        fs::write(&live, b"session").expect("write live file");
        fs::write(&dead, b"session").expect("write dead file");
        fs::create_dir_all(&listed).expect("create listed dir");

        let session = Session {
            name: "cached".to_string(),
            path: live.to_string_lossy().to_string(),
            cwd: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        let mut cache = SessionCache::default();
        store_session(&live, &session, &mut cache).expect("store live entry");
        store_session(&dead, &session, &mut cache).expect("store dead entry");
        let listing = DirCacheEntry {
            mod_time: 0,
            files: Vec::new(),
            dirs: Vec::new(),
        };
        cache
            .dirs
            .insert(listed.to_string_lossy().to_string(), listing.clone());
        cache
            .dirs
            .insert(ghost.to_string_lossy().to_string(), listing);
        assert_eq!(cache.entry_count(), 2);

        fs::remove_file(&dead).expect("delete dead source");
        let pruned = prune_dead_entries(&mut cache);
        assert_eq!(pruned, 2, "one dead entry plus one dead dir");
        assert_eq!(cache.entry_count(), 1, "the live entry survives");
        assert!(cache.dirty, "pruning must persist through the next save");
        assert!(
            cache
                .dirs
                .contains_key(&listed.to_string_lossy().to_string()),
            "live dir listings are untouched"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn entry_bound_evicts_oldest_mod_time_first() {
        // Pass-8 F8-3: the snapshot used to grow without bound. Past
        // MAX_SESSION_CACHE_ENTRIES the entries with the oldest source
        // mtime drop first (oldest work is least likely to be re-read).
        let root = std::env::temp_dir().join(format!(
            "agenttrace-bound-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache::default();
        let mut paths = Vec::new();
        for i in 0..5u64 {
            let file = root.join(format!("session-{i}.jsonl"));
            fs::write(&file, b"session").expect("write session file");
            let stamp = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_000_000 + i * 1_000);
            let handle = fs::File::options()
                .write(true)
                .open(&file)
                .expect("open file");
            handle.set_modified(stamp).expect("set deterministic mtime");
            drop(handle);
            let session = Session {
                name: format!("session-{i}"),
                path: file.to_string_lossy().to_string(),
                cwd: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            };
            store_session(&file, &session, &mut cache).expect("store entry");
            paths.push(file.to_string_lossy().to_string());
        }
        assert_eq!(cache.entry_count(), 5);

        let dropped = enforce_entry_bound(&mut cache, 3);
        assert_eq!(dropped, 2, "the two oldest entries drop");
        assert_eq!(cache.entry_count(), 3);
        assert!(cache.dirty, "eviction must persist through the next save");
        assert!(
            !paths[..2]
                .iter()
                .any(|path| cache.entries.contains_key(&cache_key(Path::new(path)))),
            "oldest mtimes are the ones evicted"
        );
        assert!(
            paths[2..]
                .iter()
                .all(|path| cache.entries.contains_key(&cache_key(Path::new(path)))),
            "newest mtimes all survive"
        );
        // Idempotent: an in-bounds cache is left alone and stays clean.
        let before = cache.dirty;
        let dropped_again = enforce_entry_bound(&mut cache, 3);
        assert_eq!(dropped_again, 0);
        assert_eq!(cache.dirty, before);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn tokens_reasoning_survives_the_cache_round_trip() {
        // CU-20: the reasoning breakdown is part of the cached metrics
        // schema (TokensReasoning); a cache written by this build must
        // restore it, and one written before the field existed must
        // default to zero instead of failing to deserialize.
        let metrics = Metrics {
            tokens_input: 80,
            tokens_output: 60,
            tokens_reasoning: 40,
            ..Metrics::default()
        };
        let cached = GoMetrics::from_metrics(&metrics);
        assert_eq!(cached.tokens_reasoning, 40);
        let restored: Metrics = cached.into_metrics();
        assert_eq!(restored.tokens_reasoning, 40);
        assert_eq!(restored.tokens_output, 60);

        // Old-schema JSON without TokensReasoning deserializes to zero.
        let legacy = serde_json::json!({
            "TokensInput": 5,
            "TokensOutput": 5
        });
        let legacy: GoMetrics = serde_json::from_value(legacy).expect("legacy entry parses");
        assert_eq!(legacy.tokens_reasoning, 0);
    }

    #[test]
    fn byte_bound_evicts_oldest_entries_until_the_estimate_fits() {
        // CU-22: the entry-count bound alone cannot stop unbounded growth;
        // once the serialized estimate exceeds the ceiling the oldest
        // source files drop first, in the same order as the count bound.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-byte-bound-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache::default();
        let mut paths = Vec::new();
        for i in 0..4u64 {
            let file = root.join(format!("session-{i}.jsonl"));
            fs::write(&file, b"session").expect("write session file");
            let stamp = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_000_000 + i * 1_000);
            let handle = fs::File::options()
                .write(true)
                .open(&file)
                .expect("open file");
            handle.set_modified(stamp).expect("set deterministic mtime");
            drop(handle);
            let session = Session {
                name: format!("session-{i}"),
                path: file.to_string_lossy().to_string(),
                cwd: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            };
            store_session(&file, &session, &mut cache).expect("store entry");
            paths.push(file.to_string_lossy().to_string());
        }

        // Four default entries are tiny; use a ceiling that fits only the
        // newest two to exercise the eviction path deterministically.
        // Byte-true ceiling (rm-298): the whole written document minus
        // the two oldest members, each freeing its `key`:value plus one
        // comma (four members remain as each drops). BTreeMap order is
        // session-0..3, matching the ascending mtimes.
        let members: Vec<usize> = cache
            .entries
            .iter()
            .map(|(path, entry)| {
                json_key_len(path) + 1 + serde_json::to_string(entry).expect("serialize").len() + 1
            })
            .collect();
        let max = serialized_doc_size(&cache) - members[0] - members[1];
        let dropped = enforce_byte_bound(&mut cache, max);
        assert_eq!(dropped, 2, "the two oldest entries drop first");
        assert_eq!(cache.entry_count(), 2);
        assert!(cache.dirty, "eviction must persist through the next save");
        assert!(
            !paths[..2]
                .iter()
                .any(|path| cache.entries.contains_key(&cache_key(Path::new(path)))),
            "oldest mtimes are the ones evicted"
        );
        assert!(
            paths[2..]
                .iter()
                .all(|path| cache.entries.contains_key(&cache_key(Path::new(path)))),
            "newest mtimes survive"
        );
        // In-bounds caches stay untouched and clean.
        let before = cache.dirty;
        assert_eq!(enforce_byte_bound(&mut cache, usize::MAX), 0);
        assert_eq!(cache.dirty, before);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bounds_count_deduplicated_paths_once() {
        // A11-2 (cycle 7): a decoded entry lives in both `entries` and
        // `raw_entries` (see `cached_entry`). Chaining the two maps'
        // keys counted such a path twice, so the entry bound wasted
        // drop slots and under-dropped in one pass (the two oldest
        // slots were the same path), and the byte bound summed both
        // copies and over-evicted near the ceiling. Both bounds now
        // walk the deduplicated union.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-union-bound-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache::default();
        let mut paths = Vec::new();
        for i in 0..4u64 {
            let file = root.join(format!("session-{i}.jsonl"));
            fs::write(&file, b"session").expect("write session file");
            let stamp = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(2_000_000 + i * 1_000);
            let handle = fs::File::options()
                .write(true)
                .open(&file)
                .expect("open file");
            handle.set_modified(stamp).expect("set deterministic mtime");
            drop(handle);
            let session = Session {
                name: format!("session-{i}"),
                path: file.to_string_lossy().to_string(),
                cwd: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            };
            store_session(&file, &session, &mut cache).expect("store entry");
            paths.push(file.to_string_lossy().to_string());
        }
        // Simulate a loaded cache: every decoded entry is mirrored in
        // raw form exactly the way a load + re-decode leaves it.
        cache.raw_entries = cache
            .entries
            .iter()
            .map(|(path, entry)| {
                (
                    path.clone(),
                    serde_json::to_value(entry).expect("serialize entry"),
                )
            })
            .collect();

        assert_eq!(cache.entry_count(), 4);
        assert_eq!(
            cache_paths_sized_once(&cache).len(),
            4,
            "the union helper must size each path once, not once per map copy"
        );

        let dropped = enforce_entry_bound(&mut cache, 2);
        assert_eq!(dropped, 2, "two distinct paths drop, not two copies of one");
        assert_eq!(cache.entry_count(), 2);
        for path in &paths[..2] {
            let key = cache_key(Path::new(path));
            assert!(!cache.entries.contains_key(&key));
            assert!(!cache.raw_entries.contains_key(&key));
        }
        for path in &paths[2..] {
            let key = cache_key(Path::new(path));
            assert!(cache.entries.contains_key(&key));
        }

        // Byte bound over the same duplicated state: the ceiling fits
        // exactly the two newest distinct paths, so exactly those two
        // survive — the pre-fix double-counted total evicted more.
        let mut cache = SessionCache::default();
        for (i, path) in paths.iter().enumerate() {
            let file = PathBuf::from(path);
            let session = Session {
                name: format!("session-{i}"),
                path: path.clone(),
                cwd: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            };
            store_session(&file, &session, &mut cache).expect("store entry");
        }
        cache.raw_entries = cache
            .entries
            .iter()
            .map(|(path, entry)| {
                (
                    path.clone(),
                    serde_json::to_value(entry).expect("serialize entry"),
                )
            })
            .collect();
        let size_of: std::collections::BTreeMap<String, usize> =
            cache_paths_sized_once(&cache).into_iter().collect();
        assert!(
            size_of.values().all(|bytes| *bytes > 0),
            "entries must be sized, not zeroed"
        );
        // `paths` was stored oldest-mtime first; the byte-true ceiling
        // (rm-298) fits exactly the two newest distinct paths: the whole
        // written document minus the two oldest members, each freeing
        // its `key`:value plus one comma (four members remain as each
        // drops).
        let member = |path: &str| json_key_len(path) + 1 + size_of[path] + 1;
        let max = serialized_doc_size(&cache) - member(&paths[0]) - member(&paths[1]);
        let dropped = enforce_byte_bound(&mut cache, max);
        assert_eq!(dropped, 2, "the two oldest distinct paths drop, exactly");
        assert_eq!(cache.entry_count(), 2);
        for path in &paths[..2] {
            let key = cache_key(Path::new(path));
            assert!(!cache.entries.contains_key(&key));
            assert!(!cache.raw_entries.contains_key(&key));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn headerless_cache_entries_are_oldest_not_unevictable() {
        // F5-5 (cycle-5 review, carried into the union form): a raw
        // entry without a decodable header used to be skipped by the
        // bounds entirely, so corrupt entries could pin the cache at
        // its ceiling forever. They count as the oldest age instead.
        let mut cache = SessionCache::default();
        cache.raw_entries.insert(
            "/gone/headerless-entry".to_string(),
            serde_json::json!({"nonsense": true}),
        );
        cache.raw_entries.insert(
            "/gone/dated-entry".to_string(),
            serde_json::json!({"mod_time": 5, "size": 1, "session": {"Name": "dated"}}),
        );
        assert_eq!(cache.entry_count(), 2);
        let dropped = enforce_entry_bound(&mut cache, 1);
        assert_eq!(dropped, 1);
        assert!(
            !cache.raw_entries.contains_key("/gone/headerless-entry"),
            "the headerless entry is the oldest and must be evictable"
        );
        assert!(cache.raw_entries.contains_key("/gone/dated-entry"));
    }

    #[test]
    fn byte_bound_covers_the_written_document_not_a_model() {
        // rm-298: `enforce_byte_bound` used to size entry VALUES only,
        // while `save_session_cache` also writes per-path keys, JSON
        // punctuation, the top-level fields, and the whole `dirs` map —
        // on a real 5,293-entry corpus that gap wrote sessions.json at
        // 72,257,626 B = 108.9% of the 64 MiB "hard bound" (assess
        // c22757c9). The contract the doc comment states is about the
        // WRITTEN FILE, so the fixture asserts on the file on disk, not
        // on the estimator's model of it: with a ceiling the old
        // values-only estimate called in-bounds, the written document
        // must still fit under it, and eviction must be oldest-first.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-byte-true-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache {
            path: root.join("sessions.json"),
            ..SessionCache::default()
        };
        for i in 0..8i64 {
            let path = format!("/corpus/projects/probe/session-{i:02}-with-a-long-key-name.jsonl");
            let value = serde_json::json!({
                "mod_time": 1_000_000 + i * 1_000,
                "size": 1,
                "session": {
                    "Name": format!("session-{i}"),
                    "Path": path,
                    "pad": "x".repeat(1_500),
                },
            });
            cache.raw_entries.insert(path, value);
        }
        for i in 0..4i64 {
            cache.dirs.insert(
                format!("/corpus/projects/probe-{i}"),
                DirCacheEntry {
                    mod_time: 500_000 + i,
                    files: vec!["f".repeat(120); 6],
                    dirs: Vec::new(),
                },
            );
        }
        // The pre-rm-298 model's in-bounds ceiling: the values-only sum.
        let values_only: usize = cache_paths_sized_once(&cache).iter().map(|(_, b)| *b).sum();
        let max = values_only;
        enforce_byte_bound(&mut cache, max);
        save_session_cache(&mut cache).expect("save cache");
        let written = fs::metadata(&cache.path).expect("cache file written").len() as usize;
        assert!(
            written <= max,
            "the written sessions.json must obey the byte bound: {written} > {max}"
        );
        // Eviction stays oldest-mtime-first and never touches dirs.
        assert!(
            !cache
                .raw_entries
                .contains_key("/corpus/projects/probe/session-00-with-a-long-key-name.jsonl"),
            "the oldest entry is the first to go"
        );
        assert!(
            cache
                .raw_entries
                .contains_key("/corpus/projects/probe/session-07-with-a-long-key-name.jsonl"),
            "the newest entry survives"
        );
        assert_eq!(
            cache.dirs.len(),
            4,
            "entry eviction never drops dir listings"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn serialized_doc_size_predicts_the_written_file_exactly() {
        // rm-298: the byte bound is only as honest as its projection.
        // Pin that `serialized_doc_size` equals the file
        // `save_session_cache` writes, byte for byte — including a key
        // that needs JSON escaping, a path living in both maps (the
        // decoded copy overwrites the raw copy at save time), and a
        // non-empty dirs map — so any drift between the model and the
        // writer fails here first.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-doc-size-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache {
            path: root.join("sessions.json"),
            ..SessionCache::default()
        };
        for i in 0..3i64 {
            let path = format!("/corpus/projects/probe \"quoted-{i}\"\\slash/s-{i}.jsonl");
            let session = GoSession {
                name: format!("session-{i}"),
                path: path.clone(),
                ..GoSession::default()
            };
            let value = serde_json::json!({
                "mod_time": 900_000 + i * 1_000,
                "size": 1,
                "session": serde_json::to_value(&session).expect("serialize session"),
                "pad": "y".repeat(600),
            });
            cache.raw_entries.insert(path.clone(), value);
            if i == 1 {
                // Raw and decoded copies differ for this path: the
                // decoded copy overwrites the raw one at save time, and
                // the projection must count the WRITTEN form (the raw
                // copy's stray fields never reach the file).
                cache.entries.insert(
                    path,
                    CacheEntry {
                        mod_time: 900_000 + i * 1_000,
                        size: 1,
                        session,
                    },
                );
            }
        }
        cache.dirs.insert(
            "/corpus/projects/probe \"quoted-0\"\\slash".to_string(),
            DirCacheEntry {
                mod_time: 500_000,
                files: vec!["f".repeat(120); 6],
                dirs: Vec::new(),
            },
        );
        let projected = serialized_doc_size(&cache);
        save_session_cache(&mut cache).expect("save cache");
        let written = fs::metadata(&cache.path).expect("cache file written").len() as usize;
        assert_eq!(
            written, projected,
            "the projection must equal the written file byte for byte"
        );
        let doc: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&cache.path).expect("read cache file"))
                .expect("written cache parses as JSON");
        assert!(
            doc.get("dirs").is_some(),
            "the dirs map is part of the written document"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dirs_map_gains_count_and_byte_bounds_of_its_own() {
        // rm-298: `dirs` had no bound at all — only entries did — so a
        // broad directory tree grew the snapshot without limit. The
        // listings now drop oldest mtime first until both the count
        // and the serialized-block budgets hold.
        let mut cache = SessionCache::default();
        for i in 0..6i64 {
            cache.dirs.insert(
                format!("/corpus/projects/probe-{i}"),
                DirCacheEntry {
                    mod_time: 400_000 + i * 1_000,
                    files: vec![format!("file-{i}-{}.jsonl", "f".repeat(60)); 4],
                    dirs: Vec::new(),
                },
            );
        }
        // Count bound alone: keep the three newest listings.
        let dropped = enforce_dirs_bound(&mut cache, 3, usize::MAX);
        assert_eq!(dropped, 3);
        assert_eq!(cache.dirs.len(), 3);
        assert!(!cache.dirs.contains_key("/corpus/projects/probe-0"));
        assert!(cache.dirs.contains_key("/corpus/projects/probe-5"));
        assert!(cache.dirty, "evicting listings marks the cache dirty");

        // Byte bound: one byte under the current block — the oldest of
        // the three goes and the block fits the budget.
        cache.dirty = false;
        let budget = json_object_len(dirs_member_bytes(&cache), cache.dirs.len()) - 1;
        let dropped = enforce_dirs_bound(&mut cache, usize::MAX, budget);
        assert_eq!(dropped, 1);
        assert_eq!(cache.dirs.len(), 2);
        assert!(!cache.dirs.contains_key("/corpus/projects/probe-3"));
        assert!(
            json_object_len(dirs_member_bytes(&cache), cache.dirs.len()) <= budget,
            "the serialized dirs block must fit the byte budget"
        );
        assert!(cache.dirty);
    }

    #[test]
    fn stale_dir_listings_from_the_pre_symlink_walker_are_dropped_once() {
        // Cycle 7 (Codex #42135): listings written before symlinked
        // child directories were followed silently hid those
        // directories until each parent's mtime changed, because the
        // cached replay never re-read the directory. A journal without
        // the current `dir_listing_version` loses its listings once at
        // load; a current one keeps them.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-listing-version-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("sessions.json");
        // The listing key must be a live directory: load-time pruning
        // evicts listings whose directory no longer exists.
        let listed_dir = root.join("real");
        fs::create_dir_all(&listed_dir).expect("create listed dir");
        let listing = serde_json::json!({
            "mod_time": 1,
            "files": [],
            "dirs": [],
        });
        let write_journal = |versioned: bool| {
            let mut doc = serde_json::Map::new();
            doc.insert(
                "schema_version".to_string(),
                serde_json::json!(SESSION_CACHE_SCHEMA_VERSION),
            );
            if versioned {
                doc.insert(
                    "dir_listing_version".to_string(),
                    serde_json::json!(DIR_LISTING_WALK_VERSION),
                );
            }
            doc.insert("entries".to_string(), serde_json::json!({}));
            doc.insert(
                "dirs".to_string(),
                serde_json::json!({listed_dir.to_string_lossy().to_string(): listing}),
            );
            fs::write(
                &journal,
                serde_json::to_string(&doc).expect("serialize journal"),
            )
            .expect("write journal");
        };

        // Shared env lock (see lib.rs `test_env`): sibling-module tests
        // (pricing, statusline) mutate the same variables.
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        write_journal(false);
        let cache = load_session_cache();
        assert_eq!(
            cache.dirs.len(),
            0,
            "unversioned listings from the pre-symlink walker must be dropped"
        );
        assert!(cache.dirty, "the one-time invalidation must persist");

        write_journal(true);
        let cache = load_session_cache();
        assert_eq!(cache.dirs.len(), 1, "current-version listings survive");
        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clear_cache_removes_every_artifact_and_only_those() {
        // rm-086: --clear-cache used to remove sessions.json and the two
        // SQLite snapshots while leaving the statusline journal and the
        // pricing catalog on disk — the two artifacts a
        // privacy-motivated user most expects to be gone. The artifact
        // set comes from cache_artifact_paths() itself (file names are
        // env-independent), so this fails if a store is added to the
        // code but not to the clear set.
        let root =
            std::env::temp_dir().join(format!("agenttrace-clear-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp cache root");
        let paths: Vec<PathBuf> = cache_artifact_paths()
            .iter()
            .map(|path| root.join(path.file_name().expect("artifact file name")))
            .collect();
        assert_eq!(
            paths.len(),
            5,
            "registry: session cache, two sqlite snapshots, statusline journal, pricing catalog"
        );
        for path in &paths {
            fs::write(path, b"x").expect("write artifact");
        }
        let bystander = root.join("unrelated.txt");
        fs::write(&bystander, b"x").expect("write bystander");
        // rm-086 review follow-up: superseded-version leftovers from older
        // builds must leave with the purge too (they are invisible to the
        // registry — no live code names them — but they hold the same
        // parsed metrics), while near-miss names stay untouched.
        let legacy_hermes = root.join("hermes-sqlite-v7-20250801.json");
        let legacy_opencode = root.join("opencode-sqlite-v7-20250801.json");
        let legacy_bystander = root.join("other-sqlite-v7.json");
        for path in [&legacy_hermes, &legacy_opencode, &legacy_bystander] {
            fs::write(path, b"x").expect("write legacy artifact");
        }

        // e2e through the real entry point: pin the env-aware roots
        // (the session-cache dir — and XDG_CACHE_HOME too, because the
        // pricing catalog resolves through user_cache_dir(), not through
        // AGENTTRACE_SESSION_CACHE_DIR) and drive clear_session_cache(),
        // which composes the registry with the legacy sweep —
        // remove_cache_artifacts alone would not exercise
        // legacy_cache_artifact_paths() at all. Artifacts are written
        // where the pinned env actually resolves them.
        let _env = crate::test_env::lock_env();
        let previous_cache_root = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        std::env::set_var("XDG_CACHE_HOME", root.join("xdg-cache"));
        let live: Vec<PathBuf> = cache_artifact_paths();
        for path in &live {
            fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
            fs::write(path, b"x").expect("write artifact");
        }
        clear_session_cache().expect("clear removes registry + legacy sweep");
        for path in &live {
            assert!(!path.exists(), "{} must be removed", path.display());
        }
        assert!(
            !legacy_hermes.exists(),
            "legacy versioned snapshot must be swept"
        );
        assert!(
            !legacy_opencode.exists(),
            "legacy versioned opencode snapshot must be swept"
        );
        assert!(
            bystander.exists(),
            "clear touches only registered artifacts, not the whole directory"
        );
        assert!(
            legacy_bystander.exists(),
            "the legacy sweep matches only the two store prefixes, not every *-sqlite-v*.json"
        );

        // A second pass is a no-op, not an error: every artifact is
        // already NotFound.
        clear_session_cache().expect("second clear is a no-op");

        match previous_cache_root {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        match previous_xdg_cache {
            Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn privacy_disclosure_lists_every_artifact() {
        // rm-086: PRIVACY.md must disclose every at-rest artifact the
        // code writes — the cache-root registry plus the preserved
        // history file — with its purge path. Names come from the same
        // constructors that build the paths, so adding a store without
        // disclosing it fails here.
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("PRIVACY.md");
        let privacy = fs::read_to_string(&repo_root)
            .unwrap_or_else(|err| panic!("read {}: {err}", repo_root.display()));
        let mut names: Vec<String> = cache_artifact_paths()
            .iter()
            .chain(std::iter::once(&crate::history::history_path()))
            .map(|path| {
                path.file_name()
                    .expect("artifact file name")
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        names.sort();
        names.dedup();
        for name in &names {
            assert!(
                privacy.contains(name.as_str()),
                "PRIVACY.md must disclose at-rest artifact {name}"
            );
        }
        // rm-086 review follow-up: the swept superseded-version leftovers
        // are disclosed as a pattern row, not as exact file names.
        for pattern in ["hermes-sqlite-v", "opencode-sqlite-v"] {
            assert!(
                privacy.contains(pattern),
                "PRIVACY.md must disclose the legacy {pattern}* purge"
            );
        }
    }

    #[test]
    #[cfg(unix)]
    fn session_cache_and_snapshot_artifacts_are_owner_only() {
        // rm-208: sessions.json and the SQLite snapshots carry session
        // names, tool arguments, and transcript-derived metrics. The
        // fs::write default (0644) hands them to every local account;
        // both artifacts must land owner-only.
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-artifact-perms-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let database = root.join("state.db");
        let snapshot = root.join("hermes.db.json");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&database, b"db").expect("write database");
        let session = Session {
            name: "private session".to_string(),
            path: database.to_string_lossy().to_string(),
            cwd: "/work/secret".to_string(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_sqlite_snapshot_at(&database, &snapshot, &[session]).expect("store snapshot");
        let mode = fs::metadata(&snapshot)
            .expect("snapshot exists")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "sqlite snapshot must be owner-only, got {:o}",
            mode & 0o777
        );
        let mut cache = SessionCache {
            path: root.join("sessions.json"),
            ..Default::default()
        };
        save_session_cache(&mut cache).expect("save cache");
        let mode = fs::metadata(&cache.path)
            .expect("sessions.json exists")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "sessions.json must be owner-only, got {:o}",
            mode & 0o777
        );
        let _ = fs::remove_dir_all(root);
    }
}

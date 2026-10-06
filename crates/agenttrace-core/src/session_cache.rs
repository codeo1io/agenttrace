use crate::{Anomaly, Diagnostics, Metrics, Session, ToolWarning};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const SESSION_CACHE_SCHEMA_VERSION: i64 = 26;
// Bumped 24 -> 26 (session-cache store integrity batch): 25 and 26
// were consumed upstream between this base and the ceiling merge
// (upstream cb625d7 landed schema 26 for sum-once token accounting,
// Cadence 155/155 dedup 1.9x), so this implementation lands ON 26 —
// post-merge the shared store must be one version family, and
// reading a ceiling-written 26 journal with a 24 binary would torch
// the whole store for nothing. The bump also carries the rm-041 key
// semantics: entries and listing members may now hold
// losslessly-encoded non-UTF-8 keys (see `LOSSLESS_KEY_PREFIX`), so
// pre-26 entries keyed under `to_string_lossy` output are invalid
// once. What the bump does NOT do anymore is discard silently: see
// the schema-invalidations path in `load_session_cache` — parsed
// entries drop (the bump changed accounting), walk-current directory
// listings survive on their own version lane, and the invalidation
// is recorded in the journal instead of happening invisibly.
// Bumped 23 -> 24 (integration of run 6403d975, rm-436/437/438): the
// pi journal accounting batch (disclosure counters, upstream
// recorded-cost passthrough, per-block multi-model pricing) changed
// derived Metrics semantics, so v23 entries carry the pre-fix pi
// totals — the wrong numbers rm-436 exists to fix — with no
// disclosure line, and nothing regenerates them until each source
// file changes again. The batch landed against schema 21 and bumped
// it to 22 there (review fix F1); integration re-bases the bump onto
// the already-advanced ceiling (22 was rm-400/rm-401, 23 was rm-408)
// per the rm-230 convention: parser-semantics changes bump the schema
// so cached sessions regenerate under corrected accounting. v24
// invalidates them once: entries regenerate on the next scan.
// Bumped 22 -> 23 (cycle-1 review fix, run 555a174d, rm-408): GoMetrics
// gained `zero_usage_events` with `#[serde(default)]`, so a warm v22
// entry written before rm-408 still parses as fresh (size+mtime
// fingerprint unchanged, schema version equal) while the new counter
// defaults to 0 — the disclosure then reports clean zeros for exactly
// the historical sessions it exists to flag. Per the rm-230 convention
// the bump invalidates those entries once so they regenerate under the
// rm-408 accounting. Proven red-first by the review 342a1349 warm-cache
// PoC (a stripped v22 cache served a zero-usage session as clean) and
// pinned by `stale_schema_22_cache_cannot_mask_the_disclosure`.
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

/// How many `schema_invalidations` records the journal keeps. The
/// ledger discloses every schema upgrade the store survived (dropped
/// entries, kept listings); it is bounded so a journal that outlives
/// many upgrades cannot grow the disclosure without limit — the byte
/// bound counts it either way.
const MAX_SCHEMA_INVALIDATION_RECORDS: usize = 8;

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
/// stale v2 listings. v4: listing members and entry keys are
/// losslessly-encoded paths (`at-bytes:` percent-escaping for
/// non-UTF-8 names, rm-041) — a v3 listing holds `to_string_lossy`
/// member names, and the replay extends `listing.files` verbatim, so a
/// warm v3 journal keeps hiding the very non-UTF-8 journals the
/// lossless keys exist to surface (`bad\u{FFFD}name.jsonl` replays as
/// a path that does not exist). v3 members cannot be told apart from
/// genuine U+FFFD filenames, so the listings drop once at load
/// instead of being filtered.
const DIR_LISTING_WALK_VERSION: i64 = 4;

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
    /// Entry keys this process invalidated (freshness misses, header
    /// decode failures, prunes). The save-side merge under the cache
    /// lock uses the set to keep a concurrent writer's state from
    /// resurrecting keys that were deleted on purpose (rm-298
    /// residual).
    removed: BTreeSet<String>,
    /// Dir-listing keys this process invalidated; same role as
    /// `removed` for the `dirs` map.
    removed_dirs: BTreeSet<String>,
    /// Durable disclosure of schema invalidations observed by this
    /// process or carried forward from the journal — see
    /// `schema_invalidations()`. Persisted as the top-level
    /// `schema_invalidations` member at save time (bounded by
    /// `MAX_SCHEMA_INVALIDATION_RECORDS`).
    schema_invalidations: Vec<Value>,
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
    /// Pricing-catalog identity the snapshot was priced under (rm-196).
    /// `None` on snapshots written before the field existed: accepted
    /// once and stamped at the next store; a mismatched `Some` drops
    /// the snapshot so the sessions re-price.
    #[serde(default)]
    pricing_catalog_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirCacheEntry {
    mod_time: i64,
    /// Byte size of the directory itself at store time — the
    /// same-tick freshness guard (rm-041): mtime granularity can hide
    /// a mutation that landed inside the same tick as the stored
    /// listing, while adding or removing a child moves the directory's
    /// own size on the common local filesystems. Absent (listings
    /// stored before the field existed) compares mtime only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dir_size: Option<u64>,
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
    /// rm-436: USD cost the source journal recorded for usage blocks;
    /// round-trips so a cached session keeps its recorded-cost pricing
    /// instead of reverting to the catalog estimate on cache hit.
    #[serde(default, rename = "UpstreamCostUSD")]
    upstream_cost_usd: f64,
    /// rm-436/rm-437: parse-time disclosure counters (pi journals),
    /// round-tripped so cache hits keep disclosing.
    #[serde(
        default,
        rename = "DisclosureCounters",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    disclosure_counters: BTreeMap<String, usize>,
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
        // Advisory lock sibling of the store (see `CacheFileLock`):
        // registered so `--clear-cache` sweeps it with the rest. It is
        // empty — it carries no session data.
        session_cache_lock_path(),
        sqlite_snapshot_path("hermes"),
        sqlite_snapshot_path("opencode"),
        crate::statusline::statusline_capture_path(),
        crate::pricing::pricing_cache_path(),
    ]
}

/// Advisory-lock sibling for the session cache: `sessions.json.lock`
/// next to the store. It exists only to serialize the save/clear
/// read-modify-write between concurrent agenttrace processes
/// (rm-298 residual) and holds no data.
pub fn session_cache_lock_path() -> PathBuf {
    lock_sibling(&session_cache_path())
}

fn lock_sibling(path: &Path) -> PathBuf {
    match path.file_name() {
        Some(name) => {
            let mut file_name = name.to_os_string();
            file_name.push(".lock");
            path.with_file_name(file_name)
        }
        None => {
            let mut fallback = path.as_os_str().to_os_string();
            fallback.push(".lock");
            PathBuf::from(fallback)
        }
    }
}

/// Bounded advisory lock over a cache rewrite (rm-298 residual).
///
/// Protocol (MSRV-1.80 std-only — `File::try_lock` is 1.89):
/// - acquire = `create_new` (O_EXCL) on the lock sibling, fenced by a
///   `<pid>-<unique>` token written into it;
/// - release = remove the file only if it still carries our token
///   (a peer that stole a stale lock owns the name now — the token
///   check keeps our Drop from deleting theirs);
/// - staleness is content-first, age-second: a lock whose token
///   names a process that no longer exists — or whose content is not
///   a token at all (a stray file parked on the lock name, or a
///   writer that crashed between creating the file and writing its
///   token, past `CACHE_LOCK_EMPTY_GRACE`) — is stolen immediately by
///   atomic rename; a live holder is stolen only after
///   `CACHE_LOCK_STALE_AFTER`, which covers a wedged-but-alive
///   writer. Content-first matters for liveness too: an inert file on
///   the lock name must not wedge every future save behind the full
///   wait budget;
/// - a stolen victim fences its own write (`still_ours` re-reads the
///   token before the rename in `save_session_cache`), so a writer
///   whose lock was stolen mid-flight aborts and re-serializes
///   instead of clobbering the thief's state;
/// - stealing (`STALE_AFTER`) fires long before waiting gives up
///   (`WAIT`): giving up means proceeding unlocked, which reopens the
///   lost-update window the lock exists to close, so it is the last
///   resort, reached only when a live peer holds for `CACHE_LOCK_WAIT`
///   straight.
struct CacheFileLock(PathBuf, String);

const CACHE_LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(30);
const CACHE_LOCK_RETRY: std::time::Duration = std::time::Duration::from_millis(10);
const CACHE_LOCK_STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(5);
const CACHE_LOCK_EMPTY_GRACE: std::time::Duration = std::time::Duration::from_secs(1);
/// How many fenced re-serialization attempts `save_session_cache`
/// makes when its lock keeps being stolen mid-flight before landing
/// the write anyway (see the fence in `save_session_cache`).
const CACHE_LOCK_ATTEMPTS: usize = 3;

impl CacheFileLock {
    fn acquire(path: &Path) -> Option<Self> {
        let lock_path = lock_sibling(path);
        if let Some(parent) = lock_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let token = format!(
            "{}-{}",
            std::process::id(),
            unique_temp_path(&lock_path).display()
        );
        let deadline = std::time::Instant::now() + CACHE_LOCK_WAIT;
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    drop(file);
                    // Owner-only, like every other cache artifact: the
                    // lock sits in the same private directory.
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(&lock_path, fs::Permissions::from_mode(0o600));
                    }
                    let _ = fs::write(&lock_path, token.as_bytes());
                    return Some(CacheFileLock(lock_path, token));
                }
                Err(_) => {
                    if lock_stealable(&lock_path) {
                        // Atomic claim: only one racer's rename
                        // succeeds; the loser re-loops. The victim, if
                        // alive, fences at its rename and retries.
                        let claimed = unique_temp_path(&lock_path);
                        if fs::rename(&lock_path, &claimed).is_ok() {
                            let _ = fs::remove_file(&claimed);
                        }
                        continue;
                    }
                    if std::time::Instant::now() >= deadline {
                        // Wait budget spent on a live holder: proceed
                        // unlocked (see the struct comment for why that
                        // is the last-resort degrade).
                        return None;
                    }
                    std::thread::sleep(CACHE_LOCK_RETRY);
                }
            }
        }
    }

    /// Fencing check: true while the lock file on disk still carries
    /// exactly our token. A writer whose lock was stolen while it was
    /// mid-rewrite must not land its rename over the thief's state.
    fn still_ours(&self) -> bool {
        matches!(fs::read_to_string(&self.0), Ok(ref text) if text == &self.1)
    }
}

/// Whether the lock file at `path` may be stolen right now: content
/// first (empty past the grace window, unreadable, or a token naming
/// a dead process), age second (a live-token lock is stolen only past
/// `CACHE_LOCK_STALE_AFTER` — a wedged-but-alive writer; the common
/// crashed-writer case is caught instantly by the dead pid). An empty
/// file inside the grace window is a creator between create and
/// token-write, so it is respected.
fn lock_stealable(lock_path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return false;
    };
    let age = metadata
        .modified()
        .ok()
        .and_then(|moment| moment.elapsed().ok())
        // An unreadable clock reads as fully aged: the age lane stays
        // usable instead of the lock becoming unstealable.
        .unwrap_or(CACHE_LOCK_STALE_AFTER);
    match fs::read_to_string(lock_path) {
        Ok(text) if text.is_empty() => age > CACHE_LOCK_EMPTY_GRACE,
        Ok(text) => match lock_token_pid(&text) {
            Some(pid) if lock_pid_is_alive(pid) => age > CACHE_LOCK_STALE_AFTER,
            // Not a recognizable token: a stray file parked on the
            // lock name (never ours to respect) or a hand-mangled
            // leftover — steal now rather than wedge every future
            // save behind it.
            _ => true,
        },
        Err(_) => true,
    }
}

/// Leading `<pid>-` of a lock token, when the content looks like one.
fn lock_token_pid(text: &str) -> Option<u32> {
    let pid = text.split('-').next()?;
    if !pid.is_empty() && pid.bytes().all(|byte| byte.is_ascii_digit()) {
        pid.parse().ok()
    } else {
        None
    }
}

#[cfg(unix)]
fn lock_pid_is_alive(pid: u32) -> bool {
    // /proc is the std-only liveness probe on the unix family this
    // tool ships on; where it is absent the age lane still catches
    // the crash.
    Path::new("/proc").join(pid.to_string()).exists()
}

#[cfg(not(unix))]
fn lock_pid_is_alive(_pid: u32) -> bool {
    true
}

impl Drop for CacheFileLock {
    fn drop(&mut self) {
        if matches!(fs::read_to_string(&self.0), Ok(ref text) if text == &self.1) {
            let _ = fs::remove_file(&self.0);
        }
        // A missing or foreign-tokened lock is not ours to remove —
        // leave it; the stale-steal path reclaims crashed writers'
        // leftovers.
    }
}

pub fn clear_session_cache() -> anyhow::Result<()> {
    // Serialize with any in-flight cache rewrite before removing the
    // artifacts (rm-298 residual). A clear that races a save can
    // still end with a resurrected store written after the purge —
    // that is inherent to the operation — but the two no longer
    // interleave mid-write.
    let _lock = CacheFileLock::acquire(&session_cache_path());
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
        || matches!(
            &snapshot.pricing_catalog_id,
            Some(stamped) if stamped != crate::pricing::catalog_identity()
        )
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
        pricing_catalog_id: Some(crate::pricing::catalog_identity().to_string()),
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
    let observed_schema_version = doc.get("schema_version").and_then(Value::as_i64);
    if observed_schema_version != Some(SESSION_CACHE_SCHEMA_VERSION) {
        // Store-integrity lead: a schema bump invalidates parsed
        // entries by design — every bump in the history above changed
        // parse or accounting semantics, so stale numbers must never
        // be served. The bump must not silently torch the whole
        // store, though. Directory listings are walk semantics on
        // their own version lane (`DIR_LISTING_WALK_VERSION`), so
        // listings whose lane is still current survive the bump — an
        // upgrade costs a re-parse, not a full tree re-walk — and the
        // invalidation itself is recorded durably in the journal
        // (`schema_invalidations`) so the discard is disclosed instead
        // of invisible. The next save re-writes the store at the
        // current schema with the survivor set: self-heal forward.
        let listings_current = doc.get("dir_listing_version").and_then(Value::as_i64)
            == Some(DIR_LISTING_WALK_VERSION);
        let mut dirs: BTreeMap<String, DirCacheEntry> = BTreeMap::new();
        if listings_current {
            if let Some(Value::Object(listing_map)) = doc.get("dirs") {
                for (key, raw) in listing_map {
                    if let Ok(entry) = serde_json::from_value::<DirCacheEntry>(raw.clone()) {
                        dirs.insert(key.clone(), entry);
                    }
                }
            }
        }
        let dropped_entries = doc
            .get("entries")
            .and_then(Value::as_object)
            .map(|entries| entries.len())
            .unwrap_or(0);
        let dropped_dirs = doc
            .get("dirs")
            .and_then(Value::as_object)
            .map(|dirs| dirs.len())
            .unwrap_or(0)
            .saturating_sub(dirs.len());
        let mut schema_invalidations = persisted_schema_invalidations(&doc);
        schema_invalidations.push(serde_json::json!({
            "reason": "schema",
            "observed_schema_version": observed_schema_version,
            "schema_version": SESSION_CACHE_SCHEMA_VERSION,
            "dropped_entries": dropped_entries,
            "dropped_dirs": dropped_dirs,
            "kept_dir_listings": dirs.len(),
        }));
        cap_schema_invalidations(&mut schema_invalidations);
        return SessionCache {
            path,
            dirs,
            schema_invalidations,
            dirty: true,
            ..SessionCache::default()
        };
    }
    // Pricing-catalog identity (rm-196): cached per-session costs are
    // only valid for the catalog that priced them — the file
    // fingerprints alone cannot tell a catalog refresh from an
    // untouched session. A stamped id that disagrees with the active
    // catalog (snapshot bump on upgrade, `--update-pricing` refresh,
    // override edit) drops every entry once so they re-price; an
    // unstamped legacy cache is accepted as-is and stamped at the next
    // save, so upgrading alone never forces a full rescan.
    let pricing_catalog_id = crate::pricing::catalog_identity().to_string();
    let pricing_matches = doc
        .get("pricing_catalog_id")
        .and_then(Value::as_str)
        .map(|stamped| stamped == pricing_catalog_id)
        .unwrap_or(true);
    let raw_entries = if pricing_matches {
        doc.get("entries")
            .and_then(Value::as_object)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|(_, value)| decode_cache_entry_header(value).is_some())
                    .map(|(path, value)| (path.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
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
    let mut schema_invalidations = persisted_schema_invalidations(&doc);
    if !pricing_matches {
        // Review fix R3 (2026-10-06, run 2d92ee95 independent_review
        // 90bc04b9): the pricing arm drops every entry so they re-parse
        // at the new catalog, but unlike the schema arm it appended
        // NOTHING to the disclosure ledger — the invalidation was
        // invisible (acceptance contract on the batch lead, rm-292:
        // "a schema OR pricing-catalog mismatch invalidates entries
        // WITH DISCLOSURE"). Same ledger, same shape, plus the
        // discriminator and the observed id. Records written before the
        // `reason` key existed are schema-arm records by construction.
        let observed_pricing_catalog_id = doc
            .get("pricing_catalog_id")
            .and_then(Value::as_str)
            .unwrap_or("<absent>");
        let dropped_entries = doc
            .get("entries")
            .and_then(Value::as_object)
            .map(|entries| entries.len())
            .unwrap_or(0);
        schema_invalidations.push(serde_json::json!({
            "reason": "pricing_catalog",
            "observed_pricing_catalog_id": observed_pricing_catalog_id,
            "observed_schema_version": doc
                .get("schema_version")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            "schema_version": SESSION_CACHE_SCHEMA_VERSION,
            "dropped_entries": dropped_entries,
            "dropped_dirs": 0,
            "kept_dir_listings": dirs.len(),
        }));
        cap_schema_invalidations(&mut schema_invalidations);
    }
    let mut cache = SessionCache {
        path,
        raw_entries,
        dirs,
        schema_invalidations,
        // A catalog mismatch dirties the cache so the emptied entry map
        // is persisted with the new id at the next save (rm-196).
        dirty: listing_stale || !pricing_matches,
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
        // Keys may be losslessly-encoded non-UTF-8 paths (rm-041) —
        // decode before asking the filesystem.
        .filter(|path| !path_from_key(path).exists())
        .cloned()
        .collect();
    for path in dead_paths {
        if cache.entries.remove(&path).is_some() | cache.raw_entries.remove(&path).is_some() {
            // Remember the prune so the save-side merge under the
            // cache lock cannot resurrect a dead key (rm-298
            // residual).
            cache.removed.insert(path);
            pruned += 1;
        }
    }
    let dead_dirs: Vec<String> = cache
        .dirs
        .keys()
        .filter(|dir| !path_from_key(dir).exists())
        .cloned()
        .collect();
    for dir in dead_dirs {
        cache.dirs.remove(&dir);
        cache.removed_dirs.insert(dir);
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
        // Losslessly-encoded keys (rm-041) decode back to the real
        // path before the directory filter runs.
        .map(|key| path_from_key(key))
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

    /// Durable record of the cache invalidations this process observed
    /// (or carried forward from the journal): each record names the
    /// schema that was found, the schema that replaced it, how many
    /// entries were dropped, and how many directory listings survived.
    /// Surfaces like the doctor lane can disclose stale-schema
    /// stranding from here without the loader staying silent.
    pub fn schema_invalidations(&self) -> &[Value] {
        self.schema_invalidations.as_slice()
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
    if entry.mod_time != file_mod_time_nanos(&metadata)
        // Same-tick freshness guard (rm-041): a listing whose directory
        // changed size since it was stored is stale even when the
        // mtime has not moved yet.
        || dir_size_changed(entry, &metadata)
    {
        cache.dirs.remove(&key);
        cache.removed_dirs.insert(key);
        cache.dirty = true;
        return None;
    }
    // Listing members may be losslessly-encoded non-UTF-8 paths
    // (rm-041): decode back to the real paths the walk consumed.
    Some(CachedDirListing {
        files: entry.files.iter().map(|file| path_from_key(file)).collect(),
        dirs: entry
            .dirs
            .iter()
            .map(|child| path_from_key(child))
            .collect(),
    })
}

/// Same-tick freshness (rm-041): mtime granularity can hide a mutation
/// that landed inside the same tick as the stored listing — `create
/// file; read` within one coarse timestamp leaves the cached replay
/// serving the pre-create file set. A stored listing also carries the
/// directory's own byte size; adding or removing a child moves that
/// size on the common local filesystems, catching same-tick mutations
/// the mtime compare cannot. Listings stored before the size was
/// recorded (field absent) compare mtime only, so they behave exactly
/// as before. Residual, documented: a create+delete pair that nets the
/// size back to the stored value within one tick stays invisible until
/// the next mtime change — closing that would cost a `read_dir` per
/// hit, which is the walk the listing exists to skip.
fn dir_size_changed(entry: &DirCacheEntry, metadata: &fs::Metadata) -> bool {
    entry.dir_size.is_some_and(|size| size != metadata.len())
}

pub(crate) fn store_dir_listing(
    dir: &Path,
    files: &[PathBuf],
    dirs: &[PathBuf],
    cache: &mut SessionCache,
) -> anyhow::Result<()> {
    let metadata = fs::metadata(dir)?;
    let key = cache_key(dir);
    // A fresh store wins over any pending deletion of this key, so the
    // save-side merge keeps it (rm-298 residual).
    cache.removed_dirs.remove(&key);
    cache.dirs.insert(
        key,
        DirCacheEntry {
            mod_time: file_mod_time_nanos(&metadata),
            dir_size: Some(metadata.len()),
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
        // Remember the invalidation so the save-side merge under the
        // cache lock cannot resurrect a key this process dropped
        // (rm-298 residual).
        cache.removed.insert(path.to_string());
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

/// Serialized length of the `schema_invalidations` array as
/// `save_session_cache` writes it (brackets, commas, and records
/// included). Zero records are omitted from the document entirely, so
/// the projection — and a fresh store's bytes — stay byte-identical
/// to the pre-ledger layout.
fn invalidations_member_bytes(records: &[Value]) -> usize {
    if records.is_empty() {
        return 0;
    }
    serde_json::to_string(records)
        .map(|text| text.len())
        .unwrap_or(0)
}

/// Length of the document `save_session_cache` writes for these
/// blocks: the fixed top-level fields, the entries map, the `dirs`
/// member (when non-empty), the `schema_invalidations` ledger (when
/// non-empty), and the closing brace (rm-298).
fn doc_frame_len(
    entries_member_bytes: usize,
    entries_count: usize,
    dirs_bytes: usize,
    dirs_count: usize,
    invalidations_bytes: usize,
) -> usize {
    let mut total = format!(
        "{{\"schema_version\":{},\"dir_listing_version\":{},\"pricing_catalog_id\":",
        SESSION_CACHE_SCHEMA_VERSION, DIR_LISTING_WALK_VERSION
    )
    .len();
    // The catalog-identity stamp rm-196 writes into every cache file
    // is part of the frame, so the byte-true projection must count it:
    // `\"<identity>\"` serialized as JSON (the digest is hex, but stay
    // defensive and round-trip it through serde like every other
    // member).
    total += serde_json::to_string(crate::pricing::catalog_identity())
        .map(|stamped| stamped.len())
        .unwrap_or(0);
    total += 1 + "\"entries\":".len();
    total += json_object_len(entries_member_bytes, entries_count);
    if dirs_count > 0 {
        total += 1 + "\"dirs\":".len() + json_object_len(dirs_bytes, dirs_count);
    }
    if invalidations_bytes > 0 {
        total += 1 + "\"schema_invalidations\":".len() + invalidations_bytes;
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
        invalidations_member_bytes(&cache.schema_invalidations),
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
        invalidations_member_bytes(&cache.schema_invalidations),
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

/// Reads the persisted `schema_invalidations` ledger out of a cache
/// document, bounded to the last `MAX_SCHEMA_INVALIDATION_RECORDS`
/// records (oldest dropped first).
fn persisted_schema_invalidations(doc: &Map<String, Value>) -> Vec<Value> {
    let mut records = doc
        .get("schema_invalidations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    cap_schema_invalidations(&mut records);
    records
}

fn cap_schema_invalidations(records: &mut Vec<Value>) {
    let len = records.len();
    if len > MAX_SCHEMA_INVALIDATION_RECORDS {
        records.drain(0..len - MAX_SCHEMA_INVALIDATION_RECORDS);
    }
}

/// Merges the on-disk store into the in-memory one under the cache
/// lock (rm-298 residual): keys another process saved after this
/// process loaded are unioned in instead of clobbered by this save.
/// Keys deleted locally (`removed` / `removed_dirs`) stay deleted,
/// keys stored locally win over their on-disk copy, and entries
/// priced under a different catalog are left for the catalog gate to
/// re-price rather than served as if they were ours. A store the
/// merge cannot trust (foreign schema, foreign walk version,
/// unparsable bytes) is skipped, which degrades to the pre-merge
/// last-writer-wins behavior instead of guessing.
fn merge_concurrent_disk_state(cache: &mut SessionCache) {
    let Ok(text) = fs::read_to_string(&cache.path) else {
        return;
    };
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if doc.get("schema_version").and_then(Value::as_i64) != Some(SESSION_CACHE_SCHEMA_VERSION) {
        return;
    }
    if doc.get("dir_listing_version").and_then(Value::as_i64) != Some(DIR_LISTING_WALK_VERSION) {
        return;
    }
    let pricing_matches = doc
        .get("pricing_catalog_id")
        .and_then(Value::as_str)
        .map(|stamped| stamped == crate::pricing::catalog_identity())
        .unwrap_or(false);
    if pricing_matches {
        if let Some(Value::Object(entries)) = doc.get("entries") {
            for (key, raw) in entries {
                if cache.entries.contains_key(key)
                    || cache.raw_entries.contains_key(key)
                    || cache.removed.contains(key)
                {
                    continue;
                }
                match serde_json::from_value::<CacheEntry>(raw.clone()) {
                    Ok(entry) => {
                        cache.entries.insert(key.clone(), entry);
                    }
                    Err(_) => {
                        cache.raw_entries.insert(key.clone(), raw.clone());
                    }
                }
            }
        }
    }
    if let Some(Value::Object(dirs)) = doc.get("dirs") {
        for (key, raw) in dirs {
            if cache.dirs.contains_key(key) || cache.removed_dirs.contains(key) {
                continue;
            }
            if let Ok(entry) = serde_json::from_value::<DirCacheEntry>(raw.clone()) {
                cache.dirs.insert(key.clone(), entry);
            }
        }
    }
    // Carry the disk's disclosure records alongside ours.
    if let Some(records) = doc.get("schema_invalidations").and_then(Value::as_array) {
        for record in records {
            if !cache.schema_invalidations.contains(record) {
                cache.schema_invalidations.push(record.clone());
            }
        }
        cap_schema_invalidations(&mut cache.schema_invalidations);
    }
}

pub fn save_session_cache(cache: &mut SessionCache) -> anyhow::Result<()> {
    if let Some(parent) = cache.path.parent() {
        fs::create_dir_all(parent)?;
    }
    // rm-298 residual: save used to be a lockless read-modify-write —
    // two agenttrace processes that loaded the store before either
    // saved each clobbered the other's entries wholesale. The rewrite
    // now serializes on the advisory lock sibling, merges the freshly
    // re-read on-disk state under it, and FENCES the rename against
    // lock theft (`still_ours`): an attempt whose lock was stolen
    // while it built the document is discarded and re-serialized
    // around the thief's state, so concurrent writers compose instead
    // of racing (see `merge_concurrent_disk_state` and
    // `CacheFileLock`).
    for attempt in 0..CACHE_LOCK_ATTEMPTS {
        let lock = CacheFileLock::acquire(&cache.path);
        merge_concurrent_disk_state(cache);
        // Hard bounds after the merge, before serializing: beyond
        // MAX_SESSION_CACHE_ENTRIES the oldest-fingerprint entries are
        // dropped (pass-8 F8-3); the dirs map keeps its own count and byte
        // budgets (rm-298); and the serialized document is capped at
        // MAX_SESSION_CACHE_BYTES over every byte this function writes —
        // keys, punctuation, top-level fields, invalidation records, and
        // dirs included (pass-9 CU-22; byte-true rm-298). The bounds run
        // over the merged state so they hold for the document actually
        // written.
        enforce_entry_bound(cache, MAX_SESSION_CACHE_ENTRIES);
        enforce_dirs_bound(cache, MAX_SESSION_CACHE_DIRS, MAX_SESSION_CACHE_DIR_BYTES);
        enforce_byte_bound(cache, MAX_SESSION_CACHE_BYTES);
        let mut doc = Map::new();
        doc.insert(
            "schema_version".to_string(),
            Value::Number(SESSION_CACHE_SCHEMA_VERSION.into()),
        );
        doc.insert(
            "dir_listing_version".to_string(),
            Value::Number(DIR_LISTING_WALK_VERSION.into()),
        );
        doc.insert(
            "pricing_catalog_id".to_string(),
            Value::String(crate::pricing::catalog_identity().to_string()),
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
        // The invalidation ledger (schema upgrades this store survived) —
        // durable disclosure, bounded to the last
        // MAX_SCHEMA_INVALIDATION_RECORDS records and omitted entirely
        // when empty so a fresh store writes the pre-ledger layout.
        if !cache.schema_invalidations.is_empty() {
            doc.insert(
                "schema_invalidations".to_string(),
                Value::Array(cache.schema_invalidations.clone()),
            );
        }
        let tmp = unique_temp_path(&cache.path);
        write_private(&tmp, &serde_json::to_vec(&Value::Object(doc))?)?;
        // Fence the rename: if our lock was stolen while this document
        // was being built, a peer owns the rewrite — discard the
        // attempt and re-serialize around their on-disk state instead
        // of clobbering it. An unlocked attempt (acquire gave up) or a
        // lock still carrying our token proceeds.
        let fenced_out = matches!(&lock, Some(lock) if !lock.still_ours());
        if fenced_out {
            let _ = fs::remove_file(&tmp);
            if attempt + 1 < CACHE_LOCK_ATTEMPTS {
                continue;
            }
            // Last attempt fenced out too: fall through and land the
            // write — the merge above already took the thief's state
            // into account, and refusing to persist at all would drop
            // this process's own entries permanently.
        }
        fs::rename(tmp, &cache.path)?;
        return Ok(());
    }
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
    // The fallback path is display-only (Session.path is a String),
    // but it must name the real file: decode a losslessly-encoded key
    // (rm-041) instead of exposing the encoded form.
    let fallback_path = path_from_key(&key).to_string_lossy().to_string();
    Some(entry.session.clone().into_session(&fallback_path))
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
    // A fresh store wins over any pending deletion of this key, so the
    // save-side merge keeps it (rm-298 residual).
    cache.removed.remove(&key);
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
            upstream_cost_usd: metrics.upstream_cost_usd,
            disclosure_counters: metrics.disclosure_counters.clone(),
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
            upstream_cost_usd: self.upstream_cost_usd,
            disclosure_counters: self.disclosure_counters,
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
    if let Some(text) = path.to_str() {
        return text.to_string();
    }
    #[cfg(unix)]
    {
        lossless_key_from_bytes(std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str()))
    }
    // Non-unix platforms have no portable lossless byte round trip
    // through JSON keys; the lossy fallback preserves the prior
    // behavior there instead of failing the walk.
    #[cfg(not(unix))]
    {
        path.to_string_lossy().into_owned()
    }
}

/// Prefix marking a key whose source path was not valid UTF-8
/// (rm-041). Cache keys are JSON object keys, so a losslessly-encoded
/// key preserves every byte of the original path instead of
/// collapsing to `U+FFFD` — a `bad\xff\xfename.jsonl` journal used to
/// be invisible through the cached replay (the listing named a file
/// that does not exist) and two distinct non-UTF-8 names could
/// collide on one lossy key. Keys are absolute paths and therefore
/// begin with the root separator, which is what keeps the prefix
/// unambiguous against verbatim keys.
const LOSSLESS_KEY_PREFIX: &str = "at-bytes:";

#[cfg(unix)]
fn lossless_key_from_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut key = String::from(LOSSLESS_KEY_PREFIX);
    for &byte in bytes {
        match byte {
            // `%` itself is escaped, so the escape alphabet is
            // self-describing and decoding is unambiguous.
            b'%' => key.push_str("%25"),
            0x21..=0x7E => key.push(byte as char),
            _ => {
                let _ = write!(key, "%{byte:02X}");
            }
        }
    }
    key
}

/// Inverse of `cache_key` (rm-041): decodes a losslessly-encoded key
/// back to the original path. Keys without the prefix are verbatim
/// paths. A prefixed key that does not decode cleanly (a truncated or
/// malformed escape — only reachable through hand-edited journals)
/// fails safe: the key is returned as-is rather than guessed at.
fn path_from_key(key: &str) -> PathBuf {
    let Some(encoded) = key.strip_prefix(LOSSLESS_KEY_PREFIX) else {
        return PathBuf::from(key);
    };
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        decode_lossless_key(encoded)
            .map(|bytes| PathBuf::from(std::ffi::OsString::from_vec(bytes)))
            .unwrap_or_else(|| PathBuf::from(key))
    }
    // Non-unix: encoded keys are never produced (see `cache_key`), so
    // the verbatim fallback is the only honest answer.
    #[cfg(not(unix))]
    {
        PathBuf::from(key)
    }
}

#[cfg(unix)]
fn decode_lossless_key(encoded: &str) -> Option<Vec<u8>> {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let high = (hex[0] as char).to_digit(16)?;
            let low = (hex[1] as char).to_digit(16)?;
            decoded.push((high * 16 + low) as u8);
            index += 3;
        } else if byte.is_ascii_graphic() {
            decoded.push(byte);
            index += 1;
        } else {
            return None;
        }
    }
    Some(decoded)
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
            dir_size: None,
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
                    dir_size: None,
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
                dir_size: None,
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
                    dir_size: None,
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
    fn pricing_catalog_refresh_invalidates_cached_costs_once() {
        // rm-196: the cache's file fingerprints (mod_time+size) cannot
        // tell a pricing-catalog refresh from an untouched session, so
        // cached per-session costs used to survive snapshot bumps and
        // --update-pricing refreshes at stale rates. The journal now
        // stamps the active catalog identity; a stamp that disagrees
        // with the live catalog drops every entry once (they re-price),
        // an unstamped legacy journal is accepted as-is, and the same
        // stamp keeps every hit.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-id-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("sessions.json");
        // The entry path must exist on disk or load-time pruning would
        // remove it for an unrelated reason.
        let session_file = root.join("session.jsonl");
        fs::write(&session_file, b"{}\n").expect("write session file");
        let stamp = |pricing_catalog_id: Option<&str>| {
            let mut doc = serde_json::Map::new();
            doc.insert(
                "schema_version".to_string(),
                serde_json::json!(SESSION_CACHE_SCHEMA_VERSION),
            );
            if let Some(id) = pricing_catalog_id {
                doc.insert("pricing_catalog_id".to_string(), serde_json::json!(id));
            }
            doc.insert(
                "entries".to_string(),
                serde_json::json!({
                    session_file.to_string_lossy().to_string(): {
                        "mod_time": 1,
                        "size": 1,
                    }
                }),
            );
            doc.insert("dirs".to_string(), serde_json::json!({}));
            fs::write(
                &journal,
                serde_json::to_string(&doc).expect("serialize journal"),
            )
            .expect("write journal");
        };

        // Shared env lock (see lib.rs `test_env`): the load path reads
        // the live pricing catalog for its identity stamp.
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // The identity read here is whatever the process's active
        // catalog computes; the test only needs internal consistency
        // between the stamp and the loader, never a specific value.
        let live_id = crate::pricing::catalog_identity().to_string();

        stamp(Some("0123456789abcdef"));
        let cache = load_session_cache();
        assert_eq!(
            cache.raw_entries.len(),
            0,
            "a journal stamped with a foreign catalog id must re-price"
        );
        assert!(
            cache.dirty,
            "the invalidation must persist the emptied journal"
        );

        stamp(Some(&live_id));
        let cache = load_session_cache();
        assert_eq!(
            cache.raw_entries.len(),
            1,
            "a journal stamped with the active catalog id keeps its hits"
        );
        assert!(!cache.dirty, "a matching stamp must not dirty the journal");

        stamp(None);
        let cache = load_session_cache();
        assert_eq!(
            cache.raw_entries.len(),
            1,
            "an unstamped legacy journal is accepted, not mass-invalidated"
        );

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
            6,
            "registry: session cache, its advisory lock, two sqlite snapshots, statusline journal, pricing catalog"
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
        // The advisory lock sibling exists only while a rewrite holds
        // it (O_EXCL acquire, token-fenced release) and must be
        // owner-only for the while (rm-298 residual) — gone once
        // released.
        let lock = CacheFileLock::acquire(&cache.path).expect("lock acquired");
        let mode = fs::metadata(lock_sibling(&cache.path))
            .expect("sessions.json.lock exists while held")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "sessions.json.lock must be owner-only, got {:o}",
            mode & 0o777
        );
        drop(lock);
        assert!(
            !lock_sibling(&cache.path).exists(),
            "a released lock leaves no file behind"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lock_staleness_is_content_first_and_never_wedges_on_stray_files() {
        // rm-298 residual protocol: an inert file parked on the lock
        // name (or a token naming a process that no longer exists)
        // must be stealable IMMEDIATELY — a purely age-based rule
        // would wedge every save behind the wait budget — while a
        // fresh lock from a live process (ours) and the empty window
        // between create and token-write are respected.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-lock-stale-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let lock_path = lock_sibling(&root.join("sessions.json"));
        fs::create_dir_all(lock_path.parent().expect("parent")).expect("create cache dir");

        // A stray non-token file: stealable at once, whatever its age.
        fs::write(&lock_path, b"x").expect("write stray lock");
        assert!(
            lock_stealable(&lock_path),
            "a stray file never wedges saves"
        );

        // A token naming a dead process: stealable at once (this
        // process is alive by construction, so borrow a pid that is
        // not — /proc has none above pid_max). The liveness probe is
        // unix-only; elsewhere the age lane carries staleness.
        #[cfg(unix)]
        {
            fs::write(&lock_path, format!("999999999-{}", lock_path.display()))
                .expect("write dead-owner lock");
            assert!(
                lock_stealable(&lock_path),
                "a dead owner's lock is stale now"
            );
        }
        assert_eq!(lock_token_pid("999999999-x"), Some(999999999));
        assert_eq!(lock_token_pid("not-a-token"), None);
        assert_eq!(lock_token_pid(""), None);

        // A fresh empty file is a creator between create and
        // token-write: respected inside the grace window.
        fs::write(&lock_path, b"").expect("write empty lock");
        assert!(
            !lock_stealable(&lock_path),
            "a just-created empty lock belongs to a live creator"
        );

        // A live owner (this process) holding a fresh lock: respected.
        let lock = CacheFileLock::acquire(&root.join("sessions.json"))
            .expect("acquire over the respected lock states");
        assert!(lock.still_ours(), "the lock we hold carries our token");
        assert!(
            !lock_stealable(&lock.0),
            "a fresh lock from a live process is not stealable"
        );
        drop(lock);
        assert!(!lock_path.exists(), "release removes the lock file");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lossless_key_encoding_round_trips_every_path() {
        // rm-041: keys are JSON object keys, so a non-UTF-8 path must
        // survive the round trip without collapsing to `U+FFFD` —
        // while UTF-8 paths keep their verbatim key (nothing changes
        // for the 99.999% case) and hand-written junk fails safe.
        let verbatim = [
            "/home/u/.claude/sessions/2026/ok.jsonl",
            "/tmp/emoji-\u{1F600}-name.jsonl",
            "/tmp/percent-%-and-%25-name.jsonl",
            "/tmp/spaces in name.jsonl",
            // Valid multi-byte UTF-8 stays verbatim too — only non-
            // UTF-8 byte sequences need the encoded form.
            "/tmp/\u{00E4}-umlaut.jsonl",
        ];
        for path in verbatim {
            let path = PathBuf::from(path);
            let key = cache_key(&path);
            assert!(
                !key.starts_with(LOSSLESS_KEY_PREFIX),
                "UTF-8 key stays verbatim: {key}"
            );
            assert_eq!(path_from_key(&key), path, "verbatim key decodes to itself");
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let raw = [
                b"/tmp/bad\xff\xfename.jsonl".to_vec(),
                b"/tmp/pct-%-in-\xff-name.jsonl".to_vec(),
                b"/tmp/lone-\x80-start.jsonl".to_vec(),
            ];
            for bytes in raw {
                let path = PathBuf::from(std::ffi::OsString::from_vec(bytes.clone()));
                let key = cache_key(&path);
                assert!(
                    key.starts_with(LOSSLESS_KEY_PREFIX),
                    "non-UTF-8 key is encoded: {key}"
                );
                assert!(
                    key.is_ascii(),
                    "encoded keys are plain JSON-safe ASCII: {key}"
                );
                assert_eq!(
                    path_from_key(&key),
                    path,
                    "lossless key round-trips every byte"
                );
            }
            // Fail-safe: a prefixed key with a malformed escape is
            // returned verbatim rather than guessed at.
            assert_eq!(
                path_from_key("at-bytes:%zz-broken"),
                PathBuf::from("at-bytes:%zz-broken")
            );
        }
        // Keys without the prefix always decode to themselves.
        assert_eq!(
            path_from_key("/plain/path.jsonl"),
            PathBuf::from("/plain/path.jsonl")
        );
    }

    #[test]
    fn schema_invalidation_ledger_is_bounded_and_exposed() {
        // Store-integrity lead: the disclosure ledger is capped at
        // MAX_SCHEMA_INVALIDATION_RECORDS (a journal that outlives
        // many upgrades cannot grow it without limit) and survives
        // saves so the disclosure stays durable until `--clear-cache`.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-ledger-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // A journal already carrying a full ledger (eight records),
        // then one more schema flip: the oldest record drops, the new
        // one lands, and the cap holds.
        let mut doc = serde_json::Map::new();
        doc.insert(
            "schema_version".to_string(),
            serde_json::json!(SESSION_CACHE_SCHEMA_VERSION - 1),
        );
        doc.insert(
            "pricing_catalog_id".to_string(),
            serde_json::json!(crate::pricing::catalog_identity()),
        );
        doc.insert(
            "dir_listing_version".to_string(),
            serde_json::json!(DIR_LISTING_WALK_VERSION),
        );
        doc.insert(
            "schema_invalidations".to_string(),
            serde_json::json!((0..MAX_SCHEMA_INVALIDATION_RECORDS)
                .map(|i| {
                    serde_json::json!({"observed_schema_version": i, "schema_version": i + 1})
                })
                .collect::<Vec<_>>()),
        );
        fs::write(
            session_cache_path(),
            serde_json::to_string(&Value::Object(doc)).expect("serialize doc"),
        )
        .expect("write doc");

        let mut cache = load_session_cache();
        let exposed = cache.schema_invalidations();
        assert_eq!(
            exposed.len(),
            MAX_SCHEMA_INVALIDATION_RECORDS,
            "the ledger stays capped after appending the new record"
        );
        let observed: Vec<i64> = exposed
            .iter()
            .filter_map(|record| {
                record
                    .get("observed_schema_version")
                    .and_then(Value::as_i64)
            })
            .collect();
        // The hand-written 0th record was dropped; the flip's own
        // record (observed = SESSION_CACHE_SCHEMA_VERSION - 1) is last.
        assert!(
            !observed.contains(&0),
            "the oldest record drops past the cap"
        );
        assert_eq!(
            *observed.last().expect("ledger non-empty"),
            SESSION_CACHE_SCHEMA_VERSION - 1,
            "the newest flip is the newest record"
        );

        // The ledger persists through a save and reloads intact.
        save_session_cache(&mut cache).expect("save store");
        let raw = fs::read_to_string(session_cache_path()).expect("read store");
        let doc: Value = serde_json::from_str(&raw).expect("store parses");
        let persisted = doc
            .get("schema_invalidations")
            .and_then(Value::as_array)
            .expect("ledger persisted");
        assert_eq!(persisted.len(), MAX_SCHEMA_INVALIDATION_RECORDS);
        let cache = load_session_cache();
        assert_eq!(
            cache.schema_invalidations().len(),
            MAX_SCHEMA_INVALIDATION_RECORDS,
            "a matching-schema load carries the ledger forward untouched"
        );

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pricing_catalog_mismatch_is_disclosed_in_the_ledger() {
        // Review fix R3 (2026-10-06, run 2d92ee95 independent_review
        // 90bc04b9): the pricing arm re-parses every entry when the
        // catalog identity changes but used to record NOTHING — the
        // invalidation was invisible, violating the lead's acceptance
        // contract ("a schema or pricing-catalog mismatch invalidates
        // entries WITH DISCLOSURE"). The disclosure must land in the
        // SAME bounded ledger the schema arm writes, naming the reason
        // and the observed catalog id.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-flip-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join("session.jsonl");
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Populate a store at the CURRENT schema and CURRENT catalog.
        let mut cache = load_session_cache();
        let session = Session {
            name: "price".to_string(),
            path: journal.to_string_lossy().to_string(),
            cwd: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_session(&journal, &session, &mut cache).expect("store session");
        store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
            .expect("store listing");
        save_session_cache(&mut cache).expect("save store");
        assert!(cache.schema_invalidations().is_empty());

        // Rewrite the persisted pricing_catalog_id to a foreign value,
        // leaving schema + walk lane untouched: exactly the reviewer's
        // PoC B.
        let raw = fs::read_to_string(session_cache_path()).expect("read store");
        let mut foreign =
            serde_json::from_str::<serde_json::Value>(&raw).expect("store parses as JSON");
        foreign["pricing_catalog_id"] = serde_json::json!("bogus-catalog-identity-0000");
        fs::write(
            session_cache_path(),
            serde_json::to_string(&foreign).expect("serialize foreign store"),
        )
        .expect("write foreign store");

        // Load under the real catalog: every entry is dropped for
        // re-parse, AND the ledger gains the disclosure record.
        let mut reloaded = load_session_cache();
        let journal_metadata = fs::metadata(&journal).expect("journal metadata");
        assert!(
            cached_file_mod_time_if_fresh(&journal, &journal_metadata, &mut reloaded).is_none(),
            "entries re-parse at the new catalog"
        );
        let records = reloaded.schema_invalidations().to_vec();
        assert_eq!(records.len(), 1, "the pricing arm appends a ledger record");
        let record = &records[0];
        assert_eq!(
            record.get("reason").and_then(serde_json::Value::as_str),
            Some("pricing_catalog"),
            "the record names the pricing arm"
        );
        assert_eq!(
            record
                .get("observed_pricing_catalog_id")
                .and_then(serde_json::Value::as_str),
            Some("bogus-catalog-identity-0000"),
            "the record names the observed (foreign) catalog id"
        );
        assert_eq!(
            record
                .get("dropped_entries")
                .and_then(serde_json::Value::as_i64),
            Some(1),
            "the record counts the re-parsed entries"
        );
        assert_eq!(
            record
                .get("kept_dir_listings")
                .and_then(serde_json::Value::as_i64),
            Some(1),
            "walk-current listings survive the pricing flip"
        );

        // The record persists: saving re-stamps the real catalog id and
        // the next load must NOT double-append.
        save_session_cache(&mut reloaded).expect("save reloaded store");
        let mut again = load_session_cache();
        assert_eq!(
            again.schema_invalidations().len(),
            1,
            "no duplicate record on reload"
        );
        save_session_cache(&mut again).expect("save again store");
        let raw = fs::read_to_string(session_cache_path()).expect("read final store");
        let final_doc: serde_json::Value = serde_json::from_str(&raw).expect("final store parses");
        assert_ne!(
            final_doc
                .get("pricing_catalog_id")
                .and_then(serde_json::Value::as_str),
            Some("bogus-catalog-identity-0000"),
            "the real catalog id is re-stamped"
        );

        let _ = fs::remove_dir_all(&root);
        if let Some(prior) = prior_cache {
            std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", prior);
        } else {
            std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
        }
    }

    #[test]
    fn schema_flip_restores_dir_listings_and_records_the_invalidation() {
        // Session-cache store integrity (lead): a schema bump must
        // invalidate parsed entries — every bump in the schema history
        // changed parse or accounting semantics, so stale numbers must
        // never be served — but it must not silently torch the whole
        // store. Directory listings are walk semantics on their own
        // version lane, so when that lane still matches they survive
        // the bump (no full tree re-walk on upgrade), and the
        // invalidation itself is recorded in the journal instead of
        // happening invisibly.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-schema-flip-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join("session.jsonl");
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Populate a store at the CURRENT schema: one entry + one
        // live listing for a real directory.
        let mut cache = load_session_cache();
        let session = Session {
            name: "flip".to_string(),
            path: journal.to_string_lossy().to_string(),
            cwd: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_session(&journal, &session, &mut cache).expect("store session");
        store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
            .expect("store listing");
        save_session_cache(&mut cache).expect("save store");

        // Flip the persisted schema to simulate an upgrade between
        // invocations; every other field of the document stays as
        // written.
        let raw = fs::read_to_string(session_cache_path()).expect("read store");
        let mut flipped: serde_json::Value =
            serde_json::from_str(&raw).expect("store parses as JSON");
        flipped["schema_version"] = serde_json::json!(SESSION_CACHE_SCHEMA_VERSION - 1);
        fs::write(
            session_cache_path(),
            serde_json::to_string(&flipped).expect("serialize flipped store"),
        )
        .expect("write flipped store");

        let mut cache = load_session_cache();
        let journal_metadata = fs::metadata(&journal).expect("journal metadata");
        assert!(
            cached_file_mod_time_if_fresh(&journal, &journal_metadata, &mut cache).is_none(),
            "stale-schema entries are never served"
        );
        let listing = cached_dir_listing(&corpus, &mut cache)
            .expect("walk-version listings survive the schema bump");
        assert_eq!(listing.files, vec![journal.clone()]);
        assert!(
            cache.is_dirty(),
            "the invalidation must persist at the next save"
        );

        // The post-flip save lands at the new schema with the listing
        // preserved, the entries dropped, and the disclosure persisted.
        save_session_cache(&mut cache).expect("save migrated store");
        let raw = fs::read_to_string(session_cache_path()).expect("read migrated store");
        let doc: serde_json::Value =
            serde_json::from_str(&raw).expect("migrated store parses as JSON");
        assert_eq!(
            doc["schema_version"],
            serde_json::json!(SESSION_CACHE_SCHEMA_VERSION)
        );
        assert_eq!(
            doc["entries"].as_object().map(|entries| entries.len()),
            Some(0),
            "stale-schema entries stay dropped"
        );
        assert_eq!(
            doc["dirs"].as_object().map(|dirs| dirs.len()),
            Some(1),
            "the restored listing is part of the migrated store"
        );
        let records = doc["schema_invalidations"]
            .as_array()
            .expect("the invalidation is disclosed in the journal");
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0]["observed_schema_version"],
            serde_json::json!(SESSION_CACHE_SCHEMA_VERSION - 1)
        );
        assert_eq!(
            records[0]["schema_version"],
            serde_json::json!(SESSION_CACHE_SCHEMA_VERSION)
        );
        assert_eq!(records[0]["dropped_entries"], serde_json::json!(1));
        assert_eq!(records[0]["dropped_dirs"], serde_json::json!(0));
        assert_eq!(records[0]["kept_dir_listings"], serde_json::json!(1));

        // The migrated store still serves the listing on the next run.
        let mut cache = load_session_cache();
        assert!(
            cached_dir_listing(&corpus, &mut cache).is_some(),
            "the restored listing still serves after the save"
        );

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_paths_round_trip_through_the_cache_store() {
        // rm-041: cache keys and listing members used to collapse
        // non-UTF-8 paths to `U+FFFD`, so a `bad\xff\xfename.jsonl`
        // journal was invisible through the cached replay — the
        // listing named a file that does not exist, and two distinct
        // non-UTF-8 names could collide on one lossy key. Keys are now
        // lossless: the listing member must equal the real path.
        use std::os::unix::ffi::OsStringExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-lossless-keys-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join(std::ffi::OsString::from_vec(
            b"bad\xff\xfename.jsonl".to_vec(),
        ));
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        let mut cache = load_session_cache();
        store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
            .expect("store listing");
        save_session_cache(&mut cache).expect("save store");

        let mut cache = load_session_cache();
        let listing =
            cached_dir_listing(&corpus, &mut cache).expect("listing serves for a UTF-8 directory");
        assert_eq!(
            listing.files,
            vec![journal.clone()],
            "the non-UTF-8 file name must round-trip losslessly"
        );

        // The entry round trip must keep working through the encoded
        // key: store, persist, reload, serve.
        let session = Session {
            name: "raw name".to_string(),
            path: journal.to_string_lossy().to_string(),
            cwd: String::new(),
            // A non-empty source tool keeps the empty-source-tool
            // eviction gate (below) from treating this entry as
            // pre-gate legacy junk once it reloads into raw_entries.
            metrics: Metrics {
                source_tool: "agenttrace-test".to_string(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_session(&journal, &session, &mut cache).expect("store session");
        save_session_cache(&mut cache).expect("save store");
        let mut cache = load_session_cache();
        let served = cached_session(&journal, &mut cache)
            .expect("the cached entry serves through the lossless key");
        assert_eq!(served.name, "raw name");

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn legacy_lossy_listing_members_are_dropped_once_at_load() {
        // rm-041 upgrade seam: a pre-v4 store carries listing members
        // built with `to_string_lossy`, so a non-UTF-8 journal is stored
        // as a verbatim U+FFFD name — and the replay extends
        // `listing.files` verbatim, which would keep hiding the real
        // file through the warm cache forever. A lossy member cannot
        // be told apart from a genuine U+FFFD filename, so the walk
        // lane (not a filter) is what drops them: the listing below
        // has a CURRENT mtime and no size to drift, so only the
        // walk-version gate can invalidate it.
        use std::os::unix::ffi::OsStringExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-legacy-lossy-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join(std::ffi::OsString::from_vec(
            b"bad\xff\xfename.jsonl".to_vec(),
        ));
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Seed the store exactly as a pre-upgrade binary would have:
        // older schema, the last lossy-member walk lane, lossy member
        // name. The lane value is the ABSOLUTE legacy 3, not
        // `DIR_LISTING_WALK_VERSION - 1`: the defect this pins is a
        // binary whose current lane still equals the lossy lane — a
        // const-relative seed would self-adjust and never see it.
        let legacy_lossy_lane: i64 = 3;
        let corpus_metadata = fs::metadata(&corpus).expect("corpus metadata");
        let lossy_member = format!("{}/bad\u{FFFD}name.jsonl", corpus.to_string_lossy());
        let legacy = serde_json::json!({
            "schema_version": SESSION_CACHE_SCHEMA_VERSION - 1,
            "dir_listing_version": legacy_lossy_lane,
            "dirs": {
                corpus.to_string_lossy().to_string(): {
                    "mod_time": file_mod_time_nanos(&corpus_metadata),
                    "files": [lossy_member],
                    "dirs": [],
                },
            },
        });
        fs::write(
            session_cache_path(),
            serde_json::to_string(&legacy).expect("serialize legacy store"),
        )
        .expect("write legacy store");

        let mut cache = load_session_cache();
        assert!(
            cached_dir_listing(&corpus, &mut cache).is_none(),
            "a lossy-member listing from the previous walk lane must not serve"
        );
        // The drop is disclosed like every schema-side invalidation.
        let record = cache
            .schema_invalidations()
            .last()
            .expect("invalidation recorded");
        assert_eq!(record["kept_dir_listings"], serde_json::json!(0));
        assert_eq!(record["dropped_dirs"], serde_json::json!(1));

        // Self-heal forward: the re-walk stores the listing with the
        // lossless member, and the store serves the real path after a
        // save/reload.
        store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
            .expect("store fresh listing");
        save_session_cache(&mut cache).expect("save store");
        let mut cache = load_session_cache();
        let listing = cached_dir_listing(&corpus, &mut cache).expect("fresh listing serves");
        assert_eq!(listing.files, vec![journal.clone()]);

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn directory_listing_freshness_also_compares_the_directory_size() {
        // rm-041: listing freshness compared the directory mtime
        // alone, so a mutation that landed inside the same mtime tick
        // as the stored listing stayed invisible until the next mtime
        // change. The stored listing also carries the directory's byte
        // size; a drift in either invalidates.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-same-tick-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join("session.jsonl");
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        let rewrite_listing = |mod_time: i64, dir_size: serde_json::Value| {
            let mut cache = load_session_cache();
            store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
                .expect("store listing");
            save_session_cache(&mut cache).expect("save store");
            let raw = fs::read_to_string(session_cache_path()).expect("read store");
            let mut doc: serde_json::Value =
                serde_json::from_str(&raw).expect("store parses as JSON");
            let key = corpus.to_string_lossy().to_string();
            doc["dirs"][key.as_str()]["mod_time"] = serde_json::json!(mod_time);
            doc["dirs"][key.as_str()]["dir_size"] = dir_size;
            fs::write(
                session_cache_path(),
                serde_json::to_string(&doc).expect("serialize store"),
            )
            .expect("write store");
        };
        let current = fs::metadata(&corpus).expect("corpus metadata");
        let now_nanos = file_mod_time_nanos(&current);
        let real_size = current.len();

        // Same-tick drift: the mtime still matches, the size does not.
        rewrite_listing(now_nanos, serde_json::json!(real_size + 4096));
        let mut cache = load_session_cache();
        assert!(
            cached_dir_listing(&corpus, &mut cache).is_none(),
            "a same-tick size drift must invalidate the listing"
        );

        // No false invalidation: mtime and size both current.
        rewrite_listing(now_nanos, serde_json::json!(real_size));
        let mut cache = load_session_cache();
        assert!(
            cached_dir_listing(&corpus, &mut cache).is_some(),
            "a listing whose mtime and size both match still serves"
        );

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_saves_merge_instead_of_last_writer_wins() {
        // rm-298 residual: save was a lockless read-modify-write — two
        // agenttrace processes that load the store before either saves
        // each overwrite the other's entries wholesale. Saves now
        // serialize on an advisory lock and merge the freshly re-read
        // on-disk state, so every writer's entry survives.
        use std::sync::{Arc, Barrier};
        let root =
            std::env::temp_dir().join(format!("agenttrace-merge-save-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Seed the on-disk store with one entry.
        let seed_file = corpus.join("seed.jsonl");
        fs::write(&seed_file, b"{}").expect("write seed");
        let mut cache = load_session_cache();
        let session = Session {
            name: "seed".to_string(),
            path: seed_file.to_string_lossy().to_string(),
            cwd: String::new(),
            metrics: Metrics {
                source_tool: "agenttrace-test".to_string(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_session(&seed_file, &session, &mut cache).expect("store seed");
        save_session_cache(&mut cache).expect("save seed");

        // Every writer loads BEFORE any of them saves (the barrier),
        // exactly like processes that start together.
        let writers: Vec<_> = (0..6)
            .map(|i| {
                let file = corpus.join(format!("writer-{i}.jsonl"));
                fs::write(&file, b"{}").expect("write journal");
                (i, file)
            })
            .collect();
        let barrier = Arc::new(Barrier::new(writers.len()));
        let mut handles = Vec::new();
        for (i, file) in writers.iter() {
            let i = *i;
            let barrier = Arc::clone(&barrier);
            let file = file.clone();
            handles.push(std::thread::spawn(move || {
                let mut cache = load_session_cache();
                barrier.wait();
                let session = Session {
                    name: format!("writer-{i}"),
                    path: file.to_string_lossy().to_string(),
                    cwd: String::new(),
                    metrics: Metrics {
                        source_tool: "agenttrace-test".to_string(),
                        ..Metrics::default()
                    },
                    anomalies: Vec::new(),
                    health: 100,
                    tool_warnings: Vec::new(),
                    diagnostics: Diagnostics::default(),
                };
                store_session(&file, &session, &mut cache).expect("store session");
                save_session_cache(&mut cache).expect("save cache");
            }));
        }
        for handle in handles {
            handle.join().expect("writer thread");
        }

        let mut cache = load_session_cache();
        let served = cached_session(&seed_file, &mut cache).expect("seed survives");
        assert_eq!(served.name, "seed");
        for (i, file) in &writers {
            let served = cached_session(file, &mut cache).unwrap_or_else(|| {
                panic!("writer-{i}'s entry was lost to a concurrent save");
            });
            assert_eq!(served.name, format!("writer-{i}"));
        }

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }
}

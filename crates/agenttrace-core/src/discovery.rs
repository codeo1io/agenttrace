use crate::session_cache::{
    cached_dir_listing, cached_file_mod_time_if_fresh, cached_session, delete_cached_session,
    load_session_cache, save_session_cache, store_dir_listing, store_session, SessionCache,
};
use crate::{
    merge_preserved_history, parse_file, preserve_derived_history, skip_sqlite_backed_file_dir,
    Session,
};
use chrono::{DateTime, Utc};
use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownSessionDir {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    pub since: Option<DateTime<Utc>>,
    pub project: String,
    pub source: String,
    pub model: String,
    pub include_history: bool,
    pub preserve_history: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LoadReport {
    pub sessions: Vec<Session>,
    pub discovered: usize,
    pub parsed: usize,
    pub skipped: usize,
    pub cache_hits: usize,
}

#[derive(Debug, Clone, Default)]
pub struct LoadProgress {
    pub discovered: usize,
    pub processed: usize,
    pub parsed: usize,
    pub skipped: usize,
    pub cache_hits: usize,
    pub session: Option<Session>,
}

pub fn known_session_dirs() -> Vec<KnownSessionDir> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let env_home = |name: &str, fallback: PathBuf| {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or(fallback)
    };
    let mut dirs = vec![
        KnownSessionDir {
            name: "Hermes Agent".to_string(),
            path: home.join(".hermes").join("sessions"),
        },
        KnownSessionDir {
            name: "Codex CLI".to_string(),
            path: env_home("CODEX_HOME", home.join(".codex")).join("sessions"),
        },
        KnownSessionDir {
            name: "Codex CLI archived".to_string(),
            path: env_home("CODEX_HOME", home.join(".codex")).join("archived_sessions"),
        },
        KnownSessionDir {
            name: "Gemini CLI".to_string(),
            path: home.join(".gemini").join("tmp"),
        },
        KnownSessionDir {
            name: "Antigravity CLI".to_string(),
            path: home.join(".gemini").join("antigravity-cli").join("brain"),
        },
        KnownSessionDir {
            name: "Antigravity CLI conversations".to_string(),
            path: home
                .join(".gemini")
                .join("antigravity-cli")
                .join("conversations"),
        },
        KnownSessionDir {
            name: "Claude Code transcripts".to_string(),
            path: env_home("CLAUDE_CONFIG_DIR", home.join(".claude")).join("transcripts"),
        },
        KnownSessionDir {
            name: "Qwen Code".to_string(),
            path: home.join(".qwen").join("projects"),
        },
        KnownSessionDir {
            name: "Claude Code".to_string(),
            path: env_home("CLAUDE_CONFIG_DIR", home.join(".claude")).join("projects"),
        },
        KnownSessionDir {
            name: "Pi".to_string(),
            path: home.join(".pi").join("agent").join("sessions"),
        },
        KnownSessionDir {
            name: "Pi XDG".to_string(),
            path: home
                .join(".config")
                .join("pi")
                .join("agent")
                .join("sessions"),
        },
        KnownSessionDir {
            name: "Oh My Pi".to_string(),
            path: home.join(".omp").join("agent").join("sessions"),
        },
        KnownSessionDir {
            name: "WorkBuddy".to_string(),
            path: home.join(".workbuddy").join("projects"),
        },
        KnownSessionDir {
            name: "Cursor".to_string(),
            path: home.join(".cursor").join("projects"),
        },
        KnownSessionDir {
            name: "GitHub Copilot CLI".to_string(),
            path: home.join(".copilot").join("session-state"),
        },
        KnownSessionDir {
            name: "GitHub Copilot CLI OTEL".to_string(),
            path: home.join(".copilot").join("otel"),
        },
        KnownSessionDir {
            name: "Kimi CLI".to_string(),
            path: home.join(".kimi").join("sessions"),
        },
    ];
    dirs.extend(open_code_known_session_dirs(&home));
    dirs.extend(cline_known_session_dirs(&home));
    dirs
}

pub fn discover_session_dirs() -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut dirs = Vec::new();
    for candidate in known_session_dirs() {
        if !candidate.path.is_dir() {
            continue;
        }
        // Admit roots by canonical target, not path spelling (rm-055): a
        // dotfile-managed home (`~/.pi -> ~/.config/pi`) presents one
        // directory as two known roots; keying admission on the exact
        // string admitted both, and every session under them was
        // discovered, parsed, and costed once per spelling (live: 2
        // Sessions / $0.0012 where the truth was 1 / $0.0006). The first
        // known spelling wins, deterministically. Unresolvable paths
        // fall back to the spelling, mirroring `SymlinkTargets::admit`.
        let key = fs::canonicalize(&candidate.path).unwrap_or_else(|_| candidate.path.clone());
        if seen.insert(key) {
            dirs.push(candidate.path);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join(".aider.chat.history.md").is_file() {
            let key = fs::canonicalize(&cwd).unwrap_or_else(|_| cwd.clone());
            if seen.insert(key) {
                dirs.push(cwd);
            }
        }
    }
    dirs
}

pub fn find_session_files(dir: Option<&Path>) -> Vec<PathBuf> {
    if let Some(dir) = dir {
        if is_cline_task_dir(dir) {
            return vec![dir.to_path_buf()];
        }
        return collect_session_files(dir);
    }
    let mut all = Vec::new();
    for dir in discover_session_dirs() {
        all.extend(collect_session_files(&dir));
    }
    // Cross-root dedupe keys on canonical identity (rm-055): distinct
    // admitted roots can still reach one file through an interior
    // symlink, and two spellings of one file used to count twice.
    let mut seen = HashSet::new();
    all.retain(|path| seen.insert(canonical_file_key(path)));
    sort_paths_by_mod_time(all)
}

pub fn load_sessions_from_dir(dir: Option<&Path>) -> Vec<Session> {
    load_sessions_with_options(dir, &LoadOptions::default()).sessions
}

pub fn load_sessions_with_options(dir: Option<&Path>, options: &LoadOptions) -> LoadReport {
    load_sessions_with_progress(dir, options, |_| {})
}

pub fn load_sessions_with_progress(
    dir: Option<&Path>,
    options: &LoadOptions,
    on_progress: impl FnMut(LoadProgress),
) -> LoadReport {
    let mut cache = load_session_cache();
    load_sessions_with_progress_from_cache(dir, options, &mut cache, on_progress)
}

pub fn load_sessions_with_progress_from_cache(
    dir: Option<&Path>,
    options: &LoadOptions,
    cache: &mut SessionCache,
    on_progress: impl FnMut(LoadProgress),
) -> LoadReport {
    load_sessions_with_progress_from_cache_mode(dir, options, cache, true, on_progress)
}

pub fn load_sessions_with_progress_from_cache_mode(
    dir: Option<&Path>,
    options: &LoadOptions,
    cache: &mut SessionCache,
    emit_progress_sessions: bool,
    mut on_progress: impl FnMut(LoadProgress),
) -> LoadReport {
    let mut sessions = Vec::new();
    let files = find_session_files_cached(dir, cache, true);
    let discovered = files.len();
    let mut cache_hits = 0;
    let mut skipped = 0;
    for (index, path) in files.into_iter().enumerate() {
        let session = if let Some(session) = cached_session(&path, cache) {
            cache_hits += 1;
            Some(session)
        } else if let Ok(session) = parse_file(&path) {
            let _ = store_session(&path, &session, cache);
            Some(session)
        } else {
            skipped += 1;
            None
        };
        if let Some(session) = session {
            on_progress(LoadProgress {
                discovered,
                processed: index + 1,
                parsed: sessions.len() + 1,
                skipped,
                cache_hits,
                session: emit_progress_sessions.then(|| session.clone()),
            });
            sessions.push(session);
        } else {
            on_progress(LoadProgress {
                discovered,
                processed: index + 1,
                parsed: sessions.len(),
                skipped,
                cache_hits,
                session: None,
            });
        }
    }
    let live_parsed = sessions.len();
    if cache.is_dirty() {
        let _ = save_session_cache(cache);
    }
    if dir.is_none() {
        sessions.extend(crate::sqlite_sessions::load_sqlite_backed_sessions_since(
            options.since,
        ));
    }
    if options.preserve_history {
        let _ = preserve_derived_history(&sessions);
    }
    if options.include_history {
        merge_preserved_history(&mut sessions);
    }
    sessions.retain(|session| {
        // Sessions with an unknown start time stay visible (unknown-time
        // bucket, N7) instead of being silently dropped from ranged views;
        // data_health counts them via `unknown_time_sessions`.
        options.since.map_or(true, |since| {
            DateTime::parse_from_rfc3339(&session.metrics.session_start)
                .map(|time| time.with_timezone(&Utc) >= since)
                .unwrap_or(true)
        }) && matches_project_filter(session, &options.project)
            && matches_filter(&session.metrics.source_tool, &options.source)
            && matches_filter(&session.metrics.model_used, &options.model)
    });
    if dir.is_none() {
        sessions.sort_by(|a, b| {
            b.metrics
                .session_start
                .cmp(&a.metrics.session_start)
                .then_with(|| b.name.cmp(&a.name))
        });
    }
    LoadReport {
        parsed: sessions.len(),
        skipped: discovered.saturating_sub(live_parsed),
        sessions,
        discovered,
        cache_hits,
    }
}

fn matches_project_filter(session: &Session, filter: &str) -> bool {
    if filter.trim().is_empty() {
        return true;
    }
    let project = crate::resolve_project(session);
    matches_filter(&project.id, filter)
        || matches_filter(&project.display_name, filter)
        || matches_filter(&project.root, filter)
}

fn matches_filter(value: &str, filter: &str) -> bool {
    filter.trim().is_empty()
        || value
            .to_ascii_lowercase()
            .contains(&filter.trim().to_ascii_lowercase())
}

pub fn collect_session_files(dir: &Path) -> Vec<PathBuf> {
    if is_cline_task_dir(dir) {
        return vec![dir.to_path_buf()];
    }
    let max_depth = max_session_dir_depth(dir);
    let mut items = Vec::new();
    let mut visited = SymlinkTargets::new(dir);
    walk_session_files(dir, 0, max_depth, &mut items, &mut visited);
    items.sort_by_key(|item| Reverse(item.1));
    items.into_iter().map(|item| item.0).collect()
}

/// Default per-file cap for discovered session files: far above any
/// real session journal observed (tens of MiB) while still bounding
/// `fs::read` on pathological files. Raise or lower it with
/// `AGENTTRACE_MAX_SESSION_FILE_BYTES`.
pub const DEFAULT_MAX_SESSION_FILE_BYTES: u64 = 512 * 1024 * 1024;

/// Configurable ceiling for a single discovered session file
/// (rm-056). Unset, non-numeric, or zero values fall back to the
/// default.
pub fn max_session_file_bytes() -> u64 {
    std::env::var_os("AGENTTRACE_MAX_SESSION_FILE_BYTES")
        .and_then(|value| value.to_str().and_then(|raw| raw.parse::<u64>().ok()))
        .filter(|bytes| *bytes > 0)
        .unwrap_or(DEFAULT_MAX_SESSION_FILE_BYTES)
}

/// Why a file that matched a session-file name was refused at collect
/// time (rm-056). Named so doctor and load diagnostics can say WHY a
/// file is missing instead of the old behavior: a FIFO matching a
/// session name reached `parse_file`'s unguarded `fs::read` and hung
/// the whole run on open() (live: `--overview`, `--doctor`, and
/// `--doctor -d` all hit timeout exit 124); oversized files were read
/// whole into memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionFileSkipReason {
    /// FIFOs, sockets, devices — anything `fs::read` would open with
    /// hang semantics (a FIFO's open blocks until a writer appears).
    NotRegularFile,
    /// Over the configurable byte cap; reading it whole would balloon
    /// memory for one pathological file.
    Oversized { size: u64, max: u64 },
}

impl SessionFileSkipReason {
    /// Short human phrasing for doctor rows and load notes.
    pub fn describe(&self) -> String {
        match self {
            SessionFileSkipReason::NotRegularFile => "not a regular file".to_string(),
            SessionFileSkipReason::Oversized { size, max } => {
                format!("{size} bytes over the {max}-byte cap")
            }
        }
    }
}

/// One recorded collect-time refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFileSkip {
    pub path: PathBuf,
    pub reason: SessionFileSkipReason,
}

/// Bounded process-wide journal of collect-time refusals; doctor and
/// the CLI load path read it to disclose skips instead of dropping
/// files silently.
const MAX_RECORDED_SKIPS: usize = 64;

fn skip_journal() -> &'static Mutex<Vec<SessionFileSkip>> {
    static SKIPS: OnceLock<Mutex<Vec<SessionFileSkip>>> = OnceLock::new();
    SKIPS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Records a refusal (deduplicated, bounded).
pub fn record_session_file_skip(path: &Path, reason: SessionFileSkipReason) {
    let skip = SessionFileSkip {
        path: path.to_path_buf(),
        reason,
    };
    let mut journal = match skip_journal().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if journal.len() >= MAX_RECORDED_SKIPS || journal.contains(&skip) {
        return;
    }
    journal.push(skip);
}

/// Collect-time refusals recorded so far in this process, in
/// discovery order.
pub fn session_file_skips() -> Vec<SessionFileSkip> {
    match skip_journal().lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// Clears the skip journal (test hygiene between isolated loads).
pub fn clear_session_file_skips() {
    match skip_journal().lock() {
        Ok(mut guard) => guard.clear(),
        Err(poisoned) => poisoned.into_inner().clear(),
    }
}

/// Collect-time admission predicate for a candidate session file
/// (rm-056): a session file must be a regular file after symlink
/// resolution (a symlink to a real file is fine) and under the
/// configurable size cap. `None` admits; `Some(reason)` refuses.
fn session_file_admission(metadata: &fs::Metadata) -> Option<SessionFileSkipReason> {
    if !metadata.is_file() {
        return Some(SessionFileSkipReason::NotRegularFile);
    }
    let max = max_session_file_bytes();
    if metadata.len() > max {
        return Some(SessionFileSkipReason::Oversized {
            size: metadata.len(),
            max,
        });
    }
    None
}

pub(crate) fn find_session_files_cached(
    dir: Option<&Path>,
    cache: &mut SessionCache,
    skip_sqlite_backed: bool,
) -> Vec<PathBuf> {
    if let Some(dir) = dir {
        if is_cline_task_dir(dir) {
            return vec![dir.to_path_buf()];
        }
        return collect_session_files_cached(dir, cache);
    }
    let mut seen = HashSet::new();
    let mut all = Vec::new();
    for dir in discover_session_dirs() {
        if skip_sqlite_backed && skip_sqlite_backed_file_dir(&dir) {
            continue;
        }
        for path in collect_session_files_cached(&dir, cache) {
            // Cross-root dedupe by canonical identity, not spelling
            // (rm-055): the aliased file is parsed and costed once.
            if seen.insert(canonical_file_key(&path)) {
                all.push(path);
            }
        }
    }
    sort_paths_by_cache(all, cache)
}

/// Canonical identity for a discovered session file (rm-055): two
/// spellings of one file (alias roots, interior symlinks) collapse to
/// one key; an unresolvable path falls back to its spelling.
fn canonical_file_key(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn collect_session_files_cached(dir: &Path, cache: &mut SessionCache) -> Vec<PathBuf> {
    if is_cline_task_dir(dir) {
        return vec![dir.to_path_buf()];
    }
    let max_depth = max_session_dir_depth(dir);
    let mut items = Vec::new();
    let mut visited = SymlinkTargets::new(dir);
    walk_session_files_cached(dir, 0, max_depth, cache, &mut items, &mut visited);
    sort_paths_by_cache(items, cache)
}

/// Loop guard for symlinked session directories (Codex `#42135`):
/// the walk records the canonical target of every directory it
/// descends and refuses to revisit one, so a cycle of symlinks cannot
/// make discovery spin (depth bounds the walk; this bounds revisits).
struct SymlinkTargets(HashSet<PathBuf>);

impl SymlinkTargets {
    fn new(root: &Path) -> Self {
        let mut seen = HashSet::new();
        if let Ok(canonical) = fs::canonicalize(root) {
            seen.insert(canonical);
        }
        SymlinkTargets(seen)
    }

    /// Returns true when the directory's canonical target has not been
    /// visited yet, recording it.
    fn admit(&mut self, dir: &Path) -> bool {
        match fs::canonicalize(dir) {
            Ok(canonical) => self.0.insert(canonical),
            // Unresolvable path: admit it; the walk's depth bound keeps
            // a degenerate case finite.
            Err(_) => true,
        }
    }
}

/// `entry.file_type()` reports the link itself, not its target, so a
/// symlinked session directory (officially supported by Codex 0.153+,
/// `openai/codex` `#42135`, and common in dotfile-managed homes) used to
/// be silently skipped. Resolve the link once: a symlink to a directory
/// is descended, anything else falls through to the file rules.
fn entry_is_dir_entry(file_type: &fs::FileType, path: &Path) -> bool {
    if file_type.is_dir() {
        return true;
    }
    if file_type.is_symlink() {
        return fs::metadata(path)
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false);
    }
    false
}

fn walk_session_files_cached(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    cache: &mut SessionCache,
    items: &mut Vec<PathBuf>,
    visited: &mut SymlinkTargets,
) {
    if depth > max_depth {
        return;
    }
    if is_cline_task_dir(dir) {
        items.push(dir.to_path_buf());
        return;
    }
    let Ok(metadata) = fs::metadata(dir) else {
        return;
    };
    if !metadata.is_dir() {
        return;
    }
    if let Some(listing) = cached_dir_listing(dir, cache) {
        items.extend(listing.files);
        for child in listing.dirs {
            if visited.admit(&child) {
                walk_session_files_cached(&child, depth + 1, max_depth, cache, items, visited);
            }
        }
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if entry_is_dir_entry(&file_type, &path) {
            if is_open_code_storage_skipped_dir(&path) {
                continue;
            }
            if is_skipped_session_dir(&path) {
                continue;
            }
            if is_cline_task_dir(&path) {
                files.push(path);
                continue;
            }
            if !visited.admit(&path) {
                continue;
            }
            dirs.push(path);
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !is_session_file_name(&name) {
            continue;
        }
        if !is_special_session_file(&path) {
            continue;
        }
        if is_gemini_temp_path(&path) && !is_gemini_temp_session_file(&path) {
            continue;
        }
        if is_open_code_storage_path(&path) && !is_open_code_storage_session_file(&path) {
            continue;
        }
        // rm-056: stat once and admit or refuse BEFORE the listing cache
        // learns the file, so a FIFO or oversized file matching a session
        // name never reaches `parse_file`'s `fs::read` (which hung the
        // whole run on a FIFO's open) and never enters the cache either.
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if let Some(reason) = session_file_admission(&metadata) {
            record_session_file_skip(&path, reason);
            continue;
        }
        files.push(path);
    }
    files.sort();
    dirs.sort();
    let _ = store_dir_listing(dir, &files, &dirs, cache);

    items.extend(files);
    for child in dirs {
        walk_session_files_cached(&child, depth + 1, max_depth, cache, items, visited);
    }
}

fn walk_session_files(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    items: &mut Vec<(PathBuf, SystemTime)>,
    visited: &mut SymlinkTargets,
) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if entry_is_dir_entry(&file_type, &path) {
            if is_open_code_storage_skipped_dir(&path) {
                continue;
            }
            if is_skipped_session_dir(&path) {
                continue;
            }
            if is_cline_task_dir(&path) {
                items.push((path, entry_mod_time(&entry)));
                continue;
            }
            if !visited.admit(&path) {
                continue;
            }
            walk_session_files(&path, depth + 1, max_depth, items, visited);
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !is_session_file_name(&name) {
            continue;
        }
        if !is_special_session_file(&path) {
            continue;
        }
        if is_gemini_temp_path(&path) && !is_gemini_temp_session_file(&path) {
            continue;
        }
        if is_open_code_storage_path(&path) && !is_open_code_storage_session_file(&path) {
            continue;
        }
        // rm-056: same admission predicate on the uncached walk; the
        // metadata needed for the sort key doubles as the stat.
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if let Some(reason) = session_file_admission(&metadata) {
            record_session_file_skip(&path, reason);
            continue;
        }
        items.push((
            path,
            metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        ));
    }
}

fn sort_paths_by_mod_time(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut items: Vec<_> = paths
        .into_iter()
        .map(|path| {
            let time = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (path, time)
        })
        .collect();
    items.sort_by_key(|item| Reverse(item.1));
    items.into_iter().map(|item| item.0).collect()
}

fn sort_paths_by_cache(paths: Vec<PathBuf>, cache: &mut SessionCache) -> Vec<PathBuf> {
    let mut items = Vec::new();
    for path in paths {
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                delete_cached_session(&path, cache);
                continue;
            }
        };
        // rm-056: re-apply the admission predicate on the metadata this
        // sort already fetches, so listings replayed from a cache written
        // with a larger cap (or by a pre-rm-056 binary) cannot feed a
        // FIFO or oversized file back into the parse loop.
        if let Some(reason) = session_file_admission(&metadata) {
            record_session_file_skip(&path, reason);
            delete_cached_session(&path, cache);
            continue;
        }
        let time = cached_file_mod_time_if_fresh(&path, &metadata, cache)
            .map(time_from_unix_nanos)
            .unwrap_or_else(|| metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH));
        items.push((path, time));
    }
    items.sort_by_key(|item| Reverse(item.1));
    items.into_iter().map(|item| item.0).collect()
}

fn time_from_unix_nanos(nanos: i64) -> SystemTime {
    if nanos <= 0 {
        return SystemTime::UNIX_EPOCH;
    }
    SystemTime::UNIX_EPOCH + std::time::Duration::from_nanos(nanos as u64)
}

pub fn is_session_file_name(name: &str) -> bool {
    if name == ".aider.chat.history.md" {
        return true;
    }
    if name.ends_with(".meta.json") {
        return false;
    }
    if name.starts_with("request_dump_") || name == "sessions.json" {
        return false;
    }
    name.ends_with(".jsonl") || name.ends_with(".json")
}

fn max_session_dir_depth(dir: &Path) -> usize {
    let slash = dir.to_string_lossy().replace('\\', "/");
    if dir.file_name().and_then(|name| name.to_str()) == Some("projects")
        && slash.contains("/.workbuddy/")
    {
        return 2;
    }
    if dir.file_name().and_then(|name| name.to_str()) == Some("projects")
        && slash.contains("/.claude/")
    {
        return 3;
    }
    if dir.file_name().and_then(|name| name.to_str()) == Some("tmp") && slash.contains("/.gemini/")
    {
        return 4;
    }
    if is_open_code_storage_root(dir) {
        return 2;
    }
    if is_open_code_storage_session_root(dir) {
        return 1;
    }
    4
}

fn is_cline_task_dir(path: &Path) -> bool {
    path.join("api_conversation_history.json").is_file()
        || path.join("ui_messages.json").is_file()
        || path.join("task_metadata.json").is_file()
}

fn is_skipped_session_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            matches!(
                name,
                "node_modules" | ".git" | "target" | "dist" | "build" | ".codegraph"
            )
        })
        .unwrap_or(false)
}

fn is_gemini_temp_path(path: &Path) -> bool {
    path.to_string_lossy()
        .replace('\\', "/")
        .contains("/.gemini/tmp/")
}

fn is_special_session_file(path: &Path) -> bool {
    let slash = path.to_string_lossy().replace('\\', "/");
    if slash.contains("/.claude/projects/") && slash.contains("/workflows/") {
        return false;
    }
    if slash.contains("/.gemini/antigravity-cli/brain/") {
        return slash.ends_with("/.system_generated/logs/transcript.jsonl");
    }
    if slash.contains("/.cursor/projects/") {
        return slash.contains("/agent-transcripts/") && slash.ends_with(".jsonl");
    }
    true
}

fn is_gemini_temp_session_file(path: &Path) -> bool {
    is_gemini_temp_path(path)
        && matches!(
            path.parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str()),
            Some("chats" | "checkpoints")
        )
}

fn is_open_code_storage_root(path: &Path) -> bool {
    path.to_string_lossy()
        .replace('\\', "/")
        .ends_with("/opencode/storage")
}

fn is_open_code_storage_session_root(path: &Path) -> bool {
    open_code_storage_rel(path).as_deref() == Some("session")
}

fn is_open_code_storage_path(path: &Path) -> bool {
    open_code_storage_rel(path).is_some()
}

fn is_open_code_storage_skipped_dir(path: &Path) -> bool {
    let Some(rel) = open_code_storage_rel(path) else {
        return false;
    };
    if rel.is_empty() {
        return false;
    }
    let parts = rel.split('/').collect::<Vec<_>>();
    if parts.first().copied() != Some("session") {
        return true;
    }
    parts.len() > 2
}

fn is_open_code_storage_session_file(path: &Path) -> bool {
    let Some(rel) = open_code_storage_rel(path) else {
        return false;
    };
    let parts = rel.split('/').collect::<Vec<_>>();
    parts.len() == 3 && parts[0] == "session" && parts[2].ends_with(".json")
}

fn open_code_storage_rel(path: &Path) -> Option<String> {
    let slash = path.to_string_lossy().replace('\\', "/");
    let marker = "/opencode/storage";
    let index = slash.find(marker)?;
    let rest = &slash[index + marker.len()..];
    Some(rest.trim_start_matches('/').to_string())
}

fn open_code_known_session_dirs(home: &Path) -> Vec<KnownSessionDir> {
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |name: &str, path: PathBuf| {
        if seen.insert(path.clone()) {
            dirs.push(KnownSessionDir {
                name: name.to_string(),
                path,
            });
        }
    };
    if let Some(data_dir) = std::env::var_os("OPENCODE_DATA_DIR").map(PathBuf::from) {
        if !data_dir.as_os_str().is_empty() {
            add("OpenCode", data_dir);
        }
    }
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        if !data_home.as_os_str().is_empty() {
            add("OpenCode", data_home.join("opencode").join("storage"));
        }
    } else {
        add(
            "OpenCode",
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("storage"),
        );
    }
    add(
        "OpenCode macOS",
        home.join("Library")
            .join("Application Support")
            .join("opencode")
            .join("storage"),
    );
    dirs
}

fn cline_known_session_dirs(home: &Path) -> Vec<KnownSessionDir> {
    vec![KnownSessionDir {
        name: "Cline".to_string(),
        path: user_config_dir(home)
            .join("Code")
            .join("User")
            .join("globalStorage")
            .join("saoudrizwan.claude-dev")
            .join("tasks"),
    }]
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

fn entry_mod_time(entry: &fs::DirEntry) -> SystemTime {
    entry
        .metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

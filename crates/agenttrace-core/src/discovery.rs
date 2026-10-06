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
        // Pi-family homes moved to structural enumeration below
        // (rm-084): forks relocate the whole root (~/.omp, ~/.senpi,
        // ~/.omo) and pi relocates the agent dir inside a root
        // (~/.pi/<name>/sessions), so any child of a pi-family root
        // that carries a sessions/ directory registers instead of a
        // per-home allowlist. An agent dir relocated outside these
        // roots stays out of discovery; `-d <dir>` is the documented
        // escape for that case.
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
    dirs.extend(pi_family_known_session_dirs(&home));
    dirs
}

pub fn discover_session_dirs() -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut dirs = Vec::new();
    for candidate in known_session_dirs() {
        if candidate.path.is_dir() && seen.insert(candidate.path.clone()) {
            dirs.push(candidate.path);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join(".aider.chat.history.md").is_file() && seen.insert(cwd.clone()) {
            dirs.push(cwd);
        }
    }
    dirs
}

/// Pi-family session homes (rm-084). pi stores transcripts under
/// `<agent-dir>/sessions/<project-slug>/*.jsonl`; the agent dir
/// defaults to `~/.pi/agent`, forks relocate the whole root (`~/.omp`,
/// `~/.senpi`, `~/.omo` — the omo home is itself an agent dir on real
/// hosts, with project dirs directly under `~/.omo/sessions`), and
/// isolated agent state / profiles relocate the agent dir to a sibling
/// (`~/.pi/agent-cliproxy-only/sessions`). Enumeration is structural:
/// the default agent dir of every known root registers, plus any child
/// directory of a root that carries a `sessions/` directory, so a new
/// fork or agent-dir variant appears without per-home code. Children
/// without a `sessions/` directory (`~/.pi/cache`, `~/.omo/memory`,
/// ...) are never registered, keeping non-session fork state out of
/// discovery. An agent dir relocated outside these roots is not
/// discoverable from the filesystem layout; `-d <dir>` is the
/// documented escape for that case.
fn pi_family_known_session_dirs(home: &Path) -> Vec<KnownSessionDir> {
    let roots = [
        (home.join(".pi"), "Pi"),
        (home.join(".config").join("pi"), "Pi XDG"),
        (home.join(".omp"), "Oh My Pi"),
        (home.join(".senpi"), "Senpi"),
        (home.join(".omo"), "Omo"),
    ];
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();
    for (root, brand) in roots {
        let mut candidates: Vec<(String, PathBuf)> = Vec::new();
        candidates.push((brand.to_string(), root.join("agent").join("sessions")));
        // A fork home whose agent dir is the root itself.
        candidates.push((brand.to_string(), root.join("sessions")));
        if let Ok(children) = fs::read_dir(&root) {
            let mut children: Vec<PathBuf> = children.flatten().map(|entry| entry.path()).collect();
            children.sort();
            for child in children {
                let Some(name) = child.file_name().and_then(std::ffi::OsStr::to_str) else {
                    continue;
                };
                let label = if name == "agent" {
                    brand.to_string()
                } else {
                    format!("{brand} ({name})")
                };
                candidates.push((label, child.join("sessions")));
            }
        }
        for (name, path) in candidates {
            if !seen.contains(&path) && path.is_dir() {
                seen.insert(path.clone());
                dirs.push(KnownSessionDir { name, path });
            }
        }
    }
    dirs
}

/// Canonical identity of a session file, used for cross-root dedup.
///
/// rm-338: registered roots can alias or nest one canonical directory
/// (e.g. `~/.omp/agent/sessions` symlinked at `~/.pi/agent/sessions`, or
/// a second root pointed inside the first root's tree). The seen-sets in
/// `find_session_files` and `find_session_files_cached` therefore key on
/// canonical identity whenever more than one root is registered, so each
/// session file is admitted exactly once. Aliasing is DEDUPED, not
/// rejected (the --doctor-side alias disclosure is owned by the rm-249
/// fold at integration); when canonicalization fails (broken symlink,
/// permissions) the listed path is used, preserving the pre-rm-338
/// literal-path behavior for that entry.
fn canonical_identity(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn find_session_files(dir: Option<&Path>) -> Vec<PathBuf> {
    if let Some(dir) = dir {
        if is_cline_task_dir(dir) {
            return vec![dir.to_path_buf()];
        }
        return collect_session_files(dir);
    }
    let dirs = discover_session_dirs();
    let cross_root = dirs.len() > 1;
    let mut identities: HashSet<PathBuf> = HashSet::new();
    let mut all = Vec::new();
    for dir in dirs {
        for path in collect_session_files(&dir) {
            // rm-338: dedup across roots by canonical identity so roots
            // aliasing or nesting one canonical directory cannot
            // double-count a session file; listed paths are kept as-is.
            let identity = if cross_root {
                canonical_identity(&path)
            } else {
                path.clone()
            };
            if identities.insert(identity) {
                all.push(path);
            }
        }
    }
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
    // Slots hold (parsed session, came from cache); progress is still emitted in file order.
    let mut slots: Vec<Option<(Option<Session>, bool)>> = Vec::with_capacity(files.len());
    let mut misses = Vec::new();
    for (index, path) in files.iter().enumerate() {
        match cached_session(path, cache) {
            Some(session) => slots.push(Some((Some(session), true))),
            None => {
                slots.push(None);
                misses.push(index);
            }
        }
    }
    // Largest files first so one huge rollout does not become the tail of the parse.
    misses.sort_by_cached_key(|index| {
        std::cmp::Reverse(fs::metadata(&files[*index]).map_or(0, |meta| meta.len()))
    });
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(16)
        .min(misses.len())
        .max(1);
    let next_miss = std::sync::atomic::AtomicUsize::new(0);
    let (tx, rx) = std::sync::mpsc::channel::<(usize, Option<Session>)>();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let tx = tx.clone();
            let (files, misses, next_miss) = (&files, &misses, &next_miss);
            scope.spawn(move || {
                while let Some(&index) =
                    misses.get(next_miss.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                {
                    if tx.send((index, parse_file(&files[index]).ok())).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);

        let mut next = 0;
        let mut drain = |slots: &mut Vec<Option<(Option<Session>, bool)>>,
                         cache: &mut SessionCache| {
            while let Some((result, from_cache)) = slots.get_mut(next).and_then(Option::take) {
                match (&result, from_cache) {
                    (Some(_), true) => cache_hits += 1,
                    (Some(session), false) => {
                        let _ = store_session(&files[next], session, cache);
                    }
                    (None, _) => skipped += 1,
                }
                report_load_progress(
                    result,
                    next,
                    discovered,
                    skipped,
                    cache_hits,
                    emit_progress_sessions,
                    &mut sessions,
                    &mut on_progress,
                );
                next += 1;
            }
        };
        drain(&mut slots, cache);
        for (index, result) in rx {
            slots[index] = Some((result, false));
            drain(&mut slots, cache);
        }
    });
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
        options.since.is_none_or(|since| {
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
    let mut file_targets = SessionFileTargets::new();
    walk_session_files(
        dir,
        0,
        max_depth,
        &mut items,
        &mut visited,
        &mut file_targets,
    );
    items.sort_by_key(|item| Reverse(item.1));
    items.into_iter().map(|item| item.0).collect()
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
    let dirs = discover_session_dirs();
    let cross_root = dirs.len() > 1;
    let mut seen = HashSet::new();
    let mut all = Vec::new();
    for dir in dirs {
        if skip_sqlite_backed && skip_sqlite_backed_file_dir(&dir) {
            continue;
        }
        for path in collect_session_files_cached(&dir, cache) {
            // rm-338: dedup across roots by canonical identity (see
            // canonical_identity); listed paths are kept as-is.
            let identity = if cross_root {
                canonical_identity(&path)
            } else {
                path.clone()
            };
            if seen.insert(identity) {
                all.push(path);
            }
        }
    }
    sort_paths_by_cache(all, cache)
}

fn collect_session_files_cached(dir: &Path, cache: &mut SessionCache) -> Vec<PathBuf> {
    if is_cline_task_dir(dir) {
        return vec![dir.to_path_buf()];
    }
    let max_depth = max_session_dir_depth(dir);
    let mut items = Vec::new();
    let mut visited = SymlinkTargets::new(dir);
    let mut file_targets = SessionFileTargets::new();
    walk_session_files_cached(
        dir,
        0,
        max_depth,
        cache,
        &mut items,
        &mut visited,
        &mut file_targets,
    );
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

/// Same-file admission for the walk's file arm (rm-597): one transcript
/// reachable through two paths — a symlink alias (`link.jsonl` pointing
/// at `real.jsonl`) or a hardlink alias (two names, one inode) — used to
/// be collected and parsed twice, double-counting every aggregate it
/// feeds (assess f376d372 F1: one $0.0011 transcript reported "Total
/// Sessions: 2" / $0.0022 across overview/sessions/search/doctor).
/// Identity is walk-scoped and enforced where per-directory file lists
/// merge into the walk output, because whether two names alias one file
/// can depend on directories the whole walk visits. Stored directory
/// listings therefore stay raw and warm cache replays dedup identically
/// to cold walks — no listing-version bump.
///
/// On Unix the resolved file's `(device, inode)` pair covers both alias
/// kinds. Off Unix, std exposes no portable file index, so the canonical
/// path covers symlink aliases and hardlink aliasing is documented as
/// uncovered there.
#[derive(Eq, PartialEq, Hash)]
enum SameFileIdentity {
    #[cfg(unix)]
    DevIno(u64, u64),
    #[cfg(not(unix))]
    Canonical(PathBuf),
}

struct SessionFileTargets(HashSet<SameFileIdentity>);

impl SessionFileTargets {
    fn new() -> Self {
        SessionFileTargets(HashSet::new())
    }

    /// Returns true when the file's identity has not been collected
    /// yet, recording it. Unresolvable or non-regular paths are
    /// admitted; downstream admission checks (parse, cache
    /// fingerprint) still gate them.
    fn admit(&mut self, path: &Path) -> bool {
        match same_file_identity(path) {
            Some(identity) => self.0.insert(identity),
            None => true,
        }
    }
}

fn same_file_identity(path: &Path) -> Option<SameFileIdentity> {
    // Follows symlinks: a symlink alias must resolve to its target's
    // identity, not the link's own.
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some(SameFileIdentity::DevIno(metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        Some(SameFileIdentity::Canonical(fs::canonicalize(path).ok()?))
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
    file_targets: &mut SessionFileTargets,
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
        // rm-597: replayed listings are raw (pre-dedup), so the same
        // same-file admission applies on warm replay as on a cold walk.
        items.extend(
            listing
                .files
                .into_iter()
                .filter(|path| file_targets.admit(path)),
        );
        for child in listing.dirs {
            if visited.admit(&child) {
                walk_session_files_cached(
                    &child,
                    depth + 1,
                    max_depth,
                    cache,
                    items,
                    visited,
                    file_targets,
                );
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
        // rm-212: only regular files (symlinks included) are session
        // candidates — a fifo or device named *.jsonl inside a session root
        // would wedge reads and was previously admitted.
        if !path.is_file() {
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
        files.push(path);
    }
    files.sort();
    dirs.sort();
    // rm-597: the stored listing is deliberately raw (undecidable at
    // per-directory scope); dedup happens as the list merges into the
    // walk output, keeping cached and uncached walks identical.
    let _ = store_dir_listing(dir, &files, &dirs, cache);

    items.extend(files.into_iter().filter(|path| file_targets.admit(path)));
    for child in dirs {
        walk_session_files_cached(
            &child,
            depth + 1,
            max_depth,
            cache,
            items,
            visited,
            file_targets,
        );
    }
}

fn walk_session_files(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    items: &mut Vec<(PathBuf, SystemTime)>,
    visited: &mut SymlinkTargets,
    file_targets: &mut SessionFileTargets,
) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    // rm-597 review fix: collect candidates first and admit them in
    // sorted order, mirroring the cached walk (files.sort() before the
    // merge filter) so the alias survivor is the same path on a cold
    // uncached walk and a cached replay instead of whichever alias
    // readdir happened to yield first.
    let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();
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
            walk_session_files(&path, depth + 1, max_depth, items, visited, file_targets);
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !is_session_file_name(&name) {
            continue;
        }
        // rm-212: only regular files (symlinks included) are session
        // candidates — a fifo or device named *.jsonl inside a session root
        // would wedge reads and was previously admitted.
        if !path.is_file() {
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
        // rm-597: same-file admission — a symlink or hardlink alias of an
        // already-collected transcript is not a second session; admitted
        // in sorted order below (see the comment above the loop).
        files.push((path, entry_mod_time(&entry)));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    items.extend(
        files
            .into_iter()
            .filter(|(path, _)| file_targets.admit(path)),
    );
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
    // rm-370 (cycle 4): npm/package manifests are not sessions. Observed as
    // doctor failure noise on custom `-d` walks over agent tool homes
    // (`agent/npm/package-lock.json`, `agent-cliproxy-only/models-store.json`).
    if name == "package-lock.json" || name == "models-store.json" {
        return false;
    }
    if name.ends_with(".lock") {
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
                "node_modules" | ".git" | "target" | "dist" | "build" | ".codegraph" | "npm"
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

#[allow(clippy::too_many_arguments)]
fn report_load_progress(
    session: Option<Session>,
    index: usize,
    discovered: usize,
    skipped: usize,
    cache_hits: usize,
    emit_progress_sessions: bool,
    sessions: &mut Vec<Session>,
    on_progress: &mut impl FnMut(LoadProgress),
) {
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

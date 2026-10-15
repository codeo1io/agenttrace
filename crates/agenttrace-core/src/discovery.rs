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
    /// rm-753: SQLite-backed ingestion failures (unreadable agent
    /// databases, dropped session rows) so the CLI can disclose
    /// partial data instead of rendering it as the whole corpus.
    pub sqlite: crate::sqlite_sessions::SqliteIngestReport,
    /// rm-548 (JSON-lane scope after the rm-791 supersession, run
    /// 2023f222): opencode fork copies excluded from aggregation —
    /// storage session docs with `parentID` set. The sqlite lane's
    /// `parent_id` rows are subagent children under rm-791 (retained
    /// and attributed; that lane's `fork_excluded` is 0), so only the
    /// doc lane feeds this count. Kept here so the journal disclosure
    /// channel can report the exclusion instead of dropping sessions
    /// silently.
    pub opencode_fork_excluded: usize,
    /// rm-835 (minted campaign-locally as rm-695 by run f7f81aea, rebound
    /// at integration — the landed wall already holds an rm-695): how many
    /// discovered files failed to parse (the parse share of `skipped`,
    /// counted explicitly so callers can tell an exhausted parse lane from
    /// a filter miss).
    pub parse_failures: usize,
    /// rm-835: the first parse failure's message, in discovery order —
    /// path plus the parser's own hint (e.g. the zstd-compressed
    /// rollout hint), so an all-corrupt directory can say what actually
    /// happened instead of "no sessions match the filters".
    pub first_parse_failure: Option<String>,
    /// rm-912 (assess F2): every discovered file the loader ATTEMPTED
    /// but failed to parse, in discovery order — the CLI's -o
    /// membership check must cover these too, or
    /// `--overview -d dir -o dir/bad.jsonl` silently overwrites the
    /// unparseable input (a `.orig` twin is not a recovery path).
    pub parse_failure_paths: Vec<PathBuf>,
    /// rm-1417: sessions folded into a same-conversation representative
    /// by the cross-file conversation merge (gemini chats/checkpoints
    /// mirror families). Disclosed per-session via the
    /// `cross_file_sessions_merged` counter on the surviving session.
    pub cross_file_merged: usize,
    /// rm-799: subagent transcripts whose parent session is NOT part
    /// of the loaded corpus (orphaned children — moved transcripts,
    /// partial copies). They still render as standalone rows, but the
    /// count is disclosed so "SUBAGENTS 0" never reads as "no
    /// subagent work happened".
    pub unlinked_subagents: usize,
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
        // rm-947: the ecosystem-default antigravity root family (ccusage
        // paths.rs DEFAULT_ANTIGRAVITY_ROOTS). Every sibling carries the
        // same documented `conversations/` sidecar store next to the
        // already-scanned antigravity-cli root; before this family lane
        // they were silently missed by default discovery (ccusage #1851
        // reports the same class for antigravity-acp). The undocumented
        // SQLite store itself stays out of scope: the trajectory sidecar
        // is the documented reader surface.
        KnownSessionDir {
            name: "Antigravity conversations".to_string(),
            path: home
                .join(".gemini")
                .join("antigravity")
                .join("conversations"),
        },
        KnownSessionDir {
            name: "Antigravity IDE conversations".to_string(),
            path: home
                .join(".gemini")
                .join("antigravity-ide")
                .join("conversations"),
        },
        KnownSessionDir {
            name: "Antigravity backup conversations".to_string(),
            path: home
                .join(".gemini")
                .join("antigravity-backup")
                .join("conversations"),
        },
        KnownSessionDir {
            name: "Antigravity config conversations".to_string(),
            path: env_home("XDG_CONFIG_HOME", home.join(".config"))
                .join("antigravity")
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
    let (files, json_fork_excluded) = find_session_files_cached(dir, cache, true);
    let discovered = files.len();
    let mut cache_hits = 0;
    let mut skipped = 0;
    // Slots hold (parsed session, came from cache); progress is still emitted in file order.
    // rm-835: third slot carries the parse-failure message (cache hits
    // parse fine by construction, so theirs is None).
    /// Parsed session (None on failure), rm-835 parse-failure message,
    /// cache-hit flag.
    type Slot = Option<(Option<Session>, Option<String>, bool)>;
    let mut slots: Vec<Slot> = Vec::with_capacity(files.len());
    let mut misses = Vec::new();
    for (index, path) in files.iter().enumerate() {
        match cached_session(path, cache) {
            Some(session) => slots.push(Some((Some(session), None, true))),
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
    // rm-835: the parse error used to die at `.ok()` — an all-corrupt
    // directory then bailed to the CLI as "no sessions match the
    // requested filters", hiding the real story (and the zstd hint the
    // explicit-path lane already surfaces). Failures now ride the
    // LoadReport so the CLI can say what actually happened.
    let mut parse_failures = 0usize;
    let mut first_parse_failure: Option<String> = None;
    // rm-912: attempted-but-failed paths in discovery order, for the
    // CLI's -o membership check (see LoadReport::parse_failure_paths).
    let mut parse_failure_paths: Vec<PathBuf> = Vec::new();
    let (tx, rx) = std::sync::mpsc::channel::<(usize, Option<Session>, Option<String>)>();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let tx = tx.clone();
            let (files, misses, next_miss) = (&files, &misses, &next_miss);
            scope.spawn(move || {
                while let Some(&index) =
                    misses.get(next_miss.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                {
                    let parsed = parse_file(&files[index]);
                    let failure = parsed.as_ref().err().map(|error| format!("{error:#}"));
                    if tx.send((index, parsed.ok(), failure)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);

        let mut next = 0;
        let mut drain = |slots: &mut Vec<Slot>, cache: &mut SessionCache| {
            while let Some((result, failure, from_cache)) =
                slots.get_mut(next).and_then(Option::take)
            {
                match (&result, from_cache) {
                    (Some(_), true) => cache_hits += 1,
                    (Some(session), false) => {
                        let _ = store_session(&files[next], session, cache);
                    }
                    (None, _) => {
                        skipped += 1;
                        // rm-835: keep the first failure's parser message
                        // (discovery order) for the LoadReport.
                        if first_parse_failure.is_none() {
                            first_parse_failure = failure;
                        }
                        // rm-912: the loader ATTEMPTED this path — it
                        // joins the -o refusal set even though no
                        // session was produced from it.
                        parse_failure_paths.push(files[next].clone());
                        parse_failures += 1;
                    }
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
        for (index, result, failure) in rx {
            slots[index] = Some((result, failure, false));
            drain(&mut slots, cache);
        }
    });
    let live_parsed = sessions.len();
    if cache.is_dirty() {
        let _ = save_session_cache(cache);
    }
    let mut sqlite_ingest = crate::sqlite_sessions::SqliteIngestReport::default();
    // rm-548: the JSON storage lane's fork exclusions (counted during
    // file discovery) seed the total; the sqlite lane adds its own —
    // zero after the rm-791 supersession (its parent_id rows are
    // retained, attributed children), but the lane-level sum keeps the
    // shared carrier honest for both lanes.
    let mut opencode_fork_excluded = json_fork_excluded;
    if dir.is_none() {
        let (sqlite_sessions, ingest) =
            crate::sqlite_sessions::load_sqlite_backed_sessions_reported(options.since);
        sessions.extend(sqlite_sessions);
        sqlite_ingest = ingest;
        opencode_fork_excluded += sqlite_ingest.fork_excluded;
    }
    // rm-1417 (run 942ed30a cycle 3): one conversation, one session. A
    // gemini tmp root can hold the live conversation under chats/ AND
    // its checkpoint snapshots under checkpoints/ — cumulative usage
    // snapshots of the SAME conversation that used to render as two
    // sessions and double-count every cumulative field. Merge mirror
    // families here, before attribution and history preserve see the
    // corpus, so downstream passes operate on the merged shape.
    let cross_file_merged = merge_cross_file_conversations(&mut sessions);
    if options.preserve_history {
        // rm-787: attribute BEFORE preserving so the derived record
        // carries the subagent rollups the live view showed. The
        // post-merge attribution below used to be the only one, so
        // every record persisted zero rollups.
        crate::subagents::attribute_subagents(&mut sessions);
        let _ = preserve_derived_history(&sessions);
    }
    if options.include_history {
        merge_preserved_history(&mut sessions);
    }
    // rm-545: link Claude Code subagent transcripts to the session that
    // spawned them AFTER the history merge and BEFORE range/filters (so
    // every view sees the linked shape). Cached parents get re-attributed
    // on every load; the cache schema stays untouched. rm-787 made this
    // honest for preserved rows: they join the list already carrying the
    // rollups persisted with their record (their synthetic `history:{id}`
    // paths cannot join the real-path index, so attribution recomputes
    // live rows only and leaves preserved rows verbatim) — and rm-487's
    // same-directory rule resolves the sidechain layout Claude Code 2.x
    // actually writes on this host.
    // rm-799: the call returns the orphan count (children whose parent
    // transcript is absent from the loaded corpus) for disclosure.
    let unlinked_subagents = crate::subagents::attribute_subagents(&mut sessions);
    sessions.retain(|session| {
        // Sessions with an unknown start time stay visible (unknown-time
        // bucket, N7) instead of being silently dropped from ranged views;
        // data_health counts them via `unknown_time_sessions`.
        // rm-502: naive-ISO starts parse through the shared lenient
        // arm (lib.rs parse_ts) exactly like ingest does, so the
        // since-filter never disagrees with --overview about which
        // sessions exist in time. rm-890: admission runs through the
        // ONE shared predicate (insights::session_admitted_since) —
        // last known activity (session_end where known, else the
        // leniently-parsed start) at or after the cutoff, the same
        // basis the sqlite lanes and session_matches_time_range use,
        // so an overnight session no longer vanishes from `--range
        // today` on any lane.
        crate::insights::session_admitted_since(session, options.since)
            && matches_project_filter(session, &options.project)
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
        sqlite: sqlite_ingest,
        opencode_fork_excluded,
        parse_failures,
        first_parse_failure,
        parse_failure_paths,
        unlinked_subagents,
        cross_file_merged,
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
) -> (Vec<PathBuf>, usize) {
    if let Some(dir) = dir {
        if is_cline_task_dir(dir) {
            return (vec![dir.to_path_buf()], 0);
        }
        // rm-548: an explicitly requested directory keeps every file —
        // a forked copy loaded by explicit path still renders (the
        // exclusion is an aggregation rule, not a parse block).
        return (collect_session_files_cached(dir, cache), 0);
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
    // rm-548: opencode fork copies (session info docs whose `parentID`
    // is set) re-emit their parent's history, so aggregating them
    // double-counts usage. Drop them before enumeration and return the
    // count so the disclosure channel reports it — never silently.
    // Independent-review fix: the per-file probe is memoized on the
    // cached dir listing (fingerprint-keyed, see
    // `opencode_session_fork_parent_cached`) so warm runs stop
    // re-reading every storage session info doc on discovery.
    let mut opencode_fork_excluded = 0usize;
    all.retain(|path| {
        if opencode_session_fork_parent_cached(path, cache).is_some() {
            opencode_fork_excluded += 1;
            false
        } else {
            true
        }
    });
    (sort_paths_by_cache(all, cache), opencode_fork_excluded)
}

/// rm-548: an opencode storage session doc (`.../storage/session/info/
/// <sid>.json`) whose `parentID` field is a non-empty string marks the
/// session as a fork of that parent (opencode Session.Info schema,
/// `session.parent_id` / `session_parent_idx` in the sqlite schema;
/// pinned in docs/guides/opencode-fork-marker.md). Replaying a fork
/// re-emits the parent's history — ccusage hit the same class as
/// #1782 — so aggregation must not count both.
pub(crate) fn opencode_session_fork_parent(path: &Path) -> Option<String> {
    let mut components = path.components().rev();
    let file_is_json = components
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .is_some_and(|name| name.ends_with(".json"));
    let path_is_session_info = ["info", "session", "storage"].iter().all(|segment| {
        components
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .is_some_and(|actual| actual == *segment)
    });
    if !file_is_json || !path_is_session_info {
        return None;
    }
    let raw = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value
        .get("parentID")?
        .as_str()
        .filter(|parent| !parent.is_empty())
        .map(str::to_string)
}

/// rm-548 (independent-review fix): [`opencode_session_fork_parent`]
/// behind the session-cache memo. Only opencode-shaped session info
/// paths are memoized (everything else goes straight to the probe —
/// no journal growth for non-opencode corpora); a memo is served only
/// when the file's fingerprint still matches the probe-time one, so a
/// rewritten doc — one that gained a `parentID`, i.e. the exact state
/// change rm-548 exists to disclose — always re-probes.
pub(crate) fn opencode_session_fork_parent_cached(
    path: &Path,
    cache: &mut SessionCache,
) -> Option<String> {
    let is_info_doc = path
        .components()
        .rev()
        .skip(1)
        .take(3)
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        == ["info", "session", "storage"];
    if !is_info_doc {
        return opencode_session_fork_parent(path);
    }
    if let Some(memo) = crate::session_cache::cached_fork_parent(path, cache) {
        return memo;
    }
    let probed = opencode_session_fork_parent(path);
    crate::session_cache::store_fork_parent_probe(path, probed.clone(), cache);
    probed
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
///
/// rm-797: `attribute_subagents` (subagents.rs) reuses this identity
/// to resolve subagent parents, keeping linkage and discovery's dedup
/// on ONE identity model — a parent reachable under two spellings
/// links from every spelling, not just the one the dedup kept.
#[derive(Eq, PartialEq, Hash)]
pub(crate) enum SameFileIdentity {
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

pub(crate) fn same_file_identity(path: &Path) -> Option<SameFileIdentity> {
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
        // rm-732: the cold walk's file-kind gate re-validates here too —
        // the stored listing is trusted for directory shape, not for
        // file kind. A listing written by a pre-rm-212 walker (retired
        // by the v4 walk-version bump) or an entry whose on-disk kind
        // changed since the walk (a fifo created at a listed path) must
        // not re-admit a non-regular file by name: the parser's open on
        // a fifo wedges with no message and no timeout. Cline task dirs
        // stay admissible — the cold walk admits them as session roots
        // ahead of the regular-file gate (see :862/:947 arms).
        items.extend(
            listing
                .files
                .into_iter()
                .filter(|path| is_cline_task_dir(path) || path.is_file())
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

/// rm-1417 (run 942ed30a cycle 3): identity key of a gemini temp
/// conversation — the `~/.gemini/tmp/<root>/` directory. One tmp root
/// holds ONE conversation's mirror family (the live chat under chats/
/// and its checkpoint snapshots under checkpoints/), so the root is the
/// stable cross-file identity; the family directory is NOT part of the
/// key (that is what made the mirror render twice).
fn gemini_conversation_root(path: &Path) -> Option<&Path> {
    if !is_gemini_temp_session_file(path) {
        return None;
    }
    // .../.gemini/tmp/<root>/(chats|checkpoints)/file.json
    let family_dir = path.parent()?; // (chats|checkpoints)
    let root = family_dir.parent()?; // <root>
    let tmp = root.parent()?; // tmp
    let gemini_dir = tmp.parent()?; // .gemini
    let names_ok = tmp.file_name().is_some_and(|n| n == "tmp")
        && gemini_dir.file_name().is_some_and(|n| n == ".gemini");
    names_ok.then_some(root)
}

/// rm-1417: fold one same-conversation member's metrics into the
/// representative. Cumulative usage snapshots of one conversation
/// appear in several files, so every cumulative field takes the
/// per-field MAX across members — never a sum, which would double-count
/// the same usage. Per-conversation activity counts max with the most
/// complete snapshot; timestamps/gaps union (the conversation spans
/// both members' spans). Cost is REPRICED from the merged token totals:
/// at the representative's model for single-model families (the live
/// conversation's model — the most complete picture of what the
/// conversation ran), and per attribution entry for MULTI-MODEL
/// families (review fix F1 — a "multiple" repricing bills the builtin
/// default); a saturated token class (rm-046 clamp) keeps the clamped
/// provenance. The merge is disclosed by the caller, never a silent
/// drop.
fn merge_cross_file_metrics(repr: &mut crate::Metrics, other: &crate::Metrics) {
    let max_map = |into: &mut std::collections::BTreeMap<String, usize>,
                   from: &std::collections::BTreeMap<String, usize>| {
        for (key, count) in from {
            into.entry(key.clone())
                .and_modify(|c| *c = (*c).max(*count))
                .or_insert(*count);
        }
    };
    max_map(&mut repr.tool_usage, &other.tool_usage);
    max_map(&mut repr.file_usage, &other.file_usage);
    max_map(&mut repr.tool_arg_usage, &other.tool_arg_usage);
    max_map(&mut repr.tool_authority, &other.tool_authority);
    if repr.highest_authority.is_empty() {
        repr.highest_authority = other.highest_authority.clone();
    }
    repr.user_messages = repr.user_messages.max(other.user_messages);
    repr.assistant_turns = repr.assistant_turns.max(other.assistant_turns);
    repr.tool_results = repr.tool_results.max(other.tool_results);
    repr.tool_calls_total = repr.tool_calls_total.max(other.tool_calls_total);
    repr.tool_calls_ok = repr.tool_calls_ok.max(other.tool_calls_ok);
    repr.tool_calls_fail = repr.tool_calls_fail.max(other.tool_calls_fail);
    repr.reasoning_blocks = repr.reasoning_blocks.max(other.reasoning_blocks);
    repr.reasoning_chars = repr.reasoning_chars.max(other.reasoning_chars);
    repr.reasoning_redact = repr.reasoning_redact.max(other.reasoning_redact);
    repr.events_total = repr.events_total.max(other.events_total);
    repr.tokens_input = repr.tokens_input.max(other.tokens_input);
    repr.tokens_output = repr.tokens_output.max(other.tokens_output);
    repr.tokens_reasoning = repr.tokens_reasoning.max(other.tokens_reasoning);
    repr.tokens_cache_w = repr.tokens_cache_w.max(other.tokens_cache_w);
    repr.tokens_cache_r = repr.tokens_cache_r.max(other.tokens_cache_r);
    repr.credit_usd = repr.credit_usd.max(other.credit_usd);
    if other.session_start < repr.session_start {
        repr.session_start = other.session_start.clone();
    }
    if other.session_end > repr.session_end {
        repr.session_end = other.session_end.clone();
    }
    for ts in &other.timestamps {
        if !repr.timestamps.contains(ts) {
            repr.timestamps.push(*ts);
        }
    }
    repr.timestamps.sort_unstable();
    for gap in &other.gaps_sec {
        if !repr.gaps_sec.contains(gap) {
            repr.gaps_sec.push(*gap);
        }
    }
    repr.gaps_sec
        .sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if repr.upstream_cost_usd < other.upstream_cost_usd {
        repr.upstream_cost_usd = other.upstream_cost_usd;
    }
    repr.zero_usage_events = repr.zero_usage_events.max(other.zero_usage_events);
    repr.naive_utc_stamps = repr.naive_utc_stamps.max(other.naive_utc_stamps);
    repr.duration_sec = repr.duration_sec.max(other.duration_sec);
    // Review fix F2 (independent review 09b3e188): the member's
    // non-priced cumulative channels must fold too, or the merged
    // session silently understates them. Parse-loss line skips max per
    // reason (counters of the same conversation's lost lines — a sum
    // would double-count the same loss), reasoning-lens entries union
    // (the same block measured by two snapshots is one entry), opencode
    // diff summaries max per field, and the stored-totals drift keeps
    // the larger magnitude.
    max_map(&mut repr.line_skips, &other.line_skips);
    for len in &other.reasoning_lens {
        if !repr.reasoning_lens.contains(len) {
            repr.reasoning_lens.push(*len);
        }
    }
    repr.reasoning_lens.sort_unstable();
    let max_option = |a: Option<i64>, b: Option<i64>| match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    };
    repr.summary_additions = max_option(repr.summary_additions, other.summary_additions);
    repr.summary_deletions = max_option(repr.summary_deletions, other.summary_deletions);
    repr.summary_files = max_option(repr.summary_files, other.summary_files);
    repr.stored_totals_delta = repr.stored_totals_delta.max(other.stored_totals_delta);

    // Review fix F1 (independent review 09b3e188): a mirror family can
    // be MULTI-MODEL — parse_gemini_object threads one mutable model,
    // so a mid-conversation model switch finalizes the chats file as
    // model_used "multiple" with per-model attribution, and the
    // canonical session prices that per attribution entry. Repricing
    // the merged totals at the representative's model would bill the
    // "multiple" lookup at the builtin default (claude-opus-4.7). Fold
    // attribution with the same cumulative-MAX rule — an unattributed
    // member contributes its totals under its own model key — then
    // reprice per entry and stamp the canonical multi-model
    // provenance. A family whose fold leaves ONE model key keeps the
    // single-model shape (no attribution minted, repriced at the
    // representative's model); a degenerate "multiple" with no keys
    // at all prices at the lookup default exactly as before.
    let family_multi_model = !repr.model_attribution.is_empty()
        || !other.model_attribution.is_empty()
        || repr.model_used == "multiple"
        || other.model_used == "multiple";
    let mut attribution: std::collections::BTreeMap<String, crate::ModelAttribution> =
        std::collections::BTreeMap::new();
    if family_multi_model {
        let fold_attribution =
            |attribution: &mut std::collections::BTreeMap<String, crate::ModelAttribution>,
             member: &crate::Metrics| {
                if !member.model_attribution.is_empty() {
                    for (model, entry) in &member.model_attribution {
                        let slot = attribution.entry(model.clone()).or_default();
                        slot.input_tokens = slot.input_tokens.max(entry.input_tokens);
                        slot.output_tokens = slot.output_tokens.max(entry.output_tokens);
                        slot.cache_w = slot.cache_w.max(entry.cache_w);
                        slot.cache_r = slot.cache_r.max(entry.cache_r);
                        slot.cost_usd = slot.cost_usd.max(entry.cost_usd);
                    }
                } else if !member.model_used.is_empty()
                    && member.model_used != "multiple"
                    && member.model_used != "unknown"
                {
                    // single-model member: its cumulative totals ride its
                    // own model key
                    let slot = attribution.entry(member.model_used.clone()).or_default();
                    slot.input_tokens = slot.input_tokens.max(member.tokens_input);
                    slot.output_tokens = slot.output_tokens.max(member.tokens_output);
                    slot.cache_w = slot.cache_w.max(member.tokens_cache_w);
                    slot.cache_r = slot.cache_r.max(member.tokens_cache_r);
                }
            };
        fold_attribution(&mut attribution, repr);
        fold_attribution(&mut attribution, other);
    }
    let repriced_multi_model = attribution.len() > 1;
    // Reprice from the merged token totals. Single-model arm: analyze's
    // four-class formula at the representative's model (gemini mirror
    // families carry no credits or recorded-cost rows on this lane).
    // Multi-model arm: each model's merged tokens at that model's own
    // rates, seeded with the merged upstream-recorded total like
    // analyze's per-block rule.
    let token_priced = if repriced_multi_model {
        let mut per_model_cost = repr.upstream_cost_usd;
        for (model, entry) in &mut attribution {
            let price = crate::pricing::lookup_price_tiered(model, None);
            let cost = entry.input_tokens as f64 / 1e6 * price.input
                + entry.output_tokens as f64 / 1e6 * price.output
                + entry.cache_w as f64 / 1e6 * price.cw
                + entry.cache_r as f64 / 1e6 * price.cr;
            per_model_cost += cost;
            entry.cost_usd = crate::round4(cost.max(entry.cost_usd));
        }
        per_model_cost
    } else {
        let price = crate::pricing::lookup_price_tiered(&repr.model_used, None);
        repr.tokens_input as f64 / 1e6 * price.input
            + repr.tokens_output as f64 / 1e6 * price.output
            + repr.tokens_cache_w as f64 / 1e6 * price.cw
            + repr.tokens_cache_r as f64 / 1e6 * price.cr
    };
    repr.cost_estimated = crate::round4(if repr.credit_usd > 0.0 {
        token_priced.max(repr.credit_usd)
    } else {
        token_priced
    });
    let saturated = [
        repr.tokens_input,
        repr.tokens_output,
        repr.tokens_reasoning,
        repr.tokens_cache_w,
        repr.tokens_cache_r,
    ]
    .contains(&i64::MAX);
    if saturated {
        repr.provenance.cost = "calculated_from_tokens_clamped".to_string();
    } else if repr.credit_usd > 0.0 && repr.credit_usd >= token_priced {
        repr.provenance.cost = "calculated_from_copilot_credits".to_string();
    } else if repriced_multi_model {
        repr.provenance.cost = "calculated_per_message_tokens".to_string();
    } else {
        repr.provenance.cost = "calculated_from_tokens".to_string();
    }
    if repriced_multi_model {
        repr.model_used = "multiple".to_string();
        repr.model_attribution = attribution;
    }
    repr.provenance.tokens = "cumulative max across merged same-conversation files".to_string();
    for (key, count) in &other.disclosure_counters {
        *repr.disclosure_counters.entry(key.clone()).or_insert(0) += count;
    }
}

/// rm-1417: merge gemini mirror families in a loaded corpus. A tmp root
/// with members in more than one family directory (chats/ plus
/// checkpoints/) is one conversation rendered per file — fold the
/// family into the chats member (the live conversation) with cumulative
/// MAX metrics and a disclosed `cross_file_sessions_merged` counter.
/// Identity stays SCOPED: a root with MORE THAN ONE chats file is
/// ambiguous (two conversations sharing a tmp dir is not the layout
/// gemini writes) — nothing merges there; and a family with only
/// checkpoint snapshots still merges (representative = the first
/// checkpoint in discovery order). Returns the number of sessions
/// folded away.
fn merge_cross_file_conversations(sessions: &mut Vec<Session>) -> usize {
    // Map conversation root -> (indices of members, number of chats-lane
    // members). Retain original order via indices.
    let mut families: std::collections::BTreeMap<PathBuf, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, session) in sessions.iter().enumerate() {
        let path = Path::new(&session.path);
        if let Some(root) = gemini_conversation_root(path) {
            families.entry(root.to_path_buf()).or_default().push(index);
        }
    }
    let mut fold: Vec<(usize /*repr*/, usize /*member*/)> = Vec::new();
    for indices in families.values() {
        if indices.len() < 2 {
            continue;
        }
        let chats_members = indices
            .iter()
            .filter(|i| {
                Path::new(&sessions[**i].path)
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|n| n.to_str())
                    == Some("chats")
            })
            .count();
        // Scoped fallback: an ambiguous root (multiple live chats under
        // one tmp dir) never merges — per-file keys only.
        if chats_members > 1 {
            continue;
        }
        let repr_index = indices
            .iter()
            .copied()
            .find(|i| {
                Path::new(&sessions[*i].path)
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|n| n.to_str())
                    == Some("chats")
            })
            .or_else(|| indices.first().copied())
            .expect("family holds at least one member");
        for member in indices {
            if *member != repr_index {
                fold.push((repr_index, *member));
            }
        }
    }
    if fold.is_empty() {
        return 0;
    }
    let folded = fold.len();
    // Apply folds onto representatives (a representative may absorb
    // several members).
    for (repr_index, member_index) in &fold {
        let (repr, member) = if repr_index < member_index {
            let (left, right) = sessions.split_at_mut(*member_index);
            (&mut left[*repr_index], &right[0])
        } else {
            let (left, right) = sessions.split_at_mut(*repr_index);
            (&mut right[0], &left[*member_index])
        };
        // Review fix F2 (independent review 09b3e188): `diagnostics` is a
        // per-snapshot ANALYSIS (fingerprints/latencies/steps index into
        // their own member's event order), so it is not additive — but
        // dropping the folded member's loses the more complete picture
        // when the chats file is windowed down and the checkpoint
        // snapshot is the fuller member. Carry the richer member's
        // analysis; events_total before the metrics fold is the
        // completeness measure (it is maxed inside the fold).
        let member_richer = member.metrics.events_total > repr.metrics.events_total;
        merge_cross_file_metrics(&mut repr.metrics, &member.metrics);
        if member_richer {
            repr.diagnostics = member.diagnostics.clone();
        }
        let anomaly_key =
            |a: &crate::Anomaly| (a.kind.clone(), a.severity.clone(), a.detail.clone());
        let mut anomaly_keys: Vec<_> = repr.anomalies.iter().map(anomaly_key).collect();
        for anomaly in &member.anomalies {
            let key = anomaly_key(anomaly);
            if !anomaly_keys.contains(&key) {
                anomaly_keys.push(key);
                repr.anomalies.push(anomaly.clone());
            }
        }
        repr.health = repr.health.min(member.health);
        let warning_key = |w: &crate::ToolWarning| {
            (
                w.tool_name.clone(),
                w.pattern.clone(),
                w.count,
                w.detail.clone(),
                w.severity.clone(),
            )
        };
        let mut warning_keys: Vec<_> = repr.tool_warnings.iter().map(warning_key).collect();
        for warning in &member.tool_warnings {
            let key = warning_key(warning);
            if !warning_keys.contains(&key) {
                warning_keys.push(key);
                repr.tool_warnings.push(warning.clone());
            }
        }
    }
    // Disclose: one counter per folded member on the representative.
    for (repr_index, _member) in &fold {
        *sessions[*repr_index]
            .metrics
            .disclosure_counters
            .entry("cross_file_sessions_merged".to_string())
            .or_insert(0) += 1;
    }
    // Drop folded members, preserving corpus order.
    let folded_set: std::collections::BTreeSet<usize> = fold.iter().map(|(_, m)| *m).collect();
    let mut write = 0;
    for read in 0..sessions.len() {
        if !folded_set.contains(&read) {
            sessions.swap(read, write);
            write += 1;
        }
    }
    sessions.truncate(write);
    folded
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

/// rm-1417 review-fix unit pins (independent review 09b3e188, attempt
/// 97ef8844). The corpus-level arms live in tests/ingest_truth_batch.rs;
/// these pin the merge itself per finding.
#[cfg(test)]
mod cross_file_merge_tests {
    use super::*;

    fn entry(input: i64, output: i64) -> crate::ModelAttribution {
        crate::ModelAttribution {
            input_tokens: input,
            output_tokens: output,
            ..Default::default()
        }
    }

    fn priced(model: &str, input: i64, output: i64) -> f64 {
        let price = crate::pricing::lookup_price_tiered(model, None);
        input as f64 / 1e6 * price.input + output as f64 / 1e6 * price.output
    }

    /// Independent review 09b3e188 F1 (red-first): a MULTI-MODEL mirror
    /// family must be repriced per attribution entry, never at the
    /// `"multiple"` lookup's builtin default, and the member's per-model
    /// snapshot folds per key with cumulative MAX.
    #[test]
    fn multi_model_mirror_reprices_per_attribution_entry() {
        let mut repr = crate::Metrics {
            model_used: "multiple".to_string(),
            tokens_input: 1500,
            tokens_output: 300,
            ..Default::default()
        };
        repr.model_attribution
            .insert("gemini-2.5-pro".to_string(), entry(1000, 200));
        repr.model_attribution
            .insert("gemini-2.5-flash".to_string(), entry(500, 100));
        let other = crate::Metrics {
            // single-model member: no attribution map, its totals ride its
            // own model key
            model_used: "gemini-2.5-flash".to_string(),
            tokens_input: 900,
            tokens_output: 150,
            ..Default::default()
        };
        merge_cross_file_metrics(&mut repr, &other);
        assert_eq!(repr.model_used, "multiple");
        let pro = repr
            .model_attribution
            .get("gemini-2.5-pro")
            .expect("pro attribution survives the fold");
        assert_eq!((pro.input_tokens, pro.output_tokens), (1000, 200));
        let flash = repr
            .model_attribution
            .get("gemini-2.5-flash")
            .expect("flash attribution survives the fold");
        assert_eq!(
            (flash.input_tokens, flash.output_tokens),
            (900, 150),
            "per-key cumulative MAX: the member's fuller flash snapshot wins"
        );
        assert_eq!(
            repr.cost_estimated,
            crate::round4(
                priced("gemini-2.5-pro", 1000, 200) + priced("gemini-2.5-flash", 900, 150)
            ),
            "multi-model families price each model's tokens at that model's rates"
        );
        assert_eq!(repr.provenance.cost, "calculated_per_message_tokens");
    }

    /// Independent review 09b3e188 F1 regression pin: the SINGLE-model
    /// mirror keeps its pre-existing serialized shape (no attribution
    /// minted, repriced at the representative's model).
    #[test]
    fn single_model_mirror_keeps_shape_and_reprice() {
        let mut repr = crate::Metrics {
            model_used: "gemini-2.5-pro".to_string(),
            tokens_input: 1200,
            tokens_output: 240,
            ..Default::default()
        };
        let other = crate::Metrics {
            model_used: "gemini-2.5-pro".to_string(),
            tokens_input: 900,
            tokens_output: 150,
            ..Default::default()
        };
        merge_cross_file_metrics(&mut repr, &other);
        assert!(repr.model_attribution.is_empty());
        assert_eq!(repr.model_used, "gemini-2.5-pro");
        assert_eq!(repr.provenance.cost, "calculated_from_tokens");
        assert_eq!(
            repr.cost_estimated,
            crate::round4(priced("gemini-2.5-pro", 1200, 240))
        );
    }

    /// Independent review 09b3e188 F2 (red-first): the merge must fold the
    /// member-side non-priced cumulative channels too — parse-loss line
    /// skips max per reason, reasoning lens entries union, opencode diff
    /// summaries max per field, stored-totals drift keeps the larger
    /// magnitude. The first cut dropped them wholesale.
    #[test]
    fn merge_folds_member_side_counters_not_just_totals() {
        let mut repr = crate::Metrics {
            model_used: "gemini-2.5-pro".to_string(),
            tokens_input: 1000,
            tokens_output: 200,
            reasoning_lens: vec![120, 340],
            stored_totals_delta: 3,
            ..Default::default()
        };
        repr.line_skips.insert("event_schema".to_string(), 2);
        repr.summary_additions = Some(40);
        let mut other = crate::Metrics {
            model_used: "gemini-2.5-pro".to_string(),
            tokens_input: 800,
            tokens_output: 90,
            reasoning_lens: vec![340, 512],
            stored_totals_delta: 9,
            ..Default::default()
        };
        other.line_skips.insert("event_schema".to_string(), 5);
        other.line_skips.insert("non_event".to_string(), 1);
        other.summary_additions = Some(65);
        other.summary_deletions = Some(12);
        merge_cross_file_metrics(&mut repr, &other);
        assert_eq!(repr.line_skips.get("event_schema"), Some(&5));
        assert_eq!(repr.line_skips.get("non_event"), Some(&1));
        assert_eq!(repr.reasoning_lens, vec![120, 340, 512]);
        assert_eq!(repr.summary_additions, Some(65));
        assert_eq!(repr.summary_deletions, Some(12));
        assert_eq!(repr.stored_totals_delta, 9);
    }

    /// Independent review 09b3e188 F2 (red-first): `diagnostics` is a
    /// per-snapshot ANALYSIS (fingerprints/steps index into their own
    /// member's event order), so it is not additive — but it must also not
    /// be dropped: when the folded member is the more complete snapshot
    /// (the windowed-chats case), its analysis is the one that survives.
    #[test]
    fn merge_carries_the_more_complete_member_diagnostics() {
        let mk_events = |count: usize| {
            (0..count)
                .map(|i| crate::Event {
                    role: "user".to_string(),
                    content: format!("turn {i}"),
                    ..Default::default()
                })
                .collect::<Vec<_>>()
        };
        let repr_session =
            crate::session_from_events("a", "/t/.gemini/tmp/c/chats/a.json", mk_events(4))
                .expect("repr session");
        let mut member =
            crate::session_from_events("b", "/t/.gemini/tmp/c/checkpoints/b.json", mk_events(40))
                .expect("member session");
        member.diagnostics.loop_cost.total_loop_cost = 1.25;
        let mut sessions = vec![repr_session, member];
        let folded = merge_cross_file_conversations(&mut sessions);
        assert_eq!(folded, 1, "one member folded");
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            sessions[0].diagnostics.loop_cost.total_loop_cost, 1.25,
            "the fuller checkpoint snapshot's analysis must survive the fold"
        );
        assert_eq!(sessions[0].metrics.events_total, 40);
    }
}

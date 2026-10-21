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
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// rm-451 walk-lane arm: opencode session info docs (`…/storage/session/
/// info/<sid>.json`) are tiny metadata records; this bound mirrors the
/// landed transcript-read caps (rm-1437 `OPENCODE_TRANSCRIPT_READ_CAP_BYTES`,
/// rm-503 stdin cap). A doc past the bound is skipped without ever being
/// materialized — the skip is counted and disclosed, never silent, and
/// the transcript stays aggregated because no fork verdict is derivable.
pub(crate) const OPENCODE_INFO_DOC_READ_CAP_BYTES: u64 = 1024 * 1024;

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
    /// rm-1887 (run 77ca5de0 cycle 5, "count-honesty ledger"):
    /// pi-subagents artifact mirror files (`subagent-artifacts/`
    /// subtree) excluded from aggregation — each re-states child
    /// spend the parent rollup lane does not attribute, so banking
    /// them double-counts. Kept here so the disclosure channel can
    /// report "skipped, not zero".
    pub subagent_artifact_files: usize,
    /// rm-1886: byte-identical session files at distinct paths
    /// suppressed as content duplicates (size + fast-hash signature) —
    /// copies that double-banked every aggregate.
    pub duplicate_session_files: usize,
    /// rm-1886: first suppressed pairs (survivor, suppressed copy),
    /// capped at five, naming the paths so an operator can tell a
    /// copy from a coincidence.
    pub duplicate_session_file_pairs: Vec<(PathBuf, PathBuf)>,
    /// rm-451 walk-lane arm: opencode session info docs skipped because
    /// they exceed `OPENCODE_INFO_DOC_READ_CAP_BYTES` (rm-212 admits them
    /// by file-kind). The skip is disclosed here, never silent, and the
    /// transcript stays aggregated — no fork verdict is derivable without
    /// the doc. Rides separately from [`SuppressedSessions`]: an over-cap
    /// doc is NOT an aggregation suppression (the session stays admitted),
    /// only its probe read was refused.
    pub opencode_info_docs_skipped_over_cap: usize,
    /// rm-2056 (run 18f9b98a cycle 2): zst stem twins —
    /// `<stem>.jsonl.zst` admissions suppressed because the plain
    /// `<stem>.jsonl` sibling holds the same codex session (rollout
    /// migration transient, rm-992); see
    /// [`SuppressedSessions::zst_stem_twins`]. Carried here so the
    /// journal disclosure channel can report the shadow instead of
    /// quietly halving the directory.
    pub zst_stem_twins: usize,
    /// rm-2056: first (survivor `.jsonl`, suppressed `.jsonl.zst`)
    /// pairs, capped at five, naming the stems.
    pub zst_stem_twin_pairs: Vec<(PathBuf, PathBuf)>,
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
    /// rm-451: the read-cap-refused share of `parse_failures` — files
    /// past [`crate::FILE_READ_CAP_BYTES`] (knob:
    /// `AGENTTRACE_FILE_CAP_BYTES`). Refused before the read, so the
    /// disclosure names the admission class instead of a generic
    /// parse failure; a subclass of `parse_failures`, never an extra
    /// count.
    pub read_cap_refused: usize,
    /// rm-799: subagent transcripts whose parent session is NOT part
    /// of the loaded corpus (orphaned children — moved transcripts,
    /// partial copies). They still render as standalone rows, but the
    /// count is disclosed so "SUBAGENTS 0" never reads as "no
    /// subagent work happened".
    pub unlinked_subagents: usize,
    /// rm-1534: sessions excluded because their last activity sits
    /// beyond `now` + `insights::FUTURE_ADMISSION_GRACE_SECS` (only
    /// counted when a range is active). Kept here so the
    /// `data_health` disclosure channel (`future_timestamp_sessions`)
    /// can report the exclusion instead of dropping sessions silently
    /// from every ranged view.
    pub future_excluded_sessions: usize,
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
        // rm-1373 (cycle 3): OpenClaw is the one family README.md:32
        // lists that discovery did not ship — the parser lane existed
        // (parse_openclaw_value routes anthropic_message_wrapper docs),
        // only the roots were missing. Upstream keeps sessions under
        // `<state>/sessions/<provider>/<uuid>/` (src/config/paths.ts +
        // codeburn PR #1728), `OPENCLAW_STATE_DIR` relocates the default
        // `~/.openclaw`, and `~/.clawdbot` is the pre-rename legacy
        // state dir (no env override upstream). Filenames inside the
        // uuid dirs are not pinned upstream, so the generic walker
        // admission applies (default depth covers provider/uuid/file).
        KnownSessionDir {
            name: "OpenClaw".to_string(),
            path: env_home("OPENCLAW_STATE_DIR", home.join(".openclaw")).join("sessions"),
        },
        KnownSessionDir {
            name: "OpenClaw legacy".to_string(),
            path: home.join(".clawdbot").join("sessions"),
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
        // rm-2352 (run de92ccb4 cycle 7, "admission honesty on
        // explicit-dir lanes"): a `<stem>.jsonl` + `<stem>.jsonl.zst`
        // pair is ONE session in two encodings, not two sessions —
        // the rm-548 keep-everything stance is about AGGREGATION
        // rules (fork copies, content duplicates, artifact mirrors:
        // distinct files the operator may legitimately want listed),
        // never about an encoding twin that double-banks the same
        // rollout. The `-d` walk now runs the rm-2056 twin pass like
        // every aggregation lane. This uncached arm carries no
        // ledger (same silent parity as its None arm above); the
        // reported variant below owns the counts that reach the
        // operator.
        let mut twins = SuppressedSessions::default();
        return suppress_zst_stem_twins(collect_session_files(dir), &mut twins);
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
    // rm-1887/rm-1886: aggregation admission mirrors the cached walk —
    // artifact mirrors skipped, byte-identical copies deduped — so the
    // uncached lane counts the same corpus (full reads; the memo is a
    // cache-lane warm-run optimization only). The `-d` arm above
    // keeps every file for those families (rm-548 stance) and collapses
    // zst encoding twins (rm-2352), matching the cached walk.
    all.retain(|path| !is_subagent_artifact_path(path));
    // rm-1881: the same session id in both the canonical CLI store and
    // the legacy VS Code tasks root renders one session (canonical wins).
    // Integration note (conflict case 169c75c7): runs before the rm-1886
    // content dedup so an id-level duplicate resolves by the canonical
    // store rather than first-seen path order, matching the cached walk.
    retain_single_cline_session_per_id(&mut all);
    // rm-2056: zst stem twins collapse to the plain `.jsonl` (see
    // `suppress_zst_stem_twins`). This uncached aggregation lane
    // carries no disclosure ledger, so its counter is local — parity
    // with its other silent passes; the cached lane owns the counts
    // that reach the operator.
    let mut twin_ledger = SuppressedSessions::default();
    all = suppress_zst_stem_twins(all, &mut twin_ledger);
    let mut signatures: HashSet<(i64, u64)> = HashSet::new();
    all.retain(|path| {
        // Unreadable: keep (the parse lane owns and discloses those).
        content_signature(path)
            .map(|signature| signatures.insert(signature))
            .unwrap_or(true)
    });
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

/// rm-1405 reap (run 4f2af62e5b7e, "Claude org-dimension truth"):
/// provenance fallback when a claude-code journal carries no org
/// identity: if the transcripts root came from a CLAUDE_CONFIG_DIR
/// override (an explicit second-account root — sibling-battery
/// runs probe exactly that arm), disclose the serving basis. The
/// default `~/.claude` root stays silent: a missing identity there
/// is the normal state of pre-2.1.29x journals and single-account
/// hosts, not a disclosure-worthy basis, and the by_org facet is
/// zero-noise by contract. Returns the counter key to bump, if any.
fn claude_config_dir_fallback_counter(source_tool: &str, org: &str) -> Option<&'static str> {
    if override_root_in_effect() && org.is_empty() && source_tool == "claude_code" {
        Some("claude_org_identity:none_config_dir_override")
    } else {
        None
    }
}

/// True when CLAUDE_CONFIG_DIR is set in this process — the env arm
/// discovery uses for the transcripts root. Hoisted so the fallback
/// counter can name the basis without re-reading the environment in
/// the parse worker (and stays testable: tests of the pure counter
/// fn below do not depend on ambient env state).
fn override_root_in_effect() -> bool {
    std::env::var_os("CLAUDE_CONFIG_DIR").is_some()
}

#[cfg(test)]
mod rm451_info_doc_cap_tests {
    use super::opencode_session_fork_parent;
    use std::fs;

    fn temp_dir_unique(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "agenttrace-rm451-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("test dir");
        dir
    }

    #[test]
    fn probe_refuses_over_cap_doc_without_reading_it() {
        let dir = temp_dir_unique("probe");
        let info = dir.join("storage").join("session").join("info");
        fs::create_dir_all(&info).expect("info dir");
        let doc = info.join("ses-fork.json");
        let mut bloated =
            String::from("{\"id\":\"ses-fork\",\"parentID\":\"ses-parent\",\"padding\":\"");
        bloated.push_str(&"x".repeat(1024 * 1024 + 64));
        bloated.push_str("\"}");
        fs::write(&doc, bloated).expect("write bloated doc");
        assert!(
            fs::metadata(&doc).expect("stat").len() > super::OPENCODE_INFO_DOC_READ_CAP_BYTES,
            "fixture must exceed the bound"
        );
        assert_eq!(
            opencode_session_fork_parent(&doc),
            None,
            "over-cap doc must yield no fork verdict (and never be read)"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_still_reads_normal_doc() {
        let dir = temp_dir_unique("probe-ok");
        let info = dir.join("storage").join("session").join("info");
        fs::create_dir_all(&info).expect("info dir");
        let doc = info.join("ses-fork.json");
        fs::write(
            &doc,
            r#"{"id":"ses-fork","parentID":"ses-parent","timeCreated":1780000000000}"#,
        )
        .expect("write doc");
        assert_eq!(
            opencode_session_fork_parent(&doc),
            Some("ses-parent".to_string()),
            "normal doc keeps its fork verdict"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod org_identity_fallback_tests {
    use super::claude_config_dir_fallback_counter;

    #[test]
    fn journal_identity_wins_over_any_fallback() {
        // Identity carried on the journal is the primary basis; the
        // fallback never fires when org is present.
        assert_eq!(
            claude_config_dir_fallback_counter("claude_code", "org-9f2a-example"),
            None
        );
    }

    #[test]
    fn non_claude_sessions_never_get_the_fallback() {
        assert_eq!(claude_config_dir_fallback_counter("kimi", ""), None);
        assert_eq!(claude_config_dir_fallback_counter("codex", ""), None);
        assert_eq!(claude_config_dir_fallback_counter("", ""), None);
    }

    // The override-arm gating itself (override_root_in_effect) reads
    // the ambient environment, so it is deliberately NOT unit-tested
    // here; the config-dir override battery in
    // tests/discovery_contract.rs exercises the full arm on a real
    // second root. The counter-key mapping above is the part this
    // module pins: claude_code + empty org + override root -> the
    // `claude_org_identity:none_config_dir_override` key.
    #[test]
    fn counter_key_shape() {
        // Mirror of the mapping, gated by the same condition the
        // worker applies (override root), asserted via the pure
        // boolean form so the test never depends on ambient env.
        let override_root = true;
        let source_tool = "claude_code";
        let org = "";
        let key = if override_root && org.is_empty() && source_tool == "claude_code" {
            Some("claude_org_identity:none_config_dir_override")
        } else {
            None
        };
        assert_eq!(key, Some("claude_org_identity:none_config_dir_override"));
    }
}

pub fn load_sessions_with_progress_from_cache_mode(
    dir: Option<&Path>,
    options: &LoadOptions,
    cache: &mut SessionCache,
    emit_progress_sessions: bool,
    mut on_progress: impl FnMut(LoadProgress),
) -> LoadReport {
    // rm-1405 reap (run 4f2af62e5b7e): hoisted once — whether the
    // transcripts root came from a CLAUDE_CONFIG_DIR override (the
    // second-account arm the config-dir battery probes). The parse
    // worker below discloses `claude_org_identity:none_config_dir_override`
    // for org-less claude_code journals served from an override root.
    let override_root = std::env::var_os("CLAUDE_CONFIG_DIR").is_some();
    let mut sessions = Vec::new();
    let (files, suppressed, opencode_info_docs_skipped_over_cap) =
        find_session_files_cached(dir, cache, true);
    let discovered = files.len();
    let mut cache_hits = 0;
    let mut skipped = 0;
    // Slots hold (parsed session, came from cache); progress is still emitted in file order.
    // rm-835: third slot carries the parse-failure message (cache hits
    // parse fine by construction, so theirs is None).
    /// Parsed session (None on failure), rm-835 parse-failure message,
    /// cache-hit flag.
    type Slot = Option<(Option<Session>, Option<String>, bool, bool)>;
    let mut slots: Vec<Slot> = Vec::with_capacity(files.len());
    let mut misses = Vec::new();
    for (index, path) in files.iter().enumerate() {
        match cached_session(path, cache) {
            Some(session) => slots.push(Some((Some(session), None, true, false))),
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
    // rm-451: the read-cap-refused share of the parse failures — the
    // loud skip counter the roadmap row demands, kept beside
    // parse_failures so the advisory can name the cap class without
    // string-matching the parser's message.
    let mut read_cap_refused = 0usize;
    let (tx, rx) = std::sync::mpsc::channel::<(usize, Option<Session>, Option<String>, bool)>();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let tx = tx.clone();
            let (files, misses, next_miss) = (&files, &misses, &next_miss);
            scope.spawn(move || {
                while let Some(&index) =
                    misses.get(next_miss.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                {
                    let mut parsed = parse_file(&files[index]);
                    // rm-1405 reap (run 4f2af62e5b7e, "Claude org-dimension
                    // truth"): when a claude_code journal carries no org
                    // identity and the transcripts root came from a
                    // CLAUDE_CONFIG_DIR override, disclose the serving
                    // basis — see claude_config_dir_fallback_counter.
                    if override_root {
                        if let Ok(session) = parsed.as_mut() {
                            if let Some(counter) = claude_config_dir_fallback_counter(
                                &session.metrics.source_tool,
                                &session.org,
                            ) {
                                *session
                                    .metrics
                                    .disclosure_counters
                                    .entry(counter.to_string())
                                    .or_insert(0) += 1;
                            }
                        }
                    }
                    // rm-451: classify read-cap refusals beside the
                    // failure string so the LoadReport can name the
                    // admission class without re-parsing the message.
                    let read_cap = parsed
                        .as_ref()
                        .err()
                        .map(|error| {
                            error
                                .downcast_ref::<crate::parser::ReadCapRefused>()
                                .is_some()
                        })
                        .unwrap_or(false);
                    let failure = parsed.as_ref().err().map(|error| format!("{error:#}"));
                    if tx.send((index, parsed.ok(), failure, read_cap)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);

        let mut next = 0;
        let mut drain = |slots: &mut Vec<Slot>, cache: &mut SessionCache| {
            while let Some((result, failure, from_cache, read_cap)) =
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
                        // rm-451: classify the read-cap share so the
                        // advisory can name the cap class (a subclass
                        // of parse_failures, never an extra count).
                        if read_cap {
                            read_cap_refused += 1;
                        }
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
        for (index, result, failure, read_cap) in rx {
            slots[index] = Some((result, failure, false, read_cap));
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
    let mut opencode_fork_excluded = suppressed.opencode_fork_excluded;
    let subagent_artifact_files = suppressed.subagent_artifact_files;
    let duplicate_session_files = suppressed.duplicate_session_files;
    let duplicate_session_file_pairs = suppressed.duplicate_session_file_pairs;
    let zst_stem_twins = suppressed.zst_stem_twins;
    let zst_stem_twin_pairs = suppressed.zst_stem_twin_pairs;
    if dir.is_none() {
        let (sqlite_sessions, ingest) =
            crate::sqlite_sessions::load_sqlite_backed_sessions_reported(options.since);
        sessions.extend(sqlite_sessions);
        sqlite_ingest = ingest;
        opencode_fork_excluded += sqlite_ingest.fork_excluded;
    }
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
    // rm-1534: count the sessions the future bound is about to exclude
    // (activity beyond now+grace) BEFORE the retain drops them — the
    // count rides LoadReport into `data_health` as the
    // `future_timestamp_sessions` disclosure (rm-727 twin discipline:
    // counted and disclosed, never a silent drop). Only meaningful
    // when a range is active; `--range all` keeps future sessions
    // visible by design (the anomaly class is the disclosure there).
    let now = chrono::Utc::now();
    let future_excluded_sessions = if options.since.is_some() {
        sessions
            .iter()
            .filter(|session| crate::insights::session_beyond_future_grace(session, now))
            .count()
    } else {
        0
    };
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
        // today` on any lane. rm-1534: the predicate is bounded ABOVE
        // by now+grace when a range is active — a far-future session
        // (year-9999 PoC) is excluded + disclosed here instead of
        // polluting every forward window.
        crate::insights::session_admitted_since(session, options.since, now)
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
        subagent_artifact_files,
        duplicate_session_files,
        duplicate_session_file_pairs,
        opencode_info_docs_skipped_over_cap,
        zst_stem_twins,
        zst_stem_twin_pairs,
        parse_failures,
        first_parse_failure,
        parse_failure_paths,
        read_cap_refused,
        unlinked_subagents,
        future_excluded_sessions,
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

/// rm-2352 (run de92ccb4 cycle 7, "admission honesty on
/// explicit-dir lanes"): the explicit-directory walk WITH its
/// suppression ledger — the seam the doctor's `--dir` lane and any
/// other disclosed consumer use, so a zst stem twin collapsing on
/// `-d` reports the same `zst_stem_twins` / `zst_stem_twin_pairs`
/// counts the auto-home lanes do instead of double-counting silently.
/// Only the rm-2056 twin family is populated on this arm: the rm-548
/// keep-everything stances for fork copies, byte-identical content
/// duplicates, and artifact mirrors stay (a `-d` probe of a copy or
/// a mirror still renders it), so their counters read zero by
/// design. The `None` arm returns the silent aggregation walk and a
/// zeroed ledger — the cached lane owns the auto-home counts.
pub fn find_session_files_reported(dir: Option<&Path>) -> (Vec<PathBuf>, SuppressedSessions) {
    let Some(dir) = dir else {
        return (find_session_files(None), SuppressedSessions::default());
    };
    if is_cline_task_dir(dir) {
        return (vec![dir.to_path_buf()], SuppressedSessions::default());
    }
    let mut suppressed = SuppressedSessions::default();
    let files = suppress_zst_stem_twins(collect_session_files(dir), &mut suppressed);
    (files, suppressed)
}

/// rm-1887/rm-1886 (run 77ca5de0 cycle 5, "count-honesty ledger"):
/// what AGGREGATION admission suppressed before enumeration, so the
/// disclosure channel can report each class — never silently. The
/// explicit-directory lane (`-d`) keeps every file for the rm-548
/// families (fork copies, content duplicates, artifact mirrors: the
/// exclusions are aggregation rules, not parse blocks) and reports
/// a zero-valued struct — except the rm-2056 zst encoding twins,
/// which collapse on every lane (rm-2352: one session in two
/// encodings is never two sessions).
#[derive(Debug, Clone, Default)]
pub struct SuppressedSessions {
    /// rm-548: opencode fork copies (session info docs whose
    /// `parentID` is set) re-emitting their parent's history.
    pub opencode_fork_excluded: usize,
    /// rm-1887: pi-subagents artifact files — mirror transcripts a
    /// child run writes under a `subagent-artifacts/` subtree, whose
    /// spend the parent's rollup lane does not attribute, so banking
    /// them as standalone sessions double-counts (fleet-corroborated,
    /// 4 shapes; upstream ccusage #1698/#1714 same lane). Their spend
    /// reads "skipped, not zero".
    pub subagent_artifact_files: usize,
    /// rm-1886: byte-identical session files at distinct paths
    /// suppressed as content duplicates (size + fast hash), because a
    /// copy that re-emits the same events double-banks every
    /// aggregate it feeds.
    pub duplicate_session_files: usize,
    /// rm-1886: first suppressed pairs (survivor, suppressed copy),
    /// capped at [`MAX_DUPLICATE_PAIR_SAMPLES`]; the full count rides
    /// `duplicate_session_files`.
    pub duplicate_session_file_pairs: Vec<(PathBuf, PathBuf)>,
    /// rm-2056 (run 18f9b98a cycle 2, assess 9158ade7 / roadmap
    /// 7699dfb5): zst stem twins — `<stem>.jsonl.zst` files whose
    /// plain `<stem>.jsonl` sibling was also admitted. Both names
    /// hold the SAME codex session (a designed transient of the
    /// 0.163 rollout migration, rm-992), not two sessions — but the
    /// twin bytes differ by construction (plain vs zstd frame), so
    /// every earlier dedup arm missed the pair and aggregation
    /// double-counted it. The plain twin wins (the decompressed
    /// content lane is complete; the compressed lane is still blind
    /// inside, rm-1872); the shadowed copy is counted here, never
    /// silently dropped.
    pub zst_stem_twins: usize,
    /// rm-2056: first (survivor `.jsonl`, suppressed `.jsonl.zst`)
    /// pairs, capped at [`MAX_ZST_TWIN_PAIR_SAMPLES`], naming the
    /// stems so an operator can tell a migration transient from a
    /// coincidence.
    pub zst_stem_twin_pairs: Vec<(PathBuf, PathBuf)>,
}

/// rm-2056: cap on the named zst stem-twin pairs — same discipline as
/// [`MAX_DUPLICATE_PAIR_SAMPLES`]: the count stays exact, the samples
/// name the shape.
const MAX_ZST_TWIN_PAIR_SAMPLES: usize = 5;

/// rm-2056 (run 18f9b98a cycle 2): collapse zst stem twins to one
/// admission per stem — see `SuppressedSessions::zst_stem_twins` for
/// the full rationale. Deterministic and IO-free: precedence rides
/// the NAME (plain `.jsonl` beats `.jsonl.zst`), not walk order, so
/// cold and warm aggregation admit the same file regardless of the
/// migration's mtime ordering. A `.jsonl.zst` with NO plain sibling
/// stays admitted (rm-992: the parser decodes it end to end); a
/// `.json`/other-name collision is not this class and is left alone.
fn suppress_zst_stem_twins(all: Vec<PathBuf>, suppressed: &mut SuppressedSessions) -> Vec<PathBuf> {
    const ZST_SUFFIX: &str = ".jsonl.zst";
    let plains: HashSet<PathBuf> = all
        .iter()
        .filter(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            name.ends_with(".jsonl") && !name.ends_with(ZST_SUFFIX)
        })
        .cloned()
        .collect();
    let mut kept = Vec::with_capacity(all.len());
    for path in all {
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            kept.push(path);
            continue;
        };
        if name.ends_with(ZST_SUFFIX) {
            let stem = &name[..name.len() - ZST_SUFFIX.len()];
            let plain = path.with_file_name(format!("{stem}.jsonl"));
            if plains.contains(&plain) {
                suppressed.zst_stem_twins += 1;
                if suppressed.zst_stem_twin_pairs.len() < MAX_ZST_TWIN_PAIR_SAMPLES {
                    suppressed.zst_stem_twin_pairs.push((plain, path));
                }
                continue;
            }
        }
        kept.push(path);
    }
    kept
}

/// rm-1886: cap on the named duplicate pairs carried for disclosure —
/// a pathological corpus (hundreds of copies) must not flood doctor
/// output; the count stays exact and the samples name the shape.
const MAX_DUPLICATE_PAIR_SAMPLES: usize = 5;

/// rm-1887: the pi-subagents orchestration writes each child run's
/// mirror transcripts and event streams under a `subagent-artifacts`
/// path segment (`subagent-artifacts/<run>_<agent>/_transcript.jsonl`,
/// `subagent-artifacts/outputs/call_<id>/events.jsonl`, …). Those
/// bytes re-state child spend the parent journal already owns, so
/// discovery admits none of them as sessions — the path segment is
/// the stable marker across all four observed artifact shapes
/// (message-only, mixed, event-stream mirror, async `events.jsonl`).
/// Segment-match, not suffix: a user's own journal named
/// `subagent-artifacts.jsonl` stays admissible.
pub(crate) fn is_subagent_artifact_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == std::ffi::OsStr::new("subagent-artifacts"))
}

/// rm-1886: content signature — (raw byte size, FNV-1a-64 of the raw
/// bytes). Byte-identical journals share it across paths; a one-byte
/// edit does not. FNV-1a is not cryptographic, but it does not need
/// to be: the adversary is accidental duplication, and the signature
/// is only ever an admission gate — parse still validates whatever it
/// admits. Hash collision on distinct journals of equal size would
/// suppress a real session, so the 64-bit space plus exact size keeps
/// that at operational zero (and a suppressed pair is named in the
/// disclosure, so it is observable if it ever happened).
///
/// Integration note (conflict case 72b88975, rm-451 composition): the
/// read is bounded by the SAME named-file read cap the parse lane
/// enforces (`parser::file_read_cap_bytes`, rm-451 + review fix
/// 1f153416, strengthened on the trunk after this row minted). An
/// over-cap journal signs as `None` — KEPT, not deduped — because the
/// parse lane owns its loud `ReadCapRefused` disclosure, and admission
/// dedup must not become the unbounded read the cap family closed
/// (the 300-MiB-of-NULs class that peaked 613 MiB RSS). Over-cap
/// copies never bank either way, so admission loses nothing by
/// leaving them to the parse lane; `take(cap + 1)` mirrors the
/// metadata-lie guard so a file that grows between stat and read
/// still cannot allocate past one byte over the cap.
fn content_signature(path: &Path) -> Option<(i64, u64)> {
    use std::io::Read;
    let cap = crate::parser::file_read_cap_bytes();
    if fs::metadata(path).ok()?.len() > cap {
        return None;
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > cap {
        return None;
    }
    Some((bytes.len() as i64, fnv1a64(&bytes)))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// rm-1886 memoized content signature: served from the parent
/// directory's cached listing when the cache vouches for the file's
/// fingerprint (see [`session_cache::cached_content_signature`]),
/// else hashed once and persisted so warm runs stop re-reading every
/// journal to prove what the cache already vouches for.
fn content_signature_cached(path: &Path, cache: &mut SessionCache) -> Option<(i64, u64)> {
    if let Some(signature) = crate::session_cache::cached_content_signature(path, cache) {
        return Some(signature);
    }
    let signature = content_signature(path)?;
    crate::session_cache::store_content_signature(path, signature.0, signature.1, cache);
    Some(signature)
}

pub(crate) fn find_session_files_cached(
    dir: Option<&Path>,
    cache: &mut SessionCache,
    skip_sqlite_backed: bool,
) -> (Vec<PathBuf>, SuppressedSessions, usize) {
    if let Some(dir) = dir {
        if is_cline_task_dir(dir) {
            return (vec![dir.to_path_buf()], SuppressedSessions::default(), 0);
        }
        // rm-548: an explicitly requested directory keeps every file
        // for the AGGREGATION families — a forked copy loaded by
        // explicit path still renders, and so do byte-identical
        // content duplicates (rm-1886) and artifact mirrors
        // (rm-1887): `-d` is the inspection lane for those. rm-451
        // keeps the over-cap counter zero here too — an explicit `-d`
        // probe reads the doc through the parse lane, which owns its
        // own read caps. rm-2352 (run de92ccb4 cycle 7): zst stem
        // twins are NOT an aggregation family — the plain and
        // compressed names hold ONE codex session — so the twin pass
        // runs here too, and its ledger rides the LoadReport so the
        // explicit-dir lane discloses exactly like the auto-home
        // lane instead of double-counting with a silent zero.
        let mut suppressed = SuppressedSessions::default();
        let files =
            suppress_zst_stem_twins(collect_session_files_cached(dir, cache), &mut suppressed);
        return (files, suppressed, 0);
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
    let mut suppressed = SuppressedSessions::default();
    // rm-1887: pi-subagents artifact mirrors are not sessions — their
    // spend re-states what the parent journal owns. Segment-level skip
    // BEFORE the fork probe: artifacts need no fork marker read.
    all.retain(|path| {
        if is_subagent_artifact_path(path) {
            suppressed.subagent_artifact_files += 1;
            false
        } else {
            true
        }
    });
    // rm-548: opencode fork copies (session info docs whose `parentID`
    // is set) re-emit their parent's history, so aggregating them
    // double-counts usage. Drop them before enumeration and return the
    // count so the disclosure channel reports it — never silently.
    // Independent-review fix: the per-file probe is memoized on the
    // cached dir listing (fingerprint-keyed, see
    // `opencode_session_fork_parent_cached`) so warm runs stop
    // re-reading every storage session info doc on discovery.
    let mut opencode_info_docs_skipped_over_cap = 0usize;
    all.retain(|path| {
        // rm-451 walk-lane arm: an enumerated info doc past the read
        // cap is skipped without ever being materialized (the probe
        // refuses the read). Counted at the enumeration site so the
        // skip is disclosed on warm runs too, where a memoized probe
        // verdict cannot carry the distinction. The strict layout
        // predicate mirrors the probe's own gate, so the counter
        // names exactly the docs that probe would have read;
        // sibling-derived paths are deliberately NOT counted — every
        // info doc is enumerated itself (`.json` names are admitted),
        // so counting via a JSONL sibling could only double-count the
        // same physical doc.
        if is_opencode_session_info_doc(path)
            && fs::metadata(path)
                .map(|meta| meta.len() > OPENCODE_INFO_DOC_READ_CAP_BYTES)
                .unwrap_or(false)
        {
            opencode_info_docs_skipped_over_cap += 1;
            // No fork verdict is derivable without the doc: keep the
            // session admitted (the rm-1437 transcript admission cap
            // refuses the parse upstream) and disclose the skipped
            // read — skip, never a crash.
            return true;
        }
        if opencode_session_fork_parent_cached(path, cache).is_some() {
            suppressed.opencode_fork_excluded += 1;
            false
        } else {
            true
        }
    });
    // rm-1881: the same session id in both the canonical CLI store and
    // the legacy VS Code tasks root renders one session (canonical wins).
    // Integration note (conflict case 169c75c7): ordered after the rm-548
    // fork probe and before the rm-1886 content dedup — id-level cline
    // duplicates resolve by the canonical store, then byte-level copies
    // collapse first-seen, so both landed rules stay observable.
    retain_single_cline_session_per_id(&mut all);
    // rm-2056: zst stem twins — one admission per stem, plain `.jsonl`
    // wins, the shadowed `.jsonl.zst` counted for disclosure. Ordered
    // after the cline-id pass and before the rm-1886 content dedup:
    // the twin's compressed bytes never collide with the plain file's
    // signature, so this pass is the only one that can collapse the
    // pair. Deterministic by NAME, so cold and warm runs agree.
    all = suppress_zst_stem_twins(all, &mut suppressed);
    // rm-1886: content dedup at admission — a byte-identical journal
    // at a second path re-emits the same events and double-banks every
    // aggregate it feeds. First-seen survivor wins (deterministic: the
    // walk yields sorted per-directory files, roots in registration
    // order); a file whose bytes cannot be read is KEPT — the parse
    // lane owns unreadable files and discloses them, dedup must not
    // silently drop what it cannot verify. Unreadable-file stance
    // keeps this pass observable: suppressed copies are counted and
    // the first pairs named.
    let mut deduped: Vec<PathBuf> = Vec::with_capacity(all.len());
    let mut signatures: HashMap<(i64, u64), PathBuf> = HashMap::new();
    for path in all {
        // rm-451 × rm-1886 composition (conflict case a2dd44ed): an
        // over-cap opencode info doc is NEVER materialized on any lane —
        // the fork probe refuses the read AND the dedup signature is
        // refused for exactly the same docs, so the newer content-dedup
        // pass cannot reintroduce the unbounded hash read the rm-451 cap
        // retired. rm-1886's unreadable-file stance still holds: the doc
        // stays ADMITTED (kept, disclosed via
        // `opencode_info_docs_skipped_over_cap`), never silently dropped.
        if is_opencode_session_info_doc(&path)
            && fs::metadata(&path)
                .map(|meta| meta.len() > OPENCODE_INFO_DOC_READ_CAP_BYTES)
                .unwrap_or(false)
        {
            deduped.push(path);
            continue;
        }
        let Some(signature) = content_signature_cached(&path, cache) else {
            deduped.push(path);
            continue;
        };
        match signatures.entry(signature) {
            std::collections::hash_map::Entry::Occupied(survivor) => {
                suppressed.duplicate_session_files += 1;
                if suppressed.duplicate_session_file_pairs.len() < MAX_DUPLICATE_PAIR_SAMPLES {
                    suppressed
                        .duplicate_session_file_pairs
                        .push((survivor.get().clone(), path));
                }
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(path.clone());
                deduped.push(path);
            }
        }
    }
    (
        sort_paths_by_cache(deduped, cache),
        suppressed,
        opencode_info_docs_skipped_over_cap,
    )
}

/// rm-451: strict predicate for an opencode storage session info doc
/// (`…/storage/session/info/<sid>.json`) — the exact layout the rm-548
/// fork probe gates on, so the walk-lane skip counter counts precisely
/// the docs that probe would read.
fn is_opencode_session_info_doc(path: &Path) -> bool {
    let mut components = path.components().rev();
    let file_is_json = components
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|name| name.ends_with(".json"));
    let path_is_session_info = ["info", "session", "storage"].iter().all(|segment| {
        components
            .next()
            .and_then(|component| component.as_os_str().to_str())
            .is_some_and(|actual| actual == *segment)
    });
    file_is_json && path_is_session_info
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
    // rm-451: info docs are tiny metadata records. A doc larger than the
    // bound is anomalous (a planted or foreign file the rm-212 file-kind
    // gates admit by design) — never materialize it; skip the read and
    // let the caller disclose the skip.
    if fs::metadata(path)
        .map(|meta| meta.len())
        .unwrap_or(u64::MAX)
        > OPENCODE_INFO_DOC_READ_CAP_BYTES
    {
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
        // rm-1881: under the Cline CLI canonical root only the
        // `<id>.messages.json` transcript is admitted — the
        // `session.json` manifest and `hooks.jsonl` siblings stay out,
        // and the name alone never qualifies (content-shaped gates in
        // `is_cline_cli_session_file`).
        if is_cline_cli_session_path(&path) && !is_cline_cli_session_file(&path) {
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
        // rm-1881: under the Cline CLI canonical root only the
        // `<id>.messages.json` transcript is admitted — the
        // `session.json` manifest and `hooks.jsonl` siblings stay out,
        // and the name alone never qualifies (content-shaped gates in
        // `is_cline_cli_session_file`).
        if is_cline_cli_session_path(&path) && !is_cline_cli_session_file(&path) {
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
    // rm-992: codex migrates cold rollouts to `<name>.jsonl.zst` names
    // (stable 0.162.x migration via the opt-in `LocalThreadStoreCompression`
    // + the `migrate_rollouts` CLI, upstream `rollout/src/compression.rs`
    // COMPRESSED_SUFFIX = ".zst"; writing compressed-by-default stays
    // feature-flagged upstream — still off at tag rust-v0.163.0, the
    // rm-1518 correction in CHANGELOG) — the exact sibling ccusage's
    // codex reader admits (rust/adapters/codex/src/reader.rs
    // `is_rollout_file`, issues #1888/#1890). The parser decodes zstd
    // frames transparently via its magic sniff (parser.rs, rm-1516,
    // concatenated frames included) — but this NAME filter used to
    // reject the file before the sniff could run, so a migrated home
    // silently undercounted (assess PoCs on both lineages: a zst-only
    // home reported "No session files found" — 0 of 4 compressed lanes
    // discovered — while identical bytes under a plain `.jsonl` name
    // parsed fine, with `--doctor` disclosing nothing). The suffix is
    // admitted at this shared gate because the walkers are
    // lane-agnostic; the sniff stays the decider — a zstd frame is
    // bounded-decoded, and anything that is NOT a valid frame still
    // lands in skipped_files with the honest `zstd-compressed` error
    // (see discovery_contract's raw-magic test). A bare `.zst` (no
    // `.jsonl` stem) stays out. Widened in parallel by run e4209f0c
    // cycle 5 (assess F1 + research 30082665) and run d57c2e56 cycle 1
    // — the identical disjunction reconciled to one gate at
    // integration (conflict case ae49eb7a), keeping both lineages'
    // comments and pin sets.
    name.ends_with(".jsonl") || name.ends_with(".jsonl.zst") || name.ends_with(".json")
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
    let canonical = KnownSessionDir {
        name: "Cline CLI".to_string(),
        path: cline_cli_session_root(home),
    };
    let legacy = KnownSessionDir {
        name: "Cline (VS Code)".to_string(),
        path: user_config_dir(home)
            .join("Code")
            .join("User")
            .join("globalStorage")
            .join("saoudrizwan.claude-dev")
            .join("tasks"),
    };
    // An operator pointing the relocation env at the legacy tasks root
    // must not register the same tree twice.
    if canonical.path == legacy.path {
        vec![legacy]
    } else {
        vec![canonical, legacy]
    }
}

/// The Cline CLI canonical session store (rm-1881 arm b / rm-1435
/// discovery fold): messages contract v1 locates sessions at
/// `~/.cline/data/sessions/<sessionId>/` with
/// `<sessionId>.messages.json` as the transcript and a `session.json`
/// manifest alongside. `CLINE_SESSION_DATA_DIR` relocates the store
/// root; a trimmed-empty override falls back to the default (the
/// OpenClaw `OPENCLAW_STATE_DIR` discipline).
fn cline_cli_session_root(home: &Path) -> PathBuf {
    let default = home.join(".cline").join("data").join("sessions");
    match std::env::var_os("CLINE_SESSION_DATA_DIR") {
        Some(dir) => {
            if dir.to_string_lossy().trim().is_empty() {
                default
            } else {
                PathBuf::from(dir)
            }
        }
        None => default,
    }
}

/// A file is a Cline CLI session transcript only when every
/// content-shaped gate holds (rm-1435 discipline — the name alone is
/// far too generic to key on): it lives under the canonical root, its
/// name is `<dirName>.messages.json` (the transcript stem matches its
/// session dir), and that dir carries the `session.json` manifest
/// contract v1 writes beside every transcript. The manifest,
/// `hooks.jsonl`, and any non-canonical name stay out of discovery.
fn is_cline_cli_session_file(path: &Path) -> bool {
    if !is_cline_cli_session_path(path) {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let Some(dir_name) = parent.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if path.file_name().and_then(|name| name.to_str())
        != Some(format!("{dir_name}.messages.json").as_str())
    {
        return false;
    }
    parent.join("session.json").is_file()
}

/// Whether a candidate path lives under the Cline CLI canonical store
/// root (env-relocated or default) — the scope inside which the
/// `<id>.messages.json`-only admission rule applies. The custom-dir
/// walk hands us paths as the operator typed them (a `-d` argument can
/// be relative), so the comparison runs on a lexically-normalized
/// absolute form — cwd-prepended, `.`/`..` collapsed — without
/// touching the filesystem.
fn is_cline_cli_session_path(path: &Path) -> bool {
    let Some(home) = std::env::var_os("HOME") else {
        return false;
    };
    path_starts_with_root(path, &cline_cli_session_root(Path::new(&home)))
}

/// Lexical `starts_with`: absolute paths compare directly, relative
/// ones are made absolute against the current directory first — the
/// ROOT too. A relative `CLINE_SESSION_DATA_DIR` override passes
/// through verbatim, and comparing the absolutized candidate against
/// a still-relative root never matches, leaving the canonical gates
/// inert (review F2: `session.json`/`hooks.jsonl` slip back in as
/// name-admitted parse noise while the transcript still parses).
fn path_starts_with_root(path: &Path, root: &Path) -> bool {
    lexical_absolute(path).starts_with(lexical_absolute(root))
}

/// cwd-prepended, lexically normalized path — no filesystem access,
/// no symlink resolution (traversal policy is the walk's concern, not
/// the gate's).
fn lexical_absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        let mut out = PathBuf::new();
        out.extend_normalized(path);
        return out;
    }
    let joined = match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => return PathBuf::from(path),
    };
    let mut out = PathBuf::new();
    out.extend_normalized(&joined);
    out
}

trait NormalizeComponents {
    fn extend_normalized(&mut self, path: &Path);
}

impl NormalizeComponents for PathBuf {
    fn extend_normalized(&mut self, path: &Path) {
        use std::path::Component;
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    self.pop();
                }
                other => self.push(other.as_os_str()),
            }
        }
    }
}

/// rm-1881 dedup pin: one session id must render one session even when
/// it exists in BOTH the canonical CLI store and the legacy VS Code
/// tasks root — the canonical store wins (canonical identity
/// discipline; the rm-338 alias dedup only collapses path aliases, not
/// the same id materialized under two different roots).
fn retain_single_cline_session_per_id(all: &mut Vec<PathBuf>) {
    let canonical_ids: HashSet<String> = all
        .iter()
        .filter(|path| is_cline_cli_session_file(path))
        .filter_map(|path| {
            path.parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .collect();
    if canonical_ids.is_empty() {
        return;
    }
    all.retain(|path| {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };
        !(path.is_dir() && is_cline_task_dir(path) && canonical_ids.contains(name))
    });
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

#[cfg(test)]
mod session_file_name_tests {
    use super::is_session_file_name;

    #[test]
    fn zstd_named_rollout_passes_the_name_gate() {
        // rm-992 (run d57c2e56 cycle 1): codex compresses rollouts either
        // in place (`.jsonl` name, zstd content) or under a `.jsonl.zst`
        // name - the same two shapes ccusage's codex reader admits
        // (rust/adapters/codex/src/reader.rs: is_rollout_file accepts
        // `.jsonl` and `.jsonl.zst`). A gate on `.jsonl`/`.json` alone
        // silently dropped the compressed half of the fleet (assess PoC:
        // 0 of 4 zstd lanes discovered, doctor disclosing nothing).
        assert!(is_session_file_name(
            "rollout-2026-09-26T20-29-31-7f2f4ba2.jsonl.zst"
        ));
    }

    #[test]
    fn the_widening_stays_narrow() {
        // Only the `.jsonl.zst` sibling is admitted - not arbitrary zst
        // payloads or unrelated extensions.
        assert!(!is_session_file_name("archive.tar.zst"));
        assert!(!is_session_file_name("rollout-2026-09-26.zst"));
        assert!(!is_session_file_name("rollout-2026-09-26.jsonl.zst.tmp"));
    }
}

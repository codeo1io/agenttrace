use crate::reports::guard_formula_cell;
use crate::sqlite_sessions::{
    load_sqlite_backed_sessions_reported_bounded, SqlitePartialDb, SqliteScanBudget,
};
use crate::{
    cache_dir::user_cache_dir, cached_session, known_session_dirs, load_session_cache, parse_file,
    store_session, Session, SqliteIngestReport, VERSION,
};
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// rm-367(b)/(d): top-level honest-truncation disclosure for the
/// doctor scan. Built only when a bound actually truncated work;
/// partial counts are never presented as complete ones.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DoctorScanPartial {
    pub roots: usize,
    pub roots_truncated: usize,
    pub session_files: usize,
    pub files_parsed: usize,
    pub files_skipped_budget: usize,
    pub files_skipped_size: usize,
    pub sqlite_dbs_partial: usize,
    pub elapsed_ms: u64,
    pub note: String,
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

thread_local! {
    /// rm-1554: unparsable doctor budget-knob values observed by
    /// `doctor_env_u64` during the current pass, drained into the
    /// report's disclosures before it leaves `build_doctor_report`.
    /// A typo'd knob previously ran the default SILENTLY — the
    /// operator asking for a tighter or looser bound got neither the
    /// value nor a warning, which the doctor's own
    /// AGENTTRACE_CACHE_DIR typo warning (:1319 neighborhood) already
    /// treats as a bug class.
    static DOCTOR_ENV_FALLBACKS: RefCell<Vec<(String, String)>> =
        const { RefCell::new(Vec::new()) };
}

fn doctor_env_u64(name: &str, default: u64) -> u64 {
    match std::env::var(name) {
        Ok(raw) => match raw.trim().parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                DOCTOR_ENV_FALLBACKS.with(|fallbacks| {
                    fallbacks.borrow_mut().push((name.to_string(), raw.clone()));
                });
                default
            }
        },
        Err(_) => default,
    }
}

/// rm-1554: fold the fallbacks recorded so far into the report's
/// disclosures. Keys carry the variable name and the raw value (run
/// through the control-byte sanitizer — the value is operator input,
/// the disclosure lane treats every key as hostile). Counts are
/// per-key: a knob read twice in one pass (the scan-partial note
/// re-reads the sqlite total) reports count 2, which is the truth.
fn drain_doctor_env_fallbacks(disclosures: &mut BTreeMap<String, usize>) {
    DOCTOR_ENV_FALLBACKS.with(|fallbacks| {
        for (name, raw) in std::mem::take(&mut *fallbacks.borrow_mut()) {
            let key = format!(
                "doctor_env_fallback:{}={}",
                name,
                crate::parser::capped_disclosure_value(&raw)
            );
            *disclosures.entry(key).or_insert(0) += 1;
        }
    });
}

/// rm-367(b)/(d): the per-root scan bound — a wall-clock budget for
/// fresh parses and a cumulative parsed-byte cap for oversized
/// roots. A root that crosses either bound keeps what it parsed and
/// discloses the rest as `skipped_budget` / `skipped_size` counts.
#[derive(Debug, Clone, Copy)]
struct DoctorRootBound {
    budget_ms: u64,
    max_bytes: u64,
}

impl DoctorRootBound {
    fn from_env() -> Self {
        Self {
            budget_ms: doctor_env_u64("AGENTTRACE_DOCTOR_ROOT_BUDGET_MS", 60_000),
            max_bytes: doctor_env_u64("AGENTTRACE_DOCTOR_ROOT_MAX_BYTES", 536_870_912),
        }
    }

    fn deadline(&self) -> Option<Instant> {
        Some(Instant::now() + Duration::from_millis(self.budget_ms))
    }

    fn deadline_exceeded(&self, deadline: Option<Instant>) -> bool {
        deadline.is_some_and(|bound| Instant::now() >= bound)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DoctorReport {
    pub version: String,
    pub mode: String,
    pub cache_path: String,
    pub cache_entries: usize,
    pub cache_dirs: usize,
    pub cached_valid: usize,
    /// rm-381 disclosure: outcomes of reversing `-`-encoded agent project
    /// directory names for scanned sessions without a `cwd`.
    pub project_decode: DoctorProjectDecodeReport,
    /// rm-408 disclosure: sessions whose transcripts REPORT all-zero
    /// usage blocks, and how many events carry them. Counted as measured
    /// zeros; the share is surfaced so a present-zero corpus never reads
    /// as clean.
    pub zero_usage: DoctorZeroUsageReport,
    /// rm-1534: sessions whose last activity sits beyond
    /// `now` + `insights::FUTURE_ADMISSION_GRACE_SECS` (24h), counted
    /// across both doctor lanes (walk-parsed files and the sqlite-backed
    /// corpus). The row keeps the doctor's inventory honest about
    /// ranged-view exclusions — `--range today` drops these sessions
    /// (counted + disclosed as `future_timestamp_sessions`), and the
    /// sessions themselves carry a `future_timestamp` anomaly wherever
    /// they still render. 0-count-clean: zero renders identically to
    /// the pre-rm-1534 report.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub future_timestamp_sessions: usize,
    /// Parse-time journal disclosures aggregated over scanned sessions
    /// (rm-436/rm-437, pi-family journals): `pi_usage_entry:<kind>`,
    /// `pi_branches`, `pi_entry_skipped:<type>`,
    /// `pi_message_role:<role>`, and the other INFORMATIONAL
    /// `disclosure_counters` (`kimi_usage_alias:…`,
    /// `duplicate_spelling_normalized`,
    /// `model_label_non_assistant_ignored`, …): journal facts the
    /// accounting deliberately does not count, kept visible. Since
    /// rm-862 this map no longer carries the `line_skips` channel —
    /// real parse loss lives in [`Self::dropped_lines`] and renders as
    /// its own "Dropped lines (parse loss)" row, so a reader can tell
    /// a loss from a healthy disclosure. Empty for corpora without
    /// such journals.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub disclosures: BTreeMap<String, usize>,
    /// rm-862: parse-loss counters (`metrics.line_skips`:
    /// `unparseable_line`, `non_object_line`, `event_schema`,
    /// `codex_ignorable_line`, …) aggregated over scanned sessions.
    /// The rm-526 fold landed them under `disclosures`, rendering
    /// next to informational counters under one "Disclosed facts"
    /// line — a doctor reader could not tell a real loss from a
    /// healthy disclosure. They now render as their own
    /// "Dropped lines (parse loss)" row, mirroring the overview's
    /// two-tier vocabulary (rm-862's doctor vocabulary map, see
    /// docs/guides/governance-reports.md).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub dropped_lines: BTreeMap<String, usize>,
    /// On-disk size of `sessions.json`, zero when absent.
    pub cache_size_bytes: u64,
    /// The bounds `save_session_cache` enforces before serializing:
    /// the EFFECTIVE entry bound (rm-298 capacity arm: the
    /// `session_cache_entries` config key / `AGENTTRACE_SESSION_CACHE_ENTRIES`
    /// env knob when set, else the built-in default) and the
    /// serialized-bytes cap (pass-9 CU-22).
    pub cache_limits: String,
    /// rm-298 capacity arm: the entry bound actually in force this
    /// run (`cache_limits` renders it as `entries<=N`); kept numeric
    /// so JSON consumers can compare it against `cache_entries`.
    pub cache_entry_bound: usize,
    /// rm-298 capacity arm: which layer supplied the entry bound —
    /// `"config file"`, `"env"`, or `"default"`.
    pub cache_entry_bound_source: String,
    /// rm-298 capacity arm: discovered session files this scan could
    /// NOT serve from the cache (no entry, or size/mtime stale) and
    /// therefore re-parses from source — the recurring CPU cost the
    /// assess run measured directly (2,768 of 25,889 reusable on the
    /// operator corpus). `session_files - cached_valid`.
    pub reparsed_this_scan: usize,
    /// rm-292 disclosure parity (review fix R3, run 2d92ee95): the
    /// journal's `schema_invalidations` ledger summarized for humans —
    /// how many invalidations the surviving journal records and the
    /// newest record verbatim (schema bumps and pricing-catalog
    /// changes alike). `None` when the journal has never recorded one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_invalidation_note: Option<String>,
    pub sessions: usize,
    pub session_files: usize,
    /// rm-607 disclosure: sibling sqlite databases (outside the
    /// canonical ones) whose sessions were suppressed as cross-file
    /// duplicates — backups and copies that used to double every
    /// aggregate before the dedup landed. Empty when no sibling db
    /// matched the discovery glob.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sqlite_duplicate_dbs: Vec<crate::sqlite_sessions::SqliteDuplicateDb>,
    /// rm-367(b)/(d): present exactly when the scan budget or the
    /// per-root size cap truncated something ('scanned N of M roots in
    /// Ts'); a complete scan leaves it `None` so output stays
    /// byte-identical to pre-budget binaries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_partial: Option<DoctorScanPartial>,
    /// rm-367(b): sqlite databases whose ingest pass was cut short by
    /// the per-db/lane wall-clock budget (kept sessions disclosed).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sqlite_partial_dbs: Vec<SqlitePartialDb>,
    /// rm-1886 (run 77ca5de0 cycle 5, "count-honesty ledger"): first
    /// content-duplicate pairs suppressed at admission, rendered
    /// `survivor <- suppressed copy` — capped at five, the exact count
    /// rides `disclosures["duplicate_session_files"]`. Empty when no
    /// byte-identical copies were found.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub duplicate_session_file_pairs: Vec<String>,
    pub directories: Vec<DoctorDirReport>,
    /// Statusline capture journal (candidate 53, cycle 7): present when
    /// `agenttrace statusline` has been configured as the statusLine
    /// command; the journal is the only local source of subscription
    /// limit pressure and upstream prompt-cache analytics.
    pub statusline: DoctorStatuslineReport,
    /// Offline pricing catalog provenance (cycle 7 R1): the bundled
    /// snapshot's date, model count, and age, so reports can disclose
    /// how current the prices behind `cost_estimated` are.
    pub pricing: String,
    /// rm-1978: present exactly when the bundled demo corpus failed to
    /// parse under `--doctor --demo` (build-time content corruption);
    /// the report then renders zero sessions and names the error in a
    /// recommendation instead of panicking. `None` on every valid
    /// corpus, so report bytes are unchanged from pre-rm-1978
    /// binaries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_corpus_error: Option<String>,
    /// rm-734 riding the landed rm-753 ingest report: sqlite session
    /// databases that exist but could not be read — the doctor's job
    /// is to name them (path + lane + reason) with a repair
    /// recommendation. Empty (omitted from JSON) when every present
    /// database reads cleanly.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sqlite_read_failures: Vec<crate::sqlite_sessions::SqliteUnreadableDb>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DoctorStatuslineReport {
    pub path: String,
    pub exists: bool,
    pub captures: usize,
    pub sessions: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DoctorDirReport {
    pub name: String,
    pub path: String,
    pub exists: bool,
    pub files: usize,
    pub parsed: usize,
    pub failed: usize,
    /// rm-451: the read-cap-refused share of `failed` — files past
    /// [`crate::FILE_READ_CAP_BYTES`] (knob
    /// `AGENTTRACE_FILE_CAP_BYTES`), refused before the read.
    pub read_cap_refused: usize,
    pub cache_hits: usize,
    /// rm-367(b): files this root skipped because its wall-clock
    /// budget was exhausted — counted, never labeled as failures.
    #[serde(skip_serializing_if = "is_zero_usize")]
    pub skipped_budget: usize,
    /// rm-367(d): files this root skipped because the cumulative
    /// parsed bytes crossed the size cap (count-sampled pass).
    #[serde(skip_serializing_if = "is_zero_usize")]
    pub skipped_size: usize,
    /// Present when the provider directory is itself a symlink, so the
    /// doctor names the link it follows instead of implying a plain
    /// directory (cycle 7, Codex `#42135`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symlink_target: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failure_samples: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct SessionCacheReport {
    path: PathBuf,
    entries: BTreeMap<String, CacheEntryHeader>,
    dirs: usize,
    invalidation_note: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct CacheEntryHeader {
    mod_time: i64,
    size: i64,
}

pub fn render_doctor_report(
    dir: Option<&Path>,
    demo: bool,
    format: &str,
) -> anyhow::Result<String> {
    let report = build_doctor_report(dir, demo);
    if format == "json" {
        Ok(serde_json::to_string_pretty(&report)? + "\n")
    } else {
        Ok(doctor_report_text(&report))
    }
}

/// Discovery inputs for [`build_doctor_report`] after the rm-596 demo
/// gate: the session-cache report, its on-disk size, the walked session
/// files, the non-file sessions (SQLite-backed, or the bundled demo
/// corpus under `--demo`), and the rm-753 SQLite ingest report (empty
/// under `--demo` and on custom directories, where no SQLite lane runs).
/// rm-548: the JSON storage lane's opencode fork-exclusion count (fork
/// copies dropped from the auto-discovery inventory) rides here too,
/// so the doctor discloses the same exclusion the loader does.
/// rm-1887/rm-1886 (run 77ca5de0 cycle 5, "count-honesty ledger"):
/// the aggregation admission classes the loader suppressed ride here
/// too — artifact mirrors skipped, content duplicates deduped — so
/// doctor and loader agree on what the corpus excludes.
struct DoctorDiscovery {
    cache: SessionCacheReport,
    cache_size_bytes: u64,
    files: Vec<PathBuf>,
    sessions: Vec<Session>,
    sqlite_ingest: SqliteIngestReport,
    opencode_fork_excluded: usize,
    subagent_artifact_files: usize,
    duplicate_session_files: usize,
    duplicate_session_file_pairs: Vec<(PathBuf, PathBuf)>,
    /// rm-2126 (run 82a64f5a cycle 2): directories the walk's depth
    /// bound refused whose peek still found eligible content — the
    /// doctor's `-d` arm was a proven silent lane for these.
    depth_clipped_dirs: usize,
    /// rm-2127: directories whose read_dir/stat failed for a reason
    /// other than NotFound (EACCES subtree, EIO).
    unreadable_dirs: usize,
    /// rm-1888 arm: same-inode alias files suppressed at walk
    /// admission (hardlink/symlink aliases; distinct from the
    /// content-duplicate counter above).
    same_inode_files_suppressed: usize,
    opencode_info_docs_skipped_over_cap: usize,
    /// rm-1978: set exactly when the bundled demo corpus failed to
    /// parse under `--doctor --demo` (build-time content corruption,
    /// never operator input) — the report then carries zero sessions
    /// and names the error instead of panicking mid-diagnosis.
    demo_corpus_error: Option<String>,
}

/// rm-596: the single demo gate for the doctor report's discovery lanes.
/// `--doctor --demo` used to gate only the SQLite lane, so the file
/// discovery lane walked the operator's REAL corpus while the report was
/// labeled "demo sessions" — real session counts, real project paths,
/// and a full home walk disclosed under a demo label. Under `demo` every
/// host-derived lane now short-circuits here: the file lane walks
/// nothing, the session lane carries the bundled [`crate::demo_sessions`]
/// corpus, and the session-cache lane reports its path with zeroed
/// counts (the shape [`doctor_statusline_report`]'s demo arm
/// established), so a demo report reads no operator state beyond path
/// strings.
fn doctor_discovery(dir: Option<&Path>, demo: bool) -> DoctorDiscovery {
    if demo {
        // rm-1978: a demo-corpus parse failure is a broken build-time
        // invariant, disclosed in the report (`demo_corpus_error` + a
        // recommendation) with zero sessions rendered — never a panic
        // mid-diagnosis, never a silent empty report. Failure-mode PoC:
        // a parser refactor that breaks the bundled fixtures turns
        // `--doctor --demo` from a crash into a visible, named error
        // row; nothing changes on a valid corpus.
        let (sessions, demo_corpus_error) = match crate::demo_sessions() {
            Ok(sessions) => (sessions, None),
            Err(error) => (Vec::new(), Some(format!("{error:#}"))),
        };
        return DoctorDiscovery {
            cache: SessionCacheReport {
                path: session_cache_path(),
                entries: BTreeMap::new(),
                dirs: 0,
                invalidation_note: None,
            },
            cache_size_bytes: 0,
            files: Vec::new(),
            sessions,
            sqlite_ingest: SqliteIngestReport::default(),
            opencode_fork_excluded: 0,
            subagent_artifact_files: 0,
            duplicate_session_files: 0,
            duplicate_session_file_pairs: Vec::new(),
            depth_clipped_dirs: 0,
            unreadable_dirs: 0,
            same_inode_files_suppressed: 0,
            opencode_info_docs_skipped_over_cap: 0,
            demo_corpus_error,
        };
    }
    let cache = load_session_cache_report();
    let cache_size_bytes = std::fs::metadata(&cache.path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    // rm-2126/rm-2127/rm-1888 arm (run 82a64f5a cycle 2, "nothing
    // read-side is silent"): BOTH doctor arms carry the walk
    // disclosure now — the explicit `-d` arm was one of the assess
    // PoCs' silent lanes (a rollout nested past the depth bound and a
    // mode-000 subdir each vanished with zero output), so it must not
    // be the lane that still hides them.
    let (files, suppressed, opencode_info_docs_skipped_over_cap, walk) = if dir.is_none() {
        find_reportable_session_files(None)
    } else {
        let (files, walk) = crate::discovery::find_session_files_reported(dir);
        (
            files,
            crate::discovery::SuppressedSessions::default(),
            0,
            walk,
        )
    };
    // rm-753 (minted campaign-locally as rm-596; rebind recorded in
    // the ROADMAP): the SQLite-backed load returns its per-file failure
    // report alongside the sessions so doctor counters stop printing
    // parsed=0 failed=0 over discovered-but-unreadable databases
    // (P12/P14) and dropped session rows surface as failures. The demo
    // arm early-returned above, so this lane only runs on the real
    // discovery path. rm-548: the ingest report carries the opencode
    // fork-exclusion count as well (the legacy count-dropping wrapper
    // was deleted at the campaign's review fix), so the doctor
    // inventory discloses the exclusion like every other lane.
    // rm-607 rider: the same ingest report carries the sibling
    // databases suppressed as cross-file duplicates, so doctor
    // disclosures the dedup through one struct instead of a second
    // load lane.
    let (sessions, sqlite_ingest) = if dir.is_none() {
        // rm-367(b): the sqlite lane runs under the doctor's
        // wall-clock budget (per-db + lane total, interrupt-based)
        // with live stderr progress; truncated databases are
        // disclosed as partial passes, never dropped or cached.
        let budget = SqliteScanBudget {
            per_db: Duration::from_millis(doctor_env_u64("AGENTTRACE_DOCTOR_DB_BUDGET_MS", 25_000)),
            total: Duration::from_millis(doctor_env_u64(
                "AGENTTRACE_DOCTOR_SQLITE_TOTAL_BUDGET_MS",
                90_000,
            )),
            // rm-367(b): per-database progress shares the same
            // terminal gate as the root-lane line — a captured stderr
            // stays JSON/disclosure-only.
            progress: std::io::stderr().is_terminal(),
        };
        load_sqlite_backed_sessions_reported_bounded(None, Some(budget))
    } else {
        (Vec::new(), SqliteIngestReport::default())
    };
    DoctorDiscovery {
        cache,
        cache_size_bytes,
        files,
        sessions,
        sqlite_ingest,
        opencode_fork_excluded: suppressed.opencode_fork_excluded,
        subagent_artifact_files: suppressed.subagent_artifact_files,
        duplicate_session_files: suppressed.duplicate_session_files,
        duplicate_session_file_pairs: suppressed.duplicate_session_file_pairs,
        depth_clipped_dirs: walk.depth_clipped_dirs,
        unreadable_dirs: walk.unreadable_dirs,
        same_inode_files_suppressed: walk.same_inode_files_suppressed,
        opencode_info_docs_skipped_over_cap,
        demo_corpus_error: None,
    }
}

/// rm-1534: `skip_serializing_if` arm for the future-timestamp
/// census — zero stays out of the JSON so clean corpora render
/// byte-identically to the pre-rm-1534 report.
fn is_zero(value: &usize) -> bool {
    *value == 0
}

pub fn build_doctor_report(dir: Option<&Path>, demo: bool) -> DoctorReport {
    // rm-367(b): the scan disclosure's clock covers the whole pass,
    // discovery included (the sqlite lane runs inside discovery).
    let started = Instant::now();
    // rm-1554: clear any fallback left behind by an aborted prior
    // build so one pass never discloses another pass's knobs.
    DOCTOR_ENV_FALLBACKS.with(|fallbacks| fallbacks.borrow_mut().clear());
    let DoctorDiscovery {
        cache,
        cache_size_bytes,
        files,
        sessions,
        sqlite_ingest,
        opencode_fork_excluded,
        subagent_artifact_files,
        duplicate_session_files,
        duplicate_session_file_pairs,
        depth_clipped_dirs,
        unreadable_dirs,
        same_inode_files_suppressed,
        opencode_info_docs_skipped_over_cap,
        demo_corpus_error,
    } = doctor_discovery(dir, demo);
    let cached_valid = valid_cached_session_count(&files, &cache);
    // rm-298 capacity arm: disclose the bound actually in force (and
    // its layer) plus the re-parse cost this scan paid — the honest
    // numbers the entry-bound knob exists to tune.
    let entry_bound = crate::session_cache::effective_session_cache_entries();
    let entry_bound_source = crate::session_cache::session_cache_entries_source();
    let reparsed_this_scan = files.len().saturating_sub(cached_valid);
    let mode = if demo {
        "demo sessions"
    } else if dir.is_some() {
        "custom directory"
    } else {
        "auto-discovery"
    };
    // rm-862: parse loss aggregates separately from informational
    // disclosures — see the `dropped_lines` field; the scan state
    // carries the mutable reports through the directory walk.
    let mut scan = DoctorScanState::default();
    let directories = doctor_directories(dir, demo, &files, &sessions, &sqlite_ingest, &mut scan);
    let DoctorScanState {
        mut project_decode,
        zero_usage,
        future_timestamp_sessions,
        disclosure_fold,
        // scanned_sessions is consumed inside doctor_directories by the
        // rm-904 linkage pass — nothing here needs the corpus again.
        ..
    } = scan;
    let DoctorDisclosureFold {
        mut disclosures,
        dropped_lines,
    } = disclosure_fold;
    // rm-512: the walk records per-project tallies; render them into the
    // deduplicated sample lines before the report leaves this function.
    project_decode.finalize_samples();
    let opencode_fork_excluded_total = opencode_fork_excluded + sqlite_ingest.fork_excluded;
    if opencode_fork_excluded_total > 0 {
        // Same disclosure key the loader/overview lanes use, so
        // `--doctor` and `--overview` agree on the exclusion instead
        // of the doctor silently under-counting.
        disclosures.insert(
            "opencode_fork_excluded_sessions".to_string(),
            opencode_fork_excluded_total,
        );
    }
    // rm-1887: same-key discipline for the artifact-mirror skip — the
    // loader's LoadReport counter and the doctor's disclosure must
    // agree, so the spend reads "skipped, not zero" on both surfaces.
    if subagent_artifact_files > 0 {
        disclosures.insert(
            "subagent_artifact_files".to_string(),
            subagent_artifact_files,
        );
    }
    // rm-1886: content-duplicate suppression, same-key discipline; the
    // named pairs ride their own field (capped at five, the count stays
    // exact) so the disclosure stays a counter map.
    if duplicate_session_files > 0 {
        disclosures.insert(
            "duplicate_session_files".to_string(),
            duplicate_session_files,
        );
    }
    // rm-451 walk-lane arm: the over-cap info-doc skip is disclosed with
    // its own key so a planted or bloated corpus is visible in `--doctor`,
    // exactly like the fork exclusion above — never a silent skip.
    if opencode_info_docs_skipped_over_cap > 0 {
        disclosures.insert(
            "opencode_info_docs_skipped_over_cap".to_string(),
            opencode_info_docs_skipped_over_cap,
        );
    }
    // rm-2126/rm-2127/rm-1888 arm (run 82a64f5a cycle 2, "nothing
    // read-side is silent"): the walk's own suppression classes —
    // depth-bound clips that lost content, unreadable subtrees,
    // same-inode aliases — were the last counters that never reached
    // this map. Same-key discipline with the loader's LoadReport (the
    // --audit JSON attaches the identical keys), so all three
    // surfaces agree.
    if depth_clipped_dirs > 0 {
        disclosures.insert("depth_clipped_dirs".to_string(), depth_clipped_dirs);
    }
    if unreadable_dirs > 0 {
        disclosures.insert("unreadable_dirs".to_string(), unreadable_dirs);
    }
    if same_inode_files_suppressed > 0 {
        disclosures.insert(
            "same_inode_files_suppressed".to_string(),
            same_inode_files_suppressed,
        );
    }
    let mut report = DoctorReport {
        version: VERSION.to_string(),
        mode: mode.to_string(),
        cache_path: cache.path.to_string_lossy().to_string(),
        cache_entries: cache.entries.len(),
        cache_dirs: cache.dirs,
        cached_valid,
        cache_size_bytes,
        cache_limits: format!(
            "entries<={}, bytes<={}",
            entry_bound,
            crate::session_cache::MAX_SESSION_CACHE_BYTES
        ),
        cache_entry_bound: entry_bound,
        cache_entry_bound_source: entry_bound_source.to_string(),
        reparsed_this_scan,
        cache_invalidation_note: cache.invalidation_note.clone(),
        sessions: files.len() + sessions.len(),
        session_files: files.len(),
        sqlite_duplicate_dbs: sqlite_ingest.duplicate_dbs,
        scan_partial: {
            // rm-367(b)/(d): honest-truncation disclosure — present
            // exactly when a bound truncated work, so a complete scan
            // serializes byte-identically to pre-budget binaries.
            let files_skipped_budget: usize =
                directories.iter().map(|row| row.skipped_budget).sum();
            let files_skipped_size: usize = directories.iter().map(|row| row.skipped_size).sum();
            let roots_truncated = directories
                .iter()
                .filter(|row| row.skipped_budget + row.skipped_size > 0)
                .count();
            let sqlite_dbs_partial = sqlite_ingest.partial_dbs.len();
            if files_skipped_budget + files_skipped_size + sqlite_dbs_partial == 0 {
                None
            } else {
                let bound = DoctorRootBound::from_env();
                Some(DoctorScanPartial {
                    roots: directories.len(),
                    roots_truncated,
                    session_files: directories.iter().map(|row| row.files).sum(),
                    files_parsed: directories.iter().map(|row| row.parsed).sum(),
                    files_skipped_budget,
                    files_skipped_size,
                    sqlite_dbs_partial,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    note: format!(
                        "scan budget reached: root wall-clock {}s, per-root size cap {} MB, sqlite lane {}s — skipped files and truncated database passes are disclosed per row; these counts are partial, not complete",
                        bound.budget_ms / 1000,
                        bound.max_bytes / (1024 * 1024),
                        doctor_env_u64("AGENTTRACE_DOCTOR_SQLITE_TOTAL_BUDGET_MS", 90_000) / 1000
                    ),
                })
            }
        },
        sqlite_partial_dbs: sqlite_ingest.partial_dbs.clone(),
        duplicate_session_file_pairs: duplicate_session_file_pairs
            .iter()
            .take(5)
            .map(|(survivor, suppressed)| {
                format!("{} <- {}", survivor.display(), suppressed.display())
            })
            .collect(),
        project_decode,
        zero_usage,
        future_timestamp_sessions,
        disclosures,
        dropped_lines,
        directories,
        statusline: doctor_statusline_report(demo),
        pricing: format!(
            "LiteLLM snapshot {} (bundled, {} models, {})",
            crate::pricing::bundled_snapshot_date(),
            crate::pricing::bundled_snapshot_model_count(),
            snapshot_age_phrase(crate::pricing::bundled_snapshot_age_days().unwrap_or(-1))
        ) + &doctor_deprecation_suffix()
            // rm-1978: when the embedded models.dev extract failed to
            // parse, the pricing provenance names the degradation —
            // extract-only models then miss through the ordinary
            // unpriced-model disclosure (absence, never a wrong
            // number), and the doctor states why.
            + &crate::pricing::models_dev_parse_failure()
                .map(|error| {
                    format!(
                        "; models.dev extract FAILED TO PARSE (rm-1978 \
                         disclosure — extract rows unserved): {error}"
                    )
                })
                .unwrap_or_default(),
        demo_corpus_error,
        sqlite_read_failures: sqlite_ingest.unreadable,
        recommendations: Vec::new(),
    };
    report.recommendations = doctor_recommendations(&report, dir, demo);
    // rm-1978: the demo-corpus failure rides the recommendations lane
    // too, so both the JSON field and the human-readable report name
    // the broken build-time invariant.
    if let Some(error) = &report.demo_corpus_error {
        report.recommendations.push(format!(
            "bundled demo corpus failed to parse — build-time content \
             corruption (rm-1978 disclosure): {error}; rebuild the binary"
        ));
    }
    // rm-1554: drained last so every knob read inside this build is
    // covered (root bound, sqlite budget, and the scan-partial
    // note's re-read of the sqlite total).
    drain_doctor_env_fallbacks(&mut report.disclosures);
    report
}

fn collect_zero_usage(out: &mut DoctorZeroUsageReport, session: &Session) {
    let events = session.metrics.zero_usage_events;
    if events == 0 {
        return;
    }
    out.sessions += 1;
    out.events += events;
    if out.samples.len() < 6 {
        out.samples.push(format!(
            "{}: {} all-zero usage block(s) counted as measured",
            crate::statusline::sanitize_line_segment(&session.name),
            events
        ));
    }
}

/// Aggregate of [`crate::insights::project_decode_status`] over the scanned
/// corpus (rm-381): how many `-`-encoded project directories resolved
/// cleanly, how many were attributed by the deterministic longest-run rule
/// with a shadowed alternative, and how many fell back to "unknown".
/// `--doctor` prints this so host-state-sensitive attribution is visible
/// instead of silent.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DoctorProjectDecodeReport {
    pub resolved: usize,
    pub ambiguous: usize,
    pub unresolved: usize,
    /// One representative sample per distinct project, in first-seen
    /// order, at most six (rm-512): repeated projects collapse into a
    /// single line carrying their session count instead of crowding the
    /// budget, so distinct projects fill the sample slots.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<String>,
    /// Per-project sample tallies accumulated during the scan — path →
    /// (session count, sanitized shadowed-alternatives list) for
    /// ambiguous projects, encoded dir → session count for unresolved
    /// ones. Rendered into [`Self::samples`] by
    /// [`Self::finalize_samples`] before the report is serialized; kept
    /// out of the JSON surface.
    #[serde(skip)]
    ambiguous_tally: BTreeMap<String, (usize, String)>,
    #[serde(skip)]
    unresolved_tally: BTreeMap<String, usize>,
    /// rm-240 resolved-decode notes composed into the same rm-512 tally
    /// at integration 2026-10-07 (conflict case d9ddc6a9): identity key
    /// (`prefix#hash` for truncated names, the bare name for opaque
    /// ones) → (session count, pre-rendered sample line).
    #[serde(skip)]
    resolved_tally: BTreeMap<String, (usize, String)>,
    /// First-seen order of distinct sample projects, tagged with the
    /// tally each entry renders from.
    #[serde(skip)]
    sample_order: Vec<(SampleChannel, String)>,
}

/// Which tally a `sample_order` entry renders from: the landed rm-512
/// pair plus the rm-240 resolved-decode notes composed in at
/// integration 2026-10-07 (conflict case d9ddc6a9).
#[derive(Clone, Copy, Debug)]
enum SampleChannel {
    Ambiguous,
    Unresolved,
    Resolved,
}

impl DoctorProjectDecodeReport {
    fn record_ambiguous(&mut self, path: &str, shadowed: String) {
        match self.ambiguous_tally.entry(path.to_string()) {
            Entry::Occupied(mut entry) => entry.get_mut().0 += 1,
            Entry::Vacant(vacant) => {
                vacant.insert((1, shadowed));
                self.sample_order
                    .push((SampleChannel::Ambiguous, path.to_string()));
            }
        }
    }

    fn record_unresolved(&mut self, encoded: &str) {
        match self.unresolved_tally.entry(encoded.to_string()) {
            Entry::Occupied(mut entry) => *entry.get_mut() += 1,
            Entry::Vacant(vacant) => {
                vacant.insert(1);
                self.sample_order
                    .push((SampleChannel::Unresolved, encoded.to_string()));
            }
        }
    }

    /// rm-240 resolved-decode note (truncated / opaque name): counted as
    /// resolved, with one deduplicated sample line per distinct identity
    /// carrying the session count — composed into the rm-512 tally
    /// semantics at integration 2026-10-07 (conflict case d9ddc6a9) so
    /// repeated names cannot crowd the six-slot budget and nothing lands
    /// on the JSON surface before `finalize_samples`.
    fn record_resolved(&mut self, key: &str, line: String) {
        match self.resolved_tally.entry(key.to_string()) {
            Entry::Occupied(mut entry) => entry.get_mut().0 += 1,
            Entry::Vacant(vacant) => {
                vacant.insert((1, line));
                self.sample_order
                    .push((SampleChannel::Resolved, key.to_string()));
            }
        }
    }

    /// Render the per-project tallies into `samples` (rm-512): one
    /// representative line per distinct project — ambiguous and
    /// unresolved interleaved in first-seen order — with the project's
    /// session count appended when it repeated, and distinct projects
    /// preferred inside the six-slot budget.
    pub fn finalize_samples(&mut self) {
        let mut samples = Vec::new();
        for (channel, project) in &self.sample_order {
            if samples.len() >= 6 {
                break;
            }
            let (line, count) = match channel {
                SampleChannel::Ambiguous => {
                    let Some((count, shadowed)) = self.ambiguous_tally.get(project) else {
                        continue;
                    };
                    (
                        format!(
                            "ambiguous: {} attributed via the longest-verified-path rule; \
                             verified alternatives shadowed: {}",
                            crate::statusline::sanitize_line_segment(project),
                            shadowed
                        ),
                        *count,
                    )
                }
                SampleChannel::Unresolved => {
                    let Some(count) = self.unresolved_tally.get(project) else {
                        continue;
                    };
                    (
                        format!(
                            "unresolved: projects/{} has no verified decode; \
                             session attributed to `unknown`",
                            crate::statusline::sanitize_line_segment(project)
                        ),
                        *count,
                    )
                }
                // rm-240: the line is pre-rendered at mint (prefix, hash
                // and name are already baked in); only the session count
                // is appended here.
                SampleChannel::Resolved => {
                    let Some((count, line)) = self.resolved_tally.get(project) else {
                        continue;
                    };
                    (line.clone(), *count)
                }
            };
            let mut line = line;
            if count > 1 {
                line.push_str(&format!(" ({count} sessions)"));
            }
            samples.push(line);
        }
        self.samples = samples;
        self.ambiguous_tally.clear();
        self.unresolved_tally.clear();
        self.resolved_tally.clear();
        self.sample_order.clear();
    }
}

/// rm-408 disclosure aggregated over the scanned corpus: sessions whose
/// transcripts carry client-REPORTED all-zero usage blocks, and the event
/// count. Counted as measured zeros — surfaced so the share is visible
/// instead of reading as clean usage.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DoctorZeroUsageReport {
    pub sessions: usize,
    pub events: usize,
    /// Up to six sanitized `session: N block(s)` examples in scan order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<String>,
}

fn collect_project_decode(out: &mut DoctorProjectDecodeReport, session: &Session) {
    use crate::insights::ProjectDecodeStatus;
    match crate::insights::project_decode_status(session) {
        ProjectDecodeStatus::NotConsulted => {}
        ProjectDecodeStatus::Resolved { .. } => out.resolved += 1,
        ProjectDecodeStatus::Ambiguous { path, shadowed } => {
            out.ambiguous += 1;
            let shadowed = shadowed
                .iter()
                .map(|alt| crate::statusline::sanitize_line_segment(alt))
                .collect::<Vec<_>>()
                .join(", ");
            out.record_ambiguous(&path, shadowed);
        }
        ProjectDecodeStatus::Unresolved { encoded } => {
            out.unresolved += 1;
            out.record_unresolved(&encoded);
        }
        // rm-240 arms: both yield a stable identity, so they count as
        // resolved — with a sample line stating exactly what happened.
        // Composed into the rm-512 tally at integration 2026-10-07
        // (conflict case d9ddc6a9): the note flows through
        // `record_resolved` (dedup by identity, session-count suffix,
        // six-slot budget at finalize) instead of pushing `samples`
        // directly, keeping the tally off the JSON surface until
        // `finalize_samples` runs.
        ProjectDecodeStatus::Truncated { prefix, hash } => {
            out.resolved += 1;
            out.record_resolved(
                &format!("{prefix}#{hash}"),
                format!(
                    "truncated: >200-char encoded name decoded to its verified prefix {}#{}; the hash tail is opaque but stable",
                    crate::statusline::sanitize_line_segment(&prefix),
                    crate::statusline::sanitize_line_segment(&hash)
                ),
            );
        }
        ProjectDecodeStatus::Opaque { name } => {
            out.resolved += 1;
            out.record_resolved(
                &name,
                format!(
                    "opaque: projects/{} kept as its own stable identity (CLAUDE_CODE_PROJECT_DIR_NAME-style names encode no path)",
                    crate::statusline::sanitize_line_segment(&name)
                ),
            );
        }
    }
}

/// Snapshot age with singular/plural grammar (rm-512): the bundled
/// snapshot is date-stamped, so "1 day old" — not "1 days old" — is the
/// literal fresh-release reading. The `-1` sentinel (unknown age)
/// keeps its pre-existing rendering.
fn snapshot_age_phrase(age_days: i64) -> String {
    if age_days == 1 {
        "1 day old".to_string()
    } else {
        format!("{age_days} days old")
    }
}

/// Deprecation disclosure for the doctor pricing line (rm-419):
/// `N` bundled models are past their vendor deprecation date, anchored
/// to the snapshot's own date so the count is a property of the
/// vendored file (deterministic across runs), never of when the
/// binary executes.
fn doctor_deprecation_suffix() -> String {
    let (count, oldest) = crate::pricing::bundled_snapshot_deprecated();
    if count == 0 {
        return String::new();
    }
    match oldest {
        Some(oldest) => {
            format!("; {count} priced models past vendor deprecation (oldest {oldest})")
        }
        None => format!("; {count} priced models past vendor deprecation"),
    }
}

fn doctor_statusline_report(demo: bool) -> DoctorStatuslineReport {
    let path = crate::statusline::statusline_capture_path();
    if demo {
        return DoctorStatuslineReport {
            path: path.to_string_lossy().to_string(),
            exists: false,
            captures: 0,
            sessions: 0,
            bytes: 0,
        };
    }
    let stats = crate::statusline::statusline_journal_stats(&path);
    let captures = crate::statusline::read_statusline_captures(&path);
    let sessions = crate::statusline::statusline_insights(&captures).sessions;
    DoctorStatuslineReport {
        path: stats.path,
        exists: stats.exists,
        captures: stats.lines,
        sessions,
        bytes: stats.bytes,
    }
}

fn find_reportable_session_files(
    dir: Option<&Path>,
) -> (
    Vec<PathBuf>,
    crate::discovery::SuppressedSessions,
    usize,
    crate::discovery::WalkDisclosure,
) {
    if dir.is_some() {
        let (files, walk) = crate::discovery::find_session_files_reported(dir);
        return (
            files,
            crate::discovery::SuppressedSessions::default(),
            0,
            walk,
        );
    }
    // rm-548 (independent-review fix): the doctor's auto-discovery
    // inventory aggregates through the SAME loader primitive the CLI
    // uses (`find_session_files_cached`), not a private walk — an
    // opencode fork copy re-emits its parent's history, so counting
    // both inflates `sessions`/`session_files`, and a private walk
    // silently diverges from the loader (different dedup, different
    // fork filter). Sharing the primitive means the doctor also
    // rides the session-cache memo (fingerprint-keyed fork-probe
    // results; a rewritten doc always re-probes) instead of
    // re-reading every storage session info doc, and the exclusion
    // rides the disclosure channel instead of dying silently. An
    // explicit `--dir` keeps every file (the exclusion is
    // aggregation-only, same rule as the loader).
    let mut cache = crate::session_cache::load_session_cache();
    let (files, suppressed, opencode_info_docs_skipped_over_cap, walk) =
        crate::discovery::find_session_files_cached(None, &mut cache, true);
    // rm-367 arm (run 82a64f5a cycle 2): a cache write failure must
    // still never fail the doctor (the cache is an optimization; the
    // inventory answer is already correct) — but it is no longer
    // silent either: one stderr note names the path, so a read-only
    // cache dir stops looking like a mysteriously slow cold start.
    crate::session_cache::persist_session_cache_disclosed(&mut cache);
    (files, suppressed, opencode_info_docs_skipped_over_cap, walk)
}

// rm-753 × rm-596 composition: the landed demo gate and the sqlite
// ingest report each added a parameter to a function that already carried
// the decode/zero-usage/disclosure accumulators — bundled into a struct it
// would mirror `DoctorInputs` one floor up, so the fleet-precedent allow
// (discovery.rs `report_load_progress`, governance.rs `recommendation`)
// was the smaller honest surface. The rm-862 `DoctorScanState` bundling
// later took `doctor_directories` back to 7 parameters, so the allow went
// inert on the struct and the cycle-1 review fix (b0446d6c F5) removed
// it — the headroom is now real, not papered over.
/// rm-862: the two doctor aggregate channels travel together —
/// informational `disclosures` and loss-tier `dropped_lines` — so the
/// fold helpers stay under clippy's `too_many_arguments` budget (the
/// same bundling precedent rm-538 set with its CheckpointSnapshot
/// alias; a third disclosure-shaped argument would have pushed
/// `doctor_dir_report` to 8).
#[derive(Default)]
struct DoctorDisclosureFold {
    disclosures: BTreeMap<String, usize>,
    dropped_lines: BTreeMap<String, usize>,
}

impl DoctorDisclosureFold {
    fn fold_session(&mut self, session: &Session) {
        for (key, count) in &session.metrics.disclosure_counters {
            *self.disclosures.entry(key.clone()).or_insert(0) += count;
        }
        for (key, count) in &session.metrics.line_skips {
            *self.dropped_lines.entry(key.clone()).or_insert(0) += count;
        }
        // rm-1500: --doctor must agree with --overview (and the insights
        // data-health census) that a non-finite cost session exists — it
        // is excluded from cost aggregates there and disclosed here under
        // the same `non_finite_costs` vocabulary.
        if !session.metrics.cost_estimated.is_finite() {
            *self
                .disclosures
                .entry("non_finite_costs".to_string())
                .or_insert(0) += 1;
        }
    }
}

/// rm-862: the mutable per-scan reports travel together through the
/// directory walk (project decode, zero-usage, and the two-channel
/// disclosure fold), keeping the fold helpers under clippy's
/// `too_many_arguments` budget — the same bundling precedent rm-538
/// set with its CheckpointSnapshot alias when a third disclosure-shaped
/// argument would have pushed `doctor_dir_report` to 8.
#[derive(Default)]
struct DoctorScanState {
    project_decode: DoctorProjectDecodeReport,
    zero_usage: DoctorZeroUsageReport,
    disclosure_fold: DoctorDisclosureFold,
    /// rm-1534: sessions whose last activity sits beyond
    /// now+grace (the `future_timestamp` anomaly class) across BOTH
    /// doctor lanes — the walk-parsed files and the sqlite-backed
    /// corpus — counted once here so the doctor row and the
    /// disclosure fold agree.
    future_timestamp_sessions: usize,
    // rm-367/rm-904 accumulator (folded into the rm-862 scan state at
    // the integration of run 5eb82325, after both landed on this wall
    // post-base): every session a directory report parses or a cache
    // hit serves is retained here so the subagent linkage pass at the
    // end of `doctor_directories` sees the whole scanned corpus.
    scanned_sessions: Vec<Session>,
}

fn doctor_directories(
    dir: Option<&Path>,
    demo: bool,
    files: &[PathBuf],
    sessions: &[Session],
    sqlite_ingest: &SqliteIngestReport,
    scan: &mut DoctorScanState,
) -> Vec<DoctorDirReport> {
    if demo {
        // rm-596 demo gate: enumerate no real root — both
        // `known_session_dirs()` and `doctor_sqlite_directories()` probe
        // the operator's home (paths, `is_dir()`/`is_file()` checks,
        // symlink targets). The bundled corpus is folded through the
        // SAME two channels as the non-demo fold below — informational
        // `disclosure_counters` and, since rm-526, `line_skips` — so a
        // scanned session discloses identically regardless of backing
        // store and the demo report still exercises that aggregate; a
        // demo fixture with a torn tail must not silently vanish here.
        // rm-862: the two channels land in separate maps (parse loss no
        // longer masquerades as a disclosed fact).
        for session in sessions {
            scan.disclosure_fold.fold_session(session);
        }
        // rm-904: the bundled corpus runs through the same subagent
        // linkage pass as a real scan, so a demo report discloses an
        // orphaned subagent identically instead of silently dropping
        // the grain the CLI lane discloses (0 → no key, clean). The
        // count is informational, so it lands on the rm-862
        // informational tier.
        disclose_unlinked_subagent_count(
            &mut sessions.to_vec(),
            &mut scan.disclosure_fold.disclosures,
        );
        return Vec::new();
    }
    // rm-367(c): stderr progress before the first parse, so a slow
    // scan is diagnosable live (stage (c) of the row). Terminal-gated:
    // a captured/piped stderr stays machine-readable — the CLI's
    // knob-disclosure JSON owns captured stderr (entrypoints
    // contract; regression: env_history_knob test).
    let mut roots_seen: HashSet<&Path> = HashSet::new();
    for file in files {
        if let Some(parent) = file.parent() {
            roots_seen.insert(parent);
        }
    }
    let root_bound = DoctorRootBound::from_env();
    if std::io::stderr().is_terminal() {
        eprintln!(
            "agenttrace: doctor: parsing {} session file(s) across {} root(s) (root budget {}s, per-root size cap {} MB)",
            files.len(),
            roots_seen.len(),
            root_bound.budget_ms / 1000,
            root_bound.max_bytes / (1024 * 1024),
        );
    }
    let mut cache = load_session_cache();
    let mut dirs: Vec<DoctorDirReport> = Vec::new();
    if let Some(dir) = dir {
        let abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        let DoctorScanState {
            project_decode,
            zero_usage,
            future_timestamp_sessions: _,
            disclosure_fold,
            scanned_sessions,
        } = scan;
        dirs.push(doctor_dir_report(
            "custom",
            &abs,
            files,
            &root_bound,
            &mut cache,
            scanned_sessions,
            project_decode,
            zero_usage,
            disclosure_fold,
        ));
    } else {
        let mut count_by_root = BTreeMap::new();
        for candidate in known_session_dirs() {
            count_by_root.insert(candidate.path, 0usize);
        }
        for file in files {
            for (root, count) in &mut count_by_root {
                if is_under(file, root) {
                    *count += 1;
                }
            }
        }

        for candidate in known_session_dirs() {
            let matching = files
                .iter()
                .filter(|file| is_under(file, &candidate.path))
                .cloned()
                .collect::<Vec<_>>();
            dirs.push(doctor_dir_report(
                &candidate.name,
                &candidate.path,
                &matching,
                &root_bound,
                &mut cache,
                &mut scan.scanned_sessions,
                &mut scan.project_decode,
                &mut scan.zero_usage,
                &mut scan.disclosure_fold,
            ));
        }
    }
    // rm-367 stage (a): persist fresh parses so an immediate --doctor
    // rerun on the same corpus reuses them (the loader lane's
    // discovery.rs save pattern) — before this the doctor READ the
    // cache but never wrote it, so every rerun re-parsed every
    // session from source. Stages b-d (finder-level reuse of the
    // walk itself) stay open on the row.
    // rm-367 arm (run 82a64f5a cycle 2): the save rides
    // `persist_session_cache_disclosed` — a failed write (read-only
    // cache dir) prints one stderr note instead of vanishing.
    if cache.is_dirty() {
        crate::session_cache::persist_session_cache_disclosed(&mut cache);
    }
    dirs.extend(doctor_sqlite_directories(sessions, sqlite_ingest));
    // SQLite-backed sessions (and, under `--demo`, the bundled corpus)
    // never flow through doctor_dir_report; fold their disclosure
    // counters in directly so every scanned session discloses
    // identically regardless of backing store.
    for session in sessions {
        // rm-526: `line_skips` (unparseable_line, non_object_line,
        // event_schema, codex_unparseable_line, …) is a disclosure
        // channel too — doctor surfaced only disclosure_counters, so
        // torn tails and format-parser drops were invisible here.
        // (rm-730 moved the codex structural counters —
        // codex_ignorable_line, codex_world_state — onto
        // disclosure_counters, so they reach this fold through the
        // loop above; true-loss counters stay on line_skips.)
        // rm-862: the fold splits the channels — parse loss lands in
        // its own loss-tier map so the render can label it as parse
        // loss, not a disclosed fact.
        scan.disclosure_fold.fold_session(session);
    }
    // rm-904: both lanes join the same subagent linkage pass the
    // loader runs (subagents::attribute_subagents, discovery.rs's
    // lane), so orphaned subagent transcripts disclose through
    // --doctor exactly as they do on every CLI report path
    // (main.rs disclose_unlinked_subagents) — 0-count-clean. The
    // count is informational, so it lands on the rm-862
    // informational tier.
    scan.scanned_sessions.extend(sessions.iter().cloned());
    disclose_unlinked_subagent_count(
        &mut scan.scanned_sessions,
        &mut scan.disclosure_fold.disclosures,
    );
    // rm-1534: census over the full scanned corpus (both lanes) —
    // the count rides the parsed session's own `future_timestamp`
    // anomaly, so the doctor, `--sessions`, and the ranged views all
    // name the same sessions. 0-count-clean: the row and the
    // disclosure key appear only when there is something to name.
    scan.future_timestamp_sessions = scan
        .scanned_sessions
        .iter()
        .filter(|session| {
            session
                .anomalies
                .iter()
                .any(|anomaly| anomaly.kind == "future_timestamp")
        })
        .count();
    if scan.future_timestamp_sessions > 0 {
        scan.disclosure_fold.disclosures.insert(
            "future_timestamp_sessions".to_string(),
            scan.future_timestamp_sessions,
        );
    }
    dirs
}

/// rm-904: counts subagent transcripts whose parent transcript is not
/// part of the scanned corpus (orphaned children) and discloses them
/// through the doctor's disclosures map — mirroring the rm-548
/// `opencode_fork_excluded_sessions` insert: the key appears only when
/// there is something to disclose (0-count-clean), so healthy corpora
/// read identically to before.
fn disclose_unlinked_subagent_count(
    sessions: &mut [Session],
    disclosures: &mut BTreeMap<String, usize>,
) {
    let unlinked = crate::subagents::attribute_subagents(sessions);
    if unlinked > 0 {
        disclosures.insert("unlinked_subagents".to_string(), unlinked);
    }
}

#[allow(clippy::too_many_arguments)] // rm-367 stage (a) adds the scanned-session accumulator
fn doctor_dir_report(
    name: &str,
    path: &Path,
    files: &[PathBuf],
    bound: &DoctorRootBound,
    cache: &mut crate::SessionCache,
    scanned_sessions: &mut Vec<Session>,
    project_decode: &mut DoctorProjectDecodeReport,
    zero_usage: &mut DoctorZeroUsageReport,
    disclosures: &mut DoctorDisclosureFold,
) -> DoctorDirReport {
    let mut parsed = 0;
    let mut cache_hits = 0;
    // rm-451: read-cap refusals counted separately from other parse
    // failures (the `failed` counter keeps covering every failure).
    let mut read_cap_refused = 0;
    let mut failure_samples = Vec::new();
    // rm-367(b)/(d): fresh parses run under the root's wall-clock
    // budget and cumulative parsed-byte cap; cache hits are cheap and
    // exempt. A skipped file is counted in its own bucket — never
    // labeled a parse failure.
    let deadline = bound.deadline();
    let mut parsed_bytes: u64 = 0;
    let mut skipped_budget = 0;
    let mut skipped_size = 0;
    for file in files {
        // Keep the parsed session where the decode disclosure (rm-381) can
        // see it; the parse accounting below is unchanged. The parse stays
        // on the cache-miss lane only (warm --doctor reruns never
        // re-parse: cache hits stay lazy, the pre-rm-451 shape).
        let session = if let Some(session) = cached_session(file, cache) {
            cache_hits += 1;
            parsed += 1;
            Some(session)
        } else if bound.deadline_exceeded(deadline) {
            // rm-367(b): the root's wall-clock budget is exhausted —
            // disclose the file as budget-skipped, never as failed.
            skipped_budget += 1;
            None
        } else if parsed_bytes > bound.max_bytes {
            // rm-367(d): count-sampled pass for oversized roots — the
            // root is counted past the cap, not parsed.
            skipped_size += 1;
            None
        } else {
            // rm-451: parse through a match so a refusal is classified
            // (ReadCapRefused) before it becomes a failure sample.
            match parse_file(file) {
                Ok(session) => {
                    parsed += 1;
                    parsed_bytes += fs::metadata(file).map(|meta| meta.len()).unwrap_or(0);
                    // rm-367 stage (a): persist the fresh parse so an
                    // immediate --doctor rerun reuses it — the loader lane's
                    // discovery.rs:382 pattern. The reuse shows up on the
                    // NEXT run (cache_hits / cached_valid), not this one.
                    let _ = store_session(file, &session, cache);
                    Some(session)
                }
                Err(error) => {
                    // rm-451: classify read-cap refusals so --doctor
                    // names the admission class (knob
                    // AGENTTRACE_FILE_CAP_BYTES), not a bare parse
                    // failure. Skip, never crash.
                    if error
                        .downcast_ref::<crate::parser::ReadCapRefused>()
                        .is_some()
                    {
                        read_cap_refused += 1;
                    }
                    if failure_samples.len() < 3 {
                        failure_samples.push(file.to_string_lossy().to_string());
                    }
                    None
                }
            }
        };
        if let Some(session) = session {
            // rm-904: the session is retained (post-folds) for the
            // subagent linkage pass at the end of doctor_directories.
            scanned_sessions.push(session.clone());
            collect_project_decode(project_decode, &session);
            collect_zero_usage(zero_usage, &session);
            // rm-436/rm-437: cache-hit and freshly parsed sessions
            // disclose identically — the counters round-trip through
            // the session cache (GoMetrics), so a warm doctor scan
            // cannot silently lose them.
            // rm-526: same fold for line_skips — parse-failure and
            // drop disclosures (torn tails) reach --doctor through
            // this channel; they round-trip the cache the same way.
            // rm-862: parse loss lands in the loss-tier map.
            disclosures.fold_session(&session);
        }
    }
    DoctorDirReport {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        exists: path.is_dir(),
        files: files.len(),
        parsed,
        failed: files
            .len()
            .saturating_sub(parsed + skipped_budget + skipped_size),
        skipped_budget,
        skipped_size,
        cache_hits,
        symlink_target: symlink_target_of(path),
        failure_samples,
        read_cap_refused,
    }
}

/// Names the link when `path` is a symlink, so symlinked session roots
/// are disclosed rather than silently followed (cycle 7, Codex
/// `#42135`).
fn symlink_target_of(path: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_symlink() {
        return None;
    }
    fs::read_link(path)
        .ok()
        .map(|target| target.to_string_lossy().to_string())
}

fn doctor_sqlite_directories(
    sessions: &[Session],
    ingest: &SqliteIngestReport,
) -> Vec<DoctorDirReport> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let mut count_by_tool = BTreeMap::new();
    for session in sessions {
        *count_by_tool
            .entry(session.metrics.source_tool.clone())
            .or_insert(0usize) += 1;
    }
    let candidates = [
        (
            "hermes_db",
            "Hermes Agent (DB)",
            home.join(".hermes").join("state.db"),
        ),
        (
            "opencode_db",
            "OpenCode (DB)",
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("opencode.db"),
        ),
    ];
    let mut dirs = Vec::new();
    for (tool, name, path) in candidates {
        let parsed = count_by_tool.get(tool).copied().unwrap_or(0);
        let exists = path.is_file();
        if !exists && parsed == 0 {
            continue;
        }
        // rm-753: the counters now come from the ingest report, so a
        // discovered-but-unreadable database reports found/failed=1
        // with a sample instead of parsed=0 failed=0, and dropped
        // session rows (NULL ids, wrong-typed columns) count as
        // failures. Non-primary databases (profile state.dbs,
        // opencode*.db siblings) stay rm-607's disclosure lane.
        let unreadable = ingest.unreadable.iter().find(|db| db.path == path);
        let dropped = ingest.dropped_rows.iter().find(|db| db.path == path);
        let failed =
            usize::from(unreadable.is_some()) + dropped.map(|entry| entry.dropped).unwrap_or(0);
        let mut failure_samples = Vec::new();
        if let Some(db) = unreadable {
            failure_samples.push(format!("unreadable: {}", db.reason));
        }
        if let Some(entry) = dropped {
            failure_samples.push(format!(
                "{} session row(s) failed to decode (e.g. {})",
                entry.dropped, entry.sample
            ));
        }
        dirs.push(DoctorDirReport {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            exists,
            files: parsed + failed,
            parsed,
            failed,
            cache_hits: 0,
            skipped_budget: 0,
            skipped_size: 0,
            symlink_target: symlink_target_of(&path),
            failure_samples,
            read_cap_refused: 0,
        });
    }
    dirs
}

/// rm-858: POSIX single-quote a path for interpolation into a shell command
/// the operator is told to paste. `foo bar` -> `'foo bar'`; embedded quotes
/// are escaped with the standard `'\''` sequence (close-quote, escaped
/// quote, reopen-quote) so the pasted command runs verbatim for paths
/// containing spaces, quotes, or shell metacharacters; non-UTF-8 path
/// bytes render lossily (U+FFFD) and the quoting still holds.
fn shell_quote(path: &std::path::Path) -> String {
    let raw = path.to_string_lossy();
    format!("'{}'", raw.replace('\'', "'\\''"))
}
fn doctor_recommendations(report: &DoctorReport, dir: Option<&Path>, demo: bool) -> Vec<String> {
    // rm-734: name every unreadable sqlite source with a repair path.
    // Computed BEFORE the empty-corpus early return on purpose — a
    // poisoned database can zero out the corpus entirely, which is
    // exactly when the operator most needs the pointer.
    let mut sqlite_repairs: Vec<String> = Vec::new();
    for failure in &report.sqlite_read_failures {
        sqlite_repairs.push(format!(
            "sqlite session source could not be read: {} ({}): {} — \
             its sessions are excluded from this report; repair the file or its \
             permissions (run `sqlite3 {} \"pragma integrity_check;\"` to classify \
             the corruption) and re-run",
            failure.path.display(),
            failure.source,
            failure.reason,
            shell_quote(&failure.path)
        ));
    }
    if report.sessions == 0 {
        if dir.is_some() {
            let mut out = vec![
                "No sessions found in this directory. Check `-d <dir>` or point it at a session JSON/JSONL directory."
                    .to_string(),
            ];
            out.extend(sqlite_repairs);
            return out;
        }
        let mut out = vec![
            "No sessions found. Run `agenttrace --demo` to try the TUI immediately.".to_string(),
        ];
        out.extend(sqlite_repairs);
        return out;
    }
    let mut recommendations = vec![
        "Ready: run `agenttrace` for the TUI or `agenttrace --overview -f json` for automation."
            .to_string(),
    ];
    recommendations.extend(sqlite_repairs);
    // rm-408: present-zero usage is counted as measured, so the corpus
    // share belongs in the recommendations — not folded into "clean".
    // This push sits ABOVE the demo early-return (review F8): demo
    // sessions carry real all-zero usage blocks (the demo corpus reports
    // 200 sessions / 200 events), and a demo doctor that counted them
    // but stayed silent about the share would break the same
    // never-report-clean-zeros-silently contract.
    if report.zero_usage.events > 0 {
        recommendations.push(format!(
            "{} of {} session(s) contain client-reported all-zero usage blocks ({} event(s) total), counted as measured zeros and flagged `zero_usage_reported` in provenance; check the recording client's version if the share is high.",
            report.zero_usage.sessions,
            report.sessions.max(report.zero_usage.sessions),
            report.zero_usage.events
        ));
    }
    // rm-1534: far-future sessions get a repair pointer, not just a
    // count — the usual cause is a machine clock years off (or a
    // journal written by one), and the fix is at the source: the
    // ranged views already exclude + disclose, but the operator
    // should also learn WHY.
    if report.future_timestamp_sessions > 0 {
        recommendations.push(format!(
            "{} session(s) carry activity timestamps beyond now+{}h (future_timestamp): ranged views exclude them; check the recording machine's clock and the journal's timestamps at the source.",
            report.future_timestamp_sessions,
            crate::insights::FUTURE_ADMISSION_GRACE_SECS / 3600
        ));
    }
    // rm-607: sibling sqlite databases suppressed as duplicates are
    // disclosed, not silently skipped — the numbers are honest but the
    // user should learn their backups are inside the discovery glob.
    // (Review 5968db61 F5, closed at conflict case 2b3bf2f0: build the
    // path list once — the first cut collected it twice and took
    // .len() of a map-collect.)
    let duplicate_paths = report
        .sqlite_duplicate_dbs
        .iter()
        .map(|db| db.path.as_str())
        .collect::<Vec<_>>();
    let duplicate_sessions: usize = report
        .sqlite_duplicate_dbs
        .iter()
        .map(|db| db.duplicate_sessions)
        .sum();
    if duplicate_sessions > 0 {
        recommendations.push(format!(
            "{} session(s) across {} sibling sqlite database(s) ({}) were counted once because they repeat sessions already carried by the canonical database; archive backups outside the agent home to keep discovery unambiguous.",
            duplicate_sessions,
            duplicate_paths.len(),
            duplicate_paths.join(", ")
        ));
    }
    // rm-1887: artifact mirrors skipped from aggregation are disclosed
    // so their child spend reads "skipped, not zero" — the parent
    // rollup lane does not attribute it, and a reader should learn
    // WHY the corpus shrank instead of trusting a silent drop.
    if let Some(&count) = report.disclosures.get("subagent_artifact_files") {
        if count > 0 {
            recommendations.push(format!(
                "{} pi-subagents artifact file(s) were skipped: each mirrors child spend its parent session already owns, so counting them would double-bank. Delete stale subagent-artifacts subtrees you no longer need.",
                count
            ));
        }
    }
    // rm-1886: byte-identical journal copies suppressed at admission.
    if let Some(&count) = report.disclosures.get("duplicate_session_files") {
        if count > 0 {
            recommendations.push(format!(
                "{} byte-identical session file copy/copies were counted once (content signature match); keep ONE canonical copy per journal so tooling that ignores content identity stays honest.",
                count
            ));
        }
    }
    // rm-2126 (run 82a64f5a cycle 2): the depth bound is a shape
    // heuristic, not a semantic one — a corpus nested past it loses
    // files with no other trace. The counter is exact (only clips
    // whose peek found eligible content), the loss is not: name the
    // escape hatch so the fix is actionable.
    if let Some(&count) = report.disclosures.get("depth_clipped_dirs") {
        if count > 0 {
            recommendations.push(format!(
                "{} nested director{} past the session-walk depth bound held file(s) the walk refused; counts may be low. Point `--dir` at the deepest level (the explicit lane walks it from scratch) or split the tree.",
                count,
                if count == 1 { "y lies" } else { "ies lie" }
            ));
        }
    }
    // rm-2127: an unreadable subtree (mode-000 dir, EIO mount) hides
    // everything beneath it; unlike the depth clip this one is
    // usually a permissions accident, so the repair is on the tree.
    if let Some(&count) = report.disclosures.get("unreadable_dirs") {
        if count > 0 {
            recommendations.push(format!(
                "{} director{} could not be read (permission or I/O error); session files beneath them are not counted. Check ownership/modes if that subtree should be scanned.",
                count,
                if count == 1 { "y was" } else { "ies were" }
            ));
        }
    }
    // rm-1888 arm: same-inode aliases are suppressed at walk
    // admission so one physical transcript banks once — if the
    // operator intends them as separate sessions, the disclosure is
    // where they learn the tool disagreed.
    if let Some(&count) = report.disclosures.get("same_inode_files_suppressed") {
        if count > 0 {
            recommendations.push(format!(
                "{} same-inode session file alias(es) (hardlink/symlink) were counted once; each alias re-states one physical transcript.",
                count
            ));
        }
    }
    if demo {
        recommendations.push(
            "Demo sessions are bundled in memory, so cache reuse does not apply in this mode."
                .to_string(),
        );
        return recommendations;
    }
    if report.cached_valid == 0 {
        recommendations.push("No reusable parsed session entries for this scan. Cached directory listings may still speed discovery; the next TUI startup should reuse parsed sessions incrementally.".to_string());
    }
    // rm-381 disclosure: attribution made under host-state ambiguity, or not
    // made at all, is surfaced instead of silently decided.
    let decode = &report.project_decode;
    if decode.ambiguous > 0 || decode.unresolved > 0 {
        let mut note = format!(
            "{} session(s) under a `-`-encoded projects directory were ",
            decode.ambiguous + decode.unresolved
        );
        if decode.ambiguous > 0 && decode.unresolved > 0 {
            note.push_str(&format!(
                "split between {} attributed by the deterministic longest-verified-path rule with a verified alternative present (ambiguous) and {} with no verified decode (attributed to `unknown`)",
                decode.ambiguous, decode.unresolved
            ));
        } else if decode.ambiguous > 0 {
            note.push_str("attributed by the deterministic longest-verified-path rule while another directory also verified (ambiguous)");
        } else {
            note.push_str(
                "left unattributed because no directory matching the encoded name verified",
            );
        }
        note.push_str("; transcripts that record a cwd attribute exactly — prefer exporting those, or see `--doctor` samples above");
        recommendations.push(note);
    }
    recommendations
}

pub fn doctor_report_text(report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("AGENTTRACE Doctor\n");
    out.push_str(&format!("Version: {}\n", report.version));
    out.push_str(&format!("Mode: {}\n", report.mode));
    out.push_str(&format!("Session files: {}\n", report.sessions));
    out.push_str(&format!("Cache: {}\n", report.cache_path));
    // rm-575: AGENTTRACE_CACHE_DIR is not read by anything (the fleet
    // constraint pins that it never relocates). A set value is almost
    // certainly a typo for AGENTTRACE_SESSION_CACHE_DIR — disclose the
    // near-miss instead of letting it silently no-op.
    if let Some(value) = std::env::var_os("AGENTTRACE_CACHE_DIR") {
        if !value.is_empty() {
            out.push_str(
                "  note: AGENTTRACE_CACHE_DIR is set but not used; did you mean AGENTTRACE_SESSION_CACHE_DIR? It does not relocate anything.\n",
            );
        }
    }
    out.push_str(&format!(
        "  {} parsed session cache entries, {} reusable for this scan, {} cached directory listings\n",
        report.cache_entries, report.cached_valid, report.cache_dirs
    ));
    out.push_str(&format!(
        "  {} of {} session file(s) re-parsed from source this scan (not served by the cache)\n",
        report.reparsed_this_scan, report.session_files
    ));
    out.push_str(&format!(
        "  cache size {} bytes, hard bounds: {} (oldest-source entries evicted first; entry bound from {})\n",
        report.cache_size_bytes, report.cache_limits, report.cache_entry_bound_source
    ));
    if let Some(note) = &report.cache_invalidation_note {
        out.push_str(&format!("  cache disclosure: {note}\n"));
    }
    let statusline_state = if report.statusline.exists {
        format!(
            "{} captures, {} distinct sessions, {} bytes",
            report.statusline.captures, report.statusline.sessions, report.statusline.bytes
        )
    } else {
        "no captures (configure statusLine to `agenttrace statusline`)".to_string()
    };
    out.push_str(&format!(
        "Statusline capture: {}\n  {}\n",
        report.statusline.path, statusline_state
    ));
    out.push_str(&format!("Pricing snapshot: {}\n", report.pricing));
    for failure in &report.sqlite_read_failures {
        out.push_str(&format!(
            "Sqlite session source unreadable: {} ({}) — {}\n",
            failure.path.display(),
            failure.source,
            failure.reason
        ));
    }
    out.push_str(&format!(
        "Project attribution: {} resolved, {} ambiguous, {} unresolved encoded project dirs\n",
        report.project_decode.resolved,
        report.project_decode.ambiguous,
        report.project_decode.unresolved
    ));
    for sample in &report.project_decode.samples {
        out.push_str(&format!("    {}\n", guard_formula_cell(sample)));
    }
    out.push_str(&format!(
        "Zero-usage reports: {} session(s), {} event(s) counted as measured zeros (rm-408)\n",
        report.zero_usage.sessions, report.zero_usage.events
    ));
    // rm-1534: the future-timestamp census names what ranged views
    // exclude — twin discipline with the zero-usage row above: an
    // excluded class gets a visible count, never a silent drop.
    // 0-count-clean: healthy corpora render identically.
    if report.future_timestamp_sessions > 0 {
        out.push_str(&format!(
            "Future timestamps: {} session(s) with activity beyond now+{}h (rm-1534) — excluded from ranged views, disclosed here and as `future_timestamp` anomalies\n",
            report.future_timestamp_sessions, crate::insights::FUTURE_ADMISSION_GRACE_SECS / 3600
        ));
    }
    for sample in &report.zero_usage.samples {
        out.push_str(&format!("    {}\n", guard_formula_cell(sample)));
    }
    if !report.sqlite_duplicate_dbs.is_empty() {
        out.push_str(&format!(
            "Duplicate sqlite databases (rm-607): {}\n",
            report
                .sqlite_duplicate_dbs
                .iter()
                .map(|db| format!(
                    "    {} — {} duplicate session(s) suppressed, canonical db kept\n",
                    guard_formula_cell(&db.path),
                    db.duplicate_sessions
                ))
                .collect::<String>()
        ));
    }
    if !report.duplicate_session_file_pairs.is_empty() {
        out.push_str(&format!(
            "Duplicate session files (rm-1888): {}\n{}",
            report
                .disclosures
                .get("duplicate_session_files")
                .copied()
                .unwrap_or(report.duplicate_session_file_pairs.len()),
            report
                .duplicate_session_file_pairs
                .iter()
                .map(|pair| format!("    {}\n", guard_formula_cell(pair)))
                .collect::<String>()
        ));
    }
    if !report.dropped_lines.is_empty() {
        // rm-862: parse loss renders as its own tier. The rm-526 fold
        // landed `line_skips` under "Disclosed facts", so a doctor
        // reader could not tell `unparseable_line=1` (a torn tail,
        // tokens possibly missing) from `kimi_usage_alias:…=1` (a
        // healthy disclosure). Keys still route through the
        // control-byte sanitizer (mirror of counts_cell in reports.rs);
        // rm-594 residual: the same single-line bound the
        // informational tier uses applies here too (first
        // DISCLOSURE_RENDER_ENTRY_CAP entries plus an explicit count
        // of what is held back).
        let mut dropped_parts: Vec<String> = report
            .dropped_lines
            .iter()
            .take(crate::parser::DISCLOSURE_RENDER_ENTRY_CAP)
            .map(|(key, count)| format!("{}={count}", crate::parser::capped_disclosure_value(key)))
            .collect();
        if report.dropped_lines.len() > crate::parser::DISCLOSURE_RENDER_ENTRY_CAP {
            dropped_parts.push(format!(
                "+{} more distinct keys",
                report.dropped_lines.len() - crate::parser::DISCLOSURE_RENDER_ENTRY_CAP
            ));
        }
        out.push_str(&format!(
            "Dropped lines (parse loss): {}\n",
            dropped_parts.join(", ")
        ));
    }
    if !report.disclosures.is_empty() {
        // rm-595: disclosure keys route through the control-byte
        // sanitizer at render — mirror of counts_cell in reports.rs; a
        // hostile key carries no ESC/OSC sequence into doctor text.
        // rm-594 residual: the same single-line bound counts_cell
        // applies — first 40 entries plus an explicit count of what is
        // held back (the 4.6MB cardinality PoC rendered 2.15MB here).
        // rm-862: informational only — parse loss moved to the
        // "Dropped lines (parse loss)" row above, so everything here
        // is a healthy disclosure.
        let mut parts: Vec<String> = report
            .disclosures
            .iter()
            .take(crate::parser::DISCLOSURE_RENDER_ENTRY_CAP)
            .map(|(key, count)| format!("{}={count}", crate::parser::capped_disclosure_value(key)))
            .collect();
        if report.disclosures.len() > crate::parser::DISCLOSURE_RENDER_ENTRY_CAP {
            parts.push(format!(
                "+{} more distinct keys",
                report.disclosures.len() - crate::parser::DISCLOSURE_RENDER_ENTRY_CAP
            ));
        }
        out.push_str(&format!(
            "Disclosed facts (informational): {}\n",
            parts.join(", ")
        ));
    }
    // rm-1550: budget truncation must be disclosed on the TEXT lane too —
    // scan_partial and sqlite_partial_dbs previously rendered JSON-only,
    // so a truncated scan presented partial counts as complete on the
    // CLI's default format. Zero-noise: nothing renders when the scan
    // completed unbounded (the rm-367 contract "partial counts are never
    // presented as complete ones" now holds on every surface).
    if let Some(partial) = &report.scan_partial {
        out.push_str(&format!(
            "Scan budget: TRUNCATED — {} of {} session file(s) parsed, {} skipped on root budget, {} skipped on size cap, {} sqlite database pass(es) cut short ({} of {} root(s) completed); counts below are partial, not complete\n",
            partial.files_parsed,
            partial.session_files,
            partial.files_skipped_budget,
            partial.files_skipped_size,
            partial.sqlite_dbs_partial,
            partial.roots - partial.roots_truncated,
            partial.roots
        ));
        if !partial.note.is_empty() {
            out.push_str(&format!(
                "  {}\n",
                crate::parser::capped_disclosure_value(&partial.note)
            ));
        }
    }
    if !report.sqlite_partial_dbs.is_empty() {
        out.push_str(&format!(
            "Sqlite session stores truncated by the scan budget ({} partial — rows below are partial, not complete):\n",
            report.sqlite_partial_dbs.len()
        ));
        for db in &report.sqlite_partial_dbs {
            out.push_str(&format!(
                "  {} — {} session(s) kept before the budget cut the pass short\n",
                crate::parser::capped_disclosure_value(&db.path),
                db.sessions_kept
            ));
        }
    }
    out.push_str("\nProviders:\n");
    for dir in &report.directories {
        let status = if dir.exists { "found" } else { "missing" };
        let symlink = match &dir.symlink_target {
            Some(target) => format!(" (symlink -> {target})"),
            None => String::new(),
        };
        out.push_str(&format!(
            "  {:20} {:7} found={:<5} parsed={:<5} failed={:<5} cache={:<5} {}{}\n",
            dir.name, status, dir.files, dir.parsed, dir.failed, dir.cache_hits, dir.path, symlink
        ));
        for sample in &dir.failure_samples {
            out.push_str(&format!("    failed: {sample}\n"));
        }
        // rm-451: name the read-cap share explicitly so the admission
        // bound is visible at a glance in the text report (the JSON
        // arm carries the same counter as a field).
        if dir.read_cap_refused > 0 {
            out.push_str(&format!(
                "    read-cap refused: {} file(s) past the AGENTTRACE_FILE_CAP_BYTES admission bound\n",
                dir.read_cap_refused
            ));
        }
    }
    out.push_str("\nRecommendations:\n");
    for rec in &report.recommendations {
        out.push_str(&format!("  - {rec}\n"));
    }
    out
}

fn load_session_cache_report() -> SessionCacheReport {
    let path = session_cache_path();
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return SessionCacheReport {
            path,
            ..SessionCacheReport::default()
        };
    };
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&raw) else {
        return SessionCacheReport {
            path,
            ..SessionCacheReport::default()
        };
    };
    let mut entries = BTreeMap::new();
    if let Some(raw_entries) = doc.get("entries").and_then(Value::as_object) {
        for (path, entry) in raw_entries {
            if let Some(header) = decode_cache_entry_header(entry) {
                entries.insert(path.clone(), header);
            }
        }
    }
    let dirs = doc
        .get("dirs")
        .and_then(Value::as_object)
        .map(|dirs| {
            dirs.values()
                .filter(|entry| entry.get("mod_time").and_then(Value::as_i64).is_some())
                .count()
        })
        .unwrap_or(0);
    // rm-292 disclosure parity (review fix R3, run 2d92ee95): surface
    // the persisted invalidation ledger — schema bumps AND pricing
    // catalog changes — so a re-parse cascade is visible from
    // `--doctor` (text and JSON) instead of only by opening the
    // journal by hand.
    let invalidation_note = doc
        .get("schema_invalidations")
        .and_then(Value::as_array)
        .filter(|records| !records.is_empty())
        .map(|records| {
            format!(
                "{} invalidation record(s) recorded; newest: {}",
                records.len(),
                serde_json::to_string(&records[records.len() - 1])
                    .unwrap_or_else(|_| "<unserializable record>".to_string())
            )
        });
    SessionCacheReport {
        path,
        entries,
        dirs,
        invalidation_note,
    }
}

fn session_cache_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR").map(PathBuf::from) {
        if !dir.as_os_str().is_empty() {
            return dir.join("sessions.json");
        }
    }
    user_cache_dir().join("agenttrace").join("sessions.json")
}

// The cache ladder lives in `crate::cache_dir` — one shared
// resolver for doctor, session_cache, pricing, and statusline
// (consolidated from this file's byte-identical copy; that copy
// resolved the macOS `Library/Caches` arm BEFORE an absolute
// $XDG_CACHE_HOME and joined a set-but-empty OR relative HOME into a
// RELATIVE `.cache` against the CWD).

fn decode_cache_entry_header(value: &Value) -> Option<CacheEntryHeader> {
    Some(CacheEntryHeader {
        mod_time: value.get("mod_time")?.as_i64()?,
        size: value.get("size")?.as_i64()?,
    })
}

fn valid_cached_session_count(paths: &[PathBuf], cache: &SessionCacheReport) -> usize {
    let mut valid = 0;
    let mut seen = HashSet::new();
    for path in paths {
        let key = path.to_string_lossy().to_string();
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(entry) = cache.entries.get(&key) else {
            continue;
        };
        let Ok(metadata) = path.metadata() else {
            continue;
        };
        if entry.size != metadata.len() as i64 {
            continue;
        }
        if file_mod_time_nanos(&metadata) == Some(entry.mod_time) {
            valid += 1;
        }
    }
    valid
}

#[cfg(unix)]
fn file_mod_time_nanos(metadata: &std::fs::Metadata) -> Option<i64> {
    use std::os::unix::fs::MetadataExt;
    Some(metadata.mtime() * 1_000_000_000 + metadata.mtime_nsec())
}

#[cfg(not(unix))]
fn file_mod_time_nanos(metadata: &std::fs::Metadata) -> Option<i64> {
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(duration.as_nanos() as i64)
}

fn is_under(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_at(cwd: &str, path: &str) -> Session {
        Session {
            name: "session.jsonl".to_string(),
            path: path.to_string(),
            cwd: cwd.to_string(),
            branch: String::new(),
            org: String::new(),
            org_email: String::new(),
            metrics: crate::Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        }
    }

    /// A session whose encoded project dir cannot decode (rm-512
    /// fixtures): the path sits under a nonexistent root, so the walk
    /// finds no verified decode. Re-anchored at integration 2026-10-07
    /// (conflict case d9ddc6a9): rm-240 landed after rm-512 and resolves
    /// such names as OPAQUE identities instead of `Unresolved`, so these
    /// fixtures now exercise the resolved-decode note tally — the
    /// rm-512 dedup/budget/surface semantics under test are unchanged.
    fn unresolved_session(encoded_dir: &str) -> Session {
        session_at("", &format!("/nowhere/projects/-{encoded_dir}/s.jsonl"))
    }

    #[test]
    fn cache_bound_and_reparse_disclosures_cover_knob_and_waste() {
        // rm-298 capacity arm: the doctor report discloses the entry
        // bound actually in force, the layer it came from, and how
        // many session files this scan re-parsed from source. Demo
        // mode is hermetic (no host walk, zero session files), so the
        // waste arm reads zero here; the knob arms are the report's.
        let _env = crate::test_env::lock_env();
        std::env::remove_var("AGENTTRACE_SESSION_CACHE_ENTRIES");
        let report = build_doctor_report(None, true);
        assert_eq!(
            report.cache_entry_bound,
            crate::session_cache::MAX_SESSION_CACHE_ENTRIES
        );
        assert_eq!(report.cache_entry_bound_source, "default");
        assert_eq!(report.reparsed_this_scan, 0, "demo mode discovers no files");
        assert!(report
            .cache_limits
            .contains(&format!("entries<={}", report.cache_entry_bound)));
        let text = doctor_report_text(&report);
        assert!(text.contains("entry bound from default"), "{text}");
        assert!(
            text.contains("0 of 0 session file(s) re-parsed from source"),
            "{text}"
        );

        std::env::set_var("AGENTTRACE_SESSION_CACHE_ENTRIES", "1500");
        let report = build_doctor_report(None, true);
        assert_eq!(report.cache_entry_bound, 1_500);
        assert_eq!(report.cache_entry_bound_source, "env");
        assert!(report.cache_limits.contains("entries<=1500"));
        assert!(doctor_report_text(&report).contains("entry bound from env"));
        std::env::remove_var("AGENTTRACE_SESSION_CACHE_ENTRIES");
    }

    #[test]
    fn unresolved_samples_dedup_by_project_with_counts() {
        // rm-512: two distinct projects × three sessions each used to
        // fill the six-slot budget with repeats; they now render one
        // line per project, each carrying its session count.
        let mut report = DoctorProjectDecodeReport::default();
        for _ in 0..3 {
            collect_project_decode(&mut report, &unresolved_session("alpha-one"));
            collect_project_decode(&mut report, &unresolved_session("beta-two"));
        }
        report.finalize_samples();

        assert_eq!(report.unresolved, 0);
        // Integration 2026-10-07 (d9ddc6a9): the fixtures count as
        // resolved opaque identities now; the dedup contract under test
        // is otherwise byte-identical.
        assert_eq!(report.resolved, 6);
        assert_eq!(
            report.samples.len(),
            2,
            "one line per distinct project: {:?}",
            report.samples
        );
        assert!(report.samples[0].contains("projects/alpha-one"));
        assert!(
            report.samples[0].ends_with("(3 sessions)"),
            "{}",
            report.samples[0]
        );
        assert!(report.samples[1].contains("projects/beta-two"));
        assert!(
            report.samples[1].ends_with("(3 sessions)"),
            "{}",
            report.samples[1]
        );
    }

    #[test]
    fn ambiguous_samples_dedup_by_project_path() {
        // rm-512 ambiguous arm: a transcript under an encoded dir that
        // decodes to both a literal and a shadowed alternative — three
        // sessions collapse to one line with the session count.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-doctor-ambiguous-{}-{}",
            std::process::id(),
            line!()
        ));
        let base = root.join("misattr");
        let true_repo = base.join("my-repo");
        std::fs::create_dir_all(&true_repo).expect("create true repo dir");
        let decoy = base.join("my").join("repo");
        std::fs::create_dir_all(&decoy).expect("plant decoy repo dir");
        let encoded = true_repo.to_string_lossy().replace('/', "-");
        let transcript = root
            .join("projects")
            .join(&encoded)
            .join("session.jsonl")
            .to_string_lossy()
            .to_string();

        let mut report = DoctorProjectDecodeReport::default();
        for _ in 0..3 {
            collect_project_decode(&mut report, &session_at("", &transcript));
        }
        report.finalize_samples();
        let _ = std::fs::remove_dir_all(root);

        assert_eq!(report.ambiguous, 3);
        assert_eq!(
            report.samples.len(),
            1,
            "three sessions of one project render one line: {:?}",
            report.samples
        );
        assert!(
            report.samples[0].contains("ambiguous: "),
            "{}",
            report.samples[0]
        );
        assert!(
            report.samples[0].ends_with("(3 sessions)"),
            "{}",
            report.samples[0]
        );
    }

    #[test]
    fn sample_budget_prefers_distinct_projects() {
        // rm-512 budget: seven distinct projects keep the sample cap at
        // six without repeats swallowing the slots, and single-session
        // projects carry no count suffix.
        let mut report = DoctorProjectDecodeReport::default();
        for i in 0..7 {
            collect_project_decode(&mut report, &unresolved_session(&format!("proj-{i}")));
        }
        report.finalize_samples();

        assert_eq!(report.unresolved, 0);
        // Integration 2026-10-07 (d9ddc6a9): opaque identities, not
        // unresolved dirs — the budget contract under test is unchanged.
        assert_eq!(report.resolved, 7);
        assert_eq!(
            report.samples.len(),
            6,
            "the budget caps at six samples: {:?}",
            report.samples
        );
        for (i, sample) in report.samples.iter().enumerate() {
            assert!(
                sample.contains(&format!("proj-{i}")),
                "first-seen order: {sample}"
            );
            assert!(
                !sample.contains("(1 sessions)"),
                "single sessions stay suffix-free: {sample}"
            );
        }
    }

    #[test]
    fn project_decode_report_json_surface_unchanged() {
        // rm-512 must not alter the serde shape: the tally state stays
        // out of the JSON surface and `samples` keeps its
        // skip-when-empty behavior.
        let mut report = DoctorProjectDecodeReport::default();
        for _ in 0..2 {
            collect_project_decode(&mut report, &unresolved_session("serde-proj"));
        }
        let before = serde_json::to_value(&report).expect("serialize before finalize");
        assert!(
            before.get("samples").is_none(),
            "empty samples stay skipped: {before}"
        );
        assert!(
            !before.to_string().contains("tally"),
            "tally state never serializes: {before}"
        );

        report.finalize_samples();
        let after = serde_json::to_value(&report).expect("serialize after finalize");
        assert_eq!(
            after.get("unresolved"),
            Some(&serde_json::json!(0)),
            "counters unchanged: {after}"
        );
        // Integration 2026-10-07 (d9ddc6a9): rm-240 reclassifies the
        // fixture as a resolved opaque identity.
        assert_eq!(
            after.get("resolved"),
            Some(&serde_json::json!(2)),
            "counters unchanged: {after}"
        );
        let samples = after
            .get("samples")
            .expect("samples render after finalize")
            .as_array()
            .expect("samples stay an array");
        assert_eq!(samples.len(), 1);
        assert!(
            samples[0]
                .as_str()
                .unwrap()
                .starts_with("opaque: projects/serde-proj"),
            "two sessions dedup to one rm-240 resolved note (integration \
             2026-10-07, d9ddc6a9): {samples:?}"
        );
    }

    #[test]
    fn resolved_decode_notes_dedup_by_identity_with_counts() {
        // Integration 2026-10-07 (conflict case d9ddc6a9): the rm-240
        // truncated and opaque arms count as resolved with one
        // deduplicated sample line per identity — composed into the
        // rm-512 tally (session-count suffix, first-seen order, nothing
        // on the JSON surface before finalize). Two hashes sharing a
        // 200-char head stay two lines; a repeated opaque name stays
        // one line with its count.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-doctor-resolved-{}-{}",
            std::process::id(),
            line!()
        ));
        let mut deep = root.clone();
        for idx in 0..30 {
            deep = deep.join(format!("seg{idx:02}-component"));
        }
        std::fs::create_dir_all(&deep).expect("create deep project dir");
        let encoded = deep.to_string_lossy().replace('/', "-");
        assert!(
            encoded.chars().count() > 220,
            "fixture must exceed the truncation limit"
        );
        let head: String = encoded.chars().take(200).collect();
        let truncated = |hash: &str| {
            let dir = root.join("projects").join(format!("{head}-{hash}"));
            std::fs::create_dir_all(&dir).expect("create truncated dir");
            let path = dir.join("s.jsonl").to_string_lossy().to_string();
            session_at("", &path)
        };
        let opaque_dir = root.join("projects").join("MyOpaque-Name_77");
        std::fs::create_dir_all(&opaque_dir).expect("create opaque dir");
        let opaque_path = opaque_dir.join("s.jsonl").to_string_lossy().to_string();
        let opaque_session = session_at("", &opaque_path);

        let mut report = DoctorProjectDecodeReport::default();
        for _ in 0..2 {
            collect_project_decode(&mut report, &truncated("aaaa1111"));
            collect_project_decode(&mut report, &truncated("bbbb2222"));
            collect_project_decode(&mut report, &opaque_session);
        }
        let before = serde_json::to_value(&report).expect("serialize before finalize");
        assert!(
            before.get("samples").is_none(),
            "tally state stays off the JSON surface: {before}"
        );
        report.finalize_samples();
        let _ = std::fs::remove_dir_all(root);

        assert_eq!(report.resolved, 6);
        assert_eq!(
            report.samples.len(),
            3,
            "one line per distinct identity: {:?}",
            report.samples
        );
        assert!(report.samples[0].starts_with("truncated: "));
        assert!(
            report.samples[0].contains("#aaaa1111"),
            "{}",
            report.samples[0]
        );
        assert!(report.samples[1].contains("#bbbb2222"));
        assert!(
            report.samples[0].ends_with("(2 sessions)"),
            "{}",
            report.samples[0]
        );
        assert!(report.samples[2].starts_with("opaque: projects/MyOpaque-Name_77"));
        assert!(
            report.samples[2].ends_with("(2 sessions)"),
            "{}",
            report.samples[2]
        );
    }

    #[test]
    fn snapshot_age_phrase_uses_singular_for_one_day() {
        // rm-512 grammar: the bundled snapshot reads "1 day old" on
        // release day; the unknown-age sentinel keeps its rendering.
        assert_eq!(snapshot_age_phrase(1), "1 day old");
        assert_eq!(snapshot_age_phrase(0), "0 days old");
        assert_eq!(snapshot_age_phrase(47), "47 days old");
        assert_eq!(snapshot_age_phrase(-1), "-1 days old");
    }

    #[test]
    fn disclosure_keys_render_sanitized() {
        // rm-595: the doctor disclosures arm mirrors counts_cell in
        // reports.rs — a hostile disclosure key carries no ESC/OSC
        // terminal-injection sequence into doctor text (PoC p6-ansi
        // class; mint sites sanitize too, this is the render choke point).
        let dir = std::env::temp_dir().join(format!(
            "agenttrace-doctor-render-test-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let mut report = build_doctor_report(Some(&dir), false);
        report
            .disclosures
            .insert("ansi\u{1b}]52;c;cHduYWdlPWNhdA==\u{7}future".to_string(), 3);
        let text = doctor_report_text(&report);
        assert!(!text.contains('\u{1b}'), "ESC leaked into doctor text");
        assert!(!text.contains('\u{7}'), "BEL leaked into doctor text");
        assert!(
            text.contains("ansi\u{fffd}]52;c;cHduYWdlPWNhdA==\u{fffd}future=3"),
            "sanitized disclosure key visible in doctor text"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn demo_directory_lane_folds_every_disclosure_channel() {
        // rm-596 × rm-526 × rm-862: the demo arm must fold the bundled
        // corpus through BOTH disclosure channels — informational
        // `disclosure_counters` and loss-tier `line_skips` — exactly
        // like the non-demo fold below it. The demo arm was authored
        // before rm-526 landed, so a demo fixture carrying a torn tail
        // would otherwise silently vanish from the demo doctor report
        // (the rm-449 nothing-silent contract); since rm-862 the two
        // channels land in SEPARATE maps so the render can label parse
        // loss as parse loss.
        let mut session = session_at("", "demo://torn-tail.jsonl");
        session
            .metrics
            .disclosure_counters
            .insert("pi_usage_entry:session".to_string(), 2);
        session
            .metrics
            .line_skips
            .insert("unparseable_line".to_string(), 3);
        let mut scan = DoctorScanState::default();
        let dirs = doctor_directories(
            None,
            true,
            &[],
            &[session],
            &SqliteIngestReport::default(),
            &mut scan,
        );
        assert!(dirs.is_empty(), "demo enumerates no real root: {dirs:?}");
        assert_eq!(
            scan.disclosure_fold
                .disclosures
                .get("pi_usage_entry:session"),
            Some(&2),
            "disclosure_counters fold: {:?}",
            scan.disclosure_fold.disclosures
        );
        assert_eq!(
            scan.disclosure_fold.dropped_lines.get("unparseable_line"),
            Some(&3),
            "line_skips fold into the loss tier (rm-526 channel, rm-862 map): {:?}",
            scan.disclosure_fold.dropped_lines
        );
        assert!(
            !scan
                .disclosure_fold
                .disclosures
                .contains_key("unparseable_line"),
            "rm-862: parse loss must not masquerade as a disclosed fact: {:?}",
            scan.disclosure_fold.disclosures
        );
    }
}

#[cfg(test)]
mod shell_quote_tests {
    use super::*;

    #[test]
    fn shell_quote_wraps_spaces_and_escapes_embedded_quotes() {
        assert_eq!(
            shell_quote(std::path::Path::new("/tmp/plain/usage.v3.db")),
            "'/tmp/plain/usage.v3.db'"
        );
        assert_eq!(
            shell_quote(std::path::Path::new("/tmp/my cache/usage.v3.db")),
            "'/tmp/my cache/usage.v3.db'"
        );
        assert_eq!(
            shell_quote(std::path::Path::new("/tmp/agent's data/usage.v3.db")),
            "'/tmp/agent'\\''s data/usage.v3.db'"
        );
    }

    #[test]
    fn sqlite_repair_hint_quotes_the_interpolated_path() {
        let mut report = DoctorReport::default();
        report
            .sqlite_read_failures
            .push(crate::sqlite_sessions::SqliteUnreadableDb {
                path: std::path::PathBuf::from("/tmp/my cache/agent's data/usage.v3.db"),
                source: "sqlite",
                reason: String::from("file is not a database"),
            });
        let lines = doctor_recommendations(&report, None, false);
        let hint = lines
            .iter()
            .find(|line| line.contains("sqlite3 "))
            .expect("sqlite repair hint present");
        assert!(
            hint.contains(
                "sqlite3 '/tmp/my cache/agent'\\''s data/usage.v3.db' \"pragma integrity_check;\""
            ),
            "rm-858: pasted repair command must quote the interpolated path: {hint}"
        );
    }
}

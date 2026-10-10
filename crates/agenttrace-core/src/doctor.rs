use crate::{
    cached_session, find_session_files, known_session_dirs, load_session_cache,
    load_sqlite_backed_sessions_reported, parse_file, save_session_cache, store_session, Session,
    SqliteIngestReport, VERSION,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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
    /// Parse-time journal disclosures aggregated over scanned sessions
    /// (rm-436/rm-437, pi-family journals): `pi_usage_entry:<kind>`,
    /// `pi_branches`, `pi_entry_skipped:<type>`,
    /// `pi_message_role:<role>`, and — since rm-526 — the `line_skips`
    /// channel (`unparseable_line`, `non_object_line`,
    /// `codex_ignorable_line`, …): a torn tail now discloses here instead
    /// of silently vanishing in a format parser. Journal facts the
    /// accounting deliberately does not count, kept visible; empty for
    /// corpora without such journals.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub disclosures: BTreeMap<String, usize>,
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
    pub sessions: usize,
    pub session_files: usize,
    /// rm-607 disclosure: sibling sqlite databases (outside the
    /// canonical ones) whose sessions were suppressed as cross-file
    /// duplicates — backups and copies that used to double every
    /// aggregate before the dedup landed. Empty when no sibling db
    /// matched the discovery glob.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sqlite_duplicate_dbs: Vec<crate::sqlite_sessions::SqliteDuplicateDb>,
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
    pub cache_hits: usize,
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
struct DoctorDiscovery {
    cache: SessionCacheReport,
    cache_size_bytes: u64,
    files: Vec<PathBuf>,
    sessions: Vec<Session>,
    sqlite_ingest: SqliteIngestReport,
    opencode_fork_excluded: usize,
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
        return DoctorDiscovery {
            cache: SessionCacheReport {
                path: session_cache_path(),
                entries: BTreeMap::new(),
                dirs: 0,
            },
            cache_size_bytes: 0,
            files: Vec::new(),
            sessions: crate::demo_sessions().expect("bundled demo corpus"),
            sqlite_ingest: SqliteIngestReport::default(),
            opencode_fork_excluded: 0,
        };
    }
    let cache = load_session_cache_report();
    let cache_size_bytes = std::fs::metadata(&cache.path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let (files, opencode_fork_excluded) = if dir.is_none() {
        find_reportable_session_files(None)
    } else {
        (find_session_files(dir), 0)
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
        load_sqlite_backed_sessions_reported(None)
    } else {
        (Vec::new(), SqliteIngestReport::default())
    };
    DoctorDiscovery {
        cache,
        cache_size_bytes,
        files,
        sessions,
        sqlite_ingest,
        opencode_fork_excluded,
    }
}

pub fn build_doctor_report(dir: Option<&Path>, demo: bool) -> DoctorReport {
    let DoctorDiscovery {
        cache,
        cache_size_bytes,
        files,
        sessions,
        sqlite_ingest,
        opencode_fork_excluded,
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
    let mut project_decode = DoctorProjectDecodeReport::default();
    let mut zero_usage = DoctorZeroUsageReport::default();
    let mut disclosures = BTreeMap::new();
    let directories = doctor_directories(
        dir,
        demo,
        &files,
        &sessions,
        &sqlite_ingest,
        &mut project_decode,
        &mut zero_usage,
        &mut disclosures,
    );
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
        sessions: files.len() + sessions.len(),
        session_files: files.len(),
        sqlite_duplicate_dbs: sqlite_ingest.duplicate_dbs,
        project_decode,
        zero_usage,
        disclosures,
        directories,
        statusline: doctor_statusline_report(demo),
        pricing: format!(
            "LiteLLM snapshot {} (bundled, {} models, {})",
            crate::pricing::bundled_snapshot_date(),
            crate::pricing::bundled_snapshot_model_count(),
            snapshot_age_phrase(crate::pricing::bundled_snapshot_age_days().unwrap_or(-1))
        ) + &doctor_deprecation_suffix(),
        sqlite_read_failures: sqlite_ingest.unreadable,
        recommendations: Vec::new(),
    };
    report.recommendations = doctor_recommendations(&report, dir, demo);
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

fn find_reportable_session_files(dir: Option<&Path>) -> (Vec<PathBuf>, usize) {
    if dir.is_some() {
        return (find_session_files(dir), 0);
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
    let (files, opencode_fork_excluded) =
        crate::discovery::find_session_files_cached(None, &mut cache, true);
    // A cache write failure must never fail the doctor (the cache is
    // an optimization; the inventory answer is already correct).
    let _ = crate::session_cache::save_session_cache(&mut cache);
    (files, opencode_fork_excluded)
}

// rm-753 × rm-596 composition: the landed demo gate and the sqlite
// ingest report each added a parameter to a function that already carried
// the decode/zero-usage/disclosure accumulators — bundled into a struct it
// would mirror `DoctorInputs` one floor up, so the fleet-precedent allow
// (discovery.rs `report_load_progress`, governance.rs `recommendation`) is
// the smaller honest surface.
#[allow(clippy::too_many_arguments)]
fn doctor_directories(
    dir: Option<&Path>,
    demo: bool,
    files: &[PathBuf],
    sessions: &[Session],
    sqlite_ingest: &SqliteIngestReport,
    project_decode: &mut DoctorProjectDecodeReport,
    zero_usage: &mut DoctorZeroUsageReport,
    disclosures: &mut BTreeMap<String, usize>,
) -> Vec<DoctorDirReport> {
    if demo {
        // rm-596 demo gate: enumerate no real root — both
        // `known_session_dirs()` and `doctor_sqlite_directories()` probe
        // the operator's home (paths, `is_dir()`/`is_file()` checks,
        // symlink targets). The bundled corpus is folded through the
        // SAME two channels as the non-demo fold below —
        // `disclosure_counters` and, since rm-526, `line_skips` — so a
        // scanned session discloses identically regardless of backing
        // store and the demo report still exercises that aggregate; a
        // demo fixture with a torn tail must not silently vanish here.
        for session in sessions {
            for (key, count) in &session.metrics.disclosure_counters {
                *disclosures.entry(key.clone()).or_insert(0) += count;
            }
            for (key, count) in &session.metrics.line_skips {
                *disclosures.entry(key.clone()).or_insert(0) += count;
            }
        }
        // rm-904: the bundled corpus runs through the same subagent
        // linkage pass as a real scan, so a demo report discloses an
        // orphaned subagent identically instead of silently dropping
        // the grain the CLI lane discloses (0 → no key, clean).
        disclose_unlinked_subagent_count(&mut sessions.to_vec(), disclosures);
        return Vec::new();
    }
    let mut cache = load_session_cache();
    let mut scanned_sessions: Vec<Session> = Vec::new();
    let mut dirs: Vec<DoctorDirReport> = Vec::new();
    if let Some(dir) = dir {
        let abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        dirs.push(doctor_dir_report(
            "custom",
            &abs,
            files,
            &mut cache,
            &mut scanned_sessions,
            project_decode,
            zero_usage,
            disclosures,
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
                &mut cache,
                &mut scanned_sessions,
                project_decode,
                zero_usage,
                disclosures,
            ));
        }
    }
    // rm-367 stage (a): persist fresh parses so an immediate --doctor
    // rerun on the same corpus reuses them (the loader lane's
    // discovery.rs save pattern) — before this the doctor READ the
    // cache but never wrote it, so every rerun re-parsed every
    // session from source. Stages b-d (finder-level reuse of the
    // walk itself) stay open on the row.
    if cache.is_dirty() {
        let _ = save_session_cache(&mut cache);
    }
    dirs.extend(doctor_sqlite_directories(sessions, sqlite_ingest));
    // SQLite-backed sessions (and, under `--demo`, the bundled corpus)
    // never flow through doctor_dir_report; fold their disclosure
    // counters in directly so every scanned session discloses
    // identically regardless of backing store.
    for session in sessions {
        for (key, count) in &session.metrics.disclosure_counters {
            *disclosures.entry(key.clone()).or_insert(0) += count;
        }
        // rm-526: `line_skips` (unparseable_line, non_object_line,
        // event_schema, codex_unparseable_line, …) is a disclosure
        // channel too — doctor surfaced only disclosure_counters, so
        // torn tails and format-parser drops were invisible here.
        // (rm-730 moved the codex structural counters —
        // codex_ignorable_line, codex_world_state — onto
        // disclosure_counters, so they reach this fold through the
        // loop above; true-loss counters stay on line_skips.)
        for (key, count) in &session.metrics.line_skips {
            *disclosures.entry(key.clone()).or_insert(0) += count;
        }
    }
    // rm-904: both lanes join the same subagent linkage pass the
    // loader runs (subagents::attribute_subagents, discovery.rs's
    // lane), so orphaned subagent transcripts disclose through
    // --doctor exactly as they do on every CLI report path
    // (main.rs disclose_unlinked_subagents) — 0-count-clean.
    scanned_sessions.extend(sessions.iter().cloned());
    disclose_unlinked_subagent_count(&mut scanned_sessions, disclosures);
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
    cache: &mut crate::SessionCache,
    scanned_sessions: &mut Vec<Session>,
    project_decode: &mut DoctorProjectDecodeReport,
    zero_usage: &mut DoctorZeroUsageReport,
    disclosures: &mut BTreeMap<String, usize>,
) -> DoctorDirReport {
    let mut parsed = 0;
    let mut cache_hits = 0;
    let mut failure_samples = Vec::new();
    for file in files {
        // Keep the parsed session where the decode disclosure (rm-381) can
        // see it; the parse accounting below is unchanged.
        let session = if let Some(session) = cached_session(file, cache) {
            cache_hits += 1;
            parsed += 1;
            Some(session)
        } else if let Ok(session) = parse_file(file) {
            parsed += 1;
            // rm-367 stage (a): persist the fresh parse so an
            // immediate --doctor rerun reuses it — the loader lane's
            // discovery.rs:382 pattern. The reuse shows up on the
            // NEXT run (cache_hits / cached_valid), not this one.
            let _ = store_session(file, &session, cache);
            Some(session)
        } else {
            if failure_samples.len() < 3 {
                failure_samples.push(file.to_string_lossy().to_string());
            }
            None
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
            for (key, count) in &session.metrics.disclosure_counters {
                *disclosures.entry(key.clone()).or_insert(0) += count;
            }
            // rm-526: same fold for line_skips — parse-failure and
            // drop disclosures (torn tails) reach --doctor through
            // this channel; they round-trip the cache the same way.
            for (key, count) in &session.metrics.line_skips {
                *disclosures.entry(key.clone()).or_insert(0) += count;
            }
        }
    }
    DoctorDirReport {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        exists: path.is_dir(),
        files: files.len(),
        parsed,
        failed: files.len().saturating_sub(parsed),
        cache_hits,
        symlink_target: symlink_target_of(path),
        failure_samples,
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
            symlink_target: symlink_target_of(&path),
            failure_samples,
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

fn doctor_report_text(report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("AGENTTRACE Doctor\n");
    out.push_str(&format!("Version: {}\n", report.version));
    out.push_str(&format!("Mode: {}\n", report.mode));
    out.push_str(&format!("Session files: {}\n", report.sessions));
    out.push_str(&format!("Cache: {}\n", report.cache_path));
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
        out.push_str(&format!("    {sample}\n"));
    }
    out.push_str(&format!(
        "Zero-usage reports: {} session(s), {} event(s) counted as measured zeros (rm-408)\n",
        report.zero_usage.sessions, report.zero_usage.events
    ));
    for sample in &report.zero_usage.samples {
        out.push_str(&format!("    {sample}\n"));
    }
    if !report.sqlite_duplicate_dbs.is_empty() {
        out.push_str(&format!(
            "Duplicate sqlite databases (rm-607): {}\n",
            report
                .sqlite_duplicate_dbs
                .iter()
                .map(|db| format!(
                    "    {} — {} duplicate session(s) suppressed, canonical db kept\n",
                    db.path, db.duplicate_sessions
                ))
                .collect::<String>()
        ));
    }
    if !report.disclosures.is_empty() {
        // rm-595: disclosure keys route through the control-byte
        // sanitizer at render — mirror of counts_cell in reports.rs; a
        // hostile key carries no ESC/OSC sequence into doctor text.
        // rm-594 residual: the same single-line bound counts_cell
        // applies — first 40 entries plus an explicit count of what is
        // held back (the 4.6MB cardinality PoC rendered 2.15MB here).
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
        out.push_str(&format!("Disclosed facts: {}\n", parts.join(", ")));
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
    SessionCacheReport {
        path,
        entries,
        dirs,
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
        // rm-596 × rm-526: the demo arm must fold the bundled corpus
        // through BOTH disclosure channels — `disclosure_counters` and
        // `line_skips` — exactly like the non-demo fold below it. The
        // demo arm was authored before rm-526 landed, so a demo fixture
        // carrying a torn tail would otherwise silently vanish from the
        // demo doctor report (the rm-449 nothing-silent contract).
        let mut session = session_at("", "demo://torn-tail.jsonl");
        session
            .metrics
            .disclosure_counters
            .insert("pi_usage_entry:session".to_string(), 2);
        session
            .metrics
            .line_skips
            .insert("unparseable_line".to_string(), 3);
        let mut project_decode = DoctorProjectDecodeReport::default();
        let mut zero_usage = DoctorZeroUsageReport::default();
        let mut disclosures = BTreeMap::new();
        let dirs = doctor_directories(
            None,
            true,
            &[],
            &[session],
            &SqliteIngestReport::default(),
            &mut project_decode,
            &mut zero_usage,
            &mut disclosures,
        );
        assert!(dirs.is_empty(), "demo enumerates no real root: {dirs:?}");
        assert_eq!(
            disclosures.get("pi_usage_entry:session"),
            Some(&2),
            "disclosure_counters fold: {disclosures:?}"
        );
        assert_eq!(
            disclosures.get("unparseable_line"),
            Some(&3),
            "line_skips fold (rm-526 channel): {disclosures:?}"
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

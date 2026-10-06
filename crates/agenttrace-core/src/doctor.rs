use crate::{
    cached_session, find_session_files, known_session_dirs, load_session_cache, parse_file,
    skip_sqlite_backed_file_dir, Session, VERSION,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
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
    /// `pi_message_role:<role>`. Journal facts the accounting
    /// deliberately does not count, kept visible; empty for corpora
    /// without such journals.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub disclosures: BTreeMap<String, usize>,
    /// On-disk size of `sessions.json`, zero when absent.
    pub cache_size_bytes: u64,
    /// The hard bounds `save_session_cache` enforces before serializing
    /// (entries and serialized bytes; pass-9 CU-22).
    pub cache_limits: String,
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
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorStatuslineReport {
    pub path: String,
    pub exists: bool,
    pub captures: usize,
    pub sessions: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
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

pub fn build_doctor_report(dir: Option<&Path>, demo: bool) -> DoctorReport {
    let cache = load_session_cache_report();
    let files = if dir.is_none() {
        find_reportable_session_files(None)
    } else {
        find_session_files(dir)
    };
    let sqlite_load = if dir.is_none() && !demo {
        crate::sqlite_sessions::load_sqlite_backed_sessions_report(None)
    } else {
        crate::sqlite_sessions::SqliteLoadReport::default()
    };
    let sqlite_sessions = sqlite_load.sessions;
    let cached_valid = valid_cached_session_count(&files, &cache);
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
        &files,
        &sqlite_sessions,
        &mut project_decode,
        &mut zero_usage,
        &mut disclosures,
    );
    // rm-512: the walk records per-project tallies; render them into the
    // deduplicated sample lines before the report leaves this function.
    project_decode.finalize_samples();
    let mut report = DoctorReport {
        version: VERSION.to_string(),
        mode: mode.to_string(),
        cache_path: cache.path.to_string_lossy().to_string(),
        cache_entries: cache.entries.len(),
        cache_dirs: cache.dirs,
        cached_valid,
        cache_size_bytes: std::fs::metadata(&cache.path)
            .map(|metadata| metadata.len())
            .unwrap_or(0),
        cache_limits: format!(
            "entries<={}, bytes<={}",
            crate::session_cache::MAX_SESSION_CACHE_ENTRIES,
            crate::session_cache::MAX_SESSION_CACHE_BYTES
        ),
        sessions: files.len() + sqlite_sessions.len(),
        session_files: files.len(),
        sqlite_duplicate_dbs: sqlite_load.duplicate_dbs,
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
    /// First-seen order of distinct sample projects (`true` = ambiguous).
    #[serde(skip)]
    sample_order: Vec<(bool, String)>,
}

impl DoctorProjectDecodeReport {
    fn record_ambiguous(&mut self, path: &str, shadowed: String) {
        match self.ambiguous_tally.entry(path.to_string()) {
            Entry::Occupied(mut entry) => entry.get_mut().0 += 1,
            Entry::Vacant(vacant) => {
                vacant.insert((1, shadowed));
                self.sample_order.push((true, path.to_string()));
            }
        }
    }

    fn record_unresolved(&mut self, encoded: &str) {
        match self.unresolved_tally.entry(encoded.to_string()) {
            Entry::Occupied(mut entry) => *entry.get_mut() += 1,
            Entry::Vacant(vacant) => {
                vacant.insert(1);
                self.sample_order.push((false, encoded.to_string()));
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
        for (ambiguous, project) in &self.sample_order {
            if samples.len() >= 6 {
                break;
            }
            let (line, count) = if *ambiguous {
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
            } else {
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

fn find_reportable_session_files(dir: Option<&Path>) -> Vec<PathBuf> {
    if dir.is_some() {
        return find_session_files(dir);
    }
    let mut out = Vec::new();
    for candidate in crate::discover_session_dirs() {
        if skip_sqlite_backed_file_dir(&candidate) {
            continue;
        }
        out.extend(crate::collect_session_files(&candidate));
    }
    out
}

fn doctor_directories(
    dir: Option<&Path>,
    files: &[PathBuf],
    sqlite_sessions: &[Session],
    project_decode: &mut DoctorProjectDecodeReport,
    zero_usage: &mut DoctorZeroUsageReport,
    disclosures: &mut BTreeMap<String, usize>,
) -> Vec<DoctorDirReport> {
    let mut cache = load_session_cache();
    if let Some(dir) = dir {
        let abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        return vec![doctor_dir_report(
            "custom",
            &abs,
            files,
            &mut cache,
            project_decode,
            zero_usage,
            disclosures,
        )];
    }

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

    let mut dirs = Vec::new();
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
            project_decode,
            zero_usage,
            disclosures,
        ));
    }
    dirs.extend(doctor_sqlite_directories(sqlite_sessions));
    // SQLite-backed sessions never flow through doctor_dir_report; fold
    // their disclosure counters in directly so every scanned session
    // discloses identically regardless of backing store.
    for session in sqlite_sessions {
        for (key, count) in &session.metrics.disclosure_counters {
            *disclosures.entry(key.clone()).or_insert(0) += count;
        }
    }
    dirs
}

fn doctor_dir_report(
    name: &str,
    path: &Path,
    files: &[PathBuf],
    cache: &mut crate::SessionCache,
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
            Some(session)
        } else {
            if failure_samples.len() < 3 {
                failure_samples.push(file.to_string_lossy().to_string());
            }
            None
        };
        if let Some(session) = session {
            collect_project_decode(project_decode, &session);
            collect_zero_usage(zero_usage, &session);
            // rm-436/rm-437: cache-hit and freshly parsed sessions
            // disclose identically — the counters round-trip through
            // the session cache (GoMetrics), so a warm doctor scan
            // cannot silently lose them.
            for (key, count) in &session.metrics.disclosure_counters {
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

fn doctor_sqlite_directories(sessions: &[Session]) -> Vec<DoctorDirReport> {
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
        let files = count_by_tool.get(tool).copied().unwrap_or(0);
        let exists = path.is_file();
        if !exists && files == 0 {
            continue;
        }
        dirs.push(DoctorDirReport {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            exists,
            files,
            parsed: files,
            failed: 0,
            cache_hits: 0,
            symlink_target: symlink_target_of(&path),
            failure_samples: Vec::new(),
        });
    }
    dirs
}

fn doctor_recommendations(report: &DoctorReport, dir: Option<&Path>, demo: bool) -> Vec<String> {
    if report.sessions == 0 {
        if dir.is_some() {
            return vec!["No sessions found in this directory. Check `-d <dir>` or point it at a session JSON/JSONL directory.".to_string()];
        }
        return vec![
            "No sessions found. Run `agenttrace --demo` to try the TUI immediately.".to_string(),
        ];
    }
    let mut recommendations = vec![
        "Ready: run `agenttrace` for the TUI or `agenttrace --overview -f json` for automation."
            .to_string(),
    ];
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
    let duplicate_dbs = report
        .sqlite_duplicate_dbs
        .iter()
        .map(|db| db.path.as_str())
        .collect::<Vec<_>>()
        .len();
    let duplicate_sessions: usize = report
        .sqlite_duplicate_dbs
        .iter()
        .map(|db| db.duplicate_sessions)
        .sum();
    if duplicate_sessions > 0 {
        recommendations.push(format!(
            "{} session(s) across {} sibling sqlite database(s) ({}) were counted once because they repeat sessions already carried by the canonical database; archive backups outside the agent home to keep discovery unambiguous.",
            duplicate_sessions,
            duplicate_dbs,
            report
                .sqlite_duplicate_dbs
                .iter()
                .map(|db| db.path.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if demo {
        recommendations.push(
            "Demo sessions use a temporary directory, so cache reuse is not expected in this mode."
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
        "  cache size {} bytes, hard bounds: {} (oldest-source entries evicted first)\n",
        report.cache_size_bytes, report.cache_limits
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
        out.push_str(&format!(
            "Journal disclosures: {}\n",
            report
                .disclosures
                .iter()
                .map(|(key, count)| format!("{key}={count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
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
            sqlite_session_id: String::new(),
            name: "session.jsonl".to_string(),
            path: path.to_string(),
            cwd: cwd.to_string(),
            metrics: crate::Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        }
    }

    /// A session whose encoded project dir cannot decode (rm-512
    /// fixtures): the path sits under a nonexistent root, so the walk
    /// finds no verified decode and the session is `Unresolved`.
    fn unresolved_session(encoded_dir: &str) -> Session {
        session_at("", &format!("/nowhere/projects/-{encoded_dir}/s.jsonl"))
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

        assert_eq!(report.unresolved, 6);
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

        assert_eq!(report.unresolved, 7);
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
            Some(&serde_json::json!(2)),
            "counters unchanged: {after}"
        );
        let samples = after
            .get("samples")
            .expect("samples render after finalize")
            .as_array()
            .expect("samples stay an array");
        assert_eq!(samples.len(), 1);
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
}

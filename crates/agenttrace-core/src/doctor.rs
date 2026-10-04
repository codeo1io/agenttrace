use crate::{
    cached_session, find_session_files, known_session_dirs, load_session_cache,
    load_sqlite_backed_sessions, parse_file, skip_sqlite_backed_file_dir, Session, VERSION,
};
use serde::Serialize;
use serde_json::Value;
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
    let sqlite_sessions = if dir.is_none() && !demo {
        load_sqlite_backed_sessions()
    } else {
        Vec::new()
    };
    let cached_valid = valid_cached_session_count(&files, &cache);
    let mode = if demo {
        "demo sessions"
    } else if dir.is_some() {
        "custom directory"
    } else {
        "auto-discovery"
    };
    let mut project_decode = DoctorProjectDecodeReport::default();
    let mut disclosures = BTreeMap::new();
    let directories = doctor_directories(
        dir,
        &files,
        &sqlite_sessions,
        &mut project_decode,
        &mut disclosures,
    );
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
        project_decode,
        disclosures,
        directories,
        statusline: doctor_statusline_report(demo),
        pricing: format!(
            "LiteLLM snapshot {} (bundled, {} models, {} days old)",
            crate::pricing::bundled_snapshot_date(),
            crate::pricing::bundled_snapshot_model_count(),
            crate::pricing::bundled_snapshot_age_days().unwrap_or(-1)
        ),
        recommendations: Vec::new(),
    };
    report.recommendations = doctor_recommendations(&report, dir, demo);
    report
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
    /// Up to six sanitized examples (ambiguous and unresolved interleaved,
    /// in scan order).
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
            if out.samples.len() < 6 {
                let shadowed = shadowed
                    .iter()
                    .map(|alt| crate::statusline::sanitize_line_segment(alt))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.samples.push(format!(
                    "ambiguous: {} attributed via the longest-verified-path rule; verified alternatives shadowed: {}",
                    crate::statusline::sanitize_line_segment(&path),
                    shadowed
                ));
            }
        }
        ProjectDecodeStatus::Unresolved { encoded } => {
            out.unresolved += 1;
            if out.samples.len() < 6 {
                out.samples.push(format!(
                    "unresolved: projects/{} has no verified decode; session attributed to `unknown`",
                    crate::statusline::sanitize_line_segment(&encoded)
                ));
            }
        }
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

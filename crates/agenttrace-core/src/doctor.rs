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
    /// rm-394 disclosure: project display names that collide on basename
    /// across distinct roots and are therefore disambiguated with their
    /// root in every rollup (`api (/home/alice/api)` …). Empty when every
    /// project name is unique in this scan.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub project_name_collisions: Vec<String>,
    /// rm-393/rm-396 disclosure: everything the discovery walk or the
    /// parser skipped — unreadable directories (with the OS error) and
    /// oversized session files (with the byte size) — so a corpus that
    /// silently shrank now says so instead of reporting a healthy scan.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub disclosed_skips: Vec<crate::disclosed::DisclosedSkip>,
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
    // rm-394: full-identity map (root -> basename) over every parsed
    // session, the input for the basename-collision disclosure.
    let mut project_identities: BTreeMap<String, String> = BTreeMap::new();
    let directories = doctor_directories(
        dir,
        &files,
        &sqlite_sessions,
        &mut project_decode,
        &mut project_identities,
    );
    for session in &sqlite_sessions {
        let identity = crate::insights::resolve_project(session);
        project_identities.insert(identity.id.clone(), identity.display_name.clone());
    }
    let mut by_basename: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (id, basename) in &project_identities {
        // "unknown" is a single unattributed bucket, not a collision.
        if id == "unknown" {
            continue;
        }
        by_basename
            .entry(basename.as_str())
            .or_default()
            .push(id.as_str());
    }
    let project_collisions: Vec<String> = by_basename
        .into_iter()
        .filter(|(_basename, roots)| roots.len() > 1)
        .map(|(basename, roots)| format!("{basename} — {}", roots.join("; ")))
        .collect();
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
        // rm-394: collision disclosure is computed over every parsed
        // session (directories below + sqlite-backed), so it rides the
        // same pass as the decode report.
        project_name_collisions: project_collisions,
        // rm-393/rm-396: everything the walk and the parse skipped this
        // scan, with the reason and (for oversized files) the size.
        disclosed_skips: crate::disclosed::snapshot(),
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
    // rm-393/rm-396: skips must read as actionable, not as healthy.
    if !report.disclosed_skips.is_empty() {
        report.recommendations.push(format!(
            "{} session file(s)/director(ies) were skipped during this scan (unreadable or oversized); counts above are computed over the readable corpus only",
            report.disclosed_skips.len()
        ));
    }
    // rm-394: colliding project names are disambiguated in every rollup.
    if !report.project_name_collisions.is_empty() {
        report.recommendations.push(format!(
            "{} project name(s) collide across distinct roots and are disambiguated with their root path in rollups",
            report.project_name_collisions.len()
        ));
    }
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
    project_identities: &mut BTreeMap<String, String>,
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
            project_identities,
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
            project_identities,
        ));
    }
    dirs.extend(doctor_sqlite_directories(sqlite_sessions));
    dirs
}

fn doctor_dir_report(
    name: &str,
    path: &Path,
    files: &[PathBuf],
    cache: &mut crate::SessionCache,
    project_decode: &mut DoctorProjectDecodeReport,
    project_identities: &mut BTreeMap<String, String>,
) -> DoctorDirReport {
    let mut parsed = 0;
    let mut cache_hits = 0;
    let mut oversized_skipped = 0;
    let mut failure_samples = Vec::new();
    for file in files {
        // Keep the parsed session where the decode disclosure (rm-381) can
        // see it; the parse accounting below is unchanged apart from the
        // rm-396 oversized class, which is a disclosed skip, not a failure.
        let session = if let Some(session) = cached_session(file, cache) {
            cache_hits += 1;
            parsed += 1;
            Some(session)
        } else {
            match parse_file(file) {
                Ok(session) => {
                    parsed += 1;
                    Some(session)
                }
                Err(error)
                    if error
                        .downcast_ref::<crate::parser::SessionFileTooLargeError>()
                        .is_some() =>
                {
                    oversized_skipped += 1;
                    if let Some(oversized) =
                        error.downcast_ref::<crate::parser::SessionFileTooLargeError>()
                    {
                        crate::disclosed::record(
                            crate::disclosed::DisclosedSkipKind::OversizedFile,
                            file,
                            format!(
                                "{} bytes above the {} byte parse bound",
                                oversized.size, oversized.bound
                            ),
                        );
                    }
                    None
                }
                Err(_) => {
                    if failure_samples.len() < 3 {
                        failure_samples.push(file.to_string_lossy().to_string());
                    }
                    None
                }
            }
        };
        if let Some(session) = session {
            collect_project_decode(project_decode, &session);
            let identity = crate::insights::resolve_project(&session);
            project_identities.insert(identity.id.clone(), identity.display_name.clone());
        }
    }
    DoctorDirReport {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        exists: path.is_dir(),
        files: files.len(),
        parsed,
        // rm-396: disclosed oversized skips are neither parsed nor failed;
        // they ride the top-level disclosed-skips disclosure with sizes.
        failed: files.len().saturating_sub(parsed + oversized_skipped),
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
    // rm-394: colliding basenames are disambiguated with their root in
    // every rollup; say so instead of letting two rows look arbitrary.
    for collision in &report.project_name_collisions {
        out.push_str(&format!(
            "    name collision (rows disambiguated by root): {collision}\n"
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
    // rm-393/rm-396: a corpus that shrank says so, with the reason and
    // (for oversized files) the size — never a silent healthy scan.
    if !report.disclosed_skips.is_empty() {
        out.push_str(&format!(
            "\nDisclosed skips: {}\n",
            report.disclosed_skips.len()
        ));
        for skip in &report.disclosed_skips {
            out.push_str(&format!(
                "  - {}: {} ({})\n",
                skip.kind.label(),
                skip.path,
                skip.detail
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

    fn pi_session(cwd: &str) -> String {
        // Minimal parseable pi session carrying the project cwd (built by
        // substitution, not format!: the JSON braces would all need
        // escaping).
        r#"{"type":"session","version":3,"id":"1","cwd":"__CWD__"}
{"type":"message","timestamp":"2026-01-01T00:00:00Z","message":{"role":"user","content":"task"}}"#
            .replace("__CWD__", cwd)
            + "\n"
    }

    /// rm-392..396 batch integration: a corpus holding an unreadable
    /// directory, an oversized file, and two projects sharing a basename
    /// must produce a doctor report that discloses all three — not a
    /// green scan over a silently shrunken corpus.
    #[test]
    #[cfg(unix)]
    fn doctor_discloses_shrinks_and_collisions() {
        use crate::parser::DEFAULT_MAX_SESSION_FILE_BYTES;

        let _guard = crate::parser::OVERSIZE_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = std::env::temp_dir().join(format!(
            "at-doctor-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let alice = root.join("alice").join("api");
        let bob = root.join("bob").join("api");
        std::fs::create_dir_all(&alice).expect("alice fixture");
        std::fs::create_dir_all(&bob).expect("bob fixture");
        std::fs::create_dir_all(root.join("secret")).expect("secret fixture");
        std::fs::write(root.join("good.jsonl"), pi_session(alice.to_str().unwrap()))
            .expect("good1");
        std::fs::write(root.join("good2.jsonl"), pi_session(bob.to_str().unwrap())).expect("good2");
        std::fs::write(root.join("secret").join("hidden.jsonl"), "{}\n").expect("hidden");
        {
            let file = std::fs::File::create(root.join("huge.jsonl")).expect("huge create");
            file.set_len(DEFAULT_MAX_SESSION_FILE_BYTES + 1)
                .expect("huge sparse len");
        }
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(root.join("secret"))
            .unwrap()
            .permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(root.join("secret"), perms).expect("chmod");

        let report = build_doctor_report(Some(&root), false);
        let mine: Vec<_> = report
            .disclosed_skips
            .iter()
            .filter(|skip| skip.path.starts_with(root.to_string_lossy().as_ref()))
            .collect();
        assert!(
            mine.iter().any(|skip| skip.kind
                == crate::disclosed::DisclosedSkipKind::UnreadableDirectory
                && skip.path.ends_with("secret")),
            "unreadable secret must be disclosed: {mine:?}"
        );
        assert!(
            mine.iter().any(
                |skip| skip.kind == crate::disclosed::DisclosedSkipKind::OversizedFile
                    && skip.path.ends_with("huge.jsonl")
                    && skip.detail.contains("above the")
            ),
            "oversized huge.jsonl must be disclosed with its size: {mine:?}"
        );
        assert!(
            !report.project_name_collisions.is_empty()
                && report
                    .project_name_collisions
                    .iter()
                    .all(|collision| collision.starts_with("api — ")),
            "colliding api basenames must be disclosed with both roots: {:?}",
            report.project_name_collisions
        );
        let text = render_doctor_report(Some(&root), false, "text").expect("render doctor");
        assert!(
            text.contains("Disclosed skips:"),
            "text shows the skip section"
        );
        assert!(text.contains("huge.jsonl") && text.contains("secret"));
        assert!(text.contains("name collision"));

        // Restore for cleanup regardless of assertions above.
        let mut perms = std::fs::metadata(root.join("secret"))
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(root.join("secret"), perms);
        let _ = std::fs::remove_dir_all(&root);
    }
}

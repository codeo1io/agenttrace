//! Claude Code statusline capture mode (roadmap candidate 53, cycle 7).
//!
//! Claude Code's status line invokes a command with the session payload
//! on stdin (<https://code.claude.com/docs/en/statusline>, fields gated
//! at v2.1.251+). That payload is the only local channel that carries
//! subscription limit pressure (`rate_limits.*`) and authoritative
//! prompt-cache analytics (`prompt_cache.*`) — two inputs session
//! transcripts never record. `agenttrace statusline` acts as the host
//! command: it answers with a one-line status (the status line must
//! never be left blank or spinning), tees the payload to a bounded
//! local JSONL journal, and the journal is aggregated into reports and
//! the TUI keyed by `session_id`.
//!
//! Host-safety contract (the status line runs inside Claude Code on
//! every prompt): `run_statusline_host` never fails — invalid or empty
//! stdin still prints a line, diagnostics go to stderr only, the tee is
//! best-effort, and the process exits 0.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Retention bound for the capture journal: when the file crosses this
/// size the oldest lines are dropped until it fits half of it. Ten MiB
/// of JSONL covers months of prompts at the documented payload size.
pub const STATUSLINE_CAPTURE_MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Hard cap on one payload read from stdin. The documented payload is a
/// few KiB; anything larger is a misconfigured host and is not captured.
const STATUSLINE_INPUT_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StatuslineRateLimitState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StatuslineMissCause {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_added: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_removed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StatuslinePromptCache {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warm: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub misses: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_rebuilds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub miss_recache_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_miss_cause: Option<StatuslineMissCause>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub miss_causes: Option<BTreeMap<String, u64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recache_tokens_if_cold: Option<u64>,
}

/// One captured payload plus the local wall time of the capture. The
/// journal never rewrites history: compaction drops whole oldest lines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapturedStatusline {
    pub captured_at: i64,
    pub payload: Value,
}

impl CapturedStatusline {
    pub fn session_id(&self) -> Option<&str> {
        self.payload.get("session_id").and_then(Value::as_str)
    }

    fn rate_limit(&self, window: &str) -> Option<StatuslineRateLimitState> {
        serde_json::from_value(self.payload.get("rate_limits")?.get(window)?.clone()).ok()
    }

    fn prompt_cache(&self) -> Option<StatuslinePromptCache> {
        serde_json::from_value(self.payload.get("prompt_cache")?.clone()).ok()
    }
}

/// Where the journal lives. Follows the session-cache root so
/// `AGENTTRACE_SESSION_CACHE_DIR` relocates both, and `--clear-cache`
/// semantics (separate file) stay obvious.
pub fn statusline_capture_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR").map(PathBuf::from) {
        if !dir.as_os_str().is_empty() {
            return dir.join("statusline.jsonl");
        }
    }
    user_cache_dir().join("agenttrace").join("statusline.jsonl")
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

/// The statusline host entry point. Contract: never fails, never blocks
/// (no session-cache locking, no discovery), prints exactly one line to
/// stdout, reports problems on stderr only.
pub fn run_statusline_host() -> anyhow::Result<()> {
    let mut input = String::new();
    let read = io::stdin()
        .lock()
        .take(STATUSLINE_INPUT_MAX_BYTES)
        .read_to_string(&mut input);
    // Same F2 class: stderr diagnostics must not be able to panic the
    // host either, so they go through the write-and-ignore form.
    let payload = match read {
        Ok(_) if input.trim().is_empty() => None,
        Ok(_) => match serde_json::from_str::<Value>(&input) {
            Ok(value) => Some(value),
            Err(err) => {
                let _ = writeln!(
                    io::stderr(),
                    "agenttrace statusline: stdin was not JSON ({err}); not captured."
                );
                None
            }
        },
        Err(err) => {
            let _ = writeln!(
                io::stderr(),
                "agenttrace statusline: could not read stdin ({err})."
            );
            None
        }
    };
    let line = payload
        .as_ref()
        .map(render_status_line)
        .unwrap_or_else(|| "agenttrace".to_string());
    // Review F2 (cycle 7): the host contract is "never fails the host" —
    // println! panics when the write fails (a full disk via /dev/full, or
    // EPIPE when the host closes the pipe), which turned a display
    // problem into exit 101. Write and ignore the error instead.
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    if let Some(payload) = payload {
        if let Err(err) = append_statusline_capture(&payload) {
            let _ = writeln!(
                io::stderr(),
                "agenttrace statusline: capture failed ({err})."
            );
        }
    }
    Ok(())
}

/// One line of status from the payload: model or session name, context
/// window pressure, subscription limit pressure with the next reset,
/// cache hit ratio, and running cost. Only fields that exist are shown.
fn render_status_line(payload: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    let name = payload
        .get("session_name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .map(sanitize_line_segment)
        .or_else(|| {
            payload
                .get("model")
                .and_then(|model| model.get("display_name"))
                .and_then(Value::as_str)
                .map(sanitize_line_segment)
        });
    if let Some(name) = name {
        parts.push(name);
    }
    if let Some(used) = payload
        .get("context_window")
        .and_then(|window| window.get("used_percentage"))
        .and_then(Value::as_f64)
    {
        let over = payload
            .get("exceeds_200k_tokens")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        parts.push(format!("ctx {used:.0}%{}", if over { " (!)" } else { "" }));
    }
    for (window, label) in [("five_hour", "5h"), ("seven_day", "7d")] {
        let state = serde_json::from_value::<StatuslineRateLimitState>(
            payload
                .get("rate_limits")
                .and_then(|limits| limits.get(window))
                .cloned()
                .unwrap_or(Value::Null),
        )
        .unwrap_or_default();
        if let Some(used) = state.used_percentage {
            let reset = state
                .resets_at
                .map(|epoch| {
                    chrono::DateTime::from_timestamp(epoch, 0)
                        .map(|at| at.format("%H:%M").to_string())
                        .unwrap_or_else(|| epoch.to_string())
                })
                .map(|at| format!(" until {at}"))
                .unwrap_or_default();
            parts.push(format!("{label} {used:.0}%{reset}"));
        }
    }
    if let Some(cache) = payload
        .get("prompt_cache")
        .and_then(|cache| cache.get("hit_ratio"))
        .and_then(Value::as_f64)
    {
        // hit_ratio is a 0..1 ratio, not a percentage.
        parts.push(format!("cache {:.0}%", cache * 100.0));
    }
    if let Some(cost) = payload
        .get("cost")
        .and_then(|cost| cost.get("total_cost_usd"))
        .and_then(Value::as_f64)
    {
        parts.push(format!("${cost:.2}"));
    }
    if parts.is_empty() {
        "agenttrace".to_string()
    } else {
        parts.join(" | ")
    }
}

/// Review F1 (cycle 7): payload strings are user-authored (session
/// names are chat titles) and go straight to the host terminal. A
/// newline would break the one-line contract; ESC/OSC/CSI sequences
/// would inject into the terminal. Control characters (C0, DEL, C1 —
/// exactly `char::is_control`) are replaced with U+FFFD so the line
/// stays one line and carries no escape sequences.
fn sanitize_line_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|c| if c.is_control() { '\u{FFFD}' } else { c })
        .collect()
}

/// Appends one capture to the journal, compacting it first when the
/// retention bound is crossed. Errors are the caller's business (the
/// host entry point turns them into stderr notes).
pub fn append_statusline_capture(payload: &Value) -> io::Result<()> {
    let path = statusline_capture_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(&CapturedStatusline {
        captured_at: chrono::Utc::now().timestamp(),
        payload: payload.clone(),
    })
    .unwrap_or_else(|_| "{}".to_string());
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let size_after_append = file.metadata()?.len() + line.len() as u64 + 1;
    writeln!(file, "{line}")?;
    if size_after_append > STATUSLINE_CAPTURE_MAX_BYTES {
        compact_statusline_capture(&path)?;
    }
    Ok(())
}

/// Retention: drop oldest whole lines until the journal fits half the
/// bound. Rewritten through a temp file + rename so a crash mid-compaction
/// cannot truncate the journal to zero.
fn compact_statusline_capture(path: &Path) -> io::Result<()> {
    compact_statusline_capture_under(path, STATUSLINE_CAPTURE_MAX_BYTES / 2)
}

fn compact_statusline_capture_under(path: &Path, keep_under: u64) -> io::Result<()> {
    let raw = fs::read_to_string(path)?;
    let mut kept: Vec<&str> = Vec::new();
    let mut kept_bytes = 0u64;
    for line in raw.lines().rev() {
        let bytes = line.len() as u64 + 1;
        if kept_bytes + bytes > keep_under && !kept.is_empty() {
            break;
        }
        kept_bytes += bytes;
        kept.push(line);
    }
    kept.reverse();
    let temp = path.with_extension("jsonl.compact");
    {
        let mut file = fs::File::create(&temp)?;
        for line in kept {
            writeln!(file, "{line}")?;
        }
        file.flush()?;
    }
    fs::rename(&temp, path)?;
    Ok(())
}

/// Reads the journal, skipping malformed lines (a torn tail line from a
/// crashed append is tolerated, matching append-only semantics).
pub fn read_statusline_captures(path: &Path) -> Vec<CapturedStatusline> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// One observed limit window: the usage seen just before `resets_at`
/// passed and the first usage observed after it. A crossing where usage
/// falls is the moment the subscription window actually reset.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StatuslineLimitCrossing {
    pub window: String,
    pub resets_at: i64,
    pub used_percentage_before: Option<f64>,
    pub used_percentage_after: Option<f64>,
}

/// Latest prompt-cache state for one session, keyed by `session_id`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StatuslineSessionCache {
    pub session_id: String,
    pub hit_ratio: Option<f64>,
    pub misses: Option<u64>,
    pub miss_recache_tokens: Option<u64>,
    pub recache_tokens_if_cold: Option<u64>,
    pub last_miss_causes: Vec<String>,
}

/// Aggregated view of the journal for reports and the TUI.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct StatuslineInsights {
    /// Distinct payloads seen, after exact-duplicate dedup (Claude Code
    /// debounces at 300 ms but identical prompts can still repeat).
    pub captures: usize,
    pub sessions: usize,
    /// Latest observed state per window.
    pub five_hour: Option<StatuslineRateLimitState>,
    pub seven_day: Option<StatuslineRateLimitState>,
    /// Peak usage per window across all captures.
    pub five_hour_peak_used_percentage: Option<f64>,
    pub seven_day_peak_used_percentage: Option<f64>,
    /// Windows whose `resets_at` has been observed passing.
    pub limit_crossings: Vec<StatuslineLimitCrossing>,
    /// Latest prompt-cache state per session, newest session first.
    pub session_caches: Vec<StatuslineSessionCache>,
    /// Total miss-cause counts across sessions.
    pub miss_causes: BTreeMap<String, u64>,
}

/// Deduplicates and aggregates captures. Captures are keyed by
/// `session_id` where present (transcripts join on the same IDs); exact
/// duplicate payloads collapse to the first occurrence.
pub fn statusline_insights(captures: &[CapturedStatusline]) -> StatuslineInsights {
    let mut seen_payloads: HashSet<String> = HashSet::new();
    let mut deduped: Vec<&CapturedStatusline> = Vec::new();
    for capture in captures {
        let key = serde_json::to_string(&capture.payload).unwrap_or_default();
        if seen_payloads.insert(key) {
            deduped.push(capture);
        }
    }
    let mut sessions: HashSet<&str> = HashSet::new();
    for capture in &deduped {
        if let Some(id) = capture.session_id() {
            sessions.insert(id);
        }
    }

    let mut latest: [Option<&CapturedStatusline>; 2] = [None, None];
    for capture in &deduped {
        if capture.rate_limit("five_hour").is_some() {
            latest[0] = Some(capture);
        }
        if capture.rate_limit("seven_day").is_some() {
            latest[1] = Some(capture);
        }
    }
    let five_hour = latest[0].and_then(|capture| capture.rate_limit("five_hour"));
    let seven_day = latest[1].and_then(|capture| capture.rate_limit("seven_day"));

    let peak = |window: &str| {
        deduped
            .iter()
            .filter_map(|capture| capture.rate_limit(window))
            .filter_map(|state| state.used_percentage)
            .fold(None::<f64>, |acc, used| {
                Some(match acc {
                    Some(max) if max >= used => max,
                    _ => used,
                })
            })
    };

    let mut crossings: Vec<StatuslineLimitCrossing> = Vec::new();
    for window in ["five_hour", "seven_day"] {
        let mut distinct: Vec<i64> = deduped
            .iter()
            .filter_map(|capture| capture.rate_limit(window))
            .filter_map(|state| state.resets_at)
            .collect();
        distinct.sort_unstable();
        distinct.dedup();
        for resets_at in distinct {
            let used_at = |capture: &&CapturedStatusline| {
                capture
                    .rate_limit(window)
                    .and_then(|state| state.used_percentage)
            };
            let before = deduped
                .iter()
                .filter(|capture| capture.captured_at < resets_at)
                .filter_map(used_at)
                .next_back();
            let after = deduped
                .iter()
                .filter(|capture| capture.captured_at >= resets_at)
                .filter_map(used_at)
                .next();
            if before.is_some() && after.is_some() {
                // A crossing is evidenced by observations on both sides:
                // usage observed while the window was running and the
                // first observation after the boundary. A bare
                // resets_at timestamp (no before-side) or a window whose
                // reset passed outside the journal proves nothing.
                crossings.push(StatuslineLimitCrossing {
                    window: window.to_string(),
                    resets_at,
                    used_percentage_before: before,
                    used_percentage_after: after,
                });
            }
        }
    }

    let mut session_caches: Vec<StatuslineSessionCache> = Vec::new();
    let mut miss_causes: BTreeMap<String, u64> = BTreeMap::new();
    let mut cache_seen: HashSet<String> = HashSet::new();
    for capture in deduped.iter().rev() {
        let Some(cache) = capture.prompt_cache() else {
            continue;
        };
        let session_id = capture.session_id().unwrap_or("(unknown)").to_string();
        if !cache_seen.insert(session_id.clone()) {
            continue;
        }
        for (cause, count) in cache.miss_causes.iter().flatten() {
            *miss_causes.entry(cause.clone()).or_insert(0) += count;
        }
        session_caches.push(StatuslineSessionCache {
            session_id,
            hit_ratio: cache.hit_ratio,
            misses: cache.misses,
            miss_recache_tokens: cache.miss_recache_tokens,
            recache_tokens_if_cold: cache.recache_tokens_if_cold,
            last_miss_causes: cache
                .last_miss_cause
                .map(|cause| cause.causes)
                .unwrap_or_default(),
        });
    }

    StatuslineInsights {
        captures: deduped.len(),
        sessions: sessions.len(),
        five_hour,
        seven_day,
        five_hour_peak_used_percentage: peak("five_hour"),
        seven_day_peak_used_percentage: peak("seven_day"),
        limit_crossings: crossings,
        session_caches,
        miss_causes,
    }
}

/// Loads and aggregates the journal at the default path. Returns `None`
/// when nothing is captured yet so reports stay unchanged until the
/// statusline mode is actually used.
pub fn load_statusline_insights() -> Option<StatuslineInsights> {
    let captures = read_statusline_captures(&statusline_capture_path());
    if captures.is_empty() {
        return None;
    }
    Some(statusline_insights(&captures))
}

/// Journal bookkeeping for `--doctor` and the report header.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StatuslineJournalStats {
    pub path: String,
    pub exists: bool,
    pub lines: usize,
    pub bytes: u64,
    /// The retention bound: newest whole lines are kept until the
    /// journal fits half of this once it crosses it.
    pub retained_max_bytes: u64,
}

pub fn statusline_journal_stats(path: &Path) -> StatuslineJournalStats {
    let exists = path.exists();
    let bytes = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    let lines = if exists {
        fs::read_to_string(path)
            .map(|raw| raw.lines().filter(|line| !line.trim().is_empty()).count())
            .unwrap_or(0)
    } else {
        0
    };
    StatuslineJournalStats {
        path: path.to_string_lossy().to_string(),
        exists,
        lines,
        bytes,
        retained_max_bytes: STATUSLINE_CAPTURE_MAX_BYTES,
    }
}

/// `--statusline-report`: journal stats plus the aggregated insights.
pub fn render_statusline_report(format: &str) -> anyhow::Result<String> {
    let path = statusline_capture_path();
    let stats = statusline_journal_stats(&path);
    let captures = read_statusline_captures(&path);
    let insights = statusline_insights(&captures);
    if format == "json" {
        let value = serde_json::json!({ "journal": stats, "insights": insights });
        return Ok(format!("{}\n", serde_json::to_string_pretty(&value)?));
    }
    Ok(render_statusline_report_text(&stats, &insights))
}

fn format_epoch(epoch: i64) -> String {
    chrono::DateTime::from_timestamp(epoch, 0)
        .map(|at| at.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| epoch.to_string())
}

fn format_percent(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:.0}%"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn render_statusline_report_text(
    stats: &StatuslineJournalStats,
    insights: &StatuslineInsights,
) -> String {
    let mut out = String::new();
    out.push_str("AGENTTRACE statusline capture\n");
    out.push_str(&format!("Journal: {}\n", stats.path));
    if !stats.exists {
        out.push_str(
            "  no captures yet; configure Claude Code's statusLine to\n\
             `agenttrace statusline` to start recording\n",
        );
        return out;
    }
    out.push_str(&format!(
        "  {} captures ({} after dedup), {} distinct sessions, {} bytes, retention keeps newest lines under {} bytes\n",
        stats.lines, insights.captures, insights.sessions, stats.bytes, stats.retained_max_bytes / 2
    ));
    out.push_str(&format!(
        "Limits: 5h {} (peak {}) · 7d {} (peak {})\n",
        format_percent(insights.five_hour.as_ref().and_then(|s| s.used_percentage)),
        format_percent(insights.five_hour_peak_used_percentage),
        format_percent(insights.seven_day.as_ref().and_then(|s| s.used_percentage)),
        format_percent(insights.seven_day_peak_used_percentage),
    ));
    for crossing in &insights.limit_crossings {
        out.push_str(&format!(
            "  {} window reset at {}: {} -> {}\n",
            crossing.window,
            format_epoch(crossing.resets_at),
            format_percent(crossing.used_percentage_before),
            format_percent(crossing.used_percentage_after),
        ));
    }
    if !insights.session_caches.is_empty() {
        out.push_str("Prompt cache per session:\n");
        for cache in &insights.session_caches {
            out.push_str(&format!(
                "  {}: hit {}, misses {}",
                cache.session_id,
                cache
                    .hit_ratio
                    .map(|ratio| format!("{:.0}%", ratio * 100.0))
                    .unwrap_or_else(|| "n/a".to_string()),
                cache
                    .misses
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
            ));
            if !cache.last_miss_causes.is_empty() {
                out.push_str(&format!(
                    " (last miss: {})",
                    cache.last_miss_causes.join(", ")
                ));
            }
            if let Some(tokens) = cache.recache_tokens_if_cold {
                out.push_str(&format!(", recache-if-cold {tokens} tokens"));
            }
            out.push('\n');
        }
    }
    if !insights.miss_causes.is_empty() {
        let causes = insights
            .miss_causes
            .iter()
            .map(|(cause, count)| format!("{cause} x{count}"))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("Miss causes: {causes}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture built field-for-field from the documented statusline input
    /// schema (https://code.claude.com/docs/en/statusline, v2.1.251+
    /// fields; fetched research pass 9, 2026-09-14). No Claude Code host
    /// ran in this environment, so this is schema-faithful rather than
    /// captured — every asserted field name and shape is the documented
    /// contract.
    const FIXTURE: &str = r#"{
        "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "session_name": "agenttrace cycle 7",
        "model": {"id": "claude-opus-4-5", "display_name": "Opus 4.5"},
        "workspace": {"current_dir": "/work/projects/agenttrace", "project_dir": "/work/projects/agenttrace"},
        "cost": {"total_cost_usd": 1.234},
        "context_window": {"used_percentage": 41.2},
        "exceeds_200k_tokens": false,
        "rate_limits": {
            "five_hour": {"used_percentage": 84.0, "resets_at": 1760000000},
            "seven_day": {"used_percentage": 46.0, "resets_at": 1760500000}
        },
        "prompt_cache": {
            "warm": true,
            "ttl": "1h",
            "expires_at": 1760000300,
            "requests": 42,
            "misses": 4,
            "expected_rebuilds": 2,
            "hit_ratio": 0.904,
            "cache_write_tokens": 180000,
            "miss_recache_tokens": 12000,
            "last_miss_cause": {"causes": ["tools_changed"], "tools_added": 3, "tools_removed": 1},
            "miss_causes": {"tools_changed": 2, "context_changed": 1},
            "recache_tokens_if_cold": 45000
        }
    }"#;

    fn capture_at(raw: &str, captured_at: i64) -> CapturedStatusline {
        CapturedStatusline {
            captured_at,
            payload: serde_json::from_str(raw).expect("fixture parses"),
        }
    }

    #[test]
    fn payload_maps_to_report_fields() {
        // The acceptance test for candidate 53: every documented field
        // this feature consumes must land in the insights output under
        // its report name, so a schema change upstream fails here first.
        let captures = vec![capture_at(FIXTURE, 1_760_000_100)];
        let insights = statusline_insights(&captures);
        assert_eq!(insights.captures, 1);
        assert_eq!(insights.sessions, 1);
        let five_hour = insights.five_hour.expect("five_hour mapped");
        assert_eq!(five_hour.used_percentage, Some(84.0));
        assert_eq!(five_hour.resets_at, Some(1_760_000_000));
        let seven_day = insights.seven_day.expect("seven_day mapped");
        assert_eq!(seven_day.used_percentage, Some(46.0));
        assert_eq!(insights.five_hour_peak_used_percentage, Some(84.0));
        assert_eq!(insights.seven_day_peak_used_percentage, Some(46.0));
        let cache = insights
            .session_caches
            .first()
            .expect("prompt cache mapped per session");
        assert_eq!(cache.session_id, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert_eq!(cache.hit_ratio, Some(0.904));
        assert_eq!(cache.misses, Some(4));
        assert_eq!(cache.miss_recache_tokens, Some(12_000));
        assert_eq!(cache.recache_tokens_if_cold, Some(45_000));
        assert_eq!(cache.last_miss_causes, vec!["tools_changed".to_string()]);
        assert_eq!(
            insights.miss_causes.get("tools_changed"),
            Some(&2),
            "miss_causes counts aggregate per cause"
        );
        assert_eq!(insights.miss_causes.get("context_changed"), Some(&1));
    }

    #[test]
    fn identical_payloads_dedup_but_newer_state_wins() {
        let before = capture_at(FIXTURE, 1_760_000_100);
        let duplicate = capture_at(FIXTURE, 1_760_000_200);
        let mut escalated = capture_at(FIXTURE, 1_760_000_300);
        escalated.payload["rate_limits"]["five_hour"]["used_percentage"] = serde_json::json!(96.0);
        let insights = statusline_insights(&[before, duplicate, escalated]);
        assert_eq!(insights.captures, 2, "the duplicate collapses");
        assert_eq!(
            insights.five_hour.and_then(|state| state.used_percentage),
            Some(96.0),
            "the latest observed state wins"
        );
        assert_eq!(insights.five_hour_peak_used_percentage, Some(96.0));
    }

    #[test]
    fn resets_at_crossings_record_usage_on_both_sides() {
        // Limit-pressure window: usage climbs to 95% before the window
        // resets, then drops to 3% after. The crossing must carry both
        // sides; a resets_at with no before-side is not a crossing.
        let mut before = capture_at(FIXTURE, 1_759_999_000);
        before.payload["rate_limits"]["five_hour"]["used_percentage"] = serde_json::json!(95.0);
        let mut after = capture_at(FIXTURE, 1_760_000_100);
        after.payload["rate_limits"]["five_hour"]["used_percentage"] = serde_json::json!(3.0);
        let insights = statusline_insights(&[before.clone(), after.clone()]);
        let five_hour_crossings: Vec<_> = insights
            .limit_crossings
            .iter()
            .filter(|crossing| crossing.window == "five_hour")
            .collect();
        assert_eq!(five_hour_crossings.len(), 1);
        let crossing = five_hour_crossings[0];
        assert_eq!(crossing.resets_at, 1_760_000_000);
        assert_eq!(crossing.used_percentage_before, Some(95.0));
        assert_eq!(crossing.used_percentage_after, Some(3.0));

        let only_after = statusline_insights(&[after]);
        assert!(
            only_after.limit_crossings.is_empty(),
            "no before-side usage means no evidenced crossing"
        );
    }

    #[test]
    fn journal_roundtrip_tolerates_a_torn_tail_line() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-journal-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        // Point the environment at the temp root before any append (the
        // default path is the user's real journal), under the shared env
        // lock so sibling-module tests cannot re-point it mid-append.
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let payload: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        append_statusline_capture(&payload).expect("first append");
        // A crash mid-append leaves a torn line; reads skip it.
        {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&journal)
                .expect("open journal");
            writeln!(file, "{{\"captured_at\":1,\"payload\":{{\"torn").expect("write torn line");
        }
        append_statusline_capture(&payload).expect("second append");
        let captures = read_statusline_captures(&journal);
        assert_eq!(
            captures.len(),
            2,
            "valid lines round-trip, the torn line is skipped"
        );
        assert!(captures[0].captured_at > 0);
        let stats = statusline_journal_stats(&journal);
        assert!(stats.exists);
        assert_eq!(stats.retained_max_bytes, STATUSLINE_CAPTURE_MAX_BYTES);
        assert!(stats.lines >= 2);
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn compaction_keeps_newest_whole_lines_under_half_the_bound() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-compact-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        // 40 lines of ~100 bytes each: 4 KiB total, well over a 2 KiB
        // ceiling; compaction must keep the newest lines under 1 KiB.
        let mut raw = String::new();
        for i in 0..40 {
            let line = format!(
                "{{\"captured_at\":{i},\"payload\":{{\"filler\":\"{}\"}}}}",
                "x".repeat(80)
            );
            raw.push_str(&line);
            raw.push('\n');
        }
        fs::write(&journal, &raw).expect("write journal");
        compact_statusline_capture_under(&journal, 1024).expect("compact");
        let kept = fs::read_to_string(&journal).expect("read journal");
        let count = kept.lines().count();
        assert!(count < 40, "old lines must drop, kept {count}");
        assert!(count > 0, "newest lines must survive");
        let newest = kept.lines().last().expect("at least one line");
        assert!(
            newest.contains("\"captured_at\":39"),
            "the newest line survives compaction: {newest}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn status_line_rendering_is_one_line_and_schema_driven() {
        let payload: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let line = render_status_line(&payload);
        assert!(!line.contains('\n'), "the host contract is one line");
        assert!(line.contains("agenttrace cycle 7"), "session name first");
        assert!(line.contains("ctx 41%"), "context window pressure");
        assert!(line.contains("5h 84%"), "five-hour limit");
        assert!(line.contains("7d 46%"), "seven-day limit");
        assert!(line.contains("cache 90%"), "hit_ratio is a 0..1 ratio");
        assert!(line.contains("$1.23"), "running cost");
        // A payload with nothing renderable still yields one honest
        // line, never an empty string (a blank status line breaks the
        // host).
        assert_eq!(render_status_line(&serde_json::json!({})), "agenttrace");
    }

    #[test]
    fn hostile_payload_names_render_without_control_characters() {
        // Review F1 (cycle 7): session names are user-authored chat
        // titles; embedded newlines break the one-line contract and
        // ESC/OSC sequences inject into the host terminal. Every
        // control character (C0, DEL, C1) must be neutralized.
        let payload = serde_json::json!({
            "session_name": "evil\nsecond\rcarriage\u{001b}[31mRED\u{001b}]777;pwned\u{0007}",
            "model": { "display_name": "M\u{009b}1P" },
            "context_window": { "used_percentage": 10.0 }
        });
        let line = render_status_line(&payload);
        assert!(
            !line.contains('\n') && !line.contains('\r'),
            "one line: {line:?}"
        );
        assert!(
            line.chars().all(|c| !c.is_control()),
            "no control bytes reach the terminal: {line:?}"
        );
        assert!(
            line.contains('\u{FFFD}'),
            "control chars are replaced visibly"
        );
        assert!(line.contains("ctx 10%"), "non-name fields unaffected");
        // The model-name fallback path is sanitized too.
        let fallback = serde_json::json!({
            "model": { "display_name": "\u{001b}]0;title\u{0007}Opus" }
        });
        let line = render_status_line(&fallback);
        assert!(
            line.chars().all(|c| !c.is_control()),
            "display_name fallback sanitized: {line:?}"
        );
    }
}

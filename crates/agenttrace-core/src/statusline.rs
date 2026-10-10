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

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
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
/// `AGENTTRACE_SESSION_CACHE_DIR` relocates both; `--clear-cache`
/// removes it by explicit path alongside the other cache artifacts
/// (rm-086: the session-cache artifact registry owns that set).
pub fn statusline_capture_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR").map(PathBuf::from) {
        if !dir.as_os_str().is_empty() {
            return dir.join("statusline.jsonl");
        }
    }
    user_cache_dir().join("agenttrace").join("statusline.jsonl")
}

fn user_cache_dir() -> PathBuf {
    resolve_user_cache_dir(
        std::env::var_os("HOME").as_deref(),
        std::env::var_os("XDG_CACHE_HOME").as_deref(),
    )
}

/// Pure resolution core for [`user_cache_dir`]: the same rules without
/// touching process environment state, so the empty-env fallbacks are
/// unit-testable without mutating (unsafe) globals (the
/// `resolve_user_config_path` pattern from rm-683).
///
/// Review 95d74221 F2 (2026-10-07): a set-but-empty `HOME` is now
/// treated as unset on BOTH arms — the same XDG-spec v0.8 rule the
/// user-config guard applies. Without the guard, `HOME=` joined the
/// RELATIVE `Library/Caches` / `.cache`, so the capture journal
/// resolved against the CWD and a `.cache/agenttrace/statusline.jsonl`
/// planted in the working directory was read into the budget view
/// (live PoC: `$999.00 ... OVER by $989.00` from a planted journal).
fn resolve_user_cache_dir(
    home: Option<&std::ffi::OsStr>,
    xdg_cache_home: Option<&std::ffi::OsStr>,
) -> PathBuf {
    if cfg!(target_os = "macos") {
        if let Some(home) = home.filter(|value| !value.is_empty()) {
            return PathBuf::from(home).join("Library").join("Caches");
        }
    }
    if let Some(cache) = xdg_cache_home.filter(|value| !value.is_empty()) {
        return PathBuf::from(cache);
    }
    if let Some(home) = home.filter(|value| !value.is_empty()) {
        return PathBuf::from(home).join(".cache");
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
///
/// rm-383: this is THE shared control-character sanitizer for every CLI
/// text renderer that prints session- or journal-derived strings
/// (search text output, session text reports, the --sessions TSV, and
/// --doctor disclosure samples); JSON output paths deliberately skip it
/// because JSON escaping already encodes control bytes losslessly, and
/// markdown/HTML renderers apply their own escaping before this class
/// applies. Printable CSI/OSC tails may legitimately survive — only the
/// control bytes themselves (ESC, BEL, …) are neutralized.
pub fn sanitize_line_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|c| if c.is_control() { '\u{FFFD}' } else { c })
        .collect()
}

/// rm-625: document-level variant of [`sanitize_line_segment`] for the
/// output-dispatch boundary (stdout and every `-o` write). Where the
/// line-segment helper must kill EVERY control byte (a status line is
/// one physical line), whole documents legitimately carry layout
/// bytes: LF, CR and TAB are layout and stay; every other control
/// byte — ESC (the OSC/CSI introducer), BEL, DEL, and the C1 range —
/// is neutralized to U+FFFD so terminal emulators never interpret
/// transcript-derived sequences. Idempotent by construction: U+FFFD is
/// printable, so already-sanitized documents (including cells that
/// first went through the rm-383/rm-540 line sanitizer) pass through
/// unchanged — the two layers compose instead of corrupting.
pub fn sanitize_output_document(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                '\u{FFFD}'
            } else {
                c
            }
        })
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
    // rm-208: the journal is captured conversation context (session
    // names, project paths, working directories); create it owner-only
    // instead of the fs default (0644/0664).
    //
    // rm-166: several status-line invocations and a compaction can be
    // in flight on the same journal at once. The append used to go
    // through a lock-less O_APPEND handle while the compaction
    // replaced the journal with a temp-file rename, so an append
    // racing the rename landed on the replaced inode and silently
    // vanished. The whole write-and-maybe-compact critical section now
    // runs under an exclusive cross-process lock
    // (`open_locked_journal`).
    let journal = open_locked_journal(&path)?;
    let size_after_append = journal.metadata()?.len() + line.len() as u64 + 1;
    let mut writer = &journal;
    let appended = writeln!(writer, "{line}").and_then(|()| writer.flush());
    let compacted = if appended.is_ok() && size_after_append > STATUSLINE_CAPTURE_MAX_BYTES {
        compact_statusline_capture_locked(&path, STATUSLINE_CAPTURE_MAX_BYTES / 2)
    } else {
        Ok(())
    };
    let unlocked = journal.unlock();
    appended.and(compacted).and(unlocked)
}

/// Bound on lock-acquisition retries. Each retry means the journal was
/// compacted (renamed) between opening and locking — rare even under
/// load; a thrashing writer exhausts the budget and surfaces an error
/// instead of spinning.
const JOURNAL_LOCK_ATTEMPTS: usize = 64;

/// Opens the journal for appending under an exclusive cross-process
/// lock (rm-166).
///
/// `File::lock` is an open-file-description advisory lock: exclusive
/// between processes and between separately-opened handles inside one
/// process, and released automatically when the descriptor closes, so
/// a crashed status line can never wedge the journal. Because a
/// compaction renames a fresh inode into place while the lock is held
/// on the old one, the lock alone cannot serialize a writer that opened
/// the path before the rename: after acquiring, the descriptor is
/// revalidated against the path's live inode and the open is retried
/// when it raced a compaction.
fn open_locked_journal(path: &Path) -> io::Result<fs::File> {
    for _ in 0..JOURNAL_LOCK_ATTEMPTS {
        let journal = crate::session_cache::open_private_append(path)?;
        journal.lock()?;
        if journal_descriptor_is_live(path, &journal) {
            return Ok(journal);
        }
        // We locked an inode a completed compaction already replaced:
        // release it and re-open the journal that is live now.
        drop(journal);
    }
    Err(io::Error::other(format!(
        "statusline journal lock lost {JOURNAL_LOCK_ATTEMPTS} races with compaction"
    )))
}

/// True when `journal`'s descriptor still refers to the inode currently
/// at `path`. Compaction renames a fresh inode over the path, orphaning
/// any handle (and lock) opened before the swap.
fn journal_descriptor_is_live(path: &Path, journal: &fs::File) -> bool {
    #[cfg(unix)]
    {
        match (journal.metadata(), fs::metadata(path)) {
            (Ok(descriptor), Ok(live)) => descriptor.ino() == live.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        // The temp-file rename is a Unix journal pattern; elsewhere the
        // advisory lock on the open descriptor is the whole protocol.
        let _ = (path, journal);
        true
    }
}

/// Retention: drop oldest whole lines until the journal fits under
/// `keep_under` bytes. Rewritten through a temp file + rename so a crash
/// mid-compaction cannot truncate the journal to zero.
///
/// rm-166: the read→rewrite→rename critical section races concurrent
/// appends unless it runs under the journal lock, so this entry takes the
/// lock itself. The append path (which already holds the lock — the
/// open-file-description lock is not reentrant across handles) calls the
/// locked core directly; no production caller needs the standalone entry
/// yet, so it is exercised by the journal race fixtures.
#[allow(dead_code)]
fn compact_statusline_capture_under(path: &Path, keep_under: u64) -> io::Result<()> {
    let journal = open_locked_journal(path)?;
    let compacted = compact_statusline_capture_locked(path, keep_under);
    let unlocked = journal.unlock();
    compacted.and(unlocked)
}

/// rm-898: bound on how much of a statusline journal this module will
/// materialize at once. Mirrors the retention bound below — the compaction
/// REWRITE always kept to it, but the READ side (three sites) loaded the
/// whole journal into memory first, so a hostile or runaway journal was
/// fully loaded before any bounded handling could kick in.
const STATUSLINE_READ_CAP_BYTES: u64 = 10 * 1024 * 1024;

/// rm-898: read at most [`STATUSLINE_READ_CAP_BYTES`] bytes from the TAIL of
/// the journal (the fleet-proven rm-821 `read_journal_capped` shape). Returns
/// the tail contents and the number of bytes cut from the head. A cut that
/// lands mid-line or mid-character is resolved by advancing to the next char
/// boundary and dropping the (torn) first line — mirroring the torn-tail
/// tolerance the journal lanes already apply to truncated writers.
fn read_journal_tail_capped(path: &Path) -> io::Result<(String, u64)> {
    use std::io::{Read, Seek, SeekFrom};
    let len = fs::metadata(path)?.len();
    if len <= STATUSLINE_READ_CAP_BYTES {
        return Ok((fs::read_to_string(path)?, 0));
    }
    let mut cut = len - STATUSLINE_READ_CAP_BYTES;
    let mut file = fs::File::open(path)?;
    let mut buf = Vec::with_capacity(STATUSLINE_READ_CAP_BYTES as usize);
    let mut advances_left = 3u32; // a valid start byte has at most 3 continuation bytes before it
    let raw = loop {
        file.seek(SeekFrom::Start(cut))?;
        buf.clear();
        (&mut file)
            .take(STATUSLINE_READ_CAP_BYTES)
            .read_to_end(&mut buf)?;
        match std::str::from_utf8(&buf) {
            Ok(s) => break s.to_string(),
            Err(e) => {
                // A cut landing inside a multi-byte character advances to the
                // next boundary — the buffer then begins with a continuation
                // byte and from_utf8 fails at offset 0; a valid start byte
                // can be preceded by at most three continuation bytes, so the
                // advance is bounded. Any other invalid UTF-8 (interior
                // corruption, a torn writer's mid-character EOF) falls back to
                // a lossy read so the lane stays usable (same tolerance as
                // torn writers). Review-fix 9fb0a017 F2: the prior guard
                // compared valid_up_to() == buf.len(), which from_utf8 can
                // never produce on an Err — that branch was dead and the raw
                // cut leaked through the lossy fallback.
                if e.valid_up_to() == 0
                    && advances_left > 0
                    && cut < len
                    && matches!(buf.first(), Some(b) if (0x80..=0xBF).contains(b))
                {
                    cut += 1;
                    advances_left -= 1;
                    continue;
                }
                break String::from_utf8_lossy(&buf).into_owned();
            }
        }
    };
    let away = cut;
    // Drop a torn head line: after a head cut the first line is complete
    // only when the cut landed exactly at a line boundary.
    let body = if away > 0 {
        match raw.find('\n') {
            Some(i) => raw[i + 1..].to_string(),
            None => String::new(),
        }
    } else {
        raw
    };
    Ok((body, away))
}

fn compact_statusline_capture_locked(path: &Path, keep_under: u64) -> io::Result<()> {
    let (raw, _capped_away_bytes) = read_journal_tail_capped(path)?;
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
    // rm-202: the temp is a per-writer sibling, not
    // a fixed `<name>.jsonl.compact` — concurrent compacts can no longer
    // interleave into (and race the rename of) one shared temp, and a
    // crash before the rename leaves a `.tmp.` orphan that the
    // cache-load sweep already removes instead of a name no sweep
    // knows. The rewrite is bounded (half the 10 MiB retention bound),
    // so building it in memory first is safe, and the owner-only write
    // (rm-208) keeps the renamed journal private as well. rm-693: the
    // stage is the shared O_EXCL staging helper — a symlink planted at
    // the predictable temp name is bumped past, never truncated
    // through (same exclusivity as every other atomic writer).
    let mut rewritten = String::with_capacity(kept_bytes.min(usize::MAX as u64) as usize);
    for line in kept {
        rewritten.push_str(line);
        rewritten.push('\n');
    }
    crate::session_cache::write_private_exclusive(path, rewritten.as_bytes())?;
    Ok(())
}

/// Reads the journal, skipping malformed lines (a torn tail line from a
/// crashed append is tolerated, matching append-only semantics).
pub fn read_statusline_captures(path: &Path) -> Vec<CapturedStatusline> {
    let (raw, _capped_away_bytes) = read_journal_tail_capped(path).unwrap_or_default();
    parse_statusline_captures(&raw)
}

/// The journal text actually read: empty when the file is absent or
/// unreadable, the same tolerance [`read_statusline_captures`] always
/// had. rm-684 (review 95d74221 F6): the budget view and the report
/// read the journal exactly ONCE per invocation — the stats
/// disclosure, the JSON arm, the text arm and the captures all share
/// this buffer instead of each re-reading the (up to 10 MiB) file.
/// rm-898: the shared read is itself bounded — at most
/// [`STATUSLINE_READ_CAP_BYTES`] taken from the TAIL — and the bytes
/// cut from the head ride along so the stats can disclose them as
/// `capped_away_bytes` instead of silently undercounting a huge
/// journal.
fn read_statusline_capture_buffer(path: &Path) -> (String, u64) {
    read_journal_tail_capped(path).unwrap_or_default()
}

/// Parses already-read journal text (the torn-tail-tolerant half of
/// [`read_statusline_captures`]).
fn parse_statusline_captures(raw: &str) -> Vec<CapturedStatusline> {
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
    /// rm-898: bytes cut from the head of the journal by
    /// [`read_journal_tail_capped`] when it exceeds
    /// [`STATUSLINE_READ_CAP_BYTES`] (`0` = the read was complete). The
    /// statusline-report JSON surfaces this so a bounded read is disclosed
    /// rather than silently undercounting a huge journal.
    pub capped_away_bytes: u64,
}

pub fn statusline_journal_stats(path: &Path) -> StatuslineJournalStats {
    let (raw, capped_away_bytes) = read_statusline_capture_buffer(path);
    statusline_journal_stats_from_buffer(path, &raw, capped_away_bytes)
}

/// Stats derived from an already-read journal buffer (rm-684: the
/// budget view computes these from its single read instead of a
/// second full read). `lines` describes the text that was actually
/// read; `capped_away_bytes` carries the head cut from that same
/// shared bounded read (rm-898); `exists` still probes the path so an
/// absent journal discloses itself, and `bytes` always names the
/// on-disk metadata size — review 95d74221 F7: an undecodable journal
/// decodes to an empty buffer and reporting `bytes: 0` exactly when
/// the journal is corrupt would hide its real footprint (and rm-898
/// extends the same rule to the over-cap case: a bounded read must
/// never shrink the reported footprint of the file it truncated).
fn statusline_journal_stats_from_buffer(
    path: &Path,
    raw: &str,
    capped_away_bytes: u64,
) -> StatuslineJournalStats {
    let exists = path.exists();
    let bytes = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    // rm-898: bounded tail read — line counts for an over-cap journal
    // cover only the retained tail; the shared buffer's `away` discloses
    // the head that was cut (no second read, rm-684/F6).
    let lines = if exists {
        raw.lines().filter(|line| !line.trim().is_empty()).count()
    } else {
        0
    };
    StatuslineJournalStats {
        path: path.to_string_lossy().to_string(),
        exists,
        lines,
        bytes,
        retained_max_bytes: STATUSLINE_CAPTURE_MAX_BYTES,
        capped_away_bytes,
    }
}

/// `--statusline-report`: journal stats plus the aggregated insights.
/// `weekly_budget` (rm-385) is the resolved weekly USD budget; `None`
/// leaves the report exactly as it was before the knob existed.
pub fn render_statusline_report(
    format: &str,
    weekly_budget: Option<f64>,
) -> anyhow::Result<String> {
    let path = statusline_capture_path();
    // Review 95d74221 F6: ONE journal read — the same rm-684
    // single-read pattern render_budget_view uses (stats and captures
    // share the buffer; this used to read the journal twice
    // back-to-back). rm-898: the shared read is the bounded tail read,
    // and the head cut rides along into the stats.
    let (raw, capped_away_bytes) = read_statusline_capture_buffer(&path);
    let stats = statusline_journal_stats_from_buffer(&path, &raw, capped_away_bytes);
    let captures = parse_statusline_captures(&raw);
    let insights = statusline_insights(&captures);
    let series = statusline_budget_series(&captures, 7, Utc::now());
    if format == "json" {
        let mut value = serde_json::json!({ "journal": stats, "insights": insights });
        if let Some(budget) = weekly_budget {
            value["budget"] = serde_json::json!({
                "weekly_budget_usd": budget,
                "spend_7d_usd": series.total,
                "remaining_usd": budget - series.total,
            });
        }
        return Ok(format!("{}\n", serde_json::to_string_pretty(&value)?));
    }
    Ok(render_statusline_report_text(
        &stats,
        &insights,
        weekly_budget,
        &series,
    ))
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
    weekly_budget: Option<f64>,
    series: &StatuslineBudgetSeries,
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
    // rm-898: an over-cap journal is read from the TAIL only, so the
    // counts above describe the retained tail, not the whole file —
    // disclose the cut instead of silently undercounting a huge
    // journal. The JSON arm carries the same fact as the
    // `capped_away_bytes` field on the serialized journal stats.
    if stats.capped_away_bytes > 0 {
        out.push_str(&format!(
            "  journal exceeds the {} MiB read bound: head truncated, {} of {} bytes unread; counts cover the retained tail only\n",
            STATUSLINE_READ_CAP_BYTES / (1024 * 1024),
            stats.capped_away_bytes,
            stats.bytes
        ));
    }
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
    // rm-385: the weekly budget line rides the already-parsed capture
    // journal; budget unset keeps the report byte-compatible.
    // rm-917: an empty calendar window discloses instead of verdicting.
    if let Some(budget) = weekly_budget {
        if series.daily.is_empty() {
            out.push_str(&format!(
                "Budget: no cost samples in the trailing 7 calendar days (budget ${budget:.2})\n"
            ));
        } else {
            let remaining = budget - series.total;
            let state = if remaining < 0.0 {
                format!("OVER by ${:.2}", -remaining)
            } else {
                format!("${remaining:.2} remaining")
            };
            out.push_str(&format!(
                "Budget: 7d spend ${:.2} of ${budget:.2} — {state}\n",
                series.total
            ));
        }
    }
    if !insights.session_caches.is_empty() {
        out.push_str("Prompt cache per session:\n");
        for cache in &insights.session_caches {
            // rm-034: session ids and miss-cause names are journal
            // payload strings and print raw no longer — they pass the
            // same control-character sanitization as the render path
            // (the journal itself keeps storing what it saw).
            out.push_str(&format!(
                "  {}: hit {}, misses {}",
                sanitize_line_segment(&cache.session_id),
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
                    sanitize_line_segment(&cache.last_miss_causes.join(", "))
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
            .map(|(cause, count)| format!("{} x{count}", sanitize_line_segment(cause)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("Miss causes: {causes}\n"));
    }
    out
}

// --- rm-385: weekly budget telemetry (cycle 1) -------------------------

/// The `cost.total_cost_usd` sample a capture carries, if any. The
/// payload gates this field at v2.1.251+, so older captures simply
/// contribute nothing rather than failing the aggregation.
fn capture_cost_usd(capture: &CapturedStatusline) -> Option<f64> {
    capture.payload.get("cost")?.get("total_cost_usd")?.as_f64()
}

fn utc_day(epoch: i64) -> String {
    use chrono::TimeZone;
    chrono::Utc
        .timestamp_opt(epoch, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// Per-day USD spend derived from the ALREADY-parsed capture journal
/// (rm-385). Captures carry each session's *cumulative* cost, so a
/// day's spend is the rise of that cumulative value across the day's
/// samples; a value that drops (session epoch reset) contributes only
/// itself. Days are UTC buckets over the trailing window, ascending.
/// rm-917: the window is CALENDAR-anchored — the trailing `days`
/// calendar days ending today (UTC), never the newest `days` *sampled*
/// days, so a stale journal cannot charge the weekly envelope a
/// weeks-old sum.
#[derive(Debug, Clone, PartialEq)]
pub struct StatuslineBudgetSeries {
    /// UTC day (`YYYY-MM-DD`) -> spend in USD, ascending: one bucket
    /// per calendar day in the window once any sample falls inside it
    /// (missing days render as explicit $0.00 rows); empty when the
    /// window holds no samples at all.
    pub daily: Vec<(String, f64)>,
    /// Sum of `daily` (in-window spend only).
    pub total: f64,
    /// rm-917: distinct sampled UTC days strictly older than the
    /// calendar window — excluded from `daily`/`total` and disclosed
    /// by the budget views so a stale journal is visible, not silent.
    pub excluded_older_days: usize,
}

pub fn statusline_budget_series(
    captures: &[CapturedStatusline],
    days: usize,
    now: DateTime<Utc>,
) -> StatuslineBudgetSeries {
    use std::collections::BTreeMap;

    // session -> day -> max cumulative cost seen that day.
    let mut per_session: BTreeMap<&str, BTreeMap<String, f64>> = BTreeMap::new();
    for capture in captures {
        if let Some(cost) = capture_cost_usd(capture) {
            let day = utc_day(capture.captured_at);
            if day.is_empty() {
                continue;
            }
            // Captures without a session id cannot be attributed to a
            // cost series; skipping them keeps the sum conservative.
            let Some(session) = capture.session_id() else {
                continue;
            };
            let entry = per_session
                .entry(session)
                .or_default()
                .entry(day)
                .or_insert(f64::NEG_INFINITY);
            if cost > *entry {
                *entry = cost;
            }
        }
    }
    let mut daily: BTreeMap<String, f64> = BTreeMap::new();
    for samples in per_session.values() {
        let mut previous = 0.0;
        for (day, cumulative) in samples {
            let rise = (cumulative - previous).max(0.0);
            *daily.entry(day.clone()).or_insert(0.0) += rise;
            previous = *cumulative;
        }
    }
    // rm-917 (run d02291d0, composed at integration over the
    // 2026-10-07 budget-window rider): keep only the trailing `days`
    // CALENDAR days ending today (UTC) — NOT the newest `days`
    // sampled days, which charged the weekly envelope a possibly
    // weeks-old sum whenever the journal went stale (the live PoC:
    // a 13-day-old sample span drove `7d total $7.00` / `OVER by
    // $2.00` against --weekly-budget 5 with zero true trailing-week
    // samples; before the rider, the window kept the first `days`
    // *observed* days, so a corpus capturing $4/$5/$1 on the
    // 21st/26th/6th rendered "7d total $10.00" with only $1 in the
    // current week). The rider's mechanics survive as the anchor: the
    // window ends at `now`'s UTC day (the pinned-`now` testability
    // param from review 95d74221) and is bounded ABOVE by today, so
    // a future-dated capture (clock skew, a hostile journal — review
    // 95d74221 F8) bills into no window. rm-917 adds on top: in-window
    // gap days zero-fill as explicit $0.00 rows, strictly-older
    // sampled days are excluded from `daily`/`total` and disclosed
    // through `excluded_older_days`, and an empty calendar window
    // keeps the "(no cost samples in the window)" disclosure honest
    // instead of verdicting off stale samples. `days == 0` is an
    // EMPTY window — zero calendar days zero-fill to zero rows
    // (superseding the rider's read that `days == 0` anchors a
    // one-day window; no test pinned that edge either way).
    let today = now.date_naive();
    if days == 0 {
        return StatuslineBudgetSeries {
            daily: Vec::new(),
            total: 0.0,
            excluded_older_days: daily.len(),
        };
    }
    let window_start = today - Duration::days(days as i64 - 1);
    let by_day: BTreeMap<chrono::NaiveDate, f64> = daily
        .into_iter()
        .filter_map(|(day, spend)| Some((day.parse::<chrono::NaiveDate>().ok()?, spend)))
        .collect();
    let excluded_older_days = by_day.range(..window_start).count();
    let any_in_window = by_day.range(window_start..=today).next().is_some();
    let window: Vec<(String, f64)> = if any_in_window {
        // Zero-fill the calendar days inside the window so gaps are
        // explicit $0.00 rows instead of silently missing days.
        (0..days as i64)
            .map(|offset| {
                let date = window_start + Duration::days(offset);
                (
                    date.format("%Y-%m-%d").to_string(),
                    by_day.get(&date).copied().unwrap_or(0.0),
                )
            })
            .collect()
    } else {
        // No sample inside the calendar window: an empty series keeps
        // the "(no cost samples in the window)" disclosure honest and
        // never verdicts off stale samples.
        Vec::new()
    };
    // Review 5b9a9470 F4: an empty window's f64 reduction is lowered
    // by LLVM to the additive identity -0.0 in optimized builds, which
    // rendered as "$-0.00" at both budget display sites (the fresh-
    // install default carries no cost samples). Adding +0.0 normalizes
    // the sign bit and changes no other value (-0.0 + 0.0 == +0.0).
    let total = window.iter().map(|(_, spend)| *spend).sum::<f64>() + 0.0;
    StatuslineBudgetSeries {
        daily: window,
        total,
        excluded_older_days,
    }
}

/// rm-385: the `--budget` window-burn view. Renders the trailing
/// seven days of spend from the capture journal against the resolved
/// weekly budget. An unset budget is not an error — the view then
/// shows the spend and how to set the knob. `-f json` renders the
/// same data as one JSON object (review 5b9a9470 F5: the machine
/// format must never silently emit prose), mirroring the
/// `--statusline-report` budget keys.
pub fn render_budget_view(format: &str, weekly_budget: Option<f64>) -> anyhow::Result<String> {
    let path = statusline_capture_path();
    // rm-684: ONE journal read per invocation — the stats disclosure,
    // the JSON arm, and the text arm all share this buffer
    // (previously the journal was read three times per invocation:
    // once for stats, once for captures, and a second capture read in
    // the text arm). rm-898: the shared read is the bounded tail read,
    // and the head cut rides along into the stats disclosure.
    let (raw, capped_away_bytes) = read_statusline_capture_buffer(&path);
    let stats = statusline_journal_stats_from_buffer(&path, &raw, capped_away_bytes);
    let captures = parse_statusline_captures(&raw);
    let series = statusline_budget_series(&captures, 7, Utc::now());
    if format == "json" {
        let mut value = serde_json::json!({
            "journal": stats,
            "daily": series
                .daily
                .iter()
                .map(|(day, spend)| serde_json::json!({
                    "date": day,
                    "spend_usd": spend,
                }))
                .collect::<Vec<_>>(),
            "spend_7d_usd": series.total,
            // rm-917: stale sampled days the calendar window excluded.
            "excluded_older_days": series.excluded_older_days,
        });
        if let Some(budget) = weekly_budget {
            value["weekly_budget_usd"] = serde_json::json!(budget);
            value["remaining_usd"] = serde_json::json!(budget - series.total);
        }
        return Ok(format!("{}\n", serde_json::to_string_pretty(&value)?));
    }
    if !stats.exists || stats.lines == 0 {
        return Ok(format!(
            "No statusline captures yet at {} — the journal is populated by the \
             `agenttrace statusline` host command.\n",
            path.display()
        ));
    }
    // rm-917 header (README/CHANGELOG pin the phrasing); the series
    // is the ONE rm-684/rm-898 shared single-read computation above —
    // the candidate's text arm re-read the journal here (superseded
    // mechanics, dropped at integration).
    let mut out = String::from("Weekly budget — trailing 7 calendar days (statusline journal)\n");
    for (day, spend) in &series.daily {
        out.push_str(&format!("  {day}  ${spend:.2}\n"));
    }
    if series.daily.is_empty() {
        out.push_str("  (no cost samples in the window)\n");
    }
    out.push_str(&format!("  7d total ${:.2}\n", series.total));
    if series.excluded_older_days > 0 {
        out.push_str(&format!(
            "  ({} older day{} outside the window not counted)\n",
            series.excluded_older_days,
            if series.excluded_older_days == 1 {
                ""
            } else {
                "s"
            }
        ));
    }
    match weekly_budget {
        // rm-917: an empty calendar window never drives a spend
        // verdict — stale samples must not read as OVER/remaining.
        Some(budget) if series.daily.is_empty() => out.push_str(&format!(
            "  budget ${budget:.2} — no cost samples in the trailing 7 calendar days; no spend verdict\n"
        )),
        Some(budget) => {
            let remaining = budget - series.total;
            if remaining < 0.0 {
                out.push_str(&format!(
                    "  budget ${budget:.2} — OVER by ${:.2}\n",
                    -remaining
                ));
            } else {
                out.push_str(&format!(
                    "  budget ${budget:.2} — ${remaining:.2} remaining ({:.0}%)\n",
                    (remaining / budget * 100.0).clamp(0.0, 100.0)
                ));
            }
        }
        None => out.push_str(
            "  no weekly budget configured — set `weekly_budget_usd` in \
             ~/.config/agenttrace/config.toml (or pass --weekly-budget)\n",
        ),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget_capture(session: &str, captured_at: i64, cost: f64) -> CapturedStatusline {
        let payload: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let mut payload = payload;
        payload["session_id"] = Value::String(session.to_string());
        payload["cost"] = serde_json::json!({ "total_cost_usd": cost });
        CapturedStatusline {
            captured_at,
            payload,
        }
    }

    fn epoch_on_day_offset(days_ago: i64, secs_into_day: i64) -> i64 {
        use chrono::TimeZone;
        // Noon UTC keeps the sample safely inside the intended UTC day
        // no matter when the test runs.
        let noon = (chrono::Utc::now().date_naive() - chrono::TimeDelta::days(days_ago))
            .and_hms_opt(12, 0, 0)
            .expect("noon exists");
        chrono::Utc.from_utc_datetime(&noon).timestamp() + secs_into_day
    }

    #[test]
    fn budget_series_stale_span_outside_calendar_window_is_empty() {
        // rm-917 (assess a38c3665 PoC shape): samples on 13 distinct
        // days, ALL strictly older than the trailing 7 calendar days.
        // The sampled window used to keep the newest 7 of those days
        // and charged the weekly envelope a weeks-old sum (live PoC:
        // "7d total $7.00" / "OVER by $2.00" against --weekly-budget 5
        // with zero true trailing-week samples). A calendar window
        // holds none of these days: the series is empty.
        let mut captures = Vec::new();
        let mut cumulative = 0.0;
        for days_ago in (8..=20).rev() {
            captures.push(budget_capture(
                "A",
                epoch_on_day_offset(days_ago, 100),
                cumulative,
            ));
            cumulative += 1.00;
        }
        let series = statusline_budget_series(&captures, 7, Utc::now());
        assert!(series.daily.is_empty(), "{:?}", series.daily);
        assert_eq!(series.total.to_bits(), 0.0f64.to_bits(), "{series:?}");
    }

    #[test]
    fn budget_series_calendar_window_zero_fills_missing_days() {
        // rm-917: the window is the trailing 7 CALENDAR days ending
        // today. Days inside the window with no samples render as
        // explicit $0.00 rows; days outside it are excluded from the
        // sum entirely.
        let captures = vec![
            budget_capture("A", epoch_on_day_offset(5, 100), 2.00),
            budget_capture("A", epoch_on_day_offset(0, 100), 5.00),
        ];
        let series = statusline_budget_series(&captures, 7, Utc::now());
        assert_eq!(series.daily.len(), 7, "{:?}", series.daily);
        let today = chrono::Utc::now().date_naive();
        for (i, (day, spend)) in series.daily.iter().enumerate() {
            let expect_date = today - chrono::TimeDelta::days(6 - i as i64);
            assert_eq!(
                day,
                &expect_date.format("%Y-%m-%d").to_string(),
                "{:?}",
                series.daily
            );
            let expect_spend = match 6 - i as i64 {
                5 => 2.00, // first sample contributes itself
                0 => 3.00, // rise from 2.00 to 5.00
                _ => 0.00,
            };
            assert!((spend - expect_spend).abs() < 1e-9, "{:?}", series.daily);
        }
        assert!((series.total - 5.00).abs() < 1e-9, "{series:?}");
    }

    #[test]
    fn budget_report_renders_no_verdict_when_calendar_window_has_no_samples() {
        // rm-917: an empty calendar window must never drive an
        // OVER/remaining verdict off stale samples — the disclosure
        // line replaces the verdict.
        let series = statusline_budget_series(&[], 7, Utc::now());
        let text = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 3,
                bytes: 100,
                retained_max_bytes: 1024,
                capped_away_bytes: 0,
            },
            &statusline_insights(&[]),
            Some(5.0),
            &series,
        );
        assert!(text.contains("no cost samples"), "{text}");
        assert!(!text.contains("remaining"), "{text}");
        assert!(!text.contains("OVER"), "{text}");
    }

    #[test]
    fn budget_series_empty_window_total_is_positive_zero() {
        // Review 5b9a9470 F4: LLVM lowers an empty f64 reduction to the
        // additive identity -0.0 in optimized builds, which rendered as
        // "$-0.00" for every fresh install (no cost samples). The sum
        // site normalizes with `+ 0.0`; this pins the sign bit so no
        // future change reintroduces negative zero.
        let series = statusline_budget_series(&[], 7, Utc::now());
        assert_eq!(series.total.to_bits(), 0.0f64.to_bits(), "{series:?}");
        assert!(series.daily.is_empty());
    }

    #[test]
    fn budget_view_json_is_one_object_with_budget_keys() {
        // Review 5b9a9470 F5: `--budget` used to ignore `-f json` and
        // print prose. The JSON arm must emit exactly one parseable
        // object mirroring the --statusline-report budget keys, with
        // the budget fields present only when the knob is set. The
        // test stays hermetic: no env mutation, and every spend
        // assertion holds for any host journal (daily rises are
        // clamped at 0, so a negative sign is impossible — negative
        // zero included).
        for (budget, has_budget) in [(None, false), (Some(12.5), true)] {
            let rendered = render_budget_view("json", budget).expect("render budget view");
            let value: Value = serde_json::from_str(rendered.trim()).expect("budget JSON parses");
            let object = value.as_object().expect("top level is an object");
            for key in ["journal", "daily", "spend_7d_usd"] {
                assert!(object.contains_key(key), "missing {key}: {rendered}");
            }
            assert_eq!(
                object.contains_key("weekly_budget_usd"),
                has_budget,
                "weekly_budget_usd presence: {rendered}"
            );
            assert_eq!(
                object.contains_key("remaining_usd"),
                has_budget,
                "remaining_usd presence: {rendered}"
            );
            let spend = object["spend_7d_usd"].as_f64().expect("spend is a number");
            assert!(spend >= 0.0 && !spend.is_sign_negative(), "{rendered}");
            let daily = object["daily"].as_array().expect("daily is an array");
            for entry in daily {
                assert!(
                    entry["date"].is_string() && entry["spend_usd"].is_number(),
                    "{entry}"
                );
            }
            if let Some(budget) = budget {
                assert_eq!(object["weekly_budget_usd"].as_f64(), Some(budget));
            }
        }
    }

    #[test]
    fn budget_series_rises_per_day_and_windows_to_the_calendar_week() {
        // Day -10: session A burns 0 -> 2.00; day -9: A continues to
        // 3.50 while B starts fresh at 1.00 (epoch reset contributes
        // only itself). Days -8..-1: A restarts its counter and burns
        // 0.25 per day (cumulative 0.25, 0.50, ...) — a reset that
        // never rises again burns nothing. The window is the seven
        // CALENDAR days ending at `now`: with `now` pinned, days
        // -6..-1 are inside, `now`'s own day zero-fills (rm-917), and
        // everything older falls OUT of the weekly budget and into
        // `excluded_older_days` instead of hiding inside an
        // observed-days gap (the 2026-10-07 budget-window rider: the
        // window used to keep the first seven *observed* days, so
        // week-old spend still counted against the current weekly
        // budget).
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
        let base = now.timestamp();
        let day = |n: i64| base + n * 86_400;
        let mut captures = vec![
            budget_capture("A", day(-10) + 100, 2.00),
            budget_capture("A", day(-9) + 100, 3.50),
            budget_capture("B", day(-9) + 200, 1.00),
        ];
        for n in -8..=-1 {
            captures.push(budget_capture("A", day(n), 0.25 * (n + 9) as f64));
        }
        // Review 95d74221 F8: a future-dated capture (clock skew, a
        // hostile journal) must not bill into a window — tomorrow's
        // $99.00 stays outside both the 7-day and the full-journal
        // sums under the today-bounded window.
        captures.push(budget_capture("F", day(1) + 50, 99.00));
        let series = statusline_budget_series(&captures, 7, now);
        // Calendar week [now-6d .. now]: day -8 is a counter RESET
        // (its 0.25 sample sits below day -9's 3.50 cumulative, so its
        // rise is max(0, ·) = 0), and days -7..0 hold only the -7..-1
        // tail — each burning 0.25. `now` itself has no samples, so
        // six sampled entries of 0.25 plus rm-917's zero-filled
        // today — seven calendar rows, same 1.50 total.
        assert_eq!(series.daily.len(), 7, "{:?}", series.daily);
        assert!((series.total - 6.0 * 0.25).abs() < 1e-9, "{:?}", series);
        // rm-917: the four sampled days strictly older than the
        // calendar window (-10, -9, -8, -7) are excluded and
        // disclosed, not billed.
        assert_eq!(series.excluded_older_days, 4, "{series:?}");
        // The full-journal variant still sums all the burn: day -8
        // contributes 0 (the reset), so only the SEVEN tail days burn
        // 0.25 — 2.00 + 1.50 + 1.00 + 7×0.25 = 6.25 (tomorrow's $99
        // future capture excluded by the upper bound; rm-917
        // zero-fills the 20 unsampled in-window days).
        let full = statusline_budget_series(&captures, 30, now);
        assert!(
            (full.total - (2.00 + 1.50 + 1.00 + 7.0 * 0.25)).abs() < 1e-9,
            "{full:?}"
        );
        let now_day = now.date_naive().format("%Y-%m-%d").to_string();
        assert!(
            series
                .daily
                .iter()
                .chain(&full.daily)
                .all(|(day, _)| day.as_str() <= now_day.as_str()),
            "future-dated capture billed into a window: {:?}",
            series.daily
        );
    }

    #[test]
    fn user_cache_dir_treats_empty_env_as_unset() {
        // Review 95d74221 F2: `HOME=` or `XDG_CACHE_HOME=` must never
        // yield a RELATIVE cache dir — an empty HOME used to join
        // `.cache`/`Library/Caches` against the CWD, where a planted
        // statusline journal loaded into the budget view (live PoC:
        // `$999.00 ... OVER by $989.00`).
        use std::ffi::OsStr;
        let empty = Some(OsStr::new(""));
        // Both empty: the temp-dir fallback — always absolute.
        let both_empty = resolve_user_cache_dir(empty, empty);
        assert!(both_empty.is_absolute(), "{both_empty:?}");
        // Empty HOME with a set XDG override: the override wins.
        assert_eq!(
            resolve_user_cache_dir(empty, Some(OsStr::new("/xdg-cache"))),
            PathBuf::from("/xdg-cache")
        );
        // Unset both: the same ladder end.
        assert_eq!(resolve_user_cache_dir(None, None), std::env::temp_dir());
        #[cfg(not(target_os = "macos"))]
        {
            // Set HOME with an empty XDG value: the spec default
            // `$HOME/.cache`, not the RELATIVE `.cache` the empty
            // value used to produce.
            assert_eq!(
                resolve_user_cache_dir(Some(OsStr::new("/home/at-test")), empty),
                PathBuf::from("/home/at-test").join(".cache")
            );
        }
    }

    #[test]
    fn budget_series_ignores_captures_without_cost_fields() {
        let payload: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let capture = CapturedStatusline {
            captured_at: 1_000_000_000,
            payload,
        };
        // The shared fixture carries no `cost` object; verify by
        // stripping it defensively anyway.
        let mut stripped = capture;
        stripped.payload.as_object_mut().map(|o| o.remove("cost"));
        let series = statusline_budget_series(&[stripped], 7, Utc::now());
        assert_eq!(series.daily.len(), 0);
        assert_eq!(series.total, 0.0);
    }

    #[test]
    fn budget_series_excludes_and_discloses_older_days() {
        // rm-917: sampled days strictly older than the calendar window
        // are excluded from `daily`/`total` and disclosed in
        // `excluded_older_days` — distinct days, not captures.
        let mut captures = Vec::new();
        let mut cumulative = 0.0;
        for days_ago in (8..=20).rev() {
            captures.push(budget_capture(
                "A",
                epoch_on_day_offset(days_ago, 100),
                cumulative,
            ));
            cumulative += 1.00;
        }
        // All 13 sampled days are older than the trailing week.
        let stale = statusline_budget_series(&captures, 7, Utc::now());
        assert_eq!(stale.excluded_older_days, 13, "{stale:?}");
        assert!(stale.daily.is_empty());
        // Mixed: two in-window days plus the same stale span.
        captures.push(budget_capture("A", epoch_on_day_offset(0, 100), cumulative));
        captures.push(budget_capture(
            "A",
            epoch_on_day_offset(3, 100),
            cumulative + 2.00,
        ));
        let mixed = statusline_budget_series(&captures, 7, Utc::now());
        assert_eq!(mixed.excluded_older_days, 13, "{mixed:?}");
        assert_eq!(mixed.daily.len(), 7, "{:?}", mixed.daily);
        // In-window: today's rise 1.00 + D-3's rise 2.00 = 3.00.
        assert!((mixed.total - 3.00).abs() < 1e-9, "{mixed:?}");
        // A 30-day window still reaches back past the stale span's
        // oldest day (D-20): nothing is excluded.
        let wide = statusline_budget_series(&captures, 30, Utc::now());
        assert_eq!(wide.excluded_older_days, 0, "{wide:?}");
    }

    #[test]
    fn budget_view_renders_spend_budget_and_unconfigured_state() {
        // Pinned `now` (review 95d74221 determinism; the candidate's
        // real-today variant retired): both captures land inside
        // `now`'s UTC day, which rm-917's calendar window keeps as
        // today's row — the first capture (2.00) contributes itself,
        // the second adds a 1.00 rise.
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
        let base = now.timestamp();
        let captures = vec![
            budget_capture("A", base, 2.00),
            budget_capture("A", base + 3_600, 3.00),
        ];
        let series = statusline_budget_series(&captures, 7, now);
        let budget = Some(10.0);
        let text = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 2,
                bytes: 100,
                retained_max_bytes: 1024,
                capped_away_bytes: 0,
            },
            &statusline_insights(&captures),
            budget,
            &series,
        );
        assert!(text.contains("Budget: 7d spend $3.00 of $10.00"), "{text}");
        assert!(text.contains("$7.00 remaining"), "{text}");
        // rm-917: a journal with nothing inside the calendar window
        // discloses instead of verdicting.
        let stale_series = statusline_budget_series(
            &[budget_capture(
                "A",
                now.timestamp() - 20 * 86_400 + 100,
                2.00,
            )],
            7,
            now,
        );
        let stale_text = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 1,
                bytes: 50,
                retained_max_bytes: 1024,
                capped_away_bytes: 0,
            },
            &statusline_insights(&[]),
            budget,
            &stale_series,
        );
        assert!(
            stale_text.contains("Budget: no cost samples in the trailing 7 calendar days"),
            "{stale_text}"
        );
        assert!(!stale_text.contains("remaining"), "{stale_text}");
        let unbudgeted = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 2,
                bytes: 100,
                retained_max_bytes: 1024,
                capped_away_bytes: 0,
            },
            &statusline_insights(&captures),
            None,
            &series,
        );
        assert!(!unbudgeted.contains("Budget:"), "{unbudgeted}");
    }

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

    /// rm-898 helper: writes a capture journal strictly larger than the
    /// read cap (N one-line captures with distinct `captured_at`) and
    /// returns the path plus the exact expectations for a tail-capped
    /// read, derived from the bytes actually written so the torn-head
    /// drop is computed, never guessed: survivors are the lines starting
    /// strictly after the cut byte (the line containing the cut arrives
    /// torn and is dropped; a cut exactly on a boundary drops that whole
    /// first line too, so the tail never begins with a fragment).
    fn over_cap_journal(root: &Path) -> (PathBuf, u64, usize, i64) {
        const N: usize = 160_000;
        let lines: Vec<String> = (0..N)
            .map(|i| {
                format!(
                    "{{\"captured_at\":{},\"payload\":{{\"session_id\":\"cap-{:024}\"}}}}",
                    1_700_000_000i64 + i as i64,
                    i
                )
            })
            .collect();
        let mut raw = lines.join("\n");
        raw.push('\n');
        let journal = root.join("statusline.jsonl");
        fs::write(&journal, &raw).expect("write over-cap journal");
        let total = raw.len() as u64;
        assert!(
            total > STATUSLINE_READ_CAP_BYTES,
            "fixture must exceed the read cap"
        );
        let cut = total - STATUSLINE_READ_CAP_BYTES;
        let mut start = 0u64;
        let mut survivors = 0usize;
        let mut first_at = 0i64;
        for (idx, line) in lines.iter().enumerate() {
            let line_start = start;
            start += line.len() as u64 + 1;
            if line_start > cut {
                if survivors == 0 {
                    first_at = 1_700_000_000 + idx as i64;
                }
                survivors += 1;
            }
        }
        assert!(
            survivors > 0 && survivors < N,
            "the cut must land mid-journal"
        );
        (journal, cut, survivors, first_at)
    }

    #[test]
    fn reads_of_an_over_cap_journal_keep_only_the_newest_tail() {
        // rm-898: all three read sites used to load the WHOLE journal
        // into memory before any bounded handling — a hostile or runaway
        // journal was fully materialized. Reads now cap at
        // STATUSLINE_READ_CAP_BYTES (10 MiB, pinned here by the literal —
        // deliberately self-contained so the cap behavior is red-testable
        // against the unfixed reader) taken from the tail: only the
        // newest whole lines are returned, the torn head fragment never
        // leaks a capture, and the oldest captures are gone from the read
        // (disclosed by the stats arm below).
        const READ_CAP: u64 = 10 * 1024 * 1024;
        const N: usize = 160_000;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-capread-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let lines: Vec<String> = (0..N)
            .map(|i| {
                format!(
                    "{{\"captured_at\":{},\"payload\":{{\"session_id\":\"cap-{:024}\"}}}}",
                    1_700_000_000i64 + i as i64,
                    i
                )
            })
            .collect();
        let mut raw = lines.join("\n");
        raw.push('\n');
        let journal = root.join("statusline.jsonl");
        fs::write(&journal, &raw).expect("write over-cap journal");
        let total = raw.len() as u64;
        assert!(total > READ_CAP, "fixture must exceed the read cap");
        let cut = total - READ_CAP;
        // Survivors are the lines starting strictly after the cut byte
        // (the line containing the cut arrives torn and is dropped; a
        // cut exactly on a boundary drops that whole first line too) —
        // computed from the bytes actually written, never guessed.
        let mut start = 0u64;
        let mut survivors = 0usize;
        let mut first_at = 0i64;
        for (idx, line) in lines.iter().enumerate() {
            let line_start = start;
            start += line.len() as u64 + 1;
            if line_start > cut {
                if survivors == 0 {
                    first_at = 1_700_000_000 + idx as i64;
                }
                survivors += 1;
            }
        }
        assert!(
            survivors > 0 && survivors < N,
            "the cut must land mid-journal"
        );
        let captures = read_statusline_captures(&journal);
        assert_eq!(
            captures.len(),
            survivors,
            "only the retained tail's captures are returned, never the whole journal"
        );
        assert_eq!(
            captures.first().expect("tail has captures").captured_at,
            first_at,
            "the torn head line is dropped; reads begin at the first whole survivor"
        );
        assert_eq!(
            captures.last().expect("tail has captures").captured_at,
            1_700_000_000 + 159_999,
            "the newest capture survives the cap"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn over_cap_reads_are_disclosed_not_silently_undercounted() {
        // rm-898 disclosure arm: `bytes` still names the real journal
        // size (metadata), `lines` counts the retained tail, and
        // `capped_away_bytes` carries the cut so both the JSON surface
        // and the text report say what happened instead of letting the
        // counts read as the whole journal.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-capstats-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let (journal, cut, survivors, _first_at) = over_cap_journal(&root);
        let stats = statusline_journal_stats(&journal);
        assert!(stats.exists);
        assert_eq!(
            stats.capped_away_bytes, cut,
            "the exact head cut is disclosed"
        );
        assert_eq!(
            stats.lines, survivors,
            "line counts cover the retained tail only"
        );
        assert!(
            stats.bytes > STATUSLINE_READ_CAP_BYTES,
            "bytes keeps naming the whole journal (metadata, not the read)"
        );
        let captures = read_statusline_captures(&journal);
        let series = statusline_budget_series(&captures, 7, Utc::now());
        let text =
            render_statusline_report_text(&stats, &statusline_insights(&captures), None, &series);
        assert!(
            text.contains("head truncated"),
            "the text report names the truncation: {text}"
        );
        assert!(
            text.contains(&format!("{cut} of {} bytes unread", stats.bytes)),
            "the text report carries the exact cut: {text}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cut_landing_inside_a_multibyte_char_advances_to_the_next_boundary() {
        // rm-898 review fix (independent-review 9fb0a017 F2, fixed at ed90e5ec):
        // the raw cap cut can land INSIDE a multi-byte UTF-8 character. The
        // boundary walk must advance past the torn sequence so the retained
        // tail starts on a char boundary and capped_away_bytes is byte-exact
        // to that boundary — not stuck at the raw cut with a lossy head.
        let cap = STATUSLINE_READ_CAP_BYTES;
        // head: 97 ASCII lines of exactly 9 bytes ("line-NNN\n") = 873 bytes,
        // ending on a line boundary.
        let head: String = (0..97).map(|i| format!("line-{i:03}\n")).collect();
        let head_len = head.len() as u64; // 873
                                          //
                                          // tail blob: exactly cap-2 bytes, starting with '\n' so the torn char
                                          // forms its own complete line, then ASCII filler lines.
        let mut tail = String::from("\n");
        let mut filler_len = (cap - 3) as usize;
        while filler_len >= 10 {
            tail.push_str("filler-aa\n"); // fixed 10 bytes, no counter growth
            filler_len -= 10;
        }
        if filler_len > 0 {
            tail.push_str(&"x".repeat(filler_len));
        }
        let tail_len = tail.len() as u64;
        assert_eq!(tail_len, cap - 2, "fixture arithmetic");
        // file = [873 ASCII][E4 B8 AD][cap-2 bytes]; raw cut = total-cap lands
        // on B8, one byte into the 3-byte char.
        let contents = format!("{head}\u{4E2D}{tail}");
        let total = head_len + 3 + tail_len;
        let journal = std::env::temp_dir().join("agenttrace-test-statusline-torn-utf8.jsonl");
        std::fs::write(&journal, contents.as_bytes()).unwrap();
        let stats = statusline_journal_stats(&journal);
        // The walk advances B8 -> AD and stops at the '\n' (an ASCII start):
        // away is byte-exact to the next char boundary, never the raw cut.
        assert_eq!(
            stats.capped_away_bytes,
            head_len + 3,
            "capped_away_bytes must advance past the torn multi-byte sequence to the next char boundary"
        );
        assert_eq!(stats.bytes, total, "bytes keeps the real file size");
        assert!(stats.lines > 100, "the ASCII filler lines survive the cut");
        std::fs::remove_file(&journal).ok();
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
        // Review fix: the ST-terminated (ESC \) form of an OSC is
        // pinned too — not just the BEL form above.
        let st_form = serde_json::json!({
            "session_name": "\u{001b}]0;title\u{001b}\\Opus",
            "model": { "display_name": "Opus" }
        });
        let line = render_status_line(&st_form);
        assert!(
            line.chars().all(|c| !c.is_control()),
            "ST-terminated OSC sanitized: {line:?}"
        );
    }

    #[test]
    fn statusline_report_sanitizes_journal_derived_strings() {
        // rm-034 regression: the journal is data (append_statusline_capture
        // stores what it saw verbatim, by design), so every journal-derived
        // string the text report prints — session ids and miss-cause names —
        // must pass the same control-character sanitization as the render
        // path. Live repro 2026-09-30 (assess attempt 6c039674): a crafted
        // statusline payload once captured emitted raw ANSI SGR + OSC-52
        // (clipboard-write) sequences through `agenttrace --statusline-report`.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-report-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        // Point the environment at the temp root before any append (the
        // default path is the user's real journal), under the shared env
        // lock so sibling-module tests cannot re-point it mid-append.
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let payload = serde_json::json!({
            "session_id": "probe2-RED\u{001b}[31mX",
            "prompt_cache": {
                "hit_ratio": 0.5,
                "misses": 3,
                "last_miss_cause": {
                    "causes": ["tools\u{001b}[2Jchanged\r\nnewline"]
                },
                "miss_causes": {"esc\u{001b}]52;c;aGk=\u{0007} OSC52": 2}
            }
        });
        append_statusline_capture(&payload).expect("append hostile capture");
        let text = render_statusline_report("text", None).expect("text report renders");
        // cat -v equivalence: no raw control bytes may reach the terminal
        // beyond the report's own line breaks.
        assert!(
            text.lines()
                .all(|line| line.chars().all(|c| !c.is_control())),
            "no control bytes reach the terminal: {text:?}"
        );
        assert!(
            text.contains('\u{FFFD}'),
            "control characters are replaced visibly: {text:?}"
        );
        assert!(
            !text.contains('\u{001b}'),
            "no raw ESC byte survives anywhere: {text:?}"
        );
        assert!(
            !text.contains("\u{001b}]52;"),
            "the OSC-52 clipboard-write opener is neutralized: {text:?}"
        );
        // The JSON path keeps the payload verbatim (serde escapes it) —
        // sanitization is a print-site concern, not a data concern.
        let json = render_statusline_report("json", None).expect("json report renders");
        let value: Value = serde_json::from_str(&json).expect("json report parses");
        assert_eq!(
            value["insights"]["session_caches"][0]["session_id"], "probe2-RED\u{001b}[31mX",
            "JSON output is unchanged: raw payload, serde-escaped"
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_compacts_stay_coherent_and_leave_no_fixed_temp() {
        // rm-202: the compaction temp used to be a FIXED
        // `<name>.jsonl.compact` sibling, so two concurrent compacts
        // interleaved their writes into one shared file (a torn journal
        // renamed into place), and a crash before the rename orphaned a
        // name no sweep knows. With a per-writer unique temp each
        // compact renames a coherent whole file, and a crash orphan
        // matches the `.tmp.` pattern the cache-load sweep already
        // removes.
        use std::sync::{Arc, Barrier};
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-race-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        // Rounds of two concurrent compacts over a ~5 MiB journal (the
        // retention bound is 10 MiB): large enough that both writers
        // hold their temp open simultaneously under a shared name, and
        // many rounds so a racy window is not missed by luck.
        for round in 0..10 {
            let mut raw = String::new();
            for i in 0..30_000 {
                raw.push_str(&format!(
                    "{{\"captured_at\":{i},\"payload\":{{\"filler\":\"{}\"}}}}\n",
                    "x".repeat(150)
                ));
            }
            fs::write(&journal, &raw).expect("seed journal");
            let barrier = Arc::new(Barrier::new(2));
            let writers: Vec<_> = [512 * 1024u64, 1024 * 1024]
                .map(|keep| {
                    let journal = journal.clone();
                    let barrier = Arc::clone(&barrier);
                    std::thread::spawn(move || {
                        barrier.wait();
                        compact_statusline_capture_under(&journal, keep)
                    })
                })
                .into_iter()
                .collect();
            for writer in writers {
                writer
                    .join()
                    .expect("writer thread must not panic")
                    .unwrap_or_else(|error| {
                        panic!("round {round}: concurrent compact must not fail: {error}")
                    });
            }
            // Whichever compact won the last rename, every surviving
            // line must be whole — a truncated or interleaved line is a
            // torn journal handed to every reader.
            for line in fs::read_to_string(&journal)
                .expect("journal survives the race")
                .lines()
            {
                serde_json::from_str::<CapturedStatusline>(line).unwrap_or_else(|error| {
                    panic!("round {round}: torn line survived compaction: {error}: {line:?}")
                });
            }
            assert!(
                !journal.with_extension("jsonl.compact").exists(),
                "the fixed-name compaction temp must never appear"
            );
            let leftovers: Vec<_> = fs::read_dir(&root)
                .expect("read journal dir")
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
                .collect();
            assert!(
                leftovers.is_empty(),
                "temps must not survive the renames: {leftovers:?}"
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn journal_and_compact_rewrite_are_owner_only() {
        // rm-208: the journal is captured conversation context (session
        // names, project paths, working directories) — a group/world-
        // readable copy hands it to every local account. It must be
        // created owner-only, and the compact rewrite (temp + rename)
        // must not relax that.
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-perms-{}-{:?}",
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
        append_statusline_capture(&payload).expect("append capture");
        let mode = fs::metadata(&journal)
            .expect("journal exists")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "journal must be owner-only, got {:o}",
            mode & 0o777
        );
        compact_statusline_capture_under(&journal, 1).expect("compact");
        let mode = fs::metadata(&journal)
            .expect("journal survives compaction")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "compaction must not relax journal privacy, got {:o}",
            mode & 0o777
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }
    // rm-166 regression fixture: hammer the journal from parallel
    // processes while a compactor continuously rewrites it, then prove
    // no append was lost. Without the journal lock, an append racing the
    // compaction's rename landed on the replaced inode and vanished.
    //
    // The child processes re-execute this test binary (`--exact` this
    // test) with AGENTTRACE_JOURNAL_CHILD_ROLE set, taking the child
    // branch below; coordination between processes is marker files in
    // the temp root.
    #[test]
    fn journal_appends_survive_parallel_compaction() {
        if let Ok(role) = std::env::var("AGENTTRACE_JOURNAL_CHILD_ROLE") {
            journal_race_child(&role);
            return;
        }
        let root = std::env::temp_dir().join(format!(
            "agenttrace-journal-race-{}-{}",
            std::process::id(),
            format!("{:?}", std::thread::current().id()).replace(char::is_alphabetic, "")
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).expect("create temp root");
        // Child runs are matched by the leaf name as a substring filter —
        // unique in this binary. (`--exact` with a constructed module path
        // is a trap: module_path!() carries a crate-name prefix libtest does
        // not use, and the mismatched child silently runs zero tests and
        // "passes" instantly.)
        let test_name = "journal_appends_survive_parallel_compaction";
        let exe = std::env::current_exe().expect("test binary path");
        let spawn = |role: String| {
            let mut command = std::process::Command::new(&exe);
            command
                .arg(test_name)
                .env("AGENTTRACE_JOURNAL_CHILD_ROLE", role)
                .env("AGENTTRACE_JOURNAL_ROOT", &root)
                .env("AGENTTRACE_SESSION_CACHE_DIR", &root)
                // Captured (not inherited) so child libtest chatter stays
                // out of the parent's output and stderr is available for
                // the failure message below.
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            command.spawn().expect("spawn journal race child")
        };
        const WRITERS: usize = 3;
        const HAMMER: usize = 500;
        let mut children = vec![spawn("compactor".to_string())];
        for writer in 0..WRITERS {
            children.push(spawn(format!("writer:{writer}")));
        }
        for (index, child) in children.into_iter().enumerate() {
            let output = child.wait_with_output().expect("journal race child exits");
            assert!(
                output.status.success(),
                "journal race child {index} failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        // The fixture is only a race if the compactor actually swapped the
        // journal under the writers; a degenerate run (compactor erroring
        // every cycle) would otherwise pass vacuously.
        let swaps: u32 = fs::read_to_string(root.join("compactor-swaps"))
            .expect("compactor swap count marker")
            .trim()
            .parse()
            .expect("swap count parses");
        assert!(
            swaps > 0,
            "compactor never swapped the journal — fixture did not race"
        );
        let journal = root.join("statusline.jsonl");
        let captures = read_statusline_captures(&journal);
        let ids: Vec<String> = captures
            .iter()
            .filter_map(|capture| capture.session_id().map(str::to_string))
            .collect();
        // 1. Every hammer id must survive: the compactor runs under a
        //    keep-everything bound (see below), so a missing id is an
        //    append lost to a rename race — the exact rm-166 defect.
        for writer in 0..WRITERS {
            for index in 0..HAMMER {
                let id = format!("w{writer}-{index}");
                assert!(ids.contains(&id), "append {id} lost to a compaction race");
            }
        }
        // 2. Every sentinel must survive. Sentinels are appended while
        //    the compactor is still renaming and are the newest lines at
        //    rest, so a missing sentinel is an append lost to a rename
        //    race, not retention.
        for writer in 0..WRITERS {
            let sentinel = format!("sentinel-{writer}");
            assert!(
                ids.contains(&sentinel),
                "sentinel {sentinel} lost to a compaction race"
            );
        }
        // 3. Integrity: no id appears twice and each writer's ids appear
        //    in strictly increasing file order — O_APPEND preserves
        //    per-writer order, so a reordered or duplicated id means the
        //    lock protocol failed. (Retention semantics have their own
        //    fixtures; this one is about the append-vs-rename race.)
        let mut seen: HashSet<String> = HashSet::new();
        let mut last_index: BTreeMap<usize, usize> = BTreeMap::new();
        for capture in &captures {
            let Some(id) = capture.session_id().map(str::to_string) else {
                continue;
            };
            let Some((writer, index)) = id
                .strip_prefix('w')
                .and_then(|rest| rest.split_once('-'))
                .map(|(writer, index)| (writer.parse::<usize>(), index.parse::<usize>()))
            else {
                continue; // sentinels checked above
            };
            let (writer, index) = (writer.expect("writer index"), index.expect("entry index"));
            assert!(seen.insert(id), "journal contains a duplicated id");
            let last = last_index.entry(writer).or_insert(index);
            assert!(
                index >= *last,
                "writer {writer} entry {index} appears after {:?} — journal order broken",
                last
            );
            *last = index;
        }
        fs::remove_dir_all(&root).ok();
    }

    // rm-166 mechanism, pinned deterministically: a compaction renames a
    // fresh inode over the journal, so any handle opened before the swap
    // (the pre-lock protocol's O_APPEND fd) writes into the replaced inode
    // and the entry never reaches the journal at the path. This is the
    // loss the cross-process fixture races for; here it is forced by
    // interleaving.
    #[test]
    fn journal_lockless_handle_loses_append_to_compaction() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-journal-mechanism-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp root");
        // Under the shared env lock so sibling tests cannot re-point the
        // cache root mid-append; prior value restored before release.
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let journal = statusline_capture_path();
        append_statusline_capture(&serde_json::json!({ "session_id": "seed" }))
            .expect("seed append");
        // The stale-handle interleaving: open as the pre-lock code did,
        // let the compaction swap the inode, then write through the stale
        // handle.
        let mut stale =
            crate::session_cache::open_private_append(&journal).expect("open stale append handle");
        compact_statusline_capture_under(&journal, u64::MAX).expect("compaction swaps inode");
        writeln!(stale, "{{\"lost\": true}}").expect("write through stale handle");
        let after_swap = fs::read_to_string(&journal).expect("journal readable");
        assert!(
            !after_swap.contains("\"lost\": true"),
            "write through a stale handle reached the journal at the path"
        );
        // The locked protocol lands the same append the compaction raced:
        // append_statusline_capture revalidates the inode under the lock,
        // so its write always reaches the journal the path names now.
        append_statusline_capture(&serde_json::json!({ "session_id": "kept" }))
            .expect("locked append");
        let after_append = fs::read_to_string(&journal).expect("journal readable");
        assert!(after_append.contains("\"session_id\":\"kept\""));
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(&root);
    }

    fn journal_race_child(role: &str) {
        let root = PathBuf::from(std::env::var_os("AGENTTRACE_JOURNAL_ROOT").expect("race root"));
        let journal = root.join("statusline.jsonl");
        let wait_for = |marker: &Path| {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while !marker.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "timed out waiting for {marker:?}"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        };
        const WRITERS: usize = 3;
        const HAMMER: usize = 500;
        let hammer_done = |writer: usize| root.join(format!("hammer-done-{writer}"));
        if let Some(writer) = role.strip_prefix("writer:") {
            let writer: usize = writer.parse().expect("writer index");
            for index in 0..HAMMER {
                let payload = serde_json::json!({ "session_id": format!("w{writer}-{index}") });
                append_statusline_capture(&payload).expect("child append");
            }
            fs::write(hammer_done(writer), b"").expect("hammer marker");
            wait_for(&root.join("sentinel-go"));
            let payload = serde_json::json!({ "session_id": format!("sentinel-{writer}") });
            append_statusline_capture(&payload).expect("sentinel append");
            fs::write(root.join(format!("sentinel-done-{writer}")), b"").expect("sentinel marker");
        } else if role == "compactor" {
            // keep-everything bound: the rewrite still swaps the inode in
            // under the writers (that is the racing surface), but retention
            // cannot delete a hammer id — the parent asserts all of them.
            let keep_under = u64::MAX;
            let mut swaps = 0u32;
            // Phase 1: tight loop — the rename window stays continuously
            // open while the writers hammer.
            while !(0..WRITERS).all(|writer| hammer_done(writer).exists()) {
                if compact_statusline_capture_under(&journal, keep_under).is_ok() {
                    swaps += 1;
                }
            }
            let _ = fs::write(root.join("compactor-swaps"), swaps.to_string());
            fs::write(root.join("sentinel-go"), b"").expect("sentinel-go marker");
            // Phase 2: keep rewriting until every sentinel has landed, so
            // the sentinel appends genuinely race the rename, then settle
            // with one final compaction.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            while !(0..WRITERS).all(|writer| root.join(format!("sentinel-done-{writer}")).exists())
            {
                let _ = compact_statusline_capture_under(&journal, keep_under);
                assert!(
                    std::time::Instant::now() < deadline,
                    "compactor timed out waiting for sentinels"
                );
            }
            compact_statusline_capture_under(&journal, keep_under).expect("final compaction");
        } else {
            panic!("unknown journal race role {role}");
        }
    }

    #[test]
    fn sanitize_output_document_kills_esc_bel_del_c1_and_keeps_layout() {
        let poisoned = concat!(
            "a\u{1b}]52;c;aGVsbG8gd29ybGQ=\u{07}b\u{1b}[2J\u{1b}[Hc\u{7f}d\u{9b}!pe",
            "\u{1b}]52;c;aGVsbG8gd29ybGQ=\u{1b}\\e",
        );
        // The last sequence is the string-terminator (ST, ESC \) form of
        // the same OSC-52 — an implementation that only handled the
        // BEL-terminated form would pass without it (review fix).
        let sanitized = sanitize_output_document(poisoned);
        assert!(!sanitized
            .chars()
            .any(|c| c != '\n' && c != '\r' && c != '\t' && c.is_control()));
        // Prefixes survive: the readable name around the sequence is kept,
        // so reports stay legible after neutralization.
        assert!(sanitized.starts_with('a') && sanitized.contains('b'));
        assert_eq!(
            sanitized.matches('\u{FFFD}').count(),
            poisoned.chars().filter(|c| c.is_control()).count()
        );
        // Layout bytes are document structure, not attacks.
        assert_eq!(
            sanitize_output_document("l1\nl2\r\n\tcol\n"),
            "l1\nl2\r\n\tcol\n"
        );
    }

    #[test]
    fn sanitize_output_document_is_idempotent_and_composes_with_line_segment() {
        let poisoned = "name\u{1b}[2J and \u{1b}]52;c;AAAA\u{07} tail";
        let once = sanitize_output_document(poisoned);
        let twice = sanitize_output_document(&once);
        assert_eq!(once, twice, "rm-625 choke point must be idempotent");
        // Composition with the rm-383 line sanitizer (cells sanitized
        // before the document) must be a fixed point too.
        let line_first = sanitize_output_document(&sanitize_line_segment(poisoned));
        assert_eq!(line_first, sanitize_output_document(poisoned));
        // Tab honesty (review fix): 0x09 is layout for the DOCUMENT
        // sanitizer — tabs survive it, including mid-line next to a
        // neutralized escape. The line sanitizer is stricter (a status
        // line is one line, so it replaces 0x09 too), which is why the
        // two orders are intentionally NOT byte-equal for tab-bearing
        // input; both are equally safe.
        let tabbed = "col1\t\u{1b}[2Jcol2\u{1b}]52;c;AAAA\u{07}\tcol3";
        let document_only = sanitize_output_document(tabbed);
        assert_eq!(
            document_only.matches('\t').count(),
            2,
            "tabs are layout for the document sanitizer: {document_only:?}"
        );
        assert!(document_only.contains("\u{FFFD}[2Jcol2"));
        let line_first = sanitize_output_document(&sanitize_line_segment(tabbed));
        assert!(
            !line_first.contains('\t'),
            "the line sanitizer is stricter — tabs die there: {line_first:?}"
        );
        for rendered in [&document_only, &line_first] {
            assert!(
                !rendered
                    .chars()
                    .any(|c| c.is_control() && c != '\t' && c != '\n' && c != '\r'),
                "no non-layout control survives either order: {rendered:?}"
            );
        }
    }
}

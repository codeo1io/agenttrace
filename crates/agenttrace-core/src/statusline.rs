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

/// rm-821: read bound for journal consumers, aligned with the
/// retention cap ([`STATUSLINE_CAPTURE_MAX_BYTES`]) — the same whole
/// the compaction lane keeps. Reads beyond this size stop at the cap
/// and DISCLOSE (see [`StatuslineTornTail::capped_away_bytes`]);
/// they never truncate silently.
const STATUSLINE_READ_MAX_BYTES: u64 = STATUSLINE_CAPTURE_MAX_BYTES;

/// rm-821: bounded journal read. Returns the (up to) cap-sized prefix
/// as UTF-8 plus `Some((bytes_read, file_bytes))` when the file was
/// larger than the cap — the disclosed marker's raw material.
fn read_journal_capped(path: &Path) -> (String, Option<(u64, u64)>) {
    let Ok(file) = fs::File::open(path) else {
        return (String::new(), None);
    };
    let file_bytes = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let take = file_bytes.min(STATUSLINE_READ_MAX_BYTES);
    let mut raw = Vec::with_capacity(take as usize);
    let mut limited = (&file).take(take);
    // rm-821 (review fix F3): read BYTES, then decode lossily. The
    // old `read_to_string` returned Err the moment the cap boundary
    // split a multibyte char (or stray junk landed mid-file) and this
    // arm answered `(empty, None)` — a VALID journal silently read
    // as missing: the report printed "no captures yet" while
    // `stats.bytes` still showed the true >10 MiB size. Lossy decode
    // keeps every parseable line: a straddled or poisoned byte
    // becomes U+FFFD inside its own line, which the per-line JSON
    // parse below already tolerates and the torn-tail walk discloses
    // as an unparsed line — never a silent whole-journal drop.
    if std::io::Read::read_to_end(&mut limited, &mut raw).is_err() {
        // A real I/O error (not an encoding artifact) reads the same
        // as a missing journal.
        return (String::new(), None);
    }
    let text = String::from_utf8_lossy(&raw).into_owned();
    let capped = (file_bytes > take).then_some((take, file_bytes));
    (text, capped)
}

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

fn compact_statusline_capture_locked(path: &Path, keep_under: u64) -> io::Result<()> {
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
    // rm-821: the read is bounded by the module's retention cap — an
    // unbounded journal (a runaway host appending forever) must not
    // balloon an inspection command's memory. The bound is disclosed
    // by [`statusline_torn_tail`], never silently truncating.
    let (raw, _capped) = read_journal_capped(path);
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// rm-821: what [`read_statusline_captures`] had to drop. The
/// append-only journal tolerates a torn tail (a crashed append leaves
/// a final line without its newline, which is not valid JSON), and
/// the report used to skip those bytes SILENTLY — the "N captures"
/// line just understated the journal the reader was looking at.
/// `last_intact_captured_at` anchors the hole: the epoch of the last
/// line that DID parse, i.e. the moment before the capture gap.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct StatuslineTornTail {
    /// Non-empty lines that failed to parse as JSON.
    pub unparsed_lines: usize,
    /// Bytes those lines occupy in the journal (newline included).
    pub unparsed_bytes: u64,
    /// The journal ends without a newline — the classic crashed-append
    /// shape. Malformed lines mid-file (a full, poisoned line) do not
    /// set this, and are disclosed separately by the counts above.
    pub torn_final_line: bool,
    /// Epoch of the last line that parsed, if any.
    pub last_intact_captured_at: Option<i64>,
    /// rm-821: bytes beyond the read cap that were NOT read (file
    /// larger than [`STATUSLINE_READ_MAX_BYTES`]). Zero on every
    /// healthy journal; nonzero is always disclosed.
    pub capped_away_bytes: u64,
}

impl StatuslineTornTail {
    fn is_quiet(&self) -> bool {
        self.unparsed_lines == 0 && !self.torn_final_line && self.capped_away_bytes == 0
    }
}

/// Measures the journal's unparseable tail region. Reads the same
/// bytes [`read_statusline_captures`] does, with the same per-line
/// tolerance — this never fails, it only reports.
pub fn statusline_torn_tail(path: &Path) -> StatuslineTornTail {
    let mut tail = StatuslineTornTail::default();
    let (raw, capped) = read_journal_capped(path);
    if let Some((_read, file_bytes)) = capped {
        tail.capped_away_bytes = file_bytes - STATUSLINE_READ_MAX_BYTES;
    }
    if !raw.is_empty() && !raw.ends_with('\n') {
        tail.torn_final_line = true;
    }
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<CapturedStatusline>(line) {
            Ok(capture) => tail.last_intact_captured_at = Some(capture.captured_at),
            Err(_) => {
                tail.unparsed_lines += 1;
                tail.unparsed_bytes += line.len() as u64 + 1;
            }
        }
    }
    tail
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
    /// rm-821 (review fix F3): bytes beyond the read cap that were
    /// NOT read when these insights were aggregated. Zero on every
    /// healthy journal; nonzero means every count above describes
    /// only the capped prefix — this lane used to have no cap
    /// disclosure at all. Skipped at zero so healthy-journal JSON
    /// stays byte-compatible.
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub journal_capped_away_bytes: u64,
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
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
        journal_capped_away_bytes: 0,
    }
}

/// Loads and aggregates the journal at the default path. Returns `None`
/// when nothing is captured yet so reports stay unchanged until the
/// statusline mode is actually used.
pub fn load_statusline_insights() -> Option<StatuslineInsights> {
    let path = statusline_capture_path();
    let captures = read_statusline_captures(&path);
    if captures.is_empty() {
        return None;
    }
    let mut insights = statusline_insights(&captures);
    // rm-821 (review fix F3): the TUI efficiency panel consumed the
    // capped journal prefix with no cap lane of its own — prefix-only
    // limit counts presented as the whole journal. Same marker the
    // report arm prints, surfaced on the struct the panel already
    // renders.
    insights.journal_capped_away_bytes = statusline_torn_tail(&path).capped_away_bytes;
    Some(insights)
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
    // rm-821: counted from the capped read (the prefix the report
    // actually aggregates); `bytes` above stays the TRUE file size so
    // the header is honest about what the cap left unread.
    let lines = read_journal_capped(path)
        .0
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    StatuslineJournalStats {
        path: path.to_string_lossy().to_string(),
        exists,
        lines,
        bytes,
        retained_max_bytes: STATUSLINE_CAPTURE_MAX_BYTES,
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
    let stats = statusline_journal_stats(&path);
    let captures = read_statusline_captures(&path);
    let torn = statusline_torn_tail(&path);
    let insights = statusline_insights(&captures);
    let series = statusline_budget_series(&captures, 7);
    if format == "json" {
        let mut value = serde_json::json!({ "journal": stats, "insights": insights });
        // rm-821: the JSON view discloses what the tolerant reader
        // dropped, so an operator diffing `journal.lines` against the
        // insight counts can see the gap is unparseable tail bytes,
        // not sessions that never existed.
        if !torn.is_quiet() {
            value["torn_tail"] = serde_json::to_value(&torn)?;
        }
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
        &torn,
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
    torn: &StatuslineTornTail,
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
    // rm-821: a tolerant reader that skips unparseable tail bytes owes
    // the operator the size of what it skipped — the "N captures"
    // line above counts raw lines including the bytes that never
    // parsed, so the gap was invisible arithmetic until now.
    if !torn.is_quiet() {
        let anchored = torn
            .last_intact_captured_at
            .map(|epoch| format!(", last intact capture {}", format_epoch(epoch)))
            .unwrap_or_default();
        let shape = if torn.torn_final_line {
            "a torn tail (crashed append, no trailing newline)"
        } else {
            "malformed line(s)"
        };
        out.push_str(&format!(
            "  ⚠ {} byte(s) across {} line(s) failed to parse as JSON — {shape}{anchored}; \
             they are excluded from every count above\n",
            torn.unparsed_bytes, torn.unparsed_lines
        ));
    }
    // rm-821: the read bound's own marker — separate from the parse
    // gap above because a capped journal can parse perfectly clean.
    if torn.capped_away_bytes > 0 {
        out.push_str(&format!(
            "  ⚠ journal exceeds the {} byte read cap; {} byte(s) beyond it were NOT read \
             and every count above covers only the capped prefix\n",
            STATUSLINE_READ_MAX_BYTES, torn.capped_away_bytes
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
    if let Some(budget) = weekly_budget {
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
#[derive(Debug, Clone, PartialEq)]
pub struct StatuslineBudgetSeries {
    /// UTC day (`YYYY-MM-DD`) -> spend in USD, ascending.
    pub daily: Vec<(String, f64)>,
    /// Sum of `daily`.
    pub total: f64,
}

pub fn statusline_budget_series(
    captures: &[CapturedStatusline],
    days: usize,
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
    // Keep only the trailing `days` window.
    let mut window: Vec<(String, f64)> = daily.into_iter().collect();
    if window.len() > days {
        window = window.split_off(window.len() - days);
    }
    // Review 5b9a9470 F4: an empty window's f64 reduction is lowered
    // by LLVM to the additive identity -0.0 in optimized builds, which
    // rendered as "$-0.00" at both budget display sites (the fresh-
    // install default carries no cost samples). Adding +0.0 normalizes
    // the sign bit and changes no other value (-0.0 + 0.0 == +0.0).
    let total = window.iter().map(|(_, spend)| *spend).sum::<f64>() + 0.0;
    StatuslineBudgetSeries {
        daily: window,
        total,
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
    let stats = statusline_journal_stats(&path);
    let captures = read_statusline_captures(&path);
    let torn = statusline_torn_tail(&path);
    let series = statusline_budget_series(&captures, 7);
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
        });
        // rm-821: same disclosure contract as the report view.
        if !torn.is_quiet() {
            value["torn_tail"] = serde_json::to_value(&torn)?;
        }
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
    let captures = read_statusline_captures(&path);
    let series = statusline_budget_series(&captures, 7);
    let mut out = String::from("Weekly budget — last 7 days (statusline journal)\n");
    for (day, spend) in &series.daily {
        out.push_str(&format!("  {day}  ${spend:.2}\n"));
    }
    if series.daily.is_empty() {
        out.push_str("  (no cost samples in the window)\n");
    }
    // rm-821 (review fix F2): branch on the two components exactly
    // like the report arm — a cap-only journal (capped_away_bytes > 0,
    // nothing unparsed) or a torn final line that still parses must
    // never mint a false "0 bytes across 0 lines failed to parse"
    // sentence, and the read cap gets its own marker in this lane
    // (it had none). `statusline_torn_tail` sets `torn_final_line`
    // whenever the taken prefix lacks a trailing newline — including
    // when the CAP itself made the cut, hence the 3-shape matrix.
    if torn.unparsed_lines > 0 {
        let anchored = torn
            .last_intact_captured_at
            .map(|epoch| format!(", last intact capture {}", format_epoch(epoch)))
            .unwrap_or_default();
        let shape = if torn.torn_final_line && torn.capped_away_bytes > 0 {
            "the read cap cut mid-line"
        } else if torn.torn_final_line {
            "a torn tail (crashed append, no trailing newline)"
        } else {
            "malformed line(s)"
        };
        out.push_str(&format!(
            "  ⚠ {} byte(s) across {} line(s) failed to parse as JSON — {shape}{anchored}; \
             they are excluded from the spend above\n",
            torn.unparsed_bytes, torn.unparsed_lines
        ));
    } else if torn.torn_final_line && torn.capped_away_bytes == 0 {
        // Torn-but-parses (crashed append whose final line is a
        // complete capture sans newline): nothing failed to parse
        // and nothing was excluded — say exactly that.
        out.push_str(
            "  ⚠ journal ends without a trailing newline (crashed append shape); \
             the final line still parsed — nothing was excluded from the spend above\n",
        );
    }
    if torn.capped_away_bytes > 0 {
        // Cap-only or cap-cut-mid-line with a clean prefix: this
        // sentence is the whole truth — bytes beyond the cap were
        // never read, the parseable prefix is fully counted.
        out.push_str(&format!(
            "  ⚠ journal exceeds the {} byte read cap; {} byte(s) beyond it were NOT read \
             — the spend above covers only the capped prefix\n",
            STATUSLINE_READ_MAX_BYTES, torn.capped_away_bytes
        ));
    }
    out.push_str(&format!("  7d total ${:.2}\n", series.total));
    match weekly_budget {
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

    #[test]
    fn budget_series_empty_window_total_is_positive_zero() {
        // Review 5b9a9470 F4: LLVM lowers an empty f64 reduction to the
        // additive identity -0.0 in optimized builds, which rendered as
        // "$-0.00" for every fresh install (no cost samples). The sum
        // site normalizes with `+ 0.0`; this pins the sign bit so no
        // future change reintroduces negative zero.
        let series = statusline_budget_series(&[], 7);
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
    fn budget_series_rises_per_day_and_windows_to_seven_days() {
        // Day 1: session A burns 0 -> 2.00; day 2: A continues to 3.50
        // while B starts fresh at 1.00 (epoch reset contributes only
        // itself). Days 3..10: A restarts its counter and burns 0.25
        // per day (cumulative 0.25, 0.50, ...) — a reset that never
        // rises again burns nothing.
        let day = |n: i64| n * 86_400;
        let mut captures = vec![
            budget_capture("A", day(1) + 100, 2.00),
            budget_capture("A", day(2) + 100, 3.50),
            budget_capture("B", day(2) + 200, 1.00),
        ];
        for n in 3..=10 {
            captures.push(budget_capture("A", day(n), 0.25 * (n - 2) as f64));
        }
        let series = statusline_budget_series(&captures, 7);
        // Window keeps the trailing 7 days: day 3 (rise 0 after the
        // reset) drops off, days 4..=10 burn 0.25 each.
        assert_eq!(series.daily.len(), 7, "{:?}", series.daily);
        assert!((series.total - 7.0 * 0.25).abs() < 1e-9, "{:?}", series);
        // The full-journal variant still sums the early burn.
        let full = statusline_budget_series(&captures, 30);
        assert!(
            (full.total - (2.00 + 1.50 + 1.00 + 7.0 * 0.25)).abs() < 1e-9,
            "{full:?}"
        );
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
        let series = statusline_budget_series(&[stripped], 7);
        assert_eq!(series.daily.len(), 0);
        assert_eq!(series.total, 0.0);
    }

    #[test]
    fn budget_view_renders_spend_budget_and_unconfigured_state() {
        let captures = vec![
            budget_capture("A", 1_000_000_000, 2.00),
            budget_capture("A", 1_000_000_000 + 3_600, 3.00),
        ];
        let series = statusline_budget_series(&captures, 7);
        let budget = Some(10.0);
        let quiet_tail = StatuslineTornTail::default();
        let text = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 2,
                bytes: 100,
                retained_max_bytes: 1024,
            },
            &statusline_insights(&captures),
            budget,
            &series,
            &quiet_tail,
        );
        assert!(text.contains("Budget: 7d spend $3.00 of $10.00"), "{text}");
        assert!(text.contains("$7.00 remaining"), "{text}");
        let unbudgeted = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 2,
                bytes: 100,
                retained_max_bytes: 1024,
            },
            &statusline_insights(&captures),
            None,
            &series,
            &quiet_tail,
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
    fn torn_tail_is_measured_and_disclosed_not_skipped_silently() {
        // rm-821 acceptance: build a journal with an intact line, a
        // malformed FULL line, and a torn final line (no newline), then
        // require the measurement to report both drops, the torn shape,
        // and the epoch of the last intact capture — and both render
        // arms to carry the same disclosure (JSON field + text line).
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-torn-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        let intact: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let capture = CapturedStatusline {
            captured_at: 1_700_000_000,
            payload: intact.clone(),
        };
        let poisoned = "{\"not\": \"a capture\"}";
        let torn_tail_text = "{\"captured_at\":2,\"payload\":{\"tor";
        let mut raw = String::new();
        raw.push_str(&serde_json::to_string(&capture).expect("serialize"));
        raw.push('\n');
        raw.push_str(poisoned);
        raw.push('\n');
        raw.push_str(torn_tail_text);
        fs::write(&journal, &raw).expect("write journal");

        let torn = statusline_torn_tail(&journal);
        assert_eq!(torn.unparsed_lines, 2, "poisoned + torn both counted");
        assert_eq!(
            torn.unparsed_bytes,
            (poisoned.len() + 1 + torn_tail_text.len() + 1) as u64,
            "bytes include each line's newline"
        );
        assert!(torn.torn_final_line, "no trailing newline = torn tail");
        assert_eq!(
            torn.last_intact_captured_at,
            Some(1_700_000_000),
            "the intact line anchors the hole"
        );

        // Both render arms disclose the same facts (env pin keeps the
        // render on the crafted journal).
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let report = render_statusline_report("text", None).expect("render report");
        assert!(
            report.contains("failed to parse as JSON"),
            "text report must disclose: {report}"
        );
        assert!(report.contains("torn tail"), "shape named: {report}");
        assert!(
            report.contains("last intact capture"),
            "epoch anchor named: {report}"
        );
        let json = render_statusline_report("json", None).expect("render json");
        assert!(json.contains("\"torn_tail\""), "json arm carries the field");
        assert!(
            json.contains("\"unparsed_bytes\""),
            "json field shape: {json}"
        );
        let budget = render_budget_view("json", None).expect("render budget");
        assert!(budget.contains("\"torn_tail\""), "budget view too");

        // An intact journal stays byte-quiet: no disclosure noise.
        fs::write(
            &journal,
            format!("{}\n", serde_json::to_string(&capture).expect("serialize")),
        )
        .expect("rewrite journal intact");
        let quiet = statusline_torn_tail(&journal);
        assert!(quiet.is_quiet());
        let report = render_statusline_report("text", None).expect("render quiet");
        assert!(
            !report.contains("failed to parse as JSON"),
            "intact journal prints no warning: {report}"
        );

        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn oversized_journal_is_capped_with_a_disclosed_marker_not_silent_truncation() {
        // rm-821 read bound: a journal larger than the 10 MiB cap is
        // read only to the cap, and every consumer says so — the
        // marker must appear even when the capped prefix parses 100%
        // clean (that is what separates a disclosed cap from the
        // parse-gap disclosure).
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-cap-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        let intact: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let capture = CapturedStatusline {
            captured_at: 1_700_000_000,
            payload: intact,
        };
        // One valid capture padded past the cap with trailing spaces
        // (serde tolerates them; the line still parses).
        let line = serde_json::to_string(&capture).expect("serialize");
        let padding = (STATUSLINE_READ_MAX_BYTES as usize + 4096).saturating_sub(line.len() + 1);
        let mut raw = format!("{line}{}\n", " ".repeat(padding));
        // A SECOND capture strictly beyond the cap boundary: if the
        // read ignored the cap it would show up in the captures.
        let beyond = CapturedStatusline {
            captured_at: 1_700_000_001,
            payload: serde_json::from_str(FIXTURE).expect("fixture parses"),
        };
        raw.push_str(&serde_json::to_string(&beyond).expect("serialize"));
        raw.push('\n');
        fs::write(&journal, &raw).expect("write oversized journal");

        let captures = read_statusline_captures(&journal);
        assert_eq!(captures.len(), 1, "the beyond-cap capture is not read");
        let torn = statusline_torn_tail(&journal);
        assert_eq!(
            torn.capped_away_bytes,
            (raw.len() as u64).saturating_sub(STATUSLINE_READ_MAX_BYTES),
            "the marker carries exactly the unread byte count"
        );
        assert!(!torn.is_quiet(), "a capped journal is never quiet");
        assert_eq!(torn.unparsed_lines, 0, "the capped prefix parses clean");

        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let report = render_statusline_report("text", None).expect("render report");
        assert!(
            report.contains("read cap"),
            "text report names the cap: {report}"
        );
        let json = render_statusline_report("json", None).expect("render json");
        assert!(
            json.contains("\"capped_away_bytes\""),
            "json arm carries the field: {json}"
        );
        let budget = render_budget_view("json", None).expect("render budget");
        assert!(budget.contains("\"capped_away_bytes\""), "budget view too");
        // rm-821 (review fix F2): the capped prefix parses 100% clean,
        // so the TEXT arm must name the cap and never mint a false
        // parse-gap sentence.
        let budget_text = render_budget_view("text", None).expect("render budget text");
        assert!(
            budget_text.contains("read cap"),
            "budget text names the cap: {budget_text}"
        );
        assert!(
            !budget_text.contains("failed to parse"),
            "capped-and-clean must not claim a parse gap: {budget_text}"
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cap_boundary_utf8_straddle_cannot_empty_the_journal() {
        // rm-821 (review fix F3): the cap boundary can land INSIDE a
        // multibyte UTF-8 character. read_to_string answered Err and
        // this arm returned (empty, None) — a VALID journal silently
        // read as missing: the report said "no captures yet" while
        // stats.bytes still showed the true >10 MiB size. The byte
        // read with lossy decode keeps every parseable line and
        // discloses the straddled tail as an unparsed line instead.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-straddle-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        let intact: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let capture = CapturedStatusline {
            captured_at: 1_700_000_000,
            payload: intact,
        };
        let line = serde_json::to_string(&capture).expect("serialize");
        // Pad with spaces so a 4-byte emoji STARTS exactly two bytes
        // before the cap: bytes [cap-2, cap) are the first half of the
        // character and the read cut lands mid-character.
        let emoji = "\u{1F600}"; // 4 bytes
        let padding = (STATUSLINE_READ_MAX_BYTES as usize - 2).saturating_sub(line.len() + 1);
        let raw = format!("{line}{}\n{emoji}", " ".repeat(padding));
        fs::write(&journal, &raw).expect("write straddled journal");

        let (text, capped) = read_journal_capped(&journal);
        assert!(
            !text.is_empty(),
            "a straddled journal no longer silently reads as missing"
        );
        assert!(capped.is_some(), "the cap marker still fires");
        let captures = read_statusline_captures(&journal);
        assert_eq!(captures.len(), 1, "the intact prefix line still reads");
        let torn = statusline_torn_tail(&journal);
        assert_eq!(
            torn.capped_away_bytes,
            (raw.len() as u64) - STATUSLINE_READ_MAX_BYTES,
            "exactly the unread byte count is carried"
        );
        assert_eq!(torn.unparsed_lines, 1, "the straddled line is disclosed");
        assert!(torn.torn_final_line, "the capped read cuts mid-line");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn budget_text_names_a_torn_tail_without_a_false_parse_gap() {
        // rm-821 (review fix F2): a journal whose final line is a
        // COMPLETE JSON object without a trailing newline (a crashed
        // append that still parses) used to print "0 byte(s) across 0
        // line(s) failed to parse" — a false claim. The text arm now
        // branches on the two components like the report arm: the
        // torn-tail shape is named, and no cap sentence appears for a
        // journal that never hit the cap.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-tornparses-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        let intact: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let first = serde_json::to_string(&CapturedStatusline {
            captured_at: 1_700_000_000,
            payload: intact.clone(),
        })
        .expect("serialize");
        let second = serde_json::to_string(&CapturedStatusline {
            captured_at: 1_700_000_100,
            payload: intact,
        })
        .expect("serialize");
        // No trailing newline on the final line: torn-but-parses.
        fs::write(&journal, format!("{first}\n{second}")).expect("write torn journal");

        let torn = statusline_torn_tail(&journal);
        assert!(torn.torn_final_line, "fixture is torn as designed");
        assert_eq!(torn.unparsed_lines, 0, "both lines parse");

        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let text = render_budget_view("text", None).expect("render budget text");
        assert!(
            text.contains("ends without a trailing newline (crashed append shape)"),
            "the shape is named: {text}"
        );
        assert!(
            !text.contains("failed to parse"),
            "nothing failed to parse — no false claim: {text}"
        );
        assert!(
            !text.contains("malformed line(s)"),
            "no false malformed claim: {text}"
        );
        assert!(
            !text.contains("read cap"),
            "no cap claim without a cap: {text}"
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_appends_render_without_panic_and_mark_torn_tails() {
        // rm-821 e2e: a writer appends captures while the reader
        // renders. Every render must succeed; whenever the snapshot
        // the reader saw ended without a newline (the torn shape), the
        // report must SAY so — a torn tail may never surface as a
        // silent zero/partial-usage line.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-race-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        let intact: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let capture = serde_json::to_string(&CapturedStatusline {
            captured_at: 1_700_000_000,
            payload: intact,
        })
        .expect("serialize");

        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Phase 1 — deterministic: the writer holds a body without its
        // newline; the render between the two writes must (a) not
        // panic and (b) mark the torn tail. The reader-side file
        // check is only for OUR bookkeeping; the marker assertion is
        // one-sided by construction here because nothing else writes.
        {
            use std::io::Write;
            let mut held = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&journal)
                .expect("open journal");
            write!(held, "{capture}   ").expect("write torn body");
            let report = render_statusline_report("text", None).expect("render torn");
            assert!(
                report.contains("torn tail"),
                "a snapshot ending mid-line is marked, never silent: {report}"
            );
            writeln!(held).expect("complete the line");
            let report = render_statusline_report("text", None).expect("render intact");
            assert!(
                !report.contains("failed to parse as JSON"),
                "completing the line leaves no residue: {report}"
            );
        }

        // Phase 2 — stress: 2_000 interleaved body/newline writes race
        // the render loop. Every render must succeed without panic.
        // (Torn-MARKING cannot be asserted here: the render reads the
        // journal at its own moment, which is neither my before-read
        // nor after-read — a torn snapshot before the render can be
        // complete by the render's read and vice versa. Phase 1 pins
        // the marker deterministically against a held-open torn
        // state; this phase pins panic-freedom under real
        // interleaving, and the final settled render pins no-residue.)
        let writer = std::thread::spawn({
            let journal = journal.clone();
            let line = capture.clone();
            move || {
                use std::io::Write;
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&journal)
                    .expect("open journal");
                for i in 0..2_000u32 {
                    // The body keeps trailing-space padding only
                    // (serde tolerates it), so a COMPLETED line always
                    // parses — the torn shape is the only defect the
                    // reader is allowed to see.
                    let padding = " ".repeat((i % 7) as usize + 1);
                    write!(file, "{line}{padding}").expect("write body");
                    std::thread::yield_now();
                    writeln!(file).expect("write newline");
                    std::thread::yield_now();
                }
            }
        });
        let mut renders = 0usize;
        while !writer.is_finished() {
            let report = render_statusline_report("text", None).expect("render never panics");
            let _ = report;
            renders += 1;
        }
        writer.join().expect("writer never panics");
        assert!(renders > 0, "the reader actually raced the writer");
        // The final state is intact: full lines, quiet tail, and all
        // 2_000 captures are counted.
        let final_report = render_statusline_report("text", None).expect("final render");
        assert!(
            !final_report.contains("failed to parse as JSON"),
            "settled journal has no residue: {final_report}"
        );
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

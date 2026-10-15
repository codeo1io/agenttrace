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
use std::io::{self, Read, Seek, SeekFrom, Write};
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

/// Token counts from the session's last API call
/// (`context_window.current_usage`, docs 2026-10-07). The object is
/// `null` before the first API call and again immediately after
/// `/compact` until the next call repopulates it; individual fields
/// stay optional so future growth is tolerated, not rejected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StatuslineContextUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u64>,
}

/// The `context_window` object (docs 2026-10-07, v2.1.251+):
/// pre-calculated utilization plus the last-API token counts — the
/// authoritative context pressure a transcript lane can only estimate.
/// `used_percentage` is input-only
/// (`input + cache_creation + cache_read`), `current_usage` is `null`
/// before the first API call and right after `/compact`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StatuslineContextWindow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_percentage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_usage: Option<StatuslineContextUsage>,
}

/// The gateway `rate_limits.spend_limit` window (docs 2026-10-07,
/// v2.1.251+; behind a Claude apps gateway with spend limits).
/// `used_percentage`/`resets_at` come with every response once the
/// object exists; `used_usd`/`limit_usd`/`period` arrive on a separate
/// ~5-minute schedule (v2.1.284+) and stay absent under
/// `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` — every field optional
/// even when the object is present. `used_usd` is a gateway estimate,
/// not a billed amount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StatuslineSpendLimit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
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

#[cfg(test)]
thread_local! {
    /// rm-921 residual test hook: counts `context_window()` invocations,
    /// so a test can pin the once-per-capture parse discipline (the peak
    /// fold and the per-session latest-window list must share one parse
    /// per capture instead of each cloning + parsing the payload).
    static CONTEXT_WINDOW_PARSE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl CapturedStatusline {
    pub fn session_id(&self) -> Option<&str> {
        self.payload.get("session_id").and_then(Value::as_str)
    }

    fn rate_limit(&self, window: &str) -> Option<StatuslineRateLimitState> {
        serde_json::from_value(self.payload.get("rate_limits")?.get(window)?.clone()).ok()
    }

    fn spend_limit(&self) -> Option<StatuslineSpendLimit> {
        serde_json::from_value(self.payload.get("rate_limits")?.get("spend_limit")?.clone()).ok()
    }

    fn context_window(&self) -> Option<StatuslineContextWindow> {
        let raw = self.payload.get("context_window")?;
        #[cfg(test)]
        CONTEXT_WINDOW_PARSE_COUNT.with(|count| count.set(count.get() + 1));
        serde_json::from_value(raw.clone()).ok()
    }

    fn prompt_cache(&self) -> Option<StatuslinePromptCache> {
        serde_json::from_value(self.payload.get("prompt_cache")?.clone()).ok()
    }

    /// rm-708: the prompt correlation key — a UUID that matches the
    /// `prompt.id` attribute on OpenTelemetry events (docs 2026-10-07).
    /// Absent until the session's first user input, so absence is a
    /// normal pre-first-input state, not an error. A non-string value
    /// is not a usable key and reports as absent.
    pub fn prompt_id(&self) -> Option<&str> {
        self.payload.get("prompt_id").and_then(Value::as_str)
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
///
/// Cycle-3 review F1 (2026-10-13, run 64e89da50d71, attempt ae97110b):
/// the RELATIVE arm now matches the rm-248 rider — `HOME=rel-home`
/// planted ./rel-home/.cache/agenttrace/statusline.jsonl (0600) in the
/// CWD and `--statusline-report` read the planted journal back (live
/// PoC: ae97110b-scratch/poc/). A set-but-empty OR RELATIVE `HOME` is
/// unset-equivalent on BOTH arms, the same rule the rider pinned at
/// the session_cache / doctor / pricing copies.
fn resolve_user_cache_dir(
    home: Option<&std::ffi::OsStr>,
    xdg_cache_home: Option<&std::ffi::OsStr>,
) -> PathBuf {
    let usable_home = home
        .filter(|value| !value.is_empty())
        .filter(|value| std::path::Path::new(value).is_absolute());
    if cfg!(target_os = "macos") {
        if let Some(home) = usable_home {
            return PathBuf::from(home).join("Library").join("Caches");
        }
    }
    // rm-248: the XDG Base Directory spec only defines ABSOLUTE
    // $XDG_CACHE_HOME values; a relative one is treated as unset and
    // falls through to the HOME-based default instead of scattering a
    // per-cwd cache (observed live: `XDG_CACHE_HOME=rel-cache
    // agenttrace --overview` wrote ./rel-cache/agenttrace from a
    // scratch cwd).
    if let Some(cache) = xdg_cache_home
        .filter(|value| !value.is_empty())
        .filter(|value| std::path::Path::new(value).is_absolute())
    {
        return PathBuf::from(cache);
    }
    if let Some(home) = usable_home {
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
/// rm-936: Unicode bidi/format controls (general category Cf) that
/// REORDER adjacent rendered text — the DIRECTIONAL subset of the
/// category. `char::is_control()` matches only category Cc, so these
/// rode raw through every export lane before this predicate existed:
/// the embedding/override initiators and pops (U+202A-202E), the
/// left/right marks (U+200E-200F), the isolate initiators and pops
/// (U+2066-2069), and the Arabic letter mark (U+061C). The
/// NON-directional Cf members (U+FEFF, U+2060, U+00AD, U+180E) do not
/// reorder anything and are deliberately OUT of this predicate's set —
/// the display/hiding tricks they enable are a separate, still-open
/// lane on this row.
pub fn is_bidi_format_control(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

pub fn sanitize_line_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|c| {
            if c.is_control() || is_bidi_format_control(c) {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .collect()
}

/// rm-625: document-level variant of [`sanitize_line_segment`] for the
/// output-dispatch boundary (stdout and every `-o` write). Where the
/// line-segment helper must kill EVERY control byte (a status line is
/// one physical line), whole documents legitimately carry layout
/// bytes: LF, CR and TAB are layout and stay; every other control
/// byte — ESC (the OSC/CSI introducer), BEL, DEL, and the C1 range —
/// byte is neutralized to U+FFFD so terminal emulators never interpret
/// transcript-derived sequences. rm-936 adds the Cf bidi/format set to the
/// same substitution (see [`is_bidi_format_control`]) — serde and every
/// structural escaper pass those through raw, so this boundary is where
/// they die in every non-JSON lane. Idempotent by construction: U+FFFD is
/// printable, so already-sanitized documents (including cells that
/// first went through the rm-383/rm-540 line sanitizer) pass through
/// unchanged — the two layers compose instead of corrupting.
pub fn sanitize_output_document(text: &str) -> String {
    text.chars()
        .map(|c| {
            if (c.is_control() || is_bidi_format_control(c)) && c != '\n' && c != '\r' && c != '\t'
            {
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
/// fully loaded before any bounded handling could kick in. rm-922 (merged
/// at integration): the read-side bound IS the retention bound — both
/// constants name the same 10 MiB — and it now guards the DECODE as well
/// (see [`read_statusline_capture_bounded`]), so the two names stay
/// lockstep-aliased instead of drifting apart again.
const STATUSLINE_READ_CAP_BYTES: u64 = STATUSLINE_CAPTURE_MAX_BYTES;

/// rm-898 + rm-922 (composed at integration): one bounded, UTF-8-recovering
/// journal read. rm-898: read at most [`STATUSLINE_READ_CAP_BYTES`] bytes
/// from the TAIL of the journal (the fleet-proven rm-821 `read_journal_capped`
/// shape), returning the tail contents and the number of bytes cut from the
/// head. A cut that lands mid-line or mid-character is resolved by advancing
/// to the next char boundary and dropping the (torn) first line — mirroring
/// the torn-tail tolerance the journal lanes already apply to truncated
/// writers. rm-922: the same read now bounds the DECODE too. The 10 MiB
/// bound used to be enforced on the APPEND side only, so a journal planted
/// or grown by any other means was read and re-serialized whole by every
/// consumer; and one invalid UTF-8 byte made the read fail outright — every
/// insight silently zeroed, and because compaction's own read failed the
/// same way, retention never repaired and the journal grew past the ceiling
/// with the error surfaced nowhere. The read decodes lossily, so every
/// VALID line still discloses and the next compaction rewrites the journal
/// as clean UTF-8.
#[derive(Debug, Default, PartialEq)]
struct JournalRead {
    text: String,
    /// The on-disk journal exceeded [`STATUSLINE_READ_CAP_BYTES`] and only
    /// its newest tail was kept (disclosed in the stats lane; benign
    /// journals stay `false` and serialize nothing).
    truncated: bool,
    /// Bytes cut from the head of the journal when it exceeded the read
    /// cap (`0` = the read was complete) — the exact count the stats lane
    /// discloses as `capped_away_bytes`.
    capped_away_bytes: u64,
}

fn read_statusline_capture_bounded(path: &Path) -> JournalRead {
    let Ok(mut file) = fs::File::open(path) else {
        return JournalRead::default();
    };
    let Ok(size) = file.metadata().map(|meta| meta.len()) else {
        return JournalRead::default();
    };
    if size <= STATUSLINE_READ_CAP_BYTES {
        // Common case: inside the bound. `take` bounds even this arm
        // against a journal growing between the metadata probe and the
        // read; the lossy decode keeps every valid line parseable while
        // the damaged line's U+FFFD replacements make it fail JSON
        // parsing — it is skipped like any torn line instead of zeroing
        // the whole report (rm-922: the prior `read_to_string` failed
        // outright on the first invalid byte).
        let mut bytes = Vec::new();
        let mut limited = file.take(STATUSLINE_READ_CAP_BYTES);
        let _ = limited.read_to_end(&mut bytes);
        return JournalRead {
            text: String::from_utf8_lossy(&bytes).into_owned(),
            truncated: false,
            capped_away_bytes: 0,
        };
    }
    // Oversized (planted, or grown by a non-statusline writer): keep the
    // NEWEST bound bytes — journal semantics keep the newest lines. A cut
    // landing inside a multi-byte character advances to the next boundary
    // (the buffer then begins with a continuation byte and from_utf8 fails
    // at offset 0; a valid start byte can be preceded by at most three
    // continuation bytes, so the advance is bounded — review-fix 9fb0a017
    // F2: the prior guard compared valid_up_to() == buf.len(), which
    // from_utf8 can never produce on an Err; that branch was dead and the
    // raw cut leaked through the lossy fallback). Any other invalid UTF-8
    // (interior corruption, a torn writer's mid-character EOF) falls back
    // to a lossy read so the lane stays usable (same tolerance as torn
    // writers, rm-922).
    let mut cut = size - STATUSLINE_READ_CAP_BYTES;
    let mut buf = Vec::with_capacity(STATUSLINE_READ_CAP_BYTES as usize);
    let mut advances_left = 3u32; // a valid start byte has at most 3 continuation bytes before it
    let raw = loop {
        if file.seek(SeekFrom::Start(cut)).is_err() {
            return JournalRead::default();
        }
        buf.clear();
        if (&mut file)
            .take(STATUSLINE_READ_CAP_BYTES)
            .read_to_end(&mut buf)
            .is_err()
        {
            return JournalRead::default();
        }
        match std::str::from_utf8(&buf) {
            Ok(s) => break s.to_string(),
            Err(e) => {
                if e.valid_up_to() == 0
                    && advances_left > 0
                    && cut < size
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
    // Drop a torn head line: after a head cut the first line is complete
    // only when the cut landed exactly at a line boundary (a tail entirely
    // inside one line keeps nothing whole).
    let text = match raw.find('\n') {
        Some(i) => raw[i + 1..].to_string(),
        None => String::new(),
    };
    JournalRead {
        text,
        truncated: true,
        capped_away_bytes: cut,
    }
}

fn compact_statusline_capture_locked(path: &Path, keep_under: u64) -> io::Result<()> {
    // rm-922: the compaction read rides the SAME bounded, lossy
    // decode as every consumer (this used to be a bare
    // `read_to_string`, so one invalid UTF-8 byte made retention
    // fail forever — the journal grew unboundedly past the 10 MiB
    // ceiling with the error surfaced nowhere). A rewrite now
    // repairs such a journal: invalid bytes are replaced with
    // U+FFFD in the rewritten — already unparseable — lines, and the
    // next read sees clean UTF-8. rm-898 (composed at integration):
    // the shared read is also TAIL-bounded, so a journal that grew
    // past the read cap without passing through the append-side
    // retention check re-fits on its next compaction.
    let raw = read_statusline_capture_buffer(path);
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
    let raw = read_statusline_capture_buffer(path);
    parse_statusline_captures(&raw)
}

/// The raw journal text actually read: empty when the file is absent or
/// unreadable, the same tolerance [`read_statusline_captures`] always
/// had. rm-684 (review 95d74221 F6): the budget view and the report read
/// the journal exactly ONCE per invocation — the stats disclosure, the
/// JSON arm, the text arm and the captures all share this buffer instead
/// of each re-reading the (up to 10 MiB) file. rm-898: the shared read is
/// itself bounded — at most [`STATUSLINE_READ_CAP_BYTES`] taken from the
/// TAIL — with the head cut riding along for stats disclosure. rm-922: it
/// is also UTF-8-recovering — see [`read_statusline_capture_bounded`],
/// which this wraps for callers that need only the text.
fn read_statusline_capture_buffer(path: &Path) -> String {
    read_statusline_capture_bounded(path).text
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

/// Latest `context_window` state for one session, newest session first
/// (rm-037): utilization is a per-session quantity, so the arm joins
/// `session_caches` in reporting the newest capture per session.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StatuslineSessionContextWindow {
    pub session_id: String,
    pub used_percentage: Option<f64>,
    pub remaining_percentage: Option<f64>,
    pub current_usage: Option<StatuslineContextUsage>,
}

/// Latest observed `prompt_id` for one session, newest session first
/// (rm-708). The value is the correlation key matching OTel `prompt.id`
/// attributes; only captures that actually carry one appear here.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StatuslineSessionPrompt {
    pub session_id: String,
    pub prompt_id: String,
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
    /// Latest `context_window` per session, newest session first
    /// (rm-037; sessions without a `context_window` arm stay absent).
    pub context_windows: Vec<StatuslineSessionContextWindow>,
    /// Peak `context_window.used_percentage` across all captures
    /// (rm-037). None when no capture carried a non-null percentage.
    pub context_peak_used_percentage: Option<f64>,
    /// Latest gateway spend-limit window (rm-037): None when no
    /// capture has run behind a Claude apps gateway with limits.
    pub spend_limit: Option<StatuslineSpendLimit>,
    /// Peak spend-limit usage across all captures (rm-037).
    pub spend_limit_peak_used_percentage: Option<f64>,
    /// Latest observed `prompt_id` per session, newest session first
    /// (rm-708; pre-first-input captures carry none and stay absent).
    pub prompt_ids: Vec<StatuslineSessionPrompt>,
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

    // rm-921: each capture's window state is parsed EXACTLY ONCE.
    // The aggregation below used to call `rate_limit` (a serde Value
    // clone + `from_value`) per capture per PROBE, so a journal whose
    // captures carry `resets_at` but no `used_percentage` (older
    // hosts, null fields, hostile input) paid one parse per capture
    // per distinct boundary — n·k parses, measured quadratic (0.05 s
    // → 8.3 s at n=250→2000 in release, minutes-scale at the 10 MiB
    // retention ceiling) on otherwise-correct output. These parsed
    // states per capture feed every consumer below.
    //
    // rm-037: the gateway spend window joins as a THIRD parsed state
    // on the same once-per-capture discipline (not a fresh parse per
    // consumer): the full `StatuslineSpendLimit` stays on `spends`
    // for the latest-state pick (USD trio + period ride only there),
    // while the crossings sweep reads its (used_percentage,
    // resets_at) pair through the same `windows` vec as the two
    // subscription windows — one uniform shape, one parse each.
    let spends: Vec<Option<StatuslineSpendLimit>> = deduped
        .iter()
        .map(|capture| capture.spend_limit())
        .collect();
    let windows: Vec<[Option<StatuslineRateLimitState>; 3]> = deduped
        .iter()
        .zip(spends.iter())
        .map(|(capture, spend)| {
            [
                capture.rate_limit("five_hour"),
                capture.rate_limit("seven_day"),
                spend.clone().map(|spend| StatuslineRateLimitState {
                    used_percentage: spend.used_percentage,
                    resets_at: spend.resets_at,
                }),
            ]
        })
        .collect();

    // rm-955: the latest-state pick shares the rm-921 crossings
    // discipline — one composite rank, captured_at first with journal
    // position only as the tie-break — instead of array position, so
    // an out-of-order journal (clock skew, an older journal merged
    // in) cannot surface a stale-by-clock observation as the current
    // window state. On an in-order journal the last position carries
    // the greatest captured_at, so the pick is identical to the old
    // array-order pick.
    let mut latest: [Option<(i64, usize)>; 3] = [None, None, None];
    for (position, states) in windows.iter().enumerate() {
        for window in 0..3 {
            if states[window].is_some() {
                let rank = (deduped[position].captured_at, position);
                if latest[window].is_none_or(|current| rank > current) {
                    latest[window] = Some(rank);
                }
            }
        }
    }
    let five_hour = latest[0].and_then(|(_, position)| windows[position][0].clone());
    let seven_day = latest[1].and_then(|(_, position)| windows[position][1].clone());
    // rm-037: the spend-limit window reports on the same schedule as
    // the subscription windows, so the latest observed capture wins —
    // picked by the same rm-955 rank (captured_at, journal position)
    // rather than array order, and cloned from the pre-parsed `spends`
    // vec so the USD trio and period ride along (the `windows`
    // projection keeps only the rate-limit pair).
    let spend_limit = latest[2].and_then(|(_, position)| spends[position].clone());

    let peak = |window: usize| {
        windows
            .iter()
            .filter_map(|states| states[window].as_ref())
            .filter_map(|state| state.used_percentage)
            .fold(None::<f64>, |acc, used| {
                Some(match acc {
                    Some(max) if max >= used => max,
                    _ => used,
                })
            })
    };

    let mut crossings: Vec<StatuslineLimitCrossing> = Vec::new();
    // rm-037: the gateway spend window resets on the same mechanism
    // as the two subscription windows (epoch `resets_at` + falling
    // usage), so it joins the rm-921 sweep as a third window through
    // its pre-parsed (used_percentage, resets_at) projection. Windows
    // never observed simply contribute nothing.
    for (window_index, window) in ["five_hour", "seven_day", "spend_limit"].iter().enumerate() {
        // rm-921 (F2+F5): one sorted sweep replaces the per-boundary
        // probe. The observations that can evidence a crossing
        // (captures with a `used_percentage` in this window) are
        // sorted once by (captured_at, journal position) and a
        // boundary's before/after is a binary search into that order
        // — and the pick is by captured_at (greatest strictly-before /
        // least at-or-after) instead of array position, so an
        // out-of-order journal (clock skew, an older journal merged
        // in) no longer reports a stale observation at the boundary.
        // On an in-order journal the picks are identical to the old
        // array-order picks: resolving captured_at ties by journal
        // position reproduces exactly what next_back/next returned
        // over the sorted array.
        let mut used: Vec<(i64, usize, f64)> = windows
            .iter()
            .enumerate()
            .filter_map(|(position, states)| {
                let used_percentage = states[window_index].as_ref()?.used_percentage?;
                Some((deduped[position].captured_at, position, used_percentage))
            })
            .collect();
        used.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut boundaries: Vec<i64> = windows
            .iter()
            .filter_map(|states| states[window_index].as_ref()?.resets_at)
            .collect();
        boundaries.sort_unstable();
        boundaries.dedup();
        for resets_at in boundaries {
            // Observations strictly before the boundary form a prefix
            // of the sorted sweep; `partition_point` finds the split.
            let split = used.partition_point(|(captured_at, _, _)| *captured_at < resets_at);
            let before = (split > 0).then(|| used[split - 1].2);
            let after = (split < used.len()).then(|| used[split].2);
            if let (Some(used_percentage_before), Some(used_percentage_after)) = (before, after) {
                // A crossing is evidenced by observations on both sides:
                // usage observed while the window was running and the
                // first observation after the boundary. A bare
                // resets_at timestamp (no before-side) or a window whose
                // reset passed outside the journal proves nothing.
                crossings.push(StatuslineLimitCrossing {
                    window: window.to_string(),
                    resets_at,
                    used_percentage_before: Some(used_percentage_before),
                    used_percentage_after: Some(used_percentage_after),
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

    // rm-037: peak context utilization across every capture that
    // carried a non-null percentage (the docs allow null before the
    // first API call and right after /compact).
    // rm-921 residual: one context_window() parse per capture feeds BOTH
    // the peak fold and the per-session latest-window list (each call
    // clones the payload Value and serde-parses the subtree; the
    // once-per-capture discipline is pinned by
    // context_window_parsed_once_per_capture).
    let context_windows_parsed: Vec<Option<StatuslineContextWindow>> = deduped
        .iter()
        .map(|capture| capture.context_window())
        .collect();
    let context_peak_used_percentage = context_windows_parsed
        .iter()
        .filter_map(|window| window.as_ref())
        .filter_map(|window| window.used_percentage)
        .fold(None::<f64>, |acc, used| {
            Some(match acc {
                Some(max) if max >= used => max,
                _ => used,
            })
        });
    let mut context_windows: Vec<StatuslineSessionContextWindow> = Vec::new();
    let mut context_seen: HashSet<String> = HashSet::new();
    for (capture, window) in deduped.iter().zip(context_windows_parsed.iter()).rev() {
        let Some(window) = window.as_ref() else {
            continue;
        };
        let session_id = capture.session_id().unwrap_or("(unknown)").to_string();
        if !context_seen.insert(session_id.clone()) {
            continue;
        }
        context_windows.push(StatuslineSessionContextWindow {
            session_id,
            used_percentage: window.used_percentage,
            remaining_percentage: window.remaining_percentage,
            current_usage: window.current_usage.clone(),
        });
    }

    // rm-708: latest prompt correlation key per session, newest
    // session first, mirroring `session_caches`.
    let mut prompt_ids: Vec<StatuslineSessionPrompt> = Vec::new();
    let mut prompt_seen: HashSet<String> = HashSet::new();
    for capture in deduped.iter().rev() {
        let Some(prompt_id) = capture.prompt_id() else {
            continue;
        };
        let session_id = capture.session_id().unwrap_or("(unknown)").to_string();
        if !prompt_seen.insert(session_id.clone()) {
            continue;
        }
        prompt_ids.push(StatuslineSessionPrompt {
            session_id,
            prompt_id: prompt_id.to_string(),
        });
    }

    StatuslineInsights {
        captures: deduped.len(),
        sessions: sessions.len(),
        five_hour,
        seven_day,
        five_hour_peak_used_percentage: peak(0),
        seven_day_peak_used_percentage: peak(1),
        limit_crossings: crossings,
        session_caches,
        miss_causes,
        context_windows,
        context_peak_used_percentage,
        spend_limit,
        spend_limit_peak_used_percentage: peak(2),
        prompt_ids,
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

/// rm-708: prompt ids shared between the statusline journal and an
/// OTLP-JSON export carrying `prompt.id` span attributes — the
/// capture-side half of the statusline↔OTel correlation the docs
/// define as "Matches the `prompt.id` attribute on OpenTelemetry
/// events, when they exist". The emitter side of that attribute lands
/// with the OTel arm; this join is pure so both lanes can exercise it
/// the moment a doc is available. Unknown shapes (missing
/// `resourceSpans`, spans without attributes, non-string values)
/// correlate nothing — never an error, mirroring the journal's
/// malformed-line tolerance.
pub fn correlated_prompt_ids(captures: &[CapturedStatusline], otel_doc: &Value) -> Vec<String> {
    let captured: HashSet<String> = captures
        .iter()
        .filter_map(|capture| capture.prompt_id())
        .map(str::to_string)
        .collect();
    if captured.is_empty() {
        return Vec::new();
    }
    let mut otel: HashSet<String> = HashSet::new();
    let resource_spans = otel_doc
        .get("resourceSpans")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for scope_spans in
        resource_spans.filter_map(|rs| rs.get("scopeSpans").and_then(Value::as_array))
    {
        let spans = scope_spans
            .iter()
            .filter_map(|ss| ss.get("spans").and_then(Value::as_array))
            .flatten();
        for attribute in spans
            .filter_map(|span| span.get("attributes").and_then(Value::as_array))
            .flatten()
        {
            let key = attribute.get("key").and_then(Value::as_str);
            let value = attribute
                .get("value")
                .and_then(|value| value.get("stringValue"))
                .and_then(Value::as_str);
            if key == Some("prompt.id") {
                if let Some(id) = value {
                    otel.insert(id.to_string());
                }
            }
        }
    }
    let mut both: Vec<String> = captured.intersection(&otel).cloned().collect();
    both.sort_unstable();
    both
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
    /// [`read_statusline_capture_bounded`] when it exceeds
    /// [`STATUSLINE_READ_CAP_BYTES`] (`0` = the read was complete). The
    /// statusline-report JSON surfaces this so a bounded read is disclosed
    /// rather than silently undercounting a huge journal.
    pub capped_away_bytes: u64,
    /// rm-922: the on-disk journal exceeded the read bound and the read
    /// kept only its newest tail (the bound used to be enforced on the
    /// append side only). Serialized only when set, so benign journals keep
    /// their report bytes identical. On the merged reader this flag means
    /// exactly `capped_away_bytes > 0`; both are kept so each side's
    /// pinned disclosure shape survives the integration.
    #[serde(skip_serializing_if = "is_false")]
    pub read_truncated: bool,
    /// rm-708 diagnostics: lines classified by prompt-correlation key
    /// presence. `prompt_id` is absent until the session's first user
    /// input (docs 2026-10-07), so an object payload without it is a
    /// pre-first-input capture — deliberately distinct from an
    /// unclassifiable payload (not a JSON object, or a non-string
    /// `prompt_id`). Torn tail lines that fail JSON parsing count in
    /// `lines` only, keeping them out of both buckets.
    pub captures_with_prompt_id: usize,
    pub captures_pre_first_input: usize,
    pub captures_unknown_payload: usize,
    /// Non-empty journal lines that failed to parse — counted so a
    /// lines-vs-samples discrepancy has a visible cause
    /// ("Statusline budget truth" batch, run 80336dfb).
    pub unparsable_lines: usize,
}

/// `serde(skip_serializing_if)` helper for the rm-922 truncation
/// flag: absent from serialized output while `false`.
fn is_false(flag: &bool) -> bool {
    !*flag
}

pub fn statusline_journal_stats(path: &Path) -> StatuslineJournalStats {
    let read = read_statusline_capture_bounded(path);
    statusline_journal_stats_from_buffer(path, &read.text, read.capped_away_bytes)
}

/// Stats derived from an already-read journal buffer (rm-684: the
/// budget view computes these from its single read instead of a
/// second full read). `lines` describes the text that was actually
/// read; `capped_away_bytes` carries the head cut from that same
/// shared bounded read (rm-898) and derives the rm-922 `read_truncated`
/// flag; `exists` still probes the path so an absent journal discloses
/// itself, and `bytes` always names the on-disk metadata size — review
/// 95d74221 F7: an undecodable journal decodes to an empty buffer and
/// reporting `bytes: 0` exactly when the journal is corrupt would hide
/// its real footprint (and rm-898 extends the same rule to the over-cap
/// case: a bounded read must never shrink the reported footprint of
/// the file it truncated; rm-922's lossy decode makes empty-buffer
/// reads rarer still, but the metadata stays the single truth source
/// — the integration keeps this landed rule over the candidate's
/// read-length accounting).
fn statusline_journal_stats_from_buffer(
    path: &Path,
    raw: &str,
    capped_away_bytes: u64,
) -> StatuslineJournalStats {
    let exists = path.exists();
    let bytes = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    // rm-898: bounded tail read — line counts for an over-cap journal
    // cover only the retained tail; the shared buffer's `away` discloses
    // the head that was cut (no second read, rm-684/F6). rm-922: the
    // truncation flag is exactly `away > 0` on the merged reader.
    // rm-708: the prompt-key classification below walks the SAME shared
    // buffer — still exactly one journal read (the candidate's second
    // full re-read is dropped; the merged reader keeps the bounded,
    // truncation-disclosing read).
    let lines = if exists {
        raw.lines().filter(|line| !line.trim().is_empty()).count()
    } else {
        0
    };
    // rm-708: classify parseable lines by `prompt_id` presence inside
    // the captured payload. A torn tail line fails parsing and counts
    // only in `lines` (the append-only journal already tolerates it),
    // so it lands in neither bucket.
    let mut captures_with_prompt_id = 0;
    let mut captures_pre_first_input = 0;
    let mut captures_unknown_payload = 0;
    let mut unparsable_lines = 0;
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(capture) = serde_json::from_str::<CapturedStatusline>(line) else {
            // "Statusline budget truth" (run 80336dfb): a torn or
            // corrupt line still counts in `lines`; it lands here
            // too so a lines-vs-samples discrepancy is disclosed.
            unparsable_lines += 1;
            continue;
        };
        if !capture.payload.is_object() {
            captures_unknown_payload += 1;
        } else if capture.prompt_id().is_some() {
            captures_with_prompt_id += 1;
        } else if capture.payload.get("prompt_id").is_some() {
            // Present but not a string: not a usable correlation key.
            captures_unknown_payload += 1;
        } else {
            // The documented pre-first-input state.
            captures_pre_first_input += 1;
        }
    }
    StatuslineJournalStats {
        path: path.to_string_lossy().to_string(),
        exists,
        lines,
        bytes,
        retained_max_bytes: STATUSLINE_CAPTURE_MAX_BYTES,
        capped_away_bytes,
        read_truncated: capped_away_bytes > 0,
        captures_with_prompt_id,
        captures_pre_first_input,
        captures_unknown_payload,
        unparsable_lines,
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
    // and the head cut rides along into the stats; rm-922: it is also
    // the lossy, over-cap-disclosing decode.
    let read = read_statusline_capture_bounded(&path);
    let stats = statusline_journal_stats_from_buffer(&path, &read.text, read.capped_away_bytes);
    let captures = parse_statusline_captures(&read.text);
    let insights = statusline_insights(&captures);
    let series = statusline_budget_series(&captures, 7, Utc::now());
    if format == "json" {
        let mut value = serde_json::json!({ "journal": stats, "insights": insights });
        if let Some(budget) = weekly_budget {
            // "Statusline budget truth" (run 80336dfb): the fold
            // above already discards hostile samples, so a negative
            // `remaining` can only mean genuine over-spend — floor it
            // into [0, budget] so the machine field never mirrors a
            // junk-driven total. The upper bound rides `.max(0.0)` so
            // a hostile `--weekly-budget -5` cannot panic the clamp.
            let remaining_usd = (budget - series.total).max(0.0).min(budget.max(0.0));
            value["budget"] = serde_json::json!({
                "weekly_budget_usd": budget,
                "spend_7d_usd": series.total,
                "remaining_usd": remaining_usd,
                // "Statusline budget truth" (run 80336dfb): the same
                // hostile-sample disclosure the --budget json arm
                // carries — discards are counted, never silent.
                "negative_capture_cost_samples":
                    series.negative_capture_cost_samples,
                "invalid_capture_cost_samples":
                    series.invalid_capture_cost_samples,
                // rm-771: the baseline-unknown bias, so machine
                // consumers see why the 7d total can under-count.
                "first_day_baseline_unknown_sessions":
                    series.first_day_baseline_unknown_sessions,
                "first_day_baseline_unknown_usd":
                    series.first_day_baseline_unknown_usd,
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
    // `capped_away_bytes` and `read_truncated` fields on the serialized
    // journal stats (rm-922's text phrasing is subsumed by this richer
    // line — the docs pin this wording).
    if stats.capped_away_bytes > 0 {
        out.push_str(&format!(
            "  journal exceeds the {} MiB read bound: head truncated, {} of {} bytes unread; counts cover the retained tail only\n",
            STATUSLINE_READ_CAP_BYTES / (1024 * 1024),
            stats.capped_away_bytes,
            stats.bytes
        ));
    }
    // "Statusline budget truth" (run 80336dfb): a non-empty line
    // that failed to parse still counts toward `lines` above —
    // disclose it so a lines-vs-captures gap has a visible cause
    // instead of reading as silently lost data. The JSON arm carries
    // the same fact as the `unparsable_lines` field on the serialized
    // journal stats.
    if stats.unparsable_lines > 0 {
        out.push_str(&format!(
            "  {} unparsable journal line{} — counted in the totals above, excluded from every capture stat\n",
            stats.unparsable_lines,
            if stats.unparsable_lines == 1 { "" } else { "s" },
        ));
    }
    // rm-708: prompt-key diagnostics. Absence until the first user
    // input is a documented state, so it is disclosed distinctly from
    // unclassifiable payloads instead of being lumped into an error.
    out.push_str(&format!(
        "  Prompt correlation: {} captures with prompt_id, {} pre-first-input, {} unknown payload\n",
        stats.captures_with_prompt_id, stats.captures_pre_first_input, stats.captures_unknown_payload
    ));
    // rm-037: the gateway spend window renders alongside the two
    // subscription windows, but only when one is actually running —
    // subscribers see no phantom arm.
    let mut limits = format!(
        "Limits: 5h {} (peak {}) · 7d {} (peak {})",
        format_percent(insights.five_hour.as_ref().and_then(|s| s.used_percentage)),
        format_percent(insights.five_hour_peak_used_percentage),
        format_percent(insights.seven_day.as_ref().and_then(|s| s.used_percentage)),
        format_percent(insights.seven_day_peak_used_percentage),
    );
    if let Some(spend) = &insights.spend_limit {
        limits.push_str(&format!(
            " · spend {} (peak {})",
            format_percent(spend.used_percentage),
            format_percent(insights.spend_limit_peak_used_percentage)
        ));
    }
    limits.push('\n');
    out.push_str(&limits);
    for crossing in &insights.limit_crossings {
        out.push_str(&format!(
            "  {} window reset at {}: {} -> {}\n",
            crossing.window,
            format_epoch(crossing.resets_at),
            format_percent(crossing.used_percentage_before),
            format_percent(crossing.used_percentage_after),
        ));
    }
    // rm-037: the USD/period trio rides a slower ~5-minute schedule
    // and can be absent while the percentage is present, so each
    // field renders only when the gateway actually sent it.
    if let Some(spend) = &insights.spend_limit {
        let mut parts: Vec<String> = Vec::new();
        let mut usd = match (spend.used_usd, spend.limit_usd) {
            (Some(used), Some(limit)) => Some(format!("${used:.2} of ${limit:.2}")),
            (Some(used), None) => Some(format!("${used:.2} spent")),
            (None, _) => None,
        };
        if let Some(period) = &spend.period {
            // rm-034 rule (review fix, run 415d77e1 537105b1 F1): the
            // period is a journal payload string, so both render arms
            // pass it through the control-character sanitization.
            let period = sanitize_line_segment(period);
            match &mut usd {
                Some(part) => {
                    part.push(' ');
                    part.push_str(&period);
                }
                None => usd = Some(period),
            }
        }
        if let Some(part) = usd {
            parts.push(part);
        }
        if let Some(resets_at) = spend.resets_at {
            parts.push(format!("resets {}", format_epoch(resets_at)));
        }
        if !parts.is_empty() {
            out.push_str(&format!("Spend limit: {}\n", parts.join(", ")));
        }
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
        // rm-771: when sessions predate the journal, say so — the
        // spend line is otherwise silently an under-count.
        if series.first_day_baseline_unknown_sessions > 0 {
            out.push_str(&format!(
                "Budget baseline: {} session(s) started before the journal — \
                 ${:.2} pre-journal spend excluded from the 7d total\n",
                series.first_day_baseline_unknown_sessions, series.first_day_baseline_unknown_usd
            ));
        }
        // "Statusline budget truth" (run 80336dfb): hostile cost
        // samples were discarded from the 7d spend above — this arm
        // discloses the same fact the --budget view carries.
        let hostile_cost_samples =
            series.negative_capture_cost_samples + series.invalid_capture_cost_samples;
        if hostile_cost_samples > 0 {
            out.push_str(&format!(
                "Budget: {} hostile cost sample{} discarded ({} negative, {} not a finite number) — excluded from the 7d spend\n",
                hostile_cost_samples,
                if hostile_cost_samples == 1 { "" } else { "s" },
                series.negative_capture_cost_samples,
                series.invalid_capture_cost_samples,
            ));
        }
    }
    // rm-037: context pressure per session — the payload's
    // pre-calculated utilization plus the last-API token counts
    // (input-only in `used_percentage` per the docs formula). Sessions
    // without a context arm stay absent; a null `current_usage`
    // (pre-first-API-call / post-compact) renders without the token
    // tail rather than as zeros.
    if !insights.context_windows.is_empty() {
        out.push_str(&format!(
            "Context window (peak used {}):\n",
            format_percent(insights.context_peak_used_percentage)
        ));
        for window in &insights.context_windows {
            // rm-034: session ids are journal payload strings.
            let mut line = format!(
                "  {}: used {}, remaining {}",
                sanitize_line_segment(&window.session_id),
                format_percent(window.used_percentage),
                format_percent(window.remaining_percentage)
            );
            if let Some(usage) = &window.current_usage {
                let mut counts: Vec<String> = Vec::new();
                if let Some(tokens) = usage.input_tokens {
                    counts.push(format!("{tokens} in"));
                }
                if let Some(tokens) = usage.output_tokens {
                    counts.push(format!("{tokens} out"));
                }
                if let Some(tokens) = usage.cache_creation_input_tokens {
                    counts.push(format!("{tokens} cache-write"));
                }
                if let Some(tokens) = usage.cache_read_input_tokens {
                    counts.push(format!("{tokens} cache-read"));
                }
                if !counts.is_empty() {
                    line.push_str(&format!(" · last API {}", counts.join(", ")));
                }
            }
            line.push('\n');
            out.push_str(&line);
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
    // rm-708: the correlation key itself, latest per session. The id
    // is a journal payload string and prints sanitized; the JSON arm
    // keeps the verbatim value.
    if !insights.prompt_ids.is_empty() {
        out.push_str("Latest prompt_id per session:\n");
        for entry in &insights.prompt_ids {
            out.push_str(&format!(
                "  {}: {}\n",
                sanitize_line_segment(&entry.session_id),
                sanitize_line_segment(&entry.prompt_id)
            ));
        }
    }
    out
}

// --- rm-385: weekly budget telemetry (cycle 1) -------------------------

/// Classified `cost.total_cost_usd` sample ("Statusline budget
/// truth" batch, run 80336dfb). The payload gates this field at
/// v2.1.251+ (rm-385), so a capture that simply carries no sample
/// yields `None`; a sample that is present but hostile is classified
/// so its discard is disclosed, never silent.
enum CaptureCostSample {
    /// Finite, non-negative spend — enters the day fold.
    Spend(f64),
    /// Finite but negative: a hostile or transient capture (editor
    /// hook, corrupted plugin payload) must never buy budget headroom.
    /// The magnitude is intentionally not carried — only the count is
    /// disclosed; keeping the value would invite rendering it.
    Negative,
    /// Present but not a finite number — `1e400` parses to `null` in
    /// the JSON tree, and strings/booleans are equally unusable.
    Invalid,
}

fn capture_cost_sample(capture: &CapturedStatusline) -> Option<CaptureCostSample> {
    let node = capture.payload.get("cost")?.get("total_cost_usd")?;
    match node.as_f64() {
        Some(v) if v.is_finite() => {
            if v >= 0.0 {
                Some(CaptureCostSample::Spend(v))
            } else {
                Some(CaptureCostSample::Negative)
            }
        }
        _ => Some(CaptureCostSample::Invalid),
    }
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
///
/// rm-771 (run e88f2da2 cycle 3): a session whose first RETAINED
/// sample already carries a nonzero cumulative cost cannot be proven
/// to have started on that day — its pre-journal spend is excluded
/// from the rises and disclosed as a baseline-unknown bias instead of
/// being booked into the first retained day (the 26x overstatement
/// class: `--weekly-budget 10` printed "OVER by $42.00" on a true
/// $2.00 window burn). Sessions observed from a zero baseline keep
/// the original full-rise accounting. The disclosure counters are
/// scoped to the same calendar window as `daily`/`total` (review
/// 9af53c3f F3): a session whose first retained day the window
/// already dropped whole never biased the windowed number and is
/// not counted.
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
    /// Sessions whose first retained sample already carried a
    /// nonzero cumulative cost; their pre-journal spend is excluded
    /// from `daily`/`total` and disclosed here (rm-771 arm 1).
    /// Scoped to the same calendar window as `daily`/`total` (review
    /// 9af53c3f F3): a session whose first retained day the window
    /// already dropped whole never biased the windowed total and is
    /// not counted.
    pub first_day_baseline_unknown_sessions: usize,
    /// Sum of those sessions' first observed cumulative values — the
    /// excluded pre-journal bias, rounded to cents (rm-771 arm 1).
    pub first_day_baseline_unknown_usd: f64,
    /// Captures whose cost node carried a finite negative
    /// `total_cost_usd` — discarded before the day fold and disclosed
    /// here: a negative cumulative must never buy budget headroom
    /// ("Statusline budget truth" batch, run 80336dfb).
    pub negative_capture_cost_samples: usize,
    /// Captures whose cost node exists but is not a finite number
    /// (`1e400` arrives as `null`; strings and booleans likewise) —
    /// discarded and disclosed here so a lines-vs-daily discrepancy
    /// has a visible cause. Not window-scoped: the discard decision
    /// happens before day keying.
    pub invalid_capture_cost_samples: usize,
}

pub fn statusline_budget_series(
    captures: &[CapturedStatusline],
    days: usize,
    now: DateTime<Utc>,
) -> StatuslineBudgetSeries {
    use std::collections::BTreeMap;

    // session -> day -> (first observed cumulative, max cumulative).
    // The first observed value of the session's earliest retained day
    // is the baseline witness: 0.0 proves the journal saw the session
    // from its start; anything above 0.0 means spend predates the
    // retained journal — the session began mid-window, or the 10 MiB
    // retention cap (rm-385) dropped its true first days (rm-771).
    let mut per_session: BTreeMap<&str, BTreeMap<String, (f64, f64)>> = BTreeMap::new();
    let mut negative_capture_cost_samples = 0usize;
    let mut invalid_capture_cost_samples = 0usize;
    for capture in captures {
        // "Statusline budget truth" (run 80336dfb): classify the cost
        // node BEFORE the fold. A hostile negative or a
        // non-representable value (1e400 parses to `null`) is
        // discarded with a disclosed count instead of silently
        // entering the series — a negative cumulative must never buy
        // budget headroom, and a junk cost must never vanish without
        // a trace.
        let cost = match capture_cost_sample(capture) {
            Some(CaptureCostSample::Spend(cost)) => cost,
            Some(CaptureCostSample::Negative) => {
                negative_capture_cost_samples += 1;
                continue;
            }
            Some(CaptureCostSample::Invalid) => {
                invalid_capture_cost_samples += 1;
                continue;
            }
            None => continue,
        };
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
            // Seed with the first observed cost itself: the
            // witness must be that value, not -inf (a -inf seed
            // can never be lowered into, so `first_seen` stayed
            // -inf and the baseline-unknown arm never fired).
            .or_insert((cost, cost));
        if cost < entry.0 {
            entry.0 = cost;
        }
        if cost > entry.1 {
            entry.1 = cost;
        }
    }
    let mut daily: BTreeMap<String, f64> = BTreeMap::new();
    // (first retained day, excluded carry) per baseline-unknown
    // session — collected here, counted against the trailing window
    // below (review 9af53c3f F3).
    let mut baseline_unknown: Vec<(String, f64)> = Vec::new();
    for samples in per_session.values() {
        let mut previous: Option<f64> = None;
        for (day, (first_seen, day_max)) in samples {
            // rm-771: the session's first retained day seeds the
            // baseline. A nonzero witness cannot prove the session
            // started at zero that day, so its value is excluded from
            // the rise (the day contributes only the delta observed
            // between its own samples) and disclosed as a bias term.
            let rise = match previous {
                None if *first_seen > 0.0 => {
                    baseline_unknown.push((day.clone(), *first_seen));
                    (day_max - first_seen).max(0.0)
                }
                None => *day_max,
                Some(prev) => (day_max - prev).max(0.0),
            };
            *daily.entry(day.clone()).or_insert(0.0) += rise;
            previous = Some(*day_max);
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
            // rm-771: an empty window counts no baseline-unknown
            // bias — F3 scopes the counters to the (empty) window.
            first_day_baseline_unknown_sessions: 0,
            first_day_baseline_unknown_usd: 0.0,
            negative_capture_cost_samples,
            invalid_capture_cost_samples,
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
    // Review 9af53c3f F3 (rm-771, re-anchored onto rm-917's calendar
    // window at integration): scope the baseline-unknown counters to
    // the same window as `daily`/`total`. A session whose first
    // retained day is OLDER than the calendar window was dropped
    // whole by the window filter — its carry never touched the
    // windowed total — so counting it overstated the window's bias
    // while both render sites said "excluded from the 7d total".
    // Only an unprovable first day INSIDE the window biases the
    // windowed number and is disclosed (a future-dated first day is
    // outside the today-bounded window the same way and counts
    // nothing).
    let window_days: HashSet<&str> = window.iter().map(|(day, _)| day.as_str()).collect();
    let mut baseline_unknown_sessions = 0usize;
    let mut baseline_unknown_usd = 0.0f64;
    for (day, carry) in &baseline_unknown {
        if window_days.contains(day.as_str()) {
            baseline_unknown_sessions += 1;
            baseline_unknown_usd += *carry;
        }
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
        excluded_older_days,
        first_day_baseline_unknown_sessions: baseline_unknown_sessions,
        first_day_baseline_unknown_usd: (baseline_unknown_usd * 100.0).round() / 100.0,
        negative_capture_cost_samples,
        invalid_capture_cost_samples,
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
    // and the head cut rides along into the stats disclosure; rm-922: it
    // is also the lossy, over-cap-disclosing decode.
    let read = read_statusline_capture_bounded(&path);
    let stats = statusline_journal_stats_from_buffer(&path, &read.text, read.capped_away_bytes);
    let captures = parse_statusline_captures(&read.text);
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
            // "Statusline budget truth" (run 80336dfb): hostile cost
            // samples are discarded from every spend number in this
            // object and disclosed here — never silently.
            "negative_capture_cost_samples": series.negative_capture_cost_samples,
            "invalid_capture_cost_samples": series.invalid_capture_cost_samples,
            // rm-917: stale sampled days the calendar window excluded.
            "excluded_older_days": series.excluded_older_days,
            // rm-771: sessions that predate the journal are excluded
            // from the 7d total; expose the bias so the number is
            // interpretable (0 / 0.0 when every session was observed
            // from a zero baseline).
            "first_day_baseline_unknown_sessions":
                series.first_day_baseline_unknown_sessions,
            "first_day_baseline_unknown_usd": series.first_day_baseline_unknown_usd,
        });
        if let Some(budget) = weekly_budget {
            value["weekly_budget_usd"] = serde_json::json!(budget);
            // "Statusline budget truth" (run 80336dfb): floored into
            // [0, budget] — same contract as the --statusline-report
            // budget block; the text arm below keeps its OVER verdict.
            value["remaining_usd"] =
                serde_json::json!((budget - series.total).max(0.0).min(budget.max(0.0)));
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
    if series.first_day_baseline_unknown_sessions > 0 {
        out.push_str(&format!(
            "  first-day baseline unknown for {} session(s) — \
             ${:.2} of pre-journal spend excluded from the 7d total\n",
            series.first_day_baseline_unknown_sessions, series.first_day_baseline_unknown_usd
        ));
    }
    // "Statusline budget truth" (run 80336dfb): hostile cost samples
    // (finite negatives, non-representable values like 1e400) are
    // excluded from the 7d total above — disclosed here, never
    // dropped silently.
    let hostile_cost_samples =
        series.negative_capture_cost_samples + series.invalid_capture_cost_samples;
    if hostile_cost_samples > 0 {
        out.push_str(&format!(
            "  {} hostile cost sample{} discarded ({} negative, {} not a finite number) — excluded from the 7d total\n",
            hostile_cost_samples,
            if hostile_cost_samples == 1 {
                ""
            } else {
                "s"
            },
            series.negative_capture_cost_samples,
            series.invalid_capture_cost_samples,
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
    #[test]
    fn rm936_is_bidi_format_control_bounds() {
        // The directional/format set (Cf) the lane sanitizers must kill.
        for c in [
            '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{200E}', '\u{200F}',
            '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{061C}',
        ] {
            assert!(is_bidi_format_control(c), "{c:?} must be flagged");
        }
        // Everything else stays on its own path.
        assert!(!is_bidi_format_control('a'));
        assert!(!is_bidi_format_control('\u{FFFD}'));
        assert!(!is_bidi_format_control('\u{1B}')); // Cc: is_control's job
        assert!(!is_bidi_format_control('\u{00AD}')); // Cf but not directional
        assert!(!is_bidi_format_control('\u{0A0}')); // Zl layout, not Cf
    }

    #[test]
    fn rm936_line_segments_neutralize_bidi_format_controls() {
        for c in ['\u{202E}', '\u{202A}', '\u{200F}', '\u{2066}', '\u{061C}'] {
            let out = sanitize_line_segment(&format!("a{c}b"));
            assert!(!out.contains(c), "raw {c:?} survived line sanitization");
            assert!(out.contains('\u{FFFD}'), "no replacement marker for {c:?}");
        }
        // The pre-existing Cc contract is unchanged.
        assert!(sanitize_line_segment("x\u{1B}[0my").contains('\u{FFFD}'));
        assert_eq!(sanitize_line_segment("plain-1"), "plain-1");
    }

    #[test]
    fn rm936_output_document_keeps_layout_bytes_but_drops_directional_controls() {
        let poisoned = "model: claude\u{202E}evil cost: $5.00";
        let out = sanitize_output_document(poisoned);
        assert!(!out.contains('\u{202E}'));
        assert!(out.contains('\u{FFFD}'));
        // rm-625's structural-whitespace contract is untouched.
        assert_eq!(sanitize_output_document("a\nb\tc\rd"), "a\nb\tc\rd");
    }
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
        // sum entirely. rm-771 (composed at integration): the
        // session's first RETAINED sample (D-5) already carries
        // $2.00 — an unprovable pre-journal baseline, excluded from
        // the day's rise and disclosed — so only the D-5→today rise
        // of $3.00 is provable window spend.
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
                5 => 0.00, // rm-771: the first sample's $2.00 carry is a baseline, not a rise
                0 => 3.00, // rise from 2.00 to 5.00
                _ => 0.00,
            };
            assert!((spend - expect_spend).abs() < 1e-9, "{:?}", series.daily);
        }
        assert!((series.total - 3.00).abs() < 1e-9, "{series:?}");
        assert_eq!(series.first_day_baseline_unknown_sessions, 1, "{series:?}");
        assert!((series.first_day_baseline_unknown_usd - 2.00).abs() < 1e-9);
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
                read_truncated: false,
                captures_with_prompt_id: 0,
                captures_pre_first_input: 0,
                captures_unknown_payload: 0,
                unparsable_lines: 0,
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
            for key in [
                "journal",
                "daily",
                "spend_7d_usd",
                // rm-771: the baseline-unknown bias is always present
                // (0 / 0.0 when every session was observed from a
                // zero baseline) so the spend stays interpretable.
                "first_day_baseline_unknown_sessions",
                "first_day_baseline_unknown_usd",
            ] {
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
        // Day -10: session A's first retained capture is 2.00 — rm-771:
        // the journal cannot prove A started that day, so the 2.00 is
        // pre-journal baseline (excluded, disclosed when the window
        // retains day -10), and day -10 contributes only the delta it
        // observed (none beyond the baseline). Day -9: A continues to
        // 3.50 (rise 1.50 from the retained baseline) while B's first
        // capture is 1.00 — same rule: excluded as B's baseline,
        // disclosed. Days -8..-1: A restarts its counter and burns
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
        // Review 9af53c3f F3 (rm-771, on the rm-917 calendar window):
        // the baseline-unknown counters scope to the window. A's
        // first retained day (day -10) and B's (day -9) both sit
        // outside the trailing 7 calendar days, so neither session's
        // unprovable carry biases the windowed total and neither is
        // counted — "excluded from the 7d total" stays literally
        // true.
        assert_eq!(series.first_day_baseline_unknown_sessions, 0, "{series:?}");
        assert!(series.first_day_baseline_unknown_usd.abs() < 1e-9);
        // The full-journal variant sums only the provable burn: the
        // day -9 rise of 1.50 plus the 7 × 0.25 tail (the 2.00/1.00
        // baselines are excluded, not booked as same-day spend) —
        // and inside a window that RETAINS days -10/-9 both
        // baseline-unknown sessions are disclosed again. Day -8 is a
        // counter RESET (contributes 0), and tomorrow's $99 future
        // capture is excluded by the today-bounded upper bound;
        // rm-917 zero-fills the unsampled in-window days.
        let full = statusline_budget_series(&captures, 30, now);
        assert!((full.total - (1.50 + 7.0 * 0.25)).abs() < 1e-9, "{full:?}");
        assert_eq!(full.first_day_baseline_unknown_sessions, 2);
        assert!((full.first_day_baseline_unknown_usd - 3.00).abs() < 1e-9);
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
    fn budget_series_first_day_baseline_unknown_not_booked() {
        // rm-771 (run e88f2da2 cycle 3) red-first, arm 3: the PoC
        // corpus (delegate spool poc-mirror/budget-overstate/) is a
        // session whose FIRST retained capture already carries a
        // $50.00 cumulative cost — the journal cannot prove the
        // session started on that day, so the $50 is pre-journal
        // spend, not a same-day rise. Booking it as a rise overstated
        // the 7d burn 26x (`--weekly-budget 10` printed "OVER by
        // $42.00" on a true $2.00 burn). Re-anchored onto rm-917's
        // calendar window at integration: the two retained days sit
        // inside a pinned trailing week whose gap days zero-fill.
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
        let day = |n: i64| now.timestamp() + n * 86_400;
        let captures = vec![
            budget_capture("s", day(-1) + 100, 50.00),
            budget_capture("s", day(0) + 100, 52.00),
        ];
        let series = statusline_budget_series(&captures, 7, now);
        assert!(
            (series.total - 2.0).abs() < 1e-9,
            "only the observed rise counts: {series:?}"
        );
        assert_eq!(series.daily.len(), 7, "{:?}", series.daily);
        assert!(
            (series.daily[5].1 - 0.0).abs() < 1e-9 && (series.daily[6].1 - 2.0).abs() < 1e-9,
            "the first retained day contributes only the delta it
             observed itself: {:?}",
            series.daily
        );
        assert_eq!(series.first_day_baseline_unknown_sessions, 1, "{series:?}");
        assert!((series.first_day_baseline_unknown_usd - 50.0).abs() < 1e-9);
    }

    #[test]
    fn budget_series_zero_start_keeps_full_rise() {
        // rm-771 compatibility arm: a session observed from a zero
        // baseline is proven to have started inside the journal, so
        // its full first-day rise still counts — and nothing is
        // disclosed as baseline-unknown.
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
        let day = |n: i64| now.timestamp() + n * 86_400;
        let captures = vec![
            budget_capture("z", day(-1) + 100, 0.0),
            budget_capture("z", day(-1) + 200, 3.00),
            budget_capture("z", day(0) + 100, 5.00),
        ];
        let series = statusline_budget_series(&captures, 7, now);
        assert!((series.total - 5.0).abs() < 1e-9, "{series:?}");
        assert_eq!(series.first_day_baseline_unknown_sessions, 0, "{series:?}");
    }

    #[test]
    fn budget_over_state_comes_from_observed_rise_not_the_carry() {
        // rm-771 arm 3, literal over-budget form: a session whose
        // journal opens at cumulative $50.00 and rises to $62.00
        // genuinely burned $12.00 inside the window, so against a
        // $10.00 weekly budget the honest render is "OVER by
        // $2.00" — from the observed rise ONLY, with the $50.00
        // pre-journal carry excluded and disclosed. (On the same
        // journal pre-fix, the carry was booked as same-day spend
        // and the render said "OVER by $52.00" — the 26x
        // overstatement class of the durable PoC corpus, whose own
        // $50 -> $52 shape pins the under-budget complement:
        // $2.00 spend, $8.00 remaining.)
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
        let day = |n: i64| now.timestamp() + n * 86_400;
        let captures = vec![
            budget_capture("s", day(-1) + 100, 50.00),
            budget_capture("s", day(0) + 100, 62.00),
        ];
        let series = statusline_budget_series(&captures, 7, now);
        assert!((series.total - 12.0).abs() < 1e-9, "{series:?}");
        // Hermetic render: `render_statusline_report_text` takes the
        // series directly, so the literal arm-3 string is pinned
        // without touching the host journal.
        let text = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 2,
                bytes: 196,
                retained_max_bytes: 10_485_760,
                capped_away_bytes: 0,
                read_truncated: false,
                captures_with_prompt_id: 0,
                captures_pre_first_input: 0,
                captures_unknown_payload: 0,
                unparsable_lines: 0,
            },
            &statusline_insights(&[]),
            Some(10.0),
            &series,
        );
        assert!(
            text.contains("Budget: 7d spend $12.00 of $10.00 — OVER by $2.00"),
            "{text}"
        );
        assert!(
            text.contains("1 session(s) started before the journal"),
            "{text}"
        );
        assert!(text.contains("$50.00 pre-journal spend excluded"), "{text}");
        assert_eq!(series.first_day_baseline_unknown_sessions, 1);
        assert!((series.first_day_baseline_unknown_usd - 50.0).abs() < 1e-9);
    }

    #[test]
    fn budget_series_baseline_unknown_counters_scope_to_the_trailing_window() {
        // Review 9af53c3f F3 (rm-771): the baseline-unknown counters
        // describe the bias OF THE WINDOWED TOTAL, so they must be
        // scoped to the same window as `daily`/`total` — the rm-917
        // CALENDAR window since integration. A session whose first
        // retained day is older than the calendar window contributed
        // no unprovable dollars to the windowed number (the window
        // filter dropped its first day whole), and counting it
        // overstated the disclosed bias — "…of pre-journal spend
        // excluded from the 7d total" named dollars that never
        // touched the 7d total.
        use chrono::TimeZone;
        // `now` pinned to 2026-10-09: the trailing 7 calendar days
        // keep 10-03..10-09; the 30-day variant reaches back to
        // 09-09 and retains both first days.
        let now = Utc.with_ymd_and_hms(2026, 10, 9, 12, 0, 0).unwrap();
        let day = |n: i64| now.timestamp() + n * 86_400;
        let mut captures = Vec::new();
        // "old": first retained capture (day -15, OUTSIDE the 7-day
        // window) already carries a $40.00 cumulative — pre-journal
        // carry, but the flat 40.00 tail burns nothing; day 0 rises
        // to 41.00.
        for n in -15..=-8 {
            captures.push(budget_capture("old", day(n) + 100, 40.00));
        }
        captures.push(budget_capture("old", day(0) + 100, 41.00));
        // "new": first retained capture (day 0, INSIDE the window)
        // at $3.00, rising to $4.50 the same day.
        captures.push(budget_capture("new", day(0) + 200, 3.00));
        captures.push(budget_capture("new", day(0) + 300, 4.50));
        let series = statusline_budget_series(&captures, 7, now);
        assert_eq!(series.daily.len(), 7, "{:?}", series.daily);
        assert!((series.total - 2.50).abs() < 1e-9, "{series:?}");
        // Only the in-window first day biases the windowed total.
        assert_eq!(series.first_day_baseline_unknown_sessions, 1, "{series:?}");
        assert!((series.first_day_baseline_unknown_usd - 3.00).abs() < 1e-9);
        // A window that retains day -15 discloses both carries
        // again — the counters follow the window, not the journal.
        let full = statusline_budget_series(&captures, 30, now);
        assert_eq!(full.first_day_baseline_unknown_sessions, 2, "{full:?}");
        assert!((full.first_day_baseline_unknown_usd - 43.00).abs() < 1e-9);
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
        // Cycle-3 review F1 (attempt ae97110b): a RELATIVE HOME is
        // unset-equivalent here too — `HOME=rel-home` planted
        // ./rel-home/.cache/agenttrace/statusline.jsonl (0600) in the
        // CWD and `--statusline-report` read it back (live PoC:
        // ae97110b-scratch/poc/), while the EMPTY arm above was
        // already guarded. Same rule the rm-248 rider pinned at the
        // session_cache / doctor / pricing copies.
        let relative = Some(OsStr::new("rel-home"));
        assert_eq!(
            resolve_user_cache_dir(relative, None),
            std::env::temp_dir(),
            "a relative HOME must behave exactly like an unset one, not ./rel-home/.cache"
        );
        // A usable absolute HOME still wins over no XDG override.
        assert_eq!(
            resolve_user_cache_dir(Some(OsStr::new("/home/at-test")), None),
            PathBuf::from("/home/at-test").join(".cache")
        );
        // Relative HOME + relative XDG: never a relative ladder end.
        let both_relative = resolve_user_cache_dir(relative, Some(OsStr::new("rel-cache")));
        assert!(both_relative.is_absolute(), "{both_relative:?}");
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
            // rm-248: a RELATIVE XDG_CACHE_HOME is treated as unset —
            // the XDG spec only defines absolute values, and honoring
            // a relative one scattered a per-cwd cache
            // (XDG_CACHE_HOME=rel-cache wrote ./rel-cache/agenttrace
            // from a scratch cwd). Same ladder as session_cache /
            // doctor / pricing (`relative_xdg_cache_home_is_ignored_per_spec`).
            assert_eq!(
                resolve_user_cache_dir(
                    Some(OsStr::new("/home/at-test")),
                    Some(OsStr::new("rel-cache"))
                ),
                PathBuf::from("/home/at-test").join(".cache")
            );
            let relative_only = resolve_user_cache_dir(empty, Some(OsStr::new("rel-cache")));
            assert!(relative_only.is_absolute(), "{relative_only:?}");
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
                read_truncated: false,
                captures_with_prompt_id: 0,
                captures_pre_first_input: 0,
                captures_unknown_payload: 0,
                unparsable_lines: 0,
            },
            &statusline_insights(&captures),
            budget,
            &series,
        );
        // rm-771: the session's journal OPENS at a cumulative $2.00,
        // so only the observed $1.00 rise is provable 7d spend; the
        // $2.00 pre-journal baseline is excluded and disclosed
        // instead of booked into the first captured day.
        assert!(text.contains("Budget: 7d spend $1.00 of $10.00"), "{text}");
        assert!(text.contains("$9.00 remaining"), "{text}");
        assert!(
            text.contains("1 session(s) started before the journal"),
            "{text}"
        );
        assert!(text.contains("$2.00 pre-journal spend excluded"), "{text}");
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
                read_truncated: false,
                captures_with_prompt_id: 0,
                captures_pre_first_input: 0,
                captures_unknown_payload: 0,
                unparsable_lines: 0,
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
                read_truncated: false,
                captures_with_prompt_id: 0,
                captures_pre_first_input: 2,
                captures_unknown_payload: 0,
                unparsable_lines: 0,
            },
            &statusline_insights(&captures),
            None,
            &series,
        );
        assert!(!unbudgeted.contains("Budget:"), "{unbudgeted}");
    }

    #[test]
    fn statusline_report_gains_context_spend_prompt_arms_json() {
        // rm-037/rm-708 (cycle 2): the 2026-10-07 payload docs add
        // context_window.{used_percentage,remaining_percentage,
        // current_usage}, the gateway rate_limits.spend_limit window
        // (with the USD/period trio) and the prompt_id correlation key.
        // The report surface must carry all three arms; a payload with
        // unknown future fields must still parse (tolerance).
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-arms-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        append_statusline_capture(&serde_json::json!({
            "session_id": "probe-ctx-1",
            "version": "2.1.292",
            "context_window": {
                "used_percentage": 41.2,
                "remaining_percentage": 58.8,
                "current_usage": {
                    "input_tokens": 8500,
                    "output_tokens": 1200,
                    "cache_creation_input_tokens": 5000,
                    "cache_read_input_tokens": 2000
                }
            },
            "rate_limits": {
                "five_hour": {"used_percentage": 84.0, "resets_at": 1790000000},
                "seven_day": {"used_percentage": 46.0, "resets_at": 1790080000},
                "spend_limit": {
                    "used_percentage": 62.8,
                    "resets_at": 1790100000,
                    "used_usd": 314.12,
                    "limit_usd": 500.0,
                    "period": "monthly"
                }
            },
            "prompt_id": "550e8400-e29b-41d4-a716-446655440000",
            "future_field_a": {"nested": [1, 2, 3]}
        }))
        .expect("append fixture");
        let json = render_statusline_report("json", None).expect("json report");
        let value: Value = serde_json::from_str(&json).expect("json parses");
        assert_eq!(
            value["insights"]["spend_limit"]["used_usd"], 314.12,
            "spend_limit USD arm extracts: {json}"
        );
        assert_eq!(
            value["insights"]["context_windows"][0]["used_percentage"], 41.2,
            "context window arm extracts: {json}"
        );
        assert_eq!(
            value["insights"]["prompt_ids"][0]["prompt_id"], "550e8400-e29b-41d4-a716-446655440000",
            "prompt_id arm extracts: {json}"
        );
        assert_eq!(
            value["journal"]["captures_with_prompt_id"], 1,
            "journal diagnostics carry the prompt-key counters: {json}"
        );
        assert_eq!(
            value["insights"]["five_hour"]["used_percentage"], 84.0,
            "subscription windows still extract alongside the new arms: {json}"
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn statusline_report_gains_context_spend_prompt_arms_text() {
        // rm-037/rm-708 text arm: spend renders alongside the two
        // subscription windows, context utilization renders per
        // session with the last-API token counts, and the prompt
        // correlation key renders — sanitized (rm-034 convention:
        // journal-derived strings never print raw control bytes).
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-arms-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        append_statusline_capture(&serde_json::json!({
            "session_id": "probe-ctx-1",
            "context_window": {
                "used_percentage": 41.2,
                "remaining_percentage": 58.8,
                "current_usage": {
                    "input_tokens": 8500,
                    "output_tokens": 1200,
                    "cache_creation_input_tokens": 5000,
                    "cache_read_input_tokens": 2000
                }
            },
            "rate_limits": {
                "spend_limit": {
                    "used_percentage": 62.8,
                    "resets_at": 1790100000,
                    "used_usd": 314.12,
                    "limit_usd": 500.0,
                    "period": "monthly"
                }
            },
            "prompt_id": "550e8400-e29b-41d4-a716-446655440000"
        }))
        .expect("append fixture");
        let text = render_statusline_report("text", None).expect("text report");
        assert!(
            text.contains("· spend 63% (peak 63%)"),
            "spend on the Limits line: {text}"
        );
        assert!(
            text.contains("Spend limit: $314.12 of $500.00 monthly"),
            "spend USD detail line: {text}"
        );
        assert!(
            text.contains("Context window (peak used 41%)"),
            "context block: {text}"
        );
        assert!(
            text.contains("used 41%, remaining 59% · last API 8500 in, 1200 out, 5000 cache-write, 2000 cache-read"),
            "context per-session line: {text}"
        );
        assert!(
            text.contains("Latest prompt_id per session:"),
            "prompt_id block: {text}"
        );
        assert!(
            text.contains("550e8400-e29b-41d4-a716-446655440000"),
            "prompt_id value renders: {text}"
        );
        assert!(
            text.contains("1 captures with prompt_id, 0 pre-first-input, 0 unknown payload"),
            "prompt diagnostics line: {text}"
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn statusline_insights_extracts_context_spend_prompt_arms() {
        // rm-037/rm-708 unit: latest-per-session context windows, peak
        // context utilization, the spend-limit window (latest + peak),
        // and latest prompt_id per session — with unknown future
        // fields riding along in the payload without breaking the
        // extraction.
        let captures = vec![
            CapturedStatusline {
                captured_at: 1_000,
                payload: serde_json::json!({
                    "session_id": "sess-a",
                    "context_window": {
                        "used_percentage": 41.2,
                        "remaining_percentage": 58.8,
                        "current_usage": {
                            "input_tokens": 8500,
                            "output_tokens": 1200,
                            "cache_creation_input_tokens": 5000,
                            "cache_read_input_tokens": 2000
                        }
                    },
                    "rate_limits": {
                        "five_hour": {"used_percentage": 84.0, "resets_at": 1_790_000_000},
                        "spend_limit": {
                            "used_percentage": 62.8,
                            "resets_at": 1_790_100_000,
                            "used_usd": 314.12,
                            "limit_usd": 500.0,
                            "period": "monthly"
                        }
                    },
                    "prompt_id": "pid-a-1",
                    "future_field": {"anything": [1]}
                }),
            },
            CapturedStatusline {
                captured_at: 2_000,
                payload: serde_json::json!({
                    "session_id": "sess-a",
                    "context_window": {"used_percentage": 90.0, "remaining_percentage": 10.0},
                    "prompt_id": "pid-a-2"
                }),
            },
            CapturedStatusline {
                captured_at: 3_000,
                payload: serde_json::json!({
                    "session_id": "sess-b",
                    "context_window": {
                        "used_percentage": null,
                        "remaining_percentage": null,
                        "current_usage": null
                    },
                    "rate_limits": {
                        "spend_limit": {"used_percentage": 10.0, "resets_at": 1_790_200_000}
                    }
                }),
            },
        ];
        let insights = statusline_insights(&captures);
        // Latest context window per session, newest session first;
        // sess-b is in the documented null state (pre-first-API-call
        // or post-compact) and still reports as a session.
        assert_eq!(insights.context_windows.len(), 2);
        assert_eq!(insights.context_windows[0].session_id, "sess-b");
        assert_eq!(insights.context_windows[0].used_percentage, None);
        assert_eq!(insights.context_windows[0].current_usage, None);
        assert_eq!(insights.context_windows[1].session_id, "sess-a");
        assert_eq!(insights.context_windows[1].used_percentage, Some(90.0));
        assert_eq!(insights.context_windows[1].remaining_percentage, Some(10.0));
        assert!(insights.context_windows[1].current_usage.is_none());
        // Peak across every capture that carried a percentage.
        assert_eq!(insights.context_peak_used_percentage, Some(90.0));
        // Spend window: latest capture wins, peak is the all-time max.
        let spend = insights.spend_limit.expect("spend_limit extracts");
        assert_eq!(spend.used_percentage, Some(10.0));
        assert_eq!(spend.used_usd, None);
        assert_eq!(insights.spend_limit_peak_used_percentage, Some(62.8));
        // Prompt ids: latest per session; sess-b has none (documented
        // pre-first-input state, not an error).
        assert_eq!(insights.prompt_ids.len(), 1);
        assert_eq!(insights.prompt_ids[0].session_id, "sess-a");
        assert_eq!(insights.prompt_ids[0].prompt_id, "pid-a-2");
        // Subscription windows unaffected.
        assert_eq!(
            insights.five_hour.as_ref().and_then(|s| s.used_percentage),
            Some(84.0)
        );
    }

    #[test]
    fn context_window_parsed_once_per_capture() {
        // rm-921 residual: the peak-utilization fold and the per-session
        // latest-window list must share ONE context_window() parse per
        // capture (each parse is a Value clone + serde deserialize on the
        // statusline hot path). Red on the double-parse layout: the
        // counter observes two invocations per capture that carries a
        // context_window payload.
        let captures = vec![
            CapturedStatusline {
                captured_at: 1_000,
                payload: serde_json::json!({
                    "session_id": "sess-a",
                    "context_window": {
                        "used_percentage": 41.2,
                        "remaining_percentage": 58.8
                    }
                }),
            },
            CapturedStatusline {
                captured_at: 2_000,
                payload: serde_json::json!({
                    "session_id": "sess-a",
                    "context_window": {
                        "used_percentage": 90.0,
                        "remaining_percentage": 10.0
                    }
                }),
            },
            CapturedStatusline {
                captured_at: 3_000,
                payload: serde_json::json!({
                    "session_id": "sess-b",
                    "prompt_id": "prompt-b-1"
                }),
            },
        ];
        CONTEXT_WINDOW_PARSE_COUNT.with(|count| count.set(0));
        let insights = statusline_insights(&captures);
        CONTEXT_WINDOW_PARSE_COUNT.with(|count| {
            assert_eq!(
                count.get(),
                2,
                "context_window() must be invoked exactly once per capture \
                 that carries a payload — the peak fold and the per-session \
                 list share the parse"
            );
        });
        // Both consumers still ran: the peak fold and the per-session
        // latest-window list (sess-a wins with the newest capture).
        assert_eq!(insights.context_peak_used_percentage, Some(90.0));
        assert_eq!(insights.context_windows.len(), 1);
        assert_eq!(insights.context_windows[0].session_id, "sess-a");
        assert_eq!(insights.context_windows[0].used_percentage, Some(90.0));
    }

    #[test]
    fn spend_limit_window_joins_crossings_peaks_and_latest() {
        // rm-037: the gateway spend window resets on the same epoch
        // mechanism as the subscription windows, so it joins the
        // crossing scan — before-side usage, falling usage after the
        // boundary.
        let captures = vec![
            CapturedStatusline {
                captured_at: 100,
                payload: serde_json::json!({
                    "session_id": "sess-a",
                    "rate_limits": {
                        "spend_limit": {"used_percentage": 62.8, "resets_at": 200}
                    }
                }),
            },
            CapturedStatusline {
                captured_at: 300,
                payload: serde_json::json!({
                    "session_id": "sess-a",
                    "rate_limits": {
                        "spend_limit": {"used_percentage": 10.0, "resets_at": 500}
                    }
                }),
            },
        ];
        let insights = statusline_insights(&captures);
        let spend_crossings: Vec<_> = insights
            .limit_crossings
            .iter()
            .filter(|crossing| crossing.window == "spend_limit")
            .collect();
        assert_eq!(spend_crossings.len(), 1, "{:?}", insights.limit_crossings);
        assert_eq!(spend_crossings[0].resets_at, 200);
        assert_eq!(spend_crossings[0].used_percentage_before, Some(62.8));
        assert_eq!(spend_crossings[0].used_percentage_after, Some(10.0));
    }

    #[test]
    fn statusline_insights_tolerates_legacy_and_unknown_payloads() {
        // rm-037 acceptance: a legacy pre-2.1.251 payload (no
        // rate_limits / prompt_cache / context_window / prompt_id at
        // all) extracts without error and renders with no phantom
        // arms; unknown future fields and documented null states are
        // tolerated.
        let legacy = vec![CapturedStatusline {
            captured_at: 1_000,
            payload: serde_json::json!({
                "hook_event_name": "Status",
                "session_id": "legacy-1",
                "transcript_path": "/tmp/legacy.jsonl",
                "cwd": "/work",
                "version": "2.0.9",
                "model": {"id": "claude-sonnet-4-5"},
                "cost": {"total_cost_usd": 0.42}
            }),
        }];
        let insights = statusline_insights(&legacy);
        assert_eq!(insights.spend_limit, None);
        assert!(insights.context_windows.is_empty());
        assert_eq!(insights.context_peak_used_percentage, None);
        assert!(insights.prompt_ids.is_empty());
        assert_eq!(insights.five_hour, None);
        // rm-917 determinism: the calendar window anchors to a pinned
        // `now` (review 95d74221 retired the real-today variant), so
        // the 1970-era fixture captures sit deterministically outside
        // the 7-day window — budget None, so no Budget line either way.
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 10, 8, 12, 0, 0).unwrap();
        let text = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 1,
                bytes: 200,
                retained_max_bytes: 1024,
                capped_away_bytes: 0,
                read_truncated: false,
                captures_with_prompt_id: 0,
                captures_pre_first_input: 1,
                captures_unknown_payload: 0,
                unparsable_lines: 0,
            },
            &insights,
            None,
            &statusline_budget_series(&legacy, 7, now),
        );
        assert!(!text.contains("spend"), "no phantom spend arm: {text}");
        assert!(
            !text.contains("Context window"),
            "no phantom context arm: {text}"
        );
        assert!(
            !text.contains("Latest prompt_id"),
            "no phantom prompt arm: {text}"
        );

        // Documented null states (pre-first-API-call / post-compact)
        // and unknown future fields parse to absent arms, not errors.
        let nulled = vec![CapturedStatusline {
            captured_at: 2_000,
            payload: serde_json::json!({
                "session_id": "null-1",
                "context_window": {
                    "used_percentage": null,
                    "remaining_percentage": null,
                    "current_usage": null
                },
                "rate_limits": {"spend_limit": {"used_percentage": null}},
                "prompt_id": null,
                "future_field": "unknown"
            }),
        }];
        let insights = statusline_insights(&nulled);
        assert_eq!(insights.context_windows.len(), 1);
        assert_eq!(insights.context_windows[0].used_percentage, None);
        assert_eq!(insights.context_windows[0].current_usage, None);
        assert_eq!(insights.context_peak_used_percentage, None);
        assert_eq!(insights.spend_limit, Some(StatuslineSpendLimit::default()));
        assert!(insights.prompt_ids.is_empty());
    }

    #[test]
    fn statusline_journal_stats_classifies_prompt_key_presence() {
        // rm-708 diagnostics: captures-without-prompt_id (the
        // documented pre-first-input state) are disclosed distinctly
        // from unclassifiable payloads; torn tail lines count only as
        // lines.
        let dir = std::env::temp_dir().join(format!(
            "agenttrace-statusline-stats-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("statusline.jsonl");
        let capture = |payload: Value| {
            serde_json::to_string(&CapturedStatusline {
                captured_at: 1,
                payload,
            })
            .unwrap()
        };
        let lines = [
            capture(serde_json::json!({"session_id": "a", "prompt_id": "pid-1"})),
            capture(serde_json::json!({"session_id": "b"})),
            capture(serde_json::json!("not an object")),
            capture(serde_json::json!({"session_id": "c", "prompt_id": 42})),
            // A torn tail line from a crashed append.
            "{\"captured_at\": 2, \"payl".to_string(),
        ];
        fs::write(&path, lines.join("\n") + "\n").expect("write journal");
        let stats = statusline_journal_stats(&path);
        assert_eq!(stats.lines, 5);
        assert_eq!(stats.captures_with_prompt_id, 1);
        assert_eq!(stats.captures_pre_first_input, 1);
        assert_eq!(stats.captures_unknown_payload, 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn correlated_prompt_ids_joins_and_tolerates_unknown_shapes() {
        // rm-708: the capture-side half of the statusline<->OTel
        // prompt correlation. Only ids present in both lanes join;
        // unknown OTLP shapes correlate nothing instead of erroring.
        let capture = |prompt_id: Option<&str>| CapturedStatusline {
            captured_at: 1,
            payload: match prompt_id {
                Some(id) => serde_json::json!({"session_id": "a", "prompt_id": id}),
                None => serde_json::json!({"session_id": "b"}),
            },
        };
        let captures = vec![
            capture(Some("pid-1")),
            capture(Some("pid-2")),
            capture(None),
        ];
        let otel_doc = serde_json::json!({
            "resourceSpans": [
                {
                    "scopeSpans": [
                        {
                            "spans": [
                                {
                                    "attributes": [
                                        {"key": "prompt.id", "value": {"stringValue": "pid-1"}},
                                        {"key": "other", "value": {"stringValue": "pid-2"}}
                                    ]
                                },
                                {"name": "no-attributes"}
                            ]
                        }
                    ]
                },
                {"resourceSpans sibling without scopeSpans": true}
            ]
        });
        assert_eq!(
            correlated_prompt_ids(&captures, &otel_doc),
            vec!["pid-1".to_string()]
        );
        // Distinct ids on the otel side alone never join.
        let disjoint = serde_json::json!({
            "resourceSpans": [{
                "scopeSpans": [{
                    "spans": [{
                        "attributes": [
                            {"key": "prompt.id", "value": {"stringValue": "pid-3"}}
                        ]
                    }]
                }]
            }]
        });
        assert!(correlated_prompt_ids(&captures, &disjoint).is_empty());
        // Unknown shapes: empty doc, missing arrays, non-string values.
        assert!(correlated_prompt_ids(&captures, &serde_json::json!({})).is_empty());
        assert!(
            correlated_prompt_ids(&captures, &serde_json::json!({"resourceSpans": "oops"}))
                .is_empty()
        );
        let non_string = serde_json::json!({
            "resourceSpans": [{
                "scopeSpans": [{
                    "spans": [{
                        "attributes": [
                            {"key": "prompt.id", "value": {"intValue": 7}}
                        ]
                    }]
                }]
            }]
        });
        assert!(correlated_prompt_ids(&captures, &non_string).is_empty());
        // No prompts captured at all: nothing to join.
        let bare = vec![capture(None)];
        assert!(correlated_prompt_ids(&bare, &otel_doc).is_empty());
    }

    #[test]
    fn statusline_report_sanitizes_prompt_id_render() {
        // rm-034 convention extended to the new arms: the prompt id is
        // a journal payload string, so the text lane prints it
        // sanitized while the journal keeps what it saw.
        let hostile = vec![CapturedStatusline {
            captured_at: 1_000,
            payload: serde_json::json!({
                "session_id": "sess\u{1b}[2Jhostile",
                "prompt_id": "pid\u{1b}[31mevil",
                "context_window": {"used_percentage": 50.0},
                "rate_limits": {"spend_limit": {
                    "used_usd": 62.8,
                    "limit_usd": 500.0,
                    "period": "monthly\u{1b}]52;evil"
                }}
            }),
        }];
        let insights = statusline_insights(&hostile);
        // rm-917: pinned `now` — same determinism as above; the
        // fixture capture (epoch 1_000) is excluded from the window.
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 10, 8, 12, 0, 0).unwrap();
        let text = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 1,
                bytes: 100,
                retained_max_bytes: 1024,
                capped_away_bytes: 0,
                read_truncated: false,
                captures_with_prompt_id: 1,
                captures_pre_first_input: 0,
                captures_unknown_payload: 0,
                unparsable_lines: 0,
            },
            &insights,
            None,
            &statusline_budget_series(&hostile, 7, now),
        );
        assert!(!text.contains('\u{1b}'), "no raw control bytes: {text}");
        assert!(text.contains("Latest prompt_id per session:"), "{text}");
        assert!(text.contains("sess\u{FFFD}[2Jhostile"), "{text}");
        assert!(text.contains("pid\u{FFFD}[31mevil"), "{text}");
        // rm-034 rule (review fix, 537105b1 F1): the spend-limit
        // period is a journal payload string too — the append arm
        // (beside the USD pair) renders it sanitized …
        assert!(text.contains("monthly\u{FFFD}]52;evil"), "{text}");

        // … and so does the period-alone arm when the gateway sent no
        // USD fields at all.
        let period_only = vec![CapturedStatusline {
            captured_at: 1_000,
            payload: serde_json::json!({
                "session_id": "sess",
                "rate_limits": {"spend_limit": {"period": "weekly\u{1b}[31mevil"}}
            }),
        }];
        // rm-917: pinned `now`, same as the append arm above (the
        // TimeZone import above this fn is reused).
        let now = Utc.with_ymd_and_hms(2026, 10, 8, 12, 0, 0).unwrap();
        let period_text = render_statusline_report_text(
            &StatuslineJournalStats {
                path: "journal".to_string(),
                exists: true,
                lines: 1,
                bytes: 100,
                retained_max_bytes: 1024,
                capped_away_bytes: 0,
                read_truncated: false,
                captures_with_prompt_id: 0,
                captures_pre_first_input: 1,
                captures_unknown_payload: 0,
                unparsable_lines: 0,
            },
            &statusline_insights(&period_only),
            None,
            &statusline_budget_series(&period_only, 7, now),
        );
        assert!(
            !period_text.contains('\u{1b}'),
            "no raw control bytes: {period_text}"
        );
        assert!(
            period_text.contains("weekly\u{FFFD}[31mevil"),
            "{period_text}"
        );
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
    fn out_of_order_journal_picks_latest_state_by_captured_at() {
        // rm-955: "latest observed state" used to be picked by journal
        // position while the rm-921 crossing probes sorted by
        // captured_at — so the exact out-of-order journal those probes
        // defend (clock skew, an older journal merged in) reported a
        // stale-by-clock window as current. The pick now ranks by
        // (captured_at, journal position), the crossings discipline:
        // here the journal's LAST capture carries the OLDER clock, and
        // the fresh-clock capture must still win both windows.
        let mut stale_clock_last_position = capture_at(FIXTURE, 1_760_000_100);
        stale_clock_last_position.payload["rate_limits"]["five_hour"]["used_percentage"] =
            serde_json::json!(41.0);
        stale_clock_last_position.payload["rate_limits"]["seven_day"]["used_percentage"] =
            serde_json::json!(21.0);
        let fresh_clock_first_position = capture_at(FIXTURE, 1_760_090_000);
        let insights =
            statusline_insights(&[fresh_clock_first_position, stale_clock_last_position]);
        assert_eq!(
            insights.five_hour.and_then(|state| state.used_percentage),
            Some(84.0),
            "the greatest captured_at wins five_hour regardless of journal position"
        );
        assert_eq!(
            insights.seven_day.and_then(|state| state.used_percentage),
            Some(46.0),
            "the greatest captured_at wins seven_day regardless of journal position"
        );
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

    /// rm-921 fixture: one capture carrying exactly the given window
    /// states (used_percentage, resets_at), so the sweep's pick and
    /// order semantics are pinned without leaning on the shared
    /// FIXTURE's fields.
    fn window_capture(
        captured_at: i64,
        five_hour: Option<(f64, Option<i64>)>,
        seven_day: Option<(f64, Option<i64>)>,
    ) -> CapturedStatusline {
        let mut rate_limits = serde_json::Map::new();
        for (window, state) in [("five_hour", five_hour), ("seven_day", seven_day)] {
            if let Some((used_percentage, resets_at)) = state {
                rate_limits.insert(
                    window.to_string(),
                    serde_json::json!({
                        "used_percentage": used_percentage,
                        "resets_at": resets_at,
                    }),
                );
            }
        }
        CapturedStatusline {
            captured_at,
            payload: serde_json::json!({
                "session_id": "session",
                "model": {"id": "claude-opus-4-5", "display_name": "Opus 4.5"},
                "cost": {"total_cost_usd": 1.0},
                "rate_limits": rate_limits,
            }),
        }
    }

    #[test]
    fn crossings_pick_observations_by_captured_at_not_array_order() {
        // rm-921 (F5): journal order is not captured_at order — clock
        // skew, or an older journal merged in. The boundary pick used
        // to take the first/last JOURNAL-POSITION match (next()/
        // next_back() over the array filter), so this journal reported
        // the stale t=50 observation before the reset (20%) and the
        // t=300 one after it (7%) — the pristine-binary PoC pinned
        // exactly those wrong numbers. The pick is by captured_at
        // now: greatest strictly before, least at-or-after.
        let captures = [
            window_capture(100, Some((10.0, Some(150))), None),
            window_capture(50, Some((20.0, None)), None),
            window_capture(300, Some((7.0, None)), None),
            window_capture(200, Some((5.0, None)), None),
        ];
        let insights = statusline_insights(&captures);
        assert_eq!(insights.limit_crossings.len(), 1);
        let crossing = &insights.limit_crossings[0];
        assert_eq!(crossing.window, "five_hour");
        assert_eq!(crossing.resets_at, 150);
        assert_eq!(crossing.used_percentage_before, Some(10.0));
        assert_eq!(crossing.used_percentage_after, Some(5.0));
    }

    #[test]
    fn crossings_output_order_is_window_then_boundary_ascending() {
        // rm-921: the sorted sweep must keep the rendered order the
        // old array probe produced on an in-order journal — windows in
        // [five_hour, seven_day] order, boundaries ascending — so
        // governance reports stay byte-identical for benign journals
        // (the no-boundary arms prove nothing: 3000 is dropped with no
        // after-side, 2500 the same).
        let captures = [
            window_capture(100, Some((10.0, Some(1000))), Some((50.0, Some(1500)))),
            window_capture(200, Some((20.0, Some(1000))), Some((60.0, Some(1500)))),
            window_capture(1100, Some((5.0, Some(2000))), Some((40.0, Some(1500)))),
            window_capture(2100, Some((8.0, Some(3000))), Some((30.0, Some(2500)))),
        ];
        let insights = statusline_insights(&captures);
        let seen: Vec<(&str, i64, Option<f64>, Option<f64>)> = insights
            .limit_crossings
            .iter()
            .map(|crossing| {
                (
                    crossing.window.as_str(),
                    crossing.resets_at,
                    crossing.used_percentage_before,
                    crossing.used_percentage_after,
                )
            })
            .collect();
        assert_eq!(
            seen,
            vec![
                ("five_hour", 1000, Some(20.0), Some(5.0)),
                ("five_hour", 2000, Some(5.0), Some(8.0)),
                ("seven_day", 1500, Some(40.0), Some(30.0)),
            ]
        );
    }

    #[test]
    fn crossings_on_sparse_journals_stay_linear() {
        // rm-921 (F2): a journal whose captures carry resets_at but no
        // used_percentage made every boundary probe re-parse every
        // capture's window state — n·k serde round-trips, measured
        // quadratic (the pristine-binary PoC: 0.05 s → 8.3 s at
        // n=250→2000 in RELEASE; the ceiling case is a 10 MiB journal,
        // minutes-scale). The sweep parses each window once and
        // binary-searches; 4000 distinct boundaries complete in
        // milliseconds where the quadratic shape needs ~35 s in
        // release. The bound is wall time with a wide margin.
        let captures: Vec<CapturedStatusline> = (0..4000)
            .map(|i| {
                let at = 1_700_000_000 + i * 60;
                CapturedStatusline {
                    captured_at: at,
                    payload: serde_json::json!({
                        "model": {"display_name": "Sonnet"},
                        "rate_limits": {
                            "five_hour": {"resets_at": at},
                            "seven_day": {"resets_at": at},
                        },
                    }),
                }
            })
            .collect();
        let started = std::time::Instant::now();
        let insights = statusline_insights(&captures);
        let elapsed = started.elapsed();
        assert_eq!(insights.captures, 4000);
        // 4000 distinct boundaries, no usable observations: no
        // crossing is evidenced, but every boundary was probed.
        assert!(
            insights.limit_crossings.is_empty(),
            "sparse shapes evidence no crossing"
        );
        assert!(
            elapsed.as_secs() < 10,
            "sparse-journal aggregation stays linear-scale: took {elapsed:?}"
        );
    }

    #[test]
    fn oversized_journal_reads_bounded_and_discloses_truncation() {
        // rm-922: the 10 MiB retention bound used to be enforced on
        // the append side only — a journal planted or grown by other
        // means was read and re-serialized whole by every consumer
        // (the pristine-binary PoC disclosed lines 130000 / bytes
        // 10938890 against this exact shape). The read keeps the
        // newest STATUSLINE_CAPTURE_MAX_BYTES now, and the stats lane
        // carries the flag.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-oversize-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        let mut raw = String::new();
        let mut seq = 0;
        while raw.len() <= STATUSLINE_CAPTURE_MAX_BYTES as usize {
            raw.push_str(&format!(
                "{{\"captured_at\":1000,\"payload\":{{\"seq\":{seq},\"filler\":\"{}\"}}}}\n",
                "x".repeat(96)
            ));
            seq += 1;
        }
        let on_disk = raw.len() as u64;
        fs::write(&journal, &raw).expect("write oversized journal");
        let read = read_statusline_capture_bounded(&journal);
        assert!(
            read.truncated,
            "an over-bound journal discloses its read bound"
        );
        assert!(read.text.len() as u64 <= STATUSLINE_CAPTURE_MAX_BYTES);
        assert!(read.text.starts_with('{'), "the torn first line is dropped");
        let stats = statusline_journal_stats(&journal);
        assert!(stats.read_truncated);
        assert!(
            stats.capped_away_bytes > 0,
            "the exact head cut is disclosed"
        );
        // Integration: the landed rm-898 rule (review 95d74221 F7)
        // keeps `bytes` naming the ON-DISK metadata size — a bounded
        // read must never shrink the reported footprint of the file it
        // truncated — superseding the candidate's read-length
        // accounting here; the cut itself is disclosed by
        // `capped_away_bytes` above and by the report text lane.
        assert_eq!(stats.bytes, on_disk);
        assert!(
            read.text.len() < on_disk as usize,
            "the head past the bound is not read"
        );
        let parsed = read_statusline_captures(&journal);
        assert_eq!(stats.lines, parsed.len(), "every kept line is valid here");
        // Only the newest tail is kept: the earliest sequences are gone.
        let first_kept = parsed[0].payload["seq"].as_i64().expect("seq");
        assert!(first_kept > 0, "oldest lines are the ones dropped");
        // The flag serializes only when set (benign byte-identity).
        let serialized = serde_json::to_string(&stats).expect("serialize stats");
        assert!(serialized.contains("\"read_truncated\":true"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_utf8_journal_still_discloses_and_retention_repairs() {
        // rm-922: one invalid UTF-8 byte used to fail the whole read —
        // every insight silently zeroed (the pristine-binary PoC:
        // captures 0, sessions 0, lines 0 against a journal holding
        // two valid lines), and because compaction's read failed the
        // same way retention never repaired. The read decodes lossily
        // now: valid lines disclose, the damaged line is skipped like
        // a torn one, and the next compaction rewrites the journal as
        // clean UTF-8.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-badbyte-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        let good1 = br#"{"captured_at":100,"payload":{"session_id":"one","model":{"display_name":"A"},"rate_limits":{"five_hour":{"used_percentage":10.0,"resets_at":200}}}}"#;
        let damaged = b"{\"captured_at\":200,\"payload\":{\"junk\":\"\xFF\xFE\"}}";
        let good2 = br#"{"captured_at":300,"payload":{"session_id":"two","model":{"display_name":"B"},"rate_limits":{"five_hour":{"used_percentage":30.0,"resets_at":400}}}}"#;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(good1);
        bytes.push(b'\n');
        bytes.extend_from_slice(damaged);
        bytes.push(b'\n');
        bytes.extend_from_slice(good2);
        bytes.push(b'\n');
        fs::write(&journal, &bytes).expect("write damaged journal");
        let captures = read_statusline_captures(&journal);
        // The damaged line's invalid bytes decode to U+FFFD INSIDE a
        // JSON string here, so it still parses and discloses — the
        // payload is nonsense but it is the journal's own nonsense,
        // not a silent hole. (A damaged line that breaks JSON
        // structure — the more common crash shape — is skipped like a
        // torn one.)
        assert_eq!(
            captures.len(),
            3,
            "every line discloses; the invalid bytes damage only their own line"
        );
        let insights = statusline_insights(&captures);
        assert_eq!(
            insights.sessions, 2,
            "the damaged line's payload carries no session_id"
        );
        let stats = statusline_journal_stats(&journal);
        assert!(!stats.read_truncated, "a small journal is not truncated");
        assert_eq!(stats.lines, 3, "the stats lane still counts every line");
        // Retention repairs instead of failing forever.
        compact_statusline_capture_under(&journal, 1024)
            .expect("compaction survives invalid UTF-8");
        let rewritten = fs::read(&journal).expect("journal rewritten");
        String::from_utf8(rewritten).expect("the rewritten journal is clean UTF-8");
        let repaired = read_statusline_captures(&journal);
        assert_eq!(repaired.len(), 3, "every line survived the repair");
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
    fn bad_byte_journal_recovers_retention_through_append() {
        // rm-922, the wedge leg: an over-bound journal carrying a
        // damaged line made append_statusline_capture fail (its
        // compaction read was the same bare read_to_string) — the
        // host tee would error on every prompt while the journal
        // grew past the ceiling. The append path rides the bounded
        // lossy read now: the append succeeds, compaction re-trims
        // under the bound, and the rewritten file is clean UTF-8.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-statusline-appendfix-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        // Point the environment at the temp root before any append
        // (the default path is the user's real journal), under the
        // shared env lock so sibling tests cannot re-point it.
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let journal = root.join("statusline.jsonl");
        let mut bytes = Vec::new();
        while bytes.len() <= STATUSLINE_CAPTURE_MAX_BYTES as usize {
            bytes.extend_from_slice(
                format!(
                    "{{\"captured_at\":1000,\"payload\":{{\"filler\":\"{}\"}}}}\n",
                    "x".repeat(96)
                )
                .as_bytes(),
            );
        }
        // A damaged line NEAR THE NEWEST END survives compaction, so
        // the rewrite's lossy repair is what makes the file valid.
        bytes.extend_from_slice(b"{\"captured_at\":2000,\"payload\":{\"junk\":\"\xFF\xFE\"}}\n");
        fs::write(&journal, &bytes).expect("write oversized damaged journal");
        let payload: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        append_statusline_capture(&payload).expect("append survives a damaged oversized journal");
        let rewritten = fs::read(&journal).expect("journal rewritten");
        String::from_utf8(rewritten).expect("the compacted journal is clean UTF-8");
        let size = fs::metadata(&journal).expect("metadata").len();
        assert!(
            size <= STATUSLINE_CAPTURE_MAX_BYTES,
            "retention re-enforced: {size} bytes"
        );
        let stats = statusline_journal_stats(&journal);
        assert!(!stats.read_truncated, "under the bound again");
        let captures = read_statusline_captures(&journal);
        let appended = captures
            .last()
            .expect("the appended payload survived compaction");
        assert_eq!(
            appended.payload["session_id"].as_str(),
            Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890"),
            "the newest capture is the appended FIXTURE payload"
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
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

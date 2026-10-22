//! Zed agent-panel `threads.db` ingestion (rm-2370, run fcbdf5ba cycle 5).
//!
//! Zed stores one agent-panel "thread" per row of the `threads` table inside
//! a SQLite database at:
//!   - macOS:   `~/Library/Application Support/Zed/threads/threads.db`
//!   - Linux:   `$XDG_DATA_HOME|~/.local/share/zed/threads/threads.db`
//!   - Windows: `%LOCALAPPDATA%\Zed\threads\threads.db`
//! (schema shape cross-checked against Zed `crates/agent/src/db.rs`:
//! `DbThread` carries `cumulative_token_usage` and `request_token_usage`
//! as serialized `SerializedTokenUsage` JSON — zstd-blob or plain text —
//! keyed per user message).
//!
//! Honesty contract (the lane's acceptance core): `request_token_usage`
//! under-covers a thread (~3x on a real corpus) because it is keyed per
//! user message; `cumulative_token_usage` is the vendor's own last word.
//! We therefore emit one attributed call per request entry PLUS exactly
//! one disclosed remainder call per thread reconciling to the cumulative
//! total — never silently under-counting, never silently over-counting.
//! A thread whose cumulative total is BELOW its summed requests (vendor
//! counter reset / db rotation) reports zero remainder, not negative.

use crate::libdef::{Event, EventAttrs, JsonObject};
use anyhow::{anyhow, Context, Result};
use std::io::Write;

/// Zed writes `CumulativeTokenUsage` through `SerializedTokenUsage`, which
/// zstd-compresses the JSON when the feature is on and stores plain text
/// otherwise. Both shapes carry the same field names.
#[derive(Default, Clone, Copy)]
struct TokenUsage {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
}

impl TokenUsage {
    fn from_json(o: &JsonObject) -> Self {
        let g = |k: &str| -> u64 {
            o.get(k)
                .and_then(|v| v.as_u64())
                .or_else(|| o.get(k).and_then(|v| v.as_i64()).map(|v| v.max(0) as u64))
                .unwrap_or(0)
        };
        TokenUsage {
            input: g("input_tokens"),
            output: g("output_tokens"),
            cache_read: g("cache_read_tokens").max(g("cached_read_tokens")),
            cache_write: g("cache_write_tokens").max(g("cached_creation_tokens")),
        }
    }
    fn is_zero(&self) -> bool {
        self.input == 0 && self.output == 0 && self.cache_read == 0 && self.cache_write == 0
    }
    fn saturating_sub(self, other: TokenUsage) -> TokenUsage {
        TokenUsage {
            input: self.input.saturating_sub(other.input),
            output: self.output.saturating_sub(other.output),
            cache_read: self.cache_read.saturating_sub(other.cache_read),
            cache_write: self.cache_write.saturating_sub(other.cache_write),
        }
    }
}

fn decode_stored_json(raw: &[u8]) -> Option<JsonObject> {
    if let Ok(txt) = std::str::from_utf8(raw) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(txt) {
            return v.as_object().cloned();
        }
    }
    // zstd-compressed JSON blob (single frame, bounded decode)
    if raw.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) && raw.len() <= 4 << 20 {
        if let Some(txt) = crate::parser::zstd_decode_bounded(raw) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                return v.as_object().cloned();
            }
        }
    }
    None
}

/// Parse a `threads.db` byte stream into events for ONE agenttrace session
/// (the db is the session; each Zed thread's usage rides distinct
/// attributed-call keys). Returns `Ok(None)` when the bytes are a SQLite
/// file but carry no `threads` table (not our shape — let the normal
/// failure path disclose it) and `Err` on unreadable/currupt stores so
/// `--doctor` can report the parse failure honestly.
pub(crate) fn parse_zed_threads_db_bytes(
    raw: &[u8],
    session_name: &str,
    session_path: &str,
) -> Result<Option<Vec<Event>>> {
    use rusqlite::OpenFlags;

    // SQLite requires a seekable file; stage the bytes in a private temp
    // file so we never write into the user's store.
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "agenttrace-zed-threads-{}-{:x}.db",
        std::process::id(),
        std::hash::Hasher::finish(&mut std::collections::hash_map::DefaultHasher::new())
    ));
    let guard = TempPathGuard {
        path: tmp.clone(),
        remove: false,
    };
    {
        let mut f = std::fs::File::create(&tmp).with_context(|| "staging zed threads.db")?;
        f.write_all(raw)?;
    }
    let conn = rusqlite::Connection::open_with_flags(
        &tmp,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("opening staged copy of {}", session_path))?;

    let has_threads: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='threads'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    if !has_threads {
        return Ok(None);
    }

    let mut events: Vec<Event> = Vec::new();
    let mut stmt = conn
        .prepare("SELECT rowid, * FROM threads ORDER BY rowid")
        .with_context(|| "preparing threads scan")?;
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let idx = |want: &str| col_names.iter().position(|c| c == want);
    let (i_id, i_upd, i_cum, i_req, i_model) = (
        idx("rowid"),
        idx("updated_at"),
        idx("cumulative_token_usage"),
        idx("request_token_usage"),
        idx("serialized_language_model"),
    );
    let i_upd = match i_upd {
        Some(i) => i,
        None => anyhow::bail!("threads table has no updated_at column"),
    };
    let i_cum = match i_cum {
        Some(i) => i,
        None => anyhow::bail!("threads table has no cumulative_token_usage column"),
    };

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let thread_id = i_id
            .and_then(|i| row.get::<_, i64>(i).ok())
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown-row".into());
        let updated: f64 = row.get::<_, f64>(i_upd).unwrap_or(0.0);
        let ts_secs = if updated > 1.0e12 { (updated / 1000.0) as i64 } else { updated as i64 };

        let model = i_model
            .and_then(|i| row.get::<_, Option<String>>(i).ok().flatten())
            .and_then(|s| decode_stored_json(s.as_bytes()))
            .and_then(|o| o.get("name").and_then(|v| v.as_str()).map(str::to_string));

        let cum_raw: Option<String> = row.get::<_, Option<String>>(i_cum).ok().flatten();
        let cum = cum_raw
            .as_deref()
            .and_then(decode_stored_json)
            .map(|o| TokenUsage::from_json(&o))
            .unwrap_or_default();

        let mut summed = TokenUsage::default();
        let mut calls = 0u32;
        if let Some(i_req) = i_req {
            if let Some(req_raw) = row.get::<_, Option<String>>(i_req).ok().flatten() {
                if let Some(obj) = decode_stored_json(req_raw.as_bytes()) {
                    // shape: { "msg-id": {input_tokens, ...}, ... } — any
                    // object-of-objects; read defensively, no schema guarantee.
                    for (k, v) in obj.iter() {
                        let per = v.as_object().map(TokenUsage::from_json).unwrap_or_default();
                        summed.input += per.input;
                        summed.output += per.output;
                        summed.cache_read += per.cache_read;
                        summed.cache_write += per.cache_write;
                        calls += 1;
                        if !per.is_zero() {
                            events.push(usage_event(
                                session_name,
                                session_path,
                                &format!("zed:{thread_id}:{k}"),
                                ts_secs,
                                per,
                                model.as_deref(),
                                false,
                            ));
                        }
                    }
                }
            }
        }
        let remainder = cum.saturating_sub(summed);
        if !cum.is_zero() {
            events.push(usage_event(
                session_name,
                session_path,
                &format!("zed:{thread_id}:cumulative-remainder"),
                ts_secs,
                remainder,
                model.as_deref(),
                true,
            ));
        }
        if calls == 0 && cum.is_zero() {
            // thread with no usage at all — keep a zero marker so the
            // thread remains visible in the session's record.
            events.push(usage_event(
                session_name,
                session_path,
                &format!("zed:{thread_id}:empty"),
                ts_secs,
                TokenUsage::default(),
                model.as_deref(),
                false,
            ));
        }
    }
    drop(rows);
    drop(stmt);
    drop(conn);
    let _ = std::fs::remove_file(&tmp);
    std::mem::forget(guard);
    Ok(Some(events))
}

fn usage_event(
    session_name: &str,
    session_path: &str,
    key: &str,
    ts_secs: i64,
    u: TokenUsage,
    model: Option<&str>,
    is_remainder: bool,
) -> Event {
    let mut attrs = EventAttrs::default();
    attrs.call_id = Some(key.to_string());
    if let Some(m) = model {
        attrs.model = Some(m.to_string());
    }
    if is_remainder {
        attrs.labels = vec![format!("{key}:remainder-vs-cumulative")];
    }
    Event::usage(session_name, session_path, ts_secs, &u.input, &u.output, &u.cache_read, &u.cache_write, attrs)
}

struct TempPathGuard {
    path: std::path::PathBuf,
    remove: bool,
}
impl Drop for TempPathGuard {
    fn drop(&mut self) {
        if self.remove {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[allow(dead_code)]
fn _unused() {
    let _ = anyhow!("x");
}

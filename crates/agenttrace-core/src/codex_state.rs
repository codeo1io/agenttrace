//! Read-only ingest of the Codex CLI per-thread state databases
//! (`~/.codex/state_*.sqlite`) — rm-1986 (run 2b30a694 cycle 3, batch
//! "codex state-truth ingest + changelog duplicate truth").
//!
//! Rollout JSONL files remain the CANONICAL codex session source; this lane
//! is an additive cross-check authority, the rm-600 verification class:
//!
//! * `threads.tokens_used` is codex's own per-thread token total. Comparing
//!   it against our rollout-derived totals yields a DISAGREEMENT ORACLE —
//!   a silent parser regression or a format change that quietly drops usage
//!   now surfaces as a disclosed disagreement instead of drifting totals.
//! * `threads.source` + `thread_spawn_edges` expose subagent attribution
//!   (codex names subagent threads `subagent:<kind>`), disclosed as counts.
//!
//! Safety contract: the databases are opened with `mode=ro&immutable=1`
//! URIs — STRICTLY sidecar-safe (the contract test fingerprints db, `-wal`
//! and `-shm` before/after; a plain `mode=ro` open provably writes the
//! `-shm`). Trade-off, disclosed in every scan's note: the view is the
//! last-checkpointed state, never the hot `-wal`. Versioned file names
//! (`state_5.sqlite`, `state_6.sqlite`, …) are discovered by glob, never
//! hardcoded, and the `threads` schema is probed per file so a codex
//! migration cannot turn the lane into a silent zero. A present-but-
//! unreadable file is DISCLOSED (`files_failed` + sample), never dropped.

use rusqlite::OpenFlags;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Rollout-derived per-session total handed to the oracle. Decoupled from
/// `Session` so the contract tests (and any future caller) need no full
/// session graph.
#[derive(Debug, Clone)]
pub struct CodexRolloutTotal {
    /// Absolute path of the rollout file the session was parsed from.
    pub path: PathBuf,
    /// `tokens_input + tokens_output + tokens_reasoning` for the session
    /// (cache reads/writes excluded — see `DERIVED_TOKENS_FORMULA`).
    pub derived_tokens: i64,
}

/// The formula the oracle compares against, verbatim for disclosure.
pub const DERIVED_TOKENS_FORMULA: &str = "tokens_input + tokens_output + tokens_reasoning";

/// Absolute disagreement tolerance: `|sqlite − derived|` up to this many
/// tokens still counts as agreement (rounding and per-event accounting
/// order differences between codex and the rollout walk).
pub const DISAGREEMENT_ABS_TOLERANCE: i64 = 100;

/// Relative disagreement tolerance (percent of the derived total).
pub const DISAGREEMENT_REL_TOLERANCE_PCT: f64 = 2.0;

/// The doctor/`-f json` disclosure section. `Default` is the absent state:
/// a `files_found == 0` scan serializes to nothing (the zero-count-clean
/// convention — output is byte-identical to pre-change on hosts without
/// codex state files).
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct CodexStateScan {
    /// Codex home that was probed (`CODEX_HOME` or `~/.codex`).
    pub home: String,
    /// `state_*.sqlite` files discovered at the home root.
    pub files_found: usize,
    /// Files successfully opened read-only and probed.
    pub files_parsed: usize,
    /// Present-but-unreadable files (disclosed, never dropped).
    pub files_failed: usize,
    /// Why a failed file failed (first few).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_samples: Vec<String>,
    /// Rows across the probed `threads` tables.
    pub threads_total: usize,
    /// Threads whose `rollout_path` matched one of our parsed sessions.
    pub threads_matched: usize,
    /// Threads with no matching parsed session (disclosed as a count; the
    /// rollout may be archived, deleted, or filtered by budget/size).
    pub threads_unmatched: usize,
    /// Rows in `thread_spawn_edges` tables, when present.
    pub spawn_edges_total: usize,
    /// Threads whose `source` names a subagent (`subagent:*`).
    pub subagent_threads: usize,
    /// Matched threads whose `tokens_used` disagrees with the derived total
    /// beyond the tolerance band. THE oracle counter.
    pub tokens_used_disagreements: usize,
    /// First few disagreements, human-readable (`rollout: sqlite=N vs derived=M (diff=+D)`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disagreement_samples: Vec<String>,
    /// What the lane is and how to read the counters (disclosure contract).
    pub note: String,
}

impl CodexStateScan {
    /// Whether this scan found anything to report.
    pub fn is_absent(&self) -> bool {
        self.files_found == 0
    }
}

fn absent() -> CodexStateScan {
    CodexStateScan::default()
}

/// Discover `state_*.sqlite` files at the codex home root. Sidecar suffixes
/// (`-wal`, `-shm`, `-journal`) are companions of a main file, not files of
/// their own; anything else that fails the pattern is ignored.
pub fn discover_state_files(codex_home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(codex_home) {
        Ok(entries) => entries,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("state_") || !name.ends_with(".sqlite") {
            continue;
        }
        if name.ends_with("-wal.sqlite") || name.ends_with("-shm.sqlite") {
            continue; // defensive: sidecars are never named this way, but stay strict
        }
        out.push(entry.path());
    }
    out.sort();
    out
}

struct ThreadRow {
    rollout_path: Option<String>,
    tokens_used: Option<i64>,
    source: Option<String>,
}

fn open_read_only(path: &Path) -> rusqlite::Result<rusqlite::Connection> {
    // STRICTLY sidecar-safe opens (the rm-1986 WAL byte-check gate):
    // plain `mode=ro` still writes the `-shm` coordination file, which the
    // contract test proved by fingerprint. `immutable=1` is the documented
    // no-file-changes mode: SQLite reads the main file's last-checkpointed
    // state and never touches `-wal`/`-shm`. Trade-off (disclosed in the
    // scan note): on a live host mid-write, the view is the last checkpoint,
    // never the hot `-wal` — for a disclosed cross-check lane that is the
    // right side of correctness-vs-freshness.
    //
    // URI form is required; percent-encode the characters SQLite treats
    // specially in URIs (? # %) so odd paths stay openable.
    let encoded = path
        .to_string_lossy()
        .replace('%', "%25")
        .replace('?', "%3f")
        .replace('#', "%23");
    let uri = format!("file:{encoded}?mode=ro&immutable=1");
    rusqlite::Connection::open_with_flags(
        uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
}

fn table_columns(conn: &rusqlite::Connection, table: &str) -> Vec<String> {
    // Pragma, not string interpolation of user data: `table` is a literal here.
    let mut stmt = match conn.prepare(&format!("PRAGMA table_info({table})")) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |row| row.get::<_, String>(1));
    match rows {
        Ok(rows) => rows.flatten().collect(),
        Err(_) => Vec::new(),
    }
}

fn read_threads(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<ThreadRow>> {
    let cols = table_columns(conn, "threads");
    if cols.is_empty() {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let rollout_path = cols.iter().any(|c| c == "rollout_path");
    let tokens_used = cols.iter().any(|c| c == "tokens_used");
    let source = cols.iter().any(|c| c == "source");
    let mut stmt = conn.prepare("SELECT rollout_path, tokens_used, source FROM threads")?;
    let rows = stmt.query_map([], |row| {
        Ok(ThreadRow {
            rollout_path: if rollout_path {
                row.get::<_, Option<String>>(0).ok().flatten()
            } else {
                None
            },
            tokens_used: if tokens_used {
                row.get::<_, Option<i64>>(1).ok().flatten()
            } else {
                None
            },
            source: if source {
                row.get::<_, Option<String>>(2).ok().flatten()
            } else {
                None
            },
        })
    })?;
    Ok(rows.flatten().collect())
}

fn count_table(conn: &rusqlite::Connection, table: &str) -> Option<usize> {
    let cols = table_columns(conn, table);
    if cols.is_empty() {
        return None;
    }
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get::<_, i64>(0)
    })
    .ok()
    .map(|n| n as usize)
}

/// Whether `|sqlite − derived|` is within the tolerance band.
pub fn tokens_agree(sqlite_tokens: i64, derived_tokens: i64) -> bool {
    let diff = (sqlite_tokens - derived_tokens).abs();
    let rel = (DISAGREEMENT_REL_TOLERANCE_PCT / 100.0) * derived_tokens.max(0) as f64;
    let band = DISAGREEMENT_ABS_TOLERANCE.max(rel as i64);
    diff <= band
}

/// Scan the codex home's state databases against the rollout-derived totals.
///
/// Ordering of matched-vs-unmatched uses exact `rollout_path` equality first,
/// then file-name equality (codex stores absolute paths; mount points and
/// archive moves make string equality too brittle alone). Each parsed session
/// matches at most one thread row.
pub fn scan_codex_state(codex_home: &Path, rollouts: &[CodexRolloutTotal]) -> CodexStateScan {
    let mut scan = absent();
    scan.home = codex_home.display().to_string();
    scan.note = format!(
        "codex per-thread state cross-check (rm-1986): threads.tokens_used vs rollout-derived \
         totals ({DERIVED_TOKENS_FORMULA}); rollouts stay canonical; tolerance band \
         max({DISAGREEMENT_ABS_TOLERANCE} tokens, {DISAGREEMENT_REL_TOLERANCE_PCT}% of derived); reads the \
         last-checkpointed state via immutable=1 opens — hot -wal/-shm are never read or written"
    );

    let files = discover_state_files(codex_home);
    scan.files_found = files.len();
    if files.is_empty() {
        return scan;
    }

    let by_path: HashMap<String, &CodexRolloutTotal> = rollouts
        .iter()
        .map(|r| (r.path.display().to_string(), r))
        .collect();
    let by_name: HashMap<String, Vec<&CodexRolloutTotal>> = {
        let mut m: HashMap<String, Vec<&CodexRolloutTotal>> = HashMap::new();
        for r in rollouts {
            if let Some(name) = r.path.file_name() {
                m.entry(name.to_string_lossy().into_owned())
                    .or_default()
                    .push(r);
            }
        }
        m
    };
    let mut claimed: HashMap<String, ()> = HashMap::new();

    for file in &files {
        let conn = match open_read_only(file) {
            Ok(conn) => conn,
            Err(err) => {
                scan.files_failed += 1;
                if scan.failure_samples.len() < 5 {
                    scan.failure_samples.push(format!(
                        "{}: unreadable ({err})",
                        file.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    ));
                }
                continue;
            }
        };
        let threads = match read_threads(&conn) {
            Ok(threads) => threads,
            Err(_) => {
                scan.files_failed += 1;
                if scan.failure_samples.len() < 5 {
                    scan.failure_samples
                        .push(format!("{}: no readable threads table", file.display()));
                }
                continue;
            }
        };
        scan.files_parsed += 1;
        scan.threads_total += threads.len();
        scan.spawn_edges_total += count_table(&conn, "thread_spawn_edges").unwrap_or(0);

        for thread in threads {
            if let Some(source) = &thread.source {
                if source.starts_with("subagent") {
                    scan.subagent_threads += 1;
                }
            }
            let rollout_path = match &thread.rollout_path {
                Some(p) if !p.is_empty() => p,
                _ => continue,
            };
            let matched = by_path.get(rollout_path).copied().or_else(|| {
                Path::new(rollout_path)
                    .file_name()
                    .and_then(|n| by_name.get(n.to_string_lossy().as_ref()))
                    .and_then(|v| {
                        v.iter()
                            .find(|r| !claimed.contains_key(&r.path.display().to_string()))
                            .copied()
                    })
            });
            let rollout = match matched {
                Some(r) if !claimed.contains_key(&r.path.display().to_string()) => r,
                _ => {
                    scan.threads_unmatched += 1;
                    continue;
                }
            };
            claimed.insert(rollout.path.display().to_string(), ());
            scan.threads_matched += 1;
            let (sqlite_tokens, derived) = match (thread.tokens_used, rollout.derived_tokens) {
                (Some(s), d) => (s, d),
                (None, _) => continue,
            };
            if !tokens_agree(sqlite_tokens, derived) {
                scan.tokens_used_disagreements += 1;
                if scan.disagreement_samples.len() < 5 {
                    let name = rollout
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| rollout_path.clone());
                    scan.disagreement_samples.push(format!(
                        "{name}: sqlite tokens_used={sqlite_tokens} vs derived={derived} (diff={:+})",
                        sqlite_tokens - derived
                    ));
                }
            }
        }
    }
    scan
}

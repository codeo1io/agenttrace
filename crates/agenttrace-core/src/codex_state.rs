//! rm-1504 — Codex state-store family projection.
//!
//! The Codex CLI keeps per-conversation state in a family of SQLite
//! databases under `CODEX_HOME` (default `~/.codex`): `state_N.sqlite`
//! (threads — the token accounting the rollout transcripts do not
//! carry), `goals_N.sqlite` (per-thread goal budgets), and the
//! `thread_history_N` / `memories_N` / `queue_N` / `logs_N` sidecars.
//! This module is a strictly read-only census over that family: it
//! never parses them as sessions (the transcript lane owns sessions),
//! it discloses what exists, whether each database matches the
//! observed schema, and the per-thread `tokens_used` totals plus
//! per-goal budget pressure as additive `--doctor` facts.
//!
//! Truthfulness contract:
//! - every `*.sqlite` under the codex home is reported — unknown
//!   classes appear as `unrecognized`, never silently skipped;
//! - a database that cannot be opened or read is reported with its
//!   error (found + failed), never as a silent zero;
//! - schema drift is disclosed both ways: missing core tables (the
//!   projection cannot run) and extra tables (a newer upstream
//!   migration state — codex-rs `state` migrations, e.g.
//!   `0034_drop_thread_goals`, evolve this family);
//! - the databases themselves are never written: connections are
//!   opened `READ_ONLY` and the main-file bytes stay untouched
//!   (md5-pinned in tests). Reading a WAL-mode member coordinates
//!   through SQLite's WAL-index, which may leave an `-shm` and an
//!   empty `-wal` beside it — a read-coordination artifact of the
//!   sqlite library, not a store write; the census never checkpoints
//!   or truncates a writer's non-empty `-wal`, and rollback-mode
//!   members provably get no sidecars at all.
//!
//! Class baselines were observed live on 2026-10-14 against codex
//! 0.147-era databases (and cross-checked against the upstream
//! `codex-rs/state` store, `sqlite.rs` @ e35a074d, which pins the
//! `state_5` / `goals_1` filenames); `state_6`+ simply reports a
//! higher version with its drift disclosed.

use rusqlite::OpenFlags;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many threads lead the rendered sample (by `tokens_used`).
const TOP_THREADS_SAMPLE: usize = 3;
/// Rendered titles are sanitized and hard-truncated to this many
/// characters — live threads carry titles thousands of characters
/// long, and the doctor must never relay raw corpus bytes.
const TITLE_RENDER_CHARS: usize = 60;
/// Per-database table list rendered with a row count, bounded so a
/// wide future schema cannot flood the report.
const MAX_COUNTED_TABLES: usize = 8;
/// Cap on stored per-source counts (the census keeps counts; the
/// render shows the leaders).
const MAX_SOURCE_ROWS: usize = 8;

/// Core tables whose absence disables that class's projection, plus
/// the informational baseline tables observed in the wild. Extras
/// beyond both sets are disclosed as newer-schema drift.
const CLASS_BASELINES: &[(&str, &[&str], &[&str])] = &[
    (
        "state",
        &["threads"],
        &[
            "backfill_state",
            "external_agent_config_imports",
            "remote_control_enrollments",
            "thread_dynamic_tools",
            "thread_sections",
            "thread_spawn_edges",
        ],
    ),
    ("goals", &["thread_goals"], &[]),
    (
        "thread_history",
        &["thread_items", "thread_turns"],
        &["projection_state"],
    ),
    ("memories", &["jobs", "stage1_outputs"], &[]),
    ("memories_v2", &[], &[]),
    ("queue", &["queued_items"], &[]),
    ("logs", &["logs"], &[]),
];

/// Whole-family census: every `*.sqlite` directly under the codex
/// home, with found/failed totals so the doctor never prints a
/// silent zero over discovered-but-unreadable databases (P12/P14).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CodexStateCensus {
    pub found: usize,
    pub failed: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dbs: Vec<CodexStateDbReport>,
}

impl CodexStateCensus {
    /// True when the family is absent (nothing to disclose, nothing
    /// to serialize — the JSON arm stays byte-identical for corpora
    /// without codex state stores).
    pub fn is_empty(&self) -> bool {
        self.dbs.is_empty()
    }
}

/// One database of the family.
#[derive(Debug, Clone, Serialize)]
pub struct CodexStateDbReport {
    pub file_name: String,
    /// `state`, `goals`, `thread_history`, `memories`, `memories_v2`,
    /// `queue`, `logs`, or `unrecognized` (still disclosed).
    pub class: String,
    /// Version suffix parsed from the file name (`state_5` → `5`).
    pub version: Option<u32>,
    /// Tables observed in `sqlite_master` (system tables excluded).
    pub tables: Vec<String>,
    /// Core tables the class baseline expects but the database lacks
    /// — the projection for that class is skipped and disclosed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_tables: Vec<String>,
    /// Tables beyond the class baseline — a newer upstream migration
    /// state, disclosed as informational drift.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra_tables: Vec<String>,
    /// Row counts for the core and informational tables present.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub row_counts: Vec<TableRowCount>,
    /// Present when the class is `state` and its `threads` core table
    /// is readable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<CodexThreadsCensus>,
    /// Present when the class is `goals` and its `thread_goals` core
    /// table is readable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goals: Option<CodexGoalsCensus>,
    /// Sanitized open/read error for discovered-but-unreadable files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One `(table, rows)` count.
#[derive(Debug, Clone, Serialize)]
pub struct TableRowCount {
    pub table: String,
    pub rows: u64,
}

/// Usage projection over `state_N.sqlite::threads`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CodexThreadsCensus {
    pub threads: u64,
    pub tokens_used_total: u64,
    /// Distinct `source` values with their thread counts, largest
    /// first.
    pub sources: Vec<SourceCount>,
    /// Largest threads by `tokens_used`, sanitized for rendering.
    pub top_threads: Vec<TopThreadByTokens>,
}

/// One `source` value with its thread count.
#[derive(Debug, Clone, Serialize)]
pub struct SourceCount {
    pub source: String,
    pub count: u64,
}

/// Sample thread leader by `tokens_used`.
#[derive(Debug, Clone, Serialize)]
pub struct TopThreadByTokens {
    pub tokens_used: u64,
    pub source: String,
    /// Sanitized, hard-truncated thread title.
    pub title: String,
}

/// Budget projection over `goals_N.sqlite::thread_goals`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CodexGoalsCensus {
    pub goals: u64,
    pub token_budget_total: u64,
    pub tokens_used_total: u64,
    /// Goals whose `tokens_used` exceeds their `token_budget`.
    pub over_budget: u64,
}

/// Census the family under the effective codex home: `$CODEX_HOME`
/// when set (the discovery-lane convention), else `~/.codex`.
pub fn codex_state_census() -> CodexStateCensus {
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".codex")));
    match home {
        Some(path) => codex_state_census_at(&path),
        None => CodexStateCensus::default(),
    }
}

/// Census the family under an explicit codex home (test core; also
/// the honest entry point for any future `--codex-home` override).
pub fn codex_state_census_at(codex_home: &Path) -> CodexStateCensus {
    let mut census = CodexStateCensus::default();
    let Ok(entries) = std::fs::read_dir(codex_home) else {
        return census;
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "sqlite"))
        .collect();
    files.sort();
    for path in files {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let (class, version) = classify(&file_name);
        census
            .dbs
            .push(inspect_db(&path, &file_name, &class, version));
    }
    census.found = census.dbs.len();
    census.failed = census.dbs.iter().filter(|db| db.error.is_some()).count();
    census
}

/// Split `state_5.sqlite` into (`state`, `Some(5)`); anything without
/// a trailing `_N` version keeps `None`; an unknown class is still
/// censused (and disclosed) as `unrecognized`.
fn classify(file_name: &str) -> (String, Option<u32>) {
    let stem = file_name.strip_suffix(".sqlite").unwrap_or(file_name);
    match stem.rsplit_once('_') {
        Some((prefix, suffix))
            if suffix.chars().all(|c| c.is_ascii_digit()) && !suffix.is_empty() =>
        {
            let class = if CLASS_BASELINES.iter().any(|(c, _, _)| *c == prefix) {
                prefix.to_string()
            } else {
                "unrecognized".to_string()
            };
            match suffix.parse::<u32>() {
                Ok(version) => (class, Some(version)),
                Err(_) => (class, None),
            }
        }
        _ => ("unrecognized".to_string(), None),
    }
}

fn inspect_db(
    path: &Path,
    file_name: &str,
    class: &str,
    version: Option<u32>,
) -> CodexStateDbReport {
    let mut report = CodexStateDbReport {
        file_name: file_name.to_string(),
        class: class.to_string(),
        version,
        tables: Vec::new(),
        missing_tables: Vec::new(),
        extra_tables: Vec::new(),
        row_counts: Vec::new(),
        threads: None,
        goals: None,
        error: None,
    };
    let conn = match rusqlite::Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(conn) => conn,
        Err(err) => {
            report.error = Some(sanitize_error(&err));
            return report;
        }
    };
    let mut tables: Vec<String> = match conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        }) {
        Ok(names) => names,
        Err(err) => {
            // A file that is not a database only fails here, on first
            // read — sqlite defers the header check past open().
            report.error = Some(sanitize_error(&err));
            return report;
        }
    };
    let (core, informational): (&[&str], &[&str]) = CLASS_BASELINES
        .iter()
        .find(|(c, _, _)| *c == class)
        .map(|(_, core, info)| (*core, *info))
        .unwrap_or((&[], &[]));
    for table in core.iter().filter(|t| !tables.contains(&t.to_string())) {
        report.missing_tables.push(table.to_string());
    }
    let mut known: Vec<&str> = core.iter().chain(informational.iter()).copied().collect();
    tables.retain(|observed| {
        if let Some(position) = known.iter().position(|k| k == observed) {
            known.swap_remove(position);
            true
        } else {
            report.extra_tables.push(observed.clone());
            false
        }
    });
    for table in core.iter().chain(informational.iter()) {
        if tables.iter().any(|t| t == table) && report.row_counts.len() < MAX_COUNTED_TABLES {
            let rows = conn
                .query_row(&format!("SELECT COUNT(*) FROM \"{}\"", table), [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap_or(-1);
            report.row_counts.push(TableRowCount {
                table: table.to_string(),
                rows: rows.max(0) as u64,
            });
        }
    }
    if class == "state" && tables.iter().any(|t| t == "threads") {
        report.threads = Some(threads_census(&conn));
    }
    if class == "goals" && tables.iter().any(|t| t == "thread_goals") {
        report.goals = Some(goals_census(&conn));
    }
    report.tables = tables;
    report
}

fn threads_census(conn: &rusqlite::Connection) -> CodexThreadsCensus {
    let mut census = CodexThreadsCensus::default();
    let mut sources: BTreeMap<String, u64> = BTreeMap::new();
    let mut tops: Vec<TopThreadByTokens> = Vec::new();
    let query =
        "SELECT COALESCE(tokens_used, 0), COALESCE(source, ''), COALESCE(title, '') FROM threads";
    let Ok(mut stmt) = conn.prepare(query) else {
        return census;
    };
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0).unwrap_or(0),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
        ))
    });
    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (tokens_used, source, title) = row;
            census.threads += 1;
            let tokens_used = tokens_used.max(0) as u64;
            census.tokens_used_total += tokens_used;
            *sources.entry(source.clone()).or_insert(0) += 1;
            tops.push(TopThreadByTokens {
                tokens_used,
                source: crate::statusline::sanitize_line_segment(&source),
                title: truncate_sanitized(&crate::statusline::sanitize_line_segment(&title)),
            });
        }
    }
    let mut sources: Vec<SourceCount> = sources
        .into_iter()
        .map(|(source, count)| SourceCount { source, count })
        .collect();
    sources.sort_by(|a, b| b.count.cmp(&a.count).then(a.source.cmp(&b.source)));
    sources.truncate(MAX_SOURCE_ROWS);
    census.sources = sources;
    tops.sort_by_key(|top| std::cmp::Reverse(top.tokens_used));
    tops.truncate(TOP_THREADS_SAMPLE);
    census.top_threads = tops;
    census
}

fn goals_census(conn: &rusqlite::Connection) -> CodexGoalsCensus {
    let mut census = CodexGoalsCensus::default();
    let Ok(mut stmt) = conn
        .prepare("SELECT COALESCE(token_budget, 0), COALESCE(tokens_used, 0) FROM thread_goals")
    else {
        return census;
    };
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0).unwrap_or(0),
            row.get::<_, i64>(1).unwrap_or(0),
        ))
    });
    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (budget, used) = row;
            census.goals += 1;
            census.token_budget_total += budget.max(0) as u64;
            let used = used.max(0) as u64;
            census.tokens_used_total += used;
            if used > budget.max(0) as u64 {
                census.over_budget += 1;
            }
        }
    }
    census
}

/// OS and sqlite error strings can embed raw paths — the doctor
/// relays them sanitized and bounded.
fn sanitize_error(err: &rusqlite::Error) -> String {
    truncate_sanitized(&crate::statusline::sanitize_line_segment(&err.to_string()))
}

fn truncate_sanitized(sanitized: &str) -> String {
    if sanitized.chars().count() > TITLE_RENDER_CHARS {
        let cut: String = sanitized.chars().take(TITLE_RENDER_CHARS).collect();
        format!("{}…", cut)
    } else {
        sanitized.to_string()
    }
}

/// Render the census as additive `--doctor` lines. Empty for an
/// absent family (zero-delta: a corpus without the state stores
/// renders byte-identically to before rm-1504). Pure function —
/// `doctor_report_text` embeds it, tests bind it directly.
pub fn codex_state_render(census: &CodexStateCensus) -> String {
    let mut out = String::new();
    if census.dbs.is_empty() {
        return out;
    }
    out.push_str(&format!(
        "Codex state stores (rm-1504): {} found, {} unreadable\n",
        census.found, census.failed
    ));
    for db in &census.dbs {
        let version = db.version.map(|v| format!(" v{v}")).unwrap_or_default();
        if let Some(error) = &db.error {
            out.push_str(&format!(
                "  {} ({}{}): unreadable — {}\n",
                db.file_name, db.class, version, error
            ));
            continue;
        }
        let counts: Vec<String> = db
            .row_counts
            .iter()
            .map(|count| format!("{}={}", count.table, count.rows))
            .collect();
        let mut line = format!(
            "  {} ({}{}): tables [{}]",
            db.file_name,
            db.class,
            version,
            counts.join(" ")
        );
        if !db.missing_tables.is_empty() {
            line.push_str(&format!(" — missing {}", db.missing_tables.join(",")));
        }
        if !db.extra_tables.is_empty() {
            line.push_str(&format!(" — extra {}", db.extra_tables.join(",")));
        }
        out.push_str(&line);
        out.push('\n');
        if let Some(threads) = &db.threads {
            let sources: Vec<String> = threads
                .sources
                .iter()
                .map(|source| format!("{}×{}", source.source, source.count))
                .collect();
            out.push_str(&format!(
                "    threads {} · tokens_used {} · sources {}\n",
                threads.threads,
                threads.tokens_used_total,
                sources.join("/")
            ));
            for top in &threads.top_threads {
                out.push_str(&format!(
                    "    top {:>8} tokens · {} · \"{}\"\n",
                    top.tokens_used, top.source, top.title
                ));
            }
        }
        if let Some(goals) = &db.goals {
            out.push_str(&format!(
                "    goals {} · budget {} · used {} · over budget {}\n",
                goals.goals, goals.token_budget_total, goals.tokens_used_total, goals.over_budget
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_versions_and_unknowns() {
        assert_eq!(classify("state_5.sqlite"), ("state".to_string(), Some(5)));
        assert_eq!(
            classify("thread_history_1.sqlite"),
            ("thread_history".to_string(), Some(1))
        );
        assert_eq!(
            classify("memories_v2_1.sqlite"),
            ("memories_v2".to_string(), Some(1))
        );
        assert_eq!(
            classify("checkbook.sqlite"),
            ("unrecognized".to_string(), None)
        );
    }

    #[test]
    fn truncate_bounds_sanitized_titles() {
        let long = "A".repeat(200);
        let cut = truncate_sanitized(&long);
        assert_eq!(cut.chars().count(), TITLE_RENDER_CHARS + 1);
        assert!(cut.ends_with('…'));
        assert_eq!(truncate_sanitized("short"), "short");
    }
}

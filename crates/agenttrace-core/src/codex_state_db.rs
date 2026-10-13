//! Codex state-DB discovery consult (rm-1040).
//!
//! The Codex CLI writes a versioned SQLite index of every thread
//! (`~/.codex/state_5.sqlite` at the time of writing; the filename carries the
//! schema version) by default, alongside the rollout `.jsonl` files under
//! `~/.codex/sessions/` that this crate parses. The rollout walk stays the
//! authoritative source for parsing and cost accounting; this module only
//! READS the state DB to cross-check listing coverage and surface a
//! disclosure when the two disagree.
//!
//! Invariants (acceptance of rm-1040):
//! - read-only access (`mode` flags), never `immutable=1` (the live DB is a
//!   WAL writer database; skipping WAL frames yields stale or torn reads);
//! - sibling databases (`goals_*`, `memories_*`, `logs_*`) are never opened;
//! - the consult never mutates the walked file list and never contributes to
//!   totals, counts or costs — disagreement is disclosed, not reconciled.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::OpenFlags;

/// Maximum number of sample rollout filenames kept in [`CodexStateDbReport`].
const MAX_SAMPLES: usize = 3;

/// Listing cross-check report for the Codex state DB.
///
/// `Default` is "not consulted" (no Codex files in the walked corpus, or no
/// resolvable Codex home).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexStateDbReport {
    /// Whether a state DB was found, opened read-only and queried.
    pub consulted: bool,
    /// The state DB that was consulted (newest `state_*.sqlite` by version).
    pub state_db_path: Option<PathBuf>,
    /// Rows in the `threads` table.
    pub db_threads: usize,
    /// Threads recorded in the DB whose rollout file was not walked.
    pub db_only_threads: usize,
    /// Walked Codex session files that the DB does not record.
    pub walk_only_files: usize,
    /// Sample rollout filenames backing `db_only_threads` (bounded).
    pub db_only_samples: Vec<String>,
    /// Why the consult was skipped, when it was.
    pub skip_reason: Option<&'static str>,
}

impl CodexStateDbReport {
    /// True when the consult found a disagreement worth disclosing.
    pub fn has_disagreement(&self) -> bool {
        self.consulted && (self.db_only_threads > 0 || self.walk_only_files > 0)
    }
}

/// Resolve the Codex home the same way `known_session_dirs` does:
/// `CODEX_HOME` override, else `$HOME/.codex`.
fn resolve_codex_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    let home = std::env::var_os("HOME")?;
    if home.is_empty() {
        return None;
    }
    Some(PathBuf::from(home).join(".codex"))
}

/// True when `path` is `home` or lives under it (both compared lexically
/// after best-effort canonicalization of the existing prefix).
fn is_under(path: &Path, home: &Path) -> bool {
    if path.starts_with(home) {
        return true;
    }
    match (path.canonicalize(), home.canonicalize()) {
        (Ok(p), Ok(h)) => p.starts_with(h),
        _ => false,
    }
}

/// Pick the newest `state_*.sqlite` under `home` by version numeral.
fn newest_state_db(home: &Path) -> Option<PathBuf> {
    let mut best: Option<(u64, PathBuf)> = None;
    let entries = std::fs::read_dir(home).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(rest) = name.strip_prefix("state_") else {
            continue;
        };
        let Some(num) = rest.strip_suffix(".sqlite") else {
            continue;
        };
        let Ok(version) = num.parse::<u64>() else {
            continue;
        };
        let path = entry.path();
        match &best {
            Some((v, _)) if *v >= version => {}
            _ => best = Some((version, path)),
        }
    }
    best.map(|(_, path)| path)
}

/// Consult the Codex state DB for the walked corpus `files`.
///
/// Pure with respect to the inputs: nothing is written, no file list is
/// changed, and the walked files stay the accounting truth. Returns
/// `Default` (not consulted) when the corpus contains no files under the
/// Codex home.
pub fn codex_state_db_report(files: &[PathBuf]) -> CodexStateDbReport {
    let Some(home) = resolve_codex_home() else {
        return CodexStateDbReport::default();
    };
    codex_state_db_report_for_home(&home, files)
}

/// Home-explicit core of [`codex_state_db_report`]. Tests drive this
/// directly so they never mutate the process-global `CODEX_HOME`/`HOME`
/// environment (parallel discovery-contract tests enumerate the same
/// variables).
fn codex_state_db_report_for_home(home: &Path, files: &[PathBuf]) -> CodexStateDbReport {
    let codex_files: Vec<&Path> = files
        .iter()
        .map(|p| p.as_path())
        .filter(|p| is_under(p, home))
        .collect();
    if codex_files.is_empty() {
        return CodexStateDbReport::default();
    }
    let Some(db_path) = newest_state_db(home) else {
        return CodexStateDbReport {
            consulted: false,
            skip_reason: Some("no state database"),
            ..CodexStateDbReport::default()
        };
    };

    let open = rusqlite::Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .and_then(|conn| {
        conn.busy_timeout(Duration::from_secs(5))?;
        Ok(conn)
    });
    let conn = match open {
        Ok(conn) => conn,
        Err(_) => {
            return CodexStateDbReport {
                consulted: false,
                state_db_path: Some(db_path),
                skip_reason: Some("unreadable database"),
                ..CodexStateDbReport::default()
            };
        }
    };

    /// `threads` may be absent (wrong-shape store) or unreadable (corrupt
    /// file — SQLite opens lazily, so corruption usually surfaces at the
    /// first prepare, not at open). Both are disclosed with their own
    /// reason, never fatal.
    fn threads_failure_reason(err: &rusqlite::Error) -> &'static str {
        let message = err.to_string().to_ascii_lowercase();
        if message.contains("not a database")
            || message.contains("malformed")
            || message.contains("corrupt")
            || message.contains("encrypted")
        {
            "unreadable database"
        } else {
            "no threads table"
        }
    }

    let mut stmt = match conn.prepare("SELECT rollout_path FROM threads") {
        Ok(stmt) => stmt,
        Err(err) => {
            return CodexStateDbReport {
                consulted: false,
                state_db_path: Some(db_path),
                skip_reason: Some(threads_failure_reason(&err)),
                ..CodexStateDbReport::default()
            };
        }
    };
    let rollout_paths: Vec<String> = match stmt.query_map([], |row| row.get::<_, Option<String>>(0))
    {
        Ok(rows) => rows.flatten().flatten().collect(),
        Err(err) => {
            return CodexStateDbReport {
                consulted: false,
                state_db_path: Some(db_path),
                skip_reason: Some(threads_failure_reason(&err)),
                ..CodexStateDbReport::default()
            };
        }
    };

    let walked_paths: BTreeSet<PathBuf> = codex_files.iter().map(|p| p.to_path_buf()).collect();
    let walked_names: BTreeSet<String> = codex_files
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let db_names: BTreeSet<String> = rollout_paths
        .iter()
        .filter_map(|p| {
            Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .collect();

    let mut db_only_samples = Vec::new();
    let mut db_only_threads = 0usize;
    for rollout in &rollout_paths {
        let path = Path::new(rollout);
        let walked_exact = path.is_absolute() && walked_paths.contains(path);
        let walked_by_name = path
            .file_name()
            .map(|n| walked_names.contains(&n.to_string_lossy().into_owned()))
            .unwrap_or(false);
        if !walked_exact && !walked_by_name {
            db_only_threads += 1;
            if db_only_samples.len() < MAX_SAMPLES {
                if let Some(name) = path.file_name() {
                    db_only_samples.push(name.to_string_lossy().into_owned());
                }
            }
        }
    }
    let walk_only_files = walked_names.difference(&db_names).count();

    CodexStateDbReport {
        consulted: true,
        state_db_path: Some(db_path),
        db_threads: rollout_paths.len(),
        db_only_threads,
        walk_only_files,
        db_only_samples,
        skip_reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "at-codex-state-db-test-{}-{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sessions")).expect("create temp codex home");
        dir
    }

    fn make_state_db(home: &Path, version: u64, rollouts: &[String]) {
        let db = home.join(format!("state_{version}.sqlite"));
        let conn = Connection::open(&db).expect("open fixture db");
        conn.execute_batch(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, rollout_path TEXT);
             CREATE TABLE backfill_state (state TEXT);",
        )
        .expect("create schema");
        for (i, rollout) in rollouts.iter().enumerate() {
            conn.execute(
                "INSERT INTO threads (id, rollout_path) VALUES (?1, ?2)",
                rusqlite::params![format!("t{i}"), rollout],
            )
            .expect("insert thread");
        }
    }

    fn touched_walked_file(home: &Path, name: &str) -> PathBuf {
        let path = home.join("sessions").join(name);
        std::fs::write(&path, b"{}\n").expect("create walked file");
        path
    }

    /// Tests drive the home-explicit core so the process-global
    /// `CODEX_HOME`/`HOME` are never mutated while other tests run.
    fn report_for(home: &Path, files: &[PathBuf]) -> CodexStateDbReport {
        codex_state_db_report_for_home(home, files)
    }

    #[test]
    fn consult_scenarios() {
        let home = temp_home("scenarios");

        // No Codex files under the home -> not consulted, no skip reason.
        let outside = std::env::temp_dir().join("at-codex-state-db-outside.jsonl");
        std::fs::write(&outside, b"{}\n").unwrap();
        assert_eq!(report_for(&home, &[outside]), CodexStateDbReport::default());

        // Missing DB: Codex files walked, nothing under the home.
        let walked_a = touched_walked_file(&home, "rollout-a.jsonl");
        let report = report_for(&home, std::slice::from_ref(&walked_a));
        assert!(!report.consulted);
        assert_eq!(report.skip_reason, Some("no state database"));

        // Drift in both directions: DB records a+b, walk has b+c.
        make_state_db(
            &home,
            5,
            &[
                walked_a.to_string_lossy().into_owned(),
                home.join("sessions")
                    .join("rollout-missing.jsonl")
                    .to_string_lossy()
                    .into_owned(),
            ],
        );
        let walked_c = touched_walked_file(&home, "rollout-c-unrecorded.jsonl");
        let walked_files = [walked_a.clone(), walked_c.clone()];
        let report = report_for(&home, &walked_files);
        assert!(report.consulted);
        assert_eq!(report.db_threads, 2);
        assert_eq!(report.db_only_threads, 1);
        assert_eq!(
            report.db_only_samples,
            vec!["rollout-missing.jsonl".to_string()]
        );
        assert_eq!(report.walk_only_files, 1);
        assert!(report.has_disagreement());

        // Purity: same inputs, same report; inputs untouched.
        let files = [walked_a.clone(), walked_c.clone()];
        let again = report_for(&home, &files);
        assert_eq!(again, report);
        assert_eq!(files, [walked_a, walked_c]);
    }

    #[test]
    fn consult_picks_newest_version() {
        let home = temp_home("newest");
        make_state_db(&home, 4, &[]);
        make_state_db(&home, 5, &[]);
        let walked = touched_walked_file(&home, "rollout-new.jsonl");
        let report = report_for(&home, std::slice::from_ref(&walked));
        assert!(report.consulted);
        assert_eq!(
            report.state_db_path,
            Some(home.join("state_5.sqlite")),
            "must consult the newest state_*.sqlite, not a fixed literal"
        );
    }

    #[test]
    fn consult_unreadable_and_wrong_shape() {
        let home = temp_home("hostile");
        let walked = touched_walked_file(&home, "rollout-h.jsonl");

        // Corrupt DB: garbage bytes must be disclosed as skipped, never panic.
        std::fs::write(
            home.join("state_5.sqlite"),
            b"this is not a sqlite database at all",
        )
        .unwrap();
        let report = report_for(&home, std::slice::from_ref(&walked));
        assert!(!report.consulted);
        assert_eq!(report.skip_reason, Some("unreadable database"));
        assert!(!report.has_disagreement());

        // Wrong shape: valid SQLite without a threads table (newest wins).
        {
            let conn = Connection::open(home.join("state_6.sqlite")).unwrap();
            conn.execute_batch("CREATE TABLE goals (id TEXT);").unwrap();
        }
        let report = report_for(&home, std::slice::from_ref(&walked));
        assert!(!report.consulted);
        assert_eq!(report.skip_reason, Some("no threads table"));
    }

    #[test]
    fn sibling_databases_are_never_picked() {
        let home = temp_home("siblings");
        {
            let conn = Connection::open(home.join("goals_1.sqlite")).unwrap();
            conn.execute_batch("CREATE TABLE goals (id TEXT);").unwrap();
            let conn = Connection::open(home.join("memories_1.sqlite")).unwrap();
            conn.execute_batch("CREATE TABLE memories (id TEXT);")
                .unwrap();
        }
        assert_eq!(
            newest_state_db(&home),
            None,
            "goals_*/memories_* must never be selected"
        );
    }
}

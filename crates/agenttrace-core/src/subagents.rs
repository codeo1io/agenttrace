//! rm-545: links Claude Code subagent transcripts (stored under
//! `<session>/subagents/agent-*.jsonl`) back into their parent session
//! and rolls each child's cost/tokens into the parent's
//! `subagent_*` metrics (kept separate from the parent's own usage —
//! the acceptance trait is the STANDALONE totals staying identical to
//! a run without subagents).
//!
//! rm-487 (escalated arm, same-directory layout): Claude Code 2.x
//! parks sidechains NEXT TO the parent — `agent-*.jsonl` beside
//! `<uuid>.jsonl`, no `subagents/` directory at all (this host: 0
//! `subagents/` directories vs 220 same-dir `isSidechain`
//! sidechains across 7 projects). The parent identity is recorded
//! inside the transcript: the journal `sessionId` equals the parent
//! file's stem. The rm-545 rule owns `subagents/` children; this
//! rule owns same-directory ones, and only links when the named
//! parent is part of the loaded corpus — no invented edges.
//!
//! rm-797: the parent is resolved through the SAME same-file identity
//! the discovery walk dedups on (rm-597 `SameFileIdentity`) — dev/inode
//! on Unix, canonicalized path elsewhere — so a parent reachable under
//! two spellings (hardlinked across scanned roots, or through a
//! symlinked component) links from EVERY spelling, not just the one
//! spelling the discovery dedup happened to keep. Exact-path matching
//! survived only as the fallback for paths that do not resolve on
//! disk (in-memory slices, unit fixtures).
//!
//! rm-791: sqlite-backed children (opencode `parent_id`) resolve their
//! parent by source row key (`Metrics::session_key`) inside their own
//! database — those rows share one file path (the DB file), so
//! path/identity linkage cannot see them and a separate key arm links
//! them instead.
//!
//! rm-799: children whose parent transcript is absent from the loaded
//! corpus (orphaned subagents — moved transcripts, partial copies) are
//! no longer silently dropped: the count is returned so callers can
//! disclose it while the child keeps rendering as a standalone row.
//!
//! rm-787: preserved-history rows keep the rollups persisted with
//! their record instead of being recomputed — their synthetic
//! `history:{id}` paths can never re-join the real-path index, so
//! recomputing would silently zero them and the `--include-history`
//! view would contradict the live view on the same corpus. Discovery
//! therefore attributes BEFORE `preserve_derived_history` (records
//! carry the rollups the live view showed) and again after the
//! history merge for live rows.

use crate::discovery::{same_file_identity, SameFileIdentity};
use crate::{total_tokens, Session};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

/// Returns the parent transcript path for a Claude Code subagent
/// transcript.
fn parent_transcript_path(path: &str) -> Option<String> {
    let path = Path::new(path);
    let name = path.file_name()?.to_str()?;
    if !(name.starts_with("agent-") && name.ends_with(".jsonl")) {
        return None;
    }
    let subagents_dir = path.parent()?;
    if subagents_dir.file_name()?.to_str()? != "subagents" {
        return None;
    }
    let session_dir = subagents_dir.parent()?;
    session_dir.file_name()?;
    // Append to the original string rather than `join`, so the separator
    // style matches the parent's own path on every platform.
    Some(format!("{}.jsonl", session_dir.to_string_lossy()))
}

/// The key a session is linked by. Mirrors discovery's dedup: files
/// that resolve on disk are keyed by [`SameFileIdentity`] (hardlinks
/// and symlinks to the same file collapse to one entry); everything
/// else falls back to the raw path string.
#[derive(PartialEq, Eq, Hash)]
enum LinkKey {
    Identity(SameFileIdentity),
    Path(String),
}

fn link_key(path: &str) -> LinkKey {
    match same_file_identity(Path::new(path)) {
        Some(identity) => LinkKey::Identity(identity),
        None => LinkKey::Path(path.to_string()),
    }
}

/// rm-487 (same-directory layout): resolve the parent of a sidechain
/// transcript parked NEXT TO its parent — `agent-*.jsonl` beside
/// `<uuid>.jsonl` — by the journal `sessionId` the transcript records
/// (which names the parent file's stem). Only applies outside
/// `subagents/` directories: the rm-545 rule owns those. Any failure
/// (missing/unreadable file, no `sessionId`, a hostile id carrying
/// path separators) leaves the child unlinked rather than inventing an
/// edge.
fn same_dir_parent_transcript_path(path: &str) -> Option<String> {
    let file_path = Path::new(path);
    let name = file_path.file_name()?.to_str()?;
    if !(name.starts_with("agent-") && name.ends_with(".jsonl")) {
        return None;
    }
    let dir = file_path.parent()?;
    if dir.file_name()?.to_str()? == "subagents" {
        return None;
    }
    let id = journal_session_id(file_path)?;
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return None;
    }
    Some(format!("{}.jsonl", dir.join(id).to_string_lossy()))
}

/// Reads the journal `sessionId` out of the first transcript entries
/// that carry one (bounded — 16 lines — so a huge journal costs one
/// buffer, not a full parse). Parse shape failures return `None`.
fn journal_session_id(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines().take(16) {
        let Ok(line) = line else {
            return None;
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if let Some(id) = value.get("sessionId").and_then(|id| id.as_str()) {
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// Rolls subagent cost and tokens up into their parent session.
///
/// Subagent sessions stay in the list (so fleet totals still count each
/// transcript exactly once) and record their parent; the parent gains
/// `subagent_*` rollups that are kept separate from its own metrics —
/// its own `cost_estimated`/tokens never absorb the children. Idempotent:
/// re-running on already attributed sessions recomputes the links from
/// scratch, and a child whose parent is absent keeps no stale link.
///
/// Linkage arms:
/// - rm-545: Claude Code subagent transcripts resolve their parent by
///   transcript path (`<session>/subagents/agent-*.jsonl`) — keyed by
///   the same-file identity (rm-797), with exact-path matching as the
///   fallback for paths that do not resolve on disk.
/// - rm-487: same-directory sidechains (`agent-*.jsonl` parked beside
///   their parent `<uuid>.jsonl`) resolve the parent by journal
///   `sessionId` == parent file stem; only a parent present in the
///   loaded corpus links, everything else stays unlinked.
/// - rm-791: sqlite-backed children (opencode `parent_id`) resolve their
///   parent by source row key (`Metrics::session_key`) inside the same
///   source database — those sessions share one `path` (the DB file),
///   so path linkage cannot see them.
///
/// Returns the number of subagent-shaped sessions whose parent
/// transcript is NOT part of the loaded corpus (rm-799) — callers
/// should disclose that count so orphaned children never read as
/// "no subagent work happened". Sqlite row-key children are not
/// subagent-shaped and never inflate that count; a child whose parent
/// row is absent keeps no stale link, as before.
pub fn attribute_subagents(sessions: &mut [Session]) -> usize {
    let mut index: HashMap<LinkKey, usize> = HashMap::with_capacity(sessions.len());
    for (i, session) in sessions.iter().enumerate() {
        // First spelling wins, mirroring the discovery dedup's
        // keep-one-survivor behavior for same-file sessions.
        index.entry(link_key(&session.path)).or_insert(i);
    }
    // rm-791: the sqlite loader parks the raw parent row id in
    // `parent_session` (source data, round-tripped through the sqlite
    // snapshot); capture those markers before the reset loop clears
    // them, then resolve them below.
    let key_links = sessions
        .iter()
        .enumerate()
        .filter(|(_, session)| {
            !session.metrics.session_key.is_empty() && !session.metrics.parent_session.is_empty()
        })
        .map(|(child, session)| (child, session.metrics.parent_session.clone()))
        .collect::<Vec<_>>();
    let mut unlinked = 0usize;
    let mut links = Vec::new();
    for (child, session) in sessions.iter().enumerate() {
        // The rm-545 `subagents/` rule first; the rm-487 same-dir
        // rule only for agent-*.jsonl files outside `subagents/`
        // directories (a missing/unreadable journal, a hostile id or
        // an absent parent leaves the child unlinked, never
        // invented).
        let Some(parent_path) = parent_transcript_path(&session.path)
            .or_else(|| same_dir_parent_transcript_path(&session.path))
        else {
            continue;
        };
        match index.get(&link_key(&parent_path)) {
            // Self-links (a child transcript hardlinked onto its own
            // parent file) would roll a session into itself — count
            // them as unlinked instead.
            Some(&parent) if parent != child => links.push((child, parent)),
            _ => unlinked += 1,
        }
    }
    for session in sessions.iter_mut() {
        // rm-787: preserved-history rows keep the rollups persisted with
        // their record — they were attributed while their transcripts
        // were live, and the synthetic `history:{id}` path can never
        // re-join the real-path index, so recomputing here would
        // silently zero them (the `--include-history` view used to
        // contradict the live view on the same corpus).
        if session.path.starts_with("history:") {
            continue;
        }
        session.metrics.subagent_count = 0;
        session.metrics.subagent_cost = 0.0;
        session.metrics.subagent_tokens = 0;
        session.metrics.parent_session = String::new();
    }
    for (child, parent) in links {
        let (cost, tokens) = (
            sessions[child].metrics.cost_estimated,
            total_tokens(&sessions[child]),
        );
        sessions[parent].metrics.subagent_count += 1;
        sessions[parent].metrics.subagent_cost += cost;
        sessions[parent].metrics.subagent_tokens += tokens;
        sessions[child].metrics.parent_session = sessions[parent].path.clone();
    }
    for (child, parent_key) in key_links {
        // The row id is only unique inside its own source database, so the
        // marker resolves within the same DB path and lane only; a child
        // whose parent is absent keeps no stale link (rm-545 semantics)
        // and a self-referential row never links to itself.
        let Some(parent) = sessions.iter().position(|candidate| {
            !candidate.metrics.session_key.is_empty()
                && candidate.metrics.session_key == parent_key
                && candidate.path == sessions[child].path
                && candidate.metrics.source_tool == sessions[child].metrics.source_tool
        }) else {
            continue;
        };
        if parent == child {
            continue;
        }
        let (cost, tokens) = (
            sessions[child].metrics.cost_estimated,
            total_tokens(&sessions[child]),
        );
        sessions[child].metrics.parent_session = parent_key;
        let metrics = &mut sessions[parent].metrics;
        metrics.subagent_count += 1;
        metrics.subagent_cost += cost;
        metrics.subagent_tokens += tokens;
    }
    unlinked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Metrics;

    fn session(path: &str, cost: f64, input: i64) -> Session {
        Session {
            org: String::new(),
            org_email: String::new(),
            name: path.to_string(),
            path: path.to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                cost_estimated: cost,
                tokens_input: input,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        }
    }

    #[test]
    fn resolves_parent_transcript_path() {
        assert_eq!(
            parent_transcript_path("/p/proj/abc/subagents/agent-a1.jsonl").as_deref(),
            Some("/p/proj/abc.jsonl")
        );
        assert_eq!(parent_transcript_path("/p/proj/abc.jsonl"), None);
        #[cfg(windows)]
        assert_eq!(
            parent_transcript_path(r"C:\p\abc\subagents\agent-a1.jsonl").as_deref(),
            Some(r"C:\p\abc.jsonl")
        );
        assert_eq!(
            parent_transcript_path("/p/proj/abc/other/agent-a1.jsonl"),
            None
        );
        assert_eq!(
            parent_transcript_path("/p/proj/abc/subagents/agent-a1.meta.json"),
            None
        );
    }

    #[test]
    fn rolls_subagents_into_parent_without_changing_own_metrics() {
        let mut sessions = vec![
            session("/p/proj/abc.jsonl", 1.0, 100),
            session("/p/proj/abc/subagents/agent-a1.jsonl", 0.5, 40),
            session("/p/proj/abc/subagents/agent-a2.jsonl", 0.25, 10),
            session("/p/proj/zzz/subagents/agent-orphan.jsonl", 9.0, 900),
        ];
        // rm-799: exactly the orphan counts as unlinked.
        assert_eq!(attribute_subagents(&mut sessions), 1);
        let parent = &sessions[0].metrics;
        assert_eq!(parent.cost_estimated, 1.0);
        assert_eq!(parent.subagent_count, 2);
        assert!((parent.subagent_cost - 0.75).abs() < 1e-9);
        assert_eq!(parent.subagent_tokens, 50);
        assert_eq!(sessions[1].metrics.parent_session, "/p/proj/abc.jsonl");
        assert!(sessions[3].metrics.parent_session.is_empty());

        // Idempotent when re-run on already attributed sessions.
        assert_eq!(attribute_subagents(&mut sessions), 1);
        assert_eq!(sessions[0].metrics.subagent_count, 2);

        // Dropping the parent clears stale child links on the next
        // pass and all three children turn unlinked.
        let mut orphans = sessions[1..].to_vec();
        assert_eq!(attribute_subagents(&mut orphans), 3);
        assert!(orphans[0].metrics.parent_session.is_empty());
    }

    fn sqlite_row_session(
        name: &str,
        session_key: &str,
        parent_row: &str,
        cost: f64,
        input: i64,
        path: &str,
    ) -> Session {
        let mut session = session(path, cost, input);
        session.name = name.to_string();
        session.metrics.session_key = session_key.to_string();
        session.metrics.parent_session = parent_row.to_string();
        session.metrics.source_tool = "opencode_db".to_string();
        session
    }

    #[test]
    fn links_sqlite_children_to_parents_by_source_row_key() {
        let db = "/home/u/.local/share/opencode/opencode.db";
        let mut sessions = vec![
            sqlite_row_session("parent", "ses-parent", "", 1.0, 100, db),
            sqlite_row_session("c1", "ses-c1", "ses-parent", 0.5, 40, db),
            sqlite_row_session("c2", "ses-c2", "ses-parent", 0.25, 10, db),
            sqlite_row_session("solo", "ses-solo", "", 9.0, 900, db),
        ];
        attribute_subagents(&mut sessions);
        let parent = &sessions[0].metrics;
        assert_eq!(parent.cost_estimated, 1.0, "parent's own cost is untouched");
        assert_eq!(parent.subagent_count, 2);
        assert!((parent.subagent_cost - 0.75).abs() < 1e-9);
        assert_eq!(parent.subagent_tokens, 50);
        assert_eq!(sessions[1].metrics.parent_session, "ses-parent");
        assert_eq!(sessions[2].metrics.parent_session, "ses-parent");
        assert_eq!(sessions[0].metrics.parent_session, "");
        // Parentless sessions stay bare.
        assert_eq!(sessions[3].metrics.subagent_count, 0);
        assert!(sessions[3].metrics.parent_session.is_empty());
        // Idempotent re-run keeps the lineage, not a double rollup.
        attribute_subagents(&mut sessions);
        assert_eq!(sessions[0].metrics.subagent_count, 2);
        assert!((sessions[0].metrics.subagent_cost - 0.75).abs() < 1e-9);
    }

    #[test]
    fn sqlite_key_linkage_stays_inside_one_database() {
        let db_a = "/home/u/.local/share/opencode/opencode.db";
        let db_b = "/home/u/.local/share/opencode2/opencode.db";
        let mut sessions = vec![
            // Same row key in another database must not link.
            sqlite_row_session("c1", "ses-c1", "ses-parent", 0.5, 40, db_a),
            sqlite_row_session("other", "ses-parent", "", 7.0, 700, db_b),
            // Parent absent from the load window: no stale link.
            sqlite_row_session("c2", "ses-c2", "ses-gone", 0.25, 10, db_a),
            // Self-referential row.
            sqlite_row_session("self", "ses-self", "ses-self", 1.0, 5, db_a),
            // Cross-lane key (hermes uuid spelling) never links opencode.
            sqlite_row_session("hermes", "ses-parent", "", 2.0, 200, db_a),
        ];
        sessions[4].metrics.source_tool = "hermes".to_string();
        attribute_subagents(&mut sessions);
        assert!(sessions[0].metrics.parent_session.is_empty());
        assert_eq!(sessions[1].metrics.subagent_count, 0);
        assert!(sessions[2].metrics.parent_session.is_empty());
        assert!(sessions[3].metrics.parent_session.is_empty());
        assert_eq!(sessions[3].metrics.subagent_count, 0);
        assert_eq!(sessions[4].metrics.subagent_count, 0);
    }

    #[test]
    fn hardlinked_parent_spellings_all_link() {
        // rm-797 red-first: the PoC shape from assess 35b3b690 —
        // A/P.jsonl and B/P.jsonl are the SAME file (hardlink), so
        // discovery dedups to one surviving spelling; the child lives
        // under the dropped spelling's tree and exact-path matching
        // never finds its parent. The identity key must link it.
        let _env = crate::test_env::lock_env();
        let tag = format!("at-subagent-hardlink-{}-{}", std::process::id(), line!());
        let root = std::env::temp_dir().join(&tag);
        let home = std::env::temp_dir().join(format!("{tag}-home"));
        let cache = home.join("cache");
        let a = root.join("A");
        let b = root.join("B");
        std::fs::create_dir_all(a.join("P/subagents")).expect("mkdir A");
        std::fs::create_dir_all(b.join("P/subagents")).expect("mkdir B");
        std::fs::create_dir_all(&cache).expect("mkdir sandbox cache");
        let parent_lines = concat!(
            r#"{"type":"user","timestamp":"2026-10-08T10:00:00Z","cwd":"/w/research","message":{"role":"user","content":"research x"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-10-08T10:00:05Z","cwd":"/w/research","message":{"role":"assistant","model":"claude-sonnet-4-5-20250929","content":[{"type":"text","text":"on it"}],"usage":{"input_tokens":1000,"output_tokens":500}}}"#,
            "\n"
        );
        std::fs::write(a.join("P.jsonl"), parent_lines).expect("write parent A");
        std::fs::hard_link(a.join("P.jsonl"), b.join("P.jsonl")).expect("hardlink parent");
        let child_lines = concat!(
            r#"{"type":"user","timestamp":"2026-10-08T10:01:00Z","cwd":"/w/research","message":{"role":"user","content":"do subtask"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-10-08T10:01:10Z","cwd":"/w/research","message":{"role":"assistant","model":"claude-sonnet-4-5-20250929","content":[{"type":"text","text":"done"}],"usage":{"input_tokens":200,"output_tokens":100}}}"#,
            "\n"
        );
        std::fs::write(b.join("P/subagents/agent-x.jsonl"), child_lines).expect("write child");
        // Sandbox HOME/XDG like the e2e harnesses (the rm-301
        // entrypoints.rs convention): the load path reads/writes the
        // session cache and must not touch the operator's real one.
        let saved: Vec<(&str, Option<std::ffi::OsString>)> =
            ["HOME", "XDG_CACHE_HOME", "AGENTTRACE_SESSION_CACHE_DIR"]
                .iter()
                .map(|k| (*k, std::env::var_os(k)))
                .collect();
        for (k, _) in &saved {
            std::env::remove_var(k);
        }
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_CACHE_HOME", &cache);
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache);
        let report = crate::discovery::load_sessions_with_options(Some(&root), &Default::default());
        for (k, v) in &saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        assert_eq!(
            report.sessions.len(),
            2,
            "discovery dedups the same-file parent to one session: {:?}",
            report
                .sessions
                .iter()
                .map(|s| s.path.clone())
                .collect::<Vec<_>>()
        );
        // rm-799 rides along: the child must NOT count as unlinked.
        assert_eq!(report.unlinked_subagents, 0);
        let parent = report
            .sessions
            .iter()
            .find(|s| s.metrics.parent_session.is_empty())
            .expect("parent row survives");
        assert_eq!(
            parent.metrics.subagent_count, 1,
            "child links through the same-file identity"
        );
        let child = report
            .sessions
            .iter()
            .find(|s| !s.metrics.parent_session.is_empty())
            .expect("child row survives");
        assert!(
            child.metrics.parent_session.ends_with("P.jsonl"),
            "child names the surviving parent spelling: {}",
            child.metrics.parent_session
        );
        std::fs::remove_dir_all(root).ok();
        std::fs::remove_dir_all(home).ok();
    }

    /// Scratch journal root that is UNIQUE to the test that creates
    /// it (pid + tag + process-wide sequence counter) and removes only
    /// its own dir on Drop (the host temp cleaner is not assumed).
    /// Pre-review, ONE pid-keyed dir was shared across the parallel
    /// unit tests and every Drop `remove_dir_all`'d the whole shared
    /// dir — deleting journals out from under the other tests — a
    /// reproduced ~2% suite flake (independent_review 355e05af, fixed
    /// at review fix b45f8690). Unique dirs make the cross-test
    /// deletion race impossible by construction.
    struct Scratch(std::path::PathBuf);
    static SCRATCH_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let seq = SCRATCH_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Scratch(std::env::temp_dir().join(format!(
                "agenttrace-subagents-{}-{tag}-{seq}",
                std::process::id()
            )))
        }
        /// Writes one journal under this test's dir; a nested name
        /// (e.g. `subagents/agent-n.jsonl`) creates its subdirs.
        fn file(&self, name: &str, first_line: &str) -> String {
            std::fs::create_dir_all(&self.0).ok();
            let path = self.0.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&path, format!("{first_line}\n")).ok();
            path.to_string_lossy().into_owned()
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolves_same_dir_parent_from_journal_session_id() {
        let scratch = Scratch::new("resolve");
        let child = scratch.file(
            "agent-side1.jsonl",
            r#"{"type":"user","sessionId":"51207be0-aaaa-bbbb-cccc-dddddddddddd","cwd":"/w"}"#,
        );
        let dir = std::path::Path::new(&child)
            .parent()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            same_dir_parent_transcript_path(&child).as_deref(),
            Some(format!("{dir}/51207be0-aaaa-bbbb-cccc-dddddddddddd.jsonl").as_str())
        );

        // No sessionId anywhere -> unlinked, not invented.
        let bare = scratch.file("agent-bare.jsonl", r#"{"type":"user","cwd":"/w"}"#);
        assert_eq!(same_dir_parent_transcript_path(&bare), None);

        // Hostile ids carrying path material -> unlinked.
        let hostile = scratch.file("agent-evil.jsonl", r#"{"sessionId":"../../etc/passwd"}"#);
        assert_eq!(same_dir_parent_transcript_path(&hostile), None);

        // The rm-545 rule owns subagents/ children: no fallback there.
        let nested = scratch.file(
            "subagents/agent-n.jsonl",
            r#"{"sessionId":"51207be0-aaaa-bbbb-cccc-dddddddddddd"}"#,
        );
        assert_eq!(same_dir_parent_transcript_path(&nested), None);
    }

    #[test]
    fn links_same_dir_sidechain_to_its_parent() {
        let scratch = Scratch::new("link");
        let parent = scratch.file("51207be0-aaaa-bbbb-cccc-dddddddddddd.jsonl", "{}");
        let child = scratch.file(
            "agent-of-51207be0.jsonl",
            r#"{"type":"assistant","sessionId":"51207be0-aaaa-bbbb-cccc-dddddddddddd"}"#,
        );
        let unrelated = scratch.file("other.jsonl", "{}");
        let mut sessions = vec![
            session(&parent, 0.01, 10),
            session(&child, 0.0021, 7),
            session(&unrelated, 1.0, 100),
        ];
        attribute_subagents(&mut sessions);
        assert_eq!(sessions[0].metrics.subagent_count, 1);
        assert!((sessions[0].metrics.subagent_cost - 0.0021).abs() < 1e-12);
        assert_eq!(sessions[1].metrics.parent_session, parent);
        assert_eq!(sessions[2].metrics.subagent_count, 0);

        // A sidechain whose journal names a parent that is not loaded
        // stays unlinked.
        let mut lone = vec![session(&child, 0.0021, 7)];
        attribute_subagents(&mut lone);
        assert!(lone[0].metrics.parent_session.is_empty());
    }

    #[test]
    fn preserved_history_rows_keep_their_persisted_rollups() {
        let live = session("/p/live.jsonl", 1.0, 100);
        let mut preserved = session("history:34235f4412345678", 0.0, 0);
        preserved.name = "history-34235f44".to_string();
        preserved.metrics.subagent_count = 2;
        preserved.metrics.subagent_cost = 0.0053;
        preserved.metrics.subagent_tokens = 91;
        preserved.metrics.parent_session = "/gone/p1-1111.jsonl".to_string();
        let mut sessions = vec![live, preserved];
        attribute_subagents(&mut sessions);
        // rm-787: the preserved row is not recomputed (its synthetic path
        // can never re-join the index) — the persisted rollups survive.
        assert_eq!(sessions[1].metrics.subagent_count, 2);
        assert!((sessions[1].metrics.subagent_cost - 0.0053).abs() < 1e-12);
        assert_eq!(sessions[1].metrics.subagent_tokens, 91);
        assert_eq!(sessions[1].metrics.parent_session, "/gone/p1-1111.jsonl");
        // Live rows still recompute from scratch.
        assert_eq!(sessions[0].metrics.subagent_count, 0);
    }
}

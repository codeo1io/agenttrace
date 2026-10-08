//! rm-545: links Claude Code subagent transcripts to the session that
//! spawned them (port of upstream #305, adapted to this fork).
//!
//! Two resolution rules, both no-invented-edges:
//! - `<session>/subagents/agent-*.jsonl` → the transcript beside the
//!   `subagents/` directory (rm-545);
//! - rm-487 (escalated arm, same-directory layout): Claude Code 2.x
//!   parks sidechains NEXT TO the parent (`agent-*.jsonl` beside
//!   `<uuid>.jsonl`, no `subagents/` directory at all). The parent
//!   identity is recorded inside the transcript: the journal
//!   `sessionId` equals the parent file's stem — proven on 220/220
//!   same-dir sidechains across 7 projects on this host (0
//!   `subagents/` directories, so the rm-545 rule links nothing
//!   there).
//!
//! Attribution runs after load (discovery.rs) and before range filters,
//! so cached sessions get the same treatment as freshly parsed ones and
//! the session-cache schema stays untouched. Preserved-history rows
//! (rm-787) keep the rollups persisted with their record instead of
//! being recomputed: their synthetic `history:{id}` paths cannot join
//! the index below.

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
pub fn attribute_subagents(sessions: &mut [Session]) {
    let index: HashMap<String, usize> = sessions
        .iter()
        .enumerate()
        .map(|(i, session)| (session.path.clone(), i))
        .collect();
    let links = sessions
        .iter()
        .enumerate()
        .filter_map(|(child, session)| {
            let parent_path = parent_transcript_path(&session.path)
                .or_else(|| same_dir_parent_transcript_path(&session.path))?;
            let parent = *index.get(&parent_path)?;
            // A sidechain whose journal id names itself (hostile or
            // self-referential) must not roll up into itself.
            (parent != child).then_some((child, parent))
        })
        .collect::<Vec<_>>();
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
        session.metrics.parent_session.clear();
    }
    for (child, parent) in links {
        let (cost, tokens) = (
            sessions[child].metrics.cost_estimated,
            total_tokens(&sessions[child]),
        );
        sessions[child].metrics.parent_session = sessions[parent].path.clone();
        let metrics = &mut sessions[parent].metrics;
        metrics.subagent_count += 1;
        metrics.subagent_cost += cost;
        metrics.subagent_tokens += tokens;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Metrics;

    fn session(path: &str, cost: f64, input: i64) -> Session {
        Session {
            name: path.to_string(),
            path: path.to_string(),
            cwd: String::new(),
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
        attribute_subagents(&mut sessions);
        let parent = &sessions[0].metrics;
        assert_eq!(parent.cost_estimated, 1.0);
        assert_eq!(parent.subagent_count, 2);
        assert!((parent.subagent_cost - 0.75).abs() < 1e-9);
        assert_eq!(parent.subagent_tokens, 50);
        assert_eq!(sessions[1].metrics.parent_session, "/p/proj/abc.jsonl");
        assert!(sessions[3].metrics.parent_session.is_empty());

        // Idempotent when re-run on already attributed sessions.
        attribute_subagents(&mut sessions);
        assert_eq!(sessions[0].metrics.subagent_count, 2);

        // Dropping the parent clears stale child links on the next pass.
        let mut orphans = sessions[1..].to_vec();
        attribute_subagents(&mut orphans);
        assert!(orphans[0].metrics.parent_session.is_empty());
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

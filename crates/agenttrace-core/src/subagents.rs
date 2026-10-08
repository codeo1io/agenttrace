//! rm-545: links Claude Code subagent transcripts (stored under
//! `<session>/subagents/agent-*.jsonl`) back into their parent session
//! and rolls each child's cost/tokens into the parent's
//! `subagent_*` metrics (kept separate from the parent's own usage —
//! the acceptance trait is the STANDALONE totals staying identical to
//! a run without subagents).
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
//! rm-799: children whose parent transcript is absent from the loaded
//! corpus (orphaned subagents — moved transcripts, partial copies) are
//! no longer silently dropped: the count is returned so callers can
//! disclose it while the child keeps rendering as a standalone row.

use crate::discovery::{same_file_identity, SameFileIdentity};
use crate::{total_tokens, Session};
use std::collections::HashMap;
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

/// Attribute subagent rollups onto parent sessions. Returns the number
/// of subagent-shaped sessions whose parent transcript is NOT part of
/// the loaded corpus (rm-799) — callers should disclose that count so
/// orphaned children never read as "no subagent work happened".
pub fn attribute_subagents(sessions: &mut [Session]) -> usize {
    let mut index: HashMap<LinkKey, usize> = HashMap::with_capacity(sessions.len());
    for (i, session) in sessions.iter().enumerate() {
        // First spelling wins, mirroring the discovery dedup's
        // keep-one-survivor behavior for same-file sessions.
        index.entry(link_key(&session.path)).or_insert(i);
    }
    let mut unlinked = 0usize;
    let mut links = Vec::new();
    for (child, session) in sessions.iter().enumerate() {
        let Some(parent_path) = parent_transcript_path(&session.path) else {
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
    unlinked
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
}

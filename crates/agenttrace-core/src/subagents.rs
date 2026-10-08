//! rm-545: links Claude Code subagent transcripts
//! (`<session>/subagents/agent-*.jsonl`) to the session that spawned
//! them (port of upstream #305, adapted to this fork).
//!
//! Attribution runs after load (discovery.rs) and before range filters,
//! so cached sessions get the same treatment as freshly parsed ones and
//! the session-cache schema stays untouched.

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

/// Rolls subagent cost and tokens up into their parent session.
///
/// Subagent sessions stay in the list (so fleet totals still count each
/// transcript exactly once) and record their parent; the parent gains
/// `subagent_*` rollups that are kept separate from its own metrics —
/// its own `cost_estimated`/tokens never absorb the children. Idempotent:
/// re-running on already attributed sessions recomputes the links from
/// scratch, and a child whose parent is absent keeps no stale link.
///
/// Two linkage arms:
/// - rm-545: Claude Code subagent transcripts resolve their parent by
///   transcript path (`<session>/subagents/agent-*.jsonl`).
/// - rm-791: sqlite-backed children (opencode `parent_id`) resolve their
///   parent by source row key (`Metrics::session_key`) inside the same
///   source database — those sessions share one `path` (the DB file),
///   so path linkage cannot see them.
pub fn attribute_subagents(sessions: &mut [Session]) {
    let index: HashMap<String, usize> = sessions
        .iter()
        .enumerate()
        .map(|(i, session)| (session.path.clone(), i))
        .collect();
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
    let links = sessions
        .iter()
        .enumerate()
        .filter_map(|(child, session)| {
            let parent = *index.get(&parent_transcript_path(&session.path)?)?;
            Some((child, parent))
        })
        .collect::<Vec<_>>();
    for session in sessions.iter_mut() {
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
}

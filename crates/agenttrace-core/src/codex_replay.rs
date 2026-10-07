//! rm-402: cross-file baseline inheritance for Codex child rollouts.
//!
//! Codex `compact` / subagent forks write a NEW rollout file whose head
//! *replays the parent's conversation* — and the replayed prefix
//! carries the parent's cumulative `token_count` snapshots verbatim.
//! Parsing each file independently (the pre-rm-402 behavior) counted
//! the replayed prefix a second time: a $1.40 parent forked once
//! reported $2.80 across the fleet. Upstream fixed the same defect in
//! #311's lane by keying the parent prefix out of the child's first
//! `session_meta` line; ccusage's `CodexReplayPlan` (rust/adapters/codex)
//! is the ported semantics here:
//!
//! 1. probe every rollout's first line — a `session_meta` line names
//!    its own session id (`payload.id`) and, for a fork, its parent
//!    (`payload.forked_from_id`, or
//!    `payload.source.subagent.thread_spawn.parent_thread_id` for
//!    subagent threads) plus the fork time (the top-level `timestamp`);
//! 2. a child's parent is the first OTHER file in the corpus carrying
//!    the parent's session id;
//! 3. the parent's cumulative usage stream is replayed up to the fork
//!    time and its high-water mark becomes the child parser's opening
//!    `prev_token_total` (see `parser::parse_codex_rollout_jsonl`),
//!    so the replayed snapshots compute zero deltas and drop out.
//!
//! A child that names a parent not present in the corpus counts from
//! zero — nothing is invented — and the gap is disclosed through the
//! `codex_replay_parent_missing` parse counter (it rides `line_skips`
//! with every other codex parse decision, rm-401's channel). Sessions
//! with no fork marker never enter the plan, so corpora without
//! cross-file relationships parse byte-identically to before.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::Value;

/// Baseline one child rollout inherits from its parent.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CodexReplayBaseline {
    /// The parent's cumulative usage keys (the six keys
    /// `parser::token_usage_map` keeps) at — or before — the fork.
    pub usage: BTreeMap<String, i64>,
    /// False when the child names a parent that is not part of the
    /// corpus: the child counts from zero and the gap is disclosed.
    pub parent_found: bool,
}

/// What a rollout's first `session_meta` line told us.
struct SessionMetaProbe {
    session_id: String,
    parent_id: Option<String>,
    forked_at: Option<DateTime<Utc>>,
}

/// Read the first line of `path` and parse it as a Codex `session_meta`
/// record. Returns `None` for every other shape: non-codex transcripts,
/// unreadable paths (directories, permission misses), empty files, or
/// malformed JSON — a probe miss simply leaves the file out of the
/// plan, exactly like ccusage's probe.
fn probe_first_line(path: &Path) -> Option<SessionMetaProbe> {
    let file = std::fs::File::open(path).ok()?;
    let mut line = String::new();
    BufReader::new(file).read_line(&mut line).ok()?;
    let obj: Value = serde_json::from_str(line.trim_end()).ok()?;
    if obj.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = obj.get("payload")?.as_object()?;
    let session_id = payload
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())?
        .to_string();
    let parent_id = payload
        .get("forked_from_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && *id != "null")
        .map(str::to_string)
        .or_else(|| {
            // `payload.source` is a tagged enum: a subagent thread
            // serializes its spawn metadata directly under `source`
            // (`{"type":"subagent","thread_spawn":{…}}`); a
            // redundant nested `source.subagent` object links too.
            let source = payload.get("source")?.as_object()?;
            let spawn = source
                .get("thread_spawn")
                .or_else(|| source.get("subagent")?.as_object()?.get("thread_spawn"))?
                .as_object()?;
            spawn
                .get("parent_thread_id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
        });
    let forked_at = obj
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339);
    Some(SessionMetaProbe {
        session_id,
        parent_id,
        forked_at,
    })
}

fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|stamp| stamp.with_timezone(&Utc))
}

/// The parent's cumulative usage high-water at (or before) `forked_at`.
///
/// Snapshots arrive already cumulative (Codex's own semantics), so the
/// high-water over the slice is a per-key max — the same merge
/// `parser::token_usage_high_water` applies inside one file. A snapshot
/// whose timestamp cannot be parsed never stops the slice (ccusage
/// treats unparsable stamps the same way); snapshots strictly after the
/// fork belong to the parent's own future, not the child's baseline.
fn parent_baseline_at_fork(
    parent: &Path,
    forked_at: Option<DateTime<Utc>>,
) -> BTreeMap<String, i64> {
    let Ok(file) = std::fs::File::open(parent) else {
        return BTreeMap::new();
    };
    let mut high_water: BTreeMap<String, i64> = BTreeMap::new();
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            break;
        };
        // Cheap prefilter, the inverse of parser's ignorable-line skip:
        // only token_count lines carry a total_token_usage snapshot.
        if !crate::parser::json_key_present(&line, r#""type":"token_count""#) {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if obj.get("type").and_then(Value::as_str) != Some("event_msg") {
            continue;
        }
        let within_prefix = match forked_at {
            None => true,
            Some(fork) => match obj
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(parse_rfc3339)
            {
                Some(stamp) => stamp <= fork,
                None => true,
            },
        };
        if !within_prefix {
            continue;
        }
        let snapshot = crate::parser::token_usage_map(
            obj.get("payload")
                .and_then(Value::as_object)
                .and_then(|payload| payload.get("info"))
                .and_then(|info| info.get("total_token_usage")),
        );
        for (key, value) in snapshot {
            let slot = high_water.entry(key).or_insert(0);
            *slot = (*slot).max(value);
        }
    }
    high_water
}

/// One baseline per child rollout in `files`, keyed by the child's path.
///
/// Only files whose first line names a parent take part; everything
/// else (the overwhelming majority of any corpus) never enters the
/// map, and a file missing from the map parses exactly as before.
/// The parent lookup is id-scoped to the same corpus, excluding the
/// child itself, in discovery's file order — the first transcript
/// carrying the parent's session id is the parent (Codex re-emits one
/// rollout per session id; a copied corpus keeps per-copy linkage
/// because the probe reads each copy's own first line).
pub(crate) fn replay_baselines(files: &[PathBuf]) -> BTreeMap<PathBuf, CodexReplayBaseline> {
    let mut probes: Vec<Option<SessionMetaProbe>> = Vec::with_capacity(files.len());
    for path in files.iter() {
        probes.push(probe_first_line(path));
    }
    // Owned by `probes`; built after probing so no borrow overlaps a
    // move. Keyed by each rollout's own first-line session id.
    let mut files_by_session_id: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, probe) in probes.iter().enumerate() {
        if let Some(session_id) = probe.as_ref().map(|probe| probe.session_id.as_str()) {
            files_by_session_id
                .entry(session_id)
                .or_default()
                .push(index);
        }
    }
    let mut baselines = BTreeMap::new();
    for (index, probe) in probes.iter().enumerate() {
        let Some(probe) = probe else {
            continue;
        };
        let Some(parent_id) = probe.parent_id.as_deref() else {
            continue;
        };
        // `files_by_session_id` borrows from `probes`; both are done
        // mutating here, and the borrows end before `files` is read.
        let parent_index = files_by_session_id
            .get(parent_id)
            .and_then(|indexes| indexes.iter().copied().find(|other| *other != index));
        let baseline = match parent_index {
            Some(parent_index) => CodexReplayBaseline {
                usage: parent_baseline_at_fork(&files[parent_index], probe.forked_at),
                parent_found: true,
            },
            None => CodexReplayBaseline {
                usage: BTreeMap::new(),
                parent_found: false,
            },
        };
        baselines.insert(files[index].clone(), baseline);
    }
    baselines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rollout(session_id: &str, parent_id: Option<&str>, fork_ts: &str) -> String {
        let mut payload = serde_json::json!({ "id": session_id });
        if let Some(id) = parent_id {
            payload["forked_from_id"] = serde_json::json!(id);
        }
        let line = serde_json::json!({
            "timestamp": fork_ts,
            "type": "session_meta",
            "payload": payload,
        });
        format!("{line}\n")
    }

    fn token_count(ts: &str, input: i64, cached: i64, output: i64) -> String {
        let line = serde_json::json!({
            "timestamp": ts,
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": input,
                        "cached_input_tokens": cached,
                        "output_tokens": output,
                    },
                },
            },
        });
        format!("{line}\n")
    }

    fn write(path: &Path, body: &str) {
        std::fs::write(path, body).expect("write fixture");
    }

    #[test]
    fn probe_reads_session_meta_and_both_parent_markers() {
        let dir = std::env::temp_dir().join("at-rm402-probe");
        std::fs::create_dir_all(&dir).unwrap();
        let plain = dir.join("plain.jsonl");
        write(
            &plain,
            &rollout("sess-plain", None, "2026-10-01T10:00:00.000Z"),
        );
        let probe = probe_first_line(&plain).unwrap();
        assert_eq!(probe.session_id, "sess-plain");
        assert!(probe.parent_id.is_none());

        let forked = dir.join("forked.jsonl");
        write(
            &forked,
            &rollout(
                "sess-child",
                Some("sess-parent"),
                "2026-10-01T11:00:00.000Z",
            ),
        );
        let probe = probe_first_line(&forked).unwrap();
        assert_eq!(probe.parent_id.as_deref(), Some("sess-parent"));
        assert!(probe.forked_at.is_some());

        let subagent = dir.join("subagent.jsonl");
        let line = serde_json::json!({
            "timestamp": "2026-10-01T12:00:00.000Z",
            "type": "session_meta",
            "payload": {
                "id": "sess-sub",
                "source": {
                    "type": "subagent",
                    "thread_spawn": { "parent_thread_id": "sess-parent" },
                },
            },
        });
        write(&subagent, &format!("{line}\n"));
        let probe = probe_first_line(&subagent).unwrap();
        assert_eq!(probe.parent_id.as_deref(), Some("sess-parent"));

        // A non-codex first line (claude-code shape) is not a probe hit.
        let foreign = dir.join("foreign.jsonl");
        write(
            &foreign,
            r#"{"type":"summary","summary":"x","leafUuid":"y"}"#,
        );
        assert!(probe_first_line(&foreign).is_none());
    }

    #[test]
    fn baseline_slices_the_parent_at_the_fork() {
        let dir = std::env::temp_dir().join("at-rm402-slice");
        std::fs::create_dir_all(&dir).unwrap();
        let parent = dir.join("parent.jsonl");
        write(
            &parent,
            &format!(
                "{}{}{}{}",
                rollout("sess-parent", None, "2026-10-01T10:00:00.000Z"),
                // Pre-fork snapshots: cumulative 400/800/200 then 400/800/500.
                token_count("2026-10-01T10:01:00.000Z", 400, 800, 200),
                token_count("2026-10-01T10:02:00.000Z", 400, 800, 500),
                // Post-fork growth belongs to the parent alone: the
                // fork happened at 10:03.
                token_count("2026-10-01T10:04:00.000Z", 400, 800, 900),
            ),
        );
        let baseline = parent_baseline_at_fork(&parent, parse_rfc3339("2026-10-01T10:03:00.000Z"));
        assert_eq!(baseline.get("input_tokens"), Some(&400));
        assert_eq!(baseline.get("cached_input_tokens"), Some(&800));
        assert_eq!(baseline.get("output_tokens"), Some(&500));

        // Without a fork timestamp the parent's final state is the
        // baseline (ccusage falls back the same way).
        let whole = parent_baseline_at_fork(&parent, None);
        assert_eq!(whole.get("output_tokens"), Some(&900));
    }

    #[test]
    fn plan_skips_plain_files_and_flags_missing_parents() {
        let dir = std::env::temp_dir().join("at-rm402-plan");
        std::fs::create_dir_all(&dir).unwrap();
        let parent = dir.join("parent.jsonl");
        write(
            &parent,
            &format!(
                "{}{}",
                rollout("sess-parent", None, "2026-10-01T10:00:00.000Z"),
                token_count("2026-10-01T10:01:00.000Z", 400, 800, 200),
            ),
        );
        let child = dir.join("child.jsonl");
        write(
            &child,
            &rollout(
                "sess-child",
                Some("sess-parent"),
                "2026-10-01T10:02:00.000Z",
            ),
        );
        let orphan = dir.join("orphan.jsonl");
        write(
            &orphan,
            &rollout("sess-orphan", Some("sess-gone"), "2026-10-01T10:02:00.000Z"),
        );
        let plain = dir.join("plain.jsonl");
        write(&plain, "not a codex rollout at all\n");

        let files = vec![parent.clone(), child.clone(), orphan.clone(), plain];
        let plan = replay_baselines(&files);
        assert_eq!(plan.len(), 2, "only the two forked children enter");
        let child_baseline = &plan[&child];
        assert!(child_baseline.parent_found);
        assert_eq!(child_baseline.usage.get("cached_input_tokens"), Some(&800));
        let orphan_baseline = &plan[&orphan];
        assert!(
            !orphan_baseline.parent_found,
            "named parent is not in corpus"
        );
        assert!(orphan_baseline.usage.is_empty());
    }
}

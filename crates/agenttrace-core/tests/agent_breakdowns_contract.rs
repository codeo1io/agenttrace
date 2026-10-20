//! rm-2076 (cycle 4, run 9cc1462150e9): `agent_breakdowns` contract —
//! the per-agent-type usage view `attribute_subagents` builds in the
//! same pass as the `subagent_*` rollups, emitted on the session JSON
//! lane (`--sessions -f json` serializes `Session`/`Metrics`
//! wholesale; an empty breakdown is skipped so childless lanes stay
//! byte-identical — the rm-232 "schema bump, never re-key" rule).
//!
//! Pinned invariants:
//! * shape — own row first ("main", or the session's `agent_name`),
//!   then children in sorted type order, same-type children merged;
//! * reconciliation — Σ rows' token classes == own `total_tokens` +
//!   `subagent_tokens` and Σ rows' `cost_usd` == `cost_estimated` +
//!   `subagent_cost` (one level: a middle session's row on its parent
//!   carries the middle's OWN metrics, never its subagent rollups);
//! * discipline parity — non-finite child costs are excluded from
//!   rows exactly as `subagent_cost` excludes them (rm-1500), token
//!   classes saturate (rm-487);
//! * idempotency — re-attribution recomputes the same rows;
//! * the discovery load path emits the field end to end and the JSON
//!   lane carries it only for linked parents.

use agenttrace_core::{attribute_subagents, AgentBreakdown, Metrics, Session};
use std::sync::{Mutex, MutexGuard, OnceLock};

fn session(path: &str, metrics: Metrics) -> Session {
    Session {
        org: String::new(),
        org_email: String::new(),
        name: path.to_string(),
        path: path.to_string(),
        cwd: String::new(),
        branch: String::new(),
        metrics,
        anomalies: Vec::new(),
        health: 100,
        tool_warnings: Vec::new(),
        diagnostics: agenttrace_core::Diagnostics::default(),
    }
}

fn metrics(
    input: i64,
    output: i64,
    cache_r: i64,
    cache_w: i64,
    cost: f64,
    duration: f64,
) -> Metrics {
    Metrics {
        tokens_input: input,
        tokens_output: output,
        tokens_cache_r: cache_r,
        tokens_cache_w: cache_w,
        cost_estimated: cost,
        duration_sec: duration,
        ..Metrics::default()
    }
}

fn class_sum(rows: &[AgentBreakdown]) -> (i64, i64, i64, f64) {
    let mut cache_r = 0i64;
    let mut cache_w = 0i64;
    let mut regular = 0i64;
    let mut cost = 0.0f64;
    for row in rows {
        cache_r = cache_r.saturating_add(row.tokens_cache_r);
        cache_w = cache_w.saturating_add(row.tokens_cache_w);
        regular = regular.saturating_add(row.tokens_regular);
        cost += row.cost_usd;
    }
    (cache_r, cache_w, regular, cost)
}

#[test]
fn path_lane_breakdown_reconciles_to_ledger() {
    let mut parent_metrics = metrics(100, 10, 7, 5, 1.0, 100.0);
    parent_metrics.agent_name = String::new();
    let mut explore_a = metrics(40, 4, 3, 2, 0.5, 10.0);
    explore_a.agent_name = "Explore".to_string();
    // Same type as explore_a (must merge into one row) but a poisoned
    // cost (excluded from rows exactly as subagent_cost skips it).
    let mut explore_b = metrics(20, 2, 1, 0, f64::NAN, 1.0);
    explore_b.agent_name = "Explore".to_string();
    let mut unlabeled = metrics(10, 1, 1, 1, 0.25, 5.0);
    unlabeled.agent_name = String::new();
    let mut sessions = vec![
        session("/p/proj/abc.jsonl", parent_metrics),
        session("/p/proj/abc/subagents/agent-a1.jsonl", explore_a),
        session("/p/proj/abc/subagents/agent-a2.jsonl", unlabeled),
        session("/p/proj/abc/subagents/agent-a3.jsonl", explore_b),
        // Orphan: linked to nothing, keeps no breakdown.
        session(
            "/p/zzz/subagents/agent-orphan.jsonl",
            metrics(9, 9, 9, 9, 9.0, 9.0),
        ),
    ];
    assert_eq!(attribute_subagents(&mut sessions), 1);

    let parent = &sessions[0];
    assert_eq!(parent.metrics.subagent_count, 3);
    let rows = &parent.metrics.agent_breakdowns;
    let labels = rows.iter().map(|r| r.agent.as_str()).collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec!["main", "Explore", "subagent"],
        "own row first, children sorted, same-type merged"
    );

    let main = &rows[0];
    assert_eq!(
        (
            main.tokens_cache_r,
            main.tokens_cache_w,
            main.tokens_regular
        ),
        (7, 5, 110)
    );
    assert!((main.cost_usd - 1.0).abs() < 1e-9);
    assert!((main.duration_sec - 100.0).abs() < 1e-9);
    let explore = &rows[1];
    assert_eq!(
        (
            explore.tokens_cache_r,
            explore.tokens_cache_w,
            explore.tokens_regular
        ),
        (4, 2, 66)
    );
    // 0.5 only — the poisoned sibling contributes nothing.
    assert!((explore.cost_usd - 0.5).abs() < 1e-9);
    assert!((explore.duration_sec - 11.0).abs() < 1e-9);
    let sub = &rows[2];
    assert_eq!(
        (sub.tokens_cache_r, sub.tokens_cache_w, sub.tokens_regular),
        (1, 1, 11)
    );

    // Ledger reconciliation: rows' classes sum to own total_tokens plus
    // the subagent_tokens rollup, and rows' cost to cost_estimated +
    // subagent_cost (poisoned cost excluded from both sides).
    let (r, w, reg, cost) = class_sum(rows);
    let own_total = agenttrace_core::total_tokens(parent);
    assert_eq!(
        r + w + reg,
        own_total.saturating_add(parent.metrics.subagent_tokens)
    );
    assert!((cost - (parent.metrics.cost_estimated + parent.metrics.subagent_cost)).abs() < 1e-9);
    assert!((parent.metrics.subagent_cost - 0.75).abs() < 1e-9);

    // Children and the orphan keep no breakdown of their own.
    for idx in [1usize, 2, 3, 4] {
        assert!(sessions[idx].metrics.agent_breakdowns.is_empty());
    }
    // rm-232: childless lanes stay byte-identical — the key is absent.
    let orphan_json = serde_json::to_string(&sessions[4].metrics).expect("serialize orphan");
    assert!(!orphan_json.contains("agent_breakdowns"));
    let parent_json = serde_json::to_string(&parent.metrics).expect("serialize parent");
    assert!(parent_json.contains("\"agent_breakdowns\""));
    assert!(parent_json.contains("\"agent\":\"Explore\""));

    // Idempotent: re-attribution recomputes the identical rows.
    let before = parent.metrics.agent_breakdowns.clone();
    attribute_subagents(&mut sessions);
    assert_eq!(sessions[0].metrics.agent_breakdowns, before);
}

#[test]
fn key_lane_breakdown_and_own_label_uses_agent_name() {
    // opencode sqlite lane: parent and child share one DB path; the
    // child carries the parent's row key as its marker.
    let mut parent_metrics = metrics(100, 0, 0, 0, 2.0, 60.0);
    parent_metrics.agent_name = "build".to_string();
    parent_metrics.session_key = "ses-parent".to_string();
    parent_metrics.source_tool = "opencode".to_string();
    let mut child_metrics = metrics(30, 3, 0, 0, 0.3, 6.0);
    child_metrics.agent_name = "plan".to_string();
    child_metrics.session_key = "ses-c1".to_string();
    child_metrics.parent_session = "ses-parent".to_string();
    child_metrics.source_tool = "opencode".to_string();
    let mut sessions = vec![
        session("/db/app.db", parent_metrics),
        session("/db/app.db", child_metrics),
    ];
    assert_eq!(attribute_subagents(&mut sessions), 0);
    let parent = &sessions[0];
    assert_eq!(parent.metrics.subagent_count, 1);
    let labels = parent
        .metrics
        .agent_breakdowns
        .iter()
        .map(|r| r.agent.as_str())
        .collect::<Vec<_>>();
    // The own row takes the session's agent_name, not the literal
    // "main", when the source carries one.
    assert_eq!(labels, vec!["build", "plan"]);
    let (r, w, reg, cost) = class_sum(&parent.metrics.agent_breakdowns);
    assert_eq!(
        r + w + reg,
        agenttrace_core::total_tokens(parent).saturating_add(parent.metrics.subagent_tokens)
    );
    assert!((cost - (parent.metrics.cost_estimated + parent.metrics.subagent_cost)).abs() < 1e-9);
    assert!(sessions[1].metrics.agent_breakdowns.is_empty());
}

#[test]
fn ledger_stays_one_level_for_middle_sessions() {
    // G -> M -> C: M is both a child of G and the parent of C. G's
    // breakdown carries M's OWN metrics only — the ledger itself is
    // one level (`subagent_cost` never rolls subagent_cost upward), so
    // the rows must not either.
    let mut g = metrics(10, 0, 0, 0, 1.0, 10.0);
    g.agent_name = String::new();
    let mut m = metrics(20, 2, 1, 1, 0.4, 20.0);
    m.agent_name = "middle".to_string();
    let mut c = metrics(30, 3, 0, 0, 0.1, 30.0);
    c.agent_name = "leaf".to_string();
    let mut sessions = vec![
        session("/p/g.jsonl", g),
        session("/p/g/subagents/agent-m.jsonl", m),
        session("/p/g/subagents/agent-m/subagents/agent-c.jsonl", c),
    ];
    attribute_subagents(&mut sessions);
    let top = &sessions[0];
    assert_eq!(top.metrics.subagent_count, 1);
    let labels = top
        .metrics
        .agent_breakdowns
        .iter()
        .map(|r| r.agent.as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["main", "middle"]);
    let middle_row = &top.metrics.agent_breakdowns[1];
    // M's row on G is M's own usage — NOT M + C (33 tokens, 0.5 cost).
    assert_eq!(
        (
            middle_row.tokens_regular,
            middle_row.tokens_cache_r,
            middle_row.tokens_cache_w
        ),
        (22, 1, 1)
    );
    assert!((middle_row.cost_usd - 0.4).abs() < 1e-9);
    // M itself keeps its own family rows.
    let mid_labels = sessions[1]
        .metrics
        .agent_breakdowns
        .iter()
        .map(|r| r.agent.as_str())
        .collect::<Vec<_>>();
    assert_eq!(mid_labels, vec!["middle", "leaf"]);
}

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within
/// this test binary (doctor_honesty_contract.rs pattern).
fn lock_env() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[test]
fn discovery_load_path_emits_breakdown_json() {
    let root = std::env::temp_dir().join(format!(
        "at-rm2076-{}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    let home = root.join("home");
    // Real layout convention (`parent_transcript_path`): a child at
    // `<dir>/subagents/agent-*.jsonl` links to the parent transcript at
    // `<dir>.jsonl` — the session directory and the transcript it owns
    // are siblings named after the session.
    let session_dir = root.join("P");
    std::fs::create_dir_all(session_dir.join("subagents")).expect("mkdir session");
    std::fs::create_dir_all(home.join("cache")).expect("mkdir sandbox cache");
    std::fs::write(
        root.join("P.jsonl"),
        concat!(
            r#"{"type":"user","timestamp":"2026-10-08T10:00:00Z","cwd":"/w","message":{"role":"user","content":"task"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-10-08T10:00:05Z","cwd":"/w","message":{"role":"assistant","model":"claude-sonnet-4-5-20250929","content":[{"type":"text","text":"ok"}],"usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":7,"cache_creation_input_tokens":5}}}"#,
            "\n"
        ),
    )
    .expect("write parent");
    std::fs::write(
        session_dir.join("subagents/agent-x.jsonl"),
        concat!(
            r#"{"type":"user","timestamp":"2026-10-08T10:01:00Z","cwd":"/w","message":{"role":"user","content":"subtask"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-10-08T10:01:10Z","cwd":"/w","message":{"role":"assistant","model":"claude-sonnet-4-5-20250929","content":[{"type":"text","text":"done"}],"usage":{"input_tokens":40,"output_tokens":4,"cache_read_input_tokens":3,"cache_creation_input_tokens":2}}}"#,
            "\n"
        ),
    )
    .expect("write child");

    let guard = lock_env();
    let saved: Vec<(&str, Option<std::ffi::OsString>)> = [
        "HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "AGENTTRACE_SESSION_CACHE_DIR",
    ]
    .iter()
    .map(|k| (*k, std::env::var_os(k)))
    .collect();
    for (k, _) in &saved {
        std::env::remove_var(k);
    }
    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_CACHE_HOME", home.join("cache"));
    std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
    let report = agenttrace_core::load_sessions_with_options(Some(&root), &Default::default());
    for (k, v) in &saved {
        match v {
            Some(v) => std::env::set_var(k, v),
            None => std::env::remove_var(k),
        }
    }
    drop(guard);

    assert_eq!(
        report.sessions.len(),
        2,
        "parent + linked child: {:?}",
        report
            .sessions
            .iter()
            .map(|s| s.path.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(report.unlinked_subagents, 0);
    let parent = report
        .sessions
        .iter()
        .find(|s| s.metrics.parent_session.is_empty())
        .expect("parent row");
    let child = report
        .sessions
        .iter()
        .find(|s| !s.metrics.parent_session.is_empty())
        .expect("child row");
    assert_eq!(parent.metrics.subagent_count, 1);
    // The parser fed the cache classes through (the parsed child's
    // own metrics are the source of truth for its row).
    assert_eq!(child.metrics.tokens_cache_r, 3);
    assert_eq!(child.metrics.tokens_cache_w, 2);
    let rows = &parent.metrics.agent_breakdowns;
    let labels = rows.iter().map(|r| r.agent.as_str()).collect::<Vec<_>>();
    assert_eq!(labels, vec!["main", "subagent"]);
    let main = &rows[0];
    assert_eq!(
        (
            main.tokens_cache_r,
            main.tokens_cache_w,
            main.tokens_regular
        ),
        (7, 5, 110)
    );
    let sub = &rows[1];
    assert_eq!(
        (sub.tokens_cache_r, sub.tokens_cache_w, sub.tokens_regular),
        (3, 2, 44)
    );
    let (r, w, reg, _) = class_sum(rows);
    assert_eq!(
        r + w + reg,
        agenttrace_core::total_tokens(parent).saturating_add(parent.metrics.subagent_tokens)
    );
    // The JSON lane (`--sessions -f json` serializes this struct
    // wholesale) carries the field for the linked parent only.
    let parent_json = serde_json::to_string(&parent.metrics).expect("serialize parent");
    let child_json = serde_json::to_string(&child.metrics).expect("serialize child");
    assert!(parent_json.contains("\"agent_breakdowns\""));
    assert!(!child_json.contains("agent_breakdowns"));

    std::fs::remove_dir_all(&root).ok();
}

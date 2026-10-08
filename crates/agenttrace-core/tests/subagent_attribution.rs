//! rm-545 / rm-487 golden contract: Claude Code subagent transcripts
//! (`<session>/subagents/agent-*.jsonl`) are attributed to the session
//! that spawned them, across parent / child / grandchild ("forked")
//! shapes, without ever double counting.
//!
//! The corpus (tests/fixtures/subagent-corpus) mirrors the PoC from the
//! orphan-stub probe: `p1-1111` spawns `agent-a1` and `agent-a2`;
//! `agent-a1` itself spawns `agent-g1` (grandchild — exercises the
//! chain); `lonely/subagents/agent-orphan.jsonl` has no parent on disk
//! (orphan — must stay unlinked, not crash); `other-session.jsonl` is
//! ordinary.

use agenttrace_core::{
    load_sessions_with_progress_from_cache, total_tokens, LoadOptions, Session, SessionCache,
};

fn corpus() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/subagent-corpus")
}

/// Loads the corpus through the real discovery pipeline with an
/// in-memory cache, so attribution is exercised exactly as the CLI
/// sees it (post history-merge, pre range-filter) without touching any
/// real cache on the host.
fn load() -> Vec<Session> {
    load_with_report().1
}

fn load_with_report() -> (agenttrace_core::LoadReport, Vec<Session>) {
    let mut cache = SessionCache::default();
    let report = load_sessions_with_progress_from_cache(
        Some(&corpus()),
        &LoadOptions::default(),
        &mut cache,
        |_| {},
    );
    assert!(
        report.sessions.len() >= 6,
        "corpus must fully discover (parent, 2 children, grandchild, unrelated, orphan): {}",
        report.sessions.len()
    );
    (report.clone(), report.sessions)
}

fn by_suffix<'a>(sessions: &'a [Session], suffix: &str) -> &'a Session {
    sessions
        .iter()
        .find(|session| session.path.ends_with(suffix))
        .unwrap_or_else(|| panic!("no session ends with {suffix}"))
}

#[test]
fn links_children_to_their_parent_transcript() {
    let sessions = load();
    let a1 = by_suffix(&sessions, "agent-a1.jsonl");
    assert!(
        a1.metrics.parent_session.ends_with("p1-1111.jsonl"),
        "child must name its parent transcript: {}",
        a1.metrics.parent_session
    );
    let a2 = by_suffix(&sessions, "agent-a2.jsonl");
    assert!(a2.metrics.parent_session.ends_with("p1-1111.jsonl"));
    // The grandchild links to its own spawning child, not the root
    // parent — chains resolve one hop, never transitively flattened.
    let g1 = by_suffix(&sessions, "agent-g1.jsonl");
    assert!(
        g1.metrics.parent_session.ends_with("agent-a1.jsonl"),
        "grandchild must link to its direct spawner: {}",
        g1.metrics.parent_session
    );
    // Unrelated sessions never gain a link.
    let other = by_suffix(&sessions, "other-session.jsonl");
    assert!(other.metrics.parent_session.is_empty());
}

#[test]
fn parent_rolls_up_children_without_double_counting() {
    let sessions = load();
    let parent = by_suffix(&sessions, "p1-1111.jsonl");
    let a1 = by_suffix(&sessions, "agent-a1.jsonl");
    let a2 = by_suffix(&sessions, "agent-a2.jsonl");
    let g1 = by_suffix(&sessions, "agent-g1.jsonl");

    // The parent's own metrics never absorb child work…
    assert_eq!(parent.metrics.cost_estimated, 0.0105);
    assert_eq!(total_tokens(parent), 1500);
    // …its rollup counts direct children only (2, not 3): the
    // grandchild's cost belongs to agent-a1's rollup, so nothing is
    // counted twice at any level.
    assert_eq!(parent.metrics.subagent_count, 2);
    let direct = a1.metrics.cost_estimated + a2.metrics.cost_estimated;
    assert!((parent.metrics.subagent_cost - direct).abs() < 1e-9);
    let direct_tokens = total_tokens(a1) + total_tokens(a2);
    assert_eq!(parent.metrics.subagent_tokens, direct_tokens);
    assert_eq!(a1.metrics.subagent_count, 1);
    assert!((a1.metrics.subagent_cost - g1.metrics.cost_estimated).abs() < 1e-9);

    // Fleet invariant (the real no-double-count): every child's own
    // cost lands in EXACTLY ONE parent rollup, and every rollup dollar
    // comes from a child. Rollups are additive views; own metrics are
    // never mutated by attribution.
    let rolled_by_parents: f64 = sessions.iter().map(|s| s.metrics.subagent_cost).sum();
    let own_by_children: f64 = sessions
        .iter()
        .filter(|s| !s.metrics.parent_session.is_empty())
        .map(|s| s.metrics.cost_estimated)
        .sum();
    assert!(
        (rolled_by_parents - own_by_children).abs() < 1e-9,
        "each child counted exactly once: rolled={rolled_by_parents} children={own_by_children}"
    );
}

#[test]
fn orphan_subagent_stays_unlinked_and_serde_stays_stable() {
    let (report, sessions) = load_with_report();
    let orphan = by_suffix(&sessions, "agent-orphan.jsonl");
    assert!(orphan.metrics.parent_session.is_empty());
    assert_eq!(orphan.metrics.subagent_count, 0);
    // rm-799: the orphan is COUNTED, not silently dropped — the CLI
    // turns this into a stderr disclosure.
    assert_eq!(
        report.unlinked_subagents, 1,
        "exactly the orphan counts as unlinked"
    );

    // The new metrics fields are skip_serializing_if-guarded, so the
    // historical JSON shape for ordinary sessions is byte-identical
    // (cache/history stability, rm-545 constraint).
    let other = by_suffix(&sessions, "other-session.jsonl");
    let serialized = serde_json::to_string(&other.metrics).expect("metrics serialize");
    assert!(
        !serialized.contains("subagent_count")
            && !serialized.contains("parent_session")
            && !serialized.contains("subagent_cost")
            // rm-790: same guard for the source-row key — sqlite lanes only.
            && !serialized.contains("session_key"),
        "ordinary sessions must not grow subagent noise in JSON: {serialized}"
    );
}

#[test]
fn tsv_session_list_carries_the_subagent_columns() {
    // rm-487 column contract: the --sessions TSV exposes SUBAGENTS and
    // SUBAGENT_COST so spawned work is visible without JSON.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let bin = manifest.join("../../target/release/agenttrace");
    if !bin.exists() {
        // Release binary may be absent in fresh checkouts (CI lane order
        // runs the test lane before the release build, and cargo artifact
        // GC can prune target/release mid-run); the column contract is
        // additionally pinned by the CLI unit test
        // session_list_tsv_carries_subagent_columns.
        eprintln!("release binary unavailable, skipping TSV leg");
        return;
    }
    let output = std::process::Command::new(bin)
        .arg("--sessions")
        .arg("--limit")
        .arg("10")
        .arg("-d")
        .arg(corpus())
        .env(
            "XDG_CACHE_HOME",
            std::env::temp_dir().join("at-subagent-tsv"),
        )
        .output()
        .expect("spawn release binary");
    if !output.status.success() {
        // A present binary that exits non-zero here is a real regression,
        // not an environment gap (absence was the only legitimate skip,
        // handled above) — surface status + stderr instead of skipping.
        panic!(
            "release binary exited {:?}; stderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let header = text.lines().next().unwrap_or_default();
    assert!(
        header.contains("SUBAGENTS") && header.contains("SUBAGENT_COST"),
        "TSV header must carry the subagent columns: {header}"
    );
    let parent_row = text
        .lines()
        .find(|line| line.contains("research x"))
        .expect("parent row in TSV");
    assert!(
        parent_row.split('\t').any(|cell| cell == "2"),
        "parent row must show its subagent count: {parent_row}"
    );
    // rm-799: the orphan in this corpus must be disclosed on stderr,
    // not silently rendered as a standalone row.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("1 subagent transcript(s) have no parent session"),
        "unlinked subagent disclosure on stderr, got: {stderr}"
    );
}

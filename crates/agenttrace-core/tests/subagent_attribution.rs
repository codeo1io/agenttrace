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

/// Serializes tests that mutate the process-global AGENTTRACE_HISTORY_DIR
/// across this binary's parallel lanes (same shape as the in-crate
/// `test_env::lock_env`). Today the retention test is the only
/// history-touching test and is safe even unlocked — the other five load
/// with both history flags false (LoadOptions::default) — but a second
/// history test would race the save/restore at scope end; the lock makes
/// that future safe instead of latent (independent_review 355e05af).
static HISTORY_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    load_with_report_at(&corpus(), &LoadOptions::default())
}

fn load_dir(dir: &std::path::Path) -> Vec<Session> {
    load_with_report_at(dir, &LoadOptions::default()).1
}

fn load_with_options(dir: &std::path::Path, options: &LoadOptions) -> Vec<Session> {
    load_with_report_at(dir, options).1
}

fn load_with_report_at(
    dir: &std::path::Path,
    options: &LoadOptions,
) -> (agenttrace_core::LoadReport, Vec<Session>) {
    let mut cache = SessionCache::default();
    let report = load_sessions_with_progress_from_cache(Some(dir), options, &mut cache, |_| {});
    assert!(
        !report.sessions.is_empty(),
        "dir {} must discover at least one session",
        dir.display()
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

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("create target dir");
    for entry in std::fs::read_dir(from).expect("read source dir") {
        let entry = entry.expect("dir entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

#[test]
fn links_same_dir_sidechains_by_journal_session_id() {
    // rm-487 escalated arm (rider to rm-787): Claude Code 2.x parks
    // sidechains NEXT TO their parent — `agent-*.jsonl` beside
    // `<uuid>.jsonl`, no `subagents/` directory at all — and the
    // journal `sessionId` names the parent file's stem. That is the
    // layout this host actually writes (220 same-dir sidechains across
    // 7 projects, 0 `subagents/` directories, so the rm-545 path rule
    // links nothing there; join key proven live on
    // -work-ci-fixes-glmplus). The fixture mirrors the live shape,
    // `isSidechain` markers included.
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/subagent-samedir");
    let sessions = load_dir(&dir);
    assert!(sessions.len() >= 3, "fixture must fully discover");
    let parent = by_suffix(&sessions, "51207be0-1a2b-4c3d-9e8f-aabbccddeeff.jsonl");
    let side = by_suffix(&sessions, "agent-51207be0-side1.jsonl");
    assert_eq!(
        side.metrics.parent_session, parent.path,
        "same-dir sidechain must name the parent its journal sessionId spells"
    );
    assert_eq!(parent.metrics.subagent_count, 1);
    assert!((parent.metrics.subagent_cost - side.metrics.cost_estimated).abs() < 1e-9);
    // The parent's own metrics never absorb the child (no-double-count).
    assert_eq!(parent.metrics.cost_estimated, 0.0105);
    // A sidechain naming a parent that is not loaded stays unlinked,
    // never invented.
    let lonely = by_suffix(&sessions, "agent-9f9f9f9f-lonely.jsonl");
    assert!(lonely.metrics.parent_session.is_empty());
    assert_eq!(lonely.metrics.subagent_count, 0);
}

#[test]
fn preserved_history_rows_keep_their_subagent_rollups() {
    // rm-787 regression, the assess-13095ad3 PoC end to end through the
    // real pipeline: with every transcript live, --preserve-history
    // renders the parent `research x` with SUBAGENTS=2 / 0.0053; once
    // the parent transcript is gone (fresh cache), --include-history
    // used to restore it as `history-xxxx ... 0 0.0000` — the preserved
    // record carried no rollups and the synthetic `history:{id}` path
    // can never re-join the real-path attribution index, so the
    // retention view contradicted the live view on the same corpus.
    // The rollups must travel with the record.
    let root = std::env::temp_dir().join(format!("agenttrace-rm787-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let full = root.join("full");
    copy_dir(&corpus(), &full);
    let pruned = root.join("pruned");
    copy_dir(&corpus(), &pruned);
    std::fs::remove_file(pruned.join("p1-1111.jsonl"))
        .expect("remove the parent transcript (retention scenario)");

    // Serialize the process-global history env mutation (see the
    // HISTORY_ENV_LOCK doc above).
    let _history_env_guard = HISTORY_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let prior_history = std::env::var_os("AGENTTRACE_HISTORY_DIR");
    std::env::set_var("AGENTTRACE_HISTORY_DIR", &root);

    // Phase 1 — every transcript live: preserve WITH the rollups the
    // live view showed. This is the red-first pin for the pre-preserve
    // attribution: without it, the record stored zeros here.
    let preserve = LoadOptions {
        preserve_history: true,
        ..LoadOptions::default()
    };
    let live = load_with_options(&full, &preserve);
    let parent = by_suffix(&live, "p1-1111.jsonl");
    assert_eq!(parent.metrics.subagent_count, 2);
    let live_rollup = parent.metrics.subagent_cost;
    assert!(
        (live_rollup - 0.0053).abs() < 1e-9,
        "pinned PoC rollup cost"
    );

    // Phase 2 — parent transcript gone, fresh cache: the restored row
    // keeps the preserved rollups instead of rendering zeros.
    let include = LoadOptions {
        include_history: true,
        ..LoadOptions::default()
    };
    let restored = load_with_options(&pruned, &include);
    let history: Vec<&Session> = restored
        .iter()
        .filter(|session| session.path.starts_with("history:"))
        .collect();
    assert_eq!(
        history.len(),
        1,
        "exactly the deleted parent restores; the still-live children dedupe by identity"
    );
    let row = history[0];
    assert!(row.name.starts_with("history-"));
    assert_eq!(
        row.metrics.subagent_count, 2,
        "preserved rollup count must survive the retention round-trip"
    );
    assert!((row.metrics.subagent_cost - live_rollup).abs() < 1e-12);

    // Live rows still recompute from scratch: the orphaned children
    // link to nothing, and agent-a1 keeps its grandchild rollup.
    let a1 = by_suffix(&restored, "agent-a1.jsonl");
    assert_eq!(a1.metrics.subagent_count, 1);
    assert!(by_suffix(&restored, "agent-a2.jsonl")
        .metrics
        .parent_session
        .is_empty());

    match prior_history {
        Some(value) => std::env::set_var("AGENTTRACE_HISTORY_DIR", value),
        None => std::env::remove_var("AGENTTRACE_HISTORY_DIR"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn hostile_overflow_corpus_saturates_parent_rollups() {
    // rm-487 end-to-end: the assess-PoC corpus class (run e775cac5
    // cycle 1) — two children each carrying i64::MAX/2 + 1 input
    // tokens — panicked debug builds at the plain `+=` rollup
    // (subagents.rs) and would wrap release totals negative. Through
    // the real discovery pipeline the parent must saturate instead:
    // no panic, both children linked, tokens pinned at i64::MAX, and a
    // finite (never poisoned) cost rollup.
    let sessions = load_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/subagent-overflow"),
    );
    let parent = by_suffix(&sessions, "p2-4242.jsonl");
    assert_eq!(parent.metrics.subagent_count, 2);
    assert_eq!(
        parent.metrics.subagent_tokens,
        i64::MAX,
        "hostile rollup must saturate, not wrap"
    );
    assert!(parent.metrics.subagent_cost.is_finite());
    assert!(parent.metrics.subagent_cost > 0.0);
    let h1 = by_suffix(&sessions, "agent-h1.jsonl");
    let h2 = by_suffix(&sessions, "agent-h2.jsonl");
    assert!(h1.metrics.parent_session.ends_with("p2-4242.jsonl"));
    assert!(h2.metrics.parent_session.ends_with("p2-4242.jsonl"));
}

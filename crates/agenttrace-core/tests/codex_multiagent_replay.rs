//! rm-402 — Codex MultiAgent V2: replayed parent prefix double-counts across files.
//!
//! A multi-agent child rollout replays the parent's transcript as its opening
//! context: the child's own token accounting starts "from zero" but its
//! cumulative `total_token_usage` climbs through the replayed prefix, so a
//! corpus that simply sums per-rollout usage bills the prefix twice. The
//! acceptance PoC: parent final 1,400 + child final 1,700 = 3,100 reported vs
//! 1,700 true (+82%).
//!
//! The fix is corpus-level: children tied to a parent by identical opening
//! `response_item` content inherit the parent's final snapshot as their
//! baseline (saturating, per field); the replayed prefix stays counted once
//! and attributed to the parent. Single-file codex corpora must stay
//! byte-identical before/after (no family, no adjustment).

use agenttrace_core::{
    parse_raw_session, reconcile_codex_multiagent_replays,
    reconcile_codex_multiagent_replays_with_store, RolloutScanStore,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codex-multiagent-v2")
        .join(name)
}

fn load(name: &str) -> agenttrace_core::Session {
    let path = fixture(name);
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let path_str = path.to_str().expect("utf-8 fixture path");
    parse_raw_session("codex", path_str, &raw).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

fn child_adjusted_counters(session: &agenttrace_core::Session) -> Option<usize> {
    session
        .metrics
        .disclosure_counters
        .get("codex_multiagent_replayed_child_adjusted")
        .copied()
}

#[test]
fn replayed_prefix_is_counted_once_and_attributed_to_the_parent() {
    let parent = load("parent.jsonl");
    let child = load("child.jsonl");

    // Pre-fix corpus arithmetic: the naive sum double-counts the replayed
    // prefix (parent 1,400 + child 1,700 = 3,100 vs 1,700 true, +82%).
    let naive = parent.metrics.tokens_input + child.metrics.tokens_input;
    assert_eq!(
        naive, 3_100,
        "PoC shape: naive sum double-counts the prefix"
    );

    let mut sessions = vec![parent, child];
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.families, 1, "one replay family detected");
    assert_eq!(stats.children_adjusted, 1);
    assert_eq!(
        stats.tokens_input_dropped, 1_400,
        "the replayed 1,400 is dropped from the child"
    );

    // Parent keeps its full attribution; the child reports only its delta.
    assert_eq!(sessions[0].metrics.tokens_input, 1_400);
    assert_eq!(
        sessions[1].metrics.tokens_input, 300,
        "child inherits the parent's final snapshot as baseline"
    );

    // Family true total = 1,700.
    let family: i64 = sessions.iter().map(|s| s.metrics.tokens_input).sum();
    assert_eq!(family, 1_700);

    // Disclosure rides the child only, via the existing counter channel.
    assert!(child_adjusted_counters(&sessions[0]).is_none());
    assert_eq!(child_adjusted_counters(&sessions[1]), Some(1));
    assert_eq!(
        sessions[1]
            .metrics
            .disclosure_counters
            .get("codex_multiagent_replayed_tokens_dropped"),
        Some(&1_400)
    );
}

#[test]
fn single_file_codex_corpus_is_unchanged() {
    // A lone rollout has no family: usage must stay exactly as parsed.
    let mut sessions = vec![load("parent.jsonl")];
    let before = sessions[0].metrics.tokens_input;
    assert_eq!(before, 1_400);
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.families, 0);
    assert_eq!(stats.children_adjusted, 0);
    assert_eq!(sessions[0].metrics.tokens_input, 1_400);
    assert!(child_adjusted_counters(&sessions[0]).is_none());
}

#[test]
fn child_that_outgrows_a_shorter_parent_reports_only_its_delta() {
    // The parent stops at 1,400 while the child continues to 1,700 — the
    // child's own work is the 300-token delta above the inherited baseline.
    let mut sessions = vec![load("parent.jsonl"), load("child.jsonl")];
    // Order in the vec must not matter: the parent is the earliest start.
    sessions.reverse();
    let _ = reconcile_codex_multiagent_replays(&mut sessions);
    let family: i64 = sessions.iter().map(|s| s.metrics.tokens_input).sum();
    assert_eq!(family, 1_700);
    let counts: Vec<i64> = sessions.iter().map(|s| s.metrics.tokens_input).collect();
    assert!(counts.contains(&1_400) && counts.contains(&300));
}

/// Review af521003 F1: the session cache's serde round-trip drops
/// `metrics.timestamps` (`#[serde(skip)]`), so every warm-run member used
/// to tie at MIN_UTC and parent/child roles fell to directory-walk order.
/// The reconciliation now re-derives each member's start from the rollout
/// FILE's own first timestamp — roles must be stable with the session
/// timestamps stripped (the exact warm-cache state).
#[test]
fn warm_cache_without_session_timestamps_keeps_roles_stable() {
    let mut sessions = vec![load("parent.jsonl"), load("child.jsonl")];
    // Simulate the serde skip: the cache holds no timestamps.
    for session in &mut sessions {
        session.metrics.timestamps.clear();
    }
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.families, 1);
    assert_eq!(stats.children_adjusted, 1);
    let counts: Vec<i64> = sessions.iter().map(|s| s.metrics.tokens_input).collect();
    assert!(
        counts.contains(&1_400) && counts.contains(&300),
        "warm-cache roles inverted: {counts:?}"
    );
    // And the same corpus fed in the reverse order resolves identically
    // (the canonical-path tie-break keeps ties deterministic).
    let mut reversed = vec![load("child.jsonl"), load("parent.jsonl")];
    for session in &mut reversed {
        session.metrics.timestamps.clear();
    }
    let _ = reconcile_codex_multiagent_replays(&mut reversed);
    let mut counts2: Vec<i64> = reversed.iter().map(|s| s.metrics.tokens_input).collect();
    counts2.sort_unstable();
    let mut counts_sorted = counts.clone();
    counts_sorted.sort_unstable();
    assert_eq!(
        counts_sorted, counts2,
        "order drifted under stripped timestamps"
    );
    // The child — not the parent — is the adjusted member in both orders.
    for corpus in [&sessions, &reversed] {
        let adjusted = corpus
            .iter()
            .find(|s| {
                s.metrics
                    .disclosure_counters
                    .iter()
                    .any(|(k, _)| k.as_str() == "codex_multiagent_replayed_child_adjusted")
            })
            .expect("an adjusted child");
        assert_eq!(
            adjusted.metrics.tokens_input, 300,
            "child adjusted, parent intact"
        );
    }
}

/// Review af521003 F2: a rebased child's cost must use the exact per-Mtok
/// catalog formula the analyze pass applies — each class /1e6 at its own
/// rate. The previous per-token multiplication inflated a 300-token child
/// to a $1,500.0000-class estimate.
#[test]
fn rebased_child_cost_uses_the_canonical_per_mtok_formula() {
    let mut sessions = vec![load("parent.jsonl"), load("child.jsonl")];
    let _ = reconcile_codex_multiagent_replays(&mut sessions);
    let child = sessions
        .iter()
        .find(|s| s.metrics.tokens_input == 300)
        .expect("rebased child");
    let price = agenttrace_core::lookup_price_tiered(
        &child.metrics.model_used,
        child.metrics.service_tier.as_deref(),
    );
    // rm-1508 seam at the 2026-10-17 integration (case 7f566ee8): cw/cr
    // are Option in the absence-preserving catalog — an unpublished rate
    // bills at the disclosed fallback ladder, mirroring rebase_cost.
    let (cw, _) = price.cache_write_billed();
    let (cr, _) = price.cache_read_billed();
    let expected = agenttrace_core::round4(
        child.metrics.tokens_input as f64 / 1e6 * price.input
            + child.metrics.tokens_output as f64 / 1e6 * price.output
            + child.metrics.tokens_cache_w as f64 / 1e6 * cw
            + child.metrics.tokens_cache_r as f64 / 1e6 * cr,
    );
    assert_eq!(child.metrics.cost_estimated, expected);
    // The x1e6 regression shape: the child's cost must be a per-Mtok
    // fraction, not a per-token multiple.
    assert!(
        child.metrics.cost_estimated < 1.0,
        "rebased cost inflated: {}",
        child.metrics.cost_estimated
    );
    // The parent's parser-derived cost is untouched by reconciliation.
    let parent = sessions
        .iter()
        .find(|s| s.metrics.tokens_input == 1_400)
        .expect("parent");
    let parent_price = agenttrace_core::lookup_price_tiered(
        &parent.metrics.model_used,
        parent.metrics.service_tier.as_deref(),
    );
    let parent_expected =
        agenttrace_core::round4(parent.metrics.tokens_input as f64 / 1e6 * parent_price.input);
    assert!(
        (parent.metrics.cost_estimated - parent_expected).abs() < 1e-9,
        "parent cost drifted: {}",
        parent.metrics.cost_estimated
    );
}

fn load_dir(dir: &str, name: &str) -> agenttrace_core::Session {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(dir)
        .join(name);
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let path_str = path.to_str().expect("utf-8 fixture path");
    parse_raw_session("codex", path_str, &raw).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

/// Review af521003 F3: the concurrent-spawn shape — the child spawned
/// mid-parent and replayed the parent's EARLIER state, so the parent's
/// final snapshot (1,400) exceeds the child's (1,300) and is not a valid
/// baseline. The child must keep its full usage and disclose distinctly,
/// never be saturated to zero under a replay label.
#[test]
fn concurrent_spawn_child_is_left_intact_and_disclosed() {
    let mut sessions = vec![
        load_dir("codex-multiagent-v2-concurrent", "parent.jsonl"),
        load_dir("codex-multiagent-v2-concurrent", "child.jsonl"),
    ];
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.families, 1, "shared 590-byte opening is a family");
    assert_eq!(stats.children_below_baseline, 1);
    assert_eq!(stats.children_adjusted, 0);
    let counts: Vec<i64> = sessions.iter().map(|s| s.metrics.tokens_input).collect();
    assert!(
        counts.contains(&1_400) && counts.contains(&1_300),
        "{counts:?}"
    );
    for session in &sessions {
        if session.metrics.tokens_input == 1_300 {
            assert_eq!(
                session
                    .metrics
                    .disclosure_counters
                    .iter()
                    .find(|(k, _)| k.as_str() == "codex_multiagent_replay_skipped_below_baseline")
                    .map(|(_, v)| *v),
                Some(1),
                "below-baseline child must disclose distinctly"
            );
            assert_eq!(
                session
                    .metrics
                    .disclosure_counters
                    .iter()
                    .find(|(k, _)| k.as_str() == "codex_multiagent_replayed_tokens_dropped")
                    .map(|(_, v)| *v),
                None,
                "below-baseline child must not be mislabeled as replayed tokens"
            );
        }
    }
}

/// Integration case 4404ce94 (semantic compose with landed rm-992): Codex
/// ≥ 0.152 writes rollouts as zstd frames, and the zstd decode lane landed
/// after this run's base — so the scanner originally read compressed files
/// as raw bytes and every line failed UTF-8, leaving rm-402 silently inert
/// on the wire format Codex actually writes. The scanner now shares the
/// parser's bounded decoder, and this pin holds the seam: a zstd family
/// reconciles identically to its uncompressed twin, and family identity is
/// content-based, so a mixed zstd-parent / plain-child corpus also ties.
#[test]
fn zstd_rollout_family_reconciles_like_the_uncompressed_twin() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex-multiagent-v2-zstd");
    let parent_path = dir.join("parent.jsonl");
    let child_path = dir.join("child.jsonl");

    // Guard the fixtures themselves: they must really be zstd frames, or
    // this test silently degrades into testing the plain path (the same
    // guard rm-992's golden parity test uses).
    for fixture in [&parent_path, &child_path] {
        let bytes = std::fs::read(fixture).unwrap();
        assert_eq!(
            &bytes[..4],
            &[0x28, 0xB5, 0x2F, 0xFD],
            "fixture {} must be a zstd frame",
            fixture.display()
        );
    }

    let parent = agenttrace_core::parse_file(&parent_path).expect("zstd parent decodes and parses");
    let child = agenttrace_core::parse_file(&child_path).expect("zstd child decodes and parses");
    assert_eq!(
        parent.metrics.tokens_input + child.metrics.tokens_input,
        3_100,
        "PoC shape survives the container: naive sum double-counts"
    );

    let mut sessions = vec![parent, child];
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.families, 1, "zstd frames must not hide the family");
    assert_eq!(stats.children_adjusted, 1);
    assert_eq!(stats.tokens_input_dropped, 1_400);
    let counts: Vec<i64> = sessions.iter().map(|s| s.metrics.tokens_input).collect();
    assert_eq!(
        counts,
        vec![1_400, 300],
        "zstd family reconciles to the same attribution as the twin"
    );
    assert_eq!(child_adjusted_counters(&sessions[1]), Some(1));

    // Mixed containers: the fingerprint hashes the PAYLOAD, not the file
    // bytes, so a zstd parent and its uncompressed child still form one
    // family — the encoding never decides attribution.
    let mut mixed = vec![
        agenttrace_core::parse_file(&parent_path).unwrap(),
        load("child.jsonl"),
    ];
    let mixed_stats = reconcile_codex_multiagent_replays(&mut mixed);
    assert_eq!(mixed_stats.families, 1, "mixed containers are one family");
    let mixed_counts: Vec<i64> = mixed.iter().map(|s| s.metrics.tokens_input).collect();
    assert_eq!(mixed_counts, vec![1_400, 300]);
}

/// Review af521003 F4: two unrelated sessions that merely share a short
/// 2-item opening (293 bytes — the prompt-retry / templated-first-turns
/// shape) must NOT merge into a family; both keep their full usage with
/// no replay disclosures.
#[test]
fn unrelated_short_opening_is_not_a_family() {
    let mut sessions = vec![
        load_dir("codex-multiagent-v2-unrelated", "a.jsonl"),
        load_dir("codex-multiagent-v2-unrelated", "b.jsonl"),
    ];
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.families, 0, "short shared opening must not qualify");
    assert_eq!(stats.children_adjusted, 0);
    let counts: Vec<i64> = sessions.iter().map(|s| s.metrics.tokens_input).collect();
    assert!(counts.contains(&100) && counts.contains(&200), "{counts:?}");
    for session in &sessions {
        assert!(
            session
                .metrics
                .disclosure_counters
                .iter()
                .all(|(k, _)| !k.starts_with("codex_multiagent_replay")),
            "non-family session must carry no replay disclosures"
        );
    }
}

// ---------------------------------------------------------------------------
// rm-1980 — rerun discrimination, keyless fallback, unreadable disclosure.
// ---------------------------------------------------------------------------

fn fixture_in(dir: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(dir)
        .join(name)
}

fn load_in(dir: &str, name: &str) -> agenttrace_core::Session {
    let path = fixture_in(dir, name);
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let path_str = path.to_str().expect("utf-8 fixture path");
    parse_raw_session("codex", path_str, &raw).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn independent_rerun_with_shared_opening_is_not_rebased() {
    // Two INDEPENDENT runs of the same deterministic task: identical
    // >384-byte openings, but fresh rollout uuids (77c2e4ff vs 33ab90cc)
    // and disjoint time windows. The old heuristic rebased the second run
    // onto the first and erased its real usage; the trace-stem veto keeps
    // both fully counted and discloses instead.
    let first = load_in("codex-multiagent-v2-rerun", "first.jsonl");
    let second = load_in("codex-multiagent-v2-rerun", "second.jsonl");
    let second_input = second.metrics.tokens_input;

    let mut sessions = vec![first, second];
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.families, 1, "the opening still forms a family");
    assert_eq!(stats.divergent_trace_families, 1);
    assert_eq!(stats.children_adjusted, 0, "no rebase happened");
    assert_eq!(
        sessions[1].metrics.tokens_input, second_input,
        "the rerun keeps every token it really burned"
    );
    assert_eq!(
        sessions[0].metrics.tokens_input, 1_400,
        "the earlier run is untouched"
    );
    for idx in [0usize, 1] {
        assert_eq!(
            sessions[idx]
                .metrics
                .disclosure_counters
                .get("codex_multiagent_replay_skipped_divergent_trace"),
            Some(&1),
            "member {idx} discloses the veto"
        );
    }
}

#[test]
fn keyless_family_falls_back_to_fingerprint_with_disclosure() {
    // Truncated rollouts carry no session_meta id. The fingerprint-only
    // fallback keeps the shipped reconciliation, but the family is no
    // longer silent about lacking id evidence.
    let a = load_in("codex-multiagent-v2-unkeyed", "a.jsonl");
    let b = load_in("codex-multiagent-v2-unkeyed", "b.jsonl");

    let mut sessions = vec![a, b];
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.families, 1);
    assert_eq!(stats.unkeyed_families, 1);
    assert_eq!(stats.divergent_trace_families, 0);
    assert_eq!(
        stats.children_adjusted, 1,
        "fingerprint fallback still rebases"
    );
    assert_eq!(sessions[1].metrics.tokens_input, 300);
    for idx in [0usize, 1] {
        assert_eq!(
            sessions[idx]
                .metrics
                .disclosure_counters
                .get("codex_multiagent_replay_family_unkeyed"),
            Some(&1)
        );
    }
}

#[test]
fn unreadable_rollout_at_reconcile_time_is_disclosed() {
    // A rollout that parses fine but disappears (or loses permissions)
    // before reconcile used to render the corpus silently unreconciled.
    // rm-1980 F3: the scan failure is now a disclosed counter.
    let dir = std::env::temp_dir().join(format!("codex-replay-unreadable-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let parent_path = dir.join("parent.jsonl");
    let child_path = dir.join("child.jsonl");
    std::fs::copy(fixture("parent.jsonl"), &parent_path).unwrap();
    std::fs::copy(fixture("child.jsonl"), &child_path).unwrap();

    // Sessions parsed from the TMP copies, so the fixture originals stay
    // readable and only the vanished file drops out at reconcile time.
    let raw_p = std::fs::read_to_string(&parent_path).unwrap();
    let raw_c = std::fs::read_to_string(&child_path).unwrap();
    let parent = parse_raw_session("codex", parent_path.to_str().unwrap(), &raw_p).unwrap();
    let child = parse_raw_session("codex", child_path.to_str().unwrap(), &raw_c).unwrap();
    let mut sessions = vec![parent, child];

    // The child's file vanishes between parse and reconcile.
    std::fs::remove_file(&child_path).unwrap();
    let stats = reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(stats.scans_unreadable, 1);
    assert_eq!(stats.families, 0, "no evidence, no family, no rebase");
    assert_eq!(sessions[1].metrics.tokens_input, 1_700, "usage intact");
    assert_eq!(
        sessions[1]
            .metrics
            .disclosure_counters
            .get("codex_replay_scan_unreadable"),
        Some(&1)
    );
    // Only the surviving parent reaches the scan path; the vanished
    // child is disclosed before any read is attempted.
    assert_eq!(stats.scan_cache_misses, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scan_store_warm_run_hits_the_memo_and_changes_nothing() {
    // rm-1979: the second pass over an unchanged corpus must serve every
    // scan from the on-disk memo (zero re-reads) while producing the
    // exact same adjustment and session totals as the cold pass.
    let dir = std::env::temp_dir().join(format!("codex-replay-store-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["parent.jsonl", "child.jsonl"] {
        std::fs::copy(fixture(name), dir.join(name)).unwrap();
    }
    let store_path = dir.join("codex-rollout-scans.json");
    let _ = std::fs::remove_file(&store_path);

    let build = || -> Vec<agenttrace_core::Session> {
        let raw_p = std::fs::read_to_string(dir.join("parent.jsonl")).unwrap();
        let raw_c = std::fs::read_to_string(dir.join("child.jsonl")).unwrap();
        let p = dir.join("parent.jsonl");
        let c = dir.join("child.jsonl");
        vec![
            parse_raw_session("codex", p.to_str().unwrap(), &raw_p).unwrap(),
            parse_raw_session("codex", c.to_str().unwrap(), &raw_c).unwrap(),
        ]
    };

    // Cold pass: every scan is a miss, results recorded and persisted.
    let mut sessions = build();
    let mut store = RolloutScanStore::load(&store_path);
    let cold = reconcile_codex_multiagent_replays_with_store(&mut sessions, Some(&mut store));
    assert_eq!(cold.scan_cache_misses, 2);
    assert_eq!(cold.scan_cache_hits, 0);
    assert_eq!(cold.families, 1);
    assert_eq!(sessions[1].metrics.tokens_input, 300);
    store.save_if_dirty(&store_path).unwrap();

    // Warm pass: unchanged files hydrate from the memo; the family
    // decision, drops, and totals come out identical.
    let mut sessions = build();
    let mut store = RolloutScanStore::load(&store_path);
    let warm = reconcile_codex_multiagent_replays_with_store(&mut sessions, Some(&mut store));
    assert_eq!(warm.scan_cache_hits, 2, "no rollout file was re-read");
    assert_eq!(warm.scan_cache_misses, 0);
    assert_eq!(warm.families, cold.families);
    assert_eq!(warm.children_adjusted, cold.children_adjusted);
    assert_eq!(warm.tokens_input_dropped, cold.tokens_input_dropped);
    assert_eq!(sessions[1].metrics.tokens_input, 300);
    assert!(!store.is_dirty(), "warm pass records nothing new");
    let _ = std::fs::remove_dir_all(&dir);
}

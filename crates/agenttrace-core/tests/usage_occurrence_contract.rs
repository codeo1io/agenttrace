//! rm-711/rm-710/rm-714 occurrence-aware usage contract tests.
//!
//! Selected batch "Truthful usage accounting across parse → cache →
//! report" (run 73fe8e1e, prioritize 312a2133, roadmap rm-710/711/714):
//!
//! - rm-711 LEAD: usage is occurrence-aware. Codex post-compaction
//!   token_count windows count their fresh `last` snapshots instead of
//!   colliding with totals already counted pre-compaction (assess SL1:
//!   the codex-revisit corpus reported 2000 of a truthful 2700), the
//!   qwen per-turn result usage accumulates instead of latching on the
//!   first record (assess SL2: the qwen-multi corpus reported 15 of a
//!   truthful 45), and the rm-554 rate-limit-only guard survives.
//! - rm-710: SESSION_CACHE_SCHEMA_VERSION 32 → 33 so warm caches built
//!   under the old accounting (and pre-rm-526 v32-era entries that
//!   replay hidden `line_skips` at confidence "high", assess NN2)
//!   regenerate instead of being served. Re-based at integration
//!   (conflict case 2ac5bbe1, run 73fe8e1e landed 2026-10-08) onto
//!   the advanced ceiling as 33 → 34 — 33 was taken by the landed
//!   rm-616 generic-lane batch — so this oracle pins the live
//!   constant against the same degraded-to-32 class (the ceiling
//!   advanced again to 35 at the rm-720/rm-721 agent-lane
//!   integration, conflict case 680aa6a6, run 3ec6cec08fb9 landed
//!   2026-10-08, per the same re-base rule).
//! - rm-714: the delivery-evidence disclaimer is hoisted to one
//!   document-level field instead of being re-serialized into every
//!   row (assess N3: ~155 bytes × 24,501 rows ≈ 3.8 MB of repeats in a
//!   real `--overview -o report.json`).

use agenttrace_core::{
    cached_session, data_health, delivery_evidence, load_session_cache, parse_file,
    save_session_cache, session_cache_path, store_session, Session, SessionCache,
    SESSION_CACHE_SCHEMA_VERSION,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

// ---------------------------------------------------------------------------
// rm-711 — codex occurrence-aware accounting (assess SL1)
// ---------------------------------------------------------------------------

/// The codex-revisit corpus: three fresh calls (1200/400, 300/100,
/// 1200/400 after a `compacted` marker re-bases the counters to a value
/// already seen pre-compaction). Truth: 2700 input / 900 output. The
/// pre-rm-711 value-dedup dropped the whole post-compaction window
/// because its rebased total collided with window 1 (reported 2000).
#[test]
fn codex_post_compaction_revisit_counts_fresh_last_usage() {
    let session = parse_file(&fixture("codex-revisit/rollout-2026-10-06T05-00-00.jsonl"))
        .expect("codex-revisit parses");
    assert_eq!(session.metrics.tokens_input, 2700);
    assert_eq!(session.metrics.tokens_output, 900);
}

/// The same running total re-emitted with no fresh call (rate-limit-only
/// update) still counts nothing — the rm-554 guard must survive rm-711's
/// envelope scoping. Two identical token_count windows: first counts,
/// re-emission is free.
#[test]
fn codex_rate_limit_reemission_stays_free() {
    let window = r#"{"timestamp":"2026-10-06T06:00:00.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"output_tokens":500},"last_token_usage":{"input_tokens":1000,"output_tokens":500}}}}"#;
    let dir = std::env::temp_dir().join("agenttrace-contract-ratelimit");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("rollout-2026-10-06T06-00-00.jsonl");
    fs::write(
        &path,
        format!(
            "{{\"timestamp\":\"2026-10-06T06:00:00.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"sess-rl\",\"timestamp\":\"2026-10-06T06:00:00.000Z\",\"cwd\":\"/home/a/p\",\"originator\":\"codex_cli_rs\",\"cli_version\":\"0.1.0\",\"instructions\":\"x\"}}}}\n{window}\n{window}\n"
        ),
    )
    .unwrap();
    let session = parse_file(&path).expect("rate-limit journal parses");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(session.metrics.tokens_output, 500);
}

// ---------------------------------------------------------------------------
// rm-711 — qwen per-turn accumulation (assess SL2)
// ---------------------------------------------------------------------------

/// The qwen-multi corpus: three turns of (10 input / 5 output). Truth:
/// 30/45 tokens. The pre-rm-711 session-wide first-wins latch kept only
/// the first turn's result usage (reported 15).
#[test]
fn qwen_multi_result_usage_accumulates_per_turn() {
    let session = parse_file(&fixture("qwen-multi/qwen-session.jsonl")).expect("qwen-multi parses");
    assert_eq!(session.metrics.tokens_input, 30);
    assert_eq!(session.metrics.tokens_output, 15);
}

// ---------------------------------------------------------------------------
// rm-710 — schema bump makes warm disclosure truthful (assess NN2)
// ---------------------------------------------------------------------------

/// One torn-tail claude journal: the cold parse discloses
/// `line_skips: {unparseable_line: 1}` and degrades confidence to
/// "low". A warm hit at the current schema must replay the SAME
/// disclosure (line_skips round-trip through the cache entry since
/// rm-526), and a hand-degraded schema-32 cache file must NOT be served
/// — that is the class of pre-rm-526 v32-era entries replaying hidden
/// skips at confidence "high", and of v32 totals undercounted by the
/// pre-rm-711 parsers.
#[test]
fn warm_cache_replays_cold_disclosure_and_schema_57_invalidates_v32() {
    assert_eq!(
        SESSION_CACHE_SCHEMA_VERSION, 57,
        "rm-710: this oracle pins the bump (landed 32 → 33 at the run's base, re-based at integration onto the advanced ceiling as 33 → 34; the ceiling advanced to 35 at the rm-720/rm-721 agent-lane integration, to 36 at the rm-502 timestamp-unification integration, to 37 at the rm-754 priced-loop-cost integration, to 38 at the rm-718/rm-719/rm-716 disclosure-plane-honesty integration, to 39 at the rm-834 report-truthfulness stream-fold integration, to 40 at the rm-585/rm-730 honest-attribution integration, to 41 at the rm-520/rm-521 recorded-cost-basis integration (run ac14e52c, conflict case c1c77f5e), to 42 at the journal-truth integration (run 14954d7a, rm-880 + the rm-406 dated 2026-10-07 arm — Metrics persist `wire_metadata` and `model_attribution` through the Go mirror, conflict case 8b8d096a), to 43 at the hostile-value-fold integration (run fabd9fb8cf73, rm-831 + the rm-721 non-finite-credit rider — fold saturation, negative-insert refusal and the copilot_credit_nonfinite disclosure, landed at its base dc65644 as 35 → 36 and re-based here per the same convention), to 44 at the order-independent codex failure-attribution integration (run fb1addd56503 — its campaign-local rm-585 ordering half, folded onto the landed rm-584 disclosure row; the batch's own 27 → 28 bump re-based onto the advanced ceiling, conflict case e0a11c511c6344f4af8cf63ae9ed485d), and to 45 at the truthful-accounting integration (run 33b7b7b5, rm-617 + rm-899 — the codex rollout reasoning breakdown rides its own tokens_reasoning line instead of billing output twice, and aider's DST-ambiguous starts resolve deterministically via earliest(); the run minted 43 above its base 9c3c599's ceiling 41 and re-based twice at review-fix — 44 at ed90e5ec after origin landed fabd9fb8's 43, 45 at b5b9c6fc after origin landed fb1addd56503's 44 — so 45 lands here as the lowest rung unique above the ceiling, conflict case f2d5a0a8), and to 46 at this integration (conflict case 58fbbd6c: the sibling integration of run ec762a618372 — case c7c07e48, the rm-760 usage-truth batch, its item claimed 2026-10-08T11:55Z — minted its own 45 above the same landed ceiling 44 while both trees were uncommitted; the older claimant keeps its rung, one invalidation either way, and this run re-bases once more — a warm v45 entry written by a post-sibling build still carries the pre-fix rollout double-add and the empty aider DST start), and to 51 at the accounting-truth residual batch (run 0a55a397 cycle 1: codex record reasoning breakdown rm-617, pi alias first-present rm-618, qwen per-reason skip tables, rm-251 R1 rate-limit rider — minted at the fleet census ceiling over the sibling 49/50 rungs, landing here above the 46 ceiling since none of 47-50 landed; a warm v46 entry still carries the alias-summed pi input, the absent qwen skip tables and the triangular rate-limit counter), and to 52 at the session-cache store-integrity integration (run 2d92ee95 — rm-292 schema-change restore lifecycle + rm-041 lossless `at-bytes:` keys, conflict case 79d8fd06 (first dispatch 51ee80dc), candidate 0aba09b: the batch's own 24 → 26 bump re-based onto the advanced 51 ceiling, one invalidation either way; a warm v51 entry keyed under `to_string_lossy` output hides the very non-UTF-8 journals the lossless keys exist to surface), and to 57 at the usage-truthfulness batch (repository-maintenance run 331684778f6b cycle 1, 2026-10-11 — rm-961 workbuddy message-only dialect admission + rm-251 id-less requestId fold; minted at the 2026-10-11 fleet census: committed ceiling 53, sibling worktree claims 54/55/56/65, so 57 is the lowest rung unique above them all; a warm v52 entry renders a recorded workbuddy journal as the generic text estimate and splits one id-less streamed response into per-emission turns), so this assert pins the live constant)"
    );
    let root = std::env::temp_dir().join(format!(
        "agenttrace-contract-warmcold-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let journal = root.join("torn-session.jsonl");
    fs::write(&journal, torn_claude_journal()).unwrap();

    // Point the cache at an isolated dir (the only env-touching test in
    // this binary; other test binaries are separate processes).
    let cache_dir = root.join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache_dir);

    // Cold leg: empty cache, parse, store, save.
    let mut cache: SessionCache = load_session_cache();
    assert!(
        cached_session(&journal, &mut cache).is_none(),
        "cold: no entry yet"
    );
    let cold = parse_file(&journal).expect("torn claude journal parses");
    assert_eq!(
        cold.metrics.line_skips.get("unparseable_line"),
        Some(&1),
        "cold parse must disclose the torn tail"
    );
    store_session(&journal, &cold, &mut cache).expect("store_session");
    save_session_cache(&mut cache).expect("save_session_cache");

    // Warm leg: fresh load from disk, entry served, disclosure equal.
    let mut warm_cache = load_session_cache();
    let warm = cached_session(&journal, &mut warm_cache).expect("warm hit at the current schema");
    assert_eq!(warm.metrics.line_skips, cold.metrics.line_skips);
    assert_eq!(warm.metrics.tokens_input, cold.metrics.tokens_input);
    let cold_health = data_health(std::slice::from_ref(&cold), 1, 0);
    let warm_health = data_health(&[warm], 1, 1);
    assert_eq!(cold_health.confidence, warm_health.confidence);
    assert_eq!(
        warm_health.confidence, "low",
        "the torn tail must degrade confidence on the warm path too"
    );

    // Stale-schema leg: degrade the persisted file to schema 32 (what a
    // pre-bump cache looks like) — it must not be served at 57.
    let cache_file = session_cache_path();
    let raw = fs::read_to_string(&cache_file).expect("cache file exists");
    let mut degraded: Value = serde_json::from_str(&raw).expect("cache file is JSON");
    degraded["schema_version"] = Value::from(32);
    fs::write(&cache_file, serde_json::to_string(&degraded).unwrap()).unwrap();
    let mut stale_cache: SessionCache = load_session_cache();
    assert!(
        cached_session(&journal, &mut stale_cache).is_none(),
        "rm-710: a schema-32 entry must not be served at the live schema ceiling (57 at this tree — the usage-truthfulness batch, run 331684778f6b, minted 57 at the 2026-10-11 fleet census over the committed ceiling 53 and the sibling worktree claims 54/55/56/65; every warm entry written before this batch regenerates once)"
    );

    std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
    let _ = fs::remove_dir_all(&root);
}

fn torn_claude_journal() -> String {
    let line1 = r#"{"timestamp":"2026-10-06T05:00:00.000Z","type":"user","message":{"role":"user","content":"run the audit"},"uuid":"u1","sessionId":"torn-1","parentUuid":null,"version":"1.0","gitBranch":null,"cwd":"/home/a/p"}"#;
    let line2 = r#"{"timestamp":"2026-10-06T05:05:00.000Z","type":"assistant","message":{"role":"assistant","model":"claude-3","content":[{"type":"text","text":"done"}]},"uuid":"a1","sessionId":"torn-1","parentUuid":"u1","version":"1.0","requestId":"r1"}"#;
    // Torn tail: the writer died mid-object — no closing brace or quote.
    let torn = r#"{"timestamp":"2026-10-06T05:10:00.000Z","type":"user","message":{"role":"user","content":"late"},"uuid":"u3","sessionId":"torn-1"#;
    format!("{line1}\n{line2}\n{torn}\n")
}

// ---------------------------------------------------------------------------
// rm-714 — disclaimer hoisted to document level (assess N3)
// ---------------------------------------------------------------------------

/// Rows carry the bare confidence word; the disclaimer appears exactly
/// once per document, and the hoist measurably shrinks the serialized
/// report: re-serializing the disclaimer into every row (the pre-rm-714
/// shape) would grow this 2000-row document by more than half.
#[test]
fn delivery_evidence_disclaimer_is_hoisted_not_per_row() {
    let template =
        parse_file(&fixture("qwen-multi/qwen-session.jsonl")).expect("qwen-multi parses");
    let sessions: Vec<Session> = (0..2000)
        .map(|i| {
            let mut session = template.clone();
            session.name = format!("s{i}");
            session
        })
        .collect();
    let report = delivery_evidence(&sessions);
    assert_eq!(report.summary.none, 2000);
    assert!(!report.confidence_note.is_empty());
    for row in &report.sessions {
        assert!(
            matches!(row.confidence.as_str(), "high" | "medium" | "low"),
            "row confidence must be the bare word, got {:?}",
            row.confidence
        );
    }
    let disclaimer = "time-window heuristic; Git commits are correlated, not attributable proof of main-merge or business value";
    let json = serde_json::to_string(&report).unwrap();
    assert_eq!(
        json.matches(disclaimer).count(),
        1,
        "the disclaimer is stated once at document level"
    );
    let pre_fix_len = json.len() + sessions.len() * (disclaimer.len() + 2);
    assert!(
        json.len() * 100 < pre_fix_len * 60,
        "hoisting must shrink the doc by ≥40% (rm-714 criterion): {} vs pre-fix {}",
        json.len(),
        pre_fix_len
    );
}

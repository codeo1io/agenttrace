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
fn warm_cache_replays_cold_disclosure_and_schema_35_invalidates_v32() {
    assert_eq!(
        SESSION_CACHE_SCHEMA_VERSION, 35,
        "rm-710: this oracle pins the bump (landed 32 → 33 at the run's base, re-based at integration onto the advanced ceiling as 33 → 34; the ceiling advanced to 35 at the rm-720/rm-721 agent-lane integration, so this assert pins the live constant)"
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
    // pre-bump cache looks like) — it must not be served at 35.
    let cache_file = session_cache_path();
    let raw = fs::read_to_string(&cache_file).expect("cache file exists");
    let mut degraded: Value = serde_json::from_str(&raw).expect("cache file is JSON");
    degraded["schema_version"] = Value::from(32);
    fs::write(&cache_file, serde_json::to_string(&degraded).unwrap()).unwrap();
    let mut stale_cache: SessionCache = load_session_cache();
    assert!(
        cached_session(&journal, &mut stale_cache).is_none(),
        "rm-710: a schema-32 entry must not be served at schema 35"
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

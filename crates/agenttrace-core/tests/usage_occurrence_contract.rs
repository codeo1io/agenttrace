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
fn warm_cache_replays_cold_disclosure_and_schema_63_invalidates_v32() {
    assert_eq!(
        SESSION_CACHE_SCHEMA_VERSION, 63,
        "rm-710: this oracle pins the bump (landed 32 → 33 at the run's base, re-based at integration onto the advanced ceiling as 33 → 34; the ceiling advanced to 35 at the rm-720/rm-721 agent-lane integration, to 36 at the rm-502 timestamp-unification integration, to 37 at the rm-754 priced-loop-cost integration, to 38 at the rm-718/rm-719/rm-716 disclosure-plane-honesty integration, to 39 at the rm-834 report-truthfulness stream-fold integration, to 40 at the rm-585/rm-730 honest-attribution integration, to 41 at the rm-520/rm-521 recorded-cost-basis integration (run ac14e52c, conflict case c1c77f5e), to 42 at the journal-truth integration (run 14954d7a, rm-880 + the rm-406 dated 2026-10-07 arm — Metrics persist `wire_metadata` and `model_attribution` through the Go mirror, conflict case 8b8d096a), to 43 at the hostile-value-fold integration (run fabd9fb8cf73, rm-831 + the rm-721 non-finite-credit rider — fold saturation, negative-insert refusal and the copilot_credit_nonfinite disclosure, landed at its base dc65644 as 35 → 36 and re-based here per the same convention), to 44 at the order-independent codex failure-attribution integration (run fb1addd56503 — its campaign-local rm-585 ordering half, folded onto the landed rm-584 disclosure row; the batch's own 27 → 28 bump re-based onto the advanced ceiling, conflict case e0a11c511c6344f4af8cf63ae9ed485d), and to 45 at the truthful-accounting integration (run 33b7b7b5, rm-617 + rm-899 — the codex rollout reasoning breakdown rides its own tokens_reasoning line instead of billing output twice, and aider's DST-ambiguous starts resolve deterministically via earliest(); the run minted 43 above its base 9c3c599's ceiling 41 and re-based twice at review-fix — 44 at ed90e5ec after origin landed fabd9fb8's 43, 45 at b5b9c6fc after origin landed fb1addd56503's 44 — so 45 lands here as the lowest rung unique above the ceiling, conflict case f2d5a0a8), and to 46 at this integration (conflict case 58fbbd6c: the sibling integration of run ec762a618372 — case c7c07e48, the rm-760 usage-truth batch, its item claimed 2026-10-08T11:55Z — minted its own 45 above the same landed ceiling 44 while both trees were uncommitted; the older claimant keeps its rung, one invalidation either way, and this run re-bases once more — a warm v45 entry written by a post-sibling build still carries the pre-fix rollout double-add and the empty aider DST start), and to 51 at the accounting-truth residual batch (run 0a55a397 cycle 1: codex record reasoning breakdown rm-617, pi alias first-present rm-618, qwen per-reason skip tables, rm-251 R1 rate-limit rider — minted at the fleet census ceiling over the sibling 49/50 rungs, landing here above the 46 ceiling since none of 47-50 landed; a warm v46 entry still carries the alias-summed pi input, the absent qwen skip tables and the triangular rate-limit counter), and to 52 at the session-cache store-integrity integration (run 2d92ee95 — rm-292 schema-change restore lifecycle + rm-041 lossless `at-bytes:` keys, conflict case 79d8fd06 (first dispatch 51ee80dc), candidate 0aba09b: the batch's own 24 → 26 bump re-based onto the advanced 51 ceiling, one invalidation either way; a warm v51 entry keyed under `to_string_lossy` output hides the very non-UTF-8 journals the lossless keys exist to surface), and to 53 at the pi compaction usage fold integration (run 4ffc4fbb, rm-939 — minted campaign-locally as rm-543 and double-minted as the campaign-local rm-551, renumbered at integration since the landed rm-551 is run 254b2417's Copilot per-model fold: pi's inline compaction/branch_summary usage block — tokens plus the upstream-recorded cost, disclosed per kind as pi_compaction_usage_counted — used to be dropped whole by the handled 'branch_summary' | 'compaction' arm, PoC $0.0023 reported vs ~$0.0143 spec-true, so a warm v52 entry keeps serving the undercount; the batch's own 27 → 28 bump re-based onto the advanced 52 ceiling, one invalidation either way), and to 54 at the usage-truthfulness integration (run e944a060a640 cycle 1, rm-619 + rm-954 + rm-955 — opencode messages whose `tokens` object carries only reasoning or only unknown keys no longer suppress the step-finish usage rescue; the batch minted 53 at the fleet census ceiling above the then-landed 51 with sibling run-57c491338818 holding an uncommitted 52, and this integration re-bases that rung onto the advanced ceiling as 53 → 54 per the c7c07e48/58fbbd6c convention — the landed 53 is run 4ffc4fbb's rung, one invalidation either way; a warm v53 entry still serves the zeroed twin totals), and to 55 at the intake → attribution → disclosure truthfulness integration (run 5eb82325ebea cycle 1, rm-863 + rm-860 + rm-861 + rm-862 — flat OpenAI-compatible reasoning/thinking aliases set the breakdown without adding to output, both-spelling lines parse under the disclosed canonical-winner rule, the session model label is restricted to authoritative rows with ignored relabels disclosed, and the doctor fold splits into a loss tier and an informational tier; minted 38 → 39 at its base 5ed9ebc, re-based to 41 at implement over the then-live origin ceiling 40, to 43 by the cycle-1 review fix (review b0446d6c F1), and to 50 by the review-fix rebase — the rung re-bases here onto the advanced ceiling as 54 → 55 per the c7c07e48/58fbbd6c convention, one invalidation either way; a warm v54 entry still double-bills subset-shaped flat reasoning aliases, keeps both-spelling lines dropped as event_schema, and keeps non-assistant model labels — conflict case f642872bb8a143f6a4a4e62e2700ea37), and to 56 at the honest-magnitudes integration (run 32f3b7a1 cycle 3, rm-726 magnitude-clamp disclosure + the rm-248 relative-XDG config/history rider — number_as_i64 magnitude clamps (u64 payloads above i64::MAX, saturating floats, numeric strings) count on the magnitude_clamped:* disclosure family, the clamp census prints beside the --overview cost total and rides CostAudit.clamped_values, and totals at or above the 1e12 plausibility bound flag a high-severity implausible_magnitude anomaly instead of a clean $42.7T at HEALTH 100; minted 32 → 33 at the batch's base e9e8fd9 and re-based here onto the advanced ceiling as 55 → 56 per the c7c07e48/58fbbd6c convention, one invalidation either way; a warm v55 entry still serves the pre-disclosure clean report — conflict case 2107fcf36cfa40b389723e98a9059636), and to 57 at the usage-accounting truthfulness closeout integration (run 2f02ecafb2674e529f662f7cc27162f4 cycle 2 — the campaign's rm-685 codex occurrence-adjacent totals with the capped value-dedup ring, landed here as rm-980 and composed with the rm-711 compaction envelope, + rm-686 generic-extractor Gemini-inclusive promptTokenCount netting, landed as rm-981, + the rm-602 qwen-stats dual-alias extension; minted 32 → 33 at the batch's base 518170a and re-based here onto the advanced ceiling as 56 → 57 per the c7c07e48/58fbbd6c convention, one invalidation either way; a warm v56 entry still serves the dropped value-equal post-compaction window, the inclusive-input over-count and the summed dual qwen aliases — conflict case edb878bb27db418fbb13184efccaa721), and to 58 at the id-less fold-key namespace integration (run 57c491338818 cycle 1, review 543ac419 F1 → fix 670753a6, 2026-10-10: id-less fold rows leave the wire tool_use_id dedup namespace, fold ordinals become one-per-block emission positions, and genuine re-folds carry one structural meta carrier (+1 events_total) — a warm v57 cache would keep serving the silently-dropped aliased fold with no disclosure; minted 42 → 52 at the run's base 15a8da8 above the fleet's in-flight frontier and re-based here onto the advanced ceiling as 57 → 58 per the c7c07e48/58fbbd6c/7762d54b convention, one invalidation either way — conflict case a4ff3f6c72be4905a00dbb4d75cad452), and to 59 at the kimi usage-XOR integration (run e5653f52 cycle 2, the upstream-#312 port fork-adjudicated: kimi sessions count `usage` OR `metadata.usage`, never both — a kimi export mirroring the session usage in both objects was double-counted, so a warm v58 entry keeps serving the doubled totals; the batch's other arms were superseded by this tree's own landed lanes (claude per-message-id fold rm-601/rm-834, qwen first-alias rm-602/rm-980, codex ring ledger rm-685/rm-711, reasoning subsets rm-863, opencode breakdown rm-619); minted 27 → 28 at the run's base 8991144 and re-based here onto the advanced ceiling as 58 → 59 per the c7c07e48/58fbbd6c/7762d54b convention, one invalidation either way — conflict case 963662941e3a40579e658a63f2b88da6), and to 60 at the pi journal accounting truth part-2 integration (run e486dc1adb9b cycle 2, rm-253 + the rm-408 pi residual: the pi v3 MESSAGE arm passes `message.usage.cost.total` through as recorded cost — recorded wins over catalog pricing and the block's tokens leave the catalog formula, a journal recording $0.0075 was served catalog-billed at $0.0165 (2.2x) — and present-but-zero pi usage blocks emit their measured zeros into zero_usage_events instead of silently reading as clean, the zero-insert gate composed with the landed rm-618 first-present alias rule; minted 24 → 25 at the run's base 1511547 and re-based here onto the advanced ceiling as 59 → 60 per the c7c07e48/58fbbd6c/7762d54b convention, one invalidation either way — conflict case 38b020c7f8f1463fb56f21772d627068, oracle swept at its independent review case ca2e54c3ce87400e978a84415bba9708), and to 62 at the gemini chats record-stream integration (run 4309a268 cycle 1, rm-715 — the >= 0.63 writer's `tokens` carrier began counting where the usageMetadata/usage/tokenUsage key list read nothing, so a warm v59 entry keeps serving the silent zeros; minted above the fleet census frontier — origin's landed 60 and a sibling's uncommitted 61 — one invalidation either way; this integration, conflict case 0dee05b41d4b4252934f5f69bcf62cd6, lands that origin 60 as the rung directly below in the merged ladder), and to 63 at the opencode agent/summary-name consumption integration (run b5affead89448d8a664fc699fe42c49 cycle 2, rm-991: a SHAPE-ONLY rung — Metrics began persisting the opencode `session.agent` name and the `summary_additions`/`summary_deletions`/`summary_files` diff columns, so a warm v62 entry replays an empty agent_name while a fresh parse of the same database splits by_agent, and the summary diff numbers plus the `files: reported_by_agent` provenance upgrade never reach a warm hit; minted 60 → 61 straight at the run's base 82e23b9, below the frontier — this tree's landed 62 rung (run 4309a268, conflict case 0dee05b41d4b4252934f5f69bcf62cd6) had left 61 free for this sibling's mint — so this integration re-bases the rung onto the advanced ceiling as 62 → 63 per the c7c07e48/58fbbd6c/7762d54b convention, one invalidation either way — conflict case fbe92e4b80db45d0a2fb2b5c020b81a4; the SQLite snapshot moved 11 → 12 in the same unit and its own oracle rung swept together), so this assert pins the live constant)"    );
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
    // pre-bump cache looks like) — it must not be served at 62.
    let cache_file = session_cache_path();
    let raw = fs::read_to_string(&cache_file).expect("cache file exists");
    let mut degraded: Value = serde_json::from_str(&raw).expect("cache file is JSON");
    degraded["schema_version"] = Value::from(32);
    fs::write(&cache_file, serde_json::to_string(&degraded).unwrap()).unwrap();
    let mut stale_cache: SessionCache = load_session_cache();
    assert!(
        cached_session(&journal, &mut stale_cache).is_none(),
        "rm-710: a schema-32 entry must not be served at the live schema ceiling (62 at this tree — the gemini chats record-stream integration, run 4309a268 cycle 1, minted above origin's landed 60 and a sibling's uncommitted 61; that landed 60 rung is the pi journal accounting truth part-2 integration, run e486dc1adb9b cycle 2, which re-based its 24 → 25 mint onto the advanced 59 ceiling per the landed-rung-keeps-numeral convention; every warm entry written before this integration regenerates once)"
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

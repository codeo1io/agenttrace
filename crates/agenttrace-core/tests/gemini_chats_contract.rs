//! rm-715 (silent-zero closeout LEAD): gemini-cli >= 0.63 records its
//! chats as an append-only record stream under
//! `~/.gemini/tmp/<hash>/chats/session-*.jsonl` (upstream
//! `chatRecordingService.ts`, tag v0.63.0): a metadata line
//! (`sessionId`/`projectHash`), one MessageRecord per turn keyed by
//! `id`, `{$set: {messages}}` compaction checkpoints, and
//! `{$rewindTo}` / `{$patch}` records (post-0.63 main). Usage rides a
//! `tokens` carrier (a TokensSummary: input/output/cached/thoughts/
//! tool/total) the usageMetadata/usage/tokenUsage key list never read,
//! so every such session served as a generic-lane silent zero before
//! this lane landed — the pre-edit PoC priced session-a at
//! generic/0/$0.0000 while the same turn under the old
//! usageMetadata shape counted 1500 tokens.
//!
//! These contracts pin the stream semantics through the public
//! `parse_raw_session` surface (metrics-level observables); the
//! event-level arms (content text, patch rewrites, event ordering)
//! live in the parser.rs unit module next to the lane. Upstream
//! semantics reference: `loadConversationRecord` at tag v0.63.0,
//! `$patch` arms per main-tip d1cc08a8.
//!
//! - the `tokens` carrier counts, and the writer's full-record
//!   re-append (tokens attach to the turn's own id) replaces in place
//!   instead of double-billing;
//! - 2026-10-22 baselining (rm-715 acceptance (a), run b58cd254 cycle
//!   5): v0.65.0 stable (2026-10-20) `chatRecordingTypes.ts` is
//!   byte-identical to the pinned v0.64.0 capture and
//!   `chatRecordingService.ts` changes only resume-dedup and lifecycle
//!   behavior (upstream #29618) — no new record shapes — so the claim
//!   rule and fixture shapes above carry unchanged; the one
//!   parse-relevant behavior change is the resume dedup, mirrored by
//!   the `_response` twin-fold (`resume_twin_response_records_…`).
//! - `{$set: {messages}}` clears and rebuilds — a warm pre-checkpoint
//!   state must not survive the checkpoint;
//! - `{$rewindTo}` truncates from the id;
//! - `{$patch}` `removeIds` and the dedicated `toolCalls` array apply;
//! - `info`/`error`/`warning` records never count as turns;
//! - the old API-response shape (usageMetadata single-document) keeps
//!   its lane — the LEAD must not regress what already counted.

use agenttrace_core::parse_raw_session;

fn parse_one(name: &str, raw: &str) -> agenttrace_core::Session {
    parse_raw_session(name, name, raw).unwrap()
}

const METADATA_LINE: &str = concat!(
    r#"{"sessionId":"sess-a","projectHash":"h1","startTime":"2026-10-12T08:00:00.000Z","#,
    r#""lastUpdated":"2026-10-12T08:01:00.000Z","kind":"main"}"#
);

/// Arm A — the 0.63 writer's plain turn: user record, gemini record,
/// then the SAME id re-appended with the tokens summary attached
/// (`recordMessageTokens` calls `pushMessage` again). The reader's
/// messagesMap replaces in place; the counter must bill exactly one
/// usage event.
fn arm_a() -> String {
    format!(
        "{METADATA_LINE}\n{}",
        concat!(
            r#"{"id":"m-user-1","timestamp":"2026-10-12T08:00:01.000Z","type":"user","content":"hello gemini"}"#,
            "\n",
            r#"{"id":"m-gen-1","timestamp":"2026-10-12T08:00:05.000Z","type":"gemini","content":"hi there","model":"gemini-2.5-pro"}"#,
            "\n",
            r#"{"$set":{"lastUpdated":"2026-10-12T08:00:05.000Z"}}"#,
            "\n",
            r#"{"id":"m-gen-1","timestamp":"2026-10-12T08:00:05.000Z","type":"gemini","content":"hi there","model":"gemini-2.5-pro","tokens":{"input":1000,"output":200,"cached":300,"thoughts":0,"tool":0,"total":1500}}"#,
            "\n"
        )
    )
}

/// Control — the same turn under the old API-response shape
/// (usageMetadata single-document). Arm A must price identically:
/// 1000 input + 200 output + 300 cached, same model, same source.
const CONTROL_OLD_SHAPE: &str = concat!(
    r#"{"modelVersion":"gemini-2.5-pro","timestamp":"2026-10-12T08:00:00Z","#,
    r#""contents":[{"role":"user","parts":[{"text":"hello gemini"}]},{"role":"model","parts":[{"text":"hi there"}]}],"#,
    r#""usageMetadata":{"promptTokenCount":1000,"candidatesTokenCount":200,"cachedContentTokenCount":300}}"#,
    "\n",
);

#[test]
fn tokens_carrier_counts_and_reappend_dedupes() {
    let arm = parse_one("session-a.jsonl", &arm_a());
    let control = parse_one("control.json", CONTROL_OLD_SHAPE);
    // rm-715 claim: the same turn, same shape of totals.
    assert_eq!(arm.metrics.source_tool, "gemini_cli");
    assert_eq!(arm.metrics.model_used, control.metrics.model_used);
    assert_eq!(
        arm.metrics.cost_estimated, control.metrics.cost_estimated,
        "the tokens carrier must price at the usageMetadata rate, not the generic zero"
    );
    // Dedupe: the writer re-appends the full record with tokens
    // attached; a replace-in-place bug would double every bucket.
    assert_eq!(arm.metrics.tokens_input, 1000);
    assert_eq!(arm.metrics.tokens_output, 200);
    assert_eq!(arm.metrics.tokens_cache_r, 300);
    assert_eq!(arm.metrics.user_messages, 1);
    assert_eq!(arm.metrics.events_total, control.metrics.events_total);
}

/// Independent-review F1: token values that are JSON floats (500.5, 1e3) or
/// numeric strings ("200") must coerce through the shared number helper
/// (rm-342 parity) — the original arm used raw `Value::as_i64`, which
/// silently zeroed every non-integer and left the session under-counted.
#[test]
fn float_and_string_token_values_coerce() {
    const HOSTILE: &str = concat!(
        r#"{"id":"m-gen-1","timestamp":"2026-10-12T08:00:01.000Z","type":"user","content":"hi"}"#,
        "\n",
        r#"{"id":"m-gen-1","timestamp":"2026-10-12T08:00:02.000Z","type":"gemini","content":"there","model":"gemini-2.5-pro","tokens":{"input":500.5,"output":"200","cached":1e3,"thoughts":0,"tool":0,"total":1700}}"#,
        "\n",
    );
    const CONTROL_INTS: &str = concat!(
        r#"{"modelVersion":"gemini-2.5-pro","timestamp":"2026-10-12T08:00:00Z","#,
        r#""contents":[{"role":"user","parts":[{"text":"hello gemini"}]},{"role":"model","parts":[{"text":"hi there"}]}],"#,
        r#""usageMetadata":{"promptTokenCount":500,"candidatesTokenCount":200,"cachedContentTokenCount":1000}}"#,
        "\n",
    );
    let arm = parse_one("session-f1.jsonl", &format!("{METADATA_LINE}\n{HOSTILE}"));
    let control = parse_one("control-f1.json", CONTROL_INTS);
    assert_eq!(arm.metrics.source_tool, "gemini_cli");
    assert_eq!(arm.metrics.tokens_input, 500);
    assert_eq!(arm.metrics.tokens_output, 200);
    assert_eq!(arm.metrics.tokens_cache_r, 1000);
    assert_eq!(arm.metrics.cost_estimated, control.metrics.cost_estimated);
}

#[test]
fn set_messages_checkpoint_clears_and_replaces() {
    let raw = format!(
        "{}{}\n",
        arm_a(),
        concat!(
            r#"{"$set":{"lastUpdated":"2026-10-12T08:05:00.000Z","messages":["#,
            r#"{"id":"m-user-2","timestamp":"2026-10-12T08:05:00.000Z","type":"user","content":"compacted hello"},"#,
            r#"{"id":"m-gen-2","timestamp":"2026-10-12T08:05:02.000Z","type":"gemini","content":"compacted reply","model":"gemini-2.5-pro","tokens":{"input":500,"output":100,"cached":0,"thoughts":0,"tool":0,"total":600}}]}}"#
        )
    );
    let session = parse_one("session-b.jsonl", &raw);
    // The checkpoint array REPLACES the whole conversation: the
    // 1500-token pre-checkpoint turn must be gone, not summed with the
    // 600-token one.
    assert_eq!(session.metrics.tokens_input, 500);
    assert_eq!(session.metrics.tokens_output, 100);
    assert_eq!(session.metrics.user_messages, 1);
    // Single-usage evidence: replace-not-sum keeps the counts at the
    // checkpoint's own totals.
    assert!(session.metrics.cost_estimated > 0.0);
}

#[test]
fn rewind_to_truncates_from_the_id() {
    let raw = format!("{}{}\n", arm_a(), r#"{"$rewindTo":"m-gen-1"}"#);
    let session = parse_one("session-r.jsonl", &raw);
    // Truncate FROM the id: the generation's RECORDED usage (1000
    // input / 200 output) is gone; the user turn stays. What remains
    // is the standing content-estimate fallback for a usage-less
    // session (~3 tokens for "hello gemini"), never the truncated
    // generation's counts.
    assert!(
        session.metrics.tokens_input < 1000,
        "the rewound generation's 1000 recorded input tokens must not survive; got {}",
        session.metrics.tokens_input
    );
    assert_eq!(session.metrics.tokens_output, 0);
    assert_eq!(session.metrics.user_messages, 1);
    assert_eq!(session.metrics.source_tool, "gemini_cli");
}

#[test]
fn patch_remove_ids_and_tool_calls_apply() {
    let raw = format!(
        "{METADATA_LINE}\n{}",
        concat!(
            r#"{"id":"m-u1","timestamp":"2026-10-12T09:00:01.000Z","type":"user","content":"patch me"}"#,
            "\n",
            r#"{"id":"m-g1","timestamp":"2026-10-12T09:00:03.000Z","type":"gemini","content":"original reply","model":"gemini-2.5-pro"}"#,
            "\n",
            r#"{"id":"m-g1","timestamp":"2026-10-12T09:00:03.000Z","type":"gemini","content":"original reply","model":"gemini-2.5-pro","tokens":{"input":700,"output":150,"cached":50,"thoughts":40,"tool":0,"total":940}}"#,
            "\n",
            r#"{"id":"m-g2","timestamp":"2026-10-12T09:00:07.000Z","type":"gemini","content":"","model":"gemini-2.5-pro","toolCalls":[{"id":"tc-1","name":"read_file","args":{"path":"x.rs"},"status":"Completed","timestamp":"2026-10-12T09:00:06.000Z","displayName":"Read File"}]}"#,
            "\n",
            r#"{"$patch":{"id":"m-g1","content":"patched reply"}}"#,
            "\n",
            r#"{"$patch":{"removeIds":["m-u1"],"orderIds":["m-g2","m-g1"]}}"#,
            "\n"
        )
    );
    let session = parse_one("session-4.jsonl", &raw);
    // removeIds: the user turn is gone.
    assert_eq!(session.metrics.user_messages, 0);
    // tokens: 700 input + (150 output + 40 thoughts folded once) +
    // 50 cached; thoughts ride their own breakdown without
    // double-billing output.
    assert_eq!(session.metrics.tokens_input, 700);
    assert_eq!(session.metrics.tokens_output, 190);
    assert_eq!(session.metrics.tokens_reasoning, 40);
    assert_eq!(session.metrics.tokens_cache_r, 50);
    // The dedicated toolCalls array (ToolCallRecord) counts as one
    // tool call, not as text.
    assert_eq!(session.metrics.tool_calls_total, 1);
    assert_eq!(
        session.metrics.tool_usage.get("read_file"),
        Some(&1),
        "the toolCalls array must drive tool_usage"
    );
}

/// rm-715 twin-fold (run b58cd254 cycle 5, batch "Gemini 0.65-stable
/// boundary truth + test-clock determinism"): pre-0.65 gemini-cli
/// resume paths re-record a turn under a synthesized `<id>_response`
/// id (client fallback in `sessionUtils.ts`, upstream fix #29618 —
/// "avoid duplicate tool response turns when resuming sessions" —
/// shipped 0.65.0 stable 2026-10-20). The 0.64-shaped journal below
/// carries the `m-g3` generation TWICE — the original record and a
/// `m-g3_response` twin re-recording the same usage — and the raw-id
/// upsert counted both (fleet PoC 2026-10-16: 10 sessions / 5 turns
/// parsed as 20 / 10). The fold must collapse the twin to ONE counted
/// turn billing its usage exactly once.
#[test]
fn resume_twin_response_records_collapse_to_one_turn() {
    let raw = format!(
        "{METADATA_LINE}\n{}",
        concat!(
            r#"{"id":"m-u1","timestamp":"2026-10-12T09:10:01.000Z","type":"user","content":"resume me"}"#,
            "\n",
            r#"{"id":"m-g3","timestamp":"2026-10-12T09:10:03.000Z","type":"gemini","content":"tool turn","model":"gemini-2.5-pro","toolCalls":[{"id":"tc-9","name":"read_file","args":{"path":"y.rs"},"status":"Completed","timestamp":"2026-10-12T09:10:02.000Z","displayName":"Read File"}],"tokens":{"input":300,"output":60,"cached":0,"thoughts":0,"tool":0,"total":360}}"#,
            "\n",
            r#"{"id":"m-g3_response","timestamp":"2026-10-12T09:10:03.000Z","type":"gemini","content":"tool turn","model":"gemini-2.5-pro","toolCalls":[{"id":"tc-9","name":"read_file","args":{"path":"y.rs"},"status":"Completed","timestamp":"2026-10-12T09:10:02.000Z","displayName":"Read File"}],"tokens":{"input":300,"output":60,"cached":0,"thoughts":0,"tool":0,"total":360}}"#,
            "\n"
        )
    );
    let session = parse_one("session-twin.jsonl", &raw);
    assert_eq!(session.metrics.source_tool, "gemini_cli");
    assert_eq!(session.metrics.user_messages, 1);
    // One logical generation: the twin must REPLACE the base record
    // in place (the lane's re-append semantics), so the turn, its
    // tools and its usage each count exactly once. Red before the
    // fold: the raw-id upsert appended the twin as a SECOND counted
    // generation — 2 tool_calls_total / 2 read_file usages (the
    // fleet PoC's doubled counts).
    assert_eq!(
        session.metrics.tool_calls_total, 1,
        "resume twin double-counted the generation's tools: {}",
        session.metrics.tool_calls_total
    );
    assert_eq!(session.metrics.tool_usage.get("read_file"), Some(&1));
    assert_eq!(session.metrics.tokens_input, 300);
    assert_eq!(session.metrics.tokens_output, 60);
}

#[test]
fn info_and_error_records_never_count_as_turns() {
    let raw = format!(
        "{METADATA_LINE}\n{}\n{}",
        r#"{"id":"m-i1","timestamp":"2026-10-12T08:00:02.000Z","type":"info","content":"context compressed 4.2k -> 1.1k"}"#,
        arm_a()
    );
    let session = parse_one("session-i.jsonl", &raw);
    assert_eq!(
        session.metrics.user_messages, 1,
        "info records are UI-synthetic and must not count as user turns"
    );
    // The usage event is unaffected by the interleaved info record.
    assert_eq!(session.metrics.tokens_input, 1000);
}

#[test]
fn empty_session_stays_unclaimed() {
    // Metadata line only: no MessageRecords, no usage. The claim rule
    // refuses it, and an unclaimed shape is parse_raw_session's
    // unsupported-format error — exactly the pre-LEAD behavior (the
    // discovery layer's generic fallback owns such files, unchanged).
    let parsed = parse_raw_session(
        "session-e.jsonl",
        "session-e.jsonl",
        &format!("{METADATA_LINE}\n"),
    );
    assert!(
        parsed.is_err(),
        "a metadata-only stream must not be claimed by the chats lane"
    );
}

#[test]
fn old_usage_metadata_shape_still_counts() {
    // Regression guard for the object lane: the pre-0.63
    // single-document shape must price exactly as before the LEAD.
    let session = parse_one("control-lane.json", CONTROL_OLD_SHAPE);
    assert_eq!(session.metrics.source_tool, "gemini_cli");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(session.metrics.tokens_output, 200);
    assert_eq!(session.metrics.tokens_cache_r, 300);
}

/// rm-715 perf rider (run 80e8beb6 cycle 1, batch "bounded inputs,
/// honest docs"; hardened by review-fix F1, attempt 994f9491): a
/// hostile append-only journal must parse in linear time. The original
/// absolute 5 s bound flaked on loaded boxes (4/6 concurrent suite runs
/// red at load 50 on 14 cores while standalone parses already took
/// 2.3-3.9 s), so the shape is asserted via a self-calibrated ratio:
/// linear parsing grows ~10x from 2k to 20k records while the
/// pre-LEAD quadratic `Vec::find` upsert grew ~100x (9.5 s at 20k).
/// min-of-two sampling sheds load spikes; the absolute ceiling only
/// bounds runaway hangs.
///
/// rm-1519 (run 1e4f6b95 cycle 4, batch "zstd rollout truth end-to-end
/// and signal truth") re-stabilized the same arm on a fleet box at load
/// 50-58: 6 probe runs put the HEALTHY ratio at 7.5-24.9x, with the
/// instability concentrated in the LARGE measurement (small stayed
/// 205-287 ms while one run's large min-of-two was 5.8 s against another
/// run's 2.1 s — cache-thrash under load multiplies per-record cost for
/// the 20k journal that does not fit the working set the way 2k does).
/// A 40.0 floor left only 1.6x headroom over that observed tail. The
/// arm now samples large min-of-THREE (the noisy side) and the floor is
/// 60.0: budget = 3x over the measured healthy tail, still a ~1.7x
/// detection margin against the quadratic's ~100x (a regressed upsert
/// at this box's speeds parses 20k in ~24 s, ratio ~109x, not 60x).
#[test]
fn hostile_append_journal_parses_within_the_linear_bound() {
    let build = |records: i64| {
        let mut journal = String::with_capacity(records as usize * 200);
        journal.push_str(METADATA_LINE);
        journal.push('\n');
        for i in 0..records {
            journal.push_str(&format!(
                "{{\"id\":\"m-gen-{i}\",\"timestamp\":\"2026-10-12T08:00:00.000Z\",\"type\":\"gemini\",\"content\":\"turn {i}\",\"model\":\"gemini-2.5-pro\",\"tokens\":{{\"input\":1,\"output\":2,\"cached\":0,\"thoughts\":0,\"tool\":0,\"total\":3}}}}\n"
            ));
        }
        journal
    };
    let parse_timed = |journal: &str| {
        let started = std::time::Instant::now();
        let session = parse_one("session-hostile.jsonl", journal);
        (started.elapsed(), session)
    };

    const SMALL: i64 = 2_000;
    const LARGE: i64 = 20_000;
    let small = build(SMALL);
    let large = build(LARGE);
    // Warm-up parse absorbs allocator/page-cache effects before timing.
    let _ = parse_timed(&small);

    let (small_a, _) = parse_timed(&small);
    let (small_b, _) = parse_timed(&small);
    let small_time = small_a.min(small_b);
    // rm-1519: large is the noisy side (see the doc comment above), so
    // it takes the extra sample; one more ~2-6 s parse under load is
    // cheap insurance against a 40->60-flake both-sides-bad run.
    let (large_a, session) = parse_timed(&large);
    let (large_b, _) = parse_timed(&large);
    let (large_c, _) = parse_timed(&large);
    let large_time = large_a.min(large_b).min(large_c);

    // Correctness alongside timing: every record bills exactly once —
    // an upsert that appends instead of replacing would double it.
    assert_eq!(session.metrics.source_tool, "gemini_cli");
    assert_eq!(session.metrics.tokens_input, LARGE);
    assert_eq!(session.metrics.tokens_output, LARGE * 2);
    assert!(
        large_time < std::time::Duration::from_secs(120),
        "20k-record gemini chats journal parsed in {large_time:?} — runaway hang"
    );
    let ratio = large_time.as_secs_f64() / small_time.as_secs_f64().max(1e-9);
    assert!(
        ratio < 60.0,
        "gemini chats parse scaled {ratio:.1}x from 2k to 20k records \
         (small {small_time:?}, large {large_time:?}) — the upsert regressed \
         to a per-record rescan (O(n^2) measured 9.5 s at 20k at cab6917, \
         ~100x ratio; the 60x floor rides 3x of load headroom, see rm-1519)"
    );
}

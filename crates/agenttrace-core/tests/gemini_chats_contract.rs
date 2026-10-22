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

// ---------------------------------------------------------------------------
// rm-715 remaining scope — gemini-cli 0.65.0 resume-dedup truth. The
// trigger fired 2026-10-20: v0.65.0 STABLE ships #29618 "avoid duplicate
// tool response turns when resuming sessions". Source-pinned writer truth
// (raw.githubusercontent.com/google-gemini/gemini-cli at v0.64.0/v0.65.0):
//
// - utils/sessionUtils.ts `convertSessionToClientHistory` synthesizes a
//   `${msg.id}_response` user turn for every gemini tool call carrying a
//   result when rebuilding client history from a recorded journal.
//   v0.64.0 synthesized it UNCONDITIONALLY — even when a subsequent live
//   user turn already covered the call. For an Array-of-Part result the
//   synthesized twin's content IS the spread parts, so it carries text.
// - services/chatRecordingService.ts `updateMessagesFromHistory` persists
//   every history turn whose id is missing from `messageMetaMap`, so a
//   v0.64.0 resume WROTE the twin into the journal (the "previously
//   inflated recordings" the v0.65.0 docstring requires to recover
//   cleanly).
// - The same v0.64.0 `initialize()` new-session path for subagents
//   (filename = `<sessionId>.jsonl`, no collision check) appended a
//   second metadata record, reset `messages: []`, and replayed the
//   whole history as content-only records (no model/tokens/toolCalls).
// - v0.65.0 cures both: synthesis stops when a subsequent user turn
//   covers the call (fallback only for legacy recordings lacking a
//   recorded response turn), and initialize() reuses the loaded record
//   instead of resetting to empty.
//
// The three tests below pin that the pre-0.65 inflated shapes account
// IDENTICALLY to the clean v0.65.0 shape.

#[test]
fn gemini_chats_resume_twin_does_not_duplicate_tool_response() {
    // v0.65.0-clean resume journal: user prompt; gemini turn with a
    // tool call whose result is a Part array; the live user response
    // turn (functionResponse parts carry no text — inert in this
    // lane); the finalized re-append carrying the token summary.
    let clean = r#"{"sessionId":"8f2c9f1c2b7d4e6a","projectHash":"5c0f7e9a","kind":"main"}
{"id":"turn-1","type":"user","timestamp":"2026-10-20T10:00:00.000Z","content":"Read the config"}
{"id":"turn-2","type":"gemini","timestamp":"2026-10-20T10:00:05.000Z","model":"gemini-2.5-pro","content":[{"text":"Reading it now."}],"toolCalls":[{"id":"call-1","name":"read_file","args":{"path":"config.toml"},"result":[{"text":"key = \"value\"\n"}]}]}
{"id":"turn-3","type":"user","timestamp":"2026-10-20T10:00:06.000Z","content":[{"functionResponse":{"id":"call-1","name":"read_file","response":{"output":"key = \"value\""}}}]}
{"id":"turn-2","type":"gemini","timestamp":"2026-10-20T10:00:05.000Z","model":"gemini-2.5-pro","content":[{"text":"Reading it now."}],"toolCalls":[{"id":"call-1","name":"read_file","args":{"path":"config.toml"},"result":[{"text":"key = \"value\"\n"}]}],"tokens":{"input":1000,"output":200,"cached":300,"thoughts":50}}
"#;
    // The SAME journal as a v0.64.0-stable resume left it: the
    // synthesized `turn-2_response` twin appended at the tail (its
    // content is the spread Array-of-Part result, so it carries the
    // tool output text a second time). The dedup contract: it must
    // account identically to the clean journal — one tool response
    // exchange happened, one is billed.
    let inflated = r#"{"sessionId":"8f2c9f1c2b7d4e6a","projectHash":"5c0f7e9a","kind":"main"}
{"id":"turn-1","type":"user","timestamp":"2026-10-20T10:00:00.000Z","content":"Read the config"}
{"id":"turn-2","type":"gemini","timestamp":"2026-10-20T10:00:05.000Z","model":"gemini-2.5-pro","content":[{"text":"Reading it now."}],"toolCalls":[{"id":"call-1","name":"read_file","args":{"path":"config.toml"},"result":[{"text":"key = \"value\"\n"}]}]}
{"id":"turn-3","type":"user","timestamp":"2026-10-20T10:00:06.000Z","content":[{"functionResponse":{"id":"call-1","name":"read_file","response":{"output":"key = \"value\""}}}]}
{"id":"turn-2","type":"gemini","timestamp":"2026-10-20T10:00:05.000Z","model":"gemini-2.5-pro","content":[{"text":"Reading it now."}],"toolCalls":[{"id":"call-1","name":"read_file","args":{"path":"config.toml"},"result":[{"text":"key = \"value\"\n"}]}],"tokens":{"input":1000,"output":200,"cached":300,"thoughts":50}}
{"id":"turn-2_response","type":"user","timestamp":"2026-10-20T10:00:07.000Z","content":[{"text":"key = \"value\"\n"}]}
"#;
    let clean_session = parse_one("resume-clean.jsonl", clean);
    let inflated_session = parse_one("resume-inflated.jsonl", inflated);
    assert_eq!(clean_session.metrics.source_tool, "gemini_cli");
    // Clean baseline pinned on its own: exactly one user message (the
    // prompt — turn-3's functionResponse parts are inert), one
    // assistant event, one tool call, tokens billed once. events_total
    // = turn-1's user event + turn-2's assistant event (text and the
    // tool call ride ONE event) + turn-2's `tokens` meta usage event.
    assert_eq!(clean_session.metrics.user_messages, 1);
    assert_eq!(clean_session.metrics.events_total, 3);
    assert_eq!(clean_session.metrics.tool_calls_total, 1);
    assert_eq!(clean_session.metrics.tokens_input, 1000);
    assert_eq!(clean_session.metrics.tokens_output, 250);
    // The rm-715 resume-dedup contract: the v0.64.0-inflated journal
    // accounts identically to the v0.65.0-clean one.
    assert_eq!(
        inflated_session.metrics.user_messages, clean_session.metrics.user_messages,
        "v0.64.0 resume twin inflated the user-turn count (#29618)"
    );
    assert_eq!(
        inflated_session.metrics.events_total, clean_session.metrics.events_total,
        "v0.64.0 resume twin inflated the event count (#29618)"
    );
    assert_eq!(
        inflated_session.metrics.tokens_input, clean_session.metrics.tokens_input,
        "v0.64.0 resume twin changed input tokens (#29618)"
    );
    assert_eq!(
        inflated_session.metrics.tokens_output, clean_session.metrics.tokens_output,
        "v0.64.0 resume twin changed output tokens (#29618)"
    );
    assert_eq!(
        inflated_session.metrics.tool_calls_total, clean_session.metrics.tool_calls_total,
        "v0.64.0 resume twin changed the tool-call count (#29618)"
    );
}

#[test]
fn gemini_chats_resume_replay_keeps_recorded_tokens() {
    // v0.64.0-stable subagent resume: the re-initialize() path for
    // kind=subagent (filename = `<sessionId>.jsonl`, no collision
    // check) appended a SECOND metadata record, reset the cached
    // messages to empty, and `updateMessagesFromHistory` replayed the
    // whole history as content-only records — no model, no toolCalls,
    // no tokens. Replace-in-place without a carry-forward merge
    // silently zeroes the session's recorded billing for every
    // replayed turn (v0.65.0 cures this at the writer by reusing the
    // loaded record; the reader-side cure must survive the journals
    // already in the wild).
    let replayed = r#"{"sessionId":"c3d1a4b2","projectHash":"5c0f7e9a","kind":"subagent"}
{"id":"turn-1","type":"user","timestamp":"2026-10-19T09:00:00.000Z","content":"Summarize the log"}
{"id":"turn-2","type":"gemini","timestamp":"2026-10-19T09:00:04.000Z","model":"gemini-2.5-flash","content":[{"text":"Scanning."}],"toolCalls":[{"id":"call-1","name":"run_command","args":{"cmd":"tail -n 5 app.log"}}]}
{"id":"turn-2","type":"gemini","timestamp":"2026-10-19T09:00:04.000Z","model":"gemini-2.5-flash","content":[{"text":"Scanning."}],"toolCalls":[{"id":"call-1","name":"run_command","args":{"cmd":"tail -n 5 app.log"}}],"tokens":{"input":1000,"output":200,"cached":300,"thoughts":0}}
{"sessionId":"c3d1a4b2","projectHash":"5c0f7e9a","kind":"subagent"}
{"id":"turn-1","type":"user","timestamp":"2026-10-19T09:00:00.000Z","content":"Summarize the log"}
{"id":"turn-2","type":"gemini","timestamp":"2026-10-19T09:00:04.000Z","content":[{"text":"Scanning."}]}
{"id":"turn-3","type":"gemini","timestamp":"2026-10-19T09:05:00.000Z","model":"gemini-2.5-flash","content":[{"text":"Done."}],"tokens":{"input":400,"output":100,"cached":0,"thoughts":0}}
"#;
    let session = parse_one("resume-replayed.jsonl", replayed);
    assert_eq!(session.metrics.source_tool, "gemini_cli");
    // turn-2's recorded token summary must survive the content-only
    // replay (1000 + turn-3's 400), its tool call must survive, and
    // the user prompt must be billed exactly once.
    assert_eq!(
        session.metrics.tokens_input, 1400,
        "resume replay clobbered the recorded token summary (#29618 writer reset)"
    );
    assert_eq!(session.metrics.tokens_output, 300);
    assert_eq!(session.metrics.tokens_cache_r, 300);
    assert_eq!(
        session.metrics.tool_calls_total, 1,
        "resume replay clobbered the recorded tool call"
    );
    assert_eq!(session.metrics.user_messages, 1);
    // turn-1 user event + turn-2 assistant event + turn-2 tokens meta
    // event + turn-3 assistant event + turn-3 tokens meta event.
    assert_eq!(session.metrics.events_total, 5);
}

#[test]
fn gemini_chats_resume_fold_is_base_gated() {
    // The `_response` fold is gated on the base record being present
    // in the journal: when the writer's history window evicted the
    // base gemini turn (MAX_HISTORY_MESSAGES), a surviving `_response`
    // record is the only carrier of that tool exchange's text and MUST
    // still be billed — exactly as a v0.65.0 reader recovers it via
    // the legacy fallback synthesis.
    let orphan_twin = r#"{"sessionId":"ab12cd34","projectHash":"5c0f7e9a","kind":"main"}
{"id":"turn-9","type":"user","timestamp":"2026-10-20T11:00:00.000Z","content":"Check the build"}
{"id":"evicted-1_response","type":"user","timestamp":"2026-10-20T11:00:02.000Z","content":[{"text":"build ok"}]}
"#;
    let session = parse_one("resume-orphan-twin.jsonl", orphan_twin);
    assert_eq!(session.metrics.source_tool, "gemini_cli");
    assert_eq!(session.metrics.user_messages, 2);
    assert_eq!(session.metrics.events_total, 2);
}

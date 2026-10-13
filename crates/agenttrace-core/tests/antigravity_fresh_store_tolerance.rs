//! rm-947 empty-store tolerance arm (run ff7615be, repository-maintenance
//! 7f0a7958 cycle 3, implement attempt 43e5c129).
//!
//! ccusage #1874/#1875 (merged 2026-10-13) exposed a fresh-install crash
//! class on the antigravity lane: a store that EXISTS but carries no
//! conversations surfaced "no such table: gen_metadata" instead of an
//! empty report. agenttrace's antigravity family is journal-based (the
//! sqlite store read is deliberately out of scope, recorded on the row),
//! so the analogous shapes are pinned here:
//!
//! * a fresh conversation file whose ONLY line is a CONVERSATION_HISTORY
//!   marker — it claims the probe type-wise but emits zero events (the
//!   "store-present, no data" shape): must be skipped as the GENERIC
//!   benign empty-file error ("empty session"), never an
//!   antigravity-specific failure, and never surface as a crash at the
//!   report level;
//! * a torn-tail journal (fresh-store write artifacts): the partial line
//!   must be skipped WITH a line-skip disclosure, and the parseable
//!   prefix must survive;
//! * the corpus-level shape (real discovery root, store present, nothing
//!   scannable) is proven end-to-end by the run probes: exit 0, empty
//!   report, 0 sessions — the empty report, never an error surface.

use agenttrace_core::parse_raw_session;

const FRESH_PATH: &str =
    "/tmp/rm947/.gemini/antigravity/conversations/2026-10-14T10-00-00/rollout.jsonl";
const TORN_PATH: &str =
    "/tmp/rm947/.gemini/antigravity/conversations/2026-10-14T10-00-01/rollout.jsonl";

fn conversation_history_only() -> String {
    concat!(
        r#"{"type":"CONVERSATION_HISTORY","step_index":0,"created_at":1757805600}"#,
        "\n"
    )
    .to_string()
}

fn torn_tail() -> String {
    concat!(
        r#"{"type":"USER_INPUT","message":"hi","step_index":0,"created_at":1757805600}"#,
        "\n",
        r#"{"type":"PLANNER_RESP"#,
    )
    .to_string()
}

#[test]
fn fresh_conversation_file_is_a_benign_skip_never_an_error_crash() {
    // Zero events => the probe declines => the generic empty-file skip.
    // The tolerance contract: the failure mode for a store-present/
    // no-data conversation is exactly this benign skip — never a
    // table/parse/surface error of the ccusage #1874 class.
    match parse_raw_session("fresh", FRESH_PATH, &conversation_history_only()) {
        Ok(session) => {
            // review 6f85e2c9 F4: accept ONLY the empty shape — a
            // garbage-but-Ok regression must not pass this arm. The one
            // counted event IS the marker line itself (the generic
            // jsonl-object census), so events_total is pinned at 1;
            // every conversation-content census — messages, turns,
            // usage, cost — must be zero.
            assert_eq!(
                session.metrics.events_total, 1,
                "fresh conversation must count only its marker line"
            );
            assert_eq!(
                (
                    session.metrics.user_messages,
                    session.metrics.assistant_turns,
                    session.metrics.tool_calls_total,
                    session.metrics.tokens_input,
                    session.metrics.tokens_output
                ),
                (0, 0, 0, 0, 0),
                "fresh conversation must carry zero conversation content and zero usage"
            );
        }
        Err(e) => assert!(
            e.to_string().contains("empty"),
            "fresh conversation must skip as the benign empty-file error, got: {e}"
        ),
    }
}

#[test]
fn torn_tail_journal_skips_with_disclosure_and_keeps_the_prefix() {
    let session = parse_raw_session("torn", TORN_PATH, &torn_tail())
        .expect("torn tail must never fail the parse");
    // The parseable USER_INPUT prefix survives as an event.
    assert!(
        session.metrics.events_total >= 1,
        "parseable prefix must survive the torn tail (events_total={})",
        session.metrics.events_total
    );
    // The torn line is disclosed via the generic line-skip census
    // (jsonl_objects_counted -> finish -> metrics.line_skips), not
    // silently dropped.
    let skips: usize = session.metrics.line_skips.values().sum();
    assert!(
        skips >= 1,
        "torn tail must be disclosed in metrics.line_skips (got {:?})",
        session.metrics.line_skips
    );
}

#[test]
fn empty_raw_file_is_the_generic_empty_skip() {
    // A zero-byte conversation file (the extreme fresh-store shape) must
    // take the same generic skip every lane takes — never an
    // antigravity-specific error.
    match parse_raw_session("empty", FRESH_PATH, "") {
        Ok(session) => {
            // review 6f85e2c9 F4: same tightening — an accepted zero-byte
            // file must be provably empty, not merely Ok.
            assert_eq!(session.metrics.events_total, 0);
            assert_eq!(
                (session.metrics.tokens_input, session.metrics.tokens_output),
                (0, 0)
            );
        }
        Err(e) => assert!(
            e.to_string().contains("empty"),
            "empty file must skip as the benign empty-file error, got: {e}"
        ),
    }
}

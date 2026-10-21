//! rm-042 contract tests: transcript-derived five-hour billing blocks.
//!
//! Red-first provenance: this file was written and run BEFORE
//! `billing_blocks` existed (E0432 unresolved import
//! `billing_blocks`), so the assertions below are the contract the
//! implementation had to satisfy, not a transcription of it.
//!
//! The parity test is the roadmap's acceptance (a): replay the
//! statusline capture semantics for the SAME session and assert the
//! corpus-derived active block reproduces the captured window
//! boundaries to the second — with no statusline environment in play
//! on the derivation side (rm-042: no statusline configured must no
//! longer mean no five-hour window truth anywhere).

use agenttrace_core::{
    billing_blocks, parse_raw_session, render_blocks_view, statusline_insights, CapturedStatusline,
    ModelAttribution,
};
use chrono::{DateTime, TimeZone, Utc};

const PARITY_FIXTURE: &str = include_str!("fixtures/five-hour-blocks-parity.jsonl");
const ROLL_FIXTURE: &str = include_str!("fixtures/five-hour-blocks-roll.jsonl");

fn parse(name: &str, raw: &str) -> agenttrace_core::Session {
    parse_raw_session(name, "session.jsonl", raw).expect("fixture must parse")
}

fn epoch(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
    Utc.with_ymd_and_hms(y, mo, d, h, mi, 0)
        .unwrap()
        .timestamp()
}

#[test]
fn derived_active_block_matches_captured_statusline_window() {
    // rm-042 acceptance (a): the derived block boundaries equal the
    // captured five_hour window boundaries for the same session.
    let session = parse("five-hour-blocks-parity", PARITY_FIXTURE);

    // The capture side, replayed through the REAL journal parse +
    // join path (serde line -> CapturedStatusline -> insights): the
    // provider says the window opened at the session's first use
    // (10:00:00Z) and closes five hours later (15:00:00Z).
    let capture_line = serde_json::json!({
        "captured_at": epoch(2026, 10, 15, 12, 30),
        "payload": {
            "session_id": "parity",
            "rate_limits": {
                "five_hour": {
                    "used_percentage": 84.0,
                    "resets_at": epoch(2026, 10, 15, 15, 0),
                }
            }
        }
    })
    .to_string();
    let capture: CapturedStatusline =
        serde_json::from_str(&capture_line).expect("capture line parses");
    let insights = statusline_insights(&[capture]);
    let captured = insights
        .five_hour
        .as_ref()
        .and_then(|state| state.resets_at)
        .expect("captured five_hour window present");

    // The derivation side: plain corpus, no statusline in play.
    let report = billing_blocks(
        &[session],
        Utc.with_ymd_and_hms(2026, 10, 15, 12, 30, 0).unwrap(),
    );
    assert_eq!(report["schema"], "agenttrace.blocks/1");
    let blocks = report["blocks"].as_array().expect("blocks array");
    assert_eq!(blocks.len(), 1, "all fixture events share one window");
    let block = &blocks[0];
    assert_eq!(block["active"], true);
    assert_eq!(
        block["start_epoch"].as_i64().unwrap(),
        captured - 5 * 3600,
        "derived block start == captured window start (first use)"
    );
    assert_eq!(
        block["end_epoch"].as_i64().unwrap(),
        captured,
        "derived block end == captured resets_at, to the second"
    );
    // Parity provenance: the report must disclose that it derived,
    // not captured, and never read insights.
    let disclosure = report["disclosure"].as_str().unwrap_or_default();
    assert!(disclosure.contains("event timestamps"), "{disclosure}");
}

#[test]
fn block_rolls_five_hours_after_first_use() {
    // rm-042 acceptance (b): events past the five-hour boundary open
    // the NEXT block at their own timestamp (first-use alignment),
    // not on any wall-clock grid.
    let session = parse("five-hour-blocks-roll", ROLL_FIXTURE);
    let report = billing_blocks(
        &[session],
        Utc.with_ymd_and_hms(2026, 10, 15, 16, 0, 0).unwrap(),
    );
    let blocks = report["blocks"].as_array().expect("blocks array");
    assert_eq!(
        blocks.len(),
        2,
        "10:00+10:30 share a block; 15:30 opens the next"
    );
    assert_eq!(
        blocks[0]["start_epoch"].as_i64().unwrap(),
        epoch(2026, 10, 15, 10, 0)
    );
    assert_eq!(
        blocks[0]["end_epoch"].as_i64().unwrap(),
        epoch(2026, 10, 15, 15, 0)
    );
    assert_eq!(blocks[0]["active"], false, "first block closed at 15:00");
    assert_eq!(
        blocks[1]["start_epoch"].as_i64().unwrap(),
        epoch(2026, 10, 15, 15, 30)
    );
    assert_eq!(
        blocks[1]["end_epoch"].as_i64().unwrap(),
        epoch(2026, 10, 15, 20, 30)
    );
    assert_eq!(blocks[1]["active"], true);
    assert_eq!(report["active_block_index"].as_u64().unwrap(), 1);
}

#[test]
fn overlapping_sessions_merge_into_one_billing_block() {
    // rm-042 acceptance (c): concurrent sessions share the provider
    // window they opened together — overlapping per-session windows
    // merge, and the census names both contributors.
    let first = parse("five-hour-blocks-parity", PARITY_FIXTURE);
    let second = parse(
        "five-hour-blocks-overlap",
        "{\"type\":\"assistant\",\"timestamp\":\"2026-10-15T11:00:00Z\",\
         \"message\":{\"usage\":{\"input_tokens\":10,\"output_tokens\":5}},\"costUSD\":0.001}\n\
         {\"type\":\"assistant\",\"timestamp\":\"2026-10-15T13:00:00Z\",\
         \"message\":{\"usage\":{\"input_tokens\":10,\"output_tokens\":5}},\"costUSD\":0.001}\n",
    );
    let report = billing_blocks(
        &[first, second],
        Utc.with_ymd_and_hms(2026, 10, 15, 12, 30, 0).unwrap(),
    );
    let blocks = report["blocks"].as_array().expect("blocks array");
    assert_eq!(
        blocks.len(),
        1,
        "both sessions fall inside one shared window"
    );
    assert_eq!(blocks[0]["sessions_contributing"], 2);
    assert_eq!(
        blocks[0]["start_epoch"].as_i64().unwrap(),
        epoch(2026, 10, 15, 10, 0)
    );
    assert_eq!(
        report["corpus"]["attribution"]["event_basis"]
            .as_u64()
            .unwrap(),
        2
    );
}

#[test]
fn sessions_without_stamps_anchor_at_span_start_and_are_censused() {
    // rm-201 census discipline, inherited: no per-event stamps (warm
    // cache or exotic lanes) -> span anchor at session start, counted
    // as span-basis, never silently blended with event-basis rows.
    let mut session = parse("five-hour-blocks-parity", PARITY_FIXTURE);
    session.metrics.timestamps.clear();
    session.metrics.session_start = "2026-10-15T09:00:00Z".to_string();
    // Known-cost attribution injected directly: the contract pins the
    // module's span/pro-rata cost math, not the parser's per-model
    // cost derivation (minimal fixtures carry no priced model).
    session.metrics.model_attribution.insert(
        "claude-sonnet-4-5".to_string(),
        ModelAttribution {
            input_tokens: 300,
            output_tokens: 150,
            cache_w: 0,
            cache_r: 0,
            cost_usd: 0.06,
        },
    );
    let report = billing_blocks(
        &[session],
        Utc.with_ymd_and_hms(2026, 10, 15, 9, 30, 0).unwrap(),
    );
    assert_eq!(report["corpus"]["attribution"]["span_basis"], 1);
    assert_eq!(report["corpus"]["attribution"]["event_basis"], 0);
    let blocks = report["blocks"].as_array().expect("blocks array");
    assert_eq!(blocks.len(), 1);
    assert_eq!(
        blocks[0]["start_epoch"].as_i64().unwrap(),
        epoch(2026, 10, 15, 9, 0)
    );
    assert_eq!(
        blocks[0]["input_tokens"].as_i64().unwrap(),
        300,
        "span basis attributes the whole session to its anchor window"
    );
    assert_eq!(
        blocks[0]["cost_usd"].as_f64().unwrap(),
        0.06,
        "span basis carries the session's full attributed cost"
    );
}

#[test]
fn json_text_render_and_format_refusals() {
    let session = parse("five-hour-blocks-parity", PARITY_FIXTURE);
    let now: DateTime<Utc> = Utc.with_ymd_and_hms(2026, 10, 15, 12, 30, 0).unwrap();
    let json =
        render_blocks_view("json", std::slice::from_ref(&session), now).expect("json renders");
    assert!(json.contains("agenttrace.blocks/1"));
    let text =
        render_blocks_view("text", std::slice::from_ref(&session), now).expect("text renders");
    assert!(text.contains("5-hour billing blocks"));
    assert!(text.contains("<== active"), "text marks the active block");
    let refused = render_blocks_view("csv", std::slice::from_ref(&session), now)
        .expect_err("csv must be refused loudly");
    assert!(refused.contains("only json and text"), "{refused}");
    let md = render_blocks_view("md", std::slice::from_ref(&session), now)
        .expect_err("md must be refused loudly");
    assert!(md.contains("only json and text"), "{md}");
}

//! rm-538 — workbuddy input-basis disclosures are not parse loss.
//!
//! The corpus below is the run-cb38b958 assess F1 PoC shape (wb-priced
//! session: a cache-inclusive input basis — `input_tokens` already has
//! `cache_read_input_tokens` subtracted upstream, so the two together
//! disclose `workbuddy_input_basis:cache_subtracted`). rm-450 first
//! surfaced that disclosure by minting it into `Metrics.line_skips`,
//! where it rode the "Dropped lines" parse-loss channel and tanked
//! `data_health.confidence` to "low" even though the parse was exact
//! (1/1 parsed, 0 unparseable, 0 schema skips). rm-538 moves the
//! counters to `Metrics.disclosure_counters` — the non-loss channel
//! the pi-family journal facts already use (rm-436/rm-437) — so:
//!
//! 1. `data_health.confidence` stays high (it keys on real parse loss);
//! 2. `data_health.line_skips` is empty and json carries the counters
//!    under `data_health.disclosures` (the named disclosure map);
//! 3. report renders show a "Disclosed facts" row (with the basis
//!    provenance note) and no "Dropped lines" row for this corpus.
//!
//! The fixture is synthetic (mirrors the live PoC at
//! /tmp/at-assess-a1a0/corpus3, wb-priced session), not a recording.
//!
//! rm-719 (cycle-3 disclosure-plane batch, run 5417681937ae)
//! generalized the channel contract onto the kimi usage-alias family:
//! every `kimi_usage_alias:<key>` counter a journal's vendor wire keys
//! produced used to ride `line_skips` too, so a healthy kimi corpus
//! reported LOW confidence and "Dropped lines" for the crime of
//! matching real vendor keys. The kimi section below pins the same
//! three-point contract on the vendored official kimi-code wire corpus
//! plus the counterfactual: genuine parse loss still degrades.

use std::fs;

use agenttrace_core::{
    compute_overview, data_health, parse_file, parse_raw_session,
    report_overview_markdown_with_context, Session, TimeRange,
};

/// Cache-inclusive basis: 2,000 reported input with 1,500 cache-read —
/// the subtraction assumption fires and is disclosed, but nothing is
/// lost (mirrors testdata/workbuddy/usage-basis-inclusive.jsonl shape
/// with the model carried on the message, as the wb-priced PoC does).
const WB_PRICED_INCLUSIVE: &str = concat!(
    r#"{"type": "function_call", "name": "bash", "arguments": "{}", "sessionId": "wb-priced", "cwd": "/tmp/proj", "callId": "c1", "providerData": {"model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "message", "role": "user", "content": [{"type": "text", "text": "priced"}], "sessionId": "wb-priced", "cwd": "/tmp/proj", "message": {"role": "user", "usage": {"input_tokens": 2000, "output_tokens": 50, "cache_read_input_tokens": 1500}}}"#,
    "\n",
    r#"{"type": "function_call_result", "output": "done", "callId": "c1", "sessionId": "wb-priced", "cwd": "/tmp/proj"}"#,
);

fn parse_fixture(name: &str, body: &str) -> Session {
    let dir = std::env::temp_dir().join(format!("rm538-{}-{}", name, std::process::id()));
    fs::create_dir_all(&dir).expect("tempdir");
    let file = dir.join(format!("{name}.jsonl"));
    fs::write(&file, body).expect("fixture write");
    let session = parse_file(&file).expect("parses as workbuddy");
    let _ = fs::remove_dir_all(&dir);
    session
}

fn parse_vendored(rel: &str) -> Session {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel);
    parse_file(&path).unwrap_or_else(|error| panic!("parse {path:?}: {error:#}"))
}

// ---------------------------------------------------------------------
// rm-719 — kimi usage-alias disclosures (informational, non-loss)
// ---------------------------------------------------------------------

/// The vendored official kimi-code wire corpus (130 lines, 15
/// usage-bearing `StatusUpdate` records) matches the vendor's real
/// wire keys (`input_other`, `output`, `input_cache_read`,
/// `input_cache_creation`) through the shared alias table, and every
/// match ships a `kimi_usage_alias:<key>` counter. Those counters are
/// informational — they ride the disclosure channel, not the loss
/// channel.
#[test]
fn kimi_alias_disclosures_ride_the_non_loss_channel() {
    let session = parse_vendored("kimi-cli/wire.jsonl");
    assert_eq!(session.metrics.source_tool, "kimi_cli");
    assert_eq!(session.metrics.tokens_input, 62_198);
    assert_eq!(session.metrics.tokens_cache_r, 496_640);
    for key in [
        "kimi_usage_alias:input_other",
        "kimi_usage_alias:output",
        "kimi_usage_alias:input_cache_read",
        "kimi_usage_alias:input_cache_creation",
    ] {
        assert_eq!(
            session.metrics.disclosure_counters.get(key),
            Some(&15),
            "disclosure_counters: {:#?}",
            session.metrics.disclosure_counters
        );
    }
    assert!(
        session.metrics.line_skips.is_empty(),
        "alias matches are not parse loss: line_skips {:#?}",
        session.metrics.line_skips
    );
}

/// A healthy kimi-alias corpus keeps high confidence with the
/// counters still visible. (The wire format carries no model field, so
/// the session reports the sentinel `default`; the test patches in a
/// concrete model to isolate the alias question — the same precedent
/// attribution_dimensions.rs uses for vendor buckets.)
#[test]
fn kimi_alias_corpus_confidence_is_not_degraded() {
    let mut session = parse_vendored("kimi-cli/wire.jsonl");
    session.metrics.model_used = "moonshot/kimi-k2.6".to_string();
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(
        health.confidence, "high",
        "alias disclosures must not degrade confidence: {health:#?}"
    );
    assert_eq!(
        health.disclosures.get("kimi_usage_alias:input_other"),
        Some(&15),
        "health.disclosures: {:#?}",
        health.disclosures
    );
    assert!(health.line_skips.is_empty());

    let overview = compute_overview(std::slice::from_ref(&session));
    let markdown = report_overview_markdown_with_context(
        &overview,
        std::slice::from_ref(&session),
        &health,
        TimeRange::All,
        false,
    );
    assert!(
        markdown.contains("Disclosed facts"),
        "markdown:\n{markdown}"
    );
    assert!(
        markdown.contains("kimi_usage_alias:input_other=15"),
        "markdown:\n{markdown}"
    );
    assert!(
        !markdown.contains("Dropped lines"),
        "alias matches must not render as dropped lines:\n{markdown}"
    );
}

/// The counterfactual, same corpus shape minus the alias question: a
/// genuinely unparseable line rides `line_skips` and DOES degrade
/// confidence. The channel split is about truthfulness, not about
/// hiding problems.
#[test]
fn parse_loss_counters_still_degrade_confidence() {
    let healthy = concat!(
        r#"{"role":"session_meta","timestamp":"2026-10-07T12:00:00Z","model_used":"gpt-4o","usage":{"input_tokens":10,"output_tokens":2}}"#,
        "\n",
        r#"{"role":"user","timestamp":"2026-10-07T12:00:01Z","content":"q"}"#,
        "\n"
    );
    let session =
        parse_raw_session("healthy.jsonl", "healthy.jsonl", healthy).expect("healthy corpus");
    assert_eq!(data_health(&[session], 1, 0).confidence, "high");

    let torn = concat!(
        r#"{"role":"session_meta","timestamp":"2026-10-07T12:00:00Z","model_used":"gpt-4o","usage":{"input_tokens":10,"output_tokens":2}}"#,
        "\n",
        r#"{"role":"user","timestamp":"2026-10-07T12:00:01Z","content":"q"}"#,
        "\n",
        "{\"role\":\"assistant\",\"timestamp\":\"2026-10-07T12:00:0",
    );
    let session = parse_raw_session("torn.jsonl", "torn.jsonl", torn).expect("torn corpus");
    assert_eq!(session.metrics.line_skips.get("unparseable_line"), Some(&1));
    assert_eq!(data_health(&[session], 1, 0).confidence, "low");
}

#[test]
fn workbuddy_basis_disclosure_rides_the_disclosure_channel_not_parse_loss() {
    let session = parse_fixture("inclusive", WB_PRICED_INCLUSIVE);
    assert_eq!(session.metrics.source_tool, "workbuddy");

    // The counters moved channels (rm-538): present under
    // disclosure_counters, absent from line_skips.
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("workbuddy_input_basis:cache_subtracted"),
        Some(&1),
        "the basis note must still be disclosed somewhere: {:?}",
        session.metrics.disclosure_counters
    );
    assert!(
        session.metrics.line_skips.is_empty(),
        "an assumption disclosure is not a dropped line: {:?}",
        session.metrics.line_skips
    );

    // And nothing was actually lost: the usage block is counted.
    assert_eq!(session.metrics.tokens_input, 500);
    assert_eq!(session.metrics.tokens_cache_r, 1500);
}

#[test]
fn exact_parse_confidence_is_not_degraded_by_a_basis_disclosure() {
    let session = parse_fixture("confidence", WB_PRICED_INCLUSIVE);
    let health = data_health(&[session], 1, 0);

    assert_eq!(health.confidence, "high");
    assert!(
        health.line_skips.is_empty(),
        "Dropped-lines map must not carry the basis note: {:?}",
        health.line_skips
    );
    assert_eq!(
        health
            .disclosures
            .get("workbuddy_input_basis:cache_subtracted"),
        Some(&1),
        "json data_health.disclosures carries the named disclosure: {:?}",
        health.disclosures
    );
}

#[test]
fn report_renders_disclosed_facts_without_a_dropped_lines_row() {
    let session = parse_fixture("render", WB_PRICED_INCLUSIVE);
    let session_ref = &session;
    let health = data_health(std::slice::from_ref(session_ref), 1, 0);
    let markdown = report_overview_markdown_with_context(
        &compute_overview(std::slice::from_ref(session_ref)),
        std::slice::from_ref(session_ref),
        &health,
        TimeRange::All,
        false,
    );
    let text = markdown;

    assert!(
        text.contains("Disclosed facts"),
        "the disclosure row must stay visible (rm-450's goal survives the channel move):\n{text}"
    );
    assert!(
        text.contains("workbuddy_input_basis:cache_subtracted=1"),
        "the counter renders with its provenance note:\n{text}"
    );
    assert!(
        !text.contains("Dropped lines"),
        "an otherwise-exact parse must not present a parse-loss row:\n{text}"
    );
    assert!(
        text.contains("| Confidence | high |"),
        "confidence stays high through the report arm:\n{text}"
    );
}

// ---------------------------------------------------------------------------
// rm-730 (run 4c3ca863, cycle 3): codex structural-skip counters are
// assumption disclosures, not parse loss. The live 39acfe43 PoC: a real
// ~/.codex corpus parsed 97/97 files with 0 skipped and every cost
// finite, yet data_health.confidence read "low" because 1,125
// codex_ignorable_line + codex_world_state counters — deterministic,
// known-non-loss shapes the codex format defines as ignorable chatter
// and as a deliberately-ignored sub-registry record — rode line_skips
// and tripped the dropped-lines degrade. The adjudication moved those
// two STRUCTURAL counters onto the disclosure_counters channel (the
// rm-538 split this file pins): still counted, still disclosed in the
// report and data_health.disclosures, but no longer degrading an
// otherwise-exact parse. TRUE loss (codex_unparseable_line,
// codex_non_object_line:*, codex_missing_type, codex_unmatched_*)
// stays on line_skips and keeps degrading confidence.
// ---------------------------------------------------------------------------

const CODEX_EXACT_PARSE_WITH_STRUCTURAL_LINES: &str = concat!(
    r#"{"timestamp":"2026-10-04T12:00:00Z","type":"session_meta","payload":{"cwd":"/tmp/probe","model":"gpt-5.3-codex"}}"#,
    "\n",
    r#"{"timestamp":"2026-10-04T12:00:01Z","type":"event_msg","payload":{"type":"agent_message_delta","delta":"chatter one"}}"#,
    "\n",
    r#"{"timestamp":"2026-10-04T12:00:02Z","type":"event_msg","payload":{"type":"agent_message_delta","delta":"chatter two"}}"#,
    "\n",
    r#"{"timestamp":"2026-10-04T12:00:03Z","type":"world_state","payload":{"registry":"opaque"}}"#,
    "\n",
    r#"{"timestamp":"2026-10-04T12:00:05Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":40,"reasoning_output_tokens":0}}}}"#,
);

#[test]
fn codex_structural_counters_ride_the_disclosure_channel_not_line_skips() {
    let session = parse_fixture("rm730-structural", CODEX_EXACT_PARSE_WITH_STRUCTURAL_LINES);

    assert_eq!(
        session.metrics.tokens_input, 100,
        "the token_count line is still rescued and counted"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("codex_ignorable_line"),
        Some(&2),
        "event_msg chatter stays counted, on the disclosure channel: {:?}",
        session.metrics.disclosure_counters
    );
    assert_eq!(
        session.metrics.disclosure_counters.get("codex_world_state"),
        Some(&1),
        "the world_state registry record stays counted, on the disclosure channel"
    );
    assert!(
        session.metrics.line_skips.is_empty(),
        "known-non-loss shapes must not render as dropped lines (rm-730): {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn codex_structural_counters_do_not_degrade_an_exact_parse_confidence() {
    // The 39acfe43 corpus shape: every file parsed, nothing skipped,
    // costs finite — confidence must read high even though 1,125→3
    // structural counters are disclosed.
    let session = parse_fixture("rm730-confidence", CODEX_EXACT_PARSE_WITH_STRUCTURAL_LINES);
    let health = data_health(std::slice::from_ref(&session), 1, 0);

    assert_eq!(
        health.confidence, "high",
        "the exact-parse corpus the 39acfe43 PoC measured must not read low"
    );
    assert_eq!(
        health.disclosures.get("codex_ignorable_line"),
        Some(&2),
        "the structural counter stays visible in json data_health.disclosures"
    );
    assert_eq!(
        health.disclosures.get("codex_world_state"),
        Some(&1),
        "both structural counters survive the move onto disclosures"
    );
}

#[test]
fn codex_true_loss_still_degrades_confidence_through_line_skips() {
    // The boundary the adjudication preserved: a genuinely dropped
    // line (torn tail / bare scalar) is real parse loss and keeps the
    // 39acfe43 contract — line_skips non-empty, confidence low.
    let mut raw = CODEX_EXACT_PARSE_WITH_STRUCTURAL_LINES.to_string();
    raw.push('\n');
    raw.push_str("{ not json");
    let session = parse_fixture("rm730-loss", &raw);

    assert_eq!(
        session.metrics.line_skips.get("codex_unparseable_line"),
        Some(&1),
        "a non-JSON line stays a true parse loss: {:?}",
        session.metrics.line_skips
    );
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(
        health.confidence, "low",
        "dropped lines must still degrade confidence alongside the disclosures"
    );
}

#[test]
fn codex_structural_counters_render_as_disclosed_facts_not_dropped_lines() {
    let session = parse_fixture("rm730-render", CODEX_EXACT_PARSE_WITH_STRUCTURAL_LINES);
    let session_ref = &session;
    let health = data_health(std::slice::from_ref(session_ref), 1, 0);
    let text = report_overview_markdown_with_context(
        &compute_overview(std::slice::from_ref(session_ref)),
        std::slice::from_ref(session_ref),
        &health,
        TimeRange::All,
        false,
    );

    assert!(
        text.contains("Disclosed facts"),
        "the structural counters render in the disclosure row:\n{text}"
    );
    assert!(
        text.contains("codex_ignorable_line=2"),
        "the chatter counter renders with its count:\n{text}"
    );
    assert!(
        text.contains("codex_world_state=1"),
        "the world_state counter renders with its count:\n{text}"
    );
    assert!(
        !text.contains("Dropped lines"),
        "an otherwise-exact codex parse must not present a parse-loss row:\n{text}"
    );
    assert!(
        text.contains("| Confidence | high |"),
        "the report arm agrees confidence is high:\n{text}"
    );
}

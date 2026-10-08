//! rm-659: pre-2026-09 Codex rollouts emit `event_msg` token_count
//! events whose `info` carries no `total_token_usage` /
//! `last_token_usage` sub-object (the count-bearing shape landed
//! upstream later). The parser used to drop those lines silently, so a
//! zero-token codex session was indistinguishable from a rollout with
//! no usage data at all — an unmarked estimate fallback.
//!
//! Post-fix: every shape-gap token_count event is counted in parse
//! diagnostics as `codex_token_count_without_totals` (the pass-7
//! line_skips disclosure channel), while a genuinely-zero snapshot
//! (count-bearing shape present, zero values) stays silent — zero is a
//! measured answer, a missing shape is not.

use agenttrace_core::parse_raw_session;

const ROLLOUT: &str = concat!(
    r#"{"timestamp":"2026-08-01T10:00:00.000Z","type":"session_meta","payload":{"cwd":"/work/x"}}"#,
    "\n",
    r#"{"timestamp":"2026-08-01T10:00:01.000Z","type":"turn_context","payload":{"model":"gpt-5-codex"}}"#,
    "\n",
    // Count-bearing shape: counted normally (500 in / 50 out).
    r#"{"timestamp":"2026-08-01T10:00:02.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":500,"cached_input_tokens":0,"output_tokens":50,"reasoning_output_tokens":0}}}}"#,
    "\n",
    // Pre-2026-09 shape: info present, no totals sub-object -> disclosed.
    r#"{"timestamp":"2026-08-01T10:00:03.000Z","type":"event_msg","payload":{"type":"token_count","info":{"rate_limits":{}}}}"#,
    "\n",
    // No info at all -> disclosed.
    r#"{"timestamp":"2026-08-01T10:00:04.000Z","type":"event_msg","payload":{"type":"token_count"}}"#,
    "\n",
    // Count-bearing but all-zero snapshot: a measured zero, NOT a
    // shape gap — must stay silent.
    r#"{"timestamp":"2026-08-01T10:00:05.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":0,"cached_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0}}}}"#,
    "\n",
);

#[test]
fn codex_token_count_events_without_totals_are_disclosed_not_silent() {
    let session =
        parse_raw_session("rollout", "rollout-2026-08-01.jsonl", ROLLOUT).expect("rollout parses");
    assert_eq!(session.metrics.tokens_input, 500);
    assert_eq!(session.metrics.tokens_output, 50);
    let disclosed = session
        .metrics
        .line_skips
        .get("codex_token_count_without_totals")
        .copied()
        .unwrap_or(0);
    assert_eq!(
        disclosed, 2,
        "exactly the two shape-gap token_count events; the counted snapshot and the measured-zero \
         snapshot must stay silent (line_skips: {:?})",
        session.metrics.line_skips
    );
}

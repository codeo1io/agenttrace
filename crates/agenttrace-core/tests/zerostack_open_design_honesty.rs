//! Zerostack + Open Design coarse-aggregate honesty lanes (rm-2375, run
//! fcbdf5ba cycle 5).
//!
//! Acceptance (band bf7894ca): naive per-call summation MUST fail these
//! fixtures — Zerostack stores session-CUMULATIVE totals (summing them
//! per-message over-counts by the message count); Open Design usage rides
//! PERIODIC agent events (summing every `usage` event over-counts by the
//! re-report factor). Granularity is DISCLOSED per session.

use agenttrace_core::{parse_raw_session, Event};

fn events_total_usage(events: &[Event], key: &str) -> i64 {
    events.iter().map(|e| e.usage.get(key).copied().unwrap_or(0)).sum()
}

// ---- Zerostack: ~/.local/share/zerostack/sessions/<id>.json
// shape (cb-zerostack.md): { created_at, workspace, total_input_tokens,
// total_output_tokens, messages: [ { estimated_tokens, ... } ] }

fn zerostack_session_json() -> String {
    r#"{
  "created_at": "2026-10-20T10:00:00Z",
  "workspace": "/work/demo",
  "total_input_tokens": 1200,
  "total_output_tokens": 300,
  "total_cost": 9.99,
  "messages": [
    {"role": "user", "estimated_tokens": 400},
    {"role": "assistant", "estimated_tokens": 100},
    {"role": "user", "estimated_tokens": 400},
    {"role": "assistant", "estimated_tokens": 200}
  ]
}"#
    .to_string()
}

#[test]
fn zerostack_reports_one_attributed_call_not_per_message_sum() {
    let events = agenttrace_core::probe_events_for_fixture(
        "s1.json",
        "zerostack",
        &zerostack_session_json(),
    );
    // The naive trap: 400+100+400+200 = 1100 input would over-count a
    // cumulative total by message count; and summing the TOTAL across
    // messages would give 4x1200. The honest value is exactly ONE call at
    // the cumulative totals.
    assert_eq!(events_total_usage(&events, "input_tokens"), 1200);
    assert_eq!(events_total_usage(&events, "output_tokens"), 300);
    // granularity disclosure present and named
    assert!(
        events.iter().any(|e| e
            .tool_calls
            .iter()
            .any(|c| c.name.contains("cumulative-session-totals")))
            || events
                .iter()
                .any(|e| e.model_used.contains("zerostack:") || e.role.contains("zerostack")),
        "zerostack attribution granularity must be disclosed on the events"
    );
    // per-message estimated tokens must NOT be added to metered totals
    assert_ne!(events_total_usage(&events, "input_tokens"), 1100);
}

// ---- Open Design: <base>/data/runs/<runId>/events.jsonl
// shape (cb-open-design.md): periodic `agent` events with
// data.type == "usage" carrying CURRENT cumulative usage snapshots;
// identical consecutive snapshots mean NO new usage. Multiple base
// aliases resolve to the same run — dedupe by resolved path happens in
// discovery, here we pin the delta semantics.

fn open_design_events_jsonl() -> String {
    let mut lines = Vec::new();
    let mut push = |typ: &str, inp: i64, out: i64, cached: i64| {
        lines.push(format!(
            r#"{{"type": "agent", "data": {{"type": "{typ}", "usage": {{"input_tokens": {inp}, "output_tokens": {out}, "cached_read_tokens": {cached}}}}}}"#
        ));
    };
    // snapshot sequence with repeats: 100 -> 100 (no delta) -> 250 -> 250
    push("usage", 100, 20, 5);
    push("usage", 100, 20, 5);
    push("usage", 250, 60, 15);
    push("usage", 250, 60, 15);
    lines.join("\n")
}

#[test]
fn open_design_sums_deltas_not_snapshots() {
    let events = agenttrace_core::probe_events_for_fixture(
        "events.jsonl",
        "open-design",
        &open_design_events_jsonl(),
    );
    // Naive trap: summing all snapshots = 700 input. Honest: forward
    // deltas of DISTINCT consecutive values = 100 + 150 = 250.
    assert_eq!(events_total_usage(&events, "input_tokens"), 250);
    assert_eq!(events_total_usage(&events, "output_tokens"), 60);
    assert_eq!(events_total_usage(&events, "cache_read_tokens"), 15);
}

#[test]
fn open_design_non_usage_agent_events_are_ignored() {
    let raw = [
        r#"{"type": "agent", "data": {"type": "message", "text": "hi"}}"#,
        r#"{"type": "run", "data": {"type": "usage", "usage": {"input_tokens": 999, "output_tokens": 999}}}"#,
    ]
    .join("\n");
    let events =
        agenttrace_core::probe_events_for_fixture("events.jsonl", "open-design", &raw);
    // run-level usage events are NOT attributed to the agent stream; only
    // agent/periodic usage shapes count (defensive against shape drift).
    assert_eq!(events_total_usage(&events, "input_tokens"), 0);
}

// ---- sanity: the shared probe entry refuses foreign shapes

#[test]
fn foreign_jsonl_is_not_claimed_by_the_new_probes() {
    let raw = r#"{"mystery": true}"#;
    let events =
        agenttrace_core::probe_events_for_fixture("x.jsonl", "open-design", raw);
    assert!(events.is_empty());
    let events =
        agenttrace_core::probe_events_for_fixture("x.json", "zerostack", r#"{"nope": 1}"#);
    assert!(events.is_empty());
}

// ---- keep a baseline call into parse_raw_session so the file exercises
// the public seam even before the new probes register.
#[test]
fn parse_raw_session_still_handles_plain_claude_lines() {
    let s = parse_raw_session("x", "claude", r#"{"type": "user", "message": {"content": []}}"#);
    assert_eq!(s.label, "claude");
}

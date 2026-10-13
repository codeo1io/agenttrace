//! pi cancelled-run consumption contract (rm-990, cycle 2 of run
//! b5affead). pi ≥1.1.0 (2026-10-07, #10607) added an `aborted` flag
//! to the agent-settled event, and the live wire census that motivated
//! this arm (23,174 journals on this host, 2026-10-12) found the
//! signal persisted in TWO shapes:
//!
//! 1. `{"type":"agent_settled","aborted":true}` — the changelog's
//!    named surface. The census found ZERO persisted entries of this
//!    type (pi's dist settles only on the in-process event stream),
//!    but format v3 is additive-only, so the arm consumes the entry
//!    defensively: named by the mint, absence-tolerant, counted when
//!    present.
//! 2. `message.stopReason:"aborted"` — the arm that actually fires on
//!    real journals (211 lines across 196 host files, including
//!    pre-1.1.0 senpi writers; pi 1.1.0 also adds a message-level
//!    `durationMs` that carries NO abort semantics and is ignored).
//!
//! Both arms mint the SAME disclosure counter, `pi_run_aborted`, so a
//! cancelled run surfaces once per abort signal in health/data-health
//! disclosures, the overview incident timeline ("Run cancelled"), and
//! the governance cost audit (`aborted_sessions`), which splits
//! cancelled vs finished sessions. Zero-signal corpora stay
//! byte-identical: no counter key, no timeline item, no audit field
//! (skip-if-zero serialization).

use agenttrace_core::{compute_overview, cost_audit, parse_raw_session};
use std::collections::BTreeMap;

fn fixture(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path))
        .unwrap_or_else(|error| panic!("missing fixture {path}: {error}"))
}

fn parse_fixture(name: &str, path: &str) -> agenttrace_core::Session {
    parse_raw_session(name, path, &fixture(path))
        .unwrap_or_else(|error| panic!("{name} must parse as pi journal: {error}"))
}

const ABORTED_FIXTURE: &str = "tests/fixtures/pi-aborted-settled/journal.jsonl";
const V3_BASELINE: &str = "tests/fixtures/pi-pi-v3/journal.jsonl";

#[test]
fn aborted_signals_land_in_disclosures() {
    // The fixture carries exactly one of each arm — one assistant
    // message with `stopReason:"aborted"` and one
    // `agent_settled{aborted:true}` entry — so the counter must be 2.
    let session = parse_fixture("pi-aborted", ABORTED_FIXTURE);
    let counters: BTreeMap<String, usize> = session
        .metrics
        .disclosure_counters
        .iter()
        .map(|(key, value)| (key.clone(), *value))
        .collect();
    assert_eq!(
        counters.get("pi_run_aborted"),
        Some(&2usize),
        "one stopReason abort + one agent_settled abort must both mint pi_run_aborted; got {counters:?}"
    );
    // The settled entry itself is HANDLED — it must no longer fall
    // through to the unknown-entry skip counter.
    assert!(
        !counters.contains_key("pi_entry_skipped:agent_settled"),
        "agent_settled must be consumed by the rm-990 arm, not disclosed as skipped; got {counters:?}"
    );
}

#[test]
fn abort_signal_counting_is_not_triggered_by_sibling_fields() {
    // `durationMs` (pi 1.1.0 #10549) and non-aborted stop reasons
    // carry no cancellation semantics: a journal with `durationMs`
    // and `stopReason:"end_turn"` must mint NO abort counter.
    let raw = concat!(
        r#"{"id":"a1","timestamp":"2026-10-08T09:00:00.000Z","type":"session","version":3,"cwd":"/p"}"#,
        "\n",
        r#"{"id":"a2","parentId":"a1","timestamp":"2026-10-08T09:00:00.100Z","type":"session_info","name":"clean"}"#,
        "\n",
        r#"{"id":"a3","timestamp":"2026-10-08T09:00:01.000Z","message":{"role":"user","content":"hi","timestamp":"2026-10-08T09:00:01.000Z"},"type":"message"}"#,
        "\n",
        r#"{"id":"a4","timestamp":"2026-10-08T09:00:05.000Z","message":{"role":"assistant","content":"done","timestamp":"2026-10-08T09:00:05.000Z","usage":{"input":10,"output":5,"cacheRead":0,"cacheWrite":0,"totalTokens":15,"cost":{"input":0.0,"output":0.0,"total":0.0001}},"model":"claude-sonnet-4-20250514","stopReason":"end_turn","durationMs":900},"type":"message"}"#,
        "\n",
    );
    let session =
        parse_raw_session("clean-settled", "pi-clean.jsonl", raw).expect("clean journal parses");
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("pi_run_aborted"),
        "end_turn + durationMs must not mint an abort signal; got {:?}",
        session.metrics.disclosure_counters
    );
}

#[test]
fn absent_signals_leave_the_v3_baseline_untouched() {
    // Golden on the pre-existing v3 corpus: no abort signal, no new
    // counter key, no change to the parsed shape. This is the
    // absence-tolerance arm — every journal written before pi 1.1.0
    // (and every future journal without an abort) must be
    // byte-identical to before the rm-990 arm existed.
    let session = parse_fixture("pi-v3-baseline", V3_BASELINE);
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("pi_run_aborted"),
        "the v3 baseline corpus has no abort signals; got {:?}",
        session.metrics.disclosure_counters
    );
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("pi_entry_skipped:agent_settled"),
        "the v3 baseline corpus has no agent_settled entries"
    );
}

#[test]
fn cancelled_run_surfaces_in_overview_incident_timeline() {
    // The insight surface: a cancelled run reads as "Run cancelled"
    // in the overview incident timeline (health view), with the
    // signal count and both provenance arms in the detail.
    let session = parse_fixture("pi-aborted", ABORTED_FIXTURE);
    let overview = compute_overview(std::slice::from_ref(&session));
    let json = agenttrace_core::report_overview_json(&overview, std::slice::from_ref(&session));
    assert!(
        json.contains("Run cancelled"),
        "overview JSON must carry the cancelled-run insight; timeline: {json}"
    );
    assert!(
        json.contains("pi_run_aborted"),
        "the incident detail must name the disclosure counter so the reader can trace it"
    );
}

#[test]
fn cancelled_sessions_split_in_the_cost_audit() {
    // The governance surface: the audit splits cancelled sessions out
    // (rm-990 mint: "cancelled-vs-finished split in health/insights/
    // governance"). One session with abort signals -> 1; the clean
    // baseline -> 0 AND the field must be absent from the serialized
    // audit so zero-signal corpora keep byte-identical audits.
    let cancelled = parse_fixture("pi-aborted", ABORTED_FIXTURE);
    let audit = cost_audit(std::slice::from_ref(&cancelled));
    assert_eq!(audit.aborted_sessions, 1, "one cancelled session");
    let rendered = serde_json::to_string(&audit).expect("audit serializes");
    assert!(
        rendered.contains("\"aborted_sessions\":1"),
        "the split must be visible in the audit JSON: {rendered}"
    );

    let clean = parse_fixture("pi-v3-baseline", V3_BASELINE);
    let clean_audit = cost_audit(std::slice::from_ref(&clean));
    assert_eq!(clean_audit.aborted_sessions, 0, "baseline has no aborts");
    let clean_rendered = serde_json::to_string(&clean_audit).expect("audit serializes");
    assert!(
        !clean_rendered.contains("aborted_sessions"),
        "skip-if-zero keeps clean audits byte-identical: {clean_rendered}"
    );
}

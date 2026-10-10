//! rm-956: Copilot OTel export seam — re-export dedupe (arm 1) and the
//! per-(conversation, model) cross-root reconcile between
//! `~/.copilot/session-state` snapshots and `~/.copilot/otel` spans
//! (arm 2), ported from ccusage #1857 (observed 2–3× inflation) and
//! PR #1858's reference semantics with the documented divergence on
//! the replacement trigger (calls-vs-reported-calls, not any-row).
//!
//! Fixtures (all under `copilot-cross-root/`, byte-stable):
//! * `session-state/conv-twin/events.jsonl` — clean shutdown: two
//!   cumulative snapshots (requests 1 → 2, usage 100/20 → 200/40);
//! * `otel/twin.jsonl` — the SAME conversation's export: two genuine
//!   calls (distinct response ids) plus ONE re-export of the first
//!   (same response id + model + usage, new trace/span) — arm 1 must
//!   drop the re-export at parse time, arm 2 must then drop BOTH kept
//!   calls as covered by the shutdown (fleet counts 200/40 once, not
//!   400/80, and the OTel session discloses the two dropped calls);
//! * `session-state/conv-killed/events.jsonl` — killed process: one
//!   checkpoint claiming 1 request/100 tokens, no shutdown;
//! * `otel/killed.jsonl` — the same conversation's export: TWO calls
//!   before the checkpoint (the checkpoint undercounted them) plus one
//!   after it, plus a re-export of the first — arm 1 drops the
//!   re-export, arm 2 fires the replacement (2 calls vs 1 reported),
//!   keeps every OTel call and subtracts the snapshot's whole usage
//!   from the session-state session (fleet counts 300/60, the export
//!   is the better witness);
//! * `otel/unpaired.jsonl` — a chat span with NO conversation id: it
//!   never joins a session-state root and its usage stands.
//! * `session-state/conv-split/events.jsonl` + `events-2.jsonl` +
//!   `otel/split.jsonl` — one conversation spanning TWO session-state
//!   journals (review 23701305 F3): the final snapshot lives in the
//!   LOWER-indexed session while `events-2.jsonl` holds an earlier
//!   fragment; the replacement delta must land on the final
//!   snapshot's exact owner, never on the max-indexed state session.
//! * `otel/timestampless.jsonl` — a re-export pair whose FIRST span
//!   lacks `startTimeUnixNano` (review 23701305 F4): the kept export
//!   must be the timestamped one so the call fact carries the real
//!   window position.

use agenttrace_core::{parse_file, reconcile_copilot_cross_root};
use std::path::PathBuf;

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/copilot-cross-root")
        .join(relative)
}

#[test]
fn arm1_reexported_chat_spans_count_once() {
    // twin.jsonl carries 3 chat spans; the second is a byte-identical
    // re-export of the first's response under a new trace/span pair.
    // Pre-rm-956 this file parsed as 300/60 (the double-count ccusage
    // #1857 observed in the wild); the kept world must be 200/40.
    let session = parse_file(&fixture("otel/twin.jsonl")).unwrap();
    let metrics = &session.metrics;
    assert_eq!(metrics.tokens_input, 200);
    assert_eq!(metrics.tokens_output, 40);
    // Two genuine calls became two reconcile facts...
    assert_eq!(metrics.copilot_otel_calls.len(), 2);
    for call in &metrics.copilot_otel_calls {
        assert_eq!(call.conversation_id, "conv-twin");
        assert_eq!(call.model, "gpt-4.1");
    }
    // ...and the marker events were stripped before analyze: the
    // session's event count is exactly the two content + two usage
    // events the spans legitimately produced (markers never move
    // events_total).
    assert_eq!(metrics.events_total, 4);
}

#[test]
fn covered_otel_calls_are_dropped_and_disclosed_not_double_counted() {
    let otel = parse_file(&fixture("otel/twin.jsonl")).unwrap();
    let state = parse_file(&fixture("session-state/conv-twin/events.jsonl")).unwrap();
    let mut sessions = vec![otel, state];
    let adjusted = reconcile_copilot_cross_root(&mut sessions);
    // Only the OTel session moved: its two calls were covered by the
    // shutdown's cumulative snapshots (requests 1→2 reported exactly the
    // two exported calls), so the shutdown stays the witness.
    assert_eq!(adjusted, 1);
    let [otel, state] = &sessions[..] else {
        panic!("session list changed length");
    };
    let otel_metrics = &otel.metrics;
    assert_eq!(otel_metrics.tokens_input, 0);
    assert_eq!(otel_metrics.tokens_output, 0);
    assert_eq!(otel_metrics.cost_estimated, 0.0);
    assert_eq!(
        otel_metrics
            .disclosure_counters
            .get("copilot_cross_root_calls_dropped"),
        Some(&2)
    );
    // The consumed calls left the fact vector (idempotency contract).
    assert!(otel_metrics.copilot_otel_calls.is_empty());
    // The session-state session keeps its own accounting — and its
    // credit floor (rm-485: 4,000,000 nanoAIU at ×1e-11 USD/nano =
    // $0.00004) is untouched.
    let state_metrics = &state.metrics;
    assert_eq!(state_metrics.tokens_input, 200);
    assert_eq!(state_metrics.tokens_output, 40);
    assert!((state_metrics.credit_usd - 0.00004).abs() < 1e-12);
    assert!(!state_metrics
        .disclosure_counters
        .contains_key("copilot_cross_root_shutdowns_replaced"));
    // The two snapshots became two interval facts (the covered case
    // consumes nothing on the state side).
    assert_eq!(state_metrics.copilot_shutdown_intervals.len(), 2);
    // Marker stripping on the state side: session.start + user +
    // assistant + one per-model snapshot meta + one credit meta.
    assert_eq!(state_metrics.events_total, 5);
    // A second pass over the consumed world adjusts nothing further.
    let mut again = sessions;
    assert_eq!(reconcile_copilot_cross_root(&mut again), 0);
    assert_eq!(again[0].metrics.tokens_input, 0);
    assert_eq!(again[1].metrics.tokens_input, 200);
}

#[test]
fn replacement_fires_when_the_export_beats_the_stale_snapshot() {
    let otel = parse_file(&fixture("otel/killed.jsonl")).unwrap();
    let state = parse_file(&fixture("session-state/conv-killed/events.jsonl")).unwrap();
    // Pre-reconcile: arm 1 already dropped the re-export (4 spans, one
    // repeat), so the export counts its three genuine calls.
    assert_eq!(otel.metrics.tokens_input, 300);
    assert_eq!(otel.metrics.copilot_otel_calls.len(), 3);
    assert_eq!(state.metrics.tokens_input, 100);
    let mut sessions = vec![otel, state];
    let adjusted = reconcile_copilot_cross_root(&mut sessions);
    // The OTel session keeps every call (the export is the witness);
    // the session-state session's stale snapshot is replaced.
    assert_eq!(adjusted, 1);
    let [otel, state] = &sessions[..] else {
        panic!("session list changed length");
    };
    assert_eq!(otel.metrics.tokens_input, 300);
    assert_eq!(otel.metrics.tokens_output, 60);
    assert_eq!(otel.metrics.copilot_otel_calls.len(), 3);
    assert!(!otel
        .metrics
        .disclosure_counters
        .contains_key("copilot_cross_root_calls_dropped"));
    let state_metrics = &state.metrics;
    assert_eq!(state_metrics.tokens_input, 0);
    assert_eq!(state_metrics.tokens_output, 0);
    // No credit floor on this journal (no shutdown/totalNanoAiu), so
    // the replaced snapshot's whole cost went with it.
    assert_eq!(state_metrics.cost_estimated, 0.0);
    assert_eq!(
        state_metrics
            .disclosure_counters
            .get("copilot_cross_root_shutdowns_replaced"),
        Some(&1)
    );
    // Idempotent: the consumed interval never replaces twice.
    let mut again = sessions;
    assert_eq!(reconcile_copilot_cross_root(&mut again), 0);
    assert_eq!(again[0].metrics.tokens_input, 300);
    assert_eq!(again[1].metrics.tokens_input, 0);
}

#[test]
fn unpaired_exports_keep_their_accounting() {
    let session = parse_file(&fixture("otel/unpaired.jsonl")).unwrap();
    let unpaired_input = session.metrics.tokens_input;
    assert_eq!(unpaired_input, 70);
    let mut sessions = vec![session];
    assert_eq!(reconcile_copilot_cross_root(&mut sessions), 0);
    assert_eq!(sessions[0].metrics.tokens_input, 70);
    assert_eq!(sessions[0].metrics.tokens_output, 7);
    // A chat span with no conversation id produces no fact at all.
    assert!(sessions[0].metrics.copilot_otel_calls.is_empty());
}

#[test]
fn all_pairs_reconcile_in_one_pass() {
    // The discovery seam calls the pass ONCE over the whole assembled
    // list; both pairs (and the unpaired export) must resolve in that
    // single call, order-independent.
    let mut sessions = vec![
        parse_file(&fixture("otel/unpaired.jsonl")).unwrap(),
        parse_file(&fixture("session-state/conv-killed/events.jsonl")).unwrap(),
        parse_file(&otel_twin()).unwrap(),
        parse_file(&fixture("session-state/conv-twin/events.jsonl")).unwrap(),
        parse_file(&fixture("otel/killed.jsonl")).unwrap(),
    ];
    let adjusted = reconcile_copilot_cross_root(&mut sessions);
    // otel/twin (drops) + session-state/conv-killed (replacement).
    assert_eq!(adjusted, 2);
    let by_path = |sessions: &[agenttrace_core::Session], needle: &str| {
        sessions
            .iter()
            .position(|session| session.path.contains(needle))
            .unwrap()
    };
    let twin = by_path(&sessions, "twin.jsonl");
    let killed_state = by_path(&sessions, "conv-killed");
    let twin_state = by_path(&sessions, "conv-twin");
    assert_eq!(sessions[twin].metrics.tokens_input, 0);
    assert_eq!(sessions[twin_state].metrics.tokens_input, 200);
    assert_eq!(sessions[killed_state].metrics.tokens_input, 0);
    let fleet_input: i64 = sessions
        .iter()
        .map(|session| session.metrics.tokens_input)
        .sum();
    let fleet_output: i64 = sessions
        .iter()
        .map(|session| session.metrics.tokens_output)
        .sum();
    // Fleet truth: conv-twin's two calls (200/40, counted once by the
    // shutdown) + conv-killed's three calls (300/60, counted by the
    // export) + the unpaired 70/7.
    assert_eq!(fleet_input, 570);
    assert_eq!(fleet_output, 107);
}

#[test]
fn replacement_delta_lands_on_the_session_owning_the_final_snapshot() {
    // Review 23701305 F3 regression: a conversation whose events span
    // two session-state files. `events.jsonl` (index 1) carries the
    // FINAL snapshot (10:00:45, cumulative 200/40, count 2);
    // `events-2.jsonl` (index 2, the HIGHER session index) holds only
    // an earlier journal fragment (09:59:50, 100/20, count 1). The
    // export's calls at 10:00:40/:41 overflow the (09:59:50, 10:00:45]
    // window (2 exported vs 1 reported), so replacement fires with
    // boundary 09:59:50 and the delta (200/40 − 100/20 = 100/20) must
    // come off the FINAL snapshot's owner — the session whose fold
    // surfaced that cumulative total — not off the max-indexed state
    // session (pre-F3 the early fragment was the one zeroed).
    let otel = parse_file(&fixture("otel/split.jsonl")).unwrap();
    let state_final_owner = parse_file(&fixture("session-state/conv-split/events.jsonl")).unwrap();
    let state_early = parse_file(&fixture("session-state/conv-split/events-2.jsonl")).unwrap();
    assert_eq!(otel.metrics.copilot_otel_calls.len(), 3);
    assert_eq!(state_final_owner.metrics.tokens_input, 200);
    assert_eq!(state_early.metrics.tokens_input, 100);
    let mut sessions = vec![otel, state_final_owner, state_early];
    let adjusted = reconcile_copilot_cross_root(&mut sessions);
    // Exactly one session's accounting moved: the final owner's.
    assert_eq!(adjusted, 1);
    let [otel, final_owner, early] = &sessions[..] else {
        panic!("session list changed length");
    };
    // The export is the witness from the boundary on: every call
    // stays, nothing is dropped as covered.
    assert_eq!(otel.metrics.tokens_input, 300);
    assert_eq!(otel.metrics.copilot_otel_calls.len(), 3);
    assert!(!otel
        .metrics
        .disclosure_counters
        .contains_key("copilot_cross_root_calls_dropped"));
    // The final owner reports the boundary truth and carries the
    // replacement disclosure for its one replaced snapshot.
    assert_eq!(final_owner.metrics.tokens_input, 100);
    assert_eq!(final_owner.metrics.tokens_output, 20);
    assert_eq!(
        final_owner
            .metrics
            .disclosure_counters
            .get("copilot_cross_root_shutdowns_replaced"),
        Some(&1)
    );
    // The early fragment is untouched: same accounting, and NO
    // replacement disclosure ever lands on it.
    assert_eq!(early.metrics.tokens_input, 100);
    assert_eq!(early.metrics.tokens_output, 20);
    assert!(!early
        .metrics
        .disclosure_counters
        .contains_key("copilot_cross_root_shutdowns_replaced"));
    // Idempotent: the replaced interval was consumed, so a second
    // pass finds a history ending at the boundary and adjusts
    // nothing.
    let mut again = sessions;
    assert_eq!(reconcile_copilot_cross_root(&mut again), 0);
    assert_eq!(again[1].metrics.tokens_input, 100);
    assert_eq!(again[2].metrics.tokens_input, 100);
}

#[test]
fn timestampless_reexport_never_evicts_the_timestamped_export() {
    // Review 23701305 F4 regression: the first span of the re-export
    // pair lacks startTimeUnixNano. Counting is identical either way
    // (the usage key matches), but the KEPT span decides the call
    // fact's window position — the timestamped export must survive,
    // so the fact carries the real timestamp instead of 0 (which
    // would misplace every arm-2 window comparison).
    let session = parse_file(&fixture("otel/timestampless.jsonl")).unwrap();
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 20);
    let calls = &session.metrics.copilot_otel_calls;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].conversation_id, "conv-tsless");
    assert_eq!(calls[0].timestamp_ns, 1_791_540_004_000_000_000);
    // No state root pairs with this conversation: reconcile is a
    // no-op and the accounting stands.
    let mut sessions = vec![session];
    assert_eq!(reconcile_copilot_cross_root(&mut sessions), 0);
    assert_eq!(sessions[0].metrics.tokens_input, 100);
}

fn otel_twin() -> PathBuf {
    fixture("otel/twin.jsonl")
}

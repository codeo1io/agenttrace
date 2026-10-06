//! rm-551: copilot usage reconciliation is PER MODEL, not per snapshot.
//!
//! These fixtures are the assess (da490392, 2026-10-06) PoC corpora,
//! byte-for-byte. Before rm-551 a `session.usage_checkpoint` REPLACED the
//! whole prior snapshot and a `session.shutdown` naming a subset of the
//! checkpointed models dropped the omitted models wholesale — both shapes
//! below silently lost whole models' cumulative tokens (rotation dropped
//! gpt-4.1's 1,000,000/100,000; the partial shutdown dropped o4-mini's
//! 400,000/40,000), violating rm-485's own landed acceptance. The fix keeps
//! a per-model freshest map (ccusage #1824's subtract_usage discipline):
//! a checkpoint/shutdown replaces only the entries it names.
//!
//! The third fixture pins the documented fallback for truncated journals:
//! copilot detection keys off `session.start`, so a journal whose
//! `session.start` line is missing parses as a generic session with no
//! tokens. That behavior is recorded here (not silently changed) — see the
//! rm-551 roadmap row for the accepted disposition.

use agenttrace_core::parse_file;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/copilot-checkpoints")
        .join(name)
}

#[test]
fn model_rotation_preserves_earlier_checkpoint_models() {
    // Two checkpoints name DIFFERENT models: gpt-4.1 1,000,000/100,000 then
    // claude-sonnet-4 50,000/5,000. The snapshot-level fold kept only the
    // last checkpoint's models; the per-model fold must keep both.
    let session = parse_file(&fixture("model-rotation.jsonl")).unwrap();
    let m = &session.metrics;
    assert_eq!(m.tokens_input, 1_050_000, "rotation must sum both models");
    assert_eq!(m.tokens_output, 105_000);
    // lib.rs labels a multi-model session "multiple" and prices each meta
    // block at its own model (usage_blocks_multi_model) — the honest
    // attribution arm that only exists when both models survive the fold.
    assert_eq!(m.model_used, "multiple");
    assert_eq!(m.source_tool, "copilot_cli");
    // rm-485 credit max-semantics: 2,000,000,000 nanoAIU * $0.01/1e9.
    assert!(
        (m.credit_usd - 0.02).abs() < 1e-9,
        "credit max-semantics must survive the per-model fold, got {}",
        m.credit_usd
    );
}

#[test]
fn partial_shutdown_preserves_omitted_checkpoint_models() {
    // Checkpoint names gpt-4.1 800,000/80,000 + o4-mini 400,000/40,000;
    // the shutdown names only gpt-4.1 (identical values) + a larger
    // totalNanoAiu. The shutdown is the freshest word on gpt-4.1 only —
    // o4-mini's checkpoint values must survive, and the credit still
    // folds with max semantics (950,000,000 nanoAIU).
    let session = parse_file(&fixture("partial-shutdown.jsonl")).unwrap();
    let m = &session.metrics;
    assert_eq!(m.tokens_input, 1_200_000, "omitted model must survive");
    assert_eq!(m.tokens_output, 120_000);
    assert_eq!(m.model_used, "multiple");
    assert_eq!(m.source_tool, "copilot_cli");
    assert!(
        (m.credit_usd - 0.0095).abs() < 1e-9,
        "credit max-semantics must survive the partial shutdown, got {}",
        m.credit_usd
    );
}

#[test]
fn shutdown_replaces_the_models_it_names_with_fresher_values() {
    // A shutdown carrying a NEWER cumulative value for a checkpointed model
    // must replace that model's entry (not sum with it — counters are
    // cumulative), while credit still takes the max across events.
    let dir = std::env::temp_dir().join("rm551-fresher-shutdown.jsonl");
    std::fs::write(
        &dir,
        concat!(
            "{\"type\":\"session.start\",\"timestamp\":\"2026-10-05T13:00:00Z\",\"data\":{\"context\":{\"cwd\":\"/tmp/z\"}}}\n",
            "{\"type\":\"session.usage_checkpoint\",\"timestamp\":\"2026-10-05T13:05:00Z\",\"data\":{\"modelMetrics\":{\"gpt-4.1\":{\"usage\":{\"inputTokens\":100000,\"outputTokens\":10000}}},\"totalNanoAiu\":100000000}}\n",
            "{\"type\":\"session.shutdown\",\"timestamp\":\"2026-10-05T13:09:00Z\",\"data\":{\"modelMetrics\":{\"gpt-4.1\":{\"usage\":{\"inputTokens\":250000,\"outputTokens\":25000}}},\"totalNanoAiu\":150000000}}\n"
        ),
    )
    .unwrap();
    let session = parse_file(&dir).unwrap();
    let m = &session.metrics;
    assert_eq!(
        m.tokens_input, 250_000,
        "shutdown value must REPLACE, not sum"
    );
    assert_eq!(m.tokens_output, 25_000);
    assert_eq!(m.model_used, "gpt-4.1");
    assert!(
        (m.credit_usd - 0.0015).abs() < 1e-9,
        "credit max across events, got {}",
        m.credit_usd
    );
}

#[test]
fn no_session_start_parses_as_generic_documented_fallback() {
    // Documented fallback (rm-551 disposition): copilot detection keys off
    // session.start, so a truncated journal that still carries a checkpoint
    // is NOT attributed to copilot_cli and reports no usage. Pinned so the
    // next drift on this lane is loud, not silent.
    let session = parse_file(&fixture("no-session-start.jsonl")).unwrap();
    let m = &session.metrics;
    assert_ne!(
        m.source_tool, "copilot_cli",
        "truncated journal must not claim copilot attribution"
    );
    assert_eq!(m.tokens_input, 0);
    assert_eq!(m.tokens_output, 0);
}

#[test]
fn agent_host_turn_markers_are_named_not_dropped() {
    // rm-721 / codeburn #1651: VS Code runs Copilot Chat's agent through
    // the Copilot CLI engine (workspace.yaml client_name:
    // vscode-agent-host); CLI 1.0.8x writes the same shape. Those journals
    // carry assistant.turn_start/turn_end (ids only — an un-counted class
    // under the rm-485 counting rules) and assistant.message entries with
    // NO outputTokens (each message is one model request; tokens arrive
    // only in the shutdown rollup). The audit contract: every request
    // class counts exactly once, un-counted classes are NAMED (the
    // rm-584 disclosure rule), and the rollup's per-model
    // modelMetrics[model].totalNanoAiu — the agent-host shape, no
    // top-level totalNanoAiu — yields exact Copilot credits.
    let session = parse_file(&fixture("agent-host-events.jsonl")).unwrap();
    let m = &session.metrics;
    assert_eq!(m.source_tool, "copilot_cli");
    assert_eq!(m.user_messages, 1, "user turn counted exactly once");
    assert_eq!(
        m.assistant_turns, 2,
        "assistant.message + tool.execution_start each counted exactly once"
    );
    assert_eq!(m.tool_calls_total, 1, "tool-call turn counted exactly once");
    assert_eq!(m.tool_results, 1);
    assert_eq!(
        m.tokens_input, 120,
        "shutdown rollup is the only token record (agent-host basis)"
    );
    assert_eq!(m.tokens_output, 45);
    assert_eq!(m.model_used, "gpt-5-mini");
    assert!(
        (m.credit_usd - 0.0025).abs() < 1e-9,
        "per-model modelMetrics totalNanoAiu (250e6 nano AIU) must price \
         credits at 1 AIU = $0.01, got {}",
        m.credit_usd
    );
    assert_eq!(
        m.disclosure_counters
            .get("copilot_uncounted_entry_type:assistant.turn_start"),
        Some(&1),
        "the un-counted turn-marker class must be named, not dropped"
    );
    assert_eq!(
        m.disclosure_counters
            .get("copilot_uncounted_entry_type:assistant.turn_end"),
        Some(&1)
    );
}

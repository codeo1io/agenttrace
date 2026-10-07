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
    //
    // Review 3e3a2198 F3/F4 widened the fixture to the full observed
    // class vocabulary (codeburn #1651): the tool-call turn stays, the
    // IDE-side command classes (hook.start/hook.end on
    // userPromptSubmitted, permission.requested/completed) and the
    // sub-agent (sub-issue) lifecycle marker subagent.deselected are
    // each NAMED un-counted classes named exactly once, and the
    // shutdown rollup carries TWO models so the per-model credit meters
    // SUM at emit (250e6 + 100e6 nano AIU) instead of a global max
    // silently dropping the second model's bill.
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
        m.tokens_input, 150,
        "shutdown rollup is the only token record (agent-host basis): \
         120 gpt-5-mini + 30 claude-haiku-4.5"
    );
    assert_eq!(m.tokens_output, 60, "45 + 15 across the two rollup models");
    assert_eq!(m.model_used, "multiple", "two models in the rollup");
    assert!(
        (m.credit_usd - 0.0035).abs() < 1e-9,
        "per-model modelMetrics totalNanoAiu meters SUM at emit \
         (250e6 + 100e6 nano AIU, review 3e3a2198 F4) and price credits \
         at 1 AIU = $0.01, got {}",
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
    // F3: the IDE-command classes (userPromptSubmitted hooks + the
    // permission gate) and the sub-agent lifecycle marker are each
    // named EXACTLY ONCE — no class is silently swallowed.
    for class in [
        "hook.start",
        "hook.end",
        "permission.requested",
        "permission.completed",
        "subagent.deselected",
    ] {
        assert_eq!(
            m.disclosure_counters
                .get(&format!("copilot_uncounted_entry_type:{class}")),
            Some(&1),
            "IDE-command / sub-agent class must be NAMED, not dropped: {class}"
        );
    }
}

// Review 3e3a2198 F1 (fix attempt 653b57af, adopting reaped 8a3231c6's
// schema bump): the warm-cache half of the agent-lane contract, in the
// stale_schema_NN_cannot_mask_the_disclosure family (zero_usage_contract
// holds the schema-22 ancestor). The rm-720/rm-721 batch corrects tokens
// and credits for UNCHANGED files, and cached entries carry the full
// derived metrics (GoMetrics::from_metrics — tokens, credit_usd,
// disclosure_counters), so a cache written by the pre-batch build
// (schema 32) keeps serving the pre-batch totals — token estimates, no
// credits — with matching fingerprints and never re-parses (proven live
// by the review's B leg). The 32 -> 33 bump retires those entries once
// (rm-230 convention). This pins the masking PoC as a regression: on a
// tree where the const is still 32 the hand-written v32 entry below IS
// served and both assertions fail.
#[test]
fn stale_schema_32_cache_cannot_mask_the_agent_lane_rollup() {
    use std::os::unix::fs::MetadataExt;

    let root = std::env::temp_dir().join(format!(
        "at-agent-host-stale-v32-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let sessions_dir = root.join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    let session_path = sessions_dir.join("agent-host.jsonl");
    std::fs::copy(fixture("agent-host-events.jsonl"), &session_path)
        .expect("copy agent-host fixture");

    // Hand-write the pre-batch shape: schema 32, entry fingerprint FRESH
    // against the file (matching mod_time/size), cached session carrying
    // the pre-batch metrics for this exact journal — content-based token
    // ESTIMATE, model "default", NO credit_usd.
    let metadata = std::fs::metadata(&session_path).expect("stat fixture");
    let mod_time = metadata.mtime() * 1_000_000_000 + metadata.mtime_nsec();
    let session_path_str = session_path.to_string_lossy().to_string();
    let mut entries = serde_json::Map::new();
    entries.insert(
        session_path_str.clone(),
        serde_json::json!({
            "mod_time": mod_time,
            "size": metadata.len() as i64,
            "session": {
                "Name": "stale",
                "Path": session_path_str,
                "Metrics": {
                    "SourceTool": "copilot_cli",
                    "ModelUsed": "cached-default",
                    "SessionStart": "2026-10-05T14:00:00Z",
                    "ToolArgUsage": {},
                },
                "Health": 91,
                "ToolWarnings": [],
                "Diagnostics": {},
            },
        }),
    );
    let cache = serde_json::json!({
        "schema_version": 32,
        "entries": serde_json::Value::Object(entries),
    });
    std::fs::write(root.join("sessions.json"), cache.to_string()).expect("write stale v32 cache");

    let key = "AGENTTRACE_SESSION_CACHE_DIR";
    let previous = std::env::var_os(key);
    std::env::set_var(key, &root);
    let sessions = agenttrace_core::load_sessions_from_dir(Some(&sessions_dir));
    match previous {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }

    assert_eq!(sessions.len(), 1, "one session discovered");
    assert_ne!(
        sessions[0].name, "stale",
        "the fresh-fingerprint v32 entry must NOT be served: its cached \
         estimate basis and absent credit are exactly the pre-batch \
         under-report the schema bump exists to retire"
    );
    assert_eq!(
        sessions[0].metrics.tokens_input, 150,
        "re-parsed from source: the two-model shutdown rollup, not the \
         cached content estimate"
    );
    assert!(
        (sessions[0].metrics.credit_usd - 0.0035).abs() < 1e-9,
        "re-parsed from source: summed per-model credit meters, got {}",
        sessions[0].metrics.credit_usd
    );
    let _ = std::fs::remove_dir_all(&root);
}

//! Cycle-1 batch `journal truth: contain hostile input, surface hidden
//! wire` — unit pins for the three code rows:
//!
//! * rm-778: journal-derived cwd is capped at parse time (and once more
//!   at the `session_from_events` choke point), with a disclosure
//!   counter on every truncation.
//! * rm-880: codex 0.160.1 `session_meta` identity/lineage/quota wire
//!   and the `session_configured` event surface as
//!   `Metrics.wire_metadata` instead of being dropped unread.
//! * rm-406 (dated 2026-10-07 arm): Claude Code advisor-model usage
//!   riding `message.usage.iterations[]` lands on the advisor's own
//!   model — session totals include it and the overview `by_model`
//!   split attributes it — instead of vanishing under top-level usage.
//!
//! End-to-end corpus pins for the journal-truth batch: rm-778 cwd
//! containment, rm-880 codex 0.160.1 wire, rm-406 advisor attribution
//! (dated 2026-10-07 arm) and the session-cache 33 round-trip. Corpus
//! fixtures live under `tests/fixtures/journal-truth/` and
//! `tests/fixtures/usage-accounting/`; the earlier render/mint
//! sanitization pins live in `hostile_journal_disclosure.rs` — these
//! are the shape pins that fail close to the code they guard.

use std::collections::BTreeMap;

use agenttrace_core::{parse_raw_session, session_from_events, Event, Session};

fn parse_one(line: &str) -> Session {
    parse_raw_session("t.jsonl", "t.jsonl", line).unwrap()
}

// ---------------------------------------------------------------- rm-778

#[test]
fn claude_lane_caps_hostile_cwd_at_parse_time() {
    // The assess F1 shape: a claude-shape journal whose TOP-LEVEL cwd
    // (sibling of `type`, the real Claude Code layout) is hundreds of
    // thousands of components. The parsed session must keep the 4096-
    // byte budget and disclose the truncation.
    let hostile = format!("/a/{}", "a/".repeat(300_000));
    let line = serde_json::json!({
        "type": "user",
        "cwd": hostile,
        "sessionId": "sess-1",
        "message": { "role": "user", "content": "hello" },
    })
    .to_string();
    let session = parse_one(&line);
    assert!(session.cwd.len() <= 4096);
    let counters: BTreeMap<String, usize> = session.metrics.disclosure_counters.clone();
    let key = counters
        .keys()
        .find(|key| key.starts_with("cwd_truncated:"))
        .expect("cwd truncation disclosed");
    assert_eq!(key, "cwd_truncated:claude_code");
    assert_eq!(counters[key], 1);
}

#[test]
fn codex_lane_caps_hostile_cwd_at_parse_time() {
    let hostile = format!("/c/{}", "b/".repeat(200_000));
    let meta = serde_json::json!({
        "timestamp": "2026-10-07T00:00:00Z",
        "type": "session_meta",
        "payload": { "cwd": hostile, "originator": "codex_cli_rs" },
    })
    .to_string();
    let turn = serde_json::json!({
        "timestamp": "2026-10-07T00:00:01Z",
        "type": "response_item",
        "payload": { "type": "message", "role": "user", "content": [{"type": "input_text", "text": "hi"}] },
    })
    .to_string();
    let session = parse_one(&format!("{meta}\n{turn}\n"));
    assert!(session.cwd.len() <= 4096);
    assert!(session
        .metrics
        .disclosure_counters
        .keys()
        .any(|key| key == "cwd_truncated:codex_cli"));
}

#[test]
fn short_cwds_pass_through_uncapped_and_undisclosed() {
    let claude = serde_json::json!({
        "type": "user",
        "cwd": "/work/projects/agenttrace",
        "sessionId": "sess-1",
        "message": { "role": "user", "content": "hello" },
    })
    .to_string();
    let session = parse_one(&claude);
    assert_eq!(session.cwd, "/work/projects/agenttrace");
    assert!(session
        .metrics
        .disclosure_counters
        .keys()
        .all(|key| !key.starts_with("cwd_truncated:")));
}

#[test]
fn generic_lane_capped_at_the_choke_point() {
    // The Vec<Event> lane (cache replays, deserialized journals) never
    // touches the five capped read sites — the choke point inside
    // session_from_events must apply the same cap.
    let hostile = format!("/g/{}", "x/".repeat(300_000));
    let session = session_from_events(
        "generic",
        "generic",
        vec![Event {
            role: "user".to_string(),
            cwd: hostile,
            source_tool: "hermes_jsonl".to_string(),
            ..Event::default()
        }],
    )
    .unwrap();
    assert!(session.cwd.len() <= 4096);
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("cwd_truncated:hermes_jsonl"),
        Some(&1)
    );
}

// ---------------------------------------------------------------- rm-880

#[test]
fn codex_session_meta_wire_metadata_surfaces_identity_and_lineage() {
    // codex 0.160.1 shape (research ee3cd7c2): session_meta carries
    // creator identity, fork lineage, parent thread, and history base
    // that the fork of record at this tree read and dropped.
    let line = serde_json::json!({
        "timestamp": "2026-10-07T10:00:00Z",
        "type": "session_meta",
        "payload": {
            "cwd": "/work/proj",
            "originator": "codex_cli_rs",
            "cli_version": "0.160.1",
            "creator_user_id": "user-AAAA",
            "creator_account_id": "acct-BBBB",
            "forked_from_id": "0a11ce00-0000-7000-8000-000000000001",
            "forked_from_ordinal_exclusive": 42,
            "parent_thread_id": "0a11ce00-0000-7000-8000-000000000001",
            "history_base": { "thread_id": "0a11ce00-0000-7000-8000-000000000001", "end_ordinal_exclusive": 7 }
        }
    })
    .to_string();
    let session_meta = line;
    let turn = serde_json::json!({
        "timestamp": "2026-10-07T10:00:02Z",
        "type": "response_item",
        "payload": { "type": "message", "role": "user", "content": [{"type": "input_text", "text": "hi"}] },
    })
    .to_string();
    let session = parse_one(&format!("{session_meta}\n{turn}\n"));
    let wire = &session.metrics.wire_metadata;
    assert_eq!(
        wire.get("codex_creator_user_id").map(String::as_str),
        Some("user-AAAA")
    );
    assert_eq!(
        wire.get("codex_creator_account_id").map(String::as_str),
        Some("acct-BBBB")
    );
    assert_eq!(
        wire.get("codex_forked_from_id").map(String::as_str),
        Some("0a11ce00-0000-7000-8000-000000000001")
    );
    assert_eq!(
        wire.get("codex_forked_from_ordinal_exclusive")
            .map(String::as_str),
        Some("42")
    );
    assert_eq!(
        wire.get("codex_parent_thread_id").map(String::as_str),
        Some("0a11ce00-0000-7000-8000-000000000001")
    );
    assert_eq!(
        wire.get("codex_history_base_thread_id").map(String::as_str),
        Some("0a11ce00-0000-7000-8000-000000000001")
    );
    assert_eq!(
        wire.get("codex_history_base_end_ordinal_exclusive")
            .map(String::as_str),
        Some("7")
    );
    // Lineage is disclosed as unfollowed: agenttrace analyzes each
    // rollout in isolation, so a forked session's parent spend is NOT
    // in this session's totals.
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("codex_fork_lineage_unfollowed"),
        Some(&1)
    );
}

#[test]
fn codex_session_configured_event_surfaces_instead_of_vanishing() {
    // The NEW persisted event line in codex 0.160.1 — previously
    // swallowed by the fast-path line skipper.
    let session_meta = serde_json::json!({
        "timestamp": "2026-10-07T10:00:00Z",
        "type": "session_meta",
        "payload": { "cwd": "/work/proj", "originator": "codex_cli_rs" }
    })
    .to_string();
    let session_configured = serde_json::json!({
        "timestamp": "2026-10-07T10:00:01Z",
        "type": "event_msg",
        "payload": {
            "type": "session_configured",
            "thread_name": "fix-the-flaky-test",
            "model_provider_id": "openai",
            "service_tier": "priority"
        }
    })
    .to_string();
    let session = parse_one(&format!("{session_meta}\n{session_configured}\n"));
    let wire = &session.metrics.wire_metadata;
    assert_eq!(
        wire.get("codex_thread_name").map(String::as_str),
        Some("fix-the-flaky-test")
    );
    assert_eq!(
        wire.get("codex_model_provider_id").map(String::as_str),
        Some("openai")
    );
    assert_eq!(
        wire.get("codex_service_tier").map(String::as_str),
        Some("priority")
    );
}

#[test]
fn codex_rate_limits_quota_snapshot_disclosed_not_dropped() {
    // The quota wire (limit id/name, plan_type) rides token_count
    // lines; plan_type would let a report explain WHY throughput
    // saturated — surfacing it is the honest minimum.
    let session_meta = serde_json::json!({
        "timestamp": "2026-10-07T10:00:00Z",
        "type": "session_meta",
        "payload": { "cwd": "/work/proj", "originator": "codex_cli_rs" }
    })
    .to_string();
    let token_count = serde_json::json!({
        "timestamp": "2026-10-07T10:00:05Z",
        "type": "event_msg",
        "payload": {
            "type": "token_count",
            "info": {
                "total_token_usage": { "input_tokens": 100, "cached_input_tokens": 0, "output_tokens": 50, "reasoning_output_tokens": 0 },
                "last_token_usage": { "input_tokens": 100, "cached_input_tokens": 0, "output_tokens": 50, "reasoning_output_tokens": 0 }
            },
            "rate_limits": {
                "limit_id": "primary",
                "limit_name": "5h",
                "plan_type": "pro"
            }
        }
    })
    .to_string();
    let session = parse_one(&format!("{session_meta}\n{token_count}\n"));
    let wire = &session.metrics.wire_metadata;
    assert_eq!(wire.get("codex_plan_type").map(String::as_str), Some("pro"));
    assert_eq!(
        wire.get("codex_rate_limit_id").map(String::as_str),
        Some("primary")
    );
    assert_eq!(
        wire.get("codex_rate_limit_name").map(String::as_str),
        Some("5h")
    );
    // Usage still accounted from the same line (no double count):
    assert_eq!(
        session.metrics.tokens_input, 100,
        "input tokens from token_count.info still land in metrics"
    );
    assert_eq!(session.metrics.tokens_output, 50);
}

// ---------------------------------------------------------------- rm-406

#[test]
fn advisor_iterations_usage_lands_on_its_own_model() {
    // tokscale #1386 shape: the top-level usage covers the MAIN model
    // only; the advisor's turns ride usage.iterations[] with
    // type=advisor_message and their own model. Both models' spend must
    // survive.
    let line = serde_json::json!({
        "type": "assistant",
        "message": {
            "id": "msg_01",
            "model": "claude-opus-5-5",
            "usage": {
                "input_tokens": 227337,
                "output_tokens": 100,
                "iterations": [
                    { "type": "message", "model": "claude-opus-5-5", "input_tokens": 227337, "output_tokens": 100 },
                    { "type": "advisor_message", "model": "claude-fable-5-1", "input_tokens": 5000, "output_tokens": 300 }
                ]
            }
        }
    })
    .to_string();
    let session = parse_one(&line);
    // Session totals include the advisor spend, once:
    assert_eq!(session.metrics.tokens_input, 227337 + 5000);
    assert_eq!(session.metrics.tokens_output, 100 + 300);
    // And the attribution keeps the split:
    let attribution = &session.metrics.model_attribution;
    assert!(attribution.contains_key("claude-fable-5-1"));
    assert!(attribution.contains_key("claude-opus-5-5"));
    assert_eq!(attribution["claude-fable-5-1"].input_tokens, 5000);
    assert_eq!(attribution["claude-fable-5-1"].output_tokens, 300);
    assert_eq!(attribution["claude-opus-5-5"].input_tokens, 227337);
}

#[test]
fn overview_by_model_attributes_advisor_spend() {
    let line = serde_json::json!({
        "type": "assistant",
        "message": {
            "id": "msg_01",
            "model": "claude-opus-5-5",
            "usage": {
                "input_tokens": 1000,
                "output_tokens": 100,
                "iterations": [
                    { "type": "message", "model": "claude-opus-5-5", "input_tokens": 1000, "output_tokens": 100 },
                    { "type": "advisor_message", "model": "claude-fable-5-1", "input_tokens": 5000, "output_tokens": 300 }
                ]
            }
        }
    })
    .to_string();
    let session = parse_one(&line);
    let overview = agenttrace_core::compute_overview(&[session]);
    // The advisor's model is a by_model bucket in its own right — its
    // spend no longer vanishes under the session headline.
    assert!(overview.by_model.contains_key("claude-fable-5-1"));
    assert!(overview.by_model.contains_key("claude-opus-5-5"));
    // Review fix F1: the by_model cost column PARTITIONS spend. The
    // headline "multiple" bucket keeps the session count but zero
    // cost, and the attributed buckets hold exactly the whole session
    // cost — sum(by_model[].cost) == overview.total_cost.
    let multiple = &overview.by_model["multiple"];
    assert_eq!(multiple.sessions, 1);
    assert_eq!(multiple.cost, 0.0);
    let summed: f64 = overview.by_model.values().map(|group| group.cost).sum();
    assert_eq!(
        agenttrace_core::round4(summed),
        overview.total_cost,
        "by_model cost column must partition the true total at the pricing precision"
    );
}

#[test]
fn single_model_sessions_keep_their_exact_shape() {
    // No iterations, one model: model_attribution stays empty and the
    // serialized metrics are byte-identical to the pre-rm-406 shape.
    let line = serde_json::json!({
        "type": "assistant",
        "message": {
            "id": "msg_01",
            "model": "claude-opus-5-5",
            "usage": { "input_tokens": 10, "output_tokens": 5 }
        }
    })
    .to_string();
    let session = parse_one(&line);
    assert!(session.metrics.model_attribution.is_empty());
    assert!(session.metrics.wire_metadata.is_empty());
    let serialized = serde_json::to_string(&session.metrics).unwrap();
    assert!(!serialized.contains("model_attribution"));
    assert!(!serialized.contains("wire_metadata"));
}

// ------------------------------------------------- durable repo fixtures
// The PoC corpora were /tmp-resident; the stewardship contract pins
// them into the repo so the batch's red shapes survive fleet sweeps.

fn fixture(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

#[test]
fn fixture_claude_deepcwd_corpus_is_capped_and_disclosed() {
    // journal-truth/claude-deepcwd: the assess F1 hostile shape at
    // fixture scale (a >4096-byte component path on a claude journal).
    let raw = fixture("journal-truth/claude-deepcwd/deep-cwd.jsonl");
    let session = parse_one(&raw);
    assert!(session.cwd.len() <= 4096, "cwd not capped");
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("cwd_truncated:claude_code"),
        Some(&1)
    );
}

#[test]
fn fixture_codex_0160_fork_pair_surfaces_wire_on_both_sides() {
    // journal-truth/codex-0160-forks: the codex 0.160.1 parent and its
    // forked child. Parent carries creator identity + quota wire; the
    // child carries fork lineage and the unfollowed disclosure.
    let parent = parse_one(&fixture("journal-truth/codex-0160-forks/parent.jsonl"));
    let child = parse_one(&fixture(
        "journal-truth/codex-0160-forks/forked-child.jsonl",
    ));
    let parent_wire = &parent.metrics.wire_metadata;
    assert!(parent_wire.contains_key("codex_creator_user_id"));
    assert!(parent_wire.contains_key("codex_thread_name"));
    assert!(parent_wire.contains_key("codex_plan_type"));
    let child_wire = &child.metrics.wire_metadata;
    assert!(child_wire.contains_key("codex_forked_from_id"));
    assert_eq!(
        child
            .metrics
            .disclosure_counters
            .get("codex_fork_lineage_unfollowed"),
        Some(&1)
    );
}

#[test]
fn fixture_advisor_iterations_attribution_round_trips() {
    // usage-accounting/claude-advisor: the tokscale #1386 shape — the
    // advisor's model must own its spend.
    let session = parse_one(&fixture(
        "usage-accounting/claude-advisor/advisor-iterations.jsonl",
    ));
    let attribution = &session.metrics.model_attribution;
    assert!(
        attribution.contains_key("claude-fable-5-1"),
        "advisor model missing from attribution: {attribution:?}"
    );
    let overview = agenttrace_core::compute_overview(&[session]);
    assert!(overview.by_model.contains_key("claude-fable-5-1"));
    // Review fix F1, real-corpus arm: partition holds on the fixture
    // too — headline bucket carries no cost while the split holds all
    // of it.
    let summed: f64 = overview.by_model.values().map(|group| group.cost).sum();
    assert_eq!(
        agenttrace_core::round4(summed),
        overview.total_cost,
        "by_model cost column must partition the true total at the pricing precision"
    );
}

//! Golden reconciliation of the upstream #312 / v0.10.1 token-accounting
//! port (rm-517). Every expected number below is lifted from upstream's own
//! #312 test additions so a future semantic drift from upstream reference
//! behavior shows up as a diff here, not as a silent totals change.
//!
//! Companion arms live in `discovery_contract.rs` (per-format stream tests);
//! this file covers the accounting rules themselves: fold-vs-count, alias
//! first-number, cache-subtraction, distinct-total dedup, and the OTEL
//! chat-span gate with both cache-attribute spellings (rm-519a).

use agenttrace_core::parse_file;
use std::fs;
use std::path::PathBuf;

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn write_session(name: &str, filename: &str, body: &str) -> PathBuf {
    let root = temp_root(name);
    let path = root.join(filename);
    fs::write(&path, body).expect("write session");
    path
}

#[test]
fn token_accounting_cc_stream_folds_per_message_id_with_field_max() {
    // Upstream #312 golden: three streaming rows for msg_1 repeat input 100 /
    // cache 7/3 while output grows 10 -> 10 -> 42. One usage event: the
    // per-field max, so input counts once and output carries the final 42.
    let path = write_session(
        "agenttrace-ta-cc-fold",
        "claude.jsonl",
        concat!(
            r#"{"type":"assistant","timestamp":"2026-05-03T10:00:00Z","message":{"id":"msg_1","role":"assistant","model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":7,"cache_creation_input_tokens":3},"content":[{"type":"text","text":"hello"}]}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-05-03T10:00:01Z","message":{"id":"msg_1","role":"assistant","model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":7,"cache_creation_input_tokens":3},"content":[{"type":"tool_use","id":"tool_1","name":"Read","input":{}}]}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-05-03T10:00:02Z","message":{"id":"msg_1","role":"assistant","model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":42,"cache_read_input_tokens":7,"cache_creation_input_tokens":3},"content":[{"type":"text","text":"done"}]}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "claude_code");
    assert_eq!(parsed.metrics.tokens_input, 100);
    assert_eq!(parsed.metrics.tokens_output, 42);
    assert_eq!(parsed.metrics.tokens_cache_r, 7);
    assert_eq!(parsed.metrics.tokens_cache_w, 3);
}

#[test]
fn token_accounting_codex_counts_distinct_totals_once_and_not_reasoning() {
    // Upstream #312 golden: the same total_token_usage re-emitted (rate-limit
    // tick) counts once; a NEW total counts its last_token_usage; reasoning
    // output is a breakdown of output, not an addition (100 + 60, not +30).
    let path = write_session(
        "agenttrace-ta-codex-distinct",
        "rollout.jsonl",
        concat!(
            r#"{"timestamp":"2026-05-03T10:00:00Z","type":"response_item","payload":{"type":"function_call","name":"read","arguments":"{}"}}"#,
            "\n",
            r#"{"timestamp":"2026-05-03T10:00:01Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":20},"total_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":20}}}}"#,
            "\n",
            r#"{"timestamp":"2026-05-03T10:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":20},"total_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":20}}}}"#,
            "\n",
            r#"{"timestamp":"2026-05-03T10:00:03Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":700,"cached_input_tokens":500,"output_tokens":60,"reasoning_output_tokens":30},"total_token_usage":{"input_tokens":1700,"cached_input_tokens":900,"output_tokens":160,"reasoning_output_tokens":50}}}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "codex_cli");
    assert_eq!(parsed.metrics.tokens_input, 600 + 200);
    assert_eq!(parsed.metrics.tokens_output, 100 + 60);
    assert_eq!(parsed.metrics.tokens_cache_r, 400 + 500);
}

#[test]
fn token_accounting_copilot_shutdown_is_last_snapshot_with_cache_subtracted() {
    // Upstream #312 golden: two shutdown snapshots for the same model are
    // cumulative session totals — only the latest counts, and gen_ai input
    // includes cached tokens (100 input - 40 read = 60 uncached).
    let path = write_session(
        "agenttrace-ta-copilot-shutdown",
        "copilot.session.jsonl",
        concat!(
            r#"{"type":"session.start","timestamp":"2026-05-03T10:00:00Z","data":{"context":{"cwd":"/tmp/copilot"}}}"#,
            "\n",
            r#"{"type":"user.message","timestamp":"2026-05-03T10:00:02Z","data":{"content":"inspect"}}"#,
            "\n",
            r#"{"type":"session.shutdown","timestamp":"2026-05-03T10:00:05Z","data":{"modelMetrics":{"gpt-5.4":{"usage":{"inputTokens":40,"outputTokens":5,"cacheReadTokens":10}}}}}"#,
            "\n",
            r#"{"type":"session.shutdown","timestamp":"2026-05-03T10:00:10Z","data":{"modelMetrics":{"gpt-5.4":{"usage":{"inputTokens":100,"outputTokens":20,"cacheReadTokens":40}}}}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "copilot_cli");
    assert_eq!(parsed.metrics.tokens_input, 60);
    assert_eq!(parsed.metrics.tokens_output, 20);
    assert_eq!(parsed.metrics.tokens_cache_r, 40);
}

#[test]
fn token_accounting_kimi_wire_counts_every_status_update() {
    // Upstream #312 golden: every StatusUpdate token_usage counts (the CLI
    // reports per-record totals, not a running total) — 100+60 input,
    // 20+10 output, 40+20 cached reads (kimi wire keeps cached reads in
    // their own bucket; input_other already excludes them).
    let path = write_session(
        "agenttrace-ta-kimi-wire",
        "kimi.session.jsonl",
        concat!(
            r#"{"type":"metadata","protocol_version":"2"}"#,
            "\n",
            r#"{"timestamp":1770000000.0,"message":{"type":"TurnBegin","payload":{"user_input":"inspect"}}}"#,
            "\n",
            r#"{"timestamp":1770000001.0,"message":{"type":"StatusUpdate","payload":{"token_usage":{"inputTokens":100,"outputTokens":20,"cacheReadInputTokens":40}}}}"#,
            "\n",
            r#"{"timestamp":1770000002.0,"message":{"type":"StatusUpdate","payload":{"token_usage":{"inputTokens":60,"outputTokens":10,"cacheReadInputTokens":20}}}}"#,
            "\n",
            r#"{"timestamp":1770000003.0,"message":{"type":"FinishReason","payload":{"finish_reason":"stop"}}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "kimi_cli");
    assert_eq!(parsed.metrics.tokens_input, 160);
    assert_eq!(parsed.metrics.tokens_output, 30);
    assert_eq!(parsed.metrics.tokens_cache_r, 60);
}

#[test]
fn token_accounting_kimi_value_counts_usage_once() {
    // Upstream #312 golden: kimi API responses can repeat usage in both
    // `usage` and `metadata.usage`; only one counts (60 input, not 120).
    let path = write_session(
        "agenttrace-ta-kimi-value",
        "kimi.json",
        r#"{"model":"kimi-k2.6","usage":{"input_tokens":60,"output_tokens":15},"metadata":{"usage":{"input_tokens":60,"output_tokens":15}},"messages":[{"role":"user","content":"Inspect billing export","timestamp":"2026-05-03T10:00:00Z"},{"role":"assistant","content":"I will inspect it.","timestamp":"2026-05-03T10:00:04Z"}]}"#,
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "kimi_cli");
    assert_eq!(parsed.metrics.tokens_input, 60);
    assert_eq!(parsed.metrics.tokens_output, 15);
}

#[test]
fn token_accounting_qwen_aliases_are_first_number_and_cache_is_subtracted() {
    // Upstream #312 golden: input_tokens/prompt_tokens are aliases for the
    // same counter (first present wins, not a sum) and cached reads are
    // inside the input count — 120 - 10 = 110, not 120 and not 230.
    let path = write_session(
        "agenttrace-ta-qwen-aliases",
        "qwen.jsonl",
        concat!(
            r#"{"type":"system","subtype":"session_start","uuid":"sys-1","session_id":"s1","model":"qwen3-coder-plus","timestamp":"2026-05-03T10:00:00Z"}"#,
            "\n",
            r#"{"type":"assistant","uuid":"a1","session_id":"s1","timestamp":"2026-05-03T10:00:02Z","message":{"id":"m1","type":"message","role":"assistant","model":"qwen3-coder-plus","content":[{"type":"text","text":"hi"}],"usage":{"input_tokens":120,"prompt_tokens":120,"output_tokens":45,"cache_read_input_tokens":10,"cache_creation_input_tokens":5}}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "qwen_code");
    assert_eq!(parsed.metrics.tokens_input, 110);
    assert_eq!(parsed.metrics.tokens_output, 45);
    assert_eq!(parsed.metrics.tokens_cache_r, 10);
    assert_eq!(parsed.metrics.tokens_cache_w, 5);
}

#[test]
fn token_accounting_workbuddy_counts_every_distinct_turn_usage() {
    // Upstream #311 golden: every record's usage counts once per (messageId,
    // usage) — the function_call and the assistant message are separate turns
    // (100-60 + 120-80 = 80 input, 20+30 output, 60+80 cached reads).
    let path = write_session(
        "agenttrace-ta-workbuddy",
        "workbuddy.jsonl",
        concat!(
            r#"{"type":"function_call","sessionId":"s1","cwd":"/repo","callId":"call_1","name":"read","arguments":"{}","timestamp":1764750001000,"providerData":{"messageId":"m1","model":"workbuddy-x"},"message":{"usage":{"inputTokens":100,"outputTokens":20,"cacheReadTokens":60}}}"#,
            "\n",
            r#"{"type":"message","sessionId":"s1","cwd":"/repo","role":"assistant","timestamp":1764750002000,"providerData":{"messageId":"m2","model":"workbuddy-x"},"content":[{"type":"text","text":"done"}],"message":{"usage":{"inputTokens":120,"outputTokens":30,"cacheReadTokens":80}}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "workbuddy");
    assert_eq!(parsed.metrics.tokens_input, 80);
    assert_eq!(parsed.metrics.tokens_output, 50);
    assert_eq!(parsed.metrics.tokens_cache_r, 140);
}

#[test]
fn token_accounting_opencode_reasoning_folds_into_output() {
    // Upstream #312 golden: opencode's tokens.reasoning counter is excluded
    // from tokens.output, so output = 17 + 5 (fixture parity with
    // testdata/opencode/storage/message/ses_abc/msg_assistant.json).
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let path = repo_root.join("testdata/opencode/storage/session/project_alpha/ses_abc.json");
    let parsed = parse_file(&path).expect("parse opencode storage session");
    assert_eq!(parsed.metrics.source_tool, "opencode");
    assert_eq!(parsed.metrics.tokens_input, 42);
    assert_eq!(parsed.metrics.tokens_output, 22);
    assert_eq!(parsed.metrics.tokens_cache_r, 3);
    assert_eq!(parsed.metrics.tokens_cache_w, 2);
}

#[test]
fn token_accounting_otel_usage_only_on_chat_spans() {
    // Upstream #312 golden: `invoke_agent` spans repeat the session totals of
    // their chat children; only per-request chat spans carry summable usage.
    // Here the invoke span carries 10k input and no operation attr, the chat
    // child carries 100/20 — totals must be 100/20, not 10100/20.
    let path = write_session(
        "agenttrace-ta-otel-chat-gate",
        "copilot.trace.jsonl",
        concat!(
            r#"{"name":"invoke_agent","traceId":"t1","kind":"SPAN","startTimeUnixNano":"1770000000000000000","attributes":{"gen_ai.usage.input_tokens":10000,"gen_ai.usage.output_tokens":2000}}"#,
            "\n",
            r#"{"name":"chat.completion","traceId":"t1","kind":"SPAN","startTimeUnixNano":"1770000001000000000","attributes":{"gen_ai.usage.input_tokens":100,"gen_ai.usage.output_tokens":20}}"#,
            "\n",
            r#"{"name":"tool.call","traceId":"t1","kind":"SPAN","startTimeUnixNano":"1770000002000000000","attributes":{"gen_ai.usage.input_tokens":5,"gen_ai.usage.output_tokens":1}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "copilot_cli");
    assert_eq!(parsed.metrics.tokens_input, 100);
    assert_eq!(parsed.metrics.tokens_output, 20);
}

#[test]
fn token_accounting_otel_operation_attr_also_gates_chat_usage() {
    // Upstream #312 golden: rename the child span and mark it with
    // gen_ai.operation.name = "chat" — usage still counts; the parent
    // (no operation attr) still does not.
    let path = write_session(
        "agenttrace-ta-otel-op-attr",
        "copilot.trace.jsonl",
        concat!(
            r#"{"name":"invoke_agent","traceId":"t2","kind":"SPAN","startTimeUnixNano":"1770000000000000000","attributes":{"gen_ai.usage.input_tokens":10000}}"#,
            "\n",
            r#"{"name":"llm_child_span","traceId":"t2","kind":"SPAN","startTimeUnixNano":"1770000001000000000","content":"hi","attributes":{"gen_ai.operation.name":"chat","gen_ai.usage.input_tokens":300,"gen_ai.usage.output_tokens":30}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&path).expect("parse");
    assert_eq!(parsed.metrics.source_tool, "copilot_cli");
    assert_eq!(parsed.metrics.tokens_input, 300);
    assert_eq!(parsed.metrics.tokens_output, 30);
}

#[test]
fn token_accounting_otel_accepts_both_cache_attribute_spellings() {
    // rm-519a: the nested GenAI convention attribute
    // (gen_ai.usage.cache_read.input_tokens) and the flat spelling
    // (gen_ai.usage.cache_read_input_tokens) must both map, and cached
    // tokens are already inside gen_ai.usage.input_tokens — input keeps
    // only the uncached part.
    let nested = write_session(
        "agenttrace-ta-otel-nested",
        "copilot.trace.jsonl",
        concat!(
            r#"{"name":"chat.completion","traceId":"t3","kind":"SPAN","startTimeUnixNano":"1770000000000000000","attributes":{"gen_ai.usage.input_tokens":100,"gen_ai.usage.output_tokens":20,"gen_ai.usage.cache_read.input_tokens":40,"gen_ai.usage.cache_creation.input_tokens":10}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&nested).expect("parse");
    assert_eq!(parsed.metrics.tokens_input, 50, "100 - 40 read - 10 write");
    assert_eq!(parsed.metrics.tokens_output, 20);
    assert_eq!(parsed.metrics.tokens_cache_r, 40);
    assert_eq!(parsed.metrics.tokens_cache_w, 10);

    let flat = write_session(
        "agenttrace-ta-otel-flat",
        "copilot.trace.jsonl",
        concat!(
            r#"{"name":"chat.completion","traceId":"t4","kind":"SPAN","startTimeUnixNano":"1770000000000000000","attributes":{"gen_ai.usage.input_tokens":100,"gen_ai.usage.output_tokens":20,"gen_ai.usage.cache_read_input_tokens":40,"gen_ai.usage.cache_creation_input_tokens":10}}"#,
            "\n",
        ),
    );
    let parsed = parse_file(&flat).expect("parse");
    assert_eq!(
        parsed.metrics.tokens_input, 50,
        "flat spelling maps the same"
    );
    assert_eq!(parsed.metrics.tokens_output, 20);
    assert_eq!(parsed.metrics.tokens_cache_r, 40);
    assert_eq!(parsed.metrics.tokens_cache_w, 10);
}

#[test]
fn token_accounting_gemini_sessions_fall_through_to_unsupported() {
    // Upstream #312 removed the Gemini parser (and its fixtures) — the
    // ledger and upstream v0.10.1 both treat these files as unsupported.
    // The antigravity (.gemini/antigravity-cli) format stays supported.
    let path = write_session(
        "agenttrace-ta-gemini-gone",
        "gemini-checkpoint.json",
        r#"{"model":"gemini-2.5-pro","contents":[{"role":"user","parts":[{"text":"hello"}]},{"role":"model","parts":[{"text":"hi"}]}],"usageMetadata":{"promptTokenCount":100,"candidatesTokenCount":20,"thoughtsTokenCount":40}}"#,
    );
    let parsed = parse_file(&path);
    assert!(
        parsed.is_err(),
        "Gemini-format sessions must fall through to the unsupported-format error"
    );
}

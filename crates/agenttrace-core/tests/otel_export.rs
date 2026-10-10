//! rm-493 contract tests — OTLP-JSON GenAI export (file-only).
//!
//! Golden assertions pin the OTLP wire shape and the `gen_ai.*` attribute
//! names against the semantic-convention snapshot recorded in
//! [`agenttrace_core::SEMCONV_SNAPSHOT_DATE`]. If the semconv attribute
//! names drift, these tests are the tripwire — update the constant and
//! the attribute spellings together, deliberately.

use agenttrace_core::{parse_file, report_otel_export, SEMCONV_SNAPSHOT_DATE};
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    std::env::temp_dir().join(format!("at-otel-test-{}-{}", std::process::id(), line!()))
}

fn write_session(file: &str, body: &str) -> PathBuf {
    let dir = fixture_dir();
    std::fs::create_dir_all(dir.join("sessions")).unwrap();
    let path = dir.join("sessions").join(file);
    std::fs::write(&path, body).unwrap();
    path
}

const CLAUDE_BODY: &str = concat!(
    "{\"type\":\"user\",\"timestamp\":\"2026-10-05T10:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"hello\"}}\n",
    "{\"type\":\"assistant\",\"timestamp\":\"2026-10-05T10:00:04Z\",\"message\":{\"id\":\"msg_1\",\"model\":\"claude-sonnet-4-5\",\"usage\":{\"input_tokens\":100,\"output_tokens\":50,\"cache_read_input_tokens\":40,\"cache_creation_input_tokens\":20,\"reasoning_tokens\":12}}}\n"
);

const CODEX_BODY: &str = concat!(
    "{\"timestamp\":\"2026-10-05T11:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"model\":\"gpt-5.2\",\"cwd\":\"/tmp\"}}\n",
    "{\"timestamp\":\"2026-10-05T11:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"q\"}]}}\n",
    "{\"timestamp\":\"2026-10-05T11:00:05Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"a\"}]}}\n",
    "{\"timestamp\":\"2026-10-05T11:00:05Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"input_tokens\":7,\"output_tokens\":3}}\n"
);

// Upstream #316 live-PoC shape (wb7): a workbuddy record reports more
// cached tokens than input; rm-600 clamps both sides, so the exported
// counters (input + cache_read) can never exceed the source-recorded
// input (100). The full journal shape (function_call + message +
// function_call_result) is required: the workbuddy detector keys on the
// tool-call records, and a bare message line would silently fall to the
// generic lane.
const WORKBUDDY_BODY: &str = concat!(
    "{\"type\":\"function_call\",\"name\":\"bash\",\"arguments\":\"{}\",\"sessionId\":\"wb-otel\",\"cwd\":\"/tmp\",\"callId\":\"c1\"}\n",
    "{\"type\":\"message\",\"role\":\"user\",\"sessionId\":\"wb-otel\",\"cwd\":\"/tmp\",\"content\":[{\"type\":\"text\",\"text\":\"go\"}],\"message\":{\"role\":\"user\",\"usage\":{\"input_tokens\":100,\"output_tokens\":10,\"cache_read_input_tokens\":150}}}\n",
    "{\"type\":\"function_call_result\",\"output\":\"done\",\"callId\":\"c1\",\"sessionId\":\"wb-otel\"}\n"
);

fn attr<'a>(span: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    span["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|kv| kv["key"] == key)
        .map(|kv| &kv["value"])
        .unwrap_or_else(|| panic!("missing attribute {key}"))
}

fn opt_attr<'a>(span: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    span["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|kv| kv["key"] == key)
        .map(|kv| &kv["value"])
}

#[test]
fn otel_export_shape_and_genai_attributes() {
    let a = write_session("claude-a.jsonl", CLAUDE_BODY);
    let b = write_session("codex-b.jsonl", CODEX_BODY);
    let c = write_session("workbuddy-c.jsonl", WORKBUDDY_BODY);
    let sessions = vec![
        parse_file(&a).unwrap(),
        parse_file(&b).unwrap(),
        parse_file(&c).unwrap(),
    ];
    let doc: serde_json::Value = serde_json::from_str(&report_otel_export(&sessions)).unwrap();

    let spans = &doc["resourceSpans"][0]["scopeSpans"][0]["spans"];
    assert_eq!(
        spans.as_array().map(Vec::len),
        Some(3),
        "one span per session"
    );

    let resource = &doc["resourceSpans"][0]["resource"]["attributes"];
    let service_name = resource
        .as_array()
        .unwrap()
        .iter()
        .find(|kv| kv["key"] == "service.name")
        .map(|kv| &kv["value"]["stringValue"])
        .unwrap();
    assert_eq!(service_name, "agenttrace");

    let claude_span = &spans[0];
    let codex_span = &spans[1];
    let workbuddy_span = &spans[2];

    // gen_ai.* attribute names pinned to the semconv snapshot.
    assert_eq!(
        attr(claude_span, "gen_ai.system")["stringValue"],
        "claude-code"
    );
    assert_eq!(attr(codex_span, "gen_ai.system")["stringValue"], "codex");
    assert_eq!(
        attr(workbuddy_span, "gen_ai.system")["stringValue"],
        "workbuddy"
    );
    assert_eq!(
        attr(claude_span, "gen_ai.request.model")["stringValue"],
        "claude-sonnet-4-5"
    );
    assert_eq!(
        attr(claude_span, "gen_ai.usage.input_tokens")["intValue"],
        100
    );
    assert_eq!(
        attr(claude_span, "gen_ai.usage.output_tokens")["intValue"],
        // reasoning_tokens folds into output_tokens (pass-9 CU-20):
        // 50 recorded + 12 reasoning = 62 on the wire, with the 12
        // ALSO reported separately below.
        62
    );

    // rm-599: the Recommended cache/reasoning subset counters of the
    // GenAI token-metrics set — cache buckets always emit (honest
    // zeros), reasoning only when the session recorded any.
    assert_eq!(
        attr(claude_span, "gen_ai.usage.cache_read.input_tokens")["intValue"],
        40
    );
    assert_eq!(
        attr(claude_span, "gen_ai.usage.cache_write.input_tokens")["intValue"],
        20
    );
    assert_eq!(
        attr(claude_span, "gen_ai.usage.reasoning.output_tokens")["intValue"],
        12
    );
    assert_eq!(
        attr(codex_span, "gen_ai.usage.cache_read.input_tokens")["intValue"],
        0
    );
    assert_eq!(
        attr(codex_span, "gen_ai.usage.cache_write.input_tokens")["intValue"],
        0
    );
    assert!(
        opt_attr(codex_span, "gen_ai.usage.reasoning.output_tokens").is_none(),
        "reasoning attr only when recorded"
    );

    // wb7 shape: 150 cached reported against 100 input clamps to
    // input 0 + cache_read 100 — the exported total (input + cache)
    // never exceeds the source-recorded input.
    assert_eq!(
        attr(workbuddy_span, "gen_ai.usage.input_tokens")["intValue"],
        0
    );
    assert_eq!(
        attr(workbuddy_span, "gen_ai.usage.output_tokens")["intValue"],
        10
    );
    assert_eq!(
        attr(workbuddy_span, "gen_ai.usage.cache_read.input_tokens")["intValue"],
        100
    );

    // The input-token basis travels as an explicit disclosure so a
    // consumer never has to guess whether input_tokens includes the
    // cached portion (it does not — see otel.rs module docs).
    for span in [claude_span, codex_span, workbuddy_span] {
        assert_eq!(
            attr(span, "agenttrace.export.input_token_basis")["stringValue"],
            "delta_excludes_cache"
        );
    }

    // Cost + provenance ride as agenttrace-namespaced attributes.
    assert!(attr(claude_span, "agenttrace.session.pricing_source")["stringValue"].is_string());
    assert!(
        attr(claude_span, "agenttrace.session.cost_usd")["doubleValue"]
            .as_f64()
            .is_some()
    );

    // Deterministic identity: trace ids differ per session, span ids are
    // 16 hex, session spans have no parent.
    assert_ne!(claude_span["traceId"], codex_span["traceId"]);
    for span in [claude_span, codex_span, workbuddy_span] {
        let trace = span["traceId"].as_str().unwrap();
        let sid = span["spanId"].as_str().unwrap();
        assert_eq!(trace.len(), 32);
        assert_eq!(sid.len(), 16);
        assert!(trace.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(sid.chars().all(|c| c.is_ascii_hexdigit()));
        // rm-599: the OTel trace-data format requires a non-zero span
        // id — the ordinal-0 session used to render the all-zero
        // 0000000000000000, which conforming OTLP receivers reject,
        // dropping the ENTIRE export request.
        assert_ne!(sid, "0000000000000000", "all-zero spanId is spec-invalid");
        assert!(span["parentSpanId"].is_null());
        assert_eq!(span["kind"], 1, "INTERNAL");
    }
    // rm-651: trace ids mirror the span-id validity rule — the all-zero
    // traceId is the other id a conforming OTLP receiver rejects (again
    // dropping the ENTIRE export request). Pin non-zero and
    // unique-per-session across the whole export.
    let mut trace_ids = std::collections::BTreeSet::new();
    for span in [claude_span, codex_span, workbuddy_span] {
        let trace = span["traceId"].as_str().unwrap();
        assert_ne!(
            trace, "00000000000000000000000000000000",
            "all-zero traceId is spec-invalid"
        );
        assert!(
            trace.bytes().any(|b| b != b'0'),
            "traceId must carry at least one non-zero byte"
        );
        trace_ids.insert(trace.to_string());
    }
    assert_eq!(trace_ids.len(), 3, "trace ids unique per session");
    // Span ids hash the session identity + ordinal: distinct across
    // sessions (incl. the FIRST span of the export — the old ordinal-0
    // bug) and stable across renders.
    assert_ne!(claude_span["spanId"], codex_span["spanId"]);
    assert_ne!(claude_span["spanId"], workbuddy_span["spanId"]);
    assert_ne!(codex_span["spanId"], workbuddy_span["spanId"]);
    let rerender: serde_json::Value = serde_json::from_str(&report_otel_export(&sessions)).unwrap();
    assert_eq!(
        rerender["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["spanId"], claude_span["spanId"],
        "span ids deterministic across renders"
    );

    // Time bounds: session span covers first..last timestamp.
    assert!(claude_span["startTimeUnixNano"].as_u64().unwrap() > 0);
    assert!(
        claude_span["endTimeUnixNano"].as_u64().unwrap()
            >= claude_span["startTimeUnixNano"].as_u64().unwrap()
    );

    // The semconv snapshot date travels with the export.
    assert_eq!(
        attr(claude_span, "agenttrace.export.semconv_snapshot_date")["stringValue"],
        SEMCONV_SNAPSHOT_DATE
    );
}

#[test]
fn span_ids_are_valid_nonzero_and_deterministic() {
    // rm-605: `span_id_for(ordinal)` used to zero-pad the seed, so the
    // FIRST span of every export carried spanId 0000000000000000 —
    // reserved as invalid by OTLP/W3C trace-id/span-id semantics and
    // dropped by collectors. Every span id must be non-zero, distinct
    // within the doc, and stable across exports.
    let a = write_session("claude-a.jsonl", CLAUDE_BODY);
    let b = write_session("codex-b.jsonl", CODEX_BODY);
    let sessions = vec![parse_file(&a).unwrap(), parse_file(&b).unwrap()];
    let first: serde_json::Value = serde_json::from_str(&report_otel_export(&sessions)).unwrap();
    let second: serde_json::Value = serde_json::from_str(&report_otel_export(&sessions)).unwrap();

    let spans = first["resourceSpans"][0]["scopeSpans"][0]["spans"]
        .as_array()
        .unwrap()
        .clone();
    let mut seen = std::collections::HashSet::new();
    for span in &spans {
        let sid = span["spanId"].as_str().unwrap();
        assert_eq!(sid.len(), 16);
        assert!(sid.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(
            sid, "0000000000000000",
            "reserved all-zero span id on an export span (rm-605)"
        );
        assert!(
            seen.insert(sid.to_string()),
            "span ids must be distinct: {sid}"
        );
        // rm-605 review rider: trace ids derive through the same hash
        // and clamp the reserved all-zero value symmetrically.
        let tid = span["traceId"].as_str().unwrap();
        assert_eq!(tid.len(), 32);
        assert!(tid.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(
            tid, "00000000000000000000000000000000",
            "reserved all-zero trace id on an export span (rm-605 rider)"
        );
    }
    assert_eq!(first, second, "export must be deterministic across calls");
}

#[test]
fn otel_export_empty_input_is_valid_otlp() {
    let doc: serde_json::Value = serde_json::from_str(&report_otel_export(&[])).unwrap();
    let spans = &doc["resourceSpans"][0]["scopeSpans"][0]["spans"];
    assert_eq!(spans.as_array().map(Vec::len), Some(0));
}

#[test]
fn non_finite_cost_is_omitted_with_disclosure_not_null() {
    // rm-599 acceptance 2: a non-finite cost (NaN/±inf from a hostile
    // or future pricing source) must never reach the wire as a
    // doubleValue — serde renders it as `null`, and a null AnyValue is
    // spec-invalid (conforming OTLP receivers reject the request).
    // The attribute is omitted and the omission disclosed instead.
    let path = write_session("nan-cost.jsonl", CLAUDE_BODY);
    let mut session = parse_file(&path).unwrap();
    session.metrics.cost_estimated = f64::NAN;
    let raw = report_otel_export(&[session]);
    assert!(
        !raw.contains("\"doubleValue\":null"),
        "null AnyValue on the wire: {raw}"
    );
    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let span = &doc["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
    assert!(
        opt_attr(span, "agenttrace.session.cost_usd").is_none(),
        "non-finite cost must be omitted, not null"
    );
    assert_eq!(
        attr(span, "agenttrace.session.cost_usd_omitted")["stringValue"],
        "non-finite"
    );
}

#[test]
fn otel_export_has_no_network_transport() {
    // File-only contract: the renderer module must not reference any
    // socket/HTTP machinery. Cheap textual tripwire over the source.
    let src = include_str!("../src/otel.rs");
    for banned in [
        "TcpStream",
        "UdpSocket",
        "reqwest",
        "ureq",
        "http://",
        "https://",
    ] {
        assert!(!src.contains(banned), "otel.rs must not reference {banned}");
    }
}

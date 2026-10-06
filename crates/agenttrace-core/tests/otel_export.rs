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
    "{\"type\":\"assistant\",\"timestamp\":\"2026-10-05T10:00:04Z\",\"message\":{\"id\":\"msg_1\",\"model\":\"claude-sonnet-4-5\",\"usage\":{\"input_tokens\":100,\"output_tokens\":50}}}\n"
);

const CODEX_BODY: &str = concat!(
    "{\"timestamp\":\"2026-10-05T11:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"model\":\"gpt-5.2\",\"cwd\":\"/tmp\"}}\n",
    "{\"timestamp\":\"2026-10-05T11:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"q\"}]}}\n",
    "{\"timestamp\":\"2026-10-05T11:00:05Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"a\"}]}}\n",
    "{\"timestamp\":\"2026-10-05T11:00:05Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"input_tokens\":7,\"output_tokens\":3}}\n"
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

#[test]
fn otel_export_shape_and_genai_attributes() {
    let a = write_session("claude-a.jsonl", CLAUDE_BODY);
    let b = write_session("codex-b.jsonl", CODEX_BODY);
    let sessions = vec![parse_file(&a).unwrap(), parse_file(&b).unwrap()];
    let doc: serde_json::Value = serde_json::from_str(&report_otel_export(&sessions)).unwrap();

    let spans = &doc["resourceSpans"][0]["scopeSpans"][0]["spans"];
    assert_eq!(
        spans.as_array().map(Vec::len),
        Some(2),
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

    // gen_ai.* attribute names pinned to the semconv snapshot.
    assert_eq!(
        attr(claude_span, "gen_ai.system")["stringValue"],
        "claude-code"
    );
    assert_eq!(attr(codex_span, "gen_ai.system")["stringValue"], "codex");
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
        50
    );

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
    for span in [claude_span, codex_span] {
        let trace = span["traceId"].as_str().unwrap();
        let sid = span["spanId"].as_str().unwrap();
        assert_eq!(trace.len(), 32);
        assert_eq!(sid.len(), 16);
        assert!(trace.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(sid.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(span["parentSpanId"].is_null());
        assert_eq!(span["kind"], 1, "INTERNAL");
    }

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
fn otel_export_empty_input_is_valid_otlp() {
    let doc: serde_json::Value = serde_json::from_str(&report_otel_export(&[])).unwrap();
    let spans = &doc["resourceSpans"][0]["scopeSpans"][0]["spans"];
    assert_eq!(spans.as_array().map(Vec::len), Some(0));
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

// ---------------------------------------------------------------------------
// rm-568: gen_ai.system from the session's source family, not path
// substrings.
// ---------------------------------------------------------------------------

fn write_under(dirname: &str, file: &str, body: &str) -> PathBuf {
    let dir = fixture_dir().join(dirname);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file);
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn rm568_gen_ai_system_follows_source_tool_not_path() {
    // The same Claude journal parsed from a directory whose name
    // contains a foreign marker: the session's source family (recorded
    // from the journal itself) must win over path inference. Before
    // rm-568 this session exported gen_ai.system=cursor because the
    // path marker scan matched the directory name first.
    let path = write_under("cursor-backup", "claude-twin.jsonl", CLAUDE_BODY);
    let session = parse_file(&path).unwrap();
    assert_eq!(session.metrics.source_tool, "claude_code");
    let doc: serde_json::Value = serde_json::from_str(&report_otel_export(&[session])).unwrap();
    let span = &doc["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
    assert_eq!(
        attr(span, "agenttrace.session.source_tool")["stringValue"],
        "claude_code"
    );
    assert_eq!(attr(span, "gen_ai.system")["stringValue"], "claude-code");
}

#[test]
fn rm568_arbitrary_dir_does_not_demote_to_other() {
    // A --dir-loaded claude session (arbitrary directory name with no
    // known marker) must not export gen_ai.system=other: the family
    // comes from the journal; path inference is only the fallback for
    // sessions that carry none.
    let path = write_under("archival-dump-2026", "session.jsonl", CLAUDE_BODY);
    let session = parse_file(&path).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&report_otel_export(&[session])).unwrap();
    let span = &doc["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
    assert_eq!(attr(span, "gen_ai.system")["stringValue"], "claude-code");
}

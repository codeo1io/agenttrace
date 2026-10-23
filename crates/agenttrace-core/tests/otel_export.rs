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
    // rm-1065: per-call unique fixture directories. The pid separates
    // concurrent processes; the monotonically increasing suffix separates
    // tests (and successive writes) within one process, so the two suites
    // that rewrite the same fixture filenames can never observe each
    // other's in-flight O_TRUNC writes.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("at-otel-test-{}-{}", std::process::id(), seq))
}

fn write_session(file: &str, body: &str) -> PathBuf {
    let dir = fixture_dir();
    std::fs::create_dir_all(dir.join("sessions")).unwrap();
    let path = dir.join("sessions").join(file);
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn fixture_dirs_are_unique_per_call() {
    // rm-1065: fixture_dir() must return a fresh directory per call — the
    // suites at this file's two write_session() sites rewrite the same
    // fixture filenames (claude-a.jsonl / codex-b.jsonl), so a shared
    // directory leaves an intra-process window where one test's O_TRUNC
    // write is observed mid-truncation by another test's
    // parse_file().unwrap(). The pid alone separates processes; the
    // per-call suffix must separate tests (and successive writes) within
    // one process.
    let a = fixture_dir();
    let b = fixture_dir();
    assert_ne!(a, b, "fixture_dir() must return a fresh directory per call");
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
        // rm-863: the flat `reasoning_tokens` spelling is a SUBSET
        // alias — the documented breakdown INSIDE output_tokens — so
        // the fold records the 12 separately WITHOUT adding (pass-9
        // CU-20 added it blindly and billed 62 on the wire).
        50
    );

    // rm-599 subset counters, as clarified at the 2026-10-21 pin
    // (rm-493, commit 9ffc7e00c639): the optional token-breakdown
    // attributes are recommended only when available and NON-ZERO
    // (#548) — claude (40/20) and workbuddy (100) emit theirs, codex
    // (all-zero buckets) omits both, and reasoning only emits when
    // the session recorded any. The always-on honest-zero disclosure
    // for cache buckets lives on the agenttrace.session.* mirror
    // attributes below.
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
    assert!(
        opt_attr(codex_span, "gen_ai.usage.cache_read.input_tokens").is_none(),
        "zero cache_read bucket must be omitted (#548, snapshot 2026-10-21)"
    );
    assert!(
        opt_attr(codex_span, "gen_ai.usage.cache_write.input_tokens").is_none(),
        "zero cache_write bucket must be omitted (#548, snapshot 2026-10-21)"
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
fn genai_attribute_name_set_matches_the_2026_10_21_pin() {
    // rm-493: pin the EXACT gen_ai.* attribute names this export may
    // emit at snapshot 2026-10-21 (semantic-conventions-genai commit
    // 9ffc7e00c639). A rename, a stray addition, or a zero bucket
    // leaking back in fails here before it reaches a consumer. The
    // claude fixture records every bucket non-zero (full set); the
    // codex fixture records none of the optional ones (base set).
    let a = write_session("claude-a.jsonl", CLAUDE_BODY);
    let b = write_session("codex-b.jsonl", CODEX_BODY);
    let sessions = vec![parse_file(&a).unwrap(), parse_file(&b).unwrap()];
    let doc: serde_json::Value = serde_json::from_str(&report_otel_export(&sessions)).unwrap();
    let spans = doc["resourceSpans"][0]["scopeSpans"][0]["spans"]
        .as_array()
        .unwrap();

    let base = [
        "gen_ai.system",
        "gen_ai.request.model",
        "gen_ai.usage.input_tokens",
        "gen_ai.usage.output_tokens",
    ];
    let breakdown = [
        "gen_ai.usage.cache_read.input_tokens",
        "gen_ai.usage.cache_write.input_tokens",
        "gen_ai.usage.reasoning.output_tokens",
    ];
    let names = |span: &serde_json::Value| {
        span["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|kv| kv["key"].as_str().unwrap().starts_with("gen_ai."))
            .map(|kv| kv["key"].as_str().unwrap().to_string())
            .collect::<std::collections::BTreeSet<_>>()
    };

    let claude = &spans[0];
    let codex = &spans[1];
    let expected_full: std::collections::BTreeSet<String> = base
        .iter()
        .chain(breakdown.iter())
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        names(claude),
        expected_full,
        "all-buckets session emits the full pinned set"
    );

    let expected_base: std::collections::BTreeSet<String> =
        base.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        names(codex),
        expected_base,
        "zero buckets are omitted (#548)"
    );

    // No unknown gen_ai.* name ever rides a span.
    for span in [claude, codex] {
        for name in names(span) {
            assert!(
                base.contains(&name.as_str()) || breakdown.contains(&name.as_str()),
                "unpinned gen_ai.* attribute on the wire: {name}"
            );
        }
    }
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

#[test]
fn prompt_id_joins_statusline_captures_to_otel_export() {
    // rm-708 end-to-end fixture pair (cycle 2, run 415d77e1): the
    // statusline payload's `prompt_id` is documented to "Match the
    // `prompt.id` attribute on OpenTelemetry events, when they exist"
    // (statusline docs, fetched 2026-10-07). The fork's own emitter
    // does not set `prompt.id` yet (that arm is a later OTel change);
    // an emitter that does — the next fork arm or a CC-native otel
    // lane — produces spans like the ones injected below on the real
    // export document. The pure capture-side join must then correlate
    // exactly the shared ids, and only those.
    use agenttrace_core::{correlated_prompt_ids, CapturedStatusline};

    let a = write_session("claude-join.jsonl", CLAUDE_BODY);
    let sessions = vec![parse_file(&a).unwrap()];
    let raw = report_otel_export(&sessions);
    let mut doc: serde_json::Value = serde_json::from_str(&raw).unwrap();

    // Inject the documented attribute onto the real exported span,
    // keeping the OTLP-JSON wire shape (`attributes[]` with
    // `key` + `value.stringValue`) this export already pins.
    let span = &mut doc["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
    span["attributes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "key": "prompt.id",
            "value": {"stringValue": "550e8400-e29b-41d4-a716-446655440000"}
        }));

    let capture = |session_id: &str, prompt_id: Option<&str>| CapturedStatusline {
        captured_at: 1_790_000_000,
        payload: match prompt_id {
            Some(id) => serde_json::json!({
                "session_id": session_id,
                "prompt_id": id,
                "context_window": {"used_percentage": 41.2}
            }),
            None => serde_json::json!({"session_id": session_id}),
        },
    };
    let captures = vec![
        capture("sess-a", Some("550e8400-e29b-41d4-a716-446655440000")),
        capture("sess-b", Some("00000000-0000-4000-8000-000000000000")),
        capture("sess-c", None),
    ];

    // The join surface: exactly the shared id correlates.
    assert_eq!(
        correlated_prompt_ids(&captures, &doc),
        vec!["550e8400-e29b-41d4-a716-446655440000".to_string()],
        "one prompt id shared between the journal and the export"
    );

    // Control: without the attribute the real export joins nothing —
    // the emitter-side arm being absent is a disclosed empty state,
    // never an error.
    let bare: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(correlated_prompt_ids(&captures, &bare).is_empty());
}

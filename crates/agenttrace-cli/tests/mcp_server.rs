//! e2e for `agenttrace mcp` (rm-455): the local read-only MCP server.
//!
//! The contract under test is the full stdio wire behavior of the
//! built binary against a seeded, hermetic `HOME`:
//!
//! * the JSON-RPC handshake (initialize → notifications/initialized →
//!   tools/list → tools/call) answers exactly one response per
//!   request and stays silent for notifications;
//! * both read-only tools render the same discovered corpus the CLI
//!   reports on (`usage_overview` is the `--overview -f json`
//!   document; `by_model_breakdown` is its `by_model` rollup);
//! * malformed JSON, unknown methods, bad tool params, and batch
//!   requests answer the JSON-RPC error arms (-32700/-32600/-32601/
//!   -32602) instead of dying;
//! * a discovered-but-empty corpus is a tool-level `isError` result —
//!   the server exits 0 either way, because the host stays connected.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

/// Resolve the repo-root `testdata/generated` fixture directory,
/// matching the convention in tests/entrypoints.rs.
fn generated_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/generated")
        .join(name)
}

/// Seed a hermetic `HOME` (optionally with one discovered session) and
/// its session-cache dir. Every test owns unique directories so
/// parallel runs never share state.
fn seed_home(tag: &str, with_fixture: bool) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("agenttrace-mcp-{tag}-{}", std::process::id()));
    let home = root.join("home");
    let cache = root.join("cache");
    if with_fixture {
        let sessions = home.join(".hermes").join("sessions");
        std::fs::create_dir_all(&sessions).expect("create sessions home");
        std::fs::copy(
            generated_fixture("detailed-tool-steps.jsonl"),
            sessions.join("demo.jsonl"),
        )
        .expect("seed fixture session");
    } else {
        std::fs::create_dir_all(&home).expect("create empty home");
    }
    std::fs::create_dir_all(&cache).expect("create cache dir");
    (home, cache)
}

/// Run `agenttrace mcp` against the seeded home, feed it the whole
/// request script, and return (exit status, parsed responses, stderr).
/// The server is strictly line-sequential, so writing stdin to EOF and
/// then reading stdout after exit is deterministic.
fn mcp_session(home: &Path, cache: &Path, requests: &str) -> (Option<i32>, Vec<Value>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("mcp")
        .env("HOME", home)
        .env("AGENTTRACE_SESSION_CACHE_DIR", cache)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn agenttrace mcp");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(requests.as_bytes())
        .expect("write request script");
    let output = child.wait_with_output().expect("collect mcp output");
    let responses = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every response line is JSON"))
        .collect();
    (
        output.status.code(),
        responses,
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn text_payload(response: &Value) -> Value {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("tool result carries text content");
    serde_json::from_str(text).expect("tool text payload is JSON")
}

#[test]
fn handshake_tools_and_report_rendering() {
    // Happy path end to end: initialize, the initialized notification,
    // tools/list, and one call per tool. Five requests, four responses
    // — the notification is answered with silence, and stderr stays
    // empty because nothing about a well-formed session is a
    // diagnostic.
    let (home, cache) = seed_home("handshake", true);
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"usage_overview","arguments":{"range":"all"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":"all"}}}"#,
        "\n",
    );
    let (code, responses, stderr) = mcp_session(&home, &cache, requests);
    assert_eq!(Some(0), code, "clean stdin EOF must exit rc0");
    assert_eq!(
        4,
        responses.len(),
        "one response per request, none for the notification"
    );
    assert!(
        stderr.trim().is_empty(),
        "no diagnostics on the happy path: {stderr}"
    );

    let init = &responses[0];
    assert_eq!(
        Some("2025-06-18"),
        init["result"]["protocolVersion"].as_str(),
        "initialize echoes the client protocol version"
    );
    assert_eq!("agenttrace", init["result"]["serverInfo"]["name"]);
    assert!(
        init["result"]["capabilities"]["tools"].is_object(),
        "capabilities declare tools"
    );

    let tools = &responses[1]["result"]["tools"];
    let names: Vec<&str> = tools
        .as_array()
        .expect("tools/list returns an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    assert_eq!(names, vec!["usage_overview", "by_model_breakdown"]);
    let range_enum = &tools[0]["inputSchema"]["properties"]["range"]["enum"];
    assert_eq!(
        serde_json::json!(["today", "7d", "30d", "all"]),
        *range_enum,
        "the tool schema must offer exactly the CLI's range labels"
    );

    // usage_overview answers with the CLI's own overview document —
    // the discovered fixture session lands in by_model under gpt-5.
    let overview = &responses[2];
    assert_eq!(false, overview["result"]["isError"]);
    let payload = text_payload(overview);
    assert_eq!(Some("all"), payload["scope"]["range"].as_str());
    let by_model = payload["by_model"]
        .as_array()
        .expect("overview carries by_model");
    assert!(
        by_model.iter().any(|row| row["name"] == "gpt-5"),
        "the seeded fixture must be reported: {by_model:?}"
    );

    let breakdown = &responses[3];
    assert_eq!(false, breakdown["result"]["isError"]);
    let breakdown_payload = text_payload(breakdown);
    assert_eq!(Some("all"), breakdown_payload["range"].as_str());
    assert_eq!(1, breakdown_payload["total_sessions"]);
    assert!(
        breakdown_payload["by_model"]
            .as_array()
            .expect("by_model array")
            .iter()
            .any(|row| row["name"] == "gpt-5" && row["sessions"] == 1),
        "breakdown must roll the seeded session: {}",
        breakdown_payload["by_model"]
    );

    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn range_scopes_the_report_window() {
    // `7d` must actually scope: the fixture is dated 2025-01-01, so a
    // rolling week around now finds nothing. The server answers with
    // a tool-level error result and keeps serving.
    let (home, cache) = seed_home("range", true);
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":"7d"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"usage_overview","arguments":{}}}"#,
        "\n",
    );
    let (code, responses, _stderr) = mcp_session(&home, &cache, requests);
    assert_eq!(Some(0), code);
    assert_eq!(2, responses.len());
    assert_eq!(
        true, responses[0]["result"]["isError"],
        "out-of-window corpus is a tool error, not a protocol one"
    );
    assert!(
        responses[0]["result"]["content"][0]["text"]
            .as_str()
            .expect("error text")
            .contains("No session files found"),
        "the empty-corpse message must name the failure"
    );
    // Absent arguments default to `all` — the second call reports.
    assert_eq!(false, responses[1]["result"]["isError"]);
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn refusal_arms_match_json_rpc_error_codes() {
    // Each failure mode answers its spec error and the server stays
    // up for the next line: parse error (-32700, id null), unknown
    // method (-32601), unknown tool / bad range / missing name /
    // non-string range (-32602), and unsupported batch (-32600).
    let (home, cache) = seed_home("refusal", true);
    let requests = concat!(
        "{oops\n",
        r#"{"jsonrpc":"2.0","id":10,"method":"nope/method"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"not_a_tool"}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"usage_overview","arguments":{"range":"since-2020"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"usage_overview","arguments":{"range":7}}}"#,
        "\n",
        r#"[{"jsonrpc":"2.0","id":15,"method":"ping"}]"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":16,"method":"ping"}"#,
        "\n",
    );
    let (code, responses, _stderr) = mcp_session(&home, &cache, requests);
    assert_eq!(
        Some(0),
        code,
        "a well-formed refusal script still exits rc0 on EOF"
    );
    assert_eq!(
        8,
        responses.len(),
        "one error arm per failure, ping answered"
    );

    let code_of = |index: usize| responses[index]["error"]["code"].as_i64();
    assert_eq!(Some(-32700), code_of(0));
    assert_eq!(
        serde_json::json!(null),
        responses[0]["id"],
        "parse errors answer with id null"
    );
    assert_eq!(Some(-32601), code_of(1));
    assert_eq!(Some(-32602), code_of(2));
    assert!(
        responses[2]["error"]["message"]
            .as_str()
            .expect("message")
            .contains("unknown tool"),
        "unknown tools are invalid params"
    );
    assert_eq!(Some(-32602), code_of(3));
    assert!(
        responses[3]["error"]["message"]
            .as_str()
            .expect("message")
            .contains("today, 7d, 30d, all"),
        "bad ranges must list the allowed labels"
    );
    assert_eq!(Some(-32602), code_of(4));
    assert_eq!(Some(-32602), code_of(5));
    assert_eq!(
        Some(-32600),
        code_of(6),
        "batch requests are rejected loudly"
    );
    assert!(
        responses[7]["result"].is_object(),
        "ping answers an empty result"
    );
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn non_object_arguments_is_invalid_params_and_does_not_execute() {
    // rm-840 (renumbered from campaign-local rm-800): `arguments` must be an object when present. The assess
    // pipe PoC sent `"garbage-not-object"` where the arguments object
    // belongs and the tool silently executed with default arguments;
    // the guard answers -32602 naming the tool and the violated
    // constraint and never runs the tool. Omitted `arguments` (and
    // JSON null — this codebase's optional-value convention) keep the
    // spec-legal empty-object default and DO execute against the
    // seeded fixture; a real object still executes.
    let (home, cache) = seed_home("rm800-guard", true);
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":"garbage-not-object"}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"usage_overview","arguments":9001}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":false}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"usage_overview","arguments":[1,2,3]}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"by_model_breakdown"}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":null}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":"all"}}}"#,
        "\n",
    );
    let (code, responses, _stderr) = mcp_session(&home, &cache, requests);
    assert_eq!(Some(0), code, "refusals still exit rc0 on EOF");
    assert_eq!(7, responses.len(), "one response per request line");

    // The four non-object arms refuse with -32602, name the tool and
    // the violated constraint, and carry no result — the tool did not
    // execute.
    for response in responses.iter().take(4) {
        let error = &response["error"];
        assert_eq!(
            Some(-32602),
            error["code"].as_i64(),
            "non-object arguments: {error}"
        );
        let message = error["message"].as_str().expect("message");
        assert!(
            message.contains("arguments")
                && message.contains("must be an object")
                && (message.contains("by_model_breakdown") || message.contains("usage_overview")),
            "message names the tool and the constraint: {message}"
        );
        assert!(
            response.get("result").is_none(),
            "the tool must not execute: {response}"
        );
    }

    // Absent, null, and a real object all execute against the seeded
    // fixture: a result comes back, not an error.
    for response in responses.iter().skip(4) {
        assert!(
            response.get("result").is_some(),
            "default arguments are spec-legal: {response}"
        );
    }
}

#[test]
fn empty_home_is_a_tool_error_not_a_crash() {
    // No discovered homes at all: the tools report `isError` and the
    // server still exits 0 — the host connection is not the failure.
    let (home, cache) = seed_home("empty", false);
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"by_model_breakdown"}}"#,
        "\n",
    );
    let (code, responses, _stderr) = mcp_session(&home, &cache, requests);
    assert_eq!(Some(0), code);
    assert_eq!(1, responses.len());
    assert_eq!(true, responses[0]["result"]["isError"]);
    assert!(responses[0]["result"]["content"][0]["text"]
        .as_str()
        .expect("error text")
        .contains("No session files found"));
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

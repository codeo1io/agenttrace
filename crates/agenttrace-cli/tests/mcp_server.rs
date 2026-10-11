//! e2e for `agenttrace mcp` (rm-455): the local read-only MCP server.
//!
//! The contract under test is the full stdio wire behavior of the
//! built binary against a seeded, hermetic `HOME`:
//!
//! * the JSON-RPC handshake (initialize → notifications/initialized →
//!   tools/list → tools/call) answers exactly one response per
//!   request and stays silent for notifications, and `initialize`
//!   negotiates the newest mutually supported protocol revision
//!   instead of echoing the client's (rm-780);
//! * both read-only tools render the same discovered corpus the CLI
//!   reports on (`usage_overview` is the `--overview -f json`
//!   document; `by_model_breakdown` is its `by_model` rollup), priced
//!   through the same layered configuration the CLI resolves — a
//!   `--config`/`--pricing-file` override moves both lanes equally
//!   (rm-781), and `-d/--dir` before the keyword scopes the corpus;
//! * flags before the `mcp` keyword are either honored (`-d`, the
//!   config family) or refused at rc2 — never silently dropped
//!   (rm-781);
//! * malformed JSON, unknown methods, bad tool params, and batch
//!   requests answer the JSON-RPC error arms (-32700/-32600/-32601/
//!   -32602) instead of dying, and so do hostile wire lines — a
//!   non-UTF-8 line answers -32700 and a line past the 1 MiB cap is
//!   refused with -32600 naming the transport cap (the landed rm-923
//!   refusal contract, which supersedes this run's -32700 draft at
//!   integration), and the server keeps serving either way (rm-782);
//! * a discovered-but-empty corpus is a tool-level `isError` result —
//!   the server exits 0 either way, because the host stays connected,
//!   and a corpus whose sessions fall outside the requested window
//!   reports a filter miss, not a discovery miss (rm-781).

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

/// Run the built CLI with `argv` (which includes the `mcp` keyword
/// when a server is wanted), a hermetic environment, and stdin fed
/// from `input` to EOF; return (exit code, stdout response objects,
/// stderr). Environment hermeticity matters as much as HOME: the mcp
/// lane resolves the layered configuration (rm-781), so a leaking
/// `XDG_CONFIG_HOME`/`AGENTTRACE_PRICING_FILE` from the parent would
/// steer a user layer into a test that never asked for one.
fn mcp_session_raw(
    home: &Path,
    cache: &Path,
    argv: &[&str],
    input: &[u8],
) -> (Option<i32>, Vec<Value>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(argv)
        .env("HOME", home)
        .env("AGENTTRACE_SESSION_CACHE_DIR", cache)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("AGENTTRACE_PRICING_FILE")
        .env_remove("AGENTTRACE_HISTORY_DIR")
        .current_dir(home.parent().expect("seeded root"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn agenttrace");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(input)
        .expect("write request script");
    let output = child.wait_with_output().expect("collect output");
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

/// Run `agenttrace mcp` against the seeded home, feed it the whole
/// request script, and return (exit status, parsed responses, stderr).
/// The server is strictly line-sequential, so writing stdin to EOF and
/// then reading stdout after exit is deterministic.
fn mcp_session(home: &Path, cache: &Path, requests: &str) -> (Option<i32>, Vec<Value>, String) {
    mcp_session_raw(home, cache, &["mcp"], requests.as_bytes())
}

/// Run the CLI to completion without a request script (the lanes that
/// exit before serving — flag refusal, `-d` admission): stdin is null
/// so a process that unexpectedly tries to serve hits EOF and exits
/// rc0, which the caller's rc2 assertion then catches.
fn cli_run(home: &Path, cache: &Path, argv: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(argv)
        .env("HOME", home)
        .env("AGENTTRACE_SESSION_CACHE_DIR", cache)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("AGENTTRACE_PRICING_FILE")
        .env_remove("AGENTTRACE_HISTORY_DIR")
        .current_dir(home.parent().expect("seeded root"))
        .stdin(Stdio::null())
        .output()
        .expect("run agenttrace");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).to_string(),
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
        "a supported client version is negotiated to itself (rm-780)"
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
        serde_json::json!([
            "today", "day", "1d", "7d", "week", "weekly", "30d", "month", "monthly", "all", ""
        ]),
        *range_enum,
        "the tool schema must publish exactly the labels TimeRange::parse \
         enforces (rm-782)"
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
            .contains("No sessions match the requested filters"),
        "a discovered corpus with zero in-window sessions is a FILTER \
         miss naming the discovery count, not a discovery miss (rm-781)"
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
            .contains("weekly")
            && responses[3]["error"]["message"]
                .as_str()
                .expect("message")
                .contains("monthly"),
        "bad ranges must list the full alias set the parser enforces \
         (rm-782): {}",
        responses[3]["error"]["message"]
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

#[test]
fn oversized_message_line_is_refused_and_the_stream_stays_framed() {
    // rm-923: the transport caps one stdin message line at 1 MiB,
    // mirroring the statusline host's stdin stance. An over-cap line —
    // VALID JSON a host could otherwise process, as here (the
    // pristine binary answered it with a normal initialize result) —
    // is answered with a JSON-RPC error (id null: the discarded
    // message cannot be trusted to carry an id) and drained through
    // its newline, so the requests after it still answer and the
    // server exits 0.
    let (home, cache) = seed_home("oversize", false);
    let padding = "x".repeat(1024 * 1024 + 64);
    let requests = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"pad\":\"{padding}\"}}}}\n\
         {{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}}\n"
    );
    let (code, responses, _stderr) = mcp_session(&home, &cache, &requests);
    assert_eq!(Some(0), code, "the server survives an over-cap line");
    assert_eq!(2, responses.len(), "one response per line: {responses:?}");
    let refused = responses[0]
        .get("error")
        .unwrap_or_else(|| panic!("the over-cap line is refused, not answered: {responses:?}"));
    assert_eq!(refused["code"], -32600);
    assert!(
        refused["message"]
            .as_str()
            .expect("error message")
            .contains("transport cap"),
        "the refusal names the bound: {refused}"
    );
    assert_eq!(responses[1]["id"], 2);
    assert!(
        responses[1].get("result").is_some(),
        "the stream stays framed after the refusal: {responses:?}"
    );
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn protocol_version_is_negotiated_never_echoed() {
    // rm-780: `initialize` answers the newest revision BOTH sides
    // speak. A declared-in-set version is answered with itself; an
    // unknown or absent one falls back to the server's newest with a
    // one-line stderr disclosure (stdout stays protocol-only). The
    // pre-fix server echoed `"1999-99-99"` back verbatim, claiming
    // support for a version it does not speak (assess F5a PoC).
    let (home, cache) = seed_home("negotiate", false);
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"1999-99-99","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"initialize"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":4,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}"#,
        "\n",
    );
    let (code, responses, stderr) = mcp_session(&home, &cache, requests);
    assert_eq!(Some(0), code);
    assert_eq!(4, responses.len());
    let answered = |index: usize| responses[index]["result"]["protocolVersion"].as_str();
    assert_eq!(Some("2026-07-28"), answered(0), "newest supported answered");
    assert_eq!(
        Some("2026-07-28"),
        answered(1),
        "unknown versions fall back to the server's newest — never an echo"
    );
    assert_eq!(
        Some("2026-07-28"),
        answered(2),
        "absent versions fall back to the server's newest"
    );
    assert_eq!(
        Some("2025-11-25"),
        answered(3),
        "older mutual revision honored"
    );
    assert!(
        stderr.contains("1999-99-99") && stderr.contains("does not speak"),
        "the fallback discloses on stderr: {stderr}"
    );
    assert!(
        stderr.contains("<none>"),
        "the absent-version arm names its shape: {stderr}"
    );
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn hostile_wire_lines_answer_32700_and_stay_up() {
    // rm-782 (assess F2): one non-UTF-8 line used to kill the whole
    // server with rc1 and no response (`lines()` surfaced InvalidData
    // as a fatal error). The byte-level lane now answers -32700 and
    // keeps serving — the next request is answered normally.
    let (home, cache) = seed_home("nonutf8", false);
    let mut payload = Vec::new();
    payload.extend_from_slice(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
    payload.extend_from_slice(&[0xff, 0xfe, 0x00, b'b', b'a', b'd', b'\n']);
    payload.extend_from_slice(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n");
    let (code, responses, _stderr) = mcp_session_raw(&home, &cache, &["mcp"], &payload);
    assert_eq!(Some(0), code, "undecodable bytes are not a crash");
    assert_eq!(3, responses.len(), "every line is answered in order");
    assert!(responses[0]["result"].is_object());
    assert_eq!(
        Some(-32700),
        responses[1]["error"]["code"].as_i64(),
        "non-UTF-8 is wire garbage: parse error"
    );
    assert!(
        responses[1]["error"]["message"]
            .as_str()
            .expect("message")
            .contains("UTF-8"),
        "the refusal names the failure class"
    );
    assert_eq!(
        serde_json::json!(null),
        responses[1]["id"],
        "parse errors answer with id null"
    );
    assert!(
        responses[2]["result"].is_object(),
        "the server keeps serving after garbage"
    );
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn oversized_line_is_refused_and_stays_framed() {
    // rm-782 (memory-bounding arm of the byte-level reader) + landed
    // rm-923 (the refusal CONTRACT): a line past the 1 MiB cap is
    // drained without storing and refused with -32600 naming the
    // transport cap — the later landed rm-923 decision supersedes this
    // run's -32700 draft; the next line is framed and served
    // normally. The payload here is raw garbage bytes (no newline
    // until after the cap), complementing
    // `oversized_message_line_is_refused_and_the_stream_stays_framed`
    // above, whose over-cap line is VALID JSON.
    let (home, cache) = seed_home("oversized", false);
    let mut payload = Vec::new();
    payload.extend_from_slice(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
    payload.extend_from_slice(&vec![b'a'; (1 << 20) + 1]);
    payload.push(b'\n');
    payload.extend_from_slice(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n");
    let (code, responses, _stderr) = mcp_session_raw(&home, &cache, &["mcp"], &payload);
    assert_eq!(Some(0), code);
    assert_eq!(3, responses.len());
    assert!(responses[0]["result"].is_object());
    assert_eq!(
        Some(-32600),
        responses[1]["error"]["code"].as_i64(),
        "an over-cap line is refused, not buffered"
    );
    assert!(
        responses[1]["error"]["message"]
            .as_str()
            .expect("message")
            .contains("transport cap"),
        "the refusal names the cap (landed rm-923 contract): {}",
        responses[1]["error"]["message"]
    );
    assert_eq!(
        serde_json::json!(null),
        responses[1]["id"],
        "the discarded message cannot be trusted to carry an id"
    );
    assert!(
        responses[2]["result"].is_object(),
        "the stream stays framed after the drain"
    );
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn overlong_line_terminated_by_eof_is_refused_not_silently_dropped() {
    // rm-921 residual, pinned on the rm-782 byte-level reader: an
    // over-cap line with NO trailing newline — sized so the
    // cap-crossing accumulate is the stream's last act and clears the
    // buffer — must still be refused at EOF. `read_wire_line`'s EOF
    // arm consults `oversized` BEFORE `line.is_empty()`; testing
    // emptiness first (the pre-fix order this run's rider closed)
    // made the refusal indistinguishable from clean EOF: the host got
    // silence and exit 0 for a message it could not know was dropped.
    // The refusal keeps the landed rm-923 contract: -32600 naming the
    // transport cap.
    let (home, cache) = seed_home("overlong-eof", false);
    let mut sized = "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}".to_string();
    while sized.len() < 1024 * 1024 + 1 {
        sized.push('x');
    }
    sized.truncate(1024 * 1024 + 1);
    let (code, responses, _stderr) = mcp_session(&home, &cache, &sized);
    assert_eq!(
        Some(0),
        code,
        "EOF after an overlong line still exits clean: {_stderr}"
    );
    assert_eq!(
        1,
        responses.len(),
        "the EOF-terminated overlong line is refused, not swallowed: {responses:?}"
    );
    let refused = responses[0]
        .get("error")
        .unwrap_or_else(|| panic!("the refusal is a JSON-RPC error: {responses:?}"));
    assert_eq!(
        Some(-32600),
        refused["code"].as_i64(),
        "the landed rm-923 refusal contract: {refused}"
    );
    assert!(
        refused["message"]
            .as_str()
            .expect("error message")
            .contains("transport cap"),
        "the refusal names the bound: {refused}"
    );
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn invalid_request_shapes_answer_spec_errors() {
    // rm-782 (assess F5b/F6a): ids outside String|Number|Null answer
    // -32600 with id null instead of being echoed back; non-object
    // `params` and non-object `arguments` answer -32602 instead of
    // silently behaving like an absent value (which used to render a
    // full default report for a structurally bad call).
    let (home, cache) = seed_home("shapes", true);
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":[1,2],"method":"ping"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":5,"method":"ping","params":7}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"usage_overview","arguments":5}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#,
        "\n",
    );
    let (code, responses, _stderr) = mcp_session(&home, &cache, requests);
    assert_eq!(Some(0), code);
    assert_eq!(4, responses.len());
    assert_eq!(
        Some(-32600),
        responses[0]["error"]["code"].as_i64(),
        "array ids violate JSON-RPC 2.0"
    );
    assert_eq!(
        serde_json::json!(null),
        responses[0]["id"],
        "an undecodable id is answered with id null, never echoed"
    );
    assert_eq!(
        Some(-32602),
        responses[1]["error"]["code"].as_i64(),
        "non-object params are invalid params"
    );
    assert!(
        responses[1]["error"]["message"]
            .as_str()
            .expect("message")
            .contains("params"),
        "the message names the field"
    );
    assert_eq!(
        Some(-32602),
        responses[2]["error"]["code"].as_i64(),
        "non-object arguments are invalid params, not a default report"
    );
    assert!(
        responses[2]["error"]["message"]
            .as_str()
            .expect("message")
            .contains("arguments"),
        "the message names the field"
    );
    assert!(
        responses[3]["result"].is_object(),
        "the server keeps serving after every refusal"
    );
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn range_alias_schema_matches_the_enforced_parser() {
    // rm-782 (assess F6b): the published enum used to under-declare the
    // parser (`day`, `1d`, `week`, `weekly`, `month`, `monthly`, `""`
    // accepted but unpublished), so schema-validating hosts refused
    // calls the server itself would have answered. The schema now
    // publishes the enforced set, and every alias round-trips.
    let (home, cache) = seed_home("aliases", true);
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":"day"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":"weekly"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":""}}}"#,
        "\n",
    );
    let (code, responses, _stderr) = mcp_session(&home, &cache, requests);
    assert_eq!(Some(0), code);
    assert_eq!(4, responses.len());
    let enums: Vec<Value> = responses[0]["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["inputSchema"]["properties"]["range"]["enum"].clone())
        .collect();
    for published in enums {
        assert_eq!(
            serde_json::json!([
                "today", "day", "1d", "7d", "week", "weekly", "30d", "month", "monthly", "all", ""
            ]),
            published,
            "every tool publishes the full enforced set"
        );
    }
    for response in &responses[1..] {
        assert!(
            response.get("error").is_none(),
            "every published alias is accepted by the parser: {response}"
        );
    }
    // The empty string is the `all` default spelled out: it reports.
    assert_eq!(false, responses[3]["result"]["isError"]);
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn inapplicable_flags_before_the_keyword_are_refused() {
    // rm-781 (assess F3): flags placed BEFORE the `mcp` keyword used
    // to be accepted by clap and then silently ignored — the server
    // started as if nothing was asked. The lane now refuses every
    // flag that cannot apply, at rc2, naming the refused flag, the
    // honored set, and the help route.
    let (home, cache) = seed_home("flagrefusal", false);
    let (code, _stdout, stderr) = cli_run(&home, &cache, &["--overview", "mcp"]);
    assert_eq!(
        Some(2),
        code,
        "a report action cannot apply to a stdio server: rc2, not silence"
    );
    assert!(
        stderr.contains("--overview") && stderr.contains("cannot apply to the MCP"),
        "the refusal names the flag and the reason: {stderr}"
    );
    assert!(
        stderr.contains("agenttrace mcp --help"),
        "the refusal routes to the keyword help: {stderr}"
    );

    let (code, _stdout, stderr) = cli_run(&home, &cache, &["-f", "json", "mcp"]);
    assert_eq!(Some(2), code);
    assert!(
        stderr.contains("--format/-f"),
        "value flags are refused by their spelled name: {stderr}"
    );

    // The honored family sails through the refusal arm — it reaches
    // the server (which then exits rc0 on stdin EOF).
    let (code, _stdout, _stderr) =
        mcp_session_raw(&home, &cache, &["--pricing-file", "/dev/null", "mcp"], b"");
    assert_eq!(
        Some(0),
        code,
        "the config family before the keyword is honored, not refused"
    );
    let _ = std::fs::remove_dir_all(home.parent().expect("root"));
}

#[test]
fn dir_flag_scopes_the_server_corpus() {
    // rm-781 (assess F3): `-d` before the keyword used to be parsed
    // and silently dropped, so an explicitly chosen corpus answered
    // the auto-discovery remedy. It is now honored — and admitted
    // with the same rc2 check the report lane runs.
    let (seeded, _seeded_cache) = seed_home("dirflag-seeded", true);
    let (empty_home, cache) = seed_home("dirflag-empty", false);
    let corpus = seeded.join(".hermes").join("sessions");
    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":"all"}}}"#,
        "\n",
    );
    let (code, responses, _stderr) = mcp_session_raw(
        &empty_home,
        &cache,
        &["-d", corpus.to_str().expect("utf-8 path"), "mcp"],
        requests.as_bytes(),
    );
    assert_eq!(Some(0), code);
    assert_eq!(1, responses.len());
    assert_eq!(
        false, responses[0]["result"]["isError"],
        "the `-d` corpus must be reported even though HOME is empty: {}",
        responses[0]["result"]["content"][0]["text"]
    );
    let payload = text_payload(&responses[0]);
    assert!(
        payload["by_model"]
            .as_array()
            .expect("by_model")
            .iter()
            .any(|row| row["name"] == "gpt-5"),
        "the seeded corpus must roll up: {payload}"
    );

    // Admission parity with the report lane: a missing `-d` target is
    // a usage error (rc2), not an empty corpus.
    let missing = empty_home.join("does-not-exist");
    let (code, _stdout, stderr) = cli_run(
        &empty_home,
        &cache,
        &["-d", missing.to_str().expect("utf-8 path"), "mcp"],
    );
    assert_eq!(Some(2), code);
    assert!(
        stderr.contains("session directory does not exist"),
        "the admission check names the failure: {stderr}"
    );
    let _ = std::fs::remove_dir_all(seeded.parent().expect("root"));
    let _ = std::fs::remove_dir_all(empty_home.parent().expect("root"));
}

#[test]
fn config_layer_prices_the_mcp_tools_identically_to_the_cli() {
    // rm-781 (assess F1): the mcp dispatch used to run BEFORE the
    // layered configuration was resolved, so a `--config` pricing
    // override moved the CLI's answer and left the MCP tools on
    // snapshot pricing under the SAME override (PoC: CLI 0.003 vs MCP
    // 0.0). Both lanes now resolve the same config: one truth, one
    // number. The control arm pins the divergence the PoC observed.
    let scratch = std::env::temp_dir().join(format!(
        "agenttrace-mcp-configparity-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    let pricing = scratch.join("pricing.json");
    std::fs::write(
        &pricing,
        r#"{"prices": {"gpt-5": {"input": 1000.0, "output": 1000.0, "cw": 1000.0, "cr": 1000.0}}, "aliases": {}}"#,
    )
    .expect("write pricing override");
    let config = scratch.join("config.toml");
    std::fs::write(
        &config,
        format!("pricing_file = \"{}\"\n", pricing.display()),
    )
    .expect("write config layer");

    let overview_call = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"usage_overview","arguments":{"range":"all"}}}"#,
        "\n",
    );

    // Priced arm: the explicit config layer reaches the MCP tools.
    let (home_priced, cache_priced) = seed_home("configparity-priced", true);
    let (code, responses, _stderr) = mcp_session_raw(
        &home_priced,
        &cache_priced,
        &["--config", config.to_str().expect("utf-8 path"), "mcp"],
        overview_call.as_bytes(),
    );
    assert_eq!(Some(0), code);
    assert_eq!(1, responses.len());
    assert_eq!(false, responses[0]["result"]["isError"]);
    let mcp_cost = text_payload(&responses[0])["summary"]["total_cost"]
        .as_f64()
        .expect("total_cost is a number");
    assert!(
        (0.002..=0.004).contains(&mcp_cost),
        "the override must price the MCP answer (expected ~0.003): {mcp_cost}"
    );

    // CLI parity arm: same config, same home shape — same number.
    let (home_cli, cache_cli) = seed_home("configparity-cli", true);
    let (code, stdout, _stderr) = cli_run(
        &home_cli,
        &cache_cli,
        &[
            "--config",
            config.to_str().expect("utf-8 path"),
            "--overview",
            "-f",
            "json",
        ],
    );
    assert_eq!(Some(0), code, "CLI priced arm must succeed");
    let cli_cost = serde_json::from_str::<Value>(&stdout).expect("overview json")["summary"]
        ["total_cost"]
        .as_f64()
        .expect("total_cost is a number");
    assert_eq!(
        mcp_cost, cli_cost,
        "MCP and CLI must report the same total under the same override"
    );

    // Control arm: no config layer, snapshot pricing — the two lanes
    // are still identical, and both far from the override's price.
    let (home_plain, cache_plain) = seed_home("configparity-plain", true);
    let (code, responses, _stderr) = mcp_session_raw(
        &home_plain,
        &cache_plain,
        &["mcp"],
        overview_call.as_bytes(),
    );
    assert_eq!(Some(0), code);
    let plain_cost = text_payload(&responses[0])["summary"]["total_cost"]
        .as_f64()
        .expect("total_cost");
    assert_eq!(
        0.0, plain_cost,
        "without the override the snapshot prices 3 tokens at zero"
    );

    let _ = std::fs::remove_dir_all(home_priced.parent().expect("root"));
    let _ = std::fs::remove_dir_all(home_cli.parent().expect("root"));
    let _ = std::fs::remove_dir_all(home_plain.parent().expect("root"));
    let _ = std::fs::remove_dir_all(&scratch);
}

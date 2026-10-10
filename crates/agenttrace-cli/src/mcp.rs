//! Local read-only MCP server (`agenttrace mcp`, rm-455).
//!
//! A coding agent that hosts tools through the Model Context Protocol
//! asks "where did my tokens go" from inside the same terminal that
//! burned them. This module answers that question by speaking
//! newline-delimited JSON-RPC 2.0 over stdio: one JSON message per
//! line in, one JSON message per line out, exit 0 on stdin EOF.
//!
//! Posture (the local-truth contract, pinned by
//! scripts/ci/check-docs-commands.sh and documented in
//! docs/guides/mcp-server.md):
//!
//! * read-only — the tools render the SAME locally discovered sessions
//!   the CLI reports on, via [`agenttrace_core::compute_overview`] and
//!   [`agenttrace_core::report_overview_json_with_context`]. There is
//!   no second renderer and no parallel pricing path.
//! * zero network — the server opens no sockets and performs no
//!   registry or HTTP probes. Stdio is the only transport.
//! * no writes outside artifact roots — the only write is the session
//!   cache the CLI itself maintains
//!   (`AGENTTRACE_SESSION_CACHE_DIR`), which loading may refresh.
//! * notifications are never answered; unknown methods answer
//!   `-32601`; malformed JSON answers `-32700`; invalid tool params
//!   answer `-32602`. A discovered-but-empty corpus is a tool-level
//!   error result (`isError: true`), mirroring the CLI's non-zero
//!   exit for the same situation — the server itself stays up.
//!
//! Dependency decision (recorded here per the rm-455 lane): the
//! JSON-RPC layer is hand-rolled over `serde_json`. The CLI carries no
//! MCP SDK dependency today and the wire surface needed here is four
//! methods of a text protocol; an SDK (with its transitive tree and
//! licensing review) would be its own change-unit if ever wanted.

use std::io::{self, BufRead, Write};

use agenttrace_core::{
    compute_overview, data_health_scoped, load_sessions_with_options,
    report_overview_json_with_context, LoadOptions, LoadReport, TimeRange,
};
use chrono::Utc;
use serde_json::{json, Map, Value};

/// JSON-RPC 2.0 error codes used by this server.
const CODE_PARSE_ERROR: i64 = -32700;
const CODE_INVALID_REQUEST: i64 = -32600;
const CODE_METHOD_NOT_FOUND: i64 = -32601;
const CODE_INVALID_PARAMS: i64 = -32602;

/// Protocol version answered when the client does not send one.
const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

/// Overview `recent_sessions` display limit; mirrors the CLI default
/// and `RECENT_SESSIONS_MAX` in `agenttrace-core` (nothing here may
/// grow past that cap).
const RECENT_SESSIONS_LIMIT: usize = 10;

/// Entry point for the `agenttrace mcp` host command.
///
/// Reads newline-delimited JSON-RPC from stdin until EOF and writes
/// one response per request line to stdout, flushing after every
/// message (hosts read incrementally). A broken stdout pipe means the
/// host went away — that is a clean shutdown, not a failure, so it
/// exits 0 the way the statusline host command treats host-side
/// disconnects.
pub fn serve() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    loop {
        let line = match read_message_line(&mut stdin.lock()) {
            Ok(MessageLine::Eof) => return Ok(()),
            Ok(MessageLine::Line(line)) => line,
            Ok(MessageLine::Overlong) => {
                // rm-923: the oversized line is already drained through
                // its newline, so the stream stays framed — the requests
                // after it still answer. The refusal is disclosed on
                // the wire (id `null`: the discarded message could not
                // be trusted to carry an id) instead of buffering
                // host-chosen bytes.
                let refused = error_response(
                    None,
                    CODE_INVALID_REQUEST,
                    "Invalid Request: message line exceeds the 1048576-byte transport cap and was discarded",
                );
                if let Err(error) = writeln!(stdout, "{refused}").and_then(|_| stdout.flush()) {
                    return if error.kind() == io::ErrorKind::BrokenPipe {
                        Ok(())
                    } else {
                        Err(error.into())
                    };
                }
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_message(line.trim()) {
            if let Err(error) = writeln!(stdout, "{response}").and_then(|_| stdout.flush()) {
                return if error.kind() == io::ErrorKind::BrokenPipe {
                    Ok(())
                } else {
                    Err(error.into())
                };
            }
        }
    }
}

/// rm-923: hard cap on one stdin message line, mirroring the
/// statusline host's `STATUSLINE_INPUT_MAX_BYTES` stance (the same
/// host, the same peer — the transport used to buffer a single
/// arbitrarily long line unboundedly; `BufRead::lines` has no bound
/// of its own). A line that crosses the cap is drained through its
/// terminating newline and reported [`MessageLine::Overlong`].
const MCP_INPUT_MAX_BYTES: usize = 1024 * 1024;

/// One framed stdin read for [`serve`]: a complete line, a refused
/// overlong line (already drained through its newline), or EOF.
enum MessageLine {
    Line(String),
    Overlong,
    Eof,
}

/// Reads one newline-terminated message with a hard byte cap.
/// `BufRead::lines` semantics are preserved exactly: UTF-8 is
/// required (errors surface as `io::ErrorKind::InvalidData`), and a
/// trailing `\r` is dropped only when the line was
/// newline-terminated — including the final line of a stream, which
/// may lack its newline and still yields.
fn read_message_line(reader: &mut impl BufRead) -> io::Result<MessageLine> {
    let mut line = Vec::new();
    let mut overlong = false;
    loop {
        let available = match reader.fill_buf() {
            Ok(available) => available,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if available.is_empty() {
            return Ok(if line.is_empty() {
                MessageLine::Eof
            } else if overlong {
                MessageLine::Overlong
            } else {
                MessageLine::Line(decode_message_line(&line, false)?)
            });
        }
        match available.iter().position(|&byte| byte == b'\n') {
            Some(position) => {
                line.extend_from_slice(&available[..position]);
                reader.consume(position + 1);
                return Ok(if overlong || line.len() > MCP_INPUT_MAX_BYTES {
                    MessageLine::Overlong
                } else {
                    MessageLine::Line(decode_message_line(&line, true)?)
                });
            }
            None => {
                line.extend_from_slice(available);
                let consumed = available.len();
                reader.consume(consumed);
                if line.len() > MCP_INPUT_MAX_BYTES {
                    overlong = true;
                    line.clear(); // the tail is discarded as it arrives
                }
            }
        }
    }
}

/// `BufRead::lines` decoding for the collected bytes of one message
/// line: invalid UTF-8 is an `InvalidData` error (as `lines` reported
/// it), and the `\r` of a CRLF pair goes only with its `\n`.
fn decode_message_line(bytes: &[u8], newline_terminated: bool) -> io::Result<String> {
    let mut text = String::from_utf8(bytes.to_vec()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })?;
    if newline_terminated && text.ends_with('\r') {
        text.pop();
    }
    Ok(text)
}

/// Handle one wire message. `None` means "no reply" (a notification);
/// `Some(json)` is the exact line to write back.
fn handle_message(raw: &str) -> Option<String> {
    let message = match serde_json::from_str::<Value>(raw) {
        Ok(message) => message,
        Err(_) => return Some(error_response(None, CODE_PARSE_ERROR, "Parse error")),
    };
    // MCP (2025-06-18) drops JSON-RPC batching; answer arrays loudly
    // instead of misparsing them as a single request.
    if message.is_array() {
        return Some(error_response(
            None,
            CODE_INVALID_REQUEST,
            "Invalid Request: batch requests are not supported",
        ));
    }
    let Some(object) = message.as_object() else {
        return Some(error_response(
            None,
            CODE_INVALID_REQUEST,
            "Invalid Request: expected a JSON-RPC 2.0 object",
        ));
    };
    let id = object.get("id").cloned();
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        // No method: a request is invalid, a notification is ignorable.
        // (`id.as_ref()?` silences a notification here without moving it.)
        id.as_ref()?;
        return Some(error_response(
            id,
            CODE_INVALID_REQUEST,
            "Invalid Request: missing method",
        ));
    };
    // Notifications (no id) are acknowledged by staying silent — that
    // includes `notifications/initialized`, which simply transitions
    // the handshake.
    let id = id?;
    let params = object.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => initialize_result(params),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => call_tool(params),
        _ => {
            return Some(error_response(
                Some(id),
                CODE_METHOD_NOT_FOUND,
                "Method not found",
            ));
        }
    };
    Some(match result {
        Ok(value) => json!({"jsonrpc": "2.0", "id": id, "result": value}).to_string(),
        Err((code, message)) => error_response(Some(id), code, &message),
    })
}

/// `initialize` result: echo the client's protocol version when sent,
/// and state the read-only, stdio-only capability surface.
fn initialize_result(params: Value) -> Result<Value, (i64, String)> {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);
    Ok(json!({
        "protocolVersion": requested,
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "serverInfo": {
            "name": "agenttrace",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Read-only usage reporting over locally discovered \
    agent sessions. Tools: usage_overview (the CLI --overview JSON), \
    by_model_breakdown (cost and sessions per model). The server is offline \
    and stdio-only; it writes nothing outside the agenttrace session cache."
    }))
}

/// `tools/list` result. Descriptions state the read-only posture so a
/// host that surfaces them to the model carries the contract too.
fn tools_list_result() -> Value {
    let range_schema = || {
        json!({
            "type": "object",
            "properties": {
                "range": {
                    "type": "string",
                    "enum": ["today", "7d", "30d", "all"],
                    "description": "Report window. `today` is your local \
        calendar day; `7d`/`30d` are rolling windows; `all` is everything \
        discovered. Defaults to `all`."
                }
            },
            "additionalProperties": false
        })
    };
    json!({
        "tools": [
            {
                "name": "usage_overview",
                "description": "Full usage overview for locally discovered \
    agent sessions — the same JSON the `agenttrace --overview -f json` CLI \
    action renders (summary, by_model, by_provider, by_task_type, top cost \
    drivers, data health). Read-only; no network.",
                "inputSchema": range_schema()
            },
            {
                "name": "by_model_breakdown",
                "description": "Cost and session counts per model for the \
    window — the `by_model` rollup of the same locally discovered sessions. \
    Read-only; no network.",
                "inputSchema": range_schema()
            }
        ]
    })
}

/// Tool execution failure: the server answered the request, the corpus
/// could not produce a report. Rendered as an `isError` result, not a
/// protocol error — the host stays connected.
fn tool_execution_error(message: String) -> Result<Value, (i64, String)> {
    Ok(json!({
        "content": [{ "type": "text", "text": message }],
        "isError": true
    }))
}

/// `tools/call` dispatch with the CLI's parameter discipline:
/// structurally bad params are `-32602` (the JSON-RPC twin of the
/// CLI's rc2 usage error), corpus-level failures are `isError`
/// results (the twin of the CLI's rc1 bail).
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

fn call_tool(params: Value) -> Result<Value, (i64, String)> {
    let invalid = |message: String| (CODE_INVALID_PARAMS, message);
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err(invalid(
            "Invalid params: `name` must name a tool".to_string(),
        ));
    };
    // rm-840 (renumbered from campaign-local rm-800 at compound 387529f7): `arguments` must be an object when present. Every tool
    // takes an optional object; a string/number/bool/array value used
    // to fall through `unwrap_or(json!({}))` and silently execute the
    // tool with default arguments — a client bug became an unintended
    // run (the assess pipe PoC sent `"garbage-not-object"` and got
    // default-range execution). Absent (and JSON null — this file's
    // own convention for optional values, `parse_range_argument`
    // treats `"range": null` the same way) keeps the spec-legal
    // empty-object default.
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => json!({}),
        Some(value) if value.is_object() => value.clone(),
        Some(other) => {
            return Err(invalid(format!(
                "Invalid params: `arguments` for tool `{name}` must be an object \
                 when present, got {}",
                json_type_name(other)
            )));
        }
    };
    let range = match parse_range_argument(&arguments) {
        Ok(range) => range,
        Err(message) => return Err(invalid(message)),
    };
    match name {
        "usage_overview" => usage_overview(range),
        "by_model_breakdown" => by_model_breakdown(range),
        other => Err(invalid(format!(
            "Invalid params: unknown tool `{other}` (available: usage_overview, \
by_model_breakdown)"
        ))),
    }
}

/// Parse the shared `range` argument. Absent means `all` (the CLI's
/// default reporting window); anything else must be one of the four
/// canonical labels, mirroring `--range` parsing in the CLI.
fn parse_range_argument(arguments: &Value) -> Result<TimeRange, String> {
    match arguments.get("range") {
        None | Some(Value::Null) => Ok(TimeRange::All),
        Some(Value::String(raw)) => TimeRange::parse(raw).ok_or_else(|| {
            format!(
                "Invalid params: `range` must be one of today, 7d, 30d, all \
(got `{raw}`)"
            )
        }),
        Some(_) => {
            Err("Invalid params: `range` must be a string (today, 7d, 30d, all)".to_string())
        }
    }
}

/// Load the locally discovered sessions for a window, exactly the way
/// the CLI's default (no `-d`) report actions do: `HOME`-rooted known
/// agent homes, `since` filter, and the session cache.
fn load_for_range(range: TimeRange) -> LoadReport {
    let since = range.since(Utc::now());
    load_sessions_with_options(
        None,
        &LoadOptions {
            since,
            ..Default::default()
        },
    )
}

fn empty_corpus_error(range: TimeRange) -> Result<Value, (i64, String)> {
    tool_execution_error(format!(
        "No session files found in any auto-discovered agent home for range \
{} — point HOME at the machine whose agents you want to report on, or see \
`agenttrace --help` for discovery roots.",
        range.label()
    ))
}

/// `usage_overview`: render through the CLI's own report path so the
/// MCP answer and `agenttrace --overview -f json` stay one truth.
fn usage_overview(range: TimeRange) -> Result<Value, (i64, String)> {
    let report = load_for_range(range);
    if report.sessions.is_empty() {
        return empty_corpus_error(range);
    }
    let overview = compute_overview(&report.sessions);
    // Truthful coverage accounting straight from the loader's census
    // (pass-8 F8-2), identical to the CLI overview arm — including the
    // rm-548 fork-exclusion count and the rm-734 unreadable-sqlite
    // records so MCP and CLI disclosures agree.
    let health = data_health_scoped(
        &report.sessions,
        report.discovered,
        report.skipped,
        report.cache_hits,
        report.opencode_fork_excluded,
        report.sqlite.unreadable.clone(),
    );
    let text = report_overview_json_with_context(
        &overview,
        &report.sessions,
        Some(&health),
        range,
        false,
        None,
        RECENT_SESSIONS_LIMIT,
    );
    Ok(tool_text_result(text))
}

/// `by_model_breakdown`: the `by_model` rollup of the same overview
/// computation — same discovery, same pricing, smaller answer.
fn by_model_breakdown(range: TimeRange) -> Result<Value, (i64, String)> {
    let report = load_for_range(range);
    if report.sessions.is_empty() {
        return empty_corpus_error(range);
    }
    let overview = compute_overview(&report.sessions);
    let mut models: Vec<Value> = overview
        .by_model
        .iter()
        .map(|(name, group)| {
            json!({
                "name": name,
                "sessions": group.sessions,
                "cost": round4(group.cost),
            })
        })
        .collect();
    // Cost-descending, name-tie-broken — the ordering the overview's
    // own `by_model` section uses.
    models.sort_by(|a, b| {
        let cost = |value: &Value| value["cost"].as_f64().unwrap_or(0.0);
        cost(b)
            .partial_cmp(&cost(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let name = |value: &Value| value["name"].as_str().unwrap_or("").to_string();
                name(a).cmp(&name(b))
            })
    });
    let body = json!({
        "range": range.label(),
        "total_sessions": overview.total_sessions,
        "total_cost": round4(overview.total_cost),
        "by_model": models,
    });
    Ok(tool_text_result(body.to_string()))
}

/// Wrap rendered report text as MCP tool content. The payload is the
/// CLI's JSON document verbatim — text content keeps every host able
/// to display it, and machine consumers parse the text.
fn tool_text_result(text: String) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false
    })
}

/// Match `agenttrace-core`'s report rounding so numbers shown over MCP
/// equal numbers shown by the CLI.
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// Serialize a JSON-RPC error response. `id: null` per the spec when
/// the failing message could not be identified (parse errors).
fn error_response(id: Option<Value>, code: i64, message: &str) -> String {
    let mut object = Map::new();
    object.insert("jsonrpc".to_string(), json!("2.0"));
    object.insert("id".to_string(), id.unwrap_or(Value::Null));
    object.insert(
        "error".to_string(),
        json!({ "code": code, "message": message }),
    );
    Value::Object(object).to_string()
}

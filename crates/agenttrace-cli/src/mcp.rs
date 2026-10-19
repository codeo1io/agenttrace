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
//!   no second renderer and no parallel pricing path: the dispatch
//!   site in `main.rs` resolves the CLI's layered configuration
//!   (explicit `--config`, project and user `config.toml`, the
//!   `--history-dir`/`--pricing-file` flags) and installs the runtime
//!   config BEFORE `serve()` runs, so file-layer pricing and history
//!   knobs reach the MCP answers exactly as they reach the CLI
//!   (rm-781); the corpus root `-d/--dir` is honored the same way.
//! * zero network — the server opens no sockets and performs no
//!   registry or HTTP probes. Stdio is the only transport.
//! * no writes outside artifact roots — the only write is the session
//!   cache the CLI itself maintains
//!   (`AGENTTRACE_SESSION_CACHE_DIR`), which loading may refresh.
//! * protocol currency (rm-780) — the server speaks the MCP revisions
//!   in [`SUPPORTED_PROTOCOL_VERSIONS`], newest first. `initialize`
//!   answers the newest revision the client also declared, never a
//!   client-supplied echo; an unknown or absent client version falls
//!   back to the newest supported revision with a one-line stderr
//!   disclosure (stdout stays protocol-only).
//! * stateless discovery (rm-1960) — the server is dual-era per the
//!   2026-07-28 revision: a request carrying a modern per-request
//!   protocol version in `_meta` (the revision's reserved key
//!   `io.modelcontextprotocol/protocolVersion`; the bare
//!   `protocolVersion` spelling is honored as a lenient alias) selects
//!   stateless semantics — an unsupported version answers the spec's
//!   `UnsupportedProtocolVersionError` `-32022` with
//!   `data: {supported, requested}` — while `initialize` stays for legacy
//!   clients, and `server/discover` — the revision's one mandatory
//!   server RPC, previously absent — answers `supportedVersions`,
//!   `capabilities`, and `_meta['io.modelcontextprotocol/serverInfo']`
//!   with no handshake at all. `ping` keeps answering for legacy
//!   clients (the revision removed it; answering anyway is harmless
//!   back-compat). `subscriptions/listen` (and the
//!   `resources/subscribe` lineage it replaces) is refused with a
//!   named `-32601`: this server is read-only and implements no
//!   subscriptions. The id echo is byte-exact: an id that overflows
//!   serde_json's number lane (999999999999999999999999 → f64 1e24)
//!   is spliced back from the raw input token, and an id-less line
//!   that is not notification-shaped (`{}`) answers `-32600` with
//!   id null instead of silence.
//! * wire discipline (rm-782 + landed rm-923) — notifications are
//!   never answered; unknown methods answer `-32601`; malformed
//!   input answers `-32700` and the server stays up, including a
//!   line that is not valid UTF-8 (the stdin lane reads bytes, not
//!   `lines()` — one binary frame used to be a fatal rc1 with no
//!   response at all); a line that exceeds
//!   [`MCP_INPUT_MAX_BYTES`] is drained, refused with `-32600` and
//!   the exact transport-cap message the landed rm-923 tests,
//!   guide, and CHANGELOG pin, and the server stays up (the stream
//!   stays framed: the requests after the discarded line still
//!   answer); request ids are validated to the JSON-RPC 2.0
//!   String|Number|Null set — anything else answers `-32600` with id
//!   null; the `jsonrpc` member must be exactly the string `"2.0"`
//!   (absent, misspelled, or non-string members answer `-32600`
//!   echoing the request id — rm-998, the member lane of the
//!   rm-780/rm-782/rm-998 validation triad; notification-shaped
//!   lines keep the stays-silent posture); non-object
//!   `params`/`arguments` answer `-32602`. A
//!   discovered-but-empty corpus is a tool-level error result
//!   (`isError: true`), mirroring the CLI's non-zero exit for the
//!   same situation — and a discovered corpus whose sessions all
//!   fall outside the requested window reports the CLI's filter-miss
//!   message, not a discovery miss (rm-781). The server itself stays
//!   up in every one of those cases.
//! * output parity (rm-1918) — every response line, success or error,
//!   crosses the CLI's JSON-lane spoofing escape
//!   ([`escape_spoofing_json`]) at the serialization boundary: no raw
//!   directional/invisible Cf control (U+202E-class) ever rides the
//!   stdio wire, while tool text payloads are escaped BEFORE wrapping
//!   so they stay byte-identical to the CLI's `-f json` documents on
//!   the same corpus (the escape is lossless and a no-op on clean
//!   text).
//!
//! Dependency decision (recorded here per the rm-455 lane): the
//! JSON-RPC layer is hand-rolled over `serde_json`. The CLI carries no
//! MCP SDK dependency today and the wire surface needed here is four
//! methods of a text protocol; an SDK (with its transitive tree and
//! licensing review) would be its own change-unit if ever wanted.

use std::io::{self, BufRead, Write};
use std::path::Path;

use agenttrace_core::{
    compute_overview, data_health_scoped, escape_spoofing_json, load_sessions_with_options,
    report_overview_json_with_context, LoadOptions, LoadReport, TimeRange,
};
use chrono::Utc;
use serde_json::{json, Map, Value};

/// JSON-RPC 2.0 error codes used by this server.
const CODE_PARSE_ERROR: i64 = -32700;
const CODE_INVALID_REQUEST: i64 = -32600;
const CODE_METHOD_NOT_FOUND: i64 = -32601;
// rm-1960: `UnsupportedProtocolVersionError`, the spec-named modern
// error of the 2026-07-28 revision (`data: {supported, requested}`) —
// the recognized error stdio clients probe `server/discover` for.
const CODE_UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;
const CODE_INVALID_PARAMS: i64 = -32602;

/// rm-1960: the `_meta` key the 2026-07-28 revision reserves for the
/// per-request protocol version (basic/index reserved-keys table; the
/// whole `io.modelcontextprotocol/` prefix is spec-reserved). The
/// revision's own prose is explicit that "every request" carries its
/// version here — NOT in a bare `protocolVersion` member, which is a
/// client-chosen, non-reserved key. See [`requested_meta_version`]
/// for the alias policy this server pairs with it.
const META_PROTOCOL_VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";

/// MCP protocol revisions this server speaks, NEWEST FIRST — the
/// order is the negotiation preference (rm-780). `initialize` answers
/// the first entry the client also declared, or falls back to `[0]`
/// for unknown/absent client versions. The set mirrors the official
/// revision ladder (modelcontextprotocol repository tags, fetched
/// live 2026-10-08: 2026-07-28 > 2025-11-25 > 2025-06-18); adding a
/// revision here is a deliberate, tested act — the cost the rm-455
/// rmcp-adoption rider tracks.
const SUPPORTED_PROTOCOL_VERSIONS: [&str; 3] = ["2026-07-28", "2025-11-25", "2025-06-18"];

/// Per-line stdin cap. rm-782 first framed the read at byte level
/// (`BufRead::lines()` has no bound of its own and cannot answer a
/// non-UTF-8 line at all); landed rm-923 pinned the refusal contract
/// this constant name documents (mirroring the statusline host's
/// `STATUSLINE_INPUT_MAX_BYTES` stance — the same host, the same
/// peer). The largest legitimate message on this server is an
/// `initialize` handshake — a few KiB; MCP hosts do not stream
/// payloads through stdio lines. 1 MiB is ~1000x that headroom while
/// bounding the memory a hostile or buggy host can make the server
/// hold for one unterminated line; over-cap lines are drained (not
/// stored), refused with `-32600`, and the server keeps serving.
/// Value chosen at implement time and recorded here per the lanes
/// above: raise it only with a reason.
const MCP_INPUT_MAX_BYTES: usize = 1024 * 1024; // 1 MiB

/// rm-1025: the oversized-line refusal text is DERIVED from
/// [`MCP_INPUT_MAX_BYTES`], not hand-written — a cap change must never
/// leave the user-facing message stating a stale numeral.
fn oversized_line_message() -> String {
    format!(
        "Invalid Request: message line exceeds the {}-byte transport cap and was discarded",
        MCP_INPUT_MAX_BYTES
    )
}

/// The `range` labels [`TimeRange::parse`] actually enforces
/// (agenttrace-core insights.rs) — the tools' `inputSchema` enum and
/// the `-32602` message list this exact set, locked by the
/// schema-equivalence test (rm-782: the old four-label enum
/// under-declared the parser, so schema-validating hosts refused
/// calls the server itself would have accepted).
const RANGE_ALIASES: [&str; 11] = [
    "today", "day", "1d", "7d", "week", "weekly", "30d", "month", "monthly", "all", "",
];

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
///
/// `root` is the corpus the caller resolved: `Some(dir)` mirrors the
/// CLI's `-d/--dir` (honored by the dispatch site since rm-781),
/// `None` keeps auto-discovery over the known agent homes.
pub fn serve(root: Option<&Path>) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout().lock();
    loop {
        let bytes = match read_wire_line(&mut reader) {
            Ok(WireLine::Eof) => return Ok(()),
            Ok(WireLine::Bytes(bytes)) => bytes,
            Ok(WireLine::Oversized) => {
                // rm-782 framed this read (the over-cap line is
                // already drained through its newline, so the stream
                // stays framed and the requests after it still
                // answer); landed rm-923 owns the refusal CONTRACT
                // this arm keeps verbatim — `-32600` with the exact
                // transport-cap message pinned by
                // `oversized_message_line_is_refused_and_the_stream_
                // stays_framed`, the guide, and the CHANGELOG — a
                // later landed decision than this run's `-32700`
                // draft, so the integration keeps the landed wire
                // answer. The refusal is disclosed on the wire (id
                // `null`: the discarded message could not be trusted
                // to carry an id) instead of buffering host-chosen
                // bytes.
                emit(
                    &mut stdout,
                    &error_response(None, CODE_INVALID_REQUEST, &oversized_line_message()),
                )?;
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let line = match String::from_utf8(bytes) {
            Ok(line) => line,
            Err(_) => {
                // rm-782: undecodable bytes are wire garbage, not a
                // reason to die — answer -32700 and keep serving.
                // (`lines()` used to surface InvalidData as a fatal
                // error: one binary frame took the whole server down
                // with rc1 and no response, contradicting the
                // documented stays-up posture.)
                emit(
                    &mut stdout,
                    &error_response(
                        None,
                        CODE_PARSE_ERROR,
                        "Parse error: request line is not valid UTF-8",
                    ),
                )?;
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_message(line.trim(), root) {
            emit(&mut stdout, &response)?;
        }
    }
}

/// Write one response line and flush. A broken pipe is the host going
/// away — clean shutdown, exit-0 semantics, not a failure.
fn emit(stdout: &mut impl Write, response: &str) -> anyhow::Result<()> {
    if let Err(error) = writeln!(stdout, "{response}").and_then(|_| stdout.flush()) {
        return if error.kind() == io::ErrorKind::BrokenPipe {
            Ok(())
        } else {
            Err(error.into())
        };
    }
    Ok(())
}

/// One decoded read from the stdin lane.
enum WireLine {
    /// Clean EOF — the host closed stdin.
    Eof,
    /// A complete line (its newline consumed and excluded; a final
    /// line without a trailing newline is still a line).
    Bytes(Vec<u8>),
    /// A line that exceeded [`MCP_INPUT_MAX_BYTES`] — already drained to
    /// its newline (or EOF) without storing; the caller answers a
    /// protocol error.
    Oversized,
}

/// Byte-level line reader (rm-782). `BufRead::lines()` cannot answer
/// a non-UTF-8 line — it surfaces `InvalidData` and the old loop let
/// that escape as a fatal error. Reading raw bytes keeps every
/// refusal on the wire (decode failures become `-32700` in
/// [`serve`]), and the incremental fill/consume loop bounds memory to
/// [`MCP_INPUT_MAX_BYTES`] per line instead of buffering an
/// unterminated line of arbitrary length.
fn read_wire_line(reader: &mut impl BufRead) -> io::Result<WireLine> {
    let mut line: Vec<u8> = Vec::new();
    let mut oversized = false;
    loop {
        let (complete, used) = {
            let available = match reader.fill_buf() {
                Ok(available) => available,
                Err(ref error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            if available.is_empty() {
                // EOF: the host closed stdin. A pending final line
                // without a trailing newline is still a line; a pending
                // oversized drain is still oversized (rm-921: the
                // oversized flag must be consulted BEFORE emptiness or
                // a cap-crossing unterminated final line reads as clean
                // EOF and the host gets silence for a message it could
                // not know was dropped — pinned by the EOF-edge test in
                // tests/mcp_server.rs).
                return Ok(if oversized {
                    WireLine::Oversized
                } else if line.is_empty() {
                    WireLine::Eof
                } else {
                    WireLine::Bytes(line)
                });
            }
            match available.iter().position(|&byte| byte == b'\n') {
                Some(position) => {
                    if !oversized {
                        line.extend_from_slice(&available[..position]);
                        if line.len() > MCP_INPUT_MAX_BYTES {
                            // the COMPLETED line crossed the cap
                            // inside this chunk (the check below only
                            // fires between chunks), so cap it here
                            // too — a line exactly MAX+1 bytes used
                            // to slip through as ordinary input.
                            oversized = true;
                            line = Vec::new();
                        }
                    }
                    (true, position + 1)
                }
                None => {
                    if !oversized {
                        line.extend_from_slice(available);
                        if line.len() > MCP_INPUT_MAX_BYTES {
                            // Stop storing, keep draining: the stream
                            // must stay framed on the next newline.
                            oversized = true;
                            line = Vec::new();
                        }
                    }
                    (false, available.len())
                }
            }
        };
        reader.consume(used);
        if complete {
            return Ok(if oversized {
                WireLine::Oversized
            } else {
                WireLine::Bytes(line)
            });
        }
    }
}

/// Handle one wire message. `None` means "no reply" (a notification);
/// `Some(json)` is the exact line to write back. `root` is the corpus
/// root the dispatch site resolved (rm-781).
fn handle_message(raw: &str, root: Option<&Path>) -> Option<String> {
    let message = match serde_json::from_str::<Value>(raw) {
        Ok(message) => message,
        Err(_) => return Some(error_response(None, CODE_PARSE_ERROR, "Parse error")),
    };
    // rm-1960 rider (exact id echo): serde_json's default number lane
    // parses integers past `u64::MAX` as f64, so the id
    // 999999999999999999999999 used to echo as `1e24`, corrupting the
    // host's correlation id at the one place the spec demands a
    // byte-exact echo. The raw token is captured via `RawValue` (it
    // preserves the input bytes) and spliced back into the response
    // whenever serde's rendering of the parsed id differs from it.
    let raw_id_token: Option<String> = serde_json::from_str::<IdTokenProbe>(raw)
        .ok()
        .and_then(|probe| probe.id.map(|token| token.get().to_string()));
    // MCP drops JSON-RPC batching (since the 2025-06-18 revision);
    // answer arrays loudly instead of misparsing them as one request.
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
    // rm-782: JSON-RPC 2.0 restricts request ids to String | Number |
    // Null. Anything else used to be cloned and echoed verbatim
    // (`id: [1,2]` came back as `id: [1,2]`); the spec's answer for a
    // request whose id could not be decoded is -32600 with id null.
    let id = match object.get("id") {
        None => None,
        Some(value) if value.is_string() || value.is_number() || value.is_null() => {
            Some(value.clone())
        }
        Some(_) => {
            return Some(error_response(
                None,
                CODE_INVALID_REQUEST,
                "Invalid Request: `id` must be a string, number, or null",
            ));
        }
    };
    // rm-998: JSON-RPC 2.0 §5.1 — the `jsonrpc` member MUST be
    // exactly the string "2.0". This is the member lane of the
    // validation triad: rm-780 hardened the `initialize`
    // negotiation, rm-782 the id-type and params lanes, and this arm
    // closes the last gap — the three-way live probe at assess
    // (`{"id":1,"method":"ping"}`, `{"id":3,"jsonrpc":"9.9",…}`)
    // showed both shapes answered results, so a host speaking any
    // other dialect was silently served. The check sits AFTER id
    // validation (so the echoed id is always spec-shaped — the
    // rm-782 discipline is not weakened) and BEFORE method
    // extraction (so the member violation wins the message when a
    // line is wrong in both). A number (`"jsonrpc": 2.0`) is not
    // the string "2.0" and is refused the same way. Lines without
    // an id keep the notification posture (silence): an id-less
    // message could never be answered with an id echo, and the
    // missing-method arm below made the same call for its lane.
    match object.get("jsonrpc") {
        Some(Value::String(version)) if version == "2.0" => {}
        _ => {
            // rm-1960 rider: silence is reserved for notifications. A
            // line with no id AND no string `method` is not one
            // (JSON-RPC 2.0 §5.1 requires the method member), so it
            // answers Invalid Request with id null instead of going
            // quiet — `{}` used to produce no response at all.
            if id.is_none() && !notification_shaped(object) {
                return Some(error_response(
                    None,
                    CODE_INVALID_REQUEST,
                    "Invalid Request: `jsonrpc` must be exactly \"2.0\"",
                ));
            }
            id.as_ref()?;
            return Some(splice_raw_id(
                error_response(
                    id,
                    CODE_INVALID_REQUEST,
                    "Invalid Request: `jsonrpc` must be exactly \"2.0\"",
                ),
                raw_id_token.as_deref(),
            ));
        }
    }
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        // No method: a request is invalid, a notification is ignorable.
        // rm-1960 rider: an id-less no-method line is not a
        // notification, so it answers with id null rather than
        // silence (previously `id.as_ref()?` swallowed it whole).
        return Some(match id {
            Some(id) => splice_raw_id(
                error_response(
                    Some(id),
                    CODE_INVALID_REQUEST,
                    "Invalid Request: missing method",
                ),
                raw_id_token.as_deref(),
            ),
            None => error_response(
                None,
                CODE_INVALID_REQUEST,
                "Invalid Request: missing method",
            ),
        });
    };
    // Notifications (no id) are acknowledged by staying silent — that
    // includes `notifications/initialized`, which simply transitions
    // the handshake.
    let id = id?;
    // rm-782: MCP params are objects. A present-but-non-object
    // `params` used to flow into `Value::get`, silently behaving like
    // an absent one (`initialize` with `params: 5` answered the
    // default version as if nothing was sent) — answer -32602, the
    // discipline the published `type: "object"` schema promises.
    let params = match object.get("params") {
        None | Some(Value::Null) => Value::Null,
        Some(value) if value.is_object() => value.clone(),
        Some(_) => {
            return Some(splice_raw_id(
                error_response(
                    Some(id),
                    CODE_INVALID_PARAMS,
                    "Invalid params: `params` must be an object",
                ),
                raw_id_token.as_deref(),
            ));
        }
    };
    // rm-1960: modern (2026-07-28) per-request version lane. The
    // stateless revision carries the requested protocol version in
    // `_meta` on every request — under its reserved key
    // `io.modelcontextprotocol/protocolVersion` — instead of the
    // legacy `initialize` handshake. A version this server does not
    // speak answers the spec's `UnsupportedProtocolVersionError`
    // (-32022, `data: {supported, requested}`) — the recognized
    // modern error stdio clients probe for; an absent or supported
    // version serves normally on the newest supported revision.
    if let Some(requested) = requested_meta_version(&params) {
        if !SUPPORTED_PROTOCOL_VERSIONS.contains(&requested) {
            return Some(splice_raw_id(
                unsupported_protocol_version_response(id, requested),
                raw_id_token.as_deref(),
            ));
        }
    }
    let result = match method {
        "initialize" => initialize_result(params),
        "ping" => Ok(json!({})),
        "server/discover" => Ok(discover_result()),
        "subscriptions/listen" => Err((
            CODE_METHOD_NOT_FOUND,
            "Method not found: subscriptions/listen — this server is read-only \
             and implements no subscriptions"
                .to_string(),
        )),
        "resources/subscribe" | "resources/unsubscribe" => Err((
            CODE_METHOD_NOT_FOUND,
            "Method not found: resources/subscribe is superseded by \
             subscriptions/listen in the 2026-07-28 revision; this server is \
             read-only and implements no subscriptions"
                .to_string(),
        )),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => call_tool(params, root),
        _ => {
            return Some(splice_raw_id(
                error_response(Some(id), CODE_METHOD_NOT_FOUND, "Method not found"),
                raw_id_token.as_deref(),
            ));
        }
    };
    Some(match result {
        Ok(value) => splice_raw_id(
            wire_sanitize(json!({"jsonrpc": "2.0", "id": id, "result": value}).to_string()),
            raw_id_token.as_deref(),
        ),
        Err((code, message)) => splice_raw_id(
            error_response(Some(id), code, &message),
            raw_id_token.as_deref(),
        ),
    })
}

/// Server identity payload reused by the legacy `initialize` result
/// and the modern `server/discover` result (rm-1960): one string, one
/// truth — the two surfaces must never drift apart.
const SERVER_INSTRUCTIONS: &str = "Read-only usage reporting over locally discovered \
    agent sessions. Tools: usage_overview (the CLI --overview JSON), \
    by_model_breakdown (cost and sessions per model). The server is offline \
    and stdio-only; it writes nothing outside the agenttrace session cache.";

/// rm-1960 rider (exact id echo): probe struct that captures the `id`
/// member of a request line as its RAW JSON text. `RawValue` preserves
/// the input bytes, so an integer that overflows serde_json's default
/// number lane (parsed as f64) is still available verbatim.
#[derive(serde::Deserialize)]
struct IdTokenProbe<'a> {
    #[serde(borrow, default)]
    id: Option<&'a serde_json::value::RawValue>,
}

/// JSON-RPC 2.0 §5.1: a Notification is a Request object without an
/// `id` — which still requires a string `method` member. Lines that
/// lack both are not notifications and must be answered, not silenced.
fn notification_shaped(object: &serde_json::Map<String, Value>) -> bool {
    object.get("method").and_then(Value::as_str).is_some()
}

/// rm-1960: the per-request protocol version of a modern request, as
/// declared in `params._meta`. The revision's reserved key
/// [`META_PROTOCOL_VERSION_KEY`] is the canonical lane; the bare
/// `protocolVersion` spelling is honored as a lenient alias — it is
/// NOT a reserved key, but honoring it costs nothing and a thin
/// client that omits the prefix still gets the version gate. When
/// both are present the reserved key wins (a non-reserved key must
/// never be able to veto the spec's own spelling).
fn requested_meta_version(params: &Value) -> Option<&str> {
    let meta = params.get("_meta")?;
    meta.get(META_PROTOCOL_VERSION_KEY)
        .or_else(|| meta.get("protocolVersion"))
        .and_then(Value::as_str)
}

/// rm-1960 rider: restore a byte-exact id echo. When serde's rendering
/// of the parsed id differs from the raw input token (the f64 mangling
/// case), splice the token back into the already-serialized response.
/// In-range integers, strings, and null render identically and are
/// untouched; a token that cannot be located is a no-op, never a
/// corruption.
fn splice_raw_id(response: String, raw_token: Option<&str>) -> String {
    let Some(token) = raw_token else {
        return response;
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&response) else {
        return response;
    };
    let Some(rendered) = parsed.get("id").map(|id| id.to_string()) else {
        return response;
    };
    if rendered == token {
        return response;
    }
    response.replacen(&format!("\"id\":{rendered}"), &format!("\"id\":{token}"), 1)
}

/// rm-1960: `UnsupportedProtocolVersionError` — the 2026-07-28
/// revision's spec-named modern error (code -32022, error.data carries
/// `supported` and `requested`). Built directly because
/// [`error_response`] carries no `data` member.
fn unsupported_protocol_version_response(id: Value, requested: &str) -> String {
    let mut object = Map::new();
    object.insert("jsonrpc".to_string(), json!("2.0"));
    object.insert("id".to_string(), id);
    object.insert(
        "error".to_string(),
        json!({
            "code": CODE_UNSUPPORTED_PROTOCOL_VERSION,
            "message": format!(
                "Unsupported protocol version: {requested} (supported: {})",
                SUPPORTED_PROTOCOL_VERSIONS.join(", ")
            ),
            "data": {
                "supported": SUPPORTED_PROTOCOL_VERSIONS,
                "requested": requested,
            },
        }),
    );
    wire_sanitize(Value::Object(object).to_string())
}

/// rm-1960: `server/discover` result — the 2026-07-28 revision's one
/// mandatory server RPC, replacing the initialize handshake for
/// modern (stateless, per-request `_meta`) clients. `supportedVersions`
/// is the same array [`initialize_result`] negotiates from, newest
/// first; `capabilities` and `_meta['io.modelcontextprotocol/serverInfo']`
/// follow the DiscoverResult shape the revision specifies. No state is
/// consulted: the answer is identical before or after any other
/// request, which is exactly what a stdio client's
/// backward-compatibility probe expects.
fn discover_result() -> Value {
    json!({
        "supportedVersions": SUPPORTED_PROTOCOL_VERSIONS,
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "_meta": {
            "io.modelcontextprotocol/serverInfo": {
                "name": "agenttrace",
                "version": env!("CARGO_PKG_VERSION")
            }
        },
        "instructions": SERVER_INSTRUCTIONS,
    })
}

/// `initialize` result (rm-780): answer with the newest protocol
/// revision BOTH sides speak. A client-declared version in
/// [`SUPPORTED_PROTOCOL_VERSIONS`] is answered with itself (that IS
/// the mutually supported one); any other value — unknown or absent —
/// falls back to the newest revision this server supports, with a
/// one-line stderr disclosure so the host's log explains what
/// happened. A client echo is never returned: the old behavior told a
/// 2026-07-28 host "supported" and then served it the older wire the
/// server was actually built for (assess F5a PoC: `1999-99-99` was
/// echoed back verbatim).
fn initialize_result(params: Value) -> Result<Value, (i64, String)> {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let answered = match requested {
        Some(version) if SUPPORTED_PROTOCOL_VERSIONS.contains(&version) => version,
        unsupported => {
            // stdout is protocol-only; the disclosure rides stderr.
            eprintln!(
                "agenttrace mcp: client requested protocol version {}, which this \
                 server does not speak; answering with its newest supported revision \
                 {} (supported: {})",
                unsupported.unwrap_or("<none>"),
                SUPPORTED_PROTOCOL_VERSIONS[0],
                SUPPORTED_PROTOCOL_VERSIONS.join(", ")
            );
            SUPPORTED_PROTOCOL_VERSIONS[0]
        }
    };
    Ok(json!({
        "protocolVersion": answered,
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "serverInfo": {
            "name": "agenttrace",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": SERVER_INSTRUCTIONS,
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
                    "enum": RANGE_ALIASES,
                    "description": "Report window, matched case-insensitively \
        exactly like the CLI's --range: `today`/`day`/`1d` is your local \
        calendar day; `7d`/`week`/`weekly` and `30d`/`month`/`monthly` are \
        rolling windows; `all` (or an empty string) is everything \
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
    // rm-1918: error texts embed filesystem-derived strings (corpus
    // paths, unreadable-database reasons), so they ride the same text
    // escape as report payloads — a spoofing control in a scanned path
    // must not reach the host's renderer through the error lane.
    Ok(json!({
        "content": [{ "type": "text", "text": escape_spoofing_json(&message) }],
        "isError": true
    }))
}

/// JSON type of a value, for `-32602` messages that name what the
/// host actually sent (landed rm-840 wording).
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

/// `tools/call` dispatch with the CLI's parameter discipline:
/// structurally bad params are `-32602` (the JSON-RPC twin of the
/// CLI's rc2 usage error), corpus-level failures are `isError`
/// results (the twin of the CLI's rc1 bail). `root` is the corpus
/// root the dispatch site resolved (rm-781).
fn call_tool(params: Value, root: Option<&Path>) -> Result<Value, (i64, String)> {
    let invalid = |message: String| (CODE_INVALID_PARAMS, message);
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err(invalid(
            "Invalid params: `name` must name a tool".to_string(),
        ));
    };
    // rm-782 (this run's assess F6a PoC) + landed rm-840 (renumbered
    // from campaign-local rm-800): `arguments` must be an object when
    // present. Every tool takes an optional object; a
    // string/number/bool/array value used to fall through
    // `unwrap_or(json!({}))` and silently execute the tool with
    // default arguments — a client bug became an unintended run. The
    // landed rm-840 wording (naming the tool and the JSON type the
    // host sent) is kept: both lanes' tests pin it. Absent (and JSON
    // null — this file's own convention for optional values,
    // `parse_range_argument` treats `"range": null` the same way)
    // keeps the spec-legal empty-object default.
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
        "usage_overview" => usage_overview(root, range),
        "by_model_breakdown" => by_model_breakdown(root, range),
        other => Err(invalid(format!(
            "Invalid params: unknown tool `{other}` (available: usage_overview, \
by_model_breakdown)"
        ))),
    }
}

/// Parse the shared `range` argument. Absent means `all` (the CLI's
/// default reporting window); anything else must be one of the labels
/// `TimeRange::parse` enforces — the SAME alias set the tools'
/// `inputSchema` publishes (rm-782: message and schema must not
/// under-declare the parser).
fn parse_range_argument(arguments: &Value) -> Result<TimeRange, String> {
    match arguments.get("range") {
        None | Some(Value::Null) => Ok(TimeRange::All),
        Some(Value::String(raw)) => TimeRange::parse(raw).ok_or_else(|| {
            format!(
                "Invalid params: `range` must be one of today, day, 1d, 7d, week, \
weekly, 30d, month, monthly, all, or \"\" for all, matched case-insensitively \
(got `{raw}`)"
            )
        }),
        Some(_) => Err(
            "Invalid params: `range` must be a string (today, day, 1d, 7d, week, \
weekly, 30d, month, monthly, all)"
                .to_string(),
        ),
    }
}

/// Load the locally discovered sessions for a window, exactly the way
/// the CLI's default (no `-d`) report actions do: `HOME`-rooted known
/// agent homes, `since` filter, and the session cache. `root` mirrors
/// the CLI's `-d/--dir` corpus choice (honored since rm-781 — the
/// dispatch site used to accept the flag and silently drop it).
fn load_for_range(root: Option<&Path>, range: TimeRange) -> LoadReport {
    let since = range.since(Utc::now());
    load_sessions_with_options(
        root,
        &LoadOptions {
            since,
            ..Default::default()
        },
    )
}

/// The corpus produced no reportable sessions. rm-781: mirror the
/// CLI's message distinction instead of reporting every empty result
/// as a discovery miss — a corpus whose sessions all fall outside the
/// window is a FILTER miss ("No sessions match the requested
/// filters", the CLI's own wording), not "no session files found"
/// (which used to send users hunting for a HOME problem they did not
/// have). Unreadable SQLite corpora keep the rm-753 distinction
/// (unreadable, not absent), and an explicit `-d` root names the
/// directory the user chose.
fn empty_corpus_result(
    root: Option<&Path>,
    range: TimeRange,
    report: &LoadReport,
) -> Result<Value, (i64, String)> {
    if report.discovered > 0 {
        return tool_execution_error(format!(
            "No sessions match the requested filters (range {}): {} session file(s) \
discovered, none inside the window.",
            range.label(),
            report.discovered
        ));
    }
    if !report.sqlite.unreadable.is_empty() {
        let listed = report
            .sqlite
            .unreadable
            .iter()
            .map(|db| format!("{} ({})", db.path.display(), db.reason))
            .collect::<Vec<_>>()
            .join(", ");
        return tool_execution_error(format!(
            "No session files found in any auto-discovered agent home, but {} agent \
database(s) were found and could not be read: {}",
            report.sqlite.unreadable.len(),
            listed
        ));
    }
    match root {
        Some(dir) => tool_execution_error(format!(
            "No session files found in {} (directory exists but holds no session \
files)",
            dir.display()
        )),
        None => tool_execution_error(format!(
            "No session files found in any auto-discovered agent home for range {} — \
point HOME at the machine whose agents you want to report on, or see \
`agenttrace --help` for discovery roots.",
            range.label()
        )),
    }
}

/// `usage_overview`: render through the CLI's own report path so the
/// MCP answer and `agenttrace --overview -f json` stay one truth.
fn usage_overview(root: Option<&Path>, range: TimeRange) -> Result<Value, (i64, String)> {
    let report = load_for_range(root, range);
    if report.sessions.is_empty() {
        return empty_corpus_result(root, range, &report);
    }
    let overview = compute_overview(&report.sessions);
    // Truthful coverage accounting straight from the loader's census
    // (pass-8 F8-2), identical to the CLI overview arm — including the
    // landed rm-548 fork-exclusion count and the landed rm-734
    // unreadable-sqlite records so MCP and CLI disclosures agree.
    // (This run's compound banner pre-recorded the re-anchor: "re-anchor
    // mcp.rs (origin carries rm-548 threading at :293-306 over this
    // base)").
    let health = data_health_scoped(
        &report.sessions,
        report.discovered,
        report.skipped,
        report.cache_hits,
        report.opencode_fork_excluded,
        // rm-1534: keep MCP and CLI disclosures agreeing — the
        // future-bound exclusion count flows through here too.
        report.future_excluded_sessions,
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
fn by_model_breakdown(root: Option<&Path>, range: TimeRange) -> Result<Value, (i64, String)> {
    let report = load_for_range(root, range);
    if report.sessions.is_empty() {
        return empty_corpus_result(root, range, &report);
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
///
/// rm-1918: "verbatim" means the SANITIZED document. The CLI's
/// dispatch boundary passes its JSON documents through
/// `escape_spoofing_json` before writing them, so composing the same
/// escape here is what keeps MCP payloads byte-identical to
/// `agenttrace --overview -f json` stdout on the same corpus — a raw
/// U+202E in a model id used to ride the tool text while the CLI lane
/// escaped it. The wire-level pass in [`wire_sanitize`] cannot provide
/// that parity on its own: escaping only the outer line would leave
/// the PARSED text payload carrying the raw control, so the escape
/// happens at both layers (idempotently — payload escapes are literal
/// backslash text by the time the outer line is serialized).
fn tool_text_result(text: String) -> Value {
    json!({
        "content": [{ "type": "text", "text": escape_spoofing_json(&text) }],
        "isError": false
    })
}

/// Match `agenttrace-core`'s report rounding so numbers shown over MCP
/// equal numbers shown by the CLI.
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// rm-1918 wire pass: the LAST serialization boundary every response
/// line crosses. serde leaves spoofing Cf controls (U+202E-class) raw
/// inside a serialized document — and host-supplied strings echo into
/// `-32602` messages (`range`/tool-name echoes) while error texts
/// embed corpus paths — so the wire line itself is escaped. Lossless
/// (`\uXXXX` decodes back to the char), a no-op on clean lines, and
/// idempotent over payloads already escaped by [`tool_text_result`]
/// (their escapes serialize as `\\uXXXX` backslash text, not Cf
/// chars, so the predicate never matches them).
fn wire_sanitize(line: String) -> String {
    escape_spoofing_json(&line)
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
    wire_sanitize(Value::Object(object).to_string())
}

#[cfg(test)]
mod rm_1025_tests {
    //! rm-1025 (silent-zero closeout rider): the refusal text must
    //! track the constant, not a hand-typed numeral. If this test
    //! fails, the message and the enforced cap have drifted apart.
    use super::*;

    #[test]
    fn oversized_line_message_tracks_the_constant() {
        let message = oversized_line_message();
        assert_eq!(
            message,
            format!(
                "Invalid Request: message line exceeds the {MCP_INPUT_MAX_BYTES}-byte transport cap and was discarded"
            )
        );
        // The numeral itself renders (today 1048576); asserting the
        // formatted value, not a re-typed literal, is what makes the
        // test fail loudly if someone re-hardcodes the string.
        assert!(message.contains(&MCP_INPUT_MAX_BYTES.to_string()));
    }
}

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
//! * protocol currency (rm-780 + rm-455, run f62ee197 cycle 4) —
//!   the server speaks the MCP revisions in
//!   [`SUPPORTED_PROTOCOL_VERSIONS`], newest first. `initialize`
//!   answers the newest revision the client also declared, never a
//!   client-supplied echo; the requested version may arrive as the
//!   `protocolVersion` member OR inline under the reserved
//!   per-request `_meta` key
//!   `io.modelcontextprotocol/protocolVersion`, honored statelessly
//!   on EVERY request. An unsupported version answers the 2026-07-28
//!   schema's typed `UnsupportedProtocolVersionError` (`-32022`,
//!   `data {requested, supported}`) — never a success result; an
//!   absent version still falls back to the newest supported
//!   revision with a one-line stderr disclosure (stdout stays
//!   protocol-only). The revision's one mandatory server RPC
//!   `server/discover` answers the stateless discovery surface (see
//!   [`discover_result`]).
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
const CODE_INVALID_PARAMS: i64 = -32602;
/// The 2026-07-28 revision's typed version-negotiation failure
/// (rm-455, run f62ee197 cycle 4). The code and the `data` shape
/// are fixed by the schema (`UnsupportedProtocolVersionError`):
/// `requested` (string — what the client asked for) plus
/// `supported` (the revisions this server speaks, newest first).
/// The message text is NOT schema-fixed — the base `Error` shape
/// only asks for "a concise single sentence".
const CODE_UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

/// Reserved per-request `_meta` member (2026-07-28): every request
/// may carry its protocol version inline under this canonical
/// reserved key, and a server honors it statelessly. The
/// `io.modelcontextprotocol/` prefix is spec-reserved — a BARE
/// `protocolVersion` `_meta` member is client-chosen, not reserved,
/// and is deliberately ignored here (honoring it would let a
/// non-version member flip the negotiated dialect).
const META_PROTOCOL_VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";

/// Reserved `_meta` member carrying server identity (the same
/// `serverInfo` object `initialize` returns) on results that can
/// reach a host without a handshake.
const META_SERVER_INFO_KEY: &str = "io.modelcontextprotocol/serverInfo";

/// MCP protocol revisions this server speaks, NEWEST FIRST — the
/// order is the negotiation preference (rm-780). `initialize` answers
/// the first entry the client also declared, or falls back to `[0]`
/// for unknown/absent client versions. The ladder mirrored the
/// official revision list (modelcontextprotocol repository tags,
/// fetched live 2026-10-08) down to `2025-06-18`; rm-455 arm 2 (run
/// e9eb92e2 cycle 2) deliberately FLOORS the ladder at the two
/// legacy revisions older hosts actually declare (`2025-03-26`, the
/// spec revision that introduced `_meta` and jsonrpc 2.0 framing, and
/// `2024-11-05`, the original revision) — a legacy host that
/// negotiated down to "unsupported version" while the server could
/// have served its wire verbatim was the acceptance's own bug class.
/// Adding or removing a revision here is a deliberate, tested act —
/// the cost the rm-455 rmcp-adoption rider tracks.
const SUPPORTED_PROTOCOL_VERSIONS: [&str; 5] = [
    "2026-07-28",
    "2025-11-25",
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
];

/// `tools/list` cache hint (rm-455, run e4209f0c cycle 5): the
/// 2026-07-28 revision's CacheableResult rule (SEP-2549) makes
/// `ttlMs` REQUIRED on `tools/list` results. One hour is honest
/// here: the tool list is compiled into the binary (it changes only
/// with a binary upgrade, and `initialize` declares `listChanged:
/// false`), TTL-based freshness is the only invalidation channel a
/// host has, and the value matches the canonical
/// `ListToolsResultResponse` example in the spec tree at the
/// 2026-07-28 tag. Raising or lowering it is a deliberate, tested
/// act (the `tools_list_carries_2026_07_28_cache_hints` contract
/// pins the field's presence, not this literal).
const TOOLS_LIST_TTL_MS: u64 = 3_600_000;

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

/// `session_list`'s display limit when the host sends none — the
/// CLI's `--sessions` `--limit` default (rm-455 arm 1, run e9eb92e2
/// cycle 2). Mirrored, not shared: the CLI default lives in the
/// clap attributes on `Args::limit`; the pin
/// `session_tools_render_the_cli_documents` and the published
/// `inputSchema` `default` keep the two honest against each other.
const SESSIONS_LIMIT_DEFAULT: usize = 20;

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
            id.as_ref()?;
            return Some(error_response(
                id,
                CODE_INVALID_REQUEST,
                "Invalid Request: `jsonrpc` must be exactly \"2.0\"",
            ));
        }
    }
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
    // rm-782: MCP params are objects. A present-but-non-object
    // `params` used to flow into `Value::get`, silently behaving like
    // an absent one (`initialize` with `params: 5` answered the
    // default version as if nothing was sent) — answer -32602, the
    // discipline the published `type: "object"` schema promises.
    let params = match object.get("params") {
        None | Some(Value::Null) => Value::Null,
        Some(value) if value.is_object() => value.clone(),
        Some(_) => {
            return Some(error_response(
                Some(id),
                CODE_INVALID_PARAMS,
                "Invalid params: `params` must be an object",
            ));
        }
    };
    // A2+A3 (rm-455, run f62ee197 cycle 4): resolve the requested
    // version from the `protocolVersion` member OR the reserved
    // canonical `_meta` key — the `_meta` lane is stateless and
    // applies to EVERY method, matching the revision's
    // inline-negotiation rule. An unsupported version answers the
    // schema's typed error; the old fall-back-to-newest answered a
    // client that cannot speak this wire with a success result — a
    // lie on both ends (assess F1 probe: `1999-99-99` negotiated
    // "successfully").
    if let Some(requested) = requested_protocol_version(&params) {
        if !SUPPORTED_PROTOCOL_VERSIONS.contains(&requested) {
            return Some(unsupported_protocol_version_response(id, requested));
        }
    }
    let result = match method {
        "initialize" => initialize_result(params),
        // A1 (rm-455, run f62ee197 cycle 4): the 2026-07-28
        // revision's ONE mandatory server RPC — `DiscoverRequest`'s
        // schema description reads "Servers MUST implement
        // `server/discover`. Clients MAY call it but are not required
        // to — version negotiation can also happen inline via
        // per-request `_meta`." Stateless: no `initialize` needed
        // first. `resultType` rides the central [`with_result_type`]
        // seam like every other result.
        "server/discover" => Ok(discover_result()),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => call_tool(params, root),
        _ => {
            return Some(error_response(
                Some(id),
                CODE_METHOD_NOT_FOUND,
                "Method not found",
            ));
        }
    };
    Some(match result {
        Ok(value) => {
            // rm-455: the single serialization seam where every
            // result gains its required `resultType` member (see
            // [`with_result_type`]).
            let result = with_result_type(value);
            wire_sanitize(json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string())
        }
        Err((code, message)) => error_response(Some(id), code, &message),
    })
}

/// rm-455 (run e4209f0c cycle 5): the 2026-07-28 revision — the
/// newest this server advertises and negotiates — makes `resultType`
/// REQUIRED on every result (the `Result` base interface in
/// schema/2026-07-28/schema.ts at the spec tag: "Servers implementing
/// this protocol version MUST include this field", SEP-2322's MRTR
/// work; clients of older servers MUST treat the absent member as
/// `"complete"`). Every result this server builds is complete — the
/// multi-round-trip `input_required` shape is elicit/sampling
/// territory a read-only reporting server never enters — so the
/// member is inserted once, at this seam, and covers all four
/// method results (initialize, ping, tools/list, tools/call)
/// identically. It stays additive on the older revisions this server
/// also speaks: their `Result` shapes are index-signature
/// extensible, so an older client ignores the unknown member rather
/// than failing (the same posture the spec demands of clients
/// reading results from servers of that era).
fn with_result_type(result: Value) -> Value {
    match result {
        Value::Object(mut object) => {
            object
                .entry("resultType".to_string())
                .or_insert_with(|| json!("complete"));
            Value::Object(object)
        }
        other => other,
    }
}

/// `initialize` result (rm-780): answer with the newest protocol
/// revision BOTH sides speak. A client-declared version in
/// [`SUPPORTED_PROTOCOL_VERSIONS`] is answered with itself (that IS
/// the mutually supported one); the requested version may also arrive
/// inline under the reserved canonical `_meta` key (see
/// [`requested_protocol_version`] — rm-455, run f62ee197 cycle 4).
/// An unsupported value never reaches this function: the typed
/// `-32022` gate answers it before dispatch. An ABSENT version falls
/// back to the newest revision this server supports, with a one-line
/// stderr disclosure so the host's log explains what happened. A
/// client echo is never returned: the old behavior told a
/// 2026-07-28 host "supported" and then served it the older wire the
/// server was actually built for (assess F5a PoC: `1999-99-99` was
/// echoed back verbatim).
fn initialize_result(params: Value) -> Result<Value, (i64, String)> {
    let requested = requested_protocol_version(&params);
    let answered = match requested {
        Some(version) if SUPPORTED_PROTOCOL_VERSIONS.contains(&version) => version,
        unsupported => {
            // Reachable only for an ABSENT version (rm-455, run
            // f62ee197 cycle 4): unsupported values — member or
            // `_meta` — are answered with the typed -32022 error at
            // the dispatch gate before this function runs. Absent
            // stays permissive on purpose: the strict reading would
            // lock out exactly the legacy clients the fallback
            // serves. stdout is protocol-only; the disclosure rides
            // stderr.
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
        "instructions": "Read-only usage reporting over locally discovered \
    agent sessions. Tools: usage_overview (the CLI --overview JSON), \
    by_model_breakdown (cost and sessions per model), session_list (the \
    CLI --sessions listing), session_detail (one session, the CLI \
    --inspect JSON), doctor_verdict (the corpus doctor's verdict, the \
    CLI doctor JSON). The server is offline \
    and stdio-only; it writes nothing outside the agenttrace session cache."
    }))
}

/// A2+A3 helper (rm-455, run f62ee197 cycle 4): resolve the
/// requested protocol version for ANY request — the
/// `protocolVersion` member when present (precedence: the explicit
/// legacy member wins over `_meta`), else the reserved canonical
/// `_meta` key. A non-string member is not a version signal and is
/// ignored (the schema types both members as string). A bare,
/// non-reserved `protocolVersion` `_meta` spelling is deliberately
/// ignored too — see [`META_PROTOCOL_VERSION_KEY`].
fn requested_protocol_version(params: &Value) -> Option<&str> {
    params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .or_else(|| {
            params
                .get("_meta")
                .and_then(Value::as_object)
                .and_then(|meta| meta.get(META_PROTOCOL_VERSION_KEY))
                .and_then(Value::as_str)
        })
}

/// A2 (rm-455, run f62ee197 cycle 4): the typed version-negotiation
/// failure. Fixed by the 2026-07-28 schema: code `-32022` and the
/// `data` object with `requested` (what the client asked for) and
/// `supported` (the same newest-first list `server/discover`
/// advertises, so the client can recover without a second round
/// trip). The message text is this server's choice ("Unsupported
/// version") — the schema fixes the code and `data`, not the
/// message.
fn unsupported_protocol_version_response(id: Value, requested: &str) -> String {
    wire_sanitize(
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": CODE_UNSUPPORTED_PROTOCOL_VERSION,
                "message": "Unsupported version",
                "data": {
                    "requested": requested,
                    "supported": SUPPORTED_PROTOCOL_VERSIONS,
                }
            }
        })
        .to_string(),
    )
}

/// A1 (rm-455, run f62ee197 cycle 4): the `server/discover` result.
/// The 2026-07-28 schema's `DiscoverResult` requires all five
/// members; `supportedVersions` is the same newest-first
/// negotiation list `initialize` uses; `capabilities` is the honest
/// static surface (two read-only tools, never invalidated
/// mid-session — `listChanged: false`); the cache-hint pair reuses
/// [`TOOLS_LIST_TTL_MS`] for the same compile-time-static rationale
/// (this surface changes only with a binary upgrade). The reserved
/// `_meta` server-info key carries the identity a host that skips
/// `initialize` still needs. `resultType` is inserted centrally by
/// [`with_result_type`] ("complete" — discovery is not an
/// incremental shape).
fn discover_result() -> Value {
    let mut meta = Map::new();
    meta.insert(
        META_SERVER_INFO_KEY.to_string(),
        json!({
            "name": "agenttrace",
            "version": env!("CARGO_PKG_VERSION"),
        }),
    );
    json!({
        "supportedVersions": SUPPORTED_PROTOCOL_VERSIONS,
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "ttlMs": TOOLS_LIST_TTL_MS,
        "cacheScope": "public",
        "_meta": Value::Object(meta),
    })
}

/// `tools/list` result. Descriptions state the read-only posture so a
/// host that surfaces them to the model carries the contract too.
/// rm-455 (run e4209f0c cycle 5): the 2026-07-28 revision REQUIRES the
/// CacheableResult pair (SEP-2549) on this result — `ttlMs` plus
/// `cacheScope`. `"public"` is the spec's own recommendation
/// (docs/specification/2026-07-28/server/utilities/caching.mdx) for
/// lists of tools "identical for all users", which these are: the
/// list is static per binary and carries no user or corpus data.
/// `resultType` is added centrally by [`with_result_type`] (it is
/// required on all results, not just this one).
fn tools_list_result() -> Value {
    // rm-455 arm 1 (run e9eb92e2 cycle 2): the shared `range` property
    // is factored out so the three new tools publish the EXACT enum +
    // description the two report tools do — schema drift between
    // tools was rm-782's original bug class.
    let range_property = || {
        json!({
            "type": "string",
            "enum": RANGE_ALIASES,
            "description": "Report window, matched case-insensitively \
        exactly like the CLI's --range: `today`/`day`/`1d` is your local \
        calendar day; `7d`/`week`/`weekly` and `30d`/`month`/`monthly` are \
        rolling windows; `all` (or an empty string) is everything \
        discovered. Defaults to `all`."
        })
    };
    let range_schema = || {
        json!({
            "type": "object",
            "properties": {
                "range": range_property()
            },
            "additionalProperties": false
        })
    };
    json!({
        // rm-455 (run e4209f0c cycle 5): CacheableResult pair REQUIRED on
        // tools/list by the 2026-07-28 revision (SEP-2549) — see the
        // function docs for the value choices.
        "ttlMs": TOOLS_LIST_TTL_MS,
        "cacheScope": "public",
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
            },
            // rm-455 arm 1 (run e9eb92e2 cycle 2): the tool-scope
            // acceptance — the docs had promised session list/detail and
            // a doctor verdict since the guide's first cut; the server
            // shipped two report tools and grep'd zero of the three.
            // Order here is the docs guide's Tools-section order, pinned
            // by `handshake_tools_and_report_rendering` and
            // scripts/ci/check-docs-commands.sh.
            {
                "name": "session_list",
                "description": "The session listing — the same JSON the \
    `agenttrace --sessions -f json` CLI action renders over the same \
    discovered corpus (matched/returned counts, truncation disclosure, one \
    row per session in the CLI's default `--sort recent --order desc` \
    order). Read-only; no network.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "range": range_property(),
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "default": SESSIONS_LIMIT_DEFAULT,
                            "description": "Maximum sessions returned — \
            the `--limit` default of `agenttrace --sessions`; the document \
            discloses `truncated` and `matched_sessions` so a capped \
            listing cannot be mistaken for the whole corpus."
                        }
                    },
                    "required": [],
                    "additionalProperties": false
                }
            },
            {
                "name": "session_detail",
                "description": "One session's full diagnostics — the same \
    JSON `agenttrace --inspect <rank> -f json` renders for that rank \
    (1-based position in session_list's order): the session object, \
    pricing basis, cost anomaly alert, and fix suggestions. Read-only; \
    no network.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "rank": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "1-based position in the \
            session_list order (the CLI --sessions order); 1 is the \
            most recent session."
                        },
                        "range": range_property()
                    },
                    "required": ["rank"],
                    "additionalProperties": false
                }
            },
            {
                "name": "doctor_verdict",
                "description": "The corpus doctor's verdict — the same JSON \
    the `agenttrace --doctor -f json` CLI action renders (agent version, \
    mode, session cache census, finding categories). Takes no arguments; \
    it scans the same auto-discovered homes `--doctor` does. Read-only; \
    no network.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": [],
                    "additionalProperties": false
                }
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
        // rm-455 arm 1 (run e9eb92e2 cycle 2): the three tools the
        // guide always promised. Each renders through the CLI's own
        // lane (crate::sessions_listing_json / session_detail_json /
        // doctor_report_json) so the MCP answer and the CLI document
        // are one truth — the same posture usage_overview set.
        "session_list" => {
            let limit = match parse_limit_argument(&arguments) {
                Ok(limit) => limit,
                Err(message) => return Err(invalid(message)),
            };
            let report = load_for_range(root, range);
            if report.sessions.is_empty() {
                return empty_corpus_result(root, range, &report);
            }
            match crate::sessions_listing_json(report.sessions.clone(), limit) {
                Ok(text) => Ok(tool_text_result(text)),
                Err(error) => tool_execution_error(error.to_string()),
            }
        }
        "session_detail" => {
            let rank = match parse_rank_argument(&arguments) {
                Ok(rank) => rank,
                Err(message) => return Err(invalid(message)),
            };
            let report = load_for_range(root, range);
            if report.sessions.is_empty() {
                return empty_corpus_result(root, range, &report);
            }
            match crate::session_detail_json(report.sessions.clone(), rank) {
                Ok(text) => Ok(tool_text_result(text)),
                Err(error) => tool_execution_error(error.to_string()),
            }
        }
        "doctor_verdict" => {
            // The schema publishes zero properties and
            // `additionalProperties: false` — so does the dispatch:
            // any argument is a -32602, not a silently ignored extra
            // (the same discipline `parse_range_argument` applies to
            // its lane).
            if let Some(fields) = arguments.as_object() {
                if !fields.is_empty() {
                    let unexpected = fields.keys().cloned().collect::<Vec<_>>().join(", ");
                    return Err(invalid(format!(
                        "Invalid params: `doctor_verdict` takes no arguments \
                         (unexpected: {unexpected}) — remove every argument; the \
                         tool scans the same auto-discovered corpus \
                         `agenttrace --doctor` does"
                    )));
                }
            }
            match crate::doctor_report_json(root) {
                Ok(text) => Ok(tool_text_result(text)),
                Err(error) => tool_execution_error(error.to_string()),
            }
        }
        other => Err(invalid(format!(
            "Invalid params: unknown tool `{other}` (available: usage_overview, \
by_model_breakdown, session_list, session_detail, doctor_verdict)"
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

/// Parse `session_list`'s `limit` argument (rm-455 arm 1, run e9eb92e2
/// cycle 2). Absent means the CLI's `--sessions` default
/// ([`SESSIONS_LIMIT_DEFAULT`]); a non-integer or a value below 1 is
/// `-32602` in the landed rm-840 wording (name the field, name what
/// the host sent) — the CLI's rc2 usage-error twin, not an execution
/// failure.
fn parse_limit_argument(arguments: &Value) -> Result<usize, String> {
    match arguments.get("limit") {
        None | Some(Value::Null) => Ok(SESSIONS_LIMIT_DEFAULT),
        Some(Value::Number(number)) => match number.as_i64() {
            Some(limit) if limit >= 1 => Ok(limit as usize),
            Some(limit) => Err(format!(
                "Invalid params: `limit` must be at least 1 (got {limit})"
            )),
            None => Err(format!(
                "Invalid params: `limit` must be an integer (got {number})"
            )),
        },
        Some(other) => Err(format!(
            "Invalid params: `limit` for tool `session_list` must be an \
             integer when present, got {}",
            json_type_name(other)
        )),
    }
}

/// Parse `session_detail`'s required `rank` argument (rm-455 arm 1,
/// run e9eb92e2 cycle 2). `rank` is REQUIRED — a rank-less detail
/// call has no session to render — and must be an integer ≥ 1 (1 =
/// the most recent session in the `--sessions` order). Out-of-window
/// ranks are NOT a params error: a rank past the listing's end is an
/// execution outcome and answers an `isError` result, exactly like
/// the CLI's `--inspect` bail.
fn parse_rank_argument(arguments: &Value) -> Result<usize, String> {
    match arguments.get("rank") {
        None | Some(Value::Null) => Err(
            "Invalid params: `rank` for tool `session_detail` is required \
             (1-based position in the session_list order)"
                .to_string(),
        ),
        Some(Value::Number(number)) => match number.as_i64() {
            Some(rank) if rank >= 1 => Ok(rank as usize),
            Some(rank) => Err(format!(
                "Invalid params: `rank` must be at least 1 (got {rank})"
            )),
            None => Err(format!(
                "Invalid params: `rank` must be an integer (got {number})"
            )),
        },
        Some(other) => Err(format!(
            "Invalid params: `rank` for tool `session_detail` must be an \
             integer when present, got {}",
            json_type_name(other)
        )),
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

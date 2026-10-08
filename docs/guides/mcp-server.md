# Local MCP server (`agenttrace mcp`)

`agenttrace mcp` turns the reports the CLI already renders into tools a
coding agent can call from inside its own session: the agent asks
"where did my tokens go" and gets the same `--overview` JSON a human
gets in the terminal. It is a host-keyword command like
`agenttrace statusline` and `agenttrace upstream` — flags placed
BEFORE the keyword are either honored (`-d/--dir` and the config
family: `--config`, `--history-dir`, `--pricing-file`) or refused at
exit 2, because a flag that cannot apply to a stdio server used to be
accepted and silently dropped; run `agenttrace mcp --help` for the
on-disk help text.

## Posture

The server is deliberately minimal, and the limits are part of the
contract (enforced in code, and pinned by
`scripts/ci/check-docs-commands.sh`):

- **Read-only.** The tools report on the locally discovered sessions;
  they never edit, delete, or send them anywhere. There is no second
  renderer — both tools render through the same discovery and pricing
  stack as `agenttrace --overview`, and the mcp lane resolves the SAME
  layered configuration the CLI does (user/project `config.toml`,
  `--config`, `--pricing-file`, `--history-dir`): a pricing override
  moves the CLI's answer and the MCP answer equally (rm-781).
- **No network.** The server opens no sockets and performs no registry
  or HTTP probes. Stdio is the only transport, and there is no
  `--fetch`-style opt-in: this command has no network mode at all.
- **No writes outside artifact roots.** Loading sessions may refresh
  the agenttrace session cache (`AGENTTRACE_SESSION_CACHE_DIR`) — the
  same cache the CLI maintains. Nothing else is written.
- **Host-friendly exit.** The server serves until stdin closes. A host
  disconnect (broken pipe) is a clean exit 0, not a failure.

## Registration

Any host that speaks stdio MCP can register the server. Generic form:

```json
{
  "mcpServers": {
    "agenttrace": {
      "command": "agenttrace",
      "args": ["mcp"]
    }
  }
}
```

Claude Code (`claude mcp add`):

```bash
claude mcp add agenttrace -- agenttrace mcp
```

Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.agenttrace]
command = "agenttrace"
args = ["mcp"]
```

## Protocol surface

Newline-delimited JSON-RPC 2.0 over stdio — one JSON message per line
in, one per line out, flushed per message.

| Method | Answers |
| --- | --- |
| `initialize` | `serverInfo`, `capabilities.tools`, and the newest **mutually supported** `protocolVersion` of `2026-07-28`, `2025-11-25`, `2025-06-18` — negotiated, never echoed; an unknown or absent client version falls back to the server's newest with a one-line stderr disclosure (rm-780) |
| `notifications/initialized` | silence (notifications are never answered) |
| `ping` | `{}` |
| `tools/list` | the two tools below |
| `tools/call` | tool result or `isError` result |

JSON-RPC batching is rejected (`-32600`), matching the current MCP
spec revision. Malformed JSON answers `-32700`; unknown methods answer
`-32601`; invalid tool parameters answer `-32602`. The wire reader is
disciplined (rm-782): a non-UTF-8 line or one past the 1 MiB line cap
answers `-32700` and the server keeps serving, request ids outside
String/Number/Null answer `-32600` with `id: null` instead of being
echoed back, and a non-object `params` or `arguments` answers
`-32602` instead of behaving like an absent value. A corpus that
discovered no sessions for the requested window is **not** a protocol
error: the call returns a tool result with `isError: true` — a
discovery miss ("No session files found", the same message the CLI
prints) when no session files exist at all, and a filter miss
("No sessions match the requested filters", naming the discovery
count) when files exist but fall outside the window or filters
(rm-781) — and the server keeps serving. The JSON-RPC error arms are
the twin of the CLI's rc2 usage errors; `isError` results are the
twin of its rc1 report failures.

## Tools

### `usage_overview`

The full `agenttrace --overview -f json` document: summary totals,
`by_model`, `by_provider`, `by_task_type`, top cost drivers,
`recent_sessions` (capped at 10), scope disclosure, and data health
from the loader's census.

| Parameter | Type | Values | Default |
| --- | --- | --- | --- |
| `range` | string | `today`, `day`, `1d`, `7d`, `week`, `weekly`, `30d`, `month`, `monthly`, `all`, `""` | `all` |

`today` is your local calendar day; `7d`/`30d` are rolling windows —
the same semantics as the CLI's `--range`, and the schema publishes
exactly the labels the parser enforces (rm-782).

### `by_model_breakdown`

The `by_model` rollup of the same overview computation: cost and
session counts per model, sorted cost-descending, with window totals.
Smaller answer, same discovery and pricing path.

| Parameter | Type | Values | Default |
| --- | --- | --- | --- |
| `range` | string | `today`, `day`, `1d`, `7d`, `week`, `weekly`, `30d`, `month`, `monthly`, `all`, `""` | `all` |

## A tool call, end to end

`initialize` and the `initialized` notification, then a report:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":"all"}}}
```

Answers — note the notification gets no response line, and the
negotiation answered `2025-06-18` because that is what BOTH sides
speak (ask for `1999-99-99` and the answer is the server's newest,
with a stderr disclosure):

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"agenttrace","version":"…"},"instructions":"…"}}
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"range\":\"all\",\"total_sessions\":1,\"total_cost\":0.0,\"by_model\":[{\"name\":\"gpt-5\",\"sessions\":1,\"cost\":0.0}]}"],"isError":false}}
```

Report payloads travel as text content holding the JSON document
verbatim — every host can display it, and machine consumers parse the
text.

## Naming: `mcp` the keyword vs `--mcp-governance` the flag

These are unrelated and easy to confuse:

- `agenttrace mcp` (this guide) **runs** a local MCP server.
- `agenttrace --mcp-governance` **filters** a report to MCP-server
  governance findings (which MCP servers your agents call) — it is a
  scope flag that rides a report action, not a server.

## Verification

The end-to-end wire contract (handshake and protocol-version
negotiation, both tools, every error arm including hostile wire
lines — non-UTF-8, over-cap, bad ids, non-object params/arguments —
the empty-corpus and filter-miss `isError` paths, the honored/refused
flag contract before the keyword, `-d` scoping and admission, and
config-layer pricing parity with the CLI, rc0 on EOF) is pinned by
`crates/agenttrace-cli/tests/mcp_server.rs`.

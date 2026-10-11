#!/usr/bin/env bash
# rm-1039: MCP service truth — ONE corpus load per process. The server
# used to re-discover and re-parse the corpus on EVERY tools/call
# (measured live at mint: 194.71s per `by_model_breakdown today` call
# against the real agent home, paid again on the next call; a 4-file
# `-d` corpus cost 0.50s — the cost is the rescan, not the report).
# This probe pins the REUSE BAND on a generated in-place corpus: the
# second identical tools/call must answer byte-identically, in under
# ONE second, AND at least 3x faster than the first call — the ratio
# is the discriminator (a re-scanning implementation answers both
# calls at the same speed, measured 1.6x on the negative-control
# shim, and fails it however fast the machine is; the absolute cap is
# the spec'd "sub-second" band). The reuse/reload
# arms over a LIVE corpus (mutate between calls of one process) are
# pinned by the e2e test
# `one_load_per_process_reuses_quiet_and_reloads_on_mutation` in
# agenttrace-cli/tests/mcp_server.rs.
#
# No python: the repo's CI lane assumes only bash + node, so the
# corpus generator and the JSON-RPC client are one heredoc'd node
# program that also performs the assertions.
set -euo pipefail

bin="${AGENTTRACE_BIN:-/tmp/agenttrace}"
out_dir="${AGENTTRACE_CI_OUT:-/tmp/agenttrace-ci}"

fail() {
  echo "check-mcp-corpus-reuse: $*" >&2
  exit 1
}

[[ -x "$bin" ]] || fail "agenttrace binary is not executable: $bin"
# node spawn resolves a relative executable against the CHILD cwd;
# absolutize so CI's conventional `AGENTTRACE_BIN=target/release/agenttrace`
# keeps working (the cwd-independent contract the other CI scripts use).
[[ "$bin" = /* ]] || bin="$PWD/$bin"
mkdir -p "$out_dir/mcp-reuse"

scratch="$(mktemp -d "$out_dir/mcp-reuse/corpus-XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

HOME_ENV="$scratch/home" CACHE_ENV="$scratch/cache" BIN="$bin" node - <<'NODE'
const fs = require("fs");
const path = require("path");
const { spawn } = require("child_process");

const home = process.env.HOME_ENV;
const cache = process.env.CACHE_ENV;
const bin = process.env.BIN;
fs.mkdirSync(path.join(home, ".hermes", "sessions"), { recursive: true });
fs.mkdirSync(cache, { recursive: true });

// One transcript per local day for the last 120 days — same shape as
// the repo's testdata/generated fixtures (the generic lane keys one
// session per file). ~120 sessions is a real parse cost on the first
// call (449 lines total) and a trivial fingerprint walk on the
// second, keeping the reuse gap wide without being slow in CI.
const now = new Date();
for (let day = 0; day < 120; day += 1) {
  const stamp = new Date(now.getTime() - day * 86_400_000);
  const iso = stamp.toISOString();
  const lines = [
    `{"role":"user","content":"probe ${day}","timestamp":"${iso}","ModelUsed":"probe-model","SourceTool":"generic"}`,
    `{"role":"assistant","content":"done","timestamp":"${iso}","ModelUsed":"probe-model","SourceTool":"generic"}`,
    `{"role":"user","content":"again","timestamp":"${iso}","ModelUsed":"probe-model","SourceTool":"generic"}`,
    `{"role":"assistant","content":"done","timestamp":"${iso}","ModelUsed":"probe-model","SourceTool":"generic"}`,
  ];
  fs.writeFileSync(
    path.join(home, ".hermes", "sessions", `probe-${day}.jsonl`),
    `${lines.join("\n")}\n`
  );
}

// Wire client: one request line in, one response line out — the
// contract tests/mcp_server.rs pins at the JSON-RPC level.
const server = spawn(bin, ["mcp"], {
  env: {
    ...process.env,
    HOME: home,
    AGENTTRACE_SESSION_CACHE_DIR: cache,
  },
  stdio: ["pipe", "pipe", "inherit"],
  // The scratch root (not the seeded home) so no accidental pickup
  // from the process cwd — same isolation discipline as the e2e
  // harness in tests/mcp_server.rs.
  cwd: path.dirname(home),
});
let buffer = "";
let pending = null;
server.stdout.on("data", (chunk) => {
  buffer += chunk.toString("utf8");
  const newline = buffer.indexOf("\n");
  if (newline !== -1 && pending) {
    const resolve = pending[1];
    pending = null;
    const text = buffer.slice(0, newline);
    buffer = buffer.slice(newline + 1);
    resolve(text.trim());
  } else if (newline !== -1) {
    buffer = buffer.slice(newline + 1);
  }
});
function request(id) {
  return new Promise((resolve) => {
    pending = [id, resolve];
    server.stdin.write(
      `{"jsonrpc":"2.0","id":${id},"method":"tools/call","params":{"name":"by_model_breakdown","arguments":{"range":"all"}}}\n`
    );
  });
}

async function timedCall(id) {
  const start = process.hrtime.bigint();
  const text = await request(id);
  const seconds = Number(process.hrtime.bigint() - start) / 1e9;
  return { text, seconds };
}

(async () => {
  const first = await timedCall(1);
  const second = await timedCall(2);

  const fail = (message) => {
    console.error(`check-mcp-corpus-reuse: ${message}`);
    process.exit(1);
  };

  // Unwrap the wire envelope to the tool's text payload — the
  // layer the reuse contract speaks at.
  const payload = (rawLine) => {
    const envelope = JSON.parse(rawLine);
    if (envelope.error) {
      fail(`tools/call answered an error: ${JSON.stringify(envelope.error)}`);
    }
    return envelope.result.content[0].text;
  };

  const firstText = payload(first.text);
  const secondText = payload(second.text);
  let firstPayload;
  try {
    firstPayload = JSON.parse(firstText);
  } catch {
    fail(`tool result is not JSON: ${firstText.slice(0, 200)}`);
  }
  if (firstPayload.total_sessions !== 120) {
    fail(`expected 120 sessions, got ${firstPayload.total_sessions}`);
  }
  if (firstText !== secondText) {
    fail("second identical tools/call answered differently (no reuse)");
  }
  console.log(
    `mcp corpus reuse: first ${first.seconds.toFixed(3)}s, second ${second.seconds.toFixed(4)}s`
  );
  if (second.seconds >= 1.0) {
    fail(
      `second identical tools/call took ${second.seconds.toFixed(3)}s (>= 1s): the corpus was re-parsed instead of reused`
    );
  }
  if (second.seconds * 3 >= first.seconds) {
    fail(
      `second call was only ${(first.seconds / second.seconds).toFixed(1)}x faster than the first (need 3x): the corpus looks re-parsed instead of reused`
    );
  }
  server.stdin.end();
  server.on("exit", (code) => {
    if (code !== 0) {
      fail(`server exited ${code}`);
    }
    process.exit(0);
  });
})().catch((error) => {
  console.error(`check-mcp-corpus-reuse: ${error}`);
  process.exit(1);
});
NODE

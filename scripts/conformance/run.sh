#!/usr/bin/env bash
# rm-053: offline token-accounting conformance runner (binary level).
#
# Drives the checked-in conformance pack (testdata/conformance/) through
# the agenttrace CLI end to end: every case's rc, token classes, model,
# provenance markers, and cost totals are compared against the manifest.
# The library-level twin (parser + pricing directly) lives in
# crates/agenttrace-core/tests/token_conformance.rs, which also carries
# the snapshot re-derivation guard for every cost literal; this runner
# adds the wire-shape half (what `--sessions -f json` actually renders).
#
# Fully offline: no network, no writes outside a mktemp scratch dir.
# Usage: AGENTTRACE_BIN=<path> scripts/conformance/run.sh
#   (defaults to target/release, then target/debug, then fails).
set -euo pipefail

fail() { echo "conformance: $*" >&2; exit 1; }

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
manifest="$repo_root/testdata/conformance/manifest.json"
[ -f "$manifest" ] || fail "missing pack manifest at $manifest"

if [ -n "${AGENTTRACE_BIN:-}" ]; then
  bin="$AGENTTRACE_BIN"
elif [ -x "$repo_root/target/release/agenttrace" ]; then
  bin="$repo_root/target/release/agenttrace"
elif [ -x "$repo_root/target/debug/agenttrace" ]; then
  bin="$repo_root/target/debug/agenttrace"
else
  fail "no agenttrace binary found; build one (cargo build -p agenttrace) or set AGENTTRACE_BIN"
fi
[ -x "$bin" ] || fail "AGENTTRACE_BIN=$bin is not executable"
echo "conformance: binary $bin"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

node -e '
const fs = require("fs");
const { spawnSync } = require("child_process");

const manifestPath = process.argv[1];
const bin = process.argv[2];
const work = process.argv[3];
const pack = manifestPath.replace(/manifest\.json$/, "");
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const tolerance = manifest.pricing_basis.formula_tolerance;

let asserted = 0;
let xfailPinned = 0;

for (const c of manifest.cases) {
  const id = c.id;
  const fixture = pack + c.fixture;
  if (!fs.existsSync(fixture)) throw new Error(`${id}: missing fixture ${c.fixture}`);

  const out = spawnSync(bin, ["--sessions", "-f", "json", fixture], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });

  if (c.expect.rc === 1) {
    if (out.status === 0) throw new Error(`${id}: hostile journal must exit nonzero`);
    const stderr = out.stderr || "";
    if (!stderr.includes(c.expect.stderr_contains))
      throw new Error(`${id}: stderr must mention "${c.expect.stderr_contains}", got: ${stderr.trim()}`);
    asserted += 1;
    continue;
  }

  if (out.status !== 0)
    throw new Error(`${id}: expected rc 0, got ${out.status} (${(out.stderr || "").trim()})`);
  const sessions = JSON.parse(out.stdout);
  if (!Array.isArray(sessions) || sessions.length !== 1)
    throw new Error(`${id}: expected exactly one session in --sessions output`);
  const m = sessions[0].metrics;
  const e = c.expect;

  if (e.model !== undefined && m.model_used !== e.model)
    throw new Error(`${id}: model_used ${m.model_used} != ${e.model}`);
  if (e.source_tool !== undefined && m.source_tool !== e.source_tool)
    throw new Error(`${id}: source_tool ${m.source_tool} != ${e.source_tool}`);

  const tokenFields = [["input", "tokens_input"], ["output", "tokens_output"],
    ["cache_w", "tokens_cache_w"], ["cache_r", "tokens_cache_r"], ["reasoning", "tokens_reasoning"]];
  for (const [key, field] of tokenFields) {
    const expected = (e.tokens && e.tokens[key]) || 0;
    if (m[field] !== expected)
      throw new Error(`${id}: ${field} ${m[field]} != ${expected}`);
  }

  if (e.cost_estimated === null) {
    if (!Number.isFinite(m.cost_estimated) || m.cost_estimated < 0)
      throw new Error(`${id}: absurd totals must price finite and non-negative`);
  } else if (c.xfail) {
    xfailPinned += 1;
    if (Math.abs(m.cost_estimated - e.cost_estimated) < 1e-9)
      throw new Error(`${id}: xfail-pinned case now matches its truth — the fix pinned to ` +
        `${c.xfail.pinned_to} landed; drop the xfail block as part of that landing pack refresh`);
  } else if (Math.abs(m.cost_estimated - e.cost_estimated) >= 1e-9) {
    throw new Error(`${id}: cost_estimated ${m.cost_estimated} != ${e.cost_estimated}`);
  }

  if (e.cost_provenance !== undefined && m.provenance.Cost !== e.cost_provenance)
    throw new Error(`${id}: provenance.Cost ${m.provenance.Cost} != ${e.cost_provenance}`);
  if (e.tokens_provenance !== undefined && m.provenance.Tokens !== e.tokens_provenance)
    throw new Error(`${id}: provenance.Tokens ${m.provenance.Tokens} != ${e.tokens_provenance}`);

  asserted += 1;
}

console.log(`conformance: ${asserted} cases asserted (${xfailPinned} xfail-pinned) — all within truth`);
' "$manifest" "$bin" "$work"

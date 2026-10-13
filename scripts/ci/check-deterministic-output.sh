#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "check-deterministic-output: $*" >&2
  exit 1
}

# rm-377 (assess fdaf8b39 F6, mirroring rm-369 in check-docs-commands.sh):
# the binary default must be the repo's own release build, not
# /tmp/agenttrace — /tmp persists across sessions and a stale or foreign
# binary silently answered gate runs (a leftover DEBUG build answered
# deterministic-output probes days after it was built). AGENTTRACE_BIN
# still overrides for A/B against a pinned build.
repo_root="$(git rev-parse --show-toplevel)" \
  || fail "must run inside the agenttrace repository (or set AGENTTRACE_BIN)"
bin="${AGENTTRACE_BIN:-$repo_root/target/release/agenttrace}"
out_dir="${AGENTTRACE_CI_OUT:-/tmp/agenttrace-ci}"

[[ -x "$bin" ]] || fail "agenttrace binary is not executable: $bin"
mkdir -p "$out_dir/determinism"

for i in 1 2 3; do
  "$bin" --demo --latest -f json >"$out_dir/determinism/latest-$i.json"
  "$bin" --demo --overview -f csv >"$out_dir/determinism/overview-csv-$i.csv"
  "$bin" --demo --overview -f json >"$out_dir/determinism/overview-$i.json"
done

for i in 1 2 3; do
  "$bin" --demo --overview -f json --baseline "$out_dir/determinism/overview-1.json" \
    >"$out_dir/determinism/baseline-$i.json"
done

for path in "$out_dir"/determinism/*.json; do
  node -e 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))' "$path" \
    || fail "invalid JSON: $path"
done

cmp -s "$out_dir/determinism/latest-1.json" "$out_dir/determinism/latest-2.json" \
  || fail "--demo --latest -f json changed between run 1 and 2"
cmp -s "$out_dir/determinism/latest-1.json" "$out_dir/determinism/latest-3.json" \
  || fail "--demo --latest -f json changed between run 1 and 3"
cmp -s "$out_dir/determinism/overview-1.json" "$out_dir/determinism/overview-2.json" \
  || fail "--demo --overview -f json changed between run 1 and 2"
cmp -s "$out_dir/determinism/overview-1.json" "$out_dir/determinism/overview-3.json" \
  || fail "--demo --overview -f json changed between run 1 and 3"
cmp -s "$out_dir/determinism/overview-csv-1.csv" "$out_dir/determinism/overview-csv-2.csv" \
  || fail "--demo --overview -f csv changed between run 1 and 2"
cmp -s "$out_dir/determinism/overview-csv-1.csv" "$out_dir/determinism/overview-csv-3.csv" \
  || fail "--demo --overview -f csv changed between run 1 and 3"
grep -q $'\r' "$out_dir/determinism/overview-csv-1.csv" \
  || fail "csv output must use CRLF row endings (RFC 4180)"
cmp -s "$out_dir/determinism/baseline-1.json" "$out_dir/determinism/baseline-2.json" \
  || fail "--demo --overview -f json --baseline changed between run 1 and 2"
cmp -s "$out_dir/determinism/baseline-1.json" "$out_dir/determinism/baseline-3.json" \
  || fail "--demo --overview -f json --baseline changed between run 1 and 3"

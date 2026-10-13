#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "check-deterministic-output: $*" >&2
  exit 1
}

# rm-1419 rider: never default to the shared /tmp/agenttrace binary or
# the shared /tmp/agenttrace-ci out dir — both persist across sessions
# and a stale artifact from another session can silently answer this
# gate (binary variant observed 2026-10-03, rm-369). Env overrides still
# win for A/B against a pinned build. The repo root derives from the
# script's own location, not `git rev-parse`, so an out-of-repo
# invocation with AGENTTRACE_BIN set works — the git guard used to
# fail it before the env was ever consulted (rm-1419 review F5).
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)" \
  || fail "cannot locate the agenttrace repository from the script path"
bin="${AGENTTRACE_BIN:-$repo_root/target/release/agenttrace}"
if [[ -n "${AGENTTRACE_CI_OUT:-}" ]]; then
  out_dir="$AGENTTRACE_CI_OUT"
else
  out_dir="$(mktemp -d /tmp/agenttrace-determinism.XXXXXX)"
  trap 'rm -rf "$out_dir"' EXIT
fi

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

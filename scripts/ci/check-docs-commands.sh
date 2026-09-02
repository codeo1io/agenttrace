#!/usr/bin/env bash
set -euo pipefail

bin="${AGENTTRACE_BIN:-/tmp/agenttrace}"
out_dir="${AGENTTRACE_CI_OUT:-/tmp/agenttrace-ci}"

fail() {
  echo "check-docs-commands: $*" >&2
  exit 1
}

[[ -x "$bin" ]] || fail "agenttrace binary is not executable: $bin"
mkdir -p "$out_dir/docs"

"$bin" --version >"$out_dir/docs/version.txt"
"$bin" --doctor -f json >"$out_dir/docs/doctor.json"
"$bin" --demo --latest -f json >"$out_dir/docs/latest.json"
"$bin" --demo --latest --lang zh -f json >"$out_dir/docs/latest-zh.json"
"$bin" --demo --overview -f json >"$out_dir/docs/overview.json"
"$bin" --demo --search billing >"$out_dir/docs/search.txt"
"$bin" --demo --search internal/ws -f json >"$out_dir/docs/search.json"
"$bin" --demo --overview -f markdown -o "$out_dir/docs/overview.md" >/tmp/agenttrace-docs-md.stdout
"$bin" --demo --overview -f html -o "$out_dir/docs/overview.html" >/tmp/agenttrace-docs-html.stdout
for path in "$out_dir"/docs/*.json; do
  node -e 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))' "$path" \
    || fail "invalid JSON from documented command: $path"
done

set +e
"$bin" --demo --overview \
  --fail-under-health 80 \
  --fail-on-critical \
  --max-tool-fail-rate 15 \
  >"$out_dir/docs/gate.stdout" \
  2>"$out_dir/docs/gate.stderr"
status=$?
set -e
[[ "$status" -eq 2 ]] || fail "documented CI gate command should exit 2 for demo data, got $status"
grep -q 'Gate failed:' "$out_dir/docs/gate.stderr" \
  || fail "documented CI gate command should explain gate failures on stderr"

# Pass-8 F8-1/F8-7 docs contract (CU-15): the docs must tell the truth
# the code pins. A guide that claims schema 4 (code says 6) or a 24h
# automatic refresh (the code is network-free outside --update-pricing)
# is a lie with a CLI-contract lifespan.
guide="docs/guides/governance-reports.md"
[[ -f "$guide" ]] || fail "missing guide: $guide"

snapshot_schema=$(grep -oE 'const SQLITE_SNAPSHOT_SCHEMA_VERSION: i64 = [0-9]+' \
  crates/agenttrace-core/src/session_cache.rs | grep -oE '[0-9]+$')
session_schema=$(grep -oE 'const SESSION_CACHE_SCHEMA_VERSION: i64 = [0-9]+' \
  crates/agenttrace-core/src/session_cache.rs | grep -oE '[0-9]+$')
[[ -n "$snapshot_schema" && -n "$session_schema" ]] \
  || fail "could not read schema constants from session_cache.rs"
grep -q "SQLite snapshot is schema $snapshot_schema" "$guide" \
  || fail "guide must state the real SQLite snapshot schema ($snapshot_schema)"
grep -q "session cache is schema $session_schema" "$guide" \
  || fail "guide must state the real session cache schema ($session_schema)"
if grep -qiE 'refreshed automatically|refresh.*in the background|background.*refresh' "$guide"; then
  fail "guide must not claim automatic background refresh: pricing runs are network-free outside --update-pricing"
fi
if grep -qE 'schema 4' "$guide"; then
  fail "guide still claims the stale schema-4 snapshot version"
fi

# README must document the Go-flag argument-order trap (F8-8): flags
# after the first positional are ignored.
grep -q 'before the session path\|before the first positional' README.md \
  || fail "README must document that flags go before the positional session path"

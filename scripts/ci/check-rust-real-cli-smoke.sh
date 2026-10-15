#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

bin="${AGENTTRACE_BIN:-$repo_root/target/release/agenttrace}"
out_dir="${AGENTTRACE_REAL_CLI_OUT:-/tmp/agenttrace-real-cli-smoke-$(date +%Y%m%d%H%M%S)}"
source_dir="${AGENTTRACE_REAL_CLI_DIR:-$HOME/.pi/agent/sessions}"
query="${AGENTTRACE_REAL_CLI_QUERY:-error}"
file_limit="${AGENTTRACE_REAL_CLI_FILE_LIMIT:-8}"

fail() {
  echo "check-rust-real-cli-smoke: $*" >&2
  exit 1
}

run_capture() {
  local name="$1"
  shift
  local status=0
  mkdir -p "$out_dir/cache-$name"
  set +e
  env AGENTTRACE_SESSION_CACHE_DIR="$out_dir/cache-$name" \
    "$bin" "$@" >"$out_dir/$name.out" 2>"$out_dir/$name.err"
  status=$?
  set -e
  printf "%s\n" "$status" >"$out_dir/$name.status"
}

expect_status() {
  local name="$1"
  local expected="$2"
  local actual
  actual="$(cat "$out_dir/$name.status")"
  [[ "$actual" == "$expected" ]] || fail "$name exit status $actual, expected $expected"
}

require_json() {
  local path="$1"
  node -e 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))' "$path" \
    || fail "invalid JSON: $path"
}

[[ -x "$bin" ]] || fail "agenttrace binary is not executable: $bin"
[[ -d "$source_dir" ]] || fail "real sessions directory does not exist: $source_dir"

# rm-525: the snapshot stages REAL operator transcripts; stage it
# private (umask 077 — observed live as -rw-rw-r-- copies landing next
# to the binary's own 0600 cache files) and every later artifact of
# this gate stays private too. The umask sits ABOVE the output/snapshot
# mkdirs (review 6cbb259b F6) so the staged DIRECTORIES are owner-only
# as well, not just the files inside them.
umask 077
rm -rf "$out_dir"
mkdir -p "$out_dir"

snapshot_dir="$out_dir/real-session-snapshot"
mkdir -p "$snapshot_dir"
copied=0
while IFS= read -r file; do
  cp "$file" "$snapshot_dir/session-$copied-${file##*/}"
  copied=$((copied + 1))
  [[ "$copied" -ge "$file_limit" ]] && break
done < <(find "$source_dir" -type f 2>/dev/null | sort)
[[ "$copied" -gt 0 ]] || fail "could not copy real session files from: $source_dir"

# rm-525 acceptance arm: prove the staging is private — no group/other
# access bits on any staged file OR directory (directories included
# per review 6cbb259b F6).
[[ -z "$(find "$snapshot_dir" -perm /077 -print -quit)" ]] ||
  fail "real-session snapshot must not be group/other accessible (rm-525)"

run_capture doctor-default --doctor -f json
run_capture doctor-snapshot --doctor -d "$snapshot_dir" -f json
run_capture overview-snapshot -d "$snapshot_dir" --overview -f json
run_capture latest-snapshot -d "$snapshot_dir" --latest -f json
run_capture search-snapshot -d "$snapshot_dir" --search "$query" -f json
run_capture compare-snapshot -d "$snapshot_dir" --compare -f json
run_capture waste-snapshot -d "$snapshot_dir" --waste

# rm-952 (cycle 3, landed by review fix fb86baf6): CI step-summary dogfood
# leg - a real overview run with GITHUB_STEP_SUMMARY set must render the
# existing overview markdown into the summary file while stdout stays pure
# JSON (the arm is additive-only; agentic purity holds with it on, not just
# off). stderr may carry ordinary discovery parse warnings from the sampled
# corpus (same as the overview-snapshot leg); the step-summary arm's own
# failure path prints "warning: could not append GITHUB_STEP_SUMMARY at
# <path>" (main.rs rm-952) - THAT message is pinned absent here.
step_summary="$out_dir/step-summary.md"
step_summary_status=0
env AGENTTRACE_SESSION_CACHE_DIR="$out_dir/cache-step-summary" \
  GITHUB_STEP_SUMMARY="$step_summary" \
  "$bin" --overview -d "$snapshot_dir" -f json \
  >"$out_dir/step-summary.out" 2>"$out_dir/step-summary.err" \
  || step_summary_status=$?
printf "%s\n" "$step_summary_status" >"$out_dir/step-summary.status"

for name in \
  doctor-default doctor-snapshot overview-snapshot \
  latest-snapshot search-snapshot compare-snapshot waste-snapshot step-summary
do
  expect_status "$name" 0
done

for name in doctor-default doctor-snapshot overview-snapshot latest-snapshot search-snapshot compare-snapshot step-summary; do
  require_json "$out_dir/$name.out"
done

grep -q "Waste Analysis" "$out_dir/waste-snapshot.out" \
  || fail "waste report should include Waste Analysis"

[[ -s "$step_summary" ]] \
  || fail "GITHUB_STEP_SUMMARY run should render a non-empty step summary"
grep -q "^# agenttrace overview" "$step_summary" \
  || fail "step summary should carry the overview render heading"
! grep -q "could not append GITHUB_STEP_SUMMARY" "$out_dir/step-summary.err" \
  || fail "step-summary arm reported an append failure on stderr"

node - "$out_dir" "$copied" <<'NODE'
const fs = require("fs");
const path = require("path");
const outDir = process.argv[2];
const copied = Number(process.argv[3]);

function readJson(name) {
  return JSON.parse(fs.readFileSync(path.join(outDir, `${name}.out`), "utf8"));
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

const doctor = readJson("doctor-snapshot");
assert(doctor.mode === "custom directory", `snapshot doctor mode changed: ${doctor.mode}`);
assert(doctor.session_files > 0, `snapshot doctor should parse at least one copied session file, copied ${copied}`);

const overview = readJson("overview-snapshot");
assert(overview.version && overview.summary, "overview snapshot missing version or summary");
assert(overview.summary.total_sessions > 0, "overview snapshot should include sessions");
assert(typeof overview.summary.total_tokens === "number", "overview snapshot missing total token number");
assert(Array.isArray(overview.recent_sessions), "overview snapshot missing recent_sessions");

const latest = readJson("latest-snapshot");
assert(latest.version && latest.session, "latest snapshot missing session metadata");
assert(latest.source_tool, "latest snapshot missing source_tool");

const search = readJson("search-snapshot");
assert(typeof search.count === "number" && Array.isArray(search.results), "search snapshot shape changed");

const compare = readJson("compare-snapshot");
assert(compare.sessions && Array.isArray(compare.sessions) && compare.sessions.length > 0, "compare snapshot shape changed");
assert(compare.sessions.every((item) => item.name && item.metrics && typeof item.health === "number"), "compare snapshot entries changed");
assert(compare.audited_sessions === compare.sessions.length, "compare must disclose the audited session count");
assert(compare.total_sessions === compare.sessions.length, "default compare audits every matching session");
NODE

echo "Rust real CLI smoke passed: source_dir=$source_dir sampled_files=$copied query=$query"

#!/usr/bin/env bash
set -euo pipefail

# rm-994 (run cebf20d5 cycle 2, batch "trusted instruments"): the
# out-dir below carries COPIES of the operator's real session
# transcripts and report renders over them into a shared /tmp path —
# every artifact this gate writes must stay owner-only (dirs 0700,
# files 0600). The umask makes that the default; the closing find leg
# pins it.
umask 077

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

rm -rf "$out_dir"
mkdir -p "$out_dir"

snapshot_dir="$out_dir/real-session-snapshot"
mkdir -p "$snapshot_dir"
copied=0
while IFS= read -r file; do
  cp "$file" "$snapshot_dir/session-$copied-${file##*/}"
  # rm-994: the copy is the transcript itself — pin it owner-only
  # regardless of the source file's mode (the umask already strips
  # group/other bits; this survives a cp that preserves modes).
  chmod 600 "$snapshot_dir/session-$copied-${file##*/}"
  copied=$((copied + 1))
  [[ "$copied" -ge "$file_limit" ]] && break
done < <(find "$source_dir" -type f 2>/dev/null | sort)
[[ "$copied" -gt 0 ]] || fail "could not copy real session files from: $source_dir"

# rm-994: the default doctor capture used to run UNscoped — default
# discovery walks the operator's REAL corpus homes (every agent lane,
# not just the sampled source dir), so this leg hung at the gate
# timeout (rc=124 at 240s) on populated hosts while CI stayed green
# only because its env pins kept $HOME empty. The default-discovery
# contract now runs against a gate-built EMPTY corpus: doctor must
# exit 0 with a valid custom-directory report when there is nothing
# to parse. The populated custom-dir shape stays covered by the
# doctor-snapshot leg below.
empty_dir="$out_dir/empty-corpus"
mkdir -p "$empty_dir"
run_capture doctor-empty --doctor -d "$empty_dir" -f json
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
  doctor-empty doctor-snapshot overview-snapshot \
  latest-snapshot search-snapshot compare-snapshot waste-snapshot step-summary
do
  expect_status "$name" 0
done

for name in doctor-empty doctor-snapshot overview-snapshot latest-snapshot search-snapshot compare-snapshot step-summary; do
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

# rm-994: pin the privacy contract the umask established — nothing
# under the out-dir (dirs, transcript copies, reports, status files)
# may carry any group or other access bit. POSIX `find -perm -0NN`
# tests one bit at a time; `-print -quit` names the first offender.
leaked="$(
  find "$out_dir" \
    \( -perm -040 -o -perm -020 -o -perm -010 \
    -o -perm -004 -o -perm -002 -o -perm -001 \) -print -quit
)"
[[ -z "$leaked" ]] || fail "real-cli smoke artifacts must stay owner-only (0700/0600): $leaked"

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

// rm-994: the empty-corpus leg — doctor on a directory with nothing
// to parse must still exit 0 with a valid custom-directory report.
const doctorEmpty = readJson("doctor-empty");
assert(doctorEmpty.mode === "custom directory", `empty doctor mode changed: ${doctorEmpty.mode}`);
assert(doctorEmpty.session_files === 0, `empty corpus doctor should disclose zero files, saw ${doctorEmpty.session_files}`);

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

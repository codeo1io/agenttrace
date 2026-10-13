#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "check-report-semantics: $*" >&2
  exit 1
}

# rm-1419 rider: never default to the shared /tmp/agenttrace binary or
# the shared /tmp/agenttrace-ci out dir (rm-369 precedent; env overrides
# still win for A/B against a pinned build). The repo root derives from
# the script's own location, not `git rev-parse`, so an out-of-repo
# invocation with AGENTTRACE_BIN set works — the git guard used to
# fail it before the env was ever consulted (rm-1419 review F5).
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)" \
  || fail "cannot locate the agenttrace repository from the script path"
bin="${AGENTTRACE_BIN:-$repo_root/target/release/agenttrace}"
if [[ -n "${AGENTTRACE_CI_OUT:-}" ]]; then
  out_dir="$AGENTTRACE_CI_OUT"
else
  out_dir="$(mktemp -d /tmp/agenttrace-report-semantics.XXXXXX)"
  trap 'rm -rf "$out_dir"' EXIT
fi

expected_cost_label="${AGENTTRACE_EXPECTED_COST_LABEL:-Total estimated cost}"

[[ -x "$bin" ]] || fail "agenttrace binary is not executable: $bin"
mkdir -p "$out_dir/semantics"

version="$("$bin" --version | sed -E 's/^agenttrace v//')"
[[ -n "$version" ]] || fail "could not read agenttrace version"

"$bin" --demo --overview >"$out_dir/semantics/overview.txt"
"$bin" --demo --overview -f json >"$out_dir/semantics/overview.json"
"$bin" --demo --overview -f markdown >"$out_dir/semantics/overview.md"
"$bin" --demo --overview -f html >"$out_dir/semantics/overview.html"

grep -q "AGENTTRACE v$version" "$out_dir/semantics/overview.txt" \
  || fail "text overview missing current version"
grep -q "$expected_cost_label" "$out_dir/semantics/overview.txt" \
  || fail "text overview missing expected cost label"
grep -q "Incident timeline" "$out_dir/semantics/overview.txt" \
  || fail "text overview missing incident timeline evidence"
grep -q "Tool authority" "$out_dir/semantics/overview.txt" \
  || fail "text overview missing tool authority summary"
grep -q "test_or_build" "$out_dir/semantics/overview.txt" \
  || fail "text overview missing highest demo authority category"
grep -q "| $expected_cost_label |" "$out_dir/semantics/overview.md" \
  || fail "markdown overview missing expected cost label"
grep -q "## Tool authority" "$out_dir/semantics/overview.md" \
  || fail "markdown overview missing tool authority summary"
grep -q "### Authority category counts" "$out_dir/semantics/overview.md" \
  || fail "markdown overview missing authority category counts"
grep -q '`test_or_build`' "$out_dir/semantics/overview.md" \
  || fail "markdown overview missing highest demo authority category"
grep -q "<span>$expected_cost_label</span>" "$out_dir/semantics/overview.html" \
  || fail "html overview missing expected cost label"
grep -q "<div class=\"meta\">v$version" "$out_dir/semantics/overview.html" \
  || fail "html overview missing current version metadata"
grep -q "Estimated session cost" "$out_dir/semantics/overview.html" \
  || fail "html overview missing cost helper text"
grep -q "Tool authority" "$out_dir/semantics/overview.html" \
  || fail "html overview missing tool authority summary"
grep -q "Authority category counts" "$out_dir/semantics/overview.html" \
  || fail "html overview missing authority category counts"
grep -q "<code>test_or_build</code>" "$out_dir/semantics/overview.html" \
  || fail "html overview missing highest demo authority category"

node -e '
const fs = require("fs");
const version = process.argv[2];
const report = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
if (report.version !== version) throw new Error(`json version ${report.version} != ${version}`);
if (!report.summary || typeof report.summary.total_cost !== "number") throw new Error("missing total_cost");
if (typeof report.summary.total_duration_seconds !== "number") throw new Error("missing total_duration_seconds");
if (report.summary.total_sessions !== 3) throw new Error("demo summary should contain 3 sessions");
if (!Array.isArray(report.recent_sessions) || report.recent_sessions.length !== 3) throw new Error("demo should contain 3 recent sessions");
if (!report.recent_sessions.some((session) => typeof session.possible_cost_driver === "string" && session.possible_cost_driver.includes("possible driver"))) {
  throw new Error("demo should expose possible cost driver evidence");
}
if (!Array.isArray(report.failure_families)) throw new Error("missing failure_families");
if (!report.surfaces || !Array.isArray(report.surfaces.tools) || !Array.isArray(report.surfaces.high_authority_tools)) {
  throw new Error("missing deterministic comparison surfaces");
}
if (!Array.isArray(report.surfaces.authority_categories) || !report.surfaces.authority_categories.includes("test_or_build")) {
  throw new Error("missing deterministic authority categories");
}
if (!report.summary.tool_authority || report.summary.tool_authority.highest !== "test_or_build") {
  throw new Error("demo should expose test_or_build as highest authority");
}
' "$out_dir/semantics/overview.json" "$version"

"$bin" --demo --overview -f json --baseline "$out_dir/semantics/overview.json" \
  >"$out_dir/semantics/baseline-compare.json"
node -e '
const fs = require("fs");
const report = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const cmp = report.baseline_comparison;
if (!cmp) throw new Error("missing baseline comparison");
if (cmp.duration_delta_pct !== 0 || cmp.cost_delta_pct !== 0 || cmp.token_delta_pct !== 0) {
  throw new Error("identical baseline should have zero deltas");
}
if (!("slower_than_baseline" in cmp) || !("broader_tool_surface" in cmp) || !("new_tool_authority_categories" in cmp) || !("new_high_authority_tool_use" in cmp)) {
  throw new Error("baseline comparison missing regression fields");
}
' "$out_dir/semantics/baseline-compare.json"

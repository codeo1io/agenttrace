#!/usr/bin/env bash
set -euo pipefail

bin="${AGENTTRACE_BIN:-/tmp/agenttrace}"
out_dir="${AGENTTRACE_CI_OUT:-/tmp/agenttrace-ci}"

fail() {
  echo "check-deterministic-output: $*" >&2
  exit 1
}

[[ -x "$bin" ]] || fail "agenttrace binary is not executable: $bin"
mkdir -p "$out_dir/determinism"

# `generated_at` is wall-clock time at second precision, so two runs that
# straddle a second boundary differ legitimately. Drop the key (recursively:
# baseline_comparison embeds nested report objects) before comparing. This is
# the class-level backstop for ANY compared artifact that embeds wall clock —
# for `--demo` runs the primary guard is the DEMO_REPORT_EPOCH pin in
# crates/agenttrace-cli/src/main.rs (demo_overview_json_is_byte_deterministic
# pins that contract; upstream #294 added this script-side normalization and
# the fork ports the design).
normalize() {
  node -e '
    const fs = require("fs");
    const strip = (v) => {
      if (Array.isArray(v)) return v.map(strip);
      if (v && typeof v === "object") {
        return Object.fromEntries(
          Object.entries(v).filter(([k]) => k !== "generated_at").map(([k, x]) => [k, strip(x)]),
        );
      }
      return v;
    };
    const data = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    fs.writeFileSync(process.argv[1], JSON.stringify(strip(data), null, 2) + "\n");
  ' "$1" || fail "could not normalize: $1"
}

# Red-case proof, exercised on every gate run: two documents that differ ONLY
# in generated_at (at any nesting depth) must compare equal after
# normalization, while a real payload difference must still compare unequal.
selftest_dir="$out_dir/determinism/normalize-selftest"
mkdir -p "$selftest_dir"
printf '%s\n' '{"generated_at":"2026-10-03T10:00:00Z","summary":{"total_sessions":3},"nested":{"generated_at":"2026-10-03T10:00:00Z","keep":true}}' \
  >"$selftest_dir/tick-a.json"
printf '%s\n' '{"generated_at":"2026-10-03T10:00:01Z","summary":{"total_sessions":3},"nested":{"generated_at":"2026-10-03T10:00:01Z","keep":true}}' \
  >"$selftest_dir/tick-b.json"
printf '%s\n' '{"generated_at":"2026-10-03T10:00:00Z","summary":{"total_sessions":4},"nested":{"generated_at":"2026-10-03T10:00:00Z","keep":true}}' \
  >"$selftest_dir/payload-b.json"
normalize "$selftest_dir/tick-a.json"
normalize "$selftest_dir/tick-b.json"
normalize "$selftest_dir/payload-b.json"
cmp -s "$selftest_dir/tick-a.json" "$selftest_dir/tick-b.json" \
  || fail "normalization self-test: generated_at-only difference must compare equal"
if cmp -s "$selftest_dir/tick-a.json" "$selftest_dir/payload-b.json"; then
  fail "normalization self-test: a real payload difference must still compare unequal"
fi

generated_files=()
for i in 1 2 3; do
  generated_files+=(
    "$out_dir/determinism/latest-$i.json"
    "$out_dir/determinism/overview-$i.json"
    "$out_dir/determinism/baseline-$i.json"
  )
done

for i in 1 2 3; do
  "$bin" --demo --latest -f json >"$out_dir/determinism/latest-$i.json"
  "$bin" --demo --overview -f json >"$out_dir/determinism/overview-$i.json"
done

for i in 1 2 3; do
  "$bin" --demo --overview -f json --baseline "$out_dir/determinism/overview-1.json" \
    >"$out_dir/determinism/baseline-$i.json"
done

for path in "${generated_files[@]}"; do
  node -e 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))' "$path" \
    || fail "invalid JSON: $path"
done

for path in "${generated_files[@]}"; do
  normalize "$path"
done

cmp -s "$out_dir/determinism/latest-1.json" "$out_dir/determinism/latest-2.json" \
  || fail "--demo --latest -f json changed between run 1 and 2"
cmp -s "$out_dir/determinism/latest-1.json" "$out_dir/determinism/latest-3.json" \
  || fail "--demo --latest -f json changed between run 1 and 3"
cmp -s "$out_dir/determinism/overview-1.json" "$out_dir/determinism/overview-2.json" \
  || fail "--demo --overview -f json changed between run 1 and 2"
cmp -s "$out_dir/determinism/overview-1.json" "$out_dir/determinism/overview-3.json" \
  || fail "--demo --overview -f json changed between run 1 and 3"
cmp -s "$out_dir/determinism/baseline-1.json" "$out_dir/determinism/baseline-2.json" \
  || fail "--demo --overview -f json --baseline changed between run 1 and 2"
cmp -s "$out_dir/determinism/baseline-1.json" "$out_dir/determinism/baseline-3.json" \
  || fail "--demo --overview -f json --baseline changed between run 1 and 3"

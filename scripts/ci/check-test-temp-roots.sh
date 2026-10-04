#!/usr/bin/env bash
# check-test-temp-roots: every fixed test root under the shared temp
# namespace must carry a pid+thread unique-suffix component
# (test-flake-prevention.md Rule 2 / Rule 7; rm-429).
#
# A bare temp_dir().join("agenttrace-<name>") is shared mutable state
# across every cargo suite on the host: a concurrently running suite
# recreates/deletes the same directory mid-run and the owner suite
# fails with ENOENT on a clean HEAD (live case 2026-10-04,
# agenttrace-rm346b-baseline, run 5bec3c93 cycle 2 assess F2).
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
fail() {
  echo "check-test-temp-roots: $*" >&2
  exit 1
}

# Rust tests: any temp_dir().join("agenttrace…") literal without a
# format! (pid/thread) component is an unsuffixed fixed root.
status=0
while IFS=: read -r file line text; do
  echo "check-test-temp-roots: unsuffixed fixed temp root at ${file}:${line}: ${text}" >&2
  status=1
done < <(
  grep -rn 'temp_dir()\.join("agenttrace' "$root/crates" --include='*.rs' 2>/dev/null || true
)

[[ $status -eq 0 ]] || fail "fixed test temp roots violate flake-prevention Rule 2/7 (see above)"
echo "check-test-temp-roots: all test temp roots uniquely suffixed"

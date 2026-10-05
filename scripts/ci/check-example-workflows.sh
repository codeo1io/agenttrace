#!/usr/bin/env bash
# rm-405: keep the shipped GitHub Actions example supply-chain honest.
#
# The example workflow is documentation that people copy; whatever it teaches
# becomes their practice. This gate fails when the example regresses to:
#   * a mutable action ref (tag/branch instead of a full commit SHA)
#   * an install that does not verify a pinned release artifact checksum
#   * a health-gate step that suppresses its own exit code (|| true)
#   * curl|sh from a moving branch head
set -euo pipefail
cd "$(dirname "$0")/../.."

example="examples/github-actions/agenttrace-health-gate.yml"
if [ ! -f "$example" ]; then
  echo "FAIL: missing example workflow ${example}" >&2
  exit 1
fi

fail=0

# Every `uses:` must pin a full 40-hex commit SHA (deny-by-default): a
# version tag (@v7.0.0) or branch name is still a mutable ref — tags and
# branches can be moved after the fact (rm-405 review finding 2: the old
# allowlist regex only caught bare-major tags/branches and let @v7.0.0 pass).
unpinned="$(grep -nE '^[[:space:]]*-?[[:space:]]*uses:' "$example" | grep -vE '^[0-9]+:[[:space:]]*-?[[:space:]]*uses:[[:space:]]+[^@[:space:]]+@[0-9a-f]{40}[[:space:]]*$' || true)"
if [ -n "$unpinned" ]; then
  echo "FAIL: uses: ref(s) not pinned to a full 40-hex commit SHA in ${example}:" >&2
  echo "$unpinned" >&2
  fail=1
fi

if ! grep -q 'sha256sum -c -' "$example"; then
  echo "FAIL: install step lacks a sha256sum -c verification" >&2
  fail=1
fi

if ! grep -qE 'releases/download/v[0-9]+' "$example"; then
  echo "FAIL: install does not fetch a pinned release tag" >&2
  fail=1
fi

if grep -qE 'curl[^|]*luoyuctl/agenttrace/(master|main|HEAD)/' "$example"; then
  echo "FAIL: install script fetched from a moving ref" >&2
  fail=1
fi

# The gate step must not swallow its exit code; report-only steps (marked
# `if: always()`) legitimately keep `|| true`.
gate_start="$(grep -n 'name: Check agent session health' "$example" | cut -d: -f1 | head -1)"
if [ -n "$gate_start" ]; then
  gate_end="$((gate_start + 1))"
  total="$(wc -l < "$example")"
  while [ "$gate_end" -le "$total" ]; do
    line="$(sed -n "${gate_end}p" "$example")"
    case "$line" in
      *'- name:'*) break ;;
    esac
    gate_end="$((gate_end + 1))"
  done
  gate_block="$(sed -n "${gate_start},$((gate_end - 1))p" "$example")"
  # Strip YAML comments first: prose explaining the report-only policy may
  # legitimately mention `|| true` — only actual run lines count.
  gate_code="$(printf '%s\n' "$gate_block" | sed 's/[[:space:]]*#.*//')"
  if printf '%s\n' "$gate_code" | grep -q '|| true'; then
    echo "FAIL: the health-gate step suppresses its own exit code (|| true)" >&2
    fail=1
  fi
  if ! printf '%s\n' "$gate_code" | grep -qE 'fail-under-health'; then
    echo "FAIL: the health-gate step no longer enforces a health threshold" >&2
    fail=1
  fi
else
  echo "FAIL: gate step 'Check agent session health' not found in ${example}" >&2
  fail=1
fi

if [ "$fail" -eq 0 ]; then
  echo "example workflow supply-chain pinning: ok"
fi
exit "$fail"

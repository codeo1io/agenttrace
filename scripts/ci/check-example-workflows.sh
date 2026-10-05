#!/usr/bin/env bash
# rm-405: keep the shipped GitHub Actions example supply-chain honest.
#
# The example workflow is documentation that people copy; whatever it teaches
# becomes their practice. This gate fails when the example regresses to:
#   * a mutable action ref (tag/branch instead of a full commit SHA)
#   * an install that does not verify a pinned release artifact checksum
#   * a health-gate step that suppresses its own exit code (|| true)
#   * curl|sh from a moving branch head
#
# rm-417 residual: docs/guides/ci-integration.md ships its own copy-paste
# yaml with the same audience, so the same rules apply to its fenced block.
# (Found live at gate-writing time: the guide taught
# `raw.githubusercontent.com/.../master/install.sh | sh` while this gate
# denied exactly that pattern for the example — the divergence this rule
# now makes impossible to reintroduce silently.)
set -euo pipefail
cd "$(dirname "$0")/../.."

example="examples/github-actions/agenttrace-health-gate.yml"
guide="docs/guides/ci-integration.md"
for f in "$example" "$guide"; do
  if [ ! -f "$f" ]; then
    echo "FAIL: missing ${f}" >&2
    exit 1
  fi
done

fail=0

# check_yaml <label> <yaml-file>
# Shared deny-by-default rule set for every copy-paste workflow this repo
# ships (the example, and the guide's fenced block).
check_yaml() {
  local label="$1" path="$2"

  # Every `uses:` must pin a full 40-hex commit SHA: a version tag (@v7.0.0)
  # or branch name is still a mutable ref — tags and branches can be moved
  # after the fact (rm-405 review finding 2: the old allowlist regex only
  # caught bare-major tags/branches and let @v7.0.0 pass).
  local unpinned
  unpinned="$(grep -nE '^[[:space:]]*-?[[:space:]]*uses:' "$path" | grep -vE '^[0-9]+:[[:space:]]*-?[[:space:]]*uses:[[:space:]]+[^@[:space:]]+@[0-9a-f]{40}[[:space:]]*$' || true)"
  if [ -n "$unpinned" ]; then
    echo "FAIL: uses: ref(s) not pinned to a full 40-hex commit SHA in ${label}:" >&2
    echo "$unpinned" >&2
    fail=1
  fi

  if ! grep -q 'sha256sum -c -' "$path"; then
    echo "FAIL: install step lacks a sha256sum -c verification in ${label}" >&2
    fail=1
  fi

  if ! grep -qE 'releases/download/v[0-9]+' "$path"; then
    echo "FAIL: install does not fetch a pinned release tag in ${label}" >&2
    fail=1
  fi

  if grep -qE 'curl[^|]*luoyuctl/agenttrace/(master|main|HEAD)/' "$path"; then
    echo "FAIL: install script fetched from a moving ref in ${label}" >&2
    fail=1
  fi

  # The gate step must not swallow its exit code; report-only steps (marked
  # `if: always()`) legitimately keep `|| true`.
  local gate_start gate_end total line gate_block gate_code
  gate_start="$(grep -n 'name: Check agent session health' "$path" | cut -d: -f1 | head -1)"
  if [ -n "$gate_start" ]; then
    gate_end="$((gate_start + 1))"
    total="$(wc -l < "$path")"
    while [ "$gate_end" -le "$total" ]; do
      line="$(sed -n "${gate_end}p" "$path")"
      case "$line" in
        *'- name:'*) break ;;
      esac
      gate_end="$((gate_end + 1))"
    done
    gate_block="$(sed -n "${gate_start},$((gate_end - 1))p" "$path")"
    # Strip YAML comments first: prose explaining the report-only policy may
    # legitimately mention `|| true` — only actual run lines count.
    gate_code="$(printf '%s\n' "$gate_block" | sed 's/[[:space:]]*#.*//')"
    if printf '%s\n' "$gate_code" | grep -q '|| true'; then
      echo "FAIL: the health-gate step suppresses its own exit code (|| true) in ${label}" >&2
      fail=1
    fi
    if ! printf '%s\n' "$gate_code" | grep -qE 'fail-under-health'; then
      echo "FAIL: the health-gate step no longer enforces a health threshold in ${label}" >&2
      fail=1
    fi
  else
    echo "FAIL: gate step 'Check agent session health' not found in ${label}" >&2
    fail=1
  fi
}

check_yaml "$example" "$example"

# rm-417 residual: run the same rule set over the guide's fenced yaml.
guide_block="$(mktemp)"
trap 'rm -f "$guide_block"' EXIT
awk '/^```yaml$/ {flag=1; next} /^```$/ && flag {exit} flag' "$guide" > "$guide_block"
if [ ! -s "$guide_block" ]; then
  echo "FAIL: no \`\`\`yaml block found in ${guide}" >&2
  fail=1
else
  check_yaml "$guide (yaml block)" "$guide_block"
fi

if [ "$fail" -eq 0 ]; then
  echo "example workflow supply-chain pinning: ok"
fi
exit "$fail"

#!/usr/bin/env bash
# Fails if any GitHub workflow pins a non-portable runner. Local runner
# labels (self-hosted, LAN hostnames) are operator policy and must never
# reach upstream-bound branches (AGENTS.md rule 3); upstream PR #282 was
# closed unmerged for exactly this contamination.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
workflows="${root}/.github/workflows"

fail() {
  echo "check-no-self-hosted: $*" >&2
  exit 1
}

[[ -d "$workflows" ]] || fail "workflows directory not found: $workflows"

status=0
while IFS= read -r hit; do
  echo "check-no-self-hosted: $hit" >&2
  status=1
done < <(grep -rn "runs-on:.*self-hosted" "$workflows" || true)

if [[ "$status" -ne 0 ]]; then
  fail "workflows must use runs-on: ubuntu-latest (or the portable repo default)"
fi

echo "check-no-self-hosted: all workflows use portable runners"

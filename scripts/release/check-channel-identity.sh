#!/usr/bin/env bash
# rm-196 — channel identity guard.
#
# The distribution identity (which GitHub repository release channels install
# from) must be defined exactly once per surface and identically on every
# surface:
#
#   scripts/release/render-channels.sh   repo="${CHANNEL_REPO:-<repo>}"
#   install.sh                           REPO="<repo>"
#   install.ps1                          $REPO = "<repo>"
#   npm/scripts/install.js               const REPOSITORY = "<repo>";
#
# render-channels.sh is redirected at release time via CHANNEL_REPO (the
# release workflow injects github.repository, so channels always resolve the
# repository actually being released), but the baked defaults must stay in
# lockstep: they are the upstream channel every installer fetches. The
# third-party registry submission lanes in .github/workflows/release.yml are
# gated to UPSTREAM_REPO, which must carry the same identity.
#
# Exit code 0 = every surface has exactly one identical definition.
# Any violation is listed on stderr with exit code 1.

set -euo pipefail

expected="${CHANNEL_IDENTITY:-luoyuctl/agenttrace}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
fail=0

check() {
	local label="$1" file="$2" extract="$3"
	local values
	values="$(sed -n "$extract" "$file")"
	local count
	count="$(printf '%s\n' "$values" | grep -c . || true)"

	if [[ "$count" -eq 0 ]]; then
		echo "FAIL $label: no identity definition found in $file" >&2
		fail=1
		return
	fi
	if [[ "$count" -gt 1 ]]; then
		echo "FAIL $label: $count identity definitions in $file (must be exactly one):" >&2
		printf '%s\n' "$values" | sed 's/^/    /' >&2
		fail=1
		return
	fi
	if [[ "$values" != "$expected" ]]; then
		echo "FAIL $label: identity is \"$values\" but expected \"$expected\":" >&2
		fail=1
		return
	fi
	echo "ok   $label: $expected"
}

check "render-channels.sh" "$root/scripts/release/render-channels.sh" \
	's/^repo="\${CHANNEL_REPO:-\([^}]*\)}"$/\1/p'
check "install.sh" "$root/install.sh" \
	's/^REPO="\([^"]*\)"$/\1/p'
check "install.ps1" "$root/install.ps1" \
	's/^\$REPO = "\([^"]*\)"$/\1/p'
check "npm/scripts/install.js" "$root/npm/scripts/install.js" \
	's/^const REPOSITORY = "\([^"]*\)";$/\1/p'

# The release workflow's submission-lane gate must carry the same identity.
if ! grep -Fq "UPSTREAM_REPO: $expected" "$root/.github/workflows/release.yml"; then
	echo "FAIL release.yml: missing \"UPSTREAM_REPO: $expected\" (submission-lane gate identity)" >&2
	fail=1
else
	echo "ok   release.yml gate: UPSTREAM_REPO: $expected"
fi

exit "$fail"

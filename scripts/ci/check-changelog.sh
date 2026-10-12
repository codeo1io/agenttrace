#!/usr/bin/env bash
set -euo pipefail

# CHANGELOG shape gate (rm-1366, run 43db92ef): within any single version
# section (`## ...`), each kind heading (`### Added`, `### Changed`,
# `### Deprecated`, `### Removed`, `### Fixed`, `### Security`) may appear
# at most once. Duplicate kind-headings render as repeated blocks and
# accrete whenever an entry is appended with a fresh heading instead of
# merged under the existing one (observed 2026-10-13: the Unreleased
# section carried 5x Added / 4x Fixed / 1x Changed). Convention: add
# entries under the EXISTING kind heading of the section.
#
# No-tag-fallback like the other doc gates: this checks the working tree's
# CHANGELOG.md directly. Accepts an optional path argument (used by probes
# to test perturbed copies outside the repository).

changelog="${1:-$(cd "$(dirname "$0")/../.." && pwd)/CHANGELOG.md}"

fail() {
	echo "check-changelog: $*" >&2
	exit 1
}

[ -f "$changelog" ] || fail "CHANGELOG not found at $changelog"

awk '
	/^## / { section = $0; delete seen; next }
	/^### / {
		if ($0 in seen) {
			printf "duplicate kind-heading in %s: %s (line %d)\n", section, $0, NR > "/dev/stderr"
			bad = 1
		}
		seen[$0] = 1
	}
	END { exit bad ? 1 : 0 }
' "$changelog" || fail "at most one kind-heading per version section — merge entries under the existing heading"

echo "check-changelog: shape OK ($changelog)"

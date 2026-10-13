#!/usr/bin/env bash
set -euo pipefail

# rm-1036 structure gate (run 4aaa4c9fd914 cycle 2, batch "release-artifact
# truth"). CHANGELOG.md's Unreleased section had accumulated duplicate
# `###` section headings (Added at :5/:44/:353/:418/:437, Fixed at
# :9/:17/:52/:446 at the 82e23b9 base). Keep-a-changelog consumers that
# take the first block per section kind silently DROP every later block —
# and the later blocks carried live entries (the schema-58/59/60
# cache-bump notes, the pi zero-usage fixes). This gate fails the moment
# any `##` section of CHANGELOG.md carries two `###` headings of the same
# kind, so the duplication cannot re-accumulate silently. It also checks
# the exact lens under which the old file lost entries: a first-per-kind
# parse of Unreleased must see every entry the section holds.

fail() {
	echo "check-changelog-structure: $*" >&2
	exit 1
}

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
changelog="$repo_root/CHANGELOG.md"

[ -f "$changelog" ] || fail "CHANGELOG.md not found at $changelog"

# Pass 1: every `### <kind>` must live inside a `## <section>`, and no
# section may see the same kind twice.
if ! dups="$(awk '
	/^## / { section = $0; sub(/^## +/, "", section); next }
	/^### / {
		kind = $0; sub(/^### +/, "", kind)
		if (section == "") {
			printf("  ### %s at line %d appears before any ## section\n", kind, NR)
			bad = 1; next
		}
		key = section "\t" kind
		if (seen[key]) {
			printf("  section [%s] repeats [%s]: first at line %d, again at line %d\n", section, kind, seen[key], NR)
			bad = 1
		} else {
			seen[key] = NR
		}
	}
	END { exit bad ? 1 : 0 }
' "$changelog")"; then
	fail "duplicate or orphaned section headings in CHANGELOG.md:"$'\n'"$dups"
fi

# Pass 2: the keep-a-changelog lens. A consumer that keeps only the FIRST
# block per section kind (what the duplication used to break) must see
# every Unreleased entry: count bullets visible to that consumer against
# the section total.
counts="$(awk '
	/^## / { unreleased = ($0 == "## Unreleased"); active = 1; next }
	unreleased && /^### / {
		if (!($0 in seen)) { seen[$0] = 1; active = 1 } else { active = 0 }
	}
	unreleased && /^- / {
		total++
		if (active) visible++
	}
	END { printf "%d %d\n", visible + 0, total + 0 }
' "$changelog")"
visible="${counts%% *}"
total="${counts##* }"
[ "$visible" -eq "$total" ] ||
	fail "a first-per-kind parse of Unreleased hides entries: $visible visible of $total — duplicate section headings"

echo "check-changelog-structure: no duplicate section kinds; first-per-kind Unreleased parse sees all $total entries"

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

# Pass 0 (rm-1928, run 9d3b927c cycle 1): unresolved merge-conflict
# fences. The standing tree's CHANGELOG.md carried committed
# `<<<<<<<` / `=======` / `>>>>>>>` markers through the 2026-10-14 fog
# while this gate returned rc0 on it — a half-merged changelog passed
# CI. Fences are rejected BEFORE any structural check so the failure
# names the real problem. Exactly seven `=` is the git separator
# shape; markdown setext underlines (three or four `=` under a text
# line) stay legal. Fence lines print so the operator sees where.
if fences="$(grep -nE '^(<<<<<<<|>>>>>>>)|^=======$' "$changelog")" && [ -n "$fences" ]; then
	fail "unresolved merge-conflict fence(s) present in CHANGELOG.md:"$'\n'"$fences"
fi

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
# the section total. rm-1390 (run bbf61fbd cycle 3): the lens matched only
# the bare `## Unreleased` spelling, so a DATED header (`## Unreleased
# (2026-10-14)`) turned the whole pass into a vacuous rc0 "all 0
# entries" — the exact shape of the assess vac3 PoC. The lens now
# matches the header by prefix, dated or not.
counts="$(awk '
	/^## Unreleased/ { unreleased = 1; active = 1; next }
	/^## / { unreleased = 0; active = 1; next }
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

# Pass 3 (rm-1390): a PRESENT-but-vacuous Unreleased section is the same
# standing red as a malformed one. The header claims a landing lane; a
# section over zero entry bullets claims it falsely. Only enforced when
# the section exists — rm-1036 deliberately does not require an
# Unreleased header at all.
if grep -q '^## Unreleased' "$changelog"; then
	[ "$total" -ge 1 ] ||
		fail "Unreleased section is present but carries no entries (rm-1390): a header over zero bullets claims a landing lane that does not exist — land the entries, date the section, or drop the header"
fi

# Pass 4 (rm-2338, run 252064e2 cycle 4): verbatim-duplicated ENTRY lines.
# Pass 1 catches duplicated `###` headings; the same re-doubling landing can
# also duplicate whole entry bullets inside one section (2026-10-22: 39
# duplicated entry blocks / 132 duplicated long lines in Unreleased). The
# significance floor (40 chars) keeps short generic bullets and repeated
# `Integration seam (...)` continuation annotations inside different
# entries legal — only verbatim re-doubled entry text fails.
dup_entries="$(awk '
	BEGIN { FS = "\n" }
	/^## / { section = $0 }
	/^- / {
		if (length($0) > 40) {
			key = section SUBSEP $0
			seen[key]++
			if (seen[key] == 2) { printf "    %s: %s\n", section, $0; bad = 1 }
		}
	}
	END { exit bad ? 1 : 0 }
' "$changelog")" || true
if grep -q . <<<"$dup_entries"; then
	fail "verbatim duplicate entry line in CHANGELOG.md (rm-2338 class — a re-doubled landing pasted the same entry twice):"$'\n'"$dup_entries"
fi

echo "check-changelog-structure: no merge-conflict fences; no duplicate section kinds; first-per-kind Unreleased parse sees all $total entries (non-vacuous); no duplicated entry lines"

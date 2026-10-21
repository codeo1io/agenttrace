#!/usr/bin/env bash
set -euo pipefail

# rm-2118 per-file size-budget gate (run d2775a7e cycle 6, batch "Pricing
# currency restored"; assess F3 supplied the concentration data): the fork's
# Rust sources concentrate hard — six tracked crates/**.rs files over 5,000
# lines (parser.rs 13,984; session_cache.rs 6,592; main.rs 5,685; lib.rs
# 5,568; statusline.rs 5,236; discovery_contract.rs 5,140 at the de3168de
# wall) and nothing bounded their growth. This gate does not shrink
# anything and does not forbid big files; it makes every next line past a
# ceiling a conscious, reviewable event instead of silent accretion:
#
#   - every tracked crates/**.rs file is pinned to a line-count ceiling;
#   - the allow-list below pins the known-big files at their measured
#     count rounded UP to the next 100 (so a listed file has ≤99 lines of
#     slack — enough to live, not enough to hide a second subsystem);
#   - every UNLISTED file gets the DEFAULT_CAP (5,000) — a file crossing
#     it must be added to the allow-list, which is a diff a reviewer sees;
#   - raising any ceiling or adding any entry requires a dated comment
#     naming the reason (same disclosure discipline as
#     scripts/pricing/snapshot-drops.txt and the rm-588 shrink-check).
#
# Exit contract: 0 = every tracked file within its ceiling; 1 = at least
# one file over (or a stale allow-list entry — the list must not rot);
# 2 = infra error (not run from a git checkout, no sources found).

fail() {
	echo "check-file-size-budget: $*" >&2
	exit 1
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# The measured wall (2026-10-20, de3168de + this batch) — ceilings are
# measured-then-rounded-up; every entry below must carry its dated
# justification. Raise an entry ONLY with a new dated comment saying why.
#
# 2026-10-20 (run d2775a7e, rm-2118): initial allow-list at the measured
# wall — the six >5k files named above; rm-164 arm 3 added the 1h-tier
# class to lib.rs (measured 5,568 → ceiling 5,600 after that batch).
default_cap=5000
allow_list="
crates/agenttrace-core/src/parser.rs:14000
crates/agenttrace-core/src/session_cache.rs:6600
crates/agenttrace-cli/src/main.rs:5700
crates/agenttrace-core/src/lib.rs:5600
crates/agenttrace-core/src/statusline.rs:5300
crates/agenttrace-core/tests/discovery_contract.rs:5200
"

git -C "$repo_root" rev-parse --git-dir >/dev/null 2>&1 || { echo "check-file-size-budget: not a git checkout ($repo_root)" >&2; exit 2; }

mapfile -t files < <(cd "$repo_root" && git ls-files 'crates/**/*.rs')
[ "${#files[@]}" -gt 0 ] || { echo "check-file-size-budget: no tracked crates/**.rs sources found" >&2; exit 2; }

over=0

ceiling_for() {
	local path="$1" entry
	while IFS= read -r entry; do
		[ -n "$entry" ] || continue
		if [ "${entry%%:*}" = "$path" ]; then
			echo "${entry##*:}"
			return 0
		fi
	done <<<"$allow_list"
	echo "$default_cap"
}

listed_paths() {
	local entry
	while IFS= read -r entry; do
		[ -n "$entry" ] || continue
		echo "${entry%%:*}"
	done <<<"$allow_list"
}

for path in "${files[@]}"; do
	count="$(wc -l <"$repo_root/$path")"
	ceiling="$(ceiling_for "$path")"
	if [ "$count" -gt "$ceiling" ]; then
		echo "  OVER: $path is $count lines (ceiling $ceiling)" >&2
		over=1
	fi
done

# The allow-list must reference real, tracked files — a stale entry (file
# moved, renamed, or deleted) fails the gate instead of rotting silently.
while IFS= read -r listed; do
	[ -n "$listed" ] || continue
	if [ ! -f "$repo_root/$listed" ] || ! git -C "$repo_root" ls-files --error-unmatch "$listed" >/dev/null 2>&1; then
		echo "  STALE allow-list entry (not a tracked file): $listed" >&2
		over=1
	fi
done < <(listed_paths)

if [ "$over" -ne 0 ]; then
	fail "file(s) over their size ceiling (or stale allow-list entries). Grow a file past its ceiling only with a dated justification comment in scripts/ci/check-file-size-budget.sh's allow-list — name the reason, then raise the MINIMUM amount (measured, rounded up to the next 100). Consider splitting instead."
fi

echo "check-file-size-budget: OK — ${#files[@]} tracked crates/**.rs files within their ceilings (default cap $default_cap)"

#!/usr/bin/env bash
set -euo pipefail

# rm-157, cycle 1 ("Truthful posture, enforced gates"); rm-210 blind-spot
# closure (cycle 2, run 51ca93a91b6d).
#
# The reviewed Cargo.lock is a security surface: rm-009 pinned rustls
# through it and the deny job gates its advisories, but that guarantee
# stays convention-only unless every dependency-resolving cargo
# invocation in CI passes --locked (or --frozen, which implies --locked
# and adds --offline) and therefore refuses to regenerate the lockfile
# at build time. This gate fails when such an invocation in
# .github/workflows/*.yml or *.yaml lacks the flag, so adding a new
# unpinned invocation breaks CI instead of silently drifting the
# reviewed lock.
#
# Scanner contract (rm-210 — the three recorded blind spots, closed):
#   1. *.yaml is scanned alongside *.yml.
#   2. Backslash line continuations are joined into one logical line
#      BEFORE matching, so a wrapped `cargo \` + `test` can no longer
#      hide the verb, while a wrapped `cargo test \` + `--locked` no
#      longer false-trips (the flag may ride on any joined line).
#   3. Every CORE dependency-resolving cargo verb is gated:
#      bench build check clippy doc fetch fix install run rustc
#      test tree. The exact accepted set, deliberately NOT gated:
#      - `cargo fmt` — resolves no dependencies.
#      - `cargo update` / `cargo add` / `cargo rm` — explicit lockfile
#        mutations, loud by design rather than silent drift.
#      - external subcommands (`cargo llvm-cov`, `cargo deny`,
#        `cargo nextest`, ...) — third-party verbs cannot be
#        enumerated; convention carries their --locked discipline.
#      A `+toolchain` argument between `cargo` and the verb IS
#      understood (`cargo +1.89 check`). `${{ ... }}` expression
#      interpolation before the verb is not resolved (no such usage in
#      this repo; keep it that way).
# Full-line comments (leading whitespace + `#`) are skipped —
# commented-out invocations do not execute. Mid-line `#` is left in
# place: a `#` inside quotes may be a literal, and the pre-rm-210
# scanner never stripped comments either.
#
# Local helper scripts (scripts/ci/check-rust-release-local.sh) and the
# user-facing install-from-source paths (install.sh, install.ps1) are
# out of scope by design — they run on arbitrary checkouts where the
# lockfile may legitimately need regenerating.
#
# Usage:
#   scripts/ci/check-locked-cargo.sh              # scan .github/workflows
#   scripts/ci/check-locked-cargo.sh <dir>        # scan an arbitrary
#                                                 # directory (temp-dir
#                                                 # test mode)
#   scripts/ci/check-locked-cargo.sh --self-test  # run the bundled
#                                                 # fixtures under
#                                                 # scripts/ci/testdata/locked-cargo

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixtures="$repo_root/scripts/ci/testdata/locked-cargo"

fail() {
	echo "check-locked-cargo: $*" >&2
	exit 1
}

# Prints one "path:startline: joined-invocation" line per offender;
# startline is the line where the (possibly wrapped) invocation begins.
# shellcheck disable=SC2016  # the awk program below must not expand
scan_dir() {
	local dir=$1
	[[ -d $dir ]] || fail "no such scan dir: $dir"
	find "$dir" -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) -print0 |
		sort -z |
		xargs -0 -r awk '
			function flush_line() {
				if (logical == "") { start = 0; return }
				if (logical ~ verb_re && logical !~ lock_re) {
					msg = logical
					gsub(/^[[:space:]]+/, "", msg)
					gsub(/[[:space:]]+/, " ", msg)
					printf "%s:%d: %s\n", FILENAME, start, msg
				}
				logical = ""
				start = 0
			}
			BEGIN {
				verb_re = "cargo[[:space:]]+(\\+[A-Za-z0-9._-]+[[:space:]]+)?(bench|build|check|clippy|doc|fetch|fix|install|run|rustc|test|tree)([^A-Za-z0-9_-]|$)"
				lock_re = "--(locked|frozen)([^A-Za-z0-9_-]|$)"
			}
			{
				line = $0
				sub(/\r$/, "", line)
				if (line ~ /^[[:space:]]*#/) next      # full-line comment
				if (line ~ /^[[:space:]]*$/) { flush_line(); next }
				if (start == 0) start = FNR
				logical = logical line                # \<newline> removal joins without a separator
				t = length(logical); b = 0
				while (t > 0 && substr(logical, t, 1) == "\\") { b++; t-- }
				if (b % 2 == 1) {                      # odd trailing backslashes = one continuation
					logical = substr(logical, 1, length(logical) - 1)
					next
				}
				flush_line()
			}
			END { flush_line() }
		'
}

scan_and_report() {
	local dir=$1 label=$2
	local offenders
	offenders="$(scan_dir "$dir")" || fail "scanner failed on $dir"
	if [[ -n $offenders ]]; then
		echo "check-locked-cargo: dependency-resolving cargo invocations without --locked/--frozen ($label):" >&2
		while IFS= read -r offender; do
			echo "  $offender" >&2
		done <<<"$offenders"
		return 1
	fi
	return 0
}

if [[ ${1:-} == "--self-test" ]]; then
	[[ -d $fixtures ]] || fail "self-test fixtures missing: $fixtures"
	clean_dir="$fixtures/clean"
	violations_dir="$fixtures/violations"

	# 1. Clean corpus (incl. the old false-positive shape: a wrapped
	#    `cargo test` + `--locked` split across lines, in a *.yaml the
	#    old glob never scanned) must produce zero offenders.
	clean_out="$(scan_dir "$clean_dir")" || fail "scanner failed on the clean corpus"
	if [[ -n $clean_out ]]; then
		echo "check-locked-cargo self-test: clean corpus flagged offenders:" >&2
		echo "$clean_out" >&2
		fail "self-test: clean corpus must be clean"
	fi

	# 2. Violation corpus must be flagged exactly once per EXPECT-TRIP
	#    marker, and MUST include the rm-210 acceptance fixture: a
	#    backslash-wrapped verb in a *.yaml file, reported at its first
	#    line with the joined invocation.
	violation_out="$(scan_dir "$violations_dir")" || fail "scanner failed on the violation corpus"
	expected="$(grep -rhoE "#[[:space:]]*EXPECT-TRIP[[:space:]]*$" "$violations_dir" | wc -l)"
	actual="$(grep -c . <<<"$violation_out" || true)"
	[[ $actual -eq $expected ]] ||
		fail "self-test: expected $expected offenders (one per EXPECT-TRIP marker), got $actual: $violation_out"
	echo "$violation_out" | grep -qE "wrapped\.yaml:[0-9]+: cargo " ||
		fail "self-test: the wrapped-yaml acceptance fixture was not flagged at its first line"
	echo "check-locked-cargo: self-test green ($expected/$expected synthetic offenders flagged; clean corpus clean)"
	exit 0
elif [[ $# -eq 0 ]]; then
	scan_and_report "$repo_root/.github/workflows" "repo workflows" ||
		fail "all workflow dependency-resolving cargo invocations must pass --locked (or --frozen)"
	echo "check-locked-cargo: all workflow dependency-resolving cargo invocations are --locked/--frozen"
else
	[[ $# -eq 1 ]] || fail "usage: $0 [SCAN_DIR | --self-test]"
	scan_and_report "$1" "scan dir" ||
		fail "dependency-resolving cargo invocations without --locked/--frozen in $1"
	echo "check-locked-cargo: all dependency-resolving cargo invocations in $1 are --locked/--frozen"
fi

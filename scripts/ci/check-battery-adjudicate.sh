#!/usr/bin/env bash
set -euo pipefail

# Battery-red environmental adjudicator (rm-1505 → lands as rm-1507 after
# the same-minute numeral collision with run 51fff5de's band; run a1f6c1f8
# cycle 2, batch "assess-truth closeout"). A `cargo test --workspace`
# battery that goes red from ONE failure forces a manual ritual every
# conductor run repeats: re-run that single test isolated and decide
# whether the red is environmental (stale TMPDIR, shared-target
# contention, corpus state) or a real defect, before the commit gate can
# bank a standing-red classification. This tool performs the ritual and
# emits a TYPED verdict the gate can bank mechanically:
#
#   environmental — red in the battery, green on every isolated re-run
#                   (exit 0; the red was the environment, not the code)
#   flaky         — red in the battery, mixed isolated re-runs
#                   (exit 0; disclosed subclass — do not bank as green)
#   defect        — red in the battery, red on every isolated re-run
#                   (exit 1; the code fails on its own terms)
#
# Isolation per attempt: a fresh mktemp TMPDIR, `--exact` single-test
# filter, `-p <pkg> [--test <target>]` scoping. The re-run command is
# echoed with the verdict so any verdict is independently reproducible.
#
#   scripts/ci/check-battery-adjudicate.sh -p agenttrace -n admission_mode_word_bail_names_the_flag_form
#   scripts/ci/check-battery-adjudicate.sh -p agenttrace-core -t pi_journal_contract -n pi_cache_write_1h_blocks_disclosed --runs 5 --json
#
# AT_ADJUDICATE_RUNNER (test-only override): replaces the cargo
# invocation with a `bash -c` payload — this is how --selftest drives
# every verdict class synthetically (no cargo, no live fleet corpus).
# Never set in CI.
#
# Exit codes: 0 environmental|flaky (adjudicated non-defect), 1 defect,
# 2 usage/internal error.

# rm-241 repo-side arm (composition fix, conflict case
# 0c4406e6bb7744fe8f99e17bb2636bb0, independent review of run
# ce9309d9's "battery-truth hygiene" landing): this gate postdates
# the batch that armed the other cargo runners (run a1f6c1f8 landed
# it after base 6cd11e0, outside that batch's 9-script census) and
# ran cargo under whatever ambient CARGO_TARGET_DIR it inherited —
# the exact shared-layout hazard the arm exists for, sharpened by
# this tool's purpose: a stale sibling test binary here does not
# merely mis-test, it BANKS A FALSE VERDICT (a pre-fix binary
# passing every isolated re-run classifies a real defect as
# "environmental" and the commit gate banks the standing-red).
# Default — only when CARGO_TARGET_DIR is UNSET — to the same
# repo-root+HEAD-namespaced dir under the user cache the landed
# gates use; an explicitly exported CARGO_TARGET_DIR always wins,
# with the shared-layout note.
if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
	at_repo="$(git rev-parse --show-toplevel 2>/dev/null || true)"
	at_head="$(git rev-parse HEAD 2>/dev/null || true)"
	at_cache_home="${XDG_CACHE_HOME:-${HOME:-}/.cache}"
	if [[ -n "$at_repo" && -n "$at_head" && -n "${HOME:-}" && -d "$at_cache_home" ]]; then
		# git hash-object for portability (sha256sum is absent on macOS runners)
		at_ns="$(printf '%s:%s' "$at_repo" "$at_head" | git hash-object --stdin | cut -c1-16)"
		export CARGO_TARGET_DIR="$at_cache_home/agenttrace-dev/target/at-$at_ns"
	fi
else
	case "$CARGO_TARGET_DIR" in
	*/agenttrace-dev/target/at-*) ;;
	*)
		echo "check-battery-adjudicate: note: ambient CARGO_TARGET_DIR=$CARGO_TARGET_DIR looks like shared layout — it can serve stale sibling artifacts and bank a false verdict (rm-241); unset it to use the per-worktree namespaced default" >&2
		;;
	esac
fi

fail() {
	echo "check-battery-adjudicate: $*" >&2
	exit 2
}

usage() {
	cat >&2 <<'EOF'
usage: check-battery-adjudicate.sh -p <pkg> [-t <test-target>] -n <test-name> [--runs N] [--json] [--selftest]
  -p  cargo package (e.g. agenttrace, agenttrace-core)
  -t  test target (as in `cargo test --test`); default: none (package-wide single-test run)
  -n  exact test name (--exact filter)
  --runs N    isolated attempts (default 3)
  --json      additionally emit a one-line typed JSON record
  --selftest  offline synthetic verification of all verdict classes
EOF
	exit 2
}

PACKAGE='' TARGET='' TESTNAME='' RUNS=3 JSON=0 SELFTEST=0
while [ $# -gt 0 ]; do
	case "$1" in
	-p) [ $# -ge 2 ] || fail "-p needs a value"; PACKAGE=$2; shift 2 ;;
	-t) [ $# -ge 2 ] || fail "-t needs a value"; TARGET=$2; shift 2 ;;
	-n) [ $# -ge 2 ] || fail "-n needs a value"; TESTNAME=$2; shift 2 ;;
	--runs) [ $# -ge 2 ] || fail "--runs needs a value"; RUNS=$2; shift 2 ;;
	--json) JSON=1; shift ;;
	--selftest) SELFTEST=1; shift ;;
	-h | --help) usage ;;
	*) fail "unknown argument: $1" ;;
	esac
done

# One isolated attempt in a fresh TMPDIR. rc 0 = pass, nonzero = fail;
# never aborts the set -e caller (the failure IS the datum).
run_attempt() {
	local fresh rc
	fresh=$(mktemp -d "${TMPDIR:-/tmp}/at-adjud.XXXXXX") || fail "mktemp failed"
	if TMPDIR="$fresh" bash -c "$1" >/dev/null 2>&1; then
		rc=0
	else
		rc=$?
	fi
	rm -rf "$fresh"
	echo "$rc"
}

# `RUNS` isolated attempts of command $2; echo one rc per line.
adjudicate() {
	local cmd=$1 n=$2 i
	for ((i = 1; i <= n; i++)); do
		run_attempt "$cmd"
	done
}

# args: every attempt rc; echo environmental|defect|flaky
verdict_of() {
	local passed=0 failed=0 rc
	for rc in "$@"; do
		if [ "$rc" -eq 0 ]; then
			passed=$((passed + 1))
		else
			failed=$((failed + 1))
		fi
	done
	if [ "$passed" -eq "$#" ]; then
		echo environmental
	elif [ "$failed" -eq "$#" ]; then
		echo defect
	else
		echo flaky
	fi
}

verdict_exit() {
	case "$1" in
	environmental | flaky) exit 0 ;;
	defect) exit 1 ;;
	*) fail "internal: unknown verdict $1" ;;
	esac
}

# report <verdict> <attempt-rs...> — human line, optional typed JSON.
report() {
	local verdict=$1
	shift
	local results
	results=$(printf '%s,' "$@")
	results=${results%,}
	echo "VERDICT: $verdict (attempts: $results)"
	if [ "$JSON" -eq 1 ]; then
		printf '{"tool":"check-battery-adjudicate","verdict":"%s","runs":%s,"results":[%s]%s}\n' \
			"$verdict" "$#" "$results" "$JSON_EXTRA"
	fi
}

selftest() {
	local scratch arm verdict
	scratch=$(mktemp -d "${TMPDIR:-/tmp}/at-adjud-selftest.XXXXXX") || fail "mktemp failed"
	trap 'rm -rf "$scratch"' EXIT
	local rs=()

	# Arm 1 environmental: red ONLY under battery-mode conditions, green on
	# every isolated attempt — a runner that fails when (and only when) the
	# AT_BATTERY_MODE marker is set, mirroring a stale-TMPDIR / shared-state
	# battery red that cannot reproduce in a fresh environment.
	# shellcheck disable=SC2016  # payload must NOT expand here; the child expands it
	arm='[ -z "${AT_BATTERY_MODE:-}" ]'
	if AT_BATTERY_MODE=1 bash -c "$arm" >/dev/null 2>&1; then
		fail "selftest arm 1: synthetic battery runner unexpectedly green"
	fi
	mapfile -t rs < <(adjudicate "$arm" 3)
	verdict=$(verdict_of "${rs[@]}") || fail "arm 1 adjudication failed"
	[ "$verdict" = environmental ] || fail "selftest arm 1: expected environmental, got $verdict"

	# Arm 2 defect: red on every isolated attempt.
	mapfile -t rs < <(adjudicate 'exit 1' 3)
	verdict=$(verdict_of "${rs[@]}") || fail "arm 2 adjudication failed"
	[ "$verdict" = defect ] || fail "selftest arm 2: expected defect, got $verdict"

	# Arm 3 flaky: alternating parity across attempts — mixed re-runs must
	# be disclosed as flaky, never banked as environmental.
	echo 0 >"$scratch/parity"
	# shellcheck disable=SC2016  # payload must NOT expand here; the child expands it
	arm='n=$(cat "$AT_STATE"); echo $((n + 1)) > "$AT_STATE"; [ $((n % 2)) -eq 0 ]'
	mapfile -t rs < <(AT_STATE="$scratch/parity" adjudicate "$arm" 4)
	verdict=$(verdict_of "${rs[@]}") || fail "arm 3 adjudication failed"
	[ "$verdict" = flaky ] || fail "selftest arm 3: expected flaky, got $verdict"

	# Arm 4 isolation proof: every attempt must see a FRESH TMPDIR — a
	# runner that fails whenever the previous attempt's marker is still
	# present proves the per-attempt tempdir actually rotates.
	# shellcheck disable=SC2016  # payload must NOT expand here; the child expands it
	arm='if [ -e "$TMPDIR/at-adjud-marker" ]; then exit 1; fi; touch "$TMPDIR/at-adjud-marker"'
	mapfile -t rs < <(adjudicate "$arm" 3)
	verdict=$(verdict_of "${rs[@]}") || fail "arm 4 adjudication failed"
	[ "$verdict" = environmental ] || fail "selftest arm 4: expected environmental (fresh TMPDIR per attempt), got $verdict — isolation broken"

	# Exit-code contract: defect exits 1 — verified through a subshell
	# invocation of this very script with the test-only runner override.
	if AT_ADJUDICATE_RUNNER='exit 1' bash "$0" -p selftest-pkg -n selftest-name --runs 2 >/dev/null 2>&1; then
		fail "selftest: defect arm must exit nonzero"
	fi

	echo "selftest: 4/4 verdict classes + isolation + exit contract OK"
	exit 0
}

if [ "$SELFTEST" -eq 1 ]; then selftest; fi

[ -n "$PACKAGE" ] || fail "-p <package> is required"
[ -n "$TESTNAME" ] || fail "-n <test-name> is required"
case "$RUNS" in
'' | *[!0-9]* | 0) fail "--runs must be a positive integer" ;;
esac

SCOPING=(-p "$PACKAGE")
if [ -n "$TARGET" ]; then
	SCOPING+=("--test" "$TARGET")
fi
RERUN="cargo test ${SCOPING[*]} --quiet $TESTNAME -- --exact (isolated TMPDIR per attempt)"
JSON_EXTRA=",\"package\":\"$PACKAGE\",\"target\":\"${TARGET:-}\",\"test\":\"$TESTNAME\",\"rerun\":\"$RERUN\""

# `--exact` is a libtest filter flag: it must ride AFTER the `--` separator.
CMD=${AT_ADJUDICATE_RUNNER:-cargo test ${SCOPING[*]} --quiet "$TESTNAME" -- --exact}

echo "adjudicating: $CMD"
mapfile -t RESULTS < <(adjudicate "$CMD" "$RUNS")
VERDICT=$(verdict_of "${RESULTS[@]}")
report "$VERDICT" "${RESULTS[@]}"
verdict_exit "$VERDICT"

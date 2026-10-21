#!/usr/bin/env bash
set -euo pipefail

# rm-2248 release glibc floor gate (run b53b5847 cycle 3, batch
# "Runner-image transition cure: CI matrix + release floor survivability").
# Every shipped v0.10.1-class Linux asset required GLIBC_2.39 because both
# Linux release builds ran on ubuntu-latest, whose glibc is whatever the
# hosted image carries — and ubuntu-latest began flipping to 26.04 on
# 2026-10-19 (actions/runner-images#14748), silently raising the floor
# release after release. Linux assets now build inside a pinned
# `container: ubuntu:22.04` (glibc 2.35), and this gate fail-closes the
# release the moment an asset requires anything newer than the floor.
#
# Extraction reuses the parsing precedent at check-install-runtime.sh
# (objdump -T | grep GLIBC_ | sort -n | tail -1) but reads the binary with
# `strings -a` instead: the gate must also read the aarch64 cross asset,
# and the container's x86_64 objdump cannot parse aarch64 ELF — a
# wrong-arch objdump yields an empty requirement set, which is a vacuous
# PASS (the exact failure class this check family rejects). strings is
# arch-independent, and any stray GLIBC_ literal it picks up can only
# tighten the gate, never loosen it.
#
# Musl/static assets carry no GLIBC_ version references and pass by
# design — that is the documented fallback surface if the container build
# ever moves to musl, not a hole.
#
# Usage:
#   check-glibc-floor.sh --self-test           fixture battery (offline)
#   check-glibc-floor.sh [--floor X.Y] BIN...  gate real release assets
#
# The floor defaults to 2.35 (Ubuntu 22.04) and must stay identical to
# the floor README.md / README.zh-CN.md advertise.

fail() {
	echo "check-glibc-floor: $*" >&2
	exit 1
}

usage() {
	sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
}

FLOOR_DEFAULT="2.35"

version_le() {
	# version_le A B — exit 0 when version A <= B, numeric field by field.
	local a="$1" b="$2" i n ai bi
	local a_fields b_fields
	IFS='.' read -r -a a_fields <<<"$a"
	IFS='.' read -r -a b_fields <<<"$b"
	n=${#a_fields[@]}
	if (( ${#b_fields[@]} > n )); then
		n=${#b_fields[@]}
	fi
	for ((i = 0; i < n; i++)); do
		ai="${a_fields[i]:-0}"
		bi="${b_fields[i]:-0}"
		if ((ai < bi)); then
			return 0
		fi
		if ((ai > bi)); then
			return 1
		fi
	done
	return 0
}

max_required_glibc() {
	# Print the highest GLIBC_X.Y[.Z] token the binary references, or
	# nothing when there is none (static/musl).
	local bin="$1" max
	max="$(strings -a "$bin" 2>/dev/null |
		grep -o 'GLIBC_[0-9]\+\(\.[0-9]\+\)*' |
		sed 's/^GLIBC_//' |
		LC_ALL=C sort -t . -k 1,1n -k 2,2n | tail -n 1 || true)"
	if [[ -n "$max" ]]; then
		printf '%s\n' "$max"
	fi
}

check_floor() {
	# check_floor BIN FLOOR — fail-closed glibc floor assertion on BIN.
	local bin="$1" floor="$2" magic max
	[[ -e "$bin" ]] || fail "$bin does not exist; build the asset first"
	[[ -f "$bin" ]] || fail "$bin is not a regular file"
	command -v strings >/dev/null 2>&1 ||
		fail "strings is required (install binutils)"
	magic="$(head -c 4 "$bin" | od -An -tx1 | tr -d ' \n')"
	[[ "$magic" == "7f454c46" ]] ||
		fail "$bin is not an ELF binary (magic $magic); the floor gate only applies to Linux assets"
	max="$(max_required_glibc "$bin")"
	if [[ -z "$max" ]]; then
		echo "glibc floor gate: $bin has no GLIBC_ requirements (static/musl) — pass"
		return 0
	fi
	echo "glibc floor gate: $bin requires GLIBC_$max (floor: $floor)"
	if ! version_le "$max" "$floor"; then
		fail "$bin requires GLIBC_$max > $floor (Ubuntu 22.04 floor)" \
			"— rebuild in the pinned ubuntu:22.04 release container or move the asset to a static/musl target"
	fi
}

make_fixture() {
	# make_fixture PATH [token...] — minimal ELF-magic file embedding the
	# given GLIBC_ tokens the way a dynsym string table would.
	local path="$1"
	shift
	{
		printf '\177ELF'
		local t
		for t in "$@"; do
			printf '%s\0' "$t"
		done
	} >"$path"
}

self_test() {
	local tmp red ok at_floor musl notelf out
	tmp="$(mktemp -d)"
	trap 'rm -rf "$tmp"' RETURN

	# v0.10.1-class RED: GLIBC_2.39 against the 2.35 floor must fail,
	# naming both the requirement and the floor.
	red="$tmp/red.elf"
	make_fixture "$red" GLIBC_2.35 GLIBC_2.39
	if out="$(check_floor "$red" "$FLOOR_DEFAULT" 2>&1)"; then
		fail "self-test: a GLIBC_2.39 asset must fail the $FLOOR_DEFAULT floor"
	fi
	[[ "$out" == *"GLIBC_2.39"* ]] ||
		fail "self-test: refusal must name the offending requirement (got: $out)"
	[[ "$out" == *"$FLOOR_DEFAULT"* ]] ||
		fail "self-test: refusal must name the floor (got: $out)"

	# GREEN: at-floor requirement passes.
	at_floor="$tmp/at-floor.elf"
	make_fixture "$at_floor" GLIBC_2.17 GLIBC_2.35
	out="$(check_floor "$at_floor" "$FLOOR_DEFAULT" 2>&1)" ||
		fail "self-test: an at-floor asset must pass (got: $out)"
	[[ "$out" == *"GLIBC_2.35"* ]] ||
		fail "self-test: pass must name the measured requirement (got: $out)"

	# Version order, not lexicographic: 2.10 < 2.9 is FALSE; 2.9 <= 2.10.
	version_le 2.10 2.9 &&
		fail "self-test: 2.10 must not compare <= 2.9 (numeric minor)"
	version_le 2.9 2.10 ||
		fail "self-test: 2.9 must compare <= 2.10 (numeric minor)"

	# Musl/static: no GLIBC_ tokens at all is a pass, not a vacuous skip.
	musl="$tmp/musl.elf"
	make_fixture "$musl" "rust_musl_anchor"
	out="$(check_floor "$musl" "$FLOOR_DEFAULT" 2>&1)" ||
		fail "self-test: a static/musl asset must pass (got: $out)"
	[[ "$out" == *"static/musl"* ]] ||
		fail "self-test: static pass must say so (got: $out)"

	# Non-ELF input is refused, never waved through as "no requirements".
	notelf="$tmp/not-an-elf"
	printf 'plain text, no ELF magic\n' >"$notelf"
	if out="$(check_floor "$notelf" "$FLOOR_DEFAULT" 2>&1)"; then
		fail "self-test: a non-ELF file must fail the gate"
	fi
	[[ "$out" == *"not an ELF"* ]] ||
		fail "self-test: refusal must say the file is not an ELF (got: $out)"

	# Floor override: --floor 2.39 admits the v0.10.1-class asset (the
	# escape hatch is explicit, never ambient).
	ok="$tmp/red-again.elf"
	make_fixture "$ok" GLIBC_2.39
	check_floor "$ok" "2.39" >/dev/null ||
		fail "self-test: an explicit --floor 2.39 must admit a GLIBC_2.39 asset"

	echo "glibc floor gate self-test passed"
}

floor="$FLOOR_DEFAULT"
self_test_mode=0
bins=()
while [[ $# -gt 0 ]]; do
	case "$1" in
		--self-test)
			self_test_mode=1
			;;
		--floor)
			[[ $# -ge 2 ]] || fail "--floor needs a version argument"
			floor="$2"
			shift
			;;
		-h | --help)
			usage
			exit 0
			;;
		--)
			shift
			while [[ $# -gt 0 ]]; do
				bins+=("$1")
				shift
			done
			break
			;;
		-*)
			fail "unknown flag: $1 (see --help)"
			;;
		*)
			bins+=("$1")
			;;
	esac
	shift
done

[[ "$floor" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
	fail "floor must be a dotted numeric version, got: $floor"

if ((self_test_mode)); then
	self_test
	exit 0
fi

(( ${#bins[@]} > 0 )) ||
	fail "no binary to gate — pass release asset paths, or --self-test"

for bin in "${bins[@]}"; do
	check_floor "$bin" "$floor"
done

echo "glibc floor gate passed"

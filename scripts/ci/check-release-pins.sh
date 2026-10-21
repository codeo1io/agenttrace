#!/usr/bin/env bash
set -euo pipefail

# rm-1038 release-pin examples gate (run 4aaa4c9fd914 cycle 2, batch
# "release-artifact truth"). Three doc surfaces carried v0.9.1 version-pin
# examples (install.cmd:3 — since removed by rm-550 — plus README.md
# and README.zh-CN.md) while
# install.sh:13 defaulted AGENTTRACE_SOURCE_REF to the current tag
# (v0.10.1) — the examples pinned a release two minor versions stale, and
# README's `AGENTTRACE_VERSION` env var is not honored by install.sh at
# all: the shell installer always fetches the newest release, only its
# build-from-source fallback takes AGENTTRACE_SOURCE_REF, and only the
# PowerShell installer takes -Version <tag>. rm-017's
# check-install-ref-drift ties install.sh's default to the newest
# CHANGELOG heading; this gate extends the chain to the example surfaces:
# every vX.Y.Z literal on them must equal install.sh's REF default, and
# both READMEs must carry it (en/zh parity).
#
# Scope note: this governs the installer-version EXAMPLES on these three
# surfaces only. A legitimate reason to cite a different tag here (say, a
# compatibility note naming an old release) means changing the example to
# a non-vX.Y.Z form or touching this gate consciously — the same posture
# as rm-017's hold-back note.

fail() {
	echo "check-release-pins: $*" >&2
	exit 1
}

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
install_sh="$repo_root/install.sh"

[ -f "$install_sh" ] || fail "install.sh not found at $install_sh"

ref_default="$(grep -m1 -oE 'AGENTTRACE_SOURCE_REF:-[^}"]+' "$install_sh" | head -1 | sed 's/.*:-//')"
[ -n "$ref_default" ] || fail "could not read AGENTTRACE_SOURCE_REF default from install.sh"
[[ "$ref_default" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
	fail "install.sh REF default '$ref_default' is not a vX.Y.Z tag"

violations=""
# rm-550 (run 1c12a0bc cycle 3, upstream #315 port) removed install.cmd
# from the surface list — it is no longer a release surface.
for surface in README.md README.zh-CN.md; do
	file="$repo_root/$surface"
	[ -f "$file" ] || fail "$surface not found at $file"
	while IFS=: read -r lineno literal _rest; do
		[ -n "$literal" ] || continue
		if [ "$literal" != "$ref_default" ]; then
			violations="${violations}  $surface:$lineno: '$literal' != install.sh default '$ref_default'"$'\n'
		fi
	done < <(grep -nEo 'v[0-9]+\.[0-9]+\.[0-9]+' "$file")
done
[ -z "$violations" ] ||
	fail "version literals that disagree with the installer default:"$'\n'"$violations"

en="$(grep -c "$ref_default" "$repo_root/README.md" || true)"
zh="$(grep -c "$ref_default" "$repo_root/README.zh-CN.md" || true)"
[ "${en:-0}" -gt 0 ] ||
	fail "README.md does not cite the installer default '$ref_default' (en/zh parity)"
[ "${zh:-0}" -gt 0 ] ||
	fail "README.zh-CN.md does not cite the installer default '$ref_default' (en/zh parity)"

echo "check-release-pins: README*.md version examples all match the installer default '$ref_default' (en $en / zh $zh mentions)"

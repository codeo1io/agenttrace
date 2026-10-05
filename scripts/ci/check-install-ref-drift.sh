#!/usr/bin/env bash
set -euo pipefail

# rm-017 drift gate (ask A4 of the 2026-10-05 assessment, folded into the
# rm-017 cadence row; landed with run 66a7d797's "Provable parser
# truthfulness" batch).
#
# install.sh pins a default AGENTTRACE_SOURCE_REF that bootstraps the
# binary users get; CHANGELOG.md's newest "## v" heading is the record of
# what the project actually shipped. Nothing tied the two together: the
# install default drifted two releases behind while the changelog moved
# (observed 2026-10-05: both v0.9.0, upstream already at v0.10.0), and
# the only signal was a human noticing. This gate fails CI the moment the
# installer's default ref diverges from the newest documented release, so
# refreshing one without the other is a build failure instead of silent
# drift.
#
# Scope note: it enforces agreement, not freshness — deliberately, so
# choosing to ship an installer pinned to an older tagged ref (e.g. a
# hold-back) requires touching this gate consciously rather than
# happening by accident.

fail() {
	echo "check-install-ref-drift: $*" >&2
	exit 1
}

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
install_sh="$repo_root/install.sh"
changelog="$repo_root/CHANGELOG.md"

[ -f "$install_sh" ] || fail "install.sh not found at $install_sh"
[ -f "$changelog" ] || fail "CHANGELOG.md not found at $changelog"

install_ref="$(grep -m1 -oE 'AGENTTRACE_SOURCE_REF:-[^}"]+' "$install_sh" | head -1 | sed 's/.*:-//')"
if [ -z "$install_ref" ]; then
	install_ref="$(grep -m1 -E '^REF=' "$install_sh" | head -1 | cut -d= -f2- | tr -d '"')"
fi
[ -n "$install_ref" ] || fail "could not read AGENTTRACE_SOURCE_REF default from install.sh"

changelog_ref="$(grep -m1 -oE '^## v[0-9]+\.[0-9]+\.[0-9]+' "$changelog" | sed 's/^## //')"
[ -n "$changelog_ref" ] || fail "could not read newest '## v' heading from CHANGELOG.md"

if [ "$install_ref" != "$changelog_ref" ]; then
	fail "install.sh default ref '$install_ref' != newest CHANGELOG heading '$changelog_ref' — refresh the installer default or the changelog so they agree"
fi

echo "check-install-ref-drift: install.sh default '$install_ref' == CHANGELOG newest '$changelog_ref'"

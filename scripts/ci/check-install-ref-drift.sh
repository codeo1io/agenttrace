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
#
# rm-195 truth extension (run 5007465422fd cycle 6): agreement says the
# installer default and the changelog agree; it says nothing about WHICH
# repository home the install family points at. The surfaces below must
# all encode the same org decision (the repository home, codeo1io), and
# the Homebrew formula must carry a real stable digest. Without this, a
# legacy org reference serves silently-stale artifacts while the gate
# stays green.

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

# --- org identity truth (rm-195, run 5007465422fd cycle 6) ---

ORG="codeo1io"
LEGACY_ORG="luoyuctl"
repo_slug="${ORG}/agenttrace"
formula="$repo_root/homebrew/Formula/agenttrace.rb"

# Guarded surfaces (the install/doc family that tells users where the
# project lives): the six stewardship surfaces plus the npm installer's
# download source, the two package READMEs, and CONTRIBUTING's clone URL.
# .codex-plugin/plugin.json is deliberately NOT in the loop: its author
# block is author identity (like npm's "author": "Luoyuctl") and stays
# legacy-keyed; only its project URLs were re-keyed.
surfaces="install.sh install.cmd install.ps1 homebrew/Formula/agenttrace.rb npm/package.json npm/scripts/install.js npm/README.md homebrew/README.md README.md README.zh-CN.md SECURITY.md CONTRIBUTING.md"
for surface in $surfaces; do
	path="$repo_root/$surface"
	[ -f "$path" ] || fail "declared surface '$surface' missing"
	if grep -n "$LEGACY_ORG" "$path" >/dev/null; then
		grep -n "$LEGACY_ORG" "$path" | sed "s/^/  $surface:/"
		fail "surface '$surface' still references legacy org '$LEGACY_ORG' — re-key to '$ORG'"
	fi
done

grep -q "^REPO=\"${repo_slug}\"" "$install_sh" || fail "install.sh REPO is not '${repo_slug}'"
grep -q "raw.githubusercontent.com/${repo_slug}/" "$repo_root/install.cmd" || fail "install.cmd does not fetch install.ps1 from '${repo_slug}'"
grep -q "^\$REPO = \"${repo_slug}\"" "$repo_root/install.ps1" || fail "install.ps1 REPO is not '${repo_slug}'"
grep -q "\"https://github.com/${repo_slug}\"" "$formula" || fail "Formula homepage is not github.com/${repo_slug}"
grep -q "https://github.com/${repo_slug}/archive/" "$formula" || fail "Formula stable url is not a ${repo_slug} archive"
grep -q "\"https://github.com/${repo_slug}.git\"" "$formula" || fail "Formula head is not github.com/${repo_slug}.git"
npm_url_count="$(grep -c "github.com/${repo_slug}" "$repo_root/npm/package.json" || true)"
[ "$npm_url_count" -ge 3 ] || fail "npm/package.json URL fields are not at github.com/${repo_slug} (found ${npm_url_count}, need homepage/repository/bugs)"
grep -q "const REPOSITORY = \"${repo_slug}\"" "$repo_root/npm/scripts/install.js" || fail "npm installer REPOSITORY is not '${repo_slug}' — postinstall would keep downloading from the legacy org"
grep -q "github.com/${repo_slug}" "$repo_root/README.md" || fail "README.md does not reference github.com/${repo_slug}"
grep -q "github.com/${repo_slug}" "$repo_root/README.zh-CN.md" || fail "README.zh-CN.md does not reference github.com/${repo_slug}"
grep -q "github.com/${repo_slug}/security/advisories" "$repo_root/SECURITY.md" || fail "SECURITY.md advisory URL is not at ${repo_slug}"

empty_sha256="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
formula_sha256="$(grep -m1 -oE 'sha256 "[0-9a-f]{64}"' "$formula" | cut -d'"' -f2)"
[ -n "$formula_sha256" ] || fail "Formula carries no 64-hex sha256"
[ "$formula_sha256" != "$empty_sha256" ] || fail "Formula sha256 is the empty-input digest"

echo "check-install-ref-drift: org truth verified — $(printf '%s\n' $surfaces | wc -l | tr -d ' ') surfaces at '${ORG}', Formula sha256 present"
echo "check-install-ref-drift: OK (agreement + truth)"

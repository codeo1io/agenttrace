#!/usr/bin/env bash
# Ties .codex-plugin/plugin.json to the latest version heading in
# CHANGELOG.md. The workspace Cargo version is a 0.0.0-dev placeholder
# (RELEASE_VERSION overrides it at release time), so without this check the
# plugin manifest could drift silently between releases.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
plugin="${root}/.codex-plugin/plugin.json"
changelog="${root}/CHANGELOG.md"

fail() {
  echo "check-plugin-version: $*" >&2
  exit 1
}

[[ -f "$plugin" ]] || fail "plugin manifest not found: $plugin"
[[ -f "$changelog" ]] || fail "changelog not found: $changelog"

plugin_version="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$plugin")"
changelog_version="$(grep -m1 -oE '^## v[0-9]+\.[0-9]+\.[0-9]+' "$changelog" | head -1 | sed 's/^## v//')"

[[ -n "$plugin_version" ]] || fail "could not read version from $plugin"
[[ -n "$changelog_version" ]] || fail "could not read the latest version heading from $changelog"

if [[ "$plugin_version" != "$changelog_version" ]]; then
  fail "plugin.json version $plugin_version does not match CHANGELOG latest v$changelog_version"
fi

echo "check-plugin-version: plugin.json v$plugin_version matches CHANGELOG v$changelog_version"

#!/usr/bin/env bash
# Gates the Codex plugin manifest (.codex-plugin/plugin.json) on two
# truthfulness invariants (rm-014):
#
#   1. Version chain: manifest version == latest CHANGELOG "## v" heading ==
#      latest git tag (when any tag is fetchable). The CHANGELOG anchor alone
#      let three releases (v0.8.0, v0.8.1, v0.9.0) ship with both numbers
#      stale in agreement — the tag anchor breaks that vacuous pass.
#   2. Asset integrity: every "./..." path value in the manifest resolves at
#      the repo root (the fork's convention, previously pinned only for the
#      two screenshots by check-release-surfaces.sh).
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

# Tag anchor: absent only in shallow checkouts with no tags fetched
# (CI default); locally and on release refs the latest version tag must
# agree. Strictly version-shaped tags only — conductor housekeeping tags
# (e.g. vautonomy-retired/...) must not become the anchor.
latest_tag="$(git -C "$root" tag 2>/dev/null | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1 | sed 's/^v//' || true)"
if [[ -n "$latest_tag" && "$latest_tag" != "$plugin_version" ]]; then
  fail "plugin.json version $plugin_version does not match the latest tag v$latest_tag (CHANGELOG agrees with the tag, the manifest does not)"
fi

# Asset integrity: every string value in the manifest that looks like a
# relative path must resolve at the repo root.
while IFS= read -r rel; do
  [[ -e "$root/$rel" ]] || fail "plugin manifest references ./$rel which does not exist at the repo root"
done < <(python3 - "$plugin" <<'PY'
import json, sys
manifest = json.load(open(sys.argv[1]))

def walk(node):
    if isinstance(node, dict):
        for value in node.values():
            yield from walk(value)
    elif isinstance(node, list):
        for value in node:
            yield from walk(value)
    elif isinstance(node, str) and node.startswith("./") and node != "./":
        print(node[2:])

walk(manifest)
PY
)

echo "check-plugin-version: plugin.json v$plugin_version matches CHANGELOG v$changelog_version${latest_tag:+ and tag v$latest_tag}; all manifest ./ paths resolve"

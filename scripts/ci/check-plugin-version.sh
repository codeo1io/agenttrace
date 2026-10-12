#!/usr/bin/env bash
# Gates the Codex plugin manifest (.codex-plugin/plugin.json) on two
# truthfulness invariants (rm-163):
#
#   1. Version chain: manifest version == latest CHANGELOG "## v" heading,
#      and >= the latest version tag MERGED INTO HEAD. The CHANGELOG anchor
#      alone let three releases (v0.8.0, v0.8.1, v0.9.0) ship with both
#      numbers stale in agreement — the tag anchor breaks that vacuous pass.
#      The tag anchor is lineage-scoped (--merged HEAD) and one-directional
#      (>=, never ==): a branch may bump AHEAD of the last release, but may
#      never sit behind or beside a released number. An equality anchor over
#      the whole tag namespace was tried and is wrong twice over (CI run
#      36700231403): it fails every pre-release bump (the tag only exists
#      after release), and it anchors on lineage-foreign tags — a clone that
#      fetched upstream's v0.9.0 passed while origin-only CI failed on
#      v0.8.0, same commit, two verdicts.
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

# Tag anchor: latest version tag merged into HEAD — lineage-true and immune
# to which remotes a clone happens to have fetched. Absent only where the
# checkout cannot see any merged version tag (shallow CI checkouts fetching
# origin tags only); the check then degrades to the CHANGELOG chain, never
# to a wrong verdict. Strictly version-shaped tags only — conductor
# housekeeping tags (e.g. vautonomy-retired/...) must not become the anchor.
latest_tag="$(git -C "$root" tag --merged HEAD 2>/dev/null | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1 | sed 's/^v//' || true)"
if [[ -n "$latest_tag" ]] \
   && [[ "$(printf '%s\n%s\n' "$latest_tag" "$plugin_version" | sort -V | tail -1)" != "$plugin_version" ]]; then
  fail "plugin.json version $plugin_version does not advance past the latest released tag v$latest_tag (merged into HEAD)"
fi

# Per-tag CHANGELOG-section arm (rm-303, run 364aa3be): every ^v semver
# tag MERGED INTO HEAD must have a CHANGELOG section — a heading
# `## vX.Y.Z ...` — or an explicit no-section marker carrying a
# rationale on one line:
#   <!-- no-changelog-section: vX.Y.Z: reason -->
# Before this arm the gate anchored only the LATEST heading and tag, so
# mid-range merged tags could stay sectionless forever (v0.8.1 shipped
# that way: heading jump v0.9.0 -> v0.7.1). Lineage-scoped and
# no-tag-fallback like the anchor above.
#
# Fork-scope cut (rm-1365, run 43db92ef): tags strictly older than v0.4.0
# are inherited upstream releases imported at the fork point — the fork's
# CHANGELOG does not document them and never should (49 such tags,
# v0.0.4..v0.3.9, made this arm red on every full clone with visible tags
# while a zero-tag checkout passed vacuously). The cut is an explicit
# constant, NOT derived from the CHANGELOG: an oldest-sectioned-tag
# derivation would silently move forward the first time a fork tag ships
# without a section — exactly the regression this arm exists to catch.
# Bump the constant only if the fork deliberately re-imports upstream
# history.
fork_first_tag='v0.4.0'
mapfile -t merged_tags < <(git -C "$root" tag --merged HEAD 2>/dev/null | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V || true)

# Vacuous-pass killer (rm-1365): a full clone that can see no merged
# semver tag has broken tag transport, and a silent green here would be
# a false pass. Shallow checkouts (git's shallow marker present) still
# degrade to the CHANGELOG chain by design, exactly like the anchor
# arm above.
if [[ ${#merged_tags[@]} -eq 0 ]] && [[ ! -e "$(git -C "$root" rev-parse --git-dir)/shallow" ]]; then
  fail "no merged semver tags visible on a full clone — tag transport broken (vacuous pass refused, rm-1365)"
fi

for tag in "${merged_tags[@]}"; do
  # Inherited upstream tag (older than the fork cut): exempt.
  [[ "$(printf '%s\n%s\n' "$fork_first_tag" "$tag" | sort -V | tail -1)" == "$tag" ]] || continue
  escaped_tag=${tag//./\\.}
  if grep -qE "^## ${escaped_tag}( |$)" "$changelog"; then
    continue
  fi
  if grep -qF "<!-- no-changelog-section: $tag:" "$changelog"; then
    continue
  fi
  fail "merged fork-owned tag $tag (>= $fork_first_tag) has no CHANGELOG section: add a '## $tag' heading or an explicit '<!-- no-changelog-section: $tag: reason -->' marker"
done

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

echo "check-plugin-version: plugin.json v$plugin_version matches CHANGELOG v$changelog_version${latest_tag:+ and advances past merged tag v$latest_tag}; all manifest ./ paths resolve"

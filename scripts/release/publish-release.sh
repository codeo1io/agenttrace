#!/usr/bin/env bash
# Idempotent GitHub Release publisher (rm-169, cycle 5).
#
# The release workflow's publish step previously called `gh release create`
# unconditionally: a re-run of the job (or a re-dispatch of the tag workflow)
# either hard-failed on the existing release or, after a partially failed
# first pass, left assets stranded with no safe way to complete them. The
# manual `publish-channels.yml` recovery lane exists precisely because this
# step was not re-run safe.
#
# This script makes the publish decision idempotent:
#   * release missing            -> create it with every asset in the dist dir
#   * release exists, complete   -> no create, no upload, exit 0 (no-op)
#   * release exists, incomplete -> upload ONLY the missing assets, exit 0
#
# The npm arm of the release workflow already follows this guard pattern
# (`npm view` -> "already published; skipping"), and publish-homebrew.sh is a
# no-op on an unchanged diff; this closes the same hole for the GitHub
# Release itself. scripts/release/idempotency-selftest.sh exercises all three
# branches against a PATH-shimmed `gh`.
#
# Usage: publish-release.sh <tag> <dist-dir>
# Requires: GITHUB_REPOSITORY, gh authenticated with repo scope.

set -euo pipefail

tag="${1:?usage: publish-release.sh <tag> <dist-dir>}"
dist="${2:?usage: publish-release.sh <tag> <dist-dir>}"
repo="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY must be set}"

shopt -s nullglob
assets=("$dist"/*)
if [ "${#assets[@]}" -eq 0 ]; then
  echo "publish-release: no artifacts in $dist — refusing to publish an empty release" >&2
  exit 1
fi

if gh release view "$tag" --repo "$repo" >/dev/null 2>&1; then
  echo "release $tag already exists; reconciling assets"
  mapfile -t existing < <(gh release view "$tag" --repo "$repo" --json assets -q '.assets[].name')
  added=0
  for f in "${assets[@]}"; do
    name="$(basename "$f")"
    present=0
    for e in "${existing[@]:-}"; do
      if [ "$e" = "$name" ]; then
        present=1
        break
      fi
    done
    if [ "$present" -eq 1 ]; then
      echo "  asset present: $name"
    else
      echo "  uploading missing asset: $name"
      gh release upload "$tag" "$f" --repo "$repo"
      added=1
    fi
  done
  if [ "$added" -eq 0 ]; then
    echo "release $tag complete; no changes (idempotent no-op)"
  else
    echo "release $tag reconciled"
  fi
  exit 0
fi

echo "creating release $tag"
# shellcheck disable=SC2068  # assets must expand here; nullglob keeps it well-formed
gh release create "$tag" ${assets[@]} \
  --repo "$repo" \
  --title "$tag" \
  --generate-notes

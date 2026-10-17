#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "check-docs-links: $*" >&2
  exit 1
}

# rm-1892 (run 77ca5de0 cycle 5, "count-honesty ledger" docs rider):
# every RELATIVE link in the repo's Markdown — docs, README, guides,
# quality gates — must resolve to a file that exists in the tree. The
# F3 finding this gate pins: docs/guides/usage-card.md:47 linked
# ../pricing.md, a file that never existed (git log --all is empty for
# it), and the docs battery of the day only smoke-tested COMMANDS, so
# the broken link shipped in the 1-broken state the assess scan
# measured (99 links / 1 broken). Absolute http(s)/mailto links and
# pure-anchor links are out of scope: this gate pins RELATIVE targets
# against the tree, cheaply, on every push.
#
# Fixture pin (red-first proof): a file containing [gone](../nope.md)
# makes this gate fail naming the linking file and the missing target;
# the repo as shipped has zero offenders (asserted by the empty
# offender list below).

repo_root="$(git rev-parse --show-toplevel)" \
  || fail "must run inside the agenttrace repository"

cd "$repo_root"

offenders=""

while IFS= read -r md; do
  dir="$(dirname "$md")"
  while IFS= read -r target; do
    case "$target" in
      http://*|https://*|mailto:*|'#'*) continue ;;
    esac
    # Strip an anchor suffix (README.md#pricing); the file part must
    # exist relative to the linking file's directory.
    file_part="${target%%#*}"
    [[ -z "$file_part" ]] && continue
    if [[ ! -f "$dir/$file_part" ]]; then
      offenders+="$md: $target"$'\n'
    fi
  done < <(grep -oE '\]\([^)]+\)' "$md" 2>/dev/null | sed -E 's/^\]\(//; s/\)$//')
done < <(git ls-files '*.md')

if [[ -n "$offenders" ]]; then
  printf '%s' "$offenders" >&2
  count="$(printf '%s' "$offenders" | grep -c . || true)"
  fail "$count broken relative link(s) — see the lines above"
fi

echo "check-docs-links: all relative links resolve"

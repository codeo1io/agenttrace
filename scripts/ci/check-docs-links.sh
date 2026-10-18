#!/usr/bin/env bash
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
#
# rm-1894 rider (run 5cbd75e3 cycle 2, "Verify, don't trust"): the
# scan covered only INLINE `](target)` links, so a reference-style
# definition — `[label]: destination` at line start — could dangle
# invisibly while every rendered inline link resolved. And the
# existence test was `-f`, which flags a legitimate DIRECTORY target
# (`](docs/quality-gates)`) as broken. Both forms now lint; targets
# may be files OR directories. See docs/quality-gates/relative-link-forms.md,
# the standing fixture for every covered form.

set -euo pipefail

fail() {
  echo "check-docs-links: $*" >&2
  exit 1
}

repo_root="$(git rev-parse --show-toplevel)" \
  || fail "must run inside the agenttrace repository"

cd "$repo_root"

offenders=""

while IFS= read -r md; do
  dir="$(dirname "$md")"
  while IFS= read -r target; do
    case "$target" in
      http://*|https://*|mailto:*) continue ;;
    esac
    # Strip an anchor suffix (README.md#pricing); the file part must
    # exist relative to the linking file's directory.
    file_part="${target%%#*}"
    [[ -z "$file_part" ]] && continue
    # rm-1894: -e, not -f — a directory target is a valid link, not a
    # broken one.
    if [[ ! -e "$dir/$file_part" ]]; then
      offenders+="$md: $target"$'\n'
    fi
  done < <(
    # Inline links: `](target)`. Each pipeline is `|| :`-guarded:
    # under `set -e -o pipefail` a file with NO inline links makes
    # grep exit 1, which would kill this process-substitution subshell
    # before the reference-style sed below ever runs (caught live by
    # the fixture test: a README with only a `[label]: destination`
    # definition passed the gate).
    { grep -oE '\]\([^)]+\)' "$md" 2>/dev/null || :; } \
      | sed -E 's/^\]\(//; s/\)$//'
    # rm-1894: reference-style definitions — `[label]: <destination>`
    # at line start (up to three leading spaces, CommonMark), the
    # destination a bare whitespace-free token or angle-bracketed,
    # optionally followed by ONE title (quoted or parenthesized) and
    # nothing else. The strict tail is load-bearing: review docs write
    # prose status lines like `[REPRODUCED]: opencode DB, one assistant
    # message with` — bare words after the destination are NOT a
    # CommonMark reference definition (a title must be quoted or
    # parenthesized), and a loose match would flag those as broken
    # links (caught live on
    # docs/reviews/2026-09-02-adversarial-repository-assessment-pass5.md).
    { sed -nE 's/^[[:space:]]{0,3}\[[^]]+\]:[[:space:]]*(<[^>]+>|[^[:space:]]+)([[:space:]]+("[^"]*"|'"'"'[^'"'"']*'"'"'|\([^)]*\)))?[[:space:]]*$/\1/p' \
        "$md" 2>/dev/null || :; } \
      | sed -E 's/^<//; s/>$//'
  )
done < <(git ls-files '*.md')

if [[ -n "$offenders" ]]; then
  printf '%s' "$offenders" >&2
  count="$(printf '%s' "$offenders" | grep -c . || true)"
  fail "$count broken relative link(s) — see the lines above"
fi

echo "check-docs-links: all relative links resolve"

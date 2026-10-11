#!/usr/bin/env bash
# rm-1021 (run 240079ac cycle 2, repository-maintenance c151f66b):
# ROADMAP commit-anchor resolvability gate.
#
# ROADMAP.md cites git shas as provenance — worktree bases (@<sha>,
# base <sha>), landed/lane commits (commit <sha>), and sibling-lane
# citations (uncommitted <sha>). When a cited sha does not exist in
# this repository, the provenance it names is unreconstructable: that
# is how 278b2bda/6e96bed5 (roadmap rm-1021, assess d6b0d11d D3) sat
# on the wall as merge-routing for a lane that never landed under
# those ids.
#
# This gate extracts every commit-shaped citation from ROADMAP.md and
# requires each one to resolve via `git cat-file -t` from this
# repository, or to be explicitly listed WITH A REASON in
# scripts/ci/roadmap-anchor-exceptions.txt (foreign-repository
# citations — upstream commits and other-fleet bases — are the
# legitimate residents of that file; dated wall records are never
# rewritten to carry markers, so the opt-out lives here).
#
# Checked contexts are deliberately narrow: the wall also carries hex
# ids that are NOT commit citations (conductor run/attempt ids,
# 32-hex conflict cases, candidate ids) and those are ignored. The
# 7-hex minimum is git's own short-sha floor — bare years and dates
# (2026, @2026-08-04) are 4-hex runs and must not match.
#
# Shallow CI checkouts (actions/checkout default depth 1) cannot
# resolve historical shas; the gate deepens best-effort and, if it
# cannot, SKIPs with a named line (the same best-effort shape as the
# Gate surface audit step) — full clones and landing gates enforce.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

ROADMAP=ROADMAP.md
EXCEPTIONS=scripts/ci/roadmap-anchor-exceptions.txt

if [ ! -f "$ROADMAP" ]; then
    echo "roadmap-anchor gate: FAIL — $ROADMAP not found"
    exit 1
fi
if [ ! -f "$EXCEPTIONS" ]; then
    echo "roadmap-anchor gate: FAIL — $EXCEPTIONS missing (create it with '<sha> <reason>' lines)"
    exit 1
fi

# Refuse exception lines that carry no reason — an unexplained opt-out
# is exactly the silent rot this gate exists to prevent.
while IFS= read -r line; do
    case "$line" in
        ''|\#*) continue ;;
    esac
    sha="${line%% *}"
    reason="${line#* }"
    if [ "$sha" = "$reason" ] || [ -z "$reason" ]; then
        echo "roadmap-anchor gate: FAIL — exception '$sha' carries no reason"
        exit 1
    fi
done < "$EXCEPTIONS"

# Deepen shallow checkouts so historical citations can resolve.
if [ "$(git rev-parse --is-shallow-repository 2>/dev/null || echo false)" = "true" ]; then
    if git fetch --unshallow --quiet 2>/dev/null; then
        echo "roadmap-anchor gate: deepened shallow checkout ($( \
            git rev-list --count HEAD 2>/dev/null || echo '?') commits reachable)"
    else
        echo "roadmap-anchor gate: SKIP — shallow checkout and 'git fetch --unshallow' failed; run locally or at landing gates"
        exit 0
    fi
fi

fail=0
# Deduplicate (line, token) pairs so one dead sha fails once per site.
# NOTE: {7,16} is greedy without a trailing boundary — a 17+ hex run
# directly after a checked context would match its 16-char prefix;
# none exists on the wall, and a mis-split prefix simply fails to
# resolve and gets adjudicated like any other red.
grep -nE '@[0-9a-f]{7,16}|(base|commit|uncommitted) [0-9a-f]{7,16}' "$ROADMAP" \
| while IFS=: read -r lineno text; do
    for tok in $(printf '%s\n' "$text" \
        | grep -oE '@[0-9a-f]{7,16}|(^| )(base|commit|uncommitted) [0-9a-f]{7,16}' \
        | grep -oE '[0-9a-f]{7,16}' | sort -u); do
        echo "$lineno $tok"
    done
done \
| sort -u \
| while IFS=' ' read -r lineno tok; do
    if grep -qE "^${tok}( |$)" "$EXCEPTIONS"; then
        echo "roadmap-anchor gate: SKIP $tok (line $lineno) — listed in $EXCEPTIONS"
        continue
    fi
    if git cat-file -t "$tok" >/dev/null 2>&1; then
        continue
    fi
    echo "roadmap-anchor gate: FAIL — ROADMAP.md:$lineno cites $tok, which does not resolve in this repository (re-anchor to a resolvable commit, or list it with a reason in $EXCEPTIONS)"
    exit 1
done || fail=1

if [ "$fail" -ne 0 ]; then
    exit 1
fi
echo "roadmap-anchor gate: OK — every commit-shaped ROADMAP citation resolves (or is exception-listed)"

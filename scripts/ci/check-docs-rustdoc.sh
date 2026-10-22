#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "check-docs-rustdoc: $*" >&2
  exit 1
}

# rm-2292 (run 940e1aab cycle 4, "doc-truth gates" rider): the API-doc
# surface must build clean under -D warnings. Pre-gate state (assessed
# 2026-10-22 at 6b8c5c0a): 11 errors — 10 public-doc links to private
# items (rustdoc::private_intra_doc_links) across discovery.rs /
# otel.rs / pricing.rs / reports.rs / statusline.rs / lib.rs, plus one
# unclosed HTML tag (`<reason>`, rustdoc::invalid_html_tags) — and NO
# cargo doc gate anywhere in .github/workflows or scripts/ci, so the
# API-doc surface could regress silently. This gate pins it: every
# push builds the workspace docs with warnings denied.
#
# Red-first proof (rm-2292): on the pre-cure tree this script fails
# with exactly the 11 errors above (verified 2026-10-22 by stashing
# the cure files); on the cured tree it passes.

repo_root="$(git rev-parse --show-toplevel)" \
  || fail "must run inside the agenttrace repository"

cd "$repo_root"

log="$(mktemp /tmp/check-docs-rustdoc.XXXXXX)"
trap 'rm -f "$log"' EXIT

if ! RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked \
    >"$log" 2>&1; then
  # Name the offenders: rustdoc errors carry `--> file:line:col`.
  grep -E '^error(\[|:)' -A 3 "$log" | head -60 >&2 || true
  count="$(grep -cE '^ +-->' "$log" || true)"
  fail "RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked failed with ${count:-?} error(s) — see above (rerun locally to reproduce)"
fi

echo "check-docs-rustdoc: OK — cargo doc --workspace --no-deps clean under -D warnings"

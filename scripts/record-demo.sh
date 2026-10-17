#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

# rm-241 repo-side arm (run ce9309d9 cycle 1): collision-proof default
# target dir — only when CARGO_TARGET_DIR is UNSET, a repo-root+HEAD-
# namespaced dir under the user cache (same worktree+HEAD stays warm,
# different worktrees never write one dir; explicit env always wins).
# The cp below follows the same dir, so build and copy can never
# diverge under an ambient fleet layout.
if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
  at_repo="$(git rev-parse --show-toplevel 2>/dev/null || true)"
  at_head="$(git rev-parse HEAD 2>/dev/null || true)"
  at_cache_home="${XDG_CACHE_HOME:-${HOME:-}/.cache}"
  if [[ -n "$at_repo" && -n "$at_head" && -n "${HOME:-}" && -d "$at_cache_home" ]]; then
    at_ns="$(printf '%s:%s' "$at_repo" "$at_head" | git hash-object --stdin | cut -c1-16)"
    export CARGO_TARGET_DIR="$at_cache_home/agenttrace-dev/target/at-$at_ns"
  fi
fi

if ! command -v vhs >/dev/null 2>&1; then
  echo "vhs is required to render docs/demos/demo.tape."
  echo "Install: https://github.com/charmbracelet/vhs"
  exit 1
fi

if ! command -v ttyd >/dev/null 2>&1; then
  echo "ttyd is required by vhs to render terminal recordings."
  echo "Install: https://github.com/tsl0922/ttyd"
  exit 1
fi

tmpbin="$(mktemp -d)"
cleanup() {
  rm -rf "$tmpbin"
}
trap cleanup EXIT

cargo build --release -p agenttrace
cp "${CARGO_TARGET_DIR:-target}/release/agenttrace" "$tmpbin/agenttrace"
PATH="$tmpbin:$PATH" vhs docs/demos/demo.tape
echo "Wrote assets/agenttrace-demo.gif"

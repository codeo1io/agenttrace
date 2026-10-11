#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "check-gate-hermeticity: $*" >&2
  exit 1
}

# rm-1010: one AGENTTRACE_BIN convention across scripts/ci. The binary
# default must be the repository's own release build, anchored with
# `git rev-parse --show-toplevel` (rm-369 lineage) — never an external
# absolute path (/tmp/agenttrace once answered gate runs with a
# days-old binary) and never a CWD-relative path (a silent no-op when
# the gate is invoked from outside the repo root). AGENTTRACE_BIN
# remains an override in every consumer.
#
# Rule 1: no script under scripts/ may default AGENTTRACE_BIN to an
#         absolute path. Any literal absolute default is external by
#         construction — the anchored form is `$repo_root/...`.
# Rule 2: every scripts/ci gate that reads AGENTTRACE_BIN must anchor
#         its default with `git rev-parse --show-toplevel`.
# Sanctioned exemption: check-install-runtime.sh's EMPTY default — a
# glibc-leg skip-guard, not a substitute binary.

repo_root="$(git rev-parse --show-toplevel)" \
  || fail "must run inside the agenttrace repository"

# Rule 1 — external absolute defaults anywhere under scripts/. The
# gate excludes itself: its own pattern literal is the rule, not a
# default.
while IFS= read -r match; do
  [[ -n "$match" ]] || continue
  echo "$match" >&2
  fail "AGENTTRACE_BIN defaults to a path outside the repository; default to \$repo_root/target/release/agenttrace (rm-369/rm-1010)"
done < <(grep -rn --include='*.sh' --exclude='check-gate-hermeticity.sh' -E 'AGENTTRACE_BIN:-/' scripts/ || true)

# Rule 2 — unanchored AGENTTRACE_BIN consumers among the CI gates.
for script in scripts/ci/*.sh; do
  grep -q 'AGENTTRACE_BIN' "$script" || continue
  if grep -q -- 'AGENTTRACE_BIN:-}' "$script" \
    && ! grep -qE -- 'AGENTTRACE_BIN:-[^}]' "$script"; then
    # Empty-default skip-guard (check-install-runtime.sh glibc leg).
    continue
  fi
  grep -q 'git rev-parse --show-toplevel' "$script" \
    || fail "$script reads AGENTTRACE_BIN without a worktree-owned default; anchor it with git rev-parse --show-toplevel (rm-1010)"
done

echo "check-gate-hermeticity: AGENTTRACE_BIN convention holds (worktree-owned defaults; no external-path defaults)"

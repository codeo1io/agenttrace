#!/usr/bin/env bash
# Release idempotency selftest (rm-169, cycle 5).
#
# Proves the release machinery is safe to re-run:
#   A. publish-release.sh on a missing release      -> creates with all assets
#   B. publish-release.sh on a complete release     -> no-op, no gh mutations
#   C. publish-release.sh on a partially uploaded   -> uploads ONLY the missing
#                                                      asset, leaves the rest
#   D. publish-release.sh with an empty dist dir    -> refuses (exit 1)
#   E. structural wiring: the workflow delegates to the script, the attest
#      step is gated on the release-existence probe, the npm arm keeps its
#      skip guard, publish-homebrew keeps its no-diff guard, and changelog
#      keeps its already-present guard.
#
# The `gh` binary is PATH-shimmed against a throwaway state directory, so the
# test runs offline. Run from the repository root:
#   scripts/release/idempotency-selftest.sh

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

fails=0
pass() { echo "  ok: $1"; }
fail() { echo "  FAIL: $1" >&2; fails=$((fails+1)); }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
state="$work/state"
mkdir -p "$state" "$work/bin" "$work/dist"
printf 'alpha\n' >"$work/dist/agenttrace-aarch64-apple-darwin.tar.gz"
printf 'beta\n'  >"$work/dist/agenttrace-x86_64-unknown-linux-gnu.tar.gz"
printf 'h1:..\n' >"$work/dist/checksums.txt"

# --- PATH shim for gh ------------------------------------------------------
cat >"$work/bin/gh" <<'SHIM'
#!/usr/bin/env bash
# Minimal gh shim: release view|create|upload against $SHIM_STATE.
set -euo pipefail
state="${SHIM_STATE:?}"
cmd="$1"; shift
case "$cmd" in
  release)
    sub="$1"; shift
    case "$sub" in
      view)
        tag="$1"; shift
        rel="$state/releases/$tag"
        [ -d "$rel" ] || exit 1
        for a in "$@"; do
          case "$a" in
            --json) fmt=assets ;;
            *) repo_flag=1 ;;
          esac
        done
        if [ "${fmt:-}" = assets ]; then
          for f in "$rel"/assets/*; do [ -e "$f" ] && basename "$f"; done
        else
          echo "shim-release $tag"
        fi
        ;;
      create)
        tag=""; files=()
        for a in "$@"; do
          case "$a" in
            --repo|--title|--generate-notes) skip=1 ;;
            *) if [ "${skip:-0}" = 1 ]; then skip=0; else
                 if [ -z "$tag" ]; then tag="$a"; else files+=("$a"); fi
               fi ;;
          esac
        done
        mkdir -p "$state/releases/$tag/assets"
        for f in "${files[@]}"; do cp "$f" "$state/releases/$tag/assets/"; done
        echo "shim created $tag"
        ;;
      upload)
        tag="$1"; file="$2"
        mkdir -p "$state/releases/$tag/assets"
        cp "$file" "$state/releases/$tag/assets/"
        echo "shim uploaded $(basename "$file")"
        ;;
      *) echo "shim: unsupported release subcommand $sub" >&2; exit 2 ;;
    esac
    ;;
  *) echo "shim: unsupported gh command $cmd" >&2; exit 2 ;;
esac
SHIM
chmod +x "$work/bin/gh"
export SHIM_STATE="$state"
export PATH="$work/bin:$PATH"
export GITHUB_REPOSITORY="example/agenttrace"

pub="$repo_root/scripts/release/publish-release.sh"

echo "A. fresh publish creates the release with every asset"
out_a="$(bash "$pub" v0.10.2 "$work/dist")"
# NOTE: text assertions use `grep -q ... <<<"$var"` (here-string), never
# `echo | grep -q`: under `set -o pipefail` grep -q's early exit can
# SIGPIPE the echo writer (status 141) and flake the gate on good input.
grep -q "creating release v0.10.2" <<<"$out_a" \
  && [ "$(ls "$state/releases/v0.10.2/assets" | wc -l)" -eq 3 ] \
  && pass "A" || fail "A: $out_a"

echo "B. complete re-run is a no-op with zero gh mutations"
# Snapshot the shim state by CONTENT (not mtime): a re-run must leave the
# release tree byte-identical. An mtime-based "marker still newest" check
# is nondeterministic under clock-tick granularity and flaked CI.
snapshot_state() { (cd "$state" && find . -type f | LC_ALL=C sort | xargs -r md5sum); }
before_b="$(snapshot_state)"
out_b="$(bash "$pub" v0.10.2 "$work/dist")"
after_b="$(snapshot_state)"
noop=1
grep -q "creating release" <<<"$out_b" && noop=0
grep -q "uploading missing asset" <<<"$out_b" && noop=0
# every existing asset reported present
[ "$(echo "$out_b" | grep -c "asset present:")" -eq 3 ] || noop=0
grep -q "idempotent no-op" <<<"$out_b" || noop=0
# shim state untouched: no file added, removed, or modified
[ "$after_b" = "$before_b" ] || noop=0
[ "$noop" -eq 1 ] && pass "B" || fail "B: $out_b"

echo "C. partial re-run uploads only the missing asset"
rm "$state/releases/v0.10.2/assets/checksums.txt"
out_c="$(bash "$pub" v0.10.2 "$work/dist")"
grep -q "uploading missing asset: checksums.txt" <<<"$out_c" \
  && [ "$(grep -c "uploading missing asset" <<<"$out_c")" -eq 1 ] \
  && [ "$(grep -c "asset present:" <<<"$out_c")" -eq 2 ] \
  && [ -f "$state/releases/v0.10.2/assets/checksums.txt" ] \
  && pass "C" || fail "C: $out_c"

echo "D. empty dist dir is refused"
empty="$(mktemp -d)"
if bash "$pub" v0.10.2 "$empty" >"$work/d.out" 2>&1; then
  fail "D: expected non-zero exit"
else
  grep -q "refusing to publish an empty release" "$work/d.out" \
    && pass "D" || fail "D: wrong diagnostic"
fi
rmdir "$empty"

echo "E. workflow wiring keeps every idempotency guard"
w=".github/workflows/release.yml"
grep -q 'scripts/release/publish-release.sh' "$w" \
  && pass "E1 release.yml delegates to publish-release.sh" \
  || fail "E1 release.yml does not call publish-release.sh"
grep -q 'id: existing_release' "$w" \
  && grep -A2 'Attest release assets' "$w" | grep 'existing_release' >/dev/null \
  && pass "E2 attest step gated on release-existence probe" \
  || fail "E2 attest step not gated"
grep -q 'already published; skipping' "$w" \
  && pass "E3 npm arm keeps skip guard" \
  || fail "E3 npm guard missing"
grep -q 'git diff --cached --quiet' scripts/release/publish-homebrew.sh \
  && pass "E4 homebrew keeps no-diff guard" \
  || fail "E4 homebrew guard missing"
grep -q 'already has' .github/workflows/changelog.yml \
  && pass "E5 changelog keeps already-present guard" \
  || fail "E5 changelog guard missing"

if [ "$fails" -gt 0 ]; then
  echo "idempotency-selftest: $fails failure(s)" >&2
  exit 1
fi
echo "idempotency-selftest: all branches green"

#!/usr/bin/env bash
# Install-surface drift gate (rm-516).
#
# Guards two things the 2026-10 cycle found rotted:
#   1. install.sh must not default to a stale pinned tag: the default
#      must be a literal vX.Y.Z tag equal to the newest tag reachable
#      in this clone (so bumping releases forces updating the default
#      consciously; "latest" is NOT accepted because the source-build
#      fallback clones `--branch "$REF"`, which cannot resolve it).
#   2. install.ps1 must keep the architecture fallback (string read +
#      PROCESSOR_ARCHITEW6432/PROCESSOR_ARCHITECTURE) and the
#      AGENTTRACE_VERSION pinning hook; both regress silently on CI because
#      non-interactive runs never load PSReadLine.
set -euo pipefail
cd "$(dirname "$0")/../.."

fail=0
err() { echo "check-install-drift: FAIL: $*" >&2; fail=1; }

# --- install.sh REF default ---
ref_line="$(grep -m1 '^REF=' install.sh || true)"
ref_default="$(printf '%s' "$ref_line" | sed -n 's/.*AGENTTRACE_SOURCE_REF:-\([^}]*\)}.*/\1/p')"
if [ -z "$ref_default" ]; then
    err "install.sh: cannot parse REF default from: ${ref_line:-<missing>}"
elif [ "$ref_default" = "latest" ]; then
    err "install.sh: REF default 'latest' cannot be resolved by the source-build fallback (git clone --branch); pin a vX.Y.Z tag"
else
    case "$ref_default" in
        v[0-9]*.[0-9]*.[0-9]*) ;;
        *) err "install.sh: REF default '$ref_default' is neither 'latest' nor a vX.Y.Z tag" ;;
    esac
    newest_tag="$(git tag --list 'v*' | sort -V | tail -1 || true)"
    if [ -n "$newest_tag" ] && [ "$newest_tag" != "$ref_default" ]; then
        older="$(printf '%s\n%s\n' "$ref_default" "$newest_tag" | sort -V | head -1)"
        if [ "$older" = "$newest_tag" ]; then
            err "install.sh: REF default $ref_default is not a known tag (newest local tag: $newest_tag)"
        elif [ "$ref_default" != "$newest_tag" ]; then
            err "install.sh: REF default $ref_default is stale (newest local tag: $newest_tag)"
        fi
    fi
fi

# --- install.ps1 architecture fallback ---
for marker in 'PROCESSOR_ARCHITEW6432' 'PROCESSOR_ARCHITECTURE' 'AMD64'; do
    grep -q "$marker" install.ps1 || err "install.ps1: architecture fallback marker '$marker' missing"
done
grep -q 'RuntimeInformation' install.ps1 || err "install.ps1: primary architecture probe missing"

# --- version pinning hooks on both platforms ---
grep -q '\$env:AGENTTRACE_VERSION' install.ps1 ||
    err "install.ps1: AGENTTRACE_VERSION pinning hook missing (a comment alone is not a hook)"
grep -q 'AGENTTRACE_SOURCE_REF' install.sh || err "install.sh: AGENTTRACE_SOURCE_REF pinning hook missing"

# --- install.sh download integrity (parity with check-install-runtime,
#     which exercises behavior; this catches silent deletion only) ---
grep -q 'CHECKSUM_URL' install.sh ||
    err "install.sh: checksum sidecar verification missing (.sha256 parity with release.yml)"
grep -q 'sha256sum' install.sh ||
    err "install.sh: sha256 verification of the downloaded asset missing"
grep -q 'cannot verify the download, not installing' install.sh ||
    err "install.sh: missing-sha256-tool refusal missing"
grep -q 'refusing to build an unverified master tip' install.sh ||
    err "install.sh: unresolvable-pin refusal (never build a floating master tip) missing"

if [ "$fail" -ne 0 ]; then
    exit 1
fi
echo "check-install-drift: ok (sh REF default=${ref_default:-?}; ps1 arch fallback + pinning hooks present)"

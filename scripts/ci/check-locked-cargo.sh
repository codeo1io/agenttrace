#!/usr/bin/env bash
set -euo pipefail

# rm-157, cycle 1 ("Truthful posture, enforced gates").
#
# The reviewed Cargo.lock is a security surface: rm-009 pinned rustls
# through it and the deny job gates its advisories, but that guarantee
# stays convention-only unless every dependency-resolving cargo
# invocation in CI passes --locked and therefore refuses to regenerate
# the lockfile at build time. This gate fails when a
# `cargo test|build|clippy` line in .github/workflows/*.yml lacks
# --locked on the same line, so adding a new unpinned invocation breaks
# CI instead of silently drifting the reviewed lock.
#
# `cargo fmt` is exempt by construction: it resolves no dependencies.
# Local helper scripts (scripts/ci/check-rust-release-local.sh) and the
# user-facing install-from-source paths (install.sh, install.ps1) are
# out of scope by design — they run on arbitrary checkouts where the
# lockfile may legitimately need regenerating.

fail() {
	echo "check-locked-cargo: $*" >&2
	exit 1
}

[[ -d .github/workflows ]] || fail "run from the repository root"

status=0
while IFS= read -r offender; do
	echo "check-locked-cargo: dependency-resolving cargo invocation without --locked:" >&2
	echo "  $offender" >&2
	status=1
done < <(grep -nE 'cargo (test|build|clippy)' .github/workflows/*.yml | grep -v -- '--locked' || true)

[[ "$status" -eq 0 ]] ||
	fail "all workflow cargo test/build/clippy invocations must pass --locked"

echo "check-locked-cargo: all workflow cargo test/build/clippy invocations are --locked"

#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# rm-241 repo-side arm (run ce9309d9 cycle 1, batch "battery-truth
# hygiene"): collision-proof default target dir. The fleet hazard
# (assess f4cb5f63 F1: sibling runs sharing one CARGO_TARGET_DIR fed a
# gate a stale PRE-rm-1881 test binary while reporting green) enters
# through the environment; cargo's own default (<repo>/target) is safe
# but cold — it dies with every swept worktree. Default — only when
# CARGO_TARGET_DIR is UNSET — to a repo-root+HEAD-namespaced dir under
# the user cache: same worktree+HEAD stays warm across runs, different
# worktrees (or HEADs) never write one dir, and an explicitly exported
# CARGO_TARGET_DIR always wins.
if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
	at_head="$(git rev-parse HEAD 2>/dev/null || true)"
	at_cache_home="${XDG_CACHE_HOME:-${HOME:-}/.cache}"
	if [[ -n "$at_head" && -n "${HOME:-}" && -d "$at_cache_home" ]]; then
		# git hash-object for portability (sha256sum is absent on macOS runners)
		at_ns="$(printf '%s:%s' "$repo_root" "$at_head" | git hash-object --stdin | cut -c1-16)"
		export CARGO_TARGET_DIR="$at_cache_home/agenttrace-dev/target/at-$at_ns"
	fi
else
	case "$CARGO_TARGET_DIR" in
		*/agenttrace-dev/target/at-*) ;;
		*)
			echo "check-cargo-manifests: note: ambient CARGO_TARGET_DIR=$CARGO_TARGET_DIR looks like shared layout — it can serve stale sibling artifacts (rm-241); unset it to use the per-worktree namespaced default" >&2
			;;
	esac
fi

fail() {
	echo "check-cargo-manifests: $*" >&2
	exit 1
}

metadata_file="$(mktemp "${TMPDIR:-/tmp}/agenttrace-cargo-metadata.XXXXXX")"
cleanup() {
	rm -f "$metadata_file"
}
trap cleanup EXIT

cargo metadata --no-deps --format-version 1 >"$metadata_file"

node - "$metadata_file" <<'NODE'
const fs = require("fs");
const metadata = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const required = {
  description: "Local-first TUI and reports for AI coding-agent session history, cost, tokens, time, and slow-run diagnosis.",
  repository: "https://github.com/luoyuctl/agenttrace",
  homepage: "https://github.com/luoyuctl/agenttrace",
  license: "MIT",
  rust_version: "1.89",
};
const expectedKeywords = ["agent", "observability", "tui", "cli", "ai"];
const expectedCategories = ["command-line-utilities", "development-tools"];
const packages = new Map(metadata.packages.map((pkg) => [pkg.name, pkg]));

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

for (const name of ["agenttrace-core", "agenttrace-tui", "agenttrace"]) {
  const pkg = packages.get(name);
  assert(pkg, `missing workspace package ${name}`);
  for (const [field, expected] of Object.entries(required)) {
    assert(pkg[field] === expected, `${name} ${field}=${pkg[field]} expected ${expected}`);
  }
  assert(Array.isArray(pkg.publish) && pkg.publish.length === 0, `${name} should be marked publish=false until crates.io release is intentionally enabled`);
  assert(JSON.stringify(pkg.keywords) === JSON.stringify(expectedKeywords), `${name} keywords drifted`);
  assert(JSON.stringify(pkg.categories) === JSON.stringify(expectedCategories), `${name} categories drifted`);
}

const version = packages.get("agenttrace").version;
assert(version === "0.0.0-dev", `workspace version ${version} must use the development-version placeholder`);
for (const [name, pkg] of packages) {
  assert(pkg.version === version, `${name} version ${pkg.version} does not match workspace version ${version}`);
  for (const dep of pkg.dependencies) {
    if (packages.has(dep.name)) {
      assert(dep.req === `^${version}`, `${name} dependency ${dep.name} req ${dep.req} does not match ${version}`);
      assert(dep.path, `${name} dependency ${dep.name} should keep a local path`);
    }
  }
}
NODE

echo "Cargo manifest metadata is aligned."

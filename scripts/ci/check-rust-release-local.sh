#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# rm-1408: fleet CI runs cargo with incremental compilation inherited
# from the caller's ambient CARGO_INCREMENTAL. An ambient
# CARGO_INCREMENTAL=1 breaks this gate two ways: every invocation
# recompiles because this worktree's fingerprints disagree with the
# caller's incremental cache, and "cargo test"/"cargo build --release"
# leave *.fingerprint + incremental/ directories next to the artifacts
# under CARGO_TARGET_DIR — metadata the release gate never declared
# itself to consume (rm-377-class stale-binary false-greens). Export
# the hermetic default explicitly so the gate behaves the same in a
# dev shell (which may export CARGO_INCREMENTAL=1 for IDE builds) and
# in the fleet harness. The export applies to every cargo invocation
# below (fmt/clippy/test/build/metadata) and to the child gates that
# inherit the environment.
export CARGO_INCREMENTAL=0

# rm-391: write only under this invocation's own unique subdirectory. The
# old `rm -rf "$AGENTTRACE_CI_OUT"` destroyed sibling gate artifacts when
# the env var pointed at a shared directory (two release-local checks, or a
# release-local next to other gates sharing one AGENTTRACE_CI_OUT, wiped
# each other's logs mid-suite). Honor AGENTTRACE_BIN like the other gates
# instead of hardcoding target/release so CARGO_TARGET_DIR layouts work.
out_parent="${AGENTTRACE_CI_OUT:-/tmp/agenttrace-rust-release-local}"
mkdir -p "$out_parent"
out_dir="$(mktemp -d "$out_parent/release-local.XXXXXX")"
bin="${AGENTTRACE_BIN:-$repo_root/target/release/agenttrace}"

fail() {
  echo "check-rust-release-local: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

run_env() {
  echo "+ $*"
  env "$@"
}

mkdir -p "$out_dir/contracts" "$out_dir/real-cli-smoke"

run cargo fmt --check
run cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace -- -D warnings
run cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace
run cargo build --release -p agenttrace
run scripts/ci/check-cargo-manifests.sh

[[ -x "$bin" ]] || fail "release binary is not executable: $bin"
run "$bin" --version
run_env AGENTTRACE_BIN="$bin" scripts/ci/check-install-runtime.sh
run cargo test -p agenttrace --test entrypoints

run_env AGENTTRACE_BIN="$bin" AGENTTRACE_CI_OUT="$out_dir/contracts" \
  scripts/ci/check-output-contract.sh
run_env AGENTTRACE_BIN="$bin" AGENTTRACE_CI_OUT="$out_dir/contracts" \
  scripts/ci/check-deterministic-output.sh
run_env AGENTTRACE_BIN="$bin" AGENTTRACE_CI_OUT="$out_dir/contracts" \
  scripts/ci/check-report-semantics.sh
run_env AGENTTRACE_BIN="$bin" AGENTTRACE_CI_OUT="$out_dir/contracts" \
  scripts/ci/check-docs-commands.sh
run_env AGENTTRACE_BIN="$bin" AGENTTRACE_REAL_CLI_OUT="$out_dir/real-cli-smoke" \
  scripts/ci/check-rust-real-cli-smoke.sh

run scripts/ci/check-release-surfaces.sh
run ruby -c homebrew/Formula/agenttrace.rb
run bash -n scripts/record-demo.sh scripts/record-real-marketing.sh scripts/ci/*.sh

if [[ "${AGENTTRACE_SKIP_TUI_REAL_SMOKE:-}" == "1" ]]; then
  echo "Skipping Rust TUI real-data smoke because AGENTTRACE_SKIP_TUI_REAL_SMOKE=1"
else
  run_env AGENTTRACE_BIN="$bin" scripts/ci/check-rust-tui-real-smoke.sh
fi

echo "Rust local release gate passed: $bin"

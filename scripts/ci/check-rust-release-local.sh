#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# rm-391: write only under this invocation's own unique subdirectory. The
# old `rm -rf "$AGENTTRACE_CI_OUT"` destroyed sibling gate artifacts when
# the env var pointed at a shared directory (two release-local checks, or a
# release-local next to other gates sharing one AGENTTRACE_CI_OUT, wiped
# each other's logs mid-suite). Honor AGENTTRACE_BIN like the other gates
# instead of hardcoding target/release so CARGO_TARGET_DIR layouts work.
out_parent="${AGENTTRACE_CI_OUT:-/tmp/agenttrace-rust-release-local}"
mkdir -p "$out_parent"
out_dir="$(mktemp -d "$out_parent/release-local.XXXXXX")"
# rm-1925: this gate compiles and runs tests, so it must not silently
# inherit an ambient CARGO_TARGET_DIR. A target dir shared across
# concurrent checkouts produced false reds and a false green in the fleet
# (sibling worktrees clobbering each other's artifacts). Modes:
#   AGENTTRACE_GATE_TARGET_MODE=private -> lane-private mktemp target dir
#   AGENTTRACE_GATE_TARGET_MODE=inherit -> accept the ambient value as-is
#   unset (auto)                        -> ambient unset keeps the repo default
#     ($repo_root/target, per-checkout private); a SET ambient value is
#     refused unless it equals the repo default.
if [[ "${AGENTTRACE_GATE_TARGET_MODE:-}" == "private" ]]; then
  export CARGO_TARGET_DIR="$(mktemp -d "${TMPDIR:-/tmp}/agenttrace-gate-target.XXXXXXXX")"
elif [[ "${AGENTTRACE_GATE_TARGET_MODE:-}" != "inherit" \
  && -n "${CARGO_TARGET_DIR:-}" \
  && "${CARGO_TARGET_DIR}" != "$repo_root/target" ]]; then
  echo "gate-check-rust-release-local: refusing inherited CARGO_TARGET_DIR='$CARGO_TARGET_DIR': shared target dirs across concurrent checkouts yield false results (rm-1925). Set AGENTTRACE_GATE_TARGET_MODE=private for a lane-private dir, AGENTTRACE_GATE_TARGET_MODE=inherit to accept it, or unset CARGO_TARGET_DIR for the repo default." >&2
  exit 1
fi
bin="${AGENTTRACE_BIN:-${CARGO_TARGET_DIR:-$repo_root/target}/release/agenttrace}"

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

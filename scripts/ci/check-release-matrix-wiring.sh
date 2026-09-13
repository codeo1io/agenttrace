#!/usr/bin/env bash
# CU-24 (pass-11 F11-2): a workflow job whose strategy matrix declares
# per-leg `os:` keys must wire `runs-on` to the matrix (reference
# `matrix.os`), otherwise every leg schedules on one runner and
# OS-specific targets silently break — the exact regression introduced
# by 6632014 (literal replaced ${{ matrix.os }}) and never restored.
#
# Self-hosted policies satisfy the rule with e.g.
#   runs-on: ["self-hosted", "${{ matrix.os }}"]
# which is what release.yml ships.
#
# Usage: check-release-matrix-wiring.sh [workflows-dir]
# Exits 1 with file/job diagnostics when an unwired matrix is found.

set -euo pipefail

workflows_dir="${1:-.github/workflows}"
if [[ ! -d "$workflows_dir" ]]; then
  echo "check-release-matrix-wiring: not a directory: $workflows_dir" >&2
  exit 1
fi

status=0

for file in "$workflows_dir"/*.yml "$workflows_dir"/*.yaml; do
  [[ -f "$file" ]] || continue
  if ! awk -v "wf=$file" '
    BEGIN { in_jobs = 0; job = ""; job_line = 0; matrix_os = 0; runs_on = "" }
    /^jobs:/ { in_jobs = 1; next }
    !in_jobs { next }
    # Job headers: exactly two spaces of indent under `jobs:`.
    /^  [A-Za-z0-9_-]+:[[:space:]]*$/ {
      if (job != "" && matrix_os && runs_on !~ /matrix\.os/) {
        printf "check-release-matrix-wiring: %s:%d: job \"%s\" defines matrix os: keys but runs-on never references matrix.os (runs-on:%s)\n", wf, job_line, job, runs_on > "/dev/stderr"
        bad = 1
      }
      job = $0
      sub(/^  /, "", job); sub(/:$/, "", job)
      job_line = NR
      matrix_os = 0
      runs_on = ""
      next
    }
    # Per-leg `os:` keys: include-style (`- os: x`) or classic (`os: [x]`).
    /^[[:space:]]*-?[[:space:]]*os:[[:space:]]*[^[:space:]#]/ { matrix_os = 1 }
    /^[[:space:]]*runs-on:/ {
      line = $0
      sub(/^[[:space:]]*runs-on:/, "", line)
      runs_on = runs_on " " line
    }
    END {
      if (job != "" && matrix_os && runs_on !~ /matrix\.os/) {
        printf "check-release-matrix-wiring: %s:%d: job \"%s\" defines matrix os: keys but runs-on never references matrix.os (runs-on:%s)\n", wf, job_line, job, runs_on > "/dev/stderr"
        bad = 1
      }
      exit bad ? 1 : 0
    }
  ' "$file"; then
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  echo "check-release-matrix-wiring: unwired os: matrix detected (see above)" >&2
  exit 1
fi

echo "check-release-matrix-wiring: every matrix os: job wires runs-on to matrix.os"

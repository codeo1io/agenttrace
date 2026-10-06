#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "check-docs-commands: $*" >&2
  exit 1
}

# rm-369 (cycle 4): the binary default must be the repo's own release build,
# not /tmp/agenttrace — /tmp persists across sessions on the CI fleet and a
# days-old binary silently answered gate runs (observed on 2026-10-03).
# AGENTTRACE_BIN still overrides for A/B against a pinned build.
repo_root="$(git rev-parse --show-toplevel)" \
  || fail "must run inside the agenttrace repository (or set AGENTTRACE_BIN)"
bin="${AGENTTRACE_BIN:-$repo_root/target/release/agenttrace}"
out_dir="${AGENTTRACE_CI_OUT:-/tmp/agenttrace-ci}"

[[ -x "$bin" ]] || fail "agenttrace binary is not executable: $bin"
mkdir -p "$out_dir/docs"

"$bin" --version >"$out_dir/docs/version.txt"
"$bin" --doctor -f json >"$out_dir/docs/doctor.json"
"$bin" --demo --latest -f json >"$out_dir/docs/latest.json"
"$bin" --demo --latest --lang zh -f json >"$out_dir/docs/latest-zh.json"
"$bin" --demo --overview -f json >"$out_dir/docs/overview.json"
"$bin" --demo --search billing >"$out_dir/docs/search.txt"
"$bin" --demo --search internal/ws -f json >"$out_dir/docs/search.json"
"$bin" --demo --overview -f markdown -o "$out_dir/docs/overview.md" >"$out_dir/docs/overview-md.stdout"
"$bin" --demo --overview -f html -o "$out_dir/docs/overview.html" >"$out_dir/docs/overview-html.stdout"
for path in "$out_dir"/docs/*.json; do
  node -e 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))' "$path" \
    || fail "invalid JSON from documented command: $path"
done

set +e
"$bin" --demo --overview \
  --fail-under-health 80 \
  --fail-on-critical \
  --max-tool-fail-rate 15 \
  >"$out_dir/docs/gate.stdout" \
  2>"$out_dir/docs/gate.stderr"
status=$?
set -e
[[ "$status" -eq 2 ]] || fail "documented CI gate command should exit 2 for demo data, got $status"
grep -q 'Gate failed:' "$out_dir/docs/gate.stderr" \
  || fail "documented CI gate command should explain gate failures on stderr"

# Pass-8 F8-1/F8-7 docs contract (CU-15): the docs must tell the truth
# the code pins. A guide that claims schema 4 (code says 6) or a 24h
# automatic refresh (the code is network-free outside --update-pricing)
# is a lie with a CLI-contract lifespan.
guide="docs/guides/governance-reports.md"
[[ -f "$guide" ]] || fail "missing guide: $guide"

snapshot_schema=$(grep -oE 'const SQLITE_SNAPSHOT_SCHEMA_VERSION: i64 = [0-9]+' \
  crates/agenttrace-core/src/session_cache.rs | grep -oE '[0-9]+$')
session_schema=$(grep -oE 'const SESSION_CACHE_SCHEMA_VERSION: i64 = [0-9]+' \
  crates/agenttrace-core/src/session_cache.rs | grep -oE '[0-9]+$')
[[ -n "$snapshot_schema" && -n "$session_schema" ]] \
  || fail "could not read schema constants from session_cache.rs"
grep -q "SQLite snapshot is schema $snapshot_schema" "$guide" \
  || fail "guide must state the real SQLite snapshot schema ($snapshot_schema)"
grep -q "session cache is schema $session_schema" "$guide" \
  || fail "guide must state the real session cache schema ($session_schema)"
if grep -qiE 'refreshed automatically|refresh.*in the background|background.*refresh' "$guide"; then
  fail "guide must not claim automatic background refresh: pricing runs are network-free outside --update-pricing"
fi
if grep -qE 'schema 4' "$guide"; then
  fail "guide still claims the stale schema-4 snapshot version"
fi

# README must document the Go-flag argument-order trap (F8-8): flags
# after the first positional are ignored.
grep -q 'before the session path\|before the first positional' README.md \
  || fail "README must document that flags go before the positional session path"

# rm-512 (cycle 4): skills/*/SKILL.md ships operator-facing agenttrace
# invocations (e.g. --fail-under-health, --max-tool-fail-rate) that CI
# gates and operator runbooks depend on. A flag renamed or removed from
# the binary leaves every operator following the skill with a broken
# command. The docs gate must verify them like any other documented
# command: every flag used in a skills doc's agenttrace invocation must
# exist in --help output of the binary this gate ran against.
skills_help="$($bin --help 2>&1)"
[[ -n "$skills_help" ]] || fail "agenttrace --help produced no output"
skills_docs=()
shopt -s nullglob
for doc in skills/*/SKILL.md; do
  skills_docs+=("$doc")
done
shopt -u nullglob
[[ ${#skills_docs[@]} -gt 0 ]] \
  || fail "no skills/*/SKILL.md found: the operator-docs sweep has nothing to verify"
for doc in "${skills_docs[@]}"; do
  # Only lines that invoke the binary directly (fenced command lines,
  # indented or not) — prose and wrapper lines are out of contract.
  while IFS= read -r line; do
    # Word-split the invocation exactly like a shell would: each token
    # is either a long flag (possibly --flag=value), a short flag, or
    # not a flag. Boundary-grepping the raw line cannot work — adjacent
    # flags would contend for the space between them.
    # shellcheck disable=SC2206
    tokens=($line)
    for tok in "${tokens[@]}"; do
      flag="${tok%%=*}"
      case "$flag" in
        --[a-zA-Z0-9]*|-[a-zA-Z]) ;;
        *) continue ;;
      esac
      # Quote ERE metacharacters so a malformed token that slips past
      # the case glob (e.g. --foo(x) cannot turn the pattern invalid.
      flag_ere=$(printf '%s' "$flag" | sed -e 's/[][\\.*^$(){}?+|]/\\&/g')
      # here-string, not a pipe: grep -q exits on first match and would
      # close the read end while printf still writes the ~6KB help text,
      # and pipefail would turn that SIGPIPE(141) into a spurious
      # 'flag missing' failure on a correct tree.
      if ! grep -qE -- "(^|[[:space:]])${flag_ere}([[:space:]=,]|$)" <<<"$skills_help"; then
        fail "$doc invokes agenttrace with flag $flag, which is missing from --help"
      fi
    done
  done < <(grep -E '^[[:space:]]*agenttrace ' "$doc" || true)
done

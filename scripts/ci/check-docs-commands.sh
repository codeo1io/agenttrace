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
# rm-444: scope the doctor leg to a repo fixture corpus. Without -d
# it walked the operator's real HOME — >5 minutes with a debug binary
# on the CI fleet, first read as a gate hang. Every documented
# command/flag stays exercised (--doctor, -d, -f json); only the input
# corpus is scoped, mirroring the --demo legs below.
"$bin" --doctor -d "$repo_root/crates/agenttrace-core/tests/fixtures/pi-oh-my-pi" \
  -f json >"$out_dir/docs/doctor.json"
# rm-692: keep the documented storage-footprint command exercised
# (read-only; json so the doc-examples JSON validity check below covers it).
"$bin" --storage -f json >"$out_dir/docs/storage.json"
# rm-088: keep the documented completions lane exercised.
"$bin" --completions >"$out_dir/docs/completions.txt"
grep -q 'complete -F _agenttrace_complete agenttrace' "$out_dir/docs/completions.txt" \
  || fail "documented completions command must emit the bash completion hook"
"$bin" --demo --latest -f json >"$out_dir/docs/latest.json"
"$bin" --demo --latest --lang zh -f json >"$out_dir/docs/latest-zh.json"
"$bin" --demo --overview -f json >"$out_dir/docs/overview.json"
"$bin" --demo --search billing >"$out_dir/docs/search.txt"
"$bin" --demo --search internal/ws -f json >"$out_dir/docs/search.json"
"$bin" --demo --overview -f markdown -o "$out_dir/docs/overview.md" >"$out_dir/docs/overview-md.stdout"
"$bin" --demo --overview -f html -o "$out_dir/docs/overview.html" >"$out_dir/docs/overview-html.stdout"
# rm-576: keep the documented usage-card command exercised.
"$bin" --demo --overview -f svg --card-theme auto -o "$out_dir/docs/overview.svg" >"$out_dir/docs/overview-svg.stdout"
grep -q '^<?xml version' "$out_dir/docs/overview.svg" \
  || fail "documented svg card command must produce SVG markup"
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
# The 4 must be a COMPLETE number, not a digit prefix: the session cache
# ceiling crossed 40 at run 4c3ca863's integration (by_branch invalidation),
# and the truthful "schema 40" sentence the dynamic check above REQUIRES
# matches a bare `schema 4` grep — a false fail that would block the whole
# docs lane on the correct sentence.
if grep -qE 'schema 4([^0-9]|$)' "$guide"; then
  fail "guide still claims the stale schema-4 snapshot version"
fi

# README must document the Go-flag argument-order trap (F8-8): flags
# after the first positional are ignored.
grep -q 'before the session path\|before the first positional' README.md \
  || fail "README must document that flags go before the positional session path"

# rm-598 (cycle 4): skills/*/SKILL.md ships operator-facing agenttrace
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
# rm-207: the README flag reference must stay in lockstep with the
# binary, and no flag's --help description may regress to blank. Both
# sides move alone in one refactor; this gate fails on either drift.
help_text="$("$bin" --help 2>/dev/null || true)"
bin_flags=$(printf '%s\n' "$help_text" | grep -cE '^ {2,6}(-[a-zA-Z], )?--')
readme_flags=$(grep -cE '^\| `(-[a-zA-Z], )?--' README.md)
[[ "$bin_flags" -gt 40 ]] || fail "--help enumerated only $bin_flags flags; binary or parser regressed"
[[ "$bin_flags" -eq "$readme_flags" ]] \
  || fail "README flag table ($readme_flags rows) must match --help ($bin_flags flags)"
blank_docs=$(printf '%s\n' "$help_text" | awk '
  /^ {2,6}(-[a-zA-Z], )?--/ {
    if (prev_opt) count++
    prev_opt = 1
    next
  }
  { if (prev_opt && $0 !~ /^ {10}/) count++; prev_opt = 0 }
  END { print count + 0 }
')
[[ "$blank_docs" -eq 0 ]] \
  || fail "$blank_docs flag(s) render a blank --help description; every Args field needs a doc comment"

# rm-455: the MCP server guide must exist and pin the local-truth
# posture truthfully — read-only, stdio-only, no network — the same
# one-truth rule the other guides carry. The guide must also keep the
# `mcp` keyword disambiguated from the unrelated --mcp-governance
# report flag, and the binary must actually expose the keyword.
guide="$repo_root/docs/guides/mcp-server.md"
[[ -f "$guide" ]] || fail "docs/guides/mcp-server.md is required (rm-455)"
grep -q 'agenttrace mcp' "$guide" \
  || fail "mcp-server.md must show the 'agenttrace mcp' host command"
grep -q 'read-only\|Read-only' "$guide" \
  || fail "mcp-server.md must state the read-only posture"
grep -q 'no network\|No network' "$guide" \
  || fail "mcp-server.md must state the no-network posture"
grep -q -- '--mcp-governance' "$guide" \
  || fail "mcp-server.md must disambiguate the --mcp-governance report flag"
"$bin" mcp --help >"$out_dir/docs/mcp-help.txt"
grep -q 'agenttrace mcp' "$out_dir/docs/mcp-help.txt" \
  || fail "'agenttrace mcp --help' must render the keyword help route"
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
  | "$bin" mcp >"$out_dir/docs/mcp-tools.json"
node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")); const names=r.result.tools.map(t=>t.name).sort(); if (names.join(",") !== "by_model_breakdown,usage_overview") { console.error("unexpected tools: " + names); process.exit(1); }' "$out_dir/docs/mcp-tools.json" \
  || fail "the mcp server must expose exactly usage_overview and by_model_breakdown"

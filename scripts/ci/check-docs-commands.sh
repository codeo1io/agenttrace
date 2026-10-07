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
"$bin" --demo --latest -f json >"$out_dir/docs/latest.json"
"$bin" --demo --latest --lang zh -f json >"$out_dir/docs/latest-zh.json"
"$bin" --demo --overview -f json >"$out_dir/docs/overview.json"
# rm-207-ext: the otel OTLP export is a documented format now — keep a
# real invocation in the gate so a ValueEnum/serializer regression fails
# here, not in an operator's collector.
"$bin" --demo --overview -f otel >"$out_dir/docs/overview.otel.json"
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

# rm-207-ext rider (cycle 2): two sweeps the skills sweep cannot see.
#
# Both read ONLY fenced code blocks (```-delimited): prose can
# legitimately start a line with the word `agenttrace` ("agenttrace
# supports multiple session formats...") without invoking anything, so
# a raw line-leading grep would flag sentence verbs as missing
# subcommands.
fenced_agenttrace_lines() {
  awk '
    /^```/ { fenced = !fenced; next }
    fenced && /^[[:space:]]*(\$[[:space:]]+)?agenttrace / { print }
  ' "$1"
}

# (1) FORMAT VALUES: every documented `-f X` / `--format X` in the
# operator docs must be a possible value of --format in the binary this
# gate ran against. A format renamed or dropped from the ValueEnum
# (e.g. the `otel` OTLP export) leaves every documented invocation
# erroring at parse time — earlier than any flag check can catch it.
# The possible-values list is read from --help next to the -f/--format
# entry, not hardcoded here, so the gate tracks the binary by
# construction.
format_values="$(
  awk '
    /-f, --format <FORMAT>/ { in_format = 1; next }
    in_format && /possible values:/ {
      line = $0
      sub(/.*possible values: /, "", line)
      sub(/\].*/, "", line)
      gsub(/[[:space:]]/, "", line)
      print line
      exit
    }
    in_format && NF == 0 { exit }
  ' <<<"$skills_help"
)"
[[ -n "$format_values" ]] || fail "could not read --format possible values from --help"
for doc in README.md docs/guides/*.md skills/*/SKILL.md; do
  [[ -f "$doc" ]] || continue
  while IFS= read -r used; do
    [[ -n "$used" ]] || continue
    case ",$format_values," in
      *",$used,"*) ;;
      *) fail "$doc uses format '$used', which is not a --format possible value ($format_values)" ;;
    esac
  done < <(
    fenced_agenttrace_lines "$doc" \
      | grep -oE -- '(^|[^a-zA-Z0-9-])(-f|--format)[[:space:]]*[=]?[[:space:]]*[a-z]+' \
      | grep -oE '[a-z]+$' | sort -u || true
  )
done

# (2) HIDDEN HOST COMMANDS: `statusline` and `upstream` are dispatched
# as positionals BEFORE action validation (main.rs
# `args.path.as_deref() == Some(...)`), so they never appear in --help
# and the flag sweep cannot vouch for them. Every bare subcommand word
# the operator docs invoke on a command line must therefore be
# recognized by the dispatcher. The exclusion class
# `[^a-zA-Z0-9./-]` skips path-shaped positionals (`sessions.jsonl`,
# `path/to/session.jsonl`): a dot or slash after the word means the
# docs mean a file, not a host command.
dispatcher="$repo_root/crates/agenttrace-cli/src/main.rs"
[[ -f "$dispatcher" ]] || fail "dispatcher source not found: $dispatcher"
recognized="$(
  grep -oE 'args\.path\.as_deref\(\) == Some\("[a-z][a-z0-9-]+"\)' "$dispatcher" \
    | grep -oE '"[a-z][a-z0-9-]+"' | tr -d '"' | tr '\n' ',' | sed 's/,$//'
)"
[[ -n "$recognized" ]] || fail "no host commands found in $dispatcher"
for doc in README.md docs/guides/*.md skills/*/SKILL.md; do
  [[ -f "$doc" ]] || continue
  while IFS= read -r word; do
    [[ -n "$word" ]] || continue
    case ",$recognized," in
      *",$word,"*) ;;
      *) fail "$doc invokes 'agenttrace $word' as a bare subcommand, but the dispatcher recognizes only: $recognized" ;;
    esac
  done < <(
    fenced_agenttrace_lines "$doc" \
      | grep -E '^[[:space:]]*(\$[[:space:]]+)?agenttrace [a-z][a-z0-9-]*[^a-zA-Z0-9./-]' \
      | grep -oE '^[[:space:]]*(\$[[:space:]]+)?agenttrace [a-z][a-z0-9-]*' \
      | awk '{print $NF}' | sort -u || true
  )
done

#!/usr/bin/env bash
set -euo pipefail

# rm-157, cycle 1 ("Truthful posture, enforced gates"): the original
# same-line grep gate.
#
# rm-226, cycle 1 ("Trustworthy evidence & gates"): parser rewrite.
# The grep had two classes of wrong answers on real workflow YAML:
#
#   FALSE NEGATIVE — `run: cargo test -p agenttrace # TODO re-add
#   --locked` looks compliant because "--locked" appears on the line,
#   but it is inside a shell comment: the invocation is unpinned.
#
#   FALSE POSITIVE — a `run: |` block whose command is continued with
#   a real backslash continuation puts --locked on the NEXT line; the
#   same-line grep reports an offender for a compliant invocation.
#
# Both matter for the same reason as the gate itself: the reviewed
# Cargo.lock is a security surface (rm-009 pinned rustls through it;
# the deny job gates its advisories), and that guarantee stays
# convention-only unless every dependency-resolving cargo invocation
# in CI passes --locked and therefore refuses to regenerate the
# lockfile at build time. A gate that can be fooled by a comment is a
# gate that certifies posture it did not check.
#
# The scanner below understands the YAML shapes actually shipped in
# .github/workflows (and the ones reviewers are likely to add):
#   * plain `run: cargo ...` single-line commands,
#   * plain scalar continuations (more-indented follow-on lines),
#   * literal `run: |` blocks — each physical line is its own shell
#     command; trailing backslashes join lines first,
#   * folded `run: >` scalars — lines fold into one command,
#   * multi-command lines (`&&`, `||`, `;`, `|`) — checked per segment,
#   * comments (`#` to end of line) — stripped before the check.
#
# `cargo fmt` is exempt by construction: it resolves no dependencies.
# Local helper scripts (scripts/ci/check-rust-release-local.sh) and the
# user-facing install-from-source paths (install.sh, install.ps1) are
# out of scope by design — they run on arbitrary checkouts where the
# lockfile may legitimately need regenerating.
#
# --self-test runs an embedded fixture suite (rm-226) so the two
# failure classes above stay fixed; it needs no repository checkout.

# Single source of truth for the dependency-resolving cargo verbs this
# gate covers. Widening the gate (the standing sibling asks rm-178 /
# rm-192 / rm-193, each of which proposed adding verbs) is a one-word
# edit HERE, not three divergent greps: the live scan, the message
# below, and the self-test all read this variable.
CARGO_VERBS='test|build|clippy|run|bench|install|tree'

fail() {
	echo "check-locked-cargo: $*" >&2
	exit 1
}

# Reads one workflow file on stdin; prints `path:line: command` for
# every dependency-resolving cargo segment that is not --locked.
scan_stream() {
	local path="$1"
	awk -v verbs="$CARGO_VERBS" -v source="$path" '
		function indent_of(text) {
			match(text, /^[ \t]*/)
			return RLENGTH
		}

		# Check one shell command line (already continuation-joined).
		function emit(text, segments, count, i, segment, line) {
			line = text
			# Strip a comment: from the first "#" at start or after
			# whitespace. This is the rm-226 false-negative fix: an
			# "--locked" that only exists in a comment no longer
			# certifies the invocation.
			if (match(line, /(^|[ \t])#/))
				line = substr(line, 1, RSTART - 1)
			count = split(line, segments, /&&|\|\||;|\|/)
			for (i = 1; i <= count; i++) {
				segment = segments[i]
				if (segment ~ ("cargo[ \t]+(" verbs ")") &&
				    segment !~ /--locked/) {
					gsub(/^[ \t]+/, "", segment)
				gsub(/[ \t]+$/, "", segment)
				print source ":" nr ": " segment
			}
			}
		}

		function flush() {
			if (cmd != "") {
				emit(cmd)
				cmd = ""
			}
		}

		# Absorb one collected line. Literal-block lines are separate
		# shell commands unless the accumulated text ends with a
		# backslash (a real shell continuation — the rm-226
		# false-positive fix: `cargo test \` + `--locked` is compliant).
		# Plain/folded scalars fold into one command instead.
		function absorb(text, piece) {
			if (collect_indent < 0 && text !~ /^[ \t]*$/) {
				collect_indent = indent_of(text)
				if (collect_indent <= run_indent) {
					in_collect = 0
					return
				}
			}
			if (text ~ /^[ \t]*$/)
				return
			piece = text
			gsub(/^[ \t]+/, "", piece)
			gsub(/[ \t]+$/, "", piece)
			if (folding) {
				cmd = (cmd == "" ? piece : cmd " " piece)
			} else if (cmd ~ /\\$/) {
				sub(/\\$/, "", cmd)
				cmd = cmd " " piece
			} else {
				flush()
				cmd = piece
				nr = NR
			}
		}

		{
			if (in_collect) {
				# A new mapping key or list item at or above the run
				# indent ends the scalar.
				if (indent_of($0) <= run_indent &&
				    $0 !~ /^[ \t]*$/) {
					in_collect = 0
					flush()
				} else if ($0 ~ /^[ \t]*(-[ \t]+[A-Za-z_]|[A-Za-z_][A-Za-z0-9_-]*:)/ &&
				         indent_of($0) <= run_indent + 2) {
					in_collect = 0
					flush()
				} else {
					absorb($0)
					next
				}
			}
			if ($0 ~ /^[ \t]*(-[ \t]+)?run:/) {
				run_indent = indent_of($0)
				rest = $0
				sub(/^[ \t]*(-[ \t]+)?run:[ \t]*/, "", rest)
				nr = NR
				cmd = ""
				if (rest ~ /^[|>][0-9]*[+-]?[ \t]*$/) {
					# Block scalar: content is strictly indented under
					# the key; folded styles join into one command.
					in_collect = 1
					folding = (rest ~ /^>/)
					collect_indent = -1
				} else if (rest != "") {
					in_collect = 1
					folding = 1
					collect_indent = run_indent
					cmd = rest
				}
			}
		}

		END { flush() }
	' "$path"
}

scan_workflow() {
	local path="$1"
	scan_stream "$path"
}

self_test() {
	local dir offenses
	dir="$(mktemp -d)"

	cat >"$dir/locked.yml" <<'EOF'
name: compliant
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --locked
      - run: cargo clippy --locked -- -D warnings
      - run: cargo fmt --all --check
      - run: cargo deny check
      - run: cargo build --locked --release -p agenttrace
      - run: |
          cargo build --locked \
            --release --all-features
      - run: >
          cargo test
          --locked
      - run: cargo test
        --locked
      - run: cd crates && cargo test --locked && cargo clippy --locked
EOF

	cat >"$dir/offenders.yml" <<'EOF'
name: offenders
jobs:
  build:
    steps:
      - run: cargo test
      - run: cargo test -p agenttrace # TODO re-add --locked
      - run: |
          cargo build
          cargo test --locked
      - run: cargo bench
      - run: cd crates && cargo build
EOF

	offenses="$(scan_workflow "$dir/offenders.yml" || true)"
	local expected=5
	local actual
	actual="$(printf '%s\n' "$offenses" | grep -c 'cargo' || true)"
	[[ "$actual" -eq "$expected" ]] ||
		fail "self-test: expected $expected offenders, got $actual:
$offenses"
	printf '%s\n' "$offenses" | grep -q 'cargo test -p agenttrace' ||
		fail "self-test: comment-hidden --locked (rm-226 false negative) must be an offender:
$offenses"
	printf '%s\n' "$offenses" | grep -q 'cargo build$' ||
		fail "self-test: unpinned build inside a mixed | block must be an offender:
$offenses"
	printf '%s\n' "$offenses" | grep -q ':11: cargo build' ||
		fail "self-test: unpinned segment after && must be an offender:
$offenses"
	printf '%s\n' "$offenses" | grep -q -- '--locked' &&
		fail "self-test: compliant text must not appear in the offender list:
$offenses"

	local clean
	clean="$(scan_workflow "$dir/locked.yml" || true)"
	[[ -z "$clean" ]] ||
		fail "self-test: the compliant fixture must yield no offenders:
$clean"
	# rm-226 false-positive regression: the backslash-continued block
	# and the plain continuation must both be recognized as compliant.
	scan_workflow "$dir/locked.yml" | grep -q 'cargo build' &&
		fail "self-test: backslash continuation (rm-226 false positive) must not be an offender"

	rm -rf "$dir"
	echo "check-locked-cargo: self-test passed (fixtures: 5 offenders caught, 0 false positives)"
}

case "${1:-}" in
--self-test)
	self_test
	exit 0
	;;
"") ;;
*)
	fail "unknown argument: $1 (only --self-test is supported)"
	;;
esac

[[ -d .github/workflows ]] || fail "run from the repository root"

status=0
while IFS= read -r offender; do
	echo "check-locked-cargo: dependency-resolving cargo invocation without --locked:" >&2
	echo "  $offender" >&2
	status=1
done < <(for workflow in .github/workflows/*.yml; do
	scan_workflow "$workflow"
done)

[[ "$status" -eq 0 ]] ||
	fail "all workflow cargo $CARGO_VERBS invocations must pass --locked"

echo "check-locked-cargo: all workflow cargo $CARGO_VERBS invocations are --locked"

# Cycle-7 learnings and prevention rules — 2026-09-14

Compound record for maintenance cycle 7 ("truthful accounting and an
unbroken release path", units CU-23..CU-26 + the F11-7 ride-along),
run `58910360ca80424aa2e3c8c9820e6ee5`. Sources: pre-review cycle
evidence only (pass-11 assessment, research pass 9, roadmap, cycle-7
prioritization, implementation, targeted test outcomes). Review and
shipping outcomes are intentionally absent — the next cycle's
assessment carries them forward. Status annotations live in ROADMAP.md
(dated, appended, never rewritten).

## What cycle 7 changed (state at compound time)

- Accounting truth (CU-23 / F11-1, HIGH): result-only tool sources can
  no longer report false 100% failure rates; the Antigravity fixture
  now shows ok=4/fail=1/total=5, success 80%, medium (not HIGH)
  anomaly, and the `--max-tool-fail-rate` gate fails truthfully at
  20.0% instead of 100%.
- Release path wiring (CU-24 / F11-2, HIGH): every `os:` matrix leg
  targets a matching runner via
  `runs-on: ["self-hosted", "${{ matrix.os }}"]` under the operator's
  self-hosted-only policy; a guard (`scripts/ci/`
  `check-release-matrix-wiring.sh`) fails any future job whose matrix
  declares `os:` keys an unwired `runs-on` ignores, and CI runs it.
- Upstream parity (CU-25): `6848aa1` / PR #284 ported (skip leading
  non-session Oh My Pi lines), keeping fork HEAD at parity with the
  single upstream commit it lacked.
- Input honesty (CU-26 / F11-3): baselines must be agenttrace reports;
  garbage now fails with a named, actionable error instead of an
  all-zero comparison verdict.
- CLI shim correctness (F11-7): `--no-baseline-gate` treated as the
  boolean it is.
- All verified: 218/218 tests, clippy `-D warnings`, fmt, eight
  script gates on the release binary, live inversions per unit.

## Reusable lessons and prevention rules

Each rule names its trigger so the next cycle can apply it
mechanically.

1. **A guard is unproven until it has failed.** Cycle 7's matrix-wiring
   guard first shipped with its awk invocation missing the input file
   operand: it read stdin, matched nothing, and exited 0 — a vacuous
   pass that "verified" nothing. The injected-failure test (copy the
   workflow, break the wiring, expect exit 1 with diagnostics) is what
   caught it. *Rule: every new check script lands together with a
   red-path demo — mutate a scratch copy, assert non-zero exit and a
   precise message — and the demo command is recorded in the change's
   evidence.* This extends the cycle-6 F1 rule (fixtures copy wire
   shapes from producer source) with its dual: negative tests prove
   the checker, not just the checked.

2. **Policy edits to `runs-on` must be matrix-aware.** History: `e005952`
   wired `${{ matrix.os }}`; `6632014` swapped in a literal and silently
   stranded four of six targets; the later policy commits (`bfa4f22`,
   `fbbf751`, `c148ffc`) restored literals everywhere and deleted the
   old guard without noticing the matrix. *Rule: any edit to a
   `runs-on:` in a job that has — or later gains — a `strategy.matrix`
   must either reference `matrix.*` or carry a comment stating why a
   literal is correct; the guard enforces the matrix case, and the
   both-labels form `["self-hosted", "${{ matrix.os }}"]` satisfies a
   self-hosted-only policy while restoring per-leg targeting.* Operator
   prerequisite recorded in the workflow comment: label the self-hosted
   fleet with the matrix `os:` values (or adjust the matrix to the
   fleet's labels).

3. **Totals must never undercut observed outcomes.** The F11-1 bug class:
   a clamp (`ok ≤ total − fail`) that is correct for paired streams
   silently erases successes when a parser emits result-only events
   (no paired assistant call), because `total` undercounts real
   invocations. *Rule: any aggregate that clamps one counter against
   another must first raise the denominator to the sum of observed
   outcomes; new parsers emitting role-`tool` events without paired
   calls inherit correct accounting automatically (candidate 52's
   Antigravity store decode must keep this invariant — its acceptance
   already says so).* The clamp itself stays: it only ever caps `ok`.

4. **Identical strings can mark different semantics — pin edits by line
   and context, not by search.** `bail!("oh_my_pi: missing session\n   header")` exists twice in parser.rs with different meanings: the
   leading-line loop (~:1325, "skip what precedes the header" — the
   port target) and the end-of-file arm (~:1400, "no header ever
   seen" — a different defect class that correctly stays). *Rule: when
   a fix targets one of several identical strings, the stewardship
   record must name file:line plus surrounding context, and the patch
   must quote the chosen arm in its diff.*

5. **Acceptance criteria must be internally consistent, and resolution
   goes to ground truth.** CU-23's acceptance text asked for "gate
   exit 0" while the fixture's true failure rate (20%) makes a
   threshold-15 gate correctly fail. *Rule: when acceptance text
   self-contradicts, resolve against the reproduced ground truth
   (here: exit 2 at 15 with the TRUE 20.0% message, exit 0 at 25) and
   record the resolution in the implementation record — never tune the
   fixture to make stale text pass.*

6. **Value-flag lists drift; pin each flag's class with a test.** F11-7:
   `flag_takes_value` had accumulated `--no-baseline-gate`, a boolean,
   so the Go-compat shim swallowed the next flag and truncated the
   command line. *Rule: every flag added to the CLI gets its shim
   class (value vs boolean) asserted in `main.rs`'s shim tests at
   introduction time; the remaining P4-2 warning work should add the
   lint-style check that flags-after-positional are dropped loudly.*

7. **Externally supplied comparison inputs are validated before they
   steer a gate.** F11-3's bug class: `add_baseline_comparison` trusted
   any JSON and produced an all-zero baseline — a misleading verdict
   from garbage input. *Rule: any artifact consumed by a gate or
   threshold decision validates its shape first and fails with a
   named, actionable error naming how to produce a valid artifact
   (`summary` object required here; the error names the export
   command).*

## Context left for the next cycle

- **Top hardening remainder:** F11-4 token-estimator unification
  (`diagnostics.rs:771` bytes/2 vs `estimate_tokens_from_text`
  `lib.rs:582`), including replacing the hardcoded 131,072-token
  window with candidate-4 metadata. Low severity, now the lane's
  smallest standalone unit — a natural ride-along.
- **Largest ready batch:** the candidate-23 dependency wave
  (upstream PRs #279/#278/#272/#259: crossterm 0.29, rusqlite 0.40,
  clap patch, serde/thiserror patches) — needs its own verification
  posture (full-suite + TUI smoke after rusqlite/crossterm jumps).
- **Still gated:** the fork dependency-review workflow fix waits for a
  push-bearing phase; candidate 53 (calendar buckets, as-of pricing)
  rides the still-open P3-2; candidate 51 and the parse-size-cap and
  installer-checksum items remain from the cycle-6 deferrals.
- **Upstream watch state at compound time:** exactly one commit
  (`6848aa1`) postdated fork HEAD and it is now ported; upstream
  remains active (push 2026-09-11), so the next research pass should
  re-census from the post-port baseline.
- **Pending runner-level evidence for CU-24:** all six legs scheduled
  on distinct runners and the four target artifacts on the next tag —
  both require Actions runs outside this cycle's permitted phases.
- **Operator action item:** label the self-hosted runner fleet with
  the matrix `os:` values (or remap the matrix to the fleet's labels)
  before the next release tag.

## Pointers

- Implementation evidence:
  `.conductor/implement/2026-09-14-cycle7-implementation-record.md`
  (run-scoped).
- Assessment: `docs/reviews/2026-09-14-adversarial-repository-assessment-pass11.md`;
  research: `docs/research/2026-09-14-extensions-research-pass9.md`.
- Cycle-6 precedent for this record shape:
  `docs/stewardship/2026-09-03-cycle6-implementation-record.md`.

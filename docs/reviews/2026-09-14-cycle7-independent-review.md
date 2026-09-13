---
type: review-record
id: cycle7-independent-review
cycle: 7
review-pass: 12
date: 2026-09-14
run: 58910360ca80424aa2e3c8c9820e6ee5
attempt: 3399fb7fe83a4fd7ae80ef32de311619
base_commit: df3b621
reviewed_tree: uncommitted working tree (9 modified + 5 untracked code/test/doc assets + run-scoped .conductor/)
status: completed
---

# Cycle-7 independent adversarial review (pass 12)

Independent re-verification of the CU-23..CU-26 implementation plus the
F11-7 ride-along against the cycle goals
(`.conductor/prioritize/2026-09-14-cycle7-batch.md`), the ROADMAP
acceptance criteria as annotated by the compound pass, the security
boundaries (no network, no local-path/secrets leakage in the shippable
diff), durability/recovery requirements (release path, gate honesty),
and the test evidence claimed by the implement / targeted_tests /
compound phases. Nothing from prior phases was trusted without
re-running it; every behavioral claim below was reproduced live this
pass. No code was modified — findings are recorded for the commit gate.

## Verdict

**PASS WITH FINDINGS — none blocking.** The batch is true to its
"truthful accounting and an unbroken release path" goals: CU-23 fully
retires the F11-1 false-gate-red defect and its arithmetic is now
pinned by three tests plus a live inversion; CU-24 restores per-leg
runner targeting with a working (though heuristic — see R1/R2) guard;
CU-25 is a faithful minimal port of upstream `6848aa1`; CU-26 turns
garbage baselines into named errors; F11-7 fixes the shim flag class.
The compound pass's ROADMAP annotations and learnings record were
checked claim-by-claim against live behavior and are accurate, honest
about what remains PENDING (CU-24 runner-level evidence), and the one
acceptance-text inconsistency (F11-1's "gate exit 0") is resolved the
right way — against ground truth, not by tuning the fixture. Findings
R1-R3 are low-severity hardening follow-ups; R4/R5 are confirmations
of disclosed trade-offs, carried here so the commit/release notes do
not lose them.

## Re-verified live this pass (evidence)

- `cargo test` (full workspace) → **218 passed, 0 failed** across the
  8 suites (14 cli, 6 entrypoints, 2 launch_guards, 79 core lib, 8
  demo_contract, 69 discovery_contract, 40 tui, 0+0 doc-tests) — matches
  the implement and targeted_tests claims exactly, including the five
  new tests (`result_only_tool_outcomes_cannot_be_clamped_away` at
  lib.rs:1645, `rust_parses_oh_my_pi_jsonl_with_leading_title_line`
  and `rust_accounts_antigravity_result_only_tool_outcomes` at
  discovery_contract.rs:1689/1717,
  `baseline_comparison_rejects_non_report_json` at demo_contract.rs:187,
  `go_flag_shim_treats_no_baseline_gate_as_a_boolean_flag` at
  main.rs:1335).
- `cargo clippy --workspace --all-targets -- -D warnings` → clean;
  `cargo fmt --check` → clean.
- All eight `scripts/ci/` gates exit 0 on the shipped tree, including
  the new `check-release-matrix-wiring.sh` (also re-proven red on
  injected regressions, see R1 for the one style it misses).
- **CU-23 live inversion** (release binary, `testdata/antigravity-mixed-outcomes.json`
  copied to a scratch dir): `-f json` → `tool_calls_ok=4, fail=1,
  total=5, tool_success_rate=80`, anomaly `tool_failures` **medium**
  `1/5 failed (20%)` (thresholds >30% high / >15% medium at
  lib.rs:807-831), `health_score=58` (was 40 in the assess repro; the
  remaining `shallow_thinking` high is fixture-inherent — `thinking:
  "hmm"`). `--overview --max-tool-fail-rate 15` → exit 2 with the TRUE
  `tool failure rate 20.0% exceeds 15.0%`; threshold 25 → exit 0.
  Exactly as the ROADMAP status-change records.
- **CU-24 statics**: `release.yml:49` = `runs-on: ["self-hosted",
  "${{ matrix.os }}"]` over the six include legs (`:52-67`); guard green
  on the shipped tree (all three workflow files); `ci.yml:69-70` runs
  it after "Release surface drift"; `python3 yaml.safe_load` parses
  both workflows (targeted_tests claim re-checked by inspection).
- **CU-25 live**: `agenttrace -f json testdata/oh-my-pi-title-prefix.jsonl`
  → exit 0, `source_tool: oh_my_pi`, model parsed; the old
  `missing session header` bail is gone from the leading-line arm
  (parser.rs:1323-1330) and preserved at the end-of-file arm (~:1400,
  the "header never seen" class). The fixture file and the inline test
  content are byte-identical (verified by extraction + diff).
- **CU-26 live**: `--overview --baseline bad.json -f json` on
  `{"foo":1}` and `[1,2,3]` → exit 1 with the named error `baseline
  file is not an agenttrace report (missing `summary` object): <path> —
  export one with `agenttrace --overview -f json``; a real report as
  baseline → exit 0.
- **F11-7 live**: `agenttrace --no-baseline-gate -f json <fixture>` →
  JSON on stdout, exit 0 (the shim no longer swallows `-f`).
- **Security/durability boundaries**: the full shippable diff (9
  modified + 5 untracked files) greps clean for `/home/agent`,
  `/work/projects`, LAN/VPN markers, and the fork org — zero leakage;
  no new network surface, no new unbounded parse (the CU-25 skip is an
  iterator `continue`), no new file writes outside `std::env::temp_dir()`
  in tests. CU-23 only ever *raises* `tool_calls_total` to observed
  outcomes (lib.rs:758-761) and the clamp still caps `ok` alone
  (lib.rs:762-767), so paired streams (total > outcomes) are unchanged
  — pinned by the second half of the new lib test.

## Findings

- **R1 (low) — guard false negative on block-style classic matrices.**
  `scripts/ci/check-release-matrix-wiring.sh:45` matches `os:` only
  when a value shares the line (`os: [x]` or `- os: x`). A classic
  matrix written as a block list (`os:` newline `- ubuntu-latest` …)
  with an unwired `runs-on` passes the guard. Live repro: scratch
  workflow with `matrix: os:` block list + `runs-on: self-hosted` →
  guard exit 0. The shipped release matrix is include-style and wired,
  so this is a regression-tripwire gap, not a live defect; the
  implementation record's "handles include-style and classic matrices"
  is accurate only for the inline classic form. Fix: also match
  `^[[:space:]]*os:[[:space:]]*$` (and optionally the following
  `- value` lines), or move the guard to a YAML-aware check.
- **R2 (low) — guard false positive on stray `os:` keys.** The same
  regex trips on any `os:` key anywhere inside a job body (a step
  `with:`/`env:` map, a container config), demanding `matrix.os` in
  `runs-on` for jobs with no matrix at all. Live repro: `with: os:
  linux` under a plain step → exit 1 with a bogus diagnostic.
  Fail-closed, so safe, but a spurious CI red is possible if any
  workflow ever gains such a key. Fix: track `strategy:`/`matrix:`
  context in the awk state machine instead of matching bare `os:`.
- **R3 (low) — CU-26 validation is key-presence only.**
  `crates/agenttrace-core/src/reports.rs:719-722` accepts any JSON
  carrying a `summary` *object*, including `{"summary":{}}`. Live: an
  empty-summary baseline still produces an all-zero comparison and a
  gate exit 2 (`baseline regression above threshold`,
  `slower_than_baseline: true` against a meaningless zero baseline).
  The direction is loud-wrong rather than the silent-wrong F11-3
  fixed, and the ROADMAP annotation honestly narrows the acceptance to
  "the `summary` object the comparator reads" — but the shallow shape
  check leaves a one-key hole. Fix: require a non-empty summary, or at
  least one of the consumed numeric keys
  (`total_duration_seconds`/`total_cost`/token totals).
- **R4 (info, disclosed) — CU-24 operational prerequisite.** The
  both-labels form at `release.yml:49` schedules a leg only on a
  self-hosted runner carrying the matching os label
  (`ubuntu-latest`, `macos-15-intel`, `macos-14`, `windows-latest`).
  Self-hosted runners do NOT carry those labels by default (defaults
  are `self-hosted`, `linux|macos|windows`, arch), so until the
  operator labels the fleet (or the matrix is remapped to the fleet's
  labels), all six build legs queue and the publish job still cannot
  run. This is properly disclosed in the workflow comment
  (`release.yml:40-48`), the ROADMAP status-change ("PENDING — plus the
  operator labeling their self-hosted fleet"), and the learnings
  "Operator action item" — recorded here so the commit message and any
  release checklist carry it. The alternative reading (drop
  `self-hosted`, use bare `${{ matrix.os }}` GitHub-hosted legs) was
  correctly rejected as an operator-policy violation.
- **R5 (info, by design) — CU-25 skips all pre-header lines, including
  messages.** `parser.rs:1323-1330` continues over EVERY non-session
  line before the header, so a producer that wrote actual `message`
  lines before the session header loses them silently. This matches
  upstream `6848aa1` exactly (port fidelity is the acceptance) and the
  format detector (`is_oh_my_pi_jsonl`) still requires a real session
  header somewhere in the file. A future provenance counter
  ("N leading lines skipped") would make the data loss visible; not
  required this cycle.

## Compound-artifact review (roadmap/status updates, lessons, next-cycle candidates)

Checked with the same rigor as the code: the five dated status-change
annotations in ROADMAP.md (F11-1, F11-2, F11-3, CU-25, F11-7) each
match the live behavior verified above — including the honest
threshold-resolution note on F11-1 and the explicit PENDING marking of
CU-24's runner-level evidence; no completed item was moved to the
Completed record (correctly — nothing is committed yet; "Items close
only when committed"). The learnings doc's seven rules are accurate
against the evidence (rule 1's vacuous-guard story is corroborated by
the script's history this run; rule 3's invariant is exactly what
lib.rs:758-761 enforces); its "Context left for the next cycle"
correctly names F11-4 as the top hardening remainder and the
candidate-23 wave as the largest ready batch. Next-cycle candidates
(53/54) were filed in the roadmap phase and are out of scope for this
batch's acceptance. One commit-gate hygiene note: `.conductor/` is not
in `.gitignore` — it must stay out of the commit (run-scoped, contains
absolute local paths).

## Pointers

- Implementation record: `.conductor/implement/2026-09-14-cycle7-implementation-record.md`
- Batch selection: `.conductor/prioritize/2026-09-14-cycle7-batch.md`
- Learnings: `docs/stewardship/2026-09-14-cycle7-learnings.md`
- Assessment: `docs/reviews/2026-09-14-adversarial-repository-assessment-pass11.md`

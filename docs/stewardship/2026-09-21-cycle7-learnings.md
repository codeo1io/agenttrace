---
type: stewardship-record
id: cycle7-learnings
cycle: 7
batch: finish-the-truth-telling-debt-hold-parser-parity
date: 2026-09-21
base_commit: df3b621
record_kind: ce-handoff/v1
status: compounded-pre-review
---

# Cycle 7 learnings — compounding record

Folds the cycle-7 evidence (assessment, research, roadmap,
prioritization, stewardship, implementation, targeted and full test
outcomes — everything before review/shipping) into durable artifacts:
the ROADMAP cycle-7 Completed entry now carries the prevention rules
below, and the tail narrative carries the cycle-8 shortlist. This
record is the full-context companion. Review and shipping outcomes are
deliberately excluded; the cycle-8 assessment carries them forward.

## What the cycle proved

- The repo's lane discipline held under adversarial re-review: the
  assess pass over ~33k lines confirmed the cycle-1..6 hardening
  (saturating SQLite paths, atomic writes, escape repair) but found the
  parser remainder the earlier passes missed — evidence that the
  "claim vs. coverage" audit style (CHANGELOG already self-corrects one
  over-broad claim) keeps paying.
- The first upstream drift event since the fork (e005952 → 6848aa1,
  a34dea2) was detected, censused, and resolved inside one cycle:
  port the parser correctness fix, decline the TUI commit with reason.
- The stalled in-flight tree was diagnosed as the unfiled cycle-5
  review remediation and landed as CU-24 rather than duplicated or
  discarded — the dirty-state audit in the stewardship phase was the
  step that caught it.

## Prevention rules (durable; also in ROADMAP cycle-7 entry)

1. **A disposition is not done until it is filed.** The cycle-5
   review's decision to file F5-1/F5-2 as cycle-6 items was never
   executed; a complete, suite-green remediation sat unrecorded for 18
   days and was nearly re-implemented by cycle 7. Every review
   disposition lands as a roadmap entry in the same cycle that records
   it.
2. **Pin duplicated truth with contract tests.** The Go-flag shim's
   hand-maintained value-flag table drifted from clap — exactly one
   boolean (`--no-baseline-gate`) misclassified, silently swallowing
   the next flag. The fix ships a 49-flag arity contract test
   (22 booleans arity-0, 27 value flags arity-1) that fails by name on
   drift. Restates the cycle-6 F1-class lesson (wire-cased fixture
   keys from the producer's source) in a second domain: wherever a
   hand-maintained copy of a declarative source exists, a contract
   test pins it.
3. **Records copy exact identifiers.** The implementation record
   paraphrased three test names; the targeted-tests pass caught it only
   because name-exact filters matched zero tests. Test names in records
   come from `cargo test -- --list`, never from prose memory.
4. **Every upstream re-sync carries a drift census.** Ported: 6848aa1
   (Oh My Pi leading-line skip — parser correctness ports promptly).
   Declined with reason: a34dea2 (TUI; the fork TUI has diverged
   through cycles 4–5, blind ports risk regression for no measured
   gain; revisit at a deliberate re-sync). Silence is not a census.

## Reusable engineering context

- Dispatch boundaries are testable: Oh My Pi parsing is sniffer-gated
  (`is_oh_my_pi_jsonl`), so "header-less file errors in
  parse_oh_my_pi_jsonl" is unreachable through `parse_file`; the
  pinned behavior is the generic fallback, and tests should target the
  boundary that actually exists.
- The saturating-arithmetic audit pattern: grep every `+=`/`-=`/
  saturating op on the counter type, classify each as
  token-counter/loop-index/other, and record the classified list — the
  roadmap acceptance ("no bare += outside the audited list") then has a
  re-runnable proof (`grep -n '+=\|-=\|saturating_' parser.rs`).
- Engine validation blocks may derive `changed_testable_surfaces: []`
  even when the run changed code (uncommitted-tree derivation). The
  correct response is: change nothing executable during validation
  turns, copy the dispatch digest verbatim, and run the real suite as
  corroboration.

## Cycle-8 context handed forward

Shortlist in dependency order (also in the ROADMAP tail narrative):

1. Windows HOME/USERPROFILE resolver (headliner; L effort, unblocks
   every Windows-distributed install; upstream has no fix either).
2. Hermes `state.db` tool_calls_ok schema research — a data-shape
   question (fabricated equality with total), not a code fix; needs a
   live database census before any parser change.
3. Fork dependency-review fix — observed only at the PR/CI stage.
4. Research spikes: candidate 53 (ACP session stores) and candidate 54
   (VS Code agent debug logs; upstream radar #47).
5. Still queued: candidate 51, parse-size-cap, installer-checksum
   (deferred by the cycle-6 prioritization).

## Evidence index (this run, pre-review)

- docs/stewardship/2026-09-21-cycle7-prioritization.md — CU-24..CU-28
  selection, scoring, deferred-with-re-entry.
- docs/stewardship/2026-09-21-cycle7-stewardship-request.md — batch
  surfaces, must_remain_separate hints, dirty-state preservation.
- docs/stewardship/2026-09-21-cycle7-implementation-record.md — per-CU
  detail, drift census, saturating-arithmetic audit list.
- ROADMAP.md — cycle-7 Completed entry (with the four prevention
  rules), lanes slimmed of closed items, tail narrative cycle-8
  shortlist.
- Test outcomes: full workspace suite exit 0 (224 tests across
  binaries), fmt clean, clippy zero warnings, eight batch tests
  verified name-exact (see the targeted/full test phase results).

---
title: "Provider-felled attempts leave adoptable work: verify the durable trail before redoing"
date: 2026-10-15
category: process-issues
module: repository-governance
problem_type: process_gap
component: docs/stewardship
symptoms:
  - "An attempt dies to a provider failure (output-limit truncation, session reap, boot death) and its typed result artifact is absent"
  - "The reflex redo-from-scratch discards hours of already-landed in-tree or spool-side work"
  - "Adopted-without-verification inherits the dead attempt's unreviewed factual claims into durable artifacts"
root_cause: wrong_assumption
resolution_type: doc_change
severity: medium
tags:
  - governance
  - delegation
  - truthfulness
  - recovery
problem_summary: "A missing phase-result envelope is a transport artifact, not proof no work happened — but a present artifact is not proof the work is right either. Both directions require the same forensic pass before the work is adopted or discarded."
applies_when:
  - "Any phase redispatched after a provider failure, truncation, or session reap"
  - "Deciding adopt-vs-redo for a prior attempt's in-tree edits, spool scratch, or typed artifact"
  - "Reviewing work whose authoring turn never banked a result"
---

## Rule

When an attempt died before banking its result, run the forensic pass in this
order BEFORE redoing anything:

1. **Event log** (`events/<attempt>.jsonl`): classify the death (boot, provider
   reap mid-turn, output-limit truncation) and bracket the turn's wall-clock
   window from `delegate_turn_started` to the terminal event.
2. **Typed artifact** (`delegate/<attempt>.json`): absent means nothing was
   banked, nothing more.
3. **Durable trail**: scratch dirs, spool mirrors, `/tmp` mirrors — and the
   worktree itself. File mtimes inside the dead turn's window are candidates;
   reconcile the porcelain count against the *previous* phase's recorded census
   so every unexplained entry is attributed (this cycle: a late-mtime docs file
   turned out to be the *prior* phase's gate fix, not the dead attempt's).
4. **Adopt only after re-verifying every load-bearing claim** in the found work
   against surviving typed artifacts and read-only repo checks — a truncated
   turn wrote its text before dying, and nobody reviewed it. For a code delta
   this includes two hard gates a file-list census cannot substitute for:
   **COMPILE the tree** (a reaped author may never have compiled what it wrote)
   and **MAP every acceptance clause to landed behavior** (an implemented-looking
   arm may satisfy only some of the classes the acceptance names).

## Case (repository-maintenance 4f9d9268, cycle 1, 2026-10-14/15)

Four attempts died to provider failures before banking results: assess
`df71639f` (boot death, 1 event), roadmap `0679bdc1` (reaped 3 messages in),
prioritize `077b97e7` (death 23 s in), and compound `3c89bdd7` (output-limit
truncation ~3 min in). The first three held nothing durable — their forensics
cost seconds each. The fourth had already written the cycle's ROADMAP compound
band in-tree (mtime inside the dead turn's 23:44:26–23:47:25Z window; +7/−0
pure insertion; porcelain reconciled to exactly +ROADMAP.md over the recorded
prior state). Adoption fact-checks caught **four factual errors** the truncated
turn had written into the wall (wrong tag numeral v0.4.0 vs the real failing
tag v0.0.4; advisory range truncated; upstream PR numbers transcribed from a
since-reaped scratch; a docs-sync attributed to the wrong phase) plus one
unverifiable count. The band landed corrected, with its adoption provenance
recorded in-band and in
`docs/stewardship/2026-10-15-cycle1-compound-record-run4f2af62e5b7e.md`.

The asymmetric costs drive the rule: redoing adoptable work wastes the dead
turn's entire output; adopting unverified work launders its errors into the
repository's most durable artifact — the wall.

## Case (repository-maintenance 71aba0acc95c40c78adfd424f5b359f2, cycle 6, 2026-10-18/19)

Three attempts died to provider failures: prioritize 3dee432b (15 messages,
lifecycle-only — nothing durable, redone from scratch), implement 448da7a8
(~19 messages — its work survived as the uncommitted 13-path tree delta), and
compound 5f85cfd9 (16 messages — envelope absent, no scratch, tree untouched:
zero durable, redone). The implement adoption is the instructive one: the
delta matched the batch file set exactly and read complete, but the
hunk-by-hunk audit plus a compile found it had NEVER COMPILED (a struct-literal
error sat in the cost_non_finite_contract tests) and its telemetry disclosure
covered only 3 of the 5 skip classes the batch acceptance named — invisible to
a presence check, fatal to a batch titled "no invisible skips". Completion
(567e6c1c) fixed both and added the missing red pin; the compound band records
the adoption provenance in-roadmap
(docs/stewardship/2026-10-19-cycle6-compound-record-rune40a66d3.md). Companion
prose rule from the same cycle: roadmap/rider text must never spell a literal
bracket-parenthesis link form — the docs gate's own inline-link regex matches it
as a broken relative target; spell link shapes descriptively and amend in-band
with a dated note when a defuse lands.

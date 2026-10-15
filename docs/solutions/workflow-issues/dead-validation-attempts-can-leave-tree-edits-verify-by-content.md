# Prevention rule: dead validation-phase attempts can leave tree edits — verify by content, not porcelain

- **Class:** workflow / conductor delegate attempt recovery (infra-death forensics — the validation-phase extension of the adopt-then-complete family)
- **Observed:** run `a64735b75a464192b47217b38a93d30f` (repository-maintenance `5f8cbeaa…` cycle 3), full_tests attempt `cb7d799d` — held 2759 s past its absolute serve-hold ceiling, terminal envelope a provider-failure ("held … without a terminal observation"), zero durable logs, `delegate/cb7d799d….json` absent. Same run, compound attempt `9b7ec3dc` — provider `credit_balance` error 26 s after turn start, zero durable trail.
- **Cost:** two symmetric failure modes. Treating the dead attempt's window as a no-op (the porcelain shorthand) ships an edit whose authorship no phase record explains — the commit gate lands "phantom" content it cannot attribute. Conversely, treating any content drift after a dead attempt as contamination and reverting it discards work a successor phase may already have VALIDATED as part of the tree.

## What happened

The dead full_tests attempt rewrote `CHANGELOG.md` mid-turn (mtime 18:43:35, inside its 18:33:47–19:19:47Z window): the Unreleased section was restructured from Added-first to Fixed-first with the batch's two new bullets on top. Every recorded phase fact stayed TRUE at the shorthand level — `git status --porcelain` was still the same 7 rows — while the tree's actual content digest moved (`git diff | md5sum` d14d3ea0… → 59053f94…). The successor full_tests attempt (`e3d1c306`) then ran the entire ci battery against exactly this tree, all green, porcelain identical pre/post. The restructure therefore rides the landing legitimately — but only the per-file mtime census and the diff-digest comparison reveal WHO wrote it and WHEN; no envelope says so.

This extends the two known families: `reaped-implement-attempts-leave-the-fix-in-the-tree-adopt-then-complete.md` (the tree drift IS the artifact — but at the implement phase, where drift is expected) and `provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` (heartbeat-only death, byte-clean tree ⇒ redo). The third shape: a **validation-phase** attempt that is forbidden from changing surfaces still can, and its death removes the only narration of the change.

## Prevention rules (content-verdict protocol)

1. **Never accept "porcelain identical" as the identity of a tree.** Porcelain names which PATHS changed, not what the changes contain. Across any dead-attempt boundary, compare `git diff | md5sum` (or per-file hashes) against the last RECORDED digest from a live phase, not against a row count.
2. **Census per-file mtimes against the attempt windows.** For every dead attempt, take its event-log first/last timestamps and `find . -newermt <start> ! -newermt <end>` (excluding `.git/`). A file whose mtime falls inside a dead attempt's window is that attempt's write until proven otherwise — attribute it explicitly in the successor's record, exactly as this cycle's compound banner does for the CHANGELOG restructure.
3. **Adjudicate the edit on the successor's validation, then disclose.** If a later phase's recorded battery ran against the content-carrying tree and passed (here: `e3d1c306`'s BATTERY, porcelain-after matching the live tree), the edit is validated-by-successor and rides the landing — the commit gate just needs the authorship disclosure so review isn't auditing a ghost. If no successor validated it, quarantine it like any un-narrated drift.
4. **Scope-of-phase is not a physical constraint.** "NO TEST EXECUTION / validation-only / review-only" bounds what a phase may DECLARE, not what a dying turn's partial work may have written before the transport failed. The forensics checklist (typed artifact? scratch? event-log shape? tree writes in-window?) runs identically for every phase family — implement is just the one where drift is expected rather than anomalous.

## Related

- `reaped-implement-attempts-leave-the-fix-in-the-tree-adopt-then-complete.md` — same directory (implement-phase family).
- `provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` — same directory (base forensics convention).
- Cycle record: `docs/stewardship/2026-10-14-cycle3-compound-record-runa64735b75a46.md` (lesson L2, with the digest/mtime evidence).

# Prevention rule: adopting a provider-dead attempt's uncommitted code — re-verify hunks, re-prove reds, re-derive every narrative claim

- **Class:** workflow / conductor delegate attempt recovery (in-tree salvage of infra-dead work)
- **Observed:** run `33b7b7b57dc4` (repository-maintenance `2ba7d07a` cycle 1, 2026-10-09): implement attempt `e278e52c` died of a provider failure at 03:51 UTC **after** writing 5 M + 1 ?? (232-line diff) into the run worktree, leaving no typed result. Sibling cycles: run `2023f222` codified the same shape for ROADMAP application only (the adoption nuance in its cycle-1 compound record); run `2a5cdb9e` cycle 3 codified the prose side (a reaped attempt's appendix/CHANGELOG narrative can confabulate cost numbers, phantom line citations, and false reachability doctrine while its code and pins are perfectly valid).
- **Cost:** the two failure modes are asymmetric. Discarding the work burns a dead attempt's valid effort and re-implements under time pressure; adopting it wholesale ships code that was never reviewed AND narrative claims that were never true. The second is worse, because the code passing tests lends the prose unearned credibility.

## The rule

A dead attempt's **code**, its **reds**, and its **prose** are three separate claims with three separate verification routes. Adopt none of them together.

1. **Hunks against the batch definition, not against plausibility.** Diff every hunk of the dead attempt's tree delta against the phase's selected batch (row acceptance criteria + designation banner). A hunk that serves no row in the batch is foreign work — foreign-attempt edits can linger in a per-RUN shared worktree (the standing hazard: `stat` mtimes + line-read before adopting). Anything unexplained goes back to the event log.
2. **Re-prove the reds live.** The dead attempt's red logs may be stale, staged, or fabricated. Restore the pre-state per hunk family (md5-pinned copies, never `git checkout` on a tree you want to keep), re-run the failing assertions yourself, and only then re-apply. A red that will not re-derive is not a red.
3. **Re-derive every NARRATIVE claim the adopted artifacts carry** — this is the step that feels paranoid and is not. Numbers cited as "the red" must exist in a surviving log; every mechanism attribution ("which arm fired") must match the provenance string / `model_used` the tree's own binary emits; every line citation must be read at the tree you are about to ship; every code-topology story ("legacy arm vs modern arm") must match the actual call graph. The `2a5cdb9e` precedent: adopted prose shipped three confabulated cost figures, a phantom `:1104` citation, and a false doctrine into the CHANGELOG and roadmap while the code was correct — caught only by re-deriving the claims with the binary itself.
4. **Re-base the safety chain, not just the feature.** Schema consts and their same-unit literal set (ladder comments, contract-test oracles, planted fixtures, docs sentences the CI gates grep) must be census-checked at adoption time, because the dead attempt could not run the census you now owe: landed origin, every sibling worktree's uncommitted const, and spool text claims. Holding a unique number below the observed frontier is collision-free; minting is `max(observed)+1`.
5. **Declare the adoption with its legs.** The successor's phase result names what was adopted, from which dead attempt, with which verification legs (hunk table, red re-proofs, narrative re-derivations, census). "Adopted" without legs is indistinguishable from "trusted".

## Why this shape keeps recurring

Provider deaths cluster in the same wall-clock windows as long implementation phases, so the dead attempt is disproportionately often an *implement* attempt sitting on a large, mostly-finished diff. The diff is almost always salvageable; the discipline is what makes salvaging safe.

## Related

- Reaped attempts with NO in-tree work (census match → redo from scratch): `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md`.
- The ROADMAP-application variant of this rule and the porcelain-census discriminator: `docs/stewardship/2026-10-08-cycle1-compound-record-run2023f222.md` (adoption nuance).
- The narrative-claims precedent (adopted prose confabulating while code held): run `2a5cdb9e` cycle 3 — its compound record and solutions doc were still **uncommitted in that run's sibling worktree** at this writing (title `2026-10-09-cycle3-compound-record-run2a5cdb9ef522.md`), so cite by run+cycle, not by an in-tree path, until it lands.
- This cycle's instance (e278e52c → adopted by implement 9d3480ed): `docs/stewardship/2026-10-10-cycle1-compound-record-run33b7b7b57dc4.md`.

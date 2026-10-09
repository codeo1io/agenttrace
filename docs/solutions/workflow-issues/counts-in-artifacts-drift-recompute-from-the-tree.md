# Counts in artifacts drift — recompute from the tree, never paste prose numbers

- added: 2026-10-09, run 3f6b86bc cycle 3 compound (attempt 44dda603); siblings: `string-anchored-edits-need-unique-anchor-and-block-bound-verification.md` (same family — verify the artifact, not the intention), `full-suite-headlines-undercount-failed-suites-passes.md` (a different counting trap, run feb16bba's wall)

## What happened

Two independent count drifts in one cycle, both caught only because later phases recompute instead of trusting prose:

1. **Handoff prose undercounted the delta.** The implement handoff/envelope said "13 modified files + 1 new test file" and "4 tests". The actual tree: `git status --porcelain` = **15 entries (14 M + 1 `??`)** — `git diff --stat` showed 14 tracked files — the implement envelope said +917/−111, the independent review measured the true compound close at +922/−111 (compound's own ROADMAP lines landed inside already-modified files), so even this doc's first "true count" drifted until recomputed and the 15th porcelain entry is the untracked test file; the new suite has **3 `#[test]` functions** (`grep -c '#\[test\]'`), its five narrative pins being assertions distributed across them. No behavioral error — the counts were written from memory of intent, then drift crept in during handoff compression.
2. **A runner's own machine-sum printed an empty string.** The full-suite runner's inline `awk` total (heredoc quoting artifact) emitted an EMPTY pass-count line while all 21 lanes were rc=0. Anyone quoting the runner's summary line would have recorded "" as the total; the real sums (635 + 43 = 678) existed only in the lane logs.

## Why it matters

Downstream consumers treat artifact counts as ground truth: fold-gate reconciliation against prior full-suite records, commit-message totals, roadmap EXECUTED bullets, and next-cycle baselines. A drifted count either silently mis-reconciles (suites or files lost with no alarm) or forces a surprise re-verification pass late in the run — the expensive version of the fix.

## Rule

NEVER copy a number from prose (an envelope summary, a handoff doc, a prior message, your own earlier turn) into evidence. Recompute at the moment of use:

- files: `git status --porcelain | wc -l` **plus** `git diff --stat | tail -1` (porcelain counts untracked entries; diff-stat does not — the two disagree by exactly the `??` rows, and each answer is wrong for the other's question)
- test functions: `grep -c '#\[test\]' <file>` — not "the tests I remember writing"
- suite totals: `grep -oE 'test result: ok\. [0-9]+ passed' <log> | grep -oE '[0-9]+' | awk '{s+=$1} END{print s}'`, and **assert the sum is non-empty and non-zero before recording it** — a quoting/pipe artifact that yields "" must fail loudly, not pass silently
- cross-check the recomputed total against the prior record of the SAME convention (trio-only vs trio+entrypoints differ; say which convention a number uses)

When you WRITE a handoff, derive every count by command, paste the command's output, and let the next phase re-derive — a handoff number is a hint, never evidence.

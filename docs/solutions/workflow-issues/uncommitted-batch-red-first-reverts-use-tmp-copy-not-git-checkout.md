# Uncommitted-batch red-first reverts: /tmp copy, never `git checkout` (PR-1)

**Rule class:** workflow issue · **First proven:** run 2a5cdb9e implement
attempt 734d716c, 2026-10-09 (agenttrace) · **Repo:** agenttrace

## The failure mode

Red-first proof on an agenttrace maintenance batch needs the unfixed
source temporarily back in place while the corrected test runs, then the
fix restored. The standard fleet protocol for committed work is
`git show HEAD:<path> > <path>` — restore the pre-fix file from HEAD, run
the single test, write the fix back. But maintenance batches run
**entirely uncommitted until the commit gate**: every hunk of the fix
exists only in the worktree. On such a tree HEAD is the *pre-batch*
revision, so `git show HEAD:<path> > <path>` and
`git checkout -- <path>` don't expose the bug — they **destroy the batch
hunk** and cannot be undone (it was never in the object store). The
single-test run then "fails" for the wrong reason or, worse, the delegate
restores from a stale copy and ships a silently truncated batch.

## The rule

1. **Never reach for HEAD while the batch is uncommitted.** Any
   `git checkout`/`git show HEAD:` on a dirty batch path is a batch
   destruction hazard, not a probe.
2. **Back up the fixed file to /tmp first:**
   `cp crates/x/src/y.rs /tmp/y.rs.good`
3. **Hand-apply the unfixed variant** (python replace or edit tool) —
   revert exactly the hunk under test, nothing else.
4. **Run the single test**, expecting the exact failure (assertion left /
   right values quoted into the evidence).
5. **Restore with `cp /tmp/y.rs.good crates/x/src/y.rs`** and **verify
   md5 equality** against the pre-probe backup — the restore is only
   proven when the bytes match, not when the command exits 0.
6. **Re-run green** on the same test before declaring the leg done.

Optional belt-and-braces: `git stash push -- <path>` also works on
uncommitted trees, but the /tmp copy survives even a botched stash-pop
conflict and makes the md5 check trivial. Prefer the copy.

First proven while proving the rm-871 risk-ladder arms red (both arms:
temp in-place ladder revert → `assertion left: "good" right: "caution"`
→ /tmp restore → md5-identical → green). Related:
`string-anchored-edits-need-unique-anchor-and-block-bound-verification.md`
(the hand-apply step's safety net) and the fleet red-first discipline
that motivates the probe.

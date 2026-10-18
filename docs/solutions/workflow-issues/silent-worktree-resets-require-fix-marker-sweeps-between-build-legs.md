# Silent worktree resets require fix-marker sweeps between build legs

**Rule:** while implementing an uncommitted batch, never trust a build or test result from
earlier in the same turn — sweep the worktree for the batch's FIX MARKERS (the distinctive
new identifiers, comments, or test names the batch introduced) immediately before every
build/test leg. A red test run after a green compile on intent-unchanged code is, first and
foremost, evidence the TREE moved, not that the code is wrong.

**Incident (2026-10-10, agenttrace run 6603da8916bd, repository-maintenance cycle 1):**
the 'truthful tokens, bounded windows' batch (rm-619 opencode reasoning-only tokens +
rm-941 future-sample bounding, implement attempt 62fb8454 @ base 428f0e9) had its tracked
file content silently reverted to HEAD **twice** mid-phase — while HEAD stayed `428f0e9`
and `git status --porcelain` read clean (content matching HEAD IS clean, so porcelain
cannot see this class of reset). The only symptom: a compile that had been green was
followed by a RED twin-fixture test run. Diagnosis by fix-marker grep showed the markers
gone; the failing set was exactly the 3 parser-arm tests of `tests/window_truth_batch.rs`
— i.e., the red arm doubled as the revert detector.

**Why porcelain missed it:** the reset restored content to HEAD byte-for-byte. A dirty-tree
watch on `git status` detects edits, additions, and losses — not a return to pristine.

**Fix + proof:** re-applied the batch, then ran the marker sweep before every subsequent
build/test leg (the final legs were clean); the downstream gates pinned the tree
independently — targeted_tests a734c2e4 and full_tests a38d80f2 both verified
`git diff | sha256sum` == the implement pin `983fcb0f…`, the new test file's sha256 pin
`8be4e8bf…`, and pre==post per-file md5s across all four batch surfaces at every leg —
and neither gate observed a further reset.

**Prevention:**

1. At implement delivery, pin the batch: `git diff | sha256sum`, per-file md5s of every
   touched surface, and a list of 2–3 grep-able fix markers unique to the batch.
2. Before EVERY build or test leg inside the phase, grep the markers (cheap) — and treat
   a marker miss as "tree moved", re-applying before interpreting any red.
3. Gate legs: re-verify the diff-sha256 + per-file md5 table pre- and post-leg (the
   silent-reset watch), so a reset between phases cannot launder itself through a green
   gate.
4. Never diagnose a surprise red from a green codebase by editing the code first — check
   the tree against the pins first.

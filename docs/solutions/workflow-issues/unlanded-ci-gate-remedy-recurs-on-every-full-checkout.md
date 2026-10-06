# Prevention rule: unlanded CI-gate remedies recur on every full checkout

- **Class:** workflow / CI-gate hygiene across campaigns and lineages
- **Observed:** 2026-10-06, run `b1ff12f8` cycle 2 (repository-maintenance `4cee1a0e`), full_tests attempt `52b44afc` at base `ea5c41e`; first observed in lineage `96f0f58` (full_tests of an earlier run) whose identical fix never landed
- **Cost:** the fork's authoritative full suite (CI `full` job, step `check-plugin-version.sh`) is red AT BASE on any full checkout — every campaign that runs the full suite locally or opens CI rediscovers the failure from scratch, re-diagnoses it, and re-fixes it in its own uncommitted delta; the fixes then compete at integration instead of compounding.

## What happened

`check-plugin-version.sh`'s per-tag arm (rm-303) requires every version-shaped tag merged into HEAD to have either a `## vX.Y.Z` CHANGELOG heading or an explicit `<!-- no-changelog-section: vX.Y.Z: reason -->` marker. The fork origin carries tags `v0.7.2..v0.8.0` (all seven are merge-base ancestors of the fork's master), but upstream's CHANGELOG never had sections for them (heading jump `v0.8.1` → `v0.7.1`), and no marker block ever landed in this lineage.

An earlier lineage already solved this: full_tests attempt `96f0f58` hit the identical failure and added the designed 7-marker block — but that delta never landed. So run `b1ff12f8`'s full_tests hit the same wall 3 days later, and had to re-derive the same diagnosis (tag ancestry via `ls-remote` + `merge-base`, heading-gap via grep, marker absence via `git log -S`) before re-applying the same 7 lines. Meanwhile every sibling worktree at any base remains red on this gate alone until the block lands.

## Prevention rules

1. **Treat "gate red at base" as a distinct finding class.** When a full-suite failure is provably independent of the cycle's delta (the delta doesn't touch the failing surface; the failure reproduces from tag/heading ancestry at the base commit), record it as a pre-existing defect with its own evidence trail — do not let it masquerade as a batch regression, and do not "fix" it by deleting the gate.
2. **Every gate remedy applied in an uncommitted worktree delta must be declared a commit-gate rider.** The compound/stewardship record must name it explicitly ("this block rides the commit until merged"), so the commit phase cannot drop it and the next campaign knows to check for it.
3. **Before diagnosing a CI-gate failure from scratch, sweep for an unlanded remedy.** Check the delegate spool for prior attempts on the same repo family mentioning the gate (this case: `96f0f58`), and check `git log -S` for the remedy string in the current lineage. A prior diagnosis cuts hours and converges the fix byte-for-byte.
4. **Gate fixes for tag-population drift belong in-tree, not in tag surgery.** The remedy is the marker (or a CHANGELOG section), never deleting or rewriting fork tags — the gate is lineage-based by design (rm-303).

## Verification

- `scripts/ci/check-plugin-version.sh` exits 0 on a tree carrying the 7-marker block before `## v0.9.0` in CHANGELOG.md (run b1ff12f8 full_tests rerun: "plugin.json v0.9.0 matches CHANGELOG v0.9.0 and advances past merged tag v0.9.0").
- The block's absence is detectable: `grep -c 'no-changelog-section' CHANGELOG.md` → 0 at base `ea5c41e`; `git tag --merged HEAD | grep -E '^v0\.[78]'` lists the seven offending tags.

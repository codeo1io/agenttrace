# Tag-complete environments flip `check-plugin-version.sh` red

## The failure mode

`scripts/ci/check-plugin-version.sh` carries a per-tag arm (landed 2026-10-03, run `364aa3be`): every `^vX.Y.Z` tag merged into HEAD must have a `## vX.Y.Z` CHANGELOG heading or an explicit `<!-- no-changelog-section: vX.Y.Z: reason -->` marker.

The fork (`codeo1io/agenttrace`, remote `origin`) carries **inherited upstream tags** `v0.7.2`–`v0.7.7` and `v0.8.0`, merged into every branch descending from the fork point, with **neither** a heading nor a marker (upstream's changelog discipline started later; only the latest triple `v0.9.0` was reconciled when the three-anchor gate landed).

Consequence: in any environment whose tag namespace is complete —

```
$ git fetch --tags
$ bash scripts/ci/check-plugin-version.sh
check-plugin-version: FAIL - merged tag v0.7.2 has no CHANGELOG section (expected '## v0.7.2' or '<!-- no-changelog-section: v0.7.2: reason -->')
```

— including **the fork's own GitHub CI**, because `ci.yml` runs `git fetch --tags` before the gate.

## Why it ever looked green

Historical local checkouts of the canonical repo held a **sparse** tag set (`v0.0.4`, `v0.3.x` loose from the 2026-09-02 clone, plus `v0.10.0`/`v0.10.1`), under which both tag arms no-op. Runs recorded the gate rc0 *vacuously* (e.g. envelope `fe2a5258`, run `364aa3be`'s all-12-gates rc0). The arm's authors could not see the tags it now demands sections for.

## Prevention rules

1. **A `git fetch --tags` in any worktree mutates the shared gitdir** (`/work/projects/agenttrace/.git` — packed-refs rewrite) and permanently flips this lane red for **every** subsequent local run until reconciliation. Do not fetch tags casually in validation gates; if you do (CI-faithfulness requires it), disclose the namespace change in the phase result.
2. **Never "fix" CHANGELOG inside a validation gate** — backfilling markers is a content decision (authored reasons about inherited upstream releases) that belongs to a reviewed change, owned by the roadmap rm-018/rm-163 lineage.
3. **Do not delete the tags to make the gate green** — the tag-complete state is the CI-faithful one; deleting hides the gap the fork's own CI will hit.
4. When reconciling: add the 7 markers with authored reasons **or** rescope the arm to tags minted on the fork; sequence with rm-018's mirror wave so the fork's first post-change commit is green.

## Provenance

Pinned 2026-10-06 by run `6aaf51aaa919` cycle-3 full_tests (attempt `4d0db5aa`): rc1 observed, proven delta-independent at base `ea5c41e` (identical 17 CHANGELOG v-headings; cycle delta added one bullet), reproduced against `git ls-remote --tags origin`. Roadmap folds: rm-163 dated append, rm-018 notes append. Project memory #16697.

# Cycle 2 ship record — run 3ec6cec08fb9 (repository-maintenance 71aba0acc cycle 2)

Batch: **Agent-lane usage truthfulness** — rm-720 (LEAD, antigravity usage/cache-read
extraction + standalone-model pricing) + rm-721 (copilot agent-host per-request-class
exactly-once audit). One atomic change-unit (stewardship 96b18147 ruling). Committed at
the commit gate after independent review APPROVED and release-integrity final validation.

## Lineage

| Gate | Attempt | Outcome |
| --- | --- | --- |
| roadmap | 64b69183 (prior 140e1d61 provider-dead, zero durable work) | patch-only, rm-720/rm-721 minted spool-side |
| prioritize | 0df1ffe8 (prior c132e08f reaped ~104s) | batch selected, rider closed |
| stewardship | 96b18147 | one change-unit in THIS worktree, base 1c5edd1 |
| implement | b127341f | 3 files, red-first proven, binary-differential PoC |
| targeted_tests | e20ddf6a | core 375/0 + copilot 5/0 + fmt/clippy rc0 |
| full_tests | 605c14d8 | 21/21 ci.yml lanes rc0, workspace 536/0 |
| compound | 65202416 | banner + 2 flips implemented, spool-side chain |
| independent_review | 3e3a2198 | NEEDS_CHANGES (8 findings) |
| review fix | 653b57af (reaped 8a3231c6 durable work adopted) + retest 1747a7914 | all 8 fixed; FULL retest 21/21 lanes, 537/0 |
| independent_review | b78f8808 (prior 25b3c79f provider-dead, evidence adopted) | APPROVED |
| final_validation | 132bf66b | release integrity PASS, digest == dispatch |
| commit | 5a1ce1fa | this commit |

Validation digest (engine re-derived PRE==POST==dispatch at every gate):
`validation:v1:42740398213dc95635d30f0a4789c61f608a162429ff6c31dfd38d8ff3ffdc8f`

## Review findings fixed (8)

- **F1 (high)** `SESSION_CACHE_SCHEMA_VERSION` 32→33 + full same-unit sweep (TUI planted
  warm-cache fixture, governance-guide sentence, discovery-contract ladder comments);
  warm v32 caches had kept serving pre-batch estimates/$0 credits for unchanged files.
  Pinned by `stale_schema_32_cache_cannot_mask_the_agent_lane_rollup` (red-first proven
  twice: stashed parser.rs → FAILED; stashed const-32 tree → stale entry IS served).
- **F2 (medium)** roadmap quota-overclaim corrected spool-side to "considered and
  DECLINED by recorded decision" (ROADMAP.reviewfix-r2.patch, chain-verified
  byte-identical; landed in this commit's ROADMAP).
- **F3 (medium)** agent-host fixture widened to the full observed class vocabulary
  (hook userPromptSubmitted, permission pair, subagent.deselected, two-model rollup);
  rewritten pin asserts the five named un-counted classes.
- **F4 (high)** per-model `totalNanoAiu` meters SUM at emit (the implement-phase global
  max silently dropped the second model's bill).
- **F5 (medium)** implement PoC re-captured with isolated cache dirs (the original RED/GREEN
  legs were byte-identical — a cache-warmed capture); fixed binary on a v32-warmed cache
  now shows cache_hits=0 ≡ cold run.
- **F6 (low)** both oracle diffs (codeburn #1655, #1651) re-fetched and pinned durably in
  delegate scratch (the /tmp pin had been swept).
- **F7 (low)** dead `modelUsage` probe dropped from `ANTIGRAVITY_USAGE_KEYS`.
- **F8 (low)** `AntigravityUsageFold` documented as per-generation event counts, not
  cumulative meters.

## Commit-gate landing notes

- Delta: 6 validated files (parser.rs, session_cache.rs,
  copilot_checkpoint_reconciliation.rs, tui tests.rs, governance-reports.md, new fixture)
  + ROADMAP chain (roadmap-64b69183 → ROADMAP.compound → ROADMAP.reviewfix-r2, applied
  check-then-apply each hop, byte-identical to the verified postimage md5 cc07d535…) +
  commit-gate done-flips for rm-720/rm-721 (rm-012 convention) + CHANGELOG Unreleased/Fixed
  bullets (both rows + the schema-bump behavior bullet) + this record.
- The untracked fixture staged explicitly (`git commit -am` would drop it).
- Integration seams: origin/master advanced past base 1c5edd1 with parser.rs bands
  @@2859/@@3139/@@3227/@@5767 — reconcile by TITLE; this batch's antigravity/copilot
  hunks are disjoint per the prioritize diff census. The schema same-unit set landed
  together (seam e). The Antigravity 2.16 quota restructure rides rm-306's refresh band.
- Next-cycle leads: rm-709 fold into rm-526's landed note after re-verification on the
  rebased base; rm-716..719 sibling band; npm probes owed a re-probe.

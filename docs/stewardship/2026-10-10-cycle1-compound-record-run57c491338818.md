# Cycle 1 compound record — run 57c491338818 (repository-maintenance 3f178c838e6f4d9491601c3e95616657)

- **Recorded:** 2026-10-10, compound attempt 58091d4d80b64ea39a6fb93f344f9ac4, worktree `run-57c491338818-57c49133` @ HEAD `15a8da8dbed4ed1297994a8c86a33e2b24d4e69a`.
- **Contract:** pre-review compounding only — consumes recorded cycle evidence; NO test/validation command re-run. Review and shipping outcomes land after this phase and are carried forward by the NEXT cycle's assessment.
- **Batch:** trust-boundary hardening — id-less fold-key namespace (F1 / rm-300 goal d), `--doctor` positional scope (F2 / rm-439 goal b), task-type render ordering (F3 / rm-919). 7 tracked files, +389/−9, uncommitted per policy.

## Phase ledger

| phase | attempt | outcome (consumed, not re-run) |
|---|---|---|
| assess | 761f340f | adversarial re-assessment at 15a8da8, gates green (check/fmt/clippy -D warnings/deny/docs/test-workspace 814 across 46 suites), F1+F2+F3 found with live PoCs |
| research | 8c308609 | R1–R6 dossier, all sources live (CC changelog, LiteLLM raw, npm, GitHub, crates index, models.dev) + first local host census; litellm-now.json saved |
| roadmap | ed44938c | family-scoped census; minted rm-919 via spool patch (+18/-0); R1–R5 cessions recorded; pins confabulated — see incident 1 |
| prioritize | 3005dea4 | selected F1+F2+F3; ranked next-cycle leads table (survives in PRIORITIZE.md scratch) |
| stewardship | 091d5426 | filed stewardship_request, 4 change-units; CAUGHT AND PROVED the pin confabulation (incident 1); corrected ROADMAP-NOTES.md 2026-10-09 16:20 |
| implement | 8c96754e | red-first throughout; 7 files +389/−9; PoC flips: hostile.jsonl tool_results 1→2 / events 3→4; --doctor positional 25s-hang → rc0 <1s; --demo task-type cost-desc |
| targeted_tests | 7ad26f73 | byte-stable turn: fmt/clippy -D warnings rc0, hostile 15/0 + attribution 14/0 + doctor_demo 2/0 + usage_occurrence 5/0, docs gate rc0; dispatch digest copied verbatim (tree unchanged) |
| full_tests | cc910101 | redone from scratch (see incident 2); 24/24 lanes rc0 — ci.yml push lanes verbatim (empty full_command → full-suite-by-convention), test-trio 818/0 over 46 suites, reconciled 814 baseline + exactly +4 census; digest re-derived MATCH pre and post |

## Roadmap deltas delivered this phase (spool chain; ROADMAP.md in-tree untouched)

Chain order for the commit gate: **ed44938c `ROADMAP.patch` FIRST, then `58091d4d` `ROADMAP.compound.patch`** (incremental on its postimage; `ROADMAP-CUMULATIVE.patch` is the pristine→final equivalent, verified both ways plus true-double-apply rejection and out-of-order rejection on isolated /tmp copies).

- rm-919 `candidate → implemented` + dated EXECUTED bullet (done-flip reserved to the gate, rm-012 convention). Def-row census 176 candidate / 106 implemented / 81 done → **175 / 107 / 81**.
- rm-300 goal (d) and rm-439 goal (b) EXECUTED arm bullets; both rows stay candidate (goals (b)/(c) and (a)/(c) respectively remain open).
- Research riders written (the roadmap mint recorded cessions; bullets landed here): R1→rm-164, R2→rm-176, R3→rm-406 + rm-037 + rm-487, R4→rm-530, R5→rm-493. R6 negatives pinned in the cycle banner only (no rows).
- New cycle banner at the top of the banner wall (above the e9dbcbc2428d banner).

## Incident 1 — confabulated integrity pins at the roadmap phase (numeric provenance)

The roadmap phase's record carried three hand-typed pins (patch sha256 "449b9435…", postimage md5 "30790142…", pristine md5 "7460221a…") that matched none of the artifacts, while the underlying patch chain was fully valid. The stewardship phase proved no tampering (mtimes unchanged since creation, HEAD/porcelain stable, roundtrip re-proof clean) and corrected ROADMAP-NOTES.md with a dated correction naming void + authoritative values. Authoritative values re-emitted by command this session (see `NOTES.md` in the compound scratch): pristine sha256 `6bd1165511632445951993e35bcbae9c1b184f5795b3b50f6efa671bb78c394b` / md5 `36810af78afc04c766170f858610745b` (3875 lines — also note the recorded "3876" line count was off by one). Prevention doc minted: `docs/solutions/workflow-issues/integrity-pins-must-be-command-emitted-never-hand-typed.md`.

## Incident 2 — dead full_tests attempt

Attempt 3fbdfeca died ~40s in (infrastructure reap, family: delivery): no typed artifact, no scratch dir; event log read in full — `phase_result_repair_started` ×2, nothing delivered. Nothing durable to adopt; full_tests redone from scratch (cc910101) and declared such. Consistent with `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md`.

## Fleet lessons (also in the cycle banner)

1. **Integrity pins are command-emitted, never typed** — incident 1 above; prevention doc minted.
2. **The full-suite lane set is re-derived from the worktree's OWN ci.yml at its base** — this base (15a8da8) carries the rm-654 two-form Clippy step; sibling base 9c3c599 carries one form. Copying the 33b7b7b5 sibling runner would have silently dropped the `clippy-workspace` lane. Extends `docs/solutions/workflow-issues/empty-full-command-mirrors-ciyml-lane-for-lane.md` (tracked doc — extension intentionally left to the next cycle, with review outcomes in hand).
3. **`-q` cargo test logs are headerless** — per-suite count attribution cross-references a non-quiet run of the same suite set with positional equality asserted (used to reconcile 814 → 818 = +exactly the 4 census tests).

## Next-cycle leads (ranked; review/shipping outcomes to be folded by the next assessment)

| rank | lead | why now |
|---|---|---|
| 1 | rm-164 tier-aware pricing rider (P80.0; with rm-176 disclosure arm) | R1: claude-haiku-5-5 ships BY DEFAULT since CC 2.1.293; live LiteLLM encodes above-100K tiers (5e-7 / 2.5e-6) and above-1hr cache-write tiers the snapshot schema cannot represent; bundle is flat 1e-7/5e-7 → >100K prompts price 5× under with zero disclosure. Two agreeing rate sources; evidence pack preserved (research scratch RESEARCH.md + litellm-now.json). |
| 2 | rm-530 date-split alias pricing (P72.0) | R4 refreshed the premise: chatgpt/-prefixed keys now exist live but context-only; fallback story unchanged. Prior cycle already named it READY LEAD. |
| 3 | rm-176 snapshot automation-posture decision | R2: upstream (ccusage) refreshes daily-automated; models.dev counts move in BOTH directions → never-drop union policy; deliberate fail-closed stays defensible but should be a recorded decision, not an accident. |
| 4 | rm-588-family second-tier-fields decision | schema question raised by R1's above_1hr cache-creation tier fields (representable-or-disclose). |
| 5 | union-at-integration sweep | unlanded sibling numeral bands 916/917/918 from the mint census (agenttrace family); rm-203 carries a sibling fold of the same ccusage #1831 datum (535cbb3f). |
| watch | rm-493 OTel export | stays gated on a first tag (tags still [] as of 2026-10-09T12:22Z push); e07f4eba severity-DEBUG recommendation parked on the row. |

## Commit-gate seams

- Stage beside the 7-file delta: this record (untracked) + the prevention doc (untracked).
- Apply the spool chain IN ORDER: ed44938c `ROADMAP.patch` → `58091d4d` `ROADMAP.compound.patch` (or the cumulative in one step); then the gate owns rm-919's done-flip.
- The batch carries **no CHANGELOG entry** (implement minted none) — the gate writes the Unreleased Fixed bullets with review outcomes in hand.

## Addendum — review-fix 670753a6 (2026-10-10, after independent_review 543ac419)

Review verdict NEEDS_CHANGES (1 high, 1 medium, 3 low) — all five findings fixed this turn:

- **F1 (high)** `SESSION_CACHE_SCHEMA_VERSION` bumped **42 → 52** (session_cache.rs:8 ladder comment carries the full census + rationale; review-prescribed mint above the observed in-flight frontier 43–45 and the review-era 51). Proven live: a v42-banked cache (`schema_version=42` in the banked sessions.json) is REGENERATED to `schema_version=52` by the fixed binary over the same corpus and cache dir — the stale bank is rejected, never served. Oracle re-based in the same unit (usage_occurrence_contract.rs: fn name `…schema_52…`, pin tuple `(SESSION_CACHE_SCHEMA_VERSION, 52, …)`, both message literals, ladder prose naming run 57c491338818 → fix 670753a6).
- **F2 (medium)** html band acceptance arm made loud (attribution_dimensions.rs): nested `if let`/`if start<=end` vacuity removed — `.expect()` on both locators + explicit inversion assert; root-caused while fixing: the extractor never matched because html rows are `<tr><td>…` (and optional `<strong>`), so the arm had been structurally dead since mint. **Red arm proven**: sed-revert of reports.rs:2044 to alphabetical map iteration → test FAILS loudly ("band endpoints inverted start=24 end=23"); restore byte-identical (sha verified) → 14/14 green.
- **F3 (low)** parser.rs double-increment removed — fold ordinals are one-per-block emission positions (post-reuse labels no longer +1 off; suite 15/15).
- **F4 (low)** one-carrier discipline at the refold meta event: structural comment (this lane mints no earlier carrier to attach to; the events_total consequence is disclosed in the 52 ladder entry) — no dead merge arm.
- **F5 (low)** rm-245 doc comment re-attached to `task_type_items` (reports.rs), rm-919 doc stays with `sorted_task_type_groups` which it actually documents.
- CHANGELOG Unreleased/Fixed written (schema bump + F1/F2/F4 credits, rm-919 render order, rm-439 doctor scope); governance guide schema sentence 42→52; roadmap chain link-3 regenerated as `ROADMAP.rfix.patch` (addendum on the compound postimage, chain ed44938c→58091d4d→670753a6 roundtrip-proven with order enforcement; worktree ROADMAP.md still pristine) — same-unit literal set per the schema-bump rule.

Gates this turn: fmt rc0, clippy -D warnings rc0, usage_occurrence 5/5, hostile 15/15, attribution 14/14 (with proven red arm), doctor_demo 2/2, entrypoints 48/0, docs gate rc0. Emission-time digest re-declared (executable surfaces changed).

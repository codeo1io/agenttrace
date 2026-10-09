# Cycle 3 compound record — run 3f6b86bc (repository-maintenance 66430025)

- date: 2026-10-09
- run: 3f6b86bc00d84bd9945581536b0ce477 (repository-maintenance:66430025d19b4725ab3e662f3811939a:cycle:3)
- worktree: run-3f6b86bc00d8-3f6b86bc @ e249a109436807869a084b6f96eec4d537c5ec66, porcelain in = 14 M + 1 ?? (implement-time +917/−111; review-recomputed compound close +922/−111 — the prose number predates compound's own ROADMAP insertions, see the counts-drift doc) including the already-applied 7c90ea78 roadmap payload on ROADMAP.md (byte-verified against the spool postimage before any code work — stewardship's sequencing note); compound attempt 44dda6034fcc459c8c15d7fab6a15804
- contract: pre-review compounding only — assessment, research, roadmap, prioritization, implementation, and recorded test outcomes consumed as evidence; ZERO test/validation commands executed at compound; review and shipping outcomes deliberately absent (they land after this phase; the next cycle's assessment carries them)

## Batch

"Exact warm numbers, honest fork boundaries", selected by prioritize c7acff0c:

| id | track / priority | subject | disposition at compound |
| --- | --- | --- | --- |
| rm-803 (LEAD) | correctness 90.0 | serde_json `float_roundtrip` — warm-cache f64 exactness on shortest-round-trip literals | flipped candidate → implemented, EXECUTED bullet on the row |
| rm-805 | correctness 66.0 | opencode fork boundary-aware accounting (extends landed rm-548 whole-drop) | flipped candidate → implemented, EXECUTED bullet on the row |
| rm-807 | test 40.0 | dispatch-sanitize arm-completeness source pin | flipped candidate → implemented, EXECUTED bullet on the row |

Done-flips stay reserved for the commit gate (rm-012 convention). rm-804 (reliability 75.0, private ENV_LOCK removal) and rm-806 (capability 58.0, ZCode lane) remain candidates — the ready next-cycle leads, not implemented work. Next free id: rm-808.

## Phase inputs consumed (recorded outcomes, not re-run)

- **assess a6e82982** @e249a10 (fresh, after provider-reaped 9d66e934 — 24s, zero tool output, nothing durable): F1 float ULP first-hand repro (`/tmp/at-assess-a6e8/fparsetest` vs this tree's lock: 11.005401611328125 → 402602c3ffffffff), F3 fork-boundary drop semantics, F4 unpinned write arms; baseline core lib 224/0 ×3 isolated, workspace 1 failure (clear_cache victim of the rm-804 class), fmt/clippy/deny/docs gates rc0.
- **research 82f5597f** pass-13: R1 completed F1 with the upstream ruling — serde-rs/json#1336 CLOSED-completed, float fast path intentional on 1.0.151 AND master, `float_roundtrip` the sanctioned exact path, no release ≥1.0.150 changes it; ccusage precedents #1814 (flex multipliers), #1821 (compaction response-id pairing), #1824 (copilot credit reconciliation); PR #312 parser-wave table; LiteLLM drift 3,743 vs vendored 3,099.
- **roadmap 7c90ea78**: minted the band rm-803..rm-807 (+5 rows, 271 → 276 defs), status 147-69-60, delivered as spool patch + postimage (md5 9a410276f985bc22c8d4faa89e2d45cc), then applied in-tree by stewardship's sequencing decision.
- **prioritize c7acff0c** (after provider-reaped 8361c0b1): batch as above with mutation-hook acceptance riders; double-implementation sweep found zero `float_roundtrip` in 49 sibling worktrees + canonical.
- **stewardship ed80a176** (after provider-reaped 193b77c6): ONE physical repository decision — canonical @ea5c41e is 71 BEHIND the worktree wall (not diverged-ahead); this worktree is the implement surface; pinned every lead surface line.
- **implement a9a26f22** (first durable attempt; dead siblings 3c313b75 + ca3da456, both zero-durable): 14 M + 1 new, +917/−111. Red-first rm-803 pin; cost probe 212.2 → 230.0 ms/parse (+8.4%, 26.67 MiB ×5 iters) recorded per acceptance (3); rm-805 boundary logic both lanes + NEW tests/opencode_fork_boundary.rs; rm-807 source pin in main.rs's test module; mutation-verified all three (2× RM805 directions, 1× injected unsanitized arm), hooks grep-clean after revert; workspace 635/0 ×3, core lib isolated 226/0 ×3 (baseline 224).
- **targeted_tests 160a20fd**: validation-only, porcelain byte-identical in/out; 463/0 (core lib 226, discovery_contract 94, opencode_fork_boundary 3, cli package 140 across 9 targets) + clippy/fmt/docs-gate rc0; digest re-derived byte-equal with the engine's own code.
- **full_tests bb965bd5**: dispatch `validation.full_command` EMPTY → ci.yml push lanes executed verbatim via scratch runner (fbc4581f/494a30a0 provenance, path-adapted): 21/21 lanes rc=0 FIRST TRY; machine sums trio 635/0 across 35 binaries + entrypoints 43/0 = 678; the e249a10-baseline clear_cache flake absent.
- **Reap ledger**: five provider-reaped attempts across the run (assess 9d66e934, prioritize 8361c0b1, stewardship 193b77c6, implement 3c313b75, implement ca3da456) — every one ≤ a few heartbeats with zero tool output, verified zero-durable, redone from scratch per the provider-reaped precedent doc.
- **Digest lineage**: validation:v1:70597f6ff287536f08a9c1c44b62ee9cb8f1e1f2ac590dfbfe3ee5c6060f7955 — declared VERBATIM at targeted and full, re-derived byte-identical four times (targeted pre+post, full post, compound). Compound touches ROADMAP.md and docs/** only — outside the engine's executable set — so the digest cannot move at compound.

## Lessons

- **L1 — an upstream ruling converts a defect into a feature decision; encode BOTH the sanctioned path and its cost in acceptance before implementing.** rm-803 would have been minted as "fix float parsing" (or worse, a serde_json bump that upstream explicitly never ships) without research R1's #1336 ruling. The acceptance pre-declared: feature flip, no schema bump, a ≥10MB cost probe with a pre-declared fallback if unacceptable. Result: zero rework at implement, and the +8.4% parse cost is now a recorded fact, not an assumption.
- **L2 — port boundary RULES from a rival implementation, not just shapes.** rm-805's semantics came from ccusage 2b1ee578: v2 forks copy parent rows with preserved seq; rows at/below the last copied seq are skipped; the load-bearing rules are "keep-orphans-never-guess" (an ORPHANED parent keeps all rows — absence of parent evidence is never grounds for a drop) while an unresolvable boundary ON A LIVE PARENT takes the deterministic rm-548 whole-drop fallback WITH disclosed counters (drop-with-disclosure, not drop-on-a-guess: the fallback is the landed rm-548 behavior, not an inference). The same family instinct as ccusage #1821's retained-child protection. Dropping data on an inference is the failure mode; disclosing magnitude next to counts is the antidote.
- **L3 — mutation-verify new pins at implement time, then strip the hooks.** Two RM805 mutations (prefix-exclusion off; orphan-drop) and one RM807 injected-unsanitized-arm mutation each failed exactly the intended pin, naming the offending line/binding; hooks were reverted and grep-verified absent. A pin that has never seen its mutation fail is unproven (vacuous-green risk, cf. the red-first-hardening doc family).
- **L4 — counts in artifacts drift; recompute from the tree.** The implement handoff said "13 modified files + 1 new test file" and "4 tests"; the tree says 14 M + 1 ?? and 3 `#[test]`s (five narrative pins distributed across three tests). The full-suite runner's inline machine-sum awk printed an EMPTY total (quoting artifact). Both were caught only because validation phases recompute. Prevention rule promoted to docs/solutions/workflow-issues/counts-in-artifacts-drift-recompute-from-the-tree.md.
- **L5 — reap forensics is now mechanical.** Five reaps this run, all the same shape (few heartbeats, zero tool output, no typed envelope, no scratch). The sweep (event log tail → envelope absence → scratch sweep → redo-from-scratch declaration) never produced a false adoption. The cost is real but bounded; no change to procedure warranted.
- **L6 — when the roadmap payload is already applied in-tree, compound extends the dirty diff instead of shipping a spool patch.** Stewardship's sequencing note applied the 7c90ea78 payload in-tree (byte-verified) BEFORE code work, so this compound's flips land directly on the dirty ROADMAP.md and the commit gate lands ONE cumulative delta (implement code + roadmap + docs). That differs from the pristine-wall cycles (ac3ac300, feb16bba) where compound ships a spool patch; both are correct — pick by whether the payload is already in the tree at compound time, and say which pattern you used in the record.

## Boundaries held

- NO session-cache schema bump FOR RM-803 (rm-803 acceptance: the writer was already correctly rounded — parse-side feature only). The rm-805 review fix later moved the dir-cache 33 → 34 and the sqlite snapshot 8 → 9 because rm-805's orphan-kept totals change alters what warm caches would serve (CHANGELOG carries both); rm-803's own boundary held.
- rm-548's whole-drop default PRESERVED for pure replay: the replayed prefix stays excluded; only post-fork continuation is counted, and only orphans are fully counted (with disclosure). Verified load-bearing by mutation 1.
- rm-804 and rm-806 deliberately NOT touched (stay candidates; any "workspace green" record for this repo stays probabilistic until rm-804 lands).
- Canonical checkout untouched (reference-only; 71 behind the wall).

## Commit-gate seams (for the next phase, not performed here)

1. Flip rm-803 / rm-805 / rm-807 implemented → done (rm-012 convention reserves done-flips to the commit gate).
2. Commit the single cumulative delta: implement code (14 M + 1 new test file) + ROADMAP.md (7c90ea78 payload + compound flips/banner) + this record + the counts-drift prevention doc. CHANGELOG.md already carries all three cycle bullets (all under `### Fixed`) — verify with `grep -n "float_roundtrip\|fork" CHANGELOG.md`.
3. Post-merge fleet note: next free id rm-808; band remainder rm-804/rm-806.

## Next-cycle candidates (ranked context, not a selection)

1. rm-804 (75.0) — mechanical ENV_LOCK→lock_env routing; kills the recurring clear_cache/statusline flake class; zero product code moves.
2. rm-806 (58.0) — ZCode discovery+pricing (demand triple-confirmed: ccusage #1831/#1832 + rm-537 census); discovery-first sequencing per rm-548's pattern.
3. Watch items from research: serde_json ≥1.0.152 releases (none change the float path as of 1.0.151 — recheck before any bump); LiteLLM 3,743 vs vendored 3,099 (650 entries incl. claude-haiku-5-5); ccusage flex-multiplier pricing wave (#1814) as the next pricing-parity refresh input.

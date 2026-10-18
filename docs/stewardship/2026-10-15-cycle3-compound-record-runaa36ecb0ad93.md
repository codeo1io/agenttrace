# Cycle 3 compound record — run aa36ecb0 (repository-maintenance 759421c5), 2026-10-15

**Run:** aa36ecb0ad934977afcbffa192ab63f4 · **Lineage:** repository-maintenance:759421c573ae4f10a34fa68b1fd6ec29:cycle:3 · **Base:** c0da781 (8 past cab6917, 2 behind origin/master 3e90993) · **Compound attempt:** d196374da61b47aa97419dc9db597100 (redo of transport-dead 58d226e2 — event log shows boot + 2 `phase_result_repair` events + reap; no typed artifact, no scratch; zero durable work, declared, redone from scratch)

This record compounds the cycle's pre-review learnings into durable artifacts per the phase contract: NO test execution at compound — every outcome below is the already-recorded phase evidence, consumed as-is.

## Phase ledger

| Phase | Attempt | Durable status |
|---|---|---|
| assess | c279ff1a | typed JSON swept before compound; work-order record authoritative. Headline: full battery green at c0da781, only red = check-install-runtime test D (rm-051) |
| research | 94d4b84c | typed JSON + scratch swept; work-order record authoritative. Third same-day pass, three-way flat vs 5a6734fb/00e074db |
| roadmap | 2d55b308 | typed JSON survived; scratch + /tmp mirror swept → band RE-MINTED by this compound from the typed record |
| prioritize | fbae0c86 | typed JSON survived. Selected "Usage truth, cross-file" |
| stewardship | 7fe9374b | typed JSON survived |
| implement | 4137f90e | typed JSON survived; scratch + `.cache/at-impl-4137` mirror swept |
| targeted_tests | f8c71c1e | typed JSON survived; `.cache/at-targeted-f8c7` mirror swept |
| full_tests | bde3b724 | typed JSON + `.cache/at-full-bde3` mirror (BATTERY-SUMMARY.txt et al.) survived |
| compound | d196374d | this record; spool scratch + `.cache/at-compound-d196374d` mirror |

Two prior attempts across the cycle died of infrastructure (roadmap 472d58f3 provider-dead at 24s; compound 58d226e2 transport-dead); both were forensically verified dead and redone — zero silent adoptions.

## Batch record — "Usage truth, cross-file"

LEAD **rm-402** (codex MultiAgent V2 replayed-prefix double-count, p=80) + riders **rm-727** (statusline future-day disclosure, p=78) and **rm-558** (keyword-lane pre-flight flag guard, p=76). Selection was the only p≥78 open candidate with zero selection claims anywhere in the fleet and a clean parser.rs seam; riders were the next two truth-lane candidates whose seams were fleet-disjoint.

- **rm-402** — NEW `crates/agenttrace-core/src/codex_replay.rs`: corpus-level `reconcile_codex_multiagent_replays` wired at both assembly sites; earliest-start rollouts are parents, each replayed child rebases onto the parent's final `total_token_usage` snapshot with saturating per-field delta + tiered-price recompute; upstream-priced children skipped and disclosed. Red-first golden fixture pair `tests/fixtures/codex-multiagent-v2/{parent,child}.jsonl`; integration suite 3/3. E2E: `--overview -f json` on the fixture dir reports family true total 1,700 (naive 3,100, +82%) with both disclosure counters in `data_health`; control dir unchanged no-op.
- **rm-727** — `statusline.rs` gains `excluded_future_days`: future-day range count in `statusline_budget_series`, JSON emit, budget-view disclosure line; the 95d74221 F8 future-dated pin asserts the counter.
- **rm-558** — `main.rs` keyword lanes pre-flight positional-report-only flags: `validate_keyword_lane_flags()` (unit-tested) errors loudly naming flag + lane + remedy; upstream keyword stays quiet; statusline journal reads pinned byte-identical.

## Recorded validation (consumed, not re-run)

- **targeted f8c71c1e:** fmt rc0 · clippy (core+cli) `-D warnings` rc0 · core lib 391/0 · replay suite 3/0 · cli bins 112/0 · codex lane 104/0 · tree md5-identical in/out (zero executable delta this turn).
- **full bde3b724 (ci.yml-verbatim):** fmt rc0 · clippy trio rc0 · 10 static gates + `ruby -c` + npm test 15/15 · workspace 63 suites 1,099/0/0 · MSRV +1.89 rc0 · `cargo deny` rc0 · conformance 2/0 + 13-case runner rc0 · release-bin gates (output-contract / deterministic-output / report-semantics / example-workflows / docs-commands) rc0 · real-cli 276 workspaces rc0 · real-TUI 55 files rc0 · **only red = check-install-runtime test D (rm-051)** — standing, doubly-claimed by 966bd5f9/a64735b7, red at pristine base, disclosed not chased.

## Independent review outcome (af521003) — fixed in the review-fix turn (779f0a48)

Adversarial review of the batch on the exact implement/compound tree returned 9 findings; all actionable ones were fixed in the same cycle (file:line in the typed review artifact `delegate/af521003edcd4842a3e59446c6a1579a.json`):

- **F1 (critical) rebased-cost x1e6 inflation** — `codex_replay.rs:155` multiplied per-token by per-Mtok prices ($1,500-class estimates). Fixed: `rebase_cost` mirrors the analyze catalog formula exactly (every class /1e6 at its own rate; reasoning carries no separate term per rm-603 wire semantics). Pinned by `rebased_child_cost_uses_the_canonical_per_mtok_formula`.
- **F2 (critical) warm-cache role inversion** — `metrics.timestamps` is `#[serde(skip)]`, so warm runs tied at MIN_UTC and roles fell to walk order (shipped fixture dir walked child-first). Fixed: member start re-derived from the rollout FILE's first line timestamp, canonical path as deterministic tie-break. Pinned by `warm_cache_without_session_timestamps_keeps_roles_stable`.
- **F3 (high) concurrent-spawn child zeroed + mislabelled** — parent-final baseline invalid when the child spawned mid-parent; saturating delta erased real usage as "replayed tokens dropped". Fixed: all-fields baseline guard — child left intact with distinct `codex_multiagent_replay_skipped_below_baseline` disclosure. Pinned by `concurrent_spawn_child_is_left_intact_and_disclosed` (new fixture dir `codex-multiagent-v2-concurrent/`).
- **F4 (high) false families from short shared openings** — prompt-retry / templated-first-turn pairs merged and dropped usage. Fixed: opening qualifies only with 2+ shared items AND ≥384 serialized bytes, plus the child must continue past the shared opening. Pinned by `unrelated_short_opening_is_not_a_family` (fixture dir `codex-multiagent-v2-unrelated/`).
- **F5 (medium) TUI naive totals** — TUI loaded through the same core loaders but never reconciled. Fixed: reconcile at both assembly sites (`App::new`, `apply_loaded_sessions`) + 0-count-clean Ready-summary clause `replay-adjusted sessions`.
- **F6a (medium)** — this record's "fully discharged" claim, amended above.
- **F7 (low) rm-727 CHANGELOG entry** — added (deviation 2 closed).
- **F8 (low) literal far-future PoC unpinned** — added `far_future_samples_disclosed_and_cannot_evict_real_days` (the row's 12-25 shape).
- **F9 (low) zero-day window mislabelled future-only journals as older** — the empty window now labels exclusions by side; pinned by `empty_window_labels_future_only_samples_by_side`.

Residual (disclosed, not chased): the content-based family tie remains probabilistic at the margin — the ≥384-byte + own-tail discriminators shrink the false-merge class but cannot eliminate it without replay markers on the wire; below-baseline families now overcount conservatively (child intact) rather than destroying usage. Golden fixtures remain synthetic (see Next-cycle starter 3).

## Deviations and debts recorded for the review/commit phases

1. **rm-402 acceptance-mechanism deviation (intentional, disclosed):** the acceptance text presumes "a child rollout whose session id / replay markers tie it to a parent". The host wire hunt (read-only `~/.codex`, 2026-09-24 team cluster) proved team-cluster rollouts share **no session ids and no opening items** — independent exec team members each re-receive the full preamble. Detection therefore landed as content-based opening-payload fingerprinting (first 4 response_items, payload-hash chain, ≥2 byte-identical). The acceptance's intent — replayed-prefix tokens counted once, attributed to the parent, disclosed, single-file no-op — was **pursued** by the first implementation; the tie mechanism is not literally session-id based. *Amended after review af521003:* the original claim said "fully discharged" — the independent review refuted that breadth with 9 findings (2 critical: rebased-cost x1e6 inflation, warm-cache parent/child role inversion; 2 high: concurrent-spawn child zeroed + mislabelled, false families from short shared openings), all fixed in the review-fix turn and pinned by new tests. See "Independent review outcome" below. Review phase must still accept-or-flag the content-based tie mechanism explicitly.
2. **rm-727 CHANGELOG entry missing:** implement's summary claimed three Unreleased entries (rm-402 Added, rm-727 Added, rm-558 Fixed); the delivered uncommitted diff holds **two** (rm-402, rm-558 — verified `git diff CHANGELOG.md`). The changelog-structure gate is green (it checks structure, not per-row coverage). Commit gate: add the rm-727 entry or record the omission. *Closed in the review-fix turn: the rm-727 entry is now in CHANGELOG Unreleased (review af521003 F7).* Narrative-vs-diff cross-check caught this — keep doing that at compound.
3. **Standing red:** check-install-runtime test D (rm-051) — pre-existing, sibling-owned, out-of-batch; re-probe at the next base.

## Reusable lessons

- **L1 — typed JSONs are the only durable carrier.** The roadmap phase's scratch AND its `/tmp` mirror AND two of three `.cache/at-*` mirrors (at-impl-4137, at-targeted-f8c7) were janitor-swept before compound arrived; every typed delegate JSON survived. Consequence recorded as the new prevention doc `docs/solutions/workflow-issues/later-phases-rebuild-swept-roadmap-bands-from-typed-jsons.md`: later phases rebuild swept artifacts from typed records and declare re-mint provenance; earlier phases should keep full artifact text inside the typed result (this cycle's roadmap phase did — which is the only reason the band was recoverable).
- **L2 — decline-sweep seam adoption.** Implement's pre-flight swept every sibling spool mention of rm-402 and found all three were DECLINES ceding to this campaign, then verified hunk-region disjointness against all dirty trees before adopting the seam. Generalize: before implementing any roadmap id, sweep sibling delegate JSONs for selection claims — declines cede, claims collide.
- **L3 — evidence-forced mechanism deviations get recorded, never buried.** When wire evidence contradicts an acceptance's presumed mechanism, discharge the intent via the evidence-forced mechanism and record the deviation at compound for review (see deviation 1).
- **L4 — empty `full_command` means derive from ci.yml lane-for-lane.** Third-plus fleet occurrence; standing doc `empty-full-command-mirrors-ciyml-lane-for-lane.md` covers it; this run's full_tests applied it correctly (63 suites, all five jobs).
- **L5 — reaped-attempt forensics before redo.** Both dead attempts this cycle (472d58f3, 58d226e2) were verified via event logs + artifact + scratch absence before redo — the `provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` discipline held.

## Next-cycle starters (concrete, with provenance)

1. **codex sqlite state lane** (fog f084f50e, rm-1398/1406 family): schema evidence banked this cycle (codex-rs `state/src/sqlite.rs` six DB names; host probes: state_5.threads 99 rows with non-null tokens_used, sources incl. `subagent:review`). Re-census fog before claiming.
2. **rm-1053 exact-key pricing** — only after BY-TITLE reconcile: sibling lanes (d130035f, bbf61fbd batches) hold unlanded exact-key work and fleet memory reports b5d391ec landed reconcile-by-title past 3e90993; verify live at next assessment.
3. **rm-402 real-corpus corroboration:** goldens are synthetic; the 2026-09-24 host cluster was read-only evidence. A real-cluster E2E pin if a corpus lands.
4. **check-install-runtime test D (rm-051):** re-probe at the new base; sibling fix in flight.
5. **86–96 code-complete flip band:** flips pend spool-side (batching rejected this cycle on censure grounds).
6. **Fleet-fresh, verify-before-adopt:** sibling assess 0915f736 M1 — insights project-decode accepts dot-directory ancestors (project under a hidden root decodes `display_name` '.hermes'; `crates/agenttrace-core/src/insights.rs:549` walk). Confirm on-wall and PoC before adopting.

## Commit-gate seams (this record + roadmap chain)

- Land the roadmap chain: wall(3e90993) ← `delegate/d196374d…-scratch/ROADMAP.band.patch` ← `ROADMAP.compound.patch` (mirror `/home/agent/.cache/at-compound-d196374d/`); reconcile BY TITLE if the origin ceiling moved. Final postimage md5 `eef2ae5563461c2a495abfc8bb09d07f`, 392 def rows (zero mints).
- The worktree `ROADMAP.md` is untouched by compound (worktree porcelain = implement's 7-path batch + this record + the workflow-issues doc).
- Done-flips for rm-402/rm-727/rm-558 ride the commit gate, AFTER the review-fix turn's corrections (review af521003: 9 findings fixed — see "Independent review outcome"; the regenerated compound chain's rider language notes the review-fix). The rm-727 CHANGELOG entry has landed (deviation 2 closed).
- See `APPLY-SEAM.md` in the compound scratch for the exact land recipe.

## Addendum (2026-10-17, run e4bff731 cycle 1, repository-maintenance 145b52fe)

The residual note above ("≥384-byte + own-tail discriminators … cannot
eliminate [the false-merge class] without replay markers on the wire") is now
half-resolved from the wire side. The codex 0.147+ turn-boundary triple
(`event_msg` `task_started`/`task_complete`/`user_message`, live census
2026-10-17: 29/29 fresh host rollouts) is parsed and ledgered as turn evidence
(rm-1949), and the family tie is now gated on it: a family reconciles only
when every member journal carries `task_started` evidence. Two consequences
for the mechanism described at the "first 4 response_items, payload-hash
chain, ≥2 byte-identical" line: (1) pre-0.147 families (no boundary evidence)
skip conservatively — over-billed, disclosed, never erased — instead of
reconciling on the bare content tie; (2) the residual false-merge class is now
confined to journals that carry turn evidence, which is the corroboration the
note asked for. The short-parent structural gap (a ≤3-item parent can never
fingerprint-match a 4-item-hashing child — the "≥2 byte-identical" arm is
unreachable for real replay children) is rm-1946, deliberately NOT loosened
this cycle: rm-1907's corroboration guards were unlanded at this base, and an
unguarded loosening would trade conservative over-billing for false-merge
under-billing. See ROADMAP rows rm-1946/rm-1949 for the follow-ups.

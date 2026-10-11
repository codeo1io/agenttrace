# Cycle 3 compound record — run 652a6461 (repository-maintenance d44cb461)

- date: 2026-10-08
- run: 652a6461a20441eb82a5bd34020c011d (repository-maintenance:d44cb46181004f10a9bc2ca76825a9c4:cycle:3)
- worktree: run-652a6461a204-652a6461 @ 39bd06b3b4970c200b0205eb590fa89c1562cc8b, porcelain in = the 6-file implement band (6 M incl. ROADMAP/CHANGELOG) + `?? crates/agenttrace-core/tests/fixtures/subagent-samedir/`; compound attempt d70f9a8f0ff744549b9e46ed6a940f8a (first attempt — no prior compound attempt in the spool, nothing adopted at this phase)
- contract: pre-review compounding only — assessment, research, roadmap, prioritization, implementation, and recorded test outcomes consumed as evidence; ZERO test/validation commands executed at compound; review and shipping outcomes deliberately absent (they land after this phase; the next cycle's assessment carries them)

## Batch

"Retention truth for subagent rollups", selected by prioritize 4a11d593 (stewardship c15405da composed the request; implement eb63ba7d consumed them):

| id | track / priority | subject | disposition at compound |
| --- | --- | --- | --- |
| rm-787 (LEAD) | correctness 74.0 (minted this cycle) | preserved sessions keep their subagent rollups across `--include-history` | flipped candidate → implemented by implement, EXECUTED bullet on the row (incl. the ADOPT-then-COMPLETE declaration); done-flip reserved for the commit gate |
| rm-487 (rider, escalated arm) | customer-experience 78.0 | same-dir isSidechain sidechains link to their parent (Claude Code 2.x parks `agent-*.jsonl` beside `<uuid>.jsonl`, no `subagents/` dir) | dated partial-close note on the row; status STAYS candidate — full-row acceptance beyond this arm (overview surfacing, forked-copy composition with rm-232, parent_session id+path pairing, sibling layout variant, bare-listing deflation) enumerated on the row and left open |

Deferred at prioritize (both minted this cycle, both stay candidate): rm-788 (CSV parity, 58.0) — in-flight twin at sibling run-f6a7e7f37bb8; rm-789 (MCP 2026-07-28 refresh, 72.0) — rebase-family behind the 749cd298 rm-780..782 band.

## Phase inputs consumed (recorded outcomes, not re-run)

- **assess 13095ad3** @39bd06b (porcelain 0 in/out): envelope green by direct execution (workspace 595/0 across 31 result lines, fmt/clippy --workspace --all-targets -D warnings/release rc0); THREE live-PoC findings — F1 the rm-487 escalation corroboration (220 same-dir isSidechain files / 0 `subagents/` dirs on this host; -work-ci-fixes-glmplus 20 sessions / 0 links), F2 the retention falsification (parent `2 / 0.0053` live → `history-34235f44 0 0.0000` restored), F3 the CSV column drift. Dossier delegate/13095ad37dc745829983e6ce2603b38b-scratch/ASSESS.md.
- **research 4d99e0b4** @39bd06b (prior attempt b413897e failed, zero durable): ONE net-new candidate N1 → minted rm-789 (MCP 2026-07-28 changelog fetched live: handshake/ping removal, server/discover MUST, resultType required, CacheableResult ttlMs/cacheScope vs the 2025-06-18-era mcp.rs surface); R4 (glm-5.3 alias-fallback pricing) REFUTED live — `--overview -f json` on the fleet's own delegate corpus: by_model glm-5.3 16,681 sessions / $10,800.70, exact_pricing_pct 100.0, fallback 0 (bundled snapshot carries aihubmix/glm-5.3; convert_litellm resolves it exact); ecosystem otherwise flat (upstream 15ed07f quiescent, LiteLLM 4,486 keys unchanged, pi 1.0.4 no session-format change). Dossier delegate/4d99e0b4…-scratch/research-dossier.md.
- **roadmap 02306975** (prior attempt 2e97c10c reaped mid-turn at a gateway restart — 11-message event log, zero durable, redone from scratch): +26/-0 append-only; minted rm-787/rm-788/rm-789 past every landed and in-flight claim (census: this wall def-max rm-753, origin/master@96b528e rm-777, live worktree ceiling rm-786 @ run-24ec00eb, spool zero claims at rm-787+); dated escalation append on rm-487. Patch delegate/0230697557124fd384e55393be8e3e0c-scratch/ROADMAP.patch.
- **prioritize 4a11d593**: selected the batch above over the 254-row wall; seam adjudication recorded on the rows (history.rs dirty in 3 sibling worktrees with disjoint hunks — 4c3ca863's `branch` named as the integration seam; subagents.rs clean fleet-wide; the f6a7e7f3 CSV twin verified in-flight, hence rm-788 deferred). Dossier delegate/4a11d593…-scratch/selected-batch.md.
- **stewardship c15405da**: one change-unit batch in this worktree (NOT canonical — ea5c41e-class bases lag); ROADMAP +delta reserved to the commit gate; 5 must_remain_separate pairs. Envelope delegate/c15405da…-scratch/stewardship-envelope.md.
- **implement eb63ba7d** (after reaped 2ebf78f0 — ADOPT-then-COMPLETE, declared): final delta 6 M + 1 `??` = +400/−19; history.rs DerivedSession gains subagent_count/subagent_cost/subagent_tokens/parent_session (Default keep-compat, round-trip pinned); subagents.rs second resolution rule (same-dir journal sessionId → parent stem, exactly-one, self-links never link) + `history:` skip in re-attribution; discovery.rs pre-preserve attribution pass + the rm-545 comment made honest; 2 new E2E pins + the subagent-samedir fixture (parent/linked sidechain/orphan); RED-CONFIRMED leg-by-leg; live PoCs: retention RUN1 `2 / 0.0053` → RUN2 `history-a6922864 2 / 0.0053`, same-dir corpus 0 → 63 attributed parents / 126 linked sidechains with sum(rollups) == 126. Gates at implement: core lib 220/0, subagent_attribution 6/0, discovery_contract 90/0, fmt rc0, clippy --workspace --all-targets -D warnings rc0, release build rc0. CHANGELOG Fixed bullet written here.
- **targeted_tests 91fd53d1**: 6 lanes rc0, tree byte-identical in/out; digest validation:v1:3e36838e29b4e66ef560d2f7cff475dc553217515b12c3253d7111cfeb30c254 declared VERBATIM pre+post (re-derived with an engine-faithful port); changed_testable_surfaces [] explained — the executable set is scripts/** + .github/workflows/** (25 files), none touched. Log /tmp/at-tt-91fd/targeted.log.
- **full_tests 30af443a** (prior attempt fbd3add4 reaped ~25s, zero durable, redone): dispatch full_command EMPTY → ci.yml full+deny mirrored lane-for-lane; cargo test --workspace --locked 600/0 across 31 result lines rc0 (base battery 595/0 — +5 = this batch's tests, both new pins grep-verified ok); release build 2m35s rc0 (12,876,760B); clippy 40s rc0; fmt rc0; artifact gates (check-docs-commands via AGENTTRACE_BIN, locked-cargo, manifests, bash -n/sh -n/ruby -c) rc0; cargo deny all-four ok. Logs /tmp/at-full-30af/.
- **Digest lineage:** validation:v1:3e36838e… unchanged since targeted_tests — compound touches ROADMAP.md and docs/** only, outside the digest's executable set, so it cannot move at compound.

## Dead-attempt registry (this cycle)

| attempt | phase | fate | disposition |
| --- | --- | --- | --- |
| b413897e | research | failed after ~8min (two internal turn-starts, then failed completion); no envelope, no scratch | redone from scratch (4d99e0b4) |
| 2e97c10c | roadmap | reaped mid-turn at a gateway restart; 11-message event log; no envelope, no scratch | redone from scratch (02306975), declared |
| 2ebf78f0 | implement | reaped at 10m56s; no envelope, no scratch — but the complete core implementation sat as worktree drift on exactly the 3 batch surfaces | ADOPT-then-COMPLETE (eb63ba7d) under the new prevention doc's legs |
| fbd3add4 | full_tests | reaped ~25s; 9 messages; no envelope, no scratch | redone from scratch (30af443a), declared |

Lesson (extends the fleet's provider-reaped family): at implement, a porcelain MISMATCH vs the prior handoff is not only contamination — the tree drift itself can be the dead attempt's durable artifact. New doc: docs/solutions/workflow-issues/reaped-implement-attempts-leave-the-fix-in-the-tree-adopt-then-complete.md.

## Prevention rules (this cycle)

- **PR-1 (new doc):** the adopt-then-complete protocol for reaped implement attempts — drift-mapped-to-batch-surfaces, line-by-line semantic read against the selection dossier, red-confirmation by reverting each leg, mechanical repairs in place, the completing attempt owns tests/fixtures/CHANGELOG/ROADMAP flips, declared with forensics legs. Doc minted above.
- **PR-2 (reaffirmed, third+ fleet hit):** dispatch `validation.full_command` EMPTY → the repo's ci.yml (full + deny jobs) is the suite authority, mirrored lane-for-lane (precedents db6b76260a11 / 84c45ccd / 53c9af4d; the local cargo-deny 0.20.2 `--all-features` flag rejection is metadata-level only).
- **PR-3 (reaffirmed):** validation phases declare the digest and EXPLAIN changed_testable_surfaces rather than leaving an empty list uninterpreted (KTD13 family; this cycle's [] traced to the executable-set enumeration, recorded first-hand).
- **PR-4 (reaffirmed):** red-first discipline for adopted work — an adopted diff is proven by reverting its legs and watching the pins fail, not by reading alone (now codified as PR-1's step 3).

## Commit-gate payload (one commit; review/shipping follow)

- 7 modified tracked files (CHANGELOG.md +1 Fixed, ROADMAP.md = roadmap +26 / prioritize +4 / implement +2 / compound banner +2 / review-fix + gate notes, history.rs, subagents.rs, discovery.rs, sqlite_sessions.rs [review-fix: shared env lock], tests/subagent_attribution.rs) + 3 new paths (`?? crates/agenttrace-core/tests/fixtures/subagent-samedir/` 3 journals — the E2E pin consumes it; this record; docs/solutions/workflow-issues/reaped-implement-attempts-leave-the-fix-in-the-tree-adopt-then-complete.md) — stage the fixture dir and both docs explicitly.
- Gate actions owed: done-flip rm-787 (implemented → done) per the rm-012 convention; rm-487 stays candidate. CHANGELOG rider already written by implement (no rider minting owed at the gate).
- Integration seams: history.rs (4c3ca863's `branch` rides the same DerivedSession construction); ROADMAP banners reconcile by title at integration (two sibling cycle-3 compound records exist unlanded in other worktrees — run397538bc, rune88f2da2fbe9).

## Next-cycle context

- Leads: **rm-788** (58.0 — verify the f6a7e7f3 twin landed before touching it; close by title if it did), **rm-789** (72.0 — after the 749cd298 rm-780..782 band lands + fresh census), **rm-487 full-row remainder** (overview surfacing, forked-copy composition with rm-232, parent_session id+path pairing, sibling layout variant, bare-listing deflation = rm-545 remaining scope), and the **same-dir parse-failure lane** (113/126 sidechain files on -work-ci-fixes-glmplus parse to nothing — below-threshold/parse-failure class; unowned; the next assess should probe it before same-dir linkage is mistaken for full coverage).
- Watch (research 4d99e0b4): upstream 15ed07f quiescent (7th consecutive); MCP spec 2026-07-28 vs `agenttrace mcp` (rm-789); LiteLLM 4,486 keys unchanged; pi 1.0.4 (2026-10-05) no session-format change; glm-5.3 pricing EXACT on this fleet's own corpus (R4 refuted — do not re-mint an alias-fallback row without new evidence).
- Fleet bands to sweep at next assess: 749cd298 (rm-780..782 MCP), 24ec00eb (rm-783..786), 14954d7a (rm-778/779), f6a7e7f3 (CSV twin), plus the two sibling 2026-10-08 cycle-3 compound records if they integrate first.
- Wall after compound: 254 def rows, 0 duplicate ids, tail rm-787/788/789, managed footer last line; status mix 131 candidate / 66 implemented / 57 done. Band ceiling rm-789; next free rm-790 AFTER a fresh live claim census.

## Review-fix appendix (attempt b45f8690, 2026-10-08)

independent_review 355e05af returned NEEDS_CHANGES (1 medium + 3 low; implementation approved, headline claims reproduced). All four repaired at b45f8690 — test-only + docs, no production behavior change:

1. **[MEDIUM, reproduced]** subagents.rs unit-test `Scratch` helper shared one pid-keyed temp dir across the parallel tests; every guard's Drop `remove_dir_all`'d the whole shared dir, deleting journals mid-assert (reproduced 1/50, /tmp/at-rev-355e/fail-47.log). Fix: unique dir per test (pid + tag + atomic seq, subagents.rs:231); race impossible by construction. Re-stress: 60/60 subagents-lane iterations + 40/40 full `--lib` runs, zero failures.
2. **[LOW]** rm-787 EXECUTED bullet cited a nonexistent unit test (`rollups_round_trip_through_preserved_history`) and a `subagent_attribution.rs:170` anchor pointing at the TSV-header test. Corrected in place (`preserved_history_rows_keep_their_persisted_rollups` subagents.rs:320; restore-leg anchor :304) + dated review-fix note on the row.
3. **[LOW]** claimed legacy keep-compat had no test. Pinned: history.rs `legacy_records_without_subagent_fields_restore_zeroed` (:517) — pre-rm-787 record decodes via decode_records and restores as `history-34235f44` zeroed, never an invented link.
4. **[LOW]** E2E retention test mutates process-global AGENTTRACE_HISTORY_DIR. Serialized under a module-level HISTORY_ENV_LOCK mirroring in-crate `test_env::lock_env` (poison-tolerant), documented in place.

**Found at revalidation (pre-existing, not a review finding, not batch-introduced):** `sqlite_sessions::tests::failed_loads_neither_store_nor_trust_an_empty_snapshot` held a LOCAL `ENV_LOCK` instead of the shared `crate::test_env::lock_env()` — the only env test left on a per-module lock fleet-wide — letting the pricing/session-cache/statusline env tests re-point/remove `AGENTTRACE_SESSION_CACHE_DIR` mid-window, so `session_cache_path()` resolved to the REAL host cache (`/home/agent/.cache/agenttrace/opencode-sqlite.json` exists there) and the `!snapshot.exists()` assert failed (1 failure in ~25 stress iterations). Same latent-flake class the review blocked on; migrated to the shared lock (test-only, sqlite_sessions.rs:1404). Post-fix: 40/40 full-lib stress green.

Post-fix validation (targeted, /tmp/at-fix-b45f/validation.log): core lib 221/0 (= 220 + new pin), subagent_attribution 6/0, sqlite_sessions 7/0, `cargo fmt --all -- --check` rc0, `cargo clippy -p agenttrace-core --all-targets --locked -- -D warnings` rc0. Digest re-derived at HEAD 39bd06b with the fix delta in tree: UNCHANGED `validation:v1:3e36838e29b4e66ef560d2f7cff475dc553217515b12c3253d7111cfeb30c254` (crates/** and *.md are outside the classifier's executable set — verified by re-derivation, engine-faithful port).

# Cycle 1 compound record — run b5b85faa61c9 (repository-maintenance ec4f7625, cycle 1)

- **Date:** 2026-10-10 · **Attempt:** 23d64ffc908f46a4a1ebdd1df570ad62 · **Base:** aad1f7b1cfff28b61f1bec6ccde307816a0c24f5 (HEAD unchanged all cycle; porcelain never left the implement batch)
- **Theme:** honest surfaces — rm-937 LEAD (sessions TSV honestly requestable + stdout purity) with two stale-anchor repairs adjudicated out (rm-618 → landed rm-602; rm-709 → fold into landed rm-584)
- **Inputs consumed (pre-review, per the compound contract):** assess 7b8da498, research ca8861dc (pass-14), roadmap b7972ceb, prioritize 5843e27b, stewardship f755bf53, implement 97201ea46, targeted_tests 29c12bca, full_tests 29a0dbc9. **No test execution at compound** — every validation figure below is transcribed from those banked results, and every load-bearing claim was re-derived against the banked artifacts (diffs, evidence files, banked JSONs), not trusted from prose.
- **Method note:** the ce-compound skill is not installed in this delegate harness; its methodology (context analysis → solution extraction → related-docs overlap check → discoverability) was read from the read-only skill text and embodied directly, single-orchestrator form. Disclosed per the standing substitution rule.

## State at compound

Uncommitted batch at base aad1f7b: **5 M** (`README.md`, `crates/agenttrace-cli/src/main.rs`, `crates/agenttrace-cli/tests/entrypoints.rs`, `crates/agenttrace-cli/tests/output_safety_matrix.rs`, `crates/agenttrace-core/tests/parse_torn_tail_contract.rs`), no untracked files. `ROADMAP.md` **pristine** (md5 `6cc9f65e44fda029d436176d086c407a`, byte-verified this pass) — the roadmap layer ships as a **spool patch chain** (below). Porcelain at close: the same 5 lines; HEAD `aad1f7b` unchanged.

## Cycle outcome

| Phase | Attempt | Outcome |
|---|---|---|
| assess | 7b8da498 | fresh adversarial pass at merged-tree aad1f7b (two divergent merges b6a0c2e/aad1f7b had never been suite-tested as a merge); cargo check/fmt/clippy/deny + 5 targeted suites green; live PoCs: rm-912 refuse-overwrite (incl. warm-cache), CU3 `--clear-cache --demo` purity, workbuddy basis-clamp disclosures, RLO raw-bytes TSV |
| research | ca8861dc | pass-14: every watch lane re-probed first-hand; opencode GRADUATED ORGS (sst→anomalyco, 212,417★); all 12 crates.io maxima flat; rust stable 1.99.0 vs the 1.98.1 pin; prior-claims sweep FIRST (379-row wall registry title-grepped per candidate) |
| roadmap | b7972ceb | minted rm-937 + rm-938 past every live claim; spool patch sha `e13ab1ba…`, postimage `6ee022bb…`; roundtrip byte-verified; worktree wall untouched |
| prioritize | 5843e27b | census: 167 TRUE unresolved candidates; status-trap caught (11 inline `candidate → implemented` transitions — full-status-line scan required); selected {rm-618 87.0, rm-709 86.0, rm-937 65.0} |
| stewardship | f755bf53 | structured stewardship JSON + companion; batch RE-ADJUDICATED: rm-618 REMOVED (landed-banner 58fbbd6c obligation "do not re-implement"; `qwen_usage` :3221-3285 verified: `select_alias` + `subtract_cached_input_v2`) |
| implement | 97201ea46 | batch {rm-709, rm-937}: rm-709 re-adjudicated ALREADY-LANDED content under rm-584 → delivered only the integration-level contract test; rm-937 implemented red-first (4-file batch); prior attempt 799fad02 verified dead-at-start |
| targeted_tests | 29c12bca | empty changed_testable_surfaces in the dispatch → focus derived from the run's actual 4-file delta; entrypoints 50/0 + launch_guards 13/0 + torn-tail 5/0 + bin 84/0; fmt/clippy rc0; digest `validation:v1:dc9b543cbf479be5ffa0f3a7b6ce…` == dispatch |
| full_tests | 29a0dbc9 | ci.yml push lanes verbatim; FIRST RUN caught the stale matrix test (`tsv_is_not_a_cli_value_and_stays_rejected` asserted rc2 for `-f tsv`), rewritten as `tsv_is_a_sessions_scoped_cli_value_with_a_named_guard_elsewhere`; rerun: main lane **912/0** (core 463 + cli 422 incl. bin 84 + tui 27), entrypoints 50/0, fmt/clippy(both CI forms)/docs-script/deny rc0; digest recomputed == `dc9b543c…` |
| compound | 23d64ffc | this record; chain link 2; CHANGELOG entry; solutions doc; one provider-dead prior attempt adopted after re-verification |

## Batch adjudication story (three removals, one landing)

1. **rm-618 (87.0) — removed at stewardship.** The wall's landed integration banner (case 58fbbd6c) records "the rm-618 stale annotation (fix landed as rm-602 at qwen_usage :2995-3009 — do not re-implement)". Code-verified at aad1f7b: `qwen_usage` (parser.rs:3221-3285) implements the full upstream #312/#316 discipline (`select_alias` first-alias-wins + `subtract_cached_input_v2` cache clamp). The row's status token still read `candidate` — trap class (2) in the new solutions doc.
2. **rm-709 (86.0) — removed at implement.** Its own block carries `SUPERSEDED AT SELECTION … landed rm-584` + `ADJUDICATED AT INTEGRATION (2026-10-08, conflict case 2ac5bbe1): resolved as FOLD … nothing further is scheduled under rm-709` while the status token read first-token `candidate` — trap class (3). The only honest deliverable was the missing integration-level arm: `codex_fast_path_torn_tail_discloses_through_line_skips` (parse_torn_tail_contract.rs), red-proven via the counter-neuter (neutralizing the parse_counters fold on an otherwise-pristine tree FAILS the contract).
3. **rm-937 (65.0) — implemented, this cycle's landing.** Mechanism: `-f tsv` accepted into the value_parser as a sessions-scoped value (`--overview -f tsv` → rc1 `tsv format requires --sessions (the session table)`); the CU3 announce set gains `tsv` AND the DEFAULT `--sessions` text lane now keys on `(sessions && format == "text")` so side-effect chatter (`Session cache cleared.`) moves to stderr; explicit `-f tsv` and the default lane are **byte-identical** on stdout (cmp g1.tsv g2.tsv). README: `-f` table (:358) + `--sessions` row (:368) gain the tsv wording; the `--compare` narrative's `--model` drift fixed to `-m` (:361) inside the same scope (discharges the rm-390 rider's README half). Tests: entrypoints `sessions_tsv_lane_is_requestable_and_stdout_pure` (red-first on the pristine aad1f7b binary) + the rewritten matrix contract test.
4. **output_safety_matrix contract rewrite — at full_tests, not implement.** The first full run failed the stale rm-625-F4 pin-the-rejection test the moment `-f tsv` became valid; full_tests 29a0dbc9 rewrote it to pin the NEW contract (rc0 sessions+tsv; rc1 overview+tsv with the named guard). Lesson L2 below.

Validation ledger (transcribed, not re-run): targeted 152/0 focused (50+13+5+84); full 912/0 + entrypoints 50/0; fmt, clippy (both CI forms), docs-command script, cargo-deny all rc0; engine digest `validation:v1:dc9b543cbf479be5ffa0f3a7b6ce…` recomputed == dispatch at BOTH the 4-file targeted tree and the 5-file full tree.

## Roadmap accounting (spool chain, worktree wall pristine)

| Link | Artifact | sha256 | postimage |
|---|---|---|---|
| 1 | b7972ceb `ROADMAP.roadmap.patch` (mints rm-937/rm-938 + rm-007/rm-390 riders) | `e13ab1baac900a15340f722082c51c71d3ea4284e63142cd917770ccb257fc08` | sha256 `6ee022bbeec0fb6b1a0680d0288fd9413e1345fff0bdd1704d54fb3a6de5cd85` |
| 2 | **23d64ffc `compound.roadmap.patch` (this phase)** | see pins.txt (command-emitted) | see pins.txt (`work/ROADMAP.compound.md`) |

- Link 2 content: rm-937 `candidate → implemented` (done-flip reserved to the commit gate, rm-012 convention) + dated implemented rider; rm-618 and rm-709 `candidate → done` **STALE-ANCHOR REPAIRS** citing their adjudications (rm-602 landing / fold 2ac5bbe1); re-verification riders on rm-584, rm-602, rm-390; the cycle-1 compound banner (blockquote, mints section).
- **Chain proven this pass, from the pristine wall:** cp worktree ROADMAP.md (md5 `6cc9f65e…`) → apply link1 → byte-identical to its postimage → apply link2 → byte-identical to this postimage (4,218 lines, footer last line); link2 re-apply on the pristine wall correctly REJECTED (rc1, by design — context-matched to the postimage); reverse-apply checks clean. Workdir `/tmp/at-compound-23d6/chain`.
- **Postimage census (this pass, by grep):** 382 backticked id rows on the postimage and final (worktree wall: 380), **0 duplicate ids**, max id rm-938, 5 managed footers, last line footer, mint id-lines exactly-once. Reconciliation note: the roadmap/prioritize phases reported 379 wall / 381 postimage — an off-by-one in their def-row sweep; the delta conclusions (2 mints, exactly-once, no dups, next free rm-939 after a fresh census) are unaffected and were re-derived here.
- **Status-token delta postimage → final: exactly 3 id-lines changed** (rm-937, rm-618, rm-709), stream-diffed by script.

## Dead-attempt ledger (three deaths, one adoption; from events/*.jsonl + disk)

| Attempt | Phase | Disposition |
|---|---|---|
| 799fad02 | implement | dead-at-start (event log empty at implement-time forensics; no typed envelope, no scratch) — nothing to adopt; declared by 97201ea46 |
| aa4b9607 | implement | provider reaped mid-turn (turn_started 08:36Z → system kill 08:56Z); no result, no scratch — nothing to adopt |
| a6e41750 | compound | provider **output-limit truncation** mid-turn (final assistant message cut mid-patch; result never banked). One durable artifact: the ROADMAP compound postimage (sha256 `694311863dc1c8dc65a4bad89e81fe17ce3278082ccf5281d20bf10ef9e5d82c`). **ADOPTED by this attempt** after: 7-hunk diff vs the link-1 postimage reviewed hunk-by-hunk; every narrative claim re-derived against banked artifacts (batch.diff, TARGETED/FULLTESTS evidence, worktree code pins); **5 mis-attributed `implement aa4b9607` cites corrected to 97201ea46** (the dead sibling never banked); one ungrounded banner phrase (L3) replaced with this run's verified evidence; the patches/record/solutions doc/pins the dead attempt promised — but never wrote — built fresh here. Per `docs/solutions/workflow-issues/adopting-provider-dead-attempts-uncommitted-code-reverify-hunks-reds-and-narrative.md` |

Correction discipline note: the count-asserted replacement script refused twice on text that did not exist in the adoptee (a banner model mis-carried from a folded diff read) — assertions before edits are what kept this record free of confabulated banner text.

## Prevention rules & lessons (this cycle's additions)

1. **False-candidate defeat modes (L1)** — new solutions doc `docs/solutions/workflow-issues/roadmap-status-censuses-read-full-status-lines-and-adjudication-notes.md`: three shapes (inline status transitions; landed-banner recorded-not-actioned obligations; a row's own dated adjudication outliving its token) + the both-directions rule (banner lines can also DESELECT live work — the rm-619 false-flip class, run cfe69077). Overlap-checked against existing docs before minting (nearest neighbors are the release version-parity and provider-reaped families — different defects).
2. **Targeted-battery blind spot (L2)** — targeted scopes by changed crates/surfaces; a CLI-contract change invalidates OLD-contract pins anywhere in the test tree. `output_safety_matrix`'s rejection test sat outside every targeted focus set and fired only at full. Rule: CLI-surface changes get a test-tree-wide grep for the old contract (value names, rc pins, guard strings) as part of the targeted phase.
3. **Mid-cycle integration landings age phase evidence (L3)** — this run's base moved a2ecea9 → aad1f7b via two divergent merges before assess; every phase re-anchored HEAD and re-pinned load-bearing line anchors first-hand (prioritize caught the wall's stale `qwen_usage :3187` signals; stewardship re-pinned :3221). Never carry line anchors across a merge without re-grepping.
4. **Engine digest surface (L4)** — `validation_digest` covers the declared executable surfaces, NOT test files: the 4-file targeted tree and the 5-file full tree (test-file edit included) produce the SAME digest `dc9b543c…` (recomputed at both phases). A matching digest therefore does NOT certify the test tree — state test evidence separately, as this record does.
5. **Def-row census arithmetic** — grep-counted 380 wall / 382 postimage vs the phases' 379/381: one-row sweep discrepancies are cheap to catch and worth recording, because mint-exactly-once claims depend on the count baseline.

## Residuals / not done (honest)

- rm-938 (CI step-summary lane) minted and deliberately **not** implemented — schedule-behind-demand gate (research N2: no user demand evidence yet).
- The ANSI observation from full_tests (info finding): a "Saved:" overview.md capture logged ANSI clear-screen sequences during the suite — pre-existing, no failing test, flagged for an adversarial re-check next assess (the #17731 vacuous-fixture neighborhood).
- The 11-transition census class and the older banner residuals (rm-617/897/898/899 done-flips still pending on this wall) are recorded, not actioned — the flips belong to the landing gate.
- Nothing from the not-selected set was touched (rm-421/rm-251 campaigns, rm-195 decision, rm-448 wave, rm-232, cx set rm-037/087/158/339/487).

## Ranked next-cycle candidates (pre-review context; next assess adjudicates)

1. **rm-619 opencode `tokens.reasoning` fold (HIGH, cross-run):** run 5a04ae3b assess 6b369cf6 live-proven — `add_opencode_tokens` returns true on any tokens object while folding only input/output/cache; reasoning-only usage yields an empty-usage meta event with no disclosure counter. Same truthfulness family as this cycle's lead; small, red-first-able.
2. **by_model round4 attribution drift (MEDIUM, cross-run):** per-block rounding vs session-total rounding breaks `sum(by_model).cost == total` (PoC: 0.0002 vs 0.0004). Pairs with the existing rm-619-adjacent accounting band.
3. **fmt_duration boundary carry (LOW, cross-run):** 3599.8s → "60.0m", 59.6s → "60s".
4. **rm-195 decision → rm-448 v0.10.0 port wave** (decision-gated; research pass-14 grounds the port's URL-tolerance requirement — sst/opencode now 301s to anomalyco/opencode).
5. **rm-176 pricing-snapshot refresh wave** (research rider: LiteLLM drift +279 true chat rows / −3 since the bundle; claude-haiku-5-5 upstream).
6. **Watch:** opencode org graduation (212,417★ under anomalyco; release probes must follow the redirect); rust stable 1.99.0 vs the 1.98.1 pin (deliberate-bump policy); rm-936 Cf/bidi lands at its own integration (unlanded spool c23d7ee8) — re-probe after it lands; MSRV 1.89 undocumented user-facing (cross-run).
7. **Numeral frontier:** next free rm-939 AFTER a fresh census (bands rm-913..936 held by unlanded fleet runs per the registry).

## Integration handoff (commit gate)

1. The 5-file batch is already in-tree (uncommitted). Stage exactly: the 5 M paths + `docs/stewardship/2026-10-10-cycle1-compound-record-runb5b85faa61c9.md` + `docs/solutions/workflow-issues/roadmap-status-censuses-read-full-status-lines-and-adjudication-notes.md` + `CHANGELOG.md` + `ROADMAP.md`. No other paths.
2. Roadmap: apply link 1 (`e13ab1ba…`, b7972ceb scratch) then link 2 (`compound.roadmap.patch`, this scratch) to the pristine wall, in order; each step byte-verifies against its postimage (pins.txt). Link 2 on the pristine wall is REJECTED by design.
3. CHANGELOG: apply `compound.changelog.patch` (one Fixed entry, inserted top of the Unreleased `### Fixed` section; verified to apply clean to the pristine CHANGELOG and to reproduce the edited reference byte-identically).
4. Docs: apply `compound.docs.newfile.patch` (both new files, git-format) — STAGE explicitly; `commit -am` drops untracked paths.
5. Durability capture: `worktree-batch.diff` (this scratch) = the 5-file uncommitted delta, for forensics if the worktree is lost.
6. Done-flips at landing: rm-937 `implemented → done`; older banner residuals rm-617/rm-897/rm-898/rm-899 **if still pending** on the landing wall (verify against the landed banner before flipping — the oldest-first convention).
7. Digest: re-derive via `hermes_conductor.validation_policy.validation_digest('<40-hex base sha>', '<worktree root>')` — docs/ROADMAP edits are digest-immobile, but the gate should re-verify against the final staged tree, and test-tree state must be certified by the recorded suites (L4), not the digest.
8. Canonical checkout `/work/projects/agenttrace` carries a staged FOREIGN CI/release dirty set (`A .coderabbit.yaml`, `A changelog.yml`, `A publish-channels.yml`, `M ci.yml`, `M release.yml` at ea5c41e) — **preserve, never fold**.

## Verification

- Chain: `cd /tmp/at-compound-23d6/chain` — pristine cp → link1 → `cmp` clean vs `6ee022bb` postimage → link2 → `cmp` clean vs `work/ROADMAP.compound.md`; link2 rejected on the pristine wall (rc1); `git apply --check -R` clean.
- Census/flips: python stream-diff over id-lines — 382 rows / 0 dups / 5 footers / last-line footer; exactly 3 changed id-lines (rm-709, rm-618, rm-937).
- CHANGELOG: fresh-copy `git apply --check` clean; applied output `cmp`-identical to the edited reference.
- Adoptee verification: `diff -u` link-1 postimage vs a6e41750 postimage = 7 hunks, all reviewed; sha256 of the adoptee re-derived by command (`694311863dc1…` — an earlier hand-transcribed digest was WRONG and was caught by the byte-assertion; consume command-emitted pins only).
- All pins in `pins.txt` are command-emitted at write time (sha256sum), never hand-typed.

# Cycle 2 compound record — run 87c6258587f9 (repository-maintenance c899be74, cycle 2)

**Compound attempt:** 6d6dfb65b3b64d038f505ac5fca76433 · 2026-10-09 · worktree `run-87c6258587f9-87c62585` @ base `5ed9ebcc6355c225da0c19f76683d4c2dc99d8c7` (5ed9ebc). **Adopted & verified by:** 0f17c189d41744c5a68756785872166f (the redo attempt) — see "Adoption forensics" below.
**Status at compound start:** porcelain = exactly the implement batch's 12 entries (4 M: `crates/agenttrace-core/src/{lib,parser}.rs`, `tests/{discovery_contract,usage_accounting_truthfulness}.rs`; 8 ??: 3 new test files + 5 fixture files). COUNT NOTE: the implement/full_tests envelope prose said "13 entries / 9 ?? / 6 fixtures" — off by one; the cryptographic baselines are authoritative and reproduce exactly over these 12 lines (sha256 `eaafdd18…` pre==post battery at full_tests, md5 `12ac9cd3…` pre==post at targeted, both re-derived at compound over the porcelain minus this compound's two additions). fmt_duration_carry.rs uses inline literals (no fixture file). No tracked-file drift introduced by assess/research/roadmap/prioritize/stewardship/targeted/full — all spool-side.
**Pre-review:** this record consumes ONLY pre-review cycle evidence. Targeted and full-validation outcomes are transcribed from their phase records, NOT re-run (compound phase contract — no test execution). Review and shipping outcomes are deliberately absent; the next cycle's assessment carries them.

## Phase ledger

| phase | attempt | outcome |
|---|---|---|
| assess | e9cd1a5853a94c298fab5276d82841c2 | 8 fresh findings F1–F8, 14-PoC hostile corpus, pristine-tree green (workspace ~780/0) |
| research | 3cf87c2d223244cd93de9f5449603e35 | upstream NEGATIVE (3rd quiet check), C1–C6 candidates, id-less PoC pair |
| roadmap | 44b65fcb685b48859d7a7540a1792376 | minted rm-880..885 spool-side (patch + postimage + gate note; worktree porcelain 0) |
| prioritize | 8fff348e15bf4b1b9d96aa347308e71f | batch selection + live fleet collision census |
| stewardship | ecda346ad60b492cb032ec6eb64eb858 | surfaces re-pinned FRESH at 5ed9ebc (assess-time line anchors had drifted) |
| implement | cb32d0015f5c4ad6ae0e1f5ec81a9bca | truthfulness trio, RED FIRST (8 tests / 3 files), then green |
| targeted_tests | 8f75a10f6ebd400ab609e896b3f9c153 | 6 suites 358/0 + clippy 0 + fmt rc0; porcelain/diff sha pre==post |
| full_tests | e77dbf79a3df47eda2c4bd0d4bb263f7 | ci.yml push-lane mirror 20 lanes rc0 + 2 conditional skips + supplemental workspace 691/0 |
| compound | 6d6dfb65b3b64d038f505ac5fca76433 (author, provider-dead mid-turn) → 0f17c189d41744c5a68756785872166f (adopted + verified) | this record + ROADMAP flips (zero mints, zero test runs) |

## Batch and ROADMAP flips

Batch **"journal-to-report truthfulness trio"**: rm-880 (LEAD) + rm-882 + rm-883's fmt_duration-carry half.

- **rm-880 → implemented** (+ dated EXECUTED bullet). `parse_claude_code_jsonl` returns `(events, idless_reemission_folds)`; four-gate discriminator (no intervening user line ∧ same model ∧ every token class non-decreasing ∧ second-scale gap) max-folds re-emissions and discloses the fold. Red-first fixture pair pinned BOTH directions from the preserved research PoC.
- **rm-882 → implemented** (+ dated EXECUTED bullet). Arm (a): generic lane maps type→role for user/assistant/tool keys; `role_unclassified` census row for the remainder. Red-first PoC pair (hello-lf 0/0/0 → 1/1 tokens 100/50; hello-tool claude_code 1/2/1 unregressed).
- **rm-883 stays candidate** (+ HALF-EXECUTED rider): only the fmt_duration minute-carry arm executed ('1h 0m' at 3599.8s, boundary cases 3599.4/3599.8/3600.0 as inline literals, red-first 0/2). The round4-dedup arm (mcp.rs:371 vs lib.rs:2029) remains open. **Twin seam:** sibling run 2a5cdb9e's uncommitted rm-872 implements the same carry with rendering `'59m 59.8s'` — reconcile by title at integration, pick one rendering.
- Done-flips stay reserved for the commit gate (rm-012 convention). ZERO ids minted at compound.
- ROADMAP delta in this worktree now carries the whole chain as one diff: spool `roadmap.patch` (44b65fcb) → verified postimage (`ROADMAP-after.md`; worktree file was sha-identical to `ROADMAP-orig.md` before the swap) → this compound's banner + flips + riders. Managed footer still last; 307 def rows; 0 duplicate ids.

## Evidence consumed (pre-review, transcribed)

- **Implement cb32d001 (red first):** idless_reemission_dedup 0/3, generic_lane_role_disclosure 0/3, fmt_duration_carry 0/2 failing on the untouched tree, then green; `cargo test -p agenttrace-core` 29 suites 494/0 incl. both flipped pins; clippy 0 warnings; fmt clean (one `cargo fmt` fix to the new parser block); live binary on preserved PoCs (claude-idless-stream 120/60 + disclosed fold; claude-idless 300/130 no fold; hello-lf generic 1/1 100/50; hello-tool claude_code 1/2/1; edge 3599.8s → '1h 0m').
- **Targeted 8f75a10f:** `cargo test -p agenttrace-core --lib --test idless_reemission_dedup --test generic_lane_role_disclosure --test fmt_duration_carry --test usage_accounting_truthfulness --test discovery_contract` → 6 suites 358/0, rc0; clippy 0; fmt rc0; porcelain sha `12ac9cd3…` and diff sha `385f0c14…` byte-identical pre==post.
- **Full e77dbf79:** ci.yml full-job push lane + deny job mirrored lane-for-lane, CI step order, commands verbatim (dispatch `full_command` empty — ci.yml is the command authority); 20 executed lanes rc0 + 2 documented conditional skips (MSRV schedule/dispatch-only; TUI real-smoke `vars.AGENTTRACE_TUI_REAL_DIR` unset); 03-tests 40 result lines **691/0**; entrypoints 44/0; all 8 scripts/ci gates; ruby -c; npm 11/0; plugin-version; bash -n/sh -n; locked-cargo; `cargo deny --all-features check` all-ok; supplemental `cargo test --workspace --all-features` 691/0 across 40 lines; tree immobility porcelain sha `eaafdd18…` + diff sha `385f0c14…` pre==post; `AGENTTRACE_CI_OUT` redirected to delegate scratch (zero repo-root residue).
- **Digest lineage:** `validation:v1:714f807c050a27f7ca71ce060fcd1e0f27e8fdea36f7eb522c78dc903fd0c666` declared VERBATIM by both validation turns (run-only turns; nothing executable changed after implement) and re-derived byte-identical with the engine's own `validation_policy.validation_digest("5ed9ebcc…", worktree)` before and after the battery. ROADMAP.md and docs/** sit outside the digest's executable set — compound cannot move it; the commit gate may re-declare this token.

## Fleet / id landscape (live census at compound, 2026-10-09)

| claim home | ceiling |
|---|---|
| uncommitted worktree diffs (fleet) | rm-904 (run-075d70c67abf); then 902 (6b369f2f), 899 (33b7b7b5), 895 (cfa48339), 891 (ac3ac300), 873 (2a5cdb9e), 863 (5eb82325) |
| spool patches/postimages | rm-905 (5e151fe6's next-free declaration; minted spool max rm-904) |
| landed origin/master | def-max rm-859 @3268404, prose citations to rm-899 (advanced past prioritize's 190b706/rm-838 view) |
| **next free** | **rm-906** after a fresh live census — never reuse without re-sweeping |

This cycle's mint band rm-880..885 is campaign-local; renumber/reconcile BY TITLE at integration (880a7b9e discipline). The fleet has already moved past the band (siblings at 886..905 territory; landed wall def-max 859, prose citations to 899) — normal racetrack load, no collision with our def rows, title-twins to reconcile: rm-883-carry × 2a5cdb9e's rm-872 (rendering choice), rm-880's parser seam × fabd9fb8's uncommitted cost-truthfulness band (disjoint hunks per stewardship census: ours ≥:3878 + call site ~230 vs theirs 597–646/832–870/1461–1512/5806–5897).

**Status accounting after flips:** 307 def rows = 160 candidate / 78 implemented / 69 done, 0 duplicate ids.

## Deferred queue / next-cycle leads (recorded, not minted)

1. **rm-881** (60.0) Windows cache placement + non-unix private-write semantics + user_cache_dir dedup — rides rm-018's staged windows smoke; extend the landed rm-693 write protocol, don't fork it.
2. **rm-883 round4-dedup arm** — mcp.rs:371 reuses core lib.rs:2029 helper (small, pairs with any MCP lane work; watch run 91c7faf5's in-flight MCP reland unit).
3. **rm-884** (44.0) MCP stdio unbounded-line cap (LOW severity, stdio trust boundary).
4. **rm-885** (30.0) SVG usage share-card — demand-gated tier-2.
5. Wall deferrals: rm-164 (80.0 tier two-source refresh; rider added this cycle), rm-042 (68.0 windows/tz port).
6. Watches: upstream 3rd quiet check (15ed07f, v0.10.1, issues {278,259,237,236,103}); serde_json 1.0.150 float-parse radar; MCP draft stable 2026-07-28; npm/vendor versions flat; crates.io `-core`/`-tui` names still free.

## Commit-gate seams

(a) CHANGELOG Unreleased Fixed bullets for rm-880 / rm-882 / rm-883-carry are NOT in the delta — add at commit (254b2417 / 3ec6cec0 precedent).
(b) Stage the 3 new test files + 5 fixture files explicitly — `git commit -am` drops untracked.
(c) ROADMAP.md lands as the one worktree diff (chain provenance in the spool); done-flips for rm-880/rm-882 (+ rm-883 half note) flip at the gate.
(d) Title-twin reconciliation at integration: rm-883-carry × 2a5cdb9e rm-872 (pick one rendering — '1h 0m' vs '59m 59.8s'); rm-880/rm-882 parser+lib hunks disjoint from fabd9fb8/3f6b86bc/a2abf1a8 bands per the stewardship census.
(e) Final validation may re-declare `validation:v1:714f807c…c666` from the full_tests record (re-derived pre+post battery; compound's md-only delta cannot move it).

## Lessons (re-confirmed, no new prevention doc warranted)

- **ci-mirror rule (4th fleet confirmation):** empty dispatch `full_command` ⇒ ci.yml is the authority — including the conditional skips (MSRV, TUI real-smoke) and `AGENTTRACE_CI_OUT` redirection. The lane-22 stumble (`cargo deny check --all-features` rc=2) re-confirmed rule 5 of `docs/solutions/workflow-issues/empty-full-command-mirrors-ciyml-lane-for-lane.md` — **consult the lane family's prevention doc BEFORE writing the driver**; it would have saved the rc=2 round-trip.
- **Stewardship fresh-pinning pays:** the stewardship phase re-pinned the rm-880 fold at parser.rs:3889/:3925–3945 after the assess-time anchors had drifted; implement hit the seam first try with zero re-anchoring.
- **RED-FIRST both directions:** the fixture pair (re-emission folds / distinct sums) is what makes rm-880's discriminator observable, not just the happy path.
- PoC fixtures copied VERBATIM from the research/assess scratch into `tests/fixtures/` (copy, never retype) — the fixture is the oracle.

## Adoption forensics (attempt 0f17c189d41744c5a68756785872166f)

Prior-attempt forensics: 6d6dfb65 died of a provider-family infra failure mid-turn (event log: turn 1 finished 02:07:30Z, turn 2 reaped 03:36Z, `session_reaped`/`failed`; typed envelope never written). Durable trail: the ROADMAP compound banner + 8 dated riders + the rm-880..885 mint band applied in-tree with EXECUTED riders/flips, plus this record. 0f17c189 adopted the work after line-by-line verification — read-only, zero test execution, per the compound contract.

Verification legs (all reproduced fresh):
1. **Baselines:** implement-only porcelain (12 lines) → sha256 `eaafdd1883064948…` and md5 `12ac9cd3c6582769a5fd3fbdcd4a113e` byte-identical to the targeted/full records (the "13 entries / 6 fixtures" envelope prose was off by one; the cryptographic baselines are authoritative); `git diff -- crates` → sha256 `385f0c1454440e8f…` — the executable delta is immobile since full_tests.
2. **Roadmap chain:** `git show HEAD:ROADMAP.md` == spool `ROADMAP-orig.md`; worktree ROADMAP == spool `ROADMAP-after.md` + exactly the compound additions (banner, riders, EXECUTED bullets, 2 candidate→implemented flips) — diff-filtered, nothing else.
3. **Census re-count:** 307 def rows = 160 candidate / 78 implemented / 69 done, 0 duplicate ids, managed footer last.
4. **Log-derived claims re-counted from surviving logs:** targeted 6 result lines / 358 / 0; full LANE 03 = 40 result lines / 691 / 0; workspace artifact 40 / 691 / 0; lane 22 rc=2 argument-position error → corrected `cargo deny --all-features check` rc=0; `digest-pre.txt` == `digest-post.txt` == `validation:v1:714f807c050a27f7ca71ce060fcd1e0f27e8fdea36f7eb522c78dc903fd0c666`.
5. **Code claims re-read in-tree:** `idless_reemission_folds` (parser.rs call site :230-237, state :3904/:3910), `role_unclassified` census (lib.rs :813/:837), fmt_duration carry arm (lib.rs :2067-2073, `(minutes*10).round() >= 600.0` → `{hours}h 0m`), `#[test]` counts 3+3+2 = 8 across the three new files; round4 twin real (agenttrace-cli mcp.rs ~:369-372 vs core lib.rs:2055 — the row's `lib.rs:2029` is the mint-time anchor, shifted +26 by the rm-882 mapping additions above it).
6. **Research figures recomputed:** models.dev 226 providers / 8,461 model entries re-derived from the preserved raw input `inputs/modelsdev.json`; 4,503→4,504 and the 119-union (vs the non-reproducing 529) confirmed in the research dossier; zh-README 191/434 confirmed in the assessment dossier.
7. **Fleet census re-run live (post-fetch):** all seven listed worktree ceilings reproduced exactly (rm-904 run-075d70c6, 902 6b369f2f, 899 33b7b7b5, 895 cfa48339, 891 ac3ac300, 873 2a5cdb9e, 863 5eb82325; every other dirty worktree ≤ rm-856); spool `rm-905` = 5e151fe6's *next-free declaration* ("…rm-902 (7d65b322) — next free rm-905"), minted spool max rm-904; landed origin/master @3268404 (02:39Z) def-max **rm-859**.

Corrections applied by 0f17c189 (pre-adoption confabulations, the #17495 class):
- codeburn **#1597** rider said "2026-10-08 updates" — dossier's 10-08 wave was #1640/#1687/#1686; #1597's last activity is 2026-10-06. Fixed on-row.
- "landed origin/master **def-max rm-899**" — prose-contaminated max: rm-899 appears on origin/master exactly once, inside run 6cb2756a's 10-08 sweep-record prose ("found 0 def hits rm-839..rm-899"); true def-max rm-859. Fixed in banner + table above.
- "spool frontier rm-905" kept but clarified — it is a next-free declaration, not a minted row; minted spool max rm-904. "Next free rm-906" stands (conservative over the highest claimed token).
- Investigated and NOT an issue: the "provider-dead 375973306" fragment belongs to run 5417681937ae's historical banner (its event log carries 4 mentions of that run, none of this one); this cycle's banner never cited it.

Envelope emitted by 0f17c189 for THIS work order only.

## Review fix (2026-10-09, attempt 9037d0c1d41744c5a68756785872166f, review e9518245 verdict NEEDS_CHANGES)

All four findings dispositioned; the review's embedded "FIX PLAN" block was self-flagged as NOT-a-reviewer-plan and was disregarded (it said to apply only findings 1+3 and skip re-running tests — contradicting the review-fix work order's "fix every actionable finding, rerun affected validation").

- F1 (medium, generic_tool_typed red gap) FIXED red-first: the rm-882 type->role mapping now also classifies tool-shaped keys (tool/tools/toolUse/tool_use/toolResult/toolResult -> role "tool") so analyze()'s tool arm counts them as tool activity instead of minting role_unclassified. New test generic_tool_typed_lines_are_classified_not_unclassified + fixture fixtures/generic/generic-tool-role.jsonl (camel-case keys deliberately: exact "tool_use" fires the claude_code probe). RED proven on F1-stripped source (left: Some(2) right: None); #17487 red-first discipline honored for the corrected expectation: the FINAL test version was re-proven red by temp arm-revert, restored md5-identical (9e3b529a4f28a852ab65f93a28e2379f). Note: tool_calls_ok is NOT asserted — analyze() clamps ok to tool_calls_total - fail (lib.rs :1379), call-anchored by design; tool_results + provenance reported_or_inferred carry the activity.
- F2 (low, disclosure-channel doc gap) RESOLVED on the wall: dated rider on the rm-882 row mapping where the census IS user-visible (agenttrace -f json metrics.line_skips; doctor.rs:28-38 aggregates parse-time disclosures incl. line_skips since rm-526) and why the statusline HOST report must not carry it (data source = CapturedStatusline payloads, statusline_insights statusline.rs:508; host contract never-fails/never-blocks, statusline.rs:135/:15). No code change, per the review's either/or.
- F3 (medium, id-less model-equality fold) NON-ACTIONABLE on this tree: the quoted construct ("fold.take().unwrap_or(&last_assistant_model)") and its cited anchors do not exist — parser.rs has zero `.take()`; last_idless_model is a plain String local (parser.rs :3898/:3923); model equality already happens on the PREVIOUS state value, not a moved-out one (match at :3955-3961). The reviewer's .fixed drafts are byte-identical to the worktree (no drafted fix). Recorded as stale-anchor prose, no code change.
- F4 (low, drifted citations) FIXED on the wall: mcp.rs:371 -> live twin block mcp.rs:369-372; core lib.rs:2029 -> live :2055 (post-rm-882-mapping shift); three sites drift-corrected (banner :17, rm-883 signals, rm-883 HALF-EXECUTED rider) with an explicit drift-note.

Validation (emission-time): I changed an executable surface (lib.rs mapping + test + fixture, all crates/** which the engine classifier treats as digest-immobile); affected command re-run: cargo test -p agenttrace-core = 29 suites 495/0 (494 review baseline + the new test); cargo clippy --workspace --all-features --all-targets 0 warnings; cargo fmt clean (one cargo fmt applied to the new arm). Digest re-derived fresh post-change with the engine's validation_policy: validation:v1:714f807c050a27f7ca71ce060fcd1e0f27e8fdea36f7eb522c78dc903fd0c666 — byte-identical to the dispatch declaration and to the pre-fix derivation.


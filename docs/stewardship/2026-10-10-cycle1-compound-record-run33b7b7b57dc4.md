# Cycle 1 compound record — run 33b7b7b57dc4 (repository-maintenance 2ba7d07a, cycle 1)

- **Date:** 2026-10-10 (cycle work 2026-10-09 → 10-10) · **Attempt:** 86abc98923834767ba94f8cd43301fa5 · **Base:** 9c3c599c0389e75f3ea7f50d9f417a0fff3f343b (HEAD unchanged all cycle)
- **Theme:** 'truthful accounting, honest surfaces' — rm-617 LEAD (codex rollout reasoning double-add) + rm-897 (markdown formula guard) + rm-898 (bounded statusline reads) + rm-899 (aider DST-ambiguous starts)
- **Inputs consumed (pre-review, per the compound contract):** assess b0c5221b, research e86cb0f (dossier pass 14), roadmap 862369f8, prioritize 57892aa7, stewardship 452e32eb, implement 9d3480ed, targeted_tests 4fde148f, full_tests 038a77e1. No test execution at compound.

## State at compound

Uncommitted batch at base 9c3c599: **11 M + 1 ??** (+544/−38; ROADMAP.md NOT among them). This phase adds two untracked docs (this record + one solutions doc) and ships the roadmap compound layer as a **spool patch chain link** — the worktree ROADMAP.md stays **pristine** (md5 `c1a3c28b738bc06eee901d071c1fcad0`, byte-verified this pass). Porcelain at record time: 12 lines (11 M + 1 ??); at this record's own close the census was 14 (this record + the solutions doc joined as ??) — corrected here at review-fix ed90e5ec (review 9fb0a017 F5d), and after the review-fix the batch is 12 M-tracked-modified paths + the review-fix edits riding them + 3 ?? docs.

## Cycle outcome

| Phase | Attempt | Outcome |
|---|---|---|
| assess | b0c5221b | 4 fresh landings since wall 3d038dc read line-by-line, well-engineered; full suite 743/0 @ 9c3c599 (`--workspace --all-features`, 37 binaries); hostile-branch PoC corpus (15 sessions) at /tmp/at-assess-33b7 |
| research | e86cb0f | pass 14: 20+ live probes; fork frozen 5th consecutive check; LiteLLM 3,743 vs vendored 3,099 (650 new keys, claude-haiku-5-5 live upstream, absent vendored); MCP spec 2026-07-28 stable vs our 2025-06-18 pin; semconv-genai tags 5th zero check |
| roadmap | 862369f8 | mints rm-897–rm-901 campaign-local (fleet frontier rm-896 held by unlanded 0264abd8); spool patch, worktree wall untouched; fleet census banner records the off-wall owners (rm-895/880/780-band/797/695) |
| prioritize | 57892aa7 | 169 candidate rows adjudicated ≥76.0 against live code at 9c3c599; batch selected (LEAD 88.0); rm-618 adjudicated STALE (landed as rm-602); alternates rm-619/rm-709/rm-901 |
| stewardship | 452e32eb | stewardship_request JSON + companion; no Git topology chosen (conductor owns it) |
| implement | 9d3480ed | full batch implemented; **ADOPTED provider-dead e278e52c's in-tree work** after hunk-by-hunk verification + reds re-proven live (see adoption doc below); schema 41→43 |
| targeted_tests | 4fde148f | fmt/clippy/targeted suites/release build/docs gate all rc0 (292/0 over 5 result lines + TUI 54/0 — corrected at review-fix ed90e5ec per review 9fb0a017 F5c: leg-core.log's ok-lines sum 269+2+4+12+5 = 292 over 5 result lines, not 297 over 6); prior attempt d1bce9f5 verified dead-at-start (23 s, zero tools) — nothing to adopt |
| full_tests | 038a77e1 | ci.yml push lanes verbatim, **23/23 rc0**; test-trio 749/0 + entrypoints 46/0; census +6 #[test] reconciles 743+6=749 exactly; digest validation:v1:0abdee67… re-derived byte-exact |
| compound | 86abc989 | this record; roadmap chain link 3; solutions doc; no test execution |

## What shipped (grounded in the uncommitted CHANGELOG.md diff + implement red logs)

1. **rm-617** — rollout record arm: `token_usage_record` rows treat `reasoning_output_tokens` as the BREAKDOWN it is (OpenAI Responses semantics; upstream #312: 1,166/1,166 rollout files over-counted, +64% compaction lane); output bills as reported, reasoning rides its own `reasoning_tokens` line (`tokens_reasoning`, CU-20); qwen's genuinely-separate thinking stays folded; verdict pin 720→600 output / 0→120 reasoning (red: 420≠300 and 620≠500 vs reverted fold); fixture `tests/fixtures/usage-accounting/codex-rollout-compaction-reasoning.jsonl`.
2. **rm-897** — `markdown_cell` applies the rm-540 CSV formula-guard dialect (leading-`'` neutralizer for `= + - @` tab CR; numerics stay bare); `is_formula_introducer` mirrored in both crates' tests; HTML cells untouched (red: hostile cells unneutralized; over-guard red on the numeric exemption).
3. **rm-898** — `STATUSLINE_READ_CAP_BYTES` 10 MiB tail-capped reads at all three sites; torn head never leaks a capture; `bytes` = real size, `lines` = retained tail; disclosed via `capped_away_bytes` (json) + `head truncated, N of M bytes unread` (text) (reds: 160000→126334; E0425/E0609 disclosure-vs-HEAD compile fail (E-codes corrected at review-fix ed90e5ec per review 9fb0a017 F5a — red3b.log shows E0425 [const] and E0609 [field], the earlier E0433/E0599 transcription was wrong)).
4. **rm-899** — `aider_time` `.single()`→`LocalResult::earliest()`; never empty start for parseable wall clocks; spring-forward gap still yields none; TZ=America/New_York pinned unit, chrono tz_info orders the pair by offset → −05:00 pin (red: '' vs `2026-11-01T01:30:00-05:00`).
5. **Schema 41→43** (jointly rm-617+rm-899, rm-230 convention): fleet master's 42 @181630e is NOT in this base, so 43 ships directly above the **landed** ceiling; held unique below the in-flight sibling frontier (uncommitted 49/50) per the moving-wall rule. Same-unit set moved together: const + ladder rung, rm-710 occurrence-contract oracle re-based 41→43 (red-proven: const alone failed the oracle), governance-guide schema sentence; docs gate rc0.

## Roadmap accounting (spool chain, worktree wall pristine)

| Link | Artifact | sha256 | postimage |
|---|---|---|---|
| 1 | 862369f8 `ROADMAP.patch` (mints) | `94423980e0820cab2b0ede3460af873f11e9fb7e08c185122e18084fe55b577e` | md5 `bac76f5f6fa8810eb17be0188e62fde9` |
| 2 | 57892aa7 `ROADMAP-designation.patch` | `5b3f65ba9621dd1f779afdfb18c298484220278df12c99efd140e020d3212557` | md5 `954836f602d19c6777d58a56e8206a55` |
| 3 | **86abc989 `ROADMAP.compound.patch` (this phase)** | `24eb78f778b92139…` | md5 `3c08517c29057d7ad227dec255ad51b5` / sha256 `366a64bb5be4058ce532df4e9b5a80ca535cdefbb8e23472cb4fd9640eb4b176` |

- Link 3 content: rm-617/897/898/899 `status: candidate` → `implemented` + dated implemented/validated bullets (validation **transcribed** from targeted 4fde148f + full 038a77e1, explicitly "NOT re-run at compound"); compound banner above the fleet-census banner in the mints section naming the chain and this record.
- **Chain proven this pass, from the pristine wall:** cp worktree ROADMAP.md (md5 `c1a3c28b738bc06eee901d071c1fcad0`) → apply link1 → byte-identical to link1 postimage → apply link2 → byte-identical to link2 postimage → apply link3 → byte-identical to link3 postimage (3422 lines, footer last line); link3 re-apply rejected; reverse applies (roundtrip). `/tmp/at-fix-ed90/chain/ (re-proof at review-fix; original /tmp/at-compound-86ab/chain2/)`.
- Postimage census: 328 backticked id rows, **0 duplicate ids**; inline status 165 candidate / 91 implemented / 72 done (+14 old-grammar `- status:` rows: 11/2/1) — the only status changes designated→compound are the four flips (status-string stream diff verified).
- **Done-flips (→done) and the rm-618 stale-annotation (fix landed as rm-602 at qwen_usage :2995-3009 — do not re-implement) are RESERVED to the commit gate.**

## Dead-attempt ledger (six deaths, one adoption; all from events/*.jsonl)

| Attempt | Phase | Window (UTC) | Disposition |
|---|---|---|---|
| 92f171bc | implement | 10-09 23:53→00:08 | provider death mid-turn; no durable work found |
| cfbce78c | implement | 10-09 03:11→03:25 | provider death; no durable edits |
| e278e52c | implement | 10-09 03:35→03:51 | provider death **after writing in-tree work** (5 M + 1 ??, 232-line diff) — **ADOPTED** by 9d3480ed after hunk-by-hunk verification + live re-proof of every red; see the adoption doc |
| c8987f02 | targeted_tests | 10-09 (retry-stacked session) | provider death; nothing to adopt |
| 6262bdab | targeted_tests | 10-09 (further retry) | provider death; nothing to adopt |
| d1bce9f5 | targeted_tests | 10-09, 23 s total | dead-at-start (turn_started → reaped, msg_count 2, zero tools); nothing to adopt — forensics recorded by 4fde148f |

Provider instability clustered on 10-09 late night; every successor declared its disposition explicitly (nothing silently ignored).

## Prevention rules & lessons (this cycle's additions)

1. **Provider-dead attempts with in-tree work: adopt only through full re-verification.** New solutions doc `docs/solutions/workflow-issues/adopting-provider-dead-attempts-uncommitted-code-reverify-hunks-reds-and-narrative.md` (extends the reaped-attempts rule doc and the 2023f222 adoption nuance from ROADMAP application to full code deltas; fleets rule: the verification must cover NARRATIVE claims too — numbers, line refs, mechanism attributions).
2. **Empty dispatch `full_command` → ci.yml push lanes, verbatim.** Re-validated this run (23 lanes incl. the schema-wall docs gate). Lane set must be re-derived from the worktree's ci.yml, not copied from a prior runner.
3. **Digest honesty at compound:** docs/ROADMAP edits are digest-immobile, but the declared digest was still **re-derived live** at full_tests via direct `validation_policy.validation_digest` import (zero repo writes; full 40-hex base sha required). The fold gate's re-derivation will match the declared `validation:v1:0abdee67…`.
4. **Shared-host CPU contention distorts wall-times, not outcomes:** a sibling worktree's `journal_appends_survive_parallel_compaction` test accumulated 54+ min CPU (observed during full_tests 038a77e1), pushing doctor-walking gates into D-state (output-contract 425 s, real-cli-smoke 805 s) — both still rc0. Candidate for a fleet reaper / test-runtime look (see residuals).

## Residuals / not done (honest)

- MSRV floor and TUI real-data smoke lanes are event/var-gated out of the push suite and were NOT exercised this cycle (TUI real-smoke's own gate variable is unset).
- rm-618 stale flip/annotation reserved to the commit gate (its acceptance is already served by landed rm-602).
- The sibling runaway-test observation is un-investigated (host contention may fully explain it; verify before claiming a defect).
- Nothing from the NOT-selected set was touched: rm-421/rm-251 (too large), rm-195 (decision), rm-448 (wave port), rm-239 (collides unlanded rm-819), rm-176 (needs scheduled-workflow green run), rm-043/044 (dep majors).

## Ranked next-cycle candidates (pre-review context; next assess adjudicates)

1. **rm-176 pricing-snapshot refresh** (row carries the pricing-freshness lane here): research pass 14 measured 3,743 upstream vs 3,099 vendored @2026-10-04 (650 new / 2 removed; claude-haiku-5-5 live upstream at {input 0.1, output 0.5}/Mtok — absent vendored; 89 new gemini keys). Blocker per the designation: needs a scheduled-workflow green run. LOW risk (data refresh via `scripts/pricing/update-snapshot.sh`), HIGH coherence with this cycle's truthfulness theme.
2. **rm-895 overview-honesty caps** (by_* group-table per-surface caps + truncation marker + tie-breaks; assess F1 live PoC 8-text/12-html rows): **owned by unlanded 0264abd8 (run cfa483394fcf) — dedupe by title at its landing before selecting**; corroborating evidence already lives in rm-388's refresh line.
3. **MCP spec 2026-07-28 refresh band** (stateless no-initialize, server/discover, subscriptions/listen, ping/logging/setLevel removed vs our mcp.rs:97 pin 2025-06-18): fresh landing evidence for the unlanded rm-780/781/782/789 band (run 749cd298) + rm-797 (run a5732ff5) — off-wall owners; renumber risk.
4. **rm-619 (70.0, first alternate)**: opencode `tokens.reasoning` at parser.rs :5049-5056 (`add_opencode_tokens` has no reasoning surface) — same truthfulness family as rm-617, small.
5. **rm-841 codeburn PR 1640** demand ref still unserved (open upstream, updated 2026-10-08T01:10Z).
6. Watch items (cheap re-checks): fork tip 15ed07f2 @2026-10-06 frozen 5×; semconv-genai tags still zero (rm-229 gate closed; rm-006 absent from this wall); waste two-tier severity → unlanded rm-695 (3ee2af04).

## Integration handoff (commit gate)

1. Apply the roadmap chain IN ORDER to the pristine worktree ROADMAP.md: 862369f8 → 57892aa7 → 86abc989 (hashes above; each step byte-verifies against its postimage).
2. Explicit adds: the 12 batch paths (11 M + fixture `??`) + `docs/stewardship/2026-10-10-cycle1-compound-record-run33b7b7b57dc4.md` + `docs/solutions/workflow-issues/adopting-provider-dead-attempts-uncommitted-code-reverify-hunks-reds-and-narrative.md`. No other paths.
3. At the gate: flip rm-617/897/898/899 →done (reserved), annotate/flip STALE rm-618 (fix landed as rm-602 — reconcile by title, do not re-implement), re-run the schema-ceiling census (moving wall: max(observed)+1 only if minting; 43 must remain unique — re-check landed origin + sibling uncommitted consts), re-derive the validation digest (docs/ROADMAP are immobile under the classifier; base = full 40-hex).
4. Campaign-local numerals rm-897–rm-901 renumber by TITLE at integration (880a7b9e discipline); the retired-numeral discipline keeps minted-but-superseded ids unused.

## Review-fix correction (2026-10-10, attempt ed90e5ec — independent-review 9fb0a017 round 1, NEEDS_CHANGES)

Round-1 verdict NEEDS_CHANGES with one integration blocker (F1), one narrative-truth defect (F2), and three annotation/figure items; fixed this pass, all dispositions:

- **F1 (high) — schema 43 collided with landed origin:** origin/master landed 43 (fabd9fb8, run fabd9fb8's rm-831 hostile-value work, 2026-10-09 16:55Z, descending from our base 9c3c599) AFTER our mint. Re-based to **44** at the review fix: session_cache.rs const+ladder (43 → 44 with the collision rung recorded), rm-710 oracle fn name/assert/message (red-proven at BOTH bumps: the const alone failed the oracle — left 44/right 43 before the oracle edit; left 43/right 42 at the first bump), governance-guide sentence (docs gate re-verified green), CHANGELOG sentence rewritten to the two-bump story. Census at the fix: origin 43 (landed), siblings max uncommitted 50 (run-5eb82325ebea) / 49 (run-2a5cdb9e), spool claims none at 44 — 44 unique below the frontier.
- **F2 (medium) — provably-dead UTF-8 walk:** statusline.rs:452's guard `e.valid_up_to() == buf.len()` is unsatisfiable for a from_utf8 error on a tail buffer (the review's probe: torn-tail 1 < len 3, head-torn 0, valid_up_to==len 0 hits), so a cut landing mid-multibyte leaked the raw cut through the lossy fallback and "UTF-8-safe boundary walk" prose was false in four artifacts. FIXED LIVE: the advance predicate is now `valid_up_to() == 0 && buf[0] is a continuation byte && advances_left > 0` bounded at 3 advances (UTF-8 max width); red-first at the fix — a 10 MiB+3-byte journal with the cut one byte into 中 (E4 B8 AD) pins capped_away_bytes 876 (boundary-exact) vs the leaked raw cut 874 (red log /tmp/at-fix-ed90/logs/red-f2-torn-utf8.log, GREEN in green-run2); CHANGELOG/record/roadmap wording corrected to describe the real predicate.
- **F3 (low) — introducer-set prose:** "=, +, -, @, tab, or CR" (CHANGELOG + roadmap rm-897 bullet) was wrong — the shipped set is =, +, -, @ plus fullwidth twins （＝＋＠－ U+FF1D/U+FF0B/U+FF20/U+FF0D), identical to csv_export.rs:1197-1207. Corrected in CHANGELOG and the regenerated roadmap link; code/tests were always right.
- **F4 (test) — DST gap pin:** VERIFIED ALREADY PRESENT at parser.rs:8219-8223 (`aider_time("2026-03-08 02:30:00") == ""` with the no-invented-instant message) — the review's "nothing pins it" was itself a miss; disposition = tightened the assert message to cite the CHANGELOG sentence and name the review check (message-only change; no behavior).
- **F5 (low) — figure transcription:** (a) red3b compile errors are E0425/E0609, not E0433/E0599 (fixed here + roadmap link); (b) the usage red log is red1-integration.log, not red1-usage.log (the implement-phase typed result's own naming; left in the consumed envelope, corrected everywhere I own); (c) targeted leg-core ok-lines sum to **292 over 5** result lines (269+2+4+12+5), not 297/6 — fixed here + roadmap link (the 297 figure was the implement phase's post-bump lib re-run sum, a different leg); (d) this record's write-time porcelain 12 vs close 14 (see State section, corrected in place).
- **F6 (low) — opencode `tokens.reasoning`:** out of scope for this review fix (a new lane, parser.rs:5049-5056 add_opencode_tokens); recorded as the top-ranked next-cycle candidate beside rm-619 — see Ranked candidates note at the end of this section.
- **Roadmap chain regenerated:** link 3 rebuilt with F1/F2/F3/F5 corrections + a REVIEW-FIX banner line; new pins sha256 `{p3sha}` / postimage md5 `{po3md5}` (this section's pins are computed by the writing command itself); chain re-proven pristine→link1→link2→link3 at /tmp/at-fix-ed90/chain (byte-identical per link, re-apply rejected, reverse applies).
- **Validation at the fix (targeted per the dispatch budget):** fmt/clippy rc0, the new torn-UTF-8 test green, aider_time test green (message-only change), all four integration suites green, TUI 54/0, release build rc0, docs gate rc0 at const 44 — logs /tmp/at-fix-ed90/logs/. Digest: executable surfaces changed this fix → re-derived and declared in the fix's validation_evidence (NOT the dispatch-time digest).

## Verification

- Chain: `cd /tmp/at-compound-86ab/chain2` (or rebuild: pristine cp → 3× `git apply` with `cmp` per link) — byte-identical at every step; `git apply --check -R` link3 passes, forward re-apply rejected.
- Flips: `grep -n '| status: implemented' <postimage>` → the four rows at :3242/:3378/:3388/:3398; status-string diff vs designated = exactly 4 changes.
- This record's pins were emitted by the writing script itself (`sha256sum`-equivalent inline; #17555 discipline), not hand-typed; the script asserted the designated postimage md5 and the pristine wall md5 before writing.
- Batch evidence: /tmp/at-impl-9d34 (reds/greens/probes), targeted 4fde148f + full 038a77e1 logs under their spool scratches.


## Review-fix correction 2 (2026-10-10, attempt b5b9c6fc)

The round-1 review-fix fold rejected ed90e5ec's evidence as targeted-scoped where the run's uncovered risk-relevant delta (the review fix's own executable changes, which had never been inside a full suite) demands scope `full` (KTD5/KTD6). This correction supersedes correction 1 where they overlap; later gates prefer this section.

1. **Second schema re-base 44 → 45 — the F1 defect class rematerialized.** A fresh moving-wall census at this attempt found origin/master had LANDED 44 under `fb1addd56503` (merge b917ff7, 2026-10-09 18:41:12Z, landing 03dc231, descending from this base 9c3c599) — concurrent with ed90e5ec's 44 mint (~18:46Z), so the 44 rung re-collided exactly as 43 had. Live census: origin 44 landed; sibling worktrees uncommitted 44 (run-472afca26bca), 49 (run-2a5cdb9e), 51 (run-0a55a397eecc); no spool patch or postimage claims 45; 45 is the lowest rung unique above the landed ceiling. Same-unit set moved together: `session_cache.rs` const + ladder rung, the rm-710 oracle's fn name/assert/both messages, the governance-guide schema sentence, the CHANGELOG bump story, and roadmap chain link 3 (regenerated below). Red-proven twice: const-alone against the still-44 oracle (left 45 / right 44, usage_occurrence_contract.rs:111) and the corrected 45-oracle against the unfixed const-44 source (left 44 / right 45); green 5/0 after; `cargo fmt --check` clean.
2. **Roadmap chain link 3 regenerated a second time** (ROADMAP.compound.patch + postimage replaced at 86abc989-scratch; the worktree ROADMAP.md stayed pristine md5 c1a3c28b throughout): the banner carries a REVIEW-FIX-2 sentence and the rm-617/rm-899 validated bullets now ride rung 45 with the two-collision story. Chain re-proven end-to-end in an isolated workdir: pristine copy + 862369f8 ROADMAP.patch → byte-identical to its postimage → + 57892aa7 designation → byte-identical (md5 954836f602d19c6777d58a56e8206a55) → + regenerated link 3 → byte-identical to the new postimage; double-apply of link 3 correctly rejected. The link-3 pins quoted in correction 1 describe the pre-b5b9c6fc generation and are superseded by the pins at the foot of this section.
3. **FULL suite over the exact post-fix tree — the scope the round-1 fold demanded.** The ci.yml push lane set (lane list byte-identical to full_tests 038a77e1): SUMMARY_RC=0, all 22 lanes rc=0; test-trio machine-sum 750 passed / 0 failed — +1 over 038a77e1's 749, the +1 being ed90e5ec's torn-multibyte statusline test previously covered only by targeted evidence, i.e. precisely the uncovered risk-relevant delta the fold named; test-entry 46/0; the docs gate green against the schema-45 governance sentence; clippy -D warnings and cargo-deny green. Post-suite hygiene: porcelain set and whole-diff sha256 byte-identical pre/post suite; ROADMAP.md md5 c1a3c28b; HEAD 9c3c599 unchanged.
4. **Validation digest** re-derived over the post-fix tree via the zero-side-effect import route: validation:v1:0abdee67bd98d1e4ced6ec6604714cf84f425f53d0607b2b6041928a698ecafa — identical to the dispatch digest (the changed-path set is unchanged; this fix's edits touch only paths already in the 11 M set).
5. **Incident disclosure (this phase's own).** An early chain-proof command in this attempt mistakenly ran `git -C <worktree> apply`, landing chain link 1 in the worktree ROADMAP.md (detected via porcelain 15 and md5 bac76f5f == the 862369f8 postimage); it was restored byte-identical to HEAD immediately (md5 c1a3c28b, `git show HEAD:ROADMAP.md | cmp` clean, porcelain back to the 14-line set) and the chain re-proven in the isolated workdir of item 2. No other file was touched by the incident. Also corrected for the record: ed90e5ec's evidence line "grep 'UTF-8-safe boundary walk' → zero" was imprecise — two in-tree CHANGELOG/record mentions of the phrase remain, both being the corrected honest text describing the now-live walk; the absence-of-phrase reading was never what made review F2 fixed.

Chain pins (computed by command at the moment of this write):
- ROADMAP.compound.patch sha256 `42f1b7c978a6af7ff50cfbb55f46a5e0515fbde65a71368e9a67e61dc03deb98` / md5 `950cede1f9a77775ce8be5f2a21a2d20`
- ROADMAP.compound.postimage.md sha256 `65bc311910c5849163f48f96a7acd49087491c66da63814530f6ba3c88c4484d` / md5 `3ebe57353882d9a41e5a7884268517bc`
- pristine wall ROADMAP.md md5 c1a3c28b738bc06eee901d071c1fcad0 (worktree, HEAD 9c3c599)

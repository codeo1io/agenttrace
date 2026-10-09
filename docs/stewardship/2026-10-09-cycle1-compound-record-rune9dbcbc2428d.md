# Cycle 1 compound record — run e9dbcbc2428d4bbeb7e5a3d541c81d1a

- repository-maintenance 1754b828247449b8b59aa46b775b5d72, cycle 1, compound attempt 6e2e955cdbf643e8ab8e9099ad7e9f86.
- Base: worktree run-e9dbcbc2428d-e9dbcbc2 @ `65f9f635113716412205f992252be63dcf2920b9`. Porcelain at compound dispatch = the implement delta exactly (8 tracked files modified, +328/−21, zero untracked). ROADMAP.md is untouched in-tree this whole run — the roadmap rides the spool patch chain; the commit gate lands it.
- Phase contract honored: **no test or validation command was executed at compound** — the targeted (3a54a68d) and full (e39775f7) outcomes are consumed as recorded evidence.
- Skill disclosure: no ce-* compound-engineering skill is installed in this session (installed set = agent-reach only); compound discipline is applied from recorded fleet precedent (5cf79d29 spool-chain delivery, bbde21568cd4 compound shape, rm-012 done-flip reservation).

## Prior-attempt check (this action)

No prior compound attempt exists: no `delegate/6e2e955c*.json` envelope before this phase, no `6e2e955c*-scratch/` directory, and the event log under `events/` is this attempt's own. Clean slate — nothing to adopt, nothing to declare as redo.

Run-level dispositions already settled in their own phases (recorded here for the lineage walk): assess 6f624f65 and roadmap dc376f28 were provider-reaped with zero durable output → forensics-then-redo-and-declare (2/2 held). The first full_tests attempt (before e39775f7) was fold-rejected **after completion** on a declaration defect — a command-string mismatch — while the suite itself was green (two rc0 gate envelopes on file); the retry re-ran the suite from scratch and declared the dispatch full_command byte-for-byte. That defect class is now a prevention doc (below).

## What compounded

1. **Wall flips** — rm-857 (LEAD, waste cost-basis propagation) and rm-858 (rider, doctor repair-hint path quoting) flipped `candidate → implemented` with dated EXECUTED bullets carrying implementation mechanics, red-first pins, and the recorded validation counts. Done-flips stay reserved to the commit gate (rm-012 convention). **Zero mints at compound.**
2. **Compound banner** — inserted at the head of the banner stack, directly under `## Open items`, above this run's cycle-1 roadmap banner (newest-first convention). Carries: batch identity, consumed outcomes (not re-run), fleet lessons, prior-attempt dispositions, next-cycle lead pointer, and status accounting (def-row census — statuses read off the 300 `^- id:` rows — 158/66/76 → 156/66/78 after the flips; substring counts over ALL status tokens, which include 15 legacy-format rows, read 169/67/79 → 167/67/81; corrected to the def-row convention per review F4, see the review-fix addendum).
3. **Deliverable chain** — patch #3 `compound-delta-6e2e955c.patch` + postimage `ROADMAP.compound.md`, built by `build_compound_delta.py` with count==1 anchored replacements plus post-edit structural and def-row-census asserts. Chain-verified in a /tmp sandbox: pristine worktree ROADMAP.md → roadmap-delta-d0ecd354 → prioritize-delta-81b107b7 → compound-delta-6e2e955c == postimage, byte-exact (`cmp`). **Amended 2026-10-09 by review fix 9cf36060** (banner census corrected to the def-row convention + review-fix lineage rider): postimage is now sha256 `1f38568972deecaa3e76c3b263ac7786a257289ec2e970556353fcbd31057c31` (1459784 bytes), chain re-verified byte-exact in a fresh sandbox (/tmp/at-compound-sbx2); the original postimage was `b58c45a9…` (1458001 bytes).
4. **This cycle record** (untracked in-worktree) and **prevention doc** `docs/solutions/workflow-issues/conductor-validation-evidence-is-a-verbatim-contract.md` (untracked in-worktree).

## Lessons / prevention rules (this cycle's evidence)

1. **The typed validation_evidence fields are a verbatim contract.** The fold gate reads the structured JSON, not the gate envelope: a green suite with a mistyped `command` field is a rejected phase. Prevention doc minted; the fix pattern is programmatic equality-check of `command` vs the dispatch `full_command` before writing the result JSON.
2. **`changed_testable_surfaces=[]` on a `crates/<crate>/` delta means the classifier is blind**, not that nothing is testable (EXECUTABLE_PREFIXES covers only top-level `src/|lib/|tests/|scripts/|bench|.github/workflows`). Derive the real surfaces from the implement envelope + `git diff`, as this run's targeted phase did.
3. **`cargo -p` filters take the package NAME, not the directory name** (`agenttrace`, not `agenttrace-cli`): the mistype is rc101 "package ID specification did not match" with ZERO tests executed — never recordable as a pass.
4. **The local validation gate's envelope digest is a base=unknown derivation, never the engine token.** The engine digest is `validation_digest(<HEAD full sha>, worktree)`; crates/**+docs/** deltas leave it byte-identical, which is why the dispatch digest stayed the correct verbatim declaration across implement/targeted/full folds.
5. **Additive JSON disclosure needs no schema bump.** `agenttrace.waste.v1.loop_cost_basis` is a render-time disclosure field: additive, version untouched, no session-cache invalidation (contrast the rm-230 convention, which governs only totals-altering parser-semantics changes). The row records the decision.
6. **Dead-attempt triage held 2/2** (absent typed JSON + empty scratch + heartbeat-only event tail ⇒ provider reap ⇒ redo from scratch and declare) — plus the two new shapes this run added: an artifact-PRESENT mid-turn reap (adopt only after first-hand re-verification, per bbde21568cd4) and a fold-REJECTION of completed green work (fix the declaration defect on a fresh run; do not re-litigate the plan).
7. **Anchor edits verify post-edit STRUCTURE, not just the anchor match.** The first banner insertion passed count==1 anchoring AND a byte-exact chain `cmp` while silently duplicating the `## Open items` header — the postimage itself was wrong, so the chain proof was necessary-but-not-sufficient. Caught by reading the emitted patch hunks; the builder now asserts header-count invariance, single `## Open items`, banner-count 1, EXECUTED-count 2, and the exact banner/roadmap junction.

## Next-cycle leads (ranked, pre-review evidence only)

| # | Lead | P | Why now |
|---|------|---|---------|
| 1 | rm-859 per-event cwd attribution | 58.0 | Deferred FRESH by this cycle's prioritize (verify-first step recorded in the row); codeburn #1689 shipped same-day demand; first-wins gate at parser.rs:3855-3858 (claude_code) and :2028 (oh_my_pi header pass). |
| 2 | rm-421 compaction model across parsers | 90.0 | Top open priority on the wall; multi-surface, wants a dedicated cycle. |
| 3 | rm-617 + rm-618 (+ rm-251 R1 rider) | 88/90/79 | The #312-absorption accounting-truth batch — coupled rows, batch as one unit. |
| 4 | rm-232 cross-session dedupe | 86.0 | High-priority open row; re-verify PoCs at next base before scoping. |
| 5 | rm-588 second-tier pricing fields decision | 71.0 | C80 folded here; claude-haiku-5-5 30× live overestimate PoC ($0.0045 vs $0.00015); context-length tiered rates unrepresentable in the snapshot schema — the decision (second-tier fields vs disclosed boundary) is the work. |
| 6 | MCP 2026-07-28 spec currency (twin set) | — | Reconcile BY TITLE: rm-780/rm-789/rm-797/rm-823 (unlanded) + 3e6e5de0's committed-unpushed rm-840; new datums recorded in this run's roadmap banner (server/discover mandatory, resultType, -32002→-32602 already matching). |
| 7 | serde_json shortest-round-trip float family | — | rm-803 / rm-825-834; 1.0.151 release body confirms UNFIXED — pin posture holds, evidence refreshed. |
| 8 | rm-239 text-asset ledger | 87.0 | Supersession verified live this cycle (dispatch_sanitize, 16 call sites, output_safety_matrix) — next cycle picks it up from the recorded state, not from scratch. |

## Commit-gate seams

1. **Patch order** (all against the in-tree ROADMAP.md): `roadmap-delta-d0ecd354.patch` FIRST → `prioritize-delta-81b107b7.patch` → `compound-delta-6e2e955c.patch`. The postimage to expect after all three: sha256 `1f38568972deecaa3e76c3b263ac7786a257289ec2e970556353fcbd31057c31` (1459784 bytes; amended by review fix 9cf36060 — banner census def-row correction + lineage rider).
2. **Two untracked paths must be staged explicitly** (a bare `git commit -am` drops them): `docs/stewardship/2026-10-09-cycle1-compound-record-rune9dbcbc2428d.md` and `docs/solutions/workflow-issues/conductor-validation-evidence-is-a-verbatim-contract.md`.
3. **No schema bump in this batch** (additive waste.v1 field only) — no ceiling re-check beyond the roadmap renumber comment's next-free pointer.
4. **Sibling reconcile BY TITLE**: the TUI arm of the basis contract is title-overlapped by UNLANDED rm-847 (9d45a4ec lane, base bfa7ff9) — this batch implemented it natively against in-tree LoopCostBasis; at integration the arms fold by title and the recorded numeral double-claims (rm-847 triple-claimed) resolve there.
5. **Done-flips for rm-857/rm-858 are NOT this gate's** — rows stay `implemented`; done is reserved to the INTEGRATION gate (fleet ruling 7eae74eae1ce + 415d77e1, which supersedes the rows' "reserved to the commit gate (rm-012)" phrasing: any gate-time wall edit would be an unvalidated delta; the integration gate flips by title after title-twin reconcile).
6. Expected porcelain at gate entry: 8 tracked M + 2 untracked docs.

## Explicit non-goals observed

- No test/validation command executed (compound contract; outcomes consumed from 3a54a68d and e39775f7).
- No review or shipping outcomes recorded here — those post-date compound; the next cycle's assessment carries them forward.
- No in-tree ROADMAP edit, no commit, no push; worktree porcelain at compound exit = 8 M + 2 ?? (verified).

## Review-fix addendum (2026-10-09, independent_review:fix attempt 9cf36060)

Independent review (fe15619c) returned NEEDS_CHANGES with 6 findings; dispositions below. The review-fix phase — unlike compound — ran the impacted targeted lanes (per required_scope=none); it did NOT rerun the full suite (budget contract), which stands at 675/0 from e39775f7 on the pre-review tree.

| # | Finding (review) | Disposition |
|---|------------------|-------------|
| F1 (high) | Priced loops' top_actions entry claims ", synthetic estimate" (waste.rs:184, unconditional in the `loop_percent > 20` arm) | FIXED: the action marker now gates on `loop_basis == Synthetic`, mirroring the summary arms; priced action reads `loop waste $X (N%) - add max retries limit`; `priced_loop_cost_basis_stays_unmarked` extended with both a no-`synthetic` assertion and an action-still-exists assertion over `top_actions` |
| F2 (low) | Basis line + EN summary marker fire on loopless sessions (vacuous $0.00 disclosure) | FIXED: compute summary arms gate on `loop_cost > 0.0` (single `synthetic_loop_disclosure` binding); text-view basis line gates on `report.loop_percent > 0.0` — matching the TUI arm's loop-presence gate; JSON `loop_cost_basis` stays unconditional (machine contract); new `loopless_waste_sessions_do_not_disclose_a_loop_basis` pins summary/EN-text/zh-text unmarked + JSON still declaring |
| F3 (low) | zh yellow summary lacks the marker; zh action translation leaves ", synthetic estimate" in English | FIXED: zh yellow arm gains the conditional （循环部分为合成估算） marker; zh orange/red gate on `loop_percent > 0.0`; zh action translation now `.replace(", synthetic estimate", "，合成估算")` |
| F4 (low) | Compound banner status accounting used substring tokens (167/67/81) vs def-row truth (156/66/78) | FIXED: banner + this record now state the def-row census as primary (156/66/78 post-flip, 158/66/76 pre-flip) with the substring figures explained (15 legacy-format rows); builder asserts the def-row census programmatically |
| F5 (low) | Implement envelope claims a `-p agenttrace-cli --test entrypoints waste` lane; targeted logs show `-p agenttrace` (package name, not directory) | RECORDED CORRECTION ONLY: the historical envelope is immutable; the correct lane spelling (`-p agenttrace`) is what actually ran and what the review-fix battery re-ran. Fleet lesson already pinned (memory #17439-style): `cargo -p` takes the package NAME. |
| F6 (low) | doctor.rs shell_quote doc comment says quotes become `'''`; to_string_lossy U+FFFD edge undocumented | FIXED: doc now states the real `'\''` escape (close-quote, escaped quote, reopen-quote) and notes non-UTF-8 bytes render lossily (U+FFFD) while quoting still holds; no behavior change |

**Review-fix validation (targeted)**: `cargo fmt --check --all` rc0; `cargo clippy --workspace --all-targets -- -D warnings` rc0; `cargo test -p agenttrace-core waste` 7/0; `-p agenttrace-core shell_quote` 2/0; `-p agenttrace-tui loop_basis` 1/0; `-p agenttrace --test entrypoints waste` 3/0 (e2e incl. `waste_json_discloses_synthetic_loop_cost_basis`). **Live PoCs re-verified on the rebuilt release binary** (`cargo build --release` rc0): the review's loopless corpus now prints no `Loop cost basis:` line and no summary marker; the loop-present corpus keeps both. **Digest**: engine digest re-derived with base=HEAD full sha → `validation:v1:ff1d9db5…357bc`, UNCHANGED from dispatch (crates/**+docs/** deltas are digest-immobile), so the dispatch digest remains the correct verbatim declaration.

Batch delta after the fix: 8 tracked files, +405/−24 (was +328/−21). Porcelain: 8 M + 2 ?? (unchanged set).

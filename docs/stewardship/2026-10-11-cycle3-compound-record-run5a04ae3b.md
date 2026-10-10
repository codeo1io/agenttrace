# Cycle 3 compound record — run 5a04ae3b0b9f (repository-maintenance 62052d7689d04bafab23fbef59139aea)

- Date: 2026-10-11
- Run: `5a04ae3b0b9f45cbbc50a8397bd0ba7b`, cycle 3, compound attempt `396cfce2853f44528e4ef30af01f61bc`
- Worktree: `run-5a04ae3b0b9f-5a04ae3b` @ HEAD `428f0e9` (porcelain at compound entry = the implement batch, uncommitted per policy)
- Review round: independent review `04eedd96` returned **NEEDS_CHANGES** (F1–F7); review-fix attempt `f89a289b` (same day) fixed every actionable finding in-tree and in the spool postimages — findings and dispositions in the review-fix section below; this record's claims were corrected with it
- Scope: compound ONLY — pre-review cycle evidence consumed, **no test/validation command re-run** (dispatch boundary; review and shipping outcomes fold after this step)

## Inputs consumed (pre-review cycle evidence)

| Phase | Attempt | Durable artifact |
|---|---|---|
| assess | 6b369cf6 | scratch: cargo-test.log, poc1/multi.jsonl, poc2/dur.jsonl, poc3/opencode/storage (twin sessions) |
| research | 15c8c46d | research-dossier.md (8 candidates; C1 = codex 0.162/0.163 drift) |
| roadmap | ae668616 | mint patch + ROADMAP.postimage.md + pins.txt (minted rm-942..rm-946) |
| prioritize | d246833f | prioritization.md (batch selection rationale, claim-state adjudication) |
| stewardship | a06acb18 | stewardship-request.md (contract, file:line surfaces) |
| implement | e0a29b1b | worktree delta (4 tracked files + fixture tree) |
| targeted_tests | 3cef9400 | cargo-test-targeted.log, targeted-3cef.log |
| full_tests | c8850265 | cargo-test-full.log, fmt.log, clippy.log, release-build.log, smoke.log, locked-cargo.log, real-cli-smoke.log |

## Batch compounded: "accounting truth: reasoning tokens, model sums, elapsed time"

| Row | Track | Landed shape | Worktree pin (md5, = implement/targeted/full end-state) |
|---|---|---|---|
| rm-619 (LEAD) | correctness 70.0 | `add_opencode_tokens` bills the object's own `output` AND its `reasoning` ADDITIVELY at the output rate — `output.saturating_add(reasoning)`, the CU-20/opencode_db convention (sqlite_sessions.rs:522/:876); reasoning is never REPLACED by output (review fix F1: the first cut folded reasoning ALONE and dropped a mixed message's own output) — breakdown visible on its own `reasoning_tokens` line, AND returns usage-added-only so an all-zero tokens object no longer suppresses the step-finish fallback; totals fold saturating (review fix F6) | parser.rs `206e4ddf…` |
| rm-944 | correctness 55.0 | by_model buckets accumulate RAW, round exactly once after the loop, largest-remainder residual to the max-cost bucket → Σ by_model == total exactly (disclosed at review F5: a bucket whose raw share falls below the round floor pays 0.0 — a credit can zero a bucket while the sum still partitions the total; no negative bucket can render) | lib.rs `8a0fa2a3…` |
| rm-946 | developer-experience 40.0 | `fmt_duration` (en) + `fmt_duration_for_language` (zh) truncate toward zero at each arm's precision — a rendered duration never claims time that did not elapse | lib.rs `8a0fa2a3…` / reports.rs `5c989c93…` |

Tests: `tests/journal_truth_batch.rs` (`f96be1b1…` after review fix + rustfmt) + `tests/fixtures/usage-accounting/opencode/` (twin shape 107/58/777 vs control; review-fix additions ses_m619 mixed + ses_h619 i64::MAX clamp). Red-first: 6 failures on the unfixed source (2 opencode twins + by_model sum + 2 duration-boundary locales + goldens-compat); review-fix red-first: the mixed twin FAILED `left: 30, right: 80` and the clamp twin panicked `attempt to add with overflow` on the pre-fix source (duration-boundary test names corrected at review F4: `fmt_duration_never_rounds_up_across_a_unit_boundary` / `…_zh_never_rounds_up…` — the compound bullet's original `fmt_duration_en_boundary`/`fmt_duration_zh_boundary` never existed).

## Recorded outcomes consumed, NOT re-run

- targeted_tests 3cef9400: journal_truth_batch 18/0 after the batch's red-first cycle; full crate 638 passed / 41 result-ok suites / 0 failed; the digest's phantom FAILED adjudicated (see prevention doc #1).
- full_tests c8850265: fmt rc0; `cargo test --workspace --locked` 874 passed / 0 failed / 52 ok suites (vs 869/47 pristine baseline at 428f0e9); clippy both forms `-D warnings` rc0; release build 14,393,432 B; 7 smoke scripts + check-locked-cargo + real-cli-smoke all rc0 (AGENTTRACE_CI_OUT in delegate scratch); dispatch full_command EMPTY → ci.yml push-suite mirror convention.
- Live release-binary replays (recorded at implement/full, not re-run): poc3 → pocA 100/50/$0.0011 unchanged, pocB 0/9000/9000/$0.135 (was 0/2-estimated/$0.0000); poc1 → Σ by_model 0.0002 == total_cost 0.0002; poc2 → "completed over 59.9m" beside "Longest idle gap: 3599.8s".

## Roadmap status delta (spool-side; the commit gate lands it)

Chain, in order, all default-`git apply` roundtrip-proven against the pristine worktree wall (sha256 `0f1ff813…`):

1. `ae668616-roadmap-mint.patch` (roadmap phase) → mint postimage `6a4807eb…` (minted rm-942..rm-946; header-normalized copy `396cfce2-derived-mint-normalized.patch` used for the chain proof — only the two `---`/`+++` header lines rewritten)
2. `396cfce2-roadmap-compound.patch` (sha256 `741b54fe…` after review fix; superseded pre-review `d9967a77…`) → compound postimage `d12cadcc…` (superseded pre-review `8d68267c…`; 4290 lines, unchanged row count):
   - **flips candidate→implemented (credit `run 5a04ae3b0b9f cycle 3 compound 396cfce2 + review-fix f89a289b; independent review 04eedd96 NEEDS_CHANGES closed; commit gate pending`)**: rm-619, rm-944, rm-946 — each with a dated EXECUTED bullet carrying the decision, the red-first evidence, the live replay, and the review-fix deltas;
   - **rm-709 discharged IN-ROW (no flip)**: status token still reads candidate despite the 2ac5bbe1 integration FOLD adjudication; prioritize d246833f re-adjudicated it stale a THIRD time — flip-or-retire is a commit-gate bookkeeping residual, no implementation owed;
   - compound banner at head of the banner stack (fleet convention), zero mints at compound;
   - status accounting over the 402 `^- id:` rows: 178 candidate / 127 implemented / 82 done / 1 open (before: 181/124/82/1).
3. `396cfce2-changelog-rider.patch` (sha256 `13fb4f4d…` after review fix; superseded pre-review `44024d2c…`) on the pristine worktree CHANGELOG.md (`7b45876b…`): 3 Fixed bullets at the top of Unreleased (review fix F3/F5 wording folded into the rm-619 and rm-944 bullets); byte-frozen no-changelog-section tail preserved (`tail -c 1731` md5 `e7a2784e…` both sides); `.md` deltas are validation-digest-immobile so the full_tests digest stands over the enlarged tree.

## Sibling overlaps — reconciliation notes for the gate

- **rm-619 is the fleet's most-reimplemented row.** Two unlanded sibling lanes claim it:
  - `349bc0f6` (run 6603da89 cycle 1, SAME base 428f0e9, on-disk spool chain f5bbcc1b → 3b889186 → 349bc0f6) flips the SAME def line with credit `run 6603da8916bd cycle 1 compound 349bc0f6` and a DIFFERENT landed shape (reasoning folds onto the reasoning channel + a >0 guard; no output-rate billing). **Textual collision on the def line: apply ONE flip, fold the other lane's evidence in as corroboration, reconcile BY TITLE.** This lane's semantics follow the ae668616 FIX CONVENTION rider (producer oracle: opencode persists reasoning as a first-class sibling; the ai-sdk adapter folds it into output) and add the all-zero suppression prong.
  - `2ba22694` (run e944a060 implement @ base 8982722, spool pins only) landed an `OpencodeTokensFold{billing,reasoning}` shape — same reconciliation rule.
- rm-944/rm-946 were minted by THIS run (ae668616) and verified unclaimed across the spool at prioritize — no sibling collision known.

## Prior-attempt dispositions (run-wide, provider-family reaps; zero durable output in every case)

- assess `af1919a1`: 429 rate-limit, failed envelope, event-log reaping — redone as 6b369cf6, which re-verified the reaped attempt's two observations live and adopted them.
- full_tests `74a3e219`: dead at message 9, 4 bookkeeping events, no scratch — redone as c8850265.
- THIS compound's prior attempt `2ec98f98`: reaped at message 2, 4 bookkeeping events, no typed artifact, no scratch — phase redone from scratch and declared.

## Prevention docs minted (this compound)

1. `docs/solutions/workflow-issues/recorded-failure-claims-need-tree-verification-before-adjudication.md` — phantom test names in recorded digests + transient cargo test-cache races.
2. `docs/solutions/quality-gates/token-folds-follow-producer-billing-semantics.md` — fold direction is decided by the producer's persisted schema, not by parser convenience.

## Independent review 04eedd96 — NEEDS_CHANGES, closed by fix f89a289b (2026-10-11)

The review ran BOTH binaries (batch + pristine-HEAD `git archive` build) over synthetic mixed/hostile opencode storage and live-proved warm-cache staleness. Findings and dispositions:

| # | Finding (file:line at review) | Severity | Disposition |
|---|---|---|---|
| F1 | Mixed message `{input:10, output:50, reasoning:30}` billed 30 output tokens — the `Some(reasoning)` arm folded reasoning ALONE and dropped the object's `output` (parser.rs:5866-5871) | high | **FIXED** — both terms add; new fixture `ses_m619` + red-first pin `opencode_mixed_message_bills_output_and_reasoning_additively` (red: `left: 30, right: 80`) |
| F2 | No schema bump: a warm v51 cache kept serving pre-fix totals for unchanged storage bytes (review PoC: 60 tokens/$0.0008 warm beside 787/$0.0117 cold; session_cache.rs:8 rm-230) | high | **FIXED** — `SESSION_CACHE_SCHEMA_VERSION` 51→56 minted at the fleet census ceiling (origin/master 53, integration 54, sibling run-8937be8e 55); same-unit sweep: const + ladder rung, rm-710 oracle fn/assert/message (renamed `schema_51`→`schema_56`), governance sentence, sweep doc (docs gate rc0 against the live constant) |
| F3 | Lesson doc + code comment described the convention as "stored output INCLUSIVE of reasoning" — both oracles are ADDITIVE (`sqlite_sessions.rs:522` `tokens_output.saturating_add(stored_reasoning.max(0))`, qwen parser.rs:3319) | medium | **FIXED** — lesson doc rule 1 rewritten additive with the review correction recorded; fn comment + this record's batch table corrected |
| F4 | ROADMAP/CHANGELOG/record cited nonexistent tests `fmt_duration_en_boundary`/`fmt_duration_zh_boundary` | low | **FIXED** — real names pinned everywhere; the phantom-name failure mode is already this compound's prevention doc #1 |
| F5 | by_model credit floor undisclosed: a bucket whose raw share is below the round floor pays 0.0 — Σ still partitions the total exactly | low | **DISCLOSED** (no code change warranted) — disclosure appended to the rm-944 bullets in ROADMAP postimage + CHANGELOG rider + the table above |
| F6 | Plain `usage.values().sum()` overflowed on rm-046-clamped i64::MAX token classes (debug panic; release wrap → negative → verdict flip) | high | **FIXED** — `usage_total` folds saturating; new fixture `ses_h619` + pin `opencode_hostile_clamp_totals_do_not_overflow` (red: overflow panic pre-fix) |
| F7 | Partial-billing residual: a `tokens:{input}`-only message still marks the message covered and suppresses the step-finish part lane (parser.rs:5846-5871) | low | **RECORDED** as the next-cycle lead below (out of this fix's scope; narrower than the original rm-619 seam) |

## Next-cycle leads (ranked; re-derive at assess)

1. **rm-619 partial-billing residual** (review 04eedd96 F7): a partial-billing message (`tokens:{input}` only, output absent) fires the usage-added verdict on input alone and suppresses the step-finish part lane (review PoC: part's 50+20 dropped, session shows TOKENS 100 on both binaries) — the same seam one step narrower; the natural next accounting-truth candidate.
2. **rm-942** codex 0.162/0.163 rollout-persistence drift (compat 70.0, VERIFY-FIRST — no 0.162+ journal on this host; release-notes-sourced datum, declared).
3. **rm-712 / rm-713** (74/72, session_cache surface, M-L effort) — deliberately deferred this cycle; the delicate schema lane.
4. Top-slice remainder to re-rank live: rm-729 / rm-758 / rm-776 / rm-845 / rm-832 / rm-881 / rm-859.
5. **rm-943** Cf-bidi sanitizer survival (security 62.0, CVE-2021-42574 class; PoC fixture preserved in `26fa10ac…-scratch/probe/`).
6. **rm-945** unset-HOME → CWD planting (security 48.0; PoC fixture preserved).
7. Pricing snapshot refresh on the rm-176 lane (research 15c8c46d census: LiteLLM live 4,034 vs bundled 3,100 = 934-key gap; models.dev SHRANK 8,454 → 8,443 — a refresh must not assume monotonic growth).

## Verification (re-runnable)

```sh
SP=/home/agent/.hermes/conductor-delegate-spool/delegate/396cfce2853f44528e4ef30af01f61bc-scratch
sha256sum $SP/ROADMAP.mintpost.md $SP/ROADMAP.compound-postimage.md $SP/396cfce2-roadmap-compound.patch $SP/396cfce2-changelog-rider.patch
# expected (post review-fix f89a289b): 6a4807eb… / d12cadcc… / 741b54fe… / 13fb4f4d…
#   (superseded pre-review values recorded in $SP/pins.txt: 8d68267c… / d9967a77… / 44024d2c…; originals snapshotted in f89a289b…-scratch/pre-fix-snapshot/)
cd "$(mktemp -d)" && git init -q && cp <worktree>/ROADMAP.md ROADMAP.md && git add . && git commit -qm b &&
  git apply $SP/396cfce2-derived-mint-normalized.patch && git apply $SP/396cfce2-roadmap-compound.patch &&
  sha256sum ROADMAP.md   # → d12cadcc…
md5sum <worktree>/crates/agenttrace-core/src/parser.rs   # 206e4ddf… (batch + review fix F1/F6)
```

Roadmap delta NOT applied to the worktree ROADMAP.md (stays pristine `908207d6` md5 per convention); CHANGELOG.md likewise untouched in-tree. This record + both prevention docs are untracked and must be staged explicitly at the gate.

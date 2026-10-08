# Cycle 3 compound record — run e88f2da2fbe9 (2026-10-08, pre-review)

Run: `e88f2da2fbe941a4a7d5b0a2cbaf874c` · repository-maintenance `546703d401544848b86c86a4c85b8bfe` cycle 3 · base `92149bd5978f945bee1d22cb91859a42f719c2df` (porcelain 0 in/out; canonical is an ancestor). Compound attempt `884f0594c3fa4ab09f07001e9f9f21df`. This record consumes ONLY pre-review cycle evidence; no test or validation command was executed at compound (compound contract).

## Phase ledger (attempts, adoptions, reaps)

| Phase | Attempt | Disposition |
|---|---|---|
| assess | `79d82fc1` | fresh; F1 budget 26x overstatement + F2/F3 corroborations + 1,400-case fuzz 0 panics |
| research | `c7f83698` | fresh; 7 candidate lanes (C1 discovery breadth = minted rm-774 lane), fleet first-hand probes |
| roadmap | `9060b4e3` → `65bb1d3a` | 9060b4e3 provider-reaped 89 s AFTER its ROADMAP.md write; delta-present ADOPT (leg-by-leg verified), redone as 65bb1d3a. 3rd recorded reap-after-write instance |
| prioritize | `8fb1bd7c` | fresh; batch 'honest dollars' selected, rm-771 narrowed to arms 1+3+4 (title-twin arm 2 deferred) |
| stewardship | `1109077635` (after `8d760c28`) | 8d760c28 transport-reaped at 27 s, 4-line event log, zero durable work — classic redo-from-scratch branch |
| implement | `20228c17` (after `d886218c`) | d886218c provider-reaped mid-work; DECLARED ADOPTION of its 3-file delta + red-first.log (4 planted reds), then completed |
| targeted_tests | `01e7fe06` | fresh; 575/0 + one docs-only CHANGELOG correction (pin real-name rule) |
| full_tests | `df982a4cc` | fresh; full battery green, digest re-derived byte-equal |

## What was implemented (uncommitted delta at compound: 6 tracked files +494/−46, zero untracked)

Batch 'honest dollars' (cost numbers tell the truth): **rm-771** (LEAD, arms 1+3+4 — statusline budget-series pre-journal baseline truth), **rm-570** (recorded-cost audit comparison basis), **rm-339** (corroborated arm — waste hit-rate above-100% graded `anomalous`, not `excellent`). Files: `crates/agenttrace-core/src/statusline.rs`, `governance.rs`, `waste.rs`, `CHANGELOG.md` (+3 Fixed bullets +1 heading), `docs/guides/governance-reports.md` (recorded-basis sentence), `ROADMAP.md` (+46 = roadmap-phase mint, carried untouched). All three rows flipped candidate→implemented with dated EXECUTED addenda at compound; done-flips and `(partial arm)` markers reserved to the commit gate (rm-012). Status census 135/63/52 → **132 candidate / 66 implemented / 52 done** at 250 def rows, 0 duplicate ids, 106/106 comment balance, managed footer last.

## Recorded validation outcomes (pre-review; consumed, NOT re-run)

- **targeted `01e7fe06`**: `cargo test -p agenttrace-core -p agenttrace -p agenttrace-tui` → 575/0 (409/118/47); per-pin 1/0 ×6 by real name; `fmt --all --check` rc0; clippy all-targets 0 warnings; `check-docs-commands.sh` + `check-report-semantics.sh` rc0. Docs-only fix in-phase: CHANGELOG rm-339 bullet cited `cache_efficiency_grades_impossible_hit_rates_anomalous` — a name that exists nowhere in the tree; corrected to the landed `cache_hit_rate_above_100_is_disclosed_not_excellent` (a7110bf0 real-name rule), erratum on `delegate/20228c17…-scratch/implement-record.md`.
- **full `df982a4cc`**: `cargo test --workspace` → **575/0 across 29 suites** (assess baseline 568/0 + exactly the batch's 7 pins; two consecutive green runs — the known sqlite env-lock flake did not fire); `clippy --workspace --all-targets` 0 warnings; `fmt` rc0; `cargo deny check` advisories/bans/licenses/sources ok; **all 14 `scripts/ci/*.sh` rc0**.
- **Digest lineage**: dispatch token `validation:v1:58f2516eab516d394143c7076c368d37133a4890e9612a52e28bf67ab1bd1730` declared VERBATIM at both validation turns, each re-derived live with the engine formula (25 executable-surface files, base `92149bd`) and byte-equal. Compound touches ROADMAP.md/docs only — digest-immobile under the classifier.

## Live PoC replays (implement phase, debug + release binaries)

- **rm-771 corpus** (`delegate/79d82fc1…-scratch/poc-mirror/budget-overstate/`): `--budget --weekly-budget 10` → `7d total $2.00` + `budget $10.00 — $8.00 remaining (80%)` + `first-day baseline unknown for 1 session(s) — $50.00 of pre-journal spend excluded` (was `budget $10.00 - OVER by $42.00` = 26× overstatement). `$62` variant → literal `OVER by $2.00`. `--budget -f json` and `--statusline-report -f json` AGREE: `spend_7d_usd 2.0 / remaining_usd 8.0 / first_day_baseline_unknown_sessions 1 / _usd 50.0` on both.
- **rm-570 PoC rebuilt at this base** (original `/tmp` corpus swept): pi standalone-usage entry, `cost.total 0.07` on 38,500 in / 950 out sonnet-4-5 → `recorded_cost_usd 0.07`, `pricing_source 'vendor-recorded cost; not re-priced'`, drift note ABSENT (was stored 0.07 vs re-priced 0.1298 + false-drift note), top-level `current_estimated_cost_usd 0.07`.
- **rm-339 hostile corpus** (`poc-mirror/cachehostile.jsonl`, 5,000 read / 10 input): rating `anomalous` + anomaly suggestion EN (was `excellent (hit 50000%)`), `hit_rate_percent 50000.0` unclamped in json.

## Cycle learnings compounded

1. **Adopt-then-run-the-reds** (extends the provider-reaped prevention doc, addendum below): a reaped attempt's planted red tests are the cheapest oracle for whether its delta is complete. The adopted implement delta was ~90% done but seeded the first day with `(NEG_INF, NEG_INF)` — `first_seen` was unreachable, the baseline-unknown arms never fired, and two planted reds stayed red. The reap had cut the attempt before any green run. Rule: an adopted delta is not trusted until the dead attempt's own red tests (or fresh equivalents) run green against it.
2. **Pre-existing pins can pin the bug**: `budget_view_renders_spend_budget_and_unconfigured_state` asserted the dishonest `$3.00` (full first-day carry). When a fix changes behavior, re-derive the OLD pin against the new contract instead of preserving it — and record the re-derivation on the row (done on rm-771's EXECUTED bullet).
3. **Prose examples must be arithmetically reachable**: the rm-771 row's arm-3 example "$52 → OVER by $2.00" is impossible ($2 max provable burn vs $10 budget ⇒ $8 remaining). Both readings are now pinned (honest corpus `$8.00 remaining`; literal `OVER by $2.00` from a `$50→$62` journal). Row text left as-written; the EXECUTED bullet records the discrepancy.
4. **The engine's surface classifier counts only `.github/workflows/*` + `scripts/**`** for this repo — a Rust-only batch shows `changed_testable_surfaces=[]` and `full_command=""`. Validation phases must derive scope from the actual delta (both did, declared). Recorded for the next dispatcher.
5. **Two seams discovered live (next-cycle mint candidates, anchors recorded on the banner + rm-570's bullet, NOT minted — zero-mints-at-compound convention)**: (a) pi message-inline `usage.cost.total` is dropped by the parser (`parser.rs` :2132 message arm vs :1925/:2041 standalone-usage arm) — the recorded-basis contract does not reach the inline shape; (b) the sqlite lane's Metrics reconstruction drops `upstream_cost_usd` — audit over sqlite-aggregated sessions (incl. 'multiple models' rows) is recorded-cost-blind.

## Dead-attempt forensics summary

Three reaps this cycle, two adoption branches, one classic redo: roadmap `9060b4e3` (delta-present ADOPT, 3rd recorded reap-after-write instance — event log reap 89 s after the ROADMAP mtime, typed result absent, mtime precedence ruling on the FLEET ADDENDUM), implement `d886218c` (delta-present ADOPT + completion, learning 1), stewardship `8d760c28` (zero-durable, 4-line event log, redone from scratch). All dispositions declared in their phase envelopes; fleet-wide pattern count now 3 reap-after-write + many zero-durable.

## Commit-gate seams (live first-hand at compound)

1. **One patch** carries the 6 modified files + the compound delta (banner + 3 flips) + this record + the prevention-doc addendum. Stage `docs/stewardship/2026-10-08-cycle3-compound-record-rune88f2da2fbe9.md` explicitly — it is untracked.
2. **Done-flips and `(partial arm)` markers AT THE GATE, by TITLE** (rm-771, rm-339 partial; rm-570 full).
3. **Numeral renumber**: this band's rm-771..rm-774 → rm-780..rm-783 per the FLEET ADDENDUM ruling (sibling run-aa41d9b5b41e minted rm-771..rm-775 first). The renumber must move row numerals AND the CHANGELOG bullets' rm-tags together or neither.
4. `SESSION_CACHE_SCHEMA_VERSION` untouched — no bump authored this batch (sibling 32f3b7a1's 32→33 does not conflict).
5. CHANGELOG Unreleased already carries the 3 Fixed bullets from implement; the gate may re-word into a release section per lineage convention.
6. Fleet contention at compound: statusline.rs dirty in 5 sibling lanes (incl. the rm-771 arm-2 title-twin run-7e00d9cbe20a — fold plan on the row), governance.rs in 4, waste.rs in 1 (zero hit-rate hunks). Sequence, don't blind-merge.

## Context for the next maintenance cycle

- **Lead candidates**: rm-772 (step-finish attribution — fold vs sibling 0e602b9a's lane; the red-first model-rotation fixture stays required regardless), rm-773 (git probe fan-out — needs fc996e6d's post-exit drain landed; the ≥50-synthetic-root fixture doubles as its red-first corpus), rm-774 (8-lane discovery breadth — after discovery clears + rm-762 ZCode title-twin reconcile). Pool: rm-421, rm-251, rm-195, rm-239.
- **Fresh mint candidates from this cycle** (learning 5): pi inline recorded-cost drop; sqlite-lane recorded-cost blindness. Mint only at a live fleet census (frontier per this wall's roadmap census: rm-779).
- **Next assessment should re-check**: the arm-2 title-twin landing (7e00d9cbe20a), the env-lock flake (fleet memory: probabilistic until sqlite_sessions tests take the shared lock — both full-suite runs green this cycle), and whether the CHANGELOG pin-name erratum class recurs (the implement envelope still carries the stale name; the tree is authoritative).
- Durable PoC corpus: `delegate/79d82fc1…-scratch/poc-mirror/` (budget-overstate 5 files + cachehostile.jsonl), `delegate/20228c17…-scratch/poc-replay/` (6 replay outputs). `/tmp/at-full-df982/` (workspace log + 14 CI logs) is sweep-volatile.

## Addendum (attempt `4c62495c` — adoption-completion of this record's author)

This record, the ROADMAP compound banner, and the three row flips were written by compound attempt `884f0594`, which was provider-reaped at 02:31:08Z — 91 s after this record's 02:29:37Z write; typed result absent, no scratch directory. That is the cycle's third delta-present adoption (after roadmap `9060b4e3` and implement `d886218c`) and the first recorded COMPOUND-phase reap-after-write in the fleet. A fourth compound attempt, `1e7485cb`, then died at 31 s with zero durable work — the dispatch's forensics block named only that one; the real adoption target `884f0594` was found by the tree/mtime census, not by the forensics notice.

Adopted by `4c62495c` after verification (legs in its envelope): status census 132 candidate / 66 implemented / 52 done re-derived live at 250 def rows / 0 duplicate ids / 106-106 comment balance / managed footer last; digest token `validation:v1:58f2516eab…`, the 575/0 targeted/full counts, and every cited PoC number match the validation envelopes; the FLEET ADDENDUM renumber ruling (rm-771..rm-774 → rm-780..rm-783 at integration) cross-checked at the ROADMAP tail; the implement-record erratum and both spool PoC mirrors confirmed live. Two completions by the successor:

1. The prevention-doc addendum referenced by learning 1 and commit-gate seam 1 was NEVER WRITTEN before the reap — `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` sat unmodified at HEAD (zero diff). Written now as that doc's 2026-10-08 Addendum: rules 7 (adopt-then-run-the-reds) and 8 (a referenced artifact is not a written artifact) plus the five-reap instance table. **Commit-gate seam 1 amended: the explicit-stage list is now THREE docs paths** — this record (untracked), the prevention doc (newly modified-tracked), and ROADMAP.md (already in the 6-file set).
2. rm-339's EXECUTED bullet mislabeled the CHANGELOG pin-name correction as "compound `01e7fe06`"; the correction happened at TARGETED `01e7fe06` (the implement-record erratum header says so, and the CHANGELOG fix's mtime matches the targeted turn). The row now reads "targeted".

Final cycle reap census: FIVE — `9060b4e3` / `d886218c` / `884f0594` delta-present-adopted, `8d760c28` / `1e7485cb` zero-durable-redone. No ids minted, no status flips, and no test executed by either compound attempt (compound contract: recorded outcomes only). The banner's counts, seam list, and next-cycle leads stand as written by `884f0594`, except where amended above.

# Cycle 2 compound record — run cb38b958 (repository-maintenance f89f5223)

Date: 2026-10-06 · Base at cycle start: e389f1a (worktree dispatched 1511547, ff'd) · Ceiling at compound: 67dfdb5 · Batch: **rm-538 SOLO** — "Disclosure-channel truthfulness: split assumption-disclosure from parse-loss"

This record compounds ONLY pre-review cycle evidence (assessment, research, roadmap, prioritization, stewardship, implementation, targeted/full validation outcomes). Review and shipping outcomes happen after compound; the next cycle's assessment carries them forward.

## What the cycle did, phase by phase

| Phase | Attempt | Outcome |
|---|---|---|
| assess | a1a044f8 (redo; prior 1b4ea0dc provider-reaped, adopted nothing) | 6 findings; F1→rm-538 with live PoC (corpus3: `confidence: low` + `Dropped lines: workbuddy_input_basis:cache_subtracted=1` with exact=1 fallback=0 unknown=0); F2 ovf panic folded to 933058's rm-529 lane; base verified e389f1a (v0.10.0-wave is upstream luoyuctl, not landed here) |
| research | e77beee0 | codeburn #1579/#1639/#1637 refreshed (rm-537 gate: #1579 still OPEN); pi-mono v1.0.4 (no session-format change); otel semconv still zero tags; LiteLLM 0-key-delta day; no-repeats census (max in-tree claim rm-532) |
| roadmap | 994ab076 | +35-line verified patch: provenance banner, minted rm-537/rm-538/rm-539, 9 watch refreshes; roundtrip byte-exact |
| prioritize | ada86d24 | rm-538 SOLO (correctness 72.0, own mint, unclaimed); declined list preserved below |
| stewardship | — | request authored from the memo, unchanged scope |
| implement | 1e8f1b7a → REJECTED (attestation-only, KTD13) → 6fbb2946 | full rm-538 implementation + the schema-bump follower fix (TUI warm-cache fixture) |
| targeted_tests | 0015393a | focused battery green (core 303/0, TUI 47/0, CLI 86/0, fmt, statics); digest proven current |
| full_tests | 9c6fc2ac | ci.yml full lane 20/20 green, 368/0 across 21 suites; canonical clippy rc0 after 2 lint-rider fixes; deny all-ok; plugin-version rc0 (tag trap resolved by landed backfill) |

## The change-unit as implemented (uncommitted delta, commit gate lands)

- workbuddy basis counters (`workbuddy_input_basis:cache_subtracted` / `:zeroed_suspected_mismatch`) moved from `Metrics.line_skips` → `Metrics.disclosure_counters` (minted at the workbuddy usage site in the lib.rs analyze pass); `DataHealth.confidence` and the "Dropped lines" presentation are now pure parse loss
- render: `Disclosed facts:` arm in reports.rs (text/md/html) + doctor.rs; `-f json` carries them under `data_health.disclosures`
- SESSION_CACHE_SCHEMA_VERSION 26→27 (newly exported at crate root; Go-compat pins read the const)
- red-first `tests/disclosure_channel.rs` (3 cases) + deliberate flips: parser_property_invariants, pi_usage_tree_accounting, the rm-450 regression test (expectation flip documented against the landed "green: disclosure lines present + confidence low" note — this row intentionally changes that landed design), TUI warm-cache fixture → 27
- CHANGELOG Unreleased entry; docs/guides/governance-reports.md
- **Lint riders from full_tests** (not rm-538 scope, must land with the batch): `crates/agenttrace-core/src/parser.rs` module-level `type CheckpointSnapshot` (type_complexity, landed f59a67d), `crates/agenttrace-cli/src/main.rs` borrow elision ×3 (needless_borrow, landed wave)
- **Compound delta** (this record + prevention rule + roadmap banner/note)

Live PoC acceptance flip (pre-review evidence): `-d corpus3 --overview` now prints `confidence: high`, NO Dropped-lines row, `Disclosed facts: workbuddy_input_basis:cache_subtracted=1`; `-f json` → `data_health.confidence: "high"`, `line_skips` absent, named `disclosures` counter.

## Lessons / prevention rules compounded

1. **NEW RULE** `docs/solutions/workflow-issues/schema-version-bumps-require-planted-fixture-sweep.md` — version-constant bumps are cross-crate changes; sweep pin sites by grep, not by diff scope. Observed as a real red in implement attempt 2.
2. **Implement folds MUST declare `validation_evidence.changed_surfaces`** (engine KTD13). Attempt 1e8f1b7a was rejected with ZERO code defect on exactly this. Declare every repo-relative path in the engine-derived tree delta; annotate ff-arrivals.
3. **Digest mechanics** (reusable): the engine's `validation:v1` digest salts with the FULL 40-char base sha (`base:<sha40>:files:<n>`) over executable-classified paths only — `crates/**` and `**.md` are digest-immobile. One dispatch digest stayed verbatim-current through implement→targeted→full, proven each time by re-deriving with the engine's own `validation_policy.validation_digest`.
4. **Landed-lint reconciliation is a full_tests obligation**: the canonical clippy lane was red at the landed base from 4 pre-existing lints under the pinned toolchain (1.98.1); the full gate cannot pass without mechanical fixes — fixed as riders this cycle (see above), and the fix files must ride the commit.

## Declined at prioritize (context for the next cycle)

- **rm-537** (HIGH 76.0): own gate — codeburn #1579 still OPEN (rechecked 2026-10-05T14:15Z); no macOS specimen on this host; 3-arm lane exceeds one cycle. **Cycle-3 lead the moment it merges** — needs a fixture-first discovery design (no live Claude-3p ledger on Linux hosts).
- **rm-539** (cursor.com sync): gated on undocumented endpoint + sequencing behind unlanded rm-531 + privacy scope decision.
- **rm-453** (parser.rs split, 6,284 lines): blocked by the unlanded parser.rs inflight set (rm-510, e486 arms) — re-census before picking.
- **rm-445..458 feature band** (spend-cap/margin/agent-share/forward-fill/invoice): unclaimed, MED value — outranked by the PoC-backed correctness unit this cycle; natural filler candidates.
- **rm-390** (zh parity), **--range '1d' token** (assess F4): docs/micro-items, mint via a future roadmap phase if a filler unit is needed.
- **rm-006/rm-176 snapshot refresh**: zero-key-delta day (LiteLLM 0 added/0 removed).
- Owned elsewhere (do not double-implement): 933058's rm-529/rm-532, 66e75e39's rm-529/rm-530, e486's rm-530/rm-531.

## Fleet numbering at compound

This run minted rm-537..539 (in-tree max def-row rm-539). Sibling 2d92ee95 has since minted rm-539/540 — **rm-539 is DOUBLE-MINTED (this run's cursor.com sync vs theirs); reconcile by title at integration.** Next free id: RE-DERIVE by live census per memory #16706 (worktree greps + canonical git-show + spool mint sweep); do not trust a ceiling below the live max.

## Commit-gate seams

1. Delta = 14 M + 1 ?? @67dfdb5 + this compound delta (2 new docs + roadmap banner + rm-538 note extension) — ALL of it lands, including the 2 lint-rider files.
2. rm-538 status stays `implemented` here; done-flip is the commit gate's (rm-012 convention).
3. The roadmap c2 patch (+35 @e389f1a) already rides in-tree — re-verify anchors at the commit ceiling (ROADMAP moved with sibling landings; apply_roadmap.py asserts exactly-once and can regenerate).
4. rm-510 textual adjacency (parser.rs basis-arithmetics, unlanded): separate hunks, sequence by title at integration.

## Evidence index

- Assessment: /tmp/at-assess-a1a0/findings.md + corpus3/ + out.json
- Research: /tmp/at-research-cb38/cycle2/research-dossier.md (+ live JSON fetches)
- Roadmap: /tmp/at-roadmap-cb38/roadmap-run-cb38b958-cycle2.patch + ROADMAP.postimage.md
- Prioritize: /tmp/at-prioritize-cb38/selection-memo.md
- Stewardship: /tmp/at-stewardship-cb38/stewardship-request.md
- Implement: /tmp/at-impl-cb38/implement-rm538-attempt2.patch + attempt2-addendum.md + red-before/green-after artifacts
- Targeted: /tmp/at-tt-cb38/targeted-tests.log + static-checks.log
- Full: /tmp/at-full-cb38/run-full-lane.sh + full-suite.log + deny-lane.log + plugin-version-probe.log

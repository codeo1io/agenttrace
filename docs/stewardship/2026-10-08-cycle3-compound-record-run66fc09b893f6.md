# Cycle 3 compound record — run 66fc09b893f64078b79d4987130d3ec2

**repository-maintenance 347d61bc · cycle 3 · 2026-10-08 · compound attempt ac3f44cd**

Worktree `run-66fc09b893f6-66fc09b8`, batch landed by conductor salvage
`ee892099` (porcelain 0). This record compounds pre-review cycle evidence
only; review and shipping outcomes land after the compound phase and are
carried forward by the next cycle's assessment.

## Phase chain

| phase | attempt | outcome |
|---|---|---|
| assess | 21b66103 | fresh adversarial pass at dc65644 (redo of provider-dead bb2059ca) |
| research | 068a3070 | channel re-sweep + codeburn/LiteLLM/models.dev deep-dives, raw fetches cached |
| roadmap | a5047b7c → adopted ffd44caa | minted rm-825/rm-826 + 12 dated riders (reap saga, byte-identical adoption) |
| prioritize | ac0c29b3 | selected rm-825 LEAD + rm-711-N2 / rm-703-N3 / rm-693-N4 arms |
| stewardship | c5b7ffcc | change-unit request, all surfaces re-pinned first-hand |
| implement | e4701a2f | 11 files +546/−183 (+ ROADMAP +30 in-state) |
| targeted_tests | 0a7a6aaa | all green (redo of provider-dead dc794e5a) |
| full_tests | 3d962f02 | 21/21 ci.yml push-lane gates rc0 |
| compound | ac3f44cd | this record |

## Batch and recorded outcomes (not re-run at compound)

- **rm-825 (LEAD, correctness)** — antigravity per-model pricing:
  `AntigravityUsageFold` rewritten per-model; one usage meta event per observed
  modelConfigId plus an unattributed block disclosed as
  `generation_model_unattributed`; `lib.rs` meta arm resolves 2+ models to
  `model_used="multiple"` and engages the per-block pricing discipline.
  Flipped candidate→implemented on-row with EXECUTED/VALIDATED bullets.
- **rm-711-N2** — seen-totals ledger rollover at
  `MAX_CODEX_SEEN_TOTALS=1024` now fires `codex_seen_totals_ledger_rollover`
  (parser.rs:3783-3800), pinned by `codex_seen_totals_ledger_rollover_is_disclosed`.
- **rm-703-N3** — quarantine generation pick race closed by
  `claim_quarantine_slot` (history.rs:111-149): the slot is claimed
  exclusively by open semantics instead of exists-then-rename.
- **rm-693-N4** — cli-vs-core staging-helper duplication collapsed: the CLI
  imports the core `write_private_exclusive` (main.rs:13, call site :1662).

Validations (recorded): targeted = fmt rc0, clippy 3-crate `-D warnings` rc0,
core lib 235/0, hostile_journal_disclosure 14/0, usage_occurrence_contract 5/0,
entrypoints 44/0, tui 51/0. Full = 21/21 ci.yml push-lane gates rc0, workspace
machine-sum 654/0 (== implement's own count — no test lost through the salvage
landing), entrypoints re-verified 44/0 after the release-build lane. Digest
`validation:v1:d68dd6f9…c83569` declared verbatim and re-derived live MATCH at
both gates (crate-only deltas never move the scripts/Cargo/.github digest).

## Reusable lessons

1. **Swept-/tmp suite wedge** (new prevention doc
   `docs/solutions/workflow-issues/swept-tmp-wedges-journal-race-suite-redirect-tmpdir.md`):
   run `cargo test` with TMPDIR outside /tmp plus a timeout wrapper; a code
   regression hangs under both roots, an environment artifact only under the
   swept one. Fixture-hardening lead recorded for next cycle.
2. **Dead-attempt forensics discipline held**: three provider-dead attempts
   this cycle (assess bb2059ca, roadmap f94467f1, targeted dc794e5a) — each
   verified against durable trail (envelope absence + event-log shape + tree
   drift census) before redo; the roadmap phase additionally verified the
   reaped-but-productive a5047b7c by byte-identical script reproduction.

## Next-cycle leads

rm-826 (fixed-subscription presets; reconcile title-adjacent sibling rm-754 BY
TITLE at integration) · rm-164 tier normalizer arm (generic `above_(N)k_tokens`
+ service-tier keying) · rm-176 pricing-lineage divergence SELF-HOSTING urgency
(2,756 pinned vs 4,504 live keys; this fleet's own glm-5.3 delegate sessions
journal cost 0) · rm-423 aborted-dimension arm (pi 1.0.4 corpus already carries
`stopReason:'aborted'`) · journal-race fixture hardening · watches: npm v0.10.2
absent, OTel semconv-genai still 0 tags.

## Commit-gate seams

rm-825 done-flip at landing (rm-012 convention); riders are
landing-acknowledgment only; CHANGELOG bullet, governance-guide sentence, and
schema-bump-sweep doc already ride the salvage delta. ZERO ids minted at
compound: 295 def rows, 0 duplicates, max rm-826, next free rm-827; status
accounting 157/72/66 → 156/73/66 (candidate/implemented/done, every row tokenized).

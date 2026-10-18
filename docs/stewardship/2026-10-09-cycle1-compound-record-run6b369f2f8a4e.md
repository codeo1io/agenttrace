# Cycle-1 compound record — run 6b369f2f8a4e (repository-maintenance 52eaa493ad)

- run: 6b369f2f8a4e43728bcfe7cf065c8622, cycle 1, base dc6564494edd99defa9b77fd55d0c48c3107c5e9
- compound attempt: ae65551f7cf34dd5b68db6fc501c4a1d (2026-10-09, pre-review by contract —
  consumes ONLY recorded outcomes, zero test execution)
- worktree: run-6b369f2f8a4e-6b369f2f (porcelain at compound close = the batch's 9 M files
  + this record and the new solutions lesson as the only 2 untracked files)

## 1. Batch delivery ledger

Batch 'cost truthfulness end-to-end' (ingest → store/read → price → render), selected at
prioritize 32a7dabf, stewarded 29c8b46d, implemented 7d65b322 as ONE atomic uncommitted
9-file delta (+851/−34):

| row | arm | status |
|-----|-----|--------|
| rm-852 (LEAD, p78) | bare journal model ids → carrier-prefixed pricing ladder with disclosed choice (convert_litellm rewrite; first-party exact key wins, strict list-rate consensus names the shallowest carrier, contested keeps legacy winner + disclosure) | implemented |
| rm-851 (p74) | hostile string-typed usage meters coerced-not-rejected (coerced/bool-reject/array-reject fixtures, line_skips disclosures, string meters pinned) | implemented |
| rm-853 (p56) | zh locale drops the 'high' confidence tier (presentation.rs delivery_confidence match gains the high arm; composes with unlanded rm-850) | implemented |
| rm-902 (minted at implement) | serde_json float_roundtrip — warm-cache re-reads bit-exact (straddle + 2^53+1 red-first pins) | implemented |

Validated (recorded outcomes, consumed at compound): targeted c202b43a — core 470/0,
cli 141/0 (pkg 'agenttrace'), tui 52/0 = 663/0 across 34 green result lines, fmt rc0,
clippy --workspace --all-targets 0 warnings. Full 943d2b24 — 22/22 ci.yml push lanes rc0
first try, zero fixes; trio machine-sum 663/0 over 35 ok result-lines == targeted exactly;
entrypoints 44/0; cargo deny advisories/bans/licenses/sources ok; plugin-version green on
the tag-complete clone via the CHANGELOG :502-517 marker block. Digest
validation:v1:d68dd6f920e99821903ad023492fa91483c56dd69657d0255a4b868c4cc83569
PRE==POST==dispatch (engine's own validation_policy at both ends).

## 2. Lessons

- L1 — a never-run plan is a hypothesis stack. The reaped roadmap attempt 39be0a16 left a
  complete build script that NEVER ran; adopting its skeleton required re-verifying every
  anchor first-hand and correcting the plan (its rm-006 rider targeted a def row that does
  not exist on this wall; its kimi mint duplicated 2e7758d3's same-base band). Prevention
  rule distilled → docs/solutions/workflow-issues/reaped-attempt-plans-and-sibling-dossiers-need-first-hand-anchor-and-number-verification.md.
- L2 — sibling dossiers are consumed with provenance labels AND first-hand re-derivation of
  load-bearing numbers: two of 068a3070's claims were disproven (snapshot pin 2026-10-04 /
  3,100 keys, not 2026-09-13 / 2,756; glm-5.3 exists prefixed throughout).
- L3 — citation drift compounds silently: the roadmap banner "corrected" assess F-numbering
  away from findings.md's actual headings and the minted id lines inherited it (rm-851 'F2'
  → actually F3; rm-852 'F1' → actually research §1; rm-902 'F4' → actually F1). Corrected
  with dated addenda at compound; every future mint must cite anchors by file+line of the
  AUTHORITATIVE artifact, not a renumbering note.
- L4 — feature-only Cargo.toml changes do not touch Cargo.lock (serde_json float_roundtrip);
  the commit gate must not "fix" a missing Cargo.lock hunk, and validation digests treat
  config-class manifest additions as non-executable (validation_policy config exclusion).
- L5 — empty dispatch full_command ⇒ run the ci.yml push lanes verbatim (22 lanes here);
  reconcile test counts by machine-summing result lines (grep -oE) — headline 663/0 matched
  the targeted phase exactly across two independent runs.
- L6 — provider-dead attempt forensics pattern (4 reaped + 2 failed priors this cycle):
  event-log tails end session_reaped/delegate_turn_completed-failed with progress pings
  only; envelopes absent; scratch may still hold a usable-but-unexecuted plan (adopt with
  L1 discipline). Zero-durable ≠ nothing-to-read: check scratch BEFORE redoing from scratch.

## 3. Commit-gate seams

1. Stage the 2 untracked docs EXPLICITLY — `git commit -am` drops them
   (docs/stewardship/2026-10-09-cycle1-compound-record-run6b369f2f8a4e.md and
   docs/solutions/workflow-issues/reaped-attempt-plans-and-sibling-dossiers-need-first-hand-anchor-and-number-verification.md).
2. The 9 M files: CHANGELOG.md, Cargo.toml, crates/agenttrace-core/src/{pricing,parser,governance}.rs,
   crates/agenttrace-core/tests/{hostile_journal_disclosure,report_numeric_truthfulness}.rs,
   crates/agenttrace-tui/src/{presentation,tests}.rs. NO Cargo.lock delta expected.
3. ROADMAP chain (one-shot non-idempotent, in order): wall@dc65644 →
   delegate/972cf938…-scratch/roadmap-delta-972cf938.patch →
   delegate/7d65b322…-scratch/roadmap-delta-implement-7d65b322.patch →
   delegate/ae65551f…-scratch/roadmap-compound-incremental-ae65551f.patch; OR the single
   cumulative roadmap-compound-cumulative-ae65551f.patch (both verified byte-identical to
   ROADMAP.compound-final.md).
4. Done-flips implemented→done for rm-851/852/853/902 happen at the commit gate (rm-012).
5. If 2e7758d3's same-base roadmap patch lands first, re-anchor the head-banner hunks per
   the 4bd1679c banner-stack union; rider/tail hunks are disjoint.
6. Title-twin reconciliations at integration (reconcile by title, first landing wins):
   rm-834 (02b8f48a) / rm-803 (3f6b86bc) vs our rm-902; rm-847 kimi; rm-850 zh-CLI;
   rm-806/rm-203 zcode; rm-804/rm-818 env-lock.

## 4. Next-cycle context (ranked leads, recorded evidence only)

1. assess F2 env-race (sqlite_sessions.rs:1465 private ENV_LOCK vs lib.rs:2367-2369) —
   folded to unlanded twins rm-804 (3f6b86bc) / rm-818 (a2abf1a8); verify-first.
   [amended at review fix 5f237ee9, review 1de66b0c F2; CITATIONS CORRECTED at review
   fix 5fffefbb, review 2e23002e F3 (the original named a nonexistent tui
   statusline.rs): family members confirmed = core lib.rs:2367 sqlite_sessions tests +
   core src/statusline.rs:1236 journal_roundtrip_tolerates_a_torn_tail_line +
   core sqlite_sessions.rs:1456 failed_loads_neither_store_nor_trust_an_empty_snapshot
   — all 12x isolated green, never red serially; NO code change this phase (pre-existing
   at base, batch-disjoint).]
2. kimi wire state.json parser — ceded to 2e7758d3's unlanded rm-847 (same base); this
   base has zero stopReason/state.json reads (greps at research 34eeaef0).
3. zcode plan-provider vocabulary (ccusage #1831/#1832 OPEN) — rm-203 rider + 3f6b86bc's
   rm-806; pricing side landed via rm-852, recognition remains.
4. pricing bundle refresh — fbc4581f's unlanded rm-006/164/176 riders, corroborated by
   34eeaef0 (LiteLLM 4,504 live vs 3,100 bundled @2026-10-04; models.dev second source).
5. zh-CLI localization rm-850 (9d45a4ec) — composes with landed rm-853.
6. batch-selection verify-first rejects: rm-421 (90.0), rm-251, rm-195 — first-hand anchors
   before any mint next cycle.
7. watches: bare npm 'agenttrace' 404; opencode scoped-name re-probe owed; OTel
   semconv-genai 0 tags (rm-229 untripped); assess F6 TOCTOU folded to fbc4581f rm-843.
8. [added at review fix 5f237ee9, review 1de66b0c F3 — DEFERRED next-cycle mint
   (candidate rm-905+); MECHANISM CORRECTED at review fix 5fffefbb, review 2e23002e F2
   — the original claimed glm-5.3-air-class prices via the ladder, which is false:
   suffixed ids split into two disclosed classes (first-hand probe at spool
   5fffefbb…-scratch/probe-pricing-lanes.txt). The minor gate (match_variants,
   pricing.rs:1192) strips ONLY ≤3-char digit-led suffixes and '-pro', so
   glm-5.3-pro/-4o/-2 DO re-price via the ladder at the carrier rate — and
   pricing_source DOES carry the carrier note there — while
   carrier_resolution_note (pricing.rs:406) keys the EXACT normalized name, so the
   governance pricing_note lane alone stays silent for them; glm-5.3-mini / -air /
   -air-class never variant-match at all and price at the built-in fallback
   (3.0/15.0, pricing_status 'fallback_estimate' — disclosed, not carrier-priced).
   Next-cycle mint: make the note lane variant-aware + fix '1 carriers agree'
   pluralization + decide whether the -mini/-air fallback deserves an explicit
   'no carrier row exists' note. Deferred: an executable pricing.rs edit in
   review-fix would force a full-suite rerun the dispatch's required_scope 'none'
   does not authorize.]

## 5. Statuses

All four rows stay `implemented` — done-flips reserved to the commit gate (rm-012). Zero
ids minted at compound; def-row census immobile: 311 broad (297 rm-numbered + 14 legacy),
same-line statuses 155 candidate / 76 implemented / 66 done. Numeral landscape at compound
(live): worktree ceilings ≤ rm-860, fork def ceiling rm-899, spool unlanded above it =
rm-902 (this wall) + rm-903/rm-904 (075d70c67abf designation band) → next free rm-905
after the standard fresh census.

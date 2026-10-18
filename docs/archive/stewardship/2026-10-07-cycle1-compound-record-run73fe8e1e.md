# Cycle 1 compound record — repository-maintenance e687a0cd, run 73fe8e1e

Date: 2026-10-07 · Base: 1c5edd1e04d8a07fa552d4ff3ebf455392907550 ·
Worktree: run-73fe8e1e8a30-73fe8e1e · Compound attempt: 00b41217d8104cd8a42acd97ec15d2b2

Completed by attempt 3b730978ad924ad28bc645622393eecb: the 00b41217 attempt was
provider-reaped ~10 s after writing this record and the ROADMAP compound banner (no
spool result, no chain proof). The completing attempt verified every claim first-hand
against the phase logs and the tree, adopted the work, and corrected two defects: the
banner's targeted-core figure (377 → 381/0 — the targeted log's 19 core result lines
sum 381, tui adds 47/0 across 2 binaries, TOTAL 428/0; validation-record.md line 16's
'377' is a section miscount that dropped zero_usage_contract's 4 and a 0-test binary),
and this record's rm-715 header (ci-integrity 30.0 → compatibility 58.0, the minted
row's actual track/priority).

This record compounds pre-review cycle evidence only. Test outcomes below are CONSUMED
from the recorded targeted_tests (6834cb3c) and full_tests (f1b153ef) results — nothing
was re-run at compound per the phase contract.

## What the cycle delivered

Batch 'Truthful usage accounting across parse → cache → report', one atomic change unit
(stewardship 22b7279b decision), 7 modified + 4 untracked paths:

- **rm-711 (LEAD)** — occurrence-aware usage accounting. Codex: every `compacted` marker
  opens a fresh accounting envelope (value-dedup ledger AND delta baseline reset); the
  re-sent context counts once via its fresh `last` snapshots; the rm-554 rate-limit-only
  guard survives as an explicit consecutive-duplicate check ahead of set-membership;
  the ledger is bounded (1024 distinct totals per envelope, rolling over — assess SL3).
  Qwen: the `result` boundary releases the session-wide first-wins usage latch; assistant
  usage still wins within a turn. Oracles: tests/usage_occurrence_contract.rs pins
  1500→2700 input tokens (codex revisit) and 10→30 (qwen 3×(10/5)); PoC corpora vendored
  byte-identical at tests/fixtures/{codex-revisit,qwen-multi}/.
- **rm-710** — SESSION_CACHE_SCHEMA_VERSION 32→33. One invalidation covers both defect
  classes: pre-rm-526 v32-era cache entries replaying hidden line_skips at confidence
  high, and pre-rm-711 entries holding under-counted totals. Same-unit set moved in
  lockstep: const + ladder rung, TUI warm-cache fixture literal, governance-guide
  sentence (the check-docs-commands gate target), discovery-contract pins read the
  exported constant.
- **rm-714** — delivery_evidence disclaimer hoisted to a document-level confidence_note;
  rows carry the bare confidence word (24,501×~155-char duplication eliminated).

Recorded outcomes consumed as evidence: targeted 428/0 + fmt/clippy rc0 + docs schema
leg; full 21/21 ci.yml full+deny+MSRV lanes, 538/0 across 27 result lines (baseline 533
+ 5 new oracles), release binary 13,517,688 B left for the commit gate. Validation digest
validation:v1:42740398213dc95635d30f0a4789c61f608a162429ff6c31dfd38d8ff3ffdc8f re-derived
PRE==POST==dispatch at both validation phases.

## Reusable lessons / prevention rules

1. **Dead-attempt economy (adoption gate).** A provider-dead attempt with a durable
   worktree trail and no result JSON can hold real, verifiable work. The gate is
   first-hand re-verification — this cycle: park the delta as a patch, check out the
   base, prove the oracles RED, reapply byte-identically (cmp). But lineage identity
   (run + cycle + base) rules out look-alike artifacts: the on-disk research artifact
   d48c1d8b belonged to a different run/cycle/base and was used only as a corroborating
   probe. Reaped-in-<60s attempts (research 13674dcf 25s, prioritize 49796722 63s) had
   zero durable output — redo was correct there. Check the event log's shape, not just
   its existence.
2. **Name the token class in PoC oracles.** The assess phrased its codex oracle as
   '2000 vs 2700 tok' (input+output combined); red-first proved the honest input-only
   oracle is 1500→2700. Without the red-first step the implement would have minted a
   self-satisfying wrong oracle. Rule: every usage PoC names input, output, or combined;
   the red run is where phrasing drift is caught.
3. **Schema-bump same-unit set is institutionalized** (cycle-3 lesson, executed cleanly
   this cycle — zero red-first catches needed): SESSION_CACHE_SCHEMA_VERSION moves
   together with the ladder rung, the TUI warm-cache fixture literal, the governance
   guide sentence, and any dynamic discovery pins. Anyone bumping the constant runs the
   four-point checklist.
4. **Validation-scope classifier artifacts (twice-confirmed).** A delta that is 100%
   crates/** Rust + .md yields changed_testable_surfaces=[] → required_scope 'none'
   at targeted_tests; an empty dispatch full_command means the engine skips its
   command-match and the authoritative suite is the ci.yml full+deny envelope (+ MSRV
   floor when the toolchain is installed). Precedents: 7e00d9cbe20a, 614624d7,
   7eae74ea, this run's 6834cb3c + f1b153ef.
5. **Digest stability is surface-set-derived.** validation:v1 digests derive from the
   changed-file-path set vs base, not diff bytes: md-only appends (banners, records) and
   edits within the already-changed set are digest-immobile; validation-only turns must
   still prove PRE==POST==dispatch. Note: ADDING a file (this record) grows the surface
   set — the commit-gate dispatch derives fresh, which is expected.

## Next-cycle leads (evidence already staged)

1. **rm-712** — semantic self-eviction at the session-cache byte cap (perf 74.0).
   Evidence: assess 9ee2 N1 (8m10s cold / 7m48.7s warm at 17,887 sessions; 14% warm
   hit-rate; sessions.json 67,098,942 B vs 67,108,864 B cap — session_cache.rs:190/:202/
   :662-688, discovery.rs:331; /tmp/at-assess-9ee2/{ov1,ov3}.json).
2. **rm-713** — sqlite snapshot multi-DB collapse (reliability 72.0). Evidence: assess
   9ee2 N2 (9.1MB → 232B; single-DB fingerprint at session_cache.rs:676-678/:713-740,
   sqlite_sessions.rs:77-86/:225-246; scratch-HOME 2-DB PoC).
3. **rm-715** — gemini-cli v0.63 transcript-rework watch (compatibility 58.0); research
   RC lane, fork-solo.
4. Assess N4 (doctor 0-byte timeouts) is folded into the existing rm-367 row — the
   dossiers remain the evidence source.

## Commit-gate seams (do not lose)

- Stage the four untracked paths explicitly (`git commit -am` drops them):
  tests/usage_occurrence_contract.rs, tests/fixtures/codex-revisit/,
  tests/fixtures/qwen-multi/, and this record.
- Integration order at landing: apply the e64d963c roadmap patch first, then this delta.
- Title-adjacent reconciliation (merge by TITLE, never id): 13b439bc rm-685 'codex
  occurrence ledger' (title-overlaps rm-711's codex arm — different mechanics, ours
  landed); 3ee2af04 rm-693-698 double-mint vs the de600e73 rm-693-707 band; bands
  8e983cf5 rm-685-692 and de600e73 share numeral ranges.
- rm-709 stays candidate pending retire-or-fold at integration (superseded by landed
  rm-584 on mainline e9e8fd9 — its fast-path arms were rewritten there).
- Done-flips for rm-710/711/714 reserved to the commit gate (rm-012 convention).
- The pre-existing codex compaction hwm pin was re-based 9500→10500 with a dated
  comment — legitimate oracle maintenance, visible in the diff.

## Trail (spool)

Result JSONs + scratch: delegate/{fdd92642,6834cb3c,f1b153ef,3b730978}*.json and
…-scratch/ (00b41217 left an event log only — events/00b41217….jsonl, 6 events,
reaped provider-failure; its durable output was this record + the ROADMAP banner,
adopted-then-corrected by 3b730978, whose scratch carries the as-written snapshots);
dossiers /tmp/at-assess-{6d57,9ee2}/, /tmp/at-research-1589/; runner
/tmp/at-full-f1b153ef/run-full.sh (mirrored in the full_tests scratch log).
Chain proof (3b730978): HEAD:ROADMAP.md → roadmap-e64d963c.patch → implement riders →
compound banner + correction, each hop git-apply-verified and byte-compared —
delegate/3b730978…-scratch/chain-verification.log.

# Cycle 3 compound record — run 1a4cce3a (2026-10-15)

**Status:** COMPLETED. Compound attempt `5b96b1c108664ca0a6e1410000964a9f` (prior compound
attempt `85c992f3016a4c0fac57e96de7c78e09` died mid-turn provider-side at message_count 15 —
delegate typed artifact absent, `results/85c992f3….json` is a 309-byte FAILED tombstone with no
payload, scratch absent, ZERO worktree drift beyond the implement batch → full redo from
scratch, declared here).
**Scope:** repository-maintenance `ec4f7625d1df447783cf3c5b32200fe0` cycle 3, base `3e90993`,
worktree `run-1a4cce3a6162-1a4cce3a`, branch `conductor/run-1a4cce3a6162`.

## Contract honored

- Compound only: consumed pre-review cycle evidence (assess / research / roadmap / prioritize /
  stewardship / implement / targeted_tests / full_tests envelopes + typed artifacts); NO test or
  validation command was executed this phase; no final_validation / commit / push / pr / ci.
- Required evidence delivered: ROADMAP.md band IN-TREE (+42/-0, strictly additive), this record,
  and a dated extension to `docs/workflow-issues/provider-dead-attempts-may-leave-real-worktree-diffs.md`.

## The cycle's batch — "one truth on the money-and-cache path"

Selected by prioritize `fcb4b749` (prior attempt `19ea1660` provider-dead at msg 3, redone),
spec'd by stewardship `02059d97` (prior attempt `81482577` provider-dead at msg 2, redone),
implemented by `1013d2a8` — which ADOPTED provider-dead attempt `0fdf4495`'s worktree-only
5-file delta after a line-by-line audit against the stewardship spec, then extended it.

- **LEAD rm-1388 (correctness 70.0, minted by roadmap `feae96d5`):** `capture_cost_usd`
  (statusline.rs:1573-1575 at base) trusted journal-authored `total_cost_usd` with none of the
  `is_finite() && >= 0.0` validation the parser's `oh_my_pi_recorded_cost_usd`
  (parser.rs:2670-2678) enforces — a planted −$50 sample flipped a $10 weekly budget to
  "7d total $-47.00" / "$57.00 remaining (100%)" (assess `ecf1d292` live PoC). Fix: ONE shared
  predicate `journal_cost_usd_is_valid` used by both call sites; `invalid_cost_samples`
  disclosed on all four render lanes (text :1275/:1432, json :1833/:1878); 4 red-first
  hostile-journal tests (−$50 flip replay, NaN, ±inf/1e999, alternating-sign span).
- **RIDER rm-1389 (reliability 30.0):** `resolve_user_cache_dir` existed as four hand-synced
  private copies (session_cache.rs / doctor.rs / pricing.rs / statusline.rs), drifted twice in
  three days. Fix: ONE `pub(crate)` definition at session_cache.rs:3250, three importers,
  base bodies proven diff-identical (4×29 lines) before deletion.
- Tree delta: 6 tracked files +214/−141 — parser.rs, statusline.rs, session_cache.rs, doctor.rs,
  pricing.rs, CHANGELOG.md (Unreleased `Fixed` + `Changed` entries). Batch diff md5
  `3ca084ac57d2a86d41a0e8348f199d88` — byte-identical before and after this compound phase
  (verified in/out; the 6 batch files were not touched).

## Validation consumed (recorded evidence, NOT re-run per the compound contract)

- **Targeted `5222f9a6`:** core 826/0; tui --lib 15/0; fmt rc0; clippy −D warnings rc0;
  changelog-structure rc0; 4 scoped binary gates rc0; nothing modified in/out → dispatch digest
  declared VERBATIM per the emission-time rule.
- **Full `b2eace9d`** (redo of provider-dead `ccc865cc`; empty `full_command` → suite derived
  VERBATIM from `.github/workflows/ci.yml` per fleet precedent bde3b724): 24 lanes green at
  private target `/var/tmp/at-ft-b2ea` — fmt / clippy-trio / test-trio 62 suites **1122/0/0**
  (baseline 1118 → +4 batch tests, zero regressions) / MSRV 1.89 / conformance 13-case runner /
  release build / static 11 + bash-n / npm / binary gates 5 / real-cli 20 files / real-tui 55
  files.
- **Two pre-existing disclosed reds at clean HEAD (fleet-owned, not regressions):**
  `check-install-runtime` rc1 test D only (rm-012 family; rm-051 is fleet fog, no def row on
  this wall) and `cargo deny` = 4 rustls RUSTSEC 2026-0346..0349 (bans/licenses/sources ok).

## Band provenance — the reaped-scratch rebuild

The roadmap phase (`feae96d5`) delivered its band spool-side (postimage + patch, worktree
untouched). By compound time BOTH the scratch dir and any `/home/agent/.cache` mirror were
reaped; the phase envelope `results/feae96d5948d4cd580debbec75738bb3.json` is the only
surviving authority. The roadmap layer of this band is REBUILT from that envelope,
subject-by-subject (2 mints + 17 riders + 1 banner), and declares the rebuild in its own
banner; byte-identity with the reaped postimage (patch md5 `eaf0f61f…`) is unreproducible by
construction. Compound layer on top: 1 banner + 7 riders, ZERO mints, ZERO flips (rm-012
convention: rm-1388/rm-1389 status flips + strikes ride the commit gate).

Wall invariants, all PASS (builder log in spool scratch `verification.txt`): def rows
392 → 394; zero duplicate ids; def-max now rm-1389; every base line survives (ordered
subsequence); insertion-only accounting +42 lines; managed render footer stays last; banner
order compound → roadmap → INTEGRATION (newest-landing-first); riders sit inside their owning
blocks (rm-201 ×2, rm-012 ×2, rm-1388 ×2, rm-1389 ×2, rm-436, rm-1370 ×2 checked).
Postimage md5 `d97c31a71af3ff6bc614585bc35681b7`. A first build misplaced the rm-1388 mint
between rm-1020's title heading and its id line; the builder's heading-anchor rule was fixed,
ROADMAP.md reverted to clean and rebuilt — the postimage cited here is the corrected one.

## Provider-death + reaping ledger (one cycle, five deaths)

| phase | dead attempt | died at | durable trail | successor action |
|---|---|---|---|---|
| prioritize | 19ea1660 | message_count 3 | event log only (4 lines) | redone from scratch (fcb4b749) |
| stewardship | 81482577 | message_count 2 (~23 s) | event log only | redone from scratch (02059d97) |
| implement | 0fdf4495 | mid-batch | WORKTREE DIRTY DIFF ONLY | ADOPTED after line-by-line audit (1013d2a8) |
| full_tests | ccc865cc | message_count 2 | none | redone from scratch (b2eace9d) |
| compound | 85c992f3 | message_count 15 | results tombstone only (FAILED, 309 B) | redone from scratch (5b96b1c1, this phase) |

## Lessons (durable)

- **L1 — adoption after forensics:** a provider-dead attempt may have finished real work; the
  worktree dirty diff is often the ONLY durable channel. Audit line-by-line against the spec
  before adopting; verify identity/test-provenance/tree-census/cleanliness; declare the
  adoption. (Extended in the workflow-issues doc.)
- **L2 — reaped scratch:** the conductor janitor reaps `*-scratch/` (and `/home/agent/.cache`
  mirrors) mid-cycle, before the successor phase runs. The phase envelope is the only band
  authority; rebuild from it, never from memory, and declare the rebuild in-band.
- **L3 — VERBATIM digest rule:** a validation phase that modifies nothing declares the dispatch
  digest VERBATIM per the emission-time rule (targeted and full both did this cycle).
- **L4 — private targets:** shared `CARGO_TARGET_DIR` hosts reap/sibling-shadow build trees;
  full batteries run at a private clean-room target (`/var/tmp/at-ft-b2ea` this cycle), and an
  empty `full_command` means the suite is ci.yml VERBATIM (bde3b724 precedent).

## Next-cycle candidates (concrete, pre-review; next assessment carries review/shipping outcomes)

1. **rm-164 tier-aware pricing** — research `821d9b9f` pinned the contract: tiers at
   `cost.tiers[]`, entry keys `{input, output, cache_read?, cache_write?, reasoning?,
   input_audio?, tier:{type:context,size:N}}`, 35 multi-tier models in models.dev `3d70bf7f`.
2. **rm-176** LiteLLM regional churn 182→194 (+12 intra-day, keys flat 4,083);
   sonnet-4-20250514 au/us-gov still absent (assess-F3 coverage gap).
3. **rm-1053** pricing.rs:1340-1367 twin ladder collapse (still standing, no drift).
4. **rm-1054** doctor.rs:642-647 banking twin (live).
5. **rm-201** serde(skip) timestamps (lib.rs:527) + the codeburn #725 local-JSONL-fallback
   demand (loudest unserved ask, rm-777 quote).
6. **rm-501** 12 ungated cfg(unix) E0433 functions.
7. **rm-715 + rm-758** gemini-cli v0.64.0 released STABLE (10-13T20:44Z) — schema-bump
   trigger ARMED.
8. **rm-776** provider cohort: openclaude + kilo-code near-free schema reuse; grok journal IS
   an ACP log; sqlite cohort (cursor store.db, warp) still unprovenance.
9. **rm-1388 follow-on (new, this record):** `invalid_cost_samples` disclosure now exists on
   the budget/statusline lanes — extending the same disclosure discipline to other
   journal-authored numeric fields (durations, token counts) is the natural next truth-lane.

## Commit-gate seams (NOT executed here — commit/push are later gates)

- **Stage exactly 9 paths:** the 6 implement files (crates/agenttrace-core/src/{parser,
  statusline, session_cache, doctor, pricing}.rs + CHANGELOG.md) + ROADMAP.md +
  docs/stewardship/2026-10-15-cycle3-compound-record-run1a4cce3a6162.md +
  docs/workflow-issues/provider-dead-attempts-may-leave-real-worktree-diffs.md.
- The band is IN-TREE (fleet precedent ff7615be): nothing to apply. Alternative seam if a
  pristine-tree re-apply is needed: spool scratch `band.patch` (base 3e90993 → postimage) —
  after any apply, run the census trio (394 rows / zero dups / footer last).
- rm-1388 + rm-1389 status flips to implemented and any row strikes ride the commit gate
  (rm-012 convention), citing the EXECUTED/validated riders landed here.
- **Live-wall fog at emission:** local `origin/master` = `d2a02af2` ("merge: autonomously
  integrate run a64735b75a46…"), 3e90993 is its ancestor; its wall foot = 383 open rows with
  ZERO rm-1388/rm-1389/1a4cce3a mentions (no twin — the sibling integration struck its own
  rows). Push reconciles by title per the rm-708 convention; fleet-ledgered numerals are never
  renumbered post-hoc.
- Standing reds stay disclosed at the gate: check-install-runtime test D (rm-012 family) and
  cargo-deny 4 rustls RUSTSEC advisories.

## Artifacts

- Spool scratch `delegate/5b96b1c108664ca0a6e1410000964a9f-scratch/`: `band-builder.py`
  (rebuild + verification), `ROADMAP.postimage.md` (md5 d97c31a7…), `band.patch`,
  `verification.txt` (15 PASS lines), `APPLY-SEAM.md`, `forensics.txt`.
- Durable mirror `/home/agent/.cache/at-compound-5b96/` (postimage + verification + patch).
- Input envelopes: `results/{ecf1d292,821d9b9f,feae96d5,fcb4b749}.json`;
  `delegate/{02059d97,1013d2a8,5222f9a6,b2eace9d}*.json`; full-validation record at
  `delegate/b2eace9d71a94de9b14c675897ae3015-scratch/full-validation-record.md`;
  prior-attempt tombstones `results/85c992f3….json` + `events/85c992f3….jsonl`.

## Post-script — re-delivery adoption (attempt `31a415031fa840ada09b961dfc141e6e`, same day)

The compound attempt above (`5b96b1c1`) was itself reaped provider-side ("This operation
was aborted") ~1 minute AFTER its final durable write (typed phase_result 06:41Z, results
entry 06:42Z), so the engine marked the attempt failed and re-dispatched the phase.
Re-delivery attempt `31a41503…` ADOPTED the completed work after full re-verification,
changing nothing already delivered:

- HEAD `3e90993`; porcelain exactly 8 M + 1 ?? (the 6 batch files + ROADMAP.md + the
  workflow-issues doc + this record); batch-only diff md5 still
  `3ca084ac57d2a86d41a0e8348f199d88` (implement batch untouched by both attempts).
- In-tree ROADMAP.md == spool postimage == `.cache` mirror, md5
  `d97c31a71af3ff6bc614585bc35681b7`; +42/-0; census 394 def rows / zero dups / def-max
  rm-1389; banner order compound → roadmap → INTEGRATION; managed footer last; rm-1388
  @2340 / rm-1389 @2189 with EXECUTED/validated riders in-block.
- This record measured 151 lines at adoption (the "159 lines" figure in the 5b96b1c1
  typed result's evidence line was an inaccurate count; the file was complete, never
  truncated). The workflow-issues doc gains a rule-5 post-script block for the sixth
  death — the cycle's ledger is now SIX provider deaths (5b96b1c1 appended below its table).
- ROADMAP.md was deliberately NOT edited at re-delivery: its postimage md5 is pinned by the
  commit seam, and the band's attribution to 5b96b1c1 is the true provenance of the work.

ZERO tests executed by either compound attempt (phase contract; targeted 5222f9a6 and full
b2eace9d outcomes consumed as recorded evidence).

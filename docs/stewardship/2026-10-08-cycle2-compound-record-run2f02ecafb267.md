# Cycle 2 compound record — run 2f02ecafb267 (repository-maintenance 78307520)

Recorded 2026-10-08 by the compound phase (attempt 956390c9eb87402fa5fe96133dae1892) from
pre-review cycle evidence only: assess 1177e59f, research d48c1d8b, roadmap 13b439bc,
prioritize f8123386, stewardship e1cdff4d, implement 4087839767, targeted_tests 7a369fc884,
full_tests f2d9ae0bdaf. No validation was re-run at compound (phase contract); every outcome
below is the recorded pre-review outcome of the cited phase.

## Batch — 'usage-accounting truthfulness closeout' (ONE change unit)

- **rm-685 (LEAD, correctness)** — Codex totals ledger re-keyed from journal-wide value
  dedup (`seen: BTreeSet`, unbounded) to occurrence-adjacent freshness: the immediately
  previous event's total + last snapshot, plus a capped 1024-entry value-dedup ring for the
  legacy no-snapshot delta path (`CODEX_RECENT_TOTALS_CAP`, ~1.4 MB steady-state). Fixes
  all three PoC'd failure modes: F1 value-equality window-drop (post-compaction total
  value-equal to an earlier one dropped), F5 same-last double-count, F2 unbounded `seen`
  growth (~1.4 KB/entry on 400k-distinct-total journals). Goldens: collision 1200/400 →
  1500/500, same-last 200/100 → 200/80; memory see "Events-matched control" below.
- **rm-686 (fold, correctness)** — generic-extractor Gemini-inclusive alias netting:
  `promptTokenCount` flagged cache-inclusive, `cachedContentTokenCount` added to the
  cache-read aliases; when the inclusive alias wins the input class, the shared clamped
  `subtract_cached_input` nets it. Golden: 900/0/70 → 100/800/70. Acceptance clause 3 met
  by a self-inventory test enumerating the input alias class beside the inclusive flag.
- **rm-602 landed-row extension (fold)** — `qwen_stats_usage`/`qwen_model_usage` now take
  the FIRST PRESENT alias per class per model (`add_first_alias`); dual-alias snapshots
  200/100 → 100/50. Extends the landed first-present-alias rule to the stats arms; no new
  numeral (independently adjudicated == rm-602 by sibling assess c90a0f00).
- Same-unit riders: SESSION_CACHE_SCHEMA_VERSION 32 → 33 with the sweep set (const +
  slot-ladder comment + TUI warm-cache fixture literal + governance-guide sentence —
  verified green by `check-docs-commands.sh` against the live constant), CHANGELOG
  Unreleased bullet, 4 new golden fixtures under tests/fixtures/usage-accounting/.

## Recorded outcomes (pre-review, NOT re-run)

- **targeted_tests 7a369fc884** — 338/0 across the 8 impacted core suites (lib 204,
  discovery_contract 89, usage_accounting_truthfulness 15, + codex_compaction_verdict /
  zero_usage_contract / report_numeric_truthfulness / parser_property_invariants /
  pi_usage_tree_accounting), tui 47/0, `cargo fmt --all --check` rc0, `cargo clippy
  --workspace --all-targets -- -D warnings` rc0. Dispatch block carried `required_scope:
  none` + empty targeted_command (crates/** classifier artifact) — covered_surfaces were
  enumerated explicitly from the implement delta.
- **full_tests f2d9ae0bdaf** — `cargo test --locked --workspace` (dispatch full_command
  EMPTY; this is the repository's authoritative full-suite envelope, the same command behind
  the assess phase's 527/0 record at this base): **533 passed / 0 failed across 25 suites**
  = 527 at-base + exactly the 6 new batch tests (4 fixture + 2 in-module). fmt/clippy rc0.
- **Digest** — `validation:v1:79d4ac01594d645ca572df9a4a34d986a15f4d05010a0d0aab5f41e669ab06ec`
  byte-identical across the implement / targeted_tests / full_tests dispatches and re-derived
  live at each phase (docs-only deltas — ROADMAP, records — are digest-immobile under the
  sh/py/ps1/yml classifier; crates/** deltas likewise). No executable surface changed after
  implement, so no re-emission was ever required.

## Implement forensics — 8th reap-after-work instance, adopted

Implement was dispatched twice. The first attempt **af78cef7** died of a provider
infrastructure failure at 18 messages with NO typed result artifact — but with the complete
batch delta already in the worktree (correct run/action/base lineage). The redo 4087839767
verified the durable trail (event log present, tree delta == exactly the batch surfaces),
re-verified every leg from scratch (all gates, CLI goldens, memory measurement), and adopted
the tree rather than redoing it; the one gap (implement-phase ROADMAP row annotations,
mandated by the stewardship contract) was completed by the redo. Fleet instances of this
class: 5th run f7f81aea, 6th run 32f3b7a1, 7th run a7110bf0, 8th this run. The adoption
protocol — lineage check + tree-census match + per-leg independent re-verification + gap
inventory — is now the settled response; an absent envelope is never evidence work didn't
happen.

## Events-matched control — the measurement lesson (NEW prevention rule)

The assess's memory pair (661/105 MB) and the first post-fix pair (464/99 MB) each differ in
TWO variables (ledger cardinality AND counted-event load), so neither can attribute the
residual. The isolating third leg — 400k events ALL counted on only 10 distinct totals —
measured 487 MB post-fix, ABOVE grow's 464 MB: proof the distinct-total-proportional
component is eliminated, and attribution of the residual ~370 MB to per-counted-window
`Event` accumulation (pre-existing, format-wide, out of scope). Recorded as prevention rule
`docs/solutions/workflow-issues/memory-parity-acceptance-needs-an-events-matched-control.md`
(one variable per pair; acceptance clauses must name their control shape; archive the
generator because /tmp probes get swept — this cycle's assess probes vanished before
implement, and the golden small fixtures had to be committed rather than referenced).

## Prevention rules / lessons carried forward

- **PR (schema bump sweep, honored):** the 32→33 bump swept the full pin set at implement
  (const, TUI fixture, guide sentence) and `check-docs-commands.sh` verified the live
  constant — zero red gates. The standing rm-230 rider (cargo-test pin binding the guide
  sentence to the constant, effort S) is STILL unlanded and remains the cheap way to move
  this class from CI-scope to targeted-scope failure; 4th cycle observing the surface.
- **PR (empty dispatch commands):** both test phases carried empty targeted_command /
  full_command (crates/** classifier artifact). Response recorded twice more: enumerate
  covered_surfaces explicitly from the delta; full scope runs the repo's authoritative
  envelope, not a local script substitute.
- **PR (probes in /tmp are volatile):** commit golden PoCs as repo fixtures at implement;
  archive generators for the big ones. Done this cycle (4 goldens in-tree; memory-probe
  generator + VmHWM poller in 4087839767-scratch).

## Next-cycle leads (concrete)

1. **Per-counted-window `Event` accumulation** (~0.9 KB/counted window; ~370 MB at 400k
   counted windows in ANY format, measured this cycle with the events-matched control):
   aggregation-during-parse or a compact per-window record is the candidate; the measurement
   method and probe shapes are in the prevention rule + implement scratch.
2. **Held-separate unlanded sibling lanes** (must_remain_separate from this batch,
   all parser-adjacent, title-disjoint): 36c9140f service-tier pricing batch (on disk),
   7e00d9cb rm-683/rm-684 surface honesty (XDG env arms; statusline single-pass budget view —
   this run's independent budget double-read PoC corroborates rm-684), c762c2b8 upstream-#305
   port hunks (parser.rs :3424-3431/:4187 — disjoint from rm-685's ledger hunks), a0d0d353
   sqlite-snapshot, rm-655/656 qwen multi-result latch (our qwen-multiresult 10/5 pin is
   landing-gate corroboration).
3. **Pricing drift triggers** (research riders landed on rm-006/rm-176): bundled snapshot
   3,099 @2026-10-04 vs 4,481 live LiteLLM keys (+1,384 net-new, 626 priced-new, 18 price
   mutations); refresh wave when the next key-delta trigger fires. Upstream is quiet past
   #318 — nothing to port.
4. **rm-230 cargo-test schema-sentence pin** (see above; smallest S-effort red-first win
   on the board).

## Commit-gate seams

- The landing delta is 7 M + 6 ?? — the ?? set (4 fixtures, the cycle record, the
  prevention-rule doc) MUST be staged explicitly; `commit -am` drops untracked files
  (lesson re-learned fleet-wide at runs 7caf7f05/330066fa).
- Status flips rm-685/rm-686 implemented→done are reserved to the commit gate (rm-012);
  the ROADMAP delta lands with the batch: +26 campaign lines, 3 implement annotations,
  the validated one-liners + banner from this compound.
- Schema ceiling: if the merge head already carries 33 from another lane, this run's bump
  re-bases 33→34 and the whole pin set moves with it (one-invalidation rule; integrated
  ladder comment in session_cache.rs is the anchor).
- Numeral hygiene: zero mints at compound; rm-685/rm-686 were minted at the roadmap phase
  from a live fleet census; NEXT FREE rm-687 after a fresh census at the next roadmap phase.

## Status accounting

Grep census at compound (pre-flip): 240 def rows — 138 candidate / 59 implemented / 44 done
(241 status-bearing lines incl. one quoted banner mention). The batch rows were flipped
candidate→implemented at compound per the rm-012 convention (done-flips are the commit
gate's alone); post-flip census 136 candidate / 61 implemented / 44 done.

## Adoption note (redo attempt 0a9f866a83b44b0788e0683d9bba5bd8)

This record was authored by compound attempt 956390c9eb87402fa5fe96133dae1892, which was
provider-reaped AFTER writing it (typed result never emitted; 7-line event log ends
`delegate_turn_completed` status failed). The redo verified the lineage
(run 2f02ecafb / phase compound / action compound:compound — match), re-verified every
claim live against the tree and the durable phase scratch, and completed exactly one gap:
the rm-685/rm-686 status fields still read `candidate` beneath the `validated`
annotations — flipped candidate→implemented above (done-flips stay with the commit gate).
Legs re-verified first-hand: 240 def rows / 0 duplicate ids / max rm-686 / managed footer
last / banners newest-first; targeted-core.log sums 338/0 and targeted-tui.log 47/0;
full-test.log 533/0 across 25 suites; implement scratch carries implement-verification.md
+ vmhwm.py as cited; porcelain at adoption 7 M + 6 ?? with HEAD 518170a unchanged.

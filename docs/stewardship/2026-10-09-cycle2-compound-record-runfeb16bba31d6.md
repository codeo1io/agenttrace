---
title: Cycle 2 compound record — run feb16bba31d64504869b4db782b0c8d3
repository-maintenance: fe743fb3399f4df68940183a2956fbf5
cycle: 2
date: 2026-10-09
base: 96b528ec166c43f9a20b4553001d16241b2bb622
compound-attempt: 9b5871e448264e728a523c2005caa118
scope: pre-review (consumes only pre-review cycle evidence; no validation re-run)
---

# Cycle 2 compound record — run feb16bba31d6 ("journal-truth composition repair")

## Outcome in one paragraph

The cycle's fresh mint **rm-856** (roadmap e3238f2a, priority 88.0) was
implemented solo, red-first, at base `96b528e` and validated green: full
suite **619 passed / 0 failed** (machine-sum; 22/22 ci.yml lanes rc0 — the runner's own SUMMARY table has 22 rows; the earlier 23 counted the bash `run()` helper twice) at
digest `validation:v1:3ff84bb4822336a50acc7ff7541cd490c49d2254be64a3bf40c0ebc150fda4a3`,
still current for final_validation (no executable surface moved after
implement). The row is flipped candidate→implemented in the cumulative
roadmap patch (done-flip reserved to the commit gate, rm-012). Code unit
is UNCOMMITTED in the run worktree: 8 modified files + vendored
`tests/fixtures/rm-856-composition/`.

## The decisive reframe of the cycle

The adversarial assess (ac7d1d53) found F1–F7 with a RED full battery
(605/1 headline — see the reconciliation lesson below) and the roadmap
minted rm-856 for the F1+F2 "composition seam". Wall forensics at
prioritize (62f09e34) then showed the landed wall `origin/master =
5ed9ebc` was **6+ merges ahead of our base** and had already decided
rm-856's acceptance arm (1): integration conflict case 15bdfe5f had
re-pinned `relocated_usage_keys_disclose_instead_of_silent_zero` to
`len()==3` + `usage_present_not_counted == Some(&1)`. The batch
therefore **adopted** the landed re-pin (porting only test hunk
`@@ -486,10 +545,25`) instead of re-deciding — the opposite direction
would have forked doctrine and forced an integration conflict — and
spent the budget on what was still live: the composition double-count.

## What landed (implement 439443bd, red-first)

- **Policy**: when a generic-lane session carries usage on a meta-role
  line, the **meta arm is the counted lane** (the meta block already
  aggregates the conversation; native-lane precedent
  `stray_conversation_usage_disclosed_beside_meta_usage`); the rm-616
  generic fold **stands down** — gated on the whole-slice
  `has_meta_usage` pre-scan, making the decision order-independent —
  and stood-down conversation lines disclose via the existing
  `usage_present_not_counted` channel instead of double-counting.
- **Pins** (`tests/generic_model_usage_truth.rs`):
  `meta_usage_makes_the_meta_arm_the_counted_lane_not_a_double_count`
  and `meta_usage_composition_is_order_independent` over the vendored
  assess PoC reconstructed verbatim (`fixtures/rm-856-composition/
  {double,double-reversed}.json`; the /tmp original had been swept).
  Pre-gate red at the exact PoC values: tokens_input 107 vs pinned 100.
- **Healed**: the cycle's original RED suite went 11-passed/1-FAILED →
  12/0 by the adopted re-pin.
- **Schema**: 33 → 34 (rm-230 convention) with pin-sites realigned —
  session_cache const + bump comment, tui warm fixture
  (`crates/agenttrace-tui/src/tests.rs:1676`), governance guide
  sentence, docs-gate live check at 34. **Wall schema advances under
  us** — 38 at prioritize time (5ed9ebc), 41 by review time (9c3c599,
  2026-10-08 21:24:30Z; next free 42): the gate RE-DERIVES the live
  ceiling and re-bases the bump onto it per the 202c4d1d convention.
- **Records**: CHANGELOG bullet atop `### Fixed`; dated correction
  APPENDED to
  `docs/stewardship/2026-10-07-cycle2-compound-record-run91833f02565b.md`
  (its "core 329/0" retest line was faithful to its lane but not to the
  merged tree, which was red until 15bdfe5f).
- **E2E probe**: `--overview` total_tokens 150 (pre-fix 160),
  `--doctor` discloses `usage_present_not_counted: 1`.

## Phase ledger (evidence consumed here, not re-run)

| phase | attempt | outcome |
|---|---|---|
| assess | ac7d1d53 (redo of dead a652fa98) | F1–F7; battery headline 605/1 (ok-only grep) |
| research | c89b2e02 (pass 12) | R1–R7; MCP 2026-07-28 stateless redesign headline |
| roadmap | e3238f2a (8th dispatch; 7 provider-dead) | minted rm-856; spool patch + postimage, worktree pristine |
| prioritize | 62f09e34 (redo of dead aaeecabb) | wall forensics; batch = rm-856 solo; 5 rejections cited |
| stewardship | 999144a3 | one change-unit, conductor owns topology |
| implement | 439443bd | red-first → green; delta 8 M + fixtures |
| targeted_tests | eda83f1a | validation-only 378/0; scope 'none' (crates/** quirk); digest byte-stable |
| full_tests | 00ee2c37 | 22/22 lanes rc0; 619/0 machine-sum; digest verbatim |

## Fleet lessons

- **L1 — check the landed wall's aheadness before implementing a mint.**
  The assess (on a base 6+ merges behind) framed F1 as open; the wall
  had already closed it. Forensics at prioritize cost minutes and saved
  a doctrine fork. Corollary: re-derive the CURRENT wall tip before any
  semantic re-decision.
- **L2 — full-suite headlines undercount when a suite fails.** 605/1
  was an ok-only grep; executed baseline was 616; 616→619 = +2 pins
  +1 healed. Prevention doc:
  `docs/solutions/workflow-issues/full-suite-headlines-undercount-failed-suites-passes.md`.
- **L3 — verify an assess finding's exact citations before building a
  rejection on them.** F6's SHAPE verified (the mirror is real and
  documented: `mcp.rs:53` `RECENT_SESSIONS_LIMIT` <-> core
  `reports.rs:491` private `RECENT_SESSIONS_MAX`, tied by comment); what
  did NOT verify was the roadmap's rm-455 append claiming 'defined twice
  (mcp.rs:25 and mcp.rs:186)' — line refs drifted with the wall and the
  'defined twice' premise was wrong. The dedup-work rejection stands on
  the corrected ground (mirror is real, comment-tied, harmless), not the
  garbled one; the rm-455 citation is corrected in this cycle's
  cumulative roadmap patch.
- **L4 — /tmp PoCs get swept; vendor fixtures immediately.** The
  double.json PoC was reconstructed from findings.md verbatim; had the
  shape not been recorded in prose, the pin would have been guesswork.
- **L5 — schema bumps have a site checklist**: const, bump comment, tui
  warm fixture literal, governance sentence, docs-gate live check,
  discovery-contract dynamic pin. Miss one and a lane goes red.
- **L6 — raw-string test fixtures: edit the minimal unique substring.**
  A full-line replace of the tui fixture dropped a quote and broke
  compilation; the focused battery caught it, `git checkout --` +
  1-line edit recovered.

## Commit-gate seams (for the next phase, not this one)

1. Apply the **cumulative** patch
   `delegate/9b5871e4…-scratch/roadmap-delta-CUMULATIVE.patch`
   (= roadmap e3238f2a + the compound c2 banner + the rm-856
   candidate→implemented flip; applying e3238f2a then flipping reaches
   the same postimage either way). Verified `git apply --check` rc0
   against the pristine worktree ROADMAP.md.
2. rm-856 done-flip (after review) per rm-012.
3. Schema re-base decision: our 34 landed below the wall's advancing
   ceiling (38 at prioritize time, 41 by review time — re-derive the live
   value at the gate; next free 42); re-base onto the live ceiling per
   convention.
4. The code unit (8 M + `?? fixtures/rm-856-composition/` + this record
   + the prevention doc + `?? docs/solutions/workflow-issues/…`) is one
   commit unit; porcelain at compound close is exactly that census.

## Next-cycle context (pre-review, ranked)

- **(a) MCP spec-currency lead** — research R1 (2026-07-28 stateless
  redesign: initialize REMOVED, per-request `_meta` version,
  UnsupportedProtocolVersionError -32022, `server/discover`,
  CacheableResult) + assess F3/F4 live findings. Reconcile unlanded
  twins FIRST: rm-780/781/782 stranded on dead branch
  `run-749cd29820f3` (terminal 5be2669, adopt-or-reimplement), plus
  rm-789/797/823/840/815/818 — three of those are -32602 guards done
  three different ways.
- **(b) rm-817** statusline phase-1 deadline — statusline.rs MOVED
  cli→core between this base and the wall; re-anchor before
  implementing.
- **(c) Pricing parity** (LiteLLM live 4,504 keys vs bundled 3,100;
  also `gpt-image-1.5` absent) vs the fbc4581f stewardship twin —
  landed-first precedence.
- **(d) rm-053 rider** — AgentMeasure iwasinnam-003 session-aggregate
  receipt vector as the external oracle for this truthfulness family;
  now banked twice.

Status accounting: 266 def rows (138 candidate / 68 implemented /
60 done). Zero ids minted at compound; fleet frontier: this wall rm-856
+ sibling spool claims to rm-857+ (d0ecd354 postimage) — the next mint
re-censuses live.


## Review fixes (2026-10-09, independent_review:fix 2cdc2f62)

Independent review fb1ed581 returned NEEDS_CHANGES with seven findings; all
seven discharged this turn:

- **F3 (code, red-first)** — the per-block multi-model pricing loop priced
  stood-down generic conversation blocks beside the meta block whenever the
  meta line's attribution differed from the session model (cost counted for
  tokens the totals disclose-not-count). Fixed with a `!has_meta_usage` gate
  in the `token_priced` event filter; pinned by
  `per_block_pricing_stands_down_generic_lines_under_meta_usage`
  (catalog-independent: cost equals the same journal with no conversation
  usage; pre-fix red 0.00087075 vs 0.0005, the exact predicted divergence).
- **F1** — lane counts corrected 23/23 -> 22/22 everywhere in this record
  and the cumulative roadmap patch (the runner's SUMMARY table has 22 rows;
  each lane's rc prints twice — LANE line + SUMMARY row).
- **F2** — ceiling wording corrected to "re-derive live at the gate"
  (38 at prioritize time, 41 at review time; next free 42).
- **F4** — the re-pin dates corrected to e249a10 2026-10-07 21:47:03Z
  (dc65644 the next day touched no such hunk) in the 2026-10-07 record's
  appendix and the cumulative patch's banner/EXECUTED bullet.
- **F5** — the cumulative roadmap patch de-dups the rm-856 notes line's
  duplicated "title-checked collision-free" phrase.
- **F6** — lesson L3 rewritten to the adjudicated truth (F6's shape
  verified; the rm-455 append's "defined twice" premise was the garbled
  part) and the rm-455 citation corrected in the cumulative patch.
- **F7** — "verbatim" claims corrected to "reconstructed from the recorded
  assessment" in the test header and CHANGELOG.

Retest after the fixes: targeted suites green (generic 12/0 incl. the new
pin, hostile 12/0, discovery 90/0, core lib 221/0), then the full 22-lane
battery and fmt/clippy re-run for the emission-time digest (lib.rs changed).

### Review-fix addendum (same turn, 2cdc2f62)

**Anomaly disclosed (protocol #17444).** Mid-turn, `generic_model_usage_truth.rs`
and the rm-856-composition fixtures gained edits NOT written by this session:
a rename to `sess`, a catalog-arm provenance assert with a correct rationale,
and a second pin over `meta-switch-multi{,-nousage}.json` — equivalent in
design to the fix this session was mid-flight on, but not authored here
(verbatim-text edits this session attempted against those exact regions
failed as not-found; no parallel attempt envelope exists in the spool).
Per protocol the foreign edits were read line-by-line and verified before
adoption; mtimes were stat'd. One adopted piece was then REJECTED on
verification: the multi-meta pin's premise (both meta models joining
`usage_models`, keeping the per-block arm) is FALSE on this base — the
modern meta arm (`lib.rs:1104`) never inserts into `usage_models`; only the
generic fold (`:1097`) and the legacy role-arm (`:1139`) do, and both are
stood down/generic when `has_meta_usage` holds (legacy aside). A pure-meta
journal therefore prices on the CATALOG arm at the session model — which is
exactly what the surviving pin asserts. The premise-false test and its two
fixtures were removed; the loop's `!has_meta_usage` filter stays as
defense-in-depth for the legacy arm and keeps per-block cost consistent with
counted lanes wherever that arm does fire.

**Reachability note for future cycles.** Post-fix, the per-block multi-model
arm for meta-bearing sessions is reachable only via the legacy `:1139` arm
(workbuddy-era role-usage with a session-start cutoff). If a future cycle
wants meta journals to price per-meta-block attribution, that is a FEATURE
(make `:1104` insert into `usage_models`), not a bug fix — mint it, do not
patch it sideways.

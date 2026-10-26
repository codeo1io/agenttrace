# Compounding Record — repository-maintenance `27767f30a85740aeb008f6af94040247` cycle 3

- Run: 7eb6138c26374ff0a725cedad6f6964d (compound attempt 7ba55dd6, BY ADOPTION — see §2)
- Wall: b425eb91634e2c5be34cee109b5a511c57ec9f9c (ROADMAP pristine md5 ff85dfbd; implement delta
  uncommitted in-tree: 10 M + 1 ?? fixture, +806/-30, porcelain md5 fa193930 / diff md5 9d0512cd
  — unchanged through targeted + full validation)
- Cycle: 2026-10-18 → 2026-10-21. Evidence: pre-review cycle records only, consumed as recorded —
  assess 54314dc2 (salvage-verified redo of reaped 3f287955), research 80b9d9eb (live fetches
  2026-10-18 05:20–05:55Z), roadmap cad725d8 (chained spool-side band, +20/-0, postimage 465 rows),
  prioritize 50422e61, stewardship 12a5b081, implement 782356cf (ADOPTION of reaped 51810f00's
  tree delta), targeted 0f78932d, full 916316b4 (CI-derived 5-job battery).
- DURABILITY WARNING (load-bearing): spool GC swept, on 2026-10-21, EVERY cycle-3 phase scratch
  dir and typed envelope listed above, plus the compound authors' own scratches (81e72fbb swept
  mid-verification of 5172887c). This in-tree record + the in-tree ROADMAP band + the ~/.cache
  mirrors named in §7 are the surviving durable record. PhaseResult summaries in the conductor's
  typed-result store remain the upstream source for the swept envelopes' evidence text.

## 1. Selected batch and outcome

"compressed-corpus honesty + recorded-cost surface" (prioritize 50422e61; 5 legs / 5 seams,
all EXECUTED, validated uncommitted, flipped in ROADMAP):

- rm-1872 (LEAD, assess A1 HIGH live-PoC) DONE — zst-honest transcript search (mechanics
  CORRECTED at review fix 5754f998: the compound-era wording named a nonexistent `scan_file_core`;
  the real seams are `transcript_evidence` search.rs:503 + `zstd_transcript_evidence` :609):
  `transcript_evidence` reads each admitted file under the caps and probes the zstd magic on the
  read buffer's head; a zstd frame (`28 B5 2F FD`) routes through `decode_zstd_rollout_bounded`
  (parser.rs:77/:163-175/:225) with the 64 MiB decode cap disclosed AT SCAN TIME, the byte budget
  charged on DECOMPRESSED bytes, compressed-content skips disclosed BEFORE budget accounting
  (`skipped: zstd decode failed (…)` / `skipped: decompressed N bytes exceed the read budget`),
  and the post-read budget mutation saturating in BOTH arms (no knife-edge double charge).
  Non-decodable admissions skip-with-disclosure, never abort the scan.
- rm-992 DONE — discovery name-gate (discovery.rs:1486) admits `.jsonl.zst`; admission and search
  now agree for compressed rollouts (assess probe: `--dir C --search response_item` 1 hit / 25447
  bytes scanned post-fix vs 1-of-2 files silently invisible pre-fix).
- rm-2019 (THIS CYCLE'S MINT, rebuilt in-tree at the ROADMAP foot — title "stored-cost exposure
  in machine formats") DONE — recorded-cost surface, MACHINE LANES ONLY (mechanics CORRECTED at
  review fix 5754f998: `CostComputation::stored_cost`, `--columns` arms, svg markers and the
  `sessions_report_records_recorded_costs` pin never existed in the delta). Every `--sessions`
  lane derives the recorded cost through `stored_cost_usd` (main.rs:2989): json rows gain
  `metrics.storedCost` (recorded spend / explicit recorded $0 as a value / `null` = absence)
  beside `cost_estimated`, a `metrics.provenance.StoredCostDivergence` stamp when the two diverge
  >$0.005 (side-by-side disclosure — the estimate is NOT overridden); the TSV gains `STORED_COST`
  and csv_export ONE `stored_cost` column (empty cell = absence); a non-finite recorded cost
  renders the shared `null` spelling in every cell, never `inf` (cured at review fix 5754f998;
  pins `session_list_json_emits_stored_cost_beside_cost` /
  `session_list_tsv_and_csv_carry_stored_cost_parity` /
  `stored_cost_non_finite_renders_null_like_the_cost_cell`). RESIDUAL arms, deliberately unlanded:
  human-lane recorded markers (svg/markdown) and probe-corpus goldens — the landed pins are
  synthetic Session arms.
- rm-1500 residuals DONE — the reports.rs `overview_summary` `total_duration` fold gates out
  non-finite per-session durations (pin `overview_summary_excludes_non_finite_durations_from_the_total`);
  the csv/tsv non-finite spelling is the shared `format_cost_cell` null arm. (Integration review
  2026-10-26, conflict case 5a588fcc: the original line named a csv_export `duration_sec` cell —
  no csv lane renders a duration; corrected to the real seams.)
- rm-784 DONE — `write_output -o` into an existing directory errors naming the target
  (main.rs:2655-2711; pin at :4268).
- New fixture: crates/agenttrace-core/tests/fixtures/codex-compressed-rollout/rollout-2026-10-15-10-00-00-3f2a1b2c.jsonl.zst
  (red-first zst lane).

## 2. Adoption chain (the cycle's defining pattern — three reaped-sibling adoptions)

1. implement 782356cf adopted reaped 51810f00's UNCOMMITTED WORKTREE DELTA (typed envelope
   absent, 28-line log, session_reaped provider +1718s): the delta covered exactly the batch's 5
   code seams; audited line-by-line against the work-order leg specs, then completed (2 rustfmt
   nits, 3 clippy warnings, stale README contract lines).
2. THIS compound: band authored by reaped 81e72fbb (+27min; postimage + patch + build-band.py
   archived before its scratch was swept), applied in-tree and re-verified by reaped 5172887c
   (+44min; its forensics captures are cited below), completed by 7ba55dd6 — stewardship record
   authored + banner lineage/archive sentences corrected (the only post-adoption edits).
3. Adoption rule (now exercised across implement + compound): a dead sibling's work is adoptable
   when it covers exactly the work-order seams; audit line-by-line against the leg specs, complete
   the gaps, disclose the correction delta, and VERIFY the adoptee's md5s against an independent
   record before trusting it.

## 3. Validation record (consumed at compound, NOT re-run — phase contract)

- targeted 0f78932d (tree unchanged: porcelain md5 fa193930 / diff md5 9d0512cd at dispatch AND
  completion): fmt rc0; clippy `--locked --all-targets` rc0 0 warnings FRESH after the
  fingerprint-bust cure (27.92s; the first 0.47s run was a stale cache — see Rule 13);
  search_transcript_contract 9/0; discovery_contract 114/0; core `--lib overview_summary` 2/0;
  `agenttrace --bins` 119/0; csv_export 8/0; entrypoints error_stderr 1/0.
- full 916316b4 (work order had required_scope=full with EMPTY full_command → suite derived
  verbatim-equivalent from .github/workflows/ci.yml's 5 jobs): Job1 lint+contracts 15/15 (fmt,
  clippy -D warnings, 9 contract scripts, ruby -c, npm test, check-plugin-version); tests-core
  449/0; tests-tui 10/0; tests-cli green under default TMPDIR=/tmp (the battery's own `-d` flag
  run was a harness error, re-run clean); actionlint rc0. Zero drift vs targeted record.

## 4. Standing reds PROVEN NOT-DELTA (next-cycle register — do NOT misread as cycle regressions)

1. RECLASSIFIED ENVIRONMENTAL at review fix 5754f998 (independent review 4ddc66b7) — was
   registered "HIGH DATE BOMB", FALSIFIED LIVE: `golden_bytes_are_pinned_for_the_demo_corpus`
   PASSED 2026-10-21 on fresh private-target binaries (svg_card_contract 11/0, re-confirmed on
   today's fresh binary at the review fix); the deprecation census is SNAPSHOT-anchored, never
   wall-clock (`bundled_snapshot_deprecated` pricing.rs:587-607 keys PRICING_SNAPSHOT_DATE;
   `catalog_source` pricing.rs:795-809 filters `past_deprecated(price, catalog.reference_date)` —
   the rm-419 "never the wall clock" anchor); `build_pricing_census` does not exist in the tree;
   demo-auto.svg is 50 lines (no :1467); the checked-in goldens pin `215 of 4441` in all three svg
   fixtures, so the registered `4443` was the STALE side of the diff, not the golden. VERDICT: the
   observed matrix red was the L1 stale-fleet-binary class (an older bundled snapshot regenerating
   4443 against the checked-in 4441 goldens on a shared CARGO_TARGET_DIR). Cure lead RETIRED;
   Rule 12 stands as prevention principle (already satisfied by rm-419); Rule 13's fresh-binary
   discipline is what catches this class.
2. **CI Job5 codex-plugin lane unconditionally red at this wall** — `git ls-tree HEAD --
   codex-plugin` = 0 rows while ci.yml:200-275 runs `npm --prefix codex-plugin test` → npm 254
   ENOENT package.json. Upstream-fix or lane-conditional needed; not touched by the cycle delta.
3. **TMPDIR-granularity twin flake, BOTH directions** (rm-1890-class): repro'd — under a
   `~/.cache`-style TMPDIR, `entrypoints count_honesty_disclosures_in_cli_lanes` FAILED
   (expected beta survivor, got alpha); under default TMPDIR=/tmp, full `-p agenttrace` rc0.
   Rider registered; Rules 12–13 companion context.

## 5. Reusable lessons / prevention rules (this cycle's new ones)

- L1 — fleet-shared CARGO_TARGET_DIR mtime-fingerprint staleness: a 0.47s clippy "rc0" was a
  stale cache; a filtered "1 passed / 483 filtered" ran a 484-test SIBLING binary against a
  461-test tree. Touch changed `.rs` files before trusting any quick green; cross-check reported
  test counts against the tree under test. Codified as test-flake-prevention Rule 13.
- L2 — a dead sibling's uncommitted tree delta is adoptable when it covers exactly the
  work-order seams: audit line-by-line, complete the gaps, disclose (§2).
- L3 — spool-side band delivery does not survive to the commit gate (third occurrence:
  13212e2f / 194456fc / f14d599b classes): this compound REBUILDS the cycle-3 roadmap band
  in-tree; mirror durable evidence to ~/.cache AT WRITE TIME (the pre-correction-81e72fbb
  archive is what made this adoption verifiable after the sweep).
- L4 — any golden embedding a now()-derived count must anchor to the artifact's own snapshot
  date (Rule 12; the date-bomb incident is its recorded case).
- L5 — an empty full_command in the work order means derive the suite from ci.yml
  verbatim-equivalent and record the job mapping (full 916316b4's ci-equiv-matrix approach).
- L6 (compound-specific, new) — the spool GC sweep hits scratch MID-VERIFICATION (observed
  2026-10-21: 5172887c's files readable at 05:0xZ, gone ~90s later): read-then-copy-first —
  capture file contents and md5s to ~/.cache BEFORE any multi-step comparison that dereferences
  spool paths twice.

## 6. Residuals and next-cycle candidates (concrete)

1. [RETIRED at review fix 5754f998 — standing red #1 was falsified as a date bomb (see §4.1);
   the census is already snapshot-anchored (rm-419) and the observed red was the L1
   stale-fleet-binary class; no cure work item remains]. The LiteLLM schema-validation-path death
   (PR #47079 deleted the schema — pin c515f80f^; research RC5) remains a live pricing-wave item.
2. CI Job5: land codex-plugin or make the lane conditional on the directory existing (§4.2).
3. TMPDIR twin-flake survivor determinism (rm-1890 rider; §4.3): sort or namespace the
   TMPDIR-derived lane so the same corpus yields one survivor set.
4. rm-241 tail from cycle 1 (check-locked-cargo.sh, check-release-surfaces.sh,
   check-install-runtime.sh) still un-namespaced — unchanged this cycle.
5. Numeral seam: this wall's in-tree ledger now 462 def rows / def-max rm-2019 (the rebuilt
   cycle-3 mint). Sibling unlanded claims at this base: b37d6f73 (rm-2016/2017/2018 + 6 riders —
   ITS OWN commit gate owes its in-tree rebuild, not re-minted here), 433dfc5c @c280887c
   (rm-2019 twin-refusal + rm-2020 file-cap + rm-2021 stored-cost TITLE-TWIN of this rm-2019 +
   rm-2022 cross-device + rm-2023 antigravity), 8931d7e8 rm-2013..2015; cross-family fog on
   advanced walls claims through rm-2272. Ids are campaign-local until merge — reconcile BY
   TITLE at integration (rm-708 convention). Next free numeral only after a fresh live census.
6. Research leftovers not selected this cycle: antigravity numeric model-id map (live ids
   1050/1277/1418/1419 + m418/m419 unpriced vs in-tree single entry 1417 at parser.rs:1470 —
   433dfc5c's subjects), ccusage cross-device 5c (#222), codex rust-v0.164 rollout-format drift.
7. CHANGELOG now IN-TREE (added at review fix 5754f998): Unreleased Added arm
   "compressed-corpus search honesty + recorded-cost surface"; Fixed arm "CSV null-parity
   residuals + non-finite stored-cost cells". check-changelog-structure rc0 re-verified
   (377 entries, no duplicate section kinds).

## 7. Status accounting, commit-gate seams, revert path, artifact map

- STATUS ACCOUNTING: ROADMAP 461 → 462 def rows (+1 rebuilt mint rm-2019), 0 duplicate ids,
  def-max rm-2019, managed footer last; flips rm-1872 candidate→implemented, rm-992
  candidate→implemented, rm-784 in_progress→implemented, rm-2019 minted-implemented (all dated
  2026-10-21, run-attributed). done-flips stay reserved to the commit gate (rm-012 convention).
- COMMIT-GATE SEAMS: stage exactly the 13 M (10 implement files + ROADMAP.md +
  docs/maintainers/test-flake-prevention.md + CHANGELOG.md — the last added by the review fix)
  + untracked crates/agenttrace-core/tests/fixtures/codex-compressed-rollout/ + this record
  (README.md's review-fix clause is within the implement file set); apply NOTHING from spool
  history for this cycle's roadmap band (the rebuilt rows are in-tree here); the CHANGELOG
  Unreleased entry is already in-tree (§6.7) — no gate action owed for it.
- REVERT PATH: `git checkout -- ROADMAP.md docs/maintainers/test-flake-prevention.md` + delete
  this record; pristine wall ROADMAP md5 ff85dfbdb1f15b3ced1aa28325d7ff21 (the 10-file implement
  delta reverts separately per its own record).
- VERIFICATION AT COMPOUND (attempt 7ba55dd6, read-only, no tests — phase contract): implement
  delta undisturbed (10 file md5s == 5172887c's pre-edit-md5s.txt capture: README e6f16824, 9
  code files as listed); in-tree ROADMAP pre-correction == archived authored postimage (md5
  4875893d ×3: tree == cache root == pre-correction archive) and `git diff HEAD -- ROADMAP.md`
  == archived band patch (mod index lines); census 462/0-dup/def-max-rm-2019/footer-last; 4
  flips + 12 rebuilt markers grep-verified; flake-doc diff vs HEAD == the Rules 12–13 addition
  captured at forensics; porcelain 12 M + 1 ?? before authoring this record.
- ARTIFACT MAP: authored band + build-band.py →
  /home/agent/.cache/at-compound-5172887c/pre-correction-81e72fbb-archive/ (postimage md5
  4875893d, patch md5 18bfb0f4); corrected postimage + verification dossier →
  /home/agent/.cache/at-compound-7ba55dd6/; this record → docs/stewardship/ (in-repo, staged by
  the commit gate). Event logs for the dead attempts: spool events/0fc187b11 (4 lines),
  events/81e72fbb (24), events/5172887c (39).

## 8. Review fix (2026-10-21, independent_review:fix 5754f998 over review 4ddc66b7 NEEDS_CHANGES)

The independent adversarial review (4ddc66b7) returned NEEDS_CHANGES; this fix closed the
actionable findings in-tree (prior fix attempt ef84d4ed reaped +44s having done nothing —
verified: 4-line event log, no envelope, no tree changes):

- F1 (ledger phantom mechanics): §1 bullets + the ROADMAP banner (:16) + the rm-1872/rm-2019
  riders (:3129/:3310) now name the REAL seams — `search.rs transcript_evidence` (:503) +
  `zstd_transcript_evidence` (:609), `main.rs stored_cost_usd` (:2989) + `session_json_row`,
  csv_export's single `stored_cost` column, the real pin names. `scan_file_core`,
  `CostComputation::stored_cost`, `--columns=recorded/--columns=stored_cost` arms, svg
  "(recorded)" tooltips and pin `sessions_report_records_recorded_costs` never existed.
- F2 (falsified standing red): §4.1 reclassified environmental (falsified live; details there),
  the ROADMAP register rider (:915) + banner reclassified, the §6.1 cure lead retired, and
  test-flake-prevention Rule 12's recorded case carries the falsification note (the rule stands
  as principle).
- F3 (stored-cost non-finite rendering, CODE): `format_cost_cell_opt` now inherits
  `format_cost_cell`'s rm-1500 null arm (`cost.map_or_else(String::new, format_cost_cell)`), the
  TSV STORED_COST cell routes through it, and the StoredCostDivergence stamp spells non-finite
  magnitudes `null` via `format_cost_cell` — so a diverging `storedCost: null` row carries the
  stamp and never reads as plain absence (absence = no stamp, empty cell). New pin
  `stored_cost_non_finite_renders_null_like_the_cost_cell` (main.rs bins) + the csv_export unit
  arm on `stored_cost_column_emits_value_zero_and_absence`.
- F4 (rm-2019 acceptance/rider contradictions): unmet arms marked RESIDUAL (human-lane markers,
  probe-corpus goldens); the rider's false "fresh-binary rerun reproduced the wall probes" claim
  corrected to consumed-not-rerun with the probes identified as assess-phase pre-fix numbers.
- F5 (CHANGELOG): Unreleased Added/Fixed arms added in-tree (§6.7); structure gate rc0.
- F6 (staging risk): disclosed — §7 seam list updated to the post-fix shape (13 M + 2 ??).
- F7 (decode-before-budget disclosure wording): accepted as-is, documented in
  `zstd_transcript_evidence` (skips are disclosed before budget accounting; no change needed).
- F8 (numeral drift 461/462/465): banner already carries the NUMERAL SEAM sentence; riders now
  also name the real pins, keeping the in-tree census the single authority (462/0/rm-2019).

Validation of the fix delta (targeted, Rule 13 fresh private CARGO_TARGET_DIR
~/.cache/at-reviewfix-5754f998, offline): `cargo fmt --check` rc0; `cargo clippy --locked
-p agenttrace -p agenttrace-core --all-targets -- -D warnings` rc0 0 warnings;
`cargo test -p agenttrace --bins` 120/0 (+1 for the new pin); `--bins session_list` 8/0;
`--bins stored_cost` 4/0; `--test csv_export` 8/0; `agenttrace-core --test svg_card_contract`
11/0 (the standing-red falsification re-confirmed on today's fresh binary, golden included);
`check-changelog-structure.sh` rc0. Zero tracked-file drift beyond the batch + this fix.

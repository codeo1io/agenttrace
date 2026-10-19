# Compounding Record — repository-maintenance `71aba0acc95c40c78adfd424f5b359f2` cycle 6

- Run: e40a66d301874cba8da57356d7efefe7 (roadmap attempt 678aed87, compound attempt
  4e06b169; prior compound attempt 5f85cfd9 provider-reaped at 16 messages — envelope
  absent, event log lifecycle-only, no scratch, tree untouched: zero durable work,
  REDONE from scratch here).
- Wall: f0649493 (HEAD never moved this cycle; the batch is the UNCOMMITTED 13-path
  delta +600/−16 — 11 M + 2 ??). ROADMAP.md entered compound at band postimage
  e43bf5aca53530db8518e1f0c09a0b1a (band applied by reaped implement 448da7a8, one
  rider defused by implement 567e6c1c).
- Cycle: 2026-10-18 → 2026-10-19. Evidence consumed as recorded — NOT re-run
  (compound contract): assess 3483a6c6, research 4e874e1b, roadmap 678aed87,
  prioritize 4e54965e, stewardship a8d2a011, implement 567e6c1c (adoption +
  completion of reaped 448da7a8), targeted 3dd59add, full 8a52e60b.

## 1. Selected batch and outcome

"hostile-input truth II: no panic, no NaN, no silent slurp, no invisible skips" — 4 legs:

- **rm-2031 (LEAD, 84.0) IMPLEMENTED pre-review** — lib.rs:3191 `let start = cutoff - span;`
  -> `checked_sub_signed`, None arm saturates to MIN_UTC. Unit pin
  `quota_extreme_event_timestamps_never_panic` covers both extremes (−262143/+262143
  years) x both spans; integration contract `quota_extreme_timestamp_contract` rides
  the 94-byte fixture `tests/fixtures/quota-past-extreme-timestamp.jsonl`. Black-box
  (targeted 3dd59add): `--quota -f json` on the assess hostile fixture rc0 with
  window_start_utc −262143-01-01T00:00:00Z, spend 0.0, sessions_contributing 1,
  resets_at −262143-01-01T05:00:00Z. Base at this wall: rc1 panic
  `DateTime - TimeDelta overflowed` (lib.rs:3191), re-proved live by assess 3483a6c6.
- **rm-1500 residual rider DISCHARGED** — the four ungated non-finite sites from
  assess F2-F5: governance.rs:500 row-cost fold + add_context_session aggregate fold
  (context_trends totals/projects) skip-while-counting; waste.rs:104 bloat>50 arm
  skips cost math on non-finite (discloses); csv_export.rs `money_cell` (crates/
  agenttrace-cli/src/csv_export.rs:71) renders `n/a` at every money column. Tests:
  cost_non_finite_contract 7/7 (2 new arms), csv_export 8/8 (2 new money-cell pins),
  compare_waste_sanitization_contract 4/4, lib pin
  bloated_session_with_non_finite_cost_skips_and_discloses.
- **rm-451 residual rider DISCHARGED** — `read_session_file_bounded` (metadata
  fast-path refuse + take(cap+1)) in codex_replay.rs; scan_rollout reconcile rides
  `parser::file_read_cap_bytes` (codex_replay.rs:512). Over-cap rollout refused
  WITHOUT reading (never slurped), per-session counter
  `codex_multiagent_reconcile_skipped_over_read_cap` (:519). Red-first
  `over_cap_rollout_discloses_reconcile_skip_without_reading_it` (sparse set_len
  cap+1): family never forms, child keeps inline turns, counter 1, parent silent,
  corpus stderr names the class. codex_multiagent_replay 15/15.
- **rm-1956 residual rider DISCHARGED** — `disclosure_line()` (codex_replay.rs:133)
  over ALL FIVE skip classes (families_skipped_no_task_started, rollouts_unreadable,
  rollouts_over_cap, + per-session reconcile counters :479/:519); the formerly
  discarded returns at main.rs:2316/:2486 consumed at :2320/:2495, corpus stderr;
  zero-when-zero byte pin. Adoption note: the reaped delta had 3 of 5 classes.

Validation record (consumed): targeted 3dd59add — 4/4 unit pins, contracts 27/27
(7+1+4+15), csv 8/8, fmt rc0, clippy −D warnings rc0, black-box rc0; full 8a52e60b —
CI-verbatim `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace`
RC=0, 91 suites 1423/0/0, exact +1 suite +10 tests over the assess 1413/90 baseline;
docs-links + changelog-structure + cargo-manifests rc0. CHANGELOG: 1 Added + 3 Fixed.

## 2. Adoption forensics (the cycle's defining event)

implement 448da7a8 was provider-reaped ~19 messages in with its work IN THE TREE
(the 13-path delta; ROADMAP already at band postimage ab3f278e, seam-verified).
The hunk-by-hunk audit found L1/L2 complete, L3 complete-in-substance but missing
its over-cap red pin, L4 complete-in-substance with a struct-literal compile error
(4 of 12 fields, in cost_non_finite_contract tests) and disclosure at only 3 of 5
skip classes. Completion (567e6c1c): compile fix, five-class extension, the over-cap
pin, CHANGELOG, and the band defuse (the F9/rm-1894 rider's original prose spelled a
literal bracket-parenthesis link form, which check-docs-links's own regex matches as
a broken relative target — reworded descriptively, dated amendment recorded in-band).

## 3. Reusable lessons / prevention rules

1. **Adoption of a reaped attempt's uncommitted delta requires COMPILE +
   ACCEPTANCE-MAPPING, not a file-list census.** The delta had never compiled and
   its disclosure satisfied 3 of 5 acceptance classes; both look "complete" to a
   presence check. Extended docs/solutions/process-issues/
   provider-felled-attempts-leave-adoptable-work.md with this case.
2. **Band/rider prose must never spell a bracket-parenthesis link form literally** —
   the docs gate's inline-link regex matches it as a broken relative target; spell
   link shapes descriptively; a landed defuse gets a dated in-band amendment.
3. **Empty full_command -> derive the authoritative command verbatim from the repo's
   own CI** (.github/workflows/ci.yml:52 test job; the KTD10 vacuous-block rule,
   second application). full 8a52e60b executed exactly this and reconciled the
   baseline exactly.
4. **Private per-lane CARGO_TARGET_DIR always** (shared conductor-cargo-target gives
   false wall results — fog rule); held at implement/targeted/full this cycle.
5. **Reap ledger discipline**: envelope absence != no work. This cycle: prioritize
   3dee432b (nothing durable -> redo), implement 448da7a8 (tree delta -> adopted),
   compound 5f85cfd9 (zero durable -> this redo). Recovery surfaces: the worktree
   itself, spool scratch, and session transcripts (the swept batch dossier was
   recovered read-only from the pi session transcript for this record's citations).

## 4. Residuals and next-cycle candidates (concrete)

1. **FIRST at cycle-7 assess**: read the Monday 2026-10-19 06:00Z pricing-cron
   outcome (rm-176 rider: removal_guard predicted rc1 on the 467 retired, no
   auto-PR; 1,556 new keys / 1,469 shared drift dominated by cache_read 0->absent,
   refresh-compatible Option fallbacks at pricing.rs:131-148). Honesty arm: census
   counter for the 373 image-step + 25 per-query keys.
2. rm-2032 (64.0) stored-cost parity per ccusage #1903 (optional storedCost, absent
   when none, never literal 0.0) + rm-2033 (50.0) `--quota --forecast` — the two
   unlanded cycle-6 mints.
3. rm-1872 zst-search arm (assess F7): decode-or-disclose, never silent blindness.
4. rm-1894 reference-style docs-gate arm (F9 EXTENSION STILL UNIMPLEMENTED — only
   the rider prose was defused).
5. rm-1533 antigravity map gap (ccusage 24 IDs vs our 1 at parser.rs:1479; PRs
   #1902/#1905 open, #1903 merged).
6. rm-44 serde_json 1.0.154 dep-refresh filler arm.
7. Fog reconciliation at integration — BY TITLE (rm-708), never renumber: sibling
   0e63a73c claims (rm-1957-class quota panic, rm-2020-class scan cap, batch-C
   non-finite sweep) and the spool rm-2024..2030 band (run c361e67f).

## 5. Commit-gate handoff

- Stage ALL 15 paths at wall f0649493 as ONE unit: the 13 batch paths (11 M incl.
  ROADMAP.md with band + defuse + compound, and CHANGELOG.md; 2 ?? test files) +
  this record (??) + the solutions-doc update (M).
- Done-flips reserved (rm-012): flip rm-2031 implemented->done only after review;
  the three residual-discharge riders do NOT flip their rows (rows already
  implemented/done; riders record extension discharge).
- Fog title-fold at integration per rm-708.
- ROADMAP census after compound: 475 def rows / 0 duplicate ids / def-max rm-2033 /
  managed footer last (unchanged counts — ONE flip, ZERO mints). Zero literal
  bracket-parenthesis link forms introduced (baseline 0, postimage 0).
- Spool seam: delegate/4e06b169…-scratch/ (patch + postimage + base copy + notes)
  + mirrors /home/agent/.cache/at-compound-4e06b169/ and /tmp/at-compound-4e06b169/.

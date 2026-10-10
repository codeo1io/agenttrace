# Prevention rule: a provider-dead attempt that left a durable delta is audited per-row, then adopted or redone — never blanket-adopted, never blanket-redone

- **Class:** workflow / delegate-attempt forensics (implement and later phases that can leave worktree drift)
- **Observed:** 2026-10-06, run `e94bb1ee` (repository-maintenance `b28e2827` cycle 3) — implement attempt `87bf5f66`; contrast cases in the same run: implement `dcd0919a`, full_tests `5cc3806a` (reaped, no durable drift) and `25b20d87` (reaped +28s, nothing ran); prior zero-drift cases: run `b1ff12f8` attempts 39bffb00/54eda020
- **Cost:** without the audit discipline, one of two symmetric failures: (a) redo-from-scratch discards a mostly-correct delta and burns a full implement turn re-deriving what already sits in the tree; (b) adopt-wholesale ships a rejected design — here it would have shipped a drift-note *explanation* where the row's acceptance demands the false alarm *stop*, i.e. the bug dressed as a feature.

## What happened

Implement attempt `87bf5f66` died of a provider failure mid-turn. Its typed PhaseResult JSON was never written and its event log ends mid-flight — but its **uncommitted worktree delta survived on disk**. The re-attempt found the tree already dirty beyond the prior phases' recorded census, and had to decide what to do with the drift.

The audit that followed classified the surviving delta **per row, not per attempt**:

1. **Adopted after fresh verification:** the rm-579 fix (upstream.rs Spawn routing + regression test) and the session_cache GoMetrics zero-defaults — each re-derived against its row's acceptance, built, and tested by the adopting phase. The adopted hunks even carried a syntax break (a missing `;` after a `git_probe` call): direct evidence the attempt died *mid-edit*, below any validation — which is exactly why adoption requires the adopter's own verification legs, not the dead attempt's claims (it made none; it never emitted a PhaseResult).
2. **Rejected and redone:** the dead attempt's rm-578 approach — appending an explanatory suffix to the false drift note. The row's acceptance ("the note stops firing on recorded-cost sessions") rules that design out regardless of how well it was implemented. The re-attempt implemented the basis-mirroring fix instead.

Meanwhile the run's *other* dead attempts (`dcd0919a`, `5cc3806a`, `25b20d87`) left the porcelain census byte-identical to the prior phase's recorded census — nothing to audit, phase redone from scratch.

## Prevention rules

1. **Read the event-log tail before anything else.** The failure point (and whether any `delegate_turn_progress` work preceded it) is in `/…/conductor-delegate-spool/events/<attempt>.jsonl`. The event log — not phase evidence — is the complete attempt registry: this run's registry showed three implement attempts and three full_tests attempts where the work-order forensics named only one each.
2. **Census the tree against the prior phase's recorded porcelain.** Identical census ⇒ census-clean reap ⇒ redo from scratch, nothing to hunt. Extra drift ⇒ a durable delta exists ⇒ audit (next rule). Do this before declaring "no work happened" — an absent envelope is not evidence the work never ran.
3. **Audit per row, against row acceptance — never per attempt.** A dead attempt's delta is a set of independent hunks, each of which either satisfies its row's acceptance or doesn't. Partial adoption is the expected outcome, and both halves (adopted, rejected-with-reason) must be recorded on the row and in the cycle record.
4. **Adopted-verified means the ADOPTER verified it.** A dead attempt emitted no PhaseResult, so nothing it left is evidence. Every adopted hunk needs the adopting phase's own build/test/live-replay legs before it counts; treat mid-edit syntax breaks as proof-of-unvalidated state, not disqualification.
5. **Never adopt a design the row's acceptance rejects, however finished it looks.** Acceptance text is the arbiter; an implementation of the wrong approach is a rejected hunk, not a shortcut.

## Related

- `docs/stewardship/2026-10-07-cycle3-compound-record-rune94bb1ee5261.md` — the full attempt registry and the adopt/reject split for run e94bb1ee cycle 3.
- `docs/stewardship/2026-10-06-cycle2-compound-record-runb1ff12f8.md` — the census-clean discriminator (md5 of `git status --porcelain`) on the zero-drift class.
- ROADMAP banner `compound c3 (2026-10-07, run e94bb1ee …)` — dead-attempt registry and flips recorded at compound.

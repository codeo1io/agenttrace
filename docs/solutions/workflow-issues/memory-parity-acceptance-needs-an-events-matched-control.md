# Prevention rule: memory-parity acceptance needs an events-matched control

- **Class:** workflow / acceptance-test design for bounded-memory or cardinality claims (any
  row whose acceptance asserts "memory no longer grows with X")
- **Observed:** 2026-10-07/08, run `2f02ecafb` (repository-maintenance `78307520` cycle 2) —
  assess `1177e59f` F2, implement `4087839767` (row rm-685)
- **Cost:** a naive pass/fail pair that cannot prove the claim. The assess's probe pair
  (661 MB vs 105 MB) and the first re-measurement pair (464 MB vs 99 MB) both left the fix's
  effect ambiguous: the "control" and "growth" legs differ in TWO variables at once — the
  ledger cardinality the fix bounds AND the number of counted events the parser accumulates
  — so neither pair can attribute the residual delta.

## What happened

rm-685 replaced the Codex totals journal-wide dedup set with an occurrence-adjacent bounded
ledger. Its acceptance clause demands memory parity with a control journal. The natural
probe pair is:

- **ctrl**: 400k events, 10 cycling distinct totals → few counted windows, small ledger
- **grow**: 400k events, 400k distinct totals → every window counted, huge ledger (pre-fix)

After the fix, grow still measured 464 MB against ctrl's 99 MB. That number alone is
unreadable: is the ledger still growing, or is the delta the cost of the counted EVENTS
themselves (one `Event` per counted window, pushed into the session's event vector
regardless of any dedup scheme)?

The isolating probe is an **events-matched control**: 400k events that are ALL counted
(same event load as grow) while carrying only 10 distinct totals (same ledger load as
ctrl). Post-fix measurements — grow 464 MB ≤ events-matched control 487 MB — prove the
distinct-total-proportional component is gone, and correctly attribute the residual ~370 MB
to per-counted-window `Event` accumulation: a pre-existing, format-wide property that is NOT
this row's defect (and became this cycle's next-cycle lead instead of a phantom regression).

## Prevention rules

1. **A parity acceptance clause names its control shape.** "Bounded vs control" must specify
   what the control matches on: same counted-event load with the bounded dimension reduced
   to its control cardinality. If the clause cannot say that, the clause is untestable as
   written — fix the clause before writing the probe.
2. **One variable per pair.** When a measured delta can come from two mechanisms, add the
   third leg that holds one mechanism constant — do not report a two-leg delta as proof of
   either mechanism's absence.
3. **Attribute residuals explicitly.** If post-fix growth still exceeds the naive control,
   the result is not "parity failed" — it is "unattributed residual", and the phase must
   either isolate it (events-matched leg) or downgrade the claim to partial with the
   residual's candidate mechanisms listed.
4. **Archive the generator, not just the numbers.** Large probes live in /tmp and get
   swept between phases (this cycle's assess probes vanished before implement). Commit the
   golden small fixtures to the repo, archive the big-probe GENERATOR (script or inline
   shape description) in the delegate scratch, and record the shape in the row annotation —
   the measurement must be reproducible from the record alone.

## Related

- `docs/stewardship/2026-10-08-cycle2-compound-record-run2f02ecafb267.md` (this cycle)
- ROADMAP rm-685 (the ledger rework; annotation records all six measured legs) and the
  next-cycle lead for per-counted-window `Event` accumulation (~0.9 KB/window, ~370 MB at
  400k counted windows — aggregation-during-parse candidate)
- Generator + VmHWM poller: `/home/agent/.hermes/conductor-delegate-spool/4087839767f944be9ee7fb28cba91fd1-scratch/`
  (`implement-verification.md` carries the inline shape; `vmhwm.py` is the poller)

# Run 5c144613 cycle 4 batch selection — 2026-10-15

Decision record for hermes-conductor run 5c144613419f4f7baa194503185221fa
(repository-maintenance d1f329d7b602414ea1fab4058d60a8a4 cycle 4), phases
roadmap → prioritize → stewardship. Method: weighted scoring (impact, risk,
effort, dependencies, strategic value) over this cycle's assess F1–F7 redo
(741939a2) and research RC1–RC9 (32a77706) lanes, minted as rm-1508/rm-1509
(renumbered at the 2026-10-17 integration, conflict case 7f566ee8: the doctor-walk
row landed as rm-1541 — the wall had since landed rm-1509 under run b01c2b6a's
OpenCode v2 summary_diffs row; rm-1508 kept its numeral)
plus 7 dated riders (ROADMAP.md c4 banner, 2026-10-14), with a live conflict
census of 95 sibling conductor worktrees before selection. Recorded here at
the compound phase to keep the cycle's selection+outcome pair durable in-repo
alongside the ROADMAP riders.

## Position

**Selected batch: "cache-rate truth at ingest" — LEAD rm-1508 (correctness
76.0).** The single highest-value lane that was title-clean against every
live sibling worktree and completable end-to-end in one cycle: the standard
cache-rate slots conflated absent rates with published-$0 at TWO sites (the
snapshot generator's `or 0` zero-fill in `scripts/pricing/update-snapshot.sh`
and the plain-f64 serde/LiteLlmModel slots), silently pricing 51% (cache
read) / 84% (cache write) of bundled costed rows' cache traffic at $0 with
no disclosure. Everything adjacent (rustls lock bump, doctor walk bounding,
the 466-row upstream purge reconcile) was recorded as must-remain-separate
units rather than batch riders, so each lands or fails on its own evidence.

## Outcome (pre-review, compound-phase fold)

Implemented end-to-end (implement ab60e5f5, adopted+reverified 0e95e075):
Option-typed standard cache slots through generator→serde→catalog, input-rate
(cache-read) / vendor-convention (cache-write) fallback with per-session
disclosure + overview census arm, the vendored snapshot left byte-identical
(presence preserved, values unchanged), and the contract pinned red-first in
`tests/cache_rate_truth_contract.rs` (one test fn asserting the five
acceptance arms) plus the census pin extended
in `tests/pricing_snapshot_hygiene.rs`. Validation (pre-review):
targeted 374/0 across the pricing/governance/pi lanes (42ef7cee, incl. a
hermetic trim() fixture proving the generator preserves absence; per-suite
counts corrected by review 95f904a7 — the phase summary's 385
mis-aggregated multi-suite cargo output), full suite
1132 passed / 0 failed over 64 suites — the 7 ci.yml jobs run verbatim
locally with job-faithful env (2254ac90). Three lane reds in that run were
proven byte-identical at the pristine base 8bc0587 and are standing, not
batch regressions: actionlint SC2001 (`ci.yml:192`), check-install-runtime
test D (rm-051 pinned-ref), cargo-deny 4× RUSTSEC rustls (rm-018).

## Next-cycle candidates left open

1. **rm-176 reconcile — now unblocked by rm-1508**: the drift gate's
   predicted-red 2026-10-20 run (466 retired upstream, 39 new-costed) can
   finally refresh WITHOUT re-baking the conflation, because the refreshed
   snapshot now preserves absence. The drop-with-disclosure vs
   pin-and-retain decision for the purged vendor prefixes is the gating
   judgement.
2. **rm-1509 doctor walk bounding + fixture-owned doctor leg** — reliability
   62.0, untouched this cycle (row landed on the wall as **rm-1541** after the
   2026-10-17 integration renumber — conflict case 7f566ee8, mapping note on the
   row and the mint footer).
3. **rm-018 rustls ≥0.23.46** — security-motivated lock bump, now also
   pristine-red-proven at base (cargo-deny lane reds on every full run until
   it lands).

# Cycle 1 batch selection — 2026-09-02

Decision record for run 314df0f829fe49af8de46938c7b579a6, phase prioritize.
Method: `ce-pov` approach-set position over the supplied roadmap items
(ROADMAP.md "Planned work (added 2026-09-02)") and the assessment findings
they encode. No prior decision record exists (no `docs/solutions/`, no ADRs).

## Position

**Selected batch: H1 + H2 — "trustworthy numbers, offline by default."**

- **H1 — Trustworthy arithmetic on untrusted logs** (assessment F1):
  checked/saturating token and cost aggregation plus `number_as_i64`
  range validation.
- **H2 — Offline-by-default pricing** (assessment F2+F3+F5, research
  candidate 1): no network on default report/test paths, refresh only via
  `--update-pricing` or explicit opt-in, vendored dated snapshot, stable
  wall-clock-free `pricing_source`.

Optional ride-alongs (near-zero effort, may drop without failing the
cycle): F12 `--lang` validation, F14 `.gitignore` cleanup, F19 plugin
version sync — all "declared surfaces match behavior" hygiene.

## Why these two

- **Impact.** They are the only two HIGH-severity assessment findings, and
  both were empirically reproduced, not inferred: adversarial token counts
  make release builds print `TOKENS=-2` and cost `166020696663385.94`
  (wrong numbers shipped to every report surface), and a stale pricing
  cache makes an ordinary `--overview -f json` fetch 2,090,796 bytes,
  contradicting `PRIVACY.md:5`, which names only `--update-pricing`.
- **Risk.** Low. Both are contained: F1's arithmetic sits in one file
  (`lib.rs:1076` plus the accumulator sites at `lib.rs:527-534,544,558,562`
  and `number_as_i64` at `parser.rs:3576`); F2's download has a single
  choke point inside `pricing_catalog()` (`pricing.rs:239-241`, five
  internal callers at `pricing.rs:49-106`), and `--update-pricing` already
  exists (`main.rs:67-68`), so the opt-in surface is standing.
- **Effort.** Medium, concentrated: `lib.rs` for H1, `pricing.rs` (1183
  lines) for H2 plus a build-time snapshot step. No cross-crate refactor.
- **Dependencies.** None between H1 and H2 (parallelizable). H2 is itself
  the prerequisite for three capability-lane items (canary C3, merge/config
  C5, models.dev C2 all assume deterministic, offline-safe pricing), so it
  unblocks the most downstream work per unit of effort.
- **Strategic value.** Closes the exact posture gap with the category
  leader — ccusage (18,282 stars) headlines "offline pre-cached pricing" —
  and makes the existing CI determinism check
  (`scripts/ci/check-deterministic-output.sh`) meaningful across cache
  states instead of only within one warm-cache job.

## Why not the alternatives

- **H3 (cache/history durability, F4/F6/F7):** real data-loss risk but
  derived-state only — reports stay correct. Different subsystems
  (`session_cache.rs` 909 lines, `history.rs` 184) and a different test
  strategy (interrupted-write fixtures). Strong candidate for batch 2;
  including it now stretches the cycle without raising its ceiling.
- **H4 (platform/channel parity, F8/F9/F12/F14/F15/F19):** the Windows
  leg cannot be verified end-to-end in this environment (no Windows host),
  failing the cycle's completability bar. Keep the trivial trio as riders.
- **H5 (no silent data loss, F16/F18 + candidate 6):** worthwhile, but the
  full attribution-ledger scope is a feature, not a fix; the cheap slice
  (count skipped files with reasons) can ride with batch 2.
- **C1 (limit-pressure diagnostics):** highest strategic value in the
  research ranking, but building new diagnostics on arithmetic that can go
  negative (F1) and pricing that shifts with cache state (F2) would bake
  known defects into a flagship feature. Dependency-ordered behind H1/H2.
- **C2 (models.dev source):** extends the pricing catalog H2 rebuilds —
  sequencing it before H2 would double-touch the same code.
- **C3 (format canary):** requires a quarantined network-explicit CI
  workflow whose design presumes the offline default H2 establishes.
- **C4 (OTel bridge):** largest item (High complexity); needs its own
  brainstorm for semconv pinning. Not a rider.
- **C5 (baseline config + merge):** byte-identical outputs are its
  precondition; explicitly blocked on H2.
- **Do nothing:** rejected — both selected defects ship wrong or
  promise-breaking behavior to users today, reproducible on demand.

## Verified facts

- Project (re-checked this session): `lib.rs:1076` `total_tokens()` and the
  accumulator sites at `lib.rs:527-534,544,558,562`; `number_as_i64` at
  `parser.rs:3576`; `pricing.rs:239-241` stale-cache `download_pricing(
  Duration::from_secs(5))` with five `pricing_catalog()` callers at
  `pricing.rs:49-106`; `--update-pricing` at `main.rs:67-68`; `PRIVACY.md:5`;
  baseline `cargo test --workspace` 147 passed / 0 failed on the current
  tree; reproducers recorded in
  `docs/reviews/2026-09-02-adversarial-repository-assessment.md`.
- External (load-bearing): ccusage offline-pricing posture (ccusage.com,
  fetched 2026-09-02 during the research phase); claude-code usage-limit
  demand cluster behind C1's deferral rationale (issues #16157/#38335/
  #9424/#41930).

## Assumptions (not verified)

- Effort sizing ("medium") is inferred from choke-point and file-size
  analysis, not measured implementation time.
- Rider effort ("near-zero") is an estimate from finding complexity, not a
  measurement.

## Conditions / definition of done

The batch is complete when the roadmap's acceptance criteria for H1 and H2
hold and their evidence expectations are recorded: H1's regression tests
(1e300 and 2^63 inputs asserting bounded, non-negative totals and cost)
plus a committed generated fixture from the repro corpus; H2's
network-blocked `--overview -f json` run succeeding, `cargo test` passing
with no network and no cache mutation, consecutive identical runs producing
byte-identical JSON, and `PRIVACY.md` matching observed behavior. The
behavior change (no automatic refresh) must be named in the CHANGELOG.

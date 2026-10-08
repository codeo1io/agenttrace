# Prevention rule: the conformance pack refreshes with the pricing snapshot, and xfail pins flip with their owning row

- **Class:** workflow / conformance fixtures (`testdata/conformance/` + `crates/agenttrace-core/tests/token_conformance.rs` + `scripts/conformance/run.sh`, the rm-053 pack)
- **Observed:** designed-in at the pack's landing, 2026-10-07, run `6d254aad` (repository-maintenance `3282bd9d` cycle 2) — the hazard class is prospective: every guard in this contract exists to make a future refresh fail loudly instead of silently re-blessing stale totals
- **Cost if unguarded:** a `pricing_snapshot.json` refresh (rm-006/rm-176 lane, drift 4,480 live vs 3,100 bundled at 2026-10-07) would move model rates; a pack whose expected-cost literals were hand-copied would then either block the snapshot refresh (best case) or silently assert against re-derived-but-unreviewed numbers (worst case — a conformance suite that certifies whatever the code currently does is a tautology, not a guard)

## The contract

The pack embeds **no price table of its own**. Every expected cost is a TOTAL derived from the bundled snapshot through the catalog formula, and `token_conformance.rs` re-derives every non-xfail literal from `pricing::lookup_price`/`default_price` at run time, failing when a literal drifts from the live snapshot by more than one round4 step (6e-5). This makes the refresh rule mechanical:

1. **A snapshot refresh that moves any model used by the pack forces a pack refresh.** The runtime re-derivation guard turns a stale literal into a red `token_conformance` run — treat that red as the contract firing, not as flake. Refresh the manifest's `expect.cost_estimated` literals from the NEW snapshot in the same change-unit as the snapshot bump, with the derivation shown in the case's `cost_derivation` field. Never widen `formula_tolerance` to make a drift pass.
2. **xfail pins flip ONLY with their owning row.** Case `012-claude-multi-model-frozen` is xfail-pinned to rm-663 (per-event model stamping): the current tree MUST NOT match its truth ($0.0495). When rm-663 lands, the pack refresh is part of that landing: drop the case's `xfail` block so the truth becomes asserted. A pin that flips green for any other reason (an accidental parser change) is a finding, not a gift — investigate before un-pinning.
3. **Verbatim corpus fixtures are never "fixed" to pass.** Cases 001-006 are byte-copies of the rm-046 adversarial corpus (`testdata/generated/adversarial/`), and 012/013 are byte-copies of the assess corpus that found the frozen-model defect. If a pack case sourced `verbatim_of` starts failing, the defect is in the code or in the snapshot — the fixture is the evidence, not the bug. Amend expectations only through rule 1 (snapshot-driven) or rule 2 (row-owned), and record provenance in the manifest's `source` field.
4. **New cases follow the manifest, not the runner.** The runner asserts exactly what `manifest.json` declares (`pack`/`version`/`cases[]` with `expect` + provenance); adding a case means manifest + fixture (+ README class-table row) in one change-unit. The runner's summary line ("N cases asserted (K xfail-pinned)") is the pack's public count — the README and the CI step name must not drift from it.
5. **The CI job owns the pinned execution context.** `.github/workflows/ci.yml`'s conformance job runs the library harness and the binary runner with `AGENTTRACE_CI_OUT` kept outside the repo; any change to the runner's output location or env contract re-checks the job step list (6 steps, toolchain/SHAs pinned) in the same change-unit.

## Related

- `docs/stewardship/2026-10-08-cycle2-compound-record-run6d254aad.md` (the cycle that landed the pack)
- ROADMAP rm-053 (the pack's row, `implemented` pending review) and rm-663 (the xfail pin's owning row, sibling band)
- `docs/solutions/workflow-issues/schema-version-bumps-require-planted-fixture-sweep.md` (the sibling rule for pin sites in a different medium: the same "a refresh must sweep its consumers" shape)
- AgentMeasure `campaigns/audit-report-2026-09.md` (the audit taxonomy the pack's seven classes encode)

---
title: "Pricing snapshot refresh + discharge recipe: census, dated drops, drift arms, and the bare-alias injection"
date: 2026-10-23
category: workflow-issues
module: "pricing snapshot lifecycle (crates/agenttrace-core/src/pricing_snapshot.json + scripts/pricing/)"
problem_type: workflow_issue
component: pricing
severity: high
applies_when:
  - "The bundled pricing snapshot (crates/agenttrace-core/src/pricing_snapshot.json) is stale against the live LiteLLM source and the weekly drift cron (pricing-drift.yml, Mon 06:00 UTC) is approaching"
  - "A refresh moves hundreds of keys at once and you must prove the drops are intentional, not silent unpricing of models the parser still emits"
  - "The parser's identity maps (ANTIGRAVITY_NUMERIC_MODEL_MAP or siblings) emit BARE model names while the live source prices those names only under vendor prefixes"
  - "A validation run shows a pricing contract red and you need to separate a real signal from a stale-binary artifact before touching code"
---

# Context

The bundle is the source of truth at runtime; the live source (LiteLLM `model_prices_and_context_window.json`) drifts daily. Run `e90edd6c` (repository-maintenance `e763aec1` cycle 1) inherited the rm-176 lane in its worst shape: bundled snapshot 14 days stale (2026-10-08, 3,130 rows, **zero** `gemini-4*` keys) while `parser.rs` mapped antigravity numeric id 1417 → bare `gemini-4-argon` — every such spend priced as unmapped. The naive fix ("re-fetch and commit") is itself the hazard: the live source had dropped 466 bundled keys at once, and a silent refresh would have unpriced models the parser still emits, converting staleness into a *different*, quieter defect. The refresh only counts as **discharged** when the drift gate — not a human eyeball — certifies the delta.

# Guidance

The recipe, as executed and validated this cycle (implement `bea8b18c`, targeted `351718b0`, full `40fc6626`):

1. **Census before refresh.** Diff the live source against the bundle first: row counts, +added/−dropped, and specifically every family your identity maps emit (grep the map's target names). Write the numbers down — they become the discharge evidence and the roadmap riders. This cycle: live 4,245 vs bundled 3,130 → +141 / −466, `gemini-4-argon` priced only under vendor prefixes (`gemini/`, `github_copilot/`, `vertex_ai/`).
2. **Refresh through `scripts/pricing/update-snapshot.sh`, never by hand-editing the JSON.** Since rm-2348 arm (a) the script injects **bare-name aliases** for prefix-priced families (fail-closed on collision / missing source row / asymmetric provider), and discloses them in `_snapshot.bare_aliases`. The alias injection is what makes a bare-name identity map priceable at all; a refresh without it silently re-creates the hole the map was supposed to close.
3. **Enumerate every drop in `scripts/pricing/snapshot-drops.txt` with a dated reason.** Undated drops are how silent unpricing ships and how the next archaeologist loses an afternoon. This cycle: 466 drops, all dated.
4. **Discharge via `scripts/pricing/drift-check.sh`, never by disabling or widening the cron.** `--selftest` first (9 arms, including the bare-alias mirror and the keep-filter control), then live: `new-costed 0 / retired 0 / rate-changed 0 / deprecation-new 0` means `removal_guard` is clear. A handful of informational `max_input` rows is fine. Do not merge a refresh whose live drift run is red on a *counted* arm.
5. **Pin the truth red-first.** `crates/agenttrace-core/tests/pricing_refresh_truth.rs` asserts that the bare key AND the numeric id price through the shipped catalog — proven red under the OLD bundle before the refresh landed. A red-first pin is what turns a future regression into a loud test failure instead of a slow billing drift.
6. **Co-travel consumers fix in the same change-unit.** `token_conformance.rs` re-derives every expected-cost literal from the bundle at runtime (see `conformance-pack-refresh-contract.md`): a rate change turns those literals red. Fix the literals from the NEW snapshot with the derivation shown; never widen `formula_tolerance`.
7. **Validate in an isolated `CARGO_TARGET_DIR`.** A sibling attempt's "argon red" was a stale binary from a shared target dir, not a code signal — it burned a diagnosis cycle before the isolated-lane rerun went green (see `validation-target-dirs-must-live-on-a-filesystem-with-headroom.md`).

# Why This Matters

The failure mode is **silent mispricing**: WARN-and-drop exclusions and 0-cost rows surface days or weeks later as spend-accounting drift, long after the change that caused them stopped being reviewable. The cron + `removal_guard` make staleness loud; this recipe makes the discharge mechanical and evidence-backed, so the weekly gate reads as a certificate rather than a threat.

# When to Apply

- Any rm-176-lane refresh, or any touch of `pricing_snapshot.json` bigger than a single-key correction.
- Any time an identity map gains arms whose target names must be priceable in the bundle (rm-1533 antigravity lane; siblings).
- Before a Monday 06:00 UTC cron when the snapshot date is more than a few days old.
- When `snapshot-drops.txt` has grown undated entries — that is the recipe's rule 3 failing.

# Examples

This cycle's discharge, end to end: refresh 3,130 → 2,805 rows (2026-10-22 bundle, +141/−466 all dated), bare `gemini-4-argon` injected and priced at 2e-06/1e-05 with `_snapshot.bare_aliases` disclosure; `drift-check.sh --selftest` 9/9 arms, live run rc0 with `removal_guard` clear before the 2026-10-26 cron; red-first pins 4/0; focused battery 546/0 across the ten touched suites; full suite 96 suites / 1,495 tests / 0 failed.

# Related

- `docs/solutions/workflow-issues/conformance-pack-refresh-contract.md` (consumer co-travel: the conformance pack's literals vs snapshot rates)
- `docs/solutions/workflow-issues/validation-target-dirs-must-live-on-a-filesystem-with-headroom.md` (the stale-binary false red)
- ROADMAP rows rm-176 (refresh workflow owner), rm-2348 (bare-alias injection), rm-2349 (zst scan cap, same batch), rm-1533 (antigravity identity arms)
- `scripts/pricing/update-snapshot.sh`, `scripts/pricing/drift-check.sh`, `scripts/pricing/snapshot-drops.txt`, `crates/agenttrace-core/tests/pricing_refresh_truth.rs`
- Cycle record: `docs/stewardship/2026-10-23-cycle1-compound-record-rune90edd6c.md`

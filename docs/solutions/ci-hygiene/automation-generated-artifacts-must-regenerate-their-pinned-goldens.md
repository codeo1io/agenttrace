# Automation that regenerates a golden-coupled artifact must regenerate the goldens in the same job

- **discovered**: 2026-10-15, run 96287e100a56469babf0aae93fd0e9ed cycle 1 (repository-maintenance bf1270b018564442ac460476a0b0fd4f, attempts 0c6db522/03fbd4cf/69a48cfb)
- **fixed by**: rm-1514 (implemented pre-commit, attempt 394ea0f9/a5c25056; validated 7a3120e2 targeted, ce41e2e9 full 1136/0)
- **surfaces**: `.github/workflows/pricing-drift.yml`, `scripts/pricing/update-snapshot.sh`, `crates/agenttrace-core/tests/svg_card_contract.rs`, `crates/agenttrace-core/tests/fixtures/svg-card/`

## Context

The weekly pricing-drift gate (cron Monday 06:00 UTC) regenerates the bundled pricing catalog and opens a PR. Three pinned svg-card goldens embed bytes derived from that catalog: the card footer renders `audit.pricing_source` (doctor's "LiteLLM snapshot {date} (bundled, {n} models…)"), so EVERY refresh moves golden bytes — a footer date or model-count change is a rendering change.

## Problem

The automation staged only `pricing_snapshot.json` + `pricing.rs`. No `UPDATE_CARD_GOLDEN` step existed in the script checklist or the workflow job, so every actionable drift PR carried a `svg_card_contract` red. Worse, the PR is `GITHUB_TOKEN`-authored, and such PRs do not trigger CI workflows — the red was invisible until merge, then surfaced on master. The manual refresh checklist also omitted the step, so the same trap fired for humans (it previously cost run b9963213 a targeted_tests cycle).

## Solution

Treat a golden-coupled artifact as one unit with its goldens, everywhere the artifact moves:

1. **Script checklist** (`update-snapshot.sh` header + tail print): `UPDATE_CARD_GOLDEN=1 cargo test -p agenttrace-core --test svg_card_contract` is the first post-refresh step, and the commit includes `tests/fixtures/svg-card`.
2. **Automation job** (`pricing-drift.yml`): the drift job installs the pinned toolchain (same SHA + version input as ci.yml — it previously had no Rust toolchain at all, which is why no regen was ever attempted there), runs the same regen line during regeneration, and `git add` stages the golden fixtures alongside the bundle.
3. **Drill evidence** (pre-commit, reverted): a live-catalog dry run proved golden bytes move WITH the bundle and re-pin byte-stable on a second regen; goldens restored byte-identical to HEAD afterward — the weekly automation owns real refreshes, not the batch.

## Prevention / corollary rules

- Any job (or human step) that regenerates an artifact whose bytes appear in a pinned test fixture must regenerate those fixtures in the same job and stage them in the same commit. "The tests will catch it" is false when the author is a token whose PRs run no CI.
- When a workflow authors PRs via `GITHUB_TOKEN`, its own comment "PRs authored by the default GITHUB_TOKEN do not trigger CI workflows" is the tell: nothing downstream of that PR will validate the change — the job itself must leave the tree green.
- A goldens test whose inputs embed repository state (dates, counts, version consts) is a coupling, not a pure snapshot. Make that coupling explicit in the fixture-regen contract (the `UPDATE_CARD_GOLDEN` env opt-in) and in every checklist that touches the coupled inputs.
- Test-robustness corollary (landed with this fix): when scrubbing a wall-clock leak in such a test, also scrub the other repository-state disclosures (here the pricing snapshot line) so the test stays about bytes, not calendar luck.

---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T04:40:00Z"
title: "Stewardship request — cycle 1 batch: trustworthy numbers, offline by default"
summary: "Hands the conductor the selected H1+H2 maintenance batch as repository change units with surfaces and separation hints; makes no Git-topology decisions."
keywords: ["agenttrace", "stewardship-request", "h1-arithmetic", "h2-offline-pricing", "change-units"]
cwd: "/work/projects/agenttrace"
resume_focus: "Establish the stewardship contract for the H1+H2 batch: inventory this repository, detect overlap, split unrelated concerns, preserve the dirty state listed below, and plan branches/worktrees before implementation begins."
repository: "luoyectl/agenttrace"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
branch: "master"
head: "e005952"
---

# Stewardship request

This document is a **request**, not a contract. Per the conductor work order
(run 314df0f829fe49af8de46938c7b579a6, phase stewardship, attempt
444e162f53b64a668ed3fba13c33bbb6), it describes the selected maintenance
batch and stops there: **it chooses no branch, worktree, commit order, or
any other Git topology.** Those decisions belong to the conductor.

## Title

agenttrace cycle 1 — "trustworthy numbers, offline by default" (H1 + H2,
with three optional hygiene riders).

## Summary

Implement the two-item batch selected by the prioritize phase
(`docs/decisions/2026-09-02-cycle-1-batch-selection.md`):

- **H1 — trustworthy arithmetic on untrusted logs** (assessment finding F1,
  high): checked/saturating token and cost aggregation plus input range
  validation, so adversarial or corrupt session logs can no longer panic
  in debug or produce negative/wild totals and costs in release.
- **H2 — offline-by-default pricing** (assessment findings F2, F3, F5,
  high/medium, merged with research candidate 1): no network access on the
  default report path or during tests, refresh only via the existing
  `--update-pricing` flag or an explicit opt-in, a vendored dated pricing
  snapshot as the fallback source, and a `pricing_source` label that is
  stable across runs (no wall-clock "fetched" timestamps).

Both items are empirically reproduced defects, not inferences; reproducers
are recorded in the assessment artifact.

## Repository candidate

- Working tree: `/work/projects/agenttrace`, branch `master`, HEAD
  `e005952` (also the repository's root commit).
- Remote: `https://github.com/luoyectl/agenttrace.git`.
- Layout: Cargo workspace — `crates/agenttrace-core` (parsing, pricing,
  metrics; `src/pricing.rs` 1183 lines, `src/session_cache.rs` 909,
  `src/history.rs` 184), `crates/agenttrace-cli` (binary), 
  `crates/agenttrace-tui`. Integration tests live in
  `crates/agenttrace-core/tests/` (`demo_contract.rs`,
  `discovery_contract.rs`).
- Baseline at this HEAD (re-verified 2026-09-02): `cargo test --workspace`
  147 passed / 0 failed; `cargo fmt --check` clean; `cargo build --release`
  succeeds; `cargo clippy` unavailable in this environment.
- **Dirty state to preserve** (conductor responsibility per the work
  order; delegate will not commit, stash, or revert any of it):
  - `ROADMAP.md` — modified, +88 lines (roadmap phase; pure append, the
    original 46 lines are intact, new section starts at line 48).
  - Untracked: `docs/reviews/` (assessment phase),
    `docs/ideation/` (research phase), `docs/decisions/`
    (prioritize phase), `docs/stewardship/` (this request).
  - Untracked: `.hermes/` — conductor/session harness state, **not
    repository content**; exclude from any inventory that feeds commits.

## Surfaces (requested change units)

### CU-1 — H1 arithmetic hardening

- `crates/agenttrace-core/src/lib.rs:1076` — `total_tokens()` (unguarded
  addition).
- `crates/agenttrace-core/src/lib.rs:527-534, 544, 558, 562` — the token
  accumulator sites inside session metrics collection.
- `crates/agenttrace-core/src/parser.rs:3576` — `number_as_i64` (entry
  point where adversarial values like `1e300` enter as JSON numbers).
- Cost computation sites that multiply the (currently overflow-prone)
  totals — same crate.
- New regression tests (file of the implementer's choosing; the two
  existing contract-test files set the pattern) covering `1e300`-scale and
  `2^63`-scale inputs, asserting bounded, non-negative totals and costs.
- `CHANGELOG.md` — entry naming the fix.

Rationale: the only correctness defect that ships wrong numbers to every
report surface today; reproduced (debug panics at `lib.rs:1077`, release
prints `TOKENS -2`, cost `166020696663385.9375`).

### CU-2 — H2 offline-by-default pricing

- `crates/agenttrace-core/src/pricing.rs:239-241` — the stale-cache
  `download_pricing(Duration::from_secs(5))` call; the single download
  choke point.
- `crates/agenttrace-core/src/pricing.rs:49, 58, 68, 85, 106` — the five
  `pricing_catalog()` callers that route through that choke point.
- `crates/agenttrace-core/src/pricing.rs:84-104` — wall-clock
  `pricing_source` construction to be made stable.
- New vendored snapshot file(s) + whatever includes them (crate has no
  `build.rs` today; the mechanism is the implementer's choice, but the
  snapshot must carry a source and date label).
- `PRIVACY.md:5` — the promise text that currently names only
  `--update-pricing`; must match the new behavior (keep this edit **with**
  CU-2, not separate — see separation hints).
- `CHANGELOG.md` — entry naming the user-visible change (no automatic
  refresh).
- Tests currently mutating the shared cache during `cargo test` (found by
  the assessment's REPRO F3) must stop doing so; offline test runs must
  pass with no cache mutation and no network.

Rationale: closes the only finding that contradicts a published promise
(`PRIVACY.md`), matches the category leader's posture (ccusage, 18,282
stars, headlines offline pre-cached pricing), and unblocks three later
capability items (C2/C3/C5) that assume deterministic pricing.

### CU-3 — optional hygiene riders (droppable without failing the cycle)

- F12: `crates/agenttrace-cli/src/main.rs:574` — `report_language()`
  accepts arbitrary `--lang` values without validation.
- F14: `.gitignore:13` and neighbors — stale entries for an `apps/`
  layout that no longer exists in this repository.
- F19: `.codex-plugin/plugin.json:3` — plugin version `0.7.1` drifts from
  the workspace `version = "0.0.0-dev"` + `RELEASE_VERSION` release
  override mechanism.

Rationale: near-zero effort, thematically "declared surfaces match
behavior"; explicitly optional — drop them if CU-1/CU-2 verification runs
long.

## must_remain_separate hints

These are hints for the conductor's overlap/split pass, not topology
decisions:

1. **CU-1 and CU-2 touch disjoint files** (`lib.rs`/`parser.rs` vs
   `pricing.rs`/snapshot/docs) and have no data dependency — they are
   cleanly separable change units if the conductor's inventory favors
   that. The batch's coherence is thematic, not structural.
2. **Phase documentation is not implementation.** `ROADMAP.md` (+88,
   uncommitted), `docs/reviews/`, `docs/ideation/`, `docs/decisions/`,
   and `docs/stewardship/` are this run's records. Keep them out of the
   implementation diffs for CU-1/CU-2 so the code review surface stays
   clean; whether they land separately is the conductor's call.
3. **`.hermes/` is harness state**, never repository content.
4. **One deliberate coupling:** the `PRIVACY.md` (and `CHANGELOG.md`)
   edits belong with CU-2, because they describe CU-2's behavior change;
   splitting them would leave the promise and the behavior in different
   units.
5. **Riders stay out of CU-1/CU-2 diffs** if units are split — they are
   individually droppable and unrelated to either defect.
6. **Dirty state above must be preserved** through any branch/worktree
   operations the conductor plans.

## Verification expectations (for the conductor's later validation gate)

From the roadmap's acceptance criteria and the decision record's
definition of done: H1 regression tests present and passing; H2 verified
by a network-blocked `agenttrace --overview -f json` run succeeding,
`cargo test` passing with no network and no cache mutation, consecutive
identical runs producing byte-identical JSON, and `PRIVACY.md` matching
observed behavior; CHANGELOG names both changes. Note `cargo clippy`
cannot be part of local verification in this environment (unavailable).

## Provenance

- User's (Hermes work order): the phase objective, the required
  `stewardship_request` shape, the prohibition on choosing Git topology,
  and the conductor's stated responsibilities (inventory, overlap
  detection, splitting, dirty-state preservation, branch/worktree
  planning).
- Delegate's calls: the change-unit decomposition above (three units, the
  CU-2/PRIVACY coupling, the rider list) and all file:line references,
  re-verified against the working tree on 2026-09-02.
- Inference (flagged): rider effort estimates and the snapshot mechanism
  suggestion; neither constrains the implementer.
- Batch selection itself comes from the prioritize phase's decision
  record, which this request cites rather than re-litigates.

## Authoritative references

- `docs/decisions/2026-09-02-cycle-1-batch-selection.md` — the selected
  batch, per-criterion rationale, rejected alternatives, definition of
  done.
- `docs/reviews/2026-09-02-adversarial-repository-assessment.md` (+
  `.json`) — the 19 findings with file:line evidence and reproducers
  (REPRO F1–F4 details in the phase evidence trail).
- `ROADMAP.md:48-136` — the Planned work section: hardening lane H1–H5
  (lines 62-103) and capability lane C1–C5 (lines 104-136), with
  acceptance criteria and evidence expectations per item.
- `docs/ideation/2026-09-02-agenttrace-extensions-ideation.md` — research
  basis for the capability lane and H2's offline-posture rationale.

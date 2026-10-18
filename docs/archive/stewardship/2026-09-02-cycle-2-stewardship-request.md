---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T07:47:00Z"
title: "Stewardship request — cycle 2 batch: trustworthy SQLite ingestion"
summary: "Hands the conductor the selected H1+C8+N7(+N9) maintenance batch as repository change units with surfaces, overlap facts, and separation hints; makes no Git-topology decisions."
keywords: ["agenttrace", "stewardship-request", "sqlite-hardening", "trust-upstream-totals", "change-units"]
cwd: "/work/projects/agenttrace"
resume_focus: "Establish the stewardship contract for the Trustworthy SQLite ingestion batch: inventory this repository, detect overlap (five batch surfaces already carry uncommitted cycle-1 edits), split unrelated concerns, preserve the dirty state listed below, and plan branches/worktrees before implementation begins."
repository: "luoyectl/agenttrace"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
branch: "master"
head: "e005952"
---

# Stewardship request

This document is a **request**, not a contract. Per the conductor work order
(run `0a36c54199de4861b50ddc2dcb26fd8f`, phase stewardship, attempt
`21891c3df36a4267acffdebe3fae0e12`), it describes the selected maintenance
batch and stops there: **it chooses no branch, worktree, commit order, or
any other Git topology.** Those decisions belong to the conductor.

Routing note: no ce-* skill owns stewardship requests (same gap as the two
prior fleet runs, both provenances read). The narrowest installed match is
**ce-handoff** — this document follows its `ce-handoff/v1` frontmatter and
pointer-first body contracts at the campaign's user-directed destination
(`docs/stewardship/`), as cycle 1's request did. Harness disclosure: this
delegate session has no subagent surface; everything here was produced
in-thread.

## Title

agenttrace cycle 2 — "trustworthy SQLite ingestion" (H1 + candidate 8 in
its totals scope + N7, with one droppable N9 rider).

## Summary

Implement the batch selected by the prioritize phase
(`docs/stewardship/2026-09-02-cycle2-prioritization.md`), closing the only
HIGH-severity finding on the board:

- **H1 — SQLite arithmetic remainder** (pass-2 findings N1 HIGH / N2 / N3):
  the SQLite ingestion path never received cycle 1's hardening. Debug
  builds panic at `sqlite_sessions.rs:403`; release builds emit wrapped
  negatives (`tokens_input: -1`) that reach `--audit`/`--context-trends`
  JSON as `total_estimated_cost: -24903104499507.895` with
  `confidence: "high"`. Both reproduced this campaign.
- **Candidate 8, totals scope — trust upstream totals**: prefer OpenCode's
  authoritative session-level `cost` and five token columns when present,
  retain the derived path for older databases, and surface the
  stored-versus-derived delta in `data_health`. The `parent_id`
  hierarchy half stays out (research-gated).
- **N7 — unknown-time bucketing**: timestamp-less SQLite sessions vanish
  from every `--range`/`--since` view today.
- **N9 rider (droppable)**: `--version` is unreachable when `--lang` is
  invalid because `report_language()` runs first.

All four are empirically reproduced defects; reproducers are recorded in
the pass-2 assessment and summarized below.

## Repository candidate

- Working tree: `/work/projects/agenttrace`, branch `master`, HEAD
  `e005952` (the repository's root commit; unchanged all campaign).
- Remote: `https://github.com/luoyectl/agenttrace.git`.
- Layout: Cargo workspace — `crates/agenttrace-core` (the batch's home;
  `sqlite_sessions.rs` 641 lines, `waste.rs` 536), `crates/agenttrace-cli`
  (binary), `crates/agenttrace-tui`. SQLite-fixture test precedent:
  `crates/agenttrace-core/tests/discovery_contract.rs:75-82` builds
  databases via `rusqlite::Connection`; both-provider tests at `:1455`
  and `:1488`.
- Baseline on this tree (re-verified 2026-09-02, prioritize phase):
  `cargo test --workspace` 159/159 passed; `cargo fmt --check` clean;
  `cargo clippy --workspace --all-targets` 0 warnings; all ten
  `scripts/ci/*.sh` check scripts pass. Note the baseline **includes the
  uncommitted cycle-1 work below** — validating against clean HEAD would
  fail.
- **Overlap facts the conductor's inventory needs** (measured this turn):
  - The batch's two primary files are **clean** — `sqlite_sessions.rs`
    and `waste.rs` carry no cycle-1 edits.
  - Five secondary surfaces are **already dirty with uncommitted cycle-1
    work**: `insights.rs`, `governance.rs`, `reports.rs`, `main.rs`,
    `tests/discovery_contract.rs` (plus `CHANGELOG.md`, and `lib.rs` /
    `parser.rs` which hold the hardened converter this batch reuses).
    Cycle-2 edits to those files land **on top of** uncommitted cycle-1
    edits, not beside them.
- **Dirty state to preserve** (delegate will not commit, stash, or revert
  any of it):
  - Modified (15 tracked files): the cycle-1 implementation set
    (`lib.rs`, `parser.rs`, `pricing.rs`, `reports.rs`, `governance.rs`,
    `insights.rs`, `session-cache-adjacent` TUI pair
    `presentation.rs`/`shared.rs`, `main.rs`, `discovery_contract.rs`,
    `ci.yml`, `.gitignore`, `CHANGELOG.md`, `PRIVACY.md`) plus this run's
    `ROADMAP.md` extension.
  - Untracked: `docs/` (`reviews/`, `ideation/`, `decisions/`,
    `stewardship/`), `scripts/pricing/`, `scripts/ci/check-plugin-version.sh`,
    `crates/agenttrace-core/src/pricing_snapshot.json`,
    `testdata/generated/adversarial/` (four JSONL fixtures).
  - Untracked: `.hermes/` — harness state, **not repository content**;
    exclude from any inventory that feeds commits.

## Surfaces (requested change units)

### CU-1 — H1: SQLite arithmetic hardening (N1/N2/N3)

- `crates/agenttrace-core/src/sqlite_sessions.rs:590-599` — delete the
  unsanitized local `number_as_i64` (`as_u64().map(|n| n as i64)` wraps;
  `as_f64` saturates only to overflow later); route through the hardened
  `parser.rs:3582` twin (adapter for the `Option<&Value>` signature).
- `sqlite_sessions.rs:403` — the unguarded `+` joining output and
  reasoning tokens → `saturating_add`.
- `sqlite_sessions.rs:410-413` — the four `+=` accumulators →
  `saturating_add`.
- `sqlite_sessions.rs:179-182` — the four Hermes token reads lack the
  `.max(0)` the adjacent `events`/`tool_calls` lines already have.
- `crates/agenttrace-core/src/waste.rs:180` — `(input - cache_r).max(0)`
  guards the sign, not the overflow → `saturating_sub` before `.max(0)`.
- `crates/agenttrace-core/src/governance.rs` — confidence computation
  sites (`:25`, `:116`, `:137`, `:206`, set around `:255`): never report
  `confidence: "high"` alongside a negative component.
- Red-today regression guards, written **first**: adversarial fixtures
  committed under `testdata/generated/adversarial/sqlite/` (new
  directory), derived from the pass-2 reproducers (machine-local:
  `/tmp/at-assess2/mk_opencode_db.py`, `mk_db2.py`, `mk_db3.py`), with
  debug-mode no-panic and release-mode bounded/non-negative assertions
  across `--sessions`, `--waste`, `--audit`, `--context-trends`.

Rationale: the only HIGH finding; every report surface that reads an
SQLite-backed session currently trusts unvalidated arithmetic.

### CU-2 — Candidate 8 (totals scope): trust upstream totals

- `sqlite_sessions.rs:267-269` (the `session` select) + `:581`
  (`sqlite_has_column`, already used at `:156` and `:261`) — sniff for
  the authoritative columns `cost`, `tokens_input`, `tokens_output`,
  `tokens_reasoning`, `tokens_cache_read`, `tokens_cache_write`; prefer
  stored values when non-null; keep the derived path for older schemas.
- `crates/agenttrace-core/src/insights.rs:279` (`data_health` home) and
  `reports.rs:477` (its JSON wiring) — surface the
  stored-versus-derived delta (count and magnitude).
- Tests: zero delta on well-formed data; non-zero when a field is
  dropped; both-schema coverage.

Rationale: root-cause fix for N1/N2 — totals the provider already
recorded displace re-derivation through the arithmetic being hardened;
the delta turns silent drift into a visible signal. Column list verified
live against upstream `packages/core/src/session/sql.ts` last phase
(`gh api`, research evidence trail).

### CU-3 — N7: unknown-time bucketing

- `sqlite_sessions.rs:243-253` — `filter_since` drops every session whose
  `session_start` fails RFC-3339 parsing whenever a range is set; bucket
  timestamp-less SQLite sessions as unknown-time instead.
- The `--range`/`--since` output path — report how many sessions were
  excluded and why.

Rationale: silent data loss in the same file and the same fixture family
as CU-1; tiny incremental effort.

### CU-4 — N9 rider (droppable without failing the cycle)

- `crates/agenttrace-cli/src/main.rs:149-152` — move the `--version`
  early return above `report_language(&args.lang)?`; add the CLI test
  `--lang fr --version` prints the version.

Rationale: one-file, two-line; explicitly optional.

### Shared

- `CHANGELOG.md` — one entry per user-visible change, kept with its unit.

## must_remain_separate hints

Hints for the conductor's overlap/split pass, not topology decisions:

1. **CU-1 and CU-2 share `sqlite_sessions.rs` and are deliberately
   coupled** — unlike cycle 1's cleanly separable units. The roadmap's
   sequencing note: candidate 8 is the root-cause fix for N1/N2, and its
   delta computation runs through the arithmetic CU-1 hardens. If the
   conductor still splits them, **CU-1 lands first**; CU-2 on unhardened
   arithmetic reintroduces the defect class.
2. **CU-3 rides CU-1's fixture family** but is logically independent;
   separable if the inventory favors it.
3. **CU-4 is disjoint from everything else** and individually droppable.
4. **Five batch surfaces already carry uncommitted cycle-1 edits**
   (`insights.rs`, `governance.rs`, `reports.rs`, `main.rs`,
   `discovery_contract.rs`, `CHANGELOG.md`; also `lib.rs`/`parser.rs` for
   the reused converter). Whatever topology the conductor plans must
   preserve that dirty state and cannot treat those files as clean
   baselines for a second change stream.
5. **Phase documentation is not implementation.** `ROADMAP.md`, `docs/`
   subdirs, and the roadmap diff artifact are this run's records — keep
   them out of the CU diffs so the code review surface stays clean.
6. **`.hermes/` is harness state**, never repository content.
7. **Behavior-change note for reviewers (not a separation hint, but it
   will look like one):** CU-2 changes reported numbers on OpenCode
   databases by design. The delta surfacing and the retained derived
   path are the mitigations — do not split them out of CU-2.

## Verification expectations (for the conductor's later validation gate)

From the prioritization artifact's definition of done: the CU-1 guards
green in debug and release; `grep "fn number_as_i64"
crates/agenttrace-core/src/sqlite_sessions.rs` → 0; adversarial SQLite
fixtures committed; `DataHealth` exposes the stored-versus-derived delta
with the two delta tests; an `opencode.db` fixture with `time_created = 0`
stays visible under `--range 7d`; `--lang fr --version` prints the
version; `cargo test --workspace`, `cargo fmt --check`, `cargo clippy
--workspace --all-targets`, and all ten `scripts/ci/*.sh` green —
**validated against the dirty tree, not clean HEAD**, since the baseline
includes cycle 1.

## Provenance

- User's (Hermes work order): the stewardship_request shape, the
  prohibition on choosing Git topology, and the conductor's stated
  responsibilities (inventory, overlap detection, splitting, dirty-state
  preservation, branch/worktree planning).
- Delegate's calls: the change-unit decomposition (four units, the
  CU-1/CU-2 coupling rule, the rider list), the overlap facts, and all
  file:line references — the latter re-verified against the working tree
  this turn (`git status --short`, grep/sed passes recorded in the
  prioritize phase's evidence trail).
- Inference (flagged): fixture conversion effort from the machine-local
  reproducers; the `DataHealth` delta mechanism (extend the struct vs a
  sibling field) is the implementer's choice.
- Batch selection itself comes from the prioritize phase's artifact,
  which this request cites rather than re-litigates. Assessment findings
  and their reproducers come from the pass-2 review artifact.

## Authoritative references

- `docs/stewardship/2026-09-02-cycle2-prioritization.md` — the selected
  batch, scoring matrix, 7-step execution order, definition of done,
  deferred queue.
- `docs/reviews/2026-09-02-adversarial-repository-assessment-pass2.md` —
  findings N1–N10 with file:line evidence and reproducers.
- `docs/ideation/2026-09-02-agenttrace-extensions-ideation.md` (second
  pass, ideas 8–12) — the upstream-column evidence behind CU-2.
- `ROADMAP.md:101-186` — the hardening lane items this batch executes,
  with acceptance criteria and evidence expectations.
- `docs/stewardship/2026-09-02-cycle-1-stewardship-request.md` — the
  cycle-1 request whose units are the uncommitted tree state this batch
  builds on.

## Re-validation — attempt 5f041eba (2026-09-02)

This stewardship phase was re-issued (attempt
`5f041ebae3a4490bbbf1ac042bccc34d` after
`21891c3df36a4267acffdebe3fae0e12`, identical objective). Per the work
order's "use, do not redo" rule the request above was **re-verified, not
rewritten**; this section is append-only and the body above is unchanged
(pre-addendum md5 `c8dc19e6639f85b0efb6151d000f1977`).

Facts re-checked against the tree this attempt, all holding:

- Repository state unchanged since the first attempt: same HEAD
  `e0059522b4fc74d53824f0e7ea7e4ac94d1465bb` on `master`, same 24-entry
  `git status` (15 modified + 9 untracked), no new branches
  (`git branch -a`), no worktrees (`git worktree list` → this checkout
  only), and no conductor contract/inventory artifacts newer than this
  request (`find .hermes docs -newer …` → none).
- Every load-bearing line reference: `sqlite_sessions.rs:590`
  (`fn number_as_i64`), `:403` (unguarded `+`), `:410` (`+=`),
  `waste.rs:180` (`(input - cache_r).max(0)`), `parser.rs:3582`
  (hardened twin), `insights.rs:279` (`pub fn data_health`),
  `reports.rs:477` (`"data_health": data_health`),
  `main.rs:149-151` (`if args.version` below the `report_language`
  call).
- Overlap facts: `sqlite_sessions.rs` and `waste.rs` still CLEAN;
  `insights.rs`, `governance.rs`, `main.rs` (and the other listed
  secondaries) still carry uncommitted cycle-1 edits.
- Baseline re-run this attempt: `cargo test --workspace` → **159
  passed, 0 failed** on the dirty tree.

The request therefore stands as written; the conductor may proceed to
its inventory/overlap/split pass with no staleness in this document.

## Re-validation — attempt 4fe2dd64 (2026-09-02)

Third identical issuance of this phase (after attempts `21891c3d…` and
`5f041eba…`, both succeeded and both listed as accepted prior
artifacts). Re-validated again; zero drift since the previous attempt:

- Same HEAD `e005952…` on `master`, same 24-entry `git status`, no new
  branches or worktrees, and **no file anywhere in the tree is newer
  than this request's previous addendum** (`find . -newer … -type f`,
  excluding `target/` and `.git/` → none). The conductor's
  inventory/contract pass has still not begun.
- The only content under `.hermes/` is
  `plans/autonomy-prop_a9630aba58334018.md` — a stale autonomy
  proposal from a **prior campaign** (fail-behavior work, fork-push
  acceptance). It is not this run's stewardship contract and imposes
  nothing on this request; it does explain the `fork` remote's
  existence.
- Line references re-spot-checked (`sqlite_sessions.rs:590`, `:403`;
  `waste.rs:180`; `parser.rs:3582`), overlap facts re-confirmed
  (`sqlite_sessions.rs`/`waste.rs` clean; `insights.rs`/`main.rs` dirty
  with cycle-1 edits), baseline re-run: **159 passed, 0 failed**.
- Body and the attempt-5f041eba addendum are unchanged — pre-addendum
  md5 `4d11bf8f13b6712e7296877b6273049c` (recorded before this
  section); stripping this section (plus the one blank separator line
  its heading introduced) reproduces that md5 exactly.

**Standing note to the conductor:** the stewardship_request has now been
delivered and twice re-verified with no new information entering the
system. The next state transition belongs to the conductor's own
inventory/overlap/split pass (or to `implement`), not to another
issuance of this phase; a third re-validation cannot add evidence the
first two did not already carry.

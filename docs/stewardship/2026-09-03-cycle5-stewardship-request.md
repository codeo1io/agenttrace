---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T16:45:24Z"
title: "Stewardship request — cycle 5 batch: honest coverage, honest cache, honest math"
summary: "Hands the conductor the selected cycle-5 maintenance batch (F8-1 lead + F8-2 + F8-3 + F8-5/F8-6 + F8-7/F8-8 as CU-11..CU-15, with C44-min zstd named error as a drop-first stretch CU-16) as repository change units with file:line surfaces, dirty-state inventory, overlap facts, and separation hints; makes no Git-topology decisions."
keywords: ["agenttrace", "stewardship-request", "governance-coverage", "discovered-counts", "cache-eviction", "float-hygiene", "cycle-5"]
cwd: "/work/projects/agenttrace"
resume_focus: "Establish the stewardship contract for the 'honest coverage, honest cache, honest math' batch: inventory this repository, detect overlap (one dirty tracked file ROADMAP.md; seven untracked docs, four of them this campaign's records), split unrelated concerns (dependency churn, pricing-tier candidate 43, CI topology are NOT part of this batch), preserve the dirty state listed below, and plan branches/worktrees before implementation begins."
repository: "luoyectl/agenttrace"
repo_root_sha: "998ade8827820479069d7d3590082a33fbf80045"
branch: "master"
head: "998ade8"
---

# Stewardship request — cycle 5

This document is a **request, not a contract**. Per the conductor work
order (run `dafd34b3940e497f9f1ac234573323ad`, phase stewardship,
attempt `2a31ecec69524bd3b9f5e6a8b3d451ce`), it describes the selected
maintenance batch and stops there: **it chooses no branch, worktree,
commit order, or any other Git topology.** Those decisions belong to
the conductor.

Routing note: no ce-* skill owns stewardship requests (same gap
disclosed by cycles 1–4, all provenances read this campaign). The
narrowest installed match is **ce-handoff**; this document follows its
`ce-handoff/v1` frontmatter and pointer-first body contract at the
campaign's user-directed destination (`docs/stewardship/`). Harness
disclosure: this delegate session has no subagent surface; everything
here was produced in-thread.

## Title

agenttrace cycle 5 — "honest coverage, honest cache, honest math"
(F8-1 lead + F8-2 + F8-3 + F8-5/F8-6 + F8-7/F8-8; five change units
CU-11..CU-15, one drop-first stretch CU-16).

## Summary

Implement the batch selected by the prioritize phase
(`docs/stewardship/2026-09-03-cycle5-prioritization.md`), closing
every way the *aggregates* currently lie or degrade — where cycle 4
closed the ways a single session's parse could lie or lose data. A
governance audit that reports $3.44 over 20 sampled sessions when the
corpus spent $698 over 1,408 (F8-1, HIGH, re-verified live at 203×
understatement this campaign); a "discovered = 71" that erases ~1,340
out-of-range files (F8-2); a session cache that only ever grows —
10,530,891 bytes, 51% dead paths, +12.7 KB in one day (F8-3); a
report path that panics on non-finite catalog math and two percentile
implementations that disagree (F8-5/F8-6); and a guide claiming
schema 4 where the code pins 6, plus a README gap on flag ordering
(F8-7/F8-8). All five were re-verified live this campaign; red→green
acceptance per unit and batch exit criteria are in the prioritization
record §5. The zstd named-error minimum (C44-min) rides as a stretch
unit only if CU-11..CU-15 land green.

**Repository candidate:** `luoyectl/agenttrace`, working tree
`/work/projects/agenttrace`, HEAD `998ade8` on `master`, parent chain
998ade8 → 6632014 → 93aaf05 → e005952 (origin/master). **Campaign PR
#282 is open and unmerged** — this batch continues the campaign's
pending-PR posture; whether cycle 5 lands under #282 or a new PR is a
conductor Git-topology decision this request does not make.

## Change units and surfaces

Every surface below was grep/sed-verified against HEAD `998ade8`
this turn (line numbers move if earlier units in the batch land first —
anchor by symbol names given).

**CU-11 — governance coverage honesty (F8-1, HIGH, lead; closes P3-5's
ambiguity).**
- `crates/agenttrace-cli/src/main.rs:122-123` — `--limit` with
  `default_value_t = 20`: the silent sampler. Redefine as a *display*
  cap for list views; add `--sample N` for explicit bounded sampling
  with disclosure.
- `crates/agenttrace-cli/src/main.rs:217-227` — governance branch
  (`audit || recommend || mcp_governance || context_trends ||
  delivery_evidence`), `.take(args.limit)` at :225 — remove the data
  filter; run unbounded (or bounded only by `--range`).
- `crates/agenttrace-cli/src/main.rs:247-251` — `compare` branch,
  `.take(args.limit)` at :249 — same treatment.
- `crates/agenttrace-cli/src/main.rs:298` + `:872`
  (`render_session_list`) — apply the display-cap semantics in the
  overview/list path too (this closes P3-5's "silently ignored" half).
- `crates/agenttrace-core/src/governance.rs` — `cost_audit`,
  `recommendations`, `mcp_governance`, `context_trends` (+ the compare
  report builder) gain `audited_sessions`/`total_sessions` (+
  exclusion-reason) fields.
- `crates/agenttrace-core/src/reports.rs` —
  `render_governance_report` prints "(auditing N of M sessions)" in
  text/HTML/Markdown outputs.
- Constraint: `main.rs:79-80` `--search-limit` is a *different,
  correct* cap (search result truncation); do not conflate the two
  flags.

**CU-12 — truthful discovery accounting (F8-2).**
- `crates/agenttrace-cli/src/main.rs:336-341` — `data_health(...)` is
  called with `sessions.len() + skipped` instead of
  `LoadReport::discovered`; the overview branch discards the loader's
  truth.
- `crates/agenttrace-core/src/discovery.rs:33-43` (`LoadReport`,
  field `discovered` at :35) and `:197` — the authoritative count.
- `crates/agenttrace-core/src/insights.rs:296-330` — `data_health`
  signature and `discovered`/`skipped` math (`skipped =
  discovered.saturating_sub(parsed)` at :310 becomes truthful once the
  input is); ranged runs must split parsed/out-of-range instead of
  shrinking the denominator.

**CU-13 — session-cache eviction (F8-3).**
- `crates/agenttrace-core/src/session_cache.rs:325`
  (`load_session_cache`), `:558` (`save_session_cache`), and the
  visit-time staleness path around `:597-616` (`cached_session` /
  `is_fresh` / `delete_cached_session_key`) — add load-time pruning of
  nonexistent paths (cache entries are derivable; pruning costs only
  re-parse time) plus a documented entry/byte bound. Acceptance is
  measurable on this host: 1,487 entries / 761 dead before → pruned
  and smaller after.

**CU-14 — float hygiene (F8-5/F8-6).**
- `crates/agenttrace-core/src/reports.rs:1446-1452` — `json_float`
  `.expect("float serializes")` panics on inf/NaN; make it total
  (null + a data-health flag).
- `crates/agenttrace-core/src/pricing.rs:330-345` — `convert_litellm`
  multiplies catalog rates by 1e6 and can produce non-finite values
  from hostile catalog data; add a finiteness guard at conversion.
- `crates/agenttrace-core/src/lib.rs:1309` vs
  `crates/agenttrace-core/src/reports.rs:1777` — two private
  `percentile` impls disagree (20 vs 19 at p=0.95). Design note: the
  survivor must satisfy the Go-parity test
  `percentile_matches_go_index_rule` (`lib.rs:1745`) — keep
  `lib.rs:1309`'s rule as the single definition unless verified
  against the Go reference otherwise, and update that test and the
  affected goldens together.

**CU-15 — docs-contract check (F8-7/F8-8). Land last: it pins the
semantics CU-11/12 settle.**
- `docs/guides/governance-reports.md:52-55` — claims "SQLite snapshot
  is schema 4"; code pins `SQLITE_SNAPSHOT_SCHEMA_VERSION = 6`
  (`session_cache.rs:13`). Note the same sentence's "session cache is
  schema 17" is *correct* (`SESSION_CACHE_SCHEMA_VERSION = 17`,
  `session_cache.rs:8`) — fix the snapshot number and the auto-refresh
  claim (loader is network-free; `--update-pricing` is the only
  network path, matching `PRIVACY.md`).
- `README.md` — add the flags-before-positional note.
- `scripts/ci/check-docs-commands.sh` — extend to pin both schema
  constants and the no-auto-refresh statement against source.

**CU-16 — stretch (drop-first): zstd named error (C44-min).**
- `crates/agenttrace-core/src/parser.rs:20-34` — beside the existing
  UTF-16 detection, sniff the zstd magic `28 B5 2F FD` and return a
  named, actionable error (Codex ≥2026-08-28 can emit zstd shared
  rollups, PR #41357; today they misparse as "not valid UTF-8"); a
  committed fixture under `testdata/generated/adversarial/`. **Full
  decode is out of scope** (adds a dependency — see separation hints).

## Repository state the conductor must preserve

- **Dirty tracked file:** `ROADMAP.md` (`+244/−14`, the roadmap
  phase's edit — this campaign's own work, not operator state).
- **Untracked docs (7):** this campaign's four —
  `docs/reviews/2026-09-03-adversarial-repository-assessment-pass8.md`,
  `docs/research/2026-09-03-extensions-research-pass7.md`,
  `docs/stewardship/2026-09-03-cycle5-prioritization.md`, and this
  file — plus three prior-run carryovers
  (`docs/stewardship/2026-09-02-cycle-4-reconciliation.md`,
  `2026-09-02-reconciliation-record.md`, and the two
  `roadmap-cycle*.diff` companions; these are flagged as hygiene debt
  F8-10 and must not be silently deleted or swept into a code commit).
- Nothing is staged; no commits/pushes are made by this phase.

## must_remain_separate (separation hints)

1. **Dependency refresh** (Cargo.lock / rusqlite→0.40.2, SQLite
   3.53.2, ureq, crossterm; dependabot PRs #278/#279) must **not** ride
   this batch — mixing dependency churn with behavior fixes muddies
   attribution; F8-4 is deferred post-#282 by the prioritization
   record.
2. **Candidate 43 pricing tiers** (`pricing.rs` `Price` fields,
   `pricing_snapshot.json` regeneration, cost goldens) is cycle-6
   scope; it churns every cost figure and would contaminate CU-11's
   coverage before/after evidence. `pricing.rs` *is* touched by CU-14
   — but only the local `convert_litellm` finiteness guard belongs to
   this batch.
3. **zstd full decode** (new crate dependency) is separate from
   CU-16's named-error minimum, which adds no dependency.
4. **Campaign doc records** (`ROADMAP.md` edit + the untracked
   review/research/stewardship files) should stay distinguishable from
   the code change units; the cycle-4 precedent landed records with
   the cycle head, but as an identifiable concern.
5. **`.github/workflows/*`** (CI topology) is untouched — CI execution
   is prohibited this cycle and is a conductor decision.
6. **`crates/agenttrace-tui/*`** is out of the batch: CU-11's
   disclosure is a CLI/report-surface change; only touch TUI if a
   failing golden forces it.
7. **`--search-limit`** (`main.rs:79-80`) keeps its own semantics —
   distinct, correct cap; not part of CU-11's `--limit` redefinition.

## What this phase did NOT do

No code changes, no Git topology choices, no commits, no pushes, no
CI. Implementation (CU-11..CU-15, stretch CU-16), final validation,
and the commit/push/PR gates are later actions in this run. Red→green
acceptance and batch exit criteria live in
`docs/stewardship/2026-09-03-cycle5-prioritization.md` §5.

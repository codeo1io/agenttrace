---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T08:05:00Z"
title: "Stewardship request update — cycle 2 batch amended: launch-safety and claim-integrity riders"
summary: "Amends the cycle-2 stewardship request with the three riders the run-9fcc0661 prioritize phase added (P4-1, P5-3, P5-5), the P5-1/P5-2 falsification evidence now riding CU-1, and the re-measured dirty-state inventory; makes no Git-topology decisions."
keywords: ["agenttrace", "stewardship-request", "cycle-2", "launch-safety", "claim-integrity", "change-units"]
cwd: "/work/projects/agenttrace"
resume_focus: "Apply the conductor's inventory/overlap/split pass to the AMENDED cycle-2 batch: the four change units already requested stand unchanged; three new rider units (CU-5 CU-6 CU-7) join them with their own surfaces and separation hints."
repository: "luoyectl/agenttrace"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
branch: "master"
head: "e005952"
---

# Stewardship request — update for run 9fcc0661

This is an **amendment**, not a replacement. The authoritative cycle-2
request is `2026-09-02-cycle-2-stewardship-request.md` in this directory
(run `0a36c54199de4861b50ddc2dcb26fd8f`, thrice issued, twice
re-validated, zero drift). Per this work order (run
`9fcc0661af474e2783a0dee7541f6ddb`, attempt `2b83d6283f7946ed9d741fb85c7c6f31`),
this document carries the **one thing that changed since**: the prioritize
phase re-validated the batch against assessment passes 3–5 and research
passes 3–4 and amended it
(`docs/stewardship/2026-09-02-cycle2-prioritization-update.md`). The batch
grew three riders and CU-1 gained falsification evidence. **No Git
topology is chosen here** — branch/worktree/commit decisions remain the
conductor's.

Routing note (unchanged gap, re-disclosed): no ce-* skill owns stewardship
requests; this amendment follows the same ce-handoff/v1 frontmatter and
pointer-first body as the base request. Harness disclosure: this delegate
session has no subagent surface.

## What changed in the batch

CU-1 through CU-4 and all six separation hints of the base request **stand
verbatim** — re-verified below. The amendments:

1. **CU-1 (H1) is now carrying falsification evidence, not just
   reproductions.** Pass 5 (`docs/reviews/2026-09-02-adversarial-repository-assessment-pass5.md`)
   showed `CHANGELOG.md:7` claims the cycle-1 hardening landed repo-wide
   while a debug build still exits 101 on an adversarial `opencode.db`
   (`sqlite_sessions.rs:410`) and a `u64::MAX` input reports
   `"input": -1` through `--latest -f json` (`:590-600`). The CHANGELOG
   correction is therefore part of CU-1's done state, and the pass-5
   reproducers (`/tmp/at-assess/repro_sqlite_overflow.py`,
   `/tmp/at-assess/fuzz_mutate.py`) are the source for the committed
   fixtures — superseding the pass-2 machine-local paths named in the base
   request.
2. **CU-5 (new, HIGH rider): P4-1 non-tty TUI launch guard.**
3. **CU-6 (new, XS rider): P5-3 pricing-snapshot date pin test.**
4. **CU-7 (new, XS rider): P5-5 ignore `.hermes/`.**

## CU-5 — P4-1: TUI must not panic when stdout is not a tty

- `crates/agenttrace-tui/src/app.rs:71-74` — `run_with_app` calls
  `ratatui::init()`, which **panics** (exit 101, Rust backtrace) when
  stdout is not a tty (ENXIO/ENOTTY); the function already returns
  `anyhow::Result<()>`, so the error path exists for free. Both callers
  are in-file (`:56`, `:60`).
- Fix: construct the terminal manually
  (`Terminal::new(CrosstermBackend::new(io::stdout()))?`) or guard with
  `std::io::IsTerminal` (std since 1.70; workspace MSRV is **1.80**,
  verified in `Cargo.toml:12` — no dependency change needed) and print
  `agenttrace: not a terminal — use --overview for non-interactive output`,
  exiting 1.
- Test precedent exists in-crate: `app.rs:1716` declares `mod tests;`.
  Regression test: run the binary with piped stdout and assert a normal
  exit code (1) with the message — never 101.
- Why a rider, not its own cycle: HIGH severity (the **README quickstart
  command**, reproduced failing from five different documents; every
  pipe/CI/cron/docker-without-t context), XS effort, one file, zero
  overlap with CU-1..CU-4.

## CU-6 — P5-3: pin `PRICING_SNAPSHOT_DATE` to the snapshot's date

- `crates/agenttrace-core/src/pricing.rs:16` — `const
  PRICING_SNAPSHOT_DATE: &str = "2026-09-02"` is currently synced to
  `pricing_snapshot.json`'s `_snapshot.date` **by prose only**
  (`scripts/pricing/update-snapshot.sh` header instructs a human).
- Fix: a unit test in `pricing.rs`'s existing `#[cfg(test)]` module that
  reads `crates/agenttrace-core/src/pricing_snapshot.json`, asserts
  `_snapshot.date == PRICING_SNAPSHOT_DATE`, and goes **red** on drift.
  Live values verified this turn: `_snapshot.date = 2026-09-02`,
  `models: 2458`. The label test at `pricing.rs:1277` already formats the
  const — the new test pins the const to the data.
- Scope discipline: **test-only**. This is deliberately NOT candidate 24
  (cost provenance, `convert_litellm`/`provider_priority`) — see
  separation hint 9.

## CU-7 — P5-5: ignore `.hermes/`

- `.gitignore` — add exactly the `.hermes/` entry. `.hermes/` is harness
  state (currently `?? .hermes/` in `git status`), whose content includes
  a **prior campaign's plan with remote-push instructions** ("Push
  feature branches to the git remote named fork, never to origin") —
  exactly the kind of file that must never ride into a commit by
  accident. Verified this turn: no `.hermes` entry exists in `.gitignore`.
- One line, one concern: no other ignore-pattern changes in this unit.

## Re-verified this turn (all hold)

- Repository state: HEAD `e005952…` on `master` (same as every prior
  issuance); `git status --short` now lists **25 entries** — 15 modified
  + 10 untracked. Delta vs the base request's 24: this run added
  `docs/research/2026-09-02-extensions-research-pass4.md` (untracked) and
  modified `ROADMAP.md` (already counted). `docs/stewardship/` and
  `docs/reviews/` gained this run's artifacts inside already-untracked
  directories.
- The 15 modified files are still exactly the base request's cycle-1 set
  plus `ROADMAP.md`; **`sqlite_sessions.rs`, `waste.rs`, and
  `app.rs` remain clean** (no uncommitted edits) — CU-1's two primary
  files and CU-5's single file all start from HEAD.
- Rider surfaces: `app.rs:71-74` (`ratatui::init()` in `run_with_app`),
  `pricing.rs:16` (const) with `_snapshot.date=2026-09-02` in
  `pricing_snapshot.json`, `.gitignore` lacks `.hermes` (grep → none).
  `pricing.rs` and `.gitignore` **are dirty with cycle-1 edits** — CU-6
  and CU-7 land on top of them, not beside them.
- Baseline (from pass 5, same tree): 159/159 tests, clippy 0 warnings,
  all ten `scripts/ci/check-*.sh` pass. No code has changed since.

## must_remain_separate — amendments

The base request's hints 1–7 stand. Added:

8. **CU-5 is disjoint from CU-1..CU-4** (different crate, clean file,
   no shared fixture). Splittable in any direction; the only ordering
   constraint is internal to the base request (CU-1 before CU-2).
9. **CU-6 must not grow into candidate 24.** Both touch
   `pricing.rs`, but CU-6 is a date-pin test while C24 (provider-rate
   provenance: `convert_litellm` collapse, `provider_priority`,
   snapshot regeneration, JSON contract change) is a separate capability
   cycle by explicit prioritization decision. If the conductor's
   inventory groups by file, override that grouping here.
10. **CU-6 and CU-7 land on dirty files** (`pricing.rs`, `.gitignore`
    carry cycle-1 edits). They cannot be split from those files' existing
    dirty state by topology; keep them as small, attributable diffs
    within it.
11. **Phase documentation stays out of CU diffs** (reaffirmed): the base
    request's hint 5 extends to this run's additions — `ROADMAP.md`
    edits, `docs/research/…pass4.md`,
    `docs/stewardship/…prioritization-update.md`, and this file are
    records, not implementation.
12. **`.hermes/` becomes ignored, never committed** — CU-7's whole point
    (base request hint 6, now with a content-based reason).

## Verification expectations for the riders

From the prioritization update's definition of done: `agenttrace <
/dev/null` (and piped-stdout invocations) exits **non-101** with a
message; the snapshot-date test passes and fails red on a mismatched
const; `git status` no longer lists `.hermes/`. All validated on the
dirty tree, as in the base request.

## Authoritative references

- `docs/stewardship/2026-09-02-cycle2-prioritization-update.md` — the
  amended selection (this run's prioritize phase): riders, scoring
  deltas, 10-step execution order, definition of done, deferred queue.
- `docs/stewardship/2026-09-02-cycle-2-stewardship-request.md` — the
  base request: CU-1..CU-4, overlap facts, hints 1–7.
- `docs/reviews/2026-09-02-adversarial-repository-assessment-pass4.md`
  (P4-1 evidence, reproduced exit 101) and `…pass5.md` (P5-1/P5-2/P5-3/
  P5-5 evidence and reproducers).
- `ROADMAP.md:150-199` — the riders' hardening-lane items with acceptance
  criteria.

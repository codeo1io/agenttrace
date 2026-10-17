---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-30T21:51:41Z"
title: "Stewardship request — campaign 1f5ad3cf cycle 1: count every session, disclose every artifact"
summary: "Hands the conductor the selected rm-084 + rm-086 batch as two repository change units with live file:line surfaces re-verified at HEAD 9d88b36, fresh fleet-overlap facts, dirty state to preserve, and separation hints; makes no Git-topology decisions."
keywords: ["agenttrace", "stewardship-request", "discovery", "pi-fork", "senpi", "omo", "privacy", "clear-cache", "change-units", "rm-084", "rm-086"]
cwd: "/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-792ef47bdeaf-792ef47b"
repository: "codeo1io/agenttrace"
repo_root_sha: "9d88b36750a991bd1436dbbc91b4579c39003067"
branch: "conductor/run-792ef47bdeaf"
head: "9d88b36"
---

# Stewardship request

This document is a **request, not a contract**. Per the conductor work order
(run `792ef47bdeaf42a0a76d02ac170473c4`, phase stewardship, attempt
`1b78297a90e447619fbe91f2975a57cc`, intent
`establish_repository_stewardship`), it describes the selected maintenance
batch and stops there: **it chooses no branch, worktree, commit order, or
any other Git topology.** Those decisions belong to the conductor.

Routing note: no compound-engineering router / ce-* skill is installed in
this delegate's pi session (only `agent-reach`, an internet-research skill,
is exposed — unused here). Per standing precedent this request was produced
directly on the work-order contract, mirroring the `ce-handoff/v1`
frontmatter and body shape of
`docs/stewardship/2026-09-30-cycle1-stewardship-request.md`.

## Title

agenttrace campaign `1f5ad3cfd4164b229213951dbecc302c` cycle 1 —
**"Count every session, disclose every artifact"** (`rm-084` + `rm-086`).

## Summary

Implement the two-unit batch selected by the prioritize phase
(`docs/stewardship/2026-10-01-cycle1-prioritization.md`, §1):

- **Unit A — `rm-084`, pi-fork and pi-profile session-home discovery
  (compatibility, 86.0, lead, M effort).** The pi-family registry at
  `crates/agenttrace-core/src/discovery.rs:103-116` registers exactly three
  homes (`~/.pi/agent/sessions`, `~/.config/pi/agent/sessions`,
  `~/.omp/agent/sessions`) while the host this tool runs on carries
  `~/.senpi/agent-cliproxy-only/sessions` (**1404 jsonl, $3.5K**), `~/.omo`
  (**150 jsonl, $31.36**), and `~/.pi/<profile>/sessions` (**8187 jsonl**,
  grew from 8010 in one day) — ≈9,741 pi-family files invisible to default
  discovery. Live baseline re-proven by the prioritize phase at HEAD
  `9d88b36`: default `--overview` reports **5700 sessions / $4.4K** with
  By-Agent "Pi 199 Sessions $307.43" and no fork/profile line. Both corpora
  are pi `version:3` transcripts the existing parser ingests unmodified via
  `-d` — **the gap is discovery-only.** Companion defects: `pi_source_for_path`
  (`crates/agenttrace-core/src/parser.rs:1415-1424`) labels every
  non-default root `oh_my_pi` (so newly discovered corpora would render
  under the wrong source name), and `-d` has blank `--help` text
  (`crates/agenttrace-cli/src/main.rs:38-39` — `#[arg(short = 'd')]` with
  no doc comment).
- **Unit B — `rm-086`, at-rest disclosure in PRIVACY.md and `--clear-cache`
  parity (security, 80.0, rider, S effort).** `PRIVACY.md` is 7 lines and
  never names `~/.cache/agenttrace`, while HEAD writes at least five
  artifact classes there: `sessions.json` (persisting conversation-derived
  `Name`/`CWD`/`FileUsage`/`ToolUsage` via `GoSession`,
  `crates/agenttrace-core/src/session_cache.rs:104-190`),
  `hermes-sqlite.json` + `opencode-sqlite.json` (whole sqlite-derived
  session sets incl. titles), `pricing.json`, and the `statusline.jsonl`
  journal (10MiB bound, `crates/agenttrace-core/src/statusline.rs:29`,
  appended on every hook run, `:275-298`).
  `clear_session_cache` (`crates/agenttrace-core/src/session_cache.rs:216-229`)
  removes exactly **three** of the five+ classes and skips the journal and
  the pricing cache. The rider compounds with the lead: widened discovery
  means more conversation-derived strings flow into `sessions.json`.

Both units are defect-class reliability/security work, provable offline on
this host, with no dependency between them and no dependency on any open
PR. **Precision added by this phase (verified at HEAD):**
`statusline_capture_path()` (`crates/agenttrace-core/src/statusline.rs:106-113`)
is already `pub` **and re-exported at `crates/agenttrace-core/src/lib.rs:70-72`**,
and honors `AGENTTRACE_SESSION_CACHE_DIR` exactly like
`session_cache_path()` (`session_cache.rs:207-213`) — so Unit B can extend
`clear_session_cache` by importing the existing function and needs **zero
edits to `statusline.rs`**, dropping that contended file from the batch's
footprint entirely.

## Change units and surfaces

### Unit A — rm-084 (pi-fork + pi-profile discovery)

- `crates/agenttrace-core/src/discovery.rs:103-116` — the pi-family
  `KnownSessionDir` registry: add fork homes (senpi/omo class) and
  `<home>/<profile>/sessions` variants with the same dedup/uniqueness rules
  as `~/.pi`; respect `PI_CODING_AGENT_DIR` relocation.
- `crates/agenttrace-core/src/discovery.rs:143` — `discover_session_dirs()`:
  **edits must stay additive** so run-83642957's uncommitted rm-055
  canonical-keying rewrite (stale base `90a4ef5`) composes at fold.
- `crates/agenttrace-core/src/parser.rs:1415-1424` — `pi_source_for_path`:
  label by actual home root (including the XDG root currently mislabeled
  `oh_my_pi`); 9-line isolated function; unit tests stay in-module.
- `crates/agenttrace-cli/src/main.rs:38-39` — `#[arg(short = 'd')]` /
  `dir: Option<String>`: add non-blank help text (1-line ride-along).
- `crates/agenttrace-core/tests/` — **new** golden test file pinning a
  multi-home, multi-profile fixture corpus (not `discovery_contract.rs`,
  which two in-flight lanes hold uncommitted).

Acceptance flavor: fixture corpus red first against default discovery;
then default `--overview` on this host grows senpi/omo/pi-profile
attribution vs the recorded 5700-session/$4.4K baseline; source labels
match actual homes; `-d --help` non-blank.

### Unit B — rm-086 (PRIVACY.md disclosure + --clear-cache parity)

- `PRIVACY.md:1-7` — rewrite as a data-class + purge-command table covering
  every artifact class the code writes under the cache root.
- `crates/agenttrace-core/src/session_cache.rs:216-229` —
  `clear_session_cache()`: remove or explicitly report every listed
  artifact, importing the already-pub `statusline_capture_path()` for the
  journal; decide `pricing.json` disposition (remove vs
  explicitly-retain-by-design) so "every listed artifact" stays truthful.
- `crates/agenttrace-core/src/session_cache.rs:104-190` + `:220-222` —
  read-only context: the `GoSession` fields and sqlite snapshots that make
  the disclosure necessary.
- `crates/agenttrace-core/src/statusline.rs:29`, `:106-113`, `:275-298` —
  read-only context: journal bound, pub path helper (already exported —
  **no edit required in this file**), append/compact mechanics.
- Registry test (in `session_cache.rs` inline `mod tests` or the new test
  file) pinning PRIVACY text against the code's cache-path construction —
  this is what guards the `hermes-sqlite-v7-*.json` orphan drift already
  visible on this host (no `v7` string exists at HEAD).

Acceptance flavor: sandboxed-HOME run → artifacts present → `--clear-cache`
→ directory empty of every class PRIVACY.md discloses (or the retained
class explicitly reported).

## Repository candidates (inventory facts, not topology choices)

- `/work/projects/agenttrace` — canonical checkout, **porcelain clean**,
  currently on branch `ci/glibc-baseline-ubuntu2204` at `90a4ef5` (the
  pre-master base; `9d88b36` is 17 commits ahead of it). Remote: `fork` →
  `git@github.com:codeo1io/agenttrace.git`; `fork/master` = `1472e35`.
- `/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-792ef47bdeaf-792ef47b`
  — this campaign's worktree at `9d88b36` on `conductor/run-792ef47bdeaf`,
  holding the run's uncommitted deliverables (see dirty state below).

## Overlap facts (fleet re-swept this phase, 2026-09-30T21:5xZ)

Live uncommitted code lanes and their intersection with this batch:

| Worktree | Base | Uncommitted `.rs` set | Batch overlap |
|---|---|---|---|
| run-83642957 | 90a4ef5 (stale, 17 behind) | discovery.rs, parser.rs, statusline.rs, main.rs, doctor.rs, history.rs, lib.rs, tests/discovery_contract.rs (rm-055..064) | **rm-084**: discovery.rs + parser.rs (composes: rm-055 rewrites admission keying, rm-084 adds registry entries + profile expansion) |
| run-52465b9e | ce27969 | main.rs, lib.rs, reports.rs, search.rs, statusline.rs, tests/discovery_contract.rs, filters.rs, render.rs (rm-034..039) | main.rs only, vs the 1-line `-d` help ride-along — trivially mergeable |
| run-aa9c4fd6 | 9d88b36 | insights.rs, lib.rs, pricing.rs, reports.rs, explorer.rs, presentation.rs, tui/tests.rs | none |
| run-e602bb69 | 9d88b36 | main.rs, upstream.rs, doctor.rs, lib.rs, pricing.rs, tui/tests.rs | main.rs only, vs the same 1-line ride-along |
| integration-58cb435705b8 | 1472e35 (= fork/master, staged merge) | parser.rs, pricing.rs | **rm-084**: parser.rs only — `pi_source_for_path` is a 9-line isolated function |

ROADMAP-only claimers (no code): run-1766ab4e, run-266b6e2b (minted
rm-091..099), run-40208f3d, and **run-b8d7db05 — the colliding mint of
rm-084..rm-087 (mtime 2026-09-30T21:04Z, 16 minutes before this campaign's
21:20Z rm-084..rm-090 mint)**. IDs are campaign-local until merge; cite by
title + line; the commit gate reconciles IDs per the PR #16/#17 precedent.

**Dirty state in this worktree (must be preserved, commit-gate owned):**
` M ROADMAP.md` (roadmap-phase deliverable, +45/-0, items rm-084..rm-090 +
rm-027 addendum) and untracked
`docs/stewardship/2026-10-01-cycle1-prioritization.md` plus this document.
Implementation phases must not revert, restage, or fold these away.
`ROADMAP.md` entries rm-084 (`:339`) and rm-086 (`:363`) remain
`status: candidate`; status flips belong to the commit gate.

## Separation hints (must_remain_separate)

1. **Unit A vs Unit B**: `rm-084` (discovery.rs, parser.rs, 1-line main.rs
   help, new test file) vs `rm-086` (PRIVACY.md, session_cache.rs) —
   disjoint files, no shared symbols, independently reviewable, revertable,
   and landable. Do not merge them into one commit-sized blob.
2. **Implementation edits vs ROADMAP.md status fields**: implementers do
   not flip `rm-084`/`rm-086` toward implemented/done; compound appends
   its cycle bullet, the commit gate flips status.
3. **Batch tests vs `crates/agenttrace-core/tests/discovery_contract.rs`**:
   new golden tests go in a NEW test file — that contract file is held
   uncommitted by two in-flight lanes (run-83642957, run-52465b9e);
   adding there triples the fold burden.
4. **Batch vs the `agenttrace-tui` crate**: this batch touches no TUI
   file. Two sibling lanes already hold independent hermeticity fixes
   (run-aa9c4fd6, run-e602bb69); a third wrapper is recorded fold-time
   debt. Consequence: every gate run uses a sandboxed HOME with
   RUSTUP_HOME/CARGO_HOME exported.
5. **rm-084 registry additions vs run-83642957's uncommitted
   discovery.rs/parser.rs rewrite**: keep rm-084's discovery edits
   append-only (registry entries + a profile-expansion step); do not
   restructure `discover_session_dirs` keying that lane owns.
6. **Live-probe scratch vs the repository and the operator's real
   `~/.cache/agenttrace`**: fixture corpora, crafted journals, and
   cold-parse cache dirs belong under `/tmp` or disposable
   `AGENTTRACE_SESSION_CACHE_DIR` roots only — never inside the repo tree,
   never in the real cache (the 2.4MiB real journal and the v7 orphans on
   this host are evidence, not test fixtures).

## Out of scope (recorded, not selected)

- `rm-085` + `rm-087` are the designated cycle-2 pair (reports.rs lane
  contention); gen-1 `rm-020` per-model pricing is the cycle-2/3 anchor.
- The hermetic TUI test fix is explicitly NOT re-implemented here (two
  sibling fixes exist; fold keeps exactly one).
- PR/merge resolution windows and the rm-084..rm-090 vs run-b8d7db05
  rm-084..087 ID collision are commit-gate concerns, surfaced as
  provisional future work, not part of this batch.
- Sibling campaign d7ca90df (cad2c25d) first proved the `~/.pi/<profile>`
  half (its research RC-1 was folded into rm-084); its selection is not
  visible from here — landing first claims the work, and the fold
  reconciles on the earlier merge.

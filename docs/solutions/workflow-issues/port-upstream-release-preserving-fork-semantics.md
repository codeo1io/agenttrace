---
title: Port upstream releases onto this fork as a merge with lineage-aware conflict rules
date: 2026-09-30
category: solutions/workflow-issues
module: fork-maintenance
problem_type: workflow_issue
component: development_workflow
severity: high
applies_when:
  - "The fork has fallen one or more upstream releases behind and upstream touched files the fork also modified"
  - "An upstream release rewrites a data model or schema constant the fork carries divergent semantics for"
tags: [upstream-sync, merge-conflicts, fork-maintenance, session-cache, release-port, drift]
---

# Port upstream releases onto this fork as a merge with lineage-aware conflict rules

## Context

This repository is a maintenance fork: its history carries salvage work the
upstream does not have (session-cache bounds enforcement, a
display-title helper, a CJK-aware token estimator, the Antigravity trajectory
sidecar parser), while upstream moves daily and periodically rewrites models
the fork depends on. Upstream v0.9.0 (PR #286) replaced the
loop-diagnostics model and touched the session cache — the exact file that
carries the fork's core fixes. Every cycle the port is deferred, the conflict
surface grows: the port cost compounds with upstream's cadence.

The cycle-1 port of v0.9.0 (13 files, three conflicts) closed that gap. The
procedure below is what made it safe to execute and cheap to re-run.

## Guidance

**1. Measure the drift before touching anything.** Run
`scripts/upstream-delta` (landed with the port). It reports merge-base,
ahead/behind counts, diverged files, upstream releases the fork lacks, and
the unported commit list; `--json` makes it consumable by tooling. Use its
output to size the port and to detect upstream motion *during* the work —
this cycle it caught a commit landing mid-port.

**2. Decide topology from the numbers, not from habit.** Count commits
ahead/behind. Far ahead of upstream (this fork: 37 ahead / 7 behind at port
time) means a patch-apply or cherry-pick series re-creates conflicts one
commit at a time; `git merge <tag>` resolves them once with three-way
context. Far behind and clean means the inverse. Do not assume "port" means
"apply a diff".

**3. Resolve conflicts by lineage-aware rules, not by picking sides:**

- *Fork safety semantics win over upstream rewrites.* When upstream rewrote a
  function the fork fixed for correctness — the session cache's entry/byte
  bounds enforcement and unique-temp-path handling — keep the fork's version
  even when upstream's is prettier. Upstream's rewrite produced equivalent
  output without the guarantees; the guarantees are the point.
- *Version-bearing constants move strictly past BOTH lineages.* The fork's
  session-cache schema was 17, upstream's was 19; the merge took 20. A value
  equal to either lineage would let one side's stale snapshots skip
  regeneration under the combined semantics. The contract tests encode this —
  when the bump invalidates fixtures, update the literals (note: the fixture
  files read as binary to `grep`; use `grep -a`).
- *Compose strict supersets when both sides carry unique value.* The fork's
  title-cleaning helper and upstream's tag/boilerplate stripper each removed
  noise the other did not; chaining them kept both behaviors with zero dead
  code. A "pick one side" resolution here would have silently dropped a fix.
- *Different features at one dispatch site can coexist.* Two distinct
  Antigravity parsers (fork sidecar format, upstream JSONL format) both
  stayed; verify the dispatcher routes to each rather than deleting the
  loser.

**4. Expect scripted conflict surgery to fail in specific, detectable ways.**
Observed failure modes, all from this port:

- Conflict-marker labels vary *between files in the same merge* (`ours`/`theirs`
  in one, `HEAD`/`theirs` in another). Enumerate with a loose pattern and read
  the raw region before any scripted edit.
- An inverted marker filter (keep-what-you-meant-to-drop) deletes whole
  functions silently. After any scripted resolution, re-read the region and
  rebuild from the conflicted state if the shape looks wrong.
- A half-merged hunk splices upstream's new signature onto the fork's old
  body, leaving a symbol defined twice. This only surfaces at compile time —
  after surgery, verify one-definition-per-symbol for every touched symbol.
- Header/import conflicts lose imports that both sides used. The compile
  error names the missing symbol; check the fork's pre-merge header before
  "fixing" the import list by hand.

**5. Hold the verification bar at zero warnings, then run the ladder.**
`cargo build --workspace` with zero errors *and* zero warnings — orphaned
upstream machinery behind a kept fork helper shows up as dead-code warnings
and means the composition is incomplete. Then focused per-crate tests, then
the full workspace suite. For this port: 240/240 across cli, core, and tui.

## Why This Matters

The failure mode this procedure prevents is silent: a mishandled merge
compiles, passes upstream's tests, and drops the fork's correctness fixes —
the bounds enforcement that guards snapshot regeneration — with no error
anywhere. The drift report makes the cost of deferral visible before it is
paid; the conflict rules make the port a reviewable, repeatable operation
instead of an archaeology exercise; the trap list converts four silent
failure modes into checkable exit conditions.

## When to Apply

- Any upstream release port onto this fork, regardless of size.
- Any merge whose conflicts touch files where the fork carries fixes upstream
  does not have (currently: the session cache, the title helpers, the token
  estimator, the Antigravity parsers).
- Periodically without porting: run `scripts/upstream-delta` to keep drift
  measured.

## Examples

The v0.9.0 port executed under this procedure: merge of the `v0.9.0` tag
(PR #285 + #286 content; earlier commits already below the merge-base),
13 files changed, three conflicts (`session_cache.rs`, `lib.rs`,
`parser.rs`). Schema constant resolved 17-vs-19 to 20; fork bounds functions
retained verbatim; title cleaning composed as a superset; both Antigravity
parsers retained with dispatch verified. Outcome: workspace build clean at
zero warnings, full suite 240/240. The upstream-delta report tells the
before/after story directly: against pre-merge HEAD it counts the fork
behind by 8 with the v0.9.0 release unported; once the merge commits, the
unported remainder is only the post-v0.9.0 governance wave (#287–#292) —
the port itself leaves no release gap.

## Related

- The parallel supply-chain learning from the sibling maintenance campaign
  ("RUSTSEC advisory shipped via an ungated lockfile", filed at
  `docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md`
  in that campaign's worktree — not yet in this branch's history at the time
  of writing). Same drift-family risk on the dependency axis: its Prevention
  section calls diffing the fork's lockfile against upstream master "a cheap
  early-warning signal"; `scripts/upstream-delta` is the standing
  implementation of that recommendation on the commit axis.
- `ROADMAP.md` items rm-012 / rm-013 (the port and its re-verification — numbered
  rm-009 / rm-010 campaign-locally until the 2026-09-30 integration renumbered
  them past 2326f88e's landed rm-009..rm-011, per the roadmap's collision note),
  and the cycle-1 disposition table on the conductor spool for post-port anchor
  shifts.

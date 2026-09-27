---
type: stewardship-record
id: pr3-ship-compound-learnings
run: 314df0f829fe49af8de46938c7b579a6
campaign: repository-maintenance:41adcd38b87f41cdb21a707aff98d8ba
date: 2026-09-21
base_commit: 5e7f7b4
record_kind: ce-handoff/v1
status: compounded
---

# Run 314df0f8 — ship and compounding record (fork PR #3)

Folds the run-314df0f8 cycle (final validation, commit, push, PR, CI
observation, three-pass reconciliation — the campaign that shipped the
TUI deadline fix and the cycle-6 closure records as fork PR #3) into
durable artifacts: the ROADMAP cycle-7 section now carries a base-ship
record, the roadmap tail carries the deadline prevention rule and the
PR #3 divergence context for cycle 8, and the CHANGELOG names the TUI
wait-loop change. This record is the full-context companion.

## What the run shipped

- `318df83` "fix: bound TUI test wait loops by deadline instead of
  fixed iterations" — `wait_for_pending_load`, `wait_for_progress`,
  and the governance-delivery loop in `crates/agenttrace-tui/src/tests.rs`
  bound every wait by deadline; 40/40 `agenttrace-tui` tests green
  twice on the rewritten loops.
- `df3b621` "docs: record cycle-6 closures and pass-10 findings in the
  roadmap" — the roadmap cycle-6 record.
- Branch `fix/tui-deadline-test-waits` pushed to the fork
  (codeo1io/agenttrace), fork-internal PR #3 per AGENTS.md rules 1–2
  (never upstream). PR #3 is OPEN but **superseded**: its payload
  (`318df83`+`df3b621`) reached fork master through PR #4's lineage
  (merged 2026-09-20). Closing it is cycle-8 housekeeping.

## CI gate determination (durable for this fork)

- The fork has **no branch-protection required checks**
  (`branches/master/protection` → 404), so no GitHub-enforced gate
  exists; the ci.yml **"Test and build" job is the repo-authoritative
  required gate**. It passed at `df3b621` (run 34153433111, self-hosted
  runner, 20/20 steps) and again at the branch's remote head
  `0f8c8616` (run 34769484460).
- **"Dependency review" is an environmental false red**, not code:
  "Dependency review is not supported on this repository" (dependency
  graph disabled in fork settings). Standing roadmap item since cycle
  6 (PRs #1–#3 all show it). Classify as infra, not failure; the fix
  is settings/workflow removal, already queued for cycle 8.

## Prevention rules and reusable lessons (run-specific)

1. **A work order's artifact chain can lag the live tree; verify the
   tree before executing any phase.** This run's final-validation
   evidence described a tree state (HEAD `e005952`, saturating
   arithmetic) that was weeks stale by commit time; the live tree had
   advanced through cycles 2–6. The commit phase correctly identified
   the actual pending validated changes from the working tree plus the
   newest stewardship records instead of trusting the chain. Rule:
   every phase re-derives its target from the repository, never from
   the prior phase's prose alone.
2. **Test wait loops bound by deadline, not iteration count.** A fixed
   iteration count is a load guess; it flakes on slow or contended
   runners. Every wait states its timeout and fails naming the
   deadline. (`318df83` is the reference rewrite; also recorded in the
   roadmap tail.)
3. **Reconciliation evidence conforms to the machine schema exactly.**
   The reconcile phase was re-issued twice for evidence-shape defects,
   not verdict defects: (a) `intended_heads`/`observed_heads` values
   must be bare 40-char shas, not annotated multi-ref strings; (b) key
   the block by repositories actually affected by the campaign — the
   fork we shipped to — not every remote inspected. Rule: structure
   machine-read evidence for the reader that re-issues it.
4. **Divergence is a compounding input, not noise.** Discovering at
   compound time that fork master had moved past the run's open PR (see
   below) converted a silent future conflict into a recorded, gated
   cycle-8 task. Rule: compounding re-checks remote state for every
   open ship target.

## Discovery: local tree vs fork master divergence (cycle-8 gate)

Compounding re-checked remote state for the open ship target and found
the repository had moved past PR #3 on every axis:

- **Fork master `a48c1ba`** (via PR #4, merged 2026-09-20 21:13, head
  branch `conductor/run-125bf93302aa`) already contains this run's
  `318df83`+`df3b621`, the upstream pair `a34dea2` (#283 TUI) +
  `6848aa1` (#284 parser), and a **second cycle-7 batch `9fe9610`**
  ("land cycle 7 trustworthy-capture maintenance batch", stewardship
  records dated 2026-09-14). PRs #6–#8 reworked CI (fast PR gate,
  single-job master, release-matrix cleanup); PR #9 **replaced
  ROADMAP.md with a fleet-sync version carrying no cycle records**
  (master ROADMAP greps 0 for "Completed in cycle"; the local branch
  carries 8 cycle-record sections).
- **Local branch `5e7f7b4`** holds six unpushed 2026-09-21 cycle-7
  commits (`3a7dd60..5e7f7b4`); merge-base with master is `df3b621`.
  `git cherry -v a48c1ba 5e7f7b4`: none are patch-equivalent to
  master commits — this is a parallel batch, not a duplicate push.
- **PR #3's remote branch** (`0f8c8616`, 2026-09-13) is a third line:
  `df3b621` + the upstream merge. All of its unique content is on
  master via PR #4.
- **Content census (spot-checked, to be completed by cycle 8):**
  redundant on master — `3718e77` (hand-port of `6848aa1`, which
  master merged) and `9ad11b7`'s shim classification (master's
  `flag_takes_value` already omits `--no-baseline-gate`; the 49-flag
  arity contract test may still be worth landing); still needed on
  master — `5383d5b` (parser saturating refs 12 local vs 7 master)
  and `f8f5303` (master `Cargo.toml` still names `luoyectl/agenttrace`);
  unverified — the F5 set in `3a7dd60` and the riders.

**Cycle-8 gate (before any new work):** re-census every local fix
against master `a48c1ba`, re-land the still-needed ones there, drop
the redundant ports, reconcile the roadmap fork (campaign cycle
records vs the fleet-sync version — decide which is canonical and
merge the other's content in), close PR #3 as superseded, and re-run
the full gates plus a drift census on the result. Two parallel
cycle-7 record sets now exist (`2026-09-14-cycle7-*` on master,
`2026-09-21-cycle7-*` local) — the reconciliation record must name
both.

## Reusable environment facts

- Remotes since cycle 6: `origin` = `git@github.com:codeo1io/agenttrace.git`
  (the fork, fetch+push), `upstream` = luoyectl/agenttrace with pushurl
  `DISABLE_PUSH_UPSTREAM` — the fork-only push policy is now enforced
  by remote configuration, not just convention (environment-only
  config, never committed).
- Upstream master moved past the fork point: `e005952` → `a34dea2`
  → `6848aa1` (via #283/#284); upstream HEAD at this compounding is
  `6848aa1`, and all three are reachable from fork master. Fork
  master is `a48c1ba` (this run's pair + second cycle-7 batch + CI
  rework + fleet-sync roadmap).
- The 2026-09-08 reconciliation companion record
  (`2026-09-08-pr3-reconciliation.md`) was written untracked in the
  delegate workspace per the cycles-4/5 convention and is not in this
  tree; the durable copy of that evidence is the conductor phase
  record (attempt ids e25db303, 16ad22bf, cfaaaf55).

## Next-cycle candidates (concrete)

1. **Local-vs-master reconciliation** (gating, above): census the six
   local cycle-7 commits against `a48c1ba`, re-land what is still
   needed, close superseded PR #3, reconcile the roadmap fork, gates +
   drift census.
2. Standing cycle-8 shortlist from the cycle-7 compounding: Windows
   HOME/USERPROFILE resolver (headliner), Hermes `state.db`
   tool_calls_ok schema research, fork dependency-review fix at the
   PR/CI stage, research spikes for candidates 53 (ACP stores) and 54
   (VS Code agent debug logs), then candidate 51 and the
   parse-size-cap/installer-checksum items.
3. Coordination prevention: two campaigns produced parallel cycle-7
   batches (2026-09-14 master-merged vs 2026-09-21 local) because no
   shared in-flight board existed. Consider a lightweight
   open-ship-target / in-flight-campaign board in the repo or the
   fleet-sync roadmap so the next campaign sees live parallel work
   before implementing (hygiene lane candidate).

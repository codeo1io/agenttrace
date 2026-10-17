# Cycle 3 compound record — run 4c3ca863 (repository-maintenance 059e84c3, cycle 3)

Date: 2026-10-08 · compound attempt 867a92b1 (after implement 3906a067, targeted_tests e71af849 +
281bc152, full_tests f90cb43d)
Base: e9e8fd9 (HEAD, campaign worktree run-4c3ca8637455-4c3ca863; porcelain at compound = the
21-file implement delta — ROADMAP.md clean; this record and the ROADMAP flips ship as the compound
patch, applied by the commit gate on top of the roadmap patch)
Scope: pre-review compounding only — no test execution at compound; all outcomes below are the
recorded envelopes and proof chains of the implement/targeted/full phases, consumed as evidence.

## Batch: "honest attribution"

| row | subject | outcome |
| --- | --- | --- |
| rm-585 (LEAD, customer-experience 60.0) | spend-by-branch from the unused `gitBranch` wire field — the compound-c1 designated lead | implemented pre-review (claude-lane first cut, gap disclosed) |
| rm-730 (RIDER, customer-experience 62.0, minted this cycle) | codex structural-skip counters ride the loss channel and floor a 97/97 corpus's confidence | implemented pre-review (adjudicated BY CODE, not the no-code arm) |

Uncommitted delta total +596/−12 across 21 files (ROADMAP.md NOT among them — the roadmap-phase
mint rides ahead via b27ec6ae's patch; compound adds the banner + two candidate→implemented flips
on top). No unselected batch rows this cycle (the batch was exactly these two).

## Recorded validation outcomes (NOT re-run at compound)

- implement 3906a067 — workspace `cargo test --all-features`: 561 passed / 0 failed across 27
  suites (assess baseline 551 → exactly the 10 new pins); `cargo build --release --locked` rc0;
  fmt / clippy `-D warnings` / docs-gate rc0. Live probes: full `~/.codex` corpus 97/97 parsed
  with line_skips gone from data_health; 2-priced-model subset reads confidence "high" with 311
  structural counters disclosed; real `~/.claude/projects` journals attribute
  conductor/run-6956132c5089 by name beside a detached-HEAD `unknown`.
- targeted_tests e71af849 + 281bc152 (the turn was re-dispatched after a transport reap; both
  turns byte-identical trees, `git diff` sha 1ab7b1c1…): core 396/0, cli 118/0, tui 47/0; fmt /
  clippy / docs-gate rc0; digest `validation:v1:1db2211f…` re-derived == dispatch at both turns.
- full_tests f90cb43d — the ci.yml push-event lane set verbatim (lint / full / deny) plus a bonus
  MSRV 1.88 lane: 26 result lines 561/0, release build rc0, entrypoints 34/0, output-contract /
  deterministic / report-semantics lanes rc0, tool availability verified live.

## Digest lineage

Dispatch token `validation:v1:1db2211f…` declared VERBATIM at both targeted turns; the tree was
byte-identical implement→targeted→full→compound (`git diff` sha256 `1ab7b1c1…` ==
sha256(3906a067-scratch/batch.patch) at every gate). The whole batch sits outside the engine
digest's re-derivation set for this repo (crates/** derives []/none per the known policy gap;
ROADMAP/record drift cannot move it).

## Dead-attempt forensics

- compound attempt 9f94f4fb42534a0fbb0a66779d529b6f: event log shows delegate_turn_started →
  progress pings only (message_count ≤6, ~11 min) → session_reaped (provider infra) →
  completed=failed; typed result artifact absent; NO scratch directory existed. Nothing adopted;
  the phase was redone from scratch and declared so (PR-4).
- implement-attempt history (recorded at implement, carried here): attempt a9dfa42a was
  session_reaped with no typed result; its salvage of the even-earlier 4a05b294 diffs kept the
  tree alive but no PhaseResult was adoptable — REDONE, per the same rule.

## Id accounting

ZERO ids minted at compound. Wall after the compound patch: 258 def rows, zero duplicate ids,
managed footer last; two rows flipped candidate→implemented (done-flips reserved to the commit
gate, rm-012 convention). Fresh live claim sweep at compound: **rm-730 uncontested** — zero
def-row claims in every live worktree wall, every dirty ROADMAP diff, and the delegate spool;
the landed integration wall (conflict case 3bc94d53) pre-registers "rm-730 (run 4c3ca863)" as
this run's in-flight claim, and sibling fc996e6d minted rm-731..733 off exactly that census.
Fleet pointer for the next mint: landed wall max rm-753; sibling in-flight rm-731..733
(fc996e6d/114a3456), rm-734..736 (a7110bf0/30019aec), rm-748..750 (dcc04243), rm-751..752
(82e59be2); next free rm-754 AFTER a fresh live census (PR-6).

## Prevention rules (reusable)

- **PR-1 — classify the counter channel at introduction, not after a false "low".** A parser
  counter emitted deterministically for a KNOWN non-loss shape is an assumption and belongs in
  `disclosure_counters` (rm-538's channel); `line_skips` is exclusively for genuinely dropped or
  unresolvable content, and confidence may degrade only on true loss. The codex lane shipped
  `codex_ignorable_line`/`codex_world_state` on the loss channel and floored a 97/97 corpus's
  confidence until a live corpus exposed it (rm-730). Any future lane adding an "ignorable"
  fast-path skip states its channel in the same diff and pins the separation
  (pin family: `tests/disclosure_channel.rs`).
- **PR-2 — wire-field adoption checklist (the gitBranch lesson).** Adopting an unused wire field
  into a served report: capture at the meta event (not per-message); fold first-non-empty at
  session construction; one explicit `unknown` bucket covering every lane that cannot carry the
  field (never special-case a literal like `HEAD`); render on ALL surfaces in the same pass
  (json/text/markdown/HTML/CSV parity); bump SESSION_CACHE_SCHEMA_VERSION in the same unit (rm-230
  convention — served-report content changed for unchanged files; serde-default keeps old caches
  decodable, the bump is the regeneration trigger, fixtures re-stamped in the same unit); and
  disclose the lane gap in the governance guide in the same batch. (rm-585 followed every clause.)
- **PR-3 — a mint that contradicts a recorded decision carries the contradiction to the
  implementer.** Adjudication rows (rm-730's arm-5) exist so a candidate can die honestly when the
  recorded decision was right. Here the opposite held: 39acfe43's own wording already classifies
  deterministic structural counters as assumptions, so the code arm won and the no-code flip was
  not taken. Either way, the implementer re-derives the original decision's intent and records
  which arm fired and why — never silently picks a side.
- **PR-4 — forensics before redo, and declare it.** On a dead prior attempt: read the typed
  artifact AND the event-log tail AND the scratch directory, verify lineage, then adopt or redo —
  and declare which. This cycle hit it twice more (targeted_tests transport-reaped and re-run;
  compound 9f94f4fb reaped with zero durable trail → redo). A reaped attempt that left no scratch
  leaves nothing adoptable; redo from the phase contract, not from the corpse.
- **PR-5 — package names beat directory names.** `cargo test -p agenttrace-cli --all-features`
  fails ("cannot specify features for packages outside of workspace") because
  crates/agenttrace-cli/Cargo.toml names the package `agenttrace`. The CLI crate is selected as
  `-p agenttrace`. Re-hit at this cycle's targeted_tests; keep suite commands on real package
  names.
- **PR-6 — live census at EVERY phase that writes the wall, compound included.** The fleet id
  space moved again mid-cycle: the landed integration wall reached rm-753 and had already
  pre-registered this run's rm-730 claim in its own census. A compound banner citing only the
  mint-time census would have mis-stated the fleet pointer. Every mint, flip, or designation
  re-runs the live sweep (landed wall + sibling worktrees + dirty diffs + spool), and rows
  reconcile BY TITLE at integration.

## Integration seams (found live, for the commit gate / integration)

1. **SESSION_CACHE 32→33 is contended in-flight.** Sibling run-32f3b7a1 carries an uncommitted
   `SESSION_CACHE_SCHEMA_VERSION: 32 → 33` of its own (implausible-magnitude contract; zero
   gitBranch/by_branch overlap — verified by diff grep). Whichever batch lands SECOND re-bases
   its constant to 34 and re-stamps its warm-cache fixtures in the same unit.
2. **rm-730 numeral is pre-registered**, not merely uncontested: conflict case 3bc94d53's census
   names this run as the claimant. Rebind by title only if a live contest appears at landing.
3. **Canonical checkout hazard stands** (stewardship 57ff2db7): /work/projects/agenttrace sits at
   HEAD ea5c41e = 50 commits BEHIND e9e8fd9 with 51 staged entries (+8120/−3231, incl.
   insights.rs). Integration reconciles by title; never fast-forward over it, never treat
   canonical as ahead.
4. **Fleet sibling claims to reconcile by title at integration**: rm-731..736, rm-748..752
   (see Id accounting); title-twins in other families (rm-756/757) are outside this wall.

## Commit-gate checklist (for the landing, not compound)

1. Apply the chain IN ORDER: `b27ec6aea587424fb39081ba071b84e0-scratch/ROADMAP.patch` (roadmap
   phase, +14/−0 append-only) → `867a92b1aaee41ab89987cfcaa4a06be-scratch/compound-867a92b1.patch`
   (this phase: banner + two flips + this record). Round-trip verified at compound:
   HEAD → chain → byte-identical `ROADMAP.final.md`, re-apply of the compound patch against the
   restored tree correctly fails (context mismatch).
2. Stage ALL census entries EXPLICITLY — `commit -a` drops the untracked record file (and would
   drop nothing else here; the 21 implement files are tracked-modified).
3. Flip rm-585 + rm-730 implemented→done BY TITLE with dated flush lines (rm-012 convention).
4. CHANGELOG Unreleased already carries the batch bullets in-tree (Added rm-585 / Fixed rm-730) —
   no changelog work owed at the gate.
5. SESSION_CACHE_SCHEMA_VERSION moved 32→33 and the docs-guide sentence rode the batch; the
   docs-gate was recorded rc0 at full_tests — re-run gates at the landing tree, do not cite this
   record as a gate. If target/ was swept, rebuild `target/release/agenttrace` before the
   docs-gate (~3.5 min).

## Next-cycle context

- **unknown_models confidence channel** — the full live ~/.codex corpus still reads confidence
  "low" via unknown_models=4 (genuinely unresolvable models: a true data-quality signal, not a
  counter artifact). Watch, not a mint: as the pricing catalog grows on the rm-176 refresh lane
  the count should fall; re-derive before claiming a defect.
- **by_branch extension** — grows beyond the claude lane only when a sibling format actually
  carries the field (codex/qwen/copilot journals carry cwd only today; the unknown bucket is the
  honest rendering until then).
- **Research N1-N7 residuals** all landed as dated appends on existing rows (rm-017 crates.io
  occupancy, rm-164 codeburn #1645 tier arm, rm-176 LiteLLM bursty rotation, rm-493 OTel semconv
  snapshot staleness behind the 2026-10-06 metric rename, rm-551 ccusage twin pair) — no orphan
  candidates were left unclaimed; the semconv row (rm-493) is the one most likely to demand a
  code touch next cycle.
- **Selection-time duties for any lead**: fresh fleet ceiling census (next free rm-754 after
  census) and a same-subject twin sweep — this cycle's twin sweep found zero gitBranch/
  by_branch diffs fleet-wide (re-verified at compound).

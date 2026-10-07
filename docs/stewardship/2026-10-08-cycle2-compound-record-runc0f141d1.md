# Cycle-2 compound record — run c0f141d1 (repository-maintenance e687a0cd cycle 2)

- **Phase:** compound (attempt 4b769647343f4994a95e542bba099f84, after full_tests 400e67db; pre-review)
- **Base:** a6a1f26f643311d3efb39c08169f307ef4ea989 == origin/master; worktree
  `agenttrace-80c75f65b7/run-c0f141d175e9-c0f141d1`
- **Batch:** "TUI renderer truthfulness — one renderer, honest tests, honest
  keystroke cost" = rm-014 (LEAD, reliability 82.0) + rm-015 (rider, 76.0)
- **Contract:** ZERO test execution at compound — every outcome below is the
  recorded pre-review result of an earlier phase of THIS run, consumed as
  evidence, never re-run (no cargo/pytest/gate command invoked this turn).
- **Prior compound attempt:** caa859e5 died 23s in (2 progress pings, then
  `session_reaped:failed`; typed artifact absent; porcelain identical to the
  full_tests handoff) — zero durable work; this attempt redid the phase from
  scratch and adopts nothing.

## Cycle inputs consumed (not redone)

| Phase | Attempt | Artifact |
|---|---|---|
| research | b1865e32 (after 2 dead) | `delegate/b1865e3206…-scratch/research-dossier-b1865e3206.md` + 30 payloads |
| roadmap | a5732ff5 | `delegate/a5732ff5…-scratch/ROADMAP.patch` (sha256 44f8aa36…, +35/−0) |
| prioritize | 9793e7e5 | `delegate/9793e7e5…-scratch/selected-batch.md` |
| stewardship | 911ab6f2 | envelope `delegate/911ab6f27d54422f82b2f5e3d8e0ee0f.json`; contract text at `/tmp/at-stewardship-911ab6f/stewardship-contract-c0f141d1.md` (verified present at review-fix; NO `-scratch/` directory exists for this attempt — re-derive the contract from the envelope if the /tmp companion is swept) |
| implement | 3cef57f2 | 4-file uncommitted tui delta (+510/−495) + `delegate/3cef57f2…-scratch/implement-record.md` |
| targeted | 10745217 (after typed-complete 85eb990f) | `delegate/10745217…-scratch/targeted-battery-10745217.log` |
| full | 400e67db (adopting reaped f4df3f3f's runner) | `delegate/400e67db…-scratch/adoption-verification.md` + 23 lane logs |

## Wall updates made at this compound (all uncommitted, for the commit gate)

1. **Roadmap patch APPLIED in-tree** (supersedes the roadmap banner's
   "commit gate: git apply the patch FIRST" and the same note in
   selected-batch.md / implement-record.md). Verification legs: sha256 of
   `ROADMAP.patch` == 44f8aa36b7af84405a9a8eea0461b7e86b1295b3d49700a4032cab4fdd6238f8
   (matches the roadmap phase's pin), `git apply --check` rc0 immediately
   before `git apply`, post-apply def-row census 253→256, mints at lines
   2547/2554/2561 all `status: candidate`. Reason: compound is required to
   update the wall in-tree, and its banner must insert at the top of the
   `## Open items` block — exactly where the patch's first hunk writes its
   context. Applying the patch first keeps one coherent layer; a later
   apply would fail its context and break newest-first ordering. **The
   commit gate stages ROADMAP.md; it must NOT re-apply the patch.**
2. **compound c2 banner** prepended above the cycle-2 roadmap banner
   (newest-first, run-cluster order preserved).
3. **rm-014 and rm-015 flipped candidate→implemented** with dated
   EXECUTED addenda on their rows. Done-flips are NOT taken here: rm-012
   reserves them past the shipping gate and the 7eae74eae ruling assigns
   them to integration.
4. **Mints untouched:** rm-797/rm-798/rm-799 stay candidate (patch
   verbatim); the roadmap phase's evidence-refresh appends untouched.
   ZERO ids minted at compound.

Wall accounting after this compound: 256 def rows = 131 candidate /
68 implemented / 57 done (was 130/66/57 of 253 at base + 3 candidate mints;
the two flips move 2 rows candidate→implemented).

## Recorded outcomes consumed (pre-review)

- **implement 3cef57f2** (itself adopting reaped 8778c9d1's drift after a
  line-by-line review against the stewardship contract):
  `cargo test -p agenttrace-tui` 54/54 rc0 (base 48; +2 guard tests,
  shared.rs unit tests rewritten per adopted semantics); E0659
  `inspect_target_view` ambiguous-name collision reproduced then fixed;
  clippy `--all-targets -D warnings` rc0; `cargo fmt --all -- --check` rc0;
  `cargo check -p agenttrace` rc0.
- **targeted 10745217** (fresh re-run; nothing adopted from 85eb990f):
  same battery all rc0; tree immobility held (4 M tui files, mtimes
  04:55–04:57Z); digest re-derived
  `validation:v1:026515da726c99fef393bf66371c25fa64ba23e84fc2caf29f4c00f6de903ba0`
  == dispatch-time digest VERBATIM.
- **full 400e67db** (survivor-runner adoption of reaped f4df3f3f, 23/23
  ci.yml push-surface lanes rc0): trio 608/0 (= clean-base 602 + tui 48→54),
  entrypoints 43/0, release build rc0, deny 4/4 checks, plugin v0.9.0 gate,
  MSRV 1.88 check rc0 ('Finished dev profile in 41.85s'), logs embed this
  worktree path; digest re-derived == dispatch VERBATIM.
- **Digest immobility of this compound's own edits:** ROADMAP.md and
  docs/** are outside the engine classifier's EXECUTABLE_PREFIXES
  (validation_policy.py:35-42 — crates/* src files and root/docs markdown
  fall through to non-executable), so the recorded digest stands unchanged;
  per the no-test contract it is cited, not re-derived, here.

## Dead-attempt ledger (provider-family, six this run)

| Attempt | Phase | Durable trail | Disposition |
|---|---|---|---|
| 84667b59, acd61f50 | research | pings+reap only, no artifact/scratch/drift | redone (by b1865e32) |
| 8778c9d1 | implement | no typed artifact, BUT uncommitted drift on exactly the batch's 4 files (mtimes in window) | drift-present ADOPT — successor read the full diff (433+624+67 lines) against the stewardship contract (envelope `delegate/911ab6f27d54422f82b2f5e3d8e0ee0f.json`; text `/tmp/at-stewardship-911ab6f/stewardship-contract-c0f141d1.md`), then re-ran every gate; still found the E0659 twin the dead attempt missed |
| 85eb990f | targeted | typed-complete PhaseResult JSON | phase re-dispatched anyway → 10745217 re-ran every leg fresh (cheapest correct disposition for a minutes-long battery) |
| f4df3f3f | full_tests | no typed artifact, BUT its backgrounded runner finished 23/23 lanes rc0 67s BEFORE the reap | survivor-runner ADOPT with genuineness legs (cd85a837/9a4d37af pattern) |
| caa859e5 | compound | 2 pings then reap at 23s; porcelain == prior handoff | redone (this attempt), nothing adopted |

Prevention addendum for all six filed in
`docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md`
(dated 2026-10-08, appended before "## Related").

## Review-fix addendum (independent_review:fix attempt 8aae8990, 2026-10-08)

Review 7c2333e5 returned NEEDS_CHANGES with CODE APPROVED on every dimension
(rm-014 unification verified fn-body-by-fn-body; rm-015 timing corroborated;
structural guards verified; wall/text greps verified). All actionable findings
fixed this turn, documentation-only:

- **B1 (blocker) — rm-015's stale supersession note:** the 2026-09-30
  integration note's "rm-023 … landed candidate-side" phrasing could
  legitimize a future merge that resurrects the write-only recompute rider B
  removed. Fixed in the wall: the note now carries the RESOLVED-at-base
  clause (rm-023 has no def row on this wall — the 3099db98 lane ended
  merged-behind and its band was not carried onto the new-format wall; the
  recompute removal is independent of any explorer-index change), and the
  cycle-2 EXECUTED bullet carries the cross-lane commit guidance to link
  future explorer-index work to this row when minted.
- **B2 (housekeeping) — stewardship citation:** this record's cycle-inputs
  table and ledger cited a `delegate/911ab6f2…-scratch/` path that does not
  exist. Both now cite the real artifacts: envelope
  `delegate/911ab6f27d54422f82b2f5e3d8e0ee0f.json` and the /tmp contract
  companion (verified present at review-fix time), with the sweep caveat.
- **F1/F2/F3 — closed by the review itself as non-issues** (probe
  `fn derive`/`od.` scans empty; `inspect_first_items_for_app` deep clone
  byte-identical at base = baseline, not regression; 22-of-22
  `assert_same_binding` pin census exact). No action needed.
- **R4** (post-review digest/resume governance) is logged by the review as
  next-cycle work, not this turn.

Because the delta is documentation-only, the engine validation block's
`required_scope: none` stands: no validation command was run, and the
recorded digest
`validation:v1:026515da726c99fef393bf66371c25fa64ba23e84fc2caf29f4c00f6de903ba0`
is carried forward unchanged (ROADMAP.md and docs/** are non-executable
surfaces under the classifier).

## Reusable lessons (durable, from this cycle's evidence)

1. **A cfg(test) shadow copy of a renderer is a correctness hole even when
   green.** 12 of 21 duplicated helpers had drifted semantics; the suite
   asserted the test copy's behavior while production shipped another. The
   durable net is structural, not diligence: bind tests to the production
   module (unconditional glob) + a source-scan guard
   (`presentation_defines_no_duplicate_helpers`) because E0659 only fires
   on *used* names — `inspect_target_view` was an identical twin invisible
   until it collided.
2. **Unifying a shadow copy costs assertion migrations, not just code
   motion.** Four tests pinned pre-unification chrome ("Loading Status",
   "Discovering"); each had to move to the ADOPTED semantics explicitly —
   semantics are decided per helper, never blanket-copied.
3. **Write-only recomputes on hot paths are measurable cheaply:** a
   temporary probe test (add → measure → remove) showed the dead
   OverviewDerived recompute was a constant ~31% of `refresh_filtered`
   across n=200..2000 — evidence without leaving a benchmark behind.
4. **Patch-then-banner ordering matters for wall hygiene.** A roadmap
   phase that ships its delta as a patch writes its banner at the exact
   context line a later compound banner needs. Apply the verified patch
   first, then edit on top; never edit the anchor region and leave a patch
   pending against it.
5. **Anchor staleness is the default for rows older than the base.** Both
   batch rows carried pre-base anchors; stewardship's first-hand
   re-verification corrected the binding site (app.rs:1800-1814, not
   1786-1799) and the drift breadth (12-of-21, not "threefold").

## Commit-gate seams (next phases — do NOT re-do compound work)

- Stage: ROADMAP.md, the 4 tui files, this record,
  `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md`.
- Do **not** re-apply `ROADMAP.patch` (already in-tree; re-apply fails).
- CHANGELOG: 2 Fixed bullets under Unreleased + the docs/governance
  sentence were explicitly deferred to the commit by implement 3cef57f2 —
  they were NOT minted at compound.
- Done-flips for rm-014/rm-015: integration's, not the commit's (7eae74eae).
- Re-verify the tui lane is still uncontended (sweep fleet worktrees) and
  that the 4 tui files are byte-identical to the validated delta (mtimes
  04:55:18×3/04:57:34Z from implement; digest
  validation:v1:026515da… is the pinned reference).
- Numeral collisions at integration: this wall's rm-797..799 vs 6cb2756a's
  rm-797..801 — reconcile BY TITLE (880a7b9e); live worktree ceilings at
  compound: rm-839 (fabd9fb8) / rm-833 (aa41d9b5) / rm-826 (66fc09b8) /
  rm-824 (6cb2756a, a2abf1a8); next free ≥ rm-840.

## Cycle-3 context left for the next maintenance cycle

- rm-016 (Unicode-aware case folding, 68.0, search.rs/filters.rs) — the
  highest-priority clean lane visible at this base.
- rm-798 waits on the 9 dirty parser.rs lanes + unlanded rm-685/rm-686.
- Pricing-refresh family (rm-176/164/175/530) waits for the rm-803
  (5daf2442) and rm-817 (e602bb69) lanes to clear.
- MCP-family subjects of rm-797/rm-799 are CEDED to run 749cd298's
  rm-782/rm-781/rm-780 selection at the same base; evidence gift:
  /tmp/assess-c0f141/bigline.json + rss.py (rc=0, peakRSS 0.94GiB,
  wall 7.5s vs a 500,000,017-byte input line).
- rm-195 (fork-identity) stays beyond a code cycle's authority.
- Watches re-arm per the cycle-2 roadmap banner (upstream frozen at
  15ed07f2/#318; LiteLLM 4,504 live rows vs 3,099-row 2026-10-04 snapshot;
  claude-code 2.1.293; codex rust-v0.161.0; MCP spec 2026-07-28).

## How to verify this record

- `git -C <this worktree> diff --stat` → ROADMAP.md (+ compound lines) and
  the 4 tui files only among tracked mods, plus this doc and the
  workflow-issues addendum.
- `grep -n "compound c2 (2026-10-08, run c0f141d1" ROADMAP.md` → banner at
  the top of the `## Open items` block, above the cycle-2 roadmap banner.
- `grep -E '^- id: .rm-014. |^- id: .rm-015. ' ROADMAP.md` → both
  `status: implemented`.
- `grep -cE '^- id: .rm-' ROADMAP.md` → 256; status census 131/68/57.
- `grep -nE '^- id: .rm-79[7-9].' ROADMAP.md` → mints still `candidate`.
- `git diff --check` clean; `sha256sum` of the roadmap phase's patch still
  44f8aa36… (the spool copy — the in-tree application is this record's
  subject).

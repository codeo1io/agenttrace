# Cycle-3 compound record — run 75ae7fb6 (repository-maintenance 6b64d192)

Compound attempt 2ba7d249, 2026-10-03, tree fd5532f (fork/master), worktree
`run-75ae7fb65c52-75ae7fb6`. Pre-review cycle evidence only — assessment
(4d763916), research (ba040416 pass 13), roadmap (cfb79159), prioritization
(a93def27), stewardship (26b3e6ec), implement (f69fa3e1), targeted_tests
(2f8f9786), full_tests (16d7263e). **No validation was re-run in this phase** —
the targeted (all named surfaces green, digest declared verbatim) and full-suite
(20/20 steps rc0, 297 passed / 0 failed) outcomes are consumed as recorded
evidence, per the compound contract.

## Cycle outcome in one paragraph

Batch "truthful calendar, verified Codex ledger, honest wall" landed
uncommitted: rm-040 implemented (`--range today` anchors to the user's local
midnight via `day_start_for_offset`, 2 new tests, live red→green across
TZ=Asia/Tokyo / UTC / America/New_York, README disclosure); rm-304's
verify-first gate discharged with a written **PROVEN** verdict + in-tree
characterization corpus (1200 of 3700 billable tokens = 67.6% under-count);
rm-194/rm-197 dispositioned done-by-content; rm-305 CEDED to sibling
d675a177's strict-superset rm-336. Wall: 110 ids, 72 candidate / 23 done /
15 implemented.

## Prevention rules (repo-durable, reuse verbatim)

**PR-1 — Reconcile the dispatched base to the canonical ceiling before any
phase that mints or anchors.** This run was dispatched at 5ef66c0, two merges
behind fork/master fd5532f; the assess envelope consequently reported 2 red
gates (docs schema 6-vs-7) already fixed at the ceiling, and every anchor was
line-drifted (pricing.rs :403→:449, finite guard :1373→:508-511, parser.rs
:2293→:2344, main.rs :1079→:1067, lib.rs :548→:550). The roadmap phase
fast-forwarded ff-only and re-verified anchors before minting — that is why
this run's mints carry true line numbers. Rule: `git fetch fork; git rev-list
--count <base>..fork/master` == 0, else ff the worktree first and record
old→new anchor drift in the mint note. Never mint against a stale base.

**PR-2 — Suspected upstream-parser defects get a verify-first gate with a
flip-contract characterization pin.** rm-304's acceptance mandated
PROVE-or-REFUTE with corpus BEFORE implementation spend. The verdict (PROVEN,
wire shapes grounded in upstream source — `CompactedItem`,
`TokenUsageRecord` — not invented) plus a deterministic in-tree
characterization test converted an unknown into evidence for the cost of one
fixture pair. The pin (`codex_compaction_verdict.rs` asserts the
UNDER-counted totals today) documents its own flip semantics: when the fix
lands, the assertions invert to true totals and that inversion is the fix's
regression gate. Rule: never "correct" such a pin to the true values before
the fix exists — that fakes green and destroys the gate.

**PR-3 — Calendar boundaries anchor to the user's local day via fixed-offset
arithmetic, never local-zone date math.** `day_start_for_offset` derives a
`FixedOffset` from `chrono::Local` and computes midnight in it — DST-ambiguous
by construction is avoided, the ±1h zone-transition-day caveat is documented
at the anchor, and the comparison stays a UTC instant. Reuse this shape for
any future "today / this week" boundary (weekly windows, statusline day
resets) instead of re-deriving zone semantics per call site.

## Process lessons (conductor-fleet facing)

- **Dead-attempt forensics before redo.** Prior roadmap attempt 1859aa31 died
  of a provider failure; the redo was gated on three on-disk probes (typed
  result JSON absent, event log progress-pings only, worktree porcelain 0 at
  base) proving NOTHING durable was left, and the redo is declared in the
  wall's mint comment. Redoing a phase without that proof risks double-minting.
- **Ids are allocated from a live sweep, never a recorded map.** This run
  minted rm-304..306 from a same-morning sweep; by afternoon the fleet had
  claimed through rm-373. The compound-time re-sweep (worktree diffs +
  agenttrace-content-filtered spool artifacts — foreign-repo ROADMAP copies
  claiming rm-5xx exist in the shared spool and MUST be content-excluded)
  found next free = rm-374, recorded in the wall's mint comment.
- **Sibling-superset cession is the correct resolution for identical-defect
  collisions.** rm-305 (this run's assess N3) vs d675a177's rm-336: their
  scope was a strict superset, so this run CEDED, left the wall item as the
  record of its own PoC, and the fold happens at integration BY TITLE. The
  same pattern now hands rm-304's fix arm to 2c2db6f5's rm-372-unit.
- **Base-stale reds are not findings.** Both assess reds self-resolved at
  fd5532f; corroborated independently by d675a177's cycle-4 compound ("do not
  re-file"). An assess envelope against a stale base must tag its reds with
  the base commit so later phases can distinguish stale from live.
- **Host-env folklore that is actually load-bearing on this fleet host:**
  `TMPDIR=/tmp` for every cargo test run (ambient-TMPDIR red, insights.rs:563
  class) and CI-output redirection (`AGENTTRACE_CI_OUT` → /tmp) so gates never
  drop `ci-artifacts/` into a repo root. Full-suite substitutions are
  disclosed in the full_tests record, not silently applied.

## Next-cycle context (concrete, for the cycle-4 assessment)

- **Recommended lead: rm-176** (pricing-snapshot freshness) — freshest widened
  gap (upstream HOURLY automation vs our 2026-09-13 pin, +34%/20d; LiteLLM
  3,699 vs bundled 2,755), all mechanisms operator-proven and fetchable
  (ccusage `update-pricing.yaml`), no sibling claim on the workflow surface.
  Line on the item.
- **Alternate: rm-201** (statusline/quota JSON) — demand double-sourced this
  run — but sequence BEHIND in-flight rm-302 (364aa3be).
- **Cheap rider: disposition pass** over rm-053/rm-175/rm-085/rm-014/rm-011
  (fd5532f landings advanced several; "a disposition pass, not new spend" per
  prioritize a93def27).
- **Unclaimed fleet follow-up (NOT minted here — discovered by sibling
  1cb61083, verified first-hand this phase):** `bin="${AGENTTRACE_BIN:-
  /tmp/agenttrace}"` at `scripts/ci/check-deterministic-output.sh:4`,
  `check-output-contract.sh:4`, `check-report-semantics.sh:4` — a stale
  /tmp default for the gate binary; mirror of the fixed
  `check-rust-release-local.sh` pattern. Whoever's compound runs next may
  mint it (next free rm-374); minting here would collide if 1cb61083's
  compound claims its own finding.
- **Collision zones to respect:** rm-195 (strategic, adjacent to in-flight
  rm-240), rm-196 (inside 6a10ae64's cache-contract cycle), rm-231/rm-232
  (6a10ae64 in flight), parser.rs rollout arms (2c2db6f5's rm-372-unit owns
  the compaction fix), pricing.rs (d675a177's rm-336).

## Carry-forward for the review/commit gates (do not double-implement)

- **Flip-contract seam:** when 2c2db6f5's Codex fix lands, fold rm-304's fix
  arm into rm-372 BY TITLE and invert `codex_compaction_verdict.rs` to true
  totals. Reviewers must not flip the pin early (PR-2).
- **Cession seams:** rm-305 folds into rm-336 at integration; no second
  implementation of pricing-override validation from this campaign.
- **Merge note:** this run's insights.rs EOF tests-append abuts 6a10ae64's
  :564,6 hunk — both additive, trivial context resolution (recorded in the
  implementation record).
- **Ids rebind at integration** per renumber-at-integration discipline; the
  in-tree ROADMAP.md diff IS the deliverable — byte-copied to
  `cfb79159-scratch/roadmap.patch` (+ `ROADMAP.after.md`, re-synced this
  phase) and `/tmp/at-roadmap-cfb79159/roadmap.patch` for the commit gate.
- Review outcomes are explicitly out of this phase's scope; the next cycle's
  assessment carries them forward.

## Artifacts (this phase)

- `ROADMAP.md` — compound annotations only: rm-304 fix-arm handoff, rm-176
  cycle-4 lead recommendation, mint-comment addendum (re-sweep result
  rm-374, no new mints). Status accounting unchanged: 110 ids,
  72 candidate / 23 done / 15 implemented.
- This record. Patch byte-copies re-synced (spool + /tmp).

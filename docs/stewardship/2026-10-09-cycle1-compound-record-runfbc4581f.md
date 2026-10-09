# Cycle-1 compound record — run fbc4581f (repository-maintenance 5f8cbeaa, 2026-10-09)

Pre-review compounding of the cycle-1 batch **"pricing snapshot refresh with
never-drop union"** (rm-588 LEAD + rm-006 rider), then the review-fix pass.
Chain of custody: prioritize cbeab4f3 → stewardship 3988907c → implement
44a68ca0 (retry after an attestation-only fold rejection) → targeted_tests
d3ea7553 → full_tests 494a30a0 → compound ca939f88 → independent_review
38de1c98 (NEEDS_CHANGES) → review fix 3c9699a5 → re-review ad3628e6
(NEEDS_CHANGES: the fix's rebuilt artifacts never shipped — patch/final
still v1 on disk, and the v2 builder crashed on 4 wrong row anchors before
writing) → chain re-land ab102a90 (this revision of the record).

## Outcome (pre-review)

- rm-588 implemented: `scripts/pricing/update-snapshot.sh` union-merges the
  committed bundle; a key removed upstream stays priced unless a dated
  manual-drop note exists in the new `scripts/pricing/snapshot-drops.txt`
  (fail-closed on malformed notes, future-dated notes inert); a shrink-check
  REFUSES any refresh that would drop priced models vs the committed prior.
- rm-006 executed: bundle refreshed 3,099 model keys @2026-10-04 → 3,130
  @2026-10-08 through the new path (net +31); claude-haiku-5-5 priced (closes
  the research-proven 30.0× disclosed-fallback overestimate on the new
  default Haiku); 2 keys union-retained and disclosed in
  `_snapshot.retained_from_previous`.
- Guards: `bundled_snapshot_prices_claude_haiku_5_5_exactly`,
  `bundled_snapshot_union_header_stays_honest` (pricing.rs).
- Validation lineage (recorded evidence): targeted 5 offline contract arms vs
  the md5-pinned live capture + shellcheck/bash -n rc0 + pricing:: 28/0 +
  attribution_dimensions 9/0; full ci.yml push lanes 21/21 green, machine-sum
  656 passed / 0 failed = assess baseline 654 + 2 new guards; validation
  digest stable across implement/targeted/full (re-derived post-fix at
  3c9699a5 — the script surface changed there; see that envelope).

## Review fix (attempt 3c9699a5, 2026-10-09 — all 5 findings closed)

- **F1 (high) rm-588 clause 3 unmet.** The refresh delta had landed only on
  the rm-006 rider and the EXECUTED bullet redefined clause (3) as the
  script's census print. Fixed: dated refresh-delta append on the **rm-176**
  row (carried by ROADMAP.compound.patch — authored at 3c9699a5, actually
  shipped at the ab102a90 re-land after re-review ad3628e6 caught the fix's
  artifacts unshipped), the script's census print ends with the standing
  rm-176 reminder, and the rm-588 EXECUTED bullet now cites the append.
- **F2 (medium) numeral drift.** The chain said 3,100 bundled / -3 removals /
  47 field-level mutations / +30 net. Ground truth (script census): 3,099
  model keys (the old count included the `_snapshot` header), 2 removals,
  39 rate-mutated keys = 78 field-level mutations, net +31. All chain texts
  corrected in place (roadmap banner, rm-006 rider, rm-588 rider, compound
  banner, this record). The azure 16.7× DROP and grok-4.6 +60% examples were
  verified true and stand.
- **F3 (low) vacuous-refresh floor.** A `{}`/partial live source previously
  refreshed rc=0 with a fresh date over 100% stale content. Fixed: the
  script REFUSES a 0-model source and any source overlapping <50% of the
  prior catalog.
- **F4 (low) non-atomic bundle write.** Fixed: sibling `.new` temp +
  `os.replace` (rm-693 discipline); the bash trap cleans the temp.
- **F5 (low) any-argv full-refresh trap.** `--help` used to perform a full
  refresh + rewrite. Fixed: no-args runs the refresh; `--help`/`-h` prints
  usage rc0; any other argument is REFUSED rc2; neither writes.
- Post-fix targeted arms (recorded at 3c9699a5): idempotence byte-identical,
  honored-drop 3,129 with honest header, vacuous and partial sources
  refused with bundle sha unchanged, `--help`/`--force` no-write, shellcheck
  + bash -n rc0, pricing:: 28/0.
- **Re-review ad3628e6 → re-land ab102a90.** The re-review's own 8 contract
  arms confirmed every hardening live in the worktree (code side green),
  but found the fix's compound artifacts unshipped (v1 shas still on disk,
  pre-review mtimes) and its rebuild irreproducible (the v2 builder's 4 row
  anchors were wrong — it crashed before writing; claimed v2 shas matched
  nothing on disk) — plus the v2 rm-176 append misnamed the haiku add as
  `anthropic/claude-haiku-5-5` (no slash form exists in the +31 adds).
  Resolved at the ab102a90 re-land: anchors matched to the real postimage
  lines, the builder run in-spool so the outputs landed, the chain verified
  hop-by-hop, and the append now names the real forms (bare
  `claude-haiku-5-5` + dot-form `anthropic.claude-haiku-5-5`).

## Prevention rules (reusable)

- **PR-1 — fold-gate attestations are typed contracts, not prose.** The
  implement fold was rejected with the batch fully intact: the only defect was
  a missing `validation_evidence.changed_surfaces` attestation (engine
  counted 1 changed executable surface). Rule: replicate the deployed
  engine's `changed_surfaces(base_full_sha, worktree)` before folding and
  declare the FULL derived set — never hand-derive, never declare only the
  executables.
- **PR-2 — empty `full_command` means the ci lanes, verbatim.** When the
  dispatch validation block leaves `full_command` empty, the authoritative
  full suite is the ci.yml push-gated lanes run verbatim (21 lanes: full job
  + lint's install-ref-drift + deny job); MSRV floor and TUI real-data smoke
  are excluded by the workflow's own event/variable gating, and that
  exclusion must be stated with the reason.
- **PR-3 — keep CI lanes out of the worktree.** Redirect
  `AGENTTRACE_CI_OUT`, `AGENTTRACE_REAL_CLI_OUT` and `CARGO_TARGET_DIR` to
  /tmp; capture each lane's rc immediately after it returns. The repo then
  stays porcelain-stable across a full-suite phase.
- **PR-4 — a single red test in an untouched surface is a flake until proven
  otherwise — but prove it.** Pattern that settled it this cycle: rerun the
  test isolated (green, 0.06s), rerun the lane verbatim (green, 656/0),
  reconcile the machine-sum against the recorded baseline (654 + 2 new =
  656 exactly). Record the flake for an audit; do NOT hot-fix a file that is
  contended by in-flight fleet siblings.
- **PR-5 — never-drop is anchored to the committed prior.** The union floor
  guarantees "no drop vs the prior bundle"; it cannot resurrect a key the
  prior already lost (nothing re-adds a key live no longer prices). State
  this in the contract doc so a hand-damaged-prior scenario is not misread
  as a guard failure.
- **PR-6 — offline refresh re-verification pattern.** `LITELLM_SNAPSHOT_SRC`
  + `SNAPSHOT_DATE` make the refresh hermetic: idempotence (byte-identical
  re-run), malformed-note refusal, union-disabled copy (the shrink-check must
  fire), future-dated-note inertness, honored-drop honesty — five arms
  against an md5-pinned capture, reusable for every future refresh cycle.
- **PR-7 — scripts that rewrite tracked files take no arguments.** A
  refresh/rewrite script triggered by ANY argv (even `--help`) is a footgun
  that already fired in the fleet (an accidental `--help` rewrote the
  tracked snapshot mid-phase). Contract: no-args runs; `--help`/`-h` prints
  usage rc0; every other argument is REFUSED non-zero; no argv path writes.
- **PR-8 — census numerals are the only numerals you may ship.** When an
  artifact cites counts (keys added/removed/mutated), the shipped text must
  quote the refresh script's own census output verbatim, not a hand-derived
  diff — this cycle shipped 3,100/-3/47/+30 while the script census said
  3,099/-2/39=78 fields/+31, because the hand diff counted the `_snapshot`
  header as a bundle key and as a removal. Cross-check every count against
  the tool that generated it before folding.
- **PR-9 — acceptance clauses name their landing site.** When an acceptance
  clause says "appends X to row Y", the delta must land on row Y — not on an
  adjacent row — and the implementing bullet must cite the append, not
  redefine the clause. Discharge site = clause site, verbatim.

## Flake on record (next-cycle candidate, deliberately not minted here)

`session_cache::tests::clear_cache_removes_every_artifact_and_only_those`
(session_cache.rs:3206) failed once under 235-test parallelism (full-suite
pass, lane 3 of 21), green isolated and on the verbatim rerun. Env-race
class: an unlocked sibling env mutation can race a `lock_env` holder's
artifact-removal window. Pre-existing at a8b95e7, outside the pricing-only
delta. Candidate audit: enumerate env-mutating tests in agenttrace-core that
do not hold `test_env::lock_env` while a sibling does (rm-804/rm-818
env-lock waves are the adjacent landed work). session_cache.rs is contended
by 6 fleet sibling waves — audit from a clean base, not mid-batch.

## Commit-gate seams (load-bearing)

1. Apply `roadmap-delta-54222fa7.patch` FIRST, then
   `ROADMAP.compound.patch` (delegate/ca939f88…-scratch/) — both one-shot,
   chain verified hop-by-hop; the compound patch carries this record as
   `docs/stewardship/2026-10-09-cycle1-compound-record-runfbc4581f.md`.
2. `git add scripts/pricing/snapshot-drops.txt` — new untracked file; the
   refreshed script fail-closes on a missing register at the next refresh.
3. CHANGELOG bullet owed for the 2026-10-08/3,130 refresh (every prior
   refresh landed one) — use the corrected numerals: 3,099 model keys →
   3,130, net +31, 39 rate-mutated keys (78 field-level), 2 union-retained.
4. rm-588/rm-006 done-flips happen at the commit gate, not at compound
   (rm-012 precedent); this record already flipped rm-588
   candidate→implemented.
5. EVERY future refresh appends its printed added/removed delta to the
   rm-176 row (rm-588 clause 3) — the script now prints the reminder at the
   end of its census; this chain carries the 2026-10-08 append.

## Next-cycle leads (context for the cycle-2 prioritize)

- rm-843 cache-append inspect-then-reopen race; rm-844 codex seen-totals
  cap-clear overcount; rm-845 plan-tier pricing second source (composes the
  now-implemented rm-588 — plan-tier entries model-prices lacks).
- The session_cache env-race audit above (mint at the next roadmap phase).
- rm-006's remaining arms: cadence ownership (age nudge; the refresh script's
  dated mutation census print is the future nudge's input) and the doctor
  drift surface for local-session models.
- The sibling roadmap wave minted an arg-guard row of the same class as the
  trap F5 closed here (rm-901 in the fleet frontier) — when that row is
  next read, it should be marked superseded by the landed no-argv contract
  if the fleet's row targets this same script class.

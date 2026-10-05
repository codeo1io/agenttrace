# Cycle-1 compound record — run 41263f584891 (repository-maintenance 887bf90f)

Compound attempt 6628fb27, 2026-10-04T16:03Z, tree 7e17ac1 (dispatched base,
unchanged), worktree `run-41263f584891-41263f58`. Pre-review cycle evidence
only — assessment (3e8493fa), research (802e0a09), roadmap (b9ad50de →
re-fire 1d6f487e → re-fire 2 89ee99a4), prioritization (f79ee937),
stewardship (79bce205), implement (526be176), targeted_tests (12dab869),
full_tests (11c31a42). **No validation was re-run in this phase** — the
targeted and full-suite outcomes are consumed as recorded evidence, per the
compound contract.

## Cycle outcome in one paragraph

Batch "make the advertised trust boundaries real" landed uncommitted at base
7e17ac1 as a 9-file delta (+220/−22): rm-403 implemented (markdown_cell
entity-escapes `& < >` before the pipe/newline table escapes, five raw
Scope-row string fields routed through it, 2-test contract suite, live PoC
re-run 0 raw tags from 2); rm-404 implemented (PRIVACY.md's false "only
exception is --update-pricing" claim replaced by an enumerated
network-touch list; `upstream --fetch` prints a pre-request stderr
disclosure built from the same constants the code uses; three
constructor-derived pin tests); rm-405 implemented (health-gate example
SHA-pinned to verified v7.0.0 tag objects, pinned-checksum v0.9.0 install
mirroring install.sh's platform detection, gate step documented as the
job-verdict owner, new `scripts/ci/check-example-workflows.sh` wired into
ci.yml with 3/3 mutation reds). Status accounting unchanged: 126 ids,
84 candidate / 23 done / 19 implemented — flips to done are reserved for
the commit gate (rm-012 precedent); the cycle-1-implemented rows on the
three items are the pre-review status record.

## Prevention rules (repo-durable)

**PR-1 — Probe the exact scoped identifier before declaring a distribution
channel dead.** The wall carried "agenttrace 404s on registry.npmjs.org"
folklore; research 802e0a09 showed the probe had used the unscoped name
while the real channel `@zack78/agenttrace` was LIVE (0.9.0 published
2026-09-29, five versions since 0.7.6, postinstall pulling signed release
binaries). A dead-channel claim must name the exact identifier probed and
its timestamp, and any rename (upstream moved distribution surfaces:
scoped npm + GitHub Release binaries + winget + brew) re-opens the probe.
This rule directly rewrote rm-017's evidence addendum and rm-404's
acceptance surface.

**PR-2 — Count BOTH id formats when sizing the wall (now rule 5 of
`docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md`).**
The backticked def-line grep under-counted this repository's wall by 14
old-format bare entries (107 counted vs 121 real). Band selection off the
grep-only count can mint inside invisible territory. Same normalization the
2026-10-01 re-hit taught for the duplicate probe, applied earlier — at
census time.

**PR-3 — Example workflows are executable supply-chain surface; a guard
that is not wired detects nothing.** rm-405's new
`check-example-workflows.sh` follows the repo's own F19 precedent: it was
wired into `.github/workflows/ci.yml` in the same delta that created it,
and its three failure modes were each proven red by mutation (mutable
`@v7` ref, gate-step `|| true`, missing checksum line) before being
recorded as working. SHA pins were verified against the tag OBJECT sha
(`git/ref/tags/v7.0.0` → commit sha), never the tag name; release
checksums were re-fetched and byte-verified the same day, with all four
platform assets.

**PR-4 — Record signal corrections in place, never silently rewrite them.**
The original rm-405 mint claimed "both overview runs append `|| true`, so
the example health gate can never fail the job" — FALSE as written (the
gate step never carried `|| true`; the two `|| true` steps are report-only
`if: always()` explainers whose exit codes cannot affect the job verdict).
The implement phase corrected the claim INSIDE the signals line with
attribution instead of editing it away: the audit trail shows what was
believed, why it was wrong, and what the real defects were. Any phase that
finds its own prior claim false does this.

**PR-5 — When two escaping families coexist, the MERGE owns their order.**
rm-403's entity escape and fc197c5e's unlanded control-byte sanitizer both
route through `markdown_cell`. The correct composed order is documented at
both sites: control bytes FIRST (they can forge structure), then printable
HTML entity-escaping (it must see real `&` to avoid double-escaping).
Reviewers folding the fc197c5e lane in must preserve the ORDER, not merely
the presence, of both escapes — and re-run the assess md-injection PoC
against the merged binary (the item's own instruction).

## Process lessons (conductor-fleet facing)

- **Ceiling churn is the norm; re-fire protocol handled it.** The ceiling
  moved four times during this one run (6a2ec87 → 20267e9 → c032f33 →
  c130802 → ea5c41e). The roadmap re-fire pattern — verify the prior
  attempt's delta is byte-intact in the worktree, re-run the three-home
  census (ceiling wall / committed lane / uncommitted bands), re-probe
  every minted anchor at the NEW ceiling, then EXTEND — kept the mints
  truthful without redoing work. Notably the c130802 landing WAS the
  2c2db6f5 lane landing (a8a7c34, rm-400/401), which resolved one of
  rm-406's two blockers mid-run.
- **Fold-not-mint when prior coverage exists — but byte-check the fold
  target.** Four fresh findings were folded into existing wall items
  (N2 → rm-346 + an rm-019 rider; A2 → rm-389; N3 → rm-212; N4 → rm-305
  amendment) instead of minting four new ids. The rm-305 amendment then
  needed its own correction (b9ad50de's "no deny_unknown_fields at
  ceiling" was a grep error, fixed by 1d6f487e): amendments inherit the
  same verification duty as mints.
- **Digest discipline held across both validation phases.** Both
  targeted_tests and full_tests re-derived
  `validation:v1:80b7390a259a59011c39db0e6f80e6876a35648a4` from the
  base sha via `validation_policy.validation_digest` before declaring it;
  neither declared the local-validation-gate envelope's own digest field,
  whose `digest_base` is 'unknown' by dispatch construction (the dispatch
  command passes no `--digest-base-sha`). Declaring that field would have
  been a fabrication, not evidence.
- **Sibling-lane hygiene at selection time paid off at implement time.**
  The batch was chosen for near-disjoint footprints from all 29 sibling
  worktrees; the compound re-sweep found only same-file-adjacent (not
  hunk-overlapping) contacts since: 5bec3c93 edits a DIFFERENT PRIVACY.md
  paragraph, fb22927c's reports.rs +15/−1 has zero markdown_cell contact.
  The one deep seam was known in advance and documented (e602bb69's
  upstream.rs rewrite).

## Next-cycle context (concrete, for the cycle-2 assessment)

- **Recommended lead: rm-406** (unknown-event-kind census, correctness 58) —
  prioritize f79ee937 already named it the natural next lead; one of its
  two blocking parser.rs lanes has since landed at the ceiling
  (2c2db6f5 @a8a7c34 via c130802), leaving only fc197c5e's salvage band
  (rm-392..399) unlanded — re-check before implementing. Live drift driver:
  CC 2.1.289's `agent.spawn` teammates and idle/waiting states will arrive
  as new dropped line classes. Census baseline on record: 330 real
  transcripts → 307 user / 212 assistant / 109 queue-operation lines
  silently dropped. The PoC corpus is under /tmp (not durable) — re-derive
  from the research-report appendix recipe.
- **Riders:** rm-405's runner-execution arm (fixture corpus at health 70
  must fail the example JOB — needs a self-hosted runner, deferred at
  implement); rm-389's amended filter arms (A2's NaN/inf view-filter PoCs
  are on disk at /tmp/at-assess-3e8493fa/poc/ts — also /tmp, re-derive);
  rm-346/rm-019 gate-flag contract, whose rc2-conflation rider (clap usage
  errors share exit 2 with gate failure) is unclaimed anywhere.
- **rm-407 stays gated** on its own precondition: pin the `type:"summary"`
  line format from a real compacted corpus BEFORE implementing (0 summary
  lines in the 330-file host census; zero in repo testdata).
- **Collision zones to respect (compound re-sweep @ ea5c41e):** reports.rs
  carries four sibling deltas (52465b9e rm-034 render.rs family — the
  PR-5 semantic seam; fb22927c; 16bbd3ae; 71f666e8), upstream.rs
  (e602bb69's stale rm-081 in-process rewrite), pricing.rs (3c24960c's
  rm-231/196/419 lane, 8 files 808+/27−), PRIVACY.md (5bec3c93).
  Older-campaign leads named by cf755698's record (rm-384 config,
  rm-196/175/176 pricing family) remain valid cycle-3 material.

## Carry-forward for the review/commit gates (do not double-implement)

- **Status flips:** rm-403/404/405 flip candidate → done at the commit gate
  (rm-012 precedent); their cycle-1-implemented rows carry the full
  evidence chains. CHANGELOG entries land with the merge per lineage
  convention (fd5532f / 1e1eb66 precedent) — none were minted at compound.
- **Merge seams:** PR-5 escape order for the fc197c5e/52465b9e fold into
  rm-403; rebase rm-404's disclosure line onto e602bb69's in-process
  rewrite if it lands first; 3c24960c's pricing.rs lane and our
  one-const-pin edit are trivially composable; 5bec3c93's PRIVACY.md
  paragraph is textually disjoint.
- **Deferred arm:** rm-405's runner-execution acceptance arm is recorded on
  the item — the commit gate either executes it with runner access or
  leaves the deferral explicit.
- **Ids rebind by TITLE at integration** (campaign-local until then). Next
  free agenttrace-space id at compound time: rm-444 (fleet re-sweep:
  uncommitted bands claim through rm-440 visible — 6403d975 rm-440,
  5bec3c93 rm-435, fb22927c rm-428, 16bbd3ae rm-423, 3c24960c rm-419,
  6557b823 rm-416, 555a174d rm-410 — with rm-441..443 recorded by
  e97ae6c9's lane, whose ROADMAP delta has since left its worktree).
- **Byte-copies for the commit gate:** full post-compound delta snapshot at
  `6628fb27…-scratch/compound-roadmap.patch` + `ROADMAP.after.md` +
  `compound-record.md`, mirrored under `/tmp/at-compound-6628fb27/` (/tmp is
  swept between phases — the spool copies are authoritative).
- Review outcomes are explicitly out of this phase's scope; the next
  cycle's assessment carries them forward.

## Artifacts (this phase)

- `ROADMAP.md` — compound annotations only: the `compound c1` header note
  (outcome, recorded validation, leads, commit-gate seams), the rm-406
  cycle-2 lead-recommendation line, and the mint-comment compound re-sweep
  addendum (ceiling ea5c41e, next free rm-444). Status accounting
  unchanged: 126 ids, 84 candidate / 23 done / 19 implemented.
- `docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md`
  — rule 5 (dual-format census) + detection-history entry (PR-2).
- This record.
- Spool byte-copies: `6628fb27…-scratch/{compound-roadmap.patch,
  ROADMAP.after.md, compound-record.md}` + `/tmp/at-compound-6628fb27/`
  mirror.

## Review outcome (recorded by the fix fold, 2026-10-05)

Independent review 0145eeb5 returned NEEDS_CHANGES — 2 medium, 3 low, all
in the batch's own trust-boundary terms; everything else survived
independent re-execution (escaper, disclosure, pins, checksums, SHAs,
roadmap accounting, full 308/0). Fix fold b64b1b86 closed all five:
main.rs:215's `--update-pricing` announcement moved from stdout to stderr
(the PRIVACY "announced on stderr" claim is now true for all three network
touches) with a new bins source-pin; the example-workflow gate's
mutable-ref rule went deny-by-default (every `uses:` must end in a full
40-hex commit SHA — closes the `@v7.0.0` gap the review proved by
mutation); the compound-note privacy tally was corrected in place with
attribution (PR-4: the correct split is cli 1 + core 2, one core pin the
pre-existing rm-086 artifact test; this record's "three constructor-derived
pin tests" wording counts the same three passing tests and stands); the
governance json-fence boundary was named on rm-403; the runner-execution
arm stays review-acknowledged-deferred on the commit-gate checklist.
Re-validated at the work order's required scope (targeted).

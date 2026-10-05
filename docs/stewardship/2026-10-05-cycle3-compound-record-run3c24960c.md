# Cycle-3 compound record — run 3c24960c (repository-maintenance 6a10ae64)

Compound attempt 254ae026, 2026-10-05, base b0e12d44 (fork/master),
campaign worktree `run-3c24960ce161-3c24960c`. Pre-review cycle evidence
only — assessment (62d48aed), research (d2ec79bb, pass 14), roadmap
(16d511fe), prioritization (fad95b72), stewardship (1d363a9a), implement
(7e665e4b → 7bd9e41c → d360bd7e), targeted_tests (8a40a6ae), full_tests
(d08e4a6f). **No validation was re-run in this phase** — the targeted
(focused 398/0; fmt/clippy/4 gates rc0; shellcheck baseline parity) and
full-suite (ci.yml mirror 20/20 steps rc0, 317 Rust tests 0 failed)
outcomes are consumed as recorded evidence, per the compound contract.

Attempt forensics: this same attempt id's first turn died to a delegate
provider failure (API-key auth, `cliproxyapi-key`) before any work —
events/254ae026….jsonl holds 2 lines (turn started + one progress ping),
no scratch directory, no typed result, worktree porcelain identical to
the inherited 8-file delta. Nothing durable was left; this turn redid the
phase from scratch on top of the intact inherited delta.

## Cycle outcome in one paragraph

Batch "Price and bound every number truthfully" landed uncommitted as an
8-file delta (+808/−27) at b0e12d44: rm-231 (context windows come from
the pricing catalog — `max_input_tokens` carried by the snapshot builder
and `lookup_context_window()`; the old substring ladder survives only as
the labeled `window_source: "fallback"` estimate; live RED/GREEN 132.2%
critical → 26.4% good on a 1M-window session), rm-196 (the session cache
stamps `pricing_catalog_id`, a 64-bit FNV-1a of catalog content, and
drops entries exactly once on mismatch so a catalog refresh re-prices
cached costs), rm-419 (the 2026-10-04 snapshot carries `deprecation_date`
— 463 of 3,099 entries — and doctor/report/session provenance disclose
vendor-deprecated models instead of silently billing them). Rider rm-417
was deferred per the stewardship default and remains a candidate. Wall:
113 ids, no duplicates — 99 new-format (61 candidate / 22 done / 16
implemented) + 14 old-format.

## Prevention rules (repo-durable, reuse verbatim)

**PR-1 — A fold rejection that names an attestation defect is repaired by
declaring, never by redoing.** Attempt 7bd9e41c's fold was rejected for
exactly one defect: the missing KTD13 `changed_surfaces` attestation. The
repair (d360bd7e) changed zero tracked files: verify the inherited delta
intact (`git diff --numstat` identical: 808/27), diagnose the engine's
delta offline against the live engine code
(`/work/projects/hermes-conductor`, `validation_policy.py`), then re-fold
with `validation_evidence.changed_surfaces` naming the repo-relative
paths actually changed. Engine facts that made the original fold fail and
the repair pass: the delta derives from the DISPATCHED base (not HEAD)
plus untracked files; the executable classifier admits `src/ tests/
scripts/ lib/ bench/ .github/workflows/` prefixes, `*.py`, and build
files — `crates/**` and `*.md` are never executable-classified; an empty
declaration against a non-empty executable delta rejects (a), a declared
executable path absent from the engine delta rejects (b), partial
declaration is fine. Redoing verified work after such a rejection wastes
a cycle and risks regression.

**PR-2 — Fold gates are dry-runnable offline before the fold is written.**
`apply_validation_gates` accepts SimpleNamespace stand-ins for run
(definition_id, definition_version, workspace_base_sha, workspace_path),
action (payload `{'validation_semantics': True}` — without it the gate
early-exits through phase_reject), and phase_result (status,
validation_evidence). A `''` return is only trustworthy after a known-bad
control (evidence with no digest → its real rejection string; stale
digest → "stale validation evidence") proves the body executed. This run
used it to reproduce 7bd9e41c's rejection verbatim before writing the
repair fold, and again at both validation folds.

**PR-3 — The pricing snapshot's keep-list is a contract, not a filter.**
`scripts/pricing/update-snapshot.sh` must carry `max_input_tokens` AND
`deprecation_date` (both were the one thing the old trim dropped); nulls
are dropped at ingestion. Regeneration is a network-required manual step
(curl of LiteLLM `model_prices_and_context_window.json` at main). Any
future refresh that loses either field silently reverts rm-231/rm-419 —
`bundled_snapshot_carries_context_windows_and_deprecation_dates` is the
pin that fails loudly.

**PR-4 — Cache-invalidation identity must be content-only and
clock-free.** `catalog_identity()` hashes entries+aliases and
deliberately EXCLUDES the source label and reference date, so the
cache/cache(stale) relabeling of the clock-free `pricing_source` contract
cannot trigger a re-pricing storm; only real content change invalidates.
Any future cache-freshness key (models.dev, ccusage overrides) should
follow the same shape: identity = content, never label or timestamp.

**PR-5 — Sibling composition goes through named seams and carve-out
comments, and goldens that will legitimately change get named in the
carry-forward.** `context_utilization_with(events, total,
window_source)` exists so the numerator lane (71f666e8) can replace the
numerator without touching the window ladder, and
diagnostics.rs:865's comment ("The numerator is deliberately untouched —
see rm-436 for the separate numerator defect") records the boundary.
Their landing UPDATES this delta's numerator golden expectations
(`context_utilization_divides_by_the_1m_vendor_window` asserts the old
numerator) — that update is correct, disclosed, and must not be flagged
as a regression or "corrected" back.

## Process lessons (conductor-fleet facing)

- **Provider-dead attempts are adoption candidates, not automatic
  redos.** 7e665e4b died ~31 min in; its durable trail was the uncommitted
  working-tree delta. It was adopted only after a full audit (every file
  diff read, seams checked, snapshot verified as a genuine regeneration,
  10 new tests present), and the adopting attempt supplied what was
  missing: bookkeeping and ALL verification. Never adopt unverified code
  without re-running its proof.
- **Validation-only turns preserve digest stability.** targeted_tests and
  full_tests touched zero tracked files (porcelain identical entry/exit),
  so `validation:v1:b5c4564e…` was byte-identical at the dispatch stamp
  and at both validation folds — three independent derivations. Content
  edits to tracked-modified files do NOT move this digest (it hashes
  committed state + true untracked files), so post-edit re-derivation at
  a fix fold returns the same token; never claim a "fresh" digest without
  deriving it.
- **Mirror-CI prerequisites.** The local ci.yml mirror needs its scratch
  tree pre-created (`mkdir -p $SC/{logs,home,tmp,ci-artifacts}`) — S03
  fails one test otherwise because `temp_session_file`
  (main.rs:1510-1511) uses `std::env::temp_dir()` and honors TMPDIR.
  Not a product regression; a mirror prerequisite. Also re-verify the
  ci.yml run bodies still match the sibling extraction when reusing a
  mirror at a different base.
- **`targeted_command: ''` disables covered_surfaces auto-inference**
  (exact-command-match rule); enumerate covered_surfaces explicitly,
  spanning the changed testable surfaces, in the evidence record.
- **`/tmp` persists across delegate sessions and runs on this host** —
  mirror logs (/tmp/at-full-d08e4a6f) and PoC caches survived between
  phases; rely on it for cross-phase evidence, but copy anything
  load-bearing into the spool.
- **The rm-436..438 band is double-minted across two live lanes.**
  71f666e8: context-pressure numerator / statusline numeric bounds /
  duration tiers (their stewardship request ed8aaeca; their wall band
  tops at rm-391 — the mints live in their lane artifacts). 6403d975:
  pi `type:"usage"` entries / pi_branches / modelId wire key (implemented
  in their worktree). Different subjects, same numbers. This run's live
  code pointer diagnostics.rs:865 cites the NUMERATOR sense of "rm-436".
  Integration gates must renumber by TITLE and then sweep live pointers
  — the first landing defines nothing while both are unlanded.
- **The ceiling moved mid-cycle.** origin/master advanced c032f33 →
  ea5c41e during the run (2c2db6f5's rm-400..402 band + 1cb61083's
  landing; ceiling wall 158 id-rows, max rm-402). Anchors minted at
  b0e12d44 may be line-drifted at integrate — the 75ae7fb6 PR-1 rule
  (reconcile to ceiling before anchoring) applies to the next cycle.

## Next-cycle context (concrete, for the cycle-4 assessment)

- **Recommended lead: rm-417** (CI example runner-trust) — this cycle's
  deferred rider, fully bounded (health-gate.yml :4/:9/:15 arms only;
  siblings own the mutable-tag/composite-action arms), no in-flight
  collision on the runner-trust subject.
- **Alternate: rm-418** (mtime prefilter for ranged discovery) — minted
  this cycle from research R1 + ccusage #1794 (9.5 s/4 GB → bounded;
  "byte-identical output" hard rule); composes with the session-cache
  freshness machinery rm-196 just landed.
- **Composition check once 71f666e8 lands:** context utilization becomes
  truthful end-to-end (their numerator + our catalog denominator);
  re-verify the combined golden at the next assess, expecting their
  legitimate expectation updates (PR-5).
- **Freshness gap re-opens immediately:** LiteLLM upstream runs hourly
  automation; our pin is 2026-10-04. rm-165/rm-176-class cadence items
  gain evidence every day the gap widens.
- **Known-open anchors** from assess 62d48aed (C1..C6): gate flags
  overview-only, `--clear-cache --demo`, evidence-row string sort,
  unbounded git output, control-byte sanitizer unlanded at this base,
  aider DST fold — no wall action taken, anchors in the assessment.
- **Watch:** upstream luoyuctl zero movement (tip 52ab2cd8, same 7 open
  issues), OTel GenAI export (rm-229), third "claude usage" demand wave
  (rm-201).
- **Next free id: rm-441** (above landed rm-402 @ ea5c41e and every
  recorded unlanded claim: rm-440 @6403d975, rm-435 @5bec3c93,
  rm-428 @fb22927c, rm-423 @16bbd3ae) — re-sweep before minting; ids
  come from a live sweep, never a recorded map.

## Carry-forward for the review/commit gates (do not double-implement)

- **One commit lands the batch** (rm-231/rm-196/rm-419) plus this
  compound's ROADMAP annotations and this record; CHANGELOG already
  carries 2 Fixed + 1 Added bullets and the refreshed snapshot-vintage
  line from the implement phase — nothing further owed there.
- **Live-pointer sweep at renumber:** diagnostics.rs:865's `rm-436`
  citation must follow the numerator item's post-renumber id (71f666e8's
  lane), never 6403d975's pi-usage item of the same number.
- **Golden-update disclosure:** 71f666e8's landing updates the numerator
  expectations in `context_utilization_divides_by_the_1m_vendor_window`;
  do not flip `fallback_context_window_ladder_is_unchanged_for_offline_models` (it
  pins the fallback ladder's intentional preservation).
- **Digest placeholders** (`validation:v1:<sha> in the shipping PR`) on
  the three items resolve post-commit; the recorded pre-commit digest is
  validation:v1:b5c4564ebc55ee1d66bef8238bf26d845356cd69c94080a8620d3d56
  bd12e769 at base b0e12d44.
- **Review scope note:** the first post-catalog-bump scan re-prices the
  corpus once (~7 min release binary on this host's real cache) — that is
  rm-196's recorded trade, not a regression.
- Review and shipping outcomes are out of this phase's scope; the next
  cycle's assessment carries them forward.

## Artifacts (this phase)

- `ROADMAP.md` — compound annotations only: the `compound c3` block
  (recorded outcomes, digest, id re-sweep, integrate-gate warning,
  cycle-4 leads) and per-item `cycle 3 implemented` outcome lines on
  rm-231 / rm-196 / rm-419 (validation outcomes cited as recorded
  evidence, done-flips reserved for the commit gate). No status line
  changed this phase — the implement phase's flips stand (61 candidate /
  22 done / 16 implemented of 99 new-format ids).
- This record. Spool copies of the verification log under
  `delegate/254ae026ef1e4e268577466252880064-scratch/`.

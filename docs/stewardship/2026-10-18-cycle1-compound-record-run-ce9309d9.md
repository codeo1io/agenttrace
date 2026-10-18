# Compounding Record — repository-maintenance `2e365acda43040f9abc6edf803acdabc` cycle 1

- Run: ce9309d9872e4da1958fc8a657f6c442 (roadmap attempt 30e969c, compound attempt 222c9e99)
- Wall: 6cd11e001557cdc001193b3e1249e55ca3de1cec (ROADMAP pristine md5 ce3cc0bd; band spool-side)
- Cycle: 2026-10-17 → 2026-10-18. Evidence: pre-review cycle records only (assess f4cb5f63,
  research fd2554e3, roadmap 30e969c, prioritize 7ba5283b, stewardship 1c668198, implement
  6413af23 + attestation redo 9a1c4081, targeted 0557992c redo, full b4b5347a).

## 1. Selected batch and outcome
"battery-truth hygiene" (rm-1954 LEAD + rm-1955 PAIR + rm-241 repo-side arm):
- rm-1954 DONE — tests/support/mod.rs `with_isolated_pricing_env` (env lock + scratch
  AGENTTRACE_SESSION_CACHE_DIR/HOME) wired into regional_pricing_contract.rs +
  pricing_snapshot_hygiene.rs; standing mutated-cache regression fixture
  (fireworks_ai/minimax-m3 window 524345 seed). Red-first preserved (assess PoC :307),
  poison-immune 9/9 post-fix.
- rm-1955 DONE — scratch_path helpers (entrypoints.rs, formula_lane_contract.rs) converted
  to teardown-on-drop; dead prologue rm removed.
- rm-241 arm LANDED (6 of 9 scripts) — inline CARGO_TARGET_DIR default
  `${XDG_CACHE_HOME:-$HOME/.cache}/agenttrace-dev/target/at-$(printf '%s:%s' "$PWD"
  "$(git rev-parse HEAD)" | git hash-object --stdin | cut -c1-16)`; explicit env wins;
  ambient-shared-layout warning. VALIDATION: 19/0 harness xtrace assertions + the full
  battery itself ran under the namespaced default.
- Validation record: targeted 0557992c (static + 3 gates + fail-legs), full b4b5347a
  (CI-exact trio per the vacuous-block discipline): **1267/0 over 78 suites** (baseline was
  1263; +4 = the new arms), fmt rc0, clippy -D warnings rc0.

## 2. Reusable lessons / prevention rules (this cycle's new ones)
1. KTD13 attestation parity: implement envelopes MUST declare validation_evidence.
   changed_surfaces covering every changed path — the engine counts +x files; a missing
   list folds-rejects despite a green tree. (Cost: one full redo attempt 9a1c4081.)
2. Emission-time digest: when a validation-only phase changes nothing executable, copy the
   dispatch-time validation_digest VERBATIM (the fold gate re-derives from the tree); never
   hand-roll a hash — no in-repo digest tooling exists to reproduce it downstream.
3. Empty full_command: derive the authoritative suite from CI's test lane (ci.yml test job)
   — already codified in
   docs/solutions/workflow-issues/vacuous-conductor-validation-block-derive-scope-from-the-diff.md.
4. Spool GC hits BOTH the delegate scratch dirs AND the ~/.cache mirrors within ~a day;
   the in-tree pristine ROADMAP (when the band is spool-side) is the only stable source —
   band builders should read the wall directly with an md5 assertion (as build_band_v2.py
   now does), and PhaseResult summaries must be self-contained.
5. Provider-reap discipline exercised twice this cycle (implement attestation redo;
   targeted redo after 5d0aecff left no adoptable envelope): forensics first (event log +
   absent typed envelope + absent scratch = redo from scratch), otherwise adopt by
   sha256 pins.
6. Patch-building rule: never embed a raw "\n" inside a single band list element — split
   before insert; difflib emits it as one malformed line otherwise (caught by the
   round-trip cmp; always run patch --fuzz=0 + git apply --check + cmp before pinning).

## 3. Residuals and next-cycle candidates (concrete)
1. rm-241 tail: check-locked-cargo.sh, check-release-surfaces.sh, check-install-runtime.sh
   still lack the namespaced default (grep 'agenttrace-dev/target/at-' = 0 ×3). Small,
   same shape as the landed six.
2. rm-176 drift refresh (sanctioned, post-cron): cron fires 2026-10-20 06:00 UTC,
   predicted red; scope = drop-with-disclosure(466 retired) + adopt(~96 costed new incl.
   bedrock commitment-duration shape; 4 glob rows stay filtered per rm-903).
3. rm-451 integration: completed fix unmerged on conductor/run-7517204088bd (da93239e) —
   land that branch rather than re-derive; parse_file at HEAD still unbounded
   (parser.rs:29).
4. zst admission (d2355d7 lineage): discovery.rs:1204 still admits only .jsonl/.json while
   the parser decodes zst rollouts.
5. fog rm-1567 kinship: if 66e28175's machine-cache precedence PIN lands, reconcile titles
   against rm-1954 (isolation vs pin — disjoint fixes, both valid).
6. Watches: antigravity gone from models.dev (re-anchor/retire the extraction lane);
   opencode v2 trigger = first anomalyco/opencode changelog entry touching session
   storage/db/schema; codex 0.163.0 stable (no parser delta); pi 1.3.0 thinking-drop.
7. New baseline: 1267/0/78 for this lineage; namespaced targets are the default lane for
   the six landed scripts — keep validating in private targets until the residual three
   land.

## 4. Artifacts ledger (spool-side, mirrors swept — re-derive from PhaseResults if lost)
- ROADMAP band v2 (this cycle's authoritative seam artifact, SUPERSEDES v1 30e969c):
  band-v2.patch md5 5ac04e1dc1a300bd9e0eed75853778c9, postimage md5
  9698b60195e2390f891001336e2f862c (432 def rows, 0 dups, max rm-1955), 8 hunks,
  insert-only on wall lines, patch/git-apply/cmp verified.
- Validation logs: /home/agent/.cache/at-full-b4b5347a/battery.log,
  /home/agent/.cache/at-tt-0557992c/logs/, /home/agent/.cache/at-impl-9a1c4081/.
- Implement delta (in-worktree, 11 M + 1 ??): pinned by per-file sha256 in the
  implement PhaseResults.

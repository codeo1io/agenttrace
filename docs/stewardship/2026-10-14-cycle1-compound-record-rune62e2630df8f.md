# Cycle 1 compound record — run e62e2630df8f (repository-maintenance 0799db9e)

Phase: compound (attempt 1f1fd994), 2026-10-14. Pre-review evidence only — this
record compiles the cycle's assess/research/roadmap/prioritize/stewardship/
implement/targeted_tests/full_tests outcomes. Review and shipping happen after.

## Batch (implemented and validated; uncommitted at dispatched pin 794b6e5 —
status stays candidate until the commit gate)

Lead **rm-1419** — pricing-catalog load-time duplicate-key collapse
(`crates/agenttrace-core/src/pricing.rs`): two-phase family conversion resolving
every duplicate normalized-key family by an explicit total order (provider
priority → shortest raw spelling → lexicographic), each losing spelling keeping
its own exact-match entry, and `lookup_price`/`lookup_price_tiered` made
raw-first so exact entries are reachable. `--doctor` discloses
"1,601 duplicate catalog keys across 459 families resolved by rule (priority,
then shortest spelling)". Census moved 61 of 1529 → 65 of 1529 priced models
(one model per family — the winner's row at the family key; svg goldens
regenerated via `UPDATE_CARD_GOLDEN=1`).

Rider — hermetic CI check gates (`scripts/ci/check-{deterministic-output,
report-semantics,output-contract}.sh`): repo release binary + run-private
`mktemp` out-dir defaults, named missing-binary error, doctor leg scoped off
the operator HOME.

Rider — lineage-lag closure guidance (zero code): F2/F5/install-runtime are
wall-fixed; rebase + verify, never re-fix.

Validation: targeted 73 passed / 0 failed over the engine-named surfaces; full
ci.yml-verbatim battery — fmt, clippy `-D warnings` (3 crates), 62 test suites
(1,159 tests) 0 failed, MSRV `+1.89`, npm `node --test` 4/4, 16 gate lanes —
with two STANDING REDS at pin, both owned elsewhere and disclosed:
`check-install-runtime` test D (rm-051) and `check-plugin-version` (missing
`## v0.0.4` CHANGELOG section, red lineage-wide). Digest
`validation:v1:dcfb99fc…cfd0ffa` stable pre-work/post-targeted/post-full
(engine re-derivation, full 40-char base sha).

## Prevention rules banked (from this cycle's failures)

1. **Result-envelope shape (KTD13):** `validation_evidence` belongs INSIDE
   `phase_result`; a top-level sibling is invisible to
   `PhaseResult.from_envelope()` and folds as an empty attestation (cost one
   rejected implement attempt; root-caused against engine source at
   `/work/projects/hermes-conductor/src`).
2. **Digest base-sha string:** `validation_digest` hashes the base-sha STRING —
   deriving with a short sha never matches the engine's full-sha digest.
3. **Local full-battery pattern:** private `CARGO_TARGET_DIR` on `/var/tmp`
   (home FS fills), scoped TMPDIR/HOME, `AGENTTRACE_BIN` override for binary
   gates, copy the release binary into gitignored `target/release` for the two
   `real-*` smoke gates that hardcode the repo path, and RECREATE TMPDIR before
   npm lanes (the /tmp sweeper strikes mid-battery).
4. **Pricing catalog rule:** duplicate families resolve by an explicit total
   order; losers keep exact-match entries except a fixed-point loser, whose
   family key keeps the WINNER's row; censuses count one model per family —
   the winner's row at the family key — never entries-minus-losers. Any
   future snapshot refresh must regenerate svg goldens with that census in
   effect.

## Artifact regeneration disclosure

The roadmap phase's delta (adopted attempt 35fab3bd, postimage 4599e9e0 vs
wall 3e90993) was delivered spool-side; its scratch and both /tmp twins were
reaped before compound. The roadmap delta + this cycle's compound band were
REGENERATED from in-context text and the surviving typed result JSON
(`delegate/35fab3bd…json`), retargeted at the LIVE wall
(origin/master 16dcc9a0, ROADMAP md5 1f1ab384, 395 rows, def-max rm-1502):
the regenerated postimage (396 rows, no duplicate ids, footer last) and patch
are authoritative per the established reaped-band rule. Round-trip proven:
`git apply` and `patch -p1 -F0 -N` both reproduce the postimage byte-exactly.

Wall-drift census at compound time: no roadmap id exists between rm-1373 and
rm-1500 — the entire unlanded fog band 1394-1418 (including the rm-1415 ×3
pricing-collapse title storm) never landed. rm-1419 is unclaimed on the live
wall and keeps its numeral; next free rm-1503. By-title reconciliation applies
at the commit gate if a sibling pricing-collapse claim lands first.

## Review-fix addendum (2026-10-15, attempt 07189837)

Independent review (13d14ef5) returned NEEDS_CHANGES with six findings; all
six are fixed in the uncommitted batch and re-validated at targeted scope:

- F1 (high): fixed-point losers no longer overwrite the winner's row at the
  family key (`pricing.rs` loser insert guarded; regression test
  `fixed_point_loser_never_overwrites_the_winner_row`). The 15 affected
  gemini families now attribute provider/flex/deprecation/window from the
  gemini winner, matching the documented rule and the collapse ledger.
- F2 (medium): censuses count distinct models — one per family, the winner's
  row at the family key (`spelling_duplicates`/`is_duplicate_spelling`
  fixed-point-aware); deprecation census prints `65 of 1529`, goldens
  regenerated, doctor line unchanged (1601/459 is ledger-counted and was
  always correct).
- F3 (high): `resolve_lookup_model` resolves user aliases (rm-511) ahead of
  raw catalog hits; regression test `user_alias_hops_outrank_exact_catalog_rows`.
- F4/F6 (low): CHANGELOG census note and this record corrected to
  `65 of 1529` with the true cause (winner re-picks + one-per-family census);
  the false zero-rate-filter attribution removed; prevention rule 4 encodes
  the one-per-family rule; heading prose normalized (status stays candidate).
  No durable artifact carries the old `63 of 1514` — the spool-side roadmap
  band and its scratch were reaped, and the surviving typed JSON never
  carried census numbers (verified).
- F5 (low): all three CI gate scripts derive the repo root from the script
  path, so out-of-repo invocations with `AGENTTRACE_BIN` set work.

Targeted re-validation: focused pricing/doctor/golden suites + the new test
file + the three gate scripts + fmt/clippy/changelog-structure — green;
see the attempt result for counts. Full suite re-runs at the next gate.

## Next-cycle candidates

- Cheap standing-red cures: install-runtime test D ref pin (rm-051); the
  missing `## v0.0.4` CHANGELOG section (plugin gate).
- Rebase + by-content-hunk reconciliation of this batch against the moved wall
  (pricing.rs was rewritten by the 04:54Z fleet drift; rm-1419's acceptance
  corridor — us-gov $5.4000 @1M/100k — must survive the rebase).
- Snapshot-refresh interplay: next LiteLLM bundle refresh re-runs
  `svg_card_contract` goldens under the duplicate-exclusion census.
- Fog hygiene: prune the stale 1394-1418 duplicate-numeral claims by title.

# Cycle 1 review fix — run a9e73293 (2026-10-01)

Run `a9e73293b08043d58ec81c73d91b6dac`, campaign `repository-maintenance:b099a3a5`,
phase `independent_review` (action `independent_review:fix`), worktree
`run-a9e73293b080-a9e73293` at HEAD `ec8acdcd59b1cc284dae5aa51936d18c83324939`,
branch `conductor/run-a9e73293b080`. Review under remediation: verdict
NEEDS_CHANGES, attempt `7e524f5b390a434a819db15fc6a42888` (5 actionable findings).

## Verdict

CLEAN — all 5 actionable findings are fixed in the working tree, the rm-198
"red on pre-fix, recorded" acceptance clause is met with first-hand evidence,
and the retest at the dispatch-required targeted scope is green (gate rc 0,
279 passed / 0 failed — identical totals to the pre-review full-suite record).

## Prior-attempt forensics

Attempt `7fe82682c853450685cef77163ac195b` (provider-reaped mid-turn) had
already applied fixes for all 5 findings plus one validation-blocking rider
in-tree before dying; its typed result was never written. This attempt
verified every one of its deltas first-hand against code facts (3s bound,
snapshot-schema version 7, LIKE-split outcome SQL, effect-disposition comment,
zero stale `2s`/`outcome column` strings anywhere), independently reproduced
the one evidentiary artifact whose worktree was gone (the rm-198 red run),
anchored its durable reference, and re-ran the validation gate itself.

## Findings fixed (all 5)

| # | sev | where (fixed) | fix |
|---|-----|---------------|-----|
| 1 | P1 | `docs/stewardship/2026-10-01-cycle1-implementation-record-runa9e73293.md:28` + spool `roadmap-status-rm198-200-202-208.patch` rm-198 cycle line | Claim corrected in both artifacts: outcomes derive from `messages` result rows via the error-marker LIKE split; the schema 6→7 bump is the LOCAL snapshot cache (`session_cache.rs:23`); no hermes schema change (`effect_disposition` stays unused, per the comment at `sqlite_sessions.rs:669-676`) |
| 2 | P2 | rm-198 acceptance clause "red on pre-fix, recorded" | RED recorded first-hand this phase: detached worktree at pristine `ec8acdc` + the tests-only hunk of the batch diff → `hermes_tool_outcomes_come_from_result_rows_not_the_call_count` FAILED (assert at :941, left 2 / right 1 — the pre-fix `row.get(5)` twin-read counts calls, not outcomes) and `hermes_tool_failures_trip_the_overview_gate` FAILED (assert at :979, `tool_fail_rate was 0`); 0 passed / 2 failed, rc 101. Log: `/home/agent/.hermes/conductor-delegate-spool/793d4afbafd04ce98fe639d8325b5cb1-scratch/rm198-red-run-reproduced.log`; record doc and status patch cite the reproduction recipe |
| 3 | P2 | record `:29` + status patch rm-200 cycle line | "bounded (2s)" / "bounded to 2s" → 3s (`CLIPBOARD_WAIT_BOUND`, `explorer.rs:1962`) |
| 4 | P3 | `docs/solutions/process-issues/validation-digest-base-and-coverage-reconciliation.md:44-48` | `scripts/` and `.github/workflows/` named as real, digest-visible roots in this repo; `src/`, `lib/`, `tests/`, `bench/` named as policy prefixes that are not top-level roots here; the crates-only (invisible) case separated from the visible gate-script case |
| 5 | P3 | `PRIVACY.md:20` + `README.md:183-186` | rm-208 evidence clause met: owner-only `0600` at creation (umask can only tighten; pre-existing files keep their mode until rewritten/cleared) and transient `.tmp.<pid>.<seq>` siblings documented, reclaimed by the next cache load |

## Validation-enabling rider (not a review finding)

`crates/agenttrace-core/tests/discovery_contract.rs:2371-2377` — the
`data_health_discovered_is_range_independent_and_splits_out_of_scope` fixture
hardcoded a "recent" session at `2026-09-01T10:00:00Z` against a
`now − 30d` window, silently aged out at `2026-10-01T10:00:00Z`, and then
failed deterministically forever (first seen live in this phase's retest
fallback: 71 passed / 1 failed at :2410; pre-existing at clean HEAD — the
independent-review envelope predates the 10:00Z boundary). The fixture is now
clock-derived (`now − 1d`) with a time-bomb guard comment — same fix shape as
sibling run 4b9cc093's rider. Without it the required retest cannot pass; it
changes a test fixture only, no product code.

## Retest evidence (this attempt)

- Command: the work order's `targeted_command` VERBATIM —
  `run_repo_impacted_tests.py --repo . --mode fast --jobs 8 --fallback-command '… local_validation_gate.py --shell-command '"'"'cargo test'"'"'' -- scripts/ci/check-install-runtime.sh`
- The selector could not prove a narrow scope and fell back to the
  authoritative full validation, as in every earlier phase of this run.
- **GATE_RC=0. 279 passed / 0 failed** across all test binaries, zero
  `FAILED` lines; totals identical to the pre-review `full_tests` record
  (272 pre-batch baseline + 7 batch tests), so the review-fix delta
  introduced zero regressions. Time-bomb binary green
  (`data_health_discovered… ok`); both hermes tool-outcome tests green.
- Envelope `result-3414212-334920619.json` (returncode 0; envelope-internal
  digest `validation:v1:4e35ac25…`, `digest_base "unknown"` — not the declared
  digest, matching the accepted full_tests adjudication). Transcript:
  `…793d4afb…-scratch/targeted-retest.log`.
- Declared digest re-derived after all edits at base `ec8acdc` →
  `validation:v1:9f95dd5c24d21ea3fb1781758e355a78fcb2891e720a7574defd8d9cd057092d`
  (byte-identical to the dispatch digest: every review-fix delta lives outside
  the digest's executable-surface set — `crates/**` and docs).
- `cargo fmt --all --check`: exactly the 8 pre-existing master-debt blocks
  (`session_cache.rs:268,1845`; `pi_family_discovery.rs:14,53,61,69,125,133`),
  zero new blocks.
- Spool status patch re-verified end-to-end after its truthfulness edits:
  `ROADMAP.md` (in-tree) → `roadmap-rm194-rm212.patch` →
  `roadmap-status-rm198-200-202-208.patch` composes clean; 81 unique
  `- id:` lines; all four status flips present with truthful cycle lines.

## State at exit

`git status --porcelain`: 14 M (the 13-file pre-review census, of which
11 tracked-modified, + `PRIVACY.md` +
`README.md` + `crates/agenttrace-core/tests/discovery_contract.rs`) + 3 ??
(the two compound-phase docs + this file). HEAD `ec8acdc` unchanged; no
tracked file outside the declared delta touched; stash empty; temp red-run
worktree removed. risk_class low — a test-fixture and docs delta; no product
executable changed this phase.

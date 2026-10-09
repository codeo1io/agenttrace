# Cycle 1 compound record — run fc996e6d4fb9 (repository-maintenance a8f30a4c, cycle 1)

Date: 2026-10-07 · compound attempt 3984457a (after targeted_tests 1318f5f0 and full_tests dc70721a)
Base: e9e8fd9 (HEAD, campaign worktree run-fc996e6d4fb9-fc996e6d; porcelain at compound = the 8-file
implement delta + this record — the d65f72c7 compound precedent)
Scope: pre-review compounding only — NO test execution at compound per the phase contract; all outcomes
below are the recorded envelopes and proof chains of the implement/targeted/full phases, consumed as
evidence.

## Batch: "honest failure boundaries II: no silent wedges on the exec/load path"

| row | subject | files | outcome |
| --- | --- | --- | --- |
| rm-731 (LEAD, reliability 82.0) | run_bounded's fast-exit arm enforced its deadline only while the child lived — post-exit drain joins unbounded (assess F1: 25,106 ms on the 10 s bound vs 81 ms control; governance wait_child_bounded had no deadline arm at all) | crates/agenttrace-cli/src/upstream.rs, crates/agenttrace-core/src/governance.rs | implemented pre-review |
| rm-732 (reliability 80.0) | warm replay re-admitted a fifo-shaped entry from a stale walk-3 dir listing and wedged every load path rc=124@15 s (assess F2; falsified the landed CHANGELOG rm-212 claim on the warm path) | crates/agenttrace-core/src/discovery.rs, session_cache.rs, tests/discovery_contract.rs, CHANGELOG.md | implemented pre-review |
| rm-733 (rider, maintainability 35.0) | cfg-duplicated byte-identical admit_session_path bodies — the exact drift factory rm-212's round-2 review broke on | crates/agenttrace-cli/src/main.rs | implemented pre-review |

ONE change-unit (stewardship decision D1): shared silent-wedge theme, all three PoC'd this cycle at this
base. Uncommitted delta total +329/−57 across 8 files (ROADMAP.md +39 = the roadmap-phase mint + the three
implemented flips with dated EXECUTED bullets). Done-flips reserved to the commit gate (rm-012 convention).

## Recorded validation outcomes (NOT re-run at compound)

- implement f7a91e15 — PRIOR-ATTEMPT ADOPTION declared: reaped attempt 321de1a6 (13:14–13:48Z, 17 progress
  events, session_reaped after the code write) authored the delta worktree-only; adopted after hunk-by-hunk
  verification against the stewardship contract and a full re-run of every pin. The dispatch-named attempt
  1750ac40 did nothing (24 s, 2 events, reaped).
- targeted 1318f5f0 (dead sibling c5cbae26's partial 4-leg battery REJECTED for adoption — no digest
  derivation, no tree census, no upstream leg): 6 prescribed legs rc0 — cli bins 62/0 (pins
  `run_bounded_fast_exit_survives_a_grandchild_holding_the_pipes`,
  `run_bounded_timeout_survives_a_grandchild_holding_the_pipes`,
  `special_file_kind_classifies_character_devices`), core governance 15/0 (pin
  `git_commits_probe_survives_a_helper_holding_the_pipe_past_exit`, 10.06 s = probe budget honored),
  discovery_contract 91/0 (pin `warm_replay_revalidates_file_kind_for_stored_listings`), session_cache 19/0,
  upstream integration 9/0; cargo fmt --check rc0; clippy -p agenttrace -p agenttrace-core --all-targets
  --locked -D warnings rc0.
- full dc70721a (dead sibling ec101797: 84 s, reaped, nothing durable — redo): work-order full_command="" →
  the authoritative suite is .github/workflows/ci.yml at base e9e8fd9 mirrored lane-for-lane (ci.yml +
  scripts/ byte-identical to the prior runner base 61570ea); 22/22 executed lanes rc0 (documented skips:
  PR-gated lint job mirrored by its steps, schedule-gated MSRV floor, repo-var-gated TUI real smoke,
  never-fetch pre-gate); tests lane 554 passed / 0 failed over 26 result lines = the 551 pristine assess
  baseline + the 3 batch pins EXACTLY (diff census +3/−0 `#[test]`); docs-commands, deny, locked-cargo,
  release-build, output-contract/deterministic/report-semantics/install-runtime/real-cli-smoke lanes all
  rc0. Tree byte-stable pre/post battery: git-diff sha 422280bf…, exactly the 8 implement files, 0 untracked.

## Digest lineage

Dispatch literal `validation:v1:1db2211fc741d0a1539fcba4abfeb36123cf5a2e7acb67afe3d83eb64469036a` vs live
re-derivation (deployed `hermes_conductor.validation_policy.validation_digest` over this tree, base
e9e8fd9) `…23cf5a2a7acb…` — they differ at exactly ONE hex char (index 53). sha256 avalanches: any real
input change moves ~half the body, so a 77/78-char match is a transcription corruption, not a stale digest.
The engine's `pending/<attempt>.json` action_payload is the authoritative source when the rendered prompt
disagrees (fleet gotcha, now observed twice in this lineage); the live-derived value was declared at
full_tests. The batch touches no digest-covered surface: executable classification for this repo is
`.github/workflows/*` + `scripts/**` (25 files), none modified here — so the current digest is also the
post-batch digest.

## Dead-attempt forensics (three this cycle, one adoption, two redos)

- implement: named attempt 1750ac40 nothing (24 s); real author 321de1a6 reaped after the code write —
  ADOPTED (event log + file mtimes + hunk-by-hunk contract verification + pin re-runs). Declared in the
  implement result.
- targeted_tests: c5cbae26 reaped twice, left a 4-leg partial battery log — REJECTED (no digest, no tree
  census, no upstream integration leg); phase redone from scratch and declared.
- full_tests: ec101797 reaped 84 s in, no typed artifact, no scratch — nothing to adopt; redo.

Standard applied each time: read the typed artifact AND the event-log tail, verify lineage/tree census/
cleanliness, then adopt or redo — and say which. An absent envelope is not evidence the work never
happened; a present artifact is not proof it is valid.

## Id accounting

ZERO ids minted at compound. Wall after compound: 260 def rows, zero duplicate ids, wall max rm-733,
managed footer last. rm-731/732/733 stay `implemented` with dated EXECUTED bullets; done-flips reserved to
the commit gate (rm-012). CHANGELOG Unreleased carries the two new Fixed bullets (already in the delta).
Fleet note: unlanded sibling bands at this base (4c3ca863's rm-730 spool patch; dirty-lane ceilings
rm-728/rm-707) — re-census live before any next mint and reconcile by title at integration.

## Prevention rules (reusable, this cycle's crop)

- **PR-1 — a deadline enforced only while the child lives is not a deadline.** Bounded-subprocess code has
  two arms (child-alive timeout; fast-exit drain) and the second is where the bound silently dies: after
  the child exits, any join on reader threads is unbounded whenever a grandchild (git-remote-https, an
  exec'd hook, `sleep &`) holds the pipe write-ends. Give every post-exit drain the REMAINDER of the
  budget (bounded mpsc recv, not join) and degrade with the named error; the stronger parked alternative
  is a dedicated process group via pre_exec setsid with a group kill. Regression shape: PATH-shim git that
  exits rc0 immediately but leaves a pipe-holding helper — the call must return inside the bound (pins:
  `run_bounded_fast_exit_survives_a_grandchild_holding_the_pipes`,
  `git_commits_probe_survives_a_helper_holding_the_pipe_past_exit`).
- **PR-2 — cache replay must re-derive admission invariants, and every admission-rule change bumps the
  artifact version.** A gate applied only on the cold/write path is bypassed verbatim by warm replay of
  pre-rule artifacts. Re-validate at read time (re-stat file kind before admitting a stored listing entry)
  AND bump the walk/schema version so stale artifacts retire once. Corollary: when a second path falsifies
  a landed CHANGELOG claim, re-scope the original bullet to name both paths — do not only add a new one
  (the rm-212 fifo clause now reads "on the cold walk … and on the cached-walk replay as well").
- **PR-3 — cfg-duplicated bodies are a drift factory.** If `#[cfg(unix)]` and `#[cfg(not(unix))]` bodies
  are byte-identical except a helper call, write the body once platform-generic and cfg only the helper
  (the repo's own pairing convention: doctor.rs:718/724, session_cache.rs ×5). Prove byte-identity before
  collapsing; keep every `std::os::unix` reference inside a cfg(unix) item so the E0433 class cannot recur.
- **PR-4 — adoption triage standard for reaped attempts.** An uncommitted delta from a dead attempt is
  adoptable ONLY after (a) event-log forensics, (b) hunk-by-hunk verification against the phase contract,
  (c) full re-run of the pins; a partial validation battery without digest derivation and tree census is
  NOT adoptable (c5cbae26). Declare adoption + verification legs in the result either way.
- **PR-5 — one-hex-char digest divergence is transcription corruption, never staleness.** When the
  rendered prompt's validation_digest differs from the engine's `pending/<attempt>.json` payload (or a live
  re-derivation via the deployed policy) by a single character, the engine file is authoritative; declare
  the live-derived value and record both strings. This lineage observed it at the same base twice.
- **PR-6 — full-suite lane mirrors must survive the session.** Run the battery detached (setsid nohup +
  pidfile) from the delegate spool scratch (never /tmp — swept between sessions; never the repo root),
  strictly sequential (this host's pids.max fork trap), heavy cargo lanes through the host admission gate
  (workers=1 — expect queue time behind sibling runs). Worktree `target/` may be swept between phases:
  budget wall time for cold rebuilds of clippy/test/release lanes (~15 min each under load).
- **PR-7 — shared-const seams reconcile as recorded unions, not conflicts.** When two lanes bump the same
  versioned const (here DIR_LISTING_WALK_VERSION 3→4 vs integration-d2bd9afa's identical bump;
  SESSION_CACHE_SCHEMA_VERSION elsewhere), the second lander re-bases onto the advanced ceiling; a
  same-value bump is a no-op union recorded at the seam (this row's re-stat + pin + CHANGELOG repair stand
  either way).

## Commit-gate checklist (for the landing, not compound)

1. ONE commit: the 8 modified files + THIS RECORD (untracked — `commit -a` drops it; stage explicitly:
   `git add docs/stewardship/2026-10-07-cycle1-compound-record-runfc996e6d4fb9.md`).
2. Flip rm-731/732/733 implemented→done by title with dated flush lines (rm-012 convention); keep the
   dated EXECUTED bullets.
3. SEQUENCE at integration: 4c3ca863's unlanded rm-730 spool patch (b27ec6ae) shares this base — apply its
   rows first by mtime, then this band's tail rows; union the co-appended rm-017 crates.io note.
4. Sibling-lane seams: second lander re-bases onto the advanced ceiling for session_cache.rs consts
   (32f3b7a1 SESSION_CACHE 32→33; integration-d2bd9afa WALK 3→4 — same-value union with ours).
5. No SESSION_CACHE_SCHEMA_VERSION or SQLITE_SNAPSHOT_SCHEMA_VERSION moved by THIS batch (WALK_VERSION is
   separate) — no governance-guide sentence alignment owed; check-docs-commands.sh already rc0 at
   full_tests (lane 12) on this exact tree.
6. Full battery is green at diffsha 422280bf… (22/22 lanes) — the fold gate's digest re-derivation will
   match the live-derived value declared at full_tests (see PR-5).

## Next-cycle context

Lead candidate: **rm-041 re-lead** (session-cache store integrity) — the orphaned origin-pushed branch
0aba09b (schema 24→26 era, +1518/−63) has zero descendants in any ref while mainline advanced to schema
32; its defects are live at HEAD (lossy `to_string_lossy` cache keys re-confirmed at
session_cache.rs:1560-1562, mtime-only freshness). Re-lead from schema-32 taking 0aba09b as a reference
diff, or record deliberate abandonment at origin — today it is neither landed nor retired. Riders:
rm-439 (positional-path swallow family — this cycle's P6/P7 probes extended it to
`--statusline-report`/`--budget`), rm-558 (--fetch swallow arm remains; the journal-append arm retired by
rm-573), rm-006/rm-176 pricing refresh (intra-day re-pricing now quantified — 18 shared-key priced-field
mutations in ~2.3 h; stamp/hash/age-disclosure arms over cadence alone). Watch items unchanged: upstream
luoyuctl quiet, OTel semconv-genai zero tags (rm-493 untripped), tool versions flat. Re-census the fleet
ceiling live before selection/mint (rm-733 is this cycle's landed tail; unlanded bands at rm-728/rm-730
contest the space above).

## Review round 1 — fix addendum (2026-10-08, independent_review:fix 049770d5)

Independent review 24bfe53f: NEEDS_CHANGES — 1 MEDIUM + 3 LOW, all fixed this round inside the
same uncommitted delta (no new ids; the two touched rows carry dated review-fix bullets):

- MEDIUM upstream.rs — the rm-731 fast-exit arm surfaced `GitRunError::Timeout` whose Display
  said "and was killed" and whose `--fetch-upstream` wrapper said "the fetch was killed" —
  false when the child exited naturally and a helper held the pipe write-ends. Display + both
  wrappers reworded to arm-neutral truth ("its output was not recovered within the bound");
  zero-test-churn honored — the three op/bound pins' exhaustive `Timeout { op, bound }`
  patterns are untouched, so no new variant fields.
- LOW1 main.rs — `write_output_honors_symlinks_instead_of_replacing_them` and
  `write_output_refuses_special_files_instead_of_materializing_them` carried ungated
  `std::os::unix` references (non-unix `cargo test` could not compile the file); both gained
  `#[cfg(unix)]`. The rm-733 EXECUTED claim is now true of the test module too; non-unix build
  still unverified on this host (no mingw toolchain).
- LOW2 governance.rs — the new PATH-shim pin now takes `crate::test_env::lock_env()` per the
  lib.rs contract for env-mutating unit tests. The pre-existing PATH-shim pins without the
  lock are out of batch scope (flagged for the next cycle).
- LOW3 CHANGELOG.md — the nonexistent `` `upstream-timeout` `` backtick literal corrected to
  `GitRunError::Timeout` (the landed rm-583 bullet's same shorthand is pre-existing text and
  was left untouched).

Retest (required_scope=none → targeted legs only, per the review-fix validation budget; the
review gate re-runs the full battery): `cargo test -p agenttrace --bins --locked`;
`cargo test -p agenttrace --test upstream --locked`; `cargo test -p agenttrace-core --lib
governance --locked`; `cargo fmt --check`; `cargo clippy -p agenttrace -p agenttrace-core
--all-targets --locked -- -D warnings` — all rc0 (logs in the fix-round scratch). Validation
digest unchanged — the batch still touches no digest-covered surface (`.github/workflows/*` +
`scripts/**` only).

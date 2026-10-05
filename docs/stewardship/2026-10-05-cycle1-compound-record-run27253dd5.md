# Cycle-1 compound record — run 27253dd5 (repository-maintenance 059e84c3)

Run `27253dd59fcc4a76a70f78c24893681f` · cycle 1 · base `b01edcd` (porcelain-clean at assess
entry) · 2026-10-05 · compound attempt `b775d339e84d47fabda8351fe899a638`, completed under
re-dispatch by attempts `40b6838f1d5c49cc821bb83ad809eb61` and `e7e056ce1ac042ff86fb131426cb583c`
(adopt + verify — see Completion notes at EOF).

Pre-review record: every outcome below was recorded by its producing phase and consumed here
as evidence — nothing was re-run (compound runs no tests). Review and shipping outcomes land
in later gates; the next cycle's assessment carries them forward.

## State at compound

Uncommitted worktree delta in `run-27253dd59fcc-27253dd5`, all owned by this cycle:
`M CHANGELOG.md`, `M ROADMAP.md` (this phase: roadmap patch 6a852769 applied in-tree +
compound appends), `M crates/agenttrace-core/src/pricing.rs`, `M
crates/agenttrace-core/src/session_cache.rs`, `M crates/agenttrace-core/src/lib.rs`, `M
crates/agenttrace-tui/src/tests.rs`, `?? crates/agenttrace-cli/tests/warm_cache_pricing.rs`,
`?? docs/stewardship/2026-10-05-cycle1-compound-record-run27253dd5.md` (this file).

## Cycle outcomes

| phase | attempt | outcome |
|---|---|---|
| assess | ddefde10 | 4 fresh live-proven findings F1–F4 + 1 evidence refresh (rm-196 warm-cache pricing PoC) at b01edcd; baseline cargo test --locked --workspace 379/0 across 14 binaries; porcelain 0 in/out |
| research | 1c1027cd | 2 new candidates + 2 evidence folds + 1 spec cross-ref, all first-hand; C1 pi UsageEntry cache_warm = 93%-of-spend blindness (HIGH, unclaimed on wall); upstream frozen (b886850), open set #308/#309/#310 |
| roadmap | 6a852769 | ADOPT+RENUMBER of unread 6dccab2c (its rm-500 double-claimed by live a6bb3c89 3 min later): 1 mint rm-501, 3 folds (rm-007/rm-196/rm-232), 5 cedes (F1→rm-496, F2→rm-497, F3→rm-437, C1→rm-436, C4→rm-455); patch verified round-trip on pristine HEAD, left as scratch patch |
| prioritize | b6873c80 | single-unit batch 'Warm-cache cost truthfulness' LEAD rm-196 (88.0); fleet-collision audit region-disjoint; no riders (all cheap candidates reserved/governance/prohibited-stage-blocked); deferral queue recorded |
| stewardship | 5b06c095 | batch charter; no riders; sequencing note: rm-196 must land BEFORE rm-176 (snapshot automation) |
| implement | 5f3a9aab | RED captured on pre-change binary → GREEN: pricing_fingerprint() + stamp/compare in session_cache.rs (schema stays 22, serde(default), no bump), e2e warm_cache_pricing.rs, 3 in-module tests; fmt/clippy clean |
| targeted_tests | 10b7a582 | gate run 1 rc=101 (TUI ctrl_r fixture planted unstamped warm entry) → fixture aligned → run 2 rc=0, 384/0 |
| full_tests | 1683a987 | authoritative gate command verbatim, rc=0, 384/0 across 13 binaries + 2 doc-test sections; digest re-derived unchanged |
| compound | b775d339 → 40b6838f → e7e056ce | this record + ROADMAP status/appends (rm-196 → implemented; rm-501 completability note); 40b6838f and e7e056ce each adopted + re-verified the end-state after the engine left the prior results unread |

## Roadmap accounting (this cycle)

- rm-196 `candidate → implemented` + compound addendum (gates, digest, fixture-class note,
  stale-signal correction: the row's `SCHEMA_VERSION=20` text predates the 21→22 bump; the
  constant read 22 at b01edcd). `done` reserved for the commit gate, house precedent.
- rm-501 minted (Linux-only test execution / CI truthfulness); compound note: not
  autonomously completable in-cycle (needs push+CI, prohibited stages) — first-in-line when a
  push-enabled gate runs; corrects the roadmap attempt's provisional "cheap lead" framing.
- Folds: rm-007 (upstream frozen + dependabot #309), rm-232 (pi parentSession detector-only),
  rm-196 (assess PoC upgrade of the acceptance test to user-reproducible).
- Cedes: F1→rm-496, F2→rm-497, F3→rm-437 (with pi v2/v3 spec cross-ref riding its
  acceptance), C1→rm-436 (our 1c1027cd PoC is the expected RED baseline for its landing),
  C4→rm-455.
- Wall at compound: 163 unique ids, 0 duplicates, footer marker intact (static checks below).

## Prevention rules (fleet-lettered, continue from PR-O — run16bbd3ae)

- **PR-P — cache-freshness-term sweep is cross-crate.** Any new term added to a cache
  freshness key (fingerprint or schema) invalidates every test fixture that hand-plants a
  warm cache entry, in every crate — a crate-scoped green run does NOT clear the change.
  This cycle: only the full-workspace gate run caught the TUI
  `ctrl_r_force_reload_clears_session_cache_before_loading` fixture (tests.rs:1647 planted an
  unstamped sessions.json entry; rm-196's fingerprint correctly refused it, left: 0 right: 1).
  Second occurrence of the class (first: the 21→22 schema-bump residuals — TUI fixture +
  discovery-contract pins + guide sentence, see CHANGELOG "residual references were aligned").
  Rule: when adding a freshness term, grep ALL `store_session(`/`cached_session(`/snapshot
  plants under `**/tests*` and run the workspace suite before declaring green.
- **PR-Q — freshness-policy moves don't bump the schema; parsed-semantics moves do.** rm-196
  stamped a `serde(default)` fingerprint under the existing schema 22: absence never matches
  a live identity, so legacy caches re-price exactly once and then behave stamped
  (dir-listing-version convention). Contrast rm-230/400/401 bumps, where reported totals for
  unchanged files would otherwise stay wrong. Deciding axis: "would a stale cache serve a
  DIFFERENT NUMBER or label than a fresh parse?" — if only cache policy moved, no bump.
- **PR-R — cache-identity fields are computed, never trusted from the artifact.** The
  fingerprint is derived from live `stat()`s (≤2 per scan); the journal copy is only ever
  compared, never parsed back — reading identity from the artifact is precisely where
  staleness would hide. Also: include only inputs that can CHANGE the answer (catalog
  mtime/size, override identity), and deliberately exclude inputs that cannot (stale-label
  flips at unchanged prices) — over-invalidation silently reverts warm caches to cold.
- **PR-S — sub-noise deltas get construction bounds, not host benchmarks.** Run-to-run
  variance on this host is 129–1010 ms for 250-session scans and interleaved ABBA medians
  sign-flip with ordering (321.5 vs 364.5 ms). A delta bounded by construction (≤2 stat() +
  one short string compare) is evidenced by the bound + a cache-hit line ("Parse: 250/250,
  250 cache hits", journal mtime unchanged), not by a benchmark slot. Corollary: perf
  acceptance criteria for cache-path changes should demand the bound + the hit-line, not a
  "before/after benchmark" that noise will dominate.
- Digest mechanics corroborated (no new letter; extends the run16bbd3ae digest-mechanics
  note / its review-fix addendum bb5bb645): the gate envelope's `digest` field derives with `digest_base: "unknown"` whenever the authoritative
  command omits `--digest-base-sha` — it is NOT comparable to the work-order token; the
  fold-relevant value comes from `validation_digest('<base_sha>', '<worktree>')`. Operationally:
  re-derive at declaration time (both validation turns did; token
  `a3ccc3bb…ddacf` byte-identical to dispatch with the whole delta uncommitted — the engine
  digest tracks committed state, so it legitimately moves only at the commit gate).

## Residuals / open scope (recorded, none begun)

1. `scripts/ci/check-docs-commands.sh` requires `target/release/agenttrace` (no release build
   in this worktree); with `AGENTTRACE_BIN=debug` it hangs scanning the real HOME corpus.
   Pre-existing, unowned by any row: needs a release build or an isolated-HOME fixture before
   it can run in a delegate worktree. Candidate cheap rider next cycle IF a release build
   lands (prioritize found every current cheap candidate reserved or governance-flavored).
2. rm-501 blocked on push+CI stages (above).
3. rm-196 review-time option (deferred to the review gate by implement 5f3a9aab): an
   `update_pricing()` → warm-invalidation note in docs if the reviewer wants the refresh path
   called out (by construction, update_pricing rewrites the cached catalog → new mtime/size →
   new fingerprint → re-price happens automatically).
4. No status flipped past `implemented`; `done`-flips reserved for the commit gate.

## Next-cycle context (from prioritize b6873c80 deferral queue, un-run)

1. rm-421 (90.0) standing LEAD (c3 selection note on row) — watch sibling 6557b823's rm-413
   fixture for compose; 2c2db6f5 moved rm-346 → rm-372.
2. rm-251 (90.0) blocked on prohibited test stage; rm-195 (90.0) decision-gated on problem
   acceptance; rm-239 (87.0) blocked on doc-command sweep.
3. rm-231 (88.0) compose with 71a666e8 numerator lane.
4. **rm-176 snapshot automation now UNBLOCKED** (sequencing required rm-196 first — done
   pre-review); rm-175 same family (etag plumbing, now unblocked pre-review).
5. rm-501 when a push-enabled gate exists; rm-500 belongs to sibling a6bb3c89 (live claim);
   this run's rows renumber-by-TITLE if sibling bands land first.
6. rm-436 (6403d975, unlanded) landing check: our research PoC
   (1c1027cd-scratch/poc/pi-usage-cache-warm.jsonl) is the expected RED baseline.
7. Backlog disposition pass ≤84: rm-053/085/014/011/367/402/164/036.

## Integration handoff (for the review/commit gates)

- Expected changed set at handoff: the 8 paths in "State at compound".
- ROADMAP.md in-tree delta = roadmap patch 6a852769 APPLIED at compound + the compound
  appends; cumulative diff mirrored at
  `b775d339e84d47fabda8351fe899a638-scratch/roadmap-2026-10-05-6a852769+compound.patch`.
  Do NOT `git apply` the 6a852769 patch onto this tree (already applied) or 6dccab2c's
  (superseded; its row is rm-501 here).
- Digest: pre-review token `validation:v1:a3ccc3bb76150bf5136846c745a7709dae3e02a745a0e5a161b8366b083ddacf`
  (declared by both validation turns, re-derived identical). At commit, re-derive and declare
  the post-commit token per the 6557b823/16bbd3ae precedent.
- Validation outcome for the fold (pre-review, recorded): authoritative gate
  `local_validation_gate.py --shell-command 'cargo test'` rc=0, 384 passed / 0 failed,
  envelope `result-553352-369538199.json`; earlier same-command run in targeted_tests rc=101
  → fixture fix → rc=0 384/0 (`/tmp/at-targeted-gate-10b7a582*.log`, mirrored).
- Fleet collision: pricing_fingerprint/freshness region audited region-disjoint from siblings
  run-32192d92/2d37535d/e43bb8f3/e602bb69 (b6873c80); no schema bump, so no schema-numeral
  collision with rm-449's schema/qwen lane at 2d37535d — coordinate only if that lands first.

## Verification performed at compound (static — no tests run)

- `git apply` 6a852769 patch → `git hash-object ROADMAP.md` = `e0ef6beb…` (roadmap phase's
  round-trip hash) before appends; after appends: id census 163 unique / 0 duplicates
  (both id formats), footer `<!-- managed by hermes-roadmap render; do not edit by hand -->`
  intact at EOF.
- `git status --porcelain` = exactly the 8-path set above; `git diff --numstat ROADMAP.md`
  recorded in the cumulative patch; no other tracked file touched by compound (CHANGELOG.md
  left as implement/targeted_tests wrote it).
- CHANGELOG already carries the rm-196 entry incl. the fixture-alignment sentence — no
  compound change needed there.

*Evidence mirrored per PR-J: gate logs, RED outputs, and digests copied under
`b775d339e84d47fabda8351fe899a638-scratch/evidence-mirror/` (see its manifest.txt).*

## Completion note (attempt `40b6838f`, re-dispatch)

The engine left attempt b775d339's result JSON (`b775d339….json`, written 12:22Z, status
succeeded) unread and re-dispatched compound — the same unread-result pattern this run's
roadmap phase hit with 6dccab2c. Attempt `40b6838f1d5c49cc821bb83ad809eb61` ADOPTED the
b775d339 end-state unchanged and re-verified it live (static checks only — no tests run,
per the compound no-execution constraint):

- `git rev-parse HEAD` = `b01edcd` (no commits made); `git status --porcelain` = exactly the
  8-path set above; `git diff --check ROADMAP.md` clean.
- Id census, both id formats: 163 total = 163 unique, 0 duplicates, max `rm-501` (next free
  rm-502); footer marker intact at EOF. `rm-196` at ROADMAP.md:673 reads `status:
  implemented`; `rm-501` at :1317; both `compound c1 2026-10-05` addenda present.
- Cumulative patch `roadmap-2026-10-05-6a852769+compound.patch` reverse-applies clean
  against the live tree (`git apply --check -R` → CLEAN) and live `git diff --numstat
  ROADMAP.md` = 22/−1 — the tree diverged from b775d339's end-state in ROADMAP.md by zero
  bytes.
- CHANGELOG.md untouched by compound (diff still carries the complete rm-196 entry incl.
  the TUI fixture-alignment sentence, exactly as implement/targeted_tests wrote it).
- PR-letter lineage: run16bbd3ae's record carries PR-G..PR-O, so PR-P..PR-S here continue
  the sequence without collision.
- Evidence mirror (b775d339-scratch/evidence-mirror/, sha256 manifest) present intact.

This re-dispatch changed only this file (three provenance amendments: header, phase-table
row, this section). Verification log:
`40b6838f1d5c49cc821bb83ad809eb61-scratch/compound-verify-2026-10-05.log`.

## Completion note (attempt `e7e056ce`, second re-dispatch)

The engine left attempt 40b6838f's result JSON (written 13:15:56Z, status succeeded) unread
as well and re-dispatched compound a third time. Attempt
`e7e056ce1ac042ff86fb131426cb583c` adopted the end-state unchanged and re-verified every
claim live (static checks only — no tests or validation commands, per the compound
no-execution constraint):

- `git rev-parse HEAD` = `b01edcd` (no commits made); `git status --porcelain` = exactly the
  8-path set above — byte-identical to 40b6838f's verified end-state; a
  `find -newermt 2026-10-05\ 13:15:57` sweep over the worktree (target/ and .git excluded)
  returned EMPTY: nothing moved between the prior verification and this one.
- Id census (both id formats, memory #16141 rule): 163 total = 163 unique, 0 duplicates, max
  `rm-501`, footer marker intact at EOF. `rm-196` at ROADMAP.md:673 reads `status:
  implemented`; `rm-501` at :1317 `status: candidate`; both `compound c1 2026-10-05`
  addenda present (2 hits).
- Cumulative patch `roadmap-2026-10-05-6a852769+compound.patch` reverse-applies CLEAN
  (`git apply --check -R`) — the live ROADMAP delta equals the recorded cumulative patch;
  `git diff --numstat ROADMAP.md` = 22/−1; `git diff --check` clean.
- CHANGELOG.md untouched by compound (full rm-196 entry incl. the TUI fixture-alignment
  sentence, as implement/targeted_tests wrote it); evidence mirror + sha256 manifest intact;
  implement artifacts confirmed in place by inspection (`pricing_fingerprint()` at
  pricing.rs:247, lib.rs re-export, 29 fingerprint references in session_cache.rs,
  tests/warm_cache_pricing.rs) — consumed as recorded evidence, not re-run.
- PR-letter lineage re-checked: run16bbd3ae carries PR-G..PR-O; this record PR-P..PR-S — no
  collision.

This attempt's delta is again provenance-only, this file (header, phase-table row, this
section). Verification log:
`e7e056ce1ac042ff86fb131426cb583c-scratch/compound-verify-2026-10-05.log`.

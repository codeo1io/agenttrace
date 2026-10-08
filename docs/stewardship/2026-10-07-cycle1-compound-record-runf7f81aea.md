# Cycle 1 compound record — run f7f81aea (repository-maintenance 8decedb7)

Recorded 2026-10-07 by the compound phase (attempt 511d2386add24864adb4c095c131f0c5)
from pre-review cycle evidence only: assess dispatches 3541 / 24a3 / bd9d (three adversarial
passes at wall 518170a7), research 5798 (network live), roadmap 3ee2af04, prioritize
001bc29a, stewardship feb3c320, implement (delta adopted from reaped attempt 69ff5c75 after
red-first verification), targeted_tests c237bf0e, full_tests 63029d0c. No validation was
re-run at compound (phase contract); every outcome below is the recorded pre-review outcome
of the cited phase.

## Batch — "Report truthfulness & honest surfacing" (truthfulness arc round 3)

- **rm-693 (LEAD, correctness 74.0)** — streaming re-emissions counted once, not once per
  block (`crates/agenttrace-core/src/parser.rs`): the rm-601 usage fold kept usage-max but
  every NON-usage surface still multiplied per re-emission (turns, tool calls/results,
  `session_end`/duration frozen at the first emission). The fold is now snapshot-based per
  message id — assistant event replaced in place by the richest snapshot, usage-slot
  timestamp advances to the final emission, tool results dedupe per
  (message id, tool_use_id); id-less rows keep legacy behavior. False `[P1] slow-tool` /
  `[P2] retry-loop` anomalies on the PoC corpus are gone (`No anomalies`). Session-cache
  schema 32 → 33 per the rm-230 convention (corrected turns/tools/spans must invalidate
  warm caches). NEW `tests/truthful_streaming_and_windows.rs` (331 lines; 8 test fns, 15 fns total — review-round recount: 381 lines; 10 test fns, 17 fns total) +
  `fixtures/usage-accounting/claude-stream-with-{content,tools}.jsonl`, red-first (5 core
  failures at base).
- **rm-694 (correctness 70.0)** — `--range` membership by activity overlap, not start:
  discovery's since filter and `insights::session_matches_time_range` share one rule
  (admit when last known activity — `session_end` where seen, else start — falls in the
  window; unknown-time sessions stay visible); scope window end and
  `data_health.latest_session_at` report last activity; `earliest` stays start-based. The
  overnight corpus (start 23:30Z prior day) now reports `Total Sessions: 1` under
  `--range today` with window `to 2026-10-07T01:00:00Z` where the base bailed rc1
  no-match. README's `--range` sentence is activity-based; `--range all` byte-identical
  pre/post on non-snapshot corpora. NEW `fixtures/window-membership/` (3 corpora).
- **rm-695 (upstream-contributable 66.0)** — discovery no longer swallows parse failures
  into the misleading no-match bail: `LoadReport` carries `parse_failures` +
  `first_parse_failure`; the empty-set bail differentiates exhaustion
  (`No sessions parsed: all N discovered session files failed to parse (first: …)`) from
  filters (`No sessions match the requested filters (N of M discovered files failed to
  parse; first: …)`); partially-unreadable corpora print a one-line stderr advisory at
  load; `--doctor`'s found/parsed/failed split unchanged. The zstd decompress hint now
  surfaces through the directory lane rc1. Upstream carries the identical `.ok()` at
  `discovery.rs:232` — the counter+message contribution ports cleanly. NEW
  `fixtures/discovery-failures/zstd-rollout.jsonl`, red-first (3 CLI failures at base).
- **rm-696 (usability 58.0)** — `--demo --preserve-history` refused loudly before any
  load (`--demo cannot be combined with --preserve-history: demo sessions are ephemeral
  samples and never enter durable history`), mirroring the `--baseline`/`--compare`
  conflict rule; each flag alone keeps documented behavior; README documents `--demo` as
  side-effect-free. Base behavior banked 3 records / $0.8106 of fiction per demo run and
  then reported 4 sessions / $0.8167 under `--include-history` vs the 1 / $0.0061
  control.

## Recorded outcomes (pre-review)

- **targeted_tests c237bf0e** — core 379/0, cli 112/0, tui 47/0; `cargo fmt --all
  --check` rc0; clippy `--workspace --all-targets --locked -D warnings` rc0;
  `scripts/ci/check-docs-commands.sh` rc0; the dispatch digest was re-derived live and
  reproduced byte-identical (surface set untouched).
- **full_tests 63029d0c** — the work order's `full_command` was EMPTY (the crates/**
  classifier artifact; the engine skips the command-match clause at
  `validation_policy.py:1458`). Standing convention applied (4th recorded occurrence;
  precedents ff0068ca/53c9af4d, 7eae74ea, 605c): the ci.yml lint+full+deny envelope was
  mirrored lane-for-lane — 21 lanes ALL rc0; cargo test 538/0 summed; entrypoints 33/0;
  npm 4/4; homebrew ruby Syntax OK; cargo-deny advisories/bans/licenses/sources ok;
  plugin-version v0.9.0 == CHANGELOG. Porcelain sha `e5b93aa4…` and git-diff sha
  `c0fed7d4…` identical pre/post.
- **Conductor validation digest** — `validation:v1:79d4ac01594d645ca572df9a4a34d986a15f4d05010a0d0aab5f41e669ab06ec`
  (base 518170a7), derived twice with identical result at both validation gates; compound
  adds no tracked-file change, so it stays current. Record this digest verbatim in the
  shipping PR.

## Prior-attempt forensics (both directions of the rule)

- **Adopted (implement phase)** — dead attempt 69ff5c751e9b (provider-failure reap
  04:04:21Z, 86 event-log messages, NO typed result, NO scratch dir) had written a real
  diff 02:28–04:03 that was mid-edit and did not compile (stray `(` in a channel
  turbofish, undeclared parser import). Adopted only after full first-hand verification:
  `git stash push` of the three core files reproduced 5 red-first failures with exact
  wrong-value assertions, and an impl-hunk revert (tests intact) reproduced 3/3 CLI
  failures — the tests were real; the delta was then repaired to compile and re-verified.
- **Redone (this compound phase)** — dead attempt a99e9823 (provider reap at ~170 s, 4
  progress pings, NO scratch dir, NO typed result) left no durable trail; phase redone
  from scratch. **The discriminator is the durable trail (scratch dir / typed result /
  file mtimes), never the event-log message count** — 86 messages hid a compilable
  near-fix, 7 messages can hide anything, and neither number proves work.

## Prevention rules recorded this cycle

- **PR-A (schema-bump same-unit checklist — first-try pass)** — the 32→33 bump moved
  the const (`session_cache.rs:8`), the TUI warm-cache fixture literal
  (`agenttrace-tui/src/tests.rs`, red at 32 as 46/1 `cache_hits`), the
  governance-guide sentence (`docs/guides/governance-reports.md:72`, grepped by
  check-docs-commands.sh) and the CHANGELOG clause in ONE unit. The class struck three
  times on earlier cycles (governance sentence lagging); the checklist pre-clears it and
  the rm-230 rider (cargo-test-enforced guide sentence) mechanizes it permanently.
- **PR-B (dead-attempt durable-trail rule)** — see forensics above; adopt only after
  red-first verification of the claimed work, never on message count or mtimes alone.
- **PR-C (empty full_command → ci.yml mirror)** — recurring classifier artifact; the
  lane-for-lane ci.yml mirror is the standing convention, now with four recorded
  precedents. Cite the workflow file as the command authority in the PR.
- **PR-D (live re-pin discipline)** — dossier line refs drifted within a day
  (`main.rs:1131` → live `:1129-1133`); every phase re-pins the surfaces it cites with
  sed/grep against the tree, not dossier recall.
- **PR-E (census at mint)** — re-verify the def-row census live at every mint (phantom
  duplicate trap on free-text `rm-NNN` mentions; non-ascending wall bodies). This run's
  mints went above every swept sibling claim (rm-685..692), landing rm-693..698 with
  next free rm-699.

## Next-cycle leads (concrete)

1. **rm-697** — `sessions.json` lock-less read-modify-write (last-writer-wins whole-map;
  `session_cache.rs:695,1258-1312`): advisory flock + a two-process interleaving test.
  Every new writer (`--latest`, doctor, audit) widens the window.
2. **rm-698** — `presentation.rs` 3,899-line monolith split behind a facade, gated on
  byte-identical output over the landed fixture corpus; the render seam is where
  rm-601..603 proved regressions are expensive.
3. **rm-449 (benched)** — plausibly still open (`sessionId` grep parser.rs:1037/1274);
  next assessment adjudicates against current acceptance text.
4. **rm-383 (OSC-52)** — unchanged fleet-strongest prior lead.
5. **Riders to carry** — rm-176 (9.53% catalog drift in 69 h; variant-coverage class,
  keyed flagships exact), rm-401 (codex alphas shipping daily; zstd handling stays
  fork-owned), rm-509 + rm-017 (bare `agenttrace` npm name still 404/free; squatter
  precedent `claude-code`).

## Commit-gate checklist

1. Stage the 14 batch paths (9 M: CHANGELOG.md, README.md,
   crates/agenttrace-cli/src/main.rs, crates/agenttrace-core/src/{discovery,insights,parser,session_cache}.rs,
   crates/agenttrace-tui/src/tests.rs, docs/guides/governance-reports.md; 5 ??:
   crates/agenttrace-core/tests/truthful_streaming_and_windows.rs,
   fixtures/discovery-failures/, fixtures/window-membership/,
   fixtures/usage-accounting/claude-stream-with-{content,tools}.jsonl).
2. Apply the ROADMAP chain in order on clean HEAD ROADMAP.md:
   `roadmap-3ee2af04.patch` → `roadmap-compound-511d2386.patch` (both verified
   byte-identical via tempdir apply), then optionally the new-file patch for this record.
3. Done-flips stay reserved to the gate (rm-012 convention) — the four rows land as
   `implemented` with their EXECUTED/VALIDATED lines.
4. Sibling seams, reconcile BY TITLE at integration: rm-685/rm-686 are double-minted
   (2f02ecaf codex-ledger/gemini-alias vs 8e983cf5 history/statusline); rm-693's notes
   cross-reference unlanded rm-687's copilot fold — implement the shared per-message-max
   helper once if both land.
5. Record the digest above verbatim in the shipping PR; review/CI outcomes are carried
   forward by the NEXT cycle's assessment, not by this record.

## Review fix round (independent_review 6845bdf6 -> fix 0772a0ca, 2026-10-07)

The independent review returned NEEDS_CHANGES with four findings; all four
were fixed and the affected validation rerun (targeted scope —
`required_scope: none` in the dispatch block, no executable-classified
script/CI surface touched this turn, digest re-derived live and UNCHANGED:
`validation:v1:79d4ac01594d645ca572df9a4a34d986a15f4d05010a0d0aab5f41e669ab06ec`).

- **HIGH — `DataHealth.latest_session_at` still start-basis** (insights.rs:678):
  the rm-694 acceptance and this record claimed activity basis, but only
  `report_scope` had been moved. Fixed: `data_health_from_parts` now uses the
  shared `session_last_activity` helper (same basis as the scope window end);
  field doc added; pinned by `data_health_latest_session_at_uses_last_activity`
  (same overnight corpus). Live proof: the overnight corpus's
  `--overview -f json` now shows `data_health.latest_session_at ==
  2026-10-06T01:00:00Z == scope.latest_session_at` (previously the health
  block said the 23:30Z start while the window ended 01:00Z).
- **MEDIUM — README never documented `--demo` as side-effect-free** (rm-696
  acceptance text): added the `--demo` comment block to the usage examples
  (nothing written to history.json or the session cache; pairing with
  `--preserve-history` refused). `scripts/ci/check-docs-commands.sh` rc0.
- **MEDIUM — tool_result dedup arm executed zero times under test**
  (parser.rs:3477-3497): new fixture
  `fixtures/usage-accounting/claude-stream-toolresult-reemit.jsonl` re-emits a
  tool_result under a repeated message id (same tool_use_id, enriched second
  snapshot) plus a distinct second result; pinned by
  `tool_result_reemission_counts_once_per_message_and_use` (turns 2, tool_results 2
  — deleting the dedup makes it 3). Counts are the observable surface: Metrics
  aggregates events, so the surviving snapshot's content is pinned structurally
  (first snapshot inserts the tool event; re-emissions skip).
- **LOW — this record said "15 test fns"** where the file had 8 `#[test]`
  fns (15 fns total): corrected below.

Retest (targeted, this round): core 381/0 (was 379 + 2 new tests), cli 112/0,
tui 47/0, `cargo fmt --all --check` rc0, clippy `--workspace --all-targets
--locked -D warnings` clean, `check-docs-commands.sh` rc0, release binary
rebuilt and the live overnight-corpus check above. Test file now 381 lines /
10 test fns (17 fns total). Worktree porcelain after this round: 15 entries
(9 M + 6 `??`, the new `??` being the reemit fixture). Chain extended by
`roadmap-reviewfix-0772a0ca.patch` (riders on the rm-693/rm-694 rows); this
record (r2) supersedes 511d2386's `compound-record.new-file.patch`.

## Status accounting / worktree census at compound end

ROADMAP wall after the chain: 230 def rows = 127 candidate / 60 implemented / 43 done;
ids unique; 244 unique headings; managed footer last line. Worktree porcelain unchanged
by compound (spool-side hygiene): exactly the 14 batch entries above; HEAD 518170a7.

# Cycle 2 compound record — run 415d77e1 (repository-maintenance 224c5cf1)

Recorded 2026-10-08 by the compound phase (attempt 7caf7f055714401ca77dc6ff48faf832)
from pre-review cycle evidence only: assess 849d4b1a, research 6cf13a6b, roadmap
4b1a265e (re-delivery of first-mint 3755c7e2), prioritize f528235b, stewardship
09dcdf66, implement 1fa165e9, targeted_tests 4f94b395, full_tests 4d005457. No
validation was re-run at compound (phase contract); every outcome below is the
recorded pre-review outcome of the cited phase. Review and shipping outcomes land
after this phase; the next cycle's assessment carries them forward.

## Batch — "Statusline payload measured-truth & prompt correlation"

- **rm-037 (LEAD, customer-experience 78.0)** — the Claude Code statusline payload
  schema catch-up shipped on the extraction→insights→report surface at base 61570ea,
  as the uncommitted 5-file worktree delta {CHANGELOG.md,
  crates/agenttrace-core/src/lib.rs, statusline.rs, tests/otel_export.rs,
  docs/guides/statusline-capture.md}:
  `context_window.{used_percentage,remaining_percentage,current_usage}` renders per
  session (latest capture per `session_id`, newest session first; sessions without the
  arm stay absent rather than zero-filled) with peak utilization across captures and
  the last-API token counts (`input`/`output`/`cache-write`/`cache-read`); the gateway
  `rate_limits.spend_limit` window renders alongside the two subscription windows on
  the Limits line (latest + peak) plus a USD detail line (`used_usd`/`limit_usd`/
  `period`/`resets_at`, each field printing only when the gateway sent it — the USD
  trio rides a ~5-minute schedule and stays absent under
  `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`) and joins the reset-crossing scan on the
  same epoch mechanism. New structs `StatuslineContextUsage`,
  `StatuslineContextWindow`, `StatuslineSpendLimit` + `CapturedStatusline` accessors;
  `--statusline-report` text and json both render the new arms; a legacy pre-v2.1.251
  journal renders unchanged with no phantom arms, and unknown future payload fields
  are tolerated, not rejected — both pinned by test. Deferred in-row by prioritize
  (still open): `workspace.repo` identity feed (insights.rs 3-lane contention),
  `fast_mode`/`thinking.enabled`/`pr.*`/`effort.level` arms, one-line status render
  arms (must compose with aa31de94's unlanded rm-165).
- **rm-708 (MEMBER, capability 61.0)** — `prompt_id` capture + correlation:
  `StatuslineInsights.prompt_ids` (latest per session, newest first), the
  `StatuslineJournalStats` classifier (`captures_with_prompt_id` /
  `captures_pre_first_input` / `captures_unknown_payload` — absence disclosed as the
  documented pre-first-input state, never an error), and the capture-side half of the
  statusline↔OTel correlation as the pure
  `agenttrace_core::correlated_prompt_ids(captures, otel_doc)` join over OTLP-JSON
  `prompt.id` span attributes, pinned end-to-end against a real `report_otel_export`
  document. Remaining scope: the emitter-side attribute (span exporter emitting
  `prompt.id` for statusline-derived spans) lands with the later OTel arm.

## Recorded outcomes (pre-review)

- **targeted_tests 4f94b395** — statusline mod 23/0 (new pins
  `statusline_report_gains_context_spend_prompt_arms_{json,text}`,
  `statusline_insights_extracts_context_spend_prompt_arms`,
  `statusline_journal_stats_classifies_prompt_key_presence`), otel_export 5/0, core
  lib 214/0, `cargo fmt --check` clean, `cargo clippy --workspace --all-targets`
  0 warnings, check-docs-commands rc0. Dispatch validation block resolved live:
  `changed_testable_surfaces=[]`/`required_scope='none'` is the crates/** classifier
  artifact (verified against the deployed engine's `changed_surfaces()` — every
  delta path is non-executable), so the implement-declared lanes ran instead; digest
  replica `validation:v1:d2b35fb3c8b92a6ffa71f7c51a0a4501766f351640207f73310a81f623a93cbc`
  == dispatch token byte-identical; mtime invariance across all 5 delta files proves
  the validation turn wrote nothing.
- **full_tests 4d005457** — 22/22 lanes rc0 (heavy cargo lanes through the conductor
  local_validation_gate); 558 passed / 0 failed across 26 ok result lines = the
  549-test assess-849d4b1a baseline at the same HEAD + exactly the 9 net-new batch
  tests (+9/−0 `#[test]` markers on the diff census); per-target core lib 214/0,
  core all-targets 395/0, otel_export 5/0, cli bins 60/0, entrypoints 34/0, tui
  47/0; tree immobility pre==post (diff sha256 `0027ec35…`, porcelain sha256
  `c53877c7…`); live digest re-derivation with the engine's own
  `validation_policy.validation_digest` MATCH. Log-genuineness anchors recorded
  (cargo-deny all-ok, plugin-version v0.9.0, real-cli-smoke `sampled_files=20`).
- **RED-first (implement 1fa165e9)** — pristine base runtime red: 15 passed / 2
  failed rc101; tests-only hybrid compile red: E0560/E0609/E0432 (missing
  `captures_with_prompt_id`/`captures_pre_first_input`/`captures_unknown_payload`/
  `context_windows`/`correlated_prompt_ids`); stash restore proven byte-identical.

## Attempt trail & transport forensics (nothing adopted; each verified against tree + spool)

1. **roadmap 3755c7e2** (2026-10-07 00:33Z) wrote its result JSON but the session died
   before engine consumption → re-delivered VERBATIM as **4b1a265e** with the
   numerals census refreshed to the live frontier; diff of the two patches = 3
   attribution/census lines only. Not reminted — reminting would orphan the
   cross-cites the rows already carry.
2. **full_tests 45db0d68** provider-reaped ~3.5 min in (8 messages): typed artifact
   absent, scratch empty, `/tmp/at-full-45db` holds only an empty `ci-artifacts/`
   mkdir → phase redone as **4d005457**.
3. **compound be72a06d** (this phase's prior attempt) provider-reaped across two
   aborted turns (event log 7 + 2 messages, `session_reaped` exit_reason failed):
   no scratch dir, no result JSON, worktree porcelain verified = exactly the
   5-file implement delta → phase redone from scratch as **7caf7f05** (this record).

## Lessons & prevention (no new solutions doc minted)

Every reusable lesson this cycle folds onto an existing artifact, so per the
ce-compound precondition ("write nothing and report why" when nothing qualifies as a
new standalone learning) no `docs/solutions/` doc was minted:

- **Validation-block emptiness for crates/** deltas** — already the recorded L5 class
  (2026-10-05 lessons block, run 4a688257): `required_scope='none'`/empty commands
  mean the classifier saw no executable surface, not that no validation is owed.
  This run's contribution is the resolution recipe, now also on the banner: consult
  the deployed engine's `changed_surfaces()` live, run the implement-declared lanes,
  and re-derive the digest with the engine's own `validation_policy`.
- **Reap-after-artifact recovery discipline** — a written-but-unconsumed result is
  re-DELIVERED with identical content and refreshed attribution, never reminted
  (3755c7e2→4b1a265e); a reap with zero durable trail is a clean redo (45db0d68,
  be72a06d). Fleet-pinned practice; recorded here with this run's three instances.
- **Live-docs schema oracle** — the 84-key extraction from the fetched
  docs.anthropic.com statusline.md (vs the bundled test surface) is rm-708's own
  evidence pattern and the standing recipe for payload-drift rows; no separate doc
  adds value over the row + dossier.
- **Vocabulary** — no new CONCEPTS.md terms: the cycle's vocabulary ("payload arm",
  "correlation key") is already carried by the rows and the landed docs table.

## Deferred queue / next-cycle leads (priority order)

1. rm-037 deferred arms: `workspace.repo` identity feed (insights.rs 3-lane
   contention), `fast_mode`/`thinking.enabled`/`pr.*`/`effort.level` capture,
   one-line status render arms (compose with aa31de94's unlanded rm-165).
2. rm-708 emitter-side OTel arm: the span exporter emits the `prompt.id` attribute
   the capture-side join already consumes; landing it should also wire the report-side
   correlated-events count the row's acceptance names (explicitly deferred with this
   arm — review 537105b1 F3; today `correlated_prompt_ids` is library-only).
3. Pricing snapshot staleness: bundled 3,100 rows pinned 2026-10-04 vs 4,481 live —
   the rm-006/rm-176 automation trigger fires on key-delta, not row-count; diff the
   keys, not the count.
4. Upstream quiet (tip 15ed07f, fork 30 behind, newest release v0.10.1, open set 5:
   #278/#259/#237/#236/#103) — rm-042/rm-045 port wave and the #103 provenance
   fork-ahead remain the fork-merge-scale decisions.
5. Morning-research a21dbcda candidates C1–C6 stand unclaimed.

## Commit-gate seams

- **Patch chain (this dispatch's hygiene clause keeps the worktree clean):** apply
  `/home/agent/.hermes/conductor-delegate-spool/delegate/4b1a265ede334bb1b3f2aa0e7faf2730-scratch/roadmap.patch`
  then
  `/home/agent/.hermes/conductor-delegate-spool/delegate/7caf7f055714401ca77dc6ff48faf832-scratch/compound.patch`
  then
  `/home/agent/.hermes/conductor-delegate-spool/delegate/e7cc02cfb7e346778347da96fc7ea0d3-scratch/roadmap-reviewfix-e7cc02cf.patch`
  (review-fix rider, independent_review 537105b1 NEEDS_CHANGES -> fix e7cc02cf: the rm-037
  review-fixed evidence line, the rm-708 explicit report-count deferral, and THIS record's
  chain/lead updates — the rider edits ROADMAP.md and this record in place)
  to ROADMAP.md (round-trip from the 61570ea base ROADMAP.md verified
  byte-identical to the r2 postimage incl. the rider); the compound patch also CREATES this record at
  `docs/stewardship/2026-10-08-cycle2-compound-record-run415d77e1.md` — stage it
  explicitly, a `git commit -am` would drop it (250cfd64 precedent).
- **One commit for the whole batch:** the 5 implement files + ROADMAP.md (chain
  postimage) + this record. Done-flips for rm-037 and rm-708 AT THE GATE, by title.
- **Digest:** ROADMAP.md/docs/.md edits are digest-immobile under the crates/**
  classifier — `validation:v1:d2b35fb3c8b92a6ffa71f7c51a0a4501766f351640207f73310a81f623a93cbc`
  stays current for the 5-file delta; re-derive live at final_validation.
- **Numerals:** zero mints at compound; frontier unchanged (rm-721 held, next free
  rm-722 per the roadmap banner). Sibling bands rm-693..707 / rm-709..721 are
  title-disjoint from this batch (prioritize f528235b contention map); re-verify at
  the gate.
- **Status accounting (compounded postimage):** 139 candidate / 48 done / 64
  implemented (`grep -oE 'status: [a-z-]+'`), def rows unchanged (zero mints),
  footer last.

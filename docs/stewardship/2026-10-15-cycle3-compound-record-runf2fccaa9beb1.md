# Cycle 3 compound record — run f2fccaa9beb1426b9d92085c783e8cfa (repository-maintenance 9f95cf5f15044e309c2187fd75b8eb2a, cycle 3)

Written at compound attempt 194456fcdcd64a28b8381f3fc208f223 (redo of transport-dead ea8f85b3: typed
result absent, event log = started + 1 progress + `session_reaped:provider` @ ~28 s +
completed-failed, no scratch, no /tmp trace, no tree drift — nothing durable, nothing adopted), at
worktree `run-f2fccaa9beb1-f2fccaa9` @ `3e90993`, **pre-review**, ZERO test execution (compound
contract — every outcome below is consumed from the recorded targeted/full evidence, not re-run).
Review and shipping outcomes land after this phase; the next cycle's assessment carries them.

Base wall at build time: `ROADMAP.md` 392 def rows, census 156 candidate / 4 done / 40 implemented /
192 in_progress, def-max `rm-1373`, md5 `1d59addaa633d721268bc6c222cdf03d` (== `git show
3e90993:ROADMAP.md`, resting wall). Batch tree at build time: 10 porcelain items, +562/−63
(the implement delta, byte-intact per targeted/full numstat verification).

## Batch outcome — 'honest dollars' (prioritize ca5a4697, stewardship 08f2ab25)

All three selected units implemented red-first and validated **uncommitted** in this worktree:

1. **rm-1053 regional-twin pricing** (PRIMARY; assess F1 HIGH). `convert_litellm` /
   `qualified_catalog_id` retains every region/vendor-qualified LiteLLM row under its own raw id
   ('/'-path deployment ids + dotted regional prefixes); the same keys still vote in the base
   collapse, so plain-id winners are unchanged. `resolution_ladder` gives rate / provider /
   pricing-source ONE shared raw-first ladder (exact id → lowercased → normalized/alias variants),
   closing both the load-time collapse and the normalize-of-a-normalize provider attribution.
2. **rm-1402 quota cutoff clamp** (PRIMARY; this run's roadmap mint; assess F3 PoC). `quota_cutoff`
   returns `(newest at-or-before-now instant, future_discarded_count)` with per-session rescue
   (earlier event ts, warm-cache `session_start`); `future_discarded_sessions` + amended
   `cutoff_basis` in both arms; text lane prints a clock-skew disclosure line.
3. **rm-339 partial arm — basis re-work** (SECONDARY; assess F11). `CacheEfficiency.hit_rate` is now
   the cache-INCLUSIVE `reads/(reads+fresh)`, bounded ≤ 100 % by construction;
   `hit_rate_exclusive_percent` in `agenttrace.waste.v1` discloses the raw exclusive ratio; the
   'anomalous' rating arm is RETIRED (it fired on the mis-read exclusive basis). This SUPERSEDES the
   e88f2da2 2026-10-08 corroborated arm; the CHANGELOG (already in the implement delta) carries the
   supersession entry.

## Validation outcomes (consumed, NOT re-run)

- **targeted bbe9fc46** (redo of reaped 0d8adf60): `regional_pricing_contract` 4/0 (incl.
  `bundled_catalog_sweep` — every chat-priced key bills at a rate that exists in some snapshot row,
  qualified keys at their OWN row + provider), core `--lib` 403/0, `pricing_flex_contract` 5/0,
  svg-card goldens 11/0, entrypoints full incl. `cli_quota_view_clamps_a_future_skewed_corpus` —
  **478 distinct tests**, fmt rc0, clippy `-D warnings` rc0 (logs t1..t8 in the phase scratch).
- **full 6a6a35f4** (run2, authoritative): `cargo test --locked -p agenttrace-core -p
  agenttrace-tui -p agenttrace` RC0, 63 `test result` lines, Σ **1127 / 0 / 0**. Run1's E0463
  ('can't find crate for serde/chrono' in doc-tests) under the inherited shared
  `CARGO_TARGET_DIR` is environmental, retained as evidence (see L3).
- **Release-binary PoCs** (implement 5d039e8d): `--test-match` renders us $3.30/$16.50 vs global
  $3.00/$15.00 vs us-gov + bedrock/us-gov-east-1 $3.60/$18.00, azure/us-gov gpt-5.1 $1.72/$13.75 —
  each at its own row; `--quota` on a 2-real-events + 2031-line fixture → cutoff clamped to the last
  real event (2026-10-12T10:30:00Z), `future_discarded_sessions: 1`, honest $4.003 weekly (pre-fix
  shape billed only the hostile line's $0.0045); `--waste` on 5K-read/10-fresh → 'excellent 99.8 %'
  inclusive with the 50 000 % exclusive ratio disclosed.
- **Digest**: `validation:v1:694c7e1f…` re-derived at targeted == dispatch value; full copied
  VERBATIM (zero executable delta after implement — ROADMAP/.md edits are digest-immobile).

## Red-first record

Implement's unfixed-base run of `regional_pricing_contract` failed 3/4
(`regional_twin_families`, `regional_provider_attribution`, `bundled_catalog_sweep`;
`base_model_pricing` PASSED — the no-collateral half already held). SVG goldens regenerated via the
documented `UPDATE_CARD_GOLDEN=1` lane moved ONLY the census footer line (4333 vs 1529 priced SKUs;
215 vs 61 past deprecation); per-model costs byte-identical.

## Reap ledger & provenance recovery (why part of this band is a rebuild)

The spool reaper swept, before compound: assess 53af6c9e evidence-bundle, research cd47277e dossier
(+ all `/tmp/at-assess-f2fcc*` / `/tmp/at-research-cd47*` mirrors), roadmap f406d0fb scratch
(ROADMAP.patch / postimage / APPLY-SEAM / pristine), prioritize ca5a4697, stewardship 08f2ab25.
Surviving: implement 5d039e8d (handoff.md + 15 logs), targeted bbe9fc46 (t1..t8), full 6a6a35f4
(run1/run2 logs). The roadmap band (mints rm-1401/rm-1402 + riders) therefore existed nowhere
durable; this compound REBUILT it from the work-order-embedded record (mints marked rebuilt,
acceptance re-anchored to live-verified seams) per the 13212e2f reaped-band precedent. The in-tree
band below REPLACES the reaped `f406d0fb/ROADMAP.patch` at the commit gate — apply nothing from
spool history.

## Durable lessons & prevention rules

- **L1 — evidence durability**: the spool reaper now sweeps CURRENT-run scratch within a day (all
  five earlier-phase scratches + every /tmp mirror of this run were gone by compound time; cycle-2
  L4 had established '/tmp < 1 h' — the spool is no safer). RULE: mirror every phase's durable
  evidence to `/home/agent/.cache/at-<phase>-<id>/` AT WRITE TIME; the work-order embedded summaries
  plus surviving sibling logs are the recovery substrate of last resort.
- **L2 — dead-attempt triage (3-for-3 this run)**: provider deaths with zero scratch = clean redo
  (roadmap 18f68fb5, implement 94ce75e7). BUT targeted 0d8adf60 was reaped at 18 messages WITH a
  real-looking E0432 red ('no `provider_for` in the root') that the live tree contradicts — the
  `lib.rs:64` re-export and `pricing.rs:439` definition predate the attempt. RULE: a reaped
  attempt's partial logs can be a FALSE red; verify against the live tree before treating as
  evidence.
- **L3 — build hygiene**: full run1 died E0463 in doc-tests under the inherited shared conductor
  `CARGO_TARGET_DIR`, run2 under a private target + private `HOME` + `TMPDIR=/tmp` went green
  63/1127/0. RULE (extends cycle-2 L5): latch private target + HOME + TMPDIR for EVERY execution
  phase.
- **L4 — fix pattern for catalog-wide changes**: retention + ONE shared resolution ladder, not
  collapse perturbation; plain-id winners are provably unchanged because the no-collateral test
  (`base_model_pricing`) passed at the unfixed base. RULE: split catalog tests into a
  no-collateral half and a flip half before touching the collapse.
- **L5 — numeral discipline**: fog postimage 978e457a (run bb80f0af) carries its OWN rm-1402
  ('quota cost-share basis') — a numeral COLLISION with this run's rm-1402 ('quota cutoff clamp').
  RULE: reconcile by TITLE at integration, never renumber (rm-708 convention).
- **L6 — orthogonality honored**: the clamp + basis rework landed with the session-cache `Metrics`
  schema UNTOUCHED (fog rm-1385 warm/cold attribution basis preserved) — the stewardship
  `must_remain_separate` hints held. Carry the same hint to whoever touches quota next.
- **L7 — goldens**: census-count-only golden drift is the cheapest byte-level no-collateral proof
  for priced-model changes (footer moved, per-model costs identical).

## Next-cycle candidates & context (pre-review)

- **rm-1401 Copilot depth** (minted candidate, the natural next lead): six-source usage hierarchy —
  session-store.db `assistant_usage_events` (cache-INCLUSIVE input, cumulative rollups), VS Code
  chatSessions delta journals (kind:0/1/2), Copilot Chat 0.69 modelTotals/copilotCredits billing,
  live quota endpoint with 5 h / weekly / model-specific resets; `discovery.rs:189-194` admits only
  `.copilot/session-state` + `.copilot/otel`. The per-lane input-basis decision (inclusive vs
  exclusive, disclosed, never silently mixed) is the acceptance hook.
- **Assess's remaining findings**: 11 findings total (2 HIGH); F1/F3/F11 were this cycle's inputs.
  The evidence-bundle was reaped — the next assessment re-derives the remainder at the landed base;
  do not treat the truncated work-order enumeration as the settled ledger.
- **Pricing-drift gate**: predicted RED ~2026-10-19/20 06:00 UTC on the fleet ledger. Snapshot
  refresh stays title-hot with unlanded sibling bands — reconcile by title first; the keep-filter
  basis must be RE-DERIVED (mode==chat AND costed AND no glob), not raw-key refreshed.
- **rm-339 MAIN acceptance** (cachemiss-parity rebuild-reason attribution + premium pricing +
  synthetic-fixture matrix) is untouched and remains the row's done-gate.
- Version axes were FLAT this cycle (claude-code 2.1.296/287/299, gemini-cli 0.64.0 /
  0.65.0-preview, codex 0.162.1 / 0.163-alpha.10, ccusage 20.0.31, opencode 1.19.0, models.dev md5
  `3d70bf7f…` unchanged) — no format-lane urgency beyond the standing watches.

## Commit-gate seams

1. This ROADMAP band REPLACES the reaped `f406d0fb/ROADMAP.patch` — the rebuilt mints + riders are
   in-tree; apply nothing from spool history.
2. Stage exactly: the 10 implement items (`CHANGELOG.md`, `README.md`,
   `crates/agenttrace-cli/tests/entrypoints.rs`, `crates/agenttrace-core/src/{lib.rs,pricing.rs,
   waste.rs}`, `crates/agenttrace-core/tests/fixtures/svg-card/demo-{auto,dark,light}.svg`,
   `crates/agenttrace-core/tests/regional_pricing_contract.rs`) **+** `ROADMAP.md` **+** this
   record.
3. Done-flips stay reserved to the commit gate (rm-012 convention). rm-1053 and rm-1402 are
   `implemented` pre-review here; their `done` flips (and rm-339's 'partial arm' done-flip) belong
   to the gate once review confirms.
4. Revert path: `git checkout -- ROADMAP.md` + delete this record (pristine md5
   `1d59addaa633d721268bc6c222cdf03d`; patch + pristine archived spool-side
   `delegate/194456fcdcd64a28b8381f3fc208f223-scratch/` and mirrored at
   `/home/agent/.cache/at-compound-194456fc/`).

## Census & verification legs

- Def rows 392 → 394 (2 mints REBUILT, marked); zero duplicate ids; def-max rm-1373 → rm-1402;
  managed footer last; census 156 candidate / 4 done / 42 implemented / 192 in_progress
  (was 156/4/40/192).
- Flips/riders: rm-1053 candidate → implemented (+ VALIDATED rider); rm-339 + basis-re-work rider
  (supersession disclosed); rm-1401 minted candidate; rm-1402 minted implemented.
- Postimage md5 `ee701793a7e91ac5aaa82fa3f3942959`; assertion-guarded builder
  `build_compound.py` (spool scratch) re-runnable against the archived pristine.

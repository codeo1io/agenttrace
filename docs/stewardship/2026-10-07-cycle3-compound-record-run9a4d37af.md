# Cycle-3 compound record — run 9a4d37af (agenttrace)

Date: 2026-10-07 · Phase: compound (attempt 2c736986) · Lane: run worktree
`run-9a4d37af94a8-9a4d37af` @ be2428849964cd1283ee1069c670eee72503251b (run's dispatched
base), 5 modified files (batch rm-640/641/642/643: +511/−34) + this compound record
(new, untracked) + this compound's ROADMAP edits (banner + four per-row validated
addenda), all uncommitted — review/shipping pending; they land after this phase, and the
next cycle's assessment carries the outcomes forward.

All outcomes below are **pre-review** evidence transcribed from the recorded phases —
NO test execution at compound, per the compound contract (targeted/full outcomes are
consumed from their recorded envelopes, never re-run here).

## Prior-attempt forensics (this run, resolved)

- **implement**: listed attempt 9da98d03 = 3-line zero-work provider-death ("Delegate
  session not found" at +18s). Real trail = UNLISTED sibling 26be8768 (dispatched
  20:14:37, reaped 21:50:08 status=failed after 95 min of real work — found by sweeping
  `events/*.jsonl` for the run_id, NOT from the work order's attempt list); its drift
  was adopted by 7389c4d0, which completed the batch and added the rm-244 rider.
- **full_tests**: listed attempt 9b5662939 (dispatched 01:39:59Z, provider death
  01:46:44Z) had launched `/tmp/at-full-9b56/run-full.sh` in the BACKGROUND at
  ~01:45:45Z; the orphaned runner survived `session_reaped` and completed 22 green
  lanes by 01:55:30Z. Adopted by c00f1b73 after four verification legs (identity /
  tree immobility / log genuineness / independent ci.yml re-derivation), then lane 23
  (workspace `--all-features`) was run fresh to close 63c6-parity. Durable mirror:
  `delegate/c00f1b7353534200af9dcd08ea61cf10-scratch/` in the conductor spool.

## Cycle outcome (pre-review evidence chain)

Batch: **"truthful CLI contract" = rm-640 (LEAD, correctness 74.0 — action-gate helpers
omit --budget/--statusline-report under --clear-cache/--update-pricing) + rm-641
(stdout-alias `-o` double-emit) + rm-642 (`--doctor -f json -o FILE` loses config
disclosure) + rm-643 rider (config `#` truncation)** — one dispatch/parsing surface,
crates/agenttrace-cli, one worktree, one validation envelope.

- **assess dba4baf6** — fresh adversarial re-derivation at be24288: baseline gates green
  (fmt rc0, clippy --workspace --all-targets -D rc0, `cargo test --workspace` 497/0,
  docs-commands gate rc0); all four batch defects live-PoC'd from
  `/tmp/at-assess-dba4/` (p1–p7); K-trap PoCs (run_bounded pipe hang rc124@40s,
  copilot checkpoint supersession) recorded and routed to existing claims.
- **research 1e206b16 (pass 12)** — upstream no movement (tip 15ed07f, v0.10.1); fork
  gap re-verified live (0 usage_points refs; no --daily/--weekly/--monthly/--tz);
  ecosystem candidates → rm-644/rm-645/rm-646 mints. Dossier
  `/tmp/at-research-1e20/research-pass12.md`.
- **roadmap b041ab38** — minted campaign-local band rm-640..rm-646 (ceiling census:
  this wall rm-598 / origin 09cb224 rm-600 / in-flight through rm-631; rm-632..639
  skipped as collision margin). NOT-MINTED map recorded (rm-628, rm-630, rm-583,
  landed rm-551, 84be17b3 seam, rm-552 lru).
- **prioritize 9c4e9af8** — ranked wall, selected the four-row batch by correctness
  concentration on one surface with live red-first PoCs already in hand; deferred
  pri-90 rows (rm-421 fork-merge, rm-251 dedup family, rm-195 decision) with rationale
  in `/tmp/at-prioritize-9c4e/prioritization.md`.
- **stewardship 334a146f** — surfaces pinned at be24288; both repositories inventoried
  (worktree campaign-delta-only; canonical /work/projects/agenttrace @ea5c41e carries
  51 foreign-porcelain entries = inventory+preserve, not ours to clean); must-remain-
  separate hints recorded.
- **implement 26be8768 → adopt-fix 7389c4d0** — 5-file change-unit, all six red-first
  tests + 3 units, A/B/C red-green-red proof via git stash replay (the pre-batch tree
  loudly rejects `--budget --range 7d`; the post-widening tree went rc0-SILENT — the
  rm-244 composition rider restored the loud rejection; see PR-3). Scratch
  `/tmp/at-impl-7389/`.
- **targeted_tests 34cdb241** — adjudicated the work order's
  `changed_testable_surfaces=[]` as the known classify_surface blind spot (crates/**
  prefixes), validated from live porcelain: `-p agenttrace` 118/0 across 6 binaries
  (58/7/38/4/9/2), six batch tests by name, fmt + clippy -p --all-targets -D rc0.
- **full_tests c00f1b73** — 23/23 ci.yml lint+full+deny lanes rc0 (empty
  `validation.full_command` → lane-for-lane mirror per fleet precedent, heavy lanes
  through the local admission gate): 03-tests locked trio 506/0 over 21 suites AND
  supplementary `cargo test --workspace --all-features` 506/0 (identical counts);
  entrypoints lane 38/0; release build rc0 from this worktree; plugin-version tag arm
  green via the CHANGELOG's 8 `no-changelog-section` markers (carrier 5b5dce0 NOT an
  ancestor — the block was re-applied on this lineage); `cargo deny --all-features
  check` correct local flag order; digest
  `validation:v1:b630f4a53f556d797678c72764a92c7ed7666c3cd13fbc905dba496903c056bc`
  declared VERBATIM (zero executable surfaces changed at full_tests) and
  engine-replicated byte-identical.

## Lessons (process, fleet-reusable)

- **PR-1 — dead-attempt triage: the listed attempt is not the trail.** Sweep
  `events/*.jsonl` by run_id; zero-work provider-deaths (3-line logs, no scratch, no
  mtimes) are noise, while unlisted reaped siblings can hold hours of adopted drift
  (26be8768) or a finished validation run (9b5662939). An absent typed result is a
  transport artifact, not a verdict.
- **PR-2 — orphaned background runners survive `session_reaped`.** The reaper does not
  kill child processes; evidence can keep landing for ~10 min past the reap. Adoption
  legs that worked: (1) script-header identity (run/attempt/REPO path), (2) tree
  immobility (git-ls-files mtime sweep + porcelain/diff-sha vs earlier-phase records),
  (3) log genuateness (cross-phase count agreement — 118 == targeted_tests' count,
  build log embedding the worktree path, gate envelopes on disk), (4) independent
  re-derivation of the lane authority (ci.yml step extraction).
- **PR-3 — widening an action gate re-audits every composition guard on that surface.**
  The rm-640 fix (enumerate budget/statusline_report in has_session_action /
  has_post_pricing_action) silently disabled rm-244's `--range` rejection for
  `--budget --range 7d` because the guard was keyed on the same helper. Caught by an
  A/B/C git-stash replay (loud → silent → loud), fixed as the same-batch rider; the
  permanent guard test `range_with_a_journal_view_is_rejected` now pins it.
- **PR-4 — KTD13 attestation: crates/** surfaces classify non-executable**, so work
  orders arrive with `changed_testable_surfaces=[]`/`required_scope=none` even when the
  live porcelain shows real code deltas. Adjudicate from `git status`/`git diff`, not
  the validation block; implement folds must name changed_surfaces explicitly.
- **PR-5 — compound consumes, never re-runs.** Targeted/full envelopes are transcribed
  with their attempt ids and digests; this phase executed zero test/validation commands
  (git/python census + file writes only).
- **PR-6 — /tmp is swept between phases; mirror what must survive.** The full-suite
  lane logs/runner/census are mirrored under the delegate spool
  (`delegate/c00f1b7353534200af9dcd08ea61cf10-scratch/`); corpora that next-cycle
  implementers need (assess PoCs `/tmp/at-assess-dba4/`, research dossier
  `/tmp/at-research-1e20/`, implement scratch `/tmp/at-impl-7389/`) are flagged
  sweep-risk — re-derive from the memos if absent.

## Next-cycle context (concrete candidates, in priority order)

1. **rm-644 (pri 78, correctness)** — copilot agent-host/CLI-1.0.8x shapes + credit-gap
   reconciliation rows (ccusage #1824, codeburn #1651 measured session). SUCCESSOR
   SCOPE over landed rm-551: this be24288 base predates it, so REBASE onto origin
   09cb224+ first and compose with rm-551's landed per-model merge; assess K2's PoC
   (15 of 1520 tokens) demonstrates the stale behavior only.
2. **rm-645 (pri 66)** — codex remote-compaction usage accounting: audit rm-401 against
   ccusage #1821's intervening-snapshot + fork-parent-out-of-range nuances
   (compact_remote_v2.rs#L461-L480); audit doc + three red-first fixtures.
3. **rm-646 (pri 40)** — `--why [session]` cost-attribution view (codeburn #1645
   shipped precedent); builds on the existing per-event pricing + anomaly lane.
4. Deferred with rationale at prioritize: rm-421 (fork-merge decision), rm-251 (dedup
   family — wait for sibling lanes to land), rm-195 (decision gate); the roadmap
   banner's NOT-MINTED map (rm-628, rm-630, rm-583, 84be17b3 seam) remains binding —
   fresh census before touching any of those subjects.
5. Watches carried from research pass 12: LiteLLM mutation-rotation pricing (daily
   snapshots miss same-key re-pricing — rows rm-006/rm-176 arms), OTel semconv-genai
   still 0 tags, upstream luoyuctl #316 cache-clamp already absorbed+exceeded here
   (signature divergence = future merge conflict), npm flat.

## Integration seams for the commit gate (after review)

- Stage the untracked artifacts explicitly — `git commit -am` silently drops this
  record (66e75e39 c1 precedent noted the same trap).
- The ROADMAP campaign delta (mint band + banners + per-row bullets incl. this
  compound's addenda) travels WITH the batch commit; done-flips (implemented → done)
  are reserved to the commit gate per the rm-012 convention.
- CHANGELOG already carries the four batch entries plus the 8 pre-existing
  `no-changelog-section` markers that keep the plugin-version tag arm green on this
  lineage.
- Cross-fleet twin (review a90bd511 F3, re-verified live at review-fix
  4b3b05d5): sibling worktree run-17319815eab4-17319815 @09cb224 holds
  UNCOMMITTED same-subject work — its rm-656 ≡ our rm-642 (doctor `-f json -o
  FILE` config_disclosure embed; their build_doctor_report refactor vs our inline
  embed_config_disclosure) and its rm-301-extension ≡ our rm-640 subject: a single
  `has_followup_action` superset (`has_session_action || list_models || test_match
  || doctor || statusline_report || budget`, 9 refs) that DELETES
  `has_post_pricing_action` outright (0 refs there vs our two-helper widening) —
  semantically incompatible, main.rs conflicts textually AND semantically at
  integration. Reconcile by intent, not by hunk: both deliver the same CLI
  contract; prefer whichever lands first and re-derive the other's tests.
- Canonical /work/projects/agenttrace @ea5c41e: 51 foreign-porcelain entries are
  inventory+preserve only (stewardship 334a146f) — not part of this change-unit.

## Reproduction pointers

- Full-suite record + 23 lane logs: `/tmp/at-full-9b56/` (durable mirror
  `delegate/c00f1b7353534200af9dcd08ea61cf10-scratch/`, incl. `full-tests-record.md`).
- Implement A/B/C stash replay + PoC corpus: `/tmp/at-impl-7389/` (sweep-risk).
- Assess PoCs: `/tmp/at-assess-dba4/` (sweep-risk; memo is the source of truth).
- Research dossier: `/tmp/at-research-1e20/research-pass12.md` (sweep-risk).
- Prioritization: `/tmp/at-prioritize-9c4e/prioritization.md` (sweep-risk).

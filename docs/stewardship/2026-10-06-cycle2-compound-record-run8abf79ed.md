# Cycle 2 compound record — run 8abf79ed (campaign 6277fbeb, repository-maintenance cycle 2)

Recorded at the compound phase (attempt 9885c1a0, 2026-10-06) from PRE-REVIEW cycle
evidence only — assess 2289ee0b, research 16a86cd5, roadmap b52be94d, prioritize
82c95d30, stewardship 72d9fee5, implement 4bf4fc04, targeted_tests 53fcf7a1,
full_tests 20f0be22. No validation command was executed at compound.

## Batch and recorded outcomes

Batch "reported-numbers truthfulness", implemented at base 2a024b6, uncommitted per
phase boundary (11 M + 1 `??` docs/guides/waste-guide.md, +889/−51, of which ROADMAP's
+60 is the roadmap phase's untouched wall delta):

- **rm-564 (flagship CU-1)** — copilot OTel cache-attr dialect: flat + upstream-#312
  dot-dialect, flat-first; torn dialect disclosed via `Event.disclosure_counters`
  `copilot_usage_dialect:cache_torn` (rm-408 carrier) instead of silent zeroing.
- **rm-568 (CU-2)** — `gen_ai_system_for(source_tool, path)`: gen_ai.system from the
  authoritative `metrics.source_tool` (13-entry family table), path markers only as
  the family-less fallback.
- **rm-567 (CU-3)** — waste truthfulness: documented component caps (cache ≤36
  = 30 rating base + 6 paid-cache penalty, loops ≤25, stuck ≤20, bloat ≤15;
  pre-stuck clamp 80 never binds — ceiling 70; scale tops out at 96) make red
  ≥70 constructible
  (was dead code, ceiling 56); Wasted $ on disjoint bases only, clamped to session
  cost with the clamp always disclosed; `docs/guides/waste-guide.md` pins scale,
  tiers, and semantics.
- **rm-566 (CU-4)** — cursor cost honesty: `estimated:true` (-f json) + `(est.)`
  (text/markdown) on cursor-derived dollars; cursor-import guide documents the
  45x-class divergence. Scope note (review 58221d71 F1): at implement time only
  the by-agent row and the json group items were marked — the "csv lane"
  phrase above then meant a test struct literal, not a shipped marker; review
  fix 25d9da7b extended the marker to every dimension: model / provider /
  task-type / project rows across text/markdown/html/json, the CSV `estimated`
  column, and the TUI top-model line.

Recorded validation (NOT re-run at compound): targeted 53fcf7a1 — impacted-crate
battery 21× `test result: ok`, 466/0/0, `rm5` filter 19/0, fmt + clippy
`--all-targets -D warnings` clean; full 20f0be22 — the `.github/workflows/ci.yml`
lint+full+deny mirror, 21 lanes rc0 (locked test lane 466/0, release build 2m20s,
8 gate scripts, docs-commands under a private `AGENTTRACE_CI_OUT` with timeout
1800, TUI smoke skipped exactly as CI gates it, cargo deny all-green). Digest
`validation:v1:6b5b0f3c703e93a1e584315877f9fffaf356da958ce22ac6110d378293f5f50a`
re-derived at compound over the unchanged tree — byte-identical.

Status flips candidate→implemented were made BY THIS COMPOUND on the four rows
(the implement phase had not flipped them); flips to done stay reserved for the
commit gate (rm-012 precedent).

## OWED ARM found at compound (disclosed, not silently dropped) — DELIVERED by review fix 25d9da7b

rm-567's acceptance bullet "--waste -f json exposes components + basis" was **not
delivered** at compound: `main.rs:474-478` routes `--waste` through
`render_waste_report_with_language` (text) regardless of `-f`. `WasteReport`
carried `wasted_raw`/`wasted_capped`/`wasted_percent`, but no json render arm
existed. Verified by first-hand diff read at compound (no re-run) and carried as
the review rider — for a batch named "reported-numbers truthfulness" the row must
not claim a closed acceptance it did not close.

**Delivered in the review-fix phase (2026-10-06, attempt 25d9da7b):** `-f json`
now routes to `waste_report_json` — component scores with their inputs
(`cache.base`/`paid_cache_penalty`/rating/hit rate, `loops.score`/percent,
`stuck.score`/pattern count, `bloat.score`/level/tools-per-turn), the pre-stuck
guard (`pre_stuck_sum`, `pre_stuck_clamped`), disjoint-basis wasted dollars
(`raw_usd`/`total_usd`/`capped_to_session_cost`/`percent_of_session_cost`), and
the `basis` block naming the session-cost denominator and the two dollar bases.
Documented in `docs/guides/waste-guide.md`.

## Prevention rules (reusable)

- **PR-1 — validation-policy engine gap (standing family precedent, third
  recurrence).** `classify_surface` matches only root-level executable prefixes;
  for this repo's `crates/**` layout `changed_testable_surfaces` is always `[]`,
  so the work order arrives with `required_scope=none`/`full` and an EMPTY
  command. Bridge, proven twice this cycle: targeted = impacted-crate battery
  over the crates the delta actually spans; full = compose the suite verbatim
  from the repository's own `.github/workflows/ci.yml` (the only authority
  available; `scripts/run_tests.sh` does not exist at this base). Fix upstream in
  hermes-conductor's `validation_policy.classify_surface` (add `crates/<name>`
  workspace-form recognition) — until then, never declare scope "none" off the
  engine's derivation when the diff demonstrably spans testable src files.
- **PR-2 — OSV clean bills need the http_code.** A bare `curl -X POST
  api.osv.dev/v1/query` returns 302 with an EMPTY body, indistinguishable from
  "0 vulns" when piped to a grep. POST with `-L`, check the code, and corroborate
  any single hit with a direct query. This cycle's corrected querybatch census
  over all 264 lock packages found exactly ONE advisory — lru 0.18.1
  RUSTSEC-2026-0253 (`LruCache::pop` not panic-safe; fix 0.18.2, max_stable
  0.18.5) — and thereby CORRECTED sibling 7f9c6d24's in-flight premise "hygiene
  only, no advisories open". Folded onto rm-007 at the roadmap phase.
- **PR-3 — prior-attempt forensics before redo.** Provider-dead attempts leave an
  event-log-only trail. Before redoing a phase, diff the worktree against the
  expected prior-phase end state: this run's compound predecessor (77eb1588) died
  of provider output trunculation mid-bank with 14 progress ticks, no typed
  result, no scratch dir — and a ROADMAP.md diff byte-count proving its edits
  never landed (exactly the roadmap phase's +60 wall delta). Same discipline
  caught research's b4bf087f (dispatch+reap only). Adoption requires a typed
  result whose identity matches; absence ⇒ redo from scratch AND declare the
  redo.
- **PR-4 — cross-fleet premises must be re-derived, never trusted.** Sibling
  c762c2b8's prioritize gated its rm-600 on "8abf79ed's implemented-uncommitted
  rm-563" — FALSE: this run's delta is rm-564/566/567/568 (CHANGELOG cites
  exactly those four; no explainability surface touched). A lane's recorded
  claim about ANOTHER run's tree is hearsay; re-derive from the tree before
  gating selection on it. Reconciliation key across the fleet is TITLE, never
  numeral.
- **PR-5 — id census is a live sweep, never a recorded map.** At compound the
  ceiling had moved 8991144 → 2db56a2 (207 defs, max rm-600) during the run, and
  the uncommitted-band landscape had churned again: this band's numerals
  rm-562..568 are FREE at the ceiling, but rm-616..619 is now DOUBLE-MINTED
  (91833f02 vs e5653f52, which reaches rm-621) and 90f54faf claims rm-622..624.
  Next free for any future mint: ≥rm-625 after a fresh def-row census covering
  ceiling + every dirty sibling worktree + spool (with foreign-fleet scoping by
  content, not header).
- **PR-6 — gate-script traps worth remembering.** `check-docs-commands.sh`
  defaults to the shared `/tmp/agenttrace-ci` and can exceed 600 s: give it a
  private `AGENTTRACE_CI_OUT` (mktemp) and `timeout 1800`. cargo-deny 0.20.2
  wants the subcommand before the flags (`cargo deny --all-features check`, not
  `cargo deny check --all-features`).

## Commit-gate checklist (for the landing gate, NOT this phase)

1. ONE commit for the whole batch: the 11 modified files + new
   `docs/guides/waste-guide.md` + ROADMAP.md (roadmap wall delta + these compound
   annotations) + this record. CHANGELOG entries already written by implement.
2. done-flips for rm-564/566/567/568 reserved to this gate (rm-012 precedent);
   rm-567 flips with the owed `--waste -f json` arm either landed or explicitly
   deferred in the flip note.
3. Sequencing seams (live first-hand sweep at compound):
   - de96d4cc races at the SAME base 2a024b6 (uncommitted rm-569..rm-575) and
     shares `crates/agenttrace-core/src/reports.rs` and
     `crates/agenttrace-core/tests/discovery_contract.rs` with this batch.
   - reports.rs additionally sits in d6432dd5's and 2d92ee95's uncommitted lanes
     (newer bases) — re-verify row anchors at merge.
   - Numeral check: rm-562..568 free at ceiling 2db56a2 (0 hits each); the landed
     rm-599/rm-600 (workbuddy keep-last lane, 99d1c79c rebounds) are
     title-disjoint from this band.

## Next-cycle leads and context

- **rm-563** (priority 70.0, highest open on this wall): per-session cost
  explainability. Zero live title claims (the landed rm-600 is the workbuddy
  lane). Every input it needs is wired by this cycle's rm-567 work (per-message
  costs, cache premium, loop/stuck shares are now disjoint-basis quantities).
- **rm-562** (55.0): Claude-3p usage-ledger home + cross-source dedupe; needs the
  synthetic-fixture strategy its acceptance already specifies.
- **rm-567 owed arm**: `--waste -f json` components+basis render.
- **rm-565** (35.0): codex dictation-history voice source, fixture-driven.
- Research negatives pinned at the roadmap phase ride forward: gemini
  retirement-vs-keep decision on the next snapshot cycle (rm-454 fold);
  semconv-genai tag watch (rm-229); LiteLLM automation triggers firing
  (rm-006/rm-176).

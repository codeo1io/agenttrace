# Cycle 3 compound record — run 32f3b7a1 (repository-maintenance 205f84bb, cycle 3)

Date: 2026-10-08 · compound attempt 9e2e6256 (after implement bf699bd9, targeted_tests 3b31a03f,
full_tests d106dde7)
Base: e9e8fd9 (HEAD, campaign worktree run-32f3b7a1fcd7-32f3b7a1; porcelain at compound = the
12 M + 2 ?? implement delta, git-diff sha256 9eeea8679de7a5b56f29c71ca2ce19dfece457d42a8634905998070c6bec8bde
byte-identical to the full_tests phase's recorded diffsha, + this record and the prevention doc)
Scope: pre-review compounding only — no test execution at compound; all outcomes below are the
recorded envelopes and proof chains of the implement/targeted/full phases, consumed as evidence.

## Batch: "honest magnitudes, honest paths"

| row | subject | files | outcome |
| --- | --- | --- | --- |
| rm-726 (LEAD, correctness 82.0) | magnitude clamps clamp silently, price to $42.7T, no anomaly | parser.rs, lib.rs, diagnostics.rs, reports.rs, governance.rs, session_cache.rs (+fixtures/implausible-magnitude/, +tests/implausible_magnitude_contract.rs) | implemented pre-review; flipped candidate→implemented |
| rm-248 (rider, compatibility 74.0) | relative XDG vars honored (config/history arms widened this cycle) | config.rs, history.rs, PRIVACY.md | config/history arms EXECUTED; row stays candidate — the original cache arm (XDG_CACHE_HOME, session_cache.rs:1141-1147 + statusline user_cache_dir) remains open by the cycle-3 scope decision (9 sibling worktrees dirty there) |

Uncommitted delta total +444/−9 over 12 M files (incl. ROADMAP.md +36 = the roadmap/prioritize
phases' pre-existing mint; code+docs delta = 11 files +408/−9) + 2 untracked code paths + this
record + the new prevention doc. Unselected, left candidate with recorded deferral residuals:
rm-727 (78.0, in-flight twin), rm-728 (74.0, in-flight twin).

## Recorded validation outcomes (NOT re-run at compound)

- implement bf699bd9 — PRIOR-ATTEMPT ADOPTION: the work order's named attempt a6f1e2c7 did NOTHING
  (event log: 2 messages in 31s, provider failure, no typed result, no scratch). The real author was
  attempt 812b2fd5541e489da083dec4880da60a on the SAME run/action lineage (22 messages 12:44–13:10Z
  2026-10-07, reaped before emitting a result envelope; its full batch sat in this worktree, mtimes
  12:58–13:07). Adopted after hunk-by-hunk verification against the stewardship contract
  (delegate/90be3955…-scratch/stewardship-request.md), then completed with 4 fixes: a diagnostics
  test appended outside `mod tests` (E0422), two unsuffixed i32-overflowing literals (1e12/9.0e15 →
  i64), a fmt nit, and the SESSION_CACHE_SCHEMA_VERSION 32→33 bump with BOTH pinned surfaces swept
  (TUI planted warm-cache literal; docs/guides/governance-reports.md:72 sentence — caught live by
  scripts/ci/check-docs-commands.sh rc1 before the sweep, rc0 after).
- rm-726 PoC flips (sandboxed HOME/XDG, /tmp/at-impl-bf69): `--overview` "$42.7T" now renders beside
  "⚠️ Clamped values: 5"; `--diagnostics` 🔴 HIGH implausible magnitude; `--sessions` HEALTH 70 /
  ANOMALIES 1 (assess baseline 100/0); `--audit` clamped_values: 5 beside the total; `--doctor`
  "Disclosed facts: magnitude_clamped:above_i64=1, above_plausibility_bound=2, negative_to_zero=2".
- rm-248 PoC flip: `cd relcfg && XDG_CONFIG_HOME=.config agenttrace --budget` → "no weekly budget
  configured" (relative ignored); absolute control unchanged ("budget $25.00 … 80%").
- main.rs untouched (forbidden surface, six siblings dirty): the --audit arm landed core-side in
  governance.rs `cost_audit()`, the --overview arm in reports.rs — exactly the selection dossier's
  contingency.
- targeted 3b31a03f — 5 focused suites + statics, all green: core lib 211/0 ·
  implausible_magnitude_contract 4/0 (NEW) · discovery_contract 90/0 · cli bin 62/0 (incl.
  `user_config_path_ignores_relative_xdg_config_home`) · tui lib 47/0; fmt rc0; clippy 3 crates
  --all-targets -D warnings 0 findings; docs-gate rc0 (binary rebuilt first).
- full d106dde7 — PRIOR-ATTEMPT ADOPTION + fresh re-execution: dead attempt b66edd79's detached
  (setsid) 22-lane ci.yml mirror battery SURVIVED the 20:18:27Z reap and completed 20:23:25Z (lanes
  07–21 log mtimes post-reap; all 22 lanes rc0; 560 passed / 27 suites / no-failed-suites), adopted
  on five verification legs (identity, porcelain census byte-identical, diff-sha match, ci.yml
  lane-provenance, machine-sums recomputed from raw logs); PLUS a fresh re-run of the ci.yml
  `Rust tests` step verbatim: rc0, 560/0/27, test-binary hashes identical to the adopted battery's
  log (deterministic reproduction). Conductor validation digest
  `validation:v1:1db2211fc741d0a1539fcba4abfeb36123cf5a2a7acb67afe3d83eb64469036a` — recomputed
  independently over the 25 executable-classified surfaces, identical to the engine's own pending
  work-order file (see PR-5 for the one-hex-char prompt-transcription deviation).

## Dead-attempt forensics (this cycle: FOUR reaped provider failures, three carried durable work)

| attempt | phase | durable trail | disposition |
| --- | --- | --- | --- |
| a6f1e2c7 | implement (named by work order) | none (2 messages / 31s) | no-op; real author found by lineage sweep |
| 812b2fd5 | implement (same run/action lineage) | FULL batch in-tree, no envelope | ADOPTED after contract verification + 4 completion fixes |
| b66edd79 | full_tests | detached battery completed POST-reap | ADOPTED (5 legs) + fresh re-execution as primary |
| 7597ef65 | compound (this phase, prior) | none (2 messages / 31s; tree byte-identical — diff sha 9eeea867… and porcelain unchanged vs full_tests records) | nothing to adopt; phase redone from scratch and declared |

## Id accounting

ZERO ids minted at compound. Wall after compound: 246 def rows, zero duplicate ids, max rm-728,
managed footer last. One status flip: rm-726 candidate→implemented (done-flips reserved to the
commit gate, rm-012 convention). rm-248 stays candidate — its own acceptance (cache arm) is not
discharged; the landed config/history arms are recorded as a dated EXECUTED bullet on the row.

## Prevention rules (reusable)

- **PR-1 — sweep the lineage, not the named attempt.** A work order names ONE dead attempt, but the
  durable work can belong to a sibling attempt on the same run/action lineage (here: named
  a6f1e2c7 was a 31s no-op while 812b2fd5 had written the entire batch). Forensics = every
  `events/*.jsonl` matching run+action, ordered by turn count and mtimes, plus the tree itself
  (unexplained porcelain = someone's work).
- **PR-2 — a reaped session does not stop its processes.** The full_tests battery (setsid) finished
  five minutes AFTER its session was reaped, and its lane logs' mtimes are the proof of
  session-independent completion. Before re-running anything expensive: check for surviving
  detached processes and freshly-written lane-results/log mtimes under the dead attempt's scratch.
- **PR-3 — adopted work is author-complete, not gate-complete.** The adopted delta needed four
  completion fixes (E0422 test placement, literal-type overflow, fmt, schema sweep) before any
  suite ran green. Adoption always ends with the focused compile+test pass, never at "the diff
  looks complete".
- **PR-4 — schema-bump same-unit set (third recorded instance).** SESSION_CACHE 32→33 moved the
  const, the TUI planted fixture AND the governance-guide sentence in one unit; the docs gate caught
  the sentence red live. Reinforces (does not replace)
  `docs/solutions/workflow-issues/schema-version-bumps-require-planted-fixture-sweep.md` and
  `session-cache-schema-bump-sweeps-all-pinned-surfaces.md`. At integration this rung re-bases onto
  the live landed ceiling (sibling ec762a618372's batch also carries 32→33 — one rung renumbers,
  the whole pin set moves with it; rm-230 one-invalidation convention).
- **PR-5 — declare the engine-file digest, not the prompt rendering.** The rendered prompt's digest
  string and the engine's pending work-order file differed by ONE hex character (62/64); sha256
  avalanche excludes any serialization difference, so it is a transcription artifact. Authority =
  the engine's own file; record the deviation when one is observed rather than "correcting" the
  declared value to match the prompt.
- **PR-6 — fresh re-execution stays primary when adopting validation evidence.** The adopted battery
  and a fresh run with IDENTICAL test-binary hashes each cover the other's weakness (stale-tree risk
  vs reap-noise risk). Adopting without re-running is acceptable only when the tree is proven
  byte-stable (porcelain + diff-sha) across the adoption boundary — both were recorded here.

Operational note: shared-box `pids.max=100` — sequence long batteries behind in-flight sibling
batteries rather than racing them (this cycle's fresh suite launched behind 8da4ef56).

## Commit-gate checklist (for the landing, not compound)

1. ONE commit on `conductor/run-32f3b7a1fcd7`, parent e9e8fd9: the 12 M files + the 2 untracked
   code paths (`crates/agenttrace-core/tests/fixtures/implausible-magnitude/`,
   `crates/agenttrace-core/tests/implausible_magnitude_contract.rs`) + this record + the prevention
   doc — stage the untracked paths EXPLICITLY (`git commit -am` drops them).
2. CHANGELOG Unreleased riders owed (not minted at compound per the 8d388605 lineage convention):
   Added/Fixed bullet for rm-726's clamp disclosure + anomaly; Fixed bullet for rm-248's
   relative-XDG config/history guards; Changed bullet for the schema 32→33 rung (guide sentence
   already aligned in-tree).
3. Flip rm-726 implemented→done at the commit gate (rm-012). rm-248 stays candidate (cache arm
   open).
4. Schema re-base check at integration: sibling ec762a618372's batch also bumps 32→33; one rung
   renumbers past the live ceiling and the same-unit pin set (const + ladder comment, TUI warm
   fixture, governance-guide sentence, any warm pins in discovery_contract) moves with it.
5. Title-reconcile at integration: none known for rm-726 (fleet-title-checked at mint and again at
   stewardship); rm-248's arms are textually disjoint from 7e00d9cb's empty-XDG arm in the same
   function (union both at integration — complementary polarity).

## Next-cycle context

Top candidates by priority are rm-727 (78.0, budget-window future-polarity) and rm-728 (74.0,
--clear-cache composition) — BOTH carry in-flight-twin residuals recorded on their rows: re-census
siblings 7e00d9cbe20a (rm-402 calendar-window arm) and 17319815eab4 (rm-301-extension
`has_followup_action` superset) LIVE before selection; fold-or-implement per each row's deferral
note (rm-727's residual narrows to disclosed-exclusion + the red-first future-dated PoC pin;
rm-728's to the enumeration test + four red-first pins if the twin lacks them).
rm-044's ureq-3 arm is PORT-READY (upstream #318's funded 2-file diff: update.rs +24/−13,
pricing.rs +9/−2) — next cycle when the Cargo.toml/lock surface is uncontested (82ccc520 was dirty
at this cycle's census). rm-053's fixture-first lane now starts from in-tree material
(fixtures/implausible-magnitude/{big,neg}.jsonl + implausible_magnitude_contract 4/0) instead of
the swept /tmp sandbox. Re-census the fleet id ceiling before any mint (ad1e1dcc's rm-722..725 was
the frontier at this cycle's mint; sibling bands move intra-day).

PoC/log sandboxes for the review gate: /tmp/at-impl-bf69/{poc,relcfg,ciout}, /tmp/at-research-e49
(/tmp is sweep-volatile; the assess sandbox /tmp/at-assess-32f3 is already swept — the in-tree
fixtures are the durable copies). Durable mirrors: the delegate envelopes and
/home/agent/.hermes/local-validation-gate/results/.

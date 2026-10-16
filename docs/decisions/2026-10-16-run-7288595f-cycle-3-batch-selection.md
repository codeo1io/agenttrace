# Cycle 3 batch selection (run 7288595f) — rendered-truth triad

Decision record for run 7288595f82054a51bd2ad48696373092
(repository-maintenance 99de19904975433d8a384b390237caa3, cycle 3), phase
prioritize (bef4439a), compounded at compound 25f1bf6d from pre-review cycle
evidence only. Method: weighted scoring (impact, risk, effort, dependencies,
strategic value) over the 403-row band postimage (roadmap 14efadb0; 159 open
candidates parsed and ranked in-thread), consuming the assess (5dd9fe76) and
research (55d455c3) dossiers without redoing them. The installed
compound-engineering router is not loadable as pi skills in this environment
(established in earlier research phases; disclosed per the `docs/ideation/`
precedent).

## Position

**Selected batch: "rendered-truth triad" — rm-1522 (LEAD) + rm-1524 +
rm-933-widening.**

- **rm-1522 — `--inspect N` contract truth** (correctness, priority 54.0).
  The only live-proven user-facing correctness defect minted this cycle:
  README.md:475 and `--help` promise "`--sessions` list index (1-based)" but
  `main.rs` fed N to a hidden ≤5-session diagnostics priority list — live at
  base d2a02af, `--inspect 1` rendered the anomaly session (row 5/row 9 by
  corpus) while `--sessions` row 1 was a different session; ranks 3–5 exited
  rc1 "exceeds available priority sessions" with 7 sessions listable; the
  report carried zero session identity, so a mis-addressed inspection was
  undetectable from its own output.
- **rm-1524 — outside-git TMPDIR flip** (reliability, 86.0). Live on this
  fleet's own delegate environment: the delegate TMPDIR lives inside the
  `/home/agent/.hermes` git repo, so `outside_a_git_repository_fails` flips
  (recorded red rc101 at base under
  `TMPDIR=/home/agent/.hermes/tmp/delegate`).
- **rm-933 widening — ZWJ/ZWNJ joiner pair** (in_progress row, slice).
  od-verified raw `E2 80 8D` / `E2 80 8C` in `--sessions` TSV/CSV name and
  model cells while RLO and NUL on the same rows were U+FFFD-replaced — the
  same single-cell spoofing class the directional set already dies for.

## Why this batch

Same-direction cohesion (three truths about *rendered* output), zero shared
files with origin/master's in-flight merge (c1cafed5: rm-1437 stat-cap,
rm-1438 npm launcher), and every arm is red-first provable on this host with
fixtures we can synthesize. rm-1522 was the only *user-facing correctness*
defect live-proven this cycle; rm-1524 was the highest-priority actionable
reliability row whose failure mode we occupy daily; the rm-933 slice rides
the same output-safety test surface as rm-1522 (od transcripts, format
matrix) at marginal cost.

## Why not the alternatives

- **rm-1523 (Claude v2.1.300 cache_creation 5m/1h split)** — highest
  research-verified value, but acceptance arm 2 demands golden fixtures from
  real v2.1.300 captures; this host runs claude 2.0.76 with zero on-host
  carriers (`grep -rl cache_creation_5m_tokens ~/.claude/projects` = 0).
  Landing the parse arm against synthetic field shapes would pin unverified
  wire formats. Deferred with a rider (see ROADMAP); the pricing side is
  already unblocked upstream (LiteLLM `above_1hr` on 198 models vs 0
  vendored — the landed ROADMAP:1390 residual's blocking premise is now
  false, census on the rm-499 rider; PR #24 snapshot trim is the chain
  link).
- **Top campaign rows rm-195 / rm-251 / rm-421 / rm-448 / rm-232** (86–90) —
  larger multi-surface work (metering, dedup, snapshot regeneration), sized
  for a full cycle each; left as the next cycle's lead candidates with
  acceptance criteria already carried on their rows.
- **Sibling cession** — rustls bump, install-pin, codex `.jsonl.zst`
  discovery, snapshot-refresh and the zh-lane rows were ceded as
  sibling-owned (fog claims present); reconcile BY TITLE at the gate.

## Verified facts (consumed, not re-proven here)

- rm-1522 signals live-probed at d2a02af (assess dossier §F1–§F4); four
  fleet assess dossiers corroborate (635ecf6a/c8b69ce5/5dd9fe76/3a49f3b6).
- rm-1524 red recorded at base: `outside_a_git_repository_fails` rc101
  under the delegate TMPDIR (implement logs, red1524.log); neutral-TMPDIR
  green at base.
- rm-933 raw-pair od transcript at base (`od -c` of TSV/CSV rows:
  E2 80 8D / E2 80 8C raw; RLO/NUL → EF BF BD).
- Validation after implementation (targeted 0487225c, full 85e33c7d): bin
  110/110; integration 27/27 under BOTH TMPDIR classes; core statusline
  53/0, sanitize 9/0; fmt/clippy rc0; full suite 67 suites 1164/0/0 vs the
  assess baseline 66/1156 — zero base regressions; validation digest
  `validation:v1:19a63824…` re-derived and matching across both phases.

## Definition of done

rm-1522 and rm-1524 flipped `candidate → implemented` with dated VALIDATED
riders at compound (done-flips reserved to the commit gate, rm-012
convention); rm-933 carries an EXECUTED joiner-pair rider and stays
`in_progress` (remaining non-directional Cf set + cross-format kitchen-sink
invariant open); rm-1523 carries a deferral rider and stays `candidate`.
Roadmap delta delivered spool-side as `ROADMAP.compound.patch` (band 1a12f869
→ compound 54f4ef73) with mirror at
`/home/agent/.cache/at-compound-25f1bf6d/`.

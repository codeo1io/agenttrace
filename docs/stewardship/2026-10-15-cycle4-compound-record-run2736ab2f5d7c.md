# Cycle 4 compound record — run 2736ab2f5d7c (repository-maintenance ec4f299d)

Date: 2026-10-15 · Base: c0da781ae78320319e226d87506e1f8876626825 · Compound attempt: e978e2da (pre-review)

## Batch

Theme: *JSON-dump lane closure — finish the formula-guard family's last human lane.*

| id | title | role | state after compound |
|---|---|---|---|
| rm-540 | CSV statement hostile-cell hardening — the `--diagnostics` TEXT renderer arm (outside the row's lane enumeration; enumerated by this run's own roadmap rider) | lead (CU-A) | **in_progress, diagnostics-renderer arm VALIDATED** (row stays open: acceptance's shared-helper sweep + the `--search` residual below; done-flip reserved for the commit gate, rm-012 convention) |
| — | CU-B: no-new-unguarded-human-lane-JSON-dump census (main.rs 13 + reports.rs 6 + mcp.rs 0 sites, each adjudicated) | rider | complete — `:2748` was the last one; census recorded in implement scratch `cu-b-census.md` |

Uncommitted worktree delta carried to the commit gate: 3 modified files, +90 lines —
`crates/agenttrace-cli/src/main.rs` (render_diagnostics guard), 
`crates/agenttrace-cli/tests/upstream.rs` (`diagnostics_lane_neutralizes_the_hostile_formula_corpus` pin),
`CHANGELOG.md` (Unreleased Fixed bullet) — plus this compound's `ROADMAP.md` band (+17) and this
untracked record. Porcelain at compound finish: exactly those 4 M + 1 `??`.

## CU-A mechanics (implement 93953c94)

`render_diagnostics` now ships
`let guarded = guard_formula_json_leaves(&serde_json::to_value(&session.diagnostics)?);`
then pretty-prints the guarded value. The `to_value` hop is required (E0308): `session.diagnostics`
is a typed struct, the guard takes `&Value` — the same discipline every landed guarded lane uses.
The `-f json` machine lane is untouched and byte-lossless (4 raw `+cmd|` occurrences preserved;
double-run `cmp` byte-identical on both lanes); the boundary is pinned by the new contract test.
Honest red: a temporary seam un-guard panicked at upstream.rs:546
(`guarded \`+cmd|\` leaf missing from diagnostics dump`); guard restored → 14/14. Live PoC flip on
`tests/fixtures/formula-lane/hostile-tool.jsonl`: assess dump lines :61/:85/:101 now paste-marked.

## Attempt trail (all four reaps adjudicated)

- assess 86519dc7 — prior attempt fdd9bbcf REJECTED/redo (typed artifact absent; event log progress
  pings ending `session_reap_failed('No module named openai', transport)`; no PhaseResult ever existed).
- research 2bd09dcd — attempt_seq 1, nothing to adopt, declared.
- roadmap e0cdbdbe2 — prior attempt 6f183ef8 provider-reaped ~19 min in but left durable scratch;
  ADOPTED per a45ba0b3/a17e2677 (builder reproduces postimage md5-for-md5 74de2291; 10 rider anchors
  re-verified; riders byte-identical after SIG swap).
- prioritize a3195d06 — prior attempt e5208cbb reaped 3 messages in (no scratch, no /tmp trace); redone.
- stewardship f9921c06 → implement 93953c94 → targeted be6c2988 → full_tests 8aca09c0 — clean.
- compound e978e2da (this record) — prior attempt 85bd0be0 died ~4 min in of a provider abort
  (event log progress-pings-only; `results/85bd0be0….json` is the failed-operation record with no
  `phase_result` key — not adoptable per the #18413(6) rule); nothing durable existed; redone from
  scratch and declared.

## Recorded outcomes (consumed as evidence at compound — NOT re-run)

- **targeted be6c2988** (env `CARGO_TARGET_DIR=$PWD/target`, `TMPDIR=/tmp`): fmt rc0;
  clippy `--locked -p agenttrace --all-targets -- -D warnings` rc0; `-p agenttrace --test upstream`
  14/14; `-p agenttrace-core --test formula_lane_contract` 5/5. Digest re-derived with
  `hermes_conductor.validation_policy.validation_digest('c0da781…', worktree)` ==
  `validation:v1:f50ab2b420f5cf77bb4fd8000c2d354b2bff84a6c54cc49aad1cc344c666262a` == dispatch
  VERBATIM (zero executable surfaces in the batch — `.rs` files are excluded by classification, so
  the digest is tree-stable across the whole fold). Porcelain fingerprint md5
  `ef7a003ae26c9b6dcf06e256c58c2513` in == out.
- **full 8aca09c0** (ci.yml-verbatim, job-by-job, explicit rc capture; dispatch `full_command` was
  empty — bde3b724 precedent): LINT fmt rc0 + clippy trio `--all-targets -D` rc0 + 7 static gates
  rc0 (manifests/surfaces/locked/install-ref/changelog/npm-files/release-pins); TEST trio
  62 suites **1109/0/0** (assess baseline 1108 + the new pin); MSRV `+1.89 check` trio rc0;
  CONFORMANCE token_conformance 2/0 + `conformance/run.sh` 13 cases OK; SMOKE 5 gates rc0 +
  real-cli rc0 (scoped read-only corpus, 8 sampled) + real-tui rc0 (testdata, 55 files) +
  release build rc0. Logs: full_tests scratch `logs/*` (surviving at spool).

### Standing reds (pre-existing, disclosed, neither batch-caused)

1. `check-install-runtime.sh` rc1 **test D** — install-runtime.sh:152 `^ref=v0.9.0$` vs install.sh:13
   `REF=v0.10.1`; byte-identical to the clean-HEAD assess finding; owned by the a64735b7/966bd5f9
   sibling lane (reconcile by title before re-fixing).
2. `cargo deny check` rc1 **RUSTSEC-2026-0346..0349 ×4** ring/rustls — advisory-DB drift, equally red
   at origin/master; the rustls 0.23.46 bump is a fleet-owned separate unit (do not bundle).

## ROADMAP band rebuild (the load-bearing disclosure of this compound)

The roadmap phase delivered its band **spool-side** (rm-012/cab6917 convention: worktree untouched,
commit gate lands it): `delegate/e0cdbdbe2…-scratch/{ROADMAP.patch, ROADMAP.postimage.md,
build_roadmap.py, wall-3e90993.md, APPLY-SEAM.md}` + `/tmp/at-roadmap-e0cdbdbe2/`. By compound time
the **spool reaper had swept every one of those** plus all /tmp mirrors of the earlier phases
(at-roadmap-6f183ef8, at-assess-8651, at-prioritize-a3195d06). The band existed nowhere durable —
the exact situation of run f2fccaa9 (9f95cf5f cycle 3).

Response (13212e2f/e9f5ab6a precedent): **rebuilt IN-TREE, uncommitted**. Source of record = the
surviving typed envelopes (`results/` + `done/` held; `delegate/e0cdbdbe2….json` carries the rider
anchor list + findings verbatim). The rebuild is content-faithful, not a byte-reproduction of the
reaped 74de2291/f0841cd5 chain; both banners say so explicitly. Delta on this wall
(HEAD c0da781, 388 def rows, def-max rm-1369): +17 lines, 0 deletions — compound banner + rebuilt
roadmap banner at the Open-items stack head (newest-first), 10 rebuilt evidence riders
(rm-017/176/195/406/421/448/485/493/540/939), the rm-540 VALIDATED rider, and a wall-foot
reconstruction note above the managed footer. **Zero numerals minted, zero statuses flipped**
(388 rows in == out; rm-540 stays in_progress). Builder: scratch `build_compound.py`; postimage
md5 `609ffb1d993b1982604ddd1c954e913d`; `patch --fuzz=0` round-trip byte-exact; assertions: row-id
census unchanged, riders in owning blocks, footer last non-empty line.

Fog rm-985 (0a3c8a9a "Formula-guard completion: governance text/fence/html, overview text/html,
compare/search text lanes") is the same subject family as this batch and **folds here by title at
the gate** — one subject, one landing.

## Reusable lessons

- **L1 (guard composition for typed structs).** `guard_formula_json_leaves` takes `&serde_json::Value`;
  typed report structs need the `serde_json::to_value(&x)?` hop first. Any future human-lane guard on
  a typed struct composes the same way — no bespoke sanitizer needed.
- **L2 (lane-boundary convention, now fully enumerated).** Machine `-f json` arms stay raw by design
  (byte-lossless wire contract); human TEXT arms guard at the value level. The CU-B census table
  (main.rs 13 / reports.rs 6 / mcp.rs 0 sites, each adjudicated by enclosing function) is the
  reference map for "which lane is which" — `:2748` closed the last unguarded human-lane JSON dump.
- **L3 (reachability ≠ coverage).** The `--search` `- tool:` residual was found by a LIVE PROBE, not
  by the grep census — grep finds call sites, only running the binary finds render behavior. Census
  first, then probe every named human surface.
- **L4 (red-first contract pins).** A guard pin that never saw red is unfalsifiable: temporarily
  revert the seam, watch the assert fire (upstream.rs:546), restore, green. Cheap, and it proves the
  test tests the guard, not the fixture.
- **L5 (ci.yml-verbatim full batteries).** When dispatch arrives with `required_scope=full` and an
  empty `full_command`, derive the suite from `.github/workflows/ci.yml` verbatim and run job-by-job
  with explicit rc capture (bde3b724) — a composite command hides per-job failures (the
  composite-truncation lesson).
- **L6 (reap taxonomy — went 4-for-4 this run).** (a) zero-work reap → redo (fdd9bbcf, e5208cbb,
  85bd0be0); (b) complete-then-reaped WITH typed artifact → adopt after identity/provenance/
  fidelity verification (the results/ envelopes); (c) durable-scratch-without-typed-artifact →
  adopt only after mechanical reproduction (6f183ef8's builder reproduced its postimage
  md5-for-md5). An absent envelope is not evidence no work happened; a present one is not proof
  it is valid.
- **L7 (spool retention is not a durability story for THIS phase's inputs).** The reaper swept the
  roadmap scratch + every /tmp mirror between roadmap and compound. Typed envelopes under `results/`
  + `done/` survived. Consequence: mirror at write time to `~/.cache/at-*` (memory #18323) AND keep
  the phase record recoverable from the typed envelope alone — this compound's rebuild was only
  possible because e0cdbdbe2's envelope carried the rider anchor list.

## Prevention rules

- **P1:** scratch that a LATER phase must consume goes to the spool scratch dir AND `~/.cache/`
  AND `/tmp` at write time, and the typed result lists all three. (#18323 reinforcement, proven
  necessary again this run.)
- **P2:** never adopt a `results/<attempt>.json` provider-failure envelope as a PhaseResult — check
  for the `phase_result` key first (#18413(6)).
- **P3:** a rebuilt band is marked REBUILT in-band; the commit gate applies NOTHING from swept spool
  history — the wall edit is the substrate (e9f5ab6a).
- **P4:** fog claims fold BY TITLE at landing (rm-985 → this batch); sibling dirty-tree claims
  adjudicate by content-hunk, never whole-file.
- **P5:** standing reds are re-proved, disclosed, and left owned — never silently fixed inside an
  unrelated batch (lane-collision discipline; test D + RUSTSEC ×4 both carry owners).

## Next-cycle starters (concrete, evidence-attached)

1. **rm-540 `--search` text-lane `- tool:` row** — renders tool names unguarded on the same lane that
   guards session names (CU-B live probe: `- tool: +cmd|' /C calc'!A0'` raw). Cheap, test-shaped,
   fixture already in-repo; fleet-contended (search lane) — title-reconcile first.
2. **rm-017 arm (a)** derive-default-ref-from-install.sh — kills standing-red test D at the root
   instead of pin-chasing; owned by a sibling lane, reconcile by title before re-fixing.
3. **rm-421** CompactionEpisode acceptance should pin the #53831 provenance-through-compaction marker
   shape (codex moved the whole surface in-window).
4. **rm-406** codex 0.163-stable fixture arm (.zst frames) — trigger is a stable release carrying the
   0.162 vocabulary; alpha is at -alpha.10.
5. Re-probe both standing reds at the next base after sibling landings (test D may close via the
   sibling fix; RUSTSEC ×4 closes only with the fleet rustls bump).
6. Cycle-5 assessment should carry the review/shipping outcomes of THIS batch forward (post-review
   scope — deliberately out of this compound per the phase contract).

## Commit-gate seams

- Land order: this ROADMAP band + the 3-file implement batch + this doc; single commit is fine
  (f2fccaa9 shape) or roadmap/code split (e4725315 9e84599/24bfc68/161a40d shape).
- Base lag to reconcile: HEAD c0da781 vs origin/master 8bc0587 (217783f, 3e90993, 94e1db6
  conductor-landing b5d391ec, 8bc0587 merge b5d391ec) — overlapping surfaces
  governance/statusline/waste/CHANGELOG/ROADMAP; pricing.rs untouched by this batch, so the
  b5d391ec reconcile-by-title seam lands clean.
- Fold by title at landing: fog rm-985 + this wall's rm-540 riders into ONE landing.
- CHANGELOG already carries the batch's Unreleased bullet (implement); the ROADMAP edit adds no
  CHANGELOG obligation.
- Digest at commit time should re-derive `validation:v1:f50ab2b4…c666262a` on the pre-commit tree
  (zero executable surfaces ⇒ unchanged until a code file moves).

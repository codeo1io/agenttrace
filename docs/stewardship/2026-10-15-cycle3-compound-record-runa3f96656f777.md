# Cycle 3 compound record — run a3f96656f777402fbf064c282c59a7a8 (repository-maintenance 1754b828 cycle 3)

**Base:** c0da781 (worktree run-a3f96656f777, HEAD unchanged all cycle) · **Recorded:** 2026-10-15, compound attempt b41bb80b2b784e8aabaac70a3157ac47 after full_tests 464a3a9f · **Scope:** pre-review evidence only — review/shipping outcomes belong to the next cycle's assessment. **No test execution at compound** per the phase contract; every number below is a recorded prior-phase outcome, consumed as evidence.

## Batch: 'lane-truth — stop printing false zeros and false prefixes' — implemented, all gates green (pre-review)

| member | outcome |
|---|---|
| **rm-1430** Cline/Roo usage-carrier extraction (LEAD) | implemented pre-review (implement 6b7d0b41): the 11-function cline lane extracted zero usage while both real store shapes carry metrics — the Roo task-dir say-entry whose text IS a metrics JSON, and the Cline CLI `.messages.json` envelope's sibling `metrics` object (tokscale #1369). Carriers fold through a synthetic `session_meta` usage event with `recorded_cost_usd` (pi precedent); provenance flips to `reported_by_agent` with text-stream estimation suppressed; input recorded net of cache legs (rm-600 basis, saturating); negatives clamp at intake and disclose; cost-only JSON is not a carrier — usage is never invented. 6 red-first core tests (5/6 failed pre-implementation). Live-binary E2E: `--overview` $0.0123 with `cline_metrics_carriers_parsed=1` disclosed; `-f json` total_tokens 1350 / summary `total_cost` 0.0123. |
| **rm-1434** doubled 'Error: Error loading …' render (PAIR) | implemented pre-review (same batch): the `main.rs` bail literal dropped its 'Error ' prefix — the `:333` printer owns the single prefix; pinned by an exact-render entrypoints test (real existing dir, missing paths bail earlier) plus a bail-literal grep guard as a test. Red asserted before the fix. |

**Validation evidence (recorded, not re-run here):** targeted 82947d86 — 23 assertions green (cline 6/0, intake_attribution_truthfulness 14/0, entrypoints error_prefix 2/0 + bail_literals 1/0), fmt + clippy clean, tree byte-identical in/out (dispatch digest copied VERBATIM — nothing executable changed). full 464a3a9f — 25 ci-verbatim local lanes green: clippy `--locked` 3 pkgs `--all-targets -D` rc0; `cargo test --locked` **1116 passed / 0 failed** across 62 suites (base 1108 + exactly the 8 batch tests, zero regressions); msrv `+1.89 check --locked` rc0; conformance 13 cases (1 xfail-pinned); release build + 6 smoke gates + real-cli + TUI rc0; 12 lint-job gates rc0. ci.yml @c0da781 md5 f9a72a6d was the lane authority.

**Standing reds, adjudicated standing (not batch-caused; pristine clone @c0da781 controls):** check-install-runtime test D (no row on THIS wall — the integration lineage lacks rm-051; master walls carry it); cargo-deny rustls RUSTSEC-2026-0346..0349 vs locked 0.23.45 (Cargo.lock zero-diff; cure owned by **open PR #22**; rider appended to rm-018); actionlint SC2001 `ci.yml:192` (ci.yml zero-diff; fog rm-1504 title-twin owns it on another lineage).

## Roadmap state (this record's companion band, in-tree `M ROADMAP.md`)

- **rm-1430 + rm-1434 minted-implemented** with EXECUTED + VALIDATED riders; done-flips reserved to the commit gate (rm-012 convention).
- **rm-1431 / rm-1432 / rm-1433 minted candidate** (deferred at prioritize 3ae952ef, reasons recorded on the rows).
- **rm-1435 minted candidate (new at compound):** the Cline CLI `.messages.json` store is parse-supported by this batch but undiscoverable — `is_cline_task_dir` (`discovery.rs:1118-1122`) probes only the Roo trio (`:1120` is its `ui_messages.json` line; no `.messages.json` probe exists); `is_cline_task_file` (`parser.rs:8133-8138`) matches only the Roo trio. Content-shaped classification is the acceptance.
- **REBUILT band disclosure:** the roadmap phase's spool-side postimage (md5 6609b83c…) was reaped before landing, and it was authored against a DIFFERENT wall (origin/master@3e90993, md5 1d59adda, 392 rows, def-max rm-1373) than this worktree's own (md5 60d023e3, 388 rows, def-max rm-1369). The band in-tree is rebuilt from typed envelope 9ced39ae's spec, re-anchored to the live wall; every rider anchor grep-verified here; the rm-1373 rider (ccusage org re-anchor + Amp dual-path) has no row on this lineage and rides the roadmap banner — master's rm-1373 receives it at the gate.
- **Next free numeral: rm-1436** (after a fresh census; fog claims beyond that across sibling lineages — reconcile by title, never by numeral).

## Durable lessons minted this cycle (add-file docs alongside this record)

1. `docs/quality-gates/untrusted-usage-metrics-intake.md` — the rm-1430 never-invent rule: every lane parser consuming externally-recorded usage must clamp-at-intake, disclose every suppression, and refuse cost-only "carriers".
2. `docs/workflow-issues/spool-reaper-mirror-at-write-time.md` — the spool reaper sweeps current-run scratch within hours; mirror at write time, and typed envelopes are the reconstruction spec when postimages are gone.

Plus process lessons recorded banner-side (L3–L5): verify every programmatic edit by tree diff, not tool ack (one edit-tool success-misreport caught this cycle only because git status was the arbiter); gate binaries that `--doctor` the real HOME need sandboxed HOME+XDG (20-min fleet-corpus scan vs 0.22s fresh); standing-red adjudication is a three-part test — prior fleet adjudication, pristine-control byte-diff, batch blast-radius intersection.

## Context for the next maintenance cycle

- **NEXT-CYCLE LEADS:** rm-1435 (discovery classification for the Cline CLI store — natural pair to rm-1430's landed lane, small and fixture-first) or rm-1431 (the 68-site entrypoints env-hermeticity sweep — own cycle, prioritize's sizing).
- **Defer-causes stand:** rm-1432 per-key pricing blocked on pricing.rs contention (b5d391ec +262 landed post-base; sibling 7bb862e4 dirty on pricing/statusline); rm-1433 OSC 7501 emission twin blocked on the statusline seam.
- **Standing reds follow their owners:** rustls → open PR #22 (merge is coordination, not work); install-runtime test D → the install-ref campaign on master walls; SC2001 → fog rm-1504.
- **Reaper forensics (this run):** research 94964658 · roadmap 9ced39ae (70258cb8 boot-death redo) · prioritize 3ae952ef · stewardship 146d2019 (ADOPTED complete-but-reaped 4918e96c) · implement 6b7d0b41 · targeted 82947d86 · full 464a3a9f (c7aeeb72 boot-death redo) · compound b41bb80b (15d636ee provider boot-death redo — heartbeats-only, nothing durable). All scratch except full 464a3a9f's (its note + `/home/agent/tmp` mirror survived) swept pre-compound; typed envelopes are the evidence chain.
- **Commit-gate seams:** stage exactly 8 paths (4 M batch + M ROADMAP.md + 3 ?? docs); never `-A`. Fold vs the live master wall by content-hunk newest-first (master at e130845, md5 61318649, 397 rows at recording time); title-reconcile numerals; flip rm-1430/rm-1434 to done with the landing. A title-twin watch applies: sibling lineages have landed independent implementations of pricing-class rows before — grep titles, not numerals, before claiming.

## Chain of custody (banked + mirrored, commit gate stages in-tree)

typed envelopes (spool, survived) → preimage 60d023e3 banked → `build_band.py` (deterministic, assertions) → postimage 6afa8d92 (394 rows, def-max rm-1435) → applied in-tree → `ROADMAP.patch` round-trip-verified byte-exact on a fresh clone @c0da781 → mirrored to `/home/agent/.cache/at-compound-b41bb80b/` (the reaper lesson, applied this same turn).

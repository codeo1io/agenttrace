# Cycle 2 compound record — run 7f9c6d24 (repository-maintenance 8dc71851)

- **Base:** 67dfdb5a1d9d5de5f3208da95b158ee016c7e659 (run worktree `run-7f9c6d24eac4-7f9c6d24`, family `agenttrace-80c75f65b7`)
- **Phases:** roadmap 4a589626 · prioritize 25959edd (adopted after a delivery-infra death past completion; verification 79040f04) · stewardship 4fecdabf · implement 80128456 (after 76e7e09b's provider-infra death at 29s with zero durable work; research was likewise a redo after d6af4e82) · targeted_tests 09669de8 · full_tests 14f7dbeb · compound 39c6cad6 (this)
- **Batch:** "usage-accounting truthfulness, cycle 2" — rm-555 (LEAD 88.0) + rm-551 (84.0) + rm-552 (80.0) + rm-553 (78.0) + rm-554 (72.0) + rm-556 (68.0)
- **Delta at compound time:** 8 M tracked (+352/−166 code ex-ROADMAP) + 2 ?? (tests/usage_accounting_truthfulness.rs + 11 fixtures). No commits.

## Recorded outcomes (pre-review; consumed at compound, not re-run)

- Implement PoC flips on target/release/agenttrace: claude-streaming 3000/600/370 → 1000/200/200 · qwen-cache2 1700 → 1100 (400 in / 600 cache / 100 out) · codex-reasoning 550 → 400 (+150 breakdown) · codex-reset 1200/520 → 1700/700 · copilot-partial 1000/200 → 1500/300 · copilot-resumed 1300/130 → 800/80 · copilot-shutdown-tail 1.0s → 20s @ $0.03. Controls unchanged: wb-multi 500/50, pi-azure $0.0017 deepseek-v4-pro.
- Schema 26 → 27 swept across all pinned surfaces (session_cache const, discovery_contract ×2, tui fixture, governance sentence live-verified by the docs gate).
- targeted_tests 09669de8: core 327/0 (14 suites), tui 47/0, cli 86/0, fmt/clippy/docs rc0.
- full_tests 14f7dbeb: ci.yml mirrored verbatim (dispatch full_command empty — crates/** classifier artifact), 22 lanes / 22 PASS / 0 FAIL; 460/0 in both debug and release profiles; 9 artifact gates green; `cargo deny --all-features check` ok; plugin-version tag arm green with no CHANGELOG remedy at this base.
- Digest `validation:v1:8d1fbc2193e3df4793aac240794c959c5b9c175c5c716c73ff547810f3e39b8d` declared verbatim at both validation turns; each re-derived live with the engine's own `validation_digest()` at the programmatic full-sha base, byte-identical (crates/** deltas are digest-immobile under the classifier; the ROADMAP/CHANGELOG/docs drift at compound cannot move it either — a deliberate property, not an accident).

## Infra-failure pattern this cycle (three reaps, zero lost work)

Three attempts across the run died of infrastructure (d6af4e82 research ~5m18s, 76e7e09b implement 29s, 25959edd prioritize after completion). Each was handled by forensics-then-adopt-or-redo: read the event log + typed-result presence + tree state before touching anything. 25959edd's completed result was adopted after verification; 76e7e09b had zero durable work and was redone; d6af4e82's raw corpus was adopted after re-validation and the synthesis redone. Lesson already practiced fleet-wide; recorded here as cycle context.

## Status flips and id landscape

- rm-551..rm-556 flipped `candidate → implemented` with EXECUTED addenda on each row. `done` stays reserved for the commit gate (rm-012).
- ZERO ids minted at compound. Live sweep at compound (local worktree walls + spool scratch, no network): this family's unlanded band tops at rm-561; fleet live ceiling rm-680 (worktree wall, dashboard family on the 8991144 lineage) / rm-681 (spool mentions) → next free ≈ rm-682, to be re-swept live at mint time.

## Cycle-3 deferred queue (from this cycle's mint band, priority order)

1. rm-558 (76.0) — keyword hosts dispatch before flag validation; leading flags silently misroute.
2. rm-557 (74.0) — pi azure-provider sessions price at wrong rates (provider dropped from the model string); catalog-side refresh rides rm-176.
3. rm-559 (52.0) — upstream default ref should follow the remote's HEAD.
4. rm-560 (48.0) — codex per-account dimension.
5. rm-561 (38.0) — dependency bump wave (upstream #309), hygiene only.

Wider pool lanes unclaimed by this family: rm-421 / rm-251 parser arcs, rm-011 statusline journal length.

## Commit-gate handoff

- Land the 10-entry delta as one batch; flip rm-551..rm-556 to `done` at the commit gate (by title anchor).
- CHANGELOG rider already minted under Unreleased → Fixed at compound (cycle-1 555a174d precedent); fold into the release section or land as-is.
- Riders to reconcile by title with sibling bands: the rm-592 needless-borrows lint fix in cli/main.rs (3 lines), the `type ModelUsageMap` alias rider in parser.rs.
- No validation re-runs are owed by this record: the newest full-suite evidence (14f7dbeb, 22 lanes) covers the tree modulo the ROADMAP/CHANGELOG/docs drift written at compound, which is digest-immobile.

## Prevention rule minted this cycle

- docs/solutions/workflow-issues/session-cache-schema-bump-sweeps-all-pinned-surfaces.md — the schema constant is pinned in five places including the sentence the docs gate verifies against the live constant; a missed surface only surfaces at a later gate.

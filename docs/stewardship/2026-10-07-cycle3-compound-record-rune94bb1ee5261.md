# Cycle 3 compound record — run e94bb1ee (2026-10-07, pre-review)

- **Run:** `e94bb1ee52614d1fbb453e82704246ea` (repository-maintenance `b28e2827b61846339157aecbe1f1241b`, cycle 3)
- **Base:** HEAD `89911442173d31f4eb21a1968bfd51ad46e32ed1` (unchanged all cycle; everything below is an uncommitted worktree delta in `run-e94bb1ee5261-e94bb1ee`)
- **Batch:** "trust & truthfulness" — rm-578 (lead, governance cost-basis false-drift) → rm-579 (upstream Spawn misdiagnosis) + the rm-007 dated arm (lru 0.18.5 + deny unmaintained policy); selected by prioritize 2b23fa4c
- **Status at compound:** implemented, pre-review. Review and shipping happen after this step; the next cycle's assessment carries them forward.

## Attempt registry (the event log is the complete registry)

| Phase | Attempt | Note |
| --- | --- | --- |
| assess | 03d2f201 | adversarial sandbox /tmp/at-assess-e94b (gov4/gov3/gov PoC corpora, env -i spawn PoC, deny/lru lane) |
| research | 00bf4f8c | upstream #318 merge (ureq 2→3) census, npm registry sweep, open issues #103/#236/#237 |
| roadmap | 0cdce7f6 | minted rm-578..rm-582 (+31 ROADMAP lines), fleet def-row sweep + title-check |
| prioritize | 2b23fa4c | batch selection, full scoring in /tmp/at-prioritize-e94b/batch.md |
| stewardship | 54e83e47 | change-unit contract for the 3-unit batch; anchors re-verified live |
| implement | dcd0919a → 87bf5f66 → bb4b83e2 | dcd0919a reaped (12:05); 87bf5f66 reaped mid-turn WITH a durable uncommitted delta — **audited, partially adopted** (below); bb4b83e2 completed (15:19) |
| targeted_tests | f7590190 | 327/0 core + 10/0 upstream + 31/0 entrypoints + 43/0 bins + 47/0 tui; clippy/fmt/deny clean |
| full_tests | 5cc3806a → 25b20d87 → 03f6b54f | 5cc3806a reaped (18:01, progress pings only — porcelain census at the next start proved zero tracked drift); 25b20d87 reaped +28s (4 events, zero durable); 03f6b54f completed: `cargo test --workspace --locked --no-fail-fast` 22 sections 471/0, RC=0 |

## Dead-attempt forensics — the third class, first seen here

This cycle hit **both** known dead-attempt classes plus the discriminator that separates them:

- **Census-clean reap** (25b20d87, 5cc3806a): porcelain census at the next attempt's start matched the prior phase's recorded census exactly → the dead attempt changed nothing → phase redone from scratch. (Also dcd0919a on implement, subsumed by the audit below.)
- **Reap with durable delta** (implement 87bf5f66): no typed PhaseResult, but the uncommitted worktree delta survived. Audited hunk-by-hunk against the row acceptance criteria, **per row, not per attempt**:
  - **Adopted** after fresh verification: the rm-579 upstream.rs Spawn-routing fix + its regression test (one repaired syntax break: a missing `;` after a `git_probe` call — proof the attempt died mid-edit, not post-validation), and the session_cache GoMetrics zero-defaults.
  - **Rejected and redone**: its rm-578 approach (append an explanatory suffix to the drift note) — the row's acceptance demands the false alarm **stop**, not gain an explanation — replaced by the basis-mirroring fix (recorded-cost token classes excluded from the governance re-price, counted at recorded dollars; cache-hit split-loss reports `unavailable`, not drift).

Prevention rule durabled at this compound: `docs/solutions/workflow-issues/provider-dead-attempt-left-durable-delta-audit-then-adopt-or-redo.md`.

## What was implemented (uncommitted delta, 10 paths, crates src +352/−62)

- **rm-578 governance cost-basis false-drift** — `crates/agenttrace-core/src/governance.rs`: per-model rows carry `upstream_priced_input/output/cache_w/cache_r` (saturating), the current-rate catalog total excludes recorded-cost token classes and counts them at their recorded dollars, `append_drift_note` fires only on genuine catalog-share drift; warm-cache sessions missing the recorded-token split get estimated/rates withheld (`null`) with the "no same-basis re-price available" note and `pricing_source: unavailable: …`. `lib.rs` upstream-priced accumulation feeding the rows; `session_cache.rs` GoMetrics zero-defaults; semantics sentence in `docs/guides/governance-reports.md`; red-first regression `tests/governance_cost_basis.rs` (4 tests).
- **rm-579 upstream Spawn misdiagnosis** — `crates/agenttrace-cli/src/upstream.rs`: the three probe-stage `Err(_) => bail!` arms (:216-217/:221-222/:233-234) route `GitRunError::Spawn` to a spawn-class terminal diagnosis ("failed to spawn git …: is git installed and on PATH?") in human and `-f json` renderings, honoring rm-543's ran-and-failed contract; +1 regression test (`tests/upstream.rs`, suite 10/0).
- **rm-007 dated arm** — `Cargo.lock` lru 0.18.1 → 0.18.5 (past RUSTSEC-2026-0253's 0.18.2 floor; transitive via ratatui 0.30.2); `deny.toml` gains `unmaintained = "all"`; the unsound advisory class (no cargo-deny v2 knob — probed live) gets the recorded bump-on-sight rule in the same comment block.

## Recorded validation outcomes (pre-review; consumed at compound, NOT re-run here)

- **targeted (f7590190):** core 327/0 (incl. `governance_cost_basis` 4/4), cli upstream 10/0, entrypoints 31/0, bins 43/0, tui 47/0; `cargo clippy --workspace --all-targets` 0 warnings; `cargo fmt --all --check` clean; `cargo deny --all-features check` all-ok; Cargo.lock pins lru 0.18.5.
- **full (03f6b54f):** `cargo test --workspace --locked --no-fail-fast` (the CI `full` job's test authority run workspace-wide) — 22 test-result sections, **471 passed / 0 failed**, 0 warnings, RC=0 (`/tmp/at-fulltests-e94b/full-test.log`). Per-section delta vs the assess baseline 463/21: `governance_cost_basis` 0→4 (new section), `upstream` 9→10, core lib 231→234 — every added test accounted for by the batch.
- **LIVE PoC replays (implement phase):** gov4 cold: stored 0.0075 == estimated 0.0075, note "exact normalized model match in pricing catalog" (pre-fix: estimated 0.0428 + false drift note); gov4 warm-cache replay: estimated/rates `null`, pricing_source `unavailable: recorded-cost split not retained by the session cache`; gov3 control: 0.0428 == 0.0428, no note (re-price path unchanged); A3 `env -i PATH=/nonexistent … upstream`: spawn-class error rc=1 (pre-fix: "not inside a git repository").
- **Digest lineage:** dispatch token `validation:v1:a72443d225dc8d687e34505eb328bd65608884abc60d932d59f83678062b7b0c` declared verbatim by targeted_tests AND full_tests, and re-derived byte-identical with the engine's own `validation_policy.validation_digest` at both turn ends (crates/** deltas are digest-immobile under the surface classifier).

## Roadmap state at compound

- Flips: rm-578, rm-579 → `implemented` with EXECUTED bullets; rm-007 dated EXECUTED arm bullet (row stays candidate — the staged dep-refresh wave is open); rm-580/581/582 untouched candidates; **done-flips reserved for the commit gate** (rm-012 convention).
- CHANGELOG: 3 Fixed riders minted under Unreleased at compound (555a174d precedent).
- Ids: ZERO minted at compound. Wall: 203 backticked def rows (217 incl. old-format), status accounting 122 candidate / 45 implemented / 36 done → 120 / 47 / 36. Live fleet sweep at compound: ceiling `rm-650` on the run-f6a7e7f3 wall — any minting phase must re-sweep live.
- **Commit-gate seam:** rm-578 is the recorded title-twin of unlanded rm-570 @ de96d4cc ("cost_audit false-drift on recorded-cost") — rebind by TITLE at integration, never by id (rule recorded on the row itself).

## Next-cycle context

- **rm-580** (lockless session-cache lost-update + temp-sweep race) — LOW by design (cache is regenerable; cost is re-parse time, never wrong data); assess PoC surfaces at session_cache.rs:560-586, 1177-1240.
- **rm-581** (qwen `/export` + `--json-file` radar, decision-first) — upstream issue #237 open; @qwen-code/qwen-code 0.25.0 (2026-10-05); title-disjoint from landed rm-488 and the alias-sum band.
- **rm-582** (gemini retire-or-keep, decision-first) — upstream #312 dropped Gemini CLI from the parser (merged 2026-10-05); radar #236 tracks the Antigravity transition; fork README still lists Gemini as supported.
- **rm-044** (ureq 2→3 wave) — upstream #318 merged 2026-10-06 is now the port reference (8-crate bump group); fork's only ureq site is pricing.rs:673 (2.12 API).
- **rm-006** (pricing refresh) — LiteLLM live snapshot JSON persists in /tmp/at-research-e94b.
- **Repro sandboxes (persist across delegate sessions):** `/tmp/at-assess-e94b` (gov4/gov3/gov corpora + cache-final2), `/tmp/at-fulltests-e94b` (full-suite log), `/tmp/at-prioritize-e94b`, `/tmp/at-stewardship-e94b`, `/tmp/at-implement-e94b` (verification log), `/tmp/at-targeted-e94b` (validation log), `/tmp/at-research-e94b` (raw API JSON). *(Correction at the commit gate, 2026-10-07, independent-review finding F4 — verified live: /tmp sandboxes are EPHEMERAL; at-assess/at-implement/at-prioritize/at-research/at-stewardship above were reaped before the review, and only `/tmp/at-fulltests-e94b`, `/tmp/at-targeted-e94b` and `/tmp/at-final-e94b` survive. Nothing shipped depends on any of them: the rm-578 regression corpora are embedded as inline constants in `tests/governance_cost_basis.rs`, and the rm-006 lead must re-pull the LiteLLM snapshot live rather than read `/tmp/at-research-e94b`.)*

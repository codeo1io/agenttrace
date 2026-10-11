# Cycle 1 compound record — run e4eb225445324d13814e81278334384f (repository-maintenance ba50ee5c cycle 1)

**Base:** e9cb9c1 (worktree run-e4eb22544532-e4eb2254, HEAD unchanged all cycle) · **Recorded:** 2026-10-11, compound attempt c8d175d0 · **Scope:** pre-review evidence only (review/shipping outcomes belong to the next cycle's assessment).

## Batch: 'nothing silently swallowed' — implemented, all gates green (pre-review)

| member | outcome |
|---|---|
| **rm-947** antigravity default-root family (LEAD) | implemented pre-review (implement 3b613f9a): 4 new KnownSessionDir roots (.gemini/antigravity, -ide, -backup; .config/antigravity via XDG fallback), parser routing content-sniffed (no classifier change needed), doctor discloses every root; red arm proven by pristine-swap; live PoC: 4 sibling roots × conversations sidecars → `--sessions` lists 4 antigravity rows, `--doctor` found=1 parsed=1 |
| **rm-921 rider** MCP EOF-edge overlong swallow | implemented pre-review (same batch): EOF arm consults `overlong` before emptiness; test `overlong_line_terminated_by_eof_is_refused_not_silently_dropped`; live PoC: exactly 1,048,577 bytes no-newline → disclosed −32600 refusal (pristine: silence) |

**Validation evidence (recorded, not re-run here):** focused legs discovery_contract 102/0, pi_family_discovery 2/0, disclosure_case_fold 7/0, mcp_server 7/0; work-order gate `cargo test` rc 0 — **878 passed / 0 failed** across 52 suites (assess baseline 876 + the 2 new red-first tests, zero regressions); `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; digest token `validation:v1:6d5b7a0b…e526` re-derived byte-identical from (HEAD sha, worktree root) at both targeted_tests and full_tests.

**Roadmap state:** rm-947 + rm-921 statuses flipped pre-review by implement 3b613f9a (commit gate owns the landing + final flips). **ZERO new numerals this compound** — next free remains **rm-948** (census: wall 387 def rows at base; this run's chain adds rm-947 only).

## Durable lessons minted this cycle (add-file patches alongside this record)

1. `docs/quality-gates/byte-caps-must-test-the-eof-edge.md` — every cap guard needs the no-terminator EOF boundary arm, not just the mid-stream refusal; the rm-921 defect class.
2. `docs/workflow-issues/provider-dead-attempts-may-leave-real-worktree-diffs.md` — dead-attempt forensics: adopt-by-hypothesis + red-arm re-verification; observed twice this run (36cb9268 left real work; 8b753a36 left none).

## Context for the next maintenance cycle

- **NEXT-CYCLE LEAD: rm-042** (calendar-window family) — its blocking predicate (rm-917's landed-at-bf1bf40 machinery) is now proven in-tree; FIRST arbitrate the unlanded rm-916 (#306 space-boundary) title-twin before designing.
- **Sibling landing-order watch:** run 5a04ae3b's claimed batch (rm-619 LEAD + rm-944 + rm-946) sits on base 428f0e9, two commits behind ours — its rm-619 A/B PoC evidence and corrected anchors are already on the wall via this run's appends; reconcile by title at integration.
- **Census correction stands:** 9b2f16bd's banner ("spool claims through rm-947") is wrong — b7972ceb's live patch mints rm-912/937/938 only. Banners are not census authority; grep the patches.
- **Standing unlanded evidence awaiting owners:** cline/discovery byte cap (rm-927 band; 942 MiB RSS PoC this cycle), npm probe byte bound (rm-052 residual; 390 MB/1.5 s exact-argv proof), future-dated budget disclosure (rm-941 band; $42-invisible PoC), by_model round4 (rm-944), fmt_duration boundary (rm-946), antigravity-acp 6th root (rm-947 acceptance (d) corpus gate — ccusage #1851 still open as of 2026-10-10).

## Chain of custody (spool, all reversible, commit gate applies)

pristine ROADMAP.md @ e9cb9c1 → c8893e54 `ROADMAP.patch` (mint + 8 appends + banner) → 26b3d228 `ROADMAP-designation.patch` → 3b613f9a `roadmap/ROADMAP-implement.patch` → c8d175d0 `ROADMAP-compound.patch` (this record's roadmap leg) + `docs-add-files.patch` (this record + the two lesson docs).

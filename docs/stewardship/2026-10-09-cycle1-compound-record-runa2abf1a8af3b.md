# Cycle 1 compound record — run a2abf1a8af3b42a890100b13154f3b16 (repository-maintenance 44a52589)

- Run worktree: `run-a2abf1a8af3b-a2abf1a8` @ `2578751654723a59c5a416d8dd21234ae36a5403` (merge of de600e73)
- Batch: "deterministic gates & hostile-input honesty" — rm-818 LEAD + rm-819/rm-820/rm-821/rm-822 (rm-823 minted, still candidate; ceded to 749cd298's lane)
- Delta at compound close: 9 code files (+1010/-34) + ROADMAP.md (+61/-0 vs HEAD: roadmap-phase +56 with 5 status flips candidate→implemented and 5 dated `executed` bullets) + 2 compound docs (this file + docs/solutions/workflow-issues/red-first-hardening-vacuous-green-and-implicit-guards.md). All uncommitted; the implemented→done flip is reserved to the commit gate per rm-012.

## Phase ledger

| Phase | Attempt | Outcome |
|---|---|---|
| assess b2bc227e | b2bc227e | 14 lanes rc0 but tests flaked run 1 → F1 HIGH env race; F2 TUI unsanitized; F3 append TOCTOU; F4 torn journal; F5 CSV marker forgery |
| research 11d9bcd9 | 11d9bcd9 | ~24 probes; upstream frozen 15ed07f2; claude-code 2.1.293 tiered Haiku 5.5; codex 0.161 format-quiet; LiteLLM gap 1,404 |
| roadmap 7d3df557 | 7d3df557 | adopted reaped 8efd1560's +56 ROADMAP delta after 6 corrections (forensics: mtime window, envelope absence) |
| prioritize cb56cd98 | cb56cd98 | batch selected from the assess's own F1–F5 findings — the run fixed what its own battery caught |
| stewardship 5a759ea5 | 5a759ea5 | one change-unit batch, file-level non-overlap; canonical checkout's staged wave preserved untouched |
| implement dd10a972 | dd10a972 | 5 units landed red-first; +1010/-34; U1 soak 10/10 |
| targeted_tests b70f9885 | b70f9885 | 413/0 focused + clippy + fmt, zero fixes (digest unchanged → declared verbatim) |
| full_tests 9dc693ab | 9dc693ab | 22/22 ci.yml push lanes rc0 FIRST TRY, trio 654/0 (assess baseline 616 executed; 619 @feb16bba) |

## Evidence anchors (all under delegate spool)

- Red/green loops: `delegate/dd10a972…-scratch/{u1-prefix-red,u1-postfix-green,u3-prefix-red,u3-postfix-green,rm818-soak}.log`
- Full suite: `delegate/9dc693ab…-scratch/{run-full.sh,full.log,full.rc}` (22 lanes, lane list byte-identical to feb16bba's runner)
- Targeted: `delegate/b70f9885…-scratch/targeted.log`
- Validation digest for the whole cycle (tree unchanged through validation phases): `validation:v1:d7e8d62a722a28d958dea17064bd10f81225227b6f21066d4e69b4ef8f7f600e`

## Lessons (L1–L6)

- **L1 — fix what your own battery catches.** The batch was selected from this run's assess findings, not from a wishlist. Every unit had a PoC on file before a line was written, which made red-first cheap and review trivial.
- **L2 — a flaky suite is a finding, not noise.** F1's "retry made it green" was the signal that found a real per-test env lock racing every other env consumer (rm-818). Retry-wrappers hide this class; soak loops expose it.
- **L3 — vacuous-green is the hardening tester's main trap.** O_DIRECT (0o40000) fat-fingered for O_NOFOLLOW (0o400000) made the race harness pass by breaking on write errors. Race harnesses must panic on every error, never break.
- **L4 — implicit platform guards are not safety.** The locked ratatui skips zero-width symbols, so ESC/BEL/C1 never reached cells pre-fix — the TUI looked safe by accident. When behavior passes pre-fix, the honest red-first is a structural source pin (grep-count pins) plus family-equality asserts; keep the behavioral walk as the regression net.
- **L5 — concurrency asserts must be one-sided or deterministic.** "Marker present iff my snapshot ends torn" is unorderable (the render reads at its own moment); the soak gate caught it. Paused-writer phase carries the deterministic assertion; the stress loop carries panic-freedom only.
- **L6 — the batch summary is not the contract; the acceptance row is.** An early percent-rounding implementation under the rm-822 label was the wrong row (sibling shape), caught by re-reading acceptance mid-implement and fully reverted. Read `acceptance:` before writing code, not after.

## Prevention rules landed in-tree this cycle

- `env_serialization_is_the_one_shared_lock_not_per_test_statics` (core lib): no `static … Mutex<()>` env-lock shape may return anywhere in agenttrace-core; exactly one `static ENV_LOCK` may be declared.
- `tui_render_sources_route_the_shared_sanitizer` (tui tests): every flagged render file must route the shared sanitizer family; the crate must re-export it (no second sanitizer).
- Race harnesses in session_cache/statusline tests panic on any I/O error (fail-loud rule from L3).

## Next-cycle ranked candidates (context, not minted)

1. **rm-823 (open candidate)** — MCP protocol-version currency: DEFAULT_PROTOCOL_VERSION still '2024-11-05' vs live spec 2026-07-28/2025-11-25 ladder; sibling band rm-780/781/782 already landed via 749cd298 — reconcile the version ladder before minting.
2. **Unbounded compaction read sibling of rm-821** — `compact_statusline_capture_locked` (statusline.rs ~:446) still reads the whole file unbounded; bounded by the keep-under rewrite design but the read itself should ride `read_journal_capped` discipline.
3. **rm-014 family (ceded twin)** — presentation.rs remains a cfg(test) render twin of shared.rs production rendering; the poisoned-corpus test renders through the twin, not production. Dual-renderer binding (rm-014) is the durable fix.
4. **Windows single-syscall reparse-point open** — rm-820's documented residual: FILE_FLAG_OPEN_REPARSE_POINT (or ReOpenFile) equivalence when Windows enters the threat model; today check-then-open + privilege analysis is the documented posture.
5. **ratatui upgrade watch** — L4's implicit zero-width skip: any ratatui upgrade must keep the buffer-walk test (the regression net for "sanitizer routing reverted × skip removed").
6. **LiteLLM pricing snapshot gap** — 4,504 live keys vs 3,100 bundled (2026-10-04); claude-haiku-5-5 tiered pricing ($0.50/$2.50 >100K) present upstream, absent from the bundle.

## Commit-gate seams (for the conductor-landing lane)

> Review-fix correction (2026-10-09, independent_review:fix e017898f over review 702d9682 F5): the implement (dd10a972) and targeted (b70f9885) envelope prose transposed two per-suite counts — ground truth, first-hand rerun and targeted.log's own per-suite lines: **output_safety_matrix = 5, entrypoints = 44** (the envelopes said 44/5 in the other order). The 413/0 total is correct either way and targeted.log was always right; only the envelope prose labels were swapped. The spool envelopes are immutable phase artifacts — this paragraph is the durable in-tree correction of record.

- One commit rides: ROADMAP delta (roadmap phase + compound flips/bullets) + 9 code files + 2 compound docs. The implemented→done status flip for rm-818..822 happens at the commit gate (rm-012 convention); rm-823 stays candidate.
- Canonical checkout `/work/projects/agenttrace` carries a staged wave (~51 files, ci.yml/release.yml/changelog workflow) — preserved untouched by this run; do not ride it into this batch's commit.
- Digest `validation:v1:d7e8d62a…` is the pre/post full-suite digest for this exact tree; if the commit gate touches anything executable it must re-emit.
- The full-suite runner is reusable verbatim: `delegate/9dc693ab…-scratch/run-full.sh` (22 lanes, ci.yml push lanes).

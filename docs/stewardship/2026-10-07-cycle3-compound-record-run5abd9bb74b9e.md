# Cycle 3 compound record — run 5abd9bb7 (2026-10-07)

- **Run:** `5abd9bb74b9e4525973bb71456334fcf` (repository-maintenance `8dc71851d3ff427aa31ccef4c2370a20`, cycle 3)
- **Base:** HEAD `09cb224ae469a0c5499a86d434665ff6b928d708` (unchanged all cycle; everything below was an uncommitted worktree delta in `run-5abd9bb74b9e-5abd9bb7` until this landing)
- **Batch:** "Stable-seam reliability" — rm-166 (LEAD, statusline journal integrity, folding campaign mint rm-686 by title at integration) + rm-688 (`--latest` stat-storm memoization) + rm-038 (TUI project-identity memoization)
- **Status at landing:** implemented → reviewed (independent_review APPROVED) → release-integrity PASS (final_validation) → committed. Review-fix riders and residual riders are recorded on-row in `ROADMAP.md`.

## Attempts and forensics

| Phase | Attempt | Note |
| --- | --- | --- |
| research | 4305f7c9 | 15th extensions-research pass @09cb224; 4 net-new candidates (rm-689–692) |
| roadmap | 8e983cf5 | REDO of dead 23ee5e69 (pings-only event log, no typed result, zero durable tree work; plan adopted, numerals re-derived) — minted rm-685–692, +8 rows |
| prioritize | 05bf0bac | selected the batch; campaign-local fold of rm-686 under rm-166 |
| stewardship | 6500b0de → ecb63e07 | 6500b0de reaped at harvest (orphaned result); redo re-verified every pin first-hand and found two IMPOSSIBLE prior pins (app.rs:2164-2181 `resolve_project`, `SessionIdx`) — the corrected seams are core `insights.rs:210-244`, not TUI app.rs |
| implement | e3d9e9f0 | 8 files +639/−37; red-proofs both units (lock-less append loses append 2/2; live-resolve stub breaks memo parity) |
| targeted_tests | 860c4919 | core-lib 204/0, cli-bin 56/0, tui-lib 49/0; fmt/clippy clean; tree unchanged in/out |
| full_tests | aee7a7ae | ci.yml lint+full+deny lanes mirrored verbatim (dispatch `full_command` empty, fleet precedent): 522/0 across 24 targets, release rc0, entrypoints 33/0, all script gates; two gate-surface fixes (manifests rust_version mirror 1.89; SIGPIPE-under-pipefail probe → here-strings, 60/60 soak; flake proven pre-existing at pristine base) |
| compound | 44ba76d4 | spool-side roadmap compound (banner, EXECUTED bullets, lessons L1–L9); worktree untouched |
| independent_review | 7eaf667e | VERDICT APPROVED (inline lens spine; prior attempt a802f887 provider-dead, redone) |
| final_validation | c51d169e | release-integrity PASS: digest `validation:v1:0552414f…` == dispatch stamp, surface classification matches, no file written after 06:32:12 UTC |

## Lessons (full text in `ROADMAP.md` tail comment, "cycle-3 lessons & prevention rules")

1. **libtest child-reexec filter trap** — filter child re-execs by bare test-name substring; `--exact` + `module_path!()` can never match.
2. **Vacuous race fixtures** — count the racing operations and assert `count > 0`; degenerate runs otherwise pass vacuously.
3. **`printf | grep -q` under `set -euo pipefail`** — grep -q's early exit closes the pipe; SIGPIPE fails the pipeline and a `!`-guarded presence check reports present markers missing (~1/10 under load). Grep here-strings. Residual same-pattern site: `check-release-surfaces.sh:124` (inert today).
4. **rc masking through pipes** — piping a gate's output through `tail` reports tail's rc; capture rc before pipes.
5. **MSRV bump protocol** — workspace `rust-version` + gated floor lane + manifest-alignment expectation + CHANGELOG, one bump.
6. **Machine-contention triage** — a green-in-isolation suite timing out under load avg ~35 is machine load, not a fixture hang; prove via no leftover processes/temp dirs.
7. **Hand-me-down pin verification** — every seam pin is re-read at the phase that acts on it; two impossible pins arrived this cycle and were caught at stewardship redo.
8. **Dead-attempt triage** — pings-only event logs / reaped sessions with orphaned results are REDO-from-scratch, adopting only plans with numerals re-derived live.
9. **Census-from-artifact rule** — def-row totals and max numerals are re-derived from artifact bytes (`^- id: \`rm-\`` lines), never copied from a prior narrative. This cycle's narratives carried heading counts (228/236) and a banner-prose numeral (rm-608) as if they were wall censuses; true figures at base: 214 def rows / def-max rm-600; post-compound wall: 222 def rows (122 candidate / 43 done / 56 implemented / 1 folded), def-max rm-692. The patch→base→postimage chain was verified byte-identical throughout, so nothing material shifted.

## Next-cycle context (recorded pre-review)

- **LEAD rm-685** — history re-key on transcript move (origin-based cycle; compose with landed rm-389's rewritten id machinery; `history.rs` lives in agenttrace-**core**).
- **rm-687** — copilot fold single-arm + true max (re-verify against origin's rewritten `parser.rs` hunks @@-358/-402 at integration).
- **rm-689–692** — external-fixture procurement: Mistral Vibe provider arm, Antigravity standalone-app cache-read + app-model pricing, codex `session_index.jsonl` titles, read-only footprint report (composes with landed rm-575).
- **Residual riders** — `app.rs:1424` top_driver live-resolve (once per refresh); `check-release-surfaces.sh:124` pipe pattern.
- **Numeral frontier** — this cycle minted rm-685–692; fleet unlanded claim ceiling at compound time was rm-736 → next free rm-737 after a fresh live census. Count def rows by `^- id:` lines only (L9).
- **Integration note** — origin/master advanced 61570ea → aa5544af during the cycle (ROADMAP +175/−5, zero new def rows above rm-605); the only batch-file overlap was `main.rs` +19 in hunks disjoint from the rm-688 seam (:1099-1114). This band unions by TITLE on rebase.

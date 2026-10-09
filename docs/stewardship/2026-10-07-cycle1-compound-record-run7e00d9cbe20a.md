# Cycle 1 compound record — run 7e00d9cbe20a (repository-maintenance e74cb714)

- **Date:** 2026-10-07 · **Cycle:** repository-maintenance e74cb714a19f408da485d698afe07eb1 cycle 1
- **Worktree:** conductor-worktrees/agenttrace-80c75f65b7/run-7e00d9cbe20a-7e00d9cb @ base `be24288` (HEAD unchanged; the batch ships as an uncommitted delta to the commit gate)
- **Compound attempt:** `edcb8578` (re-dispatch superseding `4d04171e`, which was session-reaped provider-dead at 12 messages — event log `session_reaped exit_reason=failed` / `delegate_turn_completed status=failed`, no typed artifact, no scratch dir; its in-tree riders/banner adopted only after full live re-verification under the later id)
- **Attempt trail:** assess `2696e362` · research `71cc9746` (pass 11) · roadmap `b935b766` + re-dispatch `1b1eda09` · prioritize `8c20e13c` · stewardship `b85c8bb5` · implement `d7f1c67e` (adopting provider-dead `ef6ef241`'s delta) · targeted_tests `bcd31e91` · full_tests `e2040c96` · compound `4d04171e` (reaped) → `edcb8578`

## Batch: 'surface honesty wave 1' (pre-review, uncommitted)

| Subject | Row | Disposition at compound |
| --- | --- | --- |
| XDG set-but-empty env → RELATIVE user-config path (CWD-dependent config injection) | `rm-683` (lead, minted this cycle) | **flipped candidate→implemented**; done-flip reserved to commit gate |
| `render_budget_view` triple journal read + vacuous first-writer test | `rm-684` (minted this cycle) | **flipped candidate→implemented**; done-flip reserved to commit gate |
| Budget-window calendar anchor (assess N1: $10/0% rendered vs $1 true calendar-week) | arm on parked `rm-402` | row stays **candidate** — only the window arm landed; the row's cross-file replay-baseline subject is untouched |
| `text_cell` control-byte sanitization (OSC-52/CSI/BEL) | residual arm on `rm-239` | row stays **candidate** — the umbrella's done disposition reserved to the commit gate per its own rider |

Validation outcomes **consumed as recorded, not re-run** (compound runs no suites):

- targeted `bcd31e91`: 500 cases across 22 green suites (343 core / 110 cli / 47 tui), clippy 0 warnings, fmt clean; the three rm-683 PoCs and the $1.00/90% budget PoC re-verified live under that attempt.
- full `e2040c96`: 22-lane ci.yml `full`+`deny` mirror ALL rc=0 — 03-tests 22 suites 500/0, entrypoints 33/0, docs-commands green over this batch's CHANGELOG legs, cargo-deny ok.
- digest `validation:v1:b630f4a53f556d797678c72764a92c7ed7666c3cd13fbc905dba496903c056bc` PRE==POST==dispatch, replicated with the deployed `validation_policy` at base `be24288`.

## Reusable lessons / prevention rules

**PR-1 — Provider-dead prior-attempt adoption (hit TWICE this run).** Both `ef6ef241` (implement) and `4d04171e` (compound) died mid-flight leaving on-disk work but no typed artifact. Protocol that worked: (a) confirm death in the engine's own event log (`session_reaped`/`delegate_turn_completed failed`) and the absence of `delegate/<attempt>.json`; (b) census the tree delta to attribute the orphaned work to this run/action lineage; (c) adopt it as an UNVERIFIED source only; (d) re-verify every leg live under the new attempt id and record the supersession. Expect latent defects in adopted work: the implement adoption carried **three** (mis-bumped window-test arithmetic vs the old 7.0×0.25 pin; a rm-239 test asserting sequence-STRIP against the house rm-034/rm-540 SUBSTITUTION-to-U+FFFD contract; an unsatisfiable OnceLock first-writer pin inside the shared lib process), and the compound adoption carried **one factual banner error** (PR-6).

**PR-2 — Engine classifier blind spot for `crates/**` and repo-root doc paths.** `changed_testable_surfaces=[]` → `required_scope='none'` and an EMPTY dispatch `full_command`; per deployed `validation_policy.py:1458` (`if expected_full:`) the engine then SKIPS the command-match clause entirely. This is a classifier quirk, NOT an absence of changed Rust code. Fleet answer: mirror `.github/workflows/ci.yml`'s `full`+`deny` jobs lane-for-lane and say so plainly (precedents 614624d7, 7eae74ea, 9a4d37af, this run's e2040c96).

**PR-3 — Process-global state can't be pinned in the shared lib test process.** A `OnceLock` install/reject contract asserted inside the crate's own test binary is order-dependent (any earlier test may have installed the value). Move deterministic first-writer pins to a fresh-process test target (`tests/runtime_config_first_writer.rs` pattern) and keep only order-agnostic invariants in-crate.

**PR-4 — Grep for the in-repo precedent before writing an env-path guard.** The XDG empty-guard already existed at `statusline.rs` `user_cache_dir` while `config.rs` `user_config_path` joined the env unconditionally — the same rule, ~40 lines away, missed at the seam. Rule: when touching env-derived path resolution, sweep the crate for the existing guard pattern first. Upstream's `config.rs` still lacks the guard → **upstream-PR candidate** (fork-first fix, rm-683).

**PR-5 — /tmp is swept between phases; mirror run artifacts to the spool at production time.** Of this run's seven `/tmp/at-*` dirs, four were gone by compound time (`at-roadmap-1b1e`, `at-prio-8c20`, `at-research-71cc`, `at-assess-2696`); only those mirrored into `~/.hermes/conductor-delegate-spool/<attempt>-scratch/` survived. Anything needed by a later phase (selection memos, applied snapshots, lane logs) must be mirrored when written, not when wanted.

**PR-6 — Compound banners must be fact-checked line-by-line under the writing attempt id.** The reaped 4d04171e banner cited "doctor.rs :240/:245" for a double journal read; live re-grep shows :240/:245 is `DoctorProjectDecodeReport` tally code — the real double read is `statusline_journal_stats` + `read_statusline_captures` back-to-back at `doctor.rs:371-372` inside `doctor_statusline_report`. Every `file:line` claim a compound writes gets re-derived, not copied (the 36c9140f review already flagged banner factual errors once).

**PR-7 — The validation digest is a changed-surface SET, not diff bytes.** Content edits inside an unchanged changed-file set leave `validation:v1:` identical; adding/removing a changed file moves it. Re-derive the token live on any validation-bearing turn instead of trusting a recorded one (this run: `b630f4a5…` PRE==POST==dispatch replicated at base `be24288`).

## NOT-MINTED next-cycle bounds (bound, not dropped)

1. **doctor.rs double journal read** — `doctor.rs:371-372` (`statusline_journal_stats(&path)` then `read_statusline_captures(&path)`); `statusline_journal_stats_from_buffer` (added by rm-684) already implements stats-from-one-read but is private — expose and reuse. Sibling of rm-684's triple-read; XS effort.
2. **'surface honesty wave 2'** — absolute-path disclosure sweep across the remaining doctor lanes (the rm-683 PoC class: relative/misleading disclosures), plus the two multi-subject rows' remainders (rm-402 cross-file replay baseline; rm-239 umbrella disposition).
3. **rm-232 stays deferred** in the #312 token-accounting reconcile zone (sibling e5653f52 rm-616/617 contend the same seam); re-census before selecting.
4. **LiteLLM micro-refresh** (R3, skipped as optional): +5 chat-priced rows / 36 price changes vs the bundled 2026-10-04 snapshot — rm-006's cadence lane.
5. **Upstream-PR candidacy** for the XDG empty-env guard (rm-683; upstream confirmed lacking it in pass 11).

## Artifacts

- ROADMAP.md compound delta (this attempt): banner re-dispatch supersession + corrected doctor anchor + rm-683/rm-684 status flips; patch `delegate/edcb8578…-scratch/compound-edcb8578-ROADMAP.patch` (applies on the implement-phase ROADMAP state; dispatch-state snapshot beside it).
- Roadmap phase splice + riders: `delegate/1b1eda09-scratch/ROADMAP-2026-10-07.patch` (in-tree since implement).
- Research dossier: `docs/research/2026-10-07-extensions-research-pass11.md` (landed by implement, rider R2).

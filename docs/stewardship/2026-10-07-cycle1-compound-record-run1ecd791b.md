# Cycle 1 compound record — run `1ecd791b316c4107aecf4f76522dfc20` (repository-maintenance `224c5cf19d614a9385016741f20d1cec`)

- **Phase:** compound (attempt `9146154b52804583aae25e9d41614951`, after full_tests `18b1e33107c84a4684a69fb504d5b2f3`)
- **Date:** 2026-10-07 (campaign opened 2026-10-06)
- **Worktree:** `run-1ecd791b316c-1ecd791b` under `agenttrace-80c75f65b7`; campaign base `8991144`; batch tree = salvage commit `ae7277239de9b69dc015b936c684e2c14bed7915` on branch `conductor/run-1ecd791b316c` (parent `8991144`)
- **Scope of this record:** pre-review compounding only — assessment, research, roadmap, prioritization, implementation, and recorded test outcomes. Review and shipping outcomes are deliberately absent; the next cycle's assessment carries them forward. No test execution happened at compound (phase contract); every validation number below is a consumed, already-recorded outcome.

## 1. Batch

**rm-594 — "Sanitize upstream-status text output at the render boundary"** (track: security, priority 78.0), selected SOLO by prioritize `75bd5553` (selection memo: `delegate/75bd5553-scratch/selection-memo.md`) on five dimensions: live od-verified OSC-52 clipboard-write + CSI injection in the flagship offline drift view; LOW implementation risk (render-boundary-only, `-f json` untouched); closes the last unwired sink of the rm-034/rm-383/rm-505/rm-540 hostile-bytes family; merge-clean against every sibling dirty lane; single-crate effort grounded on the existing `pub fn sanitize_line_segment` (statusline.rs:275).

Delivered shape (implement `97b559a8`, 5 files, +188/−4):

- `crates/agenttrace-cli/src/upstream.rs` — the three raw interpolation sites in `to_text` (`:486` remote_url, `:505` base_subject, `:524` commit.subject) plus the same remote-authored class on the `--fetch` npm-registry probe route through `sanitize_line_segment` at the render boundary; fields git itself constrains (release tag names under git's ref format, `%cs` sync date, static areas table, operator-config remote/ref names) deliberately stay raw, with the boundary documented in the guide.
- `crates/agenttrace-cli/tests/upstream.rs` (+82) — integration pins `hostile_upstream_bytes_render_zero_control_bytes` and `benign_repo_text_output_is_byte_stable` plus unit-boundary pins; the existing 19-fn `git()` fixture home was extended, not new-filed.
- `CHANGELOG.md` — one Unreleased Fixed bullet (rm-594) describing the threat model, the sanitization routing, the raw-by-design boundary, and the untouched `-f json` arm.
- `docs/guides/upstream-status.md` — output-contract note.
- `ROADMAP.md` (+12) — this campaign's roadmap-phase delta (the rm-594 mint row, cycle-1 banner, three dated appends), riding the batch per the carry-forward policy.

Acceptance from the mint row, all met: sanitize at the `to_text` boundary; benign byte-identical text; replay fixture from `/tmp/at-assess-59bc/osc` asserts zero raw control bytes; `-f json` unchanged; unit + integration tests pin both properties.

## 2. Fold chain and dead-attempt forensics

Three delegate attempts of this run died provider-side; none left durable work, and each death was triaged before redoing:

| Phase | Attempt | Fate | Durable work |
|---|---|---|---|
| assess | `84771adf` | provider infra; typed result absent, events tail `session_reaped:failed` | none (probe corpora at `/tmp/at-assess-84771` salvaged as inputs only) — redone from scratch as `59bc617f` |
| implement | `6340fa30` | provider infra ~2 min in; typed result absent, events tail `session_reaped:failed` at ~1791290193 | none — tree at dispatch state (`M ROADMAP.md` only); phase redone from scratch as `97b559a8` |
| targeted_tests | (unnamed prior, died 13:08:49Z) | died before any validation ran | the implement delta itself, via the conductor salvage path (below) |

**Salvage event.** The dead targeted_tests attempt left the 5-file implement delta uncommitted; the conductor salvage path committed it as `ae72772` ("conductor-salvage: run=1ecd791b…", trailers `Salvage-Run`/`Salvage-Status: failed`/`Salvage-Phase: targeted_tests:targeted_tests`, message "no validation claim — content preservation only; the integration fleet owns landing"). The targeted phase that actually ran (`4019084d`) discovered the clean tree at dispatch, verified the salvage preserved the batch **byte-exact** (`git show --stat ae72772` = the same 5 files, +188/−4, no extra hunks), and then validated fresh on top — no claim was inherited from the pre-salvage attempt. This discipline is durabled as a prevention rule: `docs/solutions/workflow-issues/conductor-salvage-commits-preserve-content-not-validation-claims.md`.

## 3. Recorded validation outcomes (consumed here, NOT re-run)

- **Targeted (`4019084d`, on the salvaged tree at `ae72772`):** `cargo test --locked -p agenttrace --bin agenttrace` → 45/0; `cargo test --locked -p agenttrace --test upstream` → 11/0; `cargo fmt --all --check` rc0; `cargo clippy --locked -p agenttrace --all-targets -- -D warnings` rc0 (22.87s). Digest reproduced with the engine base: `validation:v1:a72443d225dc8d687e34505eb328bd65608884abc60d932d59f83678062b7b0c`.
- **Full (`18b1e331`):** the work order's `full_command` VERBATIM — `python3 …/scripts/local_validation_gate.py --shell-command 'cargo test'` — gate rc0 (admitted workers=1), fresh compile from the worktree, **21 result-ok suites, 467 passed / 0 failed / 0 ignored** (3 unit suites: CLI bin 45 / core lib / tui lib; 16 integration suites; 2 doc-test targets), zero `FAILED`/`panicked`/`error` lines in the full log, porcelain 0 before and after. Log archived at `delegate/18b1e33107c84a4684a69fb504d5b2f3-scratch/full-gate.log`; gate envelope `…-scratch/gate-envelope.json` (outcome completed, returncode 0, starvation_escape false).
- **PoC replay (implement `97b559a8`, on `/tmp/at-assess-59bc/osc/fork`):** post-fix binary renders `357 277 275` (U+FFFD) at the offsets that carried raw `033` pre-fix (0600/0662); python census 0 raw ESC / 0 raw BEL / 0 non-LF control bytes. Red leg: a pristine-`8991144` baseline binary built in a detached throwaway worktree (removed after) emits raw-ESC 3 / raw-BEL 1 on the same fixture. Byte-stability A/B: benign fixture text **and** json byte-identical (`cmp`); hostile json A/B differs only in `refs_age_seconds` (14238→14239 — time passing, not content).
- **Digest lineage:** the dispatch token `validation:v1:a72443d2…7b0c` was declared VERBATIM by both validation phases and re-derived byte-exact at the post-run tree (`validation_digest('8991144…', '.')`). Compound touches ROADMAP.md/docs only — non-executable surfaces — so the token stays current.

**Envelope-vs-token note (recorded so no future phase misreads it):** `local_validation_gate.py` re-derives the validated tree's digest by path and folds `base:{base_sha or 'unknown'}` into the hash; the work order's `full_command` passes no `--digest-base-sha`, so the gate's result envelope records `digest_base: "unknown"` and a digest that **can never equal** the engine's dispatch token — by design, per the script's own v1.11 docstring (the 38d28400 two-lost-hex-chars incident is why it exists). The mismatch is not tree drift, and the envelope digest must never be declared as the validation digest; reproduce the token with `validation_digest(base_sha, repo_path)` instead.

## 4. Prevention rules (PR-1..PR-4)

1. **Salvage commits preserve content, not validation claims.** A `conductor-salvage:` commit (Salvage-Status: failed, "no validation claim — content preservation only") must be content-verified (`git show --stat` against the recorded phase delta, spot per-file diff) and then validated fresh on top; never inherit, transcribe, or assume validation evidence from pre-salvage attempts. Full doc: `docs/solutions/workflow-issues/conductor-salvage-commits-preserve-content-not-validation-claims.md`.
2. **Dead-attempt triage is three checks, then redo.** Typed result absent + `events/<attempt>.jsonl` tail `session_reaped:failed` + porcelain at dispatch state ⇒ zero durable work: redo the phase from scratch and declare the redo in the record. Do not attempt to "finish" a dead attempt's partial state sight unseen.
3. **The gate envelope's digest is base-unknown by construction.** Never compare it to the dispatch token and never declare it; re-derive the token with the engine's base sha when tree identity needs proving.
4. **Hostile-bytes fixes land at the render boundary with a byte-stability harness.** House pattern (rm-034 → rm-383 → rm-540 → rm-594): route the sink through `sanitize_line_segment` where the string meets the output medium; pin with (a) a hostile replay asserting zero raw control bytes, (b) a benign fixture asserted byte-identical old-vs-new (`cmp`), (c) the machine-format arm proven untouched. The A/B baseline binary comes from a detached pristine-HEAD worktree that is removed afterward.

## 5. Status accounting and wall state

- Def rows: 213 (roadmap phase left 213 with the rm-594 mint; compound minted nothing).
- Status mix after the compound flip: **117 candidate / 36 done / 46 implemented** (199 new-format rows; 14 old-format). `done` stays reserved to the commit gate (rm-012 convention).
- Fleet ceiling at compound (live def-row sweep 2026-10-07, this attempt, all 45 run-walls in `agenttrace-80c75f65b7`; the other two family dirs carry no run walls): this wall max `rm-594`; fleet on-disk high-water **`rm-646`** at `run-9a4d37af94a8`'s wall (225 rows), sibling maxes rm-631 (`aa31de94`), rm-627 (`a0d0d353`), rm-625 (`9873fc06`), rm-624 (`90f54faf`), rm-621 (`e5653f52`). **Next free `rm-647` after a fresh live claim census** — the space below is recorded contested.
- CHANGELOG Unreleased already carries the rm-594 Fixed bullet from implement; compound adds no CHANGELOG entry.

## 6. Commit-gate seams

- The commit gate lands: salvage commit `ae72772`'s tree + this compound's delta (`ROADMAP.md` flip/banner, this record, the prevention doc). One commit per lineage convention.
- Fold-check **by title** at integration vs de96d4cc's unlanded rm-574 (same file `upstream.rs`, disjoint subject: silent `-f` fallback + DEFAULT_REF — both may land; compose, don't dedupe).
- rm-543's bounded-runner wrapper landed on the same file; sanitization composes at the render boundary, not the subprocess boundary (recorded on the row).
- Review and shipping outcomes are out of scope here by the compound contract; the next cycle's assessment picks them up.

## 7. Context for the next maintenance cycle

- **K1 admission-gap row** (history-hash/admission signals, `ROADMAP.md:853` band): the session-**file** FIFO variant is proven (2026-10-03 note; this run's assess re-verified the PoC live — `timeout 5 agenttrace -d <fifo-dir> --sessions` → rc=124, 0 bytes, name-only admission in `discovery.rs`); the session-**root** variant remains unproven. A ready-made next-cycle candidate with a live fixture.
- **rm-006 pricing refresh trigger is live:** LiteLLM 4,480 entries (2026-10-06 research) vs the bundled snapshot; the anomalyco org-move rider and chatgpt-subscription null-cost-skip expectation are recorded on the row's dated appends.
- **Upstream quiescent** through 2026-10-06 (master `15ed07f2`, v0.10.1; 5th consecutive check) — port-wave items (rm-042 family) remain parked, not stale.
- Repro fixtures that persist across delegate sessions: `/tmp/at-assess-59bc/osc/` (hostile remote + behind-fork). The assess dossier `assessment.md` and several probe corpora were already reclaimed by tmp cleanup before compound — behaviors stay pinned by the in-tree tests and the recorded od transcripts in the spool verification log.
- No ce-* compound-engineering router exists in this harness (agent-reach only) — 8 consecutive phase confirmations across this run; phases execute directly and say so.

## Verification (how to check this record)

- `git -C run-1ecd791b316c-1ecd791b rev-parse HEAD` → `ae7277239de9b69dc015b936c684e2c14bed7915`; `git status --porcelain` → this compound's delta only (`M ROADMAP.md`, the two new docs).
- `git show --stat ae72772` → 5 files, +188/−4, salvage trailers as quoted.
- `grep -c '^- id:' ROADMAP.md` → 213; `grep -n '^- id: `rm-594`'` → one row, `status: implemented`; `tail -1 ROADMAP.md` → managed footer.
- Recorded outcomes: `delegate/18b1e33107c84a4684a69fb504d5b2f3-scratch/full-gate.log` (21 `test result: ok.` lines, 467 passed), `delegate/4019084d40f34fa1bf489a15ee8fdf28.json` (targeted typed result), `delegate/97b559a895aa48e58b80bba987041456-scratch/verification-log.md` (implement PoC replay).
- Fleet census: sweep `run-*/ROADMAP.md` max def ids across `agenttrace-80c75f65b7` (commands in the phase result evidence).

# Cycle 2 compound record — run de96d4cc48fb (repository-maintenance d3046844)

- **Base:** 2a024b69a38d036bb6daee448fd9b4bb9f2792b1 (run worktree `run-de96d4cc48fb-de96d4cc`, family `agenttrace-80c75f65b7`)
- **Phases:** roadmap 015d1767 · prioritize 4e0cfbd1 · stewardship 0cbc0c0f · implement ed48face · targeted_tests c0f99783 (redo after 9b1c07aa's provider-infra death) · full_tests bc1b5913 · compound f7fc3e47 (this). Assess and research were themselves redos after 2d667579 and 4e83920e died of provider infra.
- **Batch:** "CLI dispatch-surface truthfulness, cycle 2" — rm-569 (LEAD 78.0) + rm-573 + rm-389 + rm-212
- **Review fix pass (2026-10-06, independent_review:fix 19af3243):** verdict NEEDS_CHANGES -> fixed in-tree: F1 empty-summary/zero-fill (version + compared-fields now required, named rejections, 2 demo_contract tests), F2 legacy DefaultHasher history rows (same_derived_session identity-tuple skip + preserve fold, env-flow unit test, live sandbox replay converges the file to one key), F6 symlink admission mislabels (admit_session_path extraction, target-class naming, bin test), F7 journal-untouched e2e pin (entrypoints 30/0). Suites after the fix: core lib 183/0, demo 13/0, discovery 79/0, bin 41/0, entrypoints 30/0, fmt/clippy clean; live rc-matrix green (F1a-d, F2, F6a-c, rm-573 journal, rm-389 guard). F3's open `--health '<nan'` filter arm remains open (out of batch scope, no row).
- **Delta at compound time:** 8 M tracked (ROADMAP.md, CHANGELOG.md rider + main.rs, reports.rs, history.rs, discovery.rs, demo_contract.rs, discovery_contract.rs — live numstat +374/−10 code+tests plus ROADMAP's +64 mint delta and this compound's annotations) + 2 new docs (this record + the prevention rule). No commits.

## Recorded outcomes (pre-review; consumed at compound, not re-run)

- Implement rc-matrix (target/debug, isolated XDG_CACHE_HOME): bad-baseline valid-JSON non-overview → rc=1 truthful reject (was fabricated `delta_pct 100.0`); missing baseline → attributed with path; non-UTF-8 → named; `/dev/null` admission → "character device" (was "does not exist"); `--limit 0` → rc=1 "must be at least 1" (was `[]` rc0); `--statusline-report statusline` → report rendered rc=0, no journal file.
- targeted_tests c0f99783: 312/0 (fmt rc0; clippy `-- -D warnings` rc0; core lib 182/0; demo_contract 11/0; discovery_contract 79/0; cli bin 40/0). Prior-attempt 9b1c07aa forensics: envelope absent, ~40s provider death, tree byte-identical to the implement handoff patch — nothing to adopt, redone.
- full_tests bc1b5913: dispatch `full_command` empty (crates/** classifier artifact) → ci.yml jobs `full`+`deny` mirrored verbatim as command authority (099b6f61/0597e89a precedent), 20/20 lanes rc0 — 455 passed / 0 failed across 19 targets, release build + entrypoints 29/0, all 9 artifact gates green, ruby `Syntax OK`, npm test 0 fail, plugin-version green (CHANGELOG marker block already present at this base — no db6b7626-style remedy), locked-cargo clean, `cargo deny --all-features check` ok (advisories/bans/licenses/sources; 6 benign pre-existing duplicate-crate notices; lru 0.18.1 not flagged). Skipped exactly as CI does: TUI real smoke (repo var unset).
- Digest `validation:v1:6b5b0f3c703e93a1e584315877f9fffaf356da958ce22ac6110d378293f5f50a` declared verbatim at both validation turns; each re-derived live with the engine's own `validation_digest()` at the programmatic full-sha base, byte-identical (crates/** + .md deltas are digest-immobile under the classifier — the compound edits cannot move it either).

## In-phase defects caught and disclosed inside implement (ed48face)

1. The discovery regular-file gate first landed in `walk_session_files_cached`'s cline-task branch off a mangled anchor read — it would have dropped cline task directories. Caught while chasing the fifo-test failure (the test initially "passed" against the wrong-site build); reverted and placed correctly in both collectors; the mkfifo test is red→green at the right site.
2. While cleaning fifo-test residue, `rm -rf testdata` hit a **tracked** fixture tree (untracked-residue misread). Immediately restored via `git checkout -- testdata/`; final porcelain verified = intended files only. Prevention rule minted: `docs/solutions/workflow-issues/rm-rf-needs-git-ls-files-guard.md`.

## Provider-failure pattern this cycle (three reaps, zero lost work)

2d667579 (assess, ~23s), 4e83920e (research, ~700s — raw trail adopted), 9b1c07aa (targeted_tests, ~40s — tree-identity check proved zero durable work). Each handled by forensics-then-adopt-or-redo: event log + typed-result presence + tree state before touching anything.

## Status flips and id landscape

- rm-569 / rm-573 / rm-389 / rm-212 flipped `candidate → implemented` with EXECUTED addenda on each row. `done` stays reserved for the commit gate (rm-012). rm-389 carries an honest remaining-scope rider naming the real open arm: the NaN/inf **filter** surface (`--health '<nan'` passes all rows rc0). *(Corrected 2026-10-07, review round 2: this line and the deferred queue below first claimed the NaN/inf threshold arm was not implemented — false; the pre-existing rm-346 arm-b validation already rejects `--baseline-max-cost-delta-pct nan` rc=1.)*
- ZERO ids minted at compound. Census after flips: 203 ids = 122 candidate / 44 implemented / 37 done. Agenttrace-space live bands top at this wall's rm-575; next free ≈ rm-576 — re-sweep live at any future mint (foreign dashboard rm-6xx numerals are excluded by content-scoping).

## Cycle-3 deferred queue (priority order from prioritize 4e0cfbd1)

1. rm-570 (74.0) — cost_audit false-drift on recorded-cost sessions; PoC `stored 0.07` vs `current 0.1298` (`/tmp/at-research-de96/poc-rec-cost/` is sweep-volatile; recipe in the research memo).
2. rm-571 / rm-572 / rm-574 / rm-575 — this run's unselected mints.
3. rm-389's NaN/inf **filter** arm — non-finite `--health/--cost/--tokens/--duration` operands pass/fail every row by comparison semantics (recorded on the row's 2026-10-04/2026-10-05 addenda; no id minted).
4. Pool leads rm-421 / rm-251 / rm-195 @90.0 — verify-first and dependency deferrals per the selection dossier (`/tmp/at-research-de96/selection-2026-10-06.md`, sweep-volatile).

## Commit-gate seams

- ONE commit for the whole batch: 6 code files + ROADMAP.md (mints + 4 flips + compound note) + CHANGELOG.md rider + this record + the prevention rule. Done-flips for the four rows at the gate, **by title**.
- Sibling 4ffc4fbb's uncommitted lane also touches CHANGELOG.md (pi/schema surfaces) — reconcile riders by content, not position.
- The implement envelope's clippy evidence line renders without the `--` separator; the separator form (`cargo clippy … -- -D warnings`) is the runnable one.
- Handoff patch for the code delta: `/tmp/at-research-de96/batch-impl-ed48face.patch` (sha256 4802ac65f14f…; /tmp is sweep-volatile — the worktree is the authoritative copy).

Review and shipping outcomes deliberately absent here — they land after the compound phase; the next cycle's assessment carries them forward.

## Review round 2 correction (2026-10-07, independent_review 0d7decc3)

Round-2 review verdict NEEDS_CHANGES; this pass fixed the actionable findings in-tree and corrected the records:

1. **MF1 non-unix build break (rm-212):** `admit_session_path` was `#[cfg(unix)]`-only while called unconditionally, **and** the real `special_file_kind` was un-gated (its `std::os::unix::fs::FileTypeExt` import is an E0433 on non-unix even with stubs elsewhere — a gap beyond the round-2 memo's own framing). Both paired now; verified by a shimmed-cc `cargo check` for `x86_64-pc-windows-gnu` plus the full linux battery.
2. **MF3 clippy `--all-targets`:** history.rs test used `&[session.clone()]` (cloned_ref_to_slice_refs, rc=101 under `--all-targets -- -D warnings`) → `std::slice::from_ref(&session)`.
3. **MF2 false remaining-scope rider (four sites, not three):** ROADMAP.md:16 compound banner (missed by round 2), rm-389's EXECUTED rider, this record :27, and the deferred queue :34 — all corrected in place to name the NaN/inf **filter** surface as the open arm; REVIEW-FIXED lines added on the row.
4. **LF4 rc token:** rm-569's acceptance said rc=2 for baseline input errors; delivered rc=1 (bail → main's exit(1); rc=2 reserved for the usage shim / gate breach / -d request-wrong lanes). Token amended in place + REVIEW-FIXED line.
5. **LF5 fold breadth disclosed (behavior unchanged):** `same_derived_session` folds any same-tuple row under a different id (model/end/cost/health excluded) — disclosed in the function's doc comment and on the rm-212 row rather than scoped to legacy rows.

Open (recorded, no row): the NaN/inf filter arm above, and `--baseline <fifo>` (special-file baseline operands hang the loader — same class rm-212 fixed for positional paths; pre-existing at base).

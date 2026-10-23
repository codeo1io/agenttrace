# Cycle 1 compound record — run e90edd6c57a74e9f8f13bf7ddde89244 (repository-maintenance e763aec1966c4e2e962778a3351d21f2, cycle 1)

- **phase**: compound (action `compound:compound`, attempt `3630cc0d9fb94ca7a58cb7a685ebd216`, **adopting the reaped attempt `3ee03f342a4e439e8f42a633b7fd1e4a`** whose durable work survived and verified — see "Adoption verification" below)
- **date**: 2026-10-23
- **worktree**: `run-e90edd6c57a7-e90edd6c` @ fb8b5b673e6a01a0559fb679ef2f9f2340315f56 (HEAD never moved all cycle); at compound close the tree is the 17-surface implement batch (16 M + new `tests/pricing_refresh_truth.rs`, byte-stable since implement) **plus** `ROADMAP.md` M (+29/−0) and two new record docs (this file + `docs/solutions/workflow-issues/pricing-snapshot-refresh-discharge-recipe.md`)
- **roadmap chain of custody**: HEAD blob `a58c1070` → band-applied `f93f2bf1` (cycle band 17779a824, authored spool-side against wall 2fb20e06 and re-anchored in-tree by compound) → **compound postimage `8f741783486b4bc0fd85446422a67115`** (live worktree `ROADMAP.md`). Round-trips proven byte-exact: band patch → `f93f2bf1`, + compound-only patch → `8f741783`, and the single-shot chained patch → `8f741783`. Census 491 def rows / 0 dups / def-max rm-2349 / managed footer last, in == out.
- **deviation (declared in-banner)**: the roadmap phase delivered its band spool-side (porcelain 0); compound landed it IN-TREE. Justification: this run's spool sweeper ate every phase scratch dir and 5 of 6 `~/.cache` mirrors mid-run (only `at-roadmap-17779a82` survived), so the un-landed band had exactly one durable home left; the in-tree copy is the pre-gate home of record. Precedent: 5c144613 c4 / f923a62a c3 (in-tree) against cd7c74de c1 / a0cc78e9 c2 (spool-side) — both postures engine-accepted.

## Disposition chain (pre-review evidence consumed as recorded; nothing re-validated locally)

| phase | attempt | outcome |
|---|---|---|
| assess | 855e1234 | whole-repo adversarial baseline at fb8b5b67: clippy 0-warn, fmt rc0, 1489 tests 0 failed; F1–F8 (F1 P2 stale catalog w/ zero gemini-4* keys; F2 P2 gemini resume-twin double-count = rm-715; F3 P3 scan-cap disk/decompressed asymmetry; F4 P3 install.ps1 arch drift = rm-550/#314; F5 P3 zh-CN parity; F6 P3 runner-image readiness; F7 P3 monoliths; F8 P3 one landing behind) |
| research | 9d7cb7a2 | 16 live web fetches + adoption audits: upstream frozen 15ed07f2; fork ahead (#311/#312/#316/#317 in, #314/#315 gap); live 4,245 vs bundled 3,130 (+1,581/−466); gemini-4-argon prefix-only; ladder is suffix-only; C1–C8 |
| roadmap | 67956879 | REAPED (+10s provider reap, 3 events) — zero durable work, redone |
| roadmap | 17779a824 | band authored + verified spool-side: mints rm-2348/rm-2349 + 5 riders; census 489→491 def rows |
| prioritize | 7093c792 | batch 'pricing-truth discharge + scan-cap fairness' selected (rm-176 refresh + rm-2348 + rm-1533 arms + N1 + rm-2349); cut-line P1+P2+P3 indivisible, drop order P4→P5ii; prior scratch swept, rebuilt from typed results/*.json |
| stewardship | 5d513c55 | REAPED (+72s) — zero durable work, redone |
| stewardship | 957f6920 | contract authored for the selected batch, incl. the Mon 2026-10-26T06:00Z drift-cron deadline |
| implement | e4b4e245, 1a328ffe | both REAPED (provider); 1a328ffe left a 17-file WIP tree; its battery3 'argon red' was a STALE-BINARY artifact of a shared CARGO_TARGET_DIR, not a code signal |
| implement | bea8b18c | ADOPTED 1a328ffe's inherited WIP after hunk-by-hunk audit vs the recovered contract, completed the red legs; snapshot 3130→2805 (+141/−466, drops all dated), bare-alias injection in, 1418/1419 arms in, zst cap in; core 494/0; fmt/clippy rc0; drift selftest 9/9 + live rc0 |
| targeted_tests | 351718b0 | focused battery 546/0 across ten touched suites + drift selftest 9/9 + live rc0 (removal_guard clear) + fmt/clippy rc0; digest engine-equal |
| full_tests | 40fc6626 | 96 suites / 1495 tests / 0 failed / 0 errors; digest `validation:v1:e26eb91923b1f4265dbc3848c11f1b939c2a0e86aeb89d38b5ce1b854ac853e6` engine-equal, declared verbatim; porcelain byte-identical pre/post |
| compound | 3ee03f34 | REAPED at +1662s (serve-hold ceiling 1800s, provider family) AFTER completing the in-tree band landing + compound delta — durable trail: scratch (pristine/band-applied/compound-only patch/builder) + the live worktree ROADMAP.md; envelope never written |
| compound | 3630cc0d | THIS PHASE: verified and adopted 3ee03f34's work (legs below), completed the missing docs + mirrors + envelope |

## Status updates delivered (in the live worktree ROADMAP.md, postimage `8f741783`)

1. **Compound banner** at the head of Open items (newest-first stack): cycle outcome summary, the in-tree-landing deviation declaration, flip/rider inventory, commit-gate seams, next-cycle context.
2. **rm-2348 candidate → implemented** (+VALIDATED rider): arm (a) refresh-side BARE_ALIASES injection in `update-snapshot.sh`, fail-closed rules, `_snapshot.bare_aliases` disclosure, drift-check arm-9 mirror, red-first pin `pricing_refresh_truth.rs` 4/0; ladder-side arm (b) stays rejected. done-flip reserved to the commit gate (rm-012 convention).
3. **rm-2349 candidate → implemented** (+VALIDATED rider): per-file zstd decompressed admission cap at `search.rs` admission — dedicated `DecompressedBudgetExceeded` verdict + `zstd_bytes_refused` counter charged before the global budget; red-first twin pair in `search_transcript_contract.rs` 11/0; rm-1872 seam untouched; svg goldens byte-stable. done-flip reserved to the commit gate.
4. **rm-176 VALIDATED rider** (no flip — row owns the recurring refresh workflow): the discharge EXECUTED before the 2026-10-26T06:00Z cron — 3130 → 2805 rows @ 2026-10-22, +141/−466 all dated in `snapshot-drops.txt`, live drift rc0 removal_guard CLEAR; the prior removal_guard-rc1 prediction is superseded; joint unblock with rm-2348 puts bare gemini-4-argon in the shipped bundle.
5. **rm-1533 arm-rider** (row stays open): the 1418/1419 identity arms EXECUTED (`parser.rs` + `lib.rs` `antigravity_identity_family`); the joint unblock specified by the 2026-10-22 evidence rider is COMPLETE; residual OPEN on this row: the unmapped-id WARN/exclusion clause.

## Adoption verification (attempt 3630cc0d → 3ee03f34, all legs first-hand)

- **Identity**: event log `events/3ee03f34…jsonl` = same run/phase/action lineage, 16 events ending `session_reaped` (provider serve-hold ceiling, 1662s) — a transport death, not a work verdict; no envelope, no `phase_result` in `results/3ee03f34…json`.
- **Round-trips**: pristine scratch copy == HEAD blob (`a58c1070`); band patch (~/.cache/at-roadmap-17779a82) on pristine → byte-identical to band-applied copy (`f93f2bf1`); compound-only patch (headers normalized) on band-applied → byte-identical to the live worktree `ROADMAP.md` (`8f741783`); single-shot chain pristine → final byte-exact.
- **Census**: live postimage 491 def rows / 0 dups / def-max rm-2349 / footer last; banner + both flips + 3 VALIDATED riders + 1 arm-rider present.
- **Content cross-checks vs typed records**: targeted battery 546/0 (ten suites enumerated in results/351718b0), full 96/1495/0 + digest e26eb919… (results/40fc6626), snapshot census +141/−466 @ 2026-10-22 (results/bea8b18c), stale-binary attribution for the reaped attempt's red (results/1a328ffe battery3 + implement record), next-cycle leads rm-715/rm-550/PR#30/F7 (results/7093c792 verbatim).
- **Fog numeral noted**: the banner's "rm-2336" (macos-14 brownout 2026-10-23T14:00Z) is NOT a landed row at fb8b5b67 nor at current origin/master 87f18ebc — it is prioritize's fog-census citation carried forward verbatim; reconcile by title, never numeral (rm-708 rule).

## Commit-gate seams (this cycle's landing unit)

- Stage as ONE unit: the 17 batch surfaces + `ROADMAP.md` + this record + the knowledge doc. The banner states the same.
- Reconcile against then-tip origin/master (87f18ebc at full_tests): landings overlap `pricing.rs` / `pricing_snapshot.json` / svg goldens / `mcp_server.rs`; the mcp_server 23→20 count delta is base-tree motion, not a regression.
- Re-derive the validation digest after any rebase — never copy `validation:v1:e26eb919…` across one.
- done-flips (implemented → done) reserved to the commit gate, rm-012 convention.
- Fog rules unchanged: reconcile by title, never numeral (rm-708).

## Reaper / sweeper ledger (prevention context)

- Reaped this cycle: roadmap 67956879, stewardship 5d513c55 (both redone from scratch — zero durable trail), implement e4b4e245 + 1a328ffe (adopted-and-completed), compound 3ee03f34 (adopted here). All forensically adjudicated; zero work lost.
- The spool sweeper ate every phase scratch dir of this run plus 5 of 6 `~/.cache` mirrors (only `at-roadmap-17779a82` survived); later phases rebuilt whole from the typed `results/*.json` records. The in-tree band landing is the direct prevention.
- Recipe doc: `docs/solutions/workflow-issues/pricing-snapshot-refresh-discharge-recipe.md` (ce:compound knowledge doc for the rm-176/rm-2348 discharge method).

## Next-cycle candidates (concrete, from prioritize 7093c792 + assess 855e1234)

1. **rm-715 gemini resume-twin fold** — deferred lead, high value, no deadline; two failed sibling attempts need room (assess F2: `parse_gemini_chats_jsonl` dedups by message id only; resume journals double-count).
2. **rm-550 install triad #314+#315** — pwsh IS available locally, verifiable after all; zero deadline.
3. **PR#30 lock cures** — recheck master-CI at then-tip + PR#30 head first.
4. **Fog CI titles** — macos-14 brownout window opened 2026-10-23T14:00Z (rm-2336, fog-cited); glibc rm-2342; zh-CN rm-1062; sibling mints rm-2344..2347 in flight.
5. **F7 structural** — parser.rs 14,130 / session_cache.rs 6,624 / statusline.rs 5,236 monoliths; ROADMAP.md 3,869 lines ledger-buried; CHANGELOG all under Unreleased.

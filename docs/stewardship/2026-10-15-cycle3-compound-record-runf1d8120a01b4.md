# Cycle 3 compound record — run f1d8120a01b44fadb014f53bbf62683e (repository-maintenance e2b222adc9784aa2af0335184dcd1a6d cycle 3)

- **Date:** 2026-10-15 · **Base:** c0da781 (worktree `run-f1d8120a01b4-f1d8120a`) · **Compound attempt:** d7ceadbc23984e09ad776e846211ef9b
- **Status:** cycle compounded pre-review; validation outcomes consumed from the recorded envelopes, NOT re-run (compound contract). Review and shipping outcomes land after this phase; the next cycle's assessment carries them.
- **Live wall at compound:** `ROADMAP.md` md5 60d023e3af3e79a04e66db85298bfea0, 388 def rows, def-max rm-1369, zero dup ids — unchanged from dispatch through this phase (compound delivers spool-side per the enforcing hygiene clause; the commit gate lands the delta).

## Phase ledger

| Phase | Attempt | Outcome (recorded, consumed here) |
| --- | --- | --- |
| assess | 01a86bb8d53e421f9eca1e40105f2f7f | green baseline (fmt/clippy rc0; cargo test --locked 1094/62 binaries rc0), stale-/tmp false-green PoC, install-runtime test-D root cause; prior a9fc0078 boot-death reap, nothing adopted |
| research | 5b409696ca61488d9b5c858ce2c92dd4 | same-day delta pass vs dossier 38a58688 + hunting; 24 raw files; prior c1d84176 boot-death, nothing adopted |
| roadmap | 0c44e39937bd48b4bde9f2a459d5615e | band = 1 banner + 16 riders + mint rm-1440, +24/0 18 hunks spool-side; prior fd105a13 provider-reaped, nothing durable; **scratch since janitor-reaped — regenerated at compound from the surviving typed envelope** |
| prioritize | 454001f93d594140926ee3b113e80437 | batch 'gate truth: kill the shared-/tmp false-green' — LEAD rm-377 SOLO |
| stewardship | 300db8333ae94d0493c752114521acb2 | typed stewardship_request; MATERIAL sibling-overlap seam recorded (run-e62e2630) |
| implement | bd55834be041407e98c7dcf0bd9d35d1 | 3 gate scripts +30/−3, diff md5 30f8cccb79cbfd7343a5bbb93ae4834a; adopted its own prior attempt's byte-verified delta; fold rejection repaired by declaring changed surfaces |
| targeted_tests | a520bff339d84c79818c44d580a12daa | bash -n ×3 + gates rc0 ×3 under env -u AGENTTRACE_BIN, bash -x trace proves repo binary; prior fb828f8a folded for action_id mismatch (see L1) |
| full_tests | 7aa31bf53f7949e7bb3ed58cd5771edc | 27 green lanes, ci.yml VERBATIM (empty full_command); 1108/0/60 test lane; 3 standing reds proven delta-untouched |

## The cycle in one paragraph

The cycle killed a **false-green class**: `scripts/ci/check-output-contract.sh`, `check-deterministic-output.sh`, and `check-report-semantics.sh` all defaulted `bin="${AGENTTRACE_BIN:-/tmp/agenttrace}"`, so CI validated whichever sibling build last touched the shared path — this run's assess proved it live (an Oct-13 13:04 stale build passed the battery rc0 on today's tree). The implemented delta (uncommitted, awaiting the commit gate) derives the repo's own `target/release/agenttrace` via `git rev-parse` (mirroring `check-docs-commands.sh:13-15` / `check-rust-real-cli-smoke.sh:6-7`), keeps the `AGENTTRACE_BIN` override, and fails with a **named error** when no built binary exists. Red-proofed three ways at implement: named-error rc1 ×3, counterfactual `AGENTTRACE_BIN=/tmp/agenttrace` rc0 on the foreign stale build (the exact false-green, recorded live), poison-stub exit-7 marker `poison-stub-bd55` ×3 at the derived default. Validated end-to-end: targeted battery green ×3 with a `bash -x` trace proving `bin=<worktree>/target/release/agenttrace`; full suite 27 green lanes (fmt, clippy `-D warnings`, 1108/0/60, msrv 1.89, conformance 2/2 + 13 cases, 9 smoke lanes including the three delta gates under the CI exported-binary shape, actionlint 1.7.12 pinned script+sha).

## Where the work lives (pre-commit)

- **Code (in-worktree, uncommitted):** `scripts/ci/check-output-contract.sh`, `scripts/ci/check-deterministic-output.sh`, `scripts/ci/check-report-semantics.sh` — porcelain = exactly these 3 `M` entries, `git diff` md5 30f8cccb79cbfd7343a5bbb93ae4834a, byte-stable across implement → targeted → full → compound.
- **Roadmap/compound wall delta (spool-side):** `delegate/d7ceadbc…-scratch/roadmap-compound-delta-d7ceadbc.patch` — CUMULATIVE (supersedes-and-subsumes the reaped 0c44e399 patch): 2 banners (compound + roadmap, regenerated declaration in the roadmap banner), 16 dated evidence riders anchored at their owning rows, mint rm-1440 at the wall foot, the rm-377 candidate→implemented flip + compound VALIDATED rider, and this record as a new file. Postimage md5 7e5500fc44c827c44dbb772f6eac099c, 389 def rows / def-max rm-1440 / zero dup ids / managed footer last non-empty line / every rider inside its owning row's block / exactly one status flip.
- **Durable mirror:** `/home/agent/.hermes/conductor-delegate-spool/delegate/d7ceadbc23984e09ad776e846211ef9b-scratch/` (builder, base, postimage, corrected patch, this record, evidence log) — the `.cache` backup was swept before review and is NOT relied on; once this record lands in-tree, THIS committed file + the roadmap band are the durable authority — scratch under the spool root and /tmp sweeps within hours; this run lost research/impl/targeted scratch that way.

## EXECUTED status written into the wall (via the patch)

- `rm-377` **candidate → implemented** (done-flip reserved to the shipping gate, rm-012 convention), with the compound VALIDATED rider citing: implement red proofs, targeted rc0 ×3 + trace, full 7aa31bf5 27-lane tally, and the byte-stable diff md5.
- Mint **rm-1440** 'SQLite snapshot reuse requires db+wal+shm fingerprint equality — every live-written database re-queries in full' (performance 70.0, session_cache.rs:1672-1676 + session_cache.rs:3464) — regenerated verbatim-equivalent; original patch 6cac7720…/postimage 2cf198b1… were reaped before compound.

## Prevention rules / reusable lessons (first-hand this cycle)

- **L1 — strict-identity artifact channel:** the typed artifact's `action_id` must equal the work order's verbatim. `tests:targeted` ≠ `targeted_tests:targeted_tests` silently discarded the file and produced an evidence-less fold ("without validation evidence"). Root cause established by direct engine code-read (`scripts/conductor-delegate-spool-server.py::_phase_result_for_order`), not by retrying. Verify the identity before re-running anything.
- **L2 — reaped-scratch band recovery:** a roadmap band's rider list, anchors, and gists are recoverable from the phase's **surviving typed envelope**; riders re-insert **anchored** at each owning row's block tail (def-row id line → walk to the next heading), never at fixed line indexes. The regeneration is declared in-band and the new delta supersedes the reaped patch. This record is the second successful application of the recipe.
- **L3 — scratch sweep cadence:** /tmp and spool scratch sweep within hours; envelopes survive. Durable evidence belongs under `/home/agent/.cache/at-<phase>-<id>/`. (Lost this run: `/tmp/at-assess-*`, `/tmp/at-research-5b40`, `/tmp/at-impl-bd55`, `/tmp/at-tgt-a520bff3`, and the 0c44e399 roadmap scratch.)
- **L4 — the false-green class:** a gate that defaults to a shared /tmp binary validates whichever sibling built last. Gate truth = repo-root-derived binary + named no-binary error + poison-stub red. The three lanes now pin this shape; the same audit posture applies to any new gate script (lint-* gates already follow it).
- **L5 — dead-attempt triage:** five-for-five this run (a9fc0078 + c1d84176 boot-deaths 'No module named openai'; fd105a13/fb828f8a/1a2a172e provider reaps). Verify the durable trail (typed artifact, event log, scratch, worktree diff), then adopt or redo from scratch — and declare which.
- **L6 — porcelain byte-stability:** `git diff` md5 30f8cccb… identical through implement → targeted → full → compound; re-verify at every phase boundary before declaring state.

## Context for the next maintenance cycle (pre-review starters)

1. **rm-012 / test D standing red** — root cause pinned this cycle: stale `ref=v0.9.0` literals at `scripts/ci/check-install-runtime.sh:113/:152-153/:197-199` vs `install.sh:13` `REF=v0.10.1`; `git show ecf84dc:…` carries all 3 (fix never landed canonical). The script is dirty in 4 sibling worktrees (run-79fe78c0, run-966bd5f9, run-a64735b7, run-c2f2765c) — **title-reconcile before re-fixing**. Fix shape: derive the default ref from install.sh.
2. **rm-1054 gate-lane residual** — `check-output-contract.sh:26` runs `--doctor -f json` unsandboxed (operator-corpus walk + operator-global cache banking inside CI); the app lane is verified fixed at doctor.rs:624-625.
3. **rm-1440 (new)** — WAL-aware snapshot validity (db+wal+shm triple fingerprint) or a per-DB never-validated counter; mutes rm-967's 64 MiB byte cap at real corpus scale.
4. **rm-501** — 6 ungated `cfg(unix)` fns remain (down from 14).
5. **rm-1065-class residual** — `csv_export.rs:10-12` fixed fixture dirs (same shared-dir/O_TRUNC class otel_export closed).
6. **Standing reds** — cargo-deny 4 rustls 0.23.45 advisories (sibling-owned cure; Cargo.lock untouched) and actionlint SC2001 `ci.yml:192`.
7. **Watches** — gemini-cli 0.64.0 stable (chats-record recipe banked at rm-715); upstream `luoyuctl/agenttrace` frozen at 15ed07f2 since 2026-10-06 (20th check); models.dev flat (api.json md5 3d70bf7f…, 228 providers / 8,814 models / 639 cost.tiers / 0 negative cells; LiteLLM 0-drift; 1,420 live-only gap stable); codex state migrations 46/59/61 mined (threads 99×36 + thread_spawn_edges + rollout_path join — folded under fog rm-1027/rm-1034, no mint).

## Commit-gate seams

1. Apply `roadmap-compound-delta-d7ceadbc.patch` onto the **pristine 60d023e3** `ROADMAP.md` (verify postimage md5 7e5500fc44c827c44dbb772f6eac099c after apply; the patch also adds this record file).
2. Stage **exactly**: the three gate scripts + `ROADMAP.md` + `docs/stewardship/2026-10-15-cycle3-compound-record-runf1d8120a01b4.md`. Never `-A` (the fleet shares `.git`-adjacent state; sweep before staging).
3. **Sibling reconcile BY TITLE:** worktree `run-e62e2630df8f-e62e2630` @794b6e5 is dirty in the SAME three gate scripts implementing the same subject as an 'rm-1419 rider' (adds the /tmp/agenttrace-ci out-dir arm + a doctor-leg re-scope citing rm-444). One landing wins; union the out-dir/doctor-leg arms if they compose. Do not double-land.
4. rm-377's **done-flip stays reserved to the shipping gate** (rm-012 convention); the implemented flip + VALIDATED rider ride this patch.
5. Numeral ledger: rm-1440 fleet race documented (bad1a64e/foreign run b9963213 minted rm-1440-1443 title-disjoint; fleet frontier has since advanced past rm-1510 on foreign lineages). Never renumber post-hoc; reconcile by title. Fresh census at landing for the next free numeral.
6. CHANGELOG riders for the batch stay reserved to the gate (shipping artifact), consistent with cycle 2.

## Verification appendix (this phase)

- Base copy asserted md5 60d023e3af3e79a04e66db85298bfea0 before build; postimage written by `build_compound.py` with in-script asserts (def rows 389, def-max rm-1440, zero dups, footer last non-empty line, rider-in-own-section ×16, exactly one flip).
- `diff base postimage`: 29 added lines, **1 replaced line** (the rm-377 id flip), zero other removals.
- Round-trip: `git apply` in a fresh scratch repo and `patch -p1 -F3` on a pristine base copy → postimage md5 both ways (see `compound-evidence.md`).
- Worktree untouched by this phase: `git status --porcelain` = exactly the 3 `M` gate scripts; `git diff | md5sum` = 30f8cccb79cbfd7343a5bbb93ae4834a; `md5sum ROADMAP.md` = 60d023e3af3e79a04e66db85298bfea0 at finish.
- No test execution: zero cargo/pytest/script invocations this phase (compound contract).

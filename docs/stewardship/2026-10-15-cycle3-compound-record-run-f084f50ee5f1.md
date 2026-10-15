# Cycle 3 compound record — run f084f50ee5f14cbd84bf677f8361ad0b

**Repository:** agenttrace (`/work/projects/agenttrace`)
**Cycle:** repository-maintenance `8decedb780b04a3684047328c37e2945` cycle 3
**Base:** worktree HEAD `c0da781` (detached; `== origin/master~2` at dispatch — master has since moved repeatedly, see §5)
**Batch:** "discovery truth + gate truth" — 3 units, 8 files, +249/−11 (diff md5 `0e53f634c52d338f4af2c0110757817c`, byte-unchanged from implement through full validation)

## 1. What shipped

| Unit | Row | Outcome |
|---|---|---|
| LEAD — OpenClaw session discovery roots | rm-1373 (arm) | `known_session_dirs()` gains `OPENCLAW_STATE_DIR` → `~/.openclaw` (+ legacy `~/.clawdbot`); `sessions/<provider>/<uuid>` wrapper JSONL routed through the existing `parse_anthropic_message_wrapper` lane; synthetic fixture per the documented wrapper shape; `discovery_contract` 107 green + `docs_discovery_roots` contract pins the README:32 claim; no twin on master (0 `openclaw` refs in `discovery.rs` at `d2a02af2`) |
| PAIR — TMPDIR test hermeticity | rm-241 | the three lanes own their roots (insights `:1159`/`:1278` fixture `.git` boundaries; upstream `outside_a_git_repository_fails` via `run_cli_ceilinged`) + self-planting (runtime `git init`) hostile-ancestor trap with live liveness assertion; **flipped implemented (pre-review)** |
| RIDER — docs-gate honest not-found | rm-369 (arm) | `check-docs-commands.sh` three-arm split (missing / not-executable / present), names `AGENTTRACE_BIN` + build step; gate green in full battery under isolated fake HOME |

## 2. Validation record (pre-review)

- **Targeted:** all focused lanes green under BOTH TMPDIR directions (`/tmp` and a git-repo TMPDIR); the exact pre-fix panic site (`upstream.rs:257`) green in the hostile direction; `bash -n` + AGENTTRACE_BIN-absent arms rc1 with honest messages; release binary embeds `OPENCLAW_STATE_DIR` (strings probe).
- **Full battery (attempt 1d172e41):** 62 suites, **1112/0/0** — run under a TMPDIR *inside a git repo*, i.e. the hermeticity acceptance proven at full-suite scale. fmt rc0; clippy `-D warnings` rc0; MSRV 1.89 check rc0; token-conformance 2/2 + 13-case runner; release build; output-contract / deterministic-output / report-semantics / example-workflows / **docs-commands (the batch's changed executable surface) green**.
- **Standing reds (delta-untouched, disclosed):** install test D (rm-051) and cargo-deny 4× rustls RUSTSEC advisories — both byte-identical to clean-HEAD observations from this run's own assess phase.
- **Validation digest** (engine-derived, executable set = `scripts/ci/check-docs-commands.sh` only): `validation:v1:e25101d1f730349090c27a507dfc64bf3233b4191fcbacc1b6a0d95f2cb246cb` — docs added by this compound phase are non-executable and leave it unchanged.

## 3. Roadmap band

Delivered spool-side (worktree is behind; an in-tree band on the stale local wall would regress landed rows). `delegate/a72ec022…-scratch/roadmap-update.patch` targets **`origin/master:ROADMAP.md @ d2a02af2`** (400 rows, def-max rm-1502, md5 `695d7ee4`): cycle-3 compound banner (newest-first under `## Open items`), 11 dated riders (7 research/assess riders dated 2026-10-14 retained verbatim + 4 compound outcome riders), **mints rm-1397 (session org-identity axis) / rm-1398 (codex state-surface names)** — allocated 2026-10-14 at wall-max 1373, re-censed still-free at d2a02af2 (siblings landed 1401+; no title twin) — and the rm-241 pre-review flip. Postimage md5 `69a95146` (regenerated at review_fix b197ffbe, wording amendments in §7), 402 rows, apply-verified byte-identical on a pristine detached worktree at d2a02af2. **Supersedes the reaped fb44ae98 delta** (3e90993 base). Apply order + anchor inventory: `APPLY-SEAM-COMPOUND.md` in the same scratch.

## 4. Attempt forensics (this cycle, for the fleet ledger)

Five provider-dead/reaped attempts across phases, all with typed artifacts absent, all redone from scratch with declared forensics: assess `ca6580a8` (1-line `session_reap_failed`), research `f5a3f236` (3 message_progress + reap), targeted `fccde82cb` (2 assistant messages + reap, ~27s), full_tests `ba29f2fc` (heartbeats-only reap mid-battery — its partial battery independently reproduced the redo's 62-suite/1112 count), plus one **KTD13 fold rejection** on implement (missing `validation_evidence.changed_surfaces`) resolved by an attestation-only redo (`af88a331`) with the delta inherited byte-unchanged. The reaped roadmap scratch (fb44ae98) was recovered for this compound from its typed envelope + archive — per the established rule, never from memory.

## 5. Fleet-race observations (this cycle's new traps)

1. **Wall velocity:** origin/master moved three times inside this run (3e90993 → e130845c → d2a02af2, now 19+ commits incl. the a647 integration merge and the independent rm-1053 pricing twin from run f2fccaa9 — this run's PoC D corroborates its diagnosis and hands assess F2's window-ladder finding forward as its follow-through candidate).
2. **Shared CARGO_TARGET_DIR shadowing** (new, cost a triage detour in full_tests): the engine-shared target cache at `/home/agent/.cache/conductor-cargo-target` served a sibling's mtime-fresh `entrypoints-*` test binary against our sources → a phantom failure in a test that does not exist in our tree. Rule: full/validation batteries run with a private `CARGO_TARGET_DIR`; freshness-probe binaries via `strings` when in doubt. Documented in `docs/workflow-issues/shared-cargo-target-dir-shadowing.md`.
3. **Host oversubscription:** fleet load (3 maintenance conductors + sibling batteries) reached 42–53 on 14 cores; gate scripts stall past tool timeouts and reaping follows. Batteries serialize with generous timeouts under contention.

## 6. Next-cycle candidates (pre-review queue)

- **rm-1397** (org-identity axis, 42.0): wild-corpus survey pins the `$.session.identity()` journal carrier before any parser lands.
- **rm-1398** (codex state names, 44.0): conditional on a state-surface ingest; schema rung census first.
- rm-1053 follow-through: `lookup_context_window_in` raw-first-ladder bypass (sibling assess F2, 496/2804 divergent rows).
- models.dev flip taxonomy now 4 classes (growth/correction/shape/**semantic**) — watch continues.
- Fleet-owned standing reds: rm-051, cargo-deny rustls quartet.

## 7. Independent review + review_fix (2026-10-15)

Review `8e4e830a` (fresh re-execution, private CARGO_TARGET_DIR per the fleet rule): **NEEDS_CHANGES — code APPROVED as-is**; every implement/compound claim independently reproduced green (discovery_contract 107/0, docs roots 2/0, upstream 13/0, insights 12/0, hostile-TMPDIR pin, gate third arm rc1-honest, full-log recount 62 suites Σ1112/0). Disposition at review_fix `b197ffbe`:

- **F1 (medium, truth-of-record) FIXED** — "checked-in hostile-ancestor trap" reworded to "self-planting (runtime `git init`) hostile-ancestor trap with a live liveness assertion" in the band rider (regenerated patch; postimage md5 `69a95146`) and §1 above. No checked-in fixture exists: `crates/agenttrace-cli/tests/upstream.rs:286` runs `git init` inside the test and asserts the ancestor stays discoverable un-ceilinged — a stronger pin, now described truthfully.
- **F2 (medium, shipping completeness) FIXED** — `CHANGELOG.md` gains two Unreleased entries (Added: OpenClaw auto-discovery lane; Fixed: docs-gate honest three-arm failure messages); the APPLY-SEAM staging plan now names CHANGELOG.md (9th batch file, non-executable); `check-changelog-structure.sh` rc0.
- **F3 (medium, durability) FIXED** — the reviewer's own mitigation mirror (`~/.cache/at-review-8e4e830a/`) was reaped by fleet disk hygiene; the full band artifact set is re-mirrored AT WRITE TIME to `~/.cache/at-reviewfix-b197ffbe/compound-mirror/` with an md5 manifest. Primary copy stays in spool scratch (reap-exposed only under full-spool sweeps).
- **F4 (low, prose staleness) FIXED** — banner (patch line 13), rm-1373 rider, and mint comment now cite the actual patch base `d2a02af2` (400 rows, max rm-1502, no openclaw twin); the two 2026-10-14 mint-row `- notes:` lines keep their historical "still free at e130845c" allocation facts verbatim.
- **F5 (low, pre-existing, out-of-batch)** — `docs_discovery_roots_contract.rs:39` scratch-dir litter (rm-1022-era, observed live during review) recorded as next-cycle hygiene; not a batch defect, untouched here.

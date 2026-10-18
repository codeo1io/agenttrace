# Cycle 1 compound record — run d02291d0efbb480aa27dfe4f24db47db (repository-maintenance 169ce77ade014ce3ab5c63062478a136)

Provenance: compound:compound attempt b24ad78b0d6e417681edf341a229071, 2026-10-10. Base eafbb592 (worktree `run-d02291d0efbb-d02291d0`, agenttrace-80c75f65b7). This phase honors the compound contract: it consumes ONLY pre-review cycle evidence (assess a38c3665, research 27f31d4e, roadmap 778583d2, prioritize 8dae7b99, stewardship 6fe41d11, implement 17d6a95a, targeted_tests e56750ed, full_tests a0ea1af7) and re-runs NO test or validation command. Review and shipping outcomes happen after this step and are the next cycle's input.

Prior-attempt check (this action): none — b24ad78b is the first compound attempt of this run (0 envelopes, event log = this attempt only).

Run-level dispositions carried forward:

- assess 19cf07ce — provider-reaped, zero durable work (forensics by a38c3665; verified dead before the redo).
- **NEW SHAPE — assess 24289346** (this delegate session's first turn): ran live probes at eafbb592 (a full green `cargo test --workspace --all-targets` 805/0, clippy `-D warnings` rc0, docs gate rc0, hostile-corpus subcommand sweep with zero panics, plus two net-new findings below) but was interrupted before delivering a typed envelope; the engine's authoritative assess is a38c3665. The two orphaned observations are preserved below as **next-cycle CONTEXT only** — no wall row, zero mints at compound.

## What was compounded

1. **Wall flips** — rm-917 LEAD + rm-918 flipped `candidate → implemented` with dated EXECUTED bullets (status field credits `cycle 1 compound b24ad78b; pre-review evidence, commit gate pending`, per the e9dbcbc2 compound convention; done-flips remain reserved to the integration gate, fleet ruling 7eae74ea/415d77e1). Zero mints.
2. **Compound banner** at the head of the banner stack (directly under the standing INTEGRATION comments, above this run's cycle-1 roadmap banner).
3. **CHANGELOG rider** — implement omitted the Unreleased bullets; compound mints 2 `### Fixed` entries (rm-917, rm-918) as a single-layer patch on pristine CHANGELOG.md. The frozen `no-changelog-section` tail block is byte-untouched (asserted in the builder).
4. **This record + prevention doc** (below) as untracked in-worktree paths for the commit gate to stage.

Def-row census over the 359 `^- id:` rows after flips: 171 candidate / 107 implemented / 80 done / 1 open (before: 173 / 105 / 80 / 1). Roadmap postimage 3,827 lines (designation 3,823; base wall 3,821 + designation banner).

## Deliverable chain (commit gate lands these IN ORDER)

1. `delegate/778583d2…-scratch/ROADMAP.patch` (roadmap phase) — applies on pristine ROADMAP.md @ eafbb592.
2. `delegate/8dae7b99…-scratch/ROADMAP-designation.patch` (prioritize) — applies on (1).
3. `delegate/b24ad78b0d6e417681edf3412a229071-scratch/ROADMAP-compound.patch` — applies on (2).
   - postimage `ROADMAP.compound-postimage.md`: sha256 22948ae32dde356fa9c39cefee8dbf57e1d43e470165d0f9df5984215d45668f, md5 4a93dea7b2149e7a99450c05ec00634e
   - patch sha256 7a049e1444d19ab3feb9597c04f5800702954964468107cbe1a9da705a0333d9 (+4/-2: 1 banner + 1 blank + 2 EXECUTED bullets + 2 status-field flips)
4. `delegate/b24ad78b0d6e417681edf3412a229071-scratch/CHANGELOG-compound.patch` — single-layer on pristine CHANGELOG.md.
   - postimage sha256 e19faee19e9f5ca11ed9e0617ba5215d072183a6daf363aa5377dcb1c3903f12, md5 031691113963cb4744014f5388e99469; patch sha256 54c73f1bc598446ec8f845a84b7fe7ace4f85b43e87a5acbddd67ef590d9a50f
5. The 3-file implement batch, uncommitted in the worktree, **POST-fmt pins** (full_tests a0ea1af7 table): `git diff` sha256 acb9b8cb3b1ab604944ee1105191623d9b0786f188bf8087c96e8fe7dc6ca7fd; statusline.rs md5 cb4d02c6babf2136eb443418f90feb12; config.rs md5 6315e032b0eb4af5dc1fb5afbac45707; README.md md5 6d92bc4873423c5bf720a6c1a11ad4bc. (Implement-time pre-fmt pins d5a212ea…/8b4a8438…/0247a64d… are SUPERSEDED — do not gate on them.)
6. Stage the two untracked docs explicitly: `docs/stewardship/2026-10-10-cycle1-compound-record-rund02291d0efbb.md` and `docs/solutions/workflow-issues/implement-batches-must-run-the-fmt-lane-before-delivering.md`.

Verification recipe (sandbox, no worktree mutation):

```
SBX=$(mktemp -d) && cd $SBX && git init -q . && git config user.email x@x && git config user.name x
git -C <worktree> show eafbb592:ROADMAP.md > ROADMAP.md
git -C <worktree> show eafbb592:CHANGELOG.md > CHANGELOG.md
git add -A && git commit -qm base
git apply <spool>/778583d2…-scratch/ROADMAP.patch
git apply <spool>/8dae7b99…-scratch/ROADMAP-designation.patch
git apply <spool>/b24ad78b0d6e417681edf3412a229071-scratch/ROADMAP-compound.patch
git apply <spool>/b24ad78b0d6e417681edf3412a229071-scratch/CHANGELOG-compound.patch
cmp ROADMAP.md <spool>/b24ad78b0d6e417681edf3412a229071-scratch/ROADMAP.compound-postimage.md   # byte-identical
cmp CHANGELOG.md <spool>/b24ad78b0d6e417681edf3412a229071-scratch/CHANGELOG.compound-postimage.md
```

The builder re-proved exactly this chain byte-exact and an INVERSE transform (undo the 4 insertions + 2 flips ⇒ reproduces the designation postimage bit-for-bit), plus header-integrity counts (`+++ b/` and `--- a/` exactly 1 each, per the spool-patch integrity rule).

## Consumed outcomes (NOT re-run at compound)

- targeted_tests e56750ed: core lib 279/0; statusline filter 24/0 ×5 consecutive; cli `config::tests` 9/0; clippy `-p agenttrace-core -p agenttrace --all-targets --locked -- -D warnings` rc0; docs gate rc0. Digest `validation:v1:ee94a43c13ae9c88848c3d4586785510bda6d5c9b31e1e8f6b1391d19c570876` declared verbatim = live re-derivation ×4.
- full_tests a0ea1af7: dispatch `full_command` EMPTY → ci.yml push-suite mirror (fleet precedents 614624d7/7eae74ea/3ec6cec0/ff0068ca/9a4d37af/2023f222). 22 lanes ALL rc0 (2026-10-09T23:52:10Z→2026-10-10T00:09:00Z). Lane-04: **811 passed / 0 failed across 45 suites** = assess baseline 805 + 6 new tests (statusline 19→23, config 7→9 `#[test]` census); lane-06 entrypoints 48; lane-21 cargo-deny tail "advisories ok, bans ok, licenses ok, sources ok". Digest re-derived ×2 = dispatch.

## Lessons / prevention (durable)

1. **Implement/targeted batteries systematically omit `cargo fmt --check`** (clippy + docs gate only), so rustfmt violations in batch code surface only at full-suite lane 01. This cycle: 5 violations in the new test code (statusline.rs :998 :1048 :1094 :1208; config.rs :639), fixed by canonical `cargo fmt`, proven layout-only via whitespace-normalized-diff identity, entire suite re-ran green. Prevention doc: `docs/solutions/workflow-issues/implement-batches-must-run-the-fmt-lane-before-delivering.md`. Corollary: any fmt fix MOVES the batch pins — later gates must consume post-fmt values (this cycle: diff sha256 d5a212ea… → acb9b8cb…).
2. **EMPTY dispatch `full_command` defers to the ci.yml push-suite mirror** — 7th fleet occurrence; the mirror (push suite + PR-lint-unique step) is the authoritative full suite when the field is blank.
3. **`.rs` and `.md` deltas are validation-digest-immobile** (classifier counts only scripts/workflows/.py) — the verbatim dispatch digest remained the live digest through the fmt layout fix AND through this compound's .md additions. Re-derive to confirm; never assume a drift.
4. **Interrupted-session attempts leave orphaned evidence** — a delegate turn that runs real probes but never writes its typed envelope is invisible to the engine's chain; its observations must be explicitly carried (as here) or they are lost. Cross-cutting with the dead-attempt forensics rule (verify zero-durable before redoing).

## Orphaned assess observations (from the interrupted 24289346 turn; UNRECORDED, next-cycle context)

Both were probed live at eafbb592 this run; re-verify at the next base before minting anything (sibling lane 2a5cdb9e reportedly minted a campaign-local rm-872 for the duration item — reconcile by title at the next roadmap).

- **Σ `by_model[].cost` drift vs `summary.total_cost`**: per-block `round4` accumulation (lib.rs:1704 and :1740; helper :1742) rounds each block's cents before summing, while the session total is computed unrounded — a 3-block / 2-model PoC (`/tmp/assess-d02291d0/probe2/multi.jsonl`, `-d … --overview -f json`) showed `total_cost` 0.0002 vs Σ by_model 0.0003 (1e-4 drift). Cross-lane rollups that re-sum the per-model figures will disagree with the headline total.
- **`fmt_duration` minute-carry boundary** (lib.rs:~2394): the `{:.1}m` arm renders 3599.8s as "60.0m" (and the hours arm's `as i64` minute computation carries its own boundary artifacts). Verified with an extracted-copy probe (`/tmp/assess-d02291d0/fmtdur.rs`): 3599.4→"59.9m", 3599.8→"60.0m", 86399.9→"24h 0m".

## Next-cycle leads (prioritize 8dae7b99 ranking + research 27f31d4e folds)

| lead | why next |
|---|---|
| rm-042 (#306 time-bucket port) | BUILDS ON rm-917's calendar predicate; arbitrate unlanded sibling rm-916 by title first; leader #1696 MERGED with matching semantics (TZ-keyed daily buckets + 7-day re-derivation + completeness guard) |
| rm-164 (tier-aware pricing) | after rm-852 lands + upstream schema stabilizes — flex-field census 129→238 in ONE day (C3); 238+ rows mis-billed via base-rate fallback |
| rm-619 / rm-339 | ALTERNATE #1 (opencode reasoning fold) / #2 (waste rebuild attribution), both uncontended |
| rm-421, rm-251, rm-195, rm-232 | the 90.0 scale/decision rows — stay deferred |
| rm-588 | premise FLIPPED by research C2: live LiteLLM main now natively encodes claude-haiku-5-5 `*_above_100k_tokens` (5e-7/2.5e-6) — the 2026-10-08 "unrepresentable" claim is dead; decision arm re-opens as schema-representable |
| rm-841 | HOLD-radar: demand flip (codeburn #1640 closed UNMERGED; #1686 unmerged; #1626 open) |

## Non-goals (this phase)

No review, no final_validation, no commit/push/pr/ci, no test execution, no wall mints, no done-flips.

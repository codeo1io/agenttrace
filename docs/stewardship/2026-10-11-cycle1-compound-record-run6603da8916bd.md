# Cycle 1 compound record — run 6603da8916bd (repository-maintenance 6729a07a)

- **Run:** `6603da8916bd4bb0b54b29c2574b0b5e` · repository-maintenance `6729a07a9f3547aa89edb5ca7729b2cd` cycle 1 · compound attempt `349bc0f6b0334ac3a6067645692c353a` · 2026-10-11.
- **Worktree:** `conductor-worktrees/agenttrace-80c75f65b7/run-6603da8916bd-6603da89` @ base HEAD `428f0e9dd964723a2a3012a678b702c989fb658d` (merge of runs d02291d0efbb rm-917/918 + 0a55a397 accounting-truth + e586a5dc rm-911/912), implement batch present uncommitted.
- **Contract honored:** this phase consumed ONLY pre-review cycle evidence (the eight phase artifacts below) and re-ran ZERO test/validation commands, per the compound no-execution contract. Every number below is cited from a recorded phase outcome, not re-derived locally.

## Prior-attempt check (this action)

Attempt `5101e19945ca4184a78dcc2d86613c05` on this action died of a provider-infrastructure failure before producing anything durable: the event log `events/5101e19945ca4184a78dcc2d86613c05.jsonl` is 7 lines — `delegate_turn_started` → zero output events → reaped at ~37 s — and `delegate/5101e199…​.json` plus any `-scratch/` are absent. Nothing to adopt; this phase was redone from scratch and declares that.

Run-level dispositions of the same class, from the phases' own forensics: assess prior `6d7b5741` (provider-reaped, no typed artifact, no scratch — phase redone) and its reaped twin whose PoCs survived under `/tmp/at-assess-6f69/poc`; full_tests prior `3cdfddae` (delivery-family failure, zero tree delta — tree census matched implement pins byte-for-byte, so nothing to redo).

## Cycle inventory (consumed, not redone)

| Phase | Attempt | Outcome (recorded; not re-executed here) |
| --- | --- | --- |
| assess | `2211373c` | Envelope at HEAD 428f0e9: 869/0 x52 suites, clippy `-D warnings` rc0, fmt rc0. PoC pins: stale-json budget arms, future-dated 2030 svg card, cline unbounded read RSS 30.6 vs 21.1 MiB baseline. Prior dead attempt forensics recorded. |
| research | `d00a37f2` | luoyuctl QUIESCENT (12th+ check; tip 15ed07f2 2026-10-06). ECOSYSTEM: ccusage #1813 Flex pricing open + PR #1814 open-unmerged (parity window); LiteLLM purged openrouter prices from the live bundle; codeburn #1723 plan-value framing; ccusage #1771 Cursor local-files oracle. `agenttrace` npm name still 404. |
| roadmap | `f5bbcc1b` | 3 mints (rm-939 65.0 customer-experience plan-aware spend framing; rm-940 55.0 compatibility Cursor local journal usage lane; rm-941 62.0 correctness future-dated samples) + 10 dated appends + 1 banner; postimage 400 def rows, roundtrip-proven spool-side. |
| prioritize | `3b889186` | Selected batch **'truthful tokens, bounded windows'** (rm-619 LEAD 70.0 + rm-941 + rm-917 residual rider; rm-621 deliberately NOT re-opened) with designation patch + full ranking/claim-state corrections in `prioritization.md`. |
| stewardship | `40a50c8c` | `stewardship-request.md`: batch as structured request, `must_remain_separate` hints, fork lint plan, integration boundaries (README flag-table digest immobility, no-changelog-section tail frozen, docs-gate trap). |
| implement | `62fb8454` | 3-arm batch @ base 428f0e9 uncommitted: M `parser.rs`, M `statusline.rs`, M `reports.rs`, ?? `tests/window_truth_batch.rs` (+189/−4; batch.diff sha256 `983fcb0f…`; 7 new tests; red proofs + live PoCs at `/tmp/at-impl-62fb`). Worktree porcelain otherwise 0; 0 probe entries in the session cache. |
| targeted_tests | `a734c2e4` | `local_validation_gate.py --shell-command 'cargo test'` rc0: **876 passed / 0 failed, 52 suites, 37.5 s** (log + gate envelope `result-802852-414293347.json`); pre/post-leg pin match `983fcb0f…`/`8be4e8bf…` + md5s on all four surfaces (silent-reset watch); digest re-derivation `validation:v1:4d368783…` (base-bearing == dispatch token) and `387dae2b…` (envelope). |
| full_tests | `a38d80f2` | `local_validation_gate.py --shell-command 'cargo test'` (release `1225cda1`, command verbatim from the work order) rc0: **876 passed / 0 failed, 53 suites** (log + typed artifact + envelope `result-1200346-415435696.json`); dispatch-token digest `validation:v1:4d368783…` re-derived exactly; prior dead-attempt `3cdfddae` forensics recorded. |

## What this compound phase changed

1. **Wall flips (candidate → implemented), pre-review:** `rm-619` and `rm-941` flipped with dated EXECUTED bullets crediting `run 6603da8916bd cycle 1 compound 349bc0f6; pre-review evidence, commit gate pending` (the e9dbcbc2 compound convention; done-flips stay reserved to the integration gate). The rm-941 bullet records the DECISION made at prioritize: **tolerance = now + 1h** (clock-skew allowance), and names the DELIBERATE BOUNDARY left by implement (the svg/json corpus-window TimeRange footer still spans to a future session's timestamp — span disclosure, not an aggregate; clamping is the row's next rider).
2. **rm-917 residual discharged in-row** (no status change — the row was already implemented): the post-implementation staleness-parity residual is closed by the same batch (`--statusline-report` json budget now carries `daily[]`/`excluded_older_days`/`excluded_future_days`; text arm prints both exclusion lines).
3. **Zero mints.** The cycle's mints (rm-939/rm-940/rm-941) were made by the roadmap phase; compound consumed them.
4. **Compound banner** at the head of the banner stack (directly under the standing INTEGRATION comments, above this run's cycle-1 roadmap banner).
5. **CHANGELOG rider minted spool-side** (implement emitted no Unreleased bullets): 2 `### Fixed` entries — rm-619 (reasoning-only opencode message tokens) and rm-941 (future-dated samples; the rm-917 parity residual rides its entry) — delivered as a single-layer patch on pristine CHANGELOG.md. The frozen no-changelog-section tail block is byte-identical (last 1731 B md5 `e7a2784e199879d6b9ef5314c87e35dd`, both sides).
6. **This record + two prevention docs** (below) as untracked in-worktree paths for the commit gate to stage.

**Def-row census:** candidate 189 → 187, implemented 126 → 128, done 84, open 1; 400 def rows before and after; no duplicate ids; managed footer still the last non-empty line.

## The commit-gate delivery chain

Apply in this order, all verified to apply with default `git apply` in a clean sandbox git repo (the first two carry non-canonical `diff --git` prefixes — `a/base/…`, `a/pb/…` — but canonical `--- a/` / `+++ b/` headers, one each, per the spool-patch integrity rule; default `git apply` resolves them onto `ROADMAP.md` at the repo root, proven):

1. `delegate/f5bbcc1b46b64a6db915e6a7177c9eba-scratch/ROADMAP.patch` (roadmap: 3 mints + 10 appends + banner)
2. `delegate/3b88918643a7470a9f90658ee83742d8-scratch/designate.patch` (prioritize designation)
3. `delegate/349bc0f6b0334ac3a6067645692c353a-scratch/ROADMAP-compound.patch` (this phase: +7/−2 — 2 flips, 3 EXECUTED bullets, banner)
4. `delegate/349bc0f6b0334ac3a6067645692c353a-scratch/CHANGELOG-compound.patch` (this phase: +2)

**Proofs (all reproduced this phase, in sandboxes under `/tmp/at-compound-349b/`):** forward chain — base `428f0e9:ROADMAP.md` → patch 1 → patch 2 → patch 3 ⇒ byte-identical to `ROADMAP.compound-postimage.md`; `428f0e9:CHANGELOG.md` → patch 4 ⇒ byte-identical to `CHANGELOG.compound-postimage.md`. Reverse — `git apply -R` patch 3 ⇒ md5 `136adcebd6f89a07bb4edd09c82a0d04` (== ROADMAP.prio-post.md pin); `-R` patch 4 ⇒ md5 `7b45876b9ce205d27ee6b55295381147` (== pristine CHANGELOG.md). Pins in `349bc0f6…​-scratch/pins.txt` (sha256 + md5 for every spool artifact, mirrored to `/tmp/at-compound-349b/mirror/` with md5 parity).

The implement batch itself stays exactly as pinned: `git diff | sha256sum` = `983fcb0f89f582b9c4eddaf3c684b83f35f2115ea1584c402d9b2be7ec22900d`, `tests/window_truth_batch.rs` sha256 `8be4e8bf518fde36cd1579f1ea42b3217621a8f93a1b8e263e4682b3c72919a6`, per-file md5s `497943db…`/`bbe9afb0…`/`c3432796…`/`7084d3a3…` (parser.rs / reports.rs / statusline.rs / window_truth_batch.rs).

**README note for review:** the batch adds no flags and changes no flag semantics — the flag-table digest (`check-docs-commands.sh`) is untouched by design; the disclosure fields are document-shape additions inside existing surfaces, documented in the CHANGELOG entries. If review wants README prose for `excluded_future_days`, that is a review-time rider, not a compound gap.

## Durable lessons / prevention rules minted this cycle

1. `docs/solutions/workflow-issues/silent-worktree-resets-require-fix-marker-sweeps-between-build-legs.md` — the implement worktree reverted tracked-file content to HEAD twice mid-phase (HEAD stable, porcelain clean); only a green-compile-then-red-twin caught it. Rule: grep the fix markers between every build/test leg; a red test after a green compile on intent-unchanged code means suspect the TREE first.
2. `docs/solutions/quality-gates/budget-window-fixtures-must-model-cumulative-cost-counters.md` — statusline journal costs are CUMULATIVE per session; the series bills the day-over-day RISE, so fixtures must be monotone non-decreasing; and the two render arms emit different shapes (`render_budget_view` top-level window fields vs `render_statusline_report` nesting them under `budget`) — pin the right shape per arm.

## Context for the next maintenance cycle

| Lead | Why next | Where |
| --- | --- | --- |
| **rm-941 TimeRange-footer clamp** | deliberate boundary left by implement 62fb8454: svg/json corpus-window span still names a future datum (honest disclosure, not an aggregate); small, rides the rm-941 row | ROADMAP rm-941 EXECUTED bullet |
| **rm-042** (ccusage #306 time-bucket port) | builds directly on rm-917's calendar predicate (landed) + this cycle's window work; ARBITRATE unlanded rm-916 by title first | prioritization.md ruling 1 |
| **rm-164 + rm-530** (Flex/tier pricing) | parity window OPEN (ccusage #1813/#1814 unmerged 2026-10-11); after rm-852 lands; tier census: LiteLLM live bundle has `flex` 238 / `service_tier` 0 — the pricing seam is fork-side work | ROADMAP rows; research dossier |
| **LiteLLM bundle purge** | the live bundle purged openrouter prices → next snapshot refresh must NOT honor the drop (rm-588 never-drop union guard becomes load-bearing) | research dossier; rm-588 row |
| **rm-939** (plan-aware spend framing) | cycle-2 LEAD candidate by priority (65.0); codeburn #1723 MERGED — parity pressure is live | ROADMAP rm-939 |
| **rm-940** (Cursor local journal usage lane) | survey-first (SQLite sources per ccusage #1771); the discovery glob already sees the transcripts | ROADMAP rm-940; research dossier |
| **rm-339** (usage-parity gates) | standing alternate #1 (efficiency 60.0) | prioritization.md |
| **by_model round-4 drift + `fmt_duration` minute-carry** | proven twice (assess 2211373c + 6f690259), still UNROWED — mint at the next roadmap pass | assess findings-notes.md |
| Standing deferrals | rm-421, rm-251, rm-232, rm-195 | ROADMAP rows |
| Claim-state warnings | rm-936/rm-937/rm-927-band claimed by live sibling lanes — title-check before minting; e4917d16 review NEEDS_CHANGES (cap bypassed on the cline lane) — do NOT consume that batch as-is | prioritize claim-state corrections |

**Fleet watches carried:** upstream luoyuctl QUIESCENT (12th+ check; tip 2026-10-06); `agenttrace` bare npm name still 404; ccusage demand baseline 215k dl/wk; codeburn #1723 merged.

## Non-goals

No test re-runs (recorded outcomes only), no review/shipping outcomes (review + commit/push/PR/CI happen after this phase), no schema or pricing work, no README/flag-table edits (see note above), no work on sibling-owned rows (double-landing risk; the deferral list in `prioritization.md` stands).

# Cycle-1 compound record — run 2023f2220bfb (repository-maintenance f3c0319b, cycle 1)

Date: 2026-10-08 · Compound attempt: 6a9d66c642a547b7893f94b220758dfb (after two provider-dead compound attempts — see the dead-attempt ledger) · Base: `611242d1d04cf753f52c7d5618a2c744840e431c` (worktree `run-2023f2220bfb-2023f222`) · Theme: **sqlite-lane identity & lineage truthfulness**

State at compound: uncommitted 8-file implement+validation delta (+605/−19: the 7-file rm-790/rm-791 batch `+599/−17` — lib.rs, history.rs, sqlite_sessions.rs, subagents.rs, session_cache.rs, tests/subagent_attribution.rs, CHANGELOG.md — plus full_tests' docs rider governance-reports.md `+6/−2`) on top of this run's ROADMAP band (rm-790..796 mint + roadmap banner, roadmap phase b379dabe, +60/0). This compound adds: the ROADMAP compound layer (banner + 2 row flips + 2 dated EXECUTED lines), this record, and prevention rule **PR-U** (`docs/solutions/workflow-issues/version-bumps-and-their-docs-sentences-must-co-travel.md`). No test/validation command was executed at compound — every gate below is consumed as recorded evidence from the targeted and full folds, per the compound contract.

## Cycle outcome (all-green chain, pre-review)

| phase | attempt | outcome |
|---|---|---|
| assess | cd02 | fresh adversarial delta-scoped review e9e8fd9→611242d1 (porcelain 0); F1 HIGH same-second session_id fold (live PoC 2 sessions → 1 history record at history.rs:70 insert-overwrite; live host 6 buckets / 13 sessions already folded); full workspace 597/0 + clippy 0; claim sweep 0 subject matches |
| research | 947dcf8b | live-host corpus sweep + GitHub API: 102/310 opencode subagent children (parent ses_ff746f680ffe 2,580,628 vs children Σ15,899,131), 1,254 all-zero cache_write_input_tokens (latent), server_tool_use unpriced, glm-5.3 alias pricing, 117/6,219 truncated cliproxy tails, opencode sst→anomalyco; wall+spool sweep 0 hits per candidate |
| roadmap | b379dabe (after reaped 84f5f0fd; a still-earlier dispatch death 3205c9ee left heartbeats only) | wall +60: minted rm-790..796 with signals/acceptance/evidence; title-twin rm-783 cross-noted; round-trip provenance (patch sha 4ec240dd…, final sha a0e0d409…) |
| prioritize | 17ea1c06 | SELECTED rm-790 LEAD (78.0) + rm-791 (68.0) as "sqlite-lane identity & lineage truthfulness"; contention audit over 13 sibling lanes; deferred queue banked (below) |
| stewardship | 23568350 | request envelope composed; surfaces re-anchored fresh at 611242d1; no git topology chosen |
| implement | 6919a8ea (after reaped f57fc0c0, zero durable) | RED-first on the assess PoC → GREEN; 7 files +599/−17; live scale 309/309 zero folds, 102 children/28 parents, warm==cold except pre-existing ULP; snapshot schema 7→8 |
| targeted_tests | e3d39965 | core lib 227/0, subagent_attribution 4/0 (release TSV leg hard), sqlite_hostile_disclosure 5/0, discovery_contract 90/0, codex_compaction_verdict 4/0, fmt+clippy rc0; digest `validation:v1:dec765d46612dd14c94092349295cbc9a96f5c93784fdca7170bcb67d7fd0290` MATCH before AND after the battery (scope adjudicated `targeted` over the real 6-executable-file delta — the dispatch block's `[]` was the known crates/** classifier artifact) |
| full_tests | c4a7834c (after reaped d84e3178, zero durable) | ci.yml full+deny mirrored lane-for-lane per PR-T: 22 gates rc0, 605/0 workspace, 42/0 entrypoints, MSRV 1.88 rc0, deny rc0; gate-12 docs regression (schema sentence 7 vs 8) fixed in-phase as the `+6/−2` docs rider |

Digest lineage: `validation:v1:dec765d46612dd14c94092349295cbc9a96f5c93784fdca7170bcb67d7fd0290` declared at BOTH validation turns, each independently re-derived live with the engine's own digest code (`/tmp/at-tt-e3d3/check.py`, faithful port of `hermes_conductor.validation_policy`) over base 611242d1 — 25 executable surfaces, missing []. Docs/ROADMAP changes are digest-immobile under the engine classifier; re-derive once more at the commit gate.

## Roadmap accounting (this compound's edits)

- **rm-790 flipped candidate → implemented** (dated, attempt-attributed) + an EXECUTED notes line closing the row's acceptance arms: keyed preimage (`Metrics::session_key` → `path|session_key|session_start`), PoC 2-record survival, live 309-not-6 shape, LF5 disclosure untouched, snapshot 7→8, full battery outcomes.
- **rm-791 flipped candidate → implemented** + an EXECUTED notes line: guarded parent_id read, session_key linkage arm (no cross-DB/cross-lane/self links), live 102/309 + 28 parents + top-parent rollup, fixture suite + serde pin.
- Compound banner added at the top of the banner stack (newest-first), above the cycle-1 roadmap banner; it carries the dead-attempt ledger, the d5f8b35c adoption, the commit-gate seams and the next-cycle context.
- **ZERO ids minted at compound.** Wall: 258 backtick / 272 broad def rows before and after; 0 duplicate ids; def-max rm-796; managed footer still last. `status → done` stays reserved for the commit gate (rm-012 convention). Fleet numbering unchanged.

## Dead-attempt ledger (6 provider deaths this run, all dispositioned — adoption nuance below)

| attempt | phase | window (UTC) | durable trail | disposition |
|---|---|---|---|---|
| 3205c9ee | roadmap | 19:28:01–19:36:01 | event log = turn_started + 7 heartbeats + reap; no envelope, no scratch | redo (b379dabe did; its artifact records only 84f5f0fd — this one is disjoint and earlier) |
| 84f5f0fd | roadmap | 21:00:41–21:08:45 | 14 events, msg 13; no envelope, no scratch | redo (recorded by roadmap phase) |
| f57fc0c0 | implement | 00:27:42–00:28:05 (+23 s) | 9 events; zero file writes | redo (recorded by implement 6919a8ea) |
| d84e3178 | full_tests | 04:01:40–04:04:44 | 11 heartbeats; no envelope; /tmp window only other runs' artifacts | redo (recorded by full_tests c4a7834c) |
| d5f8b35c | compound | 05:34:41–05:37:51 | NO envelope, NO scratch — but ONE in-tree mutation: ROADMAP.md @ 05:36:36 | **ADOPTED** (see below) |
| 66f81b96 | compound | 06:10:39–06:20:43 | heartbeats only; zero writes; no scratch; no envelope | redo (this attempt, from scratch on top of the adopted band) |

**Adoption nuance (refines the 84be17b33 "redo, never adopt" rule):** envelope/scratch absence proves nothing about *in-tree* mutations. An in-tree artifact left by a dead attempt is adoptable when it byte-verifies against the owning phase's recorded chain. Legs run this attempt for d5f8b35c's ROADMAP application: (1) in-tree ROADMAP.md sha256 `a0e0d409…` == the roadmap phase's PROVENANCE.txt final sha; (2) the spool ROADMAP.patch sha256 `4ec240dd…` == PROVENANCE.txt patch sha; (3) `git show 611242d1:ROADMAP.md` + patch == in-tree file, verified structurally (exactly the patch's 60 insertions at its 9 hunk positions, 0 deletions); (4) mtime 05:36:36 falls inside d5f8b35c's 05:34:41–05:37:51 event window, and no other attempt was live then. Adoption is scoped to that one mutation; attribution kept in the compound banner.

## Prevention rules

- **PR-U (new, in-tree):** `docs/solutions/workflow-issues/version-bumps-and-their-docs-sentences-must-co-travel.md` — any delta that bumps a grep-pinned version surface (SQLITE_SNAPSHOT_SCHEMA_VERSION, plugin.json version, README flag table, TSV column contract) must update the pinned docs sentence in the SAME delta and run the cheap docs gate (`scripts/ci/check-docs-commands.sh`, seconds) inside implement — not discover the drift first at full_tests. Proven by gate 12's rc1 here (schema 7→8 without the governance-reports.md sentence).
- **Record-embedded (reusable patterns proven this cycle):**
  1. *In-tree adoption by byte-chain (above)* — supersedes blanket "redo, never adopt" for tree mutations; envelopes remain redo-by-default.
  2. *Target/ sweep resilience (#17283, again):* full_tests found `target/release/` swept mid-run a third time — rebuild `-p agenttrace` before release-pinned legs (subagent_attribution TSV, real-cli-smoke); the release-pinned leg ran hard both times this cycle because the implement phase had left the binary in place and full rebuilt after the second sweep.
  3. *Validation-block scope artifacts:* `changed_testable_surfaces: []` for this repo is the engine's root-prefix classifier never matching `crates/**` (proven at d9486cab, again at e3d39965) — adjudicate scope from the real git delta.
  4. *Empty `full_command`:* ci.yml is the authority, mirrored lane-for-lane (PR-T, proven again at c4a7834c).
  5. *Digest replica before/after every run-only phase* — the proof that verbatim-digest declarations are fresh (run twice at e3d39965, twice at c4a7834c).

## Residuals and open scope (banked for later cycles)

- **rm-790 review riders likely:** `MetricProvenance` disclosure of the new session_key linkage (mirrors rm-545's open rider), TSV/CSV `PARENT` column for consumers (CSV lane owned by sibling run 6cb2756a's uncommitted delta — do not double-implement).
- **Sweep-volatile corpora** (recreate from recorded recipes; envelopes + this record carry the durable facts): `/tmp/at-assess-cd02/` (fold PoC + 597/0 log), `/tmp/at-research-947dcf/` (dossier + rebuilt debug binary at /tmp/at-bin-947dcf), `/tmp/at-impl-6919/` (red→green replay + live-scale cold/warm pairs + base-binary control), `/tmp/at-tt-e3d3/` (digest verifier), `/tmp/at-ft-c4a7/` (22 lane logs), `/tmp/at-compound-6a9d/` (this compound's chain-verification workspace).
- **Warm-run ULP residual:** the 1-ULP duration_sec warm/cold re-render is PRE-EXISTING at base (control pair proves it), serde_json 1.0.150 read path — a separate fleet thread; do not chase it inside this batch's review.

## Next-cycle context (recorded, not re-decided)

- **Strongest lead once its blockers land: rm-239 (87.0)** — blocked only by the 6a844b9b/e88f2da2 render lanes (per 17ea1c06's contention audit); its PoCs need re-verification at the newer base.
- **rm-175 + rm-793 pricing-provenance pair** once the parser lanes land; rm-793 needs the per-tool-request rate schema.
- **rm-421 (90.0)** once the THREE parser.rs lanes land — rm-792's latent rider family (cache_write_input_tokens, model_context_window, rate_limits.credits) can share that batch's parser touchpoints.
- **rm-794: premise REFUTED at the sibling base** (652a cycle-3 banner: exact_pricing_pct 100.0, fallback 0) — re-verify against the live pricing snapshot before any selection; do not implement on the refuted premise.
- **rm-448** upstream-sync wave (needs a dedicated cycle); **rm-195** fork-owned release channels (owner decision needed).
- **rm-795/rm-796** (unpriced-JSON-line provenance, dual-adoption identity) remain candidate, unblocked, lower band.

## Integration handoff (for the review + commit gates)

Payload = 8 M files (+605/−19) + ROADMAP compound layer + this record + PR-U doc. `git add` explicit paths only (never `-A`); the two untracked docs must be staged explicitly. Verify staged numstat, assert commit parent == `611242d1`, post-commit porcelain empty. Done-flips (rm-790, rm-791) reserved to the gate. Reconcile rm-783 BY TITLE (one row survives carrying both trails; 4a11d593 cede precedent if that lane selects later); rm-787 is surface-disjoint from rm-791 — verify, don't merge. The docs rider and the CHANGELOG bullet travel in the same commit (grep-pinned sentence + user-facing behavior). v7 sqlite snapshots regenerate once on first post-upgrade run — expected. Re-derive the digest at the gate; ROADMAP/docs drift is digest-immobile, so `validation:v1:dec765d4…` must still MATCH over the final staged tree.

## Verification (static — no tests executed at compound)

`git status --porcelain` = 9 M (the 8 batch files + ROADMAP.md) + 2 untracked docs (this record, PR-U). ROADMAP: 272 broad / 258 backtick def rows, 0 duplicate ids, def-max rm-796, managed footer last, banner stack newest-first with the compound banner above the cycle-1 roadmap banner, both flips carrying dated attempt attribution. Pre/post-edit byte isolation: pre-compound ROADMAP snapshot (sha `a0e0d409…`) preserved at `/tmp/at-compound-6a9d/current-ROADMAP.md`; the compound layer alone is `6a9d66c6-scratch/compound-delta.patch`. Spool scratch `6a9d66c642a547b7893f94b220758dfb-scratch/` carries the patch + a copy of this record.

## Adoption addendum (2026-10-08, retry attempt 7c9a752d9fd94f779cd724b844c64dbb)

The author attempt 6a9d66c6 was provider-reaped at 07:53:59Z (+29m33s), ~3–5 minutes after writing this layer's three artifacts (ROADMAP layer 07:49:21Z, this record + PR-U 07:50:51Z) and before its envelope and the spool scratch copies named above — the run's 7th provider death. Retry 7c9a752d verified the durable trail and completed the phase without redoing any of it:

- **Identity**: events/6a9d66c6….jsonl carries run 2023f2220bfb4112bd21489144d36d0e / phase compound / action compound:compound — this work order's lineage; the attempt's own scratch workspace (/tmp/at-compound-6a9d/work.md) self-reports `reaped: true, envelope_written: false`.
- **Byte-chain re-run**: spool ROADMAP.patch sha256 `4ec240dd…` + `git show 611242d1:ROADMAP.md` + `git apply` reproduces sha256 `a0e0d409…` == ROADMAP.final.md; /tmp/at-compound-6a9d/current-ROADMAP.md == `a0e0d409…` (pre-layer snapshot intact); the compound layer == exactly the banner (+2 lines), the two row-status flips, and the two dated EXECUTED notes — zero landed rows touched; the in-tree ROADMAP (sha256 `d26654c1…` pre-amendment) was byte-stable from 07:49:21Z until this retry's banner-attribution amendment. (The reaped attempt's work.md lists a mid-layer "current" sha `e0b57814…` matching no reconstructible state — a stale scratch note, cited by no durable artifact.)
- **Census re-run**: 258 backtick / 272 broad `- id:` def rows (the compound layer adds zero), 0 duplicate ids, def-max rm-796, statuses 134 candidate / 67 implemented / 57 done, managed footer last.
- **No-test-execution leg**: the three compound /tmp windows (05:34–05:37, 06:10–06:20, 07:24–07:54) contain only markdown census/chain workspaces and other runs' files; zero worktree files modified between the reap and this retry.
- **Corrections applied above (the only defects found)**: the 8-file delta is `+605/−19` and the docs rider `+6/−2` per `git diff --numstat` (the `+607`/`+8` figures folded the `--stat` changed-lines total — 8 = 6+2 — into insertions). The `+599/−17` implement figure and every other recorded number re-verified exact. Cosmetic: ledger row 3205c9ee's window start is 19:28:31Z per its event log (row said 19:28:01Z).
- **Scratch materialized**: `6a9d66c642a547b7893f94b220758dfb-scratch/` now carries compound-delta.patch (post-amendment), a copy of this record (with this addendum), and PROVENANCE-7c9a.md; the full verification log lives in `7c9a752d9fd94f779cd724b844c64dbb-scratch/verification.md`.

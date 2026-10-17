# Cycle-1 compound record — run 84be17b33 (repository-maintenance 33a4e7d3, cycle 1)

Date: 2026-10-07 · Compound attempt: df0b2306c2a74bd0a7b8ae55133c7aac (no prior attempt on this action) · Base: `89911442173d31f4eb21a1968bfd51ad46e32ed1` (worktree `run-84be17b33814-84be17b3`) · Theme: **identity & gate honesty**

State at compound: uncommitted 6-file implement delta (+484/-37: `main.rs` +14, `tests/entrypoints.rs` +111, `doctor.rs` +21, `insights.rs` +353/-31, `docs/guides/governance-reports.md` +9) on top of this run's inherited ROADMAP +13 (roadmap phase 3ed32acb). This compound adds: the ROADMAP compound layer (banner + 2 row edits, below), this record, and prevention rule **PR-T** (`docs/solutions/workflow-issues/empty-full-command-mirrors-ciyml-lane-for-lane.md`). No test/validation command was executed at compound — every gate below is consumed as recorded evidence from the targeted and full folds, per the compound contract.

## Cycle outcome (all-green chain, pre-review)

| phase | attempt | outcome |
|---|---|---|
| assess | 76d962da | fresh adversarial pass over the 2a024b6..8991144 delta (porcelain 0); NN1 MED `upstream` keyword dispatch drops `-o` (live PoC, file never created); NN2 LOW `--waste` gate flag parsed never evaluated; NN3 LOW codex empty/missing `type` line silent; re-verified F1/F2/F3/N1/N3 at current line refs; full workspace run + docs gate green |
| research | c75333c0 | v0.10.1 surface fully occupied by in-flight fleet claims; PR #318 (ureq 3) read full-body — fork's 32 MiB `read_body_capped` never calls `into_string`, so the upstream 10MB→64MB body raise must NOT be ported; claude-code/codex/OTel/ccusage watch sweep |
| roadmap | 3ed32acb | wall +13: campaign banner, rm-044/rm-087 appends, rm-486 premise-refresh rider, rm-542 follow-up, **minted rm-599** (upstream `-o` honor, title-checked); NN1/NN2/NN3 folded to in-flight lanes, not re-minted; 213 def rows, footer last |
| prioritize | 915a7309 (charter) | batch = rm-240 (LEAD, 86.0) + rm-486 rider (88.0), mint-free |
| stewardship | b9cf40f3 | request composed, selection carried unchanged |
| implement | 783f3ba2 | RED-first (both decode tests `'id: unknown'/'unattributed'`; waste json rc0-where-rc2-required) → GREEN 419/0 across 19 suites + clippy workspace + fmt + docs gate rc0; live PoC `by_project [{MyOpaque-Name_77},{seg10-component#e1f2a3b4}]`, doctor `project_decode {resolved:2,ambiguous:0,unresolved:0}`; waste rc matrix write-then-gate |
| targeted_tests | d9486cab | 225/0 (core lib 186 + demo_contract 7 + entrypoints 32) + clippy/fmt/docs rc0; scope adjudicated `targeted` over the real 6-file delta (block said `none` — crates/** classifier artifact) |
| full_tests | d89c8c2b | ci.yml lint+full+deny mirrored lane-for-lane (22 lanes, rc0, 9m11s, 498/0 tests at load1 31.5) — dispatch `full_command` empty, ci.yml is the authority; see PR-T |

Digest lineage: `validation:v1:a72443d225dc8d687e34505eb328bd65608884abc60d932d59f83678062b7b0c` declared verbatim at BOTH validation turns, each independently re-derived live with engine code (`validation_policy.validation_digest`) over the programmatically-derived full-40-char base and byte-identical to the dispatch token. crates/** deltas are digest-immobile under the engine classifier; ROADMAP/docs drift at compound cannot move it either (reconciliation rule in `docs/solutions/process-issues/validation-digest-base-and-coverage-reconciliation.md`).

## Roadmap accounting (this compound's edits)

- **rm-240 flipped candidate → implemented** by title anchor, with a dated `compound c1 2026-10-07` evidence line closing the row's DISTINCT residual (official >200-char truncate+hash encoded names + `CLAUDE_CODE_PROJECT_DIR_NAME` opaque names) via the new `ProjectDecode` Truncated/Opaque arms on the landed rm-381 DFS walk.
- **rm-486's premise-refresh rider RESOLVED** on-row with a dated `premise refresh RESOLVED` line: schema'd `--waste -f json` output now joins the gated set (shared `enforce_report_gates`, write-then-gate); the text view stays ungated by design; its drift-canary rider stays open.
- Compound banner added at the top of the banner stack (newest-first), above the cycle-1 roadmap banner.
- **ZERO ids minted at compound.** Wall: 213 def rows before and after; managed footer still last. `status → done` stays reserved for the commit gate (rm-012 convention). Fleet numbering unchanged: next free above this campaign's rm-599 mint.

## Prevention rules

- **PR-T (new, in-tree):** `docs/solutions/workflow-issues/empty-full-command-mirrors-ciyml-lane-for-lane.md` — empty dispatch `full_command` → ci.yml is the authority, mirrored lane-for-lane; redirect `AGENTTRACE_CI_OUT`/`AGENTTRACE_REAL_CLI_OUT` to /tmp so the mirror leaves porcelain untouched; heavy lanes through `local_validation_gate.py` under fleet load; local `cargo deny --all-features check` flag order; vars-gated lanes skipped exactly as CI would.
- **Record-embedded (reusable patterns proven this cycle):**
  1. *Premise-refresh riders:* a row's "BY DESIGN" carve-out can be invalidated by a later landing (rm-544 turned waste into a versioned machine contract). The chain that worked end-to-end: roadmap appends a dated premise-refresh rider → prioritize carries it as a batch rider → implement re-adjudicates red-first (rc matrix) → compound records the resolution on-row. Any view→machine-contract transition should re-open every ungated carve-out that names that surface.
  2. *Validation-block scope artifacts:* `changed_testable_surfaces: []` / `required_scope: none` for this repo is the engine's root-prefix classifier never matching `crates/**` — it does NOT mean nothing changed. Validate the real git delta and declare `targeted`/`full` accordingly (proven again at d9486cab).
  3. *Dead-attempt forensics, 4-for-4 this run:* typed-envelope absence + event-log tail (`session_reaped` + `message_count`) + spool//tmp scratch sweep keyed to the attempt id = zero-durable verdict; redo, never adopt. Deaths: roadmap d60adf7f (msg 9), prioritize 45d57af3 (msg 3), stewardship 6f2a70f4 (429, msg 13), targeted_tests a4c37ceb (provider abort, msg 14).
  4. *Digest replica recipe:* before and after every run-only phase, re-derive the tree digest with the engine's own code over the programmatic full-40-char base sha — a byte-identical replica is the proof that "verbatim dispatch digest" declarations are fresh, not stale.

## Residuals and open scope (banked for later cycles)

- **rm-486 drift canary** (row rider, open): enumerate report-rendering actions against the shared `enforce_report_gates` helper so a future action flag cannot silently re-open the gap; assess F1's rc-matrix is the detection tool.
- **CHANGELOG rider owed at the commit/PR gate:** the batch changes user-facing behavior (`--waste -f json` now gates; previously-`unknown` projects now resolve) — per fleet convention the CHANGELOG entry rides the merge, not compound.
- **Sweep-volatile corpora:** `/tmp/at-assess-84be/` (NN PoCs, f3 corpus, bad-baseline.json), `/tmp/at-impl-783f/` (red/green logs + live fixtures), `/tmp/at-targeted-d948/`, `/tmp/at-full-d89c/` — /tmp is swept between phases; this record + the delegate envelopes carry the durable facts, recreate corpora from the recorded recipes when needed.

## Next-cycle context (recorded, not re-decided)

- **rm-599** (minted this campaign, candidate) is the natural next lead: the upstream status report must honor `-o` like every report action (assess NN1, main.rs:193-197 write_stdout-only arm).
- Folded families awaiting their in-flight owners: F1 keyword-dispatch → extend in-flight **rm-573**'s sweep to the upstream-keyword arm; F2 (`-f csv/markdown upstream` renders text) + F3 (master-first default error, `UPSTREAM_REF` escape hatch undocumented) → in-flight **rm-574**, fresh PoC corpus banked; N1 (arbitrary-JSON baseline fabricates deltas, `reports.rs:766-777` `unwrap_or_default`) → in-flight **rm-569**, `bad-baseline.json` re-proves it; NN3 → rm-542's dated append (landed this roadmap phase).
- Watch items carried: PR #318 merge posture (rm-044 append), ccusage #1822 (rm-087 append), OTel semconv v1.44.0 (rm-493 untriggered).

## Integration handoff (for the review + commit gates)

Payload = the 6 M files (+484/-37) + ROADMAP compound layer + this record + PR-T doc. Apply the crosstalk defense (run d65f72c7 precedent): re-read file bytes + mtime + numstat against the census before any edit at the gate; `git add` explicit paths only (never `-A`); verify staged numstat + post-commit porcelain empty; assert commit parent == `8991144`. Done-flips (rm-240, rm-486) reserved to the gate. IDs campaign-local — renumber by TITLE at integration, never by id.

## Verification (static — no tests executed at compound)

`git status --porcelain` = exactly the 6 M files + the 2 new docs files (this record, PR-T); ROADMAP def rows 213 with zero duplicate ids and the managed footer still last; banner stack newest-first with the compound banner above the cycle-1 roadmap banner; post-edit digest replica (engine code over the programmatic base) still byte-identical to `validation:v1:a72443d2…b7b0c`, demonstrating ROADMAP/docs drift is digest-immobile; spool scratch `df0b2306-scratch/` carries the ROADMAP delta patch and a copy of this record.

# Cycle 2 compound record — run 6d254aad (repository-maintenance 3282bd9d)

- **Base:** 1c5edd1e04d8a07fa552d4ff3ebf455392907550 (run worktree `run-6d254aad0772-6d254aad`, family `agenttrace-80c75f65b7`), porcelain 0 at every phase boundary in/out — the only tree delta is the implement batch itself
- **Phases:** research a21dbcda → research re-dispatch 478025d9 (re-verified first-hand at the same base; the a21dbcda dossier's "Cargo.lock 549/-399" and "sha2 0.11" figures corrected to +206/-399 and 0.10.9) · assess a01f (3 PoC-proven findings, corpus /tmp/at-assess-a01f) · roadmap 0cb5d0b2 (ZERO mints by adjudication; delta spool-side) · prioritize fe812598 · stewardship 9f53318e · implement df2a04f0 (reaped, zero durable) → c91244d5 → KTD13 attestation repair 320900a8 · targeted_tests 8fb3edb2 (reaped, zero durable) → 4a7872cc · full_tests a517982a · compound e256f6f1 (reaped) → fc264f19 (reaped) → 7afbbdf0 (reaped post-build, Oct 7 16:50Z) → 2b7ee055 (this, 2026-10-08)
- **Batch:** "Conformance guardrail: token-accounting truth" — rm-053 SOLO (reliability, 84.0; the fleet's #1 recurring defect class per the AgentMeasure ~110-tool audit)
- **Delta at compound time:** 1 M (`.github/workflows/ci.yml`) + 3 untracked roots (`crates/agenttrace-core/tests/token_conformance.rs`, `scripts/conformance/`, `testdata/conformance/`) = the engine-derived 18-path changed-surface set. No commits.

## Recorded outcomes (pre-review; consumed at compound, not re-run)

- **Implement c91244d5/320900a8:** red-first mutation legs (case 007 input 1500→3500 proves the double-count catches; case 012 truth→0.0 proves the xfail pin asserts real truth); fmt clean; clippy `--locked -D warnings` clean; `cargo test --locked -p agenttrace-core` 372/0 across 20 suites.
- **Targeted 4a7872cc:** token_conformance 2/0; `AGENTTRACE_BIN=target/debug/agenttrace scripts/conformance/run.sh` → "13 cases asserted (1 xfail-pinned) — all within truth", rc 0; digest `validation:v1:8d83ac0f81395befce67d2ab3fddd1206c7a4f8fd767a77605fa13bfe4326465` re-derived with the engine's own `validation_policy` == dispatch token; `changed_surfaces` = full engine-derived 18-path delta.
- **Full a517982a:** ci.yml lint/full/conformance/deny envelope mirrored locally (dispatch full_command empty — the crates/** classifier artifact); 535/0 across 27 test binaries (assess baseline 533/26 + token_conformance 2/0); release build rc0 2m20s (13,507,984 B); entrypoints 33/0; 17/17 script gate lanes rc0 incl. the conformance pair and cargo-deny 4-category; MSRV + TUI-real-smoke lanes excluded by the workflow's own if-gates.
- **Pack:** 13 fixtures = 6 verbatim rm-046 adversarial (001-006) + 2 assess-corpus imports (012 cc-multi xfail-pinned to rm-663, 013 opus control) + 5 net-new pack-owned (007-011), across 7 operational classes; every non-xfail cost literal re-derived from the bundled snapshot at run time (6e-5 tolerance).

## Dead-attempt pattern this cycle (five reaps, zero lost work, three adoptions)

| attempt | phase | durable trail | disposition |
|---|---|---|---|
| df2a04f0 | implement | 14-line log, no scratch, no envelope | redo (c91244d5) |
| 8fb3edb2 | targeted | 3-line event log, no scratch | redo (4a7872cc) |
| e256f6f1 | compound | postimage + compound patch + 3 round-trip repos | ROADMAP half ADOPTED after byte-identical re-verification |
| fc264f19 | compound | corrected builder (2 factual fixes), not executed | builder ADOPTED after claim re-verification + defect fix |
| 7afbbdf0 | compound | corrected-builder postimage + both new docs; no patches, no envelope | postimage + docs ADOPTED after first-hand claim re-verification; patches + round-trip completed by 2b7ee055 |
| (earlier: a21dbcda) | research | dossier with 2 wrong figures | re-verified by 478025d9, corrections recorded on the wall |

Triage rule sharpened this cycle: **a 3-line reap log with no scratch dir means redo; a scratch dir with a chain-verified postimage means adopt the verified half, re-verify every adopted claim against primary sources, and complete the remainder.** An absent envelope is not evidence no work happened; a present artifact is not proof it is valid.

## Status flips and id landscape

- rm-053 flipped `candidate → implemented` with a dated EXECUTED addendum; `done` stays reserved for the commit gate (rm-012 convention). ZERO ids minted at compound.
- This wall: 228 new-format + 14 old-format defs, ceiling rm-603, 0 duplicate ids — unchanged by the compound delta.
- Numeral census refreshed live 2026-10-08 (agenttrace-scoped: spool postimages + sibling worktree walls; the shared spool's dashboard-family files claiming rm-2026/rm-1364 are a foreign id-space, excluded): unlanded fleet frontier **rm-783** (worktree `run-e88f2da2fbe9-e88f2da2`; live 2026-10-08 census by 2b7ee055), next free **rm-784** — supersedes the roadmap banner's rm-715/716 (2026-10-07) and 7afbbdf0's intermediate rm-779/780 snapshot (the fleet minted rm-780..rm-783 after Oct 7 16:49Z). Re-census before any mint.

## Next-cycle deferred queue (priority order, from cycle evidence)

1. **rm-663** per-event model stamping (sibling 'Cost truth' band, 8fbbf166) — its landing flips pack case 012 green and owes the pack-refresh contract's flip step; the highest-leverage unlanded sibling for this pack.
2. **rm-695 + rm-702** (de600e73 band: waste red-tier unreachable; diagnostics severity inversion) — re-derive their PoCs against a pack-asserted binary after landing.
3. **rm-006** LiteLLM drift (4,480 live vs 3,100 bundled @2026-10-07) — first candidate if the harness arm is wanted before the #318 migration; rides the new pack-refresh contract.
4. **rm-044** upstream #318 ureq-3 migration port — transport-body limit ≥32MiB so the fork's own 32MiB `PRICING_DOWNLOAD_MAX_BYTES` (pricing.rs:670) stays binding.
5. **Re-audit the ≥75 pool after landings** (prioritize fe812598 enumerated 25 new-format candidates + 11 old-format + 59 implemented-awaiting-gate).

## Commit-gate handoff

- Land as ONE batch: the 18-path implement delta + the roadmap patch chain + the two new docs.
- Roadmap chain (all spool-side in `delegate/2b7ee055aea34850ae092edda985b982-scratch/`, git-apply-clean in chain order at base 1c5edd1e): `roadmap-0cb5d0b2.patch` (cycle banner + 4 dated row appends; also mirrored in the roadmap phase's own scratch) → `ROADMAP.compound.patch` (this compound: banner + rm-053 implemented flip) → `compound-new-docs.patch` (this record + the prevention rule).
- Flip rm-053 → `done` at the commit gate (by title anchor); no other row moves.
- No validation is owed by this record: the newest full-suite evidence (a517982a, 17/17 lanes) covers the tree; the compound additions are markdown-only and digest-immobile — the dispatch digest `validation:v1:8d83ac0f…` stays current.
- Review-visible decision: no CHANGELOG rider was minted for this batch (CI/test-infra only, no user-facing surface); if review wants one, add it at the review-fix stage.

## Prevention rules minted this cycle

- NEW `docs/solutions/workflow-issues/conformance-pack-refresh-contract.md` — snapshot refresh forces pack refresh; xfail pins flip only with their owning row; verbatim corpus fixtures are never "fixed" to pass.
- Carried (not tripped this cycle): schema-version bumps require a planted-fixture sweep; KTD13 — implement folds declare the full engine-derived changed-surface set.

# Cycle 1 Compound Record — repository-maintenance `5fe8a7f09ff44146b97c1952f3eea860` run `edf88a43f0ea44d2974e4580119dc983`

Worktree `run-edf88a43f0ea-edf88a43` @ base `3449b36f5d30e0caf3e9276ec2408dbcf286f211` (branch `conductor/run-edf88a43f0ea`), 2026-10-17. All phases porcelain-clean in/out except the sanctioned implement delta (3 files, +97/−2).

## Phase ledger

| Phase | Attempt | Outcome |
|---|---|---|
| assess | e5c64380 | 8 findings with file:line (F1 truncation marker, F2 guard-family split, F3 TUI boot replay discard, F4 statusline `$-5.00`); prior attempt 0b9bc6e6 reaped at 23s, zero salvage |
| research | f40eb1f2 | Live upstream census: 4 mintable (OTel intake, Daybreak, pi 1.3.0, Cline Desktop) + 2 refresh riders; 10+ checked-and-dropped |
| roadmap | 1b8e2fe0 | Spool-side band: rm-1541..rm-1548 + riders on rm-930/rm-1373 (apply-check clean, +60/−0) |
| prioritize | bdc55c9f | Batch 'Render-lane truth discipline' (rm-1543 + rm-1544) selected under the fleet collision map |
| stewardship | ac605b53 | Request emitted; units file-disjoint/order-independent; must_remain_separate ×4 |
| implement | 9f36e1d2 | Both arms landed red-first, 3 files, +97/−2 |
| targeted_tests | 80c6ceab | tui 65/65, core lib 430/430, clippy 0, fmt clean; digest re-derived == dispatch (crates/**.rs invisible to validation:v1) |
| full_tests | d8e653cd | ci.yml:134 command verbatim: 3 crates, 70 binaries, **1226/0**, fmt rc0, clippy -D warnings rc0; digest match; attempt 49daac43 reaped twice (~27s), zero salvage |

## 1. Batch and landing order

Implemented: **rm-1543** (TUI boot path discards the codex multiagent replay reconciliation count — app.rs `let _replay` discard vs reload threading; disclosure now fires on first launch, phase gated to `Ready` only when an adjustment occurred) and **rm-1544** (statusline cost render accepted negative totals — `$-5.00` live; now clamps at the zero boundary, rm-1500 dialect).

RED proofs: (A) test present + fix reverted → `left: "idle — loading 0 sessions (normal load, 0/0 cache hits)"`; (B) guard reverted → `line was $-5.00`. Full suite green after.

**Landing order (commit gate):** (1) code delta (3 files: `crates/agenttrace-tui/src/app.rs`, `crates/agenttrace-tui/src/tests.rs`, `crates/agenttrace-core/src/statusline.rs`); (2) ROADMAP in TWO chained steps — apply `roadmap-band-1b8e2fe0.patch` to the pristine wall, THEN `compound-band.patch` on top (both mirrored under `conductor-runs/edf88a43f0ea44d2974e4580119dc983-roadmap-band/` and `…-compound/`, plus `/home/agent/.cache/at-roadmap-e490029c/`); chained result pinned md5 `7b5c09f9699a609bebc8b663ee3c6b24` (3,015 lines) — `compound-band.patch` alone does NOT apply to the bare wall, and stopping at the 1b8e2fe0 patch would land the band WITHOUT the status flips; (3) this record. CHANGELOG entry owed at the gate (CHANGELOG.md has a live fleet holder in a sibling worktree — reconcile before editing).

## 2. EXECUTED rows this cycle (statuses already flipped in the unlanded band)

- **rm-1543 → implemented** (evidence bullet in-band; full suite 1226/0).
- **rm-1544 → implemented** (evidence bullet in-band; live PoC −5.0 → `$0.00`, control 1.25 → `$1.25`).

Not flipped (candidate rows standing): rm-1541/rm-1542/rm-1545..rm-1548.

## 3. Cycle defects and their standing

- **rm-1541↔rm-388 title-twin** (mint sweep too narrow; rm-388 :1300 is the umbrella): reconciliation note now lives IN the band row — implement once under rm-388, land rm-1541 as its text-arm rider.
- **rm-727 stale status**: acceptance verified satisfied at base (`now` param :1698-1702, `excluded_future_days` :1695/:1799/:1823, JSON field :1916, render clause :1961-1964 + inline tests) — evidence rider appended in-band; flip owed at next render, do not re-implement.
- **rm-956/rm-798 id guesses** (research envelope): the wall's actual rows are rm-930 (release channels) and the truncation umbrella rm-388 — riders landed on rm-930/rm-1373 instead; corrected in prioritize.

## 4. Reusable lessons / prevention rules (validated this cycle)

1. **Prioritize under the fleet collision map first.** `git worktree list` + per-worktree porcelain turned the top-priority rows into unusable picks (parser.rs ×4, cli/main.rs ×5 live holders) and revealed the zero-holder zone (tui/**, statusline.rs) — batch selection followed in minutes. Cost of skipping: merged foreign in-flight work.
2. **Title-twin sweeps must match subject keywords, not full candidate titles.** rm-1541 slipped past an exact-title sweep because the umbrella (rm-388) phrases the same defect differently. Sweep by noun phrases ("truncation", "true count") before minting.
3. **Reachability-first test design for junk-value guards.** serde_json rejects `1e999` ("number out of range") and `Value` cannot hold NaN/inf (coerces to null) — the "non-finite cost" worry narrowed to negative totals only. The test now asserts the unreachability facts; the `is_finite` clamp stays as cheap defense-in-depth. Lesson: before writing a red fixture for a "junk" input, prove the input can actually reach the guard.
4. **Statusline regressions hide behind empty-string JSON.** The base `$-5.00` live PoC needed `echo ''` warm-up lines — CC pipes empty strings into the hook; parsers must treat empty input as no-op, and tests should mimic it.
5. **validation:v1 digest blindness is reproducible.** crates/**.rs never moves the digest (proven twice this run by re-derivation matching the dispatch value) — record it in every validation envelope rather than re-deriving from scratch; script at `/home/agent/.hermes/conductor-runs/92e96004…-final_validation/recompute_digest.py`.
6. **Reaped-attempt forensics before redo.** Two reaps this cycle (0b9bc6e6 @23s, 49daac43 @27s ×2): always read the event log tail + check for envelope/scratch before declaring redo-from-scratch; both were genuinely empty.
7. **Full-suite command provenance when the dispatch block is empty**: take `.github/workflows/ci.yml`'s test lane verbatim (here `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace`, ci.yml:134), never an ad-hoc script.

## 5. Cycle context for the next maintenance cycle

- **Fresh lanes (all evidence-first):** rm-1545 OTLP-JSON intake (probe harness: parse what `-f otel` itself writes; CC 2.1.301 `tool_decision` enrichment; NO network transport — recorded privacy boundary), rm-1546 Daybreak default-on census at codex 0.163.0 stable (real corpus; pricing basis via models.dev; fog sibling rm-1922 wall 64aedffa), rm-1547 pi 1.3.0 wire (kimi-coding Chat Completions; golden journal; exercises in-progress rm-438), rm-1548 Cline Desktop (pin storage path from desktop-v0.0.47 source before any discovery code).
- **Collision map will have moved:** re-derive before selecting; main.rs zone (rm-388 umbrella + rm-1541 rider) needs a holder-free window.
- **Deferred with reasons:** rm-1542 guard-family relocation (lib.rs holder); rm-920 Windows lock race (not validatable here); rm-989 (live script holder).
- **Release channels still pinned** 0.10.1 (GitHub/npm/homebrew) = 372 commits behind; region pricing + opencode attribution unreleased — owner decision per rm-930.
- **Fog reconciliation by TITLE** (never numeral, rm-708 convention): rm-546 (wall 1af198df, OTel semconv) vs rm-1545; rm-1922 (wall 64aedffa, Daybreak pricing alias) vs rm-1546.
- **Next free roadmap id: rm-1549** (id-line def-max rm-1548 after this band lands).

## 6. Open questions for the reviewer/next cycle

- rm-1543's phase-gate judgment (`Ready` only when an adjustment occurred; zero-adjustment boots keep the pinned idle panel) — deliberate scope-minimizing choice, flagged for adjudication.
- rm-1544 clamp-vs-flag for negative totals (clamped to `$0.00` per rm-1500 dialect) — same.
- Whether rm-388's JSON/stderr arms should absorb the text-arm marker work directly (one implementation) rather than sequencing rider-style.

---

**Adopted under attempt `e490029cc04945ce901a7d072a3c325b` (2026-10-17).** Prior compound attempt 1039ab64 completed this record and the band but lost its typed envelope to a delivery failure. Verification legs: identity (event log `run_id`/`compound:compound` match this work order); worktree census (HEAD `3449b36`, porcelain = the 3 implement files + this record only); band chain re-proven this attempt (wall md5 `1d81dabf` + 1b8e2fe0 patch → `65b5e849` + compound patch → `7b5c09f9`, both apply-checks clean; census invariant 433 unique `- id:` rows, def-max `rm-1548`, zero rows added by the compound pass; prioritize's "430" was its status-filtered universe). Errata: the implement envelope's per-file md5s and +34/+26/+37 split are a stale pre-`cargo fmt` reading — the validated tree is +28/+25/+46, same +97/−2 total; the 3 files are untouched since 2026-10-16 21:14 and every validation envelope (targeted 80c6ceab, full d8e653cd) ran after, so trust the tree + validation envelopes, not those md5s. All band artifacts mirrored to `conductor-runs/…-compound/` and `/home/agent/.cache/at-roadmap-e490029c/` (spool GC had already swept 7 of 9 prior scratch dirs).

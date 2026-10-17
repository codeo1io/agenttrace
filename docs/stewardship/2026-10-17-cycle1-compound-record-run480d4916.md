# Compound Record — repository-maintenance `480d49163efa4e6587543d8bfc06d544` cycle 1

- **Run**: 480d49163efa4e6587543d8bfc06d544 (repository-maintenance df0224e855d741398c313d93fa2f1807, cycle 1)
- **Wall**: HEAD 4405d13 (porcelain 0 at assess/roadmap/stewardship; 6 M + 1 ?? from implement onward — the batch, uncommitted by policy; +1 ?? since compound: this record)
- **Date**: 2026-10-17 · **Status at compound time**: implemented + full-suite green; pre-review (review/shipping outcomes carry forward into the next cycle's assessment)

## 1. Cycle shape

| phase | attempt | outcome |
|---|---|---|
| assess | 53b18b97 | 3 fresh findings (F1 gate scratch-HOME race, F2 sqlite budget load-lane asymmetry, F3 doctor-first disclosure suppression) + 2 corroborations (Windows HOME gate, README knobs) at 4405d13; hermetic 1257/0 baseline |
| research | fa917324 | 5 candidates (zst admission, daybreak pricing collapse, fork subagents, opencode epochs, argon snapshot) + R2 rustls red HEALED (cargo deny ok) |
| roadmap | 430a81fc | insert-only band: banner + 8 mints rm-1541..rm-1548 + 6 riders; 428→436 rows, worktree left pristine (scratch-side pristine/postimage/patch chain) |
| prioritize | a904f784 | standing 90+ tier adjudicated multi-cycle; selected rm-1541 (LEAD 86.0) + rm-1543 (ARM 55.0) + rm-1546 (DOCS 35.0) — disjoint-file S-batch |
| stewardship | 247362d8 | stewardship_request emitted; 3 change-units; d2355d7 reconciliation hints (parser decode already landed; fixtures plain-only) |
| implement | 9ffb73b7 (+ re-issue 24d89611) | batch landed uncommitted: +256/−14 across 6 files + 1 fixture pair; red-first pin via stash-revert |
| targeted_tests | f0722a4d (prior 6b22de74 reaped) | mandated command rc0 (fell back to full validation: 77 suites, 0 failed); digest re-derived == dispatch 322fba39… |
| full_tests | f8f75261 | **1258/0 across 77 suites** incl. Doc-tests lanes, after 3 host-environment retries; envelope returncode 0 |
| compound | ce4ba019 → 609c71b2 (redo) | prior attempt provider-died mid-turn (patch chain unbuilt; draft record left in-tree); redone here: band chain REBUILT byte-exact (md5s equal the roadmap phase's recorded artifacts), compound.patch = 3 flips + 3 riders + prevention Rule 12, draft record audited claim-by-claim and corrected |

## 2. What shipped (pre-review; uncommitted in run-480d49163efa-480d4916)

**rm-1541 — codex `.jsonl.zst` rollout visibility** (`d2355d7` shape re-landed with wall reconciliation):
- `discovery.rs`: `is_codex_walk_root` (:1232 — `.codex` path component OR canonicalized-under a registered Codex dir) + `is_codex_migrated_rollout_name` (:1252 — non-empty stem); `codex_tree` threaded through both walks (5 recursion sites); scoped admission at the cached (:1047) and uncached (:1157) cold sites — never a global relaxation.
- `session_cache.rs:1091`: `DIR_LISTING_WALK_VERSION` 5→6 (pre-v6 listings drop once and re-walk).
- `parser.rs`: `session_name` strips the `.jsonl.zst` tail (identity across the migration — `file_stem` alone left a residual `.jsonl`); `parse_session_bytes` stamps the new `codex_compressed_rollouts_decoded` disclosure counter (this wall never had d2355d7's parser delta).
- Tests: `codex_zst_migrated_rollouts_are_admitted_with_stable_names` (+122 lines) — admission, stem identity, disclosure counter, nested-root descent, non-codex scope guard. Fixtures `tests/fixtures/codex-zstd/migrated-pair/*` byte-exact from the object store (blobs 223970e7 / 9046d48e). Red-first pinned (stash → FAILED → pop → green). d2355d7's three unreferenced frame fixtures dropped.

**rm-1543 — CI gate scratch-HOME lane isolation**: `scripts/ci/check-output-contract.sh` scratch home `${TMPDIR}/agenttrace-output-contract-home` → `$out_dir/doctor-home`. Proof: two concurrent lanes rc0 with lane-own `cache_path` embedding; same-lane rerun md5-identical (`c771560be4a709243ebfbdf5d8c631b9`, implement verify 24d89611); determinism contract (rm-1425) preserved.

**rm-1546 — README doctor knobs**: the four `AGENTTRACE_DOCTOR_*` knobs with defaults (60,000 / 536,870,912 / 25,000 / 90,000) + pointer to `docs/maintainers/test-flake-prevention.md`. Corrects the assess dossier's wrong path (`doctor-budget-knobs.md` does not exist).

## 3. Validation record (pre-review)

- Focused: discovery_contract 108/0 (+1), zstd_rollout_decode 7/0, core lib 436/0, fmt/clippy(-D warnings)/bash -n clean.
- Targeted (mandated, fell back to full): 77 suites, 0 failed, rc0; digest re-derivation `validation:v1:322fba39…` == dispatch token.
- Full (verbatim `local_validation_gate.py --shell-command 'cargo test'`): **1258 passed / 0 failed / 77 suites** incl. 2 Doc-tests lanes; envelope `result-1040983-472216012.json` returncode 0.

## 4. Reusable lessons (this cycle's cost)

1. **Gate wrapper exit ≠ verdict.** `local_validation_gate.py` can exit 0 while its envelope records returncode 1 (run 1 false-green at the shell). Fleet rule: read the result envelope, never `$?`. Landed as **Rule 12** in `docs/maintainers/test-flake-prevention.md` via `compound.patch` (envelope-authoritative verdicts + lane-private HOME/TMPDIR/CARGO_TARGET_DIR on the home partition). Fog kin: run 66e28175's unlanded delta also carries a Rule 12 (private CARGO_TARGET_DIR for phantom compile reds) and 5c144613's band claims 9–10 — merge/renumber at fold per the Rule 7→8 precedent.
2. **Full gates need ALL IO lane-private on the home partition** — HOME + TMPDIR + CARGO_TARGET_DIR under `/home/agent/.cache/<attempt>-*`. `/tmp` and `/home/agent` share one 412G root fs; fleet pressure hit 100% twice (ENOSPC run 2, StorageFull run 3) and the shared conductor-cargo-target raced a doctest rlib (run 1, #18935 class).
3. **Reference-diff re-lands must reconcile against the wall, not the reference**: two of d2355d7's four hunks were superseded here (richer decode already landed; the disclosure counter entirely absent). Verify each hunk's subject on the live tree before porting.
4. **`session_name`-style derivation predicates must be re-derived for new admission spellings** — the residual-`.jsonl` bug only surfaced at green-run time; name-derivation invariants deserve their own assertion in admission tests.

## 5. Context for the next maintenance cycle

**Deferred with recorded cause (fresh, in the campaign band):**
- rm-1542 daybreak pricing collapse (70.0) — pricing seam heat; sequence AFTER the 2026-10-19 06:00 UTC drift-cron regen so argon keys + snapshot + SVG goldens land as one unit. Watch: the cron fires Monday; drift-check rc1 removal_guard (466 upstream drops) needs per-key adjudication (snapshot-drops.txt) BEFORE it can pass.
- rm-1544 sqlite budget load lane (60.0) — M-L, own cycle; the budget machinery exists (sqlite_sessions.rs:295-346), wiring is the work.
- rm-1545 doctor-first disclosure suppression (40.0) — pairs with rm-1544's discovery-lane rework.
- rm-1547 fork subagents (65.0) — fixture-gated: claude-code 2.1.302 default-on; host journals 0 samples.
- rm-1548 opencode epoch ingest (45.0) — product-gated: host tables 0 rows.

**Library-level residual (re-verified at compound, code-read):** `search_sessions` push-then-break — `search.rs:33-41` pushes the first match, THEN checks `results.len() >= limit` (:39), so `limit==0` returns exactly 1 row. NOT reachable from the shipped CLI: `validate_gate_thresholds` rejects zero loudly (`--search-limit must be at least 1`, agenttrace-cli main.rs:3298, rm-389, with the `--limit must be at least 1` sibling) — a library-caller footgun only. Fog rm-1515's subject (e4209f0c's unlanded 12-rider band); a one-line contract fix if ever promoted.

**Fog reconciliation obligations (title-keyed, landed-first wins):** this campaign's rm-1541 twins fog rm-1527@d2355d7 + rm-1941; the standing unlanded bands (e4209f0c 12-rider, 389e223a 8-rider, 1fb75154 implement, 122b213a implement+band) must be reconciled by title at integration. Next free numeral AFTER a fresh census (this band minted through rm-1548; fog frontier sits at rm-1999+).

**No-drift pins re-verified this cycle:** rustls R2 HEALED (0.23.46, cargo deny ok — clearable from gate ledgers); OTel v1.44.0; MCP 2026-07-28; fork-parent luoyuctl/agenttrace frozen at 15ed07f2 (21 checks); gemini-cli 0.64.0 STABLE (rm-715 recipe actionable); codex 0.163.0 stable / 0.164 alphas stubs; opencode npm 1.19.3 (V2 v2.0.32 off-latest).

## 6. Artifacts (all 3×-mirrored: `delegate/609c71b2…-scratch/` + `~/.cache/at-roadmap-609c71b2/` + `/tmp/at-roadmap-609c71b2/`)

- Batch delta (this worktree, uncommitted by policy): 6 M + 1 ?? — see §2; plus this record (untracked, joins the batch).
- **Roadmap band chain — REBUILT byte-exact.** The roadmap phase's scratch + `~/.cache` mirrors were reaper-swept (the run-e30a8801 lesson); the chain was recovered from prior compound attempt ce4ba019's reconstruction and fidelity PROVEN by md5 equality with the roadmap phase's recorded evidence: `ROADMAP.pristine.md` (== wall `0ecafec0…`) → `band.patch` (md5 `e12cadd0…`, 8 hunks) → `ROADMAP.post.md` (md5 `1d2d5625…`, 436 rows); builder `build_band.py`.
- **Compound chain:** `compound.patch` (md5 `9e64dd90…`, 5 hunks — ROADMAP: 3 candidate→implemented flips + 3 evidence riders + compound banner; `docs/maintainers/test-flake-prevention.md`: +Rule 12) → `ROADMAP.compound.md` (md5 `5ae1a741…`, 436 rows, ZERO new mints); builder `build_compound.py`. Round-trip verified: pristine + `band.patch` + `compound.patch` == compound postimage, byte-exact, `--fuzz=0`.
- `APPLY-SEAM.md` — commit-gate order: `patch --fuzz=0 -p2 < band.patch`, then `patch --fuzz=0 -p2 < compound.patch`, then commit with the batch + record + fixtures.
- Phase dossiers: the engine's phase records are the durable evidence of record (assess 53b18b97, research fa917324, selection a904f784, stewardship 247362d8 — their spool scratch copies and `/tmp/litellm-live.json` were swept; the live LiteLLM counts survive in rider rm-176 and §5). Surviving scratch: implement `delegate/24d89611…-scratch/verify-24d89611.md`, targeted `delegate/f0722a4d…-scratch/` (mandated log + gate envelope), full `delegate/f8f75261…-scratch/full-gate-run4.log` (77 ok / 1258 passed / 0 failed census re-derived at compound).

# Compound record — repository-maintenance c2d69033 cycle 3 (run f923a62a4804, compound attempt 17680dd2)

Date: 2026-10-15. Scope: pre-review cycle evidence only (assessment, research, roadmap,
prioritization, implementation, targeted + full test outcomes). NO tests were executed in the
compound phase; every test outcome below is cited from the recorded phase envelopes.
Review and shipping outcomes are intentionally absent — the next cycle's assessment carries
them forward.

## Cycle ledger

| phase | attempt | outcome |
|---|---|---|
| assess | e0e6c4c9 | PoC-verified findings F1 (predict_cost_anomaly model-blind, high), F2 (search_sessions limit=0 \| cutoff_turn_idx>0, medium), F3 (loop_cost tie dual-attribution, info) |
| research | 1e4f94df | flat upstream surfaces; fresh: release-note content, fork tracker, MCP schema versions, semconv tags, competitor census |
| roadmap | 6a7d71c1 | band = 1 banner + 1 rider (rm-085) + mints rm-1438/rm-1439, 17 insertions, round-tripped patch; scratch later REAPED (rebuilt at this compound) |
| prioritize | 1a9c894d | selection-collision map + standing model; rm-1438 = 80 (selected), rm-1439 ≈ 57 (next) |
| stewardship | 870ed118 | structured stewardship_request, no git-topology decisions; flagged canonical ROADMAP md5 458e7e36 as stale |
| implement | bae5c75e | red-first 3/3 (PoC reproduced byte-for-byte) → same-model baseline + disclosed thin-history fallback → green 4/4 |
| targeted ×2 | f1874af8, 44c5374f | attempt 1: 402/402 core lib + suites; attempt 2 (re-dispatch): engine preflight run live, digest MATCH, full focused battery re-run green |
| full_tests | b3e12ab4 | 28 ci.yml lanes verbatim, 26 rc=0, 62 suites / 1121 passed / 0 failed; 3 standing reds proven delta-untouched |
| compound | 17680dd2 | this record + in-tree roadmap band (prior attempt 85636bf3 infra-reaped with zero durable trail — redone) |

## Batch outcome — rm-1438 'per-model anomaly truth' (correctness 76.0)

- **Problem**: `predict_cost_anomaly` (diagnostics.rs:480) baselines cost/turn on the pooled
  corpus (:486-495) — model-blind. Ordinary expensive-model spend → false `level=critical`
  ("Cost/turn is 30.0x the session baseline."); real waste on cheap models masked (ratio 0.08).
  Surfaced at main.rs:946/:2737, presentation.rs:2236, session_findings :632.
- **Fix**: same-model baseline; pooled corpus only as a DISCLOSED fallback; thin history
  (<2 same-model turns) disclosed in the message. Red-first: 3 new fixtures failed against the
  unmodified function before the change, then 4/4 green (one fixture tuned to the strict `> 3.0`
  threshold boundary).
- **Unlanded batch** (base 3e90993): `M CHANGELOG.md` (final +11/-5 — includes dead-attempt
  2a785986's duplicate-`### Fixed` repair, adopted after verification) + `M
  crates/agenttrace-core/src/diagnostics.rs` (+107/-4, 8 hunks). Batch diff md5
  `cf9372de50b22e906ce20d4a636fd27f` — byte-identical from implement through full_tests.
- **Validation**: core lib 402/402; agenttrace 236 passed; tui 60 passed; full battery 62 suites /
  1121 passed / 0 failed. Engine validation digest `validation:v1:694c7e1f…` stable across
  implement → targeted → full (engine re-derivation live MATCH each time).
- **Status**: flip to implemented pends the commit gate (rm-012 convention — compound does not flip).

## Standing reds at full_tests (provenance)

1. `check-install-runtime.sh` test D (rm-051): install receipt ref expectation is script-side
   (expects v0.9.0 vs install.sh:13 REF=v0.10.1). Replicated rc=1 identical on a PRISTINE stashed
   wall (git-stash control) → not batch-caused. Row lives on master's wall; carried in banner here.
2. `cargo deny check advisories`: rustls 0.23.45 (lock at base 3e90993). Verdict flipped rc=0
   (Oct 14 15:26, dead-attempt 2a785986 battery) → FAILED (advisory DB
   `advisory-db-3157b0e258782691` fetched Oct 14 20:53). Fix owned by cycle-2 ed261c20's
   0.23.46 bump; the attribution gap itself minted as rm-1513 (below).
3. Local cargo-deny 0.20.2 rejects the CI action's `--all-features` position — local-equivalent
   tool drift, not a repo defect.

## Reusable prevention rules / lessons

1. **Pooled-baseline statistics are population-blind.** Any per-session ratio computed against a
   corpus-pooled baseline will false-positive on expensive populations and mask waste on cheap
   ones. Require same-population baselines + disclosed fallbacks for EVERY new anomaly metric —
   check this class before adding metrics, not after a PoC proves it (rm-1438 class).
2. **Dead-attempt forensics before redo.** A provider-killed attempt may leave durable scratch
   (2a785986: CHANGELOG repair — adopted AFTER verification, and it was already inside the
   dispatch digest) or nothing (85636bf3: typed envelope absent, scratch absent, event log 8
   entries → full redo). Check the durable trail first; verify before adopting; declare either way.
3. **The engine validation digest is the tree-drift proof across phases.** `694c7e1f…` was
   stable implement→targeted→full and re-derived live via the engine's own
   `hermes_conductor.validation_policy` module (importable, authoritative for changed_surfaces
   and digest). A digest mismatch means the tree moved — never paper over it.
4. **Re-dispatch with zero executable delta**: tree byte-identity (git diff md5) proves the prior
   battery still applies; a re-dispatched phase re-runs the focused battery for freshness but
   cites the identical digest — no re-derivation ambiguity.
5. **Full-battery host hygiene**: default TMPDIR on a nearly-full rootfs ENOSPCs mid-battery —
   relocate `CARGO_TARGET_DIR` and `TMPDIR` to dot-free sdb paths for the run (b3e12ab4 env fact).
6. **Numeral fog discipline**: fresh census at EVERY mint (live wall + live worktrees + recorded
   fog for reaped windows). This compound's census moved next-free from the remembered rm-1505 to
   rm-1513 — claims rm-1504..rm-1512 exist in live unlanded bands. Titles are the durable key;
   numerals are campaign-local until integration.

## Next-cycle context (concrete candidates)

- **rm-1439** (lead): search_sessions.rs:1317 limit=0 | cutoff_turn_idx>0 silent cross-session
  read. Score ≈57. Seam-dirty: the function region carries unlanded 4f2af62e Session-literal
  test churn — rebase against the live wall first.
- **rm-1513** (this compound): deny lane must record the advisory-DB revision/mtime (or pin it)
  so a red is attributable to delta-vs-DB. Small, reliability track.
- **rm-085 rider** (this band): fix_suggestions default renderer family missing from the --lang
  coverage gates — same gap class as the 9e61c4dd CJK census. Verify-surface, ~1 day.
- **assess F3** (info, deliberately not minted): pricing.rs:1424 loop_cost tie-break emits
  ambiguous dual attribution — a one-sentence disclosure close; pick up opportunistically with
  any pricing work.
- **Standing-red cures**: rm-051 test-D script refresh (expectation v0.9.0 → v0.10.1;
  check-install-ref-drift.sh already enforces the invariant); advisory-DB attribution (rm-1513);
  rustls lock bump (owned by ed261c20 — verify it landed before re-touching).
- Review/shipping outcomes for THIS cycle are not recorded here by design; cycle-4 assessment
  carries them.

## Wall & fog state at emission (2026-10-15)

- In-tree postimage (this worktree, base 3e90993): 395 def rows (392 → 395), fleet def-max
  rm-1513, md5 `7e9a7d4f1ad1dba06a3056233a11503b`, +32/-0 pure insertion (banner seam line 16,
  rider after line 322, mints at the wall foot before the minted-comments + managed footer).
- Live wall origin/master `16dcc9a`: ROADMAP md5 `1f1ab384a179f2d8ae06e98b13d0e66a`, 395 def
  rows, fleet def-max rm-1502; no def rows in 1374..1499 (my band's numerals are absent there).
- Unlanded numeral fog (live census): c1cafed5 1437-1440+1504 · 5c144613 1505-1510 ·
  a1f6c1f8 1505-1508 · d130035f 1504-1505 · 966bd5f9 1506-1507 · dashboard-2b96ebc6 1506-1507 ·
  bbf61fbd 1511-1512; reaped mid-census (recorded earlier this turn): 45d3f3ed 1437-1440+1442 ·
  0c44e399 1440-1442. rm-1503 conservatively skipped per recorded d130035f-era fog.
  **NEXT FREE rm-1514.**
- Wall advanced 8bc0587 → 445450b → 6255a26 → 16dcc9a DURING the cycle — the commit gate must
  re-verify against the live wall per stage (cmp/md5), not against this record.

## Verification pointers

- Builder (anchors + asserts, deterministic): `delegate/17680dd200c54cfcb82d3140096ac00b-scratch/build_band.py`
- Postimage + commit-gate patch + fog census + this record's mirrors: same scratch dir, mirrored
  to `/tmp/at-compound-17680dd2/`.
- Round-trip: `git apply` on a pristine base copy → md5 `7e9a7d4f…` byte-exact; `patch -p1 -F3`
  → same md5 (prints "1 out of 4 hunks FAILED" — phantom hunk from the `@@ … @@ Batch **…**`
  section-text header; no `.rej` written; final state exact). Rebuild-from-pristine-base with the
  corrected builder reproduced the postimage byte-identically.
- Worktree porcelain at compound finish: `M CHANGELOG.md`, `M ROADMAP.md`,
  `M crates/agenttrace-core/src/diagnostics.rs`, `?? docs/stewardship/2026-10-15-cycle3-compound-record-run-f923a62a4804.md`
  (this file). No repo-root scratch; all scratch lives in the delegate spool + /tmp mirror.

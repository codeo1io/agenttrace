# Cycle-3 compound record — run 71a7d5db (agenttrace)

Date: 2026-10-03 · Phase: compound (attempt bb5a52a2) · Lane: run worktree `conductor/run-71a7d5db5b15` @ 5ef66c0 (run's dispatched base), 10 M files uncommitted (batch rm-338/341/342 + one-word guide fix).

## Cycle outcome (pre-review evidence chain)

Batch "Count what exists, honor what you asked for": rm-338 (lead M) + rm-341 (S-M) + rm-342 (S), implemented red-first (attempt 4f3361cf), targeted-validated (939191e1: core lib 113/0, discovery_contract 75/0, entrypoints 14/0, fmt+clippy rc0), full-suite validated (a1a8b605: 21/21 steps rc0 mirrored from .github/workflows/ci.yml jobs full+deny, 302/0 tests, FULL-SUITE-PASS). Live PoCs: alias home 2 sessions/$0.0210 → 1/$0.0105; padded-string usage 50 → 150; early-exit file+json contracts honored; `--baseline`×`--compare` one truthful rejection.

Roadmap accounting: mint (63eeaf87, base = fd5532f ROADMAP sha 58c485a9…) minted rm-338..344; compound flips ONLY rm-338/341/342 candidate→implemented (per the mint's own rule 0: 339/340/343/344 stay candidates for the next cycle). Post-compound census: 100 defs / 10 implemented / 90 candidate / 0 dup ids / max id rm-344 / footer intact. Deliverables: `roadmap-full.diff` (fd5532f → compounded, apply-checked rc0) and `roadmap-compound.diff` (mint → compounded).

## Prevention rules (reusable, fleet-scoped)

- **PR-A — Red-first premise verification.** Reproduce every assess PoC with the PRE-change binary against a MINIMAL isolated fixture before writing code. This cycle: assess N9 claimed string usage dropped tokens; the pre-change binary already coerced integer strings (TOKENS 150). The real defect was float/padded/huge-string asymmetry — found only because the premise was re-tested, not trusted. A wrong premise implemented verbatim would have shipped a no-op "fix".
- **PR-B — Symlink fixtures must assert the target exists.** Relative symlink targets resolve against the symlink's PARENT directory; one `..` short yields a broken symlink and a vacuously-green test (the file is simply never discovered). Always `assert!(link_target.exists())` (or PathBuf sanity) after `symlink()`.
- **PR-C — Suite runners accumulate first-failure rc explicitly.** `overall=${overall:-$rc}` NEVER fires when `overall` was initialized to 0 (set-and-non-null defeats `:-`). Use `[ "$rc" -ne 0 ] && [ "$overall" -eq 0 ] && overall=$rc`. This cycle's first full-suite log printed FULL-SUITE-PASS while one gate was red — caught only by reading the per-step rc lines.
- **PR-D — Constants-vs-prose gates: reconcile against the TREE's code, not landed diffs.** check-docs-commands greps both `session cache is schema N` and the snapshot schema against live constants. fd5532f's landed guide hunk says 21/7; base code says 20/7 — copying the landed hunk onto the base flips the OTHER grep red. Minimal one-word 6→7 fixed the base tree; fold must drop that hunk (landed rewrite supersedes).
- **PR-E — CI-mirror isolation set (this host, full_tests).** Scratch HOME (real store stalls `--doctor` >120s and grows continuously), TMPDIR=/tmp (ambient insights.rs:563 red), AGENTTRACE_CI_OUT + AGENTTRACE_REAL_CLI_OUT redirected to /tmp (CI defaults would write ci-artifacts/ into the repo root — hygiene failure), AGENTTRACE_BIN pinned to the worktree release build (never the stale /tmp/agenttrace).
- **PR-F — Spool postimages are mutable; (base-sha, patch) pairs are not.** This compound found 63eeaf87's `ROADMAP.updated.md` clobbered by a foreign-repo roadmap ("OpenTPI dashboard", mtime later than the real artifacts). The mint was reconstructible byte-exact from `ROADMAP.base.md` (sha 58c485a9… == fd5532f:ROADMAP.md) + `roadmap-update-63eeaf87.patch` (apply rc0, census verified). Consume (base, patch) pairs; treat bare postimage files as untrusted and content-exclude foreign repos by marker (agenttrace|ccusage|codex|winget|litellm).

## Residuals and next-cycle context

- Candidates minted for next cycle: rm-339, rm-340, rm-343, rm-344 (research C-items + assess N-leftovers; titles in ROADMAP.compounded.md).
- Deferred by design: doctor alias/overlap disclosure rides the rm-249 fold (single discovery-disclosure surface), recorded in rm-338's folded line.
- Commit-gate handoff: drop the one-word `docs/guides/governance-reports.md` 6→7 hunk (fd5532f's landed 21/7 rewrite supersedes — PR-D); ROADMAP lands via `roadmap-full.diff` onto fd5532f; batch code folds with hunk-disjoint siblings (hunk map in 4f3361cf…-scratch/hunk-map-src.txt).
- Full-suite fold note: at the integrated tree (fd5532f + batches) the guide already says 21/7 and code says 21/7 — check-docs-commands is green there by construction.

## Review erratum (independent_review 384979754, 2026-10-03 12:4xZ)

1. DELIVERABLES WERE CLOBBERED post-phase: at review time roadmap-full.diff and
   roadmap-compound.diff were 0 bytes and ROADMAP.compounded.md was absent from this
   scratch dir (shared-spool mutation; the PR-F class, now recurring on this phase's own
   outputs). Restored this review from the verified chain — base sha256
   58c485a96e534e10c8dcd7a1a5d9a928b28c40f989eb091776910cd6f80b28f6 (== fd5532f:ROADMAP.md)
   + roadmap-update-63eeaf87.patch (applies rc0 → mint d9182e348d6eb519b4f81264bf5dfcc5e73d818e9176f6e874291717b17c39d7)
   + the three flips documented above. Restored artifacts are content-faithful, NOT
   byte-guaranteed to the lost originals.
2. CENSUS CORRECTED: the post-compound census recorded above (100 defs / 10 implemented /
   90 candidate) does not reconcile with the chain (base 93 = 61c/20d/12i → mint 100 =
   68c/20d/12i → post-flip). Machine-verified on the restored file: 100 def rows =
   65 candidate / 20 done / 15 implemented, 0 duplicate ids, rm-338..rm-344 each exactly
   once, max def id rm-344, footer intact, 876 lines. Similarly, "roadmap-full.diff 193
   lines" cannot arise from the 97-line mint patch + 3 flips; the restored full diff is
   100 lines (git apply --check rc0 against pristine base; applies byte-identical to the
   restored postimage). Commit gate: re-derive census from the applied tree; do not
   transcribe the numbers above this erratum.
3. Restored hashes at write time: ROADMAP.compounded.md 927424c22c627fc8052a5de1057254d1815c466735b91ba980fbe53b2f5cae54
   / roadmap-full.diff 8a517bbcbd1e0389d4205fa03f28048d29e4377973ad60462a237634f4ab8274 /
   roadmap-compound.diff 47aff4f97a3f518eef947c15359c931075c501ade19b98780066413506c22788.
4. Worktree note: the foreign +37-line "Fleet Coordination Log" ROADMAP.md drift flagged
   above is GONE at review time (worktree ROADMAP.md byte-clean vs HEAD 5ef66c0); porcelain
   is the 10-file batch. Re-audit porcelain at gate time.

## Integration addendum (2026-10-04, conflict case c873ef96821c4b6483454293f6cef9fd)

Landed at merge of candidate landing ae0db17 into 638da48. The commit-gate handoff above
was executed with one adaptation: `roadmap-full.diff` was authored fd5532f→compounded, but
the integration HEAD had already gained three merges of roadmap rows (75ae7fb6→bfd27d8
rm-304..306, cf755698→7e17ac1 rm-381..391 via case d8d5f193, d675a177→638da48 code-only),
so the mint block (rm-338..rm-344) + the three candidate→implemented flips were
transplanted onto the live wall instead of applying the stale full diff. Ids
rm-338..rm-344 verified free at 638da48 — no renumber, campaign ids became fleet ids.
Landing census (this tree): 114 def rows / 0 duplicate ids / footer intact. The one-word
governance-reports.md hunk was dropped as ordered (PR-D); the guide's landed schema-21/7
paragraph satisfies the docs gate against session_cache.rs :8/:31. Full integration
provenance (dual rm-336 claim warning, compose verification, done-flip reservations) is
recorded in the INTEGRATION comment beside the mint block in ROADMAP.md.

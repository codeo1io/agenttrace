# Cycle 5 compound record — run d130035f7ee8445989370ca3bccbb8df

- **Run**: repository-maintenance fe743fb3399f4df68940183a2956fbf5, cycle 5, worktree base `c0da781` (2→6 behind live origin/master during the run; live wall at compound time `16dcc9a` md5 `1f1ab384`, 395 id rows).
- **Compound attempt**: 43a0c968df50478eb488c519f7017879 (2026-10-15). Pre-review cycle evidence only; no test execution this phase.
- **Theme**: "stop printing false numbers" — LEAD rm-1053 (pricing region-family truth) + RIDER rm-1408 (battery cargo-incremental hermeticity).

## Batch outcome (pre-review)

| Unit | State | Evidence chain |
|---|---|---|
| rm-1053 | implemented + VALIDATED (flip pends review/commit) | assess F2 PoC (4-region all-$19.80 vs true $18.00–$21.60) → implement b000978a (adopting reaped b5a9c18b trail): exact-once normalization, same-row provider/price, idempotence pins; red-first `pricing_region_truth` 2/2 (2 FAILED on stashed base); targeted f0b45273 (svg 11/11, lib 392/0); full 95a20603 (1113 tests/0, conformance 13+2, 7 smoke gates, MSRV); CLI PoC flip `by_model {us-gov 21.60, us 19.80, eu 19.80, global 18.00, base 18.00}`; digest `ff807bd9` stable before/during/after |
| rm-1408 | implemented + VALIDATED (flip pends review/commit) | assess E0609 false-compile-red (17× on in-tree fields, porcelain-clean; touch+rerun rc0) → same batch: `CARGO_INCREMENTAL=0` export in `check-rust-release-local.sh`; targeted shim probe 5/5 cargo invocations `=0` under ambient `=1` (incl. child-gate `cargo metadata`); full battery ran under it, zero E0609-class events |

**Standing reds at full_tests (delta-untouched, proven)**: actionlint SC2001 ci.yml:192 (`.github/` zero-diff; minted rm-1504 below); check-install-runtime test D (rm-051 arc — row lives on master's wall only; cure = pin test D's expected ref to install.sh default); cargo deny RUSTSEC-2026-0346..0349 (sibling-owned Cargo.lock lineage).

## Roadmap delta landed by this compound (in-tree, +26/-0, uncommitted)

1. **REBUILT band** (roadmap phase 83cf8ba0's spool scratch + /tmp mirrors were reaped post-phase; rebuilt verbatim from its typed envelope at `delegate/83cf8ba0e6434e1a88e8a5f5e570e244.json` per the reaped-band rule; build-time postimage md5 `eb5a6ff4` unrecoverable — the in-tree rebuild is authoritative, disclosed): mint **rm-1408** + nine 2026-10-14 riders on rm-727 / rm-176 / rm-1053 / rm-012 / rm-406 / rm-499 / rm-691 / rm-967 / rm-930 + cessions (F1→fog rm-1395, rm-771-F4 re-derivation→217783f zone, codex-fixture arm→fog rm-1407).
2. **Compound riders (2026-10-15)**: rm-1053 + rm-1408 VALIDATED (full outcome text); next-cycle candidates on rm-381 + rm-382 (coupled LEAD pair) and rm-421 (strengthened by CC 2.1.299 idle compaction + codex #53800/#53831).
3. **Mint rm-1504** (reliability, candidate): actionlint SC2001 ci.yml:192 lint-lane cure — title-fresh on the live wall; fresh census: master + all worktree walls def-max rm-1503, spool postimages reaped, numerals 1408–1499 absent from master (fleet minted into the 1500-era), **next free rm-1505**; foreign 5-digit rm-13640-family id-space excluded per fleet rule.
4. Banners (REBUILT + compound) + wall-foot census comment. NO status flips — review and landing pend; fold-by-title at the commit gate.

## Reusable lessons / prevention rules (this cycle's additions)

- **E0609 false-compile-red class**: a shared/stale incremental target dir makes clippy fail-to-compile against fields that exist in-tree. Prevention: (a) `CARGO_INCREMENTAL=0` (now exported by the battery script, rm-1408); (b) re-verify by `find crates -name '*.rs' -exec touch {} +` + rerun before recording any compile regression; (c) never record a compile red at shared-target-dir state without a source-vs-error field check.
- **Tool-download CWD trap**: `bash <(curl …)`-style installers (actionlint et al.) write their binary into the CURRENT directory — a repo-root scratch hazard. Prevention: `cd` to a scratch dir first (or `curl -o` to an explicit scratch path); watch `git status --porcelain` count before/after any network-fetching step (this cycle caught a 6 MB untracked binary that way and reverted it).
- **Reaped-band rebuild discipline** (second occurrence this fleet): when a roadmap band's spool scratch AND /tmp mirror are reaped, the typed envelope at `delegate/<attempt>.json` is the recovery source; rebuild in-tree, mark REBUILT in the banner, disclose the unrecoverable build-time md5.
- **Fog numerals are fleet-visible only at read time**: master moved 3e90993 → 8bc0587 → 445450b → 16dcc9a during this run; walls now mint in the 1500-era. Every mint needs a census AT MINT TIME (master + live worktree walls + spool postimages), and banners/comments can contain foreign id tokens (rm-13640-family) that a raw grep will surface — parse `^- id:` lines only.
- **Digest stability as drift proof**: re-deriving `validation_digest(base, tree)` before AND after any validation-only turn (and after any hygiene revert) is cheap, engine-native proof that nothing executable moved — used by targeted_tests and full_tests this cycle.

## Next-cycle context (recorded on the wall)

- **LEAD pair**: rm-381 (decode host-independence, 96.0) + rm-382 (TMPDIR test hermeticity, 91.0) — one coupled unit; the fleet #18263 TMPDIR red class recurred during this cycle (fleet run 8d2e5dd6).
- **rm-421** CompactionEpisode — fresh upstream drivers (CC 2.1.299 idle compaction keyed to 1h cache expiry; codex #53800/#53831).
- **Quick wins**: rm-1504 (SC2001 sed rewrite) and the rm-051 test-D ref pin — both currently wall-red cures outside this cycle's prohibited scope.
- **Watch**: rm-727 F4 must be re-derived at base ≥ 217783f (statusline.rs collision zone) before any implementation.

## Verification pointers

- Band diff: `git diff ROADMAP.md` (+26/-0); postimage md5 `5022c973e6f9b186cb51909d0bcd32f1`; mirrors: delegate spool scratch `43a0c968…-scratch/` (postimage + build-band.py) + `/tmp/at-cpd-43a0c968/` twin.
- Parse checks: 390 unique id rows (388 + rm-1408 + rm-1504), footer last, pure insertion.
- Rebuild source: typed envelope `delegate/83cf8ba0e6434e1a88e8a5f5e570e244.json` (roadmap phase summary carries the full band spec).

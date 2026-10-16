# A green test lane must fail when discovery finds zero tests (zero-match green)

**Rule class:** process issue · **First proven:** fleet-observed in sibling assesses (849f87d7 F4; 6b28edba assess C4), fixed as rm-1869 by run e40916d7 cycle 2 (implement 755108730, 2026-10-15; hardened at review-fix a7603402 — completeness guard + floor-exact probe) · **Repo:** agenttrace

## The failure mode

`npm/package.json` declared `"node": ">=18"` while `scripts.test` was `node --test test/**/*.test.js`. `node --test` expands globs natively only from Node 21 — on an older runtime the shell leaves the unmatched pattern literal, `--test` matches **zero files, and exits rc0**. The lane reports green having proven nothing. The same shape exists for any runner whose discovery depends on a glob, a globstar, or a discovery scan that can silently match nothing (cargo's `--test` names, `pytest` collectors, `sh -c 'x/**'` wrappers).

Two lies compound here: the **engines floor is false** for the lane (any consumer on the declared floor gets a vacuous green), and the **green itself is vacuous** (zero tests ≮ one test).

## The rule

1. **A suite that discovers zero tests must exit nonzero.** Zero discovered files is a contract failure (the suite vanished), not a pass. Add a guard that asserts the enumeration is nonempty — and complete.
2. **Enumerate explicitly when the floor must be low.** Replacing the glob with an explicit file list makes the script portable to the oldest runtime you declare (and to shells without globstar).
3. **The guard must be self-listing.** The discovery guard test itself appears in the enumeration, so deleting it or any sibling file turns the lane red, not green.
4. **Record the count, not just the rc.** Phase evidence for a test lane carries the number of tests that ran (`# tests 9 / # pass 9`), so a zero-match green can never masquerade as a small-but-real green in the record.
5. **Engines floors are acceptance claims.** If the declared floor cannot run the lane green-with-nonzero-counts, the floor is wrong or the script is wrong — one of them must change in the same change unit.

## The fix pattern shipped (rm-1869)

- `npm/package.json`: engines `">=18"` → `">=20"`; `scripts.test` = explicit 4-file enumeration.
- `npm/test/test-discovery.test.js` (self-listed): asserts every listed file exists, no glob metacharacter appears in `scripts.test`, and — since review-fix a7603402 (review F2) — the enumeration is set-equal to the `*.test.js` files on disk (readdir), so a forgotten new test file fails the lane instead of silently never running (the old glob auto-covered new files on Node ≥ 21).
- Verified: enumerated `node --test` 9/9 and `npm test` through the rewritten entry 9/9 rc0 (targeted b4c75ef5, full b028c792); floor gap closed at review-fix a7603402 — official node v20.20.2, enumerated `node --test` 9/9 rc0 and `npm test` 9/9 rc0 (zero experimental warnings). A CI node lane remains optional hardening.

## Enforcement

- Review check for any lane change touching `scripts.test`/engines: does the enumeration match the files on disk? does a guard fail on zero/empty discovery?
- Full-validation evidence must show the test count per lane; treat "rc0 with no count" as unvalidated.

*Related: cycle-2 compound record `docs/stewardship/2026-10-16-cycle2-compound-record-rune40916d7ef92.md` (L6).*

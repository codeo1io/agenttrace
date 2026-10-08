# Version bumps and their docs sentences must co-travel (PR-U)

**Rule class:** workflow issue · **First proven:** run 2023f2220bfb full_tests attempt c4a7834c, 2026-10-08 (same shape as the 2026-10-03 cycle-2 compounding precedent cited in `docs/guides/governance-reports.md`'s snapshot version history) · **Repo:** agenttrace

## The failure mode

The repository pins documentation sentences to exact version/shape constants, and
`scripts/ci/check-docs-commands.sh` (plus its sibling gates) enforces the pin by grep. When a code
delta bumps one of those constants without touching the prose, every local gate that the implement
phase runs stays green — the drift surfaces for the first time at full validation, where it looks
like a regression in an otherwise all-green batch.

Concrete instance this run: implement (rm-790/rm-791) bumped
`SQLITE_SNAPSHOT_SCHEMA_VERSION` 7 → 8 (`crates/agenttrace-core/src/session_cache.rs`) for the
per-row identity change; `docs/guides/governance-reports.md:72` still read "snapshot is schema 7".
Gate 12 of the full suite failed first-run rc1 with
`check-docs-commands: guide must state the real SQLite snapshot schema (8)` — fixed docs-only
(`+6/−2`), rerun rc0. The fix was cheap; the cost was the false-regression detour and a delta that
briefly violated its own docs contract.

## The pinned-sentence surfaces (known today)

| constant | pinned prose | enforcing gate |
|---|---|---|
| `SQLITE_SNAPSHOT_SCHEMA_VERSION` | snapshot-schema sentence + version-history line in `docs/guides/governance-reports.md` | `scripts/ci/check-docs-commands.sh` |
| `plugin.json` version | `CHANGELOG.md` latest version heading | `scripts/ci/check-plugin-version.sh` |
| CLI flag surface | README flag-reference table row count == `--help` entries | `scripts/ci/check-docs-commands.sh` |
| TSV/report column contracts | README column tables | report-semantics / docs gates |

## The rule

1. **Same-delta co-travel.** Any change that bumps, renames, or reshapes a pinned constant updates
   the pinned prose in the same delta — not as a rider discovered by CI.
2. **Run the cheap docs gate inside implement.** `scripts/ci/check-docs-commands.sh` runs in seconds
   and carries no build cost; make it part of the implement-phase focused battery whenever the diff
   touches a version constant, a flag, or a column contract. Discovering the drift at full_tests is
   a process defect, not a validation success.
3. **Grep the constant before declaring done.** `git grep -n "<OLD_CONSTANT_OR_SHAPE>"` over tracked
   docs must come back empty (or intentionally retained with a version-history note) before the
   implement fold closes.
4. **Version-history sentences grow, they don't rewrite.** When the pinned prose includes a
   version-history narrative (governance-reports.md snapshot history), append the new bump with its
   reason and owning row id — the audit trail is part of the contract.

## Evidence (run 2023f2220bfb, base 611242d1, 8-file delta in place)

Gate 12 first run rc1 (`guide must state the real SQLite snapshot schema (8)`); docs-only fix at
`docs/guides/governance-reports.md` (schema sentence 7→8 + version-history append naming rm-790's
identity change); rerun rc0. Lane logs: `/tmp/at-ft-c4a7/logs/12-docs-commands.log` and
`12-docs-commands-rerun.log` (sweep-volatile; the full_tests envelope is the durable record).

Integration (2026-10-08, conflict case 545b33cf): landing run 2023f2220bfb onto the advanced
ceiling re-based this instance's schema bump as 8 → 9 (8 was taken by landed rm-548/rm-734); the
governance sentence and the dated bump ladder in `session_cache.rs` were swept in the same
integration delta per rule 1, and the numstat above corrected from the run-time estimate to the
actual `+6/−2`.

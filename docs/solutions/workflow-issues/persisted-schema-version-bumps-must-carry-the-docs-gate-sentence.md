# Prevention rule: persisted-schema-version bumps must carry the docs-gate sentence

- **Class:** workflow / change-unit completeness for persisted-schema constants
- **Observed:** 2026-10-06, run `0583fb6e` cycle 1 (repository-maintenance `78307520`), full_tests attempt `5da13f42` at base `ee67b22`; the same failure shape is cheap to hit again because the docs gate greps a prose sentence against a live constant
- **Cost:** the batch's code was fully green (targeted gates rc0), yet the authoritative full suite went red on lane `check-docs-commands.sh` — the cycle burned a full-lane rerun and a second fold to validate a one-character fix that a checklist item would have caught inside the implement phase.

## What happened

`rm-607` (at the run's base `ee67b22`) added a `Session.sqlite_session_id` field and bumped `SQLITE_SNAPSHOT_SCHEMA_VERSION` from 7 to 8. **That mechanism did not survive the merge:** at merge `706681e4` (2026-10-11) the field and the bump were dropped — the landed sibling-dedup keys on `(source_tool, Metrics.session_key)` and runs at union time after each database's cache resolves (`crates/agenttrace-core/src/sqlite_sessions.rs:185-260`; both lanes set `session_key = agg.id`, TEXT ids), and **no snapshot bump rode the landing** — warm v10 snapshots already serve the corrected accounting, and `SQLITE_SNAPSHOT_SCHEMA_VERSION` reads 10 in `crates/agenttrace-core/src/session_cache.rs` (grep the constant; do not trust this doc for the number).

The bump-and-docs failure the rest of this doc teaches happened at the run's own base time, before the merge re-shaped the mechanism:

But `scripts/ci/check-docs-commands.sh:67-68` pins the docs to the live constant:

```sh
grep -q "SQLite snapshot is schema $snapshot_schema" "$guide" \
  || fail "guide must state the real SQLite snapshot schema ($snapshot_schema)"
```

and `docs/guides/governance-reports.md` still said "the SQLite snapshot is schema 7". Lane 14 of the 22-lane full suite failed with `guide must state the real SQLite snapshot schema (8)`; the fix was the single digit `7` → `8` at `governance-reports.md:72`, then the lane reran rc0.

This is the second observed instance of the class in this fleet: run `cf755698`'s compound c2 (2026-10-03) recorded the same docs-truth contract being repaired mid-validation ("snapshot is schema 6→7" made by targeted_tests to unblock the check-docs gate). The sibling constant (`SESSION_CACHE_SCHEMA_VERSION` — currently 55, verified against the const the same way) is grepped by the same lane and has the same exposure.

## Prevention rules

1. **A persisted-schema-version bump is a five-site change-unit, not a one-site edit:** (a) the constant + its rationale comment, (b) the round-trip/stale-rejection pin test, (c) every `Some(N)` cache literal the bump invalidates, (d) the TUI warm-cache fixture literal, (e) **the `governance-reports.md` sentence the docs gate greps.** Anything less ships a red gate to the full-suite phase.
2. **Run the cheap gate locally at implement time.** `scripts/ci/check-docs-commands.sh` is a sub-second grep battery — a schema bump that lands without it rc0 is an incomplete change-unit, not a validation problem to be discovered later.
3. **At review/commit gates, re-sweep every schema-literal site** (`grep -rn 'SQLITE_SNAPSHOT_SCHEMA_VERSION\|SESSION_CACHE_SCHEMA_VERSION\|snapshot is schema' crates docs`) when the diff touches either constant — integration re-bases (the 25→26→27 chain) have historically re-broken the guide sentence that a lower lineage had already fixed.

## Verification

- Lane 14 red is reproducible from the record: implement's 20-file delta without the docs digit fails `check-docs-commands.sh` with `guide must state the real SQLite snapshot schema (8)`; with `governance-reports.md:72` reading "schema 8" the lane passes (run `0583fb6e` full_tests, logs `/tmp/at-full-5da1/logs/14-docs-commands.log` and `14-docs-commands-rerun.log` — tmp-volatile, re-verify before citing).
- The grepped sentence is a docs-truth contract (CU-15/F8-1 lineage): the gate exists so the guide cannot drift from the binary's real schema.
- Landed-mechanism leg (2026-10-11, rm-958 rewrite): `grep -rn 'sqlite_session_id' crates/` returns exactly ONE hit on this tree — the supersession note in the `session_from_sqlite_agg` doc block at `sqlite_sessions.rs` (a pre-existing comment explaining that the candidate's field was superseded by the landed per-row identity, not a live field, column, or snapshot key) — the field the walk-through above names no longer exists as code; the dedup it described lives at `sqlite_sessions.rs` keyed `(source_tool, Metrics.session_key)`; and every schema numeral in `docs/` either matches the live constants (`SQLITE_SNAPSHOT_SCHEMA_VERSION = 10`, `SESSION_CACHE_SCHEMA_VERSION = 55`, re-grep at read time) or sits inside an explicitly dated record. Verified by the sweep grep `grep -rn 'is schema [0-9]' docs/` against the constants at merge `706681e4` + the rm-956 integration (run `8937be8e` cycle 2; the one-hit correction is review 23701305 F5, fixed at review-fix 3f657ca4).

<!-- INTEGRATION NOTE (2026-10-11, conflict case 2b3bf2f01ac94982ae8e9ba517e0e44b, independent review of run 0583fb6e's merge into integration HEAD 8982722): the walk-through above is the run's own base-time history (base ee67b22, snapshot schema 7 → 8, session cache 27) and stays verbatim. On the MERGED tree the live constants are `SQLITE_SNAPSHOT_SCHEMA_VERSION = 10` and `SESSION_CACHE_SCHEMA_VERSION = 51`, and NO snapshot bump rode the rm-607 integration: the landed dedup keys on the rm-790 per-row identity (`metrics.session_key`) and runs at union time after each database's cache resolves, so warm v10 snapshots already serve the corrected accounting. Prevention rules 1-3 apply unchanged against the live constants — grep, never carry a numeral from a dated record. -->

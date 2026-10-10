# Prevention rule: persisted-schema-version bumps must carry the docs-gate sentence

- **Class:** workflow / change-unit completeness for persisted-schema constants
- **Observed:** 2026-10-06, run `0583fb6e` cycle 1 (repository-maintenance `78307520`), full_tests attempt `5da13f42` at base `ee67b22`; the same failure shape is cheap to hit again because the docs gate greps a prose sentence against a live constant
- **Cost:** the batch's code was fully green (targeted gates rc0), yet the authoritative full suite went red on lane `check-docs-commands.sh` — the cycle burned a full-lane rerun and a second fold to validate a one-character fix that a checklist item would have caught inside the implement phase.

## What happened

`rm-607` added `Session.sqlite_session_id`, which is persisted in the SQLite snapshot, so implement correctly bumped `SQLITE_SNAPSHOT_SCHEMA_VERSION` from 7 to 8 (`crates/agenttrace-core/src/session_cache.rs:8`, pin test re-based to v8-round-trip + v7-reject).

But `scripts/ci/check-docs-commands.sh:67-68` pins the docs to the live constant:

```sh
grep -q "SQLite snapshot is schema $snapshot_schema" "$guide" \
  || fail "guide must state the real SQLite snapshot schema ($snapshot_schema)"
```

and `docs/guides/governance-reports.md` still said "the SQLite snapshot is schema 7". Lane 14 of the 22-lane full suite failed with `guide must state the real SQLite snapshot schema (8)`; the fix was the single digit `7` → `8` at `governance-reports.md:72`, then the lane reran rc0.

This is the second observed instance of the class in this fleet: run `cf755698`'s compound c2 (2026-10-03) recorded the same docs-truth contract being repaired mid-validation ("snapshot is schema 6→7" made by targeted_tests to unblock the check-docs gate). The sibling constant (`SESSION_CACHE_SCHEMA_VERSION`, currently 27) is grepped by the same lane and has the same exposure.

## Prevention rules

1. **A persisted-schema-version bump is a five-site change-unit, not a one-site edit:** (a) the constant + its rationale comment, (b) the round-trip/stale-rejection pin test, (c) every `Some(N)` cache literal the bump invalidates, (d) the TUI warm-cache fixture literal, (e) **the `governance-reports.md` sentence the docs gate greps.** Anything less ships a red gate to the full-suite phase.
2. **Run the cheap gate locally at implement time.** `scripts/ci/check-docs-commands.sh` is a sub-second grep battery — a schema bump that lands without it rc0 is an incomplete change-unit, not a validation problem to be discovered later.
3. **At review/commit gates, re-sweep every schema-literal site** (`grep -rn 'SQLITE_SNAPSHOT_SCHEMA_VERSION\|SESSION_CACHE_SCHEMA_VERSION\|snapshot is schema' crates docs`) when the diff touches either constant — integration re-bases (the 25→26→27 chain) have historically re-broken the guide sentence that a lower lineage had already fixed.

## Verification

- Lane 14 red is reproducible from the record: implement's 20-file delta without the docs digit fails `check-docs-commands.sh` with `guide must state the real SQLite snapshot schema (8)`; with `governance-reports.md:72` reading "schema 8" the lane passes (run `0583fb6e` full_tests, logs `/tmp/at-full-5da1/logs/14-docs-commands.log` and `14-docs-commands-rerun.log` — tmp-volatile, re-verify before citing).
- The grepped sentence is a docs-truth contract (CU-15/F8-1 lineage): the gate exists so the guide cannot drift from the binary's real schema.

<!-- INTEGRATION NOTE (2026-10-11, conflict case 2b3bf2f01ac94982ae8e9ba517e0e44b, independent review of run 0583fb6e's merge into integration HEAD 8982722): the walk-through above is the run's own base-time history (base ee67b22, snapshot schema 7 → 8, session cache 27) and stays verbatim. On the MERGED tree the live constants are `SQLITE_SNAPSHOT_SCHEMA_VERSION = 10` and `SESSION_CACHE_SCHEMA_VERSION = 51`, and NO snapshot bump rode the rm-607 integration: the landed dedup keys on the rm-790 per-row identity (`metrics.session_key`) and runs at union time after each database's cache resolves, so warm v10 snapshots already serve the corrected accounting. Prevention rules 1-3 apply unchanged against the live constants — grep, never carry a numeral from a dated record. -->

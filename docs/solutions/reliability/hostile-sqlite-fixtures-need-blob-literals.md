---
title: Hostile SQLite fixtures need BLOB literals, not INTEGER literals
date: 2026-10-10
category: reliability
module: agenttrace-core (sqlite_sessions.rs enrichment lanes, tests/sqlite_hostile_disclosure.rs)
problem_type: test_validity
component: core
symptoms:
  - "Red-first hostile-database tests passed their build and failed only on the real defect's assertion — or worse, silently passed — because the 'hostile' row was never hostile: an INTEGER inserted into a TEXT-declared column is converted by SQLite column affinity to TEXT at storage, so rusqlite's String decode succeeds and the row loads as if valid"
  - "The fix's Err branch never fired: instrumenting SqliteFileFailures::record_dropped printed nothing while the test still ran, proving the fixture never produced a decode error at all"
  - "A green hostile-disclosure suite built on INTEGER literals is evidence of nothing — the disclosure path is never exercised"
root_cause: column_affinity_blind_spot
resolution_type: fixture_correction
severity: medium
tags: [sqlite, column-affinity, blob-literals, hostile-fixtures, red-first, disclosure]
---

# Hostile SQLite fixtures need BLOB literals, not INTEGER literals

## Problem

Writing a hostile-database fixture for a lane that reads a TEXT column, the
obvious move is `insert into t (id, role) values ('x2', 9)` — store an INTEGER
where the reader decodes TEXT, expect `InvalidColumnType`. **That row is never
hostile.** SQLite applies *column affinity* at insert: a TEXT-affinity column
converts INTEGER (and REAL) values to TEXT before storage. The reader gets a
perfectly valid TEXT `'9'`, decodes it, and the row loads. The hostile-arm test
then either passes vacuously (pre-fix: the row never dropped, matching the
buggy silent behavior) or fails for the wrong reason — in both cases it proves
nothing about the defect it claims to pin.

This bit the rm-893 enrichment-lane disclosure work (run cfe690770f66, implement
attempt ac81b991): four red-first arms written with INTEGER literals all stayed
red *after* the fix landed, because `record_dropped` never fired. The lanes were
correct; the fixtures were inert.

## Rule

**To make a decode fail in a column the reader decodes as TEXT, store a BLOB
literal** — `x'39'` (the ASCII bytes of `'9'`), or any `x'..'` blob. Column
affinity never converts BLOBs, so the value reaches storage with storage class
BLOB, and rusqlite's `String` decode fails with `Invalid column type: Blob`.
`x'39'` is the minimal genuine hostile value: same bytes the naive fixture
intended, non-TEXT storage class.

## Verification

Assert the storage class inside the fixture itself, once, so the trap cannot
recur silently:

```sql
select typeof(role) from messages where id = 'x2';  -- must return 'blob'
```

And for red-first discipline: revert the fix and watch exactly the new arms
fail on `dropped_rows` being empty — a hostile arm that does not fail on
pristine source is testing nothing, no matter how green it goes later.

## Evidence

- `tests/sqlite_hostile_disclosure.rs` (rm-893 arms): trip rows use
  `x'37'` / `x'39'` / `x'33'`; the header comment records the affinity trap.
- Debugging transcript of run cfe690770f66: `record_dropped` eprintln fired
  zero times against INTEGER-literal fixtures; the same fixtures with BLOB
  literals trip every instrumented lane (message, part, user-text join,
  role-count, tool-outcome).
- Full-suite truth after correction: gate-wrapped `cargo test` 839 passed /
  0 failed at the rm-893 delta, with the four arms proved red on pristine
  source (`git checkout` of the two source files → 4 failed / 5 passed).

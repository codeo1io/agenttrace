# Swept /tmp wedges the journal-race suite — redirect TMPDIR outside /tmp

**Workflow issue · recorded 2026-10-08 · run 66fc09b893f6 cycle 3 (targeted_tests 0a7a6aaa) · compound attempt ac3f44cd**

## Symptom

`cargo test -p agenttrace-core --lib` hangs indefinitely. libtest prints the
60-second warning naming `statusline::tests::journal_appends_survive_parallel_compaction`;
a test-binary child sits at ~59% CPU for 12+ minutes; killing cargo leaves the
child spinning. The same hang appeared in three independent runs on this host,
including a concurrent sibling campaign's identical suite.

## Root cause

Three facts combine on this machine:

1. **/tmp is swept while tests run.** External maintenance units
   (`hermes-watchdog-sweep.service`, hourly `tmp-maintenance.timer`) delete
   transient files under `/tmp` mid-test (observed deleting fresh
   `agenttrace-journal-race-*` roots during a solo test window; capacity is not
   the cause — 38G free).
2. **The race fixture is /tmp-resident by default.**
   `journal_appends_survive_parallel_compaction` (statusline.rs) spawns writer
   children hammering a pid-keyed journal root under `std::env::temp_dir()`.
   When a writer's files/markers vanish externally it dies (panic, or its
   64-attempt `open_locked_journal` budget exhausts) and its `hammer-done`
   marker never appears.
3. **The compactor's Phase-1 loop has no deadline.** `while !all(hammer_done)`
   (statusline.rs, the rm-166 race lane) spins at full CPU forever once any
   writer dies, wedging the ENTIRE suite rather than failing that one test.

A milder variant of the same interference: `clear_cache_removes_every_artifact_`
`and_only_those` failed once in-suite (its bystander artifact deleted externally
mid-run) while passing solo and in the final full run.

## Rule

- Run `cargo test` with `TMPDIR` redirected OUTSIDE `/tmp` (delegate-spool
  scratch is durable and unswept), and wrap long suites in `timeout` as a
  belt-and-braces backstop.
- Proof pair from the incident: the same test under stock `/tmp` → `timeout`
  rc124 after 600s; under `TMPDIR=<spool>` → 1 passed in **0.32s**, and the
  full core lib suite 235/0 in 13.31s. A code regression hangs under BOTH
  roots; an environment artifact hangs only under the swept one — always run
  that A/B before suspecting the delta.

## Fix lead (next maintenance cycle)

Harden the fixture itself: give Phase 1 a deadline, aggregate writer-child exit
statuses (a dead writer should fail the test in seconds with the child's panic
output, not wedge the suite), and consider pinning the fixture root under a
parent the test creates under an explicitly unswept base.

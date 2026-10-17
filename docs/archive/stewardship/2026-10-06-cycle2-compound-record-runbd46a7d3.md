# Cycle-2 compound record — run bd46a7d3 (repository-maintenance 853a4f32, cycle 2)

Date: 2026-10-06 · compound attempt 24ba0106 · base HEAD 8991144 (== origin/master)
Status: pre-review. Review/shipping outcomes happen after this phase; the next
cycle's assessment carries them forward.

## Batch

rm-596 SOLO — "Gate the doctor demo lane onto the demo corpus the sqlite lane
already uses" (prioritize d0cf65b5 single-item selection; stewardship 38f68307
described the change-unit and chose no Git topology).

- Assess bca8 F1 (live PoC at 8991144): `agenttrace -f json --doctor --demo`
  reported `mode: "demo sessions"` while the file discovery lane walked the
  operator's REAL corpus — 17,243 sessions enumerated, real project paths
  disclosed in `project_decode.samples`, 118.68s wall. The sqlite lane
  (doctor.rs:115-119) already gated on `dir.is_none() && !demo`.
- Implemented (attempt a808f785, uncommitted worktree delta): doctor.rs
  +80/-14 — `struct DoctorDiscovery` + `fn doctor_discovery(dir, demo)` as the
  ONE shared demo gate; under demo the file lane walks nothing, the session
  lane carries the bundled `demo_sessions()` corpus (disclosure counters
  folded identically), the session-cache lane reports path-only zeroed counts
  without reading the file, and `doctor_directories`' new demo arm enumerates
  NO real root (`known_session_dirs()`/`doctor_sqlite_directories()` probe the
  operator's home even when files is empty — a second leak channel closed).
  Two NEW hermetic test files (core `tests/doctor_demo_contract.rs`, cli
  `tests/doctor_demo_contract.rs`) each with control arms proving the
  non-demo lane unchanged. demo_contract.rs / discovery_contract.rs were
  avoided as test homes (dirty in in-flight siblings).
- Recorded validation outcomes (consumed at compound; NOT re-run here):
  - targeted a6d47a2a: core-focused 105/0 (demo_contract 7 /
    discovery_contract 81 / doctor_demo_contract 2 / pi_usage_tree_accounting
    15) + cli 1/0; fmt --check rc0; clippy -D warnings rc0.
  - full a9148065: `cargo test --workspace --release` 466 passed / 0 failed
    across 23 all-ok result lines (assess baseline 463/0 + exactly this
    batch's 3 tests); fmt rc0; clippy --workspace --all-targets -D warnings
    rc0; check-docs-commands.sh rc0.
  - PoC replay on the real home: 118.68s -> 0.31s wall; sessions 17,243 -> 3;
    `directories: []`; zero corpus-root strings in the raw JSON;
    project_decode empty; cache zeroed.
- Roadmap: rm-596 flipped candidate -> implemented with its EXECUTED bullet
  (done-flip reserved to the commit gate, rm-012); dated compound banner added
  to the notes stack.

## Tree (compound end)

```
 M ROADMAP.md                      # +14 mint carry + compound delta (17 ins total)
 M crates/agenttrace-core/src/doctor.rs
?? crates/agenttrace-cli/tests/doctor_demo_contract.rs
?? crates/agenttrace-core/tests/doctor_demo_contract.rs
?? docs/stewardship/2026-10-06-cycle2-compound-record-runbd46a7d3.md   (this file)
?? docs/solutions/workflow-issues/hermetic-test-markers-must-live-inside-the-planted-home.md
```

## Dead-attempt forensics (transport deaths, zero durable work, both redone)

- research attempt 7df0a15a: 429 provider failure at +87.6s; results/ status
  failed; no typed result; phase redone from scratch (dossier 819facc9).
- stewardship attempt b125548f: provider reap at +24s; 4-event log; no typed
  result; no scratch anywhere; phase redone from scratch (attempt 38f68307).

## Prevention doc minted (no roadmap id)

`docs/solutions/workflow-issues/hermetic-test-markers-must-live-inside-the-planted-home.md`
— hermetic secrecy-marker placement rule; distilled from the implement
phase's only test failure (self-inflicted marker leak via machinery paths).

## Id-space census at compound (zero mints; recorded for the next cycle)

Live def-row sweep 2026-10-06: fleet ceiling rm-625 @ run-9873fc06's wall,
rm-619 @ run-91833f02, rm-600 @ run-17319815, earlier high-water rm-593 @
run-614624d7; spool roadmap-delta patches carry ids to rm-598; canonical
/work/projects/agenttrace still max rm-402 @ ea5c41e. Next free rm-626+ AFTER
a fresh live claim census — the cycle-2 roadmap banner's "next free rm-597"
line is superseded by fleet movement (recorded per the dated-annotation
convention).

## Micro-observation recorded, not minted

`doctor_report_text` prints `Session files: {files.len() + sessions.len()}`,
so under `--demo` the 3 bundled in-memory sessions print as "Session files: 3"
while the JSON's `session_files` is 0. Pre-existing label semantics (the
sqlite lane had the same class before rm-596). LOW; title-check across
sibling walls that discuss text renderers on other subjects would be murky;
left for a future cycle's census.

## Next-cycle context

- Pass-over list unchanged: rm-367 (designated doctor-walk lead; independent
  seam from rm-596's :110-119 gate; its cached-walk value proposition is now
  STRONGER because the demo lane no longer pays the uncapped walk), rm-14
  (clean TUI dual-renderer lane), rm-195 (parked on the naming window).
- Assess F2-F5 remain folded to unlanded sibling rows — merge by TITLE at
  their landings, never renumber: F2 -> d65f72c7 rm-583 title-twin family;
  F3 -> 614624d7 rm-593 (incl. the statusline.jsonl bogus-capture WRITE arm);
  F4 -> fb1addd5 rm-584; F5 -> d65f72c7 rm-590 + dated rm-249 append.

## Commit-gate seams

CHANGELOG Fixed entry (demo doctor no longer walks/discloses the operator
corpus; 118.68s -> 0.31s, 17,243 -> 3) + rm-596 done-flip + the compound
banner + both docs files ride the SAME single patch as the +14 mint carry
(rm-012 one-patch rule). Integration note for the resolver:
`doctor_directories`' signature gains `demo` and its param renames
`sqlite_sessions` -> `sessions` — reconcile textually against sibling arms in
the same function (e602bb69 rm-079 pricing arm :129-138 disjoint; 2d92ee95
cache-report arms :40/:94/:162/:570/:666/:687; 614624d7 user_cache_dir
:675-683).

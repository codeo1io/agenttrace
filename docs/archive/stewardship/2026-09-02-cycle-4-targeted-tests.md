# Cycle 4 Targeted-Tests Record — changed surfaces of CU-6..CU-10

- **Run**: `2a15625945fc40419fc4691c59b42a7b`
- **Phase**: targeted_tests (`targeted_tests:targeted_tests`)
- **Attempt**: `212340db873645aca7f0e51957119500`
- **Date**: 2026-09-02
- **Tree under test**: working tree at HEAD `6632014` with the implement phase's uncommitted cycle-4 changes (15 modified files + `testdata/generated/adversarial/generic-loss.jsonl`), as enumerated in `2026-09-02-cycle-4-implementation-record.md`. No files changed in this phase; no failures to fix.

## Named new/modified tests (red→green pins for this batch)

`cargo test -p agenttrace-core --lib -- <filters>` — 8/8 ok:

```
test tests::event_typed_and_string_usage_coerce_instead_of_dropping_the_event ... ok   (CU-6)
test tests::parse_jsonl_session_recovers_recoverable_lines_and_counts_losses ... ok    (CU-6)
test parser::tests::bom_is_stripped_once_at_offset_zero_and_nowhere_else ... ok        (CU-7)
test parser::tests::utf16_files_fail_with_a_named_encoding_error ... ok                (CU-7)
test parser::tests::escaped_backslash_pairs_never_mask_surrogate_repair ... ok         (CU-10)
test session_cache::tests::orphaned_temp_siblings_are_swept_on_cache_load ... ok       (CU-9)
test history::tests::torn_history_file_is_quarantined_not_silently_discarded ... ok    (CU-9)
test session_cache::tests::sqlite_snapshot_schema_six_round_trips_provenance_and_rejects_older_schemas ... ok  (CU-10, modified)
```

Plus the two modified integration tests, both green inside their suites below:
`discovery_contract::generic_fallback_recovers_recoverable_lines_and_reports_the_rest` (CU-6, fixture-driven) and
`entrypoints::baseline_regression_gates_the_exit_code_and_opt_out_flags_work` (CU-8, exit 2 + `--no-baseline-gate` exit 0) and
`demo_contract::demo_baseline_comparison_is_stable_for_identical_report` (CU-8, new `BaselineBreaches` return).

## Targeted suites for every touched crate/test target

| Command | Result |
|---------|--------|
| `cargo test -p agenttrace-core --lib` | ok. **64 passed**, 0 failed |
| `cargo test -p agenttrace-core --test discovery_contract` | ok. **61 passed**, 0 failed |
| `cargo test -p agenttrace-core --test demo_contract` | ok. **7 passed**, 0 failed |
| `cargo test -p agenttrace --test entrypoints` | ok. **3 passed**, 0 failed |
| `cargo test -p agenttrace --test launch_guards` | ok. **2 passed**, 0 failed |
| `cargo test -p agenttrace-tui --lib` (drive-by presentation fix) | ok. **40 passed**, 0 failed |

(`agenttrace` has no `--lib` target — bin crate; unchanged from before this batch.)

## Static checks

| Check | Result |
|-------|--------|
| `cargo fmt --all --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean (Finished, zero warnings) |
| `cmp target/release/agenttrace /tmp/agenttrace` | identical — release binary current with sources |

## Surface-specific CI check scripts (release binary via `AGENTTRACE_BIN=/tmp/agenttrace`)

| Script | Why targeted | Result |
|--------|--------------|--------|
| `scripts/ci/check-report-semantics.sh` | overview text/MD/HTML/JSON surfaces changed (`Dropped lines` row) | PASS |
| `scripts/ci/check-output-contract.sh` | report shape contract | PASS |
| `scripts/ci/check-deterministic-output.sh` | additive fields must not break byte-determinism | PASS |
| `scripts/ci/check-docs-commands.sh` | `docs/guides/ci-integration.md` edited (exit-2 semantics) | PASS |
| `scripts/ci/check-plugin-version.sh` | CHANGELOG edited | PASS |

## Verdict

Every changed surface is covered by a focused, passing test; all targeted suites, fmt, clippy `-D warnings`, and the five surface-specific check scripts are green. No failures encountered, nothing to fix, no files modified in this phase.

---
ce-handoff: v1
cwd: /home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-a1cafb4c1a22-a1cafb4c
repository: /work/projects/agenttrace
repo_root_sha: 5ef66c045f53e38b812bac94309207cb6fa31e5f
branch: conductor/run-a1cafb4c1a22
head: 5ef66c045f53e38b812bac94309207cb6fa31e5f
---

# Cycle 3 full-suite validation record

- **Date**: 2026-10-03 (campaign `repository-maintenance:f60d521c`, cycle 3, run `a1cafb4c1a22`)
- **State**: pre-review. Recorded at full_tests time (attempt c9abad84, first
  attempt); nothing re-run afterwards.

## Command authority

The work order's validation block carried `required_scope: full` with an empty
`full_command`, so the authoritative suite was derived from the repository's
own CI (`.github/workflows/ci.yml`): its test legs target
`-p agenttrace-core -p agenttrace-tui -p agenttrace`, which equals the full
workspace per `Cargo.toml` members (the CLI crate's package name is
`agenttrace`). CI additionally gates fmt, clippy `-D warnings`, a `--locked`
release build, and the entrypoints integration test — all mirrored below.

## Envelope (all rc0)

| Command | Result |
|---|---|
| `TMPDIR=/tmp cargo test --workspace` | 284 passed / 0 failed (cli unit 25, entrypoints 10, launch_guards 2, upstream 9; core unit 111, demo_contract 7, discovery_contract 72, pi_family_discovery 2; tui unit 46; doc-tests 0) |
| `cargo fmt --all --check` | rc0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | rc0 |
| `cargo build --release --locked -p agenttrace` | rc0 |
| `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace` | rc0, 11 suites all 0 failed |
| `cargo test --locked -p agenttrace --test entrypoints` | rc0 |

Validation was run with the implement-phase 4-file delta in place
(`README.md`, `ROADMAP.md`, `crates/agenttrace-cli/src/main.rs`,
`crates/agenttrace-cli/tests/entrypoints.rs`, uncommitted per phase policy).
No regressions; no fixes needed; no executable surface changed during the
phase.

## Release-binary regression smoke

- `--overview --fail-under-health 100 /tmp/at-assess-0bc54992/env/corpus3/bad.jsonl`
  → rc2 `Gate failed: average health 40.0 is below 100` (rm-246 holds).
- `<bad.jsonl> -o <out>` → rc2 dropped-flag error naming `-o`, output file
  absent (rm-247 holds).

## Digest

`validation:v1:ed7fc27031119b7016a55b56a385c882d068a24f803eff24a9e467a84d4a0e90`
— re-derived live after validation via
`hermes_conductor.validation_policy.validation_digest('5ef66c0…','.')`,
byte-identical to the dispatch-time digest. This run's newest passing
full-suite baseline for the fold.

Logs: `delegate/c9abad849a8c4adb98148240b414338e-scratch/`
(full-test.log, locked-test.log, fmt.log, clippy.log, release-build.log,
poc-smoke.log, full-validation-note.md).

# Empty dispatch full_command → mirror ci.yml lane-for-lane (PR-T)

**Rule class:** workflow issue · **First proven:** run 84be17b33 full_tests attempt d89c8c2b, 2026-10-07 (after ff0068ca/53c9af4d and 7eae74ea used the same recipe) · **Repo:** agenttrace

## The failure mode

Conductor's full-validation work orders can arrive with `validation.full_command: ""` (empty) while
`required_scope: "full"`. A delegate that treats the empty string as "no authoritative suite exists"
either (a) invents a narrower local command and under-validates, or (b) stalls on a question. Neither
is correct: the repository still has an authoritative full suite — the CI workflow.

## The rule

1. **ci.yml is the authority.** When `full_command` is empty, the authoritative suite is
   `.github/workflows/ci.yml`, mirrored **lane-for-lane** across the lint, full, and deny jobs.
   The engine itself skips the command-match clause when `expected_full` is empty
   (`validation_policy.py` `if expected_full:`), so the fold accepts the mirrored command — record it
   verbatim in `validation_evidence.command`.
2. **Mirror, don't approximate.** Run every step's command string as written (including
   `ruby -c`, `npm --prefix npm test`, the `scripts/ci/*` gates, `bash -n`/`sh -n` syntax lanes).
   Duplicates that appear in both jobs (fmt, clippy, check-locked-cargo) run once, first-occurrence
   order. Lanes gated on repository *variables* (e.g. the TUI real-smoke lane behind
   `vars.AGENTTRACE_TUI_REAL_DIR`) are skipped locally exactly as CI itself would skip them —
   recorded as skipped, not failed.
3. **Redirect every CI-out path out of the worktree.** `AGENTTRACE_CI_OUT` / `AGENTTRACE_REAL_CLI_OUT`
   honor absolute paths — point them at a /tmp dir so the mirror leaves the worktree porcelain
   byte-identical to the implement payload. A full-suite run that pollutes `ci-artifacts/` into the
   worktree corrupts the commit-gate census.
4. **Gate the heavy lanes.** Cargo compile/test lanes and binary smoke scripts go through
   `/work/projects/hermes-conductor/scripts/local_validation_gate.py --shell-command '<lane>'`
   (`--help` for flags) — the fleet host is routinely oversubscribed (this run: load1 31.5 on 14
   cpus); the gate bounds admission so lanes don't thrash.
5. **Local tool quirks are lane-equivalence, not substitution:** `cargo deny --all-features check`
   (local cargo-deny 0.20.2 requires the feature flag *before* the subcommand) is the equivalent of
   the CI deny job's `check --all-features`.

## Evidence (run 84be17b33, base 8991144, uncommitted 6-file delta in place)

22 lanes executed, **all rc=0**, 9m11s wall at load1 31.5: fmt; clippy ci-lane; check-locked-cargo;
check-install-ref-drift; `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace`
(+ entrypoints rerun) = 498 passed / 0 failed across 22 suites; release build; entrypoints;
check-output-contract (216s); check-deterministic-output; check-report-semantics;
check-release-surfaces; check-example-workflows; check-install-runtime; check-docs-commands;
check-rust-real-cli-smoke (119s); `ruby -c` homebrew formula; `npm --prefix npm test`;
check-cargo-manifests; check-plugin-version; `bash -n`/`sh -n` helper syntax; deny; supplementary
`clippy --workspace --all-targets`. Post-run porcelain = exactly the inherited 6 M files (zero
untracked additions) — the /tmp redirect held. Lane log: `/tmp/at-full-d89c/` (sweep-volatile;
the result envelope is the durable record).

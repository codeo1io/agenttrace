# Prevention rule: prove TMPDIR is outside any git repository before trusting a battery

- **Class:** workflow / validation batteries (cargo test suites, any tool that roots fixtures under `std::env::temp_dir()`)
- **Observed:** 2026-10-14..15, run `7ee18a0dbb6c4dae9ec02a53c373e7ee` (repository-maintenance `d50a9f62a6174a9dbbf73e310c799c67` cycle 3) — reproduced at assess (wall 3e90993), re-proved at targeted_tests (batch tree), and pinned at full_tests; same class documented on rm-241 since 2026-10-13.
- **Cost:** three test lanes fail environmentally under the conductor's default TMPDIR, masquerading as code regressions: a green-feature battery reds out, or worse — a battery run under a polluted TMPDIR can mask a real failure inside noise if the operator "fixes" tests instead of the environment.

## What happened

The conductor dispatches delegates with
`TMPDIR=/home/agent/.hermes/tmp/delegate`, which sits **inside the
`/home/agent/.hermes` git repository**. Three lanes resolve "the current git
repo" while walking up from their temp-dir-rooted fixtures and latch onto
`.hermes` instead of the fixture's own root:

1. `agenttrace --test upstream` `outside_a_git_repository_fails` — roots its
   no-git fixture under `temp_dir()`, finds `.hermes` above it, and fails
   (8 empty `agenttrace-upstream-nogit-*` dirs left behind in the polluted
   TMPDIR).
2. `insights::tests::decode_ignores_shadow_directories_at_plausible_ancestors`
   — project-dir resolve latches `.hermes` over the fixture `my-repo`.
3. `insights::tests::decode_prefers_the_longest_verified_path_over_a_decoy_split`
   — same ancestry walk.

A/B proof on the identical tree (targeted_tests, this run): default TMPDIR →
`insights` 1 ok / 2 FAILED; `TMPDIR=/tmp` → 3/3 ok. Not a code regression —
an environment-induced failure that follows the battery wherever the
conductor runs it.

The full-suite phase closed the loop with a **proof pin**: before the
battery, `git -C "$TMPDIR" rev-parse --show-toplevel` must FAIL with rc 128
("not a git repository"). It did (TMPDIR pinned to a fresh
`/home/agent/.cache/at-full-<attempt>-tmp`), the battery ran, and all three
environmental canaries passed — including the two `insights` decode tests
that had failed under the inherited TMPDIR earlier the same run.

## Prevention rules

1. **Pin TMPDIR yourself; never inherit it.** Every execution phase sets
   `TMPDIR` to a directory it created, e.g. `/tmp` for short batteries or a
   private `/home/agent/.cache/at-<phase>-<attempt>-tmp` for full suites.
2. **Prove the pin, do not assume it:** run
   `git -C "$TMPDIR" rev-parse --show-toplevel` and require rc 128 before the
   first test command. Record the command + rc in the phase evidence. This is
   a one-line check that converts "TMPDIR looked fine" into proof.
3. **A/B the suspect class when it appears:** run the failing filter once
   under the inherited TMPDIR and once under the pin, same tree, and file both
   logs. The pair is what proves "environmental, not regression" — a single
   green run proves only that one directory worked.
4. **Treat repo-ancestry-sensitive tests as a named hazard class** when adding
   fixtures: anything rooted under `temp_dir()` that asserts about git
   boundaries or path resolution inherits whatever repository encloses the
   caller's TMPDIR. Fixtures needing no-git semantics should assert the
   absence, not assume it (rm-241's widening acceptance: fixture-owned
   boundary via `GIT_CEILING_DIRECTORIES` or an injected base path).
5. **Do not "fix" the tests to tolerate the pollution** — the upstream
   corroboration (codex #53897, daemon tests isolated for disk contention)
   fixed isolation, not assertions. Weakening the assertion hides the exact
   mis-resolution the tests exist to catch.

## Related

- ROADMAP row rm-241 (fixture hermeticity) carries the full class history and
  the widened acceptance; assess re-proof + codex corroboration in its
  2026-10-14 rider.
- The battery this rule guarded to green: rm-1426 batch, targeted 430/0 and
  full 1119/0/0 — `docs/decisions/2026-10-16-cycle-3-batch-selection-run-7ee18a0d.md`.
- Staleness sibling (same batteries, different trap): touch-all-src before a
  full run when reusing a shared `CARGO_TARGET_DIR` — a green result must
  come from a binary newer than your last edit.

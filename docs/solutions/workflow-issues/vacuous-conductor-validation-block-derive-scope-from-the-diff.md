# Prevention rule: vacuous conductor validation blocks — derive scope from the diff and the suite from ci.yml

- **Class:** workflow / validation-scope derivation across every agenttrace campaign
- **Observed:** 2026-10-06, run `fb1addd5` cycle 1 (repository-maintenance `3282bd9d`), targeted_tests `f6c8fd9c` and full_tests `7bec458d` at base `8991144`; the same shape recurs on agenttrace runs generally (fleet ledger: hermes-conductor's surface classifier matches only root prefixes, and this repo is `crates/**`; the `validation_evidence` schema gap is conductor fix-queue material)
- **Cost:** the dispatch-time validation block arrives empty (`changed_testable_surfaces=[]`, `required_scope 'none'`, `full_command ''`) while the tree demonstrably changed executable surfaces — a delegate that trusts the block under-validates a real 12-file delta; a delegate that stalls on the contradiction burns the turn asking instead of deriving; and two phases independently re-derive the same recovery each run.

## What happened

The implement phase's PhaseResult predates the `validation_evidence` schema (no structured `changed_surfaces` were emitted), and even with structured surfaces the engine's `classify_surface` matches only root-level prefixes (`src/`, `tests/`, …) — this repository lays its crates under `crates/agenttrace-core`, `crates/agenttrace-cli`, `crates/agenttrace-tui`, so every code change classifies as nothing. The dispatch-time validation block for this run therefore read `changed_testable_surfaces=[] / required_scope 'none'` and `full_command ''` against a worktree that had just changed 12 files (+541/−95), including parser semantics and a session-cache schema bump.

Both validation phases recovered honestly and their recovery is the rule to copy:

- **targeted (`f6c8fd9c`):** declared the derivation gap openly, took the authoritative scope from `git diff --stat` (12 files → the five owning suites: core lib, discovery_contract, cli bin, cli upstream integration, tui lib — 373/0), and — because that turn changed no executable surface itself — copied the dispatch-time digest VERBATIM instead of inventing a re-derivation.
- **full (`7bec458d`):** with `full_command=''` but `required_scope='full'`, took the authoritative suite from the repository's own definition — the CI `full` job (`.github/workflows/ci.yml` :105, "Test and build") — and ran every locally-runnable step with its CI-exact invocation: 19 steps all rc0 (workspace `--locked` 472/0, entrypoints 31/0, release build, the eight `check-*.sh` gates, ruby/npm/manifests/plugin-version/syntax/locked-cargo) plus the deny lane.

## Prevention rules

1. **Treat a vacuous block as a derivation gap, never as "nothing to validate."** The discriminator is one command: if `git diff --stat` (or the porcelain census) is non-empty against the phase's base, executable surfaces changed and validation scope exists — declare the gap and the diff-derived scope in the PhaseResult instead of echoing `required_scope: none`.
2. **The repository's CI workflow is the suite authority when `full_command` is empty.** Run the CI job's steps with their exact invocations (lockfile flags, env gates, `AGENTTRACE_BIN`, output redirections out of the repo) rather than assembling an ad-hoc equivalent — an ad-hoc suite silently drifts from what actually gates landings.
3. **Respect the CI's own skip conditions.** A step gated on an unset repo variable (here `check-rust-tui-real-smoke.sh` on `AGENTTRACE_TUI_REAL_DIR`) is skipped locally exactly as CI skips it — record the skip and its gate, do not force it.
4. **Digest discipline when nothing executable moved:** copy the dispatch-time `validation:v1:…` token VERBATIM. No in-repo digest tooling exists (`grep -r 'validation:v1' scripts/` is empty), and hand-rolled re-hashes produce a token nothing downstream can reproduce.
5. **Fix it upstream where possible:** emit structured `validation_evidence.changed_surfaces` in implement-phase PhaseResults (conductor fix-queue item) so the engine's derivation has something to derive from; until then, rules 1–4 stand.

## Verification

- The two envelopes record the derivation gap and the recovery verbatim: `delegate/f6c8fd9c….json` ("changed_testable_surfaces=[] … the honest scope for this phase is TARGETED over exactly those surfaces — declared here with the full derivation") and `delegate/7bec458d….json` ("the authoritative full suite was taken from the repository's own definition — the CI 'full' job").
- The gap reproduces structurally: this repo has no top-level `src/` (all code under `crates/*/src`), so a root-prefix classifier yields the empty set for any code delta; `git diff --stat` at the implement end-state was 12 files / +541 / −95.
- Logs persist at `/tmp/at-full-7bec/` (19 gate logs + `cargo-deny.log`) and `/tmp/at-targeted-f6c8/targeted.log` (2026-10-07 check).

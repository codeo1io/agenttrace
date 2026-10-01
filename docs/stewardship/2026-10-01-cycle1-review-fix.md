# Cycle 1 — Independent Review Fix + Full-Scope Retest (2026-10-01)

Run `792ef47bdeaf42a0a76d02ac170473c4`, campaign `1f5ad3cf`, phase `independent_review`
(action `independent_review:fix`), worktree `run-792ef47bdeaf-792ef47b` at HEAD
`9d88b36750a991bd1436dbbc91b4579c39003067`, branch `conductor/run-792ef47bdeaf`.

## Verdict

CLEAN — all 6 findings of the NEEDS_CHANGES review (verdict `b3632470`) are fixed in the
working tree, and the fixed tree now has a **full-scope** passing validation record.

## Why this record exists (fold-gate remediation)

The prior fix attempt (`b7a525aa`) fixed all 6 findings but returned retest evidence at
`targeted` scope only, while its delta includes an executable deletion-path change
(`session_cache.rs` clear set + failure aggregation). Conductor's fold gate rejected it:
risk escalation over the run's uncovered risk-relevant delta requires scope `full`
(KTD5/KTD6) — the run's newest full-suite record (`full_tests`, 268/0) predated the fix
delta. This attempt re-ran the work order's `full_command` verbatim over the fixed tree,
making the full-suite record current.

## Findings fixed (all 6, verified in-tree this attempt)

| ID  | Where (current, fixed) | Fix |
| --- | --- | --- |
| M1  | `crates/agenttrace-tui/src/shared.rs:241-242`, `crates/agenttrace-tui/src/presentation.rs:3611-3615` | Label arms added in BOTH copies of `display_source_label`: `pi_senpi`/`pi_omo` → `Pi (senpi) sessions`/`Pi (omo) sessions`; `shared.rs:360-368` test asserts both copies via `crate::app::presentation::` |
| M2  | `crates/agenttrace-core/tests/pi_family_discovery.rs:246-257`, `crates/agenttrace-core/tests/discovery_contract.rs:2233-2244` | `XDG_DATA_HOME` pinned inside both test helpers (`with_home`, `with_home_and_cache`) — `discovery.rs:807` reads it and the host export leaked a real `agent.db` into discovery assertions |
| M3  | `crates/agenttrace-core/src/session_cache.rs:251` (`legacy_cache_artifact_paths`), composed at `:238` | `--clear-cache` now sweeps prefix-pinned `hermes-sqlite-v*.json` / `opencode-sqlite-v*.json` orphans (~60 MB stranded on this host; no live code names them) |
| L6  | `crates/agenttrace-core/src/session_cache.rs:275-296` (`remove_cache_artifacts`) | Clear-all-then-aggregate-failures: a first removal failure no longer strands the artifacts after it (partial clear was the worst outcome for a privacy-motivated purge) |
| L5  | `PRIVACY.md:20` | Names the bundled-snapshot fallback after `--clear-cache` removes `pricing.json` (no forced network) |
| L4  | `ROADMAP.md:335` (rm-084 bullet), `docs/stewardship/2026-10-01-cycle1-implementation-record.md:37`, `crates/agenttrace-core/src/statusline.rs:106` | False "`statusline ~/.pi` comment corrected" claims removed; `rm-86` → `rm-086` |

Near-miss trap recorded during the fix: `pricing_cache_path()` resolves via
`XDG_CACHE_HOME`/`user_cache_dir`, NOT `AGENTTRACE_SESSION_CACHE_DIR` — the clear-cache
e2e must pin BOTH env vars or it deletes the host's real `pricing.json`.

## Retest evidence (this attempt)

- Full command, VERBATIM, from the worktree root:
  `python3 /home/agent/.hermes/releases/hermes-conductor/4ad38e1a3112a93893f1162918693b6992ff7f22/scripts/local_validation_gate.py --shell-command 'cargo test'`
- Env prep (argv untouched): `env -u XDG_DATA_HOME HOME=/tmp/at-full-home RUSTUP_HOME=/home/agent/.rustup CARGO_HOME=/home/agent/.cargo`
- **GATE_RC=0. Workspace 268 passed / 0 failed** across all 11 test binaries:
  cli 41 (main 20 + entrypoints 10 + launch_guards 2 + upstream 9), core 182 (lib 101 +
  demo_contract 7 + discovery_contract 72 + pi_family_discovery 2), tui 45, doc-tests 0+0.
  Identical shape to the pre-fix `full_tests` record — the fix delta changed zero test
  counts and introduced zero regressions.
- Envelope: `result-2006346-330835043.json` (returncode 0, started 1790812095.8,
  finished 1790813017.9, ~922 s; envelope-internal digest `validation:v1:e9ca4b3a…`
  with `digest_base "unknown"` is not the declared digest, matching the accepted
  full_tests adjudication). Copy + full transcript:
  `/home/agent/.hermes/conductor-delegate-spool/review-fix-2f43bbde85c54088aaee3accff2d92d6/`.
- Declared digest re-derived after the run via
  `hermes_conductor.validation_policy.validation_digest('9d88b36…', '.')` →
  `validation:v1:155d1e3d82ccd7bc7369ace33562cface32567490ee3cde875121137d0a6c738`
  (identical to dispatch-time: HEAD is unchanged and the batch is uncommitted).
- Transcript greps: `FAILED|failed[^;:]|error[|warning:` → 0 hits beyond the per-binary
  `0 failed` tallies.

## State at exit

`git status --porcelain`: 13 M (PRIVACY.md, README.md, ROADMAP.md, main.rs, discovery.rs,
parser.rs, reports.rs, session_cache.rs, statusline.rs, discovery_contract.rs,
presentation.rs, shared.rs, test-flake-prevention.md) + 6 ?? (pi_family_discovery.rs +
this file + 4 prior cycle docs). Stash empty. HEAD unchanged. No tracked file touched by
the retest itself. risk_class medium (executable deletion-path change in the uncommitted
delta) now backed by a current full-suite pass over the exact tree.

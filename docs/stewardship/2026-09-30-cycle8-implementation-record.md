# Cycle 8 implementation record — truthful posture, enforced gates

- **Date**: 2026-09-30
- **Cycle**: 8 (repository-maintenance campaign, campaign cycle 1; run 30484632)
- **Base**: `7bb4dcb` ("the PR #14 merge tree; first assessed tree past the supply-chain wave")
- **Worktree**: campaign worktree `run-304846327112-30484632` at base `7bb4dcb`
- **Batch selection**: delegate spool `batch-selection-2026-09-30-run30484632-….md`
- **Stewardship request**: delegate spool `stewardship-request-2026-09-30-run30484632-….md`
- **Status**: implemented; uncommitted worktree state (commit/push/PR deliberately out of scope; roadmap status flips are owned by the commit gate)

## What shipped

| # | Item | Lineage | State |
|---|------|---------|-------|
| 1 | rm-026 truth-correct the shipped private-remote claims + OpenSSF Scorecard lane | assess AF-1 (three false locations, live `gh` evidence) | done, live-verified |
| 2 | rm-027 WinGet submission runner pinned with digest verification | assess AF-2 + upstream PR #292, ported verbatim | done, live-verified |
| 3 | rm-028 `--locked` on all dependency-resolving cargo invocations + drift gate | assess AF-3 (zero `--locked` repo-wide) + research RC-12 | done, gate-proven both directions |
| 4 | rm-002 pytest suite for the adversarial-sqlite fixture generator | prior-cycle deferral (security-adjacent, mechanical) | done, mutation-proven |

## Item notes

### 1. Truth corrections + Scorecard (rm-026)

The three locations asserting "the fork's remote is private" were corrected by
dated append, not rewrite: `deny.toml:6` (header now records the verified
public visibility and the query that established it), the rm-010 roadmap note
at `ROADMAP.md:42` (cycle-1 deferral preserved, cycle-2 correction appended),
and the solutions document's Prevention bullet
(`docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md:55`).
The new `.github/workflows/scorecard.yml` runs the real OpenSSF analysis with
every action digest-pinned; tag-to-commit resolutions were verified live
against the GitHub API at implementation time. The cycle learning is
compounded as
`docs/solutions/process-issues/2026-09-30-unverified-assumption-compounded-across-governance-artifacts.md`.

### 2. WinGet pin (rm-027)

The `ghcr.io/microsoft/winget-create:latest` docker step — the only unpinned
runner in the token-bearing release path — was replaced by a `windows-latest`
job that downloads wingetcreate at a pinned version and verifies its SHA256
before any token is used, matching upstream PR #292 exactly. Upstream's
commit message states the ghcr image never existed, which means automated
WinGet submission had been silently broken, not merely unpinned; the port is
therefore a repair as well as a pin.

### 3. Lock enforcement (rm-028)

All seven dependency-resolving cargo invocations across `ci.yml` and
`release.yml` now carry `--locked` (`cargo fmt` is exempt: no dependency
resolution). The new `scripts/ci/check-locked-cargo.sh` fails with a
file:line offender list when any invocation regresses, and runs in both the
lint (PR) and full (master) lanes. One placement detail is load-bearing:
`--locked` sits at line-end on the release build line because
`scripts/ci/check-release-surfaces.sh:90` greps that exact substring.

### 4. Fixture-generator tests (rm-002)

`scripts/fixtures/test_make_adversarial_sqlite.py` pins the generator's
output contract with seven tests: emitted file set, both overflow/wrap
adversarial classes, schema and session-row shape, byte-determinism,
regenerated-equals-committed fixtures, and the `__main__` path. The suite
redirects OUT_DIR to a pytest tmp dir, so the committed fixtures are never
rewritten by a test run.

## Validation outcomes (recorded, not re-run in this phase)

- **Targeted** (mandated impacted-tests runner over the batch's five testable
  surfaces): derived `pytest -q -n 8 scripts/fixtures/test_make_adversarial_sqlite.py`
  → 7 passed, exit 0; no fallback to the cargo gate was needed (the batch
  changed no Rust code). Mutation check during implementation: perturbing the
  generator's epoch constant failed three tests, restored green.
- **Full suite** (mandated `local_validation_gate.py --shell-command 'cargo test'`):
  236 passed, 0 failed, 0 ignored across all workspace test binaries; gate
  returncode 0; result envelope
  `/home/agent/.hermes/local-validation-gate/results/result-1355235-325026419.json`.
- **Digest freshness proof** (recorded from the full-suite phase): the gate's
  envelope digest is keyed on `unknown` when `--digest-base-sha` is not
  passed, so it will not match a dispatch digest even on an unchanged tree;
  re-deriving with the dispatch base (`7bb4dcb`) reproduced the dispatch
  digest byte-identically. Future validation phases on this repo should
  re-derive with the dispatch base before declaring a digest stale.

## Next-cycle context

- **Deferred pool** (from the cycle-8 prioritization, with recorded
  rationale): rm-029 share-safe report mode is the next feature-sized lead,
  with rm-031 (portable redacted session bundle) riding it as a pair; rm-011
  keeps its anchor as a full session of golden-test scaffolding; rm-030 and
  rm-032 remain queued; rm-001 waits for the next upstream sync.
- **Namespace collision, resolved at the commit gate** (re-derived live
  2026-09-30, run 30484632 commit phase): this campaign minted rm-017..rm-023,
  but siblings landed/pushed while the cycle ran — origin/master carries
  rm-001..rm-019, open PR #17 rm-017..rm-024, open PR #18 rm-017..rm-025 and
  rm-033, all with different content under those numbers. The commit gate
  renumbered this batch to rm-026..rm-032 (mapping recorded in the ROADMAP's
  dated comment; spool artifacts citing rm-017..rm-023 mean the renumbered
  IDs). rm-002 is also implemented by PR #17 — content overlap to reconcile
  at integration. Any future roadmap edit must re-derive the live numbering
  before minting; integration merges concatenate rather than deduplicate.
- **First remote runs pending**: the Scorecard lane and the lock-drift gate
  have never executed on GitHub Actions; their first push-event exercise is
  the natural first check of the next cycle's CI gate, alongside the winget
  job's first real release-event run.
- **Review outcomes**: independent review and shipping happen after this
  record; the next cycle's assessment carries them forward, per the compound
  contract.

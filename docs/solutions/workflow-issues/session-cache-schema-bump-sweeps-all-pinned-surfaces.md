# Session-cache schema bumps must sweep ALL pinned surfaces (five sites, one gate-visible sentence)

**Added:** 2026-10-06, run 7f9c6d24 cycle 2 (schema 26 → 27 for rm-551..rm-556).
**Prevention class:** single-source constants pinned in multiple places.

## The trap

`SESSION_CACHE_SCHEMA_VERSION` is a single constant in `crates/agenttrace-core/src/session_cache.rs:8`, but its literal value is pinned in **five** places that all break differently when the constant bumps:

1. `crates/agenttrace-core/src/session_cache.rs:8` — the constant itself (the bump).
2. `crates/agenttrace-core/tests/discovery_contract.rs:990` — the stale-schema sweep pin.
3. `crates/agenttrace-core/tests/discovery_contract.rs:1162` — the cache-hit contract pin.
4. `crates/agenttrace-tui/src/tests.rs:1645` — the planted-cache fixture (planted at the OLD number, the test asserts regeneration).
5. `docs/guides/governance-reports.md` — a prose sentence carrying the number, which `scripts/ci/check-docs-commands.sh` verifies against the **live constant** by running the release binary.

Miss any one of the four dependent sites and the failure appears at a *later* gate than your edit: the discovery pins fail only in `cargo test -p agenttrace-core`, the tui fixture only in `cargo test -p agenttrace-tui`, and the governance sentence only in the CI docs gate against a release build — i.e. possibly after you already declared the phase green on focused tests.

## The rule

When a parser-semantics change bumps the schema (the rm-230 convention: cached sessions must regenerate under corrected totals):

1. Bump the constant in `session_cache.rs`.
2. In the same change, sweep all four dependent sites: both `discovery_contract.rs` pins, the `tests.rs` planted-cache fixture number, and the `docs/guides/governance-reports.md` sentence.
3. Verify the docs sentence against the **live** constant via the docs gate with a private `AGENTTRACE_CI_OUT` (never the shared default path — see the gate's environment notes), not by eyeballing the markdown.
4. Land every dated pin-note in the same commit as the bump — a site updated in a later commit leaves a window where the suite is red on `origin/master` for anyone between the two commits.

## Why five sites and not one

The pins are deliberate adversarial coverage: each one fails for a different regression (stale cache accepted, cache-hit regressed, planted cache trusted across a semantics change, docs drifting from code). The cost is this checklist — which is cheaper than any of the regressions it catches.

# Session-cache schema bumps must sweep ALL pinned surfaces (one const, two dynamic pins, two literal pins, one gate-visible sentence)

**Added:** 2026-10-06, run 7f9c6d24 cycle 2 (schema 26 → 27 for rm-551..rm-556, campaign-local numerals — the band rebounded as rm-601/rm-602/rm-603 at integration, where the bump re-based onto the advanced landed ceiling as 31 → 32, one invalidation either way per the rm-230 convention).
**Prevention class:** single-source constants pinned in multiple places.

## The trap

`SESSION_CACHE_SCHEMA_VERSION` is a single constant in `crates/agenttrace-core/src/session_cache.rs:8`, but its value is depended on in **five** places that break differently when the constant bumps:

1. `crates/agenttrace-core/src/session_cache.rs:8` — the constant itself (the bump).
2. `crates/agenttrace-core/tests/discovery_contract.rs:1046` — inside `rust_writes_and_reuses_go_compatible_session_cache` (fn at `:999`): the cache-hit contract pin. Reads the **exported constant dynamically** (the rm-538 landing moved both discovery pins off literals), so it cannot drift numerically — it goes red when the write/read regeneration contract itself regresses.
3. `crates/agenttrace-core/tests/discovery_contract.rs:1219` — inside `rust_refreshes_cache_entries_from_old_schema_version` (fn at `:1179`): the stale-schema sweep pin. Also dynamic; its planted fixture (`:1193`) carries an ancient `schema_version` and the test asserts regeneration to the live constant, so it stays adversarial at every bump.
4. `crates/agenttrace-tui/src/tests.rs:1666` — the planted-cache fixture in `ctrl_r_force_reload_clears_session_cache_before_loading`: planted at the **current** number as a hand-written literal, and the test asserts a **warm hit** (`cache_hits == 1`). Bump the constant without sweeping this literal and the planted entry looks stale — the warm-hit assertions go red in `cargo test -p agenttrace-tui`.
5. `docs/guides/governance-reports.md:72` — a prose sentence carrying the number ("The session cache is schema 32 …"), which `scripts/ci/check-docs-commands.sh` verifies against the **live constant** by running the release binary.

Miss any one of the literal-bearing sites and the failure appears at a *later* gate than your edit: the tui fixture only in `cargo test -p agenttrace-tui`, and the governance sentence only in the CI docs gate against a release build — i.e. possibly after you already declared the phase green on focused tests. (The two discovery pins fail only on semantics regressions, not on the bump itself — which is exactly why the literal sites must be swept by hand.)

## The rule

When a parser-semantics change bumps the schema (the rm-230 convention: cached sessions must regenerate under corrected totals):

1. Bump the constant in `session_cache.rs`.
2. In the same change, sweep every dependent site: both `discovery_contract.rs` pins (confirm they still read the exported constant, not a reintroduced literal), the `tests.rs` planted-cache fixture number, and the `docs/guides/governance-reports.md` sentence.
3. Verify the docs sentence against the **live** constant via the docs gate with a private `AGENTTRACE_CI_OUT` (never the shared default path — see the gate's environment notes), not by eyeballing the markdown.
4. Land every dated pin-note in the same commit as the bump — a site updated in a later commit leaves a window where the suite is red on `origin/master` for anyone between the two commits.

## Why five sites and not one

The pins are deliberate adversarial coverage: each one fails for a different regression (stale cache accepted, cache-hit regressed, planted cache trusted across a semantics change, docs drifting from code). The cost is this checklist — which is cheaper than any of the regressions it catches. Cite the test **names**, not just line numbers, when recording a sweep: anchors drift at every integration, and this file was re-anchored once already for exactly that reason.

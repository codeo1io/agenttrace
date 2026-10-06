# Prevention rule: schema-version bumps require a planted-fixture sweep

- **Class:** workflow / cache-schema migrations (SESSION_CACHE_SCHEMA_VERSION and any serialized-shape version constants)
- **Observed:** 2026-10-06, run `cb38b958` (repository-maintenance `f89f5223` cycle 2) — implement attempt `1e8f1b7a`; caught only in re-dispatch attempt `6fbb2946`
- **Cost:** one fold-gate rejection cycle. The first implement pass bumped SESSION_CACHE_SCHEMA_VERSION 26→27 and validated every suite whose crate it touched — but missed a planted warm-cache fixture in a crate whose SOURCE it had not touched. The re-dispatch reproduced `ctrl_r_force_reload_clears_session_cache_before_loading` FAILED (cache_hits 0 instead of 1: the planted `schema_version:26` entry is stale against the new const, so the cache-miss path exercises instead of the warm-hit path).

## What happened

The version constant gates cache reuse: a stored entry is accepted only when it carries the current schema version. Tests that want a WARM hit therefore hand-plant an entry with a literal version number — and that literal is a pin on the constant, in a file the bump does not force you to open. In this instance the fixture lives in `crates/agenttrace-tui/src/tests.rs` (a crate whose source was untouched by the change-unit), and its own comment says it tracks the constant. The bump's focused battery ran core and CLI suites (the crates the diff touched) and stayed green; the stale fixture sat in an un-run suite until the fold re-dispatch ran it.

The dangerous property: **the failure hides in a crate you didn't diff**. Suite scoping by "what did I touch" is exactly wrong for version-constant bumps — the pin sites are consumers of the constant, not authors of the change.

## Prevention rules

1. **Treat a version-constant bump as a cross-crate change.** The sweep is a grep, not a diff: on every bump of a serialized-shape version constant, run `grep -rn '<CONST>' crates/ scripts/ docs/` AND a value-literal sweep for the OLD number (`grep -rn 'schema_version.:26' crates/` for the 26→27 bump, in quotes/backtick/json variants) BEFORE declaring the battery scoped.
2. **Every pin site found is a required test target.** A planted fixture pinning the old value must either be updated in the same change-unit (with its provenance comment extended) or consciously re-justified in the phase record — never left to be discovered by a later gate.
3. **Fixture comments that bind to constants are load-bearing.** `// this fixture tracks SESSION_CACHE_SCHEMA_VERSION` is a contract: the sweep grep for the constant name is the cheap way to honor it. Do not rely on a future reader noticing.
4. **Prefer runtime-derived pins over literals in new tests.** Where a fixture only needs "an entry that is current", construct it from the exported constant (the export added this cycle at the crate root enables exactly this) instead of a hardcoded integer; literals remain only where the test specifically needs a STALE entry.
5. **On any bump, run the warm/cold cache suites of ALL crates, not the changed ones.** The miss/warm distinction is invisible until those suites run; they are the behavioral probe for this change class (green-by-scoping proves nothing).

## Related

- `docs/stewardship/2026-10-06-cycle2-compound-record-runcb38b958.md` (this cycle's record; the fixture fix is in the attempt-2 addendum /tmp/at-impl-cb38/attempt2-addendum.md)
- ROADMAP rm-538 notes (the bump was part of the disclosure-channel split; schema 27 lineage: 27 = rm-538 disclosures field, 26 = rm-485 copilot credit accounting, 25 = rm-450 workbuddy basis disclosure, 24 = rm-436/437/438 pi journal accounting)
- Prior bump history in the TUI fixture's own provenance comment (`crates/agenttrace-tui/src/tests.rs`, ctrl_r warm-cache test)

# Untrusted rollup totals must saturate, never `+=`

**When**: an aggregation lane sums per-item values that originate outside the
program's control — sibling/child session totals, foreign-corpus usage counts,
per-file cost sums — anywhere downstream of parsing.

**Why**: `analyze`'s clamp-plus-saturate contract (crates/agenttrace-core/src/lib.rs,
~1540-1570) exists because parsed corpora are adversarial by construction. The
subagent-attribution lane re-derived its own rollups with plain `+=`
(subagents.rs, both the path-linked and key-linked arms) and escaped that
contract: two child sessions each carrying `i64::MAX/2 + 1` tokens panicked
every debug-profile run (`attempt to add with overflow`, rc101) — the debug
profile being the ordinary `cargo test` lane — and a release build silently
wrapped to a wrong total. Aggregation code written after the contract tends to
re-offend precisely because the contract lives in a different module.

**Fix**: integer/count rollups use `saturating_add`; poisoned (non-finite)
f64 contributions are skipped rather than accumulated (rm-1500 discipline, as
the insights rollups already do), so one NaN/Inf child cannot blank a parent's
cost forever. Pin both properties with tests that use genuinely hostile
values (`i64::MAX/2 + 1` twice; `f64::INFINITY` next to a finite sibling) plus
an end-to-end fixture corpus, so the debug profile itself is the guard.

**Evidence**: rm-487 (implemented 2026-10-16, run e775cac5 cycle 1; PoC
`attempt to add with overflow` at subagents.rs:230 pre-fix; post-fix tests
`hostile_child_totals_saturate_instead_of_panicking`,
`poisoned_child_cost_is_skipped_not_propagated`,
`hostile_overflow_corpus_saturates_parent_rollups` — 61/61 targeted, 1147/0
full suite). Search pattern for future audits: `+=` over any field fed from
parsed data outside lib.rs's own aggregation.

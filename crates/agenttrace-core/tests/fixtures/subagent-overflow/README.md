# subagent-overflow — hostile rollup corpus (rm-487)

Two Claude Code children (`p2-4242/subagents/agent-h{1,2}.jsonl`) each
carry a single assistant event with `input_tokens = 4611686018427387904`
(`i64::MAX / 2 + 1`), so the parent rollup sums to `i64::MAX + 1`. This
is the assess-PoC corpus class (run e775cac5 cycle 1, attempt 19607e2b):
before rm-487 the plain `+=` in `attribute_subagents` (subagents.rs)
panicked debug builds with `attempt to add with overflow` and wrapped
release totals negative. The end-to-end pin test lives in
`tests/subagent_attribution.rs::hostile_overflow_corpus_saturates_parent_rollups`
and asserts the parent saturates at `i64::MAX` instead. Journal shape
mirrors `tests/fixtures/subagent-corpus/`.

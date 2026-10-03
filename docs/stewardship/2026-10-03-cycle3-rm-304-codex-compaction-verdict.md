# rm-304 verdict — Codex unreported-compaction usage accounting

- **Item**: rm-304 (correctness, priority 64.0), minted run 75ae7fb6 roadmap
  attempt cfb79159 from research pass-13 candidate C75
- **Phase**: implement attempt f69fa3e1, 2026-10-03, tree fd5532f
- **Verdict**: **PROVEN** — Codex sessions that went through remote
  compaction are materially under-counted by agenttrace today.

## Mechanism

When Codex compacts a conversation remotely, the compaction turn's provider
usage is persisted as dedicated rollout records, not folded into the
cumulative `event_msg`/`token_count` snapshots:

1. `token_usage_record` rollout entries (one per response; codex-rs
   `protocol.rs` `TokenUsageRecord { response_id, usage, turn_token_usage,
   thread_token_usage }`) — agenttrace never reads this line type.
2. `compacted` entries carry `latest_token_usage_record` — codex-rs' own doc
   comment says token totals are RESTORED FROM THIS FIELD after compaction,
   i.e. the token_count path is not authoritative across it — and
   agenttrace's `codex_line_is_ignorable` drops `compacted` lines before
   parsing (parser.rs :2344-2352 at the time of writing).
3. Post-compaction `token_count` cumulatives restart from the smaller
   compacted history. The rm-162/#286 high-water guard correctly refuses
   them as rewinds (no double-count), but nothing ever adds the unreported
   compaction usage back.

Wire shapes in the fixture corpus were taken from upstream source, not
invented: `CompactedItem` (codex-rs `history/src/lib.rs` :286+, including
`latest_token_usage_record`), `TokenUsageRecord` (`protocol/src/protocol.rs`
:2264+), `TokenUsage` (:2241+). Mechanism anchors: ccusage PR #1821
(2026-10-02) and openai/codex `compact_remote_v2.rs` L461-480 /
`compact.rs` L808-L826.

## Measurement (reproducible)

```
TMPDIR=/tmp cargo test -p agenttrace-core --test codex_compaction_verdict -- --nocapture
# rm-304 verdict: reported 1200 of 3700 billable tokens (67.6% under-count)
```

Corpus `tests/fixtures/codex-compaction/remote-compaction.jsonl`: turn 1
(in 1000 / out 200, token_count), remote compaction turn (in 1500 /
cache 800 / out 300 / reasoning 120, only in `token_usage_record` +
`compacted.latest_token_usage_record`), post-compaction turn (in 600 /
out 100, present only as the rewound cumulative). True totals in
3100 / cache 800 / out 600 / reasoning 120; agenttrace reports in 1000 /
cache 0 / out 200 / reasoning 0. The control fixture proves the baseline
path still counts correctly, and the passing rewind-guard behavior proves
the miss is exactly the compaction turn plus the post-compaction window —
not a double-count regression.

## Recommended fix shape (next cycle)

Pair `token_usage_record` entries with `compacted` markers by response id
and count only usage no intervening cumulative snapshot already counted —
ccusage #1821's approach. Tests to land with it: serial and forked-parent
sequences (root_turn_id) plus a golden fixture on a real fleet rollout.
The characterization assertions in `codex_compaction_verdict.rs` flip from
the under-counted expectations to the true totals when the fix lands; that
inversion is the regression gate.

## Blast radius of the miss

Costs, waste/savings math, and any budget gate derived from Codex sessions
are computed from the under-counted totals; sessions with frequent remote
compactions (long agent sessions — the fleet's norm) drift the most.

//! rm-304 verify-first harness: Codex unreported-compaction usage accounting.
//!
//! Mechanism under test (ccusage PR #1821; codex-rs anchors
//! `compact_remote_v2.rs` L461-480 and `compact.rs` L808-826): when Codex
//! compacts a conversation REMOTELY, the provider usage of the compaction
//! turn is persisted as `token_usage_record` rollout entries (and as
//! `compacted.payload.latest_token_usage_record`), WITHOUT being added to
//! the cumulative `event_msg`/`token_count` snapshots that agenttrace
//! counts. Post-compaction `token_count` cumulatives restart from the
//! smaller compacted history, so they neither include that usage.
//!
//! agenttrace's parser reads only `event_msg`/`token_count` lines
//! (parser.rs `codex_line_is_ignorable` drops `compacted` lines up front;
//! `token_usage_record` lines fall through the type match unused), so the
//! prediction is an under-count exactly equal to the compaction turn's
//! usage. The tests below PROVE the prediction on the fixture corpus and
//! pin today's numbers: when the rm-304 fix lands (response-id pairing per
//! ccusage #1821), flip the `reported_*` expectations in the second test to
//! the `true_*` values — that inversion is the regression gate for the fix.

use agenttrace_core::parse_raw_session;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq, Eq)]
struct Usage {
    input: i64,
    cache_r: i64,
    output: i64,
    reasoning: i64,
}

impl Usage {
    fn total(&self) -> i64 {
        self.input + self.output
    }
}

fn parse_fixture(name: &str) -> Usage {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codex-compaction")
        .join(name);
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let session =
        parse_raw_session("codex", name, &raw).unwrap_or_else(|e| panic!("parse {}: {}", name, e));
    Usage {
        input: session.metrics.tokens_input,
        cache_r: session.metrics.tokens_cache_r,
        output: session.metrics.tokens_output,
        reasoning: session.metrics.tokens_reasoning,
    }
}

#[test]
fn control_cumulative_token_counts_are_counted() {
    // Baseline: a plain session with one token_count cumulative snapshot
    // is fully counted by the existing parser.
    assert_eq!(
        parse_fixture("control.jsonl"),
        Usage {
            input: 1000,
            cache_r: 0,
            output: 200,
            reasoning: 0,
        }
    );
}

#[test]
fn remote_compaction_usage_is_under_counted_today() {
    // rm-304 verdict, measured on remote-compaction.jsonl:
    //
    //   turn 1 (token_count)        in 1000 / out 200
    //   compaction turn (records)   in 1500 / cache 800 / out 300 / rea 120
    //   post-compaction (count)     in  600 / out 100
    //
    //   true totals                 in 3100 / cache 800 / out 600 / rea 120
    //   agenttrace reports          in 1000 / cache   0 / out 200 / rea   0
    //
    // The post-compaction snapshot (700 cumulative) sits below the
    // rm-162/#286 high-water mark (1200), so the rewind guard correctly
    // refuses it — no double count — but nothing anywhere reads the
    // token_usage_record / compacted payloads, so the entire compaction
    // turn (and the post-compaction window's own usage, which only exists
    // as the rewound cumulative) is dropped: 1200 of 3700 billable tokens
    // (67.6% under-count) go unreported on this corpus.
    let reported = parse_fixture("remote-compaction.jsonl");
    let true_totals = Usage {
        input: 3100,
        cache_r: 800,
        output: 600,
        reasoning: 120,
    };
    assert_eq!(
        reported,
        Usage {
            input: 1000,
            cache_r: 0,
            output: 200,
            reasoning: 0,
        }
    );
    assert!(reported.total() < true_totals.total());
    eprintln!(
        "rm-304 verdict: reported {} of {} billable tokens ({:.1}% under-count)",
        reported.total(),
        true_totals.total(),
        100.0 * (true_totals.total() - reported.total()) as f64 / true_totals.total() as f64
    );
}

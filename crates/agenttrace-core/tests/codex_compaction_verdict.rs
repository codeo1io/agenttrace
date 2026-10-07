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
//! agenttrace's parser used to read only `event_msg`/`token_count` lines
//! (`parser.rs` `codex_line_is_ignorable` dropped `compacted` lines up front;
//! `token_usage_record` lines fell through the type match unused), so the
//! original verdict measured an under-count exactly equal to the compaction
//! turn's usage: 1200 of 3700 billable tokens (67.6%) unreported.
//!
//! The fix landed as rm-401 (run 2c2db6f5, integrated 2026-10-04): compaction
//! markers pair with `token_usage_record` events by response id, counted
//! exactly once per id (the `latest_token_usage_record` copy and the replayed
//! top-level record dedup, ccusage #1821 semantics), with every decision
//! disclosed in parse diagnostics. The second test now pins the FIXED totals
//! — flipping it back to the pre-fix numbers is the regression gate. Note the
//! parser's codex decomposition counts NET input (gross input minus cached)
//! with cache tracked separately and reasoning folded into output, so the
//! pinned numbers are expressed in that decomposition, not the gross-input
//! arithmetic of the original verdict; the post-compaction rewound snapshot
//! stays refused by the rm-162/#286 high-water guard (no double count), so
//! the post-compaction window's own usage remains its own follow-up class.

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
fn remote_compaction_usage_is_counted_after_the_fix() {
    // rm-304 verdict corpus, measured on remote-compaction.jsonl:
    //
    //   turn 1 (token_count)        in 1000 / out 200
    //   compaction turn (records)   in 1500 / cache 800 / out 300 / rea 120
    //   post-compaction (count)     in  600 / out 100
    //
    //   pre-fix agenttrace report   in 1000 / cache   0 / out 200 / rea   0
    //   post-fix (this pin)         in 1700 / cache 800 / out 620 / rea   0
    //   post-#312 (this pin)        in 2300 / cache 800 / out 600 / rea   0
    //
    // The compaction turn's record is now counted once in the parser's codex
    // decomposition: net input 1500-800=700 beside turn 1's 1000, cache_read
    // 800 tracked separately, output 300 with reasoning 120 folded in (420
    // beside turn 1's 200). The post-compaction snapshot (700 cumulative)
    // still sits below the rm-162/#286 high-water mark (1200), so the rewind
    // guard refuses it — no double count — leaving the post-compaction
    // window's own usage (600 in / 100 out) as its own follow-up class, not
    // silently eaten. Dropping back to the pre-fix row above is exactly the
    // regression this pin exists to catch.
    //
    // rm-617 (upstream #312, integration of run e5653f52) re-adjudicated
    // that "follow-up class": a REAL remote compaction RESETS the
    // cumulative snapshot to the compacted history, so the climb after
    // the reset is fresh usage the high-water guard was silently
    // refusing (the 600 in / 100 out window). Per-distinct-total
    // accounting with the delta fallback counts it, reasoning_output_
    // tokens stops being added on top of output (300, not 420), and the
    // pin above moves to the third row. Dropping back to either earlier
    // row is the regression this pin exists to catch.
    let reported = parse_fixture("remote-compaction.jsonl");
    assert_eq!(
        reported,
        Usage {
            input: 2300,
            cache_r: 800,
            output: 600,
            reasoning: 0,
        }
    );
    eprintln!(
        "rm-304/rm-401: compaction turn counted ({} billable tokens: net in + cache + out)",
        reported.input + reported.cache_r + reported.output,
    );
}

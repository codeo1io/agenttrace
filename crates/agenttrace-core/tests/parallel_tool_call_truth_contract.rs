//! rm-233/rm-230 parallel-tool-call truth contract tests.
//!
//! Selected batch "Parallel-tool-call truth" (run 64b6f06e cycle 5,
//! prioritize 5b9447f2, stewardship 1f8ee79c):
//!
//! - rm-233 LEAD: loop detection keys on (tool name, argument
//!   identity) with key-order-insensitive canonicalization. Three
//!   distinct-argument parallel calls in one assistant turn are NOT a
//!   retry loop even when every call returns byte-identical output
//!   (same-dir `ls`, identical error strings) — the pre-campaign
//!   result-content-hash keying misflagged exactly that shape. The
//!   identical-arguments case is still a loop, and arguments that
//!   differ only in key order count as identical.
//! - rm-230 ARM: the flat tool_use_id → tool_call_id pairing survives
//!   3 parallel same-name tool_use blocks with 3 present results —
//!   the Bash latency entry pairs all three (count 3, unmatched 0),
//!   never reporting present-result parallel calls as unmatched.
//!
//! Corpora live in tests/fixtures/claude-parallel-truth/ (controlled
//! namespace per docs/maintainers/test-flake-prevention.md Rule 7).

use agenttrace_core::{inspect_reason, needs_attention, parse_file};
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

// ---------------------------------------------------------------------------
// rm-233 — distinct-argument parallel calls are not a retry loop
// ---------------------------------------------------------------------------

/// Three parallel `Bash` calls with distinct arguments whose results
/// are byte-identical (`"total 0\n"`). The old result-content-hash
/// keying fingerprinted all three pairs identically and reported a
/// retry loop; argument-identity keying must report none.
#[test]
fn distinct_argument_parallel_calls_are_not_loops() {
    let session = parse_file(&fixture(
        "claude-parallel-truth/parallel-distinct-args-identical-results.jsonl",
    ))
    .expect("parallel-truth corpus parses");
    assert_eq!(
        session.diagnostics.loop_cost.loop_groups, 0,
        "three distinct-argument calls must not fingerprint as one loop"
    );
    assert_eq!(session.diagnostics.loop_cost.retry_events, 0);
    assert_eq!(session.diagnostics.loop_cost.total_loop_cost, 0.0);
}

// ---------------------------------------------------------------------------
// rm-233 — identical arguments (key order aside) are a retry loop
// ---------------------------------------------------------------------------

/// Three parallel `Bash` calls with identical arguments whose JSON
/// key order shuffles across the three blocks, and whose results
/// differ. Canonicalization makes the shuffles identical, so the
/// group IS a loop regardless of result content.
#[test]
fn identical_arguments_key_order_shuffled_are_a_loop() {
    let session = parse_file(&fixture(
        "claude-parallel-truth/parallel-identical-args-key-shuffled.jsonl",
    ))
    .expect("key-shuffled corpus parses");
    assert_eq!(
        session.diagnostics.loop_cost.loop_groups, 1,
        "three identical-argument calls (key order aside) are one loop"
    );
    assert!(session.diagnostics.loop_cost.total_loop_cost > 0.0);
}

// ---------------------------------------------------------------------------
// rm-233 — the attention lane agrees with loop_cost
// ---------------------------------------------------------------------------

/// The attention classifier ranks "loops" at priority 1 from the
/// fingerprint lane. Before this batch that lane keyed on result
/// content, so the distinct-argument corpus above — identical
/// `"total 0\n"` results — flagged priority-1 loops attention while
/// loop_cost correctly reported none. The two lanes must agree.
#[test]
fn distinct_argument_parallel_calls_do_not_flag_loops_attention() {
    let session = parse_file(&fixture(
        "claude-parallel-truth/parallel-distinct-args-identical-results.jsonl",
    ))
    .expect("parallel-truth corpus parses");
    assert_eq!(inspect_reason(&session), "ok");
    assert!(!needs_attention(&session));
}

// ---------------------------------------------------------------------------
// rm-230 — parallel same-name tool_use blocks all pair with results
// ---------------------------------------------------------------------------

/// The same distinct-argument corpus drives the pairing lane: three
/// same-name tool_use blocks in one assistant message with three
/// present tool_results must pair every call by id — the `Bash`
/// latency entry reports count 3 and unmatched 0.
#[test]
fn parallel_same_name_tool_uses_pair_with_present_results() {
    let session = parse_file(&fixture(
        "claude-parallel-truth/parallel-distinct-args-identical-results.jsonl",
    ))
    .expect("parallel-truth corpus parses");
    let bash = session
        .diagnostics
        .tool_latencies
        .iter()
        .find(|latency| latency.tool_name == "Bash")
        .expect("Bash latency entry exists");
    assert_eq!(bash.count, 3, "all three parallel calls pair by id");
    assert_eq!(
        bash.unmatched, 0,
        "present-result parallel calls are never unmatched"
    );
}

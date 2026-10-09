//! rm-871: the context-utilization NUMERATOR must be vendor-measured when
//! the journal reports usage, with the historic bytes/2 estimate demoted to
//! a disclosed fallback (`numerator_source`).
//!
//! `usage-heavy.jsonl` is the cycle-3 assess probe corpus — a session
//! reporting 1,000 input + 500 cache_creation + 400,000 cache_read + 200
//! output tokens. Before rm-871 this session measured 1.4401% utilization
//! (bytes/2 heuristic) against its real ~401.7k-token context; the
//! percentage must now be derived from the reported usage.

use agenttrace_core::parse_file;
use std::fs;
use std::path::PathBuf;

const USAGE_HEAVY: &str = include_str!("fixtures/rm-871-truth/usage-heavy.jsonl");
const NO_USAGE: &str = include_str!("fixtures/rm-871-truth/no-usage.jsonl");
const NEG_OUT: &str = include_str!("fixtures/rm-871-truth/negative-output.jsonl");

fn write(tag: &str, raw: &str) -> PathBuf {
    // pid-qualify so overlapping test invocations never race on scratch
    // paths (fleet precedent 04b1f7f7 U2).
    let path = std::env::temp_dir().join(format!("at-rm871-{}-{tag}.jsonl", std::process::id()));
    fs::write(&path, raw).expect("write fixture");
    path
}

fn session(tag: &str, raw: &str) -> agenttrace_core::Session {
    let path = write(tag, raw);
    let session = parse_file(&path).expect("session parses");
    let _ = fs::remove_file(&path);
    session
}

#[test]
fn usage_reported_session_measures_utilization_from_usage() {
    let session = session("heavy", USAGE_HEAVY);
    let cu = &session.diagnostics.context_utilization;

    assert_eq!(cu.numerator_source, "usage", "usage lane must be selected");
    // prompt occupancy = input + cache_read + cache_creation; the turn's
    // output becomes context for the next turn.
    let prompt = 1_000 + 400_000 + 500;
    let completion = 200;
    assert_eq!(cu.conversation_history, prompt + completion);
    assert_eq!(cu.used, prompt + completion);
    // the vendor-measured number already contains tool/system tokens, so
    // the heuristic components are reported as 0, not double-counted.
    assert_eq!(cu.tool_definitions, 0);
    assert_eq!(cu.system_prompt, 0);

    // exact percentage against the ladder-resolved window (round4'd)
    let expected =
        agenttrace_core::round4((prompt + completion) as f64 / cu.estimated_total as f64 * 100.0);
    assert_eq!(cu.utilization_pct, expected);
    // headline regression guard: the pre-fix value was 1.4401; the
    // ladder resolves this model's input window to 1,000,000, so the
    // measured ~401.7k tokens sit at 40.17%.
    assert_eq!(cu.utilization_pct, 40.17);
    // round4: no raw-float noise (pre-fix: 1.4401000000000002)
    let scaled = cu.utilization_pct * 10_000.0;
    assert_eq!(scaled, scaled.round());
    // rm-871(b): the exact assess-probe shape — ~401.7k vendor-measured
    // tokens against the 1M catalog window — must not read `good` after
    // the fix (it read 1.44% / `good` before): 40.17% occupancy is
    // `caution` under the occupancy arm of the risk gate.
    assert_eq!(cu.risk_level, "caution");
    assert_ne!(cu.risk_level, "good");
}

#[test]
fn usage_reported_session_clamps_negatives_in_the_numerator_too() {
    // negative-output.jsonl reports output_tokens: -2_000_000 and
    // cache_read_input_tokens: -400_000. The usage lane must clamp those
    // exactly like the token fold does (disclosure pinned in
    // negative_usage_disclosure.rs), never let a negative shrink the
    // measured context.
    let session = session("negout", NEG_OUT);
    let cu = &session.diagnostics.context_utilization;
    assert_eq!(cu.numerator_source, "usage");
    assert_eq!(cu.used, 1_000, "input only; both negatives clamp to 0");
}

#[test]
fn journal_without_usage_falls_back_to_disclosed_heuristic() {
    let session = session("no-usage", NO_USAGE);
    let cu = &session.diagnostics.context_utilization;

    assert_eq!(
        cu.numerator_source, "heuristic_bytes",
        "no usage anywhere -> disclosed byte heuristic"
    );
    assert!(cu.conversation_history > 0);
    assert_eq!(cu.tool_definitions, 8 * 300, "tool floor stays 8");
    assert_eq!(cu.system_prompt, 12_000, "heuristic system allowance stays");
    assert_eq!(
        cu.used,
        cu.conversation_history + cu.tool_definitions + cu.system_prompt
    );
    assert!(cu.utilization_pct > 0.0);
    // fallback keeps the same rounding discipline
    let scaled = cu.utilization_pct * 10_000.0;
    assert_eq!(scaled, scaled.round());
}

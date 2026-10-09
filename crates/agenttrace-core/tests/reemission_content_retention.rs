//! rm-891 contract tests (run ac3ac300 cycle 2): the rm-834 claude-code
//! streaming fold is keep-max for usage but used to be last-wins for
//! CONTENT — a keyed assistant message whose FINAL re-emission omits the
//! thinking block an earlier snapshot carried silently deleted that
//! reasoning (`reasoning_blocks` collapsed 1 → 0; assess F2, run
//! c92f079f, live PoC: `agenttrace -f json poc-reasoning/session.jsonl`
//! reported `reasoning.blocks 0` where the control corpus reported 1).
//! The fold must be per-field union (keep-max): an empty field on the
//! latest snapshot keeps the previous non-empty one; a non-empty field
//! still replaces, so monotonic growth stays byte-identical.
//!
//! Secondary: within one keyed message the tool_result dedup key was
//! `(message_id, tool_use_id)` — two DISTINCT results sharing an empty
//! `tool_use_id` collapsed into the first. Id-less results key by
//! position within the emission instead (rm-891).
//!
//! Fixtures: `fixtures/reemission-thinking/` (committed minimal PoC
//! corpora, sha256 pins in its README per the rm-542 convention). The
//! rm-834 monotonic-growth pins in `truthful_streaming_and_windows.rs`
//! stay green and unloosened — nothing in this file edits them.

use std::path::PathBuf;

use agenttrace_core::parse_file;

fn fixture(rel: &[&str]) -> PathBuf {
    let mut p: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for part in rel {
        p = p.join(part);
    }
    p
}

fn parse_fixture(rel: &[&str]) -> agenttrace_core::Session {
    parse_file(&fixture(rel)).unwrap_or_else(|e| panic!("parse {rel:?}: {e:#}"))
}

#[test]
fn non_monotonic_final_emission_keeps_earlier_thinking() {
    // The finding: msg_01 is re-emitted with only the text block; the
    // union fold must keep the thinking block the earlier snapshot
    // carried. Pre-fix: reasoning_blocks == 0, reasoning_chars == 0.
    let session = parse_fixture(&["reemission-thinking", "non-monotonic.jsonl"]);
    assert_eq!(
        session.metrics.assistant_turns, 1,
        "one keyed message is still one turn"
    );
    assert_eq!(
        session.metrics.reasoning_blocks, 1,
        "rm-891: a final re-emission that omits the thinking block must not delete it"
    );
    assert!(
        session.metrics.reasoning_chars > 0,
        "the retained thinking carries characters, not just a block count"
    );
}

#[test]
fn identical_reemission_still_folds_to_one_turn() {
    // Control (PoC poc-reasoning-ctrl): both snapshots carry the thinking
    // block. Unchanged by the fix — one turn, one reasoning block, and
    // usage still keep-max (200 output, not 250).
    let session = parse_fixture(&["reemission-thinking", "control-keeps-thinking.jsonl"]);
    assert_eq!(session.metrics.assistant_turns, 1);
    assert_eq!(session.metrics.reasoning_blocks, 1);
    assert_eq!(session.metrics.tokens_output, 200, "usage stays keep-max");
}

#[test]
fn monotonic_growth_keeps_final_snapshot() {
    // The union must not over-retain either: the first emission has only
    // text, the final adds thinking — the latest non-empty fields win, so
    // growth direction is byte-identical to the old replace-in-place.
    let session = parse_fixture(&["reemission-thinking", "monotonic-growth.jsonl"]);
    assert_eq!(session.metrics.assistant_turns, 1);
    assert_eq!(session.metrics.reasoning_blocks, 1);
    assert_eq!(session.metrics.tokens_output, 200, "usage stays keep-max");
}

#[test]
fn empty_tool_use_ids_key_distinct_results() {
    // One keyed message emission carrying TWO distinct tool_result
    // blocks, both with an empty tool_use_id, re-emitted identically.
    // Pre-fix the (message_id, tool_use_id) key collapsed both into the
    // first: tool_results == 1. Ordinal keying keeps both distinct while
    // the re-emission still folds away: 2 results, not 4.
    let session = parse_fixture(&["reemission-thinking", "empty-tool-ids.jsonl"]);
    // The emission carries only tool_result blocks, so there is no
    // assistant turn to fold — the assertion target is the tool events.
    assert_eq!(
        session.metrics.tool_results, 2,
        "rm-891: distinct id-less results must not collapse into the first"
    );
}

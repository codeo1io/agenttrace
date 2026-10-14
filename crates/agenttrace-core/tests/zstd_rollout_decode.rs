//! rm-992 — in-process bounded zstd decode for Codex rollouts.
//!
//! Codex ≥0.152 stores rollout transcripts as zstd frames (upstream PR
//! #41357, merged to codex main 2026-10-13): the file keeps a `.jsonl` name
//! but its content starts with the zstd magic `28 B5 2F FD`. These tests pin
//! the three arms of the decode lane at `parse_session_bytes`:
//!
//! 1. golden parity — a compressed rollout parses to the SAME session as its
//!    uncompressed twin (the decode is transparent to the parser tail);
//! 2. bomb refusal — a frame that would decompress past the cap is refused
//!    with a loud, named error, never an unbounded allocation;
//! 3. stdin parity — the shared seam covers `parse_stdin_bytes` too.
//!
//! Empty and corrupt frames stay pinned by the pre-existing failure-path
//! tests (`truthful_streaming_and_windows.rs`,
//! `discovery_contract.rs::zstd_rollouts_fail_with_a_named_error_not_generic_utf8`).

use agenttrace_core::parse_file;
use agenttrace_core::parse_stdin_bytes;

fn fixture(parts: &[&str]) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.extend(parts);
    p
}

#[test]
fn zstd_rollout_decodes_to_the_same_session_as_the_uncompressed_twin() {
    let twin = fixture(&["tests", "fixtures", "codex-custom-tools", "rollout.jsonl"]);
    let compressed = fixture(&["tests", "fixtures", "codex-zstd", "rollout.jsonl"]);

    // Guard the fixtures themselves: the compressed one must really be a
    // zstd frame, or this test silently degrades into testing the plain path.
    let magic = std::fs::read(&compressed).unwrap();
    assert_eq!(
        &magic[..4],
        &[0x28, 0xB5, 0x2F, 0xFD],
        "golden fixture must be a zstd frame"
    );

    let plain = parse_file(&twin).expect("uncompressed twin parses");
    let decoded = parse_file(&compressed).expect("zstd rollout decodes and parses");

    assert_eq!(decoded.name, plain.name);
    assert_eq!(decoded.metrics.events_total, plain.metrics.events_total);
    assert_eq!(decoded.metrics.tokens_input, plain.metrics.tokens_input);
    assert_eq!(decoded.metrics.tokens_output, plain.metrics.tokens_output);
}

#[test]
fn zstd_rollout_over_the_decode_cap_is_refused_with_a_named_error() {
    let bomb = fixture(&["tests", "fixtures", "codex-zstd", "rollout-bomb.jsonl"]);
    let bytes = std::fs::read(&bomb).unwrap();
    assert_eq!(&bytes[..4], &[0x28, 0xB5, 0x2F, 0xFD]);

    let err =
        parse_file(&bomb).expect_err("a frame past the decode cap must be refused, never decoded");
    let msg = format!("{err:#}");
    assert!(msg.contains("zstd"), "refusal must name the format: {msg}");
    assert!(
        msg.contains("64 MiB"),
        "refusal must disclose the cap: {msg}"
    );
    assert!(
        msg.contains("rollout-bomb.jsonl"),
        "refusal must name the file: {msg}"
    );
    // The refusal must not be mis-filed as a generic UTF-8 problem.
    assert!(!msg.contains("not valid UTF-8"), "refusal wording: {msg}");
}

#[test]
fn stdin_bytes_share_the_bounded_zstd_decode_lane() {
    let bytes = std::fs::read(fixture(&[
        "tests",
        "fixtures",
        "codex-zstd",
        "rollout.jsonl",
    ]))
    .unwrap();
    let via_stdin = parse_stdin_bytes(bytes).expect("stdin path decodes the zstd rollout");
    let via_file = parse_file(&fixture(&[
        "tests",
        "fixtures",
        "codex-custom-tools",
        "rollout.jsonl",
    ]))
    .unwrap();
    assert_eq!(via_stdin.name, via_file.name);
    assert_eq!(
        via_stdin.metrics.tokens_input,
        via_file.metrics.tokens_input
    );
}

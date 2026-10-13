//! rm-1005 contract tests: Codex's zstd-framed rollouts (magic
//! 28 B5 2F FD, upstream PR #41357) decode IN-PROCESS via ruzstd —
//! no external CLI, no shell-out — bounded by the decode cap, and
//! the decoded body parses identically to its plain-JSONL twin.
//!
//! `fixtures/codex-zstd/rollout-with-usage.jsonl.zst` is the committed
//! `codex-legacy/rollout-with-usage.jsonl` corpus run through real
//! zstd -19 (the raw-block frame test below additionally pins the
//! decoder on a frame built byte-by-byte in Rust, so the suite never
//! depends on a host zstd binary).

use std::path::{Path, PathBuf};

use agenttrace_core::parse_file;

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

#[test]
fn zstd_rollout_parses_with_usage_accounting() {
    let session = parse_file(&fixture("codex-zstd/rollout-with-usage.jsonl.zst"))
        .expect("in-process zstd decode");
    assert_eq!(session.metrics.source_tool, "codex_cli");
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(session.metrics.tokens_input, 400);
}

/// Decode fidelity, not just absence of error: the zstd twin must
/// produce the SAME accounting as the plain-JSONL original.
#[test]
fn zstd_twin_matches_the_plain_rollout_exactly() {
    let plain = parse_file(&fixture("codex-legacy/rollout-with-usage.jsonl")).expect("plain");
    let zstd = parse_file(&fixture("codex-zstd/rollout-with-usage.jsonl.zst")).expect("zstd");
    assert_eq!(plain.metrics.source_tool, zstd.metrics.source_tool);
    assert_eq!(plain.metrics.tokens_input, zstd.metrics.tokens_input);
    assert_eq!(plain.metrics.tokens_output, zstd.metrics.tokens_output);
    assert_eq!(plain.metrics.tokens_cache_r, zstd.metrics.tokens_cache_r);
    assert_eq!(plain.metrics.model_used, zstd.metrics.model_used);
    assert_eq!(
        plain.metrics.provenance.tokens,
        zstd.metrics.provenance.tokens
    );
    assert_eq!(plain.metrics.cost_estimated, zstd.metrics.cost_estimated);
}

#[test]
fn corrupt_frame_is_a_named_parse_failure_not_silent() {
    let path = std::env::temp_dir().join(format!(
        "agenttrace-rm1005-corrupt-{}.zst",
        std::process::id()
    ));
    // Valid magic, everything after it garbage.
    std::fs::write(&path, [0x28, 0xB5, 0x2F, 0xFD, 0xEE, 0xEE, 0xEE, 0xEE]).expect("write");
    let error = parse_file(&path).expect_err("corrupt frame must fail");
    let message = format!("{error:#}");
    assert!(
        message.contains("zstd"),
        "failure names the format: {message}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn truncated_real_frame_reports_corruption() {
    let whole = std::fs::read(fixture("codex-zstd/rollout-with-usage.jsonl.zst")).expect("read");
    let path = std::env::temp_dir().join(format!(
        "agenttrace-rm1005-trunc-{}.zst",
        std::process::id()
    ));
    std::fs::write(&path, &whole[..whole.len() / 2]).expect("write truncated");
    let error = parse_file(&path).expect_err("truncated frame must fail");
    let message = format!("{error:#}");
    assert!(
        message.contains("zstd"),
        "failure names the format: {message}"
    );
    let _ = std::fs::remove_file(&path);
}

/// A frame assembled byte-by-byte in Rust (single-segment header, one
/// RAW block): the decoder has no host-tool dependency and accepts
/// any legal frame shape, not only what a zstd CLI emits.
#[test]
fn raw_block_frame_decodes_without_cli_dependency() {
    let body = concat!(
        r#"{"timestamp":"2026-05-14T10:00:00.000Z","type":"session_meta","payload":{"id":"rawblock-frame","timestamp":"2026-05-14T10:00:00.000Z","cwd":"/home/user/proj","originator":"codex_cli_rs","cli_version":"0.152.0","instructions":null}}"#,
        "\n",
        r#"{"timestamp":"2026-05-14T10:00:01.000Z","type":"task_started","payload":{"id":"task_rawblock"}}"#,
        "\n",
        r#"{"timestamp":"2026-05-14T10:00:02.000Z","type":"turn_context","payload":{"cwd":"/home/user/proj","model":"gpt-5-codex","effort":"medium","summary":"auto"}}"#,
        "\n"
    );
    let mut frame: Vec<u8> = vec![0x28, 0xB5, 0x2F, 0xFD];
    // Frame_Header_Descriptor: single-segment, FCS field 4 bytes
    // (the body exceeds the 255-byte ceiling of the 1-byte form).
    frame.push(0xA0);
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    // Block_Header: last_block=1, block_type=raw(0), block_size.
    let block_header: u32 = 1 | ((body.len() as u32) << 3);
    frame.extend_from_slice(&block_header.to_le_bytes()[..3]);
    frame.extend_from_slice(body.as_bytes());

    let path = std::env::temp_dir().join(format!(
        "agenttrace-rm1005-rawblock-{}.zst",
        std::process::id()
    ));
    std::fs::write(&path, &frame).expect("write frame");
    let session = parse_file(&path).expect("raw-block frame decodes");
    assert_eq!(session.metrics.source_tool, "codex_cli");
    assert_eq!(session.metrics.model_used, "gpt-5-codex");
    let _ = std::fs::remove_file(&path);
}

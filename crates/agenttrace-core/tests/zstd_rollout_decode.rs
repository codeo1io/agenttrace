//! rm-992 — in-process bounded zstd decode for Codex rollouts.
//!
//! Codex stores rollout transcripts as zstd frames (reader tolerance
//! since upstream ≥0.152, PR #41357 merged 2026-10-13): stable 0.162.x
//! migrates cold rollouts to `.jsonl.zst` names while writing compressed
//! rollouts by default stays feature-flagged (#53732 rode the 0.163
//! alphas and the flag was still off at tag rust-v0.163.0 — the rm-1518
//! correction in CHANGELOG; compressed user data on stable: ccusage
//! #1888, fixed #1890): the file keeps a `.jsonl` name but its content
//! starts with the zstd magic `28 B5 2F FD`. These tests pin
//! the decode lane at `parse_session_bytes`:
//!
//! 1. golden parity — a compressed rollout parses to the SAME session as its
//!    uncompressed twin (the decode is transparent to the parser tail);
//! 2. bomb refusal — a frame that would decompress past the cap is refused
//!    with a loud, named error, never an unbounded allocation;
//! 3. stdin parity — the shared seam covers `parse_stdin_bytes` too.
//!
//! rm-1516 — Codex appends to a compressed rollout as CONCATENATED zstd
//! frames (upstream `seekable_reader.rs` explicitly tolerates them;
//! `ruzstd`'s `StreamingDecoder` is single-frame by contract), so the lane
//! must decode every frame to true EOF:
//!
//! 4. multi-frame parity — a two-frame rollout (the golden twin split at a
//!    line boundary, re-compressed as two frames, `zstd -dc`-identical to
//!    the twin) parses to the SAME session as the twin; stopping after
//!    frame 1 was a silent half-journal truncation;
//! 5. total cap — the 64 MiB cap bounds the SUM across frames, so two
//!    under-cap frames are still refused once the total passes it;
//! 6. trailing garbage — bytes after the last frame that are not a frame
//!    are refused loudly with the byte offset, exactly like `zstd -d`;
//! 7. skippable frames — magic `184D2A50..=184D2A5F` frames are skipped,
//!    exactly like `zstd -d` (a skippable frame at the head never reaches
//!    this lane: the magic gate at `parse_session_bytes` only admits
//!    `28 B5 2F FD`, so the pin is a skip AFTER a real frame).
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

/// rm-1516 — a two-frame rollout must decode to the whole journal. The
/// fixture is the golden twin split at a line boundary and re-compressed
/// as two concatenated frames; `zstd -dc` on it is byte-identical to the
/// twin (pinned at generation time, `zstd -l` reports 2 frames / 0
/// skips), so anything less than twin-parity is record loss. Before
/// rm-1516 the decoder stopped after frame 1 and the parser silently
/// billed half the journal.
#[test]
fn concatenated_zstd_frames_decode_to_the_whole_journal() {
    let twin = fixture(&["tests", "fixtures", "codex-custom-tools", "rollout.jsonl"]);
    let multi = fixture(&[
        "tests",
        "fixtures",
        "codex-zstd",
        "rollout-multiframe.jsonl",
    ]);

    // Guard the fixture itself: it must be zstd, and it must be LONGER
    // than its single-frame sibling so the second frame is really there.
    let bytes = std::fs::read(&multi).unwrap();
    assert_eq!(
        &bytes[..4],
        &[0x28, 0xB5, 0x2F, 0xFD],
        "multi-frame fixture must be zstd"
    );
    let single = std::fs::read(fixture(&[
        "tests",
        "fixtures",
        "codex-zstd",
        "rollout.jsonl",
    ]))
    .unwrap();
    assert!(
        bytes.len() > single.len(),
        "multi-frame fixture ({}) must carry a second frame past the single-frame sibling ({})",
        bytes.len(),
        single.len()
    );

    let plain = parse_file(&twin).expect("uncompressed twin parses");
    let decoded = parse_file(&multi).expect("concatenated frames decode to the whole journal");

    assert_eq!(decoded.name, plain.name);
    assert_eq!(
        decoded.metrics.events_total, plain.metrics.events_total,
        "every frame's records must be parsed — frame-1-only decode is silent truncation"
    );
    assert_eq!(decoded.metrics.tokens_input, plain.metrics.tokens_input);
    assert_eq!(decoded.metrics.tokens_output, plain.metrics.tokens_output);
}

/// rm-1516 — the decode cap bounds the TOTAL decompressed output across
/// all frames: the single-frame bomb can only pin the per-frame arm, so
/// this fixture carries two 40 MiB frames — each under the 64 MiB cap on
/// its own, 80 MiB together. The refusal must name the format and the
/// cap, never half-decode quietly.
#[test]
fn the_decode_cap_bounds_the_total_across_concatenated_frames() {
    let bomb = fixture(&[
        "tests",
        "fixtures",
        "codex-zstd",
        "rollout-bomb-multiframe.jsonl",
    ]);
    let bytes = std::fs::read(&bomb).unwrap();
    assert_eq!(&bytes[..4], &[0x28, 0xB5, 0x2F, 0xFD]);

    let err =
        parse_file(&bomb).expect_err("frames summing past the cap must be refused, never decoded");
    let msg = format!("{err:#}");
    assert!(msg.contains("zstd"), "refusal must name the format: {msg}");
    assert!(
        msg.contains("64 MiB"),
        "refusal must disclose the cap: {msg}"
    );
    assert!(
        msg.contains("rollout-bomb-multiframe.jsonl"),
        "refusal must name the file: {msg}"
    );
    assert!(!msg.contains("not valid UTF-8"), "refusal wording: {msg}");
}

/// rm-1516 — trailing bytes after the last complete frame are refused
/// loudly with the byte offset where they start. `zstd -d` refuses the
/// same file (`unsupported format`); the pre-rm-1516 decode returned
/// frame 1 and PARSED it, dropping the tail silently. The offset is
/// derived from the frame's own length, so the pin does not hardcode
/// fixture bytes.
#[test]
fn trailing_bytes_after_the_last_frame_are_refused_with_the_offset() {
    let frame = std::fs::read(fixture(&[
        "tests",
        "fixtures",
        "codex-zstd",
        "rollout.jsonl",
    ]))
    .unwrap();
    let offset = frame.len();
    let mut bytes = frame.clone();
    bytes.extend_from_slice(b"this is not another zstd frame");

    let err = parse_stdin_bytes(bytes).expect_err("trailing non-frame bytes must not parse");
    let msg = format!("{err:#}");
    assert!(msg.contains("zstd"), "refusal must name the format: {msg}");
    assert!(
        msg.contains("trailing"),
        "refusal must call out trailing bytes: {msg}"
    );
    assert!(
        msg.contains(&format!("byte {offset}")),
        "refusal must name the byte offset where the trailing bytes start: {msg}"
    );
}

/// rm-1516 — skippable frames (`184D2A50..=184D2A5F`) carry no data and
/// are stepped over, exactly like `zstd -d`; `ruzstd` surfaces them as a
/// header error the loop must honor instead of failing the file.
#[test]
fn skippable_frames_after_a_real_frame_are_skipped() {
    let frame = std::fs::read(fixture(&[
        "tests",
        "fixtures",
        "codex-zstd",
        "rollout.jsonl",
    ]))
    .unwrap();
    let mut bytes = Vec::with_capacity(frame.len() + 16);
    bytes.extend_from_slice(&frame);
    bytes.extend_from_slice(&0x184D2A50u32.to_le_bytes()); // skippable magic
    bytes.extend_from_slice(&4u32.to_le_bytes()); // body length
    bytes.extend_from_slice(b"junk"); // the skipped body

    let skipped = parse_stdin_bytes(bytes)
        .expect("a skippable frame after a real frame must not fail the decode");
    let twin = parse_file(&fixture(&[
        "tests",
        "fixtures",
        "codex-custom-tools",
        "rollout.jsonl",
    ]))
    .unwrap();
    assert_eq!(skipped.name, twin.name);
    assert_eq!(skipped.metrics.events_total, twin.metrics.events_total);
    assert_eq!(skipped.metrics.tokens_input, twin.metrics.tokens_input);
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

#[test]
fn missing_jsonl_read_retries_the_zstd_sibling() {
    // rm-992 arm b (run d57c2e56 cycle 1): codex compresses a finished
    // rollout by RENAMING it (`.jsonl` -> `.jsonl.zst`), so a path handed
    // to `parse_file` by an earlier discovery walk can be gone at read
    // time. A NotFound must retry the sibling — the encoding-fallback
    // ccusage's codex reader ships (#1888 -> #1890).
    let dir = std::env::temp_dir().join(format!("agenttrace-rm992-sibling-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create sibling temp dir");
    std::fs::write(
        dir.join("rollout-2026-10-18T09-00-00-4f1c9d21.jsonl.zst"),
        std::fs::read(fixture(&[
            "tests",
            "fixtures",
            "codex-zstd",
            "rollout.jsonl",
        ]))
        .expect("read golden frame"),
    )
    .expect("write compressed sibling");
    let session = parse_file(&dir.join("rollout-2026-10-18T09-00-00-4f1c9d21.jsonl"))
        .expect("a NotFound on the plain name must retry the .jsonl.zst sibling");
    let twin = parse_file(&fixture(&[
        "tests",
        "fixtures",
        "codex-custom-tools",
        "rollout.jsonl",
    ]))
    .expect("plain twin parses");
    assert_eq!(session.metrics.events_total, twin.metrics.events_total);
    assert!(
        session
            .metrics
            .disclosure_counters
            .get("zstd_rollout_decoded")
            .is_some_and(|count| *count > 0),
        "the sibling read is a decode and must disclose it: {:?}",
        session.metrics.disclosure_counters
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_jsonl_zst_read_retries_the_plain_sibling() {
    // The flip is bidirectional: codex RESTORES the plain name on
    // demand, so a stale `.jsonl.zst` path must retry the `.jsonl`
    // sibling. A plain body parsed this way is NOT a decode — no
    // `zstd_rollout_decoded` counter may appear.
    let dir = std::env::temp_dir().join(format!(
        "agenttrace-rm992-sibling-rev-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create sibling temp dir");
    std::fs::write(
        dir.join("rollout-2026-10-18T09-00-00-4f1c9d21.jsonl"),
        std::fs::read(fixture(&[
            "tests",
            "fixtures",
            "codex-custom-tools",
            "rollout.jsonl",
        ]))
        .expect("read plain twin"),
    )
    .expect("write plain sibling");
    let session = parse_file(&dir.join("rollout-2026-10-18T09-00-00-4f1c9d21.jsonl.zst"))
        .expect("a NotFound on the compressed name must retry the .jsonl sibling");
    assert_eq!(session.metrics.events_total, 38);
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("zstd_rollout_decoded"),
        "a plain sibling read is not a decode: {:?}",
        session.metrics.disclosure_counters
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_file_without_a_sibling_still_names_the_requested_path() {
    // A NotFound with no sibling pair stays a NotFound FOR THE
    // REQUESTED PATH: the error names the path the caller asked for,
    // never a sibling that was only probed.
    let dir = std::env::temp_dir().join(format!("agenttrace-rm992-nosib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create nosib temp dir");
    let missing = dir.join("never-existed.jsonl");
    let error = parse_file(&missing).expect_err("no sibling exists");
    let chain = format!("{error:#}");
    assert!(
        chain.contains("never-existed.jsonl"),
        "the error must name the requested path, got: {chain}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

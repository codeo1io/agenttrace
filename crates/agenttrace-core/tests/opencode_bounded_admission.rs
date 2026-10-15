//! rm-1437: bounded admission for the opencode transcript lane.
//!
//! `read_opencode_records` stat-caps whole-file reads: over-cap transcripts are
//! skipped (never admitted into memory) and disclosed through
//! `Metrics::line_skips` — the rm-526 loss channel that `--doctor` folds next
//! to the other skip classes. These tests pin the three acceptance arms:
//!  1. under-cap corpora disclose nothing and keep their outputs identical,
//!  2. an over-cap message transcript is skipped, disclosed, and changes no
//!     surviving output,
//!  3. the part lane (per-message `part/<id>/`) is bounded and disclosed too,
//!     and both lanes accumulate into one counter pair.

use std::fs;
use std::path::{Path, PathBuf};

use agenttrace_core::{parse_file, Session};

/// 10 MiB read cap (rm-1437); one byte over is already over-cap.
const CAP_BYTES: usize = 10 * 1024 * 1024;

fn write_storage(root: &Path, oversized_message: bool, oversized_part: bool) -> PathBuf {
    write_storage_ext(root, oversized_message, oversized_part, true)
}

/// `include_m1 = false` builds the never-existed baseline: m1's message and
/// part files are simply absent (the honest equivalence target for a cap
/// skip — a skipped file must leave the session exactly as if it never
/// existed, plus the disclosure counters).
fn write_storage_ext(
    root: &Path,
    oversized_message: bool,
    oversized_part: bool,
    include_m1: bool,
) -> PathBuf {
    let session_id = "sess_1";
    fs::create_dir_all(root.join("opencode/storage/session").join(session_id)).unwrap();
    fs::create_dir_all(root.join("opencode/storage/message").join(session_id)).unwrap();
    fs::create_dir_all(root.join("opencode/storage/part/m1")).unwrap();
    fs::create_dir_all(root.join("opencode/storage/part/m2")).unwrap();

    fs::write(
        root.join(format!(
            "opencode/storage/session/{session_id}/{session_id}.json"
        )),
        r#"{
  "id": "sess_1",
  "slug": "bounded-admission",
  "projectID": "prj_1",
  "title": "Bounded admission",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    )
    .unwrap();

    let mut m1 = r#"{"id": "m1", "sessionID": "sess_1", "role": "user", "time": {"created": 1764750001000}}"#.to_string();
    if oversized_message {
        // Valid JSON object, but >10 MiB: stat-capped away before the read.
        m1 = format!(
            r#"{{"id": "m1", "sessionID": "sess_1", "role": "user", "time": {{"created": 1764750001000}}, "padding": "{}"}}"#,
            "A".repeat(CAP_BYTES + 1024)
        );
    }
    fs::write(
        root.join(format!("opencode/storage/message/{session_id}/m1.json")),
        m1,
    )
    .unwrap();
    fs::write(
        root.join(format!("opencode/storage/message/{session_id}/m2.json")),
        r#"{"id": "m2", "sessionID": "sess_1", "role": "assistant", "time": {"created": 1764750002000}}"#,
    )
    .unwrap();

    fs::write(
        root.join("opencode/storage/part/m1/p1.json"),
        r#"{"id": "p1", "type": "text", "text": "hello world", "time": {"created": 1764750001500}}"#,
    )
    .unwrap();
    let mut p2 = r#"{"id": "p2", "type": "text", "text": "answer text", "time": {"created": 1764750002500}}"#.to_string();
    if oversized_part {
        p2 = format!(
            r#"{{"id": "p2", "type": "text", "text": "{}", "time": {{"created": 1764750002500}}}}"#,
            "B".repeat(CAP_BYTES + 1024)
        );
    }
    fs::write(root.join("opencode/storage/part/m2/p2.json"), p2).unwrap();
    // Always-admitted sibling part: proves a capped part does not take down
    // its surviving siblings (m2 keeps events even when p2 is capped away).
    fs::write(
        root.join("opencode/storage/part/m2/p3.json"),
        r#"{"id": "p3", "type": "text", "text": "small sibling part", "time": {"created": 1764750002600}}"#,
    )
    .unwrap();

    if !include_m1 {
        fs::remove_file(root.join("opencode/storage/message/sess_1/m1.json")).unwrap();
        fs::remove_file(root.join("opencode/storage/part/m1/p1.json")).unwrap();
        let _ = fs::remove_dir(root.join("opencode/storage/part/m1"));
    }
    root.join(format!(
        "opencode/storage/session/{session_id}/{session_id}.json"
    ))
}

fn fingerprint(session: &Session) -> String {
    let m = &session.metrics;
    format!(
        "events={} users={} turns={} results={} calls_ok={} calls_fail={} reasoning={} authority={}",
        m.events_total,
        m.user_messages,
        m.assistant_turns,
        m.tool_results,
        m.tool_calls_ok,
        m.tool_calls_fail,
        m.reasoning_blocks,
        m.highest_authority
    )
}

#[test]
fn under_cap_corpus_discloses_nothing_and_keeps_outputs() {
    let root = std::env::temp_dir().join(format!("at_rm1437_under_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_storage(&root, false, false);

    let session = parse_file(&session_file).unwrap();

    let skips = &session.metrics.line_skips;
    assert!(
        !skips.contains_key("opencode_transcript_files_capped")
            && !skips.contains_key("opencode_transcript_bytes_capped"),
        "no disclosure may appear when the cap never fires: {skips:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn over_cap_message_transcript_is_skipped_and_disclosed() {
    let absent = std::env::temp_dir().join(format!("at_rm1437_msg_absent_{}", std::process::id()));
    let capped = std::env::temp_dir().join(format!("at_rm1437_msg_over_{}", std::process::id()));
    let _ = fs::remove_dir_all(&absent);
    let _ = fs::remove_dir_all(&capped);
    // Baseline: the same corpus with m1 never existing at all.
    let absent_file = write_storage_ext(&absent, false, false, false);
    let capped_file = write_storage(&capped, true, false);

    let absent_session = parse_file(&absent_file).unwrap();
    let capped_session = parse_file(&capped_file).unwrap();

    assert_eq!(
        fingerprint(&capped_session),
        fingerprint(&absent_session),
        "a capped message must leave the session exactly as if the file never existed"
    );

    let skips = &capped_session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_files_capped"),
        Some(&1),
        "message-lane skip must be disclosed: {skips:?}"
    );
    let bytes = skips
        .get("opencode_transcript_bytes_capped")
        .expect("bytes key");
    assert!(
        *bytes > CAP_BYTES,
        "disclosed bytes must exceed the cap: {bytes}"
    );

    let _ = fs::remove_dir_all(&absent);
    let _ = fs::remove_dir_all(&capped);
}

#[test]
fn over_cap_part_transcript_is_disclosed_and_lanes_accumulate() {
    let root = std::env::temp_dir().join(format!("at_rm1437_part_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    // Both lanes over-cap at once: one message transcript + one part transcript.
    let session_file = write_storage(&root, true, true);

    let session = parse_file(&session_file).unwrap();

    assert!(
        session.metrics.events_total > 0,
        "m2's events must survive the skips"
    );

    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_files_capped"),
        Some(&2),
        "message + part lanes must accumulate into one counter: {skips:?}"
    );
    assert!(
        skips
            .get("opencode_transcript_bytes_capped")
            .copied()
            .unwrap_or(0)
            > 2 * CAP_BYTES,
        "both over-cap files' bytes must be disclosed: {skips:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

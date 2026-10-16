//! rm-1866: read/parse failures in the opencode transcript fan-out are
//! losses, not silence.
//!
//! `read_opencode_records` used to swallow every `read_dir`, `read_to_string`,
//! and JSON-parse failure: failures returned empty vectors or `continue`d
//! with zero disclosure, and an unreadable message dir even masqueraded as
//! "unsupported session format" (the session bailed into the unsupported
//! lane). These tests pin the acceptance arms:
//!  1. unreadable files and unparseable JSON are counted per class and
//!     disclosed through `Metrics::line_skips` (the rm-526 loss channel,
//!     same as the rm-1437 caps), while surviving transcripts keep their
//!     events,
//!  2. an unreadable message DIR yields a parsed session with the
//!     dir-level disclosure — never "unsupported session format",
//!  3. a MISSING dir stays silent (a message without parts is the store's
//!     own empty-container case, not a loss).

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use std::fs;
use std::path::{Path, PathBuf};

use agenttrace_core::parse_file;

fn write_session_doc(root: &Path, session_id: &str) -> PathBuf {
    fs::create_dir_all(root.join("opencode/storage/session").join(session_id)).unwrap();
    let path = root.join(format!(
        "opencode/storage/session/{session_id}/{session_id}.json"
    ));
    fs::write(
        &path,
        r#"{
  "id": "sess_1",
  "slug": "read-failures",
  "projectID": "prj_1",
  "title": "Read failures",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    )
    .unwrap();
    path
}

fn write_message(root: &Path, session_id: &str, id: &str, body: &str) {
    fs::write(
        root.join(format!("opencode/storage/message/{session_id}/{id}.json")),
        body,
    )
    .unwrap();
}

#[test]
fn unparseable_message_transcripts_are_disclosed_and_survivors_keep_events() {
    let root = std::env::temp_dir().join(format!("at_rm1866_parse_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/message/sess_1")).unwrap();
    // One good assistant message with a good part AND a broken part, plus
    // one structurally broken message transcript — the broken message's
    // part dir is never reached (its message JSON never parses), so the
    // two losses land from different lanes.
    write_message(
        &root,
        "sess_1",
        "m2",
        r#"{"id": "m2", "sessionID": "sess_1", "role": "assistant", "time": {"created": 1764750002000}}"#,
    );
    write_message(&root, "sess_1", "m1", r#"{"id": "m1", "broken"#);
    fs::create_dir_all(root.join("opencode/storage/part/m2")).unwrap();
    fs::write(
        root.join("opencode/storage/part/m2/p1.json"),
        r#"{"id": "p1", "type": "text", "text": "answer text", "time": {"created": 1764750002500}}"#,
    )
    .unwrap();
    fs::write(
        root.join("opencode/storage/part/m2/p2.json"),
        r#"{"id": "p2", "broken"#,
    )
    .unwrap();

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.metrics.events_total, 1,
        "m2's surviving part must still produce its event"
    );
    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_files_unparseable"),
        Some(&2),
        "message- and part-lane parse failures must accumulate per class: {skips:?}"
    );
    assert!(
        !skips.contains_key("opencode_transcript_files_unreadable"),
        "no read failures happened: {skips:?}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn unreadable_files_and_dirs_disclose_instead_of_masking() {
    let root = std::env::temp_dir().join(format!("at_rm1866_unreadable_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    // Arm 1: an unreadable FILE inside a readable message dir — the dir
    // lists, the read fails; counted as a file-level loss.
    let session_file = write_session_doc(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/message/sess_1")).unwrap();
    write_message(
        &root,
        "sess_1",
        "m1",
        r#"{"id": "m1", "sessionID": "sess_1", "role": "user", "time": {"created": 1764750001000}}"#,
    );
    let blocked = root.join("opencode/storage/message/sess_1/m2.json");
    fs::write(&blocked, r#"{"id": "m2"}"#).unwrap();
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("opencode_transcript_files_unreadable"),
        Some(&1),
        "an unreadable file must be counted and disclosed: {:?}",
        session.metrics.line_skips
    );

    // Arm 2: the message DIR itself unreadable — the session must STILL
    // parse (rc0 path) carrying the dir-level disclosure, never bail into
    // "unsupported session format".
    let root2 = std::env::temp_dir().join(format!("at_rm1866_dir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root2);
    let session_file2 = write_session_doc(&root2, "sess_1");
    let message_dir = root2.join("opencode/storage/message/sess_1");
    fs::create_dir_all(&message_dir).unwrap();
    write_message(
        &root2,
        "sess_1",
        "m1",
        r#"{"id": "m1", "sessionID": "sess_1", "role": "user", "time": {"created": 1764750001000}}"#,
    );
    fs::set_permissions(&message_dir, fs::Permissions::from_mode(0o000)).unwrap();

    let session2 = parse_file(&session_file2).unwrap();
    assert_eq!(
        session2
            .metrics
            .line_skips
            .get("opencode_transcript_dirs_unreadable"),
        Some(&1),
        "an unreadable message dir must be disclosed at dir level: {:?}",
        session2.metrics.line_skips
    );

    // Restore permissions so remove_dir_all works as the unprivileged user.
    fs::set_permissions(&message_dir, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&root2);
}

#[test]
fn missing_dirs_stay_silent() {
    // A message whose PART dir does not exist is the store's own
    // empty-container case (NotFound): not a loss, no disclosure — only
    // the message lane must still produce its events.
    let root = std::env::temp_dir().join(format!("at_rm1866_missing_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/message/sess_1")).unwrap();
    write_message(
        &root,
        "sess_1",
        "m1",
        r#"{"id": "m1", "sessionID": "sess_1", "role": "assistant", "time": {"created": 1764750002000}}"#,
    );
    fs::create_dir_all(root.join("opencode/storage/part/m1")).unwrap();
    fs::write(
        root.join("opencode/storage/part/m1/p1.json"),
        r#"{"id": "p1", "type": "text", "text": "answer text", "time": {"created": 1764750002500}}"#,
    )
    .unwrap();
    // m2 exists with NO part dir at all — NotFound, silent.
    write_message(
        &root,
        "sess_1",
        "m2",
        r#"{"id": "m2", "sessionID": "sess_1", "role": "user", "time": {"created": 1764750003000}}"#,
    );

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.metrics.events_total, 1,
        "m1's event survives; m2 without a part dir contributes nothing"
    );
    assert!(
        session.metrics.line_skips.is_empty(),
        "a missing container dir is not a loss: {:?}",
        session.metrics.line_skips
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn fully_unparseable_corpus_parses_with_disclosure_instead_of_bailing() {
    // rm-1437's relaxation extended: EVERY transcript lost to parse
    // failures still yields a parsed session with the disclosure — the
    // corpus is explained, not foreign.
    let root = std::env::temp_dir().join(format!("at_rm1866_allparse_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/message/sess_1")).unwrap();
    write_message(&root, "sess_1", "m1", r#"{"broken"#);
    write_message(&root, "sess_1", "m2", r#"[not an object]"#);

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("opencode_transcript_files_unparseable"),
        Some(&2),
        "both parse failures must be disclosed: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session.metrics.events_total, 0,
        "no events survive, and that is fine — the session is explained"
    );

    let _ = fs::remove_dir_all(&root);
}

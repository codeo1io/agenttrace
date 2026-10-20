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
//!
//! The cycle-4 intake-hardening arms extend the same admission ledger:
//!  4. fog rm-1870 (id containment): a doc-supplied id that is not a single
//!     path component (absolute, `..`-carrying) must never be joined into the
//!     filesystem — its fan-out is rejected and disclosed, never read;
//!  5. fog rm-1867 (regular-file-only admission): fifos and devices report
//!     st_size 0 and sail under the stat cap — they are rejected with a
//!     counter instead of wedging the read or feeding it without bound;
//!     a symlink to a regular transcript stays admitted;
//!  6. fog rm-1871 (fail-closed stat + disclosure): a stat failure (dangling
//!     symlink, permissions) skips WITH a counter instead of cap-passing via a
//!     zeroed length, and read/decode failures are counted instead of dropped
//!     silently next to the cap counters.

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

// ---------------------------------------------------------------------------
// fog rm-1870 (id containment): the storage fan-out joins doc-supplied ids
// (`message/<sessionID>`, `part/<msgID>`) into the filesystem. `Path::join`
// lets an ABSOLUTE component replace the base and `..` components walk out
// of the storage root, so a hostile id could exfiltrate transcripts from
// entirely outside the scan root (assess F1 PoCs: escaped part text became
// the session title and `--search` hit). A hostile id's fan-out must be
// rejected and disclosed — never read.
// ---------------------------------------------------------------------------

fn write_containment_session(root: &Path, file_stem: &str, id: &str) -> PathBuf {
    let dir = root.join("opencode/storage/session/p1");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{file_stem}.json"));
    fs::write(
        &path,
        format!(
            r#"{{"id": "{id}", "slug": "containment", "projectID": "prj_1", "title": "Containment probe", "model": {{"id": "claude-sonnet-4-5"}}, "time": {{"created": 1764750000000, "updated": 1764750004000}}}}"#
        ),
    )
    .unwrap();
    path
}

#[test]
#[cfg(unix)]
fn absolute_session_id_is_rejected_with_disclosure_and_no_escape() {
    let parent = std::env::temp_dir().join(format!("at_rm1870_abs_{}", std::process::id()));
    let _ = fs::remove_dir_all(&parent);
    let root = parent.join("scan");
    // The escape target lives entirely OUTSIDE the scan root.
    let leak = parent.join("leak-abs");
    fs::create_dir_all(&leak).unwrap();
    let session_file = write_containment_session(&root, "abs", &leak.to_string_lossy());
    fs::write(
        leak.join("m1.json"),
        r#"{"id": "m1", "sessionID": "whatever", "role": "user", "content": "SECRET-PAYLOAD-ABS-LANE", "tokens": {"input": 777777, "output": 777}, "time": {"created": 1764750001000}}"#,
    )
    .unwrap();

    let session = parse_file(&session_file)
        .expect("the session still parses — the hostile fan-out is skipped, not fatal");

    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_fanout_ids_rejected"),
        Some(&1),
        "the absolute id's fan-out must be rejected and disclosed: {skips:?}"
    );
    assert_eq!(
        session.metrics.tokens_input, 0,
        "tokens from outside the storage root must never be folded"
    );
    let _ = fs::remove_dir_all(&parent);
}

#[test]
#[cfg(unix)]
fn dotdot_session_id_cannot_hop_out_of_the_storage_root() {
    let parent = std::env::temp_dir().join(format!("at_rm1870_dd_{}", std::process::id()));
    let _ = fs::remove_dir_all(&parent);
    let root = parent.join("scan");
    // scan/opencode/storage/message/../../../../leak-msg resolves to the
    // sibling of the scan root — four components outside the storage root.
    let leak = parent.join("leak-msg");
    fs::create_dir_all(&leak).unwrap();
    let session_file = write_containment_session(&root, "dd", "../../../../leak-msg");
    fs::write(
        leak.join("m1.json"),
        r#"{"id": "m1", "sessionID": "whatever", "role": "user", "content": "SECRET-PAYLOAD-DD-LANE", "tokens": {"input": 888888, "output": 888}, "time": {"created": 1764750001000}}"#,
    )
    .unwrap();

    let session = parse_file(&session_file).expect("the session still parses with its disclosure");

    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_fanout_ids_rejected"),
        Some(&1),
        "the dotdot id's fan-out must be rejected and disclosed: {skips:?}"
    );
    assert_eq!(
        session.metrics.tokens_input, 0,
        "tokens from outside the storage root must never be folded"
    );
    let _ = fs::remove_dir_all(&parent);
}

#[test]
fn hostile_message_id_cannot_hop_to_parts_outside_the_storage_root() {
    let parent = std::env::temp_dir().join(format!("at_rm1870_hop_{}", std::process::id()));
    let _ = fs::remove_dir_all(&parent);
    let root = parent.join("scan");
    // Safe session id, hostile SECOND hop: the message's own id points the
    // part fan-out outside the storage root (the assess F1 double-hop shape).
    let leak = parent.join("leak-part");
    fs::create_dir_all(&leak).unwrap();
    let session_file = write_containment_session(&root, "hop", "sess_1");
    let message_dir = root.join("opencode/storage/message/sess_1");
    fs::create_dir_all(&message_dir).unwrap();
    // No message-level tokens: the escaped step-finish part would be the
    // ONLY usage source (the rm-619 rescue), so its 999999 input is the
    // exfiltration signal.
    fs::write(
        message_dir.join("m1.json"),
        r#"{"id": "../../../../leak-part", "sessionID": "sess_1", "role": "user", "content": "hi", "time": {"created": 1764750001000}}"#,
    )
    .unwrap();
    fs::write(
        leak.join("p1.json"),
        r#"{"id": "p1", "type": "step-finish", "tokens": {"input": 999999, "output": 999999}, "time": {"created": 1764750001500}}"#,
    )
    .unwrap();

    let session = parse_file(&session_file).expect("the session still parses with its disclosure");

    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_fanout_ids_rejected"),
        Some(&1),
        "the hostile part hop must be rejected and disclosed: {skips:?}"
    );
    assert!(
        session.metrics.tokens_input < 1000,
        "escaped part usage must never fold (got {})",
        session.metrics.tokens_input
    );
    let _ = fs::remove_dir_all(&parent);
}

// ---------------------------------------------------------------------------
// fog rm-1867 (regular-file-only admission) + fog rm-1871 (fail-closed stat
// and skip disclosure): a fifo reports st_size 0 and wedges the read forever;
// a device symlink reports st_size 0 and feeds `read_to_string` without
// bound; a dangling symlink stat fails and — pre-fix — cap-PASSED via the
// zeroed length; undecodable files were dropped silently next to the cap
// counters. Every skip class is now counted on the rm-526 loss channel.
// ---------------------------------------------------------------------------

fn write_intake_session(root: &Path) -> PathBuf {
    let session_dir = root.join("opencode/storage/session/sess_1");
    let message_dir = root.join("opencode/storage/message/sess_1");
    let part_dir = root.join("opencode/storage/part/m1");
    fs::create_dir_all(&session_dir).unwrap();
    fs::create_dir_all(&message_dir).unwrap();
    fs::create_dir_all(&part_dir).unwrap();
    let session_file = session_dir.join("sess_1.json");
    fs::write(
        &session_file,
        r#"{"id": "sess_1", "slug": "intake", "projectID": "prj_1", "title": "Intake probe", "model": {"id": "claude-sonnet-4-5"}, "time": {"created": 1764750000000, "updated": 1764750004000}}"#,
    )
    .unwrap();
    session_file
}

fn write_good_message(message_dir: &Path, stem: &str, id: &str, input: i64, output: i64) {
    fs::write(
        message_dir.join(format!("{stem}.json")),
        format!(
            r#"{{"id": "{id}", "sessionID": "sess_1", "role": "user", "content": "query", "tokens": {{"input": {input}, "output": {output}}}, "time": {{"created": 1764750001000}}}}"#
        ),
    )
    .unwrap();
}

#[test]
fn undecodable_transcript_is_counted_not_silent() {
    let root = std::env::temp_dir().join(format!("at_rm1871_dec_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_intake_session(&root);
    let message_dir = root.join("opencode/storage/message/sess_1");
    write_good_message(&message_dir, "m1", "m1", 100, 10);
    fs::write(message_dir.join("bad.json"), "{ not json at all").unwrap();

    let session = parse_file(&session_file).unwrap();

    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_files_undecodable"),
        Some(&1),
        "the undecodable transcript must be disclosed: {skips:?}"
    );
    assert_eq!(
        session.metrics.tokens_input, 100,
        "the good message still folds"
    );
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn dangling_symlink_stat_fails_closed_with_disclosure() {
    let root = std::env::temp_dir().join(format!("at_rm1871_dang_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_intake_session(&root);
    let message_dir = root.join("opencode/storage/message/sess_1");
    write_good_message(&message_dir, "m1", "m1", 100, 10);
    write_good_message(&message_dir, "m2", "m2", 200, 20);
    std::os::unix::fs::symlink("nowhere-at-all.json", message_dir.join("dang.json")).unwrap();

    let session = parse_file(&session_file).unwrap();

    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_files_unreadable"),
        Some(&1),
        "the dangling symlink must stat-fail CLOSED with a counter: {skips:?}"
    );
    assert_eq!(
        session.metrics.tokens_input, 300,
        "only the good messages fold (the assess corpus-f4 shape: 150600 with zero doctor visibility)"
    );
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn device_symlink_transcript_is_rejected_with_disclosure() {
    let root = std::env::temp_dir().join(format!("at_rm1867_dev_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_intake_session(&root);
    let message_dir = root.join("opencode/storage/message/sess_1");
    write_good_message(&message_dir, "m1", "m1", 100, 10);
    // /dev/null reports st_size 0 — it sailed under the stat cap just like
    // /dev/zero (whose unbounded feed is proven at the CLI PoC level); the
    // device kind must be rejected with a counter.
    std::os::unix::fs::symlink("/dev/null", message_dir.join("z.json")).unwrap();

    let session = parse_file(&session_file).unwrap();

    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_files_nonregular"),
        Some(&1),
        "the device must be rejected with a counter: {skips:?}"
    );
    assert_eq!(
        session.metrics.tokens_input, 100,
        "the good message still folds"
    );
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn fifo_transcript_is_rejected_with_disclosure_instead_of_wedging() {
    let root = std::env::temp_dir().join(format!("at_rm1867_fifo_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_intake_session(&root);
    let message_dir = root.join("opencode/storage/message/sess_1");
    write_good_message(&message_dir, "m1", "m1", 100, 10);
    let fifo = message_dir.join("fifo.json");
    let status = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert!(status.success(), "mkfifo fixture");

    // Pre-fix this WEDGED forever (assess NF1: timeout 8, rc=124, no message,
    // no disclosure) — the test hanging IS the red.
    let session = parse_file(&session_file).unwrap();

    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_files_nonregular"),
        Some(&1),
        "the fifo must be rejected with a counter: {skips:?}"
    );
    assert_eq!(
        session.metrics.tokens_input, 100,
        "the good message still folds"
    );
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn symlink_to_regular_transcript_stays_admitted() {
    let root = std::env::temp_dir().join(format!("at_rm1867_link_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_intake_session(&root);
    let message_dir = root.join("opencode/storage/message/sess_1");
    write_good_message(&message_dir, "m1", "m1", 100, 10);
    // A regular transcript reachable through a symlink is legitimate storage
    // — the file-kind check must follow links, not reject them. The linked
    // doc's id must equal its on-disk stem (link.json → "link"): the landed
    // rm-1870 stem-identity admission applies to every admitted message
    // record regardless of how it is reached, so this fixture pins the
    // file-KIND policy, not an exemption from id admission.
    fs::write(
        root.join("real-message.json"),
        r#"{"id": "link", "sessionID": "sess_1", "role": "assistant", "tokens": {"input": 50, "output": 20}, "time": {"created": 1764750002000}}"#,
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../../../../real-message.json",
        message_dir.join("link.json"),
    )
    .unwrap();

    let session = parse_file(&session_file).unwrap();

    let skips = &session.metrics.line_skips;
    assert!(
        !skips.contains_key("opencode_transcript_files_nonregular")
            && !skips.contains_key("opencode_transcript_files_unreadable"),
        "a symlink to a regular transcript is not a skip class: {skips:?}"
    );
    assert_eq!(
        session.metrics.tokens_input, 150,
        "the linked regular transcript still folds"
    );
    let _ = fs::remove_dir_all(&root);
}

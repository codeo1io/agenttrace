//! rm-1870 + rm-1871: id-identity admission and the session-wide read
//! budget for the opencode transcript lane.
//!
//! rm-1870: doc-supplied ids flow into filesystem joins
//! (`storage/message/<id>` and `storage/part/<id>`), so an absolute id
//! REPLACES the join base and a dot-dot id traverses out of the lane —
//! the assess c0a4e32f F1 escape (arbitrary-file read whose content
//! landed in events). Admission now rejects-with-disclosure any id that
//! is not a single safe path component or fails identity against the
//! file it was discovered under (stem == id — the storage layout rule
//! ccusage's loader also relies on). A rejected record must leave the
//! session exactly as if its file never existed, plus disclosure.
//!
//! rm-1871: the per-file stat cap (rm-1437) bounded ONE file, not the
//! scan: N under-cap files admitted N×cap bytes (assess F2 measured RSS
//! 15.1→32.4→51.3 MiB across corpus scaling) and the stat-before-read
//! gate races a live writer (assess F4). The budget is now enforced on
//! the READ side (bounded reader, never parse a truncated file) across
//! every file of one session scan — bytes AND records — with the excess
//! disclosed through the same `line_skips` channel.

use std::fs;
use std::path::{Path, PathBuf};

use agenttrace_core::{parse_file, search_sessions, Session};

/// rm-1437 per-file cap (kept); rm-1871 session totals.
const CAP_BYTES: usize = 10 * 1024 * 1024;
const TOTAL_CAP_BYTES: usize = 64 * 1024 * 1024;
const TOTAL_RECORD_CAP: usize = 8192;

fn session_doc(root: &Path, session_id: &str) -> PathBuf {
    root.join(format!(
        "opencode/storage/session/{session_id}/{session_id}.json"
    ))
}

fn write_session_doc(root: &Path, file_id: &str, doc_id: &str, title: &str) -> PathBuf {
    let path = session_doc(root, file_id);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        format!(
            r#"{{"id": "{doc_id}", "slug": "admission", "projectID": "prj_1", "title": "{title}", "time": {{"created": 1764750000000, "updated": 1764750004000}}}}"#
        ),
    )
    .unwrap();
    path
}

fn write_message(root: &Path, session_id: &str, msg_id: &str, text: &str) {
    let dir = root.join(format!("opencode/storage/message/{session_id}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(format!("{msg_id}.json")),
        format!(
            r#"{{"id": "{msg_id}", "sessionID": "{session_id}", "role": "user", "time": {{"created": 1764750001000}}}}"#
        ),
    )
    .unwrap();
    write_part(root, msg_id, "p1", text);
}

/// `file_stem` is the on-disk name; `doc_id` is the id inside the JSON.
fn write_message_with_id(root: &Path, session_id: &str, file_stem: &str, doc_id: &str) {
    let dir = root.join(format!("opencode/storage/message/{session_id}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(format!("{file_stem}.json")),
        format!(
            r#"{{"id": "{doc_id}", "sessionID": "{session_id}", "role": "user", "time": {{"created": 1764750001000}}}}"#
        ),
    )
    .unwrap();
}

fn write_part(root: &Path, msg_id: &str, part_id: &str, text: &str) {
    let dir = root.join(format!("opencode/storage/part/{msg_id}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(format!("{part_id}.json")),
        format!(
            r#"{{"id": "{part_id}", "type": "text", "text": "{text}", "time": {{"created": 1764750001500}}}}"#
        ),
    )
    .unwrap();
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

fn fresh_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("at_rm1870_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

#[test]
fn absolute_session_id_escape_is_rejected_and_disclosed() {
    // F1 PoC shape: the session doc's id is an absolute path, so
    // join("message").join(id) leaves the storage root entirely and
    // reads a foreign "message" transcript.
    let leak = fresh_root("leak1");
    fs::create_dir_all(leak.join("message")).unwrap();
    fs::write(
        leak.join("message/m1.json"),
        r#"{"id": "m1", "sessionID": "whatever", "role": "user", "time": {"created": 1764750001000}}"#,
    )
    .unwrap();
    fs::create_dir_all(leak.join("part/m1")).unwrap();
    fs::write(
        leak.join("part/m1/p1.json"),
        r#"{"id": "p1", "type": "text", "text": "TOPSECRET ABSOLUTE SESSION LEAK", "time": {"created": 1764750001500}}"#,
    )
    .unwrap();
    let leak_abs = leak.join("message");

    let hostile = fresh_root("abs1");
    let session_file = write_session_doc(&hostile, "sess_1", &leak_abs.to_string_lossy(), "host");
    // A session-level rejection skips the whole message lane, so the
    // honest twin is the SAME class of rejected id pointing at a path
    // with no content: the hostile must be indistinguishable from it
    // (the leak transcript contributed nothing beyond its disclosure).
    // A clean-id twin is impossible here — a valid id with zero messages
    // bails "no messages found", by design.
    let twin = fresh_root("abs1_twin");
    let twin_file = write_session_doc(&twin, "sess_1", "/nonexistent_at_all", "host");

    let hostile_session = parse_file(&session_file).expect("reject-and-disclose, never bail");
    let twin_session = parse_file(&twin_file).unwrap();

    assert_eq!(
        fingerprint(&hostile_session),
        fingerprint(&twin_session),
        "the escaped transcript must leave no trace of its events"
    );
    assert_eq!(
        hostile_session
            .metrics
            .line_skips
            .get("opencode_session_id_rejected"),
        Some(&1),
        "the rejection must be disclosed: {:?}",
        hostile_session.metrics.line_skips
    );
    // rm-1870 acceptance (2): the escaped transcript's bytes must appear
    // in NEITHER the session title NOR --search output — the display name
    // derives from the first user event (session_display_name), which at
    // base was exactly the exfil channel the assess PoC demonstrated.
    assert!(
        !hostile_session.name.to_lowercase().contains("topsecret"),
        "session title must carry no exfiltrated bytes: {:?}",
        hostile_session.name
    );
    assert!(
        search_sessions(std::slice::from_ref(&hostile_session), "TOPSECRET", 10).is_empty(),
        "--search output must not surface the escaped transcript"
    );
    // The leak dir still exists on disk — proving the read never happened
    // is the fingerprint equality above, not deletion.
    assert!(leak.join("message/m1.json").exists());
    let _ = fs::remove_dir_all(&leak);
    let _ = fs::remove_dir_all(&hostile);
    let _ = fs::remove_dir_all(&twin);
}

#[test]
fn absolute_message_id_escape_is_rejected_and_disclosed() {
    // F1 double-hop shape: a legit session id, but a message doc whose
    // id is absolute — join("part").join(id) escapes to read foreign
    // "parts" (assess chained exactly this against the session corpus).
    let leak = fresh_root("leak2");
    fs::create_dir_all(leak.join("part/m1")).unwrap();
    fs::write(
        leak.join("part/m1/p1.json"),
        r#"{"id": "p1", "type": "text", "text": "TOPSECRET ABSOLUTE MESSAGE LEAK", "time": {"created": 1764750001500}}"#,
    )
    .unwrap();
    let leak_abs = leak.join("part").join("m1");

    let hostile = fresh_root("abs2");
    let session_file = write_session_doc(&hostile, "sess_1", "sess_1", "host");
    write_message_with_id(&hostile, "sess_1", "m1", &leak_abs.to_string_lossy());
    // A surviving sibling proves the rejection is per-record, not fatal.
    write_message(&hostile, "sess_1", "m2", "clean sibling text");

    let twin = fresh_root("abs2_twin");
    let twin_file = write_session_doc(&twin, "sess_1", "sess_1", "host");
    write_message(&twin, "sess_1", "m2", "clean sibling text");

    let hostile_session = parse_file(&session_file).expect("reject-and-disclose, never bail");
    let twin_session = parse_file(&twin_file).unwrap();

    assert_eq!(
        fingerprint(&hostile_session),
        fingerprint(&twin_session),
        "the escaped part transcript must leave no event trace"
    );
    assert_eq!(
        hostile_session
            .metrics
            .line_skips
            .get("opencode_message_id_rejected"),
        Some(&1),
        "the message-id rejection must be disclosed: {:?}",
        hostile_session.metrics.line_skips
    );
    assert!(
        hostile_session.metrics.user_messages > 0,
        "the clean sibling must survive"
    );
    assert!(
        !hostile_session.name.to_lowercase().contains("topsecret"),
        "session title must carry no exfiltrated bytes: {:?}",
        hostile_session.name
    );
    assert!(
        search_sessions(std::slice::from_ref(&hostile_session), "TOPSECRET", 10).is_empty(),
        "--search output must not surface the escaped part transcript"
    );
    assert!(leak.join("part/m1/p1.json").exists());
    let _ = fs::remove_dir_all(&leak);
    let _ = fs::remove_dir_all(&hostile);
    let _ = fs::remove_dir_all(&twin);
}

#[test]
fn dot_dot_and_stem_mismatch_ids_are_rejected_and_disclosed() {
    // The dot-dot variant (traversal inside the storage tree) and the
    // identity variant (doc id != file stem — the layout rule the join
    // relies on) are both rejected, disclosed, and non-fatal.
    let root = fresh_root("dots");
    let session_file = write_session_doc(&root, "sess_1", "sess_1", "host");
    // m1: dot-dot id → join("part").join("../session/sess_1") would read
    // the session doc itself as a part transcript.
    write_message_with_id(&root, "sess_1", "m1", "../session/sess_1");
    // m2: stem mismatch → id points at a different message's part dir.
    write_part(&root, "other", "p9", "TOPSECRET MISMATCH LEAK");
    write_message_with_id(&root, "sess_1", "m2", "other");
    // m3: clean record, survives.
    write_message(&root, "sess_1", "m3", "clean text");

    let twin = fresh_root("dots_twin");
    let twin_file = write_session_doc(&twin, "sess_1", "sess_1", "host");
    write_message(&twin, "sess_1", "m3", "clean text");

    let session = parse_file(&session_file).expect("reject-and-disclose, never bail");
    let twin_session = parse_file(&twin_file).unwrap();

    assert_eq!(fingerprint(&session), fingerprint(&twin_session));
    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_message_id_rejected"),
        Some(&2),
        "both bad ids must be disclosed: {skips:?}"
    );
    assert!(
        !session.name.to_lowercase().contains("topsecret"),
        "session title must carry no exfiltrated bytes: {:?}",
        session.name
    );
    assert!(
        search_sessions(std::slice::from_ref(&session), "TOPSECRET", 10).is_empty(),
        "--search output must not surface the mismatched-id leak"
    );
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&twin);
}

#[test]
fn session_id_stem_mismatch_is_rejected_and_disclosed() {
    // The session doc lives at session/sess_1/sess_1.json but claims
    // id "other_sess" — the message join would read another session's
    // (or an attacker's) message directory.
    let root = fresh_root("stem");
    let session_file = write_session_doc(&root, "sess_1", "other_sess", "host");
    // The directory the hostile id points at — real content that must
    // not be read.
    write_message(&root, "other_sess", "m1", "TOPSECRET STEM LEAK");

    let twin = fresh_root("stem_twin");
    // Twin: the same id/stem mismatch, with no foreign session to read —
    // the hostile must be indistinguishable from it (the other session's
    // message directory contributed nothing beyond its disclosure).
    let twin_file = write_session_doc(&twin, "sess_1", "other_sess", "host");

    let session = parse_file(&session_file).expect("reject-and-disclose, never bail");
    let twin_session = parse_file(&twin_file).unwrap();

    assert_eq!(fingerprint(&session), fingerprint(&twin_session));
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("opencode_session_id_rejected"),
        Some(&1),
        "{:?}",
        session.metrics.line_skips
    );
    assert!(
        !session.name.to_lowercase().contains("topsecret"),
        "session title must carry no exfiltrated bytes: {:?}",
        session.name
    );
    assert!(
        search_sessions(std::slice::from_ref(&session), "TOPSECRET", 10).is_empty(),
        "--search output must not surface the cross-session read"
    );
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&twin);
}

#[test]
fn total_byte_budget_bounds_the_whole_session_scan() {
    // rm-1871: 7 files of ~9 MiB sit under the per-file cap and under
    // the 64 MiB total; the 8th exhausts the budget mid-scan and must
    // be skipped-and-disclosed — identical to a corpus where it never
    // existed. Each message also carries a small text part so admitted
    // messages yield REAL user events (a padding-only message doc
    // yields none, which would make the fingerprint equality vacuous).
    let build = |root: &Path, count: usize| {
        let session_file = write_session_doc(root, "sess_1", "sess_1", "budget");
        for i in 0..count {
            let dir = root.join("opencode/storage/message/sess_1");
            fs::create_dir_all(&dir).unwrap();
            let padding = "A".repeat(9 * 1024 * 1024);
            fs::write(
                dir.join(format!("m{i}.json")),
                format!(
                    r#"{{"id": "m{i}", "sessionID": "sess_1", "role": "user", "time": {{"created": {}}}, "padding": "{padding}"}}"#,
                    1764750001000i64 + i as i64
                ),
            )
            .unwrap();
            let pdir = root.join(format!("opencode/storage/part/m{i}"));
            fs::create_dir_all(&pdir).unwrap();
            fs::write(
                pdir.join("p1.json"),
                format!(
                    r#"{{"id": "p1", "type": "text", "text": "budget payload {i}", "time": {{"created": {}}}}}"#,
                    1764750001500i64 + i as i64
                ),
            )
            .unwrap();
        }
        session_file
    };

    let capped = fresh_root("budget_over");
    let capped_file = build(&capped, 8);
    let twin = fresh_root("budget_twin");
    let twin_file = build(&twin, 7);

    let capped_session = parse_file(&capped_file).expect("budget cap is disclosed, not fatal");
    let twin_session = parse_file(&twin_file).unwrap();

    assert_eq!(
        fingerprint(&capped_session),
        fingerprint(&twin_session),
        "the over-budget file must leave no trace beyond its disclosure"
    );
    let skips = &capped_session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_budget_files_capped"),
        Some(&1),
        "{skips:?}"
    );
    assert!(
        skips
            .get("opencode_transcript_budget_bytes_capped")
            .copied()
            .unwrap_or(0)
            > 9 * 1024 * 1024,
        "the disclosed excess must be honest: {skips:?}"
    );
    let twin_skips = &twin_session.metrics.line_skips;
    assert!(
        !twin_skips.contains_key("opencode_transcript_budget_files_capped"),
        "a 63 MiB corpus (7 × 9) must stay under the 64 MiB total: {twin_skips:?}"
    );
    let _ = fs::remove_dir_all(&capped);
    let _ = fs::remove_dir_all(&twin);
}

#[test]
fn total_record_budget_bounds_the_whole_session_scan() {
    // rm-1871 record arm: 5000 messages × 1 part each = 10000 records
    // against the 8192-record budget. The message lane admits all 5000
    // (all under every byte bound), leaving 3192 for the part lane:
    // exactly 3192 text events may survive and the remaining 1808 part
    // reads must be skipped-and-disclosed.
    let root = fresh_root("records");
    let session_file = write_session_doc(&root, "sess_1", "sess_1", "budget");
    for i in 0..5000 {
        let dir = root.join("opencode/storage/message/sess_1");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("m{i:05}.json")),
            format!(
                r#"{{"id": "m{i:05}", "sessionID": "sess_1", "role": "user", "time": {{"created": {}}}}}"#,
                1764750001000i64 + i
            ),
        )
        .unwrap();
        let pdir = root.join(format!("opencode/storage/part/m{i:05}"));
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("p1.json"),
            format!(
                r#"{{"id": "p1", "type": "text", "text": "msg {i}", "time": {{"created": {}}}}}"#,
                1764750002000i64 + i
            ),
        )
        .unwrap();
    }

    let session = parse_file(&session_file).expect("record cap is disclosed, not fatal");

    assert_eq!(
        session.metrics.user_messages,
        TOTAL_RECORD_CAP - 5000,
        "message lane (5000) + part lane (3192) must hit the 8192-record budget exactly"
    );
    let skips = &session.metrics.line_skips;
    assert_eq!(
        skips.get("opencode_transcript_budget_files_capped"),
        Some(&(5000 - (TOTAL_RECORD_CAP - 5000))),
        "the 1808 post-budget part files must be disclosed: {skips:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn under_budget_corpus_discloses_nothing_new() {
    // The rm-1437 pins already cover the per-file arms; this pins that
    // an ordinary multi-record session discloses NEITHER budget NOR id
    // keys, so the new gates stay silent on clean storage.
    let root = fresh_root("clean");
    let session_file = write_session_doc(&root, "sess_1", "sess_1", "clean");
    write_message(&root, "sess_1", "m1", "hello");
    write_message(&root, "sess_1", "m2", "world");
    write_part(&root, "m2", "p2", "again");

    let session = parse_file(&session_file).unwrap();

    for key in [
        "opencode_transcript_budget_files_capped",
        "opencode_transcript_budget_bytes_capped",
        "opencode_session_id_rejected",
        "opencode_message_id_rejected",
        "opencode_part_id_rejected",
    ] {
        assert!(
            !session.metrics.line_skips.contains_key(key),
            "clean corpus must not disclose {key}: {:?}",
            session.metrics.line_skips
        );
    }
    assert_eq!(session.metrics.user_messages, 3);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn search_metadata_path_is_live_on_clean_opencode_storage() {
    // Control for the no-exfil assertions above: a needle that
    // LEGITIMATELY enters session metadata (the display name derives
    // from the first user event) must surface through search_sessions,
    // so the empty results in the escape tests mean "the leak was
    // rejected", never "search is dead on this corpus family".
    let root = fresh_root("ctrl");
    let session_file = write_session_doc(&root, "sess_1", "sess_1", "clean");
    write_message(&root, "sess_1", "m1", "CONTROLNEEDLE first user message");

    let session = parse_file(&session_file).unwrap();
    assert!(
        session.name.contains("CONTROLNEEDLE"),
        "control needle must reach the display name: {:?}",
        session.name
    );
    let hits = search_sessions(std::slice::from_ref(&session), "controlneedle", 10);
    assert_eq!(hits.len(), 1, "metadata needle must surface: {hits:?}");
    let _ = fs::remove_dir_all(&root);
}

/// Compile-time reminder of the constants the tests above encode.
const _: () = {
    assert!(TOTAL_CAP_BYTES > CAP_BYTES);
    assert!(TOTAL_RECORD_CAP > 4096);
};

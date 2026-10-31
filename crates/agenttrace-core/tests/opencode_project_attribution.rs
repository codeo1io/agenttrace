//! opencode project attribution contract. The hostile-project-id
//! escape arm pins unix "/..."-is-absolute semantics (windows treats
//! drive-less paths as relative — CI run 41201929804). Unix-only.
#![cfg(unix)]

//! rm-1865: opencode session docs carry `directory` (and `projectID`) that
//! the parser never decoded — every opencode session rolled up as
//! by_project "unknown", even docs with an explicit directory (the in-tree
//! project_alpha fixture carries `"directory": "/repo"`).
//!
//! The session's working directory is decoded from the doc's `directory`,
//! falling back to the project doc referenced by `projectID`
//! (`storage/project/<id>.json`, v1 store layout), and rides the meta
//! event's cwd so `session_from_events` attributes the session to its
//! project (`resolve_project` prefers cwd). When neither is present the
//! empty cwd keeps the disclosed unknown-project fallback.

use std::fs;
use std::path::{Path, PathBuf};

use agenttrace_core::parse_file;

fn write_session_doc(root: &Path, session_id: &str, extra: &str) -> PathBuf {
    fs::create_dir_all(root.join("opencode/storage/session").join(session_id)).unwrap();
    let path = root.join(format!(
        "opencode/storage/session/{session_id}/{session_id}.json"
    ));
    fs::write(&path, extra).unwrap();
    path
}

fn write_message(root: &Path, session_id: &str) {
    fs::create_dir_all(root.join(format!("opencode/storage/message/{session_id}"))).unwrap();
    fs::write(
        root.join(format!("opencode/storage/message/{session_id}/m1.json")),
        r#"{"id": "m1", "sessionID": "sess_1", "role": "assistant", "time": {"created": 1764750002000}}"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("opencode/storage/part/m1")).unwrap();
    fs::write(
        root.join("opencode/storage/part/m1/p1.json"),
        r#"{"id": "p1", "type": "text", "text": "answer text", "time": {"created": 1764750002500}}"#,
    )
    .unwrap();
}

#[test]
fn session_doc_directory_feeds_cwd_attribution() {
    let root = std::env::temp_dir().join(format!("at_rm1865_docdir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "attribution",
  "projectID": "prj_alpha",
  "title": "Attribution",
  "directory": "/home/dev/project-alpha",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_1");

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "/home/dev/project-alpha",
        "the doc's directory must ride the session's cwd so by_project resolves"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn project_doc_fallback_feeds_cwd_attribution() {
    // The doc omits `directory`, but its projectID references
    // storage/project/prj_beta.json — the project doc's directory is the
    // store's canonical home for that mapping.
    let root = std::env::temp_dir().join(format!("at_rm1865_projdoc_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "attribution",
  "projectID": "prj_beta",
  "title": "Attribution",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/project")).unwrap();
    fs::write(
        root.join("opencode/storage/project/prj_beta.json"),
        r#"{"id": "prj_beta", "directory": "/srv/work/beta", "time": {"created": 1764750000000}}"#,
    )
    .unwrap();

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "/srv/work/beta",
        "the referenced project doc's directory must serve as the session cwd"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn missing_directory_keeps_the_disclosed_unknown_fallback() {
    // Neither a doc directory nor a resolvable project doc: cwd stays
    // empty and the session still parses (by_project keeps its existing
    // unknown fallback — honest, not invented).
    let root = std::env::temp_dir().join(format!("at_rm1865_none_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "no-directory",
  "projectID": "prj_missing",
  "title": "No directory",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_1");
    // No storage/project/prj_missing.json on purpose.

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "an unresolvable project must degrade to the disclosed unknown fallback"
    );
    assert!(session.metrics.events_total > 0);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn hostile_project_ids_cannot_escape_the_store() {
    // Review F1 (2026-10-16): projectID is joined into a filesystem
    // path, and `Path::join` is not a confinement — an ABSOLUTE id
    // replaces the base path outright (the joined path becomes
    // <payload>/evil.json outside the store entirely) and a `../` id
    // walks out of `storage/project`. Both spellings used to read an
    // out-of-store doc and import its `directory` as this session's cwd
    // (live-proven pre-fix); both must now degrade to the disclosed
    // unknown fallback even with a payload doc sitting at the escaped
    // location.
    let tmp = std::env::temp_dir();
    let root = tmp.join(format!("at_rm1865f1_hostile_{}", std::process::id()));
    let payload = tmp.join(format!("at_rm1865f1_payload_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&payload);
    fs::create_dir_all(&payload).unwrap();
    fs::write(
        payload.join("evil.json"),
        r#"{"directory": "/zzz-pwned-marker/absolute"}"#,
    )
    .unwrap();

    // Arm 1: absolute id — pre-fix the joined path replaced the store
    // base and read the payload doc.
    let absolute_id = payload.join("evil").to_string_lossy().to_string();
    let session_file = write_session_doc(
        &root,
        "sess_1",
        &format!(
            r#"{{"id": "sess_1", "slug": "hostile", "projectID": "{absolute_id}", "title": "Hostile", "time": {{"created": 1764750000000, "updated": 1764750004000}}}}"#
        ),
    );
    write_message(&root, "sess_1");
    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "an absolute projectID must not read outside the store: {}",
        session.cwd
    );

    // Arm 2: traversal id — storage/project/../../../escape/evil.json
    // resolves inside root but OUTSIDE the opencode store (root/opencode
    // is the store base); pre-fix it read the payload doc too.
    fs::create_dir_all(root.join("escape")).unwrap();
    fs::write(
        root.join("escape/evil.json"),
        r#"{"directory": "/zzz-pwned-marker/traversal"}"#,
    )
    .unwrap();
    let session_file = write_session_doc(
        &root,
        "sess_2",
        r#"{
  "id": "sess_2",
  "slug": "hostile",
  "projectID": "../../../escape/evil",
  "title": "Hostile",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_2");
    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "a ../-spelled projectID must not walk out of the store: {}",
        session.cwd
    );
    assert!(
        session.metrics.events_total > 0,
        "the session still parses — a rejected id is the disclosed unknown, not an error"
    );

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&payload);
}

#[test]
fn oversize_project_doc_is_capped_before_read() {
    // Review F1 (rm-1437 discipline): the project doc is stat-capped
    // before the read — one byte over the transcript cap degrades to
    // the disclosed unknown fallback instead of admitting the file
    // into memory. The doc is VALID JSON whose directory sits at the
    // front, so only the cap can explain an empty cwd.
    const CAP_BYTES: usize = 10 * 1024 * 1024;
    let root = std::env::temp_dir().join(format!("at_rm1865f1_cap_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "cap",
  "projectID": "prj_huge",
  "title": "Cap",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/project")).unwrap();
    let mut body = String::from(r#"{"directory": "/zzz-oversize-marker", "pad": ""#);
    body.push_str(&"x".repeat(CAP_BYTES));
    body.push_str("\"}");
    fs::write(root.join("opencode/storage/project/prj_huge.json"), body).unwrap();

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "an over-cap project doc must be skipped before the read: {}",
        session.cwd
    );
    assert!(session.metrics.events_total > 0);

    let _ = fs::remove_dir_all(&root);
}

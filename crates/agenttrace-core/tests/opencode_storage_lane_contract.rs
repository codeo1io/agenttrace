//! rm-1011 contract: the opencode storage lane owns every session-shaped
//! file under `<root>/opencode/storage/session/<project>/<id>.json`, and
//! when the doc is malformed the lane's OWN diagnostic must surface —
//! `opencode: missing session id`, a missing `projectID`, or `opencode: no
//! messages found for session <id>` — instead of the refusal being
//! flattened through `.ok()` into the generic `unsupported session format`
//! mask at the parser fallthrough. Pass-through is pinned in both
//! directions: the benign storage corpus still parses through the storage
//! lane with the same attribution, and a foreign format at a non-storage
//! path still gets the generic mask (that path-shape is not ours to
//! diagnose).

use agenttrace_core::parse_file;
use std::fs;
use std::path::{Path, PathBuf};

/// Canonical benign corpus (same fixture lane as journal_truth_batch):
/// testdata/opencode/storage/session/project_alpha/ses_abc.json plus its
/// message records under testdata/opencode/storage/message/ses_abc/.
fn storage_fixture_session() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/opencode/storage/session/project_alpha/ses_abc.json")
}

fn temp_root(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn write_session_doc(root: &Path, project: &str, file: &str, doc: &str) -> PathBuf {
    let dir = root
        .join("opencode")
        .join("storage")
        .join("session")
        .join(project);
    fs::create_dir_all(&dir).expect("create session dir");
    let path = dir.join(file);
    fs::write(&path, doc).expect("write session doc");
    path
}

/// Mirrors the testdata session doc shape (ses_abc.json) with the two
/// ownership markers (`id`, `projectID`) individually omissible.
fn session_doc_json(id: Option<&str>, project_id: Option<&str>) -> String {
    let mut doc = serde_json::json!({
        "slug": "lane-contract",
        "directory": "/repo",
        "title": "Lane contract",
        "version": "0.4.0",
        "model": { "id": "claude-sonnet-4", "providerID": "anthropic" },
        "time": { "created": 1764750000000i64, "updated": 1764750004000i64 },
    });
    if let Some(id) = id {
        doc["id"] = serde_json::json!(id);
    }
    if let Some(project_id) = project_id {
        doc["projectID"] = serde_json::json!(project_id);
    }
    serde_json::to_string_pretty(&doc).expect("serialize session doc")
}

#[test]
fn benign_storage_session_still_parses_through_the_storage_lane() {
    let parsed = parse_file(&storage_fixture_session())
        .expect("benign opencode storage session must still parse");
    assert_eq!(parsed.metrics.source_tool, "opencode");
    assert_eq!(parsed.metrics.model_used, "claude-sonnet-4");
    assert_eq!(
        parsed.metrics.tokens_input, 42,
        "message tokens lift unchanged"
    );
    assert_eq!(parsed.metrics.tokens_output, 17);
}

#[test]
fn storage_doc_missing_id_reports_the_storage_error_not_the_format_mask() {
    let root = temp_root("at-storage-lane-no-id");
    let path = write_session_doc(
        &root,
        "p_missing_id",
        "no_id.json",
        &session_doc_json(None, Some("p_missing_id")),
    );
    let err = parse_file(&path).expect_err("missing id must be refused");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("opencode: missing session id"),
        "the storage lane's own diagnostic must surface, got: {msg}"
    );
    assert!(
        !msg.contains("unsupported session format"),
        "the generic format mask must not swallow the storage cause, got: {msg}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn storage_doc_missing_project_id_is_diagnosed_not_masked() {
    let root = temp_root("at-storage-lane-no-project");
    let path = write_session_doc(
        &root,
        "p_no_project",
        "ses_noproject.json",
        &session_doc_json(Some("ses_noproject"), None),
    );
    let err = parse_file(&path).expect_err("missing projectID must be refused");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("opencode:") && msg.contains("ses_noproject") && msg.contains("projectID"),
        "the diagnostic must name the storage lane, the session, and the missing marker, got: {msg}"
    );
    assert!(
        !msg.contains("unsupported session format"),
        "the generic format mask must not swallow the storage cause, got: {msg}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn storage_session_without_message_records_discloses_the_real_cause() {
    let root = temp_root("at-storage-lane-orphan");
    // Well-formed session doc (id + projectID), but the message directory
    // for the session id does not exist — the lane parser's own bail names
    // the orphaned session; the old shape masked it as a format problem.
    let path = write_session_doc(
        &root,
        "p_orphan",
        "ses_orphan.json",
        &session_doc_json(Some("ses_orphan"), Some("p_orphan")),
    );
    let err = parse_file(&path).expect_err("orphan session must be refused");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("opencode: no messages found for session ses_orphan"),
        "the orphan diagnosis must name the session, got: {msg}"
    );
    assert!(
        !msg.contains("unsupported session format"),
        "the generic format mask must not swallow the storage cause, got: {msg}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn unknown_format_outside_the_storage_namespace_keeps_the_generic_mask() {
    let root = temp_root("at-storage-lane-foreign");
    let dir = root.join("plain");
    fs::create_dir_all(&dir).expect("create plain dir");
    let path = dir.join("weird.json");
    fs::write(&path, "{\"hello\": \"world\"}").expect("write foreign doc");
    let err = parse_file(&path).expect_err("foreign doc must be refused");
    let msg = format!("{err:#}");
    assert!(
        msg.contains(&format!("unsupported session format: {}", path.display())),
        "a foreign format at a non-storage path keeps the generic mask, got: {msg}"
    );
    let _ = fs::remove_dir_all(root);
}

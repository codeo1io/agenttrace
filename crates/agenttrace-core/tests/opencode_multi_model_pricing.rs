//! rm-663 (opencode storage lane) + rm-020 golden: an opencode storage
//! session whose two assistant messages name two different models,
//! each carrying its own `tokens` map.
//!
//! Pre-fix: the storage parser froze the FIRST message's model
//! (`if model == "unknown"`), accumulated every message's tokens into
//! one usage map, and stamped the single aggregate meta event with the
//! frozen model — so the whole session priced at the first model:
//! (1000+2000)/1e6*$3 + (100+200)/1e6*$15 = $0.0135.
//!
//! Post-fix: per-message model attribution, per-model usage buckets,
//! one meta event per model — sonnet 1000/1e6*$3 + 100/1e6*$15 =
//! $0.0045, opus 2000/1e6*$15 + 200/1e6*$75 = $0.045, total $0.0495,
//! model_used "multiple", and the rm-020 per-model ledger carries one
//! row per model.

use agenttrace_core::parse_file;
use std::fs;
use std::path::{Path, PathBuf};

fn write_json(path: &Path, doc: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture dirs");
    }
    fs::write(path, doc).expect("write fixture json");
}

/// Builds the opencode storage tree under a unique temp root and
/// returns the session json path (the parse entry point).
fn opencode_two_model_fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agenttrace-rm020-opencode-{}-{tag}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let storage = root.join("opencode").join("storage");
    write_json(
        &storage.join("session").join("p1").join("multi.json"),
        r#"{"id":"multi","projectID":"p1","time":{"created":1700000000000}}"#,
    );
    write_json(
        &storage.join("message").join("multi").join("1.json"),
        r#"{"id":"msg_1","role":"assistant","modelID":"claude-sonnet-4-5","time":{"created":1700000001000},"tokens":{"input":1000,"output":100}}"#,
    );
    write_json(
        &storage.join("message").join("multi").join("2.json"),
        r#"{"id":"msg_2","role":"assistant","modelID":"claude-opus-4-1","time":{"created":1700000002000},"tokens":{"input":2000,"output":200}}"#,
    );
    // Text parts keep the session body non-empty (a body-less session
    // bails before pricing) and carry the per-message model stamp.
    write_json(
        &storage.join("part").join("msg_1").join("1.json"),
        r#"{"id":"p1","type":"text","text":"light answer","time":{"created":1700000001500}}"#,
    );
    write_json(
        &storage.join("part").join("msg_2").join("1.json"),
        r#"{"id":"p2","type":"text","text":"heavy answer","time":{"created":1700000002500}}"#,
    );
    storage.join("session").join("p1").join("multi.json")
}

#[test]
fn opencode_two_model_session_prices_each_message_at_its_own_model() {
    let session_path = opencode_two_model_fixture("price");
    let session = parse_file(&session_path).expect("fixture parses");
    let root = session_path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("fixture root");
    let _ = fs::remove_dir_all(root);

    assert_eq!(
        session.metrics.tokens_input, 3000,
        "session token totals are unchanged by model mix"
    );
    assert_eq!(session.metrics.tokens_output, 300);
    // Frozen-first regression gate: $0.0135 is the pre-fix number.
    assert_eq!(session.metrics.cost_estimated, 0.0495);
    assert_eq!(session.metrics.model_used, "multiple");

    let ledger = &session.metrics.model_ledger;
    assert_eq!(ledger.len(), 2, "one row per model: {ledger:?}");
    let sonnet = ledger
        .iter()
        .find(|row| row.model == "claude-sonnet-4-5")
        .expect("sonnet row");
    assert_eq!((sonnet.tokens_input, sonnet.tokens_output), (1000, 100));
    assert_eq!(sonnet.cost_usd, 0.0045);
    let opus = ledger
        .iter()
        .find(|row| row.model == "claude-opus-4-1")
        .expect("opus row");
    assert_eq!((opus.tokens_input, opus.tokens_output), (2000, 200));
    assert_eq!(opus.cost_usd, 0.045);
    let sum: f64 = ledger.iter().map(|row| row.cost_usd).sum();
    assert!((sum - session.metrics.cost_estimated).abs() < 1e-9);
}

/// rm-663 body check: the second message's text event must carry the
/// SECOND model's name, not the frozen first one.
#[test]
fn opencode_message_events_carry_their_own_model() {
    let session_path = opencode_two_model_fixture("models");
    let raw = fs::read_to_string(&session_path).expect("session json");
    // parse_raw_session reaches the same storage lane from the session
    // doc text.
    let session =
        agenttrace_core::parse_raw_session("multi", &session_path.to_string_lossy(), &raw)
            .expect("fixture parses");
    let root = session_path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("fixture root");
    let _ = fs::remove_dir_all(root);

    assert_eq!(session.metrics.source_tool, "opencode");
    let text = agenttrace_core::report_text(&session);
    assert!(
        text.contains("claude-opus-4-1"),
        "the second model must be visible in the report, not frozen away:\n{text}"
    );
}

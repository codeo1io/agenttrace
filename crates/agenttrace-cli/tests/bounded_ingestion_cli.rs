//! rm-700 bounded ingestion, CLI end-to-end: an over-cap session file is
//! skipped before it is read and the skip is disclosed in the diagnostics
//! lane — a partial corpus still succeeds with a visible counter, and an
//! all-oversize corpus fails with the truthful reason (never a hang,
//! never a silent drop).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn unique_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agenttrace-rm700-cli-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(root.join("corpus")).expect("corpus dir");
    fs::create_dir_all(root.join("cache/sess")).expect("cache dir");
    root
}

/// Claude-grammar journal, ~190 bytes total — parses under the 200-byte
/// cap used by these tests.
fn small_journal() -> String {
    concat!(
        "{\"role\":\"session_meta\",\"timestamp\":\"2026-10-10T10:00:00Z\",\"ModelUsed\":\"claude-sonnet-4\"}\n",
        "{\"role\":\"assistant\",\"content\":\"ok\",\"timestamp\":\"2026-10-10T10:00:01Z\",\"ModelUsed\":\"claude-sonnet-4\"}\n"
    )
    .to_string()
}

/// Opencode-grammar journal, ~450 bytes — exceeds the 200-byte cap.
fn big_journal() -> String {
    concat!(
        "{\"id\":\"h1\",\"timestamp\":\"2026-10-10T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp/rm700\"}\n",
        "{\"id\":\"e1\",\"parentId\":\"h1\",\"timestamp\":\"2026-10-10T10:00:01.000Z\",\"type\":\"message\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"ok\"}],\"model\":\"claude-sonnet-4\",\"usage\":{\"input\":100,\"output\":100,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":200}}}\n"
    )
    .to_string()
}

fn agenttrace(root: &Path, corpus: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
    command
        .args(["-d", corpus, "--diagnostics", "-f", "json"])
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", root.join("cache/sess"))
        .env("HOME", root)
        .env("AGENTTRACE_MAX_SESSION_FILE_BYTES", "200")
        .env_remove("AGENTTRACE_PRICING_FILE");
    command
}

#[test]
fn partial_oversize_corpus_succeeds_and_discloses_the_skip() {
    let root = unique_root("partial");
    fs::write(root.join("corpus/small.jsonl"), small_journal()).expect("small journal");
    fs::write(root.join("corpus/big.jsonl"), big_journal()).expect("big journal");
    let output = agenttrace(&root, "corpus")
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "a partial oversize corpus must still report: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("skipped over the ingestion cap"),
        "the advisory names the cap lane: {stderr}"
    );
    assert!(
        stderr.contains("big.jsonl") && stderr.contains("AGENTTRACE_MAX_SESSION_FILE_BYTES"),
        "the advisory names the skipped file and the deliberate-raise escape hatch: {stderr}"
    );
    // the surviving session is still fully priced — the diagnostics
    // payload is unaffected by the skip
    let stdout = String::from_utf8_lossy(&output.stdout);
    let doc: serde_json::Value = serde_json::from_str(&stdout).expect("diagnostics JSON parses");
    assert!(
        doc["session"]["metrics"]["assistant_turns"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "under-cap session still reported: {}",
        serde_json::to_string(&doc["session"]["metrics"]["assistant_turns"]).unwrap_or_default()
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn all_oversize_corpus_fails_with_the_true_reason() {
    let root = unique_root("all");
    fs::write(root.join("corpus/big.jsonl"), big_journal()).expect("big journal");
    let output = agenttrace(&root, "corpus")
        .output()
        .expect("run agenttrace CLI");
    assert!(
        !output.status.success(),
        "an all-oversize corpus reports failure, not an empty overview"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exceeded the ingestion cap"),
        "failure names the ingestion cap: {stderr}"
    );
    assert!(
        stderr.contains("raise AGENTTRACE_MAX_SESSION_FILE_BYTES"),
        "failure carries the deliberate-raise hint: {stderr}"
    );
    fs::remove_dir_all(&root).ok();
}

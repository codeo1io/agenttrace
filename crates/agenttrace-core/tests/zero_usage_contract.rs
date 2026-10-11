//! rm-408 contract: present-but-zero usage blocks are counted as
//! measured zeros and disclosed — never silently clean, never rerouted
//! to the text-estimate fallback. The contrast pair below pins the
//! absent-vs-present-zero distinction the acceptance names, plus the
//! mixed case (zeros counted alongside real usage) and the JSON
//! surface downstream automation reads.

use agenttrace_core::{parse_file, Session};
use std::fs;
use std::path::PathBuf;

fn fixture(tag: &str, lines: &[String]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("at-zero-usage-{tag}.jsonl"));
    fs::write(&path, lines.join("\n") + "\n").expect("write fixture");
    path
}

fn user_line() -> String {
    r#"{"type":"user","timestamp":"2026-10-04T01:00:00Z","cwd":"/tmp/zero","message":{"role":"user","content":"check the billing please"}}"#.to_string()
}

fn assistant_line(id: &str, usage: Option<&str>) -> String {
    let usage = match usage {
        Some(raw) => format!(r#","usage":{raw}"#),
        None => String::new(),
    };
    format!(
        r#"{{"type":"assistant","timestamp":"2026-10-04T01:00:0{id}Z","message":{{"id":"msg_{id}","model":"claude-sonnet-4-5-20250929","role":"assistant","content":[{{"type":"text","text":"on it"}}]{usage}}}}}"#
    )
}

const ALL_ZERO: &str = r#"{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}"#;
const REAL_USAGE: &str = r#"{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":2,"cache_read_input_tokens":3}"#;

#[test]
fn present_zero_usage_is_counted_and_flagged_not_estimated() {
    let path = fixture(
        "present",
        &[user_line(), assistant_line("5", Some(ALL_ZERO))],
    );
    let session: Session = parse_file(&path).expect("claude transcript parses");
    assert_eq!(
        session.metrics.zero_usage_events, 1,
        "one reported all-zero usage block"
    );
    assert_eq!(
        session.metrics.provenance.tokens, "reported_by_agent+zero_usage_reported:1",
        "present usage — even all-zero — is measured and the flag is appended"
    );
    // The flag names the affected-event count wherever the string lands.
    let json = serde_json::to_string(&session).expect("session serializes");
    assert!(
        json.contains("zero_usage_reported:1"),
        "provenance flag names the count, got: {}",
        session.metrics.provenance.tokens
    );
    assert!(
        json.contains("\"zero_usage_events\":1"),
        "structured field serializes for automation"
    );
    assert_eq!(
        session.metrics.tokens_input + session.metrics.tokens_output,
        0,
        "zeros count as measured zeros (no invented tokens)"
    );
    fs::remove_file(path).ok();
}

#[test]
fn absent_usage_still_falls_back_to_the_text_estimate() {
    let path = fixture("absent", &[user_line(), assistant_line("5", None)]);
    let session = parse_file(&path).expect("claude transcript parses");
    assert_eq!(
        session.metrics.zero_usage_events, 0,
        "no usage block reported, nothing to count"
    );
    assert_eq!(
        session.metrics.provenance.tokens, "estimated_from_text",
        "absent usage keeps the estimate path — the contrast half of the rule"
    );
    let json = serde_json::to_string(&session).expect("session serializes");
    assert!(
        !json.contains("zero_usage_events"),
        "zero count stays out of the JSON"
    );
    fs::remove_file(path).ok();
}

#[test]
fn mixed_session_counts_only_the_zero_blocks() {
    let path = fixture(
        "mixed",
        &[
            user_line(),
            assistant_line("5", Some(ALL_ZERO)),
            assistant_line("6", Some(REAL_USAGE)),
        ],
    );
    let session = parse_file(&path).expect("claude transcript parses");
    assert_eq!(session.metrics.zero_usage_events, 1);
    assert!(session
        .metrics
        .provenance
        .tokens
        .contains("zero_usage_reported:1"));
    // The real block is still counted; the zero block adds nothing.
    // (input stays gross; cache-read is its own column: 10/5/3.)
    assert_eq!(session.metrics.tokens_input, 10);
    assert_eq!(session.metrics.tokens_output, 5);
    assert_eq!(session.metrics.tokens_cache_r, 3);
    fs::remove_file(path).ok();
}

// Review fix (attempt 2bff960d, review 342a1349 F1): the warm-cache half
// of the contract. `zero_usage_events` deserializes with a default, so a
// cache written before rm-408 (schema 22, field absent) parses as fresh —
// size+mtime fingerprint unchanged, schema version equal — while
// defaulting the counter to 0: doctor then reports clean zeros for
// exactly the historical sessions the disclosure exists to flag. The
// 22 → 23 schema bump retires those entries once (rm-230 convention).
// This is the review's PoC pinned as a regression: on a tree where the
// const is still 22 the hand-written v22 entry below IS served and the
// first assertion fails.
#[test]
fn stale_schema_22_cache_cannot_mask_the_disclosure() {
    use std::os::unix::fs::MetadataExt;

    let root = std::env::temp_dir().join("at-zero-usage-stale-v22");
    let _ = fs::remove_dir_all(&root);
    let sessions_dir = root.join("sessions");
    fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    let session_path = sessions_dir.join("session.jsonl");
    fs::write(
        &session_path,
        [user_line(), assistant_line("5", Some(ALL_ZERO))].join("\n") + "\n",
    )
    .expect("write fixture");

    // Hand-write the exact shape the pre-rm-408 release wrote: schema 22,
    // entry fingerprint FRESH against the file (matching mod_time/size),
    // cached session WITHOUT the zero_usage_events field.
    let metadata = fs::metadata(&session_path).expect("stat fixture");
    let mod_time = metadata.mtime() * 1_000_000_000 + metadata.mtime_nsec();
    let session_path_str = session_path.to_string_lossy().to_string();
    let mut entries = serde_json::Map::new();
    entries.insert(
        session_path_str.clone(),
        serde_json::json!({
            "mod_time": mod_time,
            "size": metadata.len() as i64,
            "session": {
                "Name": "stale",
                "Path": session_path_str,
                "Metrics": {
                    "SourceTool": "hermes_jsonl",
                    "ModelUsed": "cached-model",
                    "SessionStart": "2026-10-04T01:00:00Z",
                    "ToolArgUsage": {},
                },
                "Health": 91,
                "ToolWarnings": [],
                "Diagnostics": {},
            },
        }),
    );
    let cache = serde_json::json!({
        "schema_version": 22,
        "entries": serde_json::Value::Object(entries),
    });
    fs::write(root.join("sessions.json"), cache.to_string()).expect("write stale v22 cache");

    with_session_cache_dir(&root, || {
        let sessions = agenttrace_core::load_sessions_from_dir(Some(&sessions_dir));
        assert_eq!(sessions.len(), 1, "one session discovered");
        assert_eq!(
            sessions[0].metrics.zero_usage_events, 1,
            "the fresh-fingerprint v22 entry must NOT be served: the source's \
             all-zero usage block would read back as clean zeros"
        );
        assert!(
            sessions[0]
                .metrics
                .provenance
                .tokens
                .contains("zero_usage_reported:1"),
            "re-parsed from source, so the flag is present: got {}",
            sessions[0].metrics.provenance.tokens
        );
        assert_ne!(
            sessions[0].name, "stale",
            "second tripwire: the cached Name proves the entry was served"
        );
    });

    let _ = fs::remove_dir_all(&root);
}

fn with_session_cache_dir(cache: &std::path::Path, f: impl FnOnce()) {
    let key = "AGENTTRACE_SESSION_CACHE_DIR";
    let previous = std::env::var_os(key);
    std::env::set_var(key, cache);
    f();
    match previous {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

#[test]
fn negative_usage_is_clamped_with_disclosure_not_clean_zero() {
    // rm-408 negative arm (run d6432dd5, assess 8ee739a8 F4): a hostile
    // or buggy transcript reporting negative token usage read back as a
    // clean zero-spend session — `analyze` clamps each usage key with
    // `.max(0)` and nothing disclosed the clamping. The negative arm
    // extends the landed disclosure taxonomy additively: the clamped
    // event count joins the provenance string; the shipped
    // `zero_usage_reported` / `calculated_from_tokens_clamped` markers
    // keep their meanings.
    const NEGATIVE_USAGE: &str = r#"{"input_tokens":-500,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}"#;
    let path = fixture(
        "negative",
        &[user_line(), assistant_line("5", Some(NEGATIVE_USAGE))],
    );
    let session: Session = parse_file(&path).expect("claude transcript parses");
    assert_eq!(
        session.metrics.tokens_input, 0,
        "negative input tokens clamp to zero, never below"
    );
    assert!(
        session
            .metrics
            .provenance
            .tokens
            .contains("negative_usage_clamped:1"),
        "the clamped events are disclosed on the provenance string: got {}",
        session.metrics.provenance.tokens
    );
    assert!(
        session
            .metrics
            .provenance
            .tokens
            .starts_with("reported_by_agent"),
        "the base provenance marker is unchanged: got {}",
        session.metrics.provenance.tokens
    );
    assert_eq!(
        session.metrics.zero_usage_events, 0,
        "a negative block is a clamped block, not a zero-usage block"
    );
}

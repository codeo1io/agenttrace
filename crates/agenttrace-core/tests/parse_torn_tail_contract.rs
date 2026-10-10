//! rm-526 contract: lines a format parser could not parse are disclosed,
//! never silently dropped. `jsonl_objects` used to filter-map away any
//! line failing both the strict and the lenient parse with no counter,
//! and every format-specific parser consumed that iterator — so a torn
//! tail (a writer crash mid-object, the read side of the hazard rm-250
//! hardened the writers against) vanished whole: the session parsed
//! green with `skipped: 0`, `data_health` stayed empty and confident,
//! and on the PoC corpus the dropped line carried the session's ONLY
//! usage record, silently undercounting tokens and cost.
//!
//! The contrast pair below pins the distinction the acceptance names:
//! the torn twin discloses `unparseable_line` through the same
//! `metrics.line_skips` channel the codex parser and the generic
//! fallback (P7-1) already use, while the intact twin counts the usage —
//! proving the drop is the reader swallowing corruption, not
//! misclassification. Downstream surfaces are pinned too: `data_health`
//! renders the reason and drops confidence per the disclosure policy,
//! and `--doctor` folds `line_skips` into its journal disclosures.

use agenttrace_core::{build_doctor_report, data_health, parse_file};
use std::fs;
use std::path::PathBuf;

fn head_lines() -> [String; 4] {
    [
        r#"{"type":"user","message":{"role":"user","content":"first question"},"timestamp":"2026-10-01T10:00:00Z"}"#
            .to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"answer one"}]},"timestamp":"2026-10-01T09:58:00Z"}"#
            .to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Bash","input":{"command":"ls"}}]},"timestamp":"2026-10-01T10:02:00Z"}"#
            .to_string(),
        r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"file1"}]},"timestamp":"2026-10-01T09:57:00Z"}"#
            .to_string(),
    ]
}

/// The PoC's fifth line, complete: the session's only usage record.
/// Model is the dated, priced id (like the rm-408 fixture) so the intact
/// twin's `data_health` confidence is not dragged down by fallback
/// pricing — the test isolates the line-skips channel.
fn usage_line() -> String {
    r#"{"type":"assistant","id":"msg_01","model":"claude-sonnet-4-5-20250929","message":{"id":"msg_01","role":"assistant","model":"claude-sonnet-4-5-20250929","usage":{"input_tokens":100,"output_tokens":20,"cache_read_input_tokens":50,"cache_creation_input_tokens":10}},"timestamp":"2026-10-01T10:03:00Z"}"#
        .to_string()
}

/// The same line as a torn tail: cut mid-object where the writer crashed.
fn torn_usage_line() -> String {
    usage_line()[..usage_line().len() - 120].to_string()
}

fn fixture(tag: &str, lines: &[String]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("at-torn-tail-{tag}.jsonl"));
    fs::write(&path, lines.join("\n") + "\n").expect("write fixture");
    path
}

#[test]
fn torn_tail_line_is_disclosed_not_dropped() {
    let torn = parse_file(&fixture(
        "torn",
        &[head_lines().to_vec(), vec![torn_usage_line()]].concat(),
    ))
    .expect("torn journal still parses — that is the hazard");
    // The torn line was counted, not swallowed.
    assert_eq!(
        torn.metrics.line_skips.get("unparseable_line"),
        Some(&1),
        "torn tail must be disclosed: {:?}",
        torn.metrics.line_skips
    );
    // And it took the session's only usage record with it.
    assert!(
        torn.metrics.tokens_input < 100,
        "torn usage must not leak in: {}",
        torn.metrics.tokens_input
    );

    // The intact twin counts the usage the torn tail dropped.
    let intact = parse_file(&fixture(
        "intact",
        &[head_lines().to_vec(), vec![usage_line()]].concat(),
    ))
    .expect("intact journal parses");
    assert!(
        !intact.metrics.line_skips.contains_key("unparseable_line"),
        "intact twin discloses nothing: {:?}",
        intact.metrics.line_skips
    );
    assert_eq!(intact.metrics.tokens_input, 100);
    assert_eq!(intact.metrics.tokens_output, 20);

    // data_health drops confidence and names the reason (insights.rs
    // already renders `line_skips` in every report surface).
    let health = data_health(&[torn], 1, 0);
    assert_eq!(
        health.line_skips.get("unparseable_line"),
        Some(&1),
        "data_health carries the disclosure: {:?}",
        health.line_skips
    );
    assert_eq!(health.confidence, "low");
    let healthy = data_health(&[intact], 1, 0);
    assert_eq!(healthy.confidence, "high");
}

#[test]
fn codex_fast_path_torn_tail_discloses_through_line_skips() {
    // rm-709's integration arm: the codex lane is the one format this
    // sweep does not otherwise cover at the parse_file entry — its
    // torn drops ride the FAST path (parse_codex_rollout_jsonl, taken
    // as soon as a session_meta line is seen), which the generic
    // `jsonl_objects_counted` census above never runs for codex
    // journals. rm-584 landed the counters (verified live at aad1f7b:
    // the torn twin discloses `codex_unparseable_line:1` while the
    // intact twin counts the full snapshot); this pin is the parity
    // oracle rm-709's acceptance names — the fast path must disclose
    // its torn tail exactly like the slow generic lane above
    // (`unparseable_line`) instead of silently dropping it — and it
    // must keep the drop on the LOSS channel (line_skips), not the
    // assumption channel (disclosure_counters, the rm-730 split).
    let meta = "{\"timestamp\":\"2026-10-09T10:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"cwd\":\"/tmp/proj\",\"originator\":\"codex_cli_rs\"}}";
    let turn = "{\"timestamp\":\"2026-10-09T10:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hi\"}]}}";
    let first_count = "{\"timestamp\":\"2026-10-09T10:00:05Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"input_tokens\":1500,\"cached_input_tokens\":0,\"output_tokens\":200,\"reasoning_output_tokens\":0},\"last_token_usage\":{\"input_tokens\":1500,\"cached_input_tokens\":0,\"output_tokens\":200,\"reasoning_output_tokens\":0}}}}";
    // The final cumulative snapshot, complete ...
    let final_count = "{\"timestamp\":\"2026-10-09T10:00:06Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"input_tokens\":3000,\"cached_input_tokens\":0,\"output_tokens\":400,\"reasoning_output_tokens\":0},\"last_token_usage\":{\"input_tokens\":3000,\"cached_input_tokens\":0,\"output_tokens\":400,\"reasoning_output_tokens\":0}}}}";
    // ... and torn mid-object where the writer crashed. ASCII-only
    // payloads keep the byte-slice cut a char-boundary cut.
    let torn_tail = &final_count[..final_count.len() - 80];

    let torn = parse_file(&fixture(
        "codex-torn",
        &[
            meta.to_string(),
            turn.to_string(),
            first_count.to_string(),
            torn_tail.to_string(),
        ],
    ))
    .expect("a torn codex journal still parses — that is the hazard");
    assert_eq!(
        torn.metrics.line_skips.get("codex_unparseable_line"),
        Some(&1),
        "the torn tail must surface on the loss channel: {:?}",
        torn.metrics.line_skips
    );
    assert!(
        !torn.metrics.disclosure_counters.contains_key("codex_unparseable_line"),
        "a torn drop is real loss, not an assumption: it must not ride the rm-730 disclosure channel"
    );
    // The torn line carried the session's final cumulative usage
    // snapshot; the undercount is the reader's truth, disclosed above.
    assert_eq!(torn.metrics.tokens_input, 1500);
    assert_eq!(torn.metrics.tokens_output, 200);

    let intact = parse_file(&fixture(
        "codex-intact",
        &[
            meta.to_string(),
            turn.to_string(),
            first_count.to_string(),
            final_count.to_string(),
        ],
    ))
    .expect("the intact twin parses");
    assert!(
        !intact
            .metrics
            .line_skips
            .contains_key("codex_unparseable_line"),
        "the intact twin must not report torn drops: {:?}",
        intact.metrics.line_skips
    );
    assert_eq!(intact.metrics.tokens_input, 4500);
    assert_eq!(intact.metrics.tokens_output, 600);
}

#[test]
fn workbuddy_torn_tail_discloses_through_both_channels() {
    // Regression pin for the rm-526 x rm-538/rm-600 seam (re-threaded at
    // the independent review, conflict case 7a502782): rm-538 moved
    // workbuddy out of the shared probe array so its basis counters could
    // ride `disclosure_counters` (the non-loss channel), and the moved
    // early-return initially rebuilt its session with a bare
    // `session_from_events` — silently dropping the rm-526 line-skips
    // census, so a torn-tail workbuddy journal undercounted with zero
    // disclosure again. The workbuddy arm must compose BOTH channels:
    // `unparseable_line` in `line_skips` (the loss channel, confidence
    // drops) and the basis-clamp fact in `disclosure_counters`.
    let head = r#"{"type":"function_call","name":"bash","arguments":"{}","sessionId":"wb-torn","cwd":"/tmp/proj","callId":"c1"}"#;
    let message = r#"{"type":"message","role":"user","content":[{"type":"text","text":"run"}],"sessionId":"wb-torn","cwd":"/tmp/proj","message":{"role":"user","usage":{"input_tokens":100,"output_tokens":20,"cache_read_input_tokens":150}}}"#;
    // Torn tail: the paired result line, cut mid-object where the writer
    // crashed. It carries no usage (usage rode the message line), so the
    // pin isolates the disclosure, not an undercount.
    let torn_tail = r#"{"type":"function_call_result","output":"do"#;
    let corpus = format!("{head}\n{message}\n{torn_tail}\n");
    let path = std::env::temp_dir().join("at-torn-tail-workbuddy.jsonl");
    fs::write(&path, corpus).expect("write fixture");
    let session = parse_file(&path).expect("torn workbuddy journal still parses");
    assert_eq!(session.metrics.source_tool, "workbuddy");
    assert_eq!(
        session.metrics.line_skips.get("unparseable_line"),
        Some(&1),
        "rm-526 must reach the workbuddy arm: {:?}",
        session.metrics.line_skips
    );
    // rm-538/rm-600's own channel is untouched by the re-thread: the
    // cache-above-input record still clamps and still discloses on the
    // non-loss map.
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("workbuddy_input_basis:cache_clamped"),
        Some(&1),
        "basis counters keep their own channel: {:?}",
        session.metrics.disclosure_counters
    );
    assert_eq!(session.metrics.tokens_cache_r, 100);
    assert_eq!(session.metrics.tokens_input, 0);

    // The intact twin discloses nothing — the skip is the reader's truth,
    // not the format's.
    let intact_path = std::env::temp_dir().join("at-torn-tail-workbuddy-intact.jsonl");
    fs::write(
        &intact_path,
        format!(
            "{head}\n{message}\n{}\n",
            r#"{"type":"function_call_result","output":"done","callId":"c1","sessionId":"wb-torn","cwd":"/tmp/proj"}"#
        ),
    )
    .expect("write fixture");
    let intact = parse_file(&intact_path).expect("intact workbuddy journal parses");
    assert!(!intact.metrics.line_skips.contains_key("unparseable_line"));
    assert_eq!(
        intact
            .metrics
            .disclosure_counters
            .get("workbuddy_input_basis:cache_clamped"),
        Some(&1),
        "the clamp fact is format truth, independent of the torn tail"
    );
}

#[test]
fn non_object_jsonl_line_is_disclosed_not_dropped() {
    // The second silent drop class inside `jsonl_objects`: a line that
    // parses as JSON but is not an object never reached any parser.
    let corpus = [head_lines().to_vec(), vec!["\"orphan\"".to_string()]].concat();
    let session = parse_file(&fixture("nonobject", &corpus)).expect("parses");
    assert_eq!(
        session.metrics.line_skips.get("non_object_line"),
        Some(&1),
        "non-object line must be disclosed: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn doctor_folds_line_skips_into_journal_disclosures() {
    // rm-526's doctor half: `line_skips` reaches --doctor's disclosures
    // (the map the text report prints as "Disclosed facts" — renamed from
    // "Journal disclosures" by the later-landed rm-538 lane, which moved the
    // workbuddy basis counters onto the non-loss channel — and the JSON
    // report exposes as `disclosures`) — previously only
    // `disclosure_counters` were folded in, so parse failures were
    // invisible to doctor entirely.
    let dir = std::env::temp_dir().join(format!("at-torn-tail-doctor-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch dir");
    fs::write(
        dir.join("torn.jsonl"),
        [head_lines().to_vec(), vec![torn_usage_line()]]
            .concat()
            .join("\n")
            + "\n",
    )
    .expect("write fixture");

    let report = build_doctor_report(Some(&dir), false);
    // The scratch dir holds only the torn fixture, so the top-level
    // aggregate is exactly its disclosure.
    assert_eq!(
        report.disclosures.get("unparseable_line"),
        Some(&1),
        "doctor aggregates the line-skips disclosure: {:?}",
        report.disclosures
    );
    assert!(
        report.directories.iter().any(|entry| entry.files > 0),
        "doctor scanned the scratch dir: {:?}",
        report
            .directories
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>()
    );
    // And the user-visible render prints it in the Disclosed facts
    // block (the same arm the pi counters use; the row was renamed
    // from "Journal disclosures" by rm-538 after this contract was
    // authored — rebased at integration).
    let text = agenttrace_core::render_doctor_report(Some(&dir), false, "text")
        .expect("doctor text render");
    assert!(
        text.contains("Disclosed facts:") && text.contains("unparseable_line=1"),
        "text render shows the disclosure:\n{text}"
    );

    let _ = fs::remove_dir_all(&dir);
}

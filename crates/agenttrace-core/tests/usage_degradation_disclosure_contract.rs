//! rm-2273 + rm-860 contract — usage-truth session disclosure.
//!
//! Probe v10/v13 (assess 8c1c0cea F3/F4): a journal whose usage block is
//! NEGATIVE gets zeroed and flags `implausible_magnitude` (HIGH, health
//! 70), but the SAME journal with the usage line merely UNPARSEABLE
//! (`cost_usd: 1e999` — serde_json rejects the out-of-range float, the
//! line drops whole) degraded to the word-count fallback seeded with
//! MODEL 'default' while scoring health 100 / anomalies 0, the only
//! trace a `--doctor` counter the session views never render. The
//! degradation class must be SESSION-VISIBLE at parity with the
//! negative arm, attributed per session, on every lane that can lose
//! an accounting-bearing line (generic fallback, native jsonl probes,
//! codex rollouts).
//!
//! Probe v2 (assess F2): duplicate keys inside a usage object collapse
//! last-wins the moment serde_json builds a Value (`input_tokens: 100`
//! then `input_tokens: 2000` counts a clean 2050 beside `output_tokens:
//! 50`) — the collapse is upstream of `usage_from_value`, so the
//! duplicate-key census runs at LINE level on consumed lines and
//! discloses `usage_duplicate_keys` on the informational tier WITHOUT
//! dropping the line or changing the accounting.

use agenttrace_core::{parse_file, Session};
use std::fs;
use std::path::PathBuf;

/// pid-qualified scratch file per the house contract-test convention
/// (generic_model_usage_truth.rs): caller removes.
fn write(tag: &str, raw: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "at-rm2273-{}-{tag}.jsonl",
        std::process::id()
    ));
    fs::write(&path, raw).expect("scratch journal writes");
    path
}

fn parse(tag: &str, raw: &str) -> Session {
    let path = write(tag, raw);
    let session = parse_file(&path).expect("journal parses");
    let _ = fs::remove_file(&path);
    session
}

/// v10 — the negative-usage parity oracle (unchanged behavior).
const NEGATIVE_USAGE_JOURNAL: &str = concat!(
    "{\"role\":\"user\",\"content\":\"hello there\"}\n",
    "{\"role\":\"assistant\",\"content\":\"an answer\",\"model_used\":\"m\",\"usage\":{\"input_tokens\":-5,\"output_tokens\":50}}\n",
);

/// v13 — the silent-degradation PoC: the only accounting-bearing line
/// carries `cost_usd: 1e999`, serde_json refuses the out-of-range
/// float, the line drops whole, and the session falls back to
/// word-count estimation with MODEL 'default'.
const UNPARSEABLE_USAGE_JOURNAL: &str = concat!(
    "{\"role\":\"user\",\"content\":\"hello there friend\"}\n",
    "{\"role\":\"assistant\",\"content\":\"an answer of some length\",\"model_used\":\"m\",\"usage\":{\"input_tokens\":100,\"output_tokens\":50,\"cost_usd\":1e999}}\n",
);

#[test]
fn unparseable_usage_line_discloses_at_parity_with_the_negative_arm() {
    // The side-by-side fixture: both sessions in one test home, each
    // journal losing its usage truth a different way.
    let negative = parse("v10", NEGATIVE_USAGE_JOURNAL);
    let unparseable = parse("v13", UNPARSEABLE_USAGE_JOURNAL);

    // v10 arm — unchanged oracle: negatives zeroed + HIGH
    // implausible_magnitude, health 70.
    let magnitude = negative
        .anomalies
        .iter()
        .find(|anomaly| anomaly.kind == "implausible_magnitude")
        .expect("v10 keeps its implausible_magnitude anomaly");
    assert_eq!(magnitude.severity, "high");
    assert_eq!(negative.health, 70);

    // v13 arm — the fix: the degraded session is session-visible at
    // parity (one HIGH anomaly → the same 70), with per-session
    // fallback attribution naming MODEL 'default'.
    let degraded = unparseable
        .anomalies
        .iter()
        .find(|anomaly| anomaly.kind == "usage_degraded")
        .expect("v13 must now mint the usage_degraded anomaly");
    assert_eq!(degraded.severity, "high", "parity with the v10 arm");
    assert_eq!(unparseable.health, negative.health);
    assert!(
        degraded
            .detail
            .contains("word-count estimation (model 'default')"),
        "attribution must name the model label the fallback produced: {}",
        degraded.detail
    );
    assert!(
        degraded.detail.contains("1 journal line(s)"),
        "attribution must count the dropped accounting line(s): {}",
        degraded.detail
    );
    // The parse-loss census stays on the line_skips channel.
    assert_eq!(
        unparseable.metrics.line_skips.get("unparseable_line"),
        Some(&1)
    );
    assert_eq!(
        unparseable
            .metrics
            .line_skips
            .get("model_or_usage_dropped"),
        Some(&1)
    );
    // The fallback basis is the honest one: estimation, model 'default'.
    assert!(unparseable
        .metrics
        .provenance
        .tokens
        .starts_with("estimated_from_text"));
    assert_eq!(unparseable.metrics.model_used, "default");

    // JSON lane parity (the -f json session surface serializes the
    // Session struct): the degradation must survive serialization —
    // before rm-2273 this lane rendered health 100 with zero anomalies.
    let json = serde_json::to_string_pretty(&unparseable).expect("session serializes");
    assert!(json.contains("usage_degraded"), "JSON lane must carry the anomaly");
    assert!(
        json.contains("\"health\": 70"),
        "JSON lane must carry the parity health, got: {json}"
    );
}

#[test]
fn duplicate_usage_keys_disclose_without_dropping_the_line() {
    // v2 oracle: {input 100, output 50, input 2000} — the last-wins
    // collapse counts input=2000 + output=50 = 2050, and rm-860's job
    // is NOT to change that accounting but to disclose the duplicate
    // that made it invisible: the counter appears, the line survives,
    // the health stays 100.
    let journal = concat!(
        "{\"role\":\"user\",\"content\":\"hi\"}\n",
        "{\"role\":\"assistant\",\"content\":\"answer\",\"model_used\":\"m\",\"usage\":{\"input_tokens\":100,\"output_tokens\":50,\"input_tokens\":2000}}\n",
    );
    let session = parse("v2", journal);

    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("usage_duplicate_keys"),
        Some(&1),
        "the duplicate must be counted on the informational tier"
    );
    assert!(
        !session.metrics.line_skips.contains_key("unparseable_line"),
        "the line is NEVER dropped for carrying duplicates: {:#?}",
        session.metrics.line_skips
    );
    // The accounting is deliberately unchanged: last-wins.
    assert_eq!(session.metrics.tokens_input, 2000);
    assert_eq!(session.metrics.tokens_output, 50);
    assert_eq!(session.metrics.tokens_input + session.metrics.tokens_output, 2050);
    assert_eq!(session.health, 100, "disclosure alone must not tax health");
    assert!(
        !session
            .anomalies
            .iter()
            .any(|anomaly| anomaly.kind == "usage_degraded"),
        "no parse loss happened — no degradation anomaly"
    );
}

/// The native jsonl lanes (claude flat transcripts and kin) lose the
/// torn line inside `jsonl_objects_counted`; the rm-2273 subclass
/// (`model_or_usage_dropped`) must mint there too, and the anomaly
/// must be re-sealed after the rm-526 finish-closure line_skips fold.
#[test]
fn native_lane_torn_usage_line_mints_the_degradation_anomaly() {
    let journal = concat!(
        "{\"type\":\"user\",\"timestamp\":\"2026-10-20T10:00:00Z\",\"content\":\"run the build\"}\n",
        "{\"type\":\"tool_use\",\"timestamp\":\"2026-10-20T10:00:01Z\",\"tool_name\":\"Bash\",\"content\":\"make\"}\n",
        "{\"type\":\"tool_result\",\"timestamp\":\"2026-10-20T10:00:02Z\",\"tool_name\":\"Bash\",\"content\":\"ok\"}\n",
        "{\"type\":\"assistant\",\"timestamp\":\"2026-10-20T10:00:03Z\",\"content\":\"done\",\"usage\":{\"input_tokens\":100,\"output_tokens\":50,\"cost_usd\":1e999}}\n",
    );
    let session = parse("native", journal);

    // The claude flat lane claimed it (tool grain proves the probe
    // path, not the generic fallback).
    assert!(
        session.metrics.tool_usage.contains_key("Bash"),
        "the native claude lane must claim this journal"
    );
    assert_eq!(
        session.metrics.line_skips.get("unparseable_line"),
        Some(&1),
        "the torn line must be censused: {:#?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session.metrics.line_skips.get("model_or_usage_dropped"),
        Some(&1),
        "the native lane must subclassify lost accounting"
    );
    let degraded = session
        .anomalies
        .iter()
        .find(|anomaly| anomaly.kind == "usage_degraded")
        .expect("the re-seal must make the degradation session-visible");
    assert_eq!(degraded.severity, "high");
    assert_eq!(session.health, 70, "parity on the native lane too");
}

/// The codex rollout lane counts its own per-reason skips and folds
/// them through the parse_counters merge — the third re-seal site.
#[test]
fn codex_lane_torn_usage_line_mints_the_degradation_anomaly() {
    let journal = concat!(
        "{\"timestamp\":\"2026-10-20T10:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"torn\",\"cwd\":\"/tmp\",\"originator\":\"codex_cli_rs\",\"cli_version\":\"0.44.0\",\"instructions\":\"\"}}\n",
        "{\"timestamp\":\"2026-10-20T10:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hi\"}]}}\n",
        "{\"timestamp\":\"2026-10-20T10:00:02Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"done\"}],\"usage\":{\"input_tokens\":100}},\"cost_usd\":1e999}\n",
    );
    let session = parse("codex", journal);

    assert_eq!(
        session.metrics.line_skips.get("codex_unparseable_line"),
        Some(&1),
        "codex keeps its own census: {:#?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session.metrics.line_skips.get("model_or_usage_dropped"),
        Some(&1),
        "the codex lane must subclassify lost accounting"
    );
    let degraded = session
        .anomalies
        .iter()
        .find(|anomaly| anomaly.kind == "usage_degraded")
        .expect("the codex parse_counters fold must re-seal the anomaly in");
    assert_eq!(degraded.severity, "high");
    assert_eq!(session.health, 70);
}

/// The census is CONSERVATIVE by design: estimation without parse loss
/// is the honest generic lane (journals that never carried usage), and
/// it must not mint the degradation class — only LOST accounting does.
#[test]
fn estimation_without_parse_loss_stays_clean() {
    let journal = concat!(
        "{\"role\":\"user\",\"content\":\"plain question without any usage wire\"}\n",
        "{\"role\":\"assistant\",\"content\":\"plain answer without any usage wire\"}\n",
    );
    let session = parse("clean", journal);

    assert!(session.metrics.line_skips.is_empty());
    assert!(
        !session
            .anomalies
            .iter()
            .any(|anomaly| anomaly.kind == "usage_degraded"),
        "the generic word-count lane is legitimate, not degraded"
    );
    assert_eq!(session.health, 100);
}

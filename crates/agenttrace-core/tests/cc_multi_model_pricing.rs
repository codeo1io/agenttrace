//! rm-020 golden: a Claude Code transcript whose two assistant turns
//! name two different models, each carrying its own usage block.
//!
//! Pre-fix (parser freeze, rm-663): the first assistant's model is
//! stamped onto every later usage block, so the whole session priced
//! at the frozen model — (1000+2000)/1e6*$3 + (100+200)/1e6*$15 =
//! $0.0135 — and the session claimed `claude-sonnet-4-5` as its model.
//!
//! Post-fix: each block prices at the model that produced it — sonnet
//! 1000/1e6*$3 + 100/1e6*$15 = $0.0045, opus 2000/1e6*$15 +
//! 200/1e6*$75 = $0.045 — total $0.0495, model_used "multiple", and
//! the rm-020 per-model ledger carries one row per model.
//!
//! This file is the byte-identical PoC corpus from the cycle's assess
//! pass (/tmp/at-assess-a01f/corpus/cc-multi.jsonl).

use agenttrace_core::parse_file;
use std::path::PathBuf;

#[test]
fn cc_multi_model_transcript_prices_each_block_at_its_own_model() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cc-multi-model.jsonl");
    let session = parse_file(&path).expect("fixture parses");
    assert_eq!(
        session.metrics.tokens_input, 3000,
        "tokens are session totals regardless of model mix"
    );
    assert_eq!(session.metrics.tokens_output, 300);
    // The frozen-model regression gate: $0.0135 is what the pre-fix
    // parser produced (every block priced at the first model).
    assert_eq!(session.metrics.cost_estimated, 0.0495);
    assert_eq!(session.metrics.model_used, "multiple");
    assert_eq!(
        session.metrics.provenance.cost,
        "calculated_per_message_tokens"
    );

    let ledger = &session.metrics.model_ledger;
    assert_eq!(
        ledger.len(),
        2,
        "one ledger row per model that produced usage: {ledger:?}"
    );
    let sonnet = ledger
        .iter()
        .find(|row| row.model == "claude-sonnet-4-5")
        .expect("sonnet row present");
    assert_eq!(sonnet.tokens_input, 1000);
    assert_eq!(sonnet.tokens_output, 100);
    assert_eq!(sonnet.tokens_cache_w, 0);
    assert_eq!(sonnet.tokens_cache_r, 0);
    assert_eq!(sonnet.cost_usd, 0.0045);
    let opus = ledger
        .iter()
        .find(|row| row.model == "claude-opus-4-1")
        .expect("opus row present");
    assert_eq!(opus.tokens_input, 2000);
    assert_eq!(opus.tokens_output, 200);
    assert_eq!(opus.cost_usd, 0.045);
    // The ledger sums to the session total: session cost IS the sum
    // over models of lookup_price(model) x that model's tokens.
    let sum: f64 = ledger.iter().map(|row| row.cost_usd).sum();
    assert!((sum - session.metrics.cost_estimated).abs() < 1e-9);
}

/// rm-020: single-model sessions keep the single-price path and the
/// ledger's one row reproduces the session cost exactly — the ledger
/// must not change any single-model number.
#[test]
fn single_model_session_ledger_row_matches_session_cost() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cc-multi-model.jsonl");
    let full = parse_file(&path).expect("fixture parses");
    // Drop the second (opus) assistant turn by keeping only the first
    // two lines of the fixture.
    let raw = std::fs::read_to_string(&path).expect("read fixture");
    let head: String = raw
        .lines()
        .take(2)
        .map(|line| format!("{line}\n"))
        .collect();
    let single =
        agenttrace_core::parse_raw_session("head", "cc-multi.jsonl", &head).expect("head parses");
    assert_eq!(single.metrics.model_used, "claude-sonnet-4-5");
    assert_eq!(single.metrics.cost_estimated, 0.0045);
    assert_eq!(single.metrics.model_ledger.len(), 1);
    assert_eq!(single.metrics.model_ledger[0].model, "claude-sonnet-4-5");
    assert_eq!(single.metrics.model_ledger[0].cost_usd, 0.0045);
    // Liveness guard on the fixture itself: the head really is a strict
    // prefix (the frozen-vs-per-model distinction needs both turns).
    assert!(full.metrics.cost_estimated > single.metrics.cost_estimated);
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// rm-020 render surface: the corpus overview carries the per-model
/// breakdown in text, markdown, and JSON — built from the session
/// ledgers, so a multi-model session shows one row per model at its
/// own rate instead of a blended cell under "By Model".
#[test]
fn overview_reports_render_the_per_model_breakdown() {
    let fixture = fixture_path("cc-multi-model.jsonl");
    let mut sessions = vec![agenttrace_core::parse_file(&fixture).expect("fixture parses")];
    let overview = agenttrace_core::compute_overview(&sessions);
    let text = agenttrace_core::report_overview_text(&overview, &sessions);
    assert!(
        text.contains("── Per Model (catalog-priced) ──"),
        "text overview names the rollup section:\n{text}"
    );
    for model in ["claude-sonnet-4-5", "claude-opus-4-1"] {
        assert!(
            text.contains(model),
            "text overview rows include {model}:\n{text}"
        );
    }

    let markdown = agenttrace_core::report_overview_markdown(&overview, &sessions);
    assert!(
        markdown.contains("## Per model (catalog-priced)"),
        "markdown overview names the rollup table:\n{markdown}"
    );
    assert!(markdown.contains("claude-opus-4-1"));

    let json: serde_json::Value =
        serde_json::from_str(&agenttrace_core::report_overview_json(&overview, &sessions))
            .expect("overview json parses");
    let breakdown = json
        .pointer("/summary/per_model_breakdown")
        .and_then(|value| value.as_array())
        .expect("summary.per_model_breakdown present")
        .clone();
    assert_eq!(
        breakdown.len(),
        2,
        "one rolled-up row per model: {breakdown:?}"
    );
    let opus = breakdown
        .iter()
        .find(|row| row.get("model").and_then(|m| m.as_str()) == Some("claude-opus-4-1"))
        .expect("opus row");
    assert_eq!(
        opus.pointer("/tokens/output").and_then(|v| v.as_i64()),
        Some(200),
        "opus row carries its own output tokens: {opus:?}"
    );
    assert_eq!(
        opus.get("sessions").and_then(|v| v.as_u64()),
        Some(1),
        "sessions counts the sessions the model appears in: {opus:?}"
    );
    sessions.clear();
    let empty_overview = agenttrace_core::compute_overview(&sessions);
    let empty_json = agenttrace_core::report_overview_json(&empty_overview, &sessions);
    let parsed: serde_json::Value = serde_json::from_str(&empty_json).expect("empty json parses");
    assert_eq!(
        parsed
            .pointer("/summary/per_model_breakdown")
            .and_then(|value| value.as_array())
            .map(Vec::len),
        Some(0),
        "no sessions, no breakdown rows"
    );
}

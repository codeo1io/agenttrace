//! rm-1988 (cycle 1, 2026-10-18): Cline envelope `modelInfo` truth
//! contract.
//!
//! A Cline CLI `.messages.json` envelope carries the producing model as
//! `modelInfo.id` on assistant messages. Before rm-1988 the envelope lane
//! threaded the lane model ("default" for a bare `{"messages":[...]}` file)
//! into every event, so attribution AND pricing landed on the catalog
//! fallback row: an opus transcript estimated at the default rates
//! (0.018 instead of 0.09 — assess c954461c PoC F2) with no disclosure.
//!
//! The contract (ROADMAP row rm-1988): the declaration wins for the
//! message that carries it; usage prices on the declared row; a recorded
//! `metrics.cost` still wins over row pricing; an id with no catalog row
//! is attributed AS DECLARED and disclosed (`cline_modelinfo_unpriced_ids`),
//! never silently folded to the default; envelopes without `modelInfo`
//! keep the exact pre-rm-1988 shape (fallback row, zero cline_modelinfo
//! disclosures). Search rows (`--search`, rm-1415) carry the declared
//! model because `SearchResult.model` reads the session attribution.

use agenttrace_core::search::search_sessions;

fn scratch_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "at-rm1988-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `{"messages": [...]}` envelope file — the parse_cline_value lane.
fn write_envelope(dir: &std::path::Path, name: &str, messages: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, format!("{{\"messages\": [{messages}]}}")).unwrap();
    p
}

const OPUS: &str = "claude-opus-4-1-20250805";

const OPUS_ASSISTANT: &str = r#"{"role":"user","content":"razzmatazz brief me on the fix","timestamp":"2026-10-18T09:59:00Z"},
{"role":"assistant","content":"shipped the fix","timestamp":"2026-10-18T10:00:00Z",
 "modelInfo":{"id":"OPUS_ID","provider":"anthropic"},
 "metrics":{"tokensIn":1500,"tokensOut":900}}"#
    .replace("OPUS_ID", OPUS);

/// The declaration prices the transcript on the DECLARED row, not the
/// default fallback: 1500 in + 900 out at opus $15/$75 per M = 0.0225 +
/// 0.0675 = 0.09 (the default row would charge 0.018 — assess PoC F2).
#[test]
fn declared_model_prices_on_the_declared_row_not_the_default_fallback() {
    let dir = scratch_dir("priced");
    let path = write_envelope(&dir, "sess-a.messages.json", &OPUS_ASSISTANT);
    let session = agenttrace_core::parse_file(&path).expect("envelope parses");
    assert_eq!(session.model, OPUS, "modelInfo.id must win attribution");
    assert!(
        (session.metrics.cost_estimated - 0.09).abs() < 1e-9,
        "opus row pricing expected, got {}",
        session.metrics.cost_estimated
    );
    assert_eq!(
        session.metrics.disclosure_counters.get("cline_modelinfo_messages"),
        Some(&1),
        "the declaration itself is disclosed"
    );
    let declared_list = session
        .metrics
        .wire_metadata
        .get("cline_modelinfo_models")
        .expect("declared ids ride wire_metadata");
    assert!(
        declared_list.contains(OPUS),
        "declared id list must name the id, got {declared_list}"
    );
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("cline_modelinfo_unpriced_ids"),
        "a catalog-priced id must not mint the unpriced counter"
    );
}

/// No `modelInfo` anywhere → the exact pre-rm-1988 shape: lane model
/// ("default"), fallback-row pricing, and ZERO cline_modelinfo disclosure
/// surfaces (both maps).
#[test]
fn envelope_without_modelinfo_keeps_the_fallback_shape_and_mints_no_counters() {
    let dir = scratch_dir("fallback");
    let body = r#"{"role":"user","content":"plain","timestamp":"2026-10-18T09:59:00Z"},
{"role":"assistant","content":"plain reply","timestamp":"2026-10-18T10:00:00Z",
 "metrics":{"tokensIn":1500,"tokensOut":900}}"#;
    let path = write_envelope(&dir, "sess-b.messages.json", body);
    let session = agenttrace_core::parse_file(&path).expect("envelope parses");
    assert_eq!(session.model, "default", "no declaration keeps the lane model");
    assert!(
        (session.metrics.cost_estimated - 0.018).abs() < 1e-9,
        "default-row pricing expected, got {}",
        session.metrics.cost_estimated
    );
    for map_label in ["disclosure_counters", "wire_metadata"] {
        let polluted: Vec<String> = match map_label {
            "disclosure_counters" => session
                .metrics
                .disclosure_counters
                .keys()
                .filter(|k| k.starts_with("cline_modelinfo"))
                .cloned()
                .collect(),
            _ => session
                .metrics
                .wire_metadata
                .keys()
                .filter(|k| k.starts_with("cline_modelinfo"))
                .cloned()
                .collect(),
        };
        assert!(
            polluted.is_empty(),
            "no declaration must mint no cline_modelinfo surfaces in {map_label}: {polluted:?}"
        );
    }
}

/// A recorded `metrics.cost` still wins over row pricing (recorded-cost
/// precedence), while ATTRIBUTION still lands on the declared row.
#[test]
fn recorded_cost_wins_over_declared_row_pricing() {
    let dir = scratch_dir("recorded");
    let body = OPUS_ASSISTANT.replace(
        "\"metrics\":{\"tokensIn\":1500,\"tokensOut\":900}",
        "\"metrics\":{\"tokensIn\":1500,\"tokensOut\":900,\"cost\":0.0123}",
    );
    let path = write_envelope(&dir, "sess-c.messages.json", &body);
    let session = agenttrace_core::parse_file(&path).expect("envelope parses");
    assert_eq!(session.model, OPUS, "attribution is the declared id");
    assert!(
        (session.metrics.cost_estimated - 0.0123).abs() < 1e-9,
        "recorded cost at face value expected, got {}",
        session.metrics.cost_estimated
    );
}

/// An id with no catalog row is attributed AS DECLARED and disclosed —
/// never silently folded to the default row (the fold would report the
/// transcript as a cheap `default` session).
#[test]
fn unknown_declared_id_discloses_instead_of_folding_to_default() {
    let dir = scratch_dir("unknown");
    let unknown = "opaque-fleet-model-7";
    let body = OPUS_ASSISTANT.replace(OPUS, unknown);
    let path = write_envelope(&dir, "sess-d.messages.json", &body);
    let session = agenttrace_core::parse_file(&path).expect("envelope parses");
    assert_eq!(
        session.model, unknown,
        "unknown id keeps its identity instead of folding to default"
    );
    assert_eq!(
        session.metrics.disclosure_counters.get("cline_modelinfo_unpriced_ids"),
        Some(&1),
        "the unpriced declaration is disclosed"
    );
    let unpriced = session
        .metrics
        .wire_metadata
        .get("cline_modelinfo_unpriced")
        .expect("unpriced ids ride wire_metadata");
    assert!(
        unpriced.contains(unknown),
        "unpriced list must name the id, got {unpriced}"
    );
}

/// rm-1415 surface: `--search` rows carry the DECLARED model — the fix
/// rides the same attribution `SearchResult.model` already reads, so a
/// body hit on a declared-cline transcript must not report `default`.
#[test]
fn search_rows_carry_the_declared_model() {
    let dir = scratch_dir("search");
    let path = write_envelope(&dir, "sess-e.messages.json", &OPUS_ASSISTANT);
    let session = agenttrace_core::parse_file(&path).expect("envelope parses");
    let hits = search_sessions(std::slice::from_ref(&session), "razzmatazz", 10);
    assert!(!hits.is_empty(), "body hit must surface (rm-1415)");
    assert_eq!(hits[0].model, OPUS, "search row model is the declared id");
}

//! Report-layer numeric truthfulness contract (run 933058; landed numerals
//! rm-541 + rm-532).
//!
//! rm-541: `compute_overview`'s per-task-type token accumulation saturates at
//! i64::MAX instead of panicking (debug) or wrapping (release) when a journal
//! clamps usage to the i64 ceiling. The first test is red-first: against the
//! pre-fix code it dies with `attempt to add with overflow`. The second pins
//! the repo's own adversarial corpus, which had zero coverage — the suite was
//! green while `--overview` panicked on these exact fixtures. (This run minted
//! the fix campaign-locally as rm-529; the numeral was retired at integration
//! as a title-twin of the already-landed rm-541, which carries the code change
//! and this suite.)
//!
//! rm-532: the per-project context-trends output cost is derived per million
//! tokens and agrees with the totals row it sits beside (the old
//! `cost_per_output_token` divided dollars by single tokens and read 0.0).

use agenttrace_core::{compute_overview, context_trends, parse_file};
use std::path::PathBuf;

fn generated_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/generated")
        .join(name)
}

const SATURATING_SESSION: &str = r#"{"role":"session_meta","timestamp":"2026-10-06T10:00:00Z","ModelUsed":"claude-sonnet-4-5"}
{"role":"meta","ModelUsed":"claude-sonnet-4-5","Usage":{"input_tokens":1e300,"output_tokens":1e300}}
{"role":"user","content":"plan the migration","timestamp":"2026-10-06T10:00:00Z","ModelUsed":"claude-sonnet-4-5"}
{"role":"assistant","content":"Plan recorded.","timestamp":"2026-10-06T10:00:01Z","ModelUsed":"claude-sonnet-4-5"}
"#;

/// Two journals whose usage rows clamp to i64::MAX must saturate the
/// by-task-type totals, not panic or wrap (rm-541).
#[test]
fn overview_by_task_type_saturates_instead_of_overflowing() {
    let dir = std::env::temp_dir().join(format!("agenttrace-rm529-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let one = dir.join("one.jsonl");
    let two = dir.join("two.jsonl");
    std::fs::write(&one, SATURATING_SESSION).expect("write one");
    std::fs::write(&two, SATURATING_SESSION).expect("write two");

    let first = parse_file(&one).expect("parse one");
    let second = parse_file(&two).expect("parse two");
    assert_eq!(first.metrics.tokens_input, i64::MAX, "parser clamps");
    assert_eq!(second.metrics.tokens_input, i64::MAX, "parser clamps");

    let overview = compute_overview(&[first, second]);
    assert_eq!(overview.by_task_type.len(), 1, "same task type");
    let entry = overview
        .by_task_type
        .values()
        .next()
        .expect("one task-type row");
    assert_eq!(entry.sessions, 2);
    assert_eq!(entry.tokens_input, i64::MAX, "saturate once, no wrap");
    assert_eq!(entry.tokens_output, i64::MAX, "saturate once, no wrap");
    assert!(
        entry.cost.is_finite() && entry.cost >= 0.0,
        "cost stays finite"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// The committed adversarial corpus (usage 1e300) renders finite,
/// non-negative overview totals — previously an uncovered live panic (rm-541).
#[test]
fn adversarial_corpus_overview_totals_finite() {
    let one = generated_fixture("adversarial/session-one.jsonl");
    let two = generated_fixture("adversarial/session-two.jsonl");
    let first = parse_file(&one).expect("parse adversarial one");
    let second = parse_file(&two).expect("parse adversarial two");

    let overview = compute_overview(&[first, second]);
    assert!(
        !overview.by_task_type.is_empty(),
        "adversarial corpus yields rows"
    );
    for (task, entry) in &overview.by_task_type {
        assert!(entry.tokens_input >= 0, "{task}: input non-negative");
        assert!(entry.tokens_output >= 0, "{task}: output non-negative");
        assert!(
            entry.cost.is_finite() && entry.cost >= 0.0,
            "{task}: cost finite"
        );
    }
    let planning = overview
        .by_task_type
        .get("planning")
        .expect("fixtures infer planning");
    assert_eq!(planning.tokens_input, i64::MAX);
    assert_eq!(planning.tokens_output, i64::MAX);
}

/// Per-project output cost is per-million and agrees with the totals row
/// (rm-532).
#[test]
fn context_trends_project_cost_per_million_matches_totals() {
    let dir = std::env::temp_dir().join(format!("agenttrace-rm532-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("session.jsonl");
    std::fs::write(
        &path,
        r#"{"role":"session_meta","timestamp":"2026-10-06T11:00:00Z","ModelUsed":"claude-sonnet-4-5"}
{"role":"meta","ModelUsed":"claude-sonnet-4-5","Usage":{"input_tokens":1000,"output_tokens":500}}
{"role":"user","content":"review the report","timestamp":"2026-10-06T11:00:00Z","ModelUsed":"claude-sonnet-4-5"}
{"role":"assistant","content":"Reviewed.","timestamp":"2026-10-06T11:00:01Z","ModelUsed":"claude-sonnet-4-5"}
"#,
    )
    .expect("write session");

    let session = parse_file(&path).expect("parse session");
    assert!(
        session.metrics.cost_estimated > 0.0,
        "fixture prices at non-zero cost"
    );
    let trend = context_trends(&[session]);
    assert_eq!(trend.projects.len(), 1, "one project");
    let project = &trend.projects[0];
    assert!(
        project.output_cost_per_million_tokens > 0.0,
        "per-million value is non-zero (old field read 0.0)"
    );
    assert_eq!(
        project.output_cost_per_million_tokens, trend.totals.output_cost_per_million_tokens,
        "project row and totals row share one derivation"
    );

    std::fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// rm-902 (run 6b369f2f cycle-1 batch "cost truthfulness end-to-end"):
// serde_json parses floats bit-exactly (workspace feature
// `float_roundtrip`). The assess probe (e7015d58, /tmp/at-assess-a6e8/
// fparsetest, serde_json 1.0.150 default features) showed the default
// parser loses the LAST ULP on shortest-round-trip literals:
// 11.005401611328125 parsed back as 11.00540161132812322
// (bits 0x402602c3ffffffff) and 9007199254740993.0 as
// 9007199254740994.0. The session cache is serialized WITH
// serde_json::to_string and re-read with from_str, so every cached
// duration/cost could silently drift one ULP per refresh cycle. The
// feature arm makes parse(serialize(x)) == x bit-exactly; these tests
// were red against the default-feature build.
// ---------------------------------------------------------------------------

#[test]
fn serde_float_roundtrip_is_bit_exact_on_straddle_values() {
    // Shortest-literal straddlers: each value's exact f64 prints without
    // full mantissa digits, so the default fast-path parser (power-of-ten
    // accumulate, then one corrective multiply) can land one ULP low.
    for value in [
        11.005401611328125_f64,
        0.004500000000000001,
        2300.5000000000005,
        9007199254740993.0, // 2^53+1: one ULP above the exact integer
    ] {
        let text = serde_json::to_string(&value).unwrap();
        let back: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            back.as_f64().unwrap().to_bits(),
            value.to_bits(),
            "serialize/parse round-trip must be bit-exact for {text}"
        );
    }
}

#[test]
fn serde_float_roundtrip_is_bit_exact_through_a_cache_shaped_document() {
    // The exact assess PoC shape: a session-cache style document whose
    // duration_sec straddles. Pre-fix, the re-read value's low bit was
    // lost (0x402602c400000000 became 0x402602c3ffffffff).
    let cache_doc = r#"{
        "schema_version": 35,
        "sessions": [
            {
                "id": "ulp-straddle",
                "duration_sec": 11.005401611328125,
                "cost_usd": 0.004500000000000001,
                "tokens": 9007199254740993.0
            }
        ]
    }"#;
    let value: serde_json::Value = serde_json::from_str(cache_doc).unwrap();
    let row = &value["sessions"][0];
    for (field, expected) in [
        ("duration_sec", 11.005401611328125_f64),
        ("cost_usd", 0.004500000000000001_f64),
        ("tokens", 9007199254740993.0_f64),
    ] {
        assert_eq!(
            row[field].as_f64().unwrap().to_bits(),
            expected.to_bits(),
            "cache field {field} must survive parse bit-exactly"
        );
    }
}

//! rm-053 token-accounting conformance harness: drives the checked-in
//! conformance pack (`testdata/conformance/`, manifest + fixtures) through
//! the library's parser and pricing paths and fails on any silent
//! double-count, mis-price, or unprovable total.
//!
//! The pack derives from the AgentMeasure 2026-09 audit taxonomy's five
//! recurring billing-bug classes (re-emitted events double-counted,
//! cache-tokens priced as input, price-table drift, resume/fork lineage
//! loss, overflow/clamp handling); its first cases import the rm-046
//! adversarial corpus verbatim, per rm-053's acceptance. Expected costs
//! are totals, never a second price table: this harness re-derives every
//! non-xfail cost literal from the bundled snapshot via `lookup_price`
//! and fails when a literal drifts from the live snapshot by more than
//! one round4 step — a snapshot refresh that moves a used model's rates
//! forces a pack refresh instead of silently re-blessing old totals.
//!
//! Case 012 is xfail-pinned to rm-663 (the parser currently freezes the
//! first assistant model onto every usage meta, so the per-block truth
//! is not reachable yet). When rm-663 lands, that case matches truth and
//! this harness fails loudly with a refresh instruction rather than
//! passing silently — flip it to a regular case in that landing's pack
//! refresh.
//!
//! Binary-level twins of these assertions live in
//! `scripts/conformance/run.sh` (end-to-end CLI shape), and the CI gate
//! lives in `.github/workflows/ci.yml` (`conformance` job).

use agenttrace_core::{lookup_price, parse_file, round4};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// The five audited AgentMeasure classes rm-053 must cover; the last two
/// entries are the pack's own extensions (unprovable totals rendered
/// honestly, hostile journals failing loudly).
const REQUIRED_CLASSES: [&str; 5] = [
    "overflow_clamp",
    "re_emitted_event",
    "cache_priced_as_input",
    "price_table_drift",
    "resume_fork_lineage",
];

fn pack_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/conformance")
}

fn manifest() -> Value {
    let raw = fs::read_to_string(pack_root().join("manifest.json"))
        .expect("testdata/conformance/manifest.json is checked in");
    serde_json::from_str(&raw).expect("manifest.json is valid JSON")
}

fn cases() -> Vec<Value> {
    manifest()
        .get("cases")
        .and_then(Value::as_array)
        .expect("manifest has a cases array")
        .clone()
}

fn case_session(case: &Value) -> agenttrace_core::Session {
    let fixture = case["fixture"].as_str().expect("case has a fixture");
    let path = pack_root().join(fixture);
    parse_file(&path).unwrap_or_else(|error| panic!("{}: {error}", case["id"]))
}

/// Re-derives a session total from the bundled snapshot through the same
/// catalog formula the renderer uses (round4 of the per-class token *
/// rate sum, cache classes at their own rates). `default` models price
/// at the builtin default entry.
fn snapshot_derived_cost(model: &str, tokens: &Value) -> f64 {
    let price = lookup_price(match model {
        "" | "unknown" => "default",
        m => m,
    });
    let term = |key: &str| tokens[key].as_f64().unwrap_or(0.0) / 1_000_000.0;
    round4(
        term("input") * price.input
            + term("output") * price.output
            + term("cache_w") * price.cw
            + term("cache_r") * price.cr,
    )
}

#[test]
fn conformance_pack_manifest_is_internally_consistent() {
    let cases = cases();
    assert!(cases.len() >= 7, "pack carries its rm-046 import family");

    let mut ids = HashSet::new();
    let mut classes = HashSet::new();
    for case in &cases {
        let id = case["id"].as_str().expect("case id");
        assert!(ids.insert(id.to_string()), "duplicate case id {id}");
        let fixture = case["fixture"].as_str().expect("case fixture");
        assert!(
            pack_root().join(fixture).is_file(),
            "{id}: fixture {fixture} is missing from the pack"
        );
        classes.insert(case["class"].as_str().expect("case class").to_string());

        // rm-053 acceptance: imports must stay verbatim copies of the
        // landed rm-046 corpus — never regenerated in place.
        if let Some(origin) = case["verbatim_of"].as_str() {
            if origin.starts_with("testdata/") {
                let origin_path = pack_root().join("../../").join(origin);
                let imported = fs::read(pack_root().join(fixture)).expect("imported fixture");
                let original = fs::read(&origin_path)
                    .unwrap_or_else(|error| panic!("{id}: origin {origin}: {error}"));
                assert_eq!(
                    imported, original,
                    "{id}: import of {origin} must stay byte-identical; regenerate the copy, \
                     never the corpus"
                );
            }
        }
    }

    for class in REQUIRED_CLASSES {
        assert!(
            classes.contains(class),
            "rm-053 pack must represent audited class {class}"
        );
    }

    // Case #1 is the rm-046 adversarial corpus import (acceptance clause).
    let first = &cases[0];
    assert!(
        first["fixture"]
            .as_str()
            .unwrap_or_default()
            .starts_with("cases/rm046-"),
        "first conformance case must be the rm-046 corpus import"
    );
    assert!(
        first["verbatim_of"]
            .as_str()
            .unwrap_or_default()
            .starts_with("testdata/generated/adversarial/"),
        "first conformance case must cite its rm-046 origin"
    );
}

#[test]
fn conformance_pack_holds_against_parser_and_pricing() {
    let tolerance = manifest()["pricing_basis"]["formula_tolerance"]
        .as_f64()
        .expect("formula tolerance");
    let mut asserted = 0usize;

    for case in cases() {
        let id = case["id"].as_str().expect("case id");
        let expect = &case["expect"];
        let fixture = pack_root().join(case["fixture"].as_str().expect("case fixture"));

        if expect["rc"].as_i64() == Some(1) {
            // Loud-parse arm: an unprovable journal must error, never
            // render a silent zero-total session.
            assert!(
                parse_file(&fixture).is_err(),
                "{id}: hostile journal must fail loudly instead of yielding a session"
            );
            asserted += 1;
            continue;
        }

        let session = case_session(&case);
        let metrics = &session.metrics;

        if let Some(model) = expect["model"].as_str() {
            assert_eq!(metrics.model_used, model, "{id}: model");
        }
        if let Some(tool) = expect["source_tool"].as_str() {
            assert_eq!(metrics.source_tool, tool, "{id}: source_tool");
        }

        let tokens = &expect["tokens"];
        let token_cases = [
            ("input", metrics.tokens_input),
            ("output", metrics.tokens_output),
            ("cache_w", metrics.tokens_cache_w),
            ("cache_r", metrics.tokens_cache_r),
            ("reasoning", metrics.tokens_reasoning),
        ];
        for (key, actual) in token_cases {
            let expected = tokens[key].as_i64().unwrap_or(0);
            assert_eq!(actual, expected, "{id}: tokens.{key}");
        }

        match &expect["cost_estimated"] {
            Value::Null => {
                assert!(
                    metrics.cost_estimated.is_finite() && metrics.cost_estimated >= 0.0,
                    "{id}: absurd totals must still price finite and non-negative"
                );
            }
            value => {
                let expected = value.as_f64().expect("cost literal is a number");
                let xfail = case.get("xfail").is_some();
                if xfail {
                    // xfail-pinned misprice: the current tree must NOT
                    // reach the truth; matching it means the pinned fix
                    // landed and the pack needs its refresh.
                    assert_ne!(
                        metrics.cost_estimated, expected,
                        "{id}: xfail-pinned case now matches its truth — the fix pinned to \
                         rm-663 landed; drop the xfail block as part of that landing's pack \
                         refresh"
                    );
                } else {
                    assert!(
                        (metrics.cost_estimated - expected).abs() < 1e-9,
                        "{id}: cost_estimated {} != expected {expected}",
                        metrics.cost_estimated
                    );
                    // Drift guard: the literal must stay derivable from
                    // the bundled snapshot (never an orphaned total).
                    let model = expect["model"].as_str().unwrap_or("default");
                    let derived = snapshot_derived_cost(model, tokens);
                    assert!(
                        (derived - expected).abs() <= tolerance,
                        "{id}: expected cost {expected} drifted from the snapshot-derived \
                         {derived}; refresh the pack alongside the snapshot",
                    );
                }
            }
        }

        if let Some(marker) = expect["cost_provenance"].as_str() {
            assert_eq!(metrics.provenance.cost, marker, "{id}: cost provenance");
        }
        if let Some(marker) = expect["tokens_provenance"].as_str() {
            assert_eq!(metrics.provenance.tokens, marker, "{id}: token provenance");
        }
        asserted += 1;
    }

    assert!(asserted >= 13, "every manifest case was asserted");
}

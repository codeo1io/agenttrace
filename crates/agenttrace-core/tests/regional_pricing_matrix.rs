//! rm-1053 contract: region-prefixed model ids price at their OWN
//! catalog row. LiteLLM carries regional twins of the Claude family —
//! `us.`, `au.`, `eu.`, `jp.`, `us-gov.anthropic.<model>` — as distinct
//! rows with distinct rates (us-gov bills $3.60/Mt in+out vs the
//! au/eu/jp $3.30/Mt vs base $3.00/Mt). Two catalog-pipeline defects
//! collapsed them (cycle-3 assessment F3, research R1):
//!
//! 1. build side — `convert_litellm` inserted every snapshot row under
//!    `normalize_model(row_key)`, so all regional twins collapsed onto
//!    one entry whose surviving rate was set by source iteration order
//!    (the lexicographically-first twin, `au.`, won);
//! 2. query side — `lookup_price_tiered`/`has_specific_price` ran
//!    `resolve_alias` (which normalizes) BEFORE
//!    `lookup_price_in`, so `match_variants`' raw-first arm never saw
//!    the un-normalized id.
//!
//! Red-first: at 3e90993 the us-gov fixture billed $3.3165 (the au
//! twin's rate) instead of $3.618. Post-fix each variant prices — and
//! attributes — at its own row, and the snapshot census below keeps
//! future refreshes from silently dropping the regional rows.

use agenttrace_core::{compute_overview, lookup_price_tiered, parse_file};
use std::fs;
use std::path::PathBuf;

struct Variant {
    /// Model id exactly as a transcript would carry it.
    model: &'static str,
    /// LiteLLM `input_cost_per_token` x 1e6 for this variant's own row.
    input_per_mtok: f64,
    output_per_mtok: f64,
}

const FAMILY: &[Variant] = &[
    Variant {
        model: "claude-sonnet-4-5",
        input_per_mtok: 3.0,
        output_per_mtok: 15.0,
    },
    Variant {
        model: "au.anthropic.claude-sonnet-4-5-20250929-v1:0",
        input_per_mtok: 3.3,
        output_per_mtok: 16.5,
    },
    Variant {
        model: "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
        input_per_mtok: 3.3,
        output_per_mtok: 16.5,
    },
    Variant {
        model: "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0",
        input_per_mtok: 3.6,
        output_per_mtok: 18.0,
    },
];

#[test]
fn regional_twins_lookup_at_their_own_rates() {
    for v in FAMILY {
        let price = lookup_price_tiered(v.model, None);
        assert!(
            (price.input - v.input_per_mtok).abs() < 1e-9,
            "{}: input {} Mt, want {}",
            v.model,
            price.input,
            v.input_per_mtok
        );
        assert!(
            (price.output - v.output_per_mtok).abs() < 1e-9,
            "{}: output {} Mt, want {}",
            v.model,
            price.output,
            v.output_per_mtok
        );
    }
}

#[test]
fn eu_and_jp_twins_survive_the_normalized_key_collapse() {
    // Same family, remaining two twins: both $3.30/Mt in+out, distinct
    // from base and us-gov. Guards the build-side fix against a
    // regression that keeps only one regional row per family.
    for model in [
        "eu.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "jp.anthropic.claude-sonnet-4-5-20250929-v1:0",
    ] {
        let price = lookup_price_tiered(model, None);
        assert!(
            (price.input - 3.3).abs() < 1e-9 && (price.output - 16.5).abs() < 1e-9,
            "{model}: priced {} / {} Mt, want the eu/jp twin row 3.3 / 16.5",
            price.input,
            price.output
        );
    }
}

#[test]
fn regional_twins_bill_at_their_own_rates_end_to_end() {
    // 1M input + 1K output per fixture: billed cost must be
    // input_per_mtok + output_per_mtok / 1000 at the variant's OWN row.
    let mut sessions = Vec::new();
    for (i, v) in FAMILY.iter().enumerate() {
        let path: PathBuf =
            std::env::temp_dir().join(format!("at-rm1053-{}-{i}.jsonl", std::process::id()));
        fs::write(
            &path,
            format!(
                concat!(
                    r#"{{"type":"user","timestamp":"2026-10-10T01:00:00Z","message":{{"role":"user","content":"regional billing check"}}}}"#, "\n",
                    r#"{{"type":"assistant","timestamp":"2026-10-10T01:00:05Z","message":{{"id":"msg_rm1053_{i}","model":"{model}","role":"assistant","content":[{{"type":"text","text":"ok"}}],"usage":{{"input_tokens":1000000,"output_tokens":1000,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#, "\n"
                ),
                model = v.model,
                i = i
            ),
        )
        .expect("write fixture");
        sessions.push(parse_file(&path).expect("session parses"));
        let _ = fs::remove_file(&path);
    }
    let overview = compute_overview(&sessions);
    for v in FAMILY {
        let bucket = overview
            .by_model
            .get(v.model)
            .unwrap_or_else(|| panic!("by_model bucket for {} exists", v.model));
        let want = v.input_per_mtok + v.output_per_mtok / 1000.0;
        assert!(
            (bucket.cost - want).abs() < 5e-4,
            "{}: billed ${:.4}, want ${:.4} (its own row)",
            v.model,
            bucket.cost,
            want
        );
    }
}

#[test]
fn snapshot_census_keeps_the_regional_rows() {
    // Refresh guard: a snapshot update that drops the regional twin
    // rows (or rewrites their rates) must fail here, not silently
    // fall back to the base row via normalization.
    let snapshot: serde_json::Value =
        serde_json::from_str(include_str!("../src/pricing_snapshot.json"))
            .expect("snapshot parses");
    let obj = snapshot.as_object().expect("snapshot is a map");
    for v in FAMILY {
        let row = obj
            .get(v.model)
            .unwrap_or_else(|| panic!("snapshot carries {}", v.model));
        let input = row["input_cost_per_token"].as_f64().unwrap();
        let output = row["output_cost_per_token"].as_f64().unwrap();
        assert!(
            (input * 1e6 - v.input_per_mtok).abs() < 1e-9,
            "{}: snapshot input {} — twin drift",
            v.model,
            input
        );
        assert!(
            (output * 1e6 - v.output_per_mtok).abs() < 1e-9,
            "{}: snapshot output {} — twin drift",
            v.model,
            output
        );
    }
    // The eu/jp twins too.
    for model in [
        "eu.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "jp.anthropic.claude-sonnet-4-5-20250929-v1:0",
    ] {
        assert!(
            obj.contains_key(model),
            "snapshot carries the regional twin {model}"
        );
    }
}

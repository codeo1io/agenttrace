//! rm-1053 (cycle-5 F2 refresh): regional model ids must bill and
//! attribute from their OWN snapshot row.
//!
//! The cycle-5 assessment PoC F2 built four regional sessions of one
//! model family and `--overview` rendered $19.80 for all of them while
//! the bundled snapshot prices the family $18.00–$21.60: key
//! normalization collapsed every region onto one catalog key and the
//! load-time winner selection priced all regions at the first-sorted
//! region's rates, while the provider path — which normalized the id a
//! second time — printed "anthropic" for rows that are
//! bedrock_converse SKUs. Red-first corpus preserved verbatim under
//! fixtures/rm-1053-region-family/ (1M in + 1M out per session, so the
//! per-Mtok row rates read directly as dollars):
//!
//! - us. / eu.  ($3.30/$16.50) -> $19.80
//! - us-gov.    ($3.60/$18.00) -> $21.60
//! - global.    ($3.00/$15.00) -> $18.00
//! - bare dated ($3.00/$15.00) -> $18.00
//!
//! Post-fix every session prices from its exact id's row and
//! `by_provider` cites that row's `litellm_provider`, never a
//! re-derived name-prefix guess.

use agenttrace_core::{compute_overview, parse_file};
use std::fs;
use std::path::PathBuf;

fn region_session(tag: &str) -> agenttrace_core::Session {
    // Distinct temp path per test invocation: the pid alone is not
    // enough — separate #[test] fns run as parallel threads in one
    // process, and two tests writing the same pid-keyed path for the
    // same tag raced in rm-949's original shape. Thread name + pid +
    // tag keeps every write private to its own test thread.
    let who = std::thread::current().name().unwrap_or("test").to_owned();
    let path: PathBuf = std::env::temp_dir().join(format!(
        "at-rm1053-{who}-{}-{tag}.jsonl",
        std::process::id()
    ));
    fs::write(&path, fixture(tag)).expect("write fixture");
    let parsed = parse_file(&path).expect("region session parses");
    let _ = fs::remove_file(&path);
    parsed
}

fn fixture(tag: &str) -> &'static str {
    match tag {
        "us" => include_str!("fixtures/rm-1053-region-family/us.jsonl"),
        "eu" => include_str!("fixtures/rm-1053-region-family/eu.jsonl"),
        "us-gov" => include_str!("fixtures/rm-1053-region-family/us-gov.jsonl"),
        "global" => include_str!("fixtures/rm-1053-region-family/global.jsonl"),
        "base" => include_str!("fixtures/rm-1053-region-family/base.jsonl"),
        other => panic!("unknown region fixture {other}"),
    }
}

#[test]
fn regional_sessions_bill_at_their_own_rows_rates() {
    let cases = [
        // (fixture, model id, expected cost at 1M/1M tokens)
        ("us", "us.anthropic.claude-sonnet-4-5-20250929-v1:0", 19.80),
        ("eu", "eu.anthropic.claude-sonnet-4-5-20250929-v1:0", 19.80),
        (
            "us-gov",
            "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0",
            21.60,
        ),
        (
            "global",
            "global.anthropic.claude-sonnet-4-5-20250929-v1:0",
            18.00,
        ),
        ("base", "claude-sonnet-4-5-20250929-v1:0", 18.00),
    ];
    for (tag, model, expected) in cases {
        let overview = compute_overview(&[region_session(tag)]);
        let bucket = overview
            .by_model
            .get(model)
            .unwrap_or_else(|| panic!("{tag} session must bucket under its exact model id"));
        assert!(
            (bucket.cost - expected).abs() < 1e-6,
            "{model} billed ${:.2}, expected ${expected:.2} from its own row",
            bucket.cost
        );
        assert_eq!(bucket.sessions, 1);
    }
}

#[test]
fn by_provider_cites_the_row_that_priced_the_session() {
    let sessions = [
        region_session("us"),
        region_session("eu"),
        region_session("us-gov"),
        region_session("global"),
        region_session("base"),
    ];
    let overview = compute_overview(&sessions);
    // Every row in the regional family is a bedrock_converse SKU;
    // the bare dated id is a bedrock row. Pre-fix this printed
    // "anthropic" for all five while pricing four of them from
    // bedrock_converse rows.
    let converse = overview
        .by_provider
        .get("bedrock_converse")
        .expect("regional rows attribute to bedrock_converse");
    assert_eq!(converse.sessions, 4);
    assert!(
        (converse.cost - 79.20).abs() < 1e-6,
        "four regional sessions must total $79.20 (19.80 + 19.80 + 21.60 + 18.00), got {:.2}",
        converse.cost
    );
    let bedrock = overview
        .by_provider
        .get("bedrock")
        .expect("bare dated id attributes to bedrock");
    assert_eq!(bedrock.sessions, 1);
    assert!((bedrock.cost - 18.00).abs() < 1e-6);
    assert!(
        !overview.by_provider.contains_key("anthropic"),
        "no session in this corpus prices an anthropic row; the label was a name-prefix guess"
    );
}

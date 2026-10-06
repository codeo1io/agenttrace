//! Report-layer numeric truthfulness contract (rm-529 + rm-532, run 933058).
//!
//! rm-529: `compute_overview`'s per-task-type token accumulation saturates at
//! i64::MAX instead of panicking (debug) or wrapping (release) when a journal
//! clamps usage to the i64 ceiling. The first test is red-first: against the
//! pre-fix code it dies with `attempt to add with overflow`. The second pins
//! the repo's own adversarial corpus, which had zero coverage — the suite was
//! green while `--overview` panicked on these exact fixtures.
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
/// by-task-type totals, not panic or wrap (rm-529).
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
/// non-negative overview totals — previously an uncovered live panic (rm-529).
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

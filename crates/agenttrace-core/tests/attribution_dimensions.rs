// rm-245 acceptance: provider and task-type attribution dimensions on
// the overview, plus the ranked top-cost-drivers lane. These tests pin
// the contract the roadmap item asked for - vendor attribution that
// comes from the pricing catalog (never invented from name prefixes,
// unresolvable models bucket explicitly under "unknown"), a documented
// deterministic task-type cascade, and a ranked cost-driver list - in
// the JSON, text, markdown and html surfaces.

use agenttrace_core::{
    compute_overview, demo_sessions, infer_task_type, report_overview_html, report_overview_json,
    report_overview_markdown, report_overview_text, Metrics, Session,
};
use serde_json::Value;
use std::collections::BTreeMap;

fn demo_report() -> Value {
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    serde_json::from_str(&report_overview_json(&overview, &sessions)).expect("valid json")
}

#[test]
fn overview_json_exposes_by_provider_dimension() {
    let report = demo_report();
    let providers = report["by_provider"]
        .as_array()
        .expect("by_provider is an array");
    assert!(
        !providers.is_empty(),
        "demo corpus has priced models, so by_provider must not be empty"
    );
    let session_sum: usize = providers
        .iter()
        .map(|item| item["sessions"].as_u64().unwrap_or(0) as usize)
        .sum();
    let total_sessions = report["summary"]["total_sessions"].as_u64().unwrap_or(0) as usize;
    assert_eq!(
        session_sum, total_sessions,
        "provider buckets must account for every session exactly once"
    );
    for item in providers {
        assert!(
            item["name"].as_str().is_some_and(|name| !name.is_empty()),
            "every provider bucket must carry a non-empty label"
        );
    }
    // The demo corpus prices via the LiteLLM snapshot, so its vendors -
    // anthropic and openai - must be attributed from the catalog rows
    // and not land in "unknown".
    let names: Vec<&str> = providers
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert!(
        names.contains(&"anthropic"),
        "claude models must attribute to anthropic via the catalog, got {names:?}"
    );
    assert!(
        names.contains(&"openai"),
        "gpt models must attribute to openai via the catalog, got {names:?}"
    );
    assert!(
        !names.contains(&"unknown"),
        "every demo model is priced, so nothing may bucket under unknown: {names:?}"
    );
}

#[test]
fn hostile_task_type_usage_saturates_instead_of_panicking_or_wrapping() {
    // rm-541 (run b1ff12f8, minted campaign-locally as rm-448, cycle 2): the task-type accumulator used a
    // bare `+=` (lib.rs task_type_entry.tokens_input/tokens_output), so
    // two hostile sessions each pinning i64::MAX output inside ONE bucket
    // aborted every aggregate surface in a debug build
    // (`attempt to add with overflow` on --overview/--audit/--context-trends/
    // --delivery-evidence/--mcp-governance/--recommend) and wrapped negative
    // in release, where reports.rs renders the bucket through `.max(0)` —
    // a 0-token planning bucket next to a summary claiming
    // total_tokens = 9223372036854775807. Both task-type sums must saturate
    // like every other token total in the codebase (governance.rs's
    // add_context_session comment is the precedent this fix cites).
    let mut sessions = demo_sessions().expect("demo sessions parse");
    assert!(
        sessions.len() >= 2,
        "demo corpus must supply at least two sessions for one bucket"
    );
    for session in sessions.iter_mut() {
        // Zero tool authority => every session classifies `planning`, the
        // same one-bucket setup the in-memory task-type test above uses.
        session.metrics.tool_authority.clear();
        session.metrics.tool_usage.clear();
        session.metrics.file_usage.clear();
        session.metrics.highest_authority = String::new();
        session.metrics.tool_calls_ok = 0;
        session.metrics.tool_calls_fail = 0;
        session.anomalies.clear();
        session.metrics.tokens_input = 1_000;
        session.metrics.tokens_output = i64::MAX;
    }
    let planning: Vec<&Session> = sessions
        .iter()
        .filter(|session| infer_task_type(session) == "planning")
        .collect();
    assert_eq!(
        planning.len(),
        sessions.len(),
        "hostile corpus must land in ONE task-type bucket for the overflow"
    );

    let overview = compute_overview(&sessions);
    let group = overview
        .by_task_type
        .get("planning")
        .expect("planning bucket");
    assert_eq!(
        group.sessions,
        sessions.len(),
        "every hostile session counts in the bucket"
    );
    assert_eq!(
        group.tokens_input,
        1_000 * sessions.len() as i64,
        "small inputs sum exactly even beside a saturated output"
    );
    assert_eq!(
        group.tokens_output,
        i64::MAX,
        "saturated bucket, not a debug abort and not the release wraparound \
         that reports.rs's .max(0) mask rendered as 0"
    );

    // Cross-check against the by-model baseline with the same saturating
    // arithmetic: the bucket total equals the by-model total exactly, so
    // the two dimensions can never disagree about how much was written.
    let mut by_model_output: i64 = 0;
    for session in &sessions {
        by_model_output = by_model_output.saturating_add(session.metrics.tokens_output);
    }
    assert_eq!(group.tokens_output, by_model_output);

    // The rendered JSON surface must show the saturated maximum, never the
    // wrapped-then-masked 0 the release build used to print.
    let report: Value = serde_json::from_str(&report_overview_json(&overview, &sessions))
        .expect("valid overview json");
    let rendered = report["by_task_type"]
        .as_array()
        .expect("by_task_type array")
        .iter()
        .find(|item| item["task_type"].as_str() == Some("planning"))
        .expect("planning row");
    assert_eq!(
        rendered["tokens"]["output"].as_i64(),
        Some(i64::MAX),
        "rendered bucket output saturates at i64::MAX, not 0"
    );
    assert_eq!(
        report["summary"]["total_tokens"].as_i64(),
        Some(i64::MAX),
        "summary total stays saturated-consistent with the bucket"
    );
}

#[test]
fn overview_json_exposes_by_task_type_dimension() {
    let report = demo_report();
    let task_types = report["by_task_type"]
        .as_array()
        .expect("by_task_type is an array");
    assert!(
        !task_types.is_empty(),
        "the task-type cascade classifies every session, so the rollup is never empty"
    );
    let mut session_sum = 0usize;
    for item in task_types {
        let label = item["task_type"].as_str().unwrap_or_default();
        assert!(
            matches!(label, "coding" | "debugging" | "planning"),
            "task-type taxonomy is fixed, got {label:?}"
        );
        assert!(
            item["tokens"]["input"].is_i64() && item["tokens"]["output"].is_i64(),
            "token totals must ride with the task-type rollup"
        );
        assert!(
            item["share_pct"].as_f64().is_some_and(|share| share >= 0.0),
            "cost share must be a non-negative number"
        );
        session_sum += item["sessions"].as_u64().unwrap_or(0) as usize;
    }
    let total_sessions = report["summary"]["total_sessions"].as_u64().unwrap_or(0) as usize;
    assert_eq!(
        session_sum, total_sessions,
        "task-type buckets must classify every session exactly once"
    );
}

#[test]
fn overview_json_exposes_ranked_top_cost_drivers() {
    let report = demo_report();
    let drivers = report["top_cost_drivers"]
        .as_array()
        .expect("top_cost_drivers is an array");
    assert_eq!(
        drivers.len(),
        3,
        "the demo corpus has three sessions, so the ranking shows all three"
    );
    let costs: Vec<f64> = drivers
        .iter()
        .filter_map(|item| item["cost"].as_f64())
        .collect();
    let mut sorted = costs.clone();
    sorted.sort_by(|a, b| b.total_cmp(a));
    assert_eq!(costs, sorted, "drivers must rank by cost, descending");
    for item in drivers {
        assert!(
            item["session"].as_str().is_some(),
            "each driver row names its session"
        );
        assert!(
            item["share_pct"]
                .as_f64()
                .is_some_and(|share| (0.0..=100.0).contains(&share)),
            "each driver row carries its share of total spend"
        );
    }
    let shares: f64 = drivers
        .iter()
        .filter_map(|item| item["share_pct"].as_f64())
        .sum();
    // All three demo sessions are ranked, so the shares must sum to the
    // full bill within rounding.
    assert!(
        (shares - 100.0).abs() < 0.5,
        "ranking the full corpus must account for ~100% of spend, got {shares}"
    );
}

#[test]
fn overview_surfaces_render_the_new_dimensions() {
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let text = report_overview_text(&overview, &sessions);
    assert!(text.contains("── By Provider ──"), "text: {text}");
    assert!(text.contains("── By Task Type ──"), "text: {text}");
    assert!(
        text.contains("── Top cost drivers (share of total) ──"),
        "text: {text}"
    );
    let markdown = report_overview_markdown(&overview, &sessions);
    assert!(markdown.contains("## By provider"), "markdown: {markdown}");
    assert!(markdown.contains("## By task type"), "markdown: {markdown}");
    assert!(
        markdown.contains("## Top cost drivers"),
        "markdown: {markdown}"
    );
    let html = report_overview_html(&overview, &sessions);
    assert!(html.contains("<h2>By provider</h2>"), "html");
    assert!(html.contains("<h2>By task type</h2>"), "html");
    assert!(html.contains("<h2>Top cost drivers</h2>"), "html");
}

#[test]
fn unpriceable_models_bucket_explicitly_under_unknown_provider() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    for session in sessions.iter_mut() {
        session.metrics.model_used = "definitely-not-a-catalog-model".to_string();
    }
    let overview = compute_overview(&sessions);
    let names: Vec<&str> = overview.by_provider.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        vec!["unknown"],
        "unpriceable models must not be dropped from the provider lane, got {names:?}"
    );
    let sessions_count: usize = overview
        .by_provider
        .values()
        .map(|group| group.sessions)
        .sum();
    assert_eq!(sessions_count, sessions.len());
}

#[test]
fn task_type_cascade_is_deterministic_over_aggregates() {
    let base = demo_sessions().expect("demo sessions parse");
    // The classification may only depend on parsed aggregates
    // (tool-mix and failure evidence), so two sessions with identical
    // aggregates must classify identically even with different names,
    // models and message content.
    let left = base[0].clone();
    let mut right = base[0].clone();
    right.name = "other-session".to_string();
    right.metrics.model_used = "some-other-model".to_string();
    assert_eq!(infer_task_type(&left), infer_task_type(&right));

    // Failure-driven work classifies as debugging: enough failed calls.
    let mut debugging = base[0].clone();
    debugging.metrics.tool_calls_ok = 3;
    debugging.metrics.tool_calls_fail = 1;
    debugging.metrics.tool_authority = BTreeMap::from([("write_files".to_string(), 3)]);
    debugging.anomalies.clear();
    assert_eq!(infer_task_type(&debugging), "debugging");

    // A single failure in a large otherwise-clean session is not
    // debugging - the >=25% strictness must hold.
    debugging.metrics.tool_calls_ok = 99;
    assert_eq!(infer_task_type(&debugging), "coding");

    // Write-capable authority with at least one call classifies as
    // coding even with zero failures.
    let mut coding = base[0].clone();
    coding.metrics.tool_calls_ok = 5;
    coding.metrics.tool_calls_fail = 0;
    coding.metrics.tool_authority = BTreeMap::from([("write_files".to_string(), 5)]);
    coding.anomalies.clear();
    assert_eq!(infer_task_type(&coding), "coding");

    // Read-only or zero-tool sessions classify as planning.
    let mut planning = base[0].clone();
    planning.metrics.tool_calls_ok = 7;
    planning.metrics.tool_calls_fail = 0;
    planning.metrics.tool_authority = BTreeMap::from([("read_only".to_string(), 7)]);
    planning.anomalies.clear();
    assert_eq!(infer_task_type(&planning), "planning");

    let mut no_tools = base[0].clone();
    no_tools.metrics.tool_calls_ok = 0;
    no_tools.metrics.tool_calls_fail = 0;
    no_tools.metrics.tool_authority = BTreeMap::new();
    no_tools.anomalies.clear();
    assert_eq!(infer_task_type(&no_tools), "planning");
}

#[test]
fn task_type_tokens_match_by_model_baseline() {
    // The task-type lane is a re-slice of the same parsed totals, so
    // its token sums must equal the by-model rollup's sums over the
    // same corpus.
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let mut by_type_input = 0i64;
    let mut by_type_output = 0i64;
    for group in overview.by_task_type.values() {
        by_type_input += group.tokens_input;
        by_type_output += group.tokens_output;
    }
    let mut by_model_input = 0i64;
    let mut by_model_output = 0i64;
    for session in &sessions {
        by_model_input += session.metrics.tokens_input;
        by_model_output += session.metrics.tokens_output;
    }
    assert_eq!(by_type_input, by_model_input);
    assert_eq!(by_type_output, by_model_output);
}

#[test]
fn infer_task_type_is_public_for_documentation_examples() {
    // The cascade is part of the overview contract; keep the entry
    // point public so docs and downstream users can classify the same
    // way the report does.
    let sessions = demo_sessions().expect("demo sessions parse");
    for session in &sessions {
        assert!(matches!(
            infer_task_type(session),
            "coding" | "debugging" | "planning"
        ));
    }
    let _: Option<Metrics> = None;
    let _: Option<&Session> = None;
}

use agenttrace_core::{
    add_baseline_comparison, compute_overview, demo_sessions, evaluate_overview_gate,
    report_json_with_language, report_overview_json, search_sessions, BaselineThresholds, Metrics,
    ReportLanguage, Session, VERSION,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

#[test]
fn demo_overview_exposes_ci_contract_fields() {
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report: Value =
        serde_json::from_str(&report_overview_json(&overview, &sessions)).expect("valid json");

    assert_eq!(report["version"], VERSION);
    assert_eq!(report["summary"]["total_sessions"], 3);
    assert_eq!(
        report["summary"]["tool_authority"]["highest"],
        "test_or_build"
    );
    assert!(report["summary"]["total_duration_seconds"]
        .as_f64()
        .is_some());
    assert!(report["summary"]["total_cost"].as_f64().unwrap() > 0.0);
    assert!(report["recent_sessions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|session| session["possible_cost_driver"]
            .as_str()
            .unwrap_or("")
            .contains("possible driver")));
    assert!(report["surfaces"]["authority_categories"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "test_or_build"));
    assert!(report["by_project"].is_array());
}

#[test]
fn range_and_project_filters_share_one_session_scope() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].cwd = "/work/alpha".to_string();
    sessions[1].cwd = "/work/beta".to_string();
    sessions[2].cwd = "/work/alpha".to_string();
    let now = chrono::DateTime::parse_from_rfc3339("2026-05-03T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let filtered = agenttrace_core::filter_sessions(
        &sessions,
        agenttrace_core::TimeRange::Days7,
        "alpha",
        "",
        "",
        now,
    );
    assert!(filtered
        .iter()
        .all(|session| session.cwd.ends_with("alpha")));
}

#[test]
fn demo_search_returns_metadata_evidence() {
    let sessions = demo_sessions().expect("demo sessions parse");
    let results = search_sessions(&sessions, "internal/ws", 20);
    assert_eq!(results.len(), 1);
    assert!(results[0]
        .matches
        .iter()
        .any(|item| item.contains("internal/ws")));
}

#[test]
fn overview_high_authority_tools_follow_go_classifier() {
    let metrics = Metrics {
        tool_usage: BTreeMap::from([
            ("bash".to_string(), 1),
            ("read_file".to_string(), 1),
            ("terminal".to_string(), 1),
            ("write_file".to_string(), 1),
        ]),
        tool_authority: BTreeMap::from([
            ("read_only_files".to_string(), 1),
            ("shell_exec".to_string(), 1),
            ("write_files".to_string(), 1),
        ]),
        highest_authority: "shell_exec".to_string(),
        ..Metrics::default()
    };

    let sessions = vec![Session {
        name: "authority".to_string(),
        path: "/tmp/authority.jsonl".to_string(),
        cwd: String::new(),
        branch: String::new(),
        metrics,
        anomalies: Vec::new(),
        health: 100,
        tool_warnings: Vec::new(),
        diagnostics: agenttrace_core::Diagnostics::default(),
    }];
    let overview = compute_overview(&sessions);
    let report: Value =
        serde_json::from_str(&report_overview_json(&overview, &sessions)).expect("valid json");
    let tools = report["surfaces"]["high_authority_tools"]
        .as_array()
        .expect("high authority tools");

    assert!(tools.iter().any(|item| item == "bash"));
    assert!(tools.iter().any(|item| item == "terminal"));
    assert!(tools.iter().any(|item| item == "write_file"));
    assert!(!tools.iter().any(|item| item == "read_file"));
}

#[test]
fn demo_latest_json_supports_zh_language_slice() {
    let sessions = demo_sessions().expect("demo sessions parse");
    let latest = sessions
        .iter()
        .max_by(|a, b| a.metrics.session_start.cmp(&b.metrics.session_start))
        .expect("latest demo session");
    let report: Value =
        serde_json::from_str(&report_json_with_language(latest, ReportLanguage::Zh))
            .expect("valid zh json");

    assert_eq!(report["session"]["duration_human"], "40秒");
    assert_eq!(
        report["anomalies"][0]["detail"],
        "平均推理 = 11 字符 (极浅)"
    );
    assert_eq!(report["anomalies"][1]["detail"], "无工具调用 — 纯对话会话");
}

#[test]
fn demo_baseline_comparison_is_stable_for_identical_report() {
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report = report_overview_json(&overview, &sessions);
    let path = std::env::temp_dir().join(format!(
        "agenttrace-rust-baseline-{}.json",
        std::process::id()
    ));
    fs::write(&path, &report).expect("write baseline");
    let (compared_json, breaches) = add_baseline_comparison(
        &report,
        path.to_str().unwrap(),
        BaselineThresholds {
            max_duration_delta_pct: 1.5,
            max_cost_delta_pct: 2.5,
            max_token_delta_pct: 3.5,
        },
    )
    .expect("baseline compare");
    let compared: Value = serde_json::from_str(&compared_json).expect("valid compared json");
    let _ = fs::remove_file(path);

    // Identical report vs itself: no threshold is breached, and the
    // breach booleans the exit-code gate reads (pass-7 P7-3) agree.
    assert!(!breaches.any());
    assert!(!breaches.slower_than_baseline);
    assert_eq!(
        compared["baseline_comparison"]["thresholds"]["max_duration_delta_pct"],
        1.5
    );
    assert_eq!(compared["baseline_comparison"]["cost_delta_pct"], 0.0);
    assert_eq!(compared["baseline_comparison"]["token_delta_pct"], 0.0);
    assert_eq!(
        compared["baseline_comparison"]["cost_above_threshold"],
        false
    );
    assert_eq!(
        compared["baseline_comparison"]["tokens_above_threshold"],
        false
    );
    assert!(compared["baseline_comparison"]["current"]["tools"].is_array());
    assert!(compared["baseline_comparison"]["new_tools"].is_array());
    assert!(compared["baseline_comparison"]["new_high_authority_tool_use"].is_array());
    assert_eq!(
        compared["baseline_comparison"]["slower_than_baseline"],
        false
    );
}

#[test]
fn demo_gate_fails_like_current_contract() {
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let failures = evaluate_overview_gate(&overview, &sessions, 80, true, Some(15.0));
    assert!(!failures.is_empty());
}

#[test]
fn baseline_non_overview_json_is_rejected_not_zero_filled() {
    // rm-569: `{"name": "not-an-overview"}` used to zero-fill both summaries
    // and fabricate a +100% regression; it must fail loudly with attribution.
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report = report_overview_json(&overview, &sessions);
    let path = std::env::temp_dir().join(format!(
        "agenttrace-bad-baseline-{}.json",
        std::process::id()
    ));
    fs::write(&path, r#"{"name": "not-an-overview"}"#).expect("write bad baseline");
    let err = add_baseline_comparison(
        &report,
        path.to_str().unwrap(),
        BaselineThresholds {
            max_duration_delta_pct: 1.5,
            max_cost_delta_pct: 2.5,
            max_token_delta_pct: 3.5,
        },
    )
    .expect_err("non-overview baseline must fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("is not an overview report"),
        "structural reason missing: {message}"
    );
    assert!(
        message.contains("--baseline"),
        "flag attribution missing: {message}"
    );
    assert!(
        message.contains(path.to_str().unwrap()),
        "baseline path missing: {message}"
    );
    assert!(
        message.contains("summary"),
        "expected shape hint missing: {message}"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn baseline_missing_file_is_attributed_to_the_flag() {
    // rm-569: a missing baseline must name the flag and the path instead of a
    // context-free "No such file or directory".
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report = report_overview_json(&overview, &sessions);
    let path = std::env::temp_dir().join(format!(
        "agenttrace-missing-baseline-{}.json",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    let err = add_baseline_comparison(
        &report,
        path.to_str().unwrap(),
        BaselineThresholds {
            max_duration_delta_pct: 1.5,
            max_cost_delta_pct: 2.5,
            max_token_delta_pct: 3.5,
        },
    )
    .expect_err("missing baseline must fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("--baseline"),
        "flag attribution missing: {message}"
    );
    assert!(
        message.contains("failed to read baseline file"),
        "io class missing: {message}"
    );
    assert!(
        message.contains(path.to_str().unwrap()),
        "baseline path missing: {message}"
    );
}

#[test]
fn baseline_invalid_utf8_is_reported_as_such() {
    // rm-569: non-UTF-8 bytes must surface as a UTF-8 error, attributed to
    // --baseline, not as a JSON parse error.
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report = report_overview_json(&overview, &sessions);
    let path = std::env::temp_dir().join(format!(
        "agenttrace-nonutf8-baseline-{}.json",
        std::process::id()
    ));
    fs::write(&path, [0xff, 0xfe, 0x00, 0x5a, 0xff]).expect("write non-utf8 baseline");
    let err = add_baseline_comparison(
        &report,
        path.to_str().unwrap(),
        BaselineThresholds {
            max_duration_delta_pct: 1.5,
            max_cost_delta_pct: 2.5,
            max_token_delta_pct: 3.5,
        },
    )
    .expect_err("non-utf8 baseline must fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("not valid UTF-8"),
        "utf-8 class missing: {message}"
    );
    assert!(
        message.contains("--baseline"),
        "flag attribution missing: {message}"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn baseline_version_mismatch_is_rejected() {
    // rm-569/rm-504: a structurally-valid overview from a different report
    // version must be rejected instead of comparing across versions.
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report = report_overview_json(&overview, &sessions);
    let mut baseline: Value = serde_json::from_str(&report).expect("report is json");
    baseline["version"] = Value::String("0.0.0-other".to_string());
    let path = std::env::temp_dir().join(format!(
        "agenttrace-versioned-baseline-{}.json",
        std::process::id()
    ));
    fs::write(&path, baseline.to_string()).expect("write versioned baseline");
    let err = add_baseline_comparison(
        &report,
        path.to_str().unwrap(),
        BaselineThresholds {
            max_duration_delta_pct: 1.5,
            max_cost_delta_pct: 2.5,
            max_token_delta_pct: 3.5,
        },
    )
    .expect_err("version-mismatched baseline must fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("version mismatch"),
        "version class missing: {message}"
    );
    assert!(
        message.contains("0.0.0-other"),
        "baseline version missing: {message}"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn baseline_empty_summary_object_is_rejected_not_zero_filled() {
    // Review fix (rm-569, 2026-10-06): {"summary":{}} used to pass the shape
    // check and zero-fill its comparisons into a fabricated +100% regression
    // ("Gate failed" on a healthy corpus).
    let path = std::env::temp_dir().join(format!(
        "agenttrace-empty-summary-baseline-{}.json",
        std::process::id()
    ));
    fs::write(&path, "{\"summary\":{}}").expect("write empty-summary baseline");
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report = report_overview_json(&overview, &sessions);
    let err = add_baseline_comparison(
        &report,
        path.to_str().unwrap(),
        BaselineThresholds {
            max_duration_delta_pct: 1.5,
            max_cost_delta_pct: 2.5,
            max_token_delta_pct: 3.5,
        },
    )
    .expect_err("empty-summary baseline must fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("missing the compared field")
            && message.contains("total_duration_seconds"),
        "empty-summary class missing: {message}"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn baseline_negative_totals_are_rejected_not_sign_inverted() {
    // rm-697: `Value::is_number` admitted negative summary totals, and
    // `delta_pct` divides by the baseline — total_cost = -100 turned a
    // real +$100 regression into a negative (passing) delta. Negative
    // or non-finite totals must be rejected at admission with the same
    // flag/path/regenerate-hint shape as the rm-569 rejections.
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report = report_overview_json(&overview, &sessions);
    let mut baseline: Value = serde_json::from_str(&report).expect("overview json is valid");
    baseline["summary"]["total_cost"] = serde_json::json!(-100.0);
    let path = std::env::temp_dir().join(format!(
        "agenttrace-negative-baseline-{}.json",
        std::process::id()
    ));
    fs::write(
        &path,
        serde_json::to_string(&baseline).expect("serialize negative baseline"),
    )
    .expect("write negative baseline");
    let err = add_baseline_comparison(
        &report,
        path.to_str().unwrap(),
        BaselineThresholds {
            max_duration_delta_pct: 1.5,
            max_cost_delta_pct: 2.5,
            max_token_delta_pct: 3.5,
        },
    )
    .expect_err("a negative summary total must be rejected, not compared against");
    let message = format!("{err:#}");
    assert!(
        message.contains("negative") && message.contains("total_cost"),
        "sign class and field missing: {message}"
    );
    assert!(
        message.contains("--baseline") && message.contains(path.to_str().unwrap()),
        "flag attribution missing: {message}"
    );
    assert!(
        message.contains("regenerate"),
        "regeneration hint missing: {message}"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn baseline_without_version_string_is_rejected() {
    // Review fix (rm-569, 2026-10-06): a baseline without a top-level "version"
    // used to skip the version cross-check silently.
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let report = report_overview_json(&overview, &sessions);
    let mut baseline: serde_json::Value =
        serde_json::from_str(&report).expect("report is valid json");
    baseline
        .as_object_mut()
        .expect("report root is an object")
        .remove("version");
    let path = std::env::temp_dir().join(format!(
        "agenttrace-unversioned-baseline-{}.json",
        std::process::id()
    ));
    fs::write(&path, baseline.to_string()).expect("write unversioned baseline");
    let err = add_baseline_comparison(
        &report,
        path.to_str().unwrap(),
        BaselineThresholds {
            max_duration_delta_pct: 1.5,
            max_cost_delta_pct: 2.5,
            max_token_delta_pct: 3.5,
        },
    )
    .expect_err("unversioned baseline must fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("no \"version\" string found"),
        "unversioned class missing: {message}"
    );
    let _ = fs::remove_file(&path);
}

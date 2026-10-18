//! rm-1500 contract — non-finite usage costs must never read as zero-spend.
//!
//! A session whose `cost_estimated` is NaN/±inf (poisoned pricing input, or a
//! NaN REAL column read back from a sqlite lane) is EXCLUDED from every
//! aggregate fold and disclosed via `Overview::non_finite_costs` — the same
//! vocabulary as the insights data-health census (pass-8 F8-5). `format_cost`
//! renders non-finite as a null marker, never `$0.0000`.

use agenttrace_core::{compute_overview, demo_sessions, format_cost};

fn sessions_with_one_poisoned_cost() -> Vec<agenttrace_core::Session> {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    assert!(
        !sessions.is_empty(),
        "demo corpus must provide sessions to poison"
    );
    // Poison the last session; remember its finite contribution so the
    // assertions can express exact deltas.
    let idx = sessions.len() - 1;
    sessions[idx].metrics.cost_estimated = f64::NAN;
    sessions
}

#[test]
fn non_finite_cost_is_excluded_from_total_and_disclosed() {
    let clean = demo_sessions().expect("demo sessions parse");
    let clean_overview = compute_overview(&clean);
    let poisoned_last = clean[clean.len() - 1].metrics.cost_estimated;
    assert!(
        poisoned_last.is_finite(),
        "demo corpus must have finite costs to poison"
    );

    let sessions = sessions_with_one_poisoned_cost();
    let overview = compute_overview(&sessions);

    assert_eq!(
        overview.non_finite_costs, 1,
        "the poisoned session must be counted, not silently dropped"
    );
    assert!(
        (overview.total_cost - (clean_overview.total_cost - poisoned_last)).abs() < 1e-9,
        "total_cost must be the finite remainder ({}), got {} (clean {})",
        clean_overview.total_cost - poisoned_last,
        overview.total_cost,
        clean_overview.total_cost
    );
}

#[test]
fn non_finite_cost_is_excluded_from_every_group_fold() {
    let sessions = sessions_with_one_poisoned_cost();
    let overview = compute_overview(&sessions);

    for (name, group) in overview.by_model.iter() {
        assert!(
            group.cost.is_finite(),
            "by_model[{name}] folded a non-finite cost ({})",
            group.cost
        );
    }
    for (name, group) in overview.by_provider.iter() {
        assert!(
            group.cost.is_finite(),
            "by_provider[{name}] folded a non-finite cost ({})",
            group.cost
        );
    }
    for (name, group) in overview.by_project.iter() {
        assert!(
            group.cost.is_finite(),
            "by_project[{name}] folded a non-finite cost ({})",
            group.cost
        );
    }
    for (name, group) in overview.by_task_type.iter() {
        assert!(
            group.cost.is_finite(),
            "by_task_type[{name}] folded a non-finite cost ({})",
            group.cost
        );
    }
    for (name, group) in overview.by_agent.iter() {
        assert!(
            group.cost.is_finite(),
            "by_agent[{name}] folded a non-finite cost ({})",
            group.cost
        );
    }
    // rm-1500 arm (run 22f0c980 cycle 2): the by_org fold is the last
    // overview group to gain the guard — a poisoned session used to
    // poison the whole org's cost, and org totals feed downstream
    // rankings that then ranked NaN-poisoned orgs as if they were
    // real spend.
    for (name, group) in overview.by_org.iter().filter(|(org, _)| !org.is_empty()) {
        assert!(
            group.cost.is_finite(),
            "by_org[{name}] folded a non-finite cost ({})",
            group.cost
        );
    }
}

#[test]
fn non_finite_cost_is_excluded_from_governance_folds() {
    // rm-1500 arm (run 22f0c980 cycle 2): cost_audit's per-agent rows
    // and the drift aggregate are governance's folds of the same
    // poisoned members — both now skip-while-counting like the
    // overview lanes.
    let sessions = sessions_with_one_poisoned_cost();
    let audit = agenttrace_core::cost_audit(&sessions);
    assert!(
        audit.total_estimated_cost.is_finite(),
        "cost_audit total folded a non-finite cost"
    );
    assert!(
        audit.stored_estimated_cost_usd.is_finite(),
        "cost_audit stored total folded a non-finite cost"
    );
    for row in audit.by_provider_model.iter() {
        assert!(
            row.stored_estimated_cost_usd.is_finite(),
            "cost_audit model row [{}/{}] folded a non-finite cost",
            row.provider,
            row.model
        );
        if let Some(cost) = row.estimated_cost_usd {
            assert!(
                cost.is_finite(),
                "cost_audit model row [{}/{}] re-priced a non-finite cost",
                row.provider,
                row.model
            );
        }
    }
}

#[test]
fn format_cost_renders_non_finite_as_a_null_marker_not_zero() {
    for poisoned in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let rendered = format_cost(poisoned);
        assert_ne!(
            rendered, "$0.0000",
            "non-finite must never read as clean zero-spend"
        );
        assert!(
            !rendered.contains(|c: char| c.is_ascii_digit()),
            "non-finite render must carry no digits, got {rendered:?}"
        );
        assert_eq!(rendered, "—", "pinned null marker for human lanes");
    }
    // Finite rendering is unchanged.
    assert_eq!(format_cost(0.0), "$0.0000");
    assert_eq!(format_cost(1.5), "$1.5000");
}

#[test]
fn overview_render_lanes_disclose_the_excluded_cost_sessions() {
    let sessions = sessions_with_one_poisoned_cost();
    let overview = compute_overview(&sessions);

    // Text lane: the disclosure must appear beside the totals.
    let text = agenttrace_core::report_overview_text(&overview, &sessions);
    assert!(
        text.contains("Non-finite costs"),
        "text overview must disclose the non-finite exclusion; got: {text}"
    );

    // JSON lane: the counter is a first-class summary field.
    let json = agenttrace_core::report_overview_json(&overview, &sessions);
    assert!(
        json.contains("\"non_finite_costs\": 1"),
        "overview JSON summary must carry non_finite_costs=1, got: {}",
        &json[json.find("\"summary\"").map(|i| i.min(200)).unwrap_or(0)
            ..(json.find("\"summary\"").map(|i| i + 400).unwrap_or(400)).min(json.len())]
    );
}

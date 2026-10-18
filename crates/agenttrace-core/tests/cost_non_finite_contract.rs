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
}

/// rm-1549: the by_org facet follows the same skip-while-counting
/// discipline as every other cost facet — a poisoned session must not
/// poison its org bucket, and healthy spend in the same org stays visible.
#[test]
fn non_finite_cost_is_excluded_from_by_org_fold() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    let idx = sessions.len() - 1;
    let healthy_cost = sessions[idx].metrics.cost_estimated;
    assert!(
        healthy_cost.is_finite(),
        "demo corpus must have finite costs to poison"
    );
    // Pin an org onto the poisoned session and (when the corpus allows) a
    // healthy sibling in the SAME org, so the bucket provably mixes the two.
    let sibling = idx.saturating_sub(1);
    sessions[idx].org = "acme".to_string();
    let (expected_sessions, expected_cost) = if sibling != idx {
        sessions[sibling].org = "acme".to_string();
        (2, sessions[sibling].metrics.cost_estimated)
    } else {
        (1, 0.0)
    };
    sessions[idx].metrics.cost_estimated = f64::NAN;

    let overview = compute_overview(&sessions);
    assert_eq!(
        overview.non_finite_costs, 1,
        "the poisoned org session must be counted, not silently dropped"
    );
    for (name, group) in overview.by_org.iter() {
        assert!(
            group.cost.is_finite(),
            "by_org[{name}] folded a non-finite cost ({})",
            group.cost
        );
    }
    let acme = overview.by_org.get("acme").expect("org bucket must exist");
    assert_eq!(
        acme.sessions, expected_sessions,
        "both org-carrying sessions must be counted"
    );
    assert!(
        (acme.cost - expected_cost).abs() < 1e-9,
        "by_org['acme'] must carry only the finite sibling spend ({expected_cost}), got {}",
        acme.cost
    );
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

/// rm-1996 contract — the --audit lane must uphold the same
/// exclude-and-disclose posture as compute_overview. The old plain
/// `.sum()` folded a non-finite cost into `stored_estimated_cost_usd`
/// (and its mirrored `total_estimated_cost`), nulling the totals
/// through serde with zero disclosure.
#[test]
fn cost_audit_excludes_non_finite_costs_with_a_named_census() {
    use agenttrace_core::cost_audit;

    let clean = demo_sessions().expect("demo sessions parse");
    let clean_total: f64 = clean.iter().map(|s| s.metrics.cost_estimated).sum();

    let sessions = sessions_with_one_poisoned_cost();
    let audit = cost_audit(&sessions);

    assert!(
        audit.stored_estimated_cost_usd.is_finite(),
        "the poisoned session must not fold into the audit total (got {})",
        audit.stored_estimated_cost_usd
    );
    assert!(
        (audit.stored_estimated_cost_usd
            - (clean_total - clean[clean.len() - 1].metrics.cost_estimated))
            .abs()
            < 1e-6,
        "the audit total must be the finite remainder"
    );
    assert_eq!(
        audit.non_finite_cost_sessions, 1,
        "the poisoned session must be censused, not silently dropped"
    );

    // The document must carry no null totals and must carry the census.
    let json = serde_json::to_value(&audit).expect("audit serializes");
    assert!(
        json.get("stored_estimated_cost_usd").unwrap().is_f64(),
        "serde must render a finite total, not null"
    );
    assert!(
        json.get("non_finite_cost_sessions").is_some(),
        "the census key must be present when non-zero"
    );
}

#[test]
fn cost_audit_zero_census_stays_silent_on_clean_corpora() {
    use agenttrace_core::cost_audit;

    let clean = demo_sessions().expect("demo sessions parse");
    let audit = cost_audit(&clean);
    assert_eq!(audit.non_finite_cost_sessions, 0);
    let json = serde_json::to_value(&audit).expect("audit serializes");
    assert!(
        json.get("non_finite_cost_sessions").is_none(),
        "a clean corpus keeps the audit document unchanged in shape"
    );
}

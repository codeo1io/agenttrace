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

// --- rm-2249: round4 total closure + extreme-magnitude honesty ----

#[test]
fn round4_stays_finite_on_the_scaled_overflow_band() {
    // Red-first corpus: these finite magnitudes overflow `value * 1e4`
    // to +/-inf and used to manufacture a non-finite total that
    // poisoned every downstream fold (text `—` / JSON null / CSV inf).
    let overflow_band = [1.7e308, -1.7e308, 1.0e308, f64::MAX, f64::MIN];
    for value in overflow_band {
        let rounded = agenttrace_core::round4(value);
        assert!(
            rounded.is_finite(),
            "round4({value:e}) must never manufacture a non-finite total, got {rounded:e}"
        );
        // The scaled lattice carries no fractional information at this
        // magnitude, so returning the input unchanged is exact.
        assert_eq!(rounded, value);
    }
    // Just below the band the scaled value stays finite: still finite
    // out, with rounding semantics unchanged.
    let near_band = agenttrace_core::round4(1.79e304);
    assert!(near_band.is_finite());
    assert_eq!(agenttrace_core::round4(0.12345), 0.1235);
    assert_eq!(agenttrace_core::round4(-0.12345), -0.1235);
    assert_eq!(agenttrace_core::round4(1.0), 1.0);
}

#[test]
fn format_cost_extreme_band_renders_scientific_not_a_monster_literal() {
    // Red-first corpus: a finite but astronomically large recorded cost
    // rendered as a ~300-digit `{:.4}`-style literal on the text lanes
    // (and, pre-closure, as the `—` marker once round4 turned it
    // non-finite). The extreme band now renders compact scientific form.
    assert_eq!(format_cost(1.7e308), "$1.70e308");
    assert_eq!(format_cost(-1.7e308), "$-1.70e308");
    assert_eq!(format_cost(1.0e15), "$1.00e15");
    assert_eq!(format_cost(5.0e15), "$5.00e15");
    // The existing bands and the non-finite marker are unchanged.
    assert_eq!(format_cost(1.25), "$1.2500");
    assert_eq!(format_cost(f64::NAN), "—");
    assert_eq!(format_cost(f64::INFINITY), "—");
}

#[test]
fn recorded_extreme_cost_stays_finite_and_agrees_across_lanes() {
    // The exact manufacture corpus: a pi-v3 session journal whose
    // assistant `message.usage.cost.total` carries a finite but
    // astronomically large recorded cost. The parser admits it (finite,
    // >= 0); every downstream lane must agree on the same finite value.
    let raw = concat!(
        "{\"id\":\"s1\",\"timestamp\":\"2026-10-20T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp\"}\n",
        "{\"id\":\"e1\",\"parentId\":\"s1\",\"timestamp\":\"2026-10-20T10:00:01.000Z\",\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"probe\"}}\n",
        "{\"id\":\"e2\",\"parentId\":\"e1\",\"timestamp\":\"2026-10-20T10:00:05.000Z\",\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"ok\"}],\"model\":\"claude-sonnet-4-20250514\",\"usage\":{\"input\":10,\"output\":5,\"totalTokens\":15,\"cost\":{\"total\":1.7e308}}}}\n",
    );
    let path = std::env::temp_dir().join(format!("rm2249-poc-{}.jsonl", std::process::id()));
    std::fs::write(&path, raw).unwrap();
    let session = agenttrace_core::parse_file(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let cost = session.metrics.cost_estimated;
    assert!(
        cost.is_finite(),
        "cost_estimated must stay finite, got {cost:e}"
    );
    // JSON lane: a number, never null.
    let json = serde_json::to_value(cost).unwrap();
    assert!(
        json.is_number(),
        "JSON lane must serialize a number, got {json}"
    );
    // Text lane: no null marker, compact scientific, same magnitude.
    let rendered = format_cost(cost);
    assert_ne!(rendered, "—");
    assert_eq!(rendered, "$1.70e308");
}

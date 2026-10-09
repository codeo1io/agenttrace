//! rm-895 contract: one truncation policy for the overview by_* spend
//! families (agent / model / provider / branch) across every surface.
//!
//! Policy (single-sourced at `reports::GROUP_TAKE`):
//! - Human surfaces (text, html, markdown) render at most `GROUP_TAKE`
//!   groups per family, sorted by the ONE shared comparator
//!   (cost desc -> sessions desc -> name), and print an explicit
//!   `+N more not shown` marker whenever the full set is larger.
//! - Machine surfaces (json, csv) NEVER truncate: full data is the
//!   contract; they carry no marker because nothing is cut.
//!
//! Red-first: every assertion in this file failed against the
//! pre-rm-895 state where text capped at 8, html at 12 (agents
//! unbounded on both), markdown was unbounded, and no surface
//! disclosed a cut.

use agenttrace_core::{
    report_overview_html_with_context, report_overview_json_with_context,
    report_overview_markdown_with_context, report_overview_text_with_context, DataHealth,
    GroupOverview, Overview, TimeRange, GROUP_TAKE,
};
use std::collections::BTreeMap;

/// 15 groups per family, strictly descending cost (g01 = 15.0 ...
/// g15 = 1.0) so the top-12 cut is name-deterministic under any sane
/// comparator; every group has 1 session so session counts never
/// rescue a tie.
fn group(prefix: &str, i: usize) -> (String, GroupOverview) {
    (
        format!("{prefix}{i:02}"),
        GroupOverview {
            sessions: 1,
            cost: (16 - i) as f64,
        },
    )
}

fn fifteen_families_overview() -> Overview {
    let fill = |prefix: &str| -> BTreeMap<String, GroupOverview> {
        (1..=15).map(|i| group(prefix, i)).collect()
    };
    Overview {
        total_sessions: 15,
        total_cost: 15.0 * 15.0,
        by_agent: fill("agent"),
        by_model: fill("model"),
        by_provider: fill("prov"),
        by_branch: fill("br"),
        ..Overview::default()
    }
}

fn text() -> String {
    report_overview_text_with_context(
        &fifteen_families_overview(),
        &[],
        &DataHealth::default(),
        TimeRange::All,
        false,
    )
}
fn markdown() -> String {
    report_overview_markdown_with_context(
        &fifteen_families_overview(),
        &[],
        &DataHealth::default(),
        TimeRange::All,
        false,
    )
}
fn html() -> String {
    report_overview_html_with_context(
        &fifteen_families_overview(),
        &[],
        &DataHealth::default(),
        TimeRange::All,
        false,
    )
}
fn json() -> String {
    report_overview_json_with_context(
        &fifteen_families_overview(),
        &[],
        None,
        TimeRange::All,
        false,
        None,
        25,
    )
}

/// GROUP_TAKE is the single policy source: it exists, is public, and
/// every human surface renders exactly this many rows per family.
#[test]
fn group_take_const_is_the_single_policy_source() {
    let take = GROUP_TAKE;
    assert!(take >= 1);
    assert_eq!(GROUP_TAKE, 12, "policy change must be a one-line edit");
}

#[test]
fn text_caps_every_family_at_group_take_and_discloses_the_cut() {
    let out = text();
    let expected_marker = format!("+{} more not shown", 15 - GROUP_TAKE);
    let markers = out.matches(&expected_marker).count();
    assert_eq!(
        markers, 4,
        "text must mark the cut for agent/model/provider/branch, found {markers}: \n{out}"
    );
    for prefix in ["agent", "model", "prov", "br"] {
        assert!(
            out.contains(&format!("{prefix}01")),
            "top group missing: {prefix}01"
        );
        for cut in 13..=15 {
            let name = format!("{prefix}{cut:02}");
            assert!(
                !out.contains(&name),
                "cut group {name} rendered on text surface"
            );
        }
    }
}

#[test]
fn html_caps_every_family_at_group_take_and_discloses_the_cut() {
    let out = html();
    let expected_marker = format!("+{} more not shown", 15 - GROUP_TAKE);
    let markers = out.matches(&expected_marker).count();
    assert_eq!(
        markers, 4,
        "html must mark the cut for agent/model/provider/branch (incl. agents, previously unbounded)"
    );
    for prefix in ["agent", "model", "prov", "br"] {
        for cut in 13..=15 {
            let name = format!("{prefix}{cut:02}");
            assert!(
                !out.contains(&name),
                "cut group {name} rendered on html surface"
            );
        }
    }
}

/// markdown has by_agent / by_provider / by_branch sections today (no
/// by_model section — content gap, out of rm-895's unit): each present
/// family must honor the shared cap + marker.
#[test]
fn markdown_caps_present_families_at_group_take_and_discloses_the_cut() {
    let out = markdown();
    let expected_marker = format!("+{} more not shown", 15 - GROUP_TAKE);
    let markers = out.matches(&expected_marker).count();
    assert_eq!(
        markers, 3,
        "markdown: agent/provider/branch must disclose the cut"
    );
    for prefix in ["agent", "prov", "br"] {
        for cut in 13..=15 {
            let name = format!("{prefix}{cut:02}");
            assert!(
                !out.contains(&name),
                "cut group {name} rendered on markdown surface"
            );
        }
    }
}

/// Machine surface contract: json carries the FULL set — all 15 — and
/// therefore never prints a truncation marker.
#[test]
fn json_never_truncates_and_never_marks() {
    let out = json();
    for prefix in ["agent", "model", "prov", "br"] {
        for i in [1usize, 13, 15] {
            assert!(
                out.contains(&format!("{prefix}{i:02}")),
                "json lost {prefix}{i:02} — machine surfaces must be unbounded"
            );
        }
    }
    assert!(
        !out.contains("more not shown"),
        "json must not claim a cut; it renders the full set"
    );
}

/// Comparator unification: text and html must agree on WHICH 12
/// survive when cost ties force the deeper tiebreaks (sessions, then
/// name). g01/g02 tie at cost 15 with different session counts; g03/
/// g04 tie on cost AND sessions, so name decides.
#[test]
fn comparator_is_shared_across_surfaces() {
    let mut by_agent = BTreeMap::new();
    by_agent.insert(
        "a-b".to_string(),
        GroupOverview {
            sessions: 1,
            cost: 15.0,
        },
    );
    by_agent.insert(
        "a-a".to_string(),
        GroupOverview {
            sessions: 2,
            cost: 15.0,
        },
    );
    by_agent.insert(
        "a-d".to_string(),
        GroupOverview {
            sessions: 2,
            cost: 14.0,
        },
    );
    by_agent.insert(
        "a-c".to_string(),
        GroupOverview {
            sessions: 2,
            cost: 14.0,
        },
    );
    let overview = Overview {
        total_sessions: 4,
        by_agent,
        ..Overview::default()
    };
    let text = report_overview_text_with_context(
        &overview,
        &[],
        &DataHealth::default(),
        TimeRange::All,
        false,
    );
    let html = report_overview_html_with_context(
        &overview,
        &[],
        &DataHealth::default(),
        TimeRange::All,
        false,
    );
    let expected = ["a-a", "a-b", "a-c", "a-d"];
    for surface in [("text", &text[..]), ("html", &html[..])] {
        let positions: Vec<usize> = expected
            .iter()
            .map(|name| surface.1.find(name).expect("group missing"))
            .collect();
        let mut sorted_pos = positions.clone();
        sorted_pos.sort();
        assert_eq!(
            positions, sorted_pos,
            "surface {} must order cost desc, then sessions desc, then name",
            surface.0
        );
    }
}

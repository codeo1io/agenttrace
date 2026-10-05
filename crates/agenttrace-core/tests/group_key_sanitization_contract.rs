//! rm-506 contract: transcript-derived overview group keys (the
//! `model_used` strings that key `by_model` / `by_provider` /
//! `by_task_type`) are sanitized at render entry on every
//! human-readable lane, while the JSON lane keeps the lossless
//! policy (rm-383): hostile bytes stay visible to automation and can
//! never reach a terminal or spreadsheet.
//!
//! Corpus: the assess `8ee739a8` PoC sessions (replayed from
//! `tests/fixtures/hostile-overview/`), pinned to the exact lanes the
//! PoC leaked on — text `By Model` rows, the markdown recent-sessions
//! Model column, the html group and session tables, and (in
//! agenttrace-cli `tests/csv_export.rs`) the csv group tables. The
//! assess proof-of-concept: `--overview -f text | cat -A` rendered
//! `^[]0;pwned-title^Gvis^[[31mred` in a group row on the release
//! binary at base 700a67c.

use std::path::PathBuf;

use agenttrace_core::{
    compute_overview, parse_file, report_overview_html, report_overview_json,
    report_overview_markdown, report_overview_text, Session,
};

fn fixture(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("hostile-overview");
    path.push(name);
    assert!(
        path.exists(),
        "fixture corpus is missing: {}",
        path.display()
    );
    path
}

fn hostile_sessions(name: &str) -> Vec<Session> {
    let session = parse_file(&fixture(name)).unwrap_or_else(|error| {
        panic!(
            "hostile corpus {} must parse: {error}",
            fixture(name).display()
        )
    });
    vec![session]
}

#[test]
fn text_lane_strips_terminal_control_bytes_from_group_keys() {
    let sessions = hostile_sessions("osc-title.json");
    let overview = compute_overview(&sessions);
    let text = report_overview_text(&overview, &sessions);

    assert!(
        !text.contains('\u{1b}'),
        "ESC bytes must not reach the text lane"
    );
    assert!(
        !text.contains('\u{7}'),
        "BEL from the OSC title escape must not reach the text lane"
    );
    assert!(
        text.contains("\u{FFFD}]0;pwned-title\u{FFFD}vis"),
        "the printable OSC tail survives with its control bytes replaced by\n        U+FFFD (rm-383 semantics: only the control bytes are neutralized)"
    );
    assert!(
        text.contains('\u{FFFD}'),
        "stripped control bytes are disclosed as U+FFFD"
    );
}

#[test]
fn markdown_lane_strips_terminal_control_bytes_from_model_cells() {
    let sessions = hostile_sessions("osc-title.json");
    let overview = compute_overview(&sessions);
    let markdown = report_overview_markdown(&overview, &sessions);

    assert!(
        !markdown.contains('\u{1b}') && !markdown.contains('\u{7}'),
        "markdown cells (recent-sessions Model column, group tables) must not carry raw OSC bytes"
    );
    assert!(
        markdown.contains('\u{FFFD}'),
        "stripped control bytes are disclosed as U+FFFD"
    );
}

#[test]
fn html_lane_strips_terminal_control_bytes_from_group_keys_and_model_cells() {
    let sessions = hostile_sessions("osc-title.json");
    let overview = compute_overview(&sessions);
    let html = report_overview_html(&overview, &sessions);

    assert!(
        !html.contains('\u{1b}') && !html.contains('\u{7}'),
        "html cells (group tables, recent-sessions Model column) must not carry raw OSC bytes"
    );
    assert!(
        html.contains('\u{FFFD}'),
        "stripped control bytes are disclosed as U+FFFD"
    );
}

#[test]
fn json_lane_stays_lossless_for_hostile_group_keys() {
    // rm-383 boundary: sanitization happens at the renderer entry of
    // the human-readable lanes only. The JSON lane is automation's
    // record of what was actually seen, so the raw key survives there,
    // byte for byte.
    let sessions = hostile_sessions("osc-title.json");
    let overview = compute_overview(&sessions);
    let json = report_overview_json(&overview, &sessions);

    assert!(
        json.contains("pwned-title") && json.contains("31m"),
        "the JSON lane keeps the lossless raw model key"
    );
    assert!(
        !json.contains('\u{FFFD}'),
        "the JSON lane must not sanitize"
    );
}

#[test]
fn tab_led_formula_cells_never_reach_the_report_lanes_raw() {
    // The csv-injection corpus (a model named "\t=HYPERLINK(...)"):
    // the TAB is a terminal-control byte, so every report lane must
    // render it disclosed. The csv cell itself is additionally pinned
    // in agenttrace-cli tests/csv_export.rs.
    let sessions = hostile_sessions("csv-injection.json");
    let overview = compute_overview(&sessions);

    let markdown = report_overview_markdown(&overview, &sessions);
    assert!(
        !markdown.contains("\t=HYPERLINK"),
        "a tab-led formula cell must not reach the markdown lane raw"
    );

    let html = report_overview_html(&overview, &sessions);
    assert!(
        !html.contains("\t=HYPERLINK"),
        "a tab-led formula cell must not reach the html lane raw"
    );

    let text = report_overview_text(&overview, &sessions);
    assert!(
        !text.contains("\t=HYPERLINK"),
        "a tab-led formula cell must not reach the text lane raw"
    );
}

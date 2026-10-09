use agenttrace_core::{
    compute_overview, demo_sessions, report_overview_html, report_overview_json,
    report_overview_markdown, report_overview_text, report_text,
};

/// rm-239 E2E lane fixture (independent review 95d74221 F1, fix attempt
/// 4ef1c8d9): every TEXT lane must render ZERO raw terminal-control
/// bytes from transcript-derived fields — session titles, model keys
/// (`metrics.model_used` feeds the by-model/by-provider/by-task-type
/// overview rows, the last lane that still interpolated the raw key at
/// reports.rs:975), and cost-driver (tool) names. The JSON/Markdown
/// lanes must keep the payload verbatim — escaped by their own
/// mechanisms, never U+FFFD-substituted by the text-lane sanitizer.
/// MCP server names ride the CLI's governance lane, which routes
/// through the same `sanitize_line_segment` contract (rm-383/rm-239).
const OSC52: &str = "\u{1b}]52;c;aGVsbG8=\u{7}";
const CSI: &str = "\u{1b}[31mred";

fn assert_no_raw_terminal_control(lane: &str, label: &str) {
    for (index, c) in lane.char_indices() {
        let structural = matches!(c, '\n' | '\r' | '\t');
        let c1 = ('\u{80}'..='\u{9f}').contains(&c);
        assert!(
            !(!structural && (c.is_control() || c1 || c == '\u{7f}')),
            "{label} carries raw terminal-control byte {c:?} at byte {index}"
        );
    }
}

#[test]
fn text_lanes_substitute_terminal_control_bytes_from_transcript_fields() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].name = format!("evil{OSC52}{CSI}title");
    sessions[0].metrics.model_used = format!("model{OSC52}{CSI}name");
    sessions[0]
        .metrics
        .tool_usage
        .insert(format!("tool{OSC52}name"), 3);

    let overview = compute_overview(&sessions);
    let overview_text = report_overview_text(&overview, &sessions);
    assert_no_raw_terminal_control(&overview_text, "overview text");
    let session_text = report_text(&sessions[0]);
    assert_no_raw_terminal_control(&session_text, "session text");

    // The substitution is in place and the printable body survives:
    // only the control bytes (ESC/BEL) become U+FFFD.
    assert!(
        overview_text.contains("\u{FFFD}]52;c;aGVsbG8=\u{FFFD}"),
        "overview text must substitute control bytes in place, body retained"
    );
    // The by-model row itself (review F1): the hostile model key
    // arrives substituted, never raw.
    assert!(
        overview_text.contains("model\u{FFFD}]52;c;aGVsbG8="),
        "the by-model row must render the transcript-controlled model key through text_cell"
    );
    assert!(
        !overview_text.contains(OSC52) && !overview_text.contains(CSI),
        "raw OSC-52/CSI survived into the overview text lane"
    );

    // Non-text lanes never pass through text_cell: JSON escapes the
    // control bytes losslessly (\u001b/\u0007), and neither JSON nor
    // Markdown/HTML is U+FFFD-substituted by the text-lane sanitizer.
    let json = report_overview_json(&overview, &sessions);
    assert!(
        json.contains("model\\u001b]52;c;aGVsbG8=\\u0007"),
        "JSON must keep the model payload losslessly JSON-escaped"
    );
    assert!(
        !json.contains('\u{FFFD}'),
        "JSON lane must not be substituted"
    );
    let md = report_overview_markdown(&overview, &sessions);
    assert!(
        md.contains("aGVsbG8="),
        "Markdown keeps the printable payload body"
    );
    assert!(
        !md.contains('\u{FFFD}'),
        "Markdown lane must not be substituted"
    );
    let html = report_overview_html(&overview, &sessions);
    assert!(
        !html.contains('\u{FFFD}'),
        "HTML lane must not be substituted"
    );
}

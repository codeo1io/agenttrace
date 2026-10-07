use agenttrace_core::{compute_overview, demo_sessions, report_overview_markdown};

/// rm-403: the markdown report arm must not emit executable HTML from
/// transcript-derived fields. A model string carrying an `<img onerror>`
/// payload and a session name carrying `<script>` must arrive in the markdown
/// source entity-escaped, so no renderer that honors inline HTML executes
/// them. Mirrors the live adversarial PoC (assess 3e8493fa @ 7e17ac1: raw
/// `<img onerror>`/`<script>` landed at out.md:17,29 before this fix).
#[test]
fn markdown_cells_escape_html_from_transcript_derived_fields() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].metrics.model_used = "<img onerror=alert(1) src=x>".to_string();
    sessions[0].name = "sess-<script>alert(1)</script>".to_string();
    let overview = compute_overview(&sessions);
    let md = report_overview_markdown(&overview, &sessions);

    assert!(
        !md.contains("<img"),
        "raw <img survived into markdown output"
    );
    assert!(
        !md.contains("<script"),
        "raw <script> survived into markdown output"
    );
    // NOTE: the payload words (onerror/alert) legitimately remain inside the
    // escaped entity text; the guard is the unescaped tag bytes above.
    assert!(
        md.contains("&lt;img onerror=alert(1) src=x&gt;"),
        "model payload must be entity-escaped verbatim"
    );
    assert!(
        md.contains("sess-&lt;script&gt;alert(1)&lt;/script&gt;"),
        "session-name payload must be entity-escaped verbatim"
    );
}

/// rm-403 display-faithfulness: legitimate metacharacter content must still
/// render correctly after the escape — `&` arrives as `&amp;` in source
/// (renderers display `&`), table pipes stay escaped, and our own `<br>`
/// (the one raw HTML a plain cell may contain) is untouched by the escape.
#[test]
fn markdown_cell_escape_is_display_faithful() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].name = "a&b|c\nd".to_string();
    let overview = compute_overview(&sessions);
    let md = report_overview_markdown(&overview, &sessions);

    assert!(
        md.contains("a&amp;b\\|c<br>d"),
        "expected & to become &amp;, | to stay table-escaped and newline to become <br>"
    );
}

/// rm-764: terminal-injection payloads must die in the markdown and HTML
/// cell faces — the last two unsanitized render faces (text/csv already
/// sanitize; CSV group keys landed rm-540). Session names and group keys
/// carrying ESC/BEL/OSC sequences must arrive as U+FFFD, never as raw
/// control bytes that a paged/catted report could execute. Mirrors the
/// live adversarial PoC (assess 783c9f24 F2 @ aa5544af: raw `^[]0;PWNED^G^[[31m`
/// landed in both faces while text/csv rendered U+FFFD).
/// CR pin (independent review 8b4da943 F1): CommonMark treats a lone CR as a
/// line ending, so no raw `\r` may survive ANY markdown cell — `\r\n` and
/// lone `\r` translate to `<br>` exactly like `\n` (model strings carry
/// message.model verbatim, session names collapse whitespace upstream, so
/// the model lane is the reachable hostile path).
#[test]
fn markdown_and_html_cells_neutralize_control_bytes_from_names() {
    use agenttrace_core::report_overview_html;

    let hostile_name = "pwn\u{1b}]0;PWNED\u{7}\u{1b}[31mred\u{d}evil";
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].name = hostile_name.to_string();
    sessions[0].metrics.model_used = hostile_name.to_string();
    let overview = compute_overview(&sessions);
    let md = report_overview_markdown(&overview, &sessions);
    let html = report_overview_html(&overview, &sessions);

    for (face, rendered) in [("markdown", md.as_str()), ("html", html.as_str())] {
        assert!(
            !rendered.contains('\u{1b}'),
            "raw ESC survived into {face} output"
        );
        assert!(
            !rendered.contains('\u{7}'),
            "raw BEL survived into {face} output"
        );
        assert!(
            rendered.contains('\u{FFFD}'),
            "control bytes must be neutralized to U+FFFD in {face} output"
        );
    }
    // Review 8b4da943 F1: no raw CR may survive ANY markdown cell (CommonMark
    // treats lone CR as a line ending → row/cell break; terminal CR =
    // same-line overwrite spoofing). CR must translate to <br>, display-
    // faithfully, in every markdown cell it reaches (the html face keeps CR
    // as whitespace per the layout-byte contract — HTML parsers normalize it).
    assert!(
        !md.contains('\r'),
        "raw CR survived into markdown output (review 8b4da943 F1)"
    );
    assert!(
        md.contains("red<br>evil"),
        "CR must translate to <br> display-faithfully, not drop: {md:?}"
    );
    // The session-name cell specifically (not just the model column).
    let name_row = md
        .lines()
        .find(|line| line.contains("pwn"))
        .expect("hostile session name appears in the report");
    assert!(
        !name_row.contains('\u{1b}') && !name_row.contains('\u{7}'),
        "session-name cell still carries control bytes: {name_row:?}"
    );
    assert!(
        html.contains("PWNED\u{FFFD}") || html.contains("\u{FFFD}]"),
        "html neutralization happened in-place around the OSC payload"
    );
}

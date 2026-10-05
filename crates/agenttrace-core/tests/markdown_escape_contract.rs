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

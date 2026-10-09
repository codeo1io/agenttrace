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

/// rm-897: the markdown report arm must not re-arm spreadsheet formula
/// payloads from transcript-derived fields. The landed rm-540 guard covers the
/// CSV lane only; markdown cells currently arrive with a leading `=` intact
/// (live PoC, assess b0c5221b: a gitBranch `=SUM(1+1)` renders as a bare
/// `| =SUM(1+1) |` cell in `--overview -f markdown`, while the CSV lane writes
/// `'=SUM(1+1)`), and rm-585 (spend-by-branch) made group keys journal-
/// controlled free text — the first attacker-controllable table-key surface.
/// The markdown lane must mirror the CSV guard: a cell whose first
/// non-whitespace character is an introducer (= + - @, numeric `-` exempt) is
/// neutralized with a leading `'`.
#[test]
fn markdown_cells_neutralize_formula_introducers_from_transcript_derived_fields() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].name = "=SUM(1+1)".to_string();
    sessions[0].metrics.model_used = "=HYPERLINK(\"http://evil\";\"click\")".to_string();
    let overview = compute_overview(&sessions);
    let md = report_overview_markdown(&overview, &sessions);

    assert!(
        md.contains("'=SUM(1+1)"),
        "hostile session-name formula cell must be neutralized with a leading quote"
    );
    assert!(
        !md.contains("| =SUM(1+1)"),
        "bare formula cell must not survive in the markdown table"
    );
    assert!(
        !md.contains("| =HYPERLINK("),
        "bare formula cell must not survive for model keys either"
    );
}

/// rm-897 display-faithfulness mirror of the rm-540 CSV exemption: numeric
/// cells (negative costs, counts) keep the guard honest. Plain numbers —
/// including negatives — must stay bare; only NON-NUMERIC introducer cells
/// are marked.
#[test]
fn markdown_cell_formula_guard_is_numeric_faithful() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].name = "-42.50".to_string();
    sessions[0].metrics.model_used = "+1e3".to_string();
    let overview = compute_overview(&sessions);
    let md = report_overview_markdown(&overview, &sessions);

    assert!(
        md.contains("| -42.50"),
        "numeric negative cells must stay bare (CSV rm-540 exemption mirrored)"
    );
    assert!(
        md.contains("| +1e3"),
        "numeric plus-prefixed cells must stay bare"
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

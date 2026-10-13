use agenttrace_core::{
    compute_overview, parse_file, render_doctor_report, report_compare, report_overview_html,
    report_overview_markdown, report_overview_text, report_search_text, search_sessions, Session,
};
use std::path::Path;
use std::sync::Mutex;

/// Vendored byte-for-byte from the assess fdaf8b39 live PoC corpus
/// (delegate/fdaf8b39…-scratch/poc/corpus/=2+5|cmd.jsonl): a Claude Code
/// session whose FIRST user message is
/// `=HYPERLINK("http://evil.example","click-me") please review`.
/// The parser derives the session NAME from that message (lib.rs
/// first-user-request provenance), so every report lane renders the
/// formula payload as a cell.
const HOSTILE_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/formula-lane/=2+5|cmd.jsonl"
);
/// Zero-usage variant of the hostile corpus (review a3e986ef PoC
/// fixture, vendored verbatim): the vendored hostile session with one
/// appended all-zero usage block — the smallest corpus that drives the
/// doctor's zero-usage sample lane (rm-408) with a hostile session
/// name, the exact shape the blocking review PoC executed live.
const HOSTILE_ZERO_USAGE_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/formula-lane/doctor-zero-usage.jsonl"
);
const PAYLOAD_HEAD: &str = "=HYPERLINK(";
const PAYLOAD_FULL: &str = "=HYPERLINK(\"http://evil.example\",\"click-me\") please review";

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Pins HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR around the doctor arm the
/// same way pi_family_discovery.rs does (fleet env lock, one holder at a
/// time in this binary): `build_doctor_report` both READS and WRITES the
/// session cache under the user cache root, so a test must neither read
/// the real home nor leave a cache write behind (assess hygiene rule).
fn with_pinned_home(home: &Path, f: impl FnOnce()) {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let previous_home = std::env::var_os("HOME");
    let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
    let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
    let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
    let previous_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("HOME", home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
    std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    restore_env("HOME", previous_home);
    restore_env("XDG_CONFIG_HOME", previous_xdg_config);
    restore_env("XDG_CACHE_HOME", previous_xdg_cache);
    restore_env("XDG_DATA_HOME", previous_xdg_data);
    restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_cache);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

fn hostile_sessions() -> Vec<Session> {
    let session = parse_file(Path::new(HOSTILE_FIXTURE))
        .unwrap_or_else(|error| panic!("vendored fixture must parse: {error}"));
    // Provenance pin: the lane inputs below are the transcript-derived
    // session name, not a hand-built stand-in.
    assert_eq!(session.name, PAYLOAD_FULL);
    vec![session]
}

/// Every occurrence of the payload head in a human-readable lane must be
/// immediately preceded by the lane's paste-marker (`'` for text/markdown
/// lanes, `&#39;` for the HTML lane). Substring-absence cannot be the
/// assertion: the guarded form still CONTAINS the payload after the
/// marker — only the cell-start position proves neutralization. Used for
/// the lanes whose cells embed the session name directly (overview text/
/// html/markdown, compare).
fn assert_every_occurrence_preceded(output: &str, lane: &str, marker: &str) {
    for (index, _) in output.match_indices(PAYLOAD_HEAD) {
        assert!(
            output[..index].ends_with(marker),
            "{lane}: unguarded formula payload at byte {index}: {:?}",
            &output[index.saturating_sub(30)..(index + 40).min(output.len())]
        );
    }
}

/// The paste-relevant boundary for terminal text output is the LINE: no
/// trimmed line may BEGIN with the payload (a line that merely contains
/// it after prose — e.g. the search lane's `name: =HYPERLINK…` evidence
/// item behind its `- ` bullet — is a text cell in every spreadsheet,
/// because the cell-start character is the bullet/`n`, never the `=`).
/// Used for the search lane, whose evidence items embed the payload
/// mid-line by construction (`add_match` composes `field: snippet`).
fn assert_no_line_leads_with_payload(output: &str, lane: &str) {
    for line in output.lines() {
        let trimmed = line.trim_start();
        assert!(
            !trimmed.starts_with(PAYLOAD_HEAD),
            "{lane}: a pasted line would lead with the live formula: {trimmed:?}"
        );
    }
}

/// rm-985 (assess fdaf8b39 F1, CWE-1236): the spreadsheet-formula guard
/// family (CSV rm-540, markdown rm-897, sessions TSV 33eddd4) is now
/// routed through EVERY remaining human-readable render lane — the
/// overview text lane (text_cell chokepoint: incident timeline and
/// top-cost-driver rows shipped the raw `=HYPERLINK("http://evil.exa…`
/// session name, overview-text.txt:12/:20), the overview HTML lane
/// (html_escape chokepoint: `<td>=HYPERLINK(&#34;…` at
/// overview-html.txt:39/:43/:61), the search text lane and the compare
/// text lane. A session name that survives any of these lanes verbatim
/// re-arms as a live formula the moment the output is pasted into a
/// spreadsheet. Machine lanes (json) stay byte-lossless BY DESIGN and
/// are intentionally not routed through the guard.
#[test]
fn every_human_lane_neutralizes_the_hostile_formula_corpus() {
    let sessions = hostile_sessions();
    let overview = compute_overview(&sessions);

    let text = report_overview_text(&overview, &sessions);
    assert!(
        text.contains("'=HYPERLINK("),
        "text lane must carry the paste-marker: {text}"
    );
    assert_every_occurrence_preceded(&text, "overview text", "'");

    let html = report_overview_html(&overview, &sessions);
    assert!(
        html.contains("&#39;=HYPERLINK("),
        "html lane must carry the entity-escaped paste-marker: {html}"
    );
    assert_every_occurrence_preceded(&html, "overview html", "&#39;");

    // Control lane: markdown already routed (rm-897) — same corpus must
    // stay guarded there, proving the family is uniform, not special-cased.
    let markdown = report_overview_markdown(&overview, &sessions);
    assert!(markdown.contains("'=HYPERLINK("));
    assert_every_occurrence_preceded(&markdown, "overview markdown", "'");

    let results = search_sessions(&sessions, "please", 10);
    assert!(!results.is_empty(), "hostile name must match the query");
    let search_text = report_search_text(&results, "please");
    assert!(
        search_text.contains("\n'=HYPERLINK("),
        "search text lane must guard the result-name row: {search_text}"
    );
    // Evidence items embed the payload mid-line (`field: snippet`); the
    // paste boundary is the line, and no line may lead with the formula.
    assert_no_line_leads_with_payload(&search_text, "search text");

    let compare = report_compare(&sessions, "claude-sonnet-4");
    assert!(
        compare.contains("'=HYPERLINK("),
        "compare text lane must carry the paste-marker: {compare}"
    );
    assert_every_occurrence_preceded(&compare, "compare text", "'");

    // Doctor lane (rm-985 review-fix, review a3e986ef finding 1 — the
    // blocking PoC): the zero-usage sample list renders
    // `    {name}: N all-zero usage block(s)` with the transcript-derived
    // session name as the line-leading segment, and the pre-fix binary
    // emitted the raw `=HYPERLINK(…` introducer there. The arm drives the
    // REAL directory scan over the zero-usage variant of the vendored
    // corpus (env-pinned scratch home — doctor reads and writes the
    // session cache) and asserts the rendered text carries the
    // paste-marker on the sample line and leads no line with the payload.
    let doctor_home =
        std::env::temp_dir().join(format!("at-formula-lane-doctor-{}", std::process::id()));
    let corpus = doctor_home.join("sessions");
    std::fs::create_dir_all(&corpus).expect("doctor scratch dir");
    std::fs::copy(HOSTILE_ZERO_USAGE_FIXTURE, corpus.join("hostile.jsonl"))
        .expect("vendored zero-usage fixture copy");
    with_pinned_home(&doctor_home, || {
        let text = render_doctor_report(Some(&corpus), false, "text").expect("doctor text render");
        assert!(
            text.contains("Zero-usage reports: 1 session(s)"),
            "doctor arm must actually exercise the zero-usage sample lane: {text}"
        );
        assert!(
            text.contains(
                "'=HYPERLINK(\"http://evil.example\",\"click-me\") please review: 1 all-zero usage block"
            ),
            "doctor zero-usage sample must carry the paste-marker: {text}"
        );
        assert_no_line_leads_with_payload(&text, "doctor text");
    });
    std::fs::remove_dir_all(&doctor_home).expect("doctor scratch cleanup");
}

/// The guard must not damage benign numeric-looking transcript fields:
/// a `-42.50` or `+1e3` cell stays bare in the text and HTML lanes so
/// numeric columns still import as values (the numeric exemption the
/// CSV/markdown guards established, rm-540/rm-897).
#[test]
fn numeric_looking_fields_stay_bare_in_the_guarded_lanes() {
    let mut sessions = hostile_sessions();
    sessions[0].metrics.model_used = "-42.50".to_string();
    sessions[0].branch = "+1e3".to_string();
    let overview = compute_overview(&sessions);
    let text = report_overview_text(&overview, &sessions);
    let html = report_overview_html(&overview, &sessions);
    for output in [&text, &html] {
        assert!(
            !output.contains("'-42.50"),
            "numeric-exempt model cell must stay bare: {output}"
        );
        assert!(
            !output.contains("'+1e3"),
            "numeric-exempt branch cell must stay bare: {output}"
        );
    }
    // And the hostile name is still neutralized alongside them.
    assert!(text.contains("'=HYPERLINK("));
    assert!(html.contains("&#39;=HYPERLINK("));
}

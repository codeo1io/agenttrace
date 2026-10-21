//! rm-239 rider arms (run d80f6a25, cycle 1): `--compare` and `--waste`
//! were the last default text lanes rendering transcript-derived strings
//! raw (assess b6c398bf live PoC: one OSC-52 clipboard-write sequence
//! each, from a hostile session name and a hostile tool name). Both now
//! route through the shared `sanitize_line_segment` neutralizer, like
//! every other default text lane (rm-383 family). These contracts pin
//! the lane property end-to-end: a hostile name reaches the renderer,
//! and no control byte reaches the output — the neutralized bytes are
//! disclosed in place as U+FFFD instead.

use agenttrace_core::{
    compute_waste_report, parse_raw_session, render_waste_report, render_waste_report_prepared,
    report_compare, ReportLanguage, Session,
};

/// OSC-52 clipboard-write payload in the session name (the ESC + "]" +
/// ... + ESC + "\" framing), the exact shape assess planted in journals.
/// Claude-format sessions derive their display name from the FIRST USER
/// MESSAGE, so the injection rides the transcript content — carried as
/// JSON-escaped control bytes (`\u001b`), since strict JSON forbids raw
/// control characters in strings and transcripts therefore carry the
/// escaped form, which decodes to the same hostile bytes before
/// rendering. Short enough to survive the compare row's 27-rune budget
/// so the benign tail stays assertable.
const HOSTILE_SESSION_NAME_JSON: &str = "\\u001b]52;c;aGVsbG8=\\u001b\\\\audit";
const HOSTILE_SESSION_NAME_DECODED: &str = "\u{1b}]52;c;aGVsbG8=\u{1b}\\audit";

/// Same injection in a tool name, with the BEL terminator variant, as
/// JSON-escaped control bytes (`\u001b`) — strict JSON forbids raw
/// control characters in strings, so transcripts carry the escaped
/// form, which decodes to the same hostile bytes before rendering.
const HOSTILE_TOOL_NAME_JSON: &str = "\\u001b]52;c;dGVzdA==\\u0007deploy";

fn hostile_session() -> Session {
    let user = format!(
        r#"{{"type":"user","timestamp":"2026-10-04T01:00:00Z","cwd":"/tmp/rm239","message":{{"role":"user","content":"{HOSTILE_SESSION_NAME_JSON}"}}}}"#
    );
    let assistant = format!(
        r#"{{"type":"assistant","timestamp":"2026-10-04T01:00:05Z","message":{{"id":"msg_1","model":"claude-sonnet-4-5-20250929","role":"assistant","content":[{{"type":"tool_use","name":"{HOSTILE_TOOL_NAME_JSON}","input":{{}}}}],"usage":{{"input_tokens":100,"output_tokens":10,"cache_creation_input_tokens":2,"cache_read_input_tokens":3}}}}}}"#
    );
    let raw = format!("{user}\n{assistant}\n");
    let session = parse_raw_session("hostile", "/tmp/rm239/hostile.jsonl", &raw)
        .expect("hostile claude transcript parses");
    assert_eq!(
        session.name, HOSTILE_SESSION_NAME_DECODED,
        "fixture self-check: the claude lane must surface the hostile first user message as the session name"
    );
    assert!(
        session
            .metrics
            .tool_usage
            .contains_key("\u{1b}]52;c;dGVzdA==\u{07}deploy"),
        "fixture self-check: the hostile tool name must reach metrics.tool_usage, got {:?}",
        session.metrics.tool_usage.keys().collect::<Vec<_>>()
    );
    session
}

fn assert_no_control_bytes(report: &str, lane: &str) {
    let offenders: Vec<char> = report
        .chars()
        .filter(|c| c.is_control() && *c != '\n')
        .collect();
    assert!(
        offenders.is_empty(),
        "{lane} lane rendered control bytes to the terminal: {offenders:?}\nreport: {report}"
    );
}

#[test]
fn compare_text_row_neutralizes_osc52_in_session_names() {
    let report = report_compare(&[hostile_session()], "test-model");
    assert_no_control_bytes(&report, "--compare");
    assert!(
        report.contains('\u{FFFD}'),
        "the neutralized name must be disclosed in place (U+FFFD), got: {report}"
    );
    assert!(
        report.contains("audit"),
        "the benign tail of the sanitized name must survive layout, got: {report}"
    );
}

#[test]
fn waste_tool_bloat_line_neutralizes_osc52_in_tool_names() {
    let report = render_waste_report(&hostile_session());
    assert_no_control_bytes(&report, "--waste");
    assert!(
        report.contains('\u{FFFD}'),
        "the neutralized tool name must be disclosed in place (U+FFFD), got: {report}"
    );
    assert!(
        report.contains("deploy"),
        "the benign tail of the sanitized tool name must survive, got: {report}"
    );
}

// rm-933 slice-3 rider arms (run ff7615be, repository-maintenance 7f0a7958
// cycle 3): the invisible-spoofing Cf family (U+FEFF / U+2060 / U+180E)
// rode these text lanes RAW before the predicate widening — they are
// zero-width, non-directional, and pass every structural escaper, so a
// hostile session name could hide payload invisibly next to benign
// text. The directional set was already neutralized; these contracts
// pin the widened union through the same end-to-end lane, plus the
// OSC-7501 program-status payload class (pi 1.1.0 terminal-native
// status) alongside the OSC-52 clipboard-write class.

const HOSTILE_INVISIBLE_NAME_JSON: &str = "\\ufeff\\u2060\\u180Ejim\\u202epay";
const HOSTILE_INVISIBLE_NAME_DECODED: &str = "\u{feff}\u{2060}\u{180E}jim\u{202e}pay";
const HOSTILE_OSC7501_NAME_JSON: &str = "\\u001b]7501;agenttrace\\u001b\\\\status";

fn hostile_session_with(name_json: &str, expected_name: &str) -> Session {
    let user = format!(
        r#"{{"type":"user","timestamp":"2026-10-14T01:00:00Z","cwd":"/tmp/rm933","message":{{"role":"user","content":"{name_json}"}}}}"#
    );
    let raw = format!("{user}\n");
    let session = parse_raw_session("hostile", "/tmp/rm933/hostile.jsonl", &raw).expect("parses");
    assert_eq!(
        session.name, expected_name,
        "fixture self-check: hostile name reaches the session surface"
    );
    session
}

#[test]
fn compare_text_row_neutralizes_invisible_spoofing_cf_in_session_names() {
    let report = report_compare(
        &[hostile_session_with(
            HOSTILE_INVISIBLE_NAME_JSON,
            HOSTILE_INVISIBLE_NAME_DECODED,
        )],
        "test-model",
    );
    assert_no_control_bytes(&report, "--compare");
    let offenders: Vec<char> = report
        .chars()
        .filter(|c| agenttrace_core::is_spoofing_format_control(*c))
        .collect();
    assert!(
        offenders.is_empty(),
        "--compare rendered raw invisible/bidi spoofing chars: {offenders:?}"
    );
    assert!(
        report.contains('\u{FFFD}'),
        "disclosed in place, got: {report}"
    );
    assert!(
        report.contains("jim"),
        "benign tail survives, got: {report}"
    );
}

#[test]
fn compare_text_row_neutralizes_osc7501_program_status_payloads() {
    // rm-2111 (run 1c12a0bc cycle 3) retired the old premise here —
    // render_waste_report now DOES print the session name (the Session
    // subject line), so the name-bearing row contract lands on BOTH
    // lanes; the waste arm below pins it. The --compare lane keeps its
    // own arm because its rows print many names, not just the subject.
    let name = "\u{1b}]7501;agenttrace\u{1b}\\status";
    let report = report_compare(
        &[hostile_session_with(HOSTILE_OSC7501_NAME_JSON, name)],
        "test-model",
    );
    assert_no_control_bytes(&report, "--compare");
    assert!(
        report.contains('\u{FFFD}'),
        "the neutralized OSC-7501 payload must be disclosed in place (U+FFFD), got: {report}"
    );
    assert!(
        report.contains("status"),
        "benign tail survives, got: {report}"
    );
}

#[test]
fn waste_subject_line_neutralizes_hostile_session_names() {
    // rm-2111 (run 1c12a0bc cycle 3; assess bff3a140 F2): --waste now
    // names its subject (Session + Journal lines), so the hostile-name
    // contract lands on this lane too — both subject fields route
    // through the shared `sanitize_line_segment` neutralizer like every
    // other transcript-derived cell this file pins.
    let report = render_waste_report(&hostile_session());
    assert_no_control_bytes(&report, "--waste");
    assert!(
        report.contains('\u{FFFD}'),
        "the neutralized OSC-52 payload must be disclosed in place (U+FFFD), got: {report}"
    );
    assert!(
        report.contains("audit"),
        "benign tail survives, got: {report}"
    );
    assert!(
        report.contains("/tmp/rm239/hostile.jsonl"),
        "the subject Journal line names the parsed fixture path, got: {report}"
    );
}

#[test]
fn waste_subject_line_neutralizes_osc7501_program_status_payloads() {
    // rm-2111 companion arm: the OSC-7501 variant (program-status
    // reporting) rides the same Session subject line and must neutralize
    // identically — the name derives from the first user message, so
    // both terminator variants (ESC-\ and the BEL form in the tool
    // name) reach the renderer from one transcript.
    let name = "\u{1b}]7501;agenttrace\u{1b}\\status";
    let report = render_waste_report(&hostile_session_with(HOSTILE_OSC7501_NAME_JSON, name));
    assert_no_control_bytes(&report, "--waste");
    assert!(
        report.contains('\u{FFFD}'),
        "the neutralized OSC-7501 payload must be disclosed in place (U+FFFD), got: {report}"
    );
    assert!(
        report.contains("status"),
        "benign tail survives, got: {report}"
    );
}

#[test]
fn waste_skip_disclosure_neutralizes_hostile_skipped_paths() {
    // review fd3a37ab M1/F4 companion arm: the skip disclosure gained
    // NAMES alongside the count (bounded list in the text lane,
    // skipped_unparseable_paths in the json cell), and a skipped path
    // is operator-inspectable output derived from the filesystem walk
    // — the same sanitize_line_segment contract as the subject lines
    // above must hold for it. The CLI pre-sanitizes the paths on
    // ingest; this arm pins the RENDER side directly so a hostile
    // path can never reach the terminal even from a prepared report
    // (the json cell rides serde's control-byte escaping instead, so
    // only the text lane needs the terminal-safety pin).
    let hostile_path = "\u{1b}]52;c;aGVsbG8=\u{1b}\\audit.jsonl";
    let mut report = compute_waste_report(&hostile_session());
    report.skipped_unparseable = 1;
    report.skipped_unparseable_paths = vec![hostile_path.to_string()];
    let rendered = render_waste_report_prepared(&report, ReportLanguage::En);
    assert_no_control_bytes(&rendered, "--waste skips");
    assert!(
        rendered.contains('\u{FFFD}'),
        "the neutralized skipped path must be disclosed in place (U+FFFD), got: {rendered}"
    );
    assert!(
        rendered.contains("audit.jsonl"),
        "the benign tail of the skipped path must survive, got: {rendered}"
    );
    assert!(
        rendered.contains("Skipped: 1 discovered session file(s) failed to parse"),
        "the skip line must carry the count alongside the names, got: {rendered}"
    );
}

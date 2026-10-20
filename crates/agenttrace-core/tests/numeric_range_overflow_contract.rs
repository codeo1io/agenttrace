//! rm-342 contract (cycle 4, run 5712cb41 F1): a NUMBER literal beyond
//! f64 range is a disclosed numeric-range drop, never a silent torn
//! tail. serde_json refuses the WHOLE line with "number out of range"
//! when a single literal overflows (309+ nines, `1e400`), which used to
//! surface as generic `unparseable_line` — a usage-bearing line then
//! read as a writer crash while its accounting vanished with NO
//! `model_or_usage_dropped` marker (assess PoC: a claude_code line whose
//! `input_tokens` literal carried 309+ digits reported health 100 and
//! an empty disclosure channel on `--diagnostics`).
//!
//! The classifier oracle is Rust's `f64::from_str`, which is infinite
//! exactly where serde_json refuses (`parser::fails_only_on_numeric_
//! overflow`): every all-digits literal up to 308 digits is finite, so
//! the in-range twins below PARSE and count their usage, while the
//! 309/310-nine and `1e400` twins drop-and-disclose on their own
//! `numeric_range_overflow` class with the rm-616 accounting-loss
//! marker — through the same `metrics.line_skips` channel the rm-526
//! torn-tail contract pins, and folded by `--doctor` into its loss
//! tier. The boundary twins (301 vs 309 nines) pin that the classifier
//! fires on RANGE, not on "long number" — the assess probe's 308-digit
//! note is superseded by the oracle: 308 nines are 9.99e307, finite.

use agenttrace_core::{build_doctor_report, data_health, parse_file, parse_jsonl_session};
use std::fs;
use std::path::PathBuf;

fn head_lines() -> [String; 2] {
    [
        r#"{"type":"user","message":{"role":"user","content":"first question"},"timestamp":"2026-10-01T10:00:00Z"}"#
            .to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"answer one"}]},"timestamp":"2026-10-01T09:58:00Z"}"#
            .to_string(),
    ]
}

/// The session's only usage record, with the `input_tokens` literal
/// substituted: in-range twins count 100, overflow twins carry the
/// hostile literal.
fn usage_line(input_tokens: &str) -> String {
    format!(
        r#"{{"type":"assistant","id":"msg_01","model":"claude-sonnet-4-5-20250929","message":{{"id":"msg_01","role":"assistant","model":"claude-sonnet-4-5-20250929","usage":{{"input_tokens":{input_tokens},"output_tokens":20,"cache_read_input_tokens":50,"cache_creation_input_tokens":10}}}},"timestamp":"2026-10-01T10:03:00Z"}}"#
    )
}

fn fixture(tag: &str, lines: &[String]) -> PathBuf {
    // rm-1882 (rm-1065 discipline): pid+seq-unique scratch paths, same
    // class as the torn-tail contract's write site.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir()
        .join(format!(
            "at-numeric-range-{tag}-{}-{seq}.jsonl",
            std::process::id()
        ))
        .tap_write(lines)
}

/// Tiny tap so the fixture helper stays one expression per line.
trait TapWrite {
    fn tap_write(self, lines: &[String]) -> PathBuf;
}

impl TapWrite for PathBuf {
    fn tap_write(self, lines: &[String]) -> PathBuf {
        fs::write(&self, lines.join("\n") + "\n").expect("write fixture");
        self
    }
}

#[test]
fn in_range_digit_literals_parse_and_count_their_usage() {
    // The boundary's PARSE side: 25/40/100/301 nines are finite f64
    // (9.99e301 at the largest), so the line parses and the session's
    // only usage record is counted. `numeric_range_overflow` must not
    // appear — the classifier fires on range, not on length. The
    // 308-nine twin pins the boundary's edge: 9.99e307 is finite, so a
    // 308-digit literal still parses (the assess probe's 308-digit
    // saturation note was a downstream saturation, not a parse drop).
    for digits in [25, 40, 100, 301, 308] {
        let literal = "9".repeat(digits);
        let session = parse_file(&fixture(
            &format!("in-range-{digits}"),
            &[head_lines().to_vec(), vec![usage_line(&literal)]].concat(),
        ))
        .expect("in-range literal parses");
        assert!(
            !session
                .metrics
                .line_skips
                .contains_key("numeric_range_overflow"),
            "{digits} nines are finite and must not skip: {:?}",
            session.metrics.line_skips
        );
        assert!(
            !session
                .metrics
                .line_skips
                .contains_key("model_or_usage_dropped"),
            "{digits} nines: no usage was dropped, so no marker: {:?}",
            session.metrics.line_skips
        );
        assert!(
            session.metrics.tokens_input > 0,
            "{digits} nines: the usage record must be counted, not dropped"
        );
    }
}

#[test]
fn out_of_range_literals_drop_and_disclose_on_their_own_class() {
    // The boundary's DROP side: 309/310 nines (9.99e308 > f64::MAX),
    // `1e400` and `-1e400` are refused by serde_json for the whole
    // line. Each drop must carry its own class — never the generic
    // torn-tail class — and, because the line is the session's only
    // usage record, the rm-616 accounting-loss marker must fire with
    // it.
    let cases = [
        ("309-nines", "9".repeat(309)),
        ("310-nines", "9".repeat(310)),
        ("1e400", "1e400".to_string()),
        ("-1e400", "-1e400".to_string()),
    ];
    for (tag, literal) in &cases {
        let session = parse_file(&fixture(
            &format!("overflow-{tag}"),
            &[head_lines().to_vec(), vec![usage_line(literal)]].concat(),
        ))
        .expect("the session still parses — that is the hazard");
        assert_eq!(
            session.metrics.line_skips.get("numeric_range_overflow"),
            Some(&1),
            "{tag}: the overflow drop must carry its own class: {:?}",
            session.metrics.line_skips
        );
        assert_eq!(
            session.metrics.line_skips.get("model_or_usage_dropped"),
            Some(&1),
            "{tag}: a dropped usage record is lost ACCOUNTING (rm-616): {:?}",
            session.metrics.line_skips
        );
        assert_eq!(
            session.metrics.line_skips.get("unparseable_line"),
            None,
            "{tag}: an overflow-only drop must not masquerade as a torn tail: {:?}",
            session.metrics.line_skips
        );
        assert!(
            session.metrics.tokens_input < 100,
            "{tag}: the hostile literal's usage must not leak in: {}",
            session.metrics.tokens_input
        );
    }
}

#[test]
fn torn_tails_still_keep_their_own_class() {
    // The classifier must not eat the rm-526 contract: a genuinely torn
    // line — even one whose torn interior happens to contain
    // overflow-looking digits — stays `unparseable_line`, because the
    // clamp never touches string interiors and a line with a real
    // syntax error never re-parses.
    let torn_overflow_interior = format!(
        r#"{{"note":"{}", "usage":{{"input_tokens":100}}"#,
        "9".repeat(309)
    )[..86]
        .to_string();
    let torn = parse_file(&fixture(
        "torn-interior",
        &[head_lines().to_vec(), vec![torn_overflow_interior]].concat(),
    ))
    .expect("torn journal still parses — that is the hazard");
    assert_eq!(
        torn.metrics.line_skips.get("unparseable_line"),
        Some(&1),
        "a torn tail stays a torn tail: {:?}",
        torn.metrics.line_skips
    );
    assert_eq!(
        torn.metrics.line_skips.get("numeric_range_overflow"),
        None,
        "the classifier must not reclassify torn tails: {:?}",
        torn.metrics.line_skips
    );
}

#[test]
fn generic_fallback_lane_discloses_the_same_class_and_marker() {
    // The lib.rs fallback lane (`parse_jsonl_session`, P7-1) mirrors
    // the probe lanes' classifier — the class and the rm-616 marker
    // must survive there too, since that lane is what non-probe
    // formats fall back to.
    let overflow = format!(r#"{{"usage":{{"input_tokens":{}}}}}"#, "9".repeat(309));
    // The fallback lane refuses a corpus with zero events ("generic:
    // unable to parse as JSONL or JSON array"), so the overflow line
    // rides behind two healthy head lines — the skip map is per-line,
    // so the class and the marker still provenance from the dropped
    // line alone.
    let raw = [head_lines().to_vec(), vec![overflow]].concat().join("\n") + "\n";
    let session = parse_jsonl_session("fallback", "fallback.jsonl", &raw)
        .expect("the fallback lane never fails a corpus");
    assert_eq!(
        session.metrics.line_skips.get("numeric_range_overflow"),
        Some(&1),
        "fallback lane class: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session.metrics.line_skips.get("model_or_usage_dropped"),
        Some(&1),
        "fallback lane rm-616 marker: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn data_health_and_doctor_carry_the_new_class() {
    // Downstream surfaces, same pins as the rm-526 contract: data_health
    // renders the reason on the loss channel and drops confidence;
    // --doctor folds it into the loss tier (rm-862), never into the
    // informational disclosures.
    let overflow = format!(r#"{{"usage":{{"input_tokens":{}}}}}"#, "9".repeat(309));
    // Same zero-events refusal as the fallback-lane twin: the overflow
    // line rides behind the healthy head lines.
    let raw = [head_lines().to_vec(), vec![overflow]].concat().join("\n") + "\n";
    let session = parse_jsonl_session("dh", "dh.jsonl", &raw)
        .expect("the fallback lane never fails a corpus");
    let health = data_health(&[session], 1, 0);
    assert_eq!(
        health.line_skips.get("numeric_range_overflow"),
        Some(&1),
        "data_health carries the disclosure: {:?}",
        health.line_skips
    );
    assert_eq!(health.confidence, "low");

    // A dedicated scratch DIRECTORY for the doctor leg: it scans a
    // directory, and the fixture's parent must not be the shared temp
    // root — scanning all of /tmp would fold unrelated files into the
    // report and make the count assertions ambient.
    let dir = std::env::temp_dir().join(format!(
        "at-numeric-range-doctor-dir-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).expect("create doctor scratch dir");
    std::fs::write(
        dir.join("session.jsonl"),
        [head_lines().to_vec(), vec![usage_line(&"9".repeat(309))]]
            .concat()
            .join("\n")
            + "\n",
    )
    .expect("write doctor fixture");
    let report = build_doctor_report(Some(&dir), false);
    assert_eq!(
        report.dropped_lines.get("numeric_range_overflow"),
        Some(&1),
        "doctor aggregates the overflow drop into the loss tier: {:?}",
        report.dropped_lines
    );
    assert!(
        !report.disclosures.contains_key("numeric_range_overflow"),
        "parse loss never masquerades as a disclosed fact: {:?}",
        report.disclosures
    );
    let _ = std::fs::remove_dir_all(&dir);
}

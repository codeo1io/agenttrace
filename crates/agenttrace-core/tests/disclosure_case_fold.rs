//! rm-756 / rm-757 / rm-594-residual — usage-truth disclosure coverage.
//!
//! The run-9ab0afad assess (d85b3bf8, base 92149bd) pinned three
//! disclosure-channel truth gaps with live differentials; this file is
//! their contract, written red-first against the unfixed tree:
//!
//! * rm-756 (F1): `Event` deserializes the usage field from BOTH wire
//!   spellings (`#[serde(rename = "Usage", alias = "usage")]`), but the
//!   `unrecognized_usage_keys` classifier probed only lowercase
//!   `usage` / `message.usage` / `providerData.usage` — a capitalized
//!   `Usage` block was accepted by accounting's lenient lane while the
//!   disclosure channel reported nothing (`hermes-cap.jsonl`: ZERO
//!   `usage_*` entries where the lowercase twin disclosed two).
//! * rm-757 (F2): the top-level JSON-array lane in `parse_raw_session`
//!   handed `Vec<Event>` straight to `session_from_events`, never
//!   entering the per-line disclosure mint — array journals owed ZERO
//!   usage disclosures by construction (`arr-low.jsonl`).
//! * rm-594 residual (F4): the per-KEY sanitizer capped one hostile
//!   key's length, but nothing capped how many DISTINCT keys the
//!   wire-controlled channel minted — 50,000 unique `zz_*` keys became
//!   50,000 `line_skips` entries, a 2.7MB diagnostics JSON and a
//!   2.15MB single-line "Disclosed facts" render. The residual mints a
//!   distinct-key cap with a counter that discloses its own
//!   suppression, and bounds single-line renders.
//!
//! Fixtures are inline mirrors of the assess corpus shapes (the live
//! PoCs lived under /tmp/at-assess-d85b — deliberately not a test
//! dependency).

use std::fs;
use std::path::Path;

use agenttrace_core::{build_doctor_report, parse_file, render_doctor_report, Session};

fn parse_fixture(name: &str, body: &str) -> Session {
    let dir = std::env::temp_dir().join(format!("rm756-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("tempdir");
    let file = dir.join(format!("{name}.jsonl"));
    fs::write(&file, body).expect("fixture write");
    let session = parse_file(&file).expect("fixture parses");
    let _ = fs::remove_dir_all(&dir);
    session
}

/// Lowercase spelling (control arm): the meta line's canonical keys at
/// the accounting's read location stay silent; the assistant line's
/// top-level usage block is unconsumed and disclosed.
const HERMES_LOW: &str = concat!(
    r#"{"role":"session_meta","cwd":"/tmp/p","usage":{"input_tokens":100,"output_tokens":0}}"#,
    "\n",
    r#"{"role":"assistant","content":[{"type":"text","text":"hi"}],"usage":{"input_tokens":5,"output_tokens":7}}"#,
    "\n",
);

/// rm-756 F1 differential: identical except the assistant line spells
/// the field `Usage` — exactly what `Event`'s rename/alias accepts.
const HERMES_CAP: &str = concat!(
    r#"{"role":"session_meta","cwd":"/tmp/p","usage":{"input_tokens":100,"output_tokens":0}}"#,
    "\n",
    r#"{"role":"assistant","content":[{"type":"text","text":"hi"}],"Usage":{"input_tokens":5,"output_tokens":7}}"#,
    "\n",
);

/// rm-756 F1 nested differential: `message.Usage` (the lowercase twin
/// is the `hermes-msglow` control).
const HERMES_MSGCAP: &str = concat!(
    r#"{"role":"assistant","content":[{"type":"text","text":"hi"}],"message":{"Usage":{"input_tokens":3,"output_tokens":4}}}"#,
    "\n",
);

/// rm-757 F2: a top-level JSON-array journal whose usage owes two
/// disclosures — an unknown wire key (`zz_wire`) and a canonical key at
/// an unconsumed top-level location (`input_tokens` on a user line) —
/// plus the rm-756 interplay arm (capitalized `Usage` on the second
/// element).
const ARRAY_JOURNAL: &str = concat!(
    r#"[{"role":"user","content":"hi","usage":{"zz_wire":7,"input_tokens":9000}},"#,
    r#"{"role":"user","content":"again","Usage":{"input_tokens":11}}]"#,
);

/// rm-594 residual F4 shape: 1,500 lines each carrying one DISTINCT
/// unknown usage key — the per-key sanitizer's blind spot.
fn cardinality_journal(entries: usize) -> String {
    let mut body = String::new();
    for i in 0..entries {
        body.push_str(&format!(
            r#"{{"role":"user","content":"x","usage":{{"zz_{i}":1}}}}"#,
        ));
        body.push('\n');
    }
    body
}

fn usage_disclosure_count(session: &Session) -> usize {
    session
        .metrics
        .line_skips
        .keys()
        .filter(|key| key.starts_with("usage_"))
        .count()
}

#[test]
fn control_lowercase_top_level_usage_discloses() {
    // Control arm (green before AND after): lowercase spelling reaches
    // the classifier today; anchors the differential below.
    let session = parse_fixture("control-low", HERMES_LOW);
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:input_tokens"),
        Some(&1),
        "lowercase control discloses: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn capitalized_top_level_usage_discloses() {
    // rm-756: red on the unfixed tree (hermes-cap showed ZERO
    // disclosures); the fix folds the probe the way serde folds the
    // field.
    let session = parse_fixture("cap", HERMES_CAP);
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:input_tokens"),
        Some(&1),
        "capitalized Usage must disclose like the lowercase twin: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:output_tokens"),
        Some(&1)
    );
}

#[test]
fn capitalized_nested_message_usage_discloses() {
    // rm-756 nested arm: message.Usage (hermes-msgcap).
    let session = parse_fixture("msgcap", HERMES_MSGCAP);
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:input_tokens"),
        Some(&1),
        "message.Usage must disclose like message.usage: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:output_tokens"),
        Some(&1)
    );
}

#[test]
fn array_journal_mints_usage_disclosures() {
    // rm-757: red on the unfixed tree (arr-low showed no usage_*
    // disclosure anywhere in the JSON); also proves the rm-756 fold
    // rides the shared classifier into the array lane.
    let session = parse_fixture("array", ARRAY_JOURNAL);
    assert_eq!(
        session.metrics.line_skips.get("usage_unknown_key:zz_wire"),
        Some(&1),
        "array journals must mint unknown-key disclosures: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:input_tokens"),
        Some(&2),
        "both array elements' top-level usage blocks are unconsumed locations: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn disclosure_distinct_keys_are_capped_and_the_cap_discloses_itself() {
    // rm-594 residual mint arm: red on the unfixed tree (all 1,500
    // distinct keys entered line_skips and nothing named the growth).
    let session = parse_fixture("cardinality", &cardinality_journal(1_500));
    assert_eq!(
        usage_disclosure_count(&session),
        1_000,
        "distinct wire-controlled disclosure keys cap at 1,000: got {}",
        usage_disclosure_count(&session)
    );
    assert_eq!(
        session.metrics.line_skips.get("disclosure_keys_capped"),
        Some(&500),
        "the cap must count its own suppression: {:?}",
        session
            .metrics
            .line_skips
            .keys()
            .take(3)
            .collect::<Vec<_>>()
    );
    // Occurrences, not distinctness, drive existing counters: repeating
    // a suppressed key keeps counting into the cap disclosure.
    assert!(session
        .metrics
        .line_skips
        .keys()
        .all(|key| key.starts_with("usage_unknown_key:") || key == "disclosure_keys_capped"));
}

#[test]
fn single_line_renders_bound_the_disclosed_facts_row() {
    // rm-594 residual render arm: doctor's "Disclosed facts" line (the
    // 2.15MB single-line hazard) stays bounded and names what it holds
    // back. Red on the unfixed tree.
    let dir = std::env::temp_dir().join(format!("rm756-render-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("tempdir");
    fs::write(dir.join("cardinality.jsonl"), cardinality_journal(1_500)).expect("fixture write");

    let report = build_doctor_report(Some(&dir), false);
    assert!(
        report.disclosures.contains_key("disclosure_keys_capped"),
        "doctor folds the mint-side cap: {:?}",
        report.disclosures.len()
    );
    let text = render_doctor_report(Some(&dir), false, "text").expect("doctor text render");
    let disclosed_line = text
        .lines()
        .find(|line| line.starts_with("Disclosed facts:"))
        .expect("Disclosed facts row present");
    assert!(
        disclosed_line.contains("more distinct keys"),
        "bounded render names what it holds back: {disclosed_line}"
    );
    assert!(
        disclosed_line.chars().count() < 2_000,
        "single-line render stays bounded ({} chars): {}…",
        disclosed_line.chars().count(),
        disclosed_line.chars().take(120).collect::<String>()
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn capped_journal_still_records_every_event() {
    // The cap is a disclosure-channel bound only: parsing, event
    // counts and the non-disclosure skips channel stay untouched.
    let session = parse_fixture("cap-events", &cardinality_journal(1_500));
    assert_eq!(session.metrics.events_total, 1_500);
    assert!(
        !session.metrics.line_skips.contains_key("unparseable_line")
            && !session.metrics.line_skips.contains_key("event_schema"),
        "no parse loss on the cardinality corpus: {:?}",
        session
            .metrics
            .line_skips
            .keys()
            .take(5)
            .collect::<Vec<_>>()
    );
    // Path is exercised via parse_fixture for parity with the other
    // tests; keep the helper's contract obvious.
    assert!(Path::new(&session.path).ends_with("cap-events.jsonl"));
}

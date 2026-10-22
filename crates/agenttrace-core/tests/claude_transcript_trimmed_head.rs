//! rm-2271 — Claude Code transcript local-GC trimmed-head detection
//! (run d3bd7260 cycle 6, research 766fb081 C1).
//!
//! anthropics/claude-code CHANGELOG §2.1.304: "Fixed
//! CLAUDE_CODE_TRANSCRIPT_LOCAL_GC dropping messages from the saved
//! transcript when the disk filled up while the file was being
//! trimmed." The client therefore ships an env knob that TRIMS the
//! saved transcript journals this lane parses, and pre-2.1.304
//! disk-full trims could DROP messages from the head of the journal —
//! historical transcripts on real hosts can already be lossy. The
//! knob is undocumented (zero `TRANSCRIPT` occurrences in the live
//! code.claude.com settings page), so this contract pins the parser
//! to the DETECTABLE journal shape, never guessed knob internals: a
//! journal whose first content record is an `assistant` turn with a
//! non-null `parentUuid` (a dangling parent — the opening `user`
//! record the chain points at is gone) and no leading `summary` head
//! marker discloses `claude_transcript_trimmed_head` on the session's
//! `disclosure_counters` (per-session admission disclosure, riding
//! the same one-carrier channel as `claude_unmatched_type`).
//!
//! Negative discipline: a leading `summary` record is Claude Code's
//! own continuation/compaction marker — that head is deliberate, not
//! GC loss, and MUST NOT flag; a journal led by ANY `user` record
//! (fresh session, resume, fork) never flags. The disclosure is
//! additive only — accounting stays byte-identical: the parser never
//! fabricates the dropped usage and never refuses the session.

use std::fs;
use std::path::Path;

use agenttrace_core::{build_doctor_report, parse_file, render_doctor_report, Session};

const TRIMMED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-transcript-trimmed/trimmed-head.jsonl"
);
const CONTROL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-transcript-trimmed/control-full.jsonl"
);
const RESUME: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-transcript-trimmed/resume-summary-negative.jsonl"
);
/// A claude journal led by a `user` record that predates this
/// contract (the vendored rm-1522 corpus) must never flag.
const VENDORED_V2_1_300: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-code-2-1-300/request-reproduction.jsonl"
);

const KEY: &str = "claude_transcript_trimmed_head";

fn trimmed_counter(session: &Session) -> usize {
    session
        .metrics
        .disclosure_counters
        .get(KEY)
        .copied()
        .unwrap_or(0)
}

/// The trimmed head discloses, and the disclosure is the ONLY
/// difference: the trimmed journal and its untrimmed twin (the same
/// content records with the dropped opening `user` line restored so
/// the `parentUuid` chain completes) account identically — same
/// cost, same tokens — so detection can neither fabricate the lost
/// turn's usage nor penalize the session.
#[test]
fn trimmed_head_discloses_and_accounting_is_unchanged() {
    let trimmed = parse_file(Path::new(TRIMMED)).expect("trimmed corpus parses");
    let control = parse_file(Path::new(CONTROL)).expect("control corpus parses");
    assert_eq!(
        trimmed_counter(&trimmed),
        1,
        "assistant-led journal with dangling parentUuid must disclose {KEY}"
    );
    assert_eq!(
        trimmed_counter(&control),
        0,
        "user-led journal (chain complete) must not disclose {KEY}"
    );
    assert_eq!(
        trimmed.metrics.cost_estimated, control.metrics.cost_estimated,
        "detection must not change accounting (cost)"
    );
    assert_eq!(
        trimmed.metrics.tokens_input, control.metrics.tokens_input,
        "detection must not change accounting (input tokens)"
    );
    assert_eq!(
        trimmed.metrics.tokens_output, control.metrics.tokens_output,
        "detection must not change accounting (output tokens)"
    );
}

/// Claude Code's own continuation marker: a leading `summary` record
/// names a deliberate compacted/resumed head — the `assistant` turn
/// after it still carries a dangling `parentUuid`, and that shape
/// MUST NOT be called GC loss.
#[test]
fn leading_summary_head_is_not_trimmed() {
    let resumed = parse_file(Path::new(RESUME)).expect("resume corpus parses");
    assert_eq!(
        trimmed_counter(&resumed),
        0,
        "summary-led head is a deliberate continuation marker, never {KEY}"
    );
}

/// No vendored or inline user-led claude journal ever flags — the
/// regression guard for false positives across the existing corpora.
#[test]
fn user_led_journals_never_flag() {
    for fixture in [VENDORED_V2_1_300, CONTROL] {
        let session = parse_file(Path::new(fixture))
            .unwrap_or_else(|error| panic!("corpus {fixture} must parse: {error:#}"));
        assert_eq!(
            trimmed_counter(&session),
            0,
            "user-led journal {fixture} must not disclose {KEY}"
        );
    }
}

/// `--doctor` names the census: the per-session counter folds into
/// the disclosures map AND a dedicated `transcripts_trimmed_sessions`
/// row (0-count-clean — a healthy corpus renders byte-identically,
/// field absent from the JSON and text).
#[test]
fn doctor_report_names_the_trimmed_census() {
    let dir = std::env::temp_dir().join(format!("rm2271-doctor-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("tempdir");
    fs::copy(TRIMMED, dir.join("trimmed-head.jsonl")).expect("fixture copy");
    let report = build_doctor_report(Some(&dir), false);
    let text = render_doctor_report(Some(&dir), false, "text").expect("doctor text render");
    assert_eq!(
        report.transcripts_trimmed_sessions, 1,
        "doctor row must count the flagged session"
    );
    assert_eq!(
        report.disclosures.get(KEY).copied().unwrap_or(0),
        1,
        "disclosure fold must carry the per-session key"
    );
    assert!(
        text.contains("Trimmed transcripts"),
        "doctor text must surface the trimmed-transcript row: {text}"
    );
    let _ = fs::remove_dir_all(&dir);

    // 0-count-clean arm: a control-only directory renders no row and
    // carries no key.
    let dir = std::env::temp_dir().join(format!("rm2271-doctor-clean-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("tempdir");
    fs::copy(CONTROL, dir.join("control-full.jsonl")).expect("fixture copy");
    let report = build_doctor_report(Some(&dir), false);
    let text = render_doctor_report(Some(&dir), false, "text").expect("doctor text render");
    assert_eq!(
        report.transcripts_trimmed_sessions, 0,
        "healthy corpus must not count trimmed sessions"
    );
    assert!(
        !report.disclosures.contains_key(KEY),
        "healthy corpus must not carry the per-session key"
    );
    assert!(
        !text.contains("Trimmed transcripts"),
        "healthy corpus must render no trimmed-transcript row: {text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

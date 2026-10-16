//! rm-1522 (`--inspect N` contract truth, 2026-10-16, run 7288595f):
//! README and `--help` have always promised "Inspect a single session
//! by its `--sessions` list index (1-based)" — but the lane fed N to
//! the hidden inspect_first() triage list instead, so `--inspect 1`
//! silently rendered a priority pick that was NOT --sessions row 1
//! (live at base d2a02af: `--inspect 1` rendered the row-6 session,
//! identified by its 1h5m duration with zero identity in the report
//! body), ranks past the ≤5-entry triage list failed with "exceeds
//! available priority sessions" while many more rows were listable,
//! and the rendered report carried no session identity. These tests
//! pin the fixed contract end to end: the ordinal, the identity
//! header, the loud out-of-range message, the lossless JSON lane, and
//! the prose agreement (README flag-table row + `--help` doc +
//! rendered behavior).
//!
//! Corpus shape (≥5 sessions covering the triage categories the old
//! lane surfaced — failing tools, high cost, long-gap latency, plus
//! plain healthy rows), newest first so the default `--sort recent
//! --order desc` listing order is deterministic.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "agenttrace-inspect-rank-{}-{}",
        tag,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Writes one minimal hermes_jsonl session whose display name (first
/// real user request) is `name`. Timestamps are staggered by the
/// caller so the default recent-descending listing is deterministic.
fn write_session(corpus: &Path, file: &str, lines: &[String]) {
    fs::write(corpus.join(file), lines.join("\n") + "\n").expect("write session journal");
}

fn line(payload: &str) -> String {
    payload.to_string()
}

/// Returns the expected session names in --sessions listing order
/// (row 1 first). Six sessions: newest plain, failing tools, costly
/// usage, plain, long-gap latency, oldest plain.
fn write_rank_corpus(corpus: &Path) -> Vec<(String, String)> {
    let model = "\"ModelUsed\":\"gpt-5\",\"SourceTool\":\"generic\"";
    let mut expected = Vec::new();

    write_session(
        corpus,
        "01-fresh.jsonl",
        &[
            line(&format!(
                "{{\"role\":\"user\",\"content\":\"fresh-plain\",\"timestamp\":\"2026-10-10T10:00:00Z\",{model}}}"
            )),
            line(&format!(
                "{{\"role\":\"assistant\",\"content\":\"hi\",\"timestamp\":\"2026-10-10T10:00:05Z\",{model}}}"
            )),
        ],
    );
    expected.push(("fresh-plain".to_string(), "01-fresh.jsonl".to_string()));

    write_session(
        corpus,
        "02-flaky.jsonl",
        &[
            line(&format!(
                "{{\"role\":\"user\",\"content\":\"flaky-tools\",\"timestamp\":\"2026-10-09T10:00:00Z\",{model}}}"
            )),
            line(&format!(
                "{{\"role\":\"assistant\",\"content\":\"trying\",\"timestamp\":\"2026-10-09T10:00:05Z\",\"tool_calls\":[{{\"id\":\"f1\",\"name\":\"Shell\",\"args\":{{\"cmd\":\"false\"}}}},{{\"id\":\"f2\",\"name\":\"Shell\",\"args\":{{\"cmd\":\"false\"}}}},{{\"id\":\"f3\",\"name\":\"Shell\",\"args\":{{\"cmd\":\"false\"}}}}],{model}}}"
            )),
            line(r#"{"role":"tool","content":"boom","tool_call_id":"f1","is_error":true,"timestamp":"2026-10-09T10:00:06Z","SourceTool":"generic"}"#),
            line(r#"{"role":"tool","content":"boom","tool_call_id":"f2","is_error":true,"timestamp":"2026-10-09T10:00:07Z","SourceTool":"generic"}"#),
            line(r#"{"role":"tool","content":"boom","tool_call_id":"f3","is_error":true,"timestamp":"2026-10-09T10:00:08Z","SourceTool":"generic"}"#),
        ],
    );
    expected.push(("flaky-tools".to_string(), "02-flaky.jsonl".to_string()));

    write_session(
        corpus,
        "03-pricey.jsonl",
        &[
            line(&format!(
                "{{\"role\":\"user\",\"content\":\"pricey-session\",\"timestamp\":\"2026-10-08T10:00:00Z\",{model}}}"
            )),
            line(&format!(
                "{{\"role\":\"assistant\",\"content\":\"{}\",\"timestamp\":\"2026-10-08T10:00:05Z\",\"usage\":{{\"input_tokens\":120000,\"output_tokens\":80000}},{model}}}",
                "x".repeat(4000)
            )),
        ],
    );
    expected.push(("pricey-session".to_string(), "03-pricey.jsonl".to_string()));

    write_session(
        corpus,
        "04-steady.jsonl",
        &[
            line(&format!(
                "{{\"role\":\"user\",\"content\":\"steady-plain\",\"timestamp\":\"2026-10-07T10:00:00Z\",{model}}}"
            )),
            line(&format!(
                "{{\"role\":\"assistant\",\"content\":\"ok\",\"timestamp\":\"2026-10-07T10:00:05Z\",{model}}}"
            )),
        ],
    );
    expected.push(("steady-plain".to_string(), "04-steady.jsonl".to_string()));

    write_session(
        corpus,
        "05-lagging.jsonl",
        &[
            line(&format!(
                "{{\"role\":\"user\",\"content\":\"lagging-gaps\",\"timestamp\":\"2026-10-06T10:00:00Z\",{model}}}"
            )),
            line(&format!(
                "{{\"role\":\"assistant\",\"content\":\"hmm\",\"timestamp\":\"2026-10-06T10:00:05Z\",{model}}}"
            )),
            line(&format!(
                "{{\"role\":\"user\",\"content\":\"more\",\"timestamp\":\"2026-10-06T11:00:00Z\",{model}}}"
            )),
            line(&format!(
                "{{\"role\":\"assistant\",\"content\":\"done\",\"timestamp\":\"2026-10-06T11:05:00Z\",{model}}}"
            )),
        ],
    );
    expected.push(("lagging-gaps".to_string(), "05-lagging.jsonl".to_string()));

    write_session(
        corpus,
        "06-tail.jsonl",
        &[
            line(&format!(
                "{{\"role\":\"user\",\"content\":\"tail-plain\",\"timestamp\":\"2026-10-05T10:00:00Z\",{model}}}"
            )),
            line(&format!(
                "{{\"role\":\"assistant\",\"content\":\"bye\",\"timestamp\":\"2026-10-05T10:00:05Z\",{model}}}"
            )),
        ],
    );
    expected.push(("tail-plain".to_string(), "06-tail.jsonl".to_string()));

    expected
}

fn run_agenttrace(corpus: &Path, home: &Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
    command.arg("-d").arg(corpus);
    for arg in args {
        command.arg(arg);
    }
    command
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join(".cache"));
    command.output().expect("run agenttrace CLI")
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn inspect_addresses_the_sessions_listing_ordinal_for_every_row() {
    let tmp = tempdir("ordinal");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();
    let expected = write_rank_corpus(&corpus);
    assert!(
        expected.len() >= 5,
        "corpus must cover the triage categories"
    );

    // The listing itself: names in the pinned newest-first order.
    let listing = run_agenttrace(&corpus, &home, &["--sessions", "-f", "tsv"]);
    assert!(listing.status.success(), "{}", stderr_of(&listing));
    let listing_text = stdout_of(&listing);
    let rows: Vec<&str> = listing_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .skip(1) // header row
        .collect();
    assert_eq!(rows.len(), expected.len(), "listing row count");
    for (row, (name, _)) in rows.iter().zip(&expected) {
        assert!(
            row.split('\t')
                .next()
                .is_some_and(|first| first.contains(name.as_str())),
            "listing row {row:?} does not carry expected name {name:?}"
        );
    }

    // rm-1522 core: for EVERY listing row N, `--inspect N` renders that
    // same session — the identity header names it, its journal, and
    // the ordinal. Under the old lane, `--inspect 1` rendered the
    // triage pick instead of row 1.
    let total = expected.len();
    let mut triage_reasons = Vec::new();
    for (index, (name, journal)) in expected.iter().enumerate() {
        let rank = index + 1;
        let output = run_agenttrace(&corpus, &home, &["--inspect", &rank.to_string()]);
        assert!(
            output.status.success(),
            "--inspect {rank} failed: {}",
            stderr_of(&output)
        );
        let text = stdout_of(&output);
        assert!(
            text.contains(&format!("Inspected session: {name}")),
            "--inspect {rank} must name row {rank}'s session {name:?} (got header: {:?})",
            text.lines().take(3).collect::<Vec<_>>()
        );
        assert!(
            text.contains(journal),
            "--inspect {rank} must name row {rank}'s journal {journal:?}"
        );
        let row_line = text
            .lines()
            .find(|line| line.starts_with("--sessions row"))
            .unwrap_or_else(|| panic!("--inspect {rank} header missing the row line"));
        assert!(
            row_line.contains(&format!("row {rank} of {total}")),
            "row line must state the ordinal: {row_line:?}"
        );
        let reason = row_line
            .split("triage:")
            .nth(1)
            .unwrap_or_default()
            .trim()
            .trim_end_matches(')')
            .to_string();
        triage_reasons.push(reason);
    }

    // Row 1 is the newest PLAIN session: its triage reason is "none" —
    // the ordinal beat the hidden triage list, which is the whole fix.
    assert_eq!(
        triage_reasons[0], "none",
        "row 1 is a plain session; the header must say triage: none"
    );
    // The corpus carries real triage signals, so at least one row must
    // surface a non-none reason — the reason field the old lane never
    // rendered is now live.
    assert!(
        triage_reasons.iter().any(|reason| reason != "none"),
        "no row surfaced a triage reason: the corpus stopped carrying signals (fixture vacuity)"
    );
}

#[test]
fn inspect_out_of_range_ranks_fail_loudly_against_the_listing() {
    let tmp = tempdir("range");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();
    let expected = write_rank_corpus(&corpus);
    let total = expected.len();

    let beyond = run_agenttrace(&corpus, &home, &["--inspect", &(total + 1).to_string()]);
    assert!(
        !beyond.status.success(),
        "rank {} ({} listable) must fail",
        total + 1,
        total
    );
    let stderr = stderr_of(&beyond);
    assert!(
        stderr.contains("exceeds the --sessions listing"),
        "must name the listing contract, got {stderr:?}"
    );
    assert!(
        stderr.contains(&format!("1..={total}")),
        "must list the valid range 1..={total}, got {stderr:?}"
    );
    assert!(
        !stderr.contains("priority sessions"),
        "the retired hidden-list message must be gone, got {stderr:?}"
    );

    let zero = run_agenttrace(&corpus, &home, &["--inspect", "0"]);
    assert!(!zero.status.success(), "rank 0 must fail");
    assert!(
        stderr_of(&zero).contains("starts at 1"),
        "rank 0 must explain the 1-based contract, got {:?}",
        stderr_of(&zero)
    );
}

#[test]
fn inspect_json_lane_stays_lossless_without_the_identity_header() {
    let tmp = tempdir("json");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();
    let expected = write_rank_corpus(&corpus);

    let output = run_agenttrace(&corpus, &home, &["--inspect", "1", "-f", "json"]);
    assert!(
        output.status.success(),
        "--inspect 1 -f json failed: {}",
        stderr_of(&output)
    );
    let text = stdout_of(&output);
    assert!(
        !text.contains("Inspected session:"),
        "the identity header is a human-lane decoration; JSON must stay machine-parseable"
    );
    let trimmed = text.trim();
    assert!(
        trimmed.starts_with('{') && trimmed.ends_with('}'),
        "JSON lane must emit a JSON document"
    );
    assert!(
        text.contains(&format!("\"name\": \"{}\"", expected[0].0)),
        "JSON carries the selected session identity losslessly"
    );
    // Row selection is the same ordinal in the JSON lane.
    assert!(
        !text.contains(&format!("\"name\": \"{}\"", expected[1].0)),
        "--inspect 1 must select row 1's session in every format lane"
    );
}

#[test]
fn inspect_prose_and_rendered_behavior_agree() {
    // rm-1522 acceptance arm 5: the flag-table prose-vs-code gap gets
    // a gate arm — README's flag table, `--help`, and the rendered
    // behavior must promise and deliver the SAME ordinal contract.
    let tmp = tempdir("prose");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();
    let expected = write_rank_corpus(&corpus);

    let help = run_agenttrace(&corpus, &home, &["--help"]);
    assert!(help.status.success());
    let help_text = stdout_of(&help);
    let help_lines: Vec<&str> = help_text.lines().collect();
    let flag_index = help_lines
        .iter()
        .position(|line| line.contains("--inspect"))
        .unwrap_or_else(|| panic!("--help has no --inspect row"));
    // clap renders the flag and its doc on consecutive lines; take a
    // window so line wrapping cannot hide the promise.
    let window: String = help_lines[flag_index..(flag_index + 2).min(help_lines.len())].join(" ");
    assert!(
        window.contains("--sessions"),
        "--help must tie --inspect to the --sessions listing, got {window:?}"
    );
    assert!(
        window.replace('`', "").contains("--sessions list index"),
        "--help must promise the list-index contract, got {window:?}"
    );

    let readme = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../README.md");
    let readme_text = fs::read_to_string(&readme)
        .expect("README.md readable from the crate (test runs from the repo checkout)");
    let readme_row = readme_text
        .lines()
        .find(|line| line.contains("--inspect"))
        .unwrap_or_else(|| panic!("README flag table has no --inspect row"));
    assert!(
        readme_row
            .replace('`', "")
            .contains("--sessions list index"),
        "README flag table must promise the same contract, got {readme_row:?}"
    );

    // And the rendered report delivers the promised tie explicitly.
    let output = run_agenttrace(&corpus, &home, &["--inspect", "1"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(
        stdout_of(&output).contains(&format!("--sessions row 1 of {}", expected.len())),
        "the rendered report must state the --sessions ordinal it served"
    );
}

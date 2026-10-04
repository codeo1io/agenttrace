//! rm-409 end-to-end: `-f csv` against a real parsed corpus — RFC 4180
//! shape, the rm-408 disclosure column riding along, formula-guard and
//! quoting on transcript-derived cells, and byte-determinism across
//! runs. Companion unit tests live in `src/csv_export.rs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir(tag: &str, lines: &[String]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("at-csv-{tag}"));
    fs::create_dir_all(&dir).expect("mkdir fixture dir");
    fs::write(dir.join("session.jsonl"), lines.join("\n") + "\n").expect("write fixture");
    dir
}

fn user_line(content: &str) -> String {
    // No `cwd` key: the session event a `cwd`-carrying line pushes is
    // content-less, and naming would fall back to the file name before
    // this content is consulted. Naming rules are not this batch's
    // contract — the specials need to reach a rendered cell.
    format!(
        r#"{{"type":"user","timestamp":"2026-10-04T01:00:00Z","message":{{"role":"user","content":"{}"}}}}"#,
        json_escape(content)
    )
}

/// Escape the two characters JSONL fixtures care about so the wire
/// line stays valid JSON while the parsed cell keeps the specials.
fn json_escape(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\");
    escaped.replace('"', "\\\"")
}

fn assistant_line(usage: &str, model: &str) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"2026-10-04T01:00:05Z","message":{{"id":"msg_1","model":"{model}","role":"assistant","content":[{{"type":"text","text":"on it"}}],"usage":{usage}}}}}"#
    )
}

const ALL_ZERO: &str = r#"{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}"#;
const CLAUDE_MODEL: &str = "claude-sonnet-4-5-20250929";

fn run_csv(dir: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(args)
        .arg("-d")
        .arg(dir)
        .output()
        .expect("run agenttrace CLI");
    assert!(output.status.success(), "CLI failed: {:?}", output);
    String::from_utf8(output.stdout).expect("csv stdout is UTF-8")
}

#[test]
fn sessions_csv_carries_the_disclosure_column_and_crlf_rows() {
    let dir = fixture_dir(
        "sessions",
        &[
            user_line("check the billing please"),
            assistant_line(ALL_ZERO, CLAUDE_MODEL),
        ],
    );
    let out = run_csv(&dir, &["--sessions", "-f", "csv"]);
    let lines: Vec<&str> = out.split("\r\n").collect();
    assert_eq!(lines[0], "# sessions", "table marker first");
    assert_eq!(
        lines[1],
        "session,health,data,source,model,cost,tokens,fail,anomalies,zero_usage_events"
    );
    // rm-408: the reported-zero block is disclosed in the statement.
    assert!(
        lines[2].ends_with(",1"),
        "zero_usage_events=1 on the data row, got: {}",
        lines[2]
    );
    assert!(out.ends_with("\r\n"));
    fs::remove_dir_all(dir).ok();
}

#[test]
fn sessions_csv_quotes_commas_and_guards_formulas() {
    let dir = fixture_dir(
        "guards",
        &[
            // JSON-escaped quotes so the wire line stays valid JSONL;
            // the parsed cell then genuinely contains `fix, the "billing"`.
            user_line("fix, the \"billing\""),
            // A model id that begins like a spreadsheet formula — model
            // ids are transcript-controlled, so the cell must be guarded.
            assistant_line(ALL_ZERO, "=SUM(A1)"),
        ],
    );
    let out = run_csv(&dir, &["--sessions", "-f", "csv"]);
    assert!(
        out.contains("\"fix, the \"\"billing\"\""),
        "embedded comma/quote cells are RFC 4180 quoted: {out}"
    );
    assert!(
        out.contains("'=SUM(A1)"),
        "formula-looking model id gets the guard quote: {out}"
    );
    fs::remove_dir_all(dir).ok();
}

#[test]
fn overview_csv_emits_summary_and_attribution_tables() {
    let dir = fixture_dir(
        "overview",
        &[
            user_line("check the billing please"),
            assistant_line(ALL_ZERO, CLAUDE_MODEL),
        ],
    );
    let out = run_csv(&dir, &["--overview", "-f", "csv"]);
    assert!(out.contains("# summary\r\n"), "summary table present");
    assert!(
        out.contains("sessions,healthy,warning,critical,cost"),
        "summary header"
    );
    assert!(out.contains("# by_model"), "attribution table present");
    assert!(
        out.contains("claude-sonnet-4-5-20250929"),
        "model row names the model"
    );
    fs::remove_dir_all(dir).ok();
}

#[test]
fn csv_output_is_byte_deterministic_across_runs() {
    let dir = fixture_dir(
        "determinism",
        &[
            user_line("check the billing please"),
            assistant_line(ALL_ZERO, CLAUDE_MODEL),
        ],
    );
    let first = run_csv(&dir, &["--sessions", "-f", "csv"]);
    let second = run_csv(&dir, &["--sessions", "-f", "csv"]);
    assert_eq!(first, second, "two runs must be byte-identical");
    let first_overview = run_csv(&dir, &["--overview", "-f", "csv"]);
    let second_overview = run_csv(&dir, &["--overview", "-f", "csv"]);
    assert_eq!(first_overview, second_overview);
    fs::remove_dir_all(dir).ok();
}

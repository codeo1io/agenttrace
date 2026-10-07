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

/// rm-540: an assistant line whose model is given in ALREADY-ESCAPED
/// wire form — hostile payloads carry control bytes and quotes that
/// must ride the wire as `\uXXXX`/`\"` escapes to keep the fixture
/// line valid JSONL while the parsed cell keeps the specials. Each
/// line needs its own message id: the parser dedupes assistant
/// messages by id, so a shared `msg_1` collapses to one usage event
/// (and one by_model row) per session.
fn assistant_line_model_json(id: &str, model_json: &str) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"2026-10-04T01:00:05Z","message":{{"id":"{id}","model":"{model_json}","role":"assistant","content":[{{"type":"text","text":"on it"}}],"usage":{ALL_ZERO}}}}}"#
    )
}

const ALL_ZERO: &str = r#"{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}"#;
const CLAUDE_MODEL: &str = "claude-sonnet-4-5-20250929";

/// Spawns the CLI for one test. The HOME/XDG sandbox is per test
/// thread (the rm-301 entrypoints.rs convention): run_csv used to
/// inherit the operator's real HOME, so every spawned CLI
/// read-modify-wrote the shared ~/.cache/agenttrace/sessions.json —
/// parallel test threads raced on that file (observed twice as
/// csv_export e2e failures during this batch's verification while
/// concurrent agenttrace processes wrote the same cache), and every
/// test run polluted the operator's real cache with fixture entries
/// and schema churn.
fn run_csv(dir: &Path, args: &[&str]) -> String {
    let sandbox = std::env::temp_dir().join(format!(
        "at-csv-sandbox-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let cache = sandbox.join("cache");
    fs::create_dir_all(&cache).expect("create sandbox cache dir");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &sandbox)
        .env("XDG_CACHE_HOME", &cache)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
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
        "session,health,data,source,model,cost,tokens,fail,anomalies,zero_usage_events,subagents,subagent_cost"
    );
    // rm-408: the reported-zero block is disclosed in the statement;
    // rm-487: the subagent rollup columns ride beside it (0 / 0.0000
    // on a corpus without subagent transcripts).
    assert!(
        lines[2].ends_with(",1,0,0.0000"),
        "zero_usage_events=1 plus empty subagent rollup on the data row, got: {}",
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
fn overview_csv_sanitizes_hostile_group_keys() {
    // rm-540 (assess PoC F1): transcript-controlled ModelUsed strings
    // reached the by_model group keys RAW — live OSC-52 clipboard-write
    // bytes, a tab-prefixed =HYPERLINK, and a CRLF row-forgery payload
    // all rode into `--overview -f csv` (od -c transcripts in assess
    // 871cae94). csv_row now sanitizes every cell at the render
    // boundary and guard_formula judges the first non-whitespace
    // character.
    let dir = std::env::temp_dir().join("at-csv-hostile-groups");
    fs::create_dir_all(&dir).expect("mkdir fixture dir");
    // One session FILE per hostile model: by_model attributes the
    // session's primary model, so a single session with three models
    // collapses to one row (the assess corpus used one file per
    // payload for the same reason).
    let models: [(&str, &str); 3] = [
        // wire form: esc<ESC>]52;c;pwn<BEL>
        ("a", r"esc\u001b]52;c;pwn\u0007"),
        // wire form: <TAB>=HYPERLINK("http://evil.example";"click me")
        ("b", r#"\t=HYPERLINK(\"http://evil.example\";\"click me\")"#),
        // wire form: fake-table<CRLF>name, sessions, cost<CRLF>
        // EVIL-ROW, 99999, 88888.0<CRLF>real-model
        (
            "c",
            r"fake-table\r\nname, sessions, cost\r\nEVIL-ROW, 99999, 88888.0\r\nreal-model",
        ),
    ];
    for (tag, model_json) in models {
        fs::write(
            dir.join(format!("session-{tag}.jsonl")),
            user_line("check the billing please")
                + "\n"
                + &assistant_line_model_json("msg_1", model_json)
                + "\n",
        )
        .expect("write fixture");
    }
    let out = run_csv(&dir, &["--overview", "-f", "csv"]);
    // F1a: no raw control bytes anywhere except the CRLF row
    // terminators — OSC bytes are neutralized to U+FFFD.
    assert!(
        !out.chars()
            .any(|c| c.is_control() && c != '\r' && c != '\n'),
        "no raw control bytes may ride the csv: {out:?}"
    );
    assert!(
        !out.contains('\u{1b}') && !out.contains('\u{7}') && !out.contains('\t'),
        "ESC/BEL/tab must be sanitized: {out:?}"
    );
    assert!(
        out.contains("esc\u{FFFD}]52;c;pwn\u{FFFD}"),
        "OSC-52 payload becomes U+FFFD in the by_model key: {out:?}"
    );
    // F1b: the tab prefix is a control byte and becomes U+FFFD, so
    // the cell no longer leads with a formula character at all (any
    // leading quote is the RFC 4180 quote for embedded double-quotes,
    // not the formula guard).
    assert!(
        out.contains("\u{FFFD}=HYPERLINK"),
        "tab-prefixed HYPERLINK loses its formula start: {out:?}"
    );
    assert!(
        !out.contains("'\u{FFFD}"),
        "no stale guard on FFFD: {out:?}"
    );
    // F1c containment: the CRLF forgery payload stays ONE cell — no
    // EVIL-ROW row of its own and no raw CRLF inside the cell.
    assert!(
        !out.lines().any(|l| l.starts_with("EVIL-ROW")),
        "no forged rows from the CRLF payload: {out:?}"
    );
    assert!(
        out.contains("fake-table\u{FFFD}\u{FFFD}name, sessions, cost"),
        "CRLF payload is neutralized and stays one cell: {out:?}"
    );
    fs::remove_dir_all(dir).ok();
}

#[test]
fn sessions_csv_guards_whitespace_prefixed_formulas() {
    // rm-540 (assess PoC F2): ` =SUM(9+9)` passed the old
    // first-char-only formula guard in the model column; the guard now
    // judges the first non-whitespace character.
    let dir = fixture_dir(
        "lead-space",
        &[
            user_line("check the billing please"),
            assistant_line(ALL_ZERO, " =SUM(9+9)"),
        ],
    );
    let out = run_csv(&dir, &["--sessions", "-f", "csv"]);
    assert!(
        out.contains("' =SUM(9+9)"),
        "leading-space formula gets the guard quote: {out}"
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

#[test]
fn csv_outside_its_composable_set_bails_like_the_markdown_guard() {
    // rm-409 review fix: `-f csv` is composable with exactly --overview
    // and --sessions (the README column contract). Everything else used
    // to fall through to a silent TEXT render — the incoherence the
    // markdown/html guard exists to prevent. Pin the loud bail, with the
    // pre-existing markdown guard as the contrast arm.
    let dir = fixture_dir(
        "guard",
        &[
            user_line("check the billing please"),
            assistant_line(ALL_ZERO, CLAUDE_MODEL),
        ],
    );
    for (args, needle) in [
        (
            &["--latest", "-f", "csv"][..],
            "csv format requires --overview or --sessions",
        ),
        (
            &["--doctor", "-f", "csv"][..],
            "csv format requires --overview or --sessions",
        ),
        (
            &["--latest", "-f", "markdown"][..],
            "markdown and html formats require --overview",
        ),
    ] {
        // Same per-thread HOME/XDG sandbox as run_csv: no test in this
        // file may touch operator state, even on bail paths.
        let sandbox = std::env::temp_dir().join(format!(
            "at-csv-sandbox-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let cache = sandbox.join("cache");
        fs::create_dir_all(&cache).expect("create sandbox cache dir");
        let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .env("HOME", &sandbox)
            .env("XDG_CACHE_HOME", &cache)
            .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
            .args(args)
            .arg("-d")
            .arg(&dir)
            .output()
            .expect("run agenttrace CLI");
        assert!(
            !output.status.success(),
            "`{args:?}` should bail instead of silently rendering"
        );
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        assert!(
            stderr.contains(needle),
            "expected `{needle}` in stderr, got: {stderr}"
        );
    }
    fs::remove_dir_all(dir).ok();
}

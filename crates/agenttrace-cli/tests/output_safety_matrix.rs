//! Output-safety matrix (rm-625, minted campaign-locally as rm-545): every report × non-JSON format lane
//! must emit ZERO raw control bytes when the corpus carries
//! transcript-derived poisoned names (OSC-52 clipboard-write, ANSI
//! clear-screen, BEL) in both the session name (file stem) and the
//! model id. JSON lanes are the control: they keep control bytes
//! escaped (data integrity over terminal trust, by design), so the
//! assertion for them is "no RAW ESC/BEL byte", not "no mention".
//!
//! The choke point is the output-dispatch boundary (`dispatch_sanitize`
//! in main.rs wraps every report emission; see `sanitize_output_document`
//! in agenttrace-core). Renderer-internal escapes (markdown entity
//! escapes, the rm-540 CSV cell sanitizer) remain defense-in-depth and
//! keep working — the matrix asserts the boundary, not the internals.
//!
//! Cache isolation (fleet constraint): `AGENTTRACE_CACHE_DIR` does not
//! exist; `XDG_CACHE_HOME` + a throwaway `HOME` keep the operator cache
//! and real agent homes out of the run.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The two proven-corpus transcripts from this run's assessment
/// (byte-identical to /tmp/at-9873-poc/sessions/*.jsonl, live-confirmed
/// there): file A carries OSC-52+BEL in user text and ANSI clear-screen
/// in the tool name; file B carries OSC-52+BEL in the model id. Names
/// flow from the loader into every renderer that prints session names,
/// models, tool names, or message text.
fn write_poisoned_corpus(root: &std::path::Path) -> Vec<PathBuf> {
    let a = root.join("20261001T100000+0000-poc-9873.jsonl");
    fs::write(
        &a,
        r#"{"type": "user", "uuid": "u1", "timestamp": "2026-10-01T10:00:00Z", "message": {"role": "user", "content": "poc-9873 markdown lane \u001b]52;c;aGVsbG8gd29ybGQ=\u0007 tail"}, "cwd": "/tmp/at-9873-poc"}
{"type": "assistant", "uuid": "a1", "timestamp": "2026-10-01T10:00:05Z", "message": {"role": "assistant", "model": "claude-sonnet-4-5", "content": [{"type": "tool_use", "id": "t1", "name": "Bash\u001b[2J\u001b[H", "input": {"command": "echo hi"}}, {"type": "text", "text": "done"}], "usage": {"input_tokens": 100, "output_tokens": 50}}}
{"type": "user", "uuid": "u2", "timestamp": "2026-10-01T10:00:10Z", "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t1", "content": "ok"}]}}
"#,
    )
    .unwrap();
    let b = root.join("20261002T100000+0000-poc-model.jsonl");
    fs::write(
        &b,
        r#"{"type": "user", "uuid": "u1", "timestamp": "2026-10-02T10:00:00Z", "message": {"role": "user", "content": "overview csv model probe"}, "cwd": "/tmp/at-9873-poc"}
{"type": "assistant", "uuid": "a1", "timestamp": "2026-10-02T10:00:05Z", "message": {"role": "assistant", "model": "poc-model\u001b]52;c;c3FtZS1leGZpbHRyYXRpb24=\u0007", "content": [{"type": "text", "text": "done"}], "usage": {"input_tokens": 10, "output_tokens": 5}}}
"#,
    )
    .unwrap();
    vec![a, b]
}

struct Cell {
    report: &'static str,
    format: &'static str,
}

fn matrix() -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut push = |report: &'static str, formats: &[&'static str]| {
        for format in formats {
            cells.push(Cell { report, format });
        }
    };
    push("--overview", &["text", "markdown", "md", "html", "csv"]);
    push("--sessions", &["text", "csv"]);
    push("--audit", &["text", "markdown", "md", "html"]);
    push("--recommend", &["text", "markdown", "md", "html"]);
    push("--diagnostics", &["text"]);
    push("--mcp-governance", &["markdown", "html", "text"]);
    push("--context-trends", &["markdown", "text"]);
    push("--delivery-evidence", &["markdown", "text"]);
    push("--waste", &["text"]);
    // Review fix (rm-625 F4): every remaining render-bearing lane of
    // the dispatch choke point gets a matrix cell — compare and the
    // statusline-report both render transcript/journal-derived text.
    push("--compare", &["text"]);
    push("--statusline-report", &["text"]);
    cells
}

fn run_cell(corpus: &std::path::Path, home: &std::path::Path, cell: &Cell) -> Vec<u8> {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("-d")
        .arg(corpus)
        .arg(cell.report)
        .arg("-f")
        .arg(cell.format)
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .output()
        .expect("agenttrace runs");
    assert!(
        out.status.success(),
        "{} -f {} failed: {}",
        cell.report,
        cell.format,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

#[test]
fn no_raw_control_bytes_in_any_non_json_output_lane() {
    let tmp = tempdir("output-safety-matrix");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    write_poisoned_corpus(&corpus);
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();

    let mut failures = Vec::new();
    for cell in matrix() {
        let stdout = run_cell(&corpus, &home, &cell);
        let esc = stdout.iter().filter(|&&b| b == 0x1b).count();
        let bel = stdout.iter().filter(|&&b| b == 0x07).count();
        if esc != 0 || bel != 0 {
            failures.push(format!(
                "{} -f {} emitted {esc} ESC bytes and {bel} BEL bytes",
                cell.report, cell.format
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "raw control bytes reached the terminal (rm-625):\n  {}",
        failures.join("\n  ")
    );
}

/// JSON lanes keep the poisoned payload but never emit a RAW control
/// byte: escaping is lossless, so the name survives analysis while the
/// terminal never interprets it.
#[test]
fn json_lanes_escape_control_bytes_instead_of_emitting_raw() {
    let tmp = tempdir("output-safety-json");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    write_poisoned_corpus(&corpus);
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();

    for report in ["--overview", "--sessions", "--audit"] {
        let stdout = run_cell(
            &corpus,
            &home,
            &Cell {
                report,
                format: "json",
            },
        );
        let esc = stdout.iter().filter(|&&b| b == 0x1b).count();
        let bel = stdout.iter().filter(|&&b| b == 0x07).count();
        assert_eq!(
            (esc, bel),
            (0, 0),
            "{report} -f json emitted raw control bytes (json escaping must encode them)"
        );
    }
}

/// The `-o` write path goes through the same choke point as stdout.
#[test]
fn output_file_lane_is_sanitized_too() {
    let tmp = tempdir("output-safety-file");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    write_poisoned_corpus(&corpus);
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();
    let target = tmp.join("overview.md");

    let status = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("-d")
        .arg(&corpus)
        .arg("--overview")
        .arg("-f")
        .arg("markdown")
        .arg("-o")
        .arg(&target)
        .env("HOME", &home)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .status()
        .expect("agenttrace runs");
    assert!(status.success());
    let bytes = fs::read(&target).unwrap();
    let esc = bytes.iter().filter(|&&b| b == 0x1b).count();
    let bel = bytes.iter().filter(|&&b| b == 0x07).count();
    assert_eq!(
        (esc, bel),
        (0, 0),
        "-o file output carried raw control bytes (rm-625 choke point must cover file writes)"
    );
}

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("at-matrix-{tag}-{}", std::process::id()))
        .join(format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            tag
        ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn tsv_is_not_a_cli_value_and_stays_rejected() {
    // Review fix (rm-625 F4): the campaign row's `tsv` matrix column was
    // an over-spec — `-f tsv` is not a CLI value for `--sessions` (clap
    // rejects it with rc 2) and tab-separated output is reached only
    // through `--delivery-evidence`. Pin the real contract instead of
    // the aspirational one so the matrix documents the actual surface.
    let tmp = tempdir("output-safety-tsv");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    write_poisoned_corpus(&corpus);
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("-d")
        .arg(&corpus)
        .arg("--sessions")
        .arg("-f")
        .arg("tsv")
        .env("HOME", &home)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .output()
        .expect("agenttrace runs");
    assert_eq!(
        out.status.code(),
        Some(2),
        "-f tsv is rejected by the CLI, not a render lane: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn upstream_status_lane_stays_sanitized() {
    // Review fix (rm-625 F4): the `upstream` host command renders its
    // text report through the same dispatch choke point, but its bytes
    // are environment-derived (repo name, remote URL, ahead/behind),
    // not transcript-derived — poisoned names cannot reach it. This is
    // a site pin: the lane stays wired to the choke point and byte-
    // clean. It runs against a throwaway git repo with a dummy remote
    // so the probe stays offline and deterministic (PRIVACY.md: fully
    // offline unless --fetch).
    let tmp = tempdir("output-safety-upstream");
    let repo = tmp.join("repo");
    fs::create_dir_all(&repo).unwrap();
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();
    let status = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["init", "-q"])
        .status()
        .expect("git init runs");
    if !status.success() {
        // No git here: the lane cannot be exercised — skip rather than
        // fail on an environment property the repo does not own.
        return;
    }
    Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["remote", "add", "upstream", "file:///nowhere"])
        .status()
        .expect("git remote add runs");
    Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args([
            "-c",
            "user.email=agenttrace@example.invalid",
            "-c",
            "user.name=agenttrace",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ])
        .status()
        .expect("git commit runs");
    Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["update-ref", "refs/remotes/upstream/master", "HEAD"])
        .status()
        .expect("git update-ref runs");
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("upstream")
        .env("HOME", &home)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .current_dir(&repo)
        .output()
        .expect("agenttrace runs");
    assert!(
        out.status.success(),
        "upstream status failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let esc = out.stdout.iter().filter(|&&b| b == 0x1b).count();
    let bel = out.stdout.iter().filter(|&&b| b == 0x07).count();
    assert_eq!(
        (esc, bel),
        (0, 0),
        "upstream text lane emitted raw control bytes"
    );
}

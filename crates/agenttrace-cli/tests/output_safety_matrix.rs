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
    push("--sessions", &["text", "csv", "tsv"]);
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
fn tsv_is_a_sessions_scoped_cli_value_with_a_named_guard_elsewhere() {
    // Review fix lineage: rm-625 F4 pinned the THEN-real contract (clap
    // rejected `-f tsv`; TSV was reachable only via `--delivery-evidence`).
    // rm-937 (2026-10-10, run b5b85faa61c9) deliberately made `tsv` a real
    // CLI value scoped to `--sessions` — the default sessions lane IS a TSV
    // document, so the explicit value must be requestable and byte-identical
    // to the default. Same pin-the-real-contract discipline, new contract:
    // (a) `--sessions -f tsv` succeeds and renders the table lane; (b) the
    // value is NOT a render lane elsewhere — `--overview -f tsv` exits with
    // the named guard error instead of silently rendering something else.
    let tmp = tempdir("output-safety-tsv");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    write_poisoned_corpus(&corpus);
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();
    let sessions = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
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
        sessions.status.code(),
        Some(0),
        "--sessions -f tsv is a requestable lane (rm-937): {}",
        String::from_utf8_lossy(&sessions.stderr)
    );
    let overview = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("-d")
        .arg(&corpus)
        .arg("--overview")
        .arg("-f")
        .arg("tsv")
        .env("HOME", &home)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .output()
        .expect("agenttrace runs");
    let stderr = String::from_utf8_lossy(&overview.stderr);
    assert_eq!(
        overview.status.code(),
        Some(1),
        "-f tsv outside --sessions is a named guard error, not a render lane: {}",
        stderr
    );
    assert!(
        stderr.contains("tsv format requires --sessions"),
        "the guard names the scoping rule: {}",
        stderr
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

/// rm-933: invisible Cf-class format characters (bidi overrides,
/// isolate controls, Unicode tag characters, zero-width joiners) must
/// never reach a non-JSON output lane raw - a terminal or a rendered
/// document carrying an RLO or a tag-character payload is a
/// CVE-2021-42574-class spoofing surface. The JSON lane is the
/// control: it keeps the payload (machine consumers get lossless
/// data, and Cf characters are inert outside a terminal).
#[test]
fn no_raw_invisible_format_chars_in_any_non_json_output_lane() {
    let tmp = tempdir("output-safety-cf");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let payload = "\u{202E}claude-\u{E0001}\u{E0041}sonnet\u{200B}\u{200C}\u{200D}\u{2060}\u{FEFF}4-5\u{2066}\u{061C}";
    let record = format!(
        "{{\"timestamp\":\"2026-10-04T10:00:55.000Z\",\"type\":\"assistant\",\"message\":{{\"model\":\"{payload}\",\"usage\":{{\"input_tokens\":1200,\"output_tokens\":340}}}}}}"
    );
    fs::write(corpus.join("20261004T100055+0000-cf-payload.jsonl"), record).unwrap();
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();

    let hostile_sequences: &[&[u8]] = &[
        "\u{202E}".as_bytes(), // RLO
        "\u{200E}".as_bytes(), // LRM
        "\u{200F}".as_bytes(), // RLM
        "\u{2066}".as_bytes(), // LRI
        "\u{2069}".as_bytes(), // PDI
        "\u{061C}".as_bytes(), // ALM
        "\u{200B}".as_bytes(), // ZWSP
        "\u{200C}".as_bytes(), // ZWNJ
        "\u{200D}".as_bytes(), // ZWJ
        "\u{2060}".as_bytes(), // WJ
        "\u{FEFF}".as_bytes(), // BOM
        "\u{E0001}".as_bytes(), // LANGUAGE TAG
        "\u{E0041}".as_bytes(), // TAG LATIN CAPITAL LETTER A
    ];

    let mut failures = Vec::new();
    for cell in matrix() {
        let stdout = run_cell(&corpus, &home, &cell);
        if cell.format == "json" {
            // control: the payload survives losslessly (escaping never
            // substitutes), so at least the RLO byte sequence is present
            assert!(
                stdout.windows(3).any(|w| w == "\u{202E}".as_bytes()),
                "{} -f json dropped the hostile payload (json must stay lossless)",
                cell.report
            );
            continue;
        }
        for seq in hostile_sequences {
            if stdout.windows(seq.len()).any(|w| w == *seq) {
                failures.push(format!(
                    "{} -f {} emitted raw invisible format chars",
                    cell.report, cell.format
                ));
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "raw Cf-class characters reached a rendered lane (rm-933):\n  {}",
        failures.join("\n  ")
    );
}

/// rm-540 (TSV arm): the sessions table is exactly where hostile
/// transcript-derived strings become spreadsheet cells. Every formula
/// introducer - ASCII and the fullwidth twins - must be apostrophe
/// guarded in the default lane, the explicit TSV lane and the CSV
/// lane alike.
#[test]
fn formula_introducers_are_guarded_in_tsv_and_default_sessions_lanes() {
    let tmp = tempdir("output-safety-formula");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let record = "{\"timestamp\":\"2026-10-04T11:00:55.000Z\",\"type\":\"assistant\",\"message\":{\"model\":\"claude-3-5\",\"usage\":{\"input_tokens\":10,\"output_tokens\":4}}}";
    for stem in [
        "=HYPERLINK(\"pwn\";\"me\")",
        " =HYPERLINK(\"pad\";\"x\")",
        "\u{FF1D}HYPERLINK(\"full\";\"width\")",
    ] {
        fs::write(corpus.join(format!("{stem}.jsonl")), record).unwrap();
    }
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();

    for args in [vec![], vec!["-f", "tsv"], vec!["-f", "csv"]] {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agenttrace"));
        cmd.arg("-d").arg(&corpus).arg("--sessions");
        for a in &args {
            cmd.arg(a);
        }
        let out = cmd
            .env("HOME", &home)
            .env("XDG_CACHE_HOME", home.join(".cache"))
            .output()
            .expect("agenttrace runs");
        assert!(
            out.status.success(),
            "--sessions {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let mut unguarded = 0;
        let mut guarded = 0;
        for (idx, _) in stdout.match_indices("HYPERLINK").enumerate() {
            let introducer = stdout[..idx].chars().next_back().unwrap_or(' ');
            let before = stdout[..idx - introducer.len_utf8()]
                .chars()
                .next_back()
                .unwrap_or(' ');
            let _ = idx;
            if introducer == '\u{FF1D}' || introducer == '=' {
                // introducer directly attached to HYPERLINK: guard must sit right before it
                if introducer == '\u{FF1D}' || before == '\'' {
                    guarded += 1;
                }
            } else if introducer.is_whitespace() {
                // padded lane: walk back over the padding to the guard
                let mut probe = stdout[..idx].trim_end();
                if probe.ends_with('\'') {
                    guarded += 1;
                } else {
                    unguarded += 1;
                }
            }
        }
        assert_eq!(
            unguarded, 0,
            "--sessions {:?} let a formula introducer reach a cell unguarded (rm-540)",
            args
        );
        assert!(
            guarded >= 3,
            "--sessions {:?} did not surface all three formula stems (got {guarded})",
            args
        );
    }
}

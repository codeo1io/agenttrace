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
///
/// FIXTURE FORM IS CONTRACT (review fix ff418327 F1): the control bytes
/// must be JSON-escape TEXT (`\u001b` / `\u0007`) inside these raw
/// strings. `\u{001b}` is Rust escape syntax that raw strings never
/// process — it stays 8 literal characters, `\u{` is an INVALID JSON
/// u-escape, and serde_json drops the whole line: every zero-ESC
/// assertion then goes green VACUOUSLY (no ESC ever enters the
/// pipeline; the session name falls back to the filename, the model
/// renders 'default'). A raw ESC byte written directly inside a JSON
/// string is equally invalid — only JSON-escape text parses.
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
    // rm-933 widening-arm review F2 (2026-10-16, run 7288595f
    // independent review): "tsv" joins the --sessions cells — the
    // explicit `-f tsv` spelling is the od-proven carrier lane of the
    // raw joiner pair (E2 80 8D / E2 80 8C in the name/model cells),
    // so the regression pin must cover it, not just text/csv.
    push("--sessions", &["text", "tsv", "csv"]);
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

    // Vacuity guard (review fix ff418327 F1): a green zero-ESC matrix
    // proves nothing unless the poison actually REACHED the lanes. The
    // overview text lane renders the poisoned model id through the
    // substitution path — if the fixtures ever stop parsing (e.g. a
    // raw-string `\u{001b}` "fix" that serde_json drops), the model
    // renders 'default', no U+FFFD is substituted, and this assertion
    // goes red instead of the matrix silently passing empty.
    let text = run_cell(
        &corpus,
        &home,
        &Cell {
            report: "--overview",
            format: "text",
        },
    );
    assert!(
        text.windows(3).any(|w| w == [0xef, 0xbf, 0xbd].as_slice()),
        "poison never reached the text lane: no U+FFFD substitution found — fixture vacuity (see write_poisoned_corpus contract note)"
    );
    assert!(
        text.windows(b"poc-model".len()).any(|w| w == b"poc-model"),
        "poisoned model id missing from the text lane (rendered 'default'? fixture vacuity)"
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
        // Vacuity guard (review fix ff418327 F1): JSON lanes must keep
        // the payload LOSSLESSLY — the \u001b escape TEXT is the proof
        // the poisoned fixtures parsed and delivered. If the fixtures
        // break (invalid \u{ escape), the escape text vanishes and this
        // assertion catches the vacuous green.
        assert!(
            stdout
                .windows(b"\\u001b".len())
                .any(|w| w == b"\\u001b"),
            "{report} -f json carries no \\u001b escape text — the poisoned payload never parsed (fixture vacuity)"
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
    // Vacuity guard (review fix ff418327 F1): same contract as the
    // stdout guards — the written file must show the substitution
    // (U+FFFD) and the poisoned model id, proving the fixtures parsed.
    assert!(
        bytes.windows(3).any(|w| w == [0xef, 0xbf, 0xbd].as_slice()),
        "-o file carries no U+FFFD substitution — fixture vacuity"
    );
    assert!(
        bytes.windows(b"poc-model".len()).any(|w| w == b"poc-model"),
        "-o file lost the poisoned model id — fixture vacuity"
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

/// rm-540 arm (cycle 3): the default-text `--sessions` lane renders
/// tab-separated rows whose transcript-derived cells (name from the
/// file stem, source tool, model) previously rode byte-for-byte — a
/// journal named `=SUM(9+9).jsonl` imported straight into a
/// spreadsheet as a live formula while the CSV lane guarded the same
/// cell (`"'=2+5|cmd"`). The TSV lane now routes those three cells
/// through the same `guard_formula`; this is the lane-parity pin:
/// whichever of the two lanes a machine consumer imports, the cell
/// is inert.
#[test]
// The fixtures write journals NAMED after hostile formula stems
// ("=SUM(9+9).jsonl") — legal POSIX filenames, invalid on windows
// (os error 123, CI run 41194809329). The CSV-introducer guard
// itself is filename-agnostic and stays pinned on unix runners.
#[cfg(unix)]
fn default_text_lane_guards_spreadsheet_formulas_like_csv() {
    let tmp = tempdir("output-safety-formula");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let hostile_stems = [
        "=SUM(9+9)",
        "+SUM(A1:A9)",
        "@HYPERLINK(evil.example)",
        "-2+5|cmd",
        "\u{ff1d}1+1",    // fullwidth equals
        "\u{ff20}SUM(9)", // fullwidth commercial at
    ];
    for (i, stem) in hostile_stems.iter().enumerate() {
        // Bare stem: the session NAME is the file stem, and a formula
        // only executes when an introducer leads the CELL — the
        // interesting case is the stem itself leading.
        let file = corpus.join(format!("{stem}.jsonl"));
        fs::write(
            &file,
            format!(
                r#"{{"type": "assistant", "uuid": "a1", "timestamp": "2026-10-0{i}T10:00:05Z", "message": {{"role": "assistant", "model": "=2+5|cmd", "content": [{{"type": "text", "text": "done"}}], "usage": {{"input_tokens": 10, "output_tokens": 5}}}}}}
"#
            ),
        )
        .unwrap();
    }
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();

    let stdout = run_cell(
        &corpus,
        &home,
        &Cell {
            report: "--sessions",
            format: "text",
        },
    );
    let text = String::from_utf8_lossy(&stdout);
    // Transcript-derived text columns in the sessions TSV: name (0),
    // source tool (3), model (4) — the cells guard_formula wraps.
    let mut failures = Vec::new();
    for line in text.lines().skip(1) {
        let cells: Vec<&str> = line.split('\t').collect();
        for col in [0usize, 3, 4] {
            let Some(cell) = cells.get(col) else { continue };
            let introducer = ["=", "+", "-", "@", "\u{ff1d}", "\u{ff20}"]
                .iter()
                .any(|p| cell.starts_with(p));
            if introducer && !cell.starts_with('\'') {
                failures.push(format!("col {col} cell {cell:?} unguarded"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "default-text TSV lane emitted unguarded formula cells (rm-540 arm):\n  {}",
        failures.join("\n  ")
    );
    // And the guard actually fired: every hostile stem shows up
    // apostrophe-prefixed in the name column, never row-leading raw.
    for stem in &hostile_stems {
        assert!(
            text.contains(&format!("'{stem}")),
            "hostile stem {stem:?} missing its formula guard in the TSV lane"
        );
        assert!(
            !text.contains(&format!("\n{stem}\t")),
            "hostile stem {stem:?} rode raw (row-leading) into the TSV lane"
        );
    }
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

/// rm-936 PoC corpus: directional/format controls (Cf) in every
/// transcript-derived position — session-name source (first user message),
/// model id, tool name, assistant text. None of these may survive into any
/// lane; they reorder adjacent cost/model text in terminals and
/// spreadsheets (the OSC-52 sibling of this class rides the same lanes).
fn write_bidi_corpus(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let f = root.join("20261003T100000+0000-poc-bidi.jsonl");
    fs::write(
        &f,
        concat!(
            "{\"type\": \"user\", \"uuid\": \"u1\", \"timestamp\": \"2026-10-03T10:00:00Z\", ",
            "\"cwd\": \"/tmp/at-rm936-poc\", \"message\": {\"role\": \"user\", ",
            "\"content\": \"poc-bidi \u{202e} name \u{202d} tail \u{200f} mark\"}}\n",
            "{\"type\": \"assistant\", \"uuid\": \"a1\", \"timestamp\": \"2026-10-03T10:00:05Z\", ",
            "\"message\": {\"role\": \"assistant\", \"model\": \"claude-sonnet-4\u{202e}5x\", ",
            "\"content\": [ {\"type\": \"tool_use\", \"id\": \"t1\", \"name\": \"Bash\u{2066}iso\", ",
            "\"input\": {\"command\": \"echo hi\"}}, {\"type\": \"text\", \"text\": \"done \u{061c}\"}], ",
            "\"usage\": {\"input_tokens\": 10, \"output_tokens\": 5}}}\n"
        ),
    )
    .unwrap();
    vec![f]
}

#[test]
fn rm936_bidi_format_controls_neutralized_across_every_lane() {
    let tmp = tempdir("output-safety-bidi");
    let corpus = tmp.join("corpus-bidi");
    fs::create_dir_all(&corpus).unwrap();
    write_bidi_corpus(&corpus);
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();

    // Every lane the dispatch boundary knows, plus the two formats the
    // default matrix omits: json (rm-625 carve-out now Cf-filtered) and
    // otel (attribute-value sanitizer).
    let mut cells = matrix();
    cells.push(Cell {
        report: "--overview",
        format: "json",
    });
    cells.push(Cell {
        report: "--sessions",
        format: "json",
    });
    cells.push(Cell {
        report: "--overview",
        format: "otel",
    });
    // Review fix (ff418327 F3): pin the svg lane too — it is one of the
    // seven acceptance lanes and was previously verified only by the
    // implementer's manual PoC, not by this matrix.
    cells.push(Cell {
        report: "--overview",
        format: "svg",
    });

    let raw_cf: [&[u8]; 5] = [
        &[0xe2, 0x80, 0xae], // U+202E RLO
        &[0xe2, 0x80, 0xaa], // U+202A LRE
        &[0xe2, 0x80, 0x8f], // U+200F RLM
        &[0xe2, 0x81, 0xa6], // U+2066 LRI
        &[0xd8, 0x9c],       // U+061C ALM
    ];
    for cell in &cells {
        let out = run_cell(&corpus, &home, cell);
        assert!(
            !out.is_empty(),
            "empty output for {} -f {} (rm-936 lane lost)",
            cell.report,
            cell.format
        );
        for enc in raw_cf {
            assert!(
                !out.windows(enc.len()).any(|w| w == enc),
                "raw Cf directional control leaked into lane {} -f {}",
                cell.report,
                cell.format
            );
        }
    }
}

#[test]
fn zwj_zwnj_joiner_pair_dies_on_render_lanes_and_survives_json() {
    // rm-933 widening arm (2026-10-16, run 7288595f): ZWJ U+200D and
    // ZWNJ U+200C are invisible general-category-Cf characters, and
    // they rode the rendered lanes RAW at d2a02af — od-verified E2 80
    // 8D / E2 80 8C byte-for-byte in the `--sessions` TSV/CSV name and
    // model cells while RLO and NUL on the same rows were
    // U+FFFD-replaced (delegate assess 5dd9fe76). A zero-width joiner
    // glues what a terminal renders as separate cells — the same
    // single-cell spoofing class the directional Cf set already dies
    // for. This arm subscribes the shared line-segment and document
    // sanitizers to the joiner pair (`is_joiner_format_control` in
    // agenttrace-core). The JSON lanes are the CONTROL and must keep
    // the pair raw: losslessness is their recorded contract, and the
    // carve-out is deliberately pinned against the directional
    // predicate only — this arm does NOT widen it.
    let tmp = tempdir("output-safety-zwj");
    let corpus = tmp.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let a = corpus.join("20261001T110000+0000-zwj.jsonl");
    fs::write(
        &a,
        r#"{"type": "user", "uuid": "u1", "timestamp": "2026-10-01T11:00:00Z", "message": {"role": "user", "content": "zw\u200djoin\u200cnj lane probe"}, "cwd": "/tmp/at-zwj-poc"}
{"type": "assistant", "uuid": "a1", "timestamp": "2026-10-01T11:00:05Z", "message": {"role": "assistant", "model": "zw\u200dmodel", "content": [{"type": "text", "text": "done"}], "usage": {"input_tokens": 10, "output_tokens": 5}}}
"#,
    )
    .unwrap();
    let home = tmp.join("home");
    fs::create_dir_all(home.join(".cache")).unwrap();

    let zwj: &[u8] = &[0xe2, 0x80, 0x8d];
    let zwnj: &[u8] = &[0xe2, 0x80, 0x8c];
    let mut failures = Vec::new();
    for cell in matrix() {
        let stdout = run_cell(&corpus, &home, &cell);
        for (name, enc) in [("ZWJ", zwj), ("ZWNJ", zwnj)] {
            if stdout.windows(enc.len()).any(|w| w == enc) {
                failures.push(format!(
                    "raw {name} leaked into {} -f {}",
                    cell.report, cell.format
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "raw joiner-pair bytes reached a rendered lane (rm-933):\n  {}",
        failures.join("\n  ")
    );

    // Vacuity guard: the poison must REACH the lanes (the session name
    // renders from the first user request). The sessions text lane
    // must carry the substituted marker AND the honest body around it.
    let sessions = run_cell(
        &corpus,
        &home,
        &Cell {
            report: "--sessions",
            format: "text",
        },
    );
    assert!(
        sessions.windows(3).any(|w| w == [0xef, 0xbf, 0xbd].as_slice()),
        "no U+FFFD in the sessions lane — the joiner pair never reached the renderer (fixture vacuity)"
    );
    assert!(
        sessions.windows(4).any(|w| w == b"join"),
        "honest text around the joiner pair missing (name fell back? fixture vacuity)"
    );

    // JSON control lanes: the pair survives raw (lossless carve-out).
    for report in ["--sessions", "--overview"] {
        let stdout = run_cell(
            &corpus,
            &home,
            &Cell {
                report,
                format: "json",
            },
        );
        assert!(
            stdout.windows(zwj.len()).any(|w| w == zwj),
            "{report} -f json must keep the ZWJ raw (lossless-by-design carve-out; rm-933 did not widen it)"
        );
    }
}

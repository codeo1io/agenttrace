//! Launch-guard regression tests that exercise the built binary the way
//! a user's shell does: `--version` must work regardless of `--lang`
//! validity (CU-4 / N9), and the default TUI must fail with a normal
//! error — not a Rust panic with exit 101 — when stdout is not a
//! terminal (CU-5 / P4-1: the README quickstart `agenttrace` panics in
//! every piped context).

use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_agenttrace")
}

/// rm-534 / rm-533 test fixture: a minimal but real claude-format
/// journal (the zero_usage_contract line shapes) so the lanes under
/// test load a genuine session instead of failing for an unrelated
/// parse reason.
fn claude_journal_lines() -> String {
    let user = r#"{"type":"user","timestamp":"2026-10-04T01:00:00Z","cwd":"/tmp/guard","message":{"role":"user","content":"check the billing please"}}"#;
    let assistant = r#"{"type":"assistant","timestamp":"2026-10-04T01:00:05Z","message":{"id":"msg_5","model":"claude-sonnet-4-5-20250929","role":"assistant","content":[{"type":"text","text":"on it"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":2,"cache_read_input_tokens":3}}}"#;
    format!("{user}\n{assistant}\n")
}

/// Runs the built binary with every host-derived store pointed at a
/// scratch root so boundary assertions can also pin “nothing else
/// wrote host state”.
fn run_isolated(args: &[&str], scratch: &std::path::Path) -> (i32, String, String) {
    let history_dir = scratch.join("history");
    let cache_dir = scratch.join("cache");
    std::fs::create_dir_all(&history_dir).expect("mkdir history scratch");
    std::fs::create_dir_all(&cache_dir).expect("mkdir cache scratch");
    let output = Command::new(bin())
        .args(args)
        .env("AGENTTRACE_HISTORY_DIR", &history_dir)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn agenttrace");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn run_with_pipes(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(bin())
        .args(args)
        .env(
            "AGENTTRACE_SESSION_CACHE_DIR",
            std::env::temp_dir().join(format!("agenttrace-launch-guard-{}", std::process::id())),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn agenttrace");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn version_wins_over_invalid_lang() {
    // `--lang fr --version` used to fail in report_language() before the
    // --version early return was reachable.
    let (code, stdout, stderr) = run_with_pipes(&["--lang", "fr", "--version"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.starts_with("agenttrace v"),
        "expected version banner, got: {stdout}"
    );
}

#[test]
fn tui_launch_fails_cleanly_when_stdout_is_not_a_terminal() {
    // The README quickstart (`agenttrace`) panicked with exit 101 and a
    // Rust backtrace whenever stdout was a pipe. It must exit with a
    // normal error code and point the user at --overview instead.
    let (code, stdout, stderr) = run_with_pipes(&["--demo"]);
    assert_ne!(
        code, 101,
        "TUI launch must not panic when stdout is piped; stderr: {stderr}"
    );
    assert_ne!(code, 0, "non-tty TUI launch should not report success");
    let message = format!("{stdout}{stderr}").to_ascii_lowercase();
    assert!(
        message.contains("not a terminal"),
        "expected a 'not a terminal' explanation, got: {message}"
    );
    assert!(
        message.contains("--overview"),
        "expected the message to suggest --overview, got: {message}"
    );
}

#[test]
fn range_without_a_session_action_exits_with_a_flag_naming_error() {
    // rm-244: `agenttrace --demo --range 30d` used to enter the TUI and
    // silently ignore the filter. It must exit rc!=0 before the TUI is
    // attempted, name `--range`, and suggest composable actions.
    let (code, stdout, stderr) = run_with_pipes(&["--demo", "--range", "30d"]);
    assert_ne!(
        code, 0,
        "--range without a consumer must not report success"
    );
    let message = format!("{stdout}{stderr}");
    assert!(
        message.contains("--range"),
        "error must name the ignored flag, got: {message}"
    );
    assert!(
        message.contains("--overview"),
        "error must suggest a composable report action, got: {message}"
    );
    let lowered = message.to_ascii_lowercase();
    assert!(
        !lowered.contains("not a terminal"),
        "the guard must fire before TUI launch, got: {message}"
    );
}

#[test]
fn range_with_a_session_action_still_works_piped() {
    // The guard must not over-reject: with a report action consuming it,
    // --range keeps filtering. rm-735 moved this control off the demo
    // corpus (--demo now refuses explicit session sources including a
    // non-default --range, pinned in demo_source_* tests below), so the
    // range filter is exercised against a real freshly-timestamped -d
    // corpus instead: two days old, inside a 30d window. The pinned
    // outcome is the overview (or the pre-existing empty-filter error)
    // - never the applicability guard and never a panic.
    let root = std::env::temp_dir().join(format!(
        "agenttrace-launch-guard-range-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("temp corpus dir");
    let ts = (chrono::Utc::now() - chrono::Duration::days(2))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let corpus = format!(
        "{{\"type\":\"user\",\"timestamp\":\"{ts}\",\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n\
         {{\"type\":\"assistant\",\"timestamp\":\"{ts}\",\"message\":{{\"id\":\"msg_01\",\"model\":\"claude-sonnet-4-20250514\",\"usage\":{{\"input_tokens\":300,\"output_tokens\":30}}}}}}\n"
    );
    let file = root.join("range-control.jsonl");
    std::fs::write(&file, corpus).expect("corpus write");
    let dir_arg = root.to_string_lossy().to_string();
    let (code, stdout, stderr) =
        run_with_pipes(&["--overview", "-d", dir_arg.as_str(), "--range", "30d"]);
    let _ = std::fs::remove_dir_all(&root);
    assert_ne!(code, 101, "must not panic; stderr: {stderr}");
    let message = format!("{stdout}{stderr}");
    assert!(
        !message.contains("--range requires"),
        "the guard must not fire when a report action consumes the flag, got: {message}"
    );
    assert!(
        stdout.contains("sessions") || stderr.contains("No sessions match"),
        "expected the overview or the pre-existing empty-filter error, got: {message}"
    );
}

#[test]
fn demo_with_any_explicit_session_source_fails_loudly_not_silently() {
    // rm-735 (assess F2, run 06cc5c7d): `--demo` used to silently
    // substitute the bundled demo corpus for every explicit session
    // source. Pre-fix, `--demo --overview -d /real/logs` rendered DEMO
    // numbers presented as the user's corpus. The guard now fires once
    // at launch, before any lane dispatch, so every session-reading
    // action is covered by construction; this matrix pins each conflict pair
    // (positional path, -d/--dir, --range) against a representative
    // spread of those actions.
    const SOURCE_FLAGS: [(&str, [&str; 4]); 3] = [
        (
            "a positional path",
            ["--demo", "--overview", "probe.jsonl", ""],
        ),
        (
            "-d/--dir",
            ["--demo", "--overview", "-d", "/tmp/probe-logs"],
        ),
        ("--range", ["--demo", "--overview", "--range", "30d"]),
    ];
    for (source_name, argv) in SOURCE_FLAGS {
        let mut argv = argv.to_vec();
        argv.retain(|s| !s.is_empty());
        let (code, stdout, stderr) = run_with_pipes(&argv);
        assert_ne!(
            code, 0,
            "--demo + {source_name} must not report success (argv: {argv:?}); stdout: {stdout}"
        );
        assert_ne!(
            code, 101,
            "must fail with an error, not a panic; stderr: {stderr}"
        );
        let message = format!("{stdout}{stderr}");
        assert!(
            message.contains("--demo"),
            "error must name --demo, got: {message}"
        );
        assert!(
            message.contains(source_name),
            "error must name the ignored source ({source_name}), got: {message}"
        );
        assert!(
            message.to_ascii_lowercase().contains("ignores"),
            "error must state the substitution is refused, got: {message}"
        );
    }
}

#[test]
fn demo_source_guard_covers_every_session_reading_action() {
    // rm-735 acceptance leg (2): pin the conflict pair on EVERY
    // session-reading action, not just --overview. The launch guard
    // fires before any lane dispatch, so --doctor is in the sweep
    // too (integration: landed rm-596 made `--doctor --demo` render
    // only the bundled corpus, which silently discarded `-d`); each
    // must exit non-zero naming --demo instead of substituting.
    let actions: [(&str, Vec<&str>); 11] = [
        ("--sessions", vec!["--sessions"]),
        ("--search", vec!["--search", "probe"]),
        ("--diagnostics", vec!["--diagnostics"]),
        ("--waste", vec!["--waste"]),
        ("--compare", vec!["--compare"]),
        ("--audit", vec!["--audit"]),
        ("--recommend", vec!["--recommend"]),
        ("--inspect", vec!["--inspect", "1"]),
        ("--latest", vec!["--latest"]),
        ("--doctor", vec!["--doctor"]),
        ("single-session report", vec![]),
    ];
    for (label, action) in actions {
        let mut argv = vec!["--demo", "-d", "/tmp/probe-logs"];
        argv.extend(action.iter().copied());
        if action.is_empty() {
            argv.push("probe.jsonl");
        }
        let argv_refs: Vec<&str> = argv.to_vec();
        let (code, stdout, stderr) = run_with_pipes(&argv_refs);
        assert_ne!(
            code, 0,
            "--demo + -d on {label} must not report success; stdout: {stdout}"
        );
        let message = format!("{stdout}{stderr}");
        assert!(
            message.contains("--demo") && message.contains("-d/--dir"),
            "{label}: error must name --demo and -d/--dir, got: {message}"
        );
    }
}

#[test]
fn demo_source_guard_leaves_legal_demo_invocations_alone() {
    // rm-735 control: the guard must not over-reject. A bare --demo
    // overview still works (rc 0), and an explicit `--range all` is
    // indistinguishable from the clap default and stays legal.
    let (code, stdout, stderr) = run_with_pipes(&["--demo", "--overview"]);
    assert_eq!(
        code, 0,
        "bare --demo --overview must keep working; stderr: {stderr}"
    );
    assert!(stdout.contains("sessions"), "expected a rendered overview");

    let (code, stdout, stderr) = run_with_pipes(&["--demo", "--overview", "--range", "all"]);
    assert_eq!(
        code, 0,
        "explicit --range all is the default and must stay legal; stderr: {stderr}"
    );
    assert!(stdout.contains("sessions"), "expected a rendered overview");
}

#[test]
fn demo_preserve_history_exits_with_a_flag_naming_error() {
    // rm-533: `--demo --preserve-history` used to fabricate 4 rows
    // ($0.8950) into the REAL history.json and every later
    // history-consuming report repeated them (assess b6c398bf PoC).
    // The demo lane must stay hermetic: the pair is rejected loudly,
    // with both flags named, before anything is loaded or rendered.
    let (code, stdout, stderr) = run_with_pipes(&["--demo", "--preserve-history", "--overview"]);
    assert_ne!(code, 0, "the pair must not succeed; stdout: {stdout}");
    assert!(
        stderr.contains("--demo") && stderr.contains("--preserve-history"),
        "expected a flag-naming error, got: {stderr}"
    );
    assert!(
        !stdout.contains("Saved:"),
        "no output may be written before the hermeticity guard fires, got: {stdout}"
    );
}

#[test]
fn demo_preserve_history_leaves_host_state_untouched() {
    // rm-533 host-state pin: with a pre-existing history.json under a
    // scratch AGENTTRACE_HISTORY_DIR and an empty scratch session cache,
    // the rejected invocation must leave both byte-identical/empty —
    // demo state never reaches host stores by any path.
    let scratch = std::env::temp_dir().join(format!("at-rm533-host-state-{}", std::process::id()));
    let history_dir = scratch.join("history");
    let cache_dir = scratch.join("cache");
    std::fs::create_dir_all(&history_dir).expect("mkdir history scratch");
    std::fs::create_dir_all(&cache_dir).expect("mkdir cache scratch");
    let history_path = history_dir.join("history.json");
    let sentinel: &[u8] = b"{\"sentinel\":\"pre-existing host history must survive a --demo run\"}";
    std::fs::write(&history_path, sentinel).expect("seed scratch history");

    let (code, _stdout, stderr) =
        run_isolated(&["--demo", "--preserve-history", "--overview"], &scratch);
    assert_ne!(code, 0, "stderr: {stderr}");
    assert!(
        stderr.contains("--demo") && stderr.contains("--preserve-history"),
        "the hermeticity guard must be the failure, got: {stderr}"
    );
    assert_eq!(
        std::fs::read(&history_path).expect("history survives"),
        sentinel,
        "real history.json must be byte-identical after the rejected --demo run"
    );
    assert_eq!(
        std::fs::read_dir(&cache_dir)
            .expect("cache dir survives")
            .count(),
        0,
        "the session cache must stay empty"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

#[test]
fn output_colliding_with_input_journal_is_rejected_without_touching_it() {
    // rm-534 positional arm, the assess PoC verbatim:
    // `--overview -o X.jsonl X.jsonl` used to exit 0 with a "Saved:"
    // banner while write_output's temp+rename atomically replaced the
    // just-loaded transcript with the rendered report. It must exit
    // non-zero naming --output, with the journal byte-identical.
    let scratch = std::env::temp_dir().join(format!("at-rm534-positional-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("mkdir scratch");
    let journal = scratch.join("X.jsonl");
    std::fs::write(&journal, claude_journal_lines()).expect("write journal");
    let before = std::fs::read(&journal).expect("read journal before");
    let journal_arg = journal.to_str().expect("utf-8 scratch path").to_string();

    let (code, stdout, stderr) =
        run_isolated(&["--overview", "-o", &journal_arg, &journal_arg], &scratch);
    assert_ne!(code, 0, "the collision must not succeed; stdout: {stdout}");
    assert!(
        stderr.contains("--output") && stderr.contains("input session journal"),
        "expected a --output membership error, got: {stderr}"
    );
    assert_eq!(
        std::fs::read(&journal).expect("read journal after"),
        before,
        "the input journal must be byte-identical (no staging, no rename)"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

#[test]
fn output_colliding_with_walked_journal_is_rejected_without_touching_it() {
    // rm-534 -d-walk arm: `-o <name>.jsonl` colliding with a journal a
    // discovery walk admitted must be rejected identically — the
    // membership check covers every loaded source, not just positional
    // files.
    let scratch = std::env::temp_dir().join(format!("at-rm534-walk-{}", std::process::id()));
    let sessions_dir = scratch.join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("mkdir sessions scratch");
    let journal = sessions_dir.join("victim.jsonl");
    std::fs::write(&journal, claude_journal_lines()).expect("write journal");
    let before = std::fs::read(&journal).expect("read journal before");
    let dir_arg = sessions_dir
        .to_str()
        .expect("utf-8 scratch path")
        .to_string();
    let journal_arg = journal.to_str().expect("utf-8 scratch path").to_string();

    let (code, stdout, stderr) = run_isolated(
        &["--overview", "-d", &dir_arg, "-o", &journal_arg],
        &scratch,
    );
    assert_ne!(
        code, 0,
        "the walk-arm collision must not succeed; stdout: {stdout}"
    );
    assert!(
        stderr.contains("--output") && stderr.contains("input session journal"),
        "expected a --output membership error, got: {stderr}"
    );
    assert_eq!(
        std::fs::read(&journal).expect("read journal after"),
        before,
        "the walked journal must be byte-identical"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

#[test]
fn output_colliding_with_unparseable_walked_journal_is_rejected() {
    // rm-912 (assess F2 PoC): the -o membership check used to cover
    // only the sessions the loader PRODUCED, so a discovered-but-
    // unparseable journal (here: a real journal saved as UTF-16LE, the
    // classic Windows PowerShell redirect) was invisible to the guard
    // — `--overview -d dir -o dir/bad.jsonl` exited 0 and OVERWROTE
    // the unparseable input (assess live PoC: bad.jsonl replaced, only
    // a silent .orig twin left behind). The guard must cover every
    // path the loader ATTEMPTED: parse-failure paths join the
    // membership set and the run refuses BEFORE any write, naming the
    // input transcript — the user's only unfixable copy is never
    // destroyed.
    let scratch = std::env::temp_dir().join(format!(
        "agenttrace-rm912-mixed-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let sessions_dir = scratch.join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("mkdir sessions scratch");
    std::fs::write(sessions_dir.join("good.jsonl"), claude_journal_lines())
        .expect("write good journal");
    let mut utf16: Vec<u8> = vec![0xff, 0xfe];
    utf16.extend(
        claude_journal_lines()
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes()),
    );
    let victim = sessions_dir.join("bad.jsonl");
    std::fs::write(&victim, utf16).expect("write bad journal");
    let before = std::fs::read(&victim).expect("read bad journal before");
    let dir_arg = sessions_dir
        .to_str()
        .expect("utf-8 scratch path")
        .to_string();
    let victim_arg = victim.to_str().expect("utf-8 scratch path").to_string();

    let (code, stdout, stderr) =
        run_isolated(&["--overview", "-d", &dir_arg, "-o", &victim_arg], &scratch);
    assert_ne!(
        code, 0,
        "-o at an attempted-but-unparseable input must refuse; stdout: {stdout}"
    );
    assert!(
        stderr.contains("--output") && stderr.contains("failed to parse"),
        "refusal must name --output and the parse failure, got: {stderr}"
    );
    assert!(
        stderr.contains("bad.jsonl"),
        "refusal must name the input transcript, got: {stderr}"
    );
    assert_eq!(
        std::fs::read(&victim).expect("read bad journal after"),
        before,
        "the unparseable input must stay byte-identical — no overwrite, no .orig twin"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

#[test]
fn all_unparseable_dir_with_colliding_output_keeps_the_parse_bail() {
    // rm-912 regression pin: when EVERY discovered journal fails to
    // parse, the rm-835 all-failed bail still fires FIRST — the run is
    // condemned for having produced nothing, long before the -o
    // collision matters. The guard must not reorder or swallow that.
    let scratch = std::env::temp_dir().join(format!(
        "agenttrace-rm912-allbad-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let sessions_dir = scratch.join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("mkdir sessions scratch");
    let mut utf16: Vec<u8> = vec![0xff, 0xfe];
    utf16.extend(
        claude_journal_lines()
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes()),
    );
    let victim = sessions_dir.join("bad.jsonl");
    std::fs::write(&victim, utf16).expect("write bad journal");
    let before = std::fs::read(&victim).expect("read bad journal before");
    let dir_arg = sessions_dir
        .to_str()
        .expect("utf-8 scratch path")
        .to_string();
    let victim_arg = victim.to_str().expect("utf-8 scratch path").to_string();

    let (code, _stdout, stderr) =
        run_isolated(&["--overview", "-d", &dir_arg, "-o", &victim_arg], &scratch);
    assert_ne!(code, 0, "an all-failed corpus must stay a failure");
    assert!(
        stderr.contains("No sessions parsed"),
        "the rm-835 all-failed bail must win over the -o guard, got: {stderr}"
    );
    assert_eq!(
        std::fs::read(&victim).expect("read bad journal after"),
        before,
        "nothing may be written either way"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

// ---------------------------------------------------------------------------
// rm-997 (batch run 12e2c18c implement): `--search` with an explicitly
// passed empty/whitespace operand. Red-first: against the pre-change
// binary the empty operand matched no action row (the table only
// counted a non-empty query), fell through run() into the interactive
// TUI launcher, and a piped stdout rendered the launcher's own failure
// ("stdout is not a terminal", assess aa5eb07a P-SEARCH PoC) instead of
// a usage error — and `--search '' --overview` silently reported rc0
// with an empty-string header. The guard must reject the operand loudly
// and BEFORE any output staging or loading.
// ---------------------------------------------------------------------------

#[test]
fn empty_search_operand_is_a_loud_usage_error() {
    let scratch = std::env::temp_dir().join(format!(
        "agenttrace-rm997-empty-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(scratch.join("sessions")).expect("mkdir sessions scratch");
    let sessions_dir = scratch
        .join("sessions")
        .to_str()
        .expect("utf-8 scratch path")
        .to_string();

    for operand in ["", "   "] {
        let out_path = scratch.join(format!("out-{:?}.jsonl", operand));
        let out_arg = out_path.to_str().expect("utf-8 path").to_string();
        let (code, stdout, stderr) = run_isolated(
            &["--search", operand, "-d", &sessions_dir, "-o", &out_arg],
            &scratch,
        );
        assert_ne!(
            code, 0,
            "an explicitly empty/whitespace --search operand ({operand:?}) must be an error"
        );
        assert!(
            stdout.is_empty(),
            "no report bytes on stdout for a rejected operand, got: {stdout}"
        );
        assert!(
            stderr.contains("--search"),
            "the usage error must name the flag, got: {stderr}"
        );
        assert!(
            stderr.contains("non-empty"),
            "the usage error must say what is wrong, got: {stderr}"
        );
        assert!(
            !stderr.contains("stdout is not a terminal"),
            "the rejection must happen in validate_primary_action, not by \
             falling through into the interactive launcher, got: {stderr}"
        );
        assert!(
            !out_path.exists(),
            "a rejected operand must not stage or write any --output file"
        );
    }
    std::fs::remove_dir_all(&scratch).ok();
}

#[test]
fn empty_search_operand_never_reaches_the_launcher_or_companions() {
    let scratch = std::env::temp_dir().join(format!(
        "agenttrace-rm997-companion-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(scratch.join("sessions")).expect("mkdir sessions scratch");
    let sessions_dir = scratch
        .join("sessions")
        .to_str()
        .expect("utf-8 scratch path")
        .to_string();

    // The companion arm of the PoC: `--search '' --overview` used to
    // render rc0 with an `Search results: ""` header. The empty operand
    // must dominate the companion flag.
    let (code, stdout, stderr) = run_isolated(
        &["--search", "", "--overview", "-d", &sessions_dir],
        &scratch,
    );
    assert_ne!(code, 0, "the empty operand must dominate --overview");
    assert!(
        stdout.is_empty(),
        "no overview bytes may render for a rejected operand, got: {stdout}"
    );
    assert!(
        stderr.contains("--search"),
        "error must name --search, got: {stderr}"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

#[test]
fn nonempty_zero_hit_search_output_is_unchanged() {
    let scratch = std::env::temp_dir().join(format!(
        "agenttrace-rm997-zerohit-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let sessions_dir = scratch.join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("mkdir sessions scratch");
    std::fs::write(sessions_dir.join("a.jsonl"), claude_journal_lines())
        .expect("write claude journal");
    let sessions_arg = sessions_dir
        .to_str()
        .expect("utf-8 scratch path")
        .to_string();

    // A non-empty query with zero hits keeps its pre-change story
    // byte-for-byte: rc0, the Search-results header naming the query
    // and the 0 count (assess PoC pinned `--search z` rc0).
    let (code, stdout, stderr) = run_isolated(&["--search", "z", "-d", &sessions_arg], &scratch);
    assert_eq!(code, 0, "zero-hit is a successful empty answer");
    assert!(
        stdout.contains("Search results"),
        "the header must render, got: {stdout}"
    );
    assert!(
        stdout.contains("\"z\""),
        "header names the query, got: {stdout}"
    );
    assert!(
        stdout.contains("0"),
        "header reports the 0 count, got: {stdout}"
    );
    assert!(
        stderr.is_empty(),
        "no warning for a clean zero-hit search, got: {stderr}"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

// ---------------------------------------------------------------------------
// rm-995 (batch run 12e2c18c implement): the CLI surfaces of the
// generation-speed analytics — `--sessions -f json` carries per-row
// `generation_tokens_per_sec` (identical cold/warm: render-time
// arithmetic, not a cache field) and `time_to_first_token_sec` (null
// warm: the cache does not round-trip event stamps, and null is the
// honest basis), plus the sibling `generation_speed` rollup mirroring
// the overview's. The overview text renders the "── Generation speed
// ──" section with its basis sizes. Fixture: 20 counted tokens
// (10+5+2+3) over a 5s span (01:00:00 -> 01:00:05) = 4.0 tok/s, TTFT
// 5s.
// ---------------------------------------------------------------------------

#[test]
fn sessions_json_and_overview_text_carry_generation_speed() {
    let scratch = std::env::temp_dir().join(format!(
        "agenttrace-rm995-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let sessions_dir = scratch.join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("mkdir sessions scratch");
    std::fs::write(sessions_dir.join("a.jsonl"), claude_journal_lines())
        .expect("write claude journal");
    let sessions_arg = sessions_dir
        .to_str()
        .expect("utf-8 scratch path")
        .to_string();

    // Cold pass.
    let (code, stdout, stderr) =
        run_isolated(&["--sessions", "-f", "json", "-d", &sessions_arg], &scratch);
    assert_eq!(code, 0, "sessions json exits clean, stderr: {stderr}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).expect("sessions json parses");
    assert_eq!(
        doc.pointer("/generation_speed/tokens_per_sec")
            .and_then(serde_json::Value::as_f64),
        Some(4.0),
        "sibling rollup: 20 tokens / 5s = 4.0 tok/s: {stdout}"
    );
    let row = doc.pointer("/sessions/0").expect("one session row");
    assert_eq!(
        row.get("generation_tokens_per_sec")
            .and_then(serde_json::Value::as_f64),
        Some(4.0),
        "per-row rate: {row}"
    );
    assert_eq!(
        row.get("time_to_first_token_sec")
            .and_then(serde_json::Value::as_f64),
        Some(5.0),
        "cold row carries TTFT (assistant stamp 5s after first): {row}"
    );

    // Warm pass over the same scratch cache: rate identical, TTFT
    // honestly null.
    let (code, stdout, stderr) =
        run_isolated(&["--sessions", "-f", "json", "-d", &sessions_arg], &scratch);
    assert_eq!(code, 0, "warm sessions json exits clean, stderr: {stderr}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).expect("warm sessions json parses");
    let row = doc.pointer("/sessions/0").expect("warm session row");
    assert_eq!(
        row.get("generation_tokens_per_sec")
            .and_then(serde_json::Value::as_f64),
        Some(4.0),
        "warm-cache rate identical to cold (render-time arithmetic): {row}"
    );
    assert!(
        row.get("time_to_first_token_sec")
            .map(serde_json::Value::is_null)
            == Some(true),
        "warm-cache TTFT must be null (cache does not round-trip event stamps): {row}"
    );

    // Overview text, COLD (fresh scratch): section + basis sizes,
    // including the TTFT clause.
    let cold_scratch = std::env::temp_dir().join(format!(
        "agenttrace-rm995-cold-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let cold_sessions = cold_scratch.join("sessions");
    std::fs::create_dir_all(&cold_sessions).expect("mkdir cold sessions");
    std::fs::write(cold_sessions.join("a.jsonl"), claude_journal_lines())
        .expect("write cold journal");
    let cold_arg = cold_sessions
        .to_str()
        .expect("utf-8 scratch path")
        .to_string();
    let (code, stdout, stderr) = run_isolated(&["--overview", "-d", &cold_arg], &cold_scratch);
    assert_eq!(code, 0, "overview exits clean, stderr: {stderr}");
    assert!(
        stdout.contains("── Generation speed ──"),
        "overview renders the speed section: {stdout}"
    );
    assert!(stdout.contains("4.0 tok/s"), "rate renders: {stdout}");
    assert!(
        stdout.contains("corpus: 4.0 tok/s over 1 session(s) with a span"),
        "corpus basis renders: {stdout}"
    );
    assert!(
        stdout.contains("time-to-first-token: 5.0s avg over 1 session(s)"),
        "cold overview carries the TTFT clause: {stdout}"
    );
    std::fs::remove_dir_all(&cold_scratch).ok();

    // Overview text, WARM (the scratch that already cached): the rate
    // and its basis render identically, but the TTFT clause is
    // honestly absent — no parse-time stamps survive the cache.
    let (code, stdout, stderr) = run_isolated(&["--overview", "-d", &sessions_arg], &scratch);
    assert_eq!(code, 0, "warm overview exits clean, stderr: {stderr}");
    assert!(
        stdout.contains("corpus: 4.0 tok/s over 1 session(s) with a span"),
        "warm overview keeps the rate: {stdout}"
    );
    assert!(
        !stdout.contains("time-to-first-token"),
        "warm overview must not fabricate a TTFT basis: {stdout}"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

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
    // numbers presented as the user's corpus. The guard now fires on
    // the single load chokepoint, so every session-reading action is
    // covered by construction; this matrix pins each conflict pair
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
    // session-reading action, not just --overview. All of these load
    // sessions through load_sessions_report, where the guard lives;
    // each must exit non-zero naming --demo instead of substituting.
    let actions: [(&str, Vec<&str>); 10] = [
        ("--sessions", vec!["--sessions"]),
        ("--search", vec!["--search", "probe"]),
        ("--diagnostics", vec!["--diagnostics"]),
        ("--waste", vec!["--waste"]),
        ("--compare", vec!["--compare"]),
        ("--audit", vec!["--audit"]),
        ("--recommend", vec!["--recommend"]),
        ("--inspect", vec!["--inspect", "1"]),
        ("--latest", vec!["--latest"]),
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

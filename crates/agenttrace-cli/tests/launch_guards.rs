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
    // --range keeps filtering. The pinned demo timestamps fall outside
    // a 30d window, so the legitimate outcome here is either the
    // overview or the pre-existing empty-filter error - never the
    // applicability guard and never a panic.
    let (code, stdout, stderr) = run_with_pipes(&["--demo", "--range", "30d", "--overview"]);
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

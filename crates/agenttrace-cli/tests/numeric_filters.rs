//! rm-364: `--cost` / `--health` numeric filters share one finite-only dialect.
//!
//! Red transcripts (base b0e12d44, pre-fix): `--cost '>=NaN'` exited 1 with
//! "No sessions match the requested filters" — indistinguishable from an
//! honest empty result — while `--cost '<=inf'` vacuously passed everything
//! and `--cost '>=abc'` hard-errored: three semantics for one typo class.
//! These tests pin the unified dialect: non-finite thresholds are rejected at
//! parse time with the unchanged `invalid --cost/--health filter:` message,
//! and a bare number means `>=` in the CLI exactly as it already did in the
//! TUI.

use std::path::PathBuf;
use std::process::Command;

fn generated_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/generated")
}

/// Run the CLI with HOME / XDG cache sandboxed per test so the session cache
/// can neither leak between tests nor pollute the developer's real cache.
fn run_agenttrace(tag: &str, args: &[&str]) -> (Option<i32>, String, String) {
    let sandbox =
        std::env::temp_dir().join(format!("at-numeric-filters-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(sandbox.join("cache")).expect("create sandbox cache");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &sandbox)
        .env("XDG_CACHE_HOME", sandbox.join("cache"))
        .env("TMPDIR", std::env::temp_dir())
        .args(args)
        .output()
        .expect("run agenttrace CLI");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn assert_invalid_filter(flag: &str, filter: &str) {
    let (code, stdout, stderr) = run_agenttrace(
        "invalid",
        &[
            "--sessions",
            "-d",
            generated_dir().to_str().unwrap(),
            "-f",
            "json",
            flag,
            filter,
        ],
    );
    assert_eq!(
        code,
        Some(1),
        "{flag} {filter:?} must exit 1, stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(
        stderr.contains(&format!("invalid {flag} filter: {filter}")),
        "expected the invalid-filter message for {flag} {filter:?}, stderr={stderr:?}"
    );
    assert!(
        !stderr.contains("No sessions match") && !stdout.contains("No sessions match"),
        "{flag} {filter:?} must be a parse-time rejection, not a silent no-op"
    );
}

#[test]
fn nonfinite_cost_filters_are_rejected_at_parse_time() {
    for filter in [
        ">=NaN",
        "<=NaN",
        ">NaN",
        "=NaN",
        "<=inf",
        ">=inf",
        ">inf",
        ">-inf",
        "<-inf",
        "=inf",
        ">=1e400",
        "<=1e400",
        "1e400",
        ">=infinity",
    ] {
        assert_invalid_filter("--cost", filter);
    }
}

#[test]
fn nonfinite_health_filters_are_rejected_at_parse_time() {
    for filter in [">=NaN", "<=inf", ">=1e400"] {
        assert_invalid_filter("--health", filter);
    }
}

#[test]
fn invalid_text_filter_message_shape_is_unchanged() {
    // Control: the pre-existing hard-error class keeps its message and rc.
    assert_invalid_filter("--cost", ">=abc");
    assert_invalid_filter("--health", "nope");
}

#[test]
fn bare_number_means_gte_for_cost() {
    let generated = generated_dir();
    let dir = generated.to_str().unwrap();
    let (code_bare, stdout_bare, stderr_bare) = run_agenttrace(
        "bare-cost",
        &["--sessions", "-d", dir, "-f", "json", "--cost", "0"],
    );
    let (code_prefixed, stdout_prefixed, stderr_prefixed) = run_agenttrace(
        "bare-cost-prefixed",
        &["--sessions", "-d", dir, "-f", "json", "--cost", ">=0"],
    );
    assert_eq!(
        code_bare, code_prefixed,
        "bare and prefixed forms must both succeed: {stderr_bare:?} {stderr_prefixed:?}"
    );
    assert_eq!(code_bare, Some(0), "stderr={stderr_bare:?}");
    assert!(
        stdout_bare.trim_start().starts_with('['),
        "expected a JSON session list, got {stdout_bare:?}"
    );
    assert_eq!(
        stdout_bare, stdout_prefixed,
        "`--cost 0` must filter identically to `--cost '>=0'`"
    );
}

#[test]
fn bare_number_means_gte_for_health() {
    let generated = generated_dir();
    let dir = generated.to_str().unwrap();
    let (code_bare, stdout_bare, stderr_bare) = run_agenttrace(
        "bare-health",
        &["--sessions", "-d", dir, "-f", "json", "--health", "0"],
    );
    let (code_prefixed, stdout_prefixed, _) = run_agenttrace(
        "bare-health-prefixed",
        &["--sessions", "-d", dir, "-f", "json", "--health", ">=0"],
    );
    assert_eq!(code_bare, Some(0), "stderr={stderr_bare:?}");
    assert_eq!(code_bare, code_prefixed);
    assert_eq!(
        stdout_bare, stdout_prefixed,
        "`--health 0` must filter identically to `--health '>=0'`"
    );
}

#[test]
fn whitespace_after_operator_is_accepted_in_the_unified_dialect() {
    // The TUI always accepted `>= 0.10`; the CLI now shares that dialect.
    let generated = generated_dir();
    let dir = generated.to_str().unwrap();
    let (code_spaced, stdout_spaced, stderr_spaced) = run_agenttrace(
        "spaced",
        &["--sessions", "-d", dir, "-f", "json", "--cost", ">= 0"],
    );
    assert_eq!(code_spaced, Some(0), "stderr={stderr_spaced:?}");
    let (code_tight, stdout_tight, _) = run_agenttrace(
        "tight",
        &["--sessions", "-d", dir, "-f", "json", "--cost", ">=0"],
    );
    assert_eq!(code_spaced, code_tight);
    assert_eq!(stdout_spaced, stdout_tight);
}

#[test]
fn finite_filters_keep_their_historical_behavior() {
    let generated = generated_dir();
    let dir = generated.to_str().unwrap();
    for (flag, filter) in [
        ("--cost", ">=0.01"),
        ("--cost", "<=1000000"),
        ("--health", ">=79.5"),
        ("--health", "good"),
    ] {
        let (code, stdout, stderr) = run_agenttrace(
            "finite",
            &["--sessions", "-d", dir, "-f", "json", flag, filter],
        );
        assert_eq!(
            code,
            Some(0),
            "{flag} {filter:?} must keep working, stderr={stderr:?}"
        );
        assert!(
            stdout.trim_start().starts_with('['),
            "{flag} {filter:?}: expected JSON sessions, got {stdout:?}"
        );
    }
}

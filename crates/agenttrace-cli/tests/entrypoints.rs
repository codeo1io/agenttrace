use std::path::PathBuf;
use std::process::Command;

fn generated_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/generated")
        .join(name)
}

#[test]
fn cli_entrypoint_reads_generated_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--sessions",
            "--limit",
            "1",
            generated_fixture("detailed-tool-steps.jsonl")
                .to_str()
                .expect("fixture path is valid UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI");

    assert!(output.status.success(), "CLI failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SESSION\tHEALTH\tDATA"));
}

#[test]
fn cli_version_wins_over_action_validation() {
    // Pass-6 P6-2: `--overview --version` used to exit 1 because action
    // validation ran before the version early-return while `--version` is
    // itself an action, contradicting the CHANGELOG claim. Version must
    // win over argument validation, in either order.
    for flag_order in [
        vec!["--overview", "--version"],
        vec!["--version", "--overview"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .args(&flag_order)
            .output()
            .expect("run agenttrace CLI");
        assert!(
            output.status.success(),
            "--version must win over action validation: {:?}",
            output
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.starts_with("agenttrace v"),
            "expected version banner, got {stdout:?}"
        );
    }
}

#[test]
fn baseline_regression_gates_the_exit_code_and_opt_out_flags_work() {
    // Pass-7 P7-3: `--baseline-max-*-delta-pct` used to leave the breach
    // booleans buried in the JSON while the process exited 0 — a gate
    // that never gates. A breach must exit 2 (like --fail-under-health)
    // unless --no-baseline-gate opts out.
    let work = std::env::temp_dir().join(format!(
        "agenttrace-baseline-gate-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&work).expect("create temp dir");
    let report = work.join("report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--demo",
            "--overview",
            "-f",
            "json",
            "-o",
            report.to_str().expect("report path is valid UTF-8"),
        ])
        .output()
        .expect("generate demo report");
    assert!(output.status.success(), "demo report failed: {output:?}");
    // Forge a baseline that reports zero tokens while the run reports
    // thousands: any token threshold breach must trip the gate.
    let mut baseline: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).expect("read report"))
            .expect("report json");
    baseline["summary"]["total_tokens"] = serde_json::json!(0);
    let baseline_path = work.join("baseline.json");
    std::fs::write(
        &baseline_path,
        serde_json::to_string(&baseline).expect("serialize baseline"),
    )
    .expect("write baseline");

    let gated = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--demo",
            "--overview",
            "-f",
            "json",
            "--baseline",
            baseline_path
                .to_str()
                .expect("baseline path is valid UTF-8"),
            "--baseline-max-token-delta-pct",
            "1",
        ])
        .output()
        .expect("run gated compare");
    assert_eq!(
        gated.status.code(),
        Some(2),
        "token regression above threshold must exit 2, got {:?}",
        gated.status
    );
    let stderr = String::from_utf8_lossy(&gated.stderr);
    assert!(
        stderr.contains("baseline regression"),
        "stderr must name the failed gate, got: {stderr}"
    );
    assert!(
        stderr.contains("--no-baseline-gate"),
        "stderr must name the opt-out, got: {stderr}"
    );
    // The report JSON is still produced before the gate fires.
    assert!(!gated.stdout.is_empty(), "report still prints");

    let opted_out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--demo",
            "--overview",
            "-f",
            "json",
            "--baseline",
            baseline_path
                .to_str()
                .expect("baseline path is valid UTF-8"),
            "--baseline-max-token-delta-pct",
            "1",
            "--no-baseline-gate",
        ])
        .output()
        .expect("run opted-out compare");
    assert!(
        opted_out.status.success(),
        "--no-baseline-gate must keep the run green, got {:?}",
        opted_out.status
    );
    let stdout = String::from_utf8_lossy(&opted_out.stdout);
    assert!(
        stdout.contains("\"baseline_comparison\""),
        "comparison stays in the report when opted out"
    );
    let _ = std::fs::remove_dir_all(work);
}

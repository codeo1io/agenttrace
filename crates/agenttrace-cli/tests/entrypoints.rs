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

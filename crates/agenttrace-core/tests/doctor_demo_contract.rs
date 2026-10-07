//! rm-596 contract: `--doctor --demo` walks the bundled demo corpus and
//! never the operator's real sessions. The demo report is built from
//! [`agenttrace_core::demo_sessions`] alone — zero files walked, zero
//! discovery roots enumerated, session cache zeroed — while the non-demo
//! lanes behave exactly as before (pinned here as a control arm).

use agenttrace_core::{build_doctor_report, render_doctor_report};
use std::fs;
use std::time::Instant;

const PLANTED_SESSION: &str = concat!(
    "{\"role\":\"session_meta\",\"timestamp\":\"2026-05-01T09:00:00Z\",\"ModelUsed\":\"claude-sonnet-4-5\"}\n",
    "{\"role\":\"meta\",\"ModelUsed\":\"claude-sonnet-4-5\",\"Usage\":{\"input_tokens\":120,\"output_tokens\":30}}\n",
);

fn planted_corpus(marker: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("at-rm596-core-{marker}-{}", std::process::id()));
    let project = root.join("projects").join(format!("home-agent-{marker}"));
    fs::create_dir_all(&project).expect("create planted project dir");
    fs::write(project.join("session.jsonl"), PLANTED_SESSION).expect("plant session file");
    root
}

#[test]
fn doctor_demo_dir_arg_gates_the_directory_lane() {
    let root = planted_corpus("dirlane");
    let marker_path = root.to_string_lossy().to_string();

    let report = build_doctor_report(Some(&root), true);
    assert_eq!(report.mode, "demo sessions");
    assert_eq!(report.sessions, 3, "demo must report the bundled corpus");
    assert_eq!(report.session_files, 0, "demo must walk no session files");
    assert!(
        report.directories.is_empty(),
        "demo must not enumerate discovery roots: {:?}",
        report.directories
    );
    let json = render_doctor_report(Some(&root), true, "json").expect("render demo json");
    assert!(
        !json.contains(&marker_path) && !json.contains("at-rm596-core-dirlane"),
        "demo report leaked the probed directory: {json}"
    );

    // Control: without --demo the same directory is walked and disclosed,
    // proving the gate changed the demo lane only.
    let control = build_doctor_report(Some(&root), false);
    assert_eq!(control.mode, "custom directory");
    assert!(control.session_files >= 1);
    assert!(control.directories.iter().any(|d| d.files >= 1));
    let control_json = render_doctor_report(Some(&root), false, "json").expect("render json");
    assert!(
        control_json.contains(&marker_path),
        "control must disclose the probed directory: {control_json}"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn doctor_demo_auto_discovery_is_fast_and_deterministic() {
    // Pre-rm-596 this lane walked the operator's real corpus (17k+
    // sessions, ~119s) while labeling the report "demo sessions".
    let start = Instant::now();
    let first = render_doctor_report(None, true, "json").expect("demo json");
    let elapsed = start.elapsed();
    let second = render_doctor_report(None, true, "json").expect("demo json");
    assert_eq!(first, second, "demo doctor output must be deterministic");
    let value: serde_json::Value = serde_json::from_str(&first).expect("parse demo json");
    assert_eq!(value["mode"], "demo sessions");
    assert_eq!(value["sessions"], 3u64);
    assert_eq!(value["session_files"], 0u64);
    assert_eq!(
        value["directories"].as_array().map(Vec::len),
        Some(0),
        "demo must not enumerate the operator's discovery roots"
    );
    assert!(
        elapsed.as_secs() < 5,
        "demo doctor must not walk the real corpus (took {elapsed:?})"
    );
}

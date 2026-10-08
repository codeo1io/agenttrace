//! rm-596 acceptance (binary-level): `agenttrace --doctor --demo` must
//! read and disclose nothing from the operator's home — no discovery
//! root, no planted project, no cache content — while the same
//! invocation without `--demo` still discovers the planted corpus.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const PLANTED_SESSION: &str = concat!(
    "{\"role\":\"session_meta\",\"timestamp\":\"2026-05-01T09:00:00Z\",\"ModelUsed\":\"claude-sonnet-4-5\"}\n",
    "{\"role\":\"meta\",\"ModelUsed\":\"claude-sonnet-4-5\",\"Usage\":{\"input_tokens\":120,\"output_tokens\":30}}\n",
);

fn planted_home(marker: &str) -> (PathBuf, PathBuf) {
    // The marker must exist ONLY inside the planted home: the sandbox
    // root itself (and the XDG cache redirected under it) shows up in
    // machinery path strings by design, so its name stays marker-free.
    let root = std::env::temp_dir().join(format!("at-rm596-cli-{}", std::process::id()));
    let home = root.join("home");
    let projects = home
        .join(".claude")
        .join("projects")
        .join(format!("home-agent-{marker}"));
    fs::create_dir_all(&projects).expect("create planted projects dir");
    for name in ["session-1.jsonl", "session-2.jsonl"] {
        fs::write(projects.join(name), PLANTED_SESSION).expect("plant session file");
    }
    let cache = root.join("cache");
    fs::create_dir_all(&cache).expect("create sandbox cache dir");
    (home, cache)
}

fn doctor(home: &Path, cache: &Path, demo: bool) -> (String, Duration) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
    cmd.arg("--doctor")
        .arg("-f")
        .arg("json")
        .env("HOME", home)
        .env("XDG_CACHE_HOME", cache)
        .env_remove("AGENTTRACE_SESSION_CACHE_DIR")
        .env_remove("XDG_CONFIG_HOME");
    if demo {
        cmd.arg("--demo");
    }
    let start = Instant::now();
    let output = cmd.output().expect("spawn agenttrace");
    let elapsed = start.elapsed();
    assert!(
        output.status.success(),
        "doctor failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8(output.stdout).expect("utf8 stdout"),
        elapsed,
    )
}

#[test]
fn doctor_demo_discloses_no_operator_home_paths() {
    let marker = format!("secretproj{}", std::process::id());
    let (home, cache) = planted_home(&marker);
    let home_str = home.to_string_lossy().to_string();

    let (demo_json, elapsed) = doctor(&home, &cache, true);
    assert!(
        elapsed.as_secs() < 5,
        "demo doctor must not walk the planted corpus (took {elapsed:?})"
    );
    assert!(
        !demo_json.contains(&home_str),
        "demo report leaked the operator home: {demo_json}"
    );
    assert!(
        !demo_json.contains(&marker),
        "demo report leaked the planted corpus: {demo_json}"
    );
    let demo: serde_json::Value = serde_json::from_str(&demo_json).expect("parse demo json");
    assert_eq!(demo["mode"], "demo sessions");
    assert_eq!(
        demo["sessions"], 3u64,
        "demo must report the bundled corpus"
    );
    assert_eq!(demo["session_files"], 0u64);
    assert_eq!(
        demo["directories"].as_array().map(Vec::len),
        Some(0),
        "demo must not enumerate the operator's discovery roots"
    );
    // rm-298 capacity arm: the doctor JSON discloses the entry bound
    // actually in force, its layer, and the re-parse cost. In the
    // planted home no knob is set, so the defaults hold.
    assert_eq!(
        demo["cache_entry_bound"],
        agenttrace_core::MAX_SESSION_CACHE_ENTRIES as u64
    );
    assert_eq!(demo["cache_entry_bound_source"], "default");
    assert_eq!(demo["reparsed_this_scan"], 0u64);

    // Control: without --demo the planted corpus IS discovered and
    // disclosed — the demo gate must not change the real lane.
    let (real_json, _) = doctor(&home, &cache, false);
    let real: serde_json::Value = serde_json::from_str(&real_json).expect("parse json");
    assert_eq!(real["mode"], "auto-discovery");
    assert!(
        real["sessions"].as_u64().unwrap_or(0) >= 2,
        "control must find the planted sessions: {real_json}"
    );
    assert!(
        real_json.contains(&marker),
        "control must disclose the planted project: {real_json}"
    );
    fs::remove_dir_all(home.parent().expect("sandbox root")).ok();
}

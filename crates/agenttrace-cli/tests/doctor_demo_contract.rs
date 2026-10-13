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

#[test]
fn doctor_positional_root_refuses_missing_and_file_valued_paths() {
    // rm-1012 (run 975ac79d assess F2): the positional scan root must
    // get the same exists + is_dir refusal as the --dir arm (rm-608) —
    // pre-fix `--doctor /nonexistent-dir-xyz` and `--doctor <file>`
    // exited rc=0 reporting a "healthy" scan over 0 sessions, the
    // exact false-healthy class main.rs's own rm-608 comment names.
    let root = std::env::temp_dir().join(format!("at-rm1012-cli-{}", std::process::id()));
    let home = root.join("home");
    let cache = root.join("cache");
    fs::create_dir_all(&home).expect("home dir");
    fs::create_dir_all(&cache).expect("cache dir");
    let file_valued = root.join("plain-file.txt");
    fs::write(&file_valued, "not a directory\n").expect("plant plain file");
    let missing = root.join("nonexistent-dir-xyz");

    let run = |args: &[std::ffi::OsString]| {
        Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .arg("--doctor")
            .arg("-f")
            .arg("json")
            .args(args)
            .env("HOME", &home)
            .env("XDG_CACHE_HOME", &cache)
            .env_remove("AGENTTRACE_SESSION_CACHE_DIR")
            .env_remove("XDG_CONFIG_HOME")
            .output()
            .expect("spawn agenttrace")
    };

    // Nonexistent positional: rc=2, stderr names the path, and stdout
    // stays EMPTY — never rc=0 with a healthy 0-session report.
    let out = run(&[missing.clone().into()]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "missing positional must refuse with rc=2, got {:?}; stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "refusal must precede any report");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("session directory does not exist"),
        "refusal must name the failure class: {stderr}"
    );
    assert!(
        stderr.contains("nonexistent-dir-xyz"),
        "refusal must name the positional: {stderr}"
    );

    // File-valued positional: same rc=2 refusal, is_not_a_directory
    // wording.
    let out = run(&[file_valued.clone().into()]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "file-valued positional must refuse with rc=2, got {:?}; stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("is not a directory") && stderr.contains("plain-file.txt"),
        "refusal must name the file-valued positional: {stderr}"
    );

    // Contrast pin: the --dir arm's refusal stays byte-identical to the
    // landed rm-608 shape (this batch must not touch it).
    let out = run(&["-d".into(), "/nonexistent-dir-xyz".into()]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert_eq!(
        stderr,
        "agenttrace: session directory does not exist: /nonexistent-dir-xyz\n\
         - inspect: the -d/--dir value; drop -d to auto-discover agent homes\n"
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn doctor_positional_path_is_honored_as_scan_root() {
    // rm-439 arm (run 57c491338818 assess F2): `--doctor <positional>`
    // must scan that root. Pre-fix, the positional is silently ignored
    // and the scan runs over the DEFAULT agent homes — on a normal host
    // that is a wrong-scope report, and on a large store an apparent
    // hang (49,457-file walk observed). The planted home here makes the
    // wrong-scope outcome observable without any hang risk.
    // rm-439 arm: DISTINCT marker — the sibling test derives its marker
    // from the same pid, so a shared name makes the two tests delete
    // each other's planted project during cleanup.
    let marker = format!("secretprojpositional{}", std::process::id());
    let (home, cache) = planted_home(&marker);
    let fixtures =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../agenttrace-core/tests/fixtures/generic");

    // rm-439 arm: this test must not share the pid-keyed doctor cache
    // with the file's other concurrent test — two writers racing one
    // sessions.json make the sibling's discovery flaky. Give this scan
    // its own cache via the supported override.
    let own_cache =
        std::env::temp_dir().join(format!("at-rm439-positional-{}", std::process::id()));
    let _ = fs::remove_dir_all(&own_cache);
    fs::create_dir_all(&own_cache).expect("own cache dir");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--doctor")
        .arg("-f")
        .arg("json")
        .arg(&fixtures)
        .env("HOME", &home)
        .env("XDG_CACHE_HOME", &cache)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &own_cache)
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .expect("spawn agenttrace");
    assert!(
        output.status.success(),
        "doctor with a positional root failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("parse doctor json");
    let dirs = report["directories"]
        .as_array()
        .map(|a| serde_json::to_string(a).unwrap_or_default())
        .unwrap_or_default();
    assert!(
        dirs.contains("fixtures"),
        "the positional root must be scanned: directories={dirs}"
    );
    assert!(
        report["session_files"].as_u64().unwrap_or(0) >= 1,
        "the positional root's journals must be counted: {stdout}"
    );
    assert!(
        !stdout.contains(&marker),
        "default agent homes must not be scanned behind the operator's back: {stdout}"
    );
    // rm-439 arm: planted_home's sandbox is PID-keyed and shared with
    // the file's other concurrent test (same home dir, different marker
    // projects) — clean up ONLY this test's marker project and its
    // private cache; never the shared home/root.
    fs::remove_dir_all(
        home.join(".claude")
            .join("projects")
            .join(format!("home-agent-{marker}")),
    )
    .ok();
    fs::remove_dir_all(&own_cache).ok();
}

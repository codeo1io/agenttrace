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

/// rm-367 stage (a) made the non-demo scan PERSIST fresh parses through
/// the session cache — this control arm must never reach the operator's
/// real cache, so the lane runs under a sandboxed home (the
/// pi_family_discovery `with_home` pattern).
fn with_sandboxed_home(f: impl FnOnce()) {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    fn restore(key: &str, previous: Option<std::ffi::OsString>) {
        match previous {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    let result = {
        let home = std::env::temp_dir().join(format!("at-rm596-doctor-{}", std::process::id()));
        fs::create_dir_all(&home).expect("create sandbox home");
        let previous_home = std::env::var_os("HOME");
        let previous_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_cache = std::env::var_os("XDG_CACHE_HOME");
        let previous_data = std::env::var_os("XDG_DATA_HOME");
        let previous_session_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore("HOME", previous_home);
        restore("XDG_CONFIG_HOME", previous_config);
        restore("XDG_CACHE_HOME", previous_cache);
        restore("XDG_DATA_HOME", previous_data);
        restore("AGENTTRACE_SESSION_CACHE_DIR", previous_session_cache);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn doctor_demo_dir_arg_gates_the_directory_lane() {
    with_sandboxed_home(|| {
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
    });
}

#[test]
fn doctor_demo_auto_discovery_is_fast_and_deterministic() {
    // Pre-rm-596 this lane walked the operator's real corpus (17k+
    // sessions, ~119s) while labeling the report "demo sessions".
    // Runs under the same sandboxed-home lock as the gate test above:
    // cargo runs this binary's tests in parallel, and the demo report
    // embeds the session-cache path derived from the environment.
    with_sandboxed_home(|| {
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
    });
}

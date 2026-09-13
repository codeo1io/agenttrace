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

fn run_json(args: &[&str]) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(args)
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "agenttrace {:?} failed: {:?}",
        args,
        output
    );
    serde_json::from_slice(&output.stdout).expect("parse JSON report")
}

#[test]
fn governance_audit_matches_overview_totals_and_discloses_coverage() {
    // Pass-8 F8-1: governance reports used to sample the newest 20
    // sessions by default (176x cost understatement on the operator
    // corpus, exit 0, no disclosure). By default every matching session
    // is audited, the JSON discloses audited_sessions/total_sessions,
    // and the audit totals equal the overview's cost audit on the same
    // corpus.
    let audit = run_json(&["--demo", "--audit", "-f", "json"]);
    assert_eq!(audit["audited_sessions"], 3, "demo audit covers 3 sessions");
    assert_eq!(audit["total_sessions"], 3);
    assert!(
        audit["excluded_reason"].is_null(),
        "default run excludes nothing silently"
    );
    let overview = run_json(&["--demo", "--overview", "-f", "json"]);
    assert_eq!(
        audit["total_estimated_cost"], overview["cost_audit"]["total_estimated_cost"],
        "audit totals must equal overview totals on the same corpus"
    );
    assert_eq!(
        overview["data_health"]["discovered"], 3,
        "overview discovered count must cover the whole demo corpus"
    );
}

#[test]
fn governance_sampling_is_explicit_and_disclosed() {
    // Bounded sampling exists only behind --sample and always discloses
    // both counts and the exclusion reason.
    let sampled = run_json(&["--demo", "--audit", "-f", "json", "--sample", "2"]);
    assert_eq!(sampled["audited_sessions"], 2);
    assert_eq!(sampled["total_sessions"], 3);
    let reason = sampled["excluded_reason"]
        .as_str()
        .expect("sampled run names its exclusion");
    assert!(
        reason.contains("--sample 2"),
        "exclusion reason must name the sampling flag: {reason}"
    );
    let full = run_json(&["--demo", "--audit", "-f", "json"]);
    assert_ne!(
        sampled["total_estimated_cost"], full["total_estimated_cost"],
        "sampling a subset must change the aggregate"
    );
    // CU-21: the reason names the active view. `--sample` takes the first
    // N of the --sort/--order ordering, so the disclosure must say which
    // ordering produced the sample instead of claiming "newest".
    let sorted = run_json(&[
        "--demo", "--audit", "-f", "json", "--sample", "2", "--sort", "cost", "--order", "asc",
    ]);
    let sorted_reason = sorted["excluded_reason"]
        .as_str()
        .expect("sorted sample names its exclusion");
    assert!(
        sorted_reason.contains("--sort cost"),
        "reason must name the sort: {sorted_reason}"
    );
    assert!(
        sorted_reason.contains("--order asc"),
        "reason must name the order: {sorted_reason}"
    );
    // The --compare branch carries its own copy of the disclosure string;
    // pin it too so the twin cannot drift back to "newest".
    let compared = run_json(&[
        "--demo",
        "--compare",
        "-f",
        "json",
        "--sample",
        "2",
        "--sort",
        "cost",
        "--order",
        "asc",
    ]);
    let compared_reason = compared["excluded_reason"]
        .as_str()
        .expect("compare sample names its exclusion");
    assert!(
        compared_reason.contains("--sort cost") && compared_reason.contains("--order asc"),
        "compare reason must name the view: {compared_reason}"
    );

    let text = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--audit", "--sample", "2"])
        .output()
        .expect("run text audit");
    assert!(text.status.success());
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("(auditing 2 of 3 sessions)"),
        "text output must disclose coverage, got: {stdout}"
    );

    let zero = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--audit", "--sample", "0"])
        .output()
        .expect("run zero-sample audit");
    assert_eq!(
        zero.status.code(),
        Some(1),
        "--sample 0 must be rejected loudly, got {:?}",
        zero.status
    );

    let recommend = run_json(&["--demo", "--recommend", "-f", "json"]);
    assert_eq!(recommend["audited_sessions"], 3);
    assert!(
        recommend["recommendations"].is_array(),
        "wrapped recommendation list stays addressable"
    );
}

#[test]
fn overview_limit_caps_list_views_only() {
    // Pass-3 P3-5: --overview used to ignore --limit entirely while the
    // documented --baseline CI recipe used it. The limit is now a
    // display cap for list views (recent_sessions); every aggregate
    // still covers the whole corpus.
    let capped = run_json(&["--demo", "--overview", "-f", "json", "--limit", "2"]);
    assert_eq!(
        capped["recent_sessions"]
            .as_array()
            .expect("recent_sessions list")
            .len(),
        2,
        "--limit must cap the recent_sessions list view"
    );
    assert_eq!(
        capped["summary"]["total_sessions"], 3,
        "aggregates must stay unbounded by --limit"
    );
    let full = run_json(&["--demo", "--overview", "-f", "json"]);
    assert_eq!(
        full["recent_sessions"]
            .as_array()
            .expect("recent_sessions list")
            .len(),
        3,
        "default limit (20) keeps every demo session visible up to the internal cap"
    );
}

#[test]
fn no_baseline_gate_is_a_boolean_not_a_value_flag() {
    // Cycle-7 shim fix (pass-11 A11-5, cycle-4 review F2 — found twice,
    // independently): `--no-baseline-gate` was registered in
    // `flag_takes_value`, so the Go-flag shim consumed the following
    // positional as its value and kept scanning instead of stopping at
    // the first positional. Go semantics: flags after the session path
    // are ignored. With the misregistration,
    // `--sessions --no-baseline-gate <fixture> --overview` let the
    // post-positional `--overview` through and the run died on
    // "choose exactly one report action"; with the fix the trailing
    // flag is ignored and the session list prints.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--sessions",
            "--no-baseline-gate",
            generated_fixture("detailed-tool-steps.jsonl")
                .to_str()
                .expect("fixture path is valid UTF-8"),
            "--overview",
        ])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "trailing post-positional flags must be ignored, not validated: {:?}",
        output
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SESSION\tHEALTH\tDATA"),
        "the session list must print, got {stdout:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("choose exactly one report action"),
        "the post-positional --overview must never reach action validation"
    );
}

#[test]
fn statusline_host_mode_never_fails_the_host() {
    // Candidate 53 (cycle 7): Claude Code invokes the statusLine command
    // on every prompt; the host contract is exactly one stdout line and
    // exit 0 for any input — valid payload, garbage, or nothing. The
    // capture journal follows AGENTTRACE_SESSION_CACHE_DIR so the test
    // owns it.
    let cache = std::env::temp_dir().join(format!(
        "agenttrace-statusline-entrypoint-{}",
        std::process::id()
    ));
    let run = |stdin: &str| {
        use std::io::Write as _;
        let mut child = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .arg("statusline")
            .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn agenttrace statusline");
        child
            .stdin
            .as_mut()
            .expect("stdin piped")
            .write_all(stdin.as_bytes())
            .expect("write stdin");
        child.wait_with_output().expect("collect statusline output")
    };

    let payload = r#"{"session_id":"s1","session_name":"cycle 7","model":{"display_name":"Opus 4.5"},"cost":{"total_cost_usd":1.25},"context_window":{"used_percentage":41.0},"rate_limits":{"five_hour":{"used_percentage":84.0,"resets_at":1760000000}},"prompt_cache":{"hit_ratio":0.9,"misses":4,"last_miss_cause":{"causes":["tools_changed"]}}}"#;
    let valid = run(payload);
    assert!(valid.status.success(), "valid payload must exit 0");
    let stdout = String::from_utf8_lossy(&valid.stdout);
    assert_eq!(stdout.lines().count(), 1, "exactly one line: {stdout:?}");
    assert!(
        stdout.contains("cycle 7") && stdout.contains("5h 84%") && stdout.contains("cache 90%"),
        "status fields must render: {stdout:?}"
    );
    let stderr = String::from_utf8_lossy(&valid.stderr);
    assert!(
        stderr.trim().is_empty(),
        "valid payloads report nothing on stderr: {stderr:?}"
    );

    let garbage = run("definitely not json {");
    assert!(garbage.status.success(), "garbage must exit 0");
    let stdout = String::from_utf8_lossy(&garbage.stdout);
    assert_eq!(
        stdout.trim(),
        "agenttrace",
        "the fallback line is the binary name: {stdout:?}"
    );
    assert!(
        !String::from_utf8_lossy(&garbage.stderr).trim().is_empty(),
        "the diagnostic goes to stderr"
    );

    let empty = run("");
    assert!(empty.status.success(), "empty stdin must exit 0");
    assert_eq!(String::from_utf8_lossy(&empty.stdout).trim(), "agenttrace");

    // The journal captured exactly the valid payloads, and the report
    // reads them back keyed by session_id.
    let journal = cache.join("statusline.jsonl");
    assert!(journal.exists(), "the capture journal must exist");
    let report = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--statusline-report", "-f", "json"])
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .output()
        .expect("run statusline report");
    assert!(report.status.success());
    let stdout = String::from_utf8_lossy(&report.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("report is JSON");
    assert_eq!(parsed["insights"]["sessions"], 1);
    assert_eq!(
        parsed["insights"]["five_hour"]["used_percentage"], 84.0,
        "payload fields must land in the report"
    );
    assert!(
        stdout.contains("tools_changed"),
        "cache-miss causes must land in the report"
    );
    let _ = std::fs::remove_dir_all(cache);
}

#[test]
fn statusline_host_mode_survives_stdout_write_failure() {
    // Review F2 (cycle 7): the host contract is exit 0 for any input —
    // and for any stdout condition. A host whose pipe is closed or whose
    // disk is full must not get a panic (was: exit 101 "failed printing
    // to stdout"). /dev/full fails every write with ENOSPC.
    use std::io::Write as _;
    let cache = std::env::temp_dir().join(format!(
        "agenttrace-statusline-devfull-{}",
        std::process::id()
    ));
    let devfull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full");
    let mut child = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("statusline")
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::from(devfull))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn agenttrace statusline against /dev/full");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(br#"{"session_name":"full disk"}"#)
        .expect("write stdin");
    let output = child.wait_with_output().expect("collect output");
    assert!(
        output.status.success(),
        "stdout write failure must not fail the host: {}",
        output.status
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "no panic text on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(cache);
}

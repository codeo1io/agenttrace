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
fn help_exits_zero() {
    // Maestro finding 47fa1154 (run 3988bfe5): the installed-runtime probe
    // runs `agenttrace --help` and requires exit code 0. A release binary
    // built on a newer distro (GLIBC_2.39) failed that probe from the
    // loader on Ubuntu 22.04 after a silent install. Pin the probe contract
    // at the source: the built CLI must answer --help with rc 0 and
    // non-empty stdout, next to the --version contract above.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--help")
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "--help must exit 0 (installed-runtime probe contract), got {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "--help must print usage to stdout, got {stdout:?}"
    );
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

// ---- Cycle-4 B1/B2: pricing overrides fail loud; -d paths fail distinctly ----
// Red-first evidence: before this batch, AGENTTRACE_PRICING_FILE={bad json,
// wrong schema, negative rate} all produced rc0 + empty stderr while
// --test-match kept advertising the bundled catalog (assessment 8acb07dc N1,
// same shape as ccusage #1810 in the wild).

fn unique_temp(name: &str, contents: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("at-cycle4-b1-{}-{}.json", name, std::process::id()));
    std::fs::write(&path, contents).expect("write temp pricing file");
    path
}

fn run_with_pricing_file(file: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--test-match")
        .env("AGENTTRACE_PRICING_FILE", file)
        .output()
        .expect("run agenttrace --test-match")
}

#[test]
fn pricing_override_invalid_json_fails_loudly() {
    let file = unique_temp("bad", "not json at all");
    let out = run_with_pricing_file(&file);
    let _ = std::fs::remove_file(&file);
    assert!(
        out.status.success(),
        "warning posture keeps rc0: {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stderr.contains("AGENTTRACE_PRICING_FILE ignored") && stderr.contains("invalid JSON"),
        "stderr must name the file and the reason: {stderr}"
    );
    assert!(
        stdout.contains("user overrides FAILED"),
        "the pricing label must disclose the failure: {stdout}"
    );
    assert!(
        !stdout.contains("overrides applied"),
        "a failed file must never claim applied overrides: {stdout}"
    );
}

#[test]
fn pricing_override_wrong_schema_names_the_unknown_key() {
    // The natural trap: pasting a LiteLLM snapshot straight into the env var.
    let file = unique_temp(
        "docschema",
        r#"{"gpt-4o": {"input_cost_per_token": 2.5e-6, "mode": "chat"}}"#,
    );
    let out = run_with_pricing_file(&file);
    let _ = std::fs::remove_file(&file);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown field") && stderr.contains("gpt-4o"),
        "reason must name the offending key: {stderr}"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("user overrides FAILED"),
        "label must disclose: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn pricing_override_negative_rate_is_rejected_by_name() {
    let file = unique_temp(
        "neg",
        r#"{"prices":{"my-model":{"input":-5,"output":1,"cw":0,"cr":0}}}"#,
    );
    let out = run_with_pricing_file(&file);
    let _ = std::fs::remove_file(&file);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("my-model") && stderr.contains("negative") && stderr.contains("input"),
        "reason must name model, field, defect: {stderr}"
    );
}

#[test]
fn pricing_override_success_discloses_applied_overrides() {
    let file = unique_temp(
        "good",
        r#"{"prices":{"my-model":{"input":5,"output":10,"cw":0,"cr":0}},"aliases":{"mm":"my-model"}}"#,
    );
    let out = run_with_pricing_file(&file);
    let _ = std::fs::remove_file(&file);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "rc0 on success: {:?}", out.status);
    assert!(
        stdout.contains("user overrides applied") && stdout.contains("1 model(s)"),
        "label must count what was applied: {stdout}"
    );
    assert!(
        stdout.contains("good"),
        "label must name the override file: {stdout}"
    );
    assert!(
        !stderr.contains("ignored"),
        "a good file must not warn: {stderr}"
    );
}

#[test]
fn doctor_honors_the_same_dir_guard_as_sessions() {
    // rm-412 (run 6557b823): --doctor used to exit 0 on a typo'd -d
    // ("Mode: custom directory"). One -d contract: rc2 + the same
    // message family the sessions path already enforces.
    let missing = std::env::temp_dir().join(format!("at-rm412-missing-{}", std::process::id()));
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--doctor", "-d", missing.to_str().expect("utf8")])
        .output()
        .expect("run agenttrace doctor with missing -d");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "bad request class, same as the sessions path: {stderr}"
    );
    assert!(
        stderr.contains("session directory does not exist"),
        "message family must match the sessions path: {stderr}"
    );
    assert!(
        !stdout_of(&out).contains("Mode: custom directory"),
        "a typo'd dir must not render a doctor report: {}",
        stdout_of(&out)
    );

    // File-as-dir: same class, same helper arm.
    let file = std::env::temp_dir().join(format!("at-rm412-file-{}", std::process::id()));
    std::fs::write(&file, b"not a directory").expect("write file-as-dir fixture");
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--doctor", "-d", file.to_str().expect("utf8")])
        .output()
        .expect("run agenttrace doctor on file-as-dir");
    let _ = std::fs::remove_file(&file);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "file-as-dir is a bad request on --doctor too: {stderr}"
    );
    assert!(
        stderr.contains("-d/--dir is not a directory"),
        "message must name the file-as-dir case: {stderr}"
    );

    // A valid directory keeps today's rc0 doctor behavior.
    let empty = std::env::temp_dir().join(format!("at-rm412-empty-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("create empty dir");
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--doctor", "-d", empty.to_str().expect("utf8")])
        .output()
        .expect("run agenttrace doctor on a valid empty dir");
    let _ = std::fs::remove_dir(&empty);
    assert_eq!(
        out.status.code(),
        Some(0),
        "valid directory must keep the rc0 doctor report: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn stdout_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn nonexistent_session_dir_exits_two_with_distinct_message() {
    let missing = std::env::temp_dir().join(format!("at-cycle4-b2-missing-{}", std::process::id()));
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--sessions", "-d", missing.to_str().expect("utf8")])
        .output()
        .expect("run agenttrace with missing -d");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "bad request class, not empty result"
    );
    assert!(
        stderr.contains("does not exist"),
        "message must say the directory is missing: {stderr}"
    );
    assert!(
        !stderr.contains("No session files found"),
        "must not be confusable with the empty-directory case: {stderr}"
    );
    // B2: --dir works as the long form of -d.
    let out_long = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--sessions", "--dir", missing.to_str().expect("utf8")])
        .output()
        .expect("run agenttrace with missing --dir");
    assert_eq!(out_long.status.code(), Some(2), "--dir long form parses");
}

#[test]
fn empty_session_dir_message_differs_from_missing() {
    let empty = std::env::temp_dir().join(format!("at-cycle4-b2-empty-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("create empty dir");
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--sessions", "-d", empty.to_str().expect("utf8")])
        .output()
        .expect("run agenttrace on empty -d");
    let _ = std::fs::remove_dir(&empty);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "empty corpus stays rc1");
    assert!(
        stderr.contains("No session files found") && stderr.contains("holds no session files"),
        "message must describe the empty-directory case: {stderr}"
    );
    assert!(
        !stderr.contains("does not exist"),
        "must not be confusable with the missing-directory case: {stderr}"
    );
}

#[test]
fn early_exit_test_match_honors_output_and_json() {
    // rm-341 (run 71a7d5db, cycle 3): the early exits used to ignore -o
    // and -f json. --test-match must write the requested file and emit a
    // JSON document under -f json.
    let tmp = std::env::temp_dir().join(format!("agenttrace-rm341-tm-{}", std::process::id()));
    let out_file = tmp.join("tm.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--test-match",
            "-o",
            out_file.to_str().expect("path is UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI");
    assert!(output.status.success(), "CLI failed: {:?}", output);
    assert!(out_file.is_file(), "-o must be honored by --test-match");
    let saved = std::fs::read_to_string(&out_file).expect("read -o target");
    assert!(
        saved.starts_with("Pricing:"),
        "unexpected file body: {saved:?}"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--test-match", "-f", "json"])
        .output()
        .expect("run agenttrace CLI");
    assert!(output.status.success(), "CLI failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("--test-match -f json must emit JSON");
    assert!(
        value["models"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        "expected a non-empty models array, got {stdout}"
    );
    assert!(
        value["source"].as_str().is_some(),
        "expected a pricing source field, got {stdout}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn early_exit_list_models_honors_output_and_json() {
    // rm-341: --list-models must honor -o and -f json (previously wrote
    // text to stdout regardless and never created the file).
    let tmp = std::env::temp_dir().join(format!("agenttrace-rm341-lm-{}", std::process::id()));
    let out_file = tmp.join("lm.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--list-models",
            "-o",
            out_file.to_str().expect("path is UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI");
    assert!(output.status.success(), "CLI failed: {:?}", output);
    assert!(out_file.is_file(), "-o must be honored by --list-models");

    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--list-models", "-f", "json"])
        .output()
        .expect("run agenttrace CLI");
    assert!(output.status.success(), "CLI failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("--list-models -f json must emit JSON");
    assert!(
        value["models"]
            .as_object()
            .is_some_and(|rows| !rows.is_empty()),
        "expected a non-empty models map, got {stdout}"
    );
    assert!(
        value["source"].as_str().is_some(),
        "expected a pricing source field, got {stdout}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn baseline_compare_pair_fails_loudly_and_coherently() {
    // rm-341: '--compare --baseline f' used to claim "--baseline requires
    // --overview -f json", pointing users at a combination that then died
    // with a DIFFERENT error ("choose exactly one report action"). The
    // pair must be rejected up front with one truthful message.
    for argv in [
        vec!["agenttrace", "--compare", "--baseline", "f.json"],
        vec![
            "agenttrace",
            "--overview",
            "--compare",
            "--baseline",
            "f.json",
            "-f",
            "json",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .args(&argv[1..])
            .output()
            .expect("run agenttrace CLI");
        assert!(
            !output.status.success(),
            "{argv:?} must exit non-zero, got {:?}",
            output.status
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--baseline cannot be combined with --compare"),
            "{argv:?} must name both flags, got stderr: {stderr}"
        );
    }

    // The multi-action rejection without --baseline is unchanged.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--overview", "--compare"])
        .output()
        .expect("run agenttrace CLI");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("choose exactly one report action"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn readme_documents_compare_and_baseline_flags() {
    // rm-341: --compare was completely undocumented (grep-empty across
    // README and docs/guides in the assess pass); keep it documented.
    let readme = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md"),
    )
    .expect("read README.md");
    assert!(
        readme.contains("--compare"),
        "README must document --compare"
    );
    assert!(
        readme.contains("--baseline"),
        "README must document --baseline gating"
    );
}

#[test]
fn clear_cache_json_stdout_is_a_single_json_document() {
    // rm-301: `--clear-cache --overview -f json` used to prefix
    // "Session cache cleared." to the machine-readable report, so a
    // downstream `jq` broke on the very first line. Under `-f json` the
    // side-effect announcements must move to stderr (stdout = one
    // parseable JSON document end to end); the human path keeps them
    // on stdout exactly as before.
    let sandbox = std::env::temp_dir().join(format!(
        "agenttrace-rm301-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let cache = sandbox.join("cache");
    std::fs::create_dir_all(&cache).expect("create sandbox cache dir");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            // --clear-cache removes every cache artifact through their
            // env-aware constructors; pin them all inside the sandbox.
            .env("HOME", &sandbox)
            .env("XDG_CACHE_HOME", &cache)
            .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
            .args(args)
            .output()
            .expect("run agenttrace CLI")
    };

    let machine = run(&["--clear-cache", "--overview", "--demo", "-f", "json"]);
    assert!(
        machine.status.success(),
        "CLI failed: {:?}",
        String::from_utf8_lossy(&machine.stderr)
    );
    let stdout = String::from_utf8_lossy(&machine.stdout);
    assert!(
        stdout.starts_with('{'),
        "stdout under -f json must start with the JSON document, got {stdout:?}"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim_end()).expect("stdout parses as one JSON document");
    assert!(parsed.is_object(), "the overview report is a JSON object");
    let stderr = String::from_utf8_lossy(&machine.stderr);
    assert!(
        stderr.contains("Session cache cleared."),
        "the announcement moves to stderr, not away: {stderr:?}"
    );

    let human = run(&["--clear-cache", "--overview", "--demo"]);
    assert!(
        human.status.success(),
        "human-path CLI failed: {:?}",
        String::from_utf8_lossy(&human.stderr)
    );
    let human_stdout = String::from_utf8_lossy(&human.stdout);
    assert!(
        human_stdout.contains("Session cache cleared."),
        "the human path keeps its announcements on stdout"
    );
    let _ = std::fs::remove_dir_all(&sandbox);
}

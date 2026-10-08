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
fn compare_honors_the_fail_under_health_gate() {
    // rm-486: the --compare branch's own comment claims the same coverage
    // contract as the governance branch, but it returned Ok(()) without
    // evaluating the --fail-* gates — a gated `--compare` exited rc0 on a
    // corpus the same flags condemn on every sibling report, so a CI gate
    // built on --compare passed silently.
    let work = std::env::temp_dir().join(format!(
        "agenttrace-compare-gate-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&work).expect("create temp dir");
    // One unhealthy session (failed tool call) keeps health below 100.
    std::fs::write(
        work.join("unhealthy.jsonl"),
        r#"{"type":"user","message":{"role":"user","content":"run the thing"},"sessionId":"s1","timestamp":"2026-05-07T02:00:00Z"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tu1","name":"bash","input":{"command":"ls"}}]},"sessionId":"s1","timestamp":"2026-05-07T02:00:05Z"}
{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu1","content":"boom","is_error":true}]},"sessionId":"s1","timestamp":"2026-05-07T02:00:06Z"}
"#,
    )
    .expect("write unhealthy session");

    // rm-301 style sandbox (see `discover_via_env_overrides_agent_default`
    // at the bottom of this file): the spawned CLI loads and saves the
    // session cache, so without pinning HOME/XDG_CACHE_HOME/
    // AGENTTRACE_SESSION_CACHE_DIR this test would read and REWRITE the
    // operator's real ~/.cache/agenttrace/sessions.json (cycle-1 review
    // 08e99143, F2 — caught by mtime).
    let cache = work.join("cache");
    std::fs::create_dir_all(&cache).expect("create sandbox cache dir");
    let gated = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &work)
        .env("XDG_CACHE_HOME", &cache)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .args([
            "-d",
            work.to_str().expect("temp dir is valid UTF-8"),
            "--compare",
            "--fail-under-health",
            "100",
        ])
        .output()
        .expect("run gated compare");
    assert_eq!(
        gated.status.code(),
        Some(2),
        "--compare must exit 2 when the gate fails, got {:?}",
        gated.status
    );
    let stderr = String::from_utf8_lossy(&gated.stderr);
    assert!(
        stderr.contains("Gate failed"),
        "expected gate failure evidence on stderr, got: {stderr}"
    );

    let _ = std::fs::remove_dir_all(work);
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
    // the first positional. With the misregistration,
    // `--sessions --no-baseline-gate <fixture> --overview` let the
    // post-positional `--overview` through and the run died on
    // "choose exactly one report action".
    //
    // rm-247 (cycle 3) flipped the tail contract deliberately: flags
    // after the positional are now a LOUD usage error instead of being
    // silently ignored, because silent deletion made documented
    // invocations lie (`<file> -o out.txt` exited 0 without writing
    // anything). The rejection must name the dropped flag, must exit 2
    // (clap usage-error convention), and the flag must still never
    // reach action validation.
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
    assert_eq!(
        output.status.code(),
        Some(2),
        "flags after the positional are a loud usage error: {:?}",
        output
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--overview"),
        "the rejection must name the dropped flag, got {stderr:?}"
    );
    assert!(
        !stderr.contains("choose exactly one report action"),
        "the post-positional --overview must never reach action validation"
    );

    // The original boolean registration still holds: with flags placed
    // before the positional, `--no-baseline-gate` must not swallow the
    // fixture as its value, and the session list prints.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--sessions",
            "--no-baseline-gate",
            generated_fixture("detailed-tool-steps.jsonl")
                .to_str()
                .expect("fixture path is valid UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "flags before the positional must parse: {:?}",
        output
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SESSION\tHEALTH\tDATA"),
        "the session list must print, got {stdout:?}"
    );
}

#[test]
fn positional_path_hits_the_gate_like_the_dash_d_control() {
    // Cycle-3 review F1 (attempt 3fb3e39d): rm-246's acceptance requires
    // a committed regression pinning the positional-path gate exit code
    // against the `-d` control. The unit truth-table pins the guard
    // predicate only — a re-introduced earlier single-session dispatch
    // in run() would pass it and silently skip the gate again. This
    // test pins the END-TO-END contract: identical fixture, identical
    // gate verdict, only the path-delivery form differs, and neither
    // form may exit 0.
    let fixture = generated_fixture("detailed-tool-steps.jsonl"); // health 70
    let control_dir =
        std::env::temp_dir().join(format!("agenttrace-rm246-reviewfix-{}", std::process::id()));
    std::fs::create_dir_all(&control_dir).expect("create control dir");
    let control_file = control_dir.join("detailed-tool-steps.jsonl");
    std::fs::copy(&fixture, &control_file).expect("copy fixture into control dir");

    fn gate_verdict(stderr: &str) -> &str {
        stderr
            .lines()
            .find(|line| line.starts_with("Gate failed:"))
            .unwrap_or_else(|| panic!("expected a gate verdict on stderr, got {stderr:?}"))
    }

    let positional = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--overview",
            "--fail-under-health",
            "100",
            fixture.to_str().expect("fixture path is valid UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI (positional form)");
    assert_eq!(
        positional.status.code(),
        Some(2),
        "a positional session path must not dodge the quality gate: {:?}",
        positional
    );
    let positional_stderr = String::from_utf8_lossy(&positional.stderr);
    let positional_verdict = gate_verdict(&positional_stderr);

    let control = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--overview",
            "--fail-under-health",
            "100",
            "-d",
            control_dir.to_str().expect("control dir is valid UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI (-d control form)");
    assert_eq!(
        control.status.code(),
        Some(2),
        "the -d control must keep failing the gate: {:?}",
        control
    );
    let control_stderr = String::from_utf8_lossy(&control.stderr);
    let control_verdict = gate_verdict(&control_stderr);

    assert_eq!(
        positional_verdict, control_verdict,
        "positional and -d forms must yield the identical gate verdict"
    );
    assert!(
        positional_verdict.contains("70.0") && positional_verdict.contains("below 100"),
        "verdict must name the failing average, got {positional_verdict:?}"
    );

    let _ = std::fs::remove_dir_all(&control_dir);
}

#[test]
fn flags_after_the_positional_are_rejected_end_to_end() {
    // Cycle-3 review F2 (attempt 3fb3e39d): the shim-unit tests pin the
    // rejection mechanism flag-agnostically, but the two documented PoCs
    // (`<file> -o out.txt` writing nothing; `<file> --clear-cache`
    // no-op'ing) deserve end-to-end pins: the binary must exit 2 naming
    // the dropped flag, and a rejected `-o` must leave no artifact.
    let fixture = generated_fixture("detailed-tool-steps.jsonl");
    let out_path = std::env::temp_dir().join(format!(
        "agenttrace-rm247-reviewfix-{}.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&out_path);

    let dropped_o = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            fixture.to_str().expect("fixture path is valid UTF-8"),
            "-o",
            out_path.to_str().expect("out path is valid UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI (-o after positional)");
    assert_eq!(
        dropped_o.status.code(),
        Some(2),
        "`-o` after the positional must be a loud usage error: {:?}",
        dropped_o
    );
    let stderr = String::from_utf8_lossy(&dropped_o.stderr);
    assert!(
        stderr.contains("-o") && stderr.contains("silently dropped"),
        "the rejection must name the dropped `-o` tail, got {stderr:?}"
    );
    assert!(
        !out_path.exists(),
        "a rejected `-o` must not create its output file"
    );

    let dropped_clear_cache = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            fixture.to_str().expect("fixture path is valid UTF-8"),
            "--clear-cache",
        ])
        .output()
        .expect("run agenttrace CLI (--clear-cache after positional)");
    assert_eq!(
        dropped_clear_cache.status.code(),
        Some(2),
        "`--clear-cache` after the positional must be a loud usage error: {:?}",
        dropped_clear_cache
    );
    let stderr = String::from_utf8_lossy(&dropped_clear_cache.stderr);
    assert!(
        stderr.contains("--clear-cache"),
        "the rejection must name `--clear-cache`, got {stderr:?}"
    );

    let _ = std::fs::remove_file(&out_path);
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

    // Integration review (conflict case 144e508e): `--dir` is a value
    // flag like `-d`, so the `--dir <path> <flags>` order must keep
    // parsing down to the -d validation. Before the shim's value-flag
    // table gained `--dir`, the directory was misread as the positional
    // and every following flag was rejected with the wrong error class
    // (`flag ... follows the positional session path`).
    let out_long_first = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--dir", missing.to_str().expect("utf8"), "--sessions"])
        .output()
        .expect("run agenttrace with --dir before other flags");
    let stderr_long = String::from_utf8_lossy(&out_long_first.stderr);
    assert_eq!(
        out_long_first.status.code(),
        Some(2),
        "--dir value order must reach the -d validation: {stderr_long}"
    );
    assert!(
        stderr_long.contains("does not exist"),
        "must fail on the missing directory, not on argument order: {stderr_long}"
    );
    assert!(
        !stderr_long.contains("follows the positional"),
        "the shim must treat --dir as a value flag: {stderr_long}"
    );
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

#[test]
fn governance_reports_honor_gate_flags() {
    // rm-346 arm-a: the --fail-* gate flags are parsed on every report
    // path, but used to be evaluated only in the --overview arm — `--audit
    // --fail-under-health 100` on a corpus the same flags condemn under
    // --overview exited rc0 with 0-byte stderr. The gate must fire after the
    // report as well: report on stdout, failures + evidence on stderr,
    // exit code 2.
    let gated_audit = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--demo",
            "--audit",
            "-f",
            "json",
            "--fail-under-health",
            "100",
        ])
        .output()
        .expect("run gated audit");
    assert_eq!(
        gated_audit.status.code(),
        Some(2),
        "--audit must honor --fail-under-health like --overview does"
    );
    let stderr = String::from_utf8_lossy(&gated_audit.stderr).to_string();
    assert!(
        stderr.contains("Gate failed"),
        "gate failure on stderr: {stderr}"
    );
    assert!(
        !gated_audit.stdout.is_empty(),
        "the audit report itself must still print to stdout"
    );
    assert!(
        !stderr.contains("panicked"),
        "no panic text on stderr: {stderr}"
    );

    // The same honoring applies to the other governance report paths that
    // parse the same flags (--recommend shares the branch).
    let gated_recommend = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--demo",
            "--recommend",
            "-f",
            "json",
            "--fail-under-health",
            "100",
        ])
        .output()
        .expect("run gated recommend");
    assert_eq!(
        gated_recommend.status.code(),
        Some(2),
        "--recommend must honor --fail-under-health like --overview does"
    );

    // Control: without gate flags the governance report stays green.
    let plain_audit = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--audit", "-f", "json"])
        .output()
        .expect("run plain audit");
    assert!(
        plain_audit.status.success(),
        "plain --audit stays rc0: {:?}",
        String::from_utf8_lossy(&plain_audit.stderr)
    );
}

#[test]
fn baseline_delta_pct_flags_reject_nan_and_negative_values() {
    // rm-346 arm-b: --baseline-max-{duration,cost,token}-delta-pct used to
    // reach the comparison unvalidated. NaN made the bound vacuous (rc0,
    // zero stderr, on a corpus the baseline condemns) and -1 false-failed a
    // byte-identical baseline (rc2 "Gate failed"). Both must be rc1 naming
    // the flag, before any comparison runs.
    let work = std::env::temp_dir().join("agenttrace-rm346b-baseline");
    std::fs::create_dir_all(&work).expect("create temp dir");
    let baseline = work.join("baseline.json");
    let generate = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--demo",
            "--overview",
            "-f",
            "json",
            "-o",
            baseline.to_str().expect("baseline path is valid UTF-8"),
        ])
        .output()
        .expect("generate demo baseline");
    assert!(generate.status.success(), "baseline generation failed");

    for (flag, value) in [
        ("--baseline-max-cost-delta-pct", "NaN"),
        ("--baseline-max-token-delta-pct", "NaN"),
        ("--baseline-max-duration-delta-pct", "NaN"),
        ("--baseline-max-cost-delta-pct", "-1"),
        ("--baseline-max-token-delta-pct", "-1"),
        ("--baseline-max-duration-delta-pct", "-1"),
    ] {
        // The joined `--flag=value` form is required for negative values —
        // a bare `-1` is eaten by the argument parser before validation.
        let joined = format!("{flag}={value}");
        let rejected = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .args([
                "--demo",
                "--overview",
                "-f",
                "json",
                "--baseline",
                baseline.to_str().expect("baseline path is valid UTF-8"),
                &joined,
            ])
            .output()
            .expect("run gated overview");
        let stderr = String::from_utf8_lossy(&rejected.stderr).to_string();
        assert_eq!(
            rejected.status.code(),
            Some(1),
            "{flag} {value} must be rejected rc1, stderr: {stderr}"
        );
        assert!(
            stderr.contains(flag),
            "rejection must name the flag {flag}: {stderr}"
        );
    }

    // Control: a byte-identical baseline with valid (default) thresholds
    // stays green — no false Gate failed.
    let identical = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--demo",
            "--overview",
            "-f",
            "json",
            "--baseline",
            baseline.to_str().expect("baseline path is valid UTF-8"),
        ])
        .output()
        .expect("run identical baseline");
    assert!(
        identical.status.success(),
        "identical baseline must stay rc0: {:?}",
        String::from_utf8_lossy(&identical.stderr)
    );

    let _ = std::fs::remove_dir_all(&work);
}

// ---------------------------------------------------------------------------
// rm-503 / rm-505 (cycle 4, run adcef255d604): CLI entry-surface honesty.
// ---------------------------------------------------------------------------

use std::io::Write as _;
use std::process::Stdio;

fn stdin_dash_overview_bytes(fixture: &std::path::Path) -> Vec<u8> {
    let payload = std::fs::read(fixture).expect("fixture must be readable");
    let mut child = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--overview", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn agenttrace for stdin probe");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(&payload)
        .expect("pipe fixture bytes");
    let out = child.wait_with_output().expect("collect stdin run");
    assert!(
        out.status.success(),
        "`--overview -` must succeed on piped session bytes: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

#[test]
fn stdin_dash_overview_is_byte_identical_to_the_same_file() {
    // rm-503 pinning test: `-` must be a pure byte-source swap — the
    // rendered report for a session piped on stdin must equal the report
    // for the same session named as a file, byte for byte.
    let fixture = generated_fixture("detailed-tool-steps.jsonl");
    let via_file = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--overview",
            fixture.to_str().expect("fixture path is valid UTF-8"),
        ])
        .output()
        .expect("run file-mode baseline");
    assert!(
        via_file.status.success(),
        "file-mode baseline must succeed: {}",
        String::from_utf8_lossy(&via_file.stderr)
    );
    let via_stdin = stdin_dash_overview_bytes(&fixture);
    assert_eq!(
        via_file.stdout, via_stdin,
        "`--overview -` output must be byte-identical to the file path"
    );
}

#[test]
fn stdin_dash_empty_input_fails_like_an_empty_file() {
    // rm-503: empty stdin is the empty-session error class (rc1), the
    // same failure an empty named file produces — not a hang, not a
    // zero-session success.
    let empty = std::env::temp_dir().join("agenttrace-stdin-empty-probe.jsonl");
    std::fs::write(&empty, b"").expect("write empty probe file");

    let mut child = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--overview", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn empty-stdin probe");
    drop(child.stdin.take()); // close the pipe: EOF
    let out = child.wait_with_output().expect("collect empty-stdin run");
    assert_eq!(
        Some(1),
        out.status.code(),
        "empty stdin must exit rc1, got {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("empty session"),
        "stderr must name the empty-session class: {stderr}"
    );

    let via_file = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--overview", empty.to_str().expect("tmp path")])
        .output()
        .expect("run empty-file baseline");
    assert_eq!(Some(1), via_file.status.code());
    let _ = std::fs::remove_file(&empty);
}

#[test]
fn keyword_host_commands_have_real_help_routes() {
    // rm-505: `statusline` and `upstream` are documented host keywords,
    // but `<keyword> --help` used to exit 2 with a mislabeled
    // "flag follows the positional session path" error and no help
    // route at all. Both keywords now render per-command help at rc0,
    // with `-h` parity.
    for keyword in ["statusline", "upstream"] {
        for help_flag in ["--help", "-h"] {
            let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
                .args([keyword, help_flag])
                .output()
                .expect("run keyword help probe");
            assert_eq!(
                Some(0),
                out.status.code(),
                "`{keyword} {help_flag}` must exit rc0, got {:?}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            );
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains(&format!("`agenttrace {keyword}`")),
                "keyword help must name the command: {stdout}"
            );
            assert!(
                stdout.contains("flags after it are rejected"),
                "keyword help must state the no-flags-after contract: {stdout}"
            );
        }
    }
}

#[test]
fn keyword_bad_flag_error_names_the_keyword_not_a_session_path() {
    // rm-505: a bad flag after a keyword is still rc2 (the dropped-flag
    // discipline stands), but the error must be keyword-scoped usage —
    // not the "positional session path" mislabel — and must point at
    // the keyword's help route.
    for keyword in ["statusline", "upstream"] {
        let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .args([keyword, "--bogus-flag"])
            .output()
            .expect("run keyword bad-flag probe");
        assert_eq!(
            Some(2),
            out.status.code(),
            "`{keyword} --bogus-flag` must stay rc2, got {:?}",
            out.status
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(&format!("follows the `{keyword}` keyword")),
            "error must scope to the keyword: {stderr}"
        );
        assert!(
            stderr.contains(&format!("agenttrace {keyword} --help")),
            "error must point at the keyword help route: {stderr}"
        );
        assert!(
            !stderr.contains("positional session path"),
            "error must not mislabel the keyword as a session path: {stderr}"
        );
    }
}

#[test]
fn waste_report_format_matrix_pins_every_machine_surface() {
    // rm-544 (run b1ff12f8, cycle 2, minted campaign-locally as
    // rm-451): `--waste -f json` passed the format guard (json is admitted for every action) and then fell through to
    // the text renderer — exit 0 with a prose banner where scripts
    // expected data, an invisible mismatch to the exit code. One rule for
    // the whole matrix: every format the guard admits is honored (text
    // and json both render), every format it rejects fails loudly
    // (markdown names the actions that own it).
    let text = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--waste"])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        text.status.success(),
        "text waste report must exit 0: {:?}",
        String::from_utf8_lossy(&text.stderr)
    );
    let text_stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        text_stdout.contains("Waste Analysis"),
        "text cell renders the banner: {text_stdout}"
    );

    // json cell: the whole of stdout must be one parseable JSON document
    // (no banner before or after it), carrying the waste schema.
    let json = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--waste", "-f", "json"])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        json.status.success(),
        "--waste -f json must exit 0: {:?}",
        String::from_utf8_lossy(&json.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&json.stdout)
        .expect("--waste -f json stdout is strictly JSON, not a text banner");
    assert_eq!(report["schema"], "agenttrace.waste.v1");
    assert!(
        report["waste_score"]
            .as_i64()
            .is_some_and(|score| (0..=100).contains(&score)),
        "waste_score is a clamped 0-100 integer"
    );
    for key in [
        "waste_level",
        "total_wasted_cost",
        "loop_waste_percent",
        "session_cost",
        "cache",
        "tool_bloat",
        "stuck_patterns",
        "summary",
        "top_actions",
    ] {
        assert!(!report[key].is_null(), "waste json carries {key}");
    }
    assert!(
        report["top_actions"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "top_actions is a non-empty action list"
    );

    // markdown cell: the guard rejects it loudly and names the owning
    // actions — the same contract the overview surfaces live under.
    let markdown = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--waste", "-f", "markdown"])
        .output()
        .expect("run agenttrace CLI");
    assert_eq!(
        markdown.status.code(),
        Some(1),
        "--waste -f markdown is rejected, not silently rendered as text"
    );
    let stderr = String::from_utf8_lossy(&markdown.stderr);
    assert!(
        stderr
            .contains("markdown and html formats require --overview or a governance report action"),
        "rejection names the format's owning actions: {stderr}"
    );
}

#[test]
fn cli_alias_paths_report_one_session() {
    // rm-597 (run 250cfd64, cycle 4; minted campaign-locally as rm-511,
    // rebound at integration 2026-10-06): one transcript reachable through
    // a symlink alias must count exactly once end-to-end. Pre-fix this
    // corpus reported "Total Sessions: 2" and "Session files: 2" for a
    // single transcript (assess f376d372 F1 live PoC).
    let dir = std::env::temp_dir().join(format!("agenttrace-cli-rm597-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create corpus dir");
    let real = dir.join("real.jsonl");
    std::fs::copy(generated_fixture("detailed-tool-steps.jsonl"), &real)
        .expect("seed real session");
    std::os::unix::fs::symlink("real.jsonl", dir.join("link.jsonl")).expect("symlink alias");

    let overview = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--overview", "-d", dir.to_str().expect("utf-8 dir")])
        .output()
        .expect("run agenttrace CLI overview");
    assert!(
        overview.status.success(),
        "overview failed: {:?}",
        String::from_utf8_lossy(&overview.stderr)
    );
    let stdout = String::from_utf8_lossy(&overview.stdout);
    let sessions_line = stdout
        .lines()
        .find(|line| line.contains("Total Sessions"))
        .expect("overview prints a Total Sessions line");
    assert!(
        sessions_line.trim_end().ends_with('1'),
        "alias corpus must report exactly one session, got: {sessions_line:?}"
    );

    let doctor = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--doctor", "-d", dir.to_str().expect("utf-8 dir")])
        .output()
        .expect("run agenttrace CLI doctor");
    assert!(
        doctor.status.success(),
        "doctor failed: {:?}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let stdout = String::from_utf8_lossy(&doctor.stdout);
    let files_line = stdout
        .lines()
        .find(|line| line.contains("Session files"))
        .expect("doctor prints a Session files line");
    assert!(
        files_line.trim_end().ends_with('1'),
        "doctor must count the alias once, got: {files_line:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn env_history_knob_sits_below_config_layers_end_to_end() {
    // Review 5b9a9470 F3 / rm-384 level 5: the in-suite precedence
    // matrix pinned the four file/flag layers, but the env level was
    // demonstrated only by live PoCs — a reordered `or_else` in
    // history.rs (env above the config override) would have passed
    // the whole suite silently. These isolated subprocess runs pin
    // BOTH the disclosure source AND the actual derived-history write
    // target through core's fallback chain, with a scrubbed
    // environment (no real HOME, XDG, or project config can leak in).
    use std::fs;
    use std::path::PathBuf;

    let root = std::env::temp_dir().join(format!("at-env-level5-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let home = root.join("home");
    let cfg_home = root.join("cfg");
    let cwd = root.join("cwd");
    let scan = root.join("scan");
    let env_dir = root.join("env-history");
    let config_dir = root.join("config-history");
    for dir in [&home, &cfg_home, &cwd, &scan, &env_dir, &config_dir] {
        fs::create_dir_all(dir).unwrap();
    }
    let fixture = generated_fixture("detailed-tool-steps.jsonl");

    let run = |args: &[&str]| -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
        cmd.env_clear()
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", &cfg_home)
            .env("AGENTTRACE_HISTORY_DIR", &env_dir)
            .current_dir(&cwd)
            .args(args)
            .output()
            .expect("run agenttrace")
    };

    let history_knob = |stderr: &str| -> (String, String) {
        let value: serde_json::Value =
            serde_json::from_str(stderr.trim()).expect("disclosure JSON on stderr");
        let knob = &value["knobs"][0];
        assert_eq!(knob["key"].as_str(), Some("history_dir"));
        (
            knob["value"]
                .as_str()
                .expect("history_dir is set")
                .to_string(),
            knob["source"].as_str().expect("source is set").to_string(),
        )
    };

    // Leg A — no config anywhere, so the env knob IS the effective
    // level 5: the derived-history write must land in it, and the
    // disclosure must name env as the source.
    let preserve = [
        "--sessions",
        "--preserve-history",
        fixture.to_str().unwrap(),
    ];
    let output = run(&preserve);
    assert!(
        output.status.success(),
        "leg A preserve run failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        env_dir.join("history.json").is_file(),
        "env knob must own the derived-history write when no config layer sets history_dir"
    );
    let output = run(&["--doctor", "-f", "json", "-d", scan.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "leg A doctor run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let (value, source) = history_knob(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(source, "env");
    assert_eq!(PathBuf::from(&value), env_dir, "disclosed value");
    // stdout stays a pure JSON document even while the disclosure
    // goes to stderr.
    let stdout: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
            .expect("doctor stdout is one JSON object");
    assert!(stdout.get("config").is_some() || stdout.is_object());

    // Leg B — a user config file outranks the env knob: the write must
    // land in the config dir and NOT in the env dir (the swap a
    // reordered fallback chain would make).
    let user_config = cfg_home.join("agenttrace");
    fs::create_dir_all(&user_config).unwrap();
    fs::write(
        user_config.join("config.toml"),
        format!("history_dir = \"{}\"\n", config_dir.display()),
    )
    .unwrap();
    fs::remove_file(env_dir.join("history.json")).unwrap();
    let output = run(&preserve);
    assert!(
        output.status.success(),
        "leg B preserve run failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        config_dir.join("history.json").is_file(),
        "config layer must own the write when a user config sets history_dir"
    );
    assert!(
        !env_dir.join("history.json").exists(),
        "env knob must NOT win over the user config file (reordered fallback chain)"
    );
    let output = run(&["--doctor", "-f", "json", "-d", scan.to_str().unwrap()]);
    assert!(output.status.success());
    let (value, source) = history_knob(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(source, "config file");
    assert_eq!(PathBuf::from(&value), config_dir, "disclosed value");

    let _ = fs::remove_dir_all(&root);
}

fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read_dir sandbox") {
        let path = entry.expect("sandbox entry").path();
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn statusline_report_leaves_the_statusline_journal_untouched_e2e() {
    // Review fix (rm-573, 2026-10-06): end-to-end pin that the report lane never
    // appends to (or otherwise touches) the real statusline journal, and creates
    // nothing else under the sandboxed cache/history roots.
    let sandbox =
        std::env::temp_dir().join(format!("agenttrace-journal-e2e-{}", std::process::id()));
    let cache = sandbox.join("cache");
    std::fs::create_dir_all(&cache).expect("sandbox cache dir");
    let journal = cache.join("statusline.jsonl");
    std::fs::write(&journal, "ab").expect("plant journal sentinel");

    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--statusline-report")
        .arg("statusline")
        .env("HOME", &sandbox)
        .env("XDG_CACHE_HOME", &cache)
        .env("XDG_CONFIG_HOME", sandbox.join("config"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .env("AGENTTRACE_HISTORY_DIR", sandbox.join("history"))
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run agenttrace");

    assert!(
        out.status.success(),
        "rc={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read(&journal).expect("journal after run"),
        b"ab".to_vec(),
        "journal must be byte-identical after --statusline-report"
    );
    let mut files = Vec::new();
    collect_files(&sandbox, &mut files);
    assert_eq!(
        files,
        vec![journal.clone()],
        "no new files may appear under the sandbox: {files:?}"
    );
    let _ = std::fs::remove_dir_all(&sandbox);
}

/// rm-749: the ungated view lanes early-return before
/// `enforce_report_gates` can run, so a CI step that passed a gate flag
/// to them got a silent green — the flag vanished and the pipeline
/// believed its threshold was enforced. Every ungated view now discloses
/// on stderr which flags it ignored and names the gated actions where
/// they DO apply; rc stays 0 (views cannot fail), and stdout stays
/// machine-stable. The rm-486 byte-stable views (--waste, plain
/// --diagnostics) are exempt by their named carve-out.
#[test]
fn ungated_view_lanes_disclose_ignored_gate_flags_on_stderr() {
    let work = std::env::temp_dir().join(format!(
        "agenttrace-rm749-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&work).expect("create temp dir");
    std::fs::write(
        work.join("session.jsonl"),
        r#"{"type":"user","message":{"role":"user","content":"run the thing"},"sessionId":"s1","timestamp":"2026-05-07T02:00:00Z"}
{"type":"assistant","message":{"role":"assistant","content":"done"},"sessionId":"s1","timestamp":"2026-05-07T02:00:05Z"}
"#,
    )
    .expect("write session");

    // rm-301 style sandbox (see compare_honors_the_fail_under_health_gate):
    // the CLI loads and saves the session cache, so pin HOME/XDG_CACHE_HOME.
    let cache = work.join("cache");
    std::fs::create_dir_all(&cache).expect("create sandbox cache dir");
    let dir = work.to_str().expect("temp dir is valid UTF-8");
    for lane in [
        vec!["--sessions"],
        vec!["--inspect", "1"],
        vec!["--latest"],
        vec!["--search", "run"],
        vec!["--statusline-report"],
        vec!["--budget"],
    ] {
        let mut args = vec!["-d", dir];
        args.extend_from_slice(&lane);
        args.extend_from_slice(&["--fail-under-health", "100"]);
        let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .env("HOME", &work)
            .env("XDG_CACHE_HOME", &cache)
            .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
            .args(&args)
            .output()
            .expect("run agenttrace CLI");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(0),
            "view lanes stay rc0 even with gate flags: {lane:?} -> {stderr}"
        );
        assert!(
            stderr.contains("gates ignored on this action (--fail-under-health 100)"),
            "lane {lane:?} must disclose the ignored flag, stderr: {stderr}"
        );
        assert!(
            stderr.contains("--overview"),
            "disclosure must name where the flags DO apply: {stderr}"
        );
        let plain = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .env("HOME", &work)
            .env("XDG_CACHE_HOME", &cache)
            .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
            .args({
                let mut base = vec!["-d", dir];
                base.extend_from_slice(&lane);
                base
            })
            .output()
            .expect("run agenttrace CLI without gate flags");
        assert_eq!(
            plain.stdout, out.stdout,
            "stdout must be identical with and without gate flags: {lane:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&work);
}

/// rm-749 carve-out: --waste and plain --diagnostics are rm-486
/// byte-stable views. They stay exempt from the disclosure — their
/// stderr (empty) and stdout are byte-identical with and without a
/// gate flag, so anything scripted against them keeps working.
#[test]
fn waste_and_plain_diagnostics_stay_byte_stable_with_gate_flags() {
    let work = std::env::temp_dir().join(format!(
        "agenttrace-rm749-carveout-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&work).expect("create temp dir");
    std::fs::write(
        work.join("session.jsonl"),
        r#"{"type":"user","message":{"role":"user","content":"run the thing"},"sessionId":"s1","timestamp":"2026-05-07T02:00:00Z"}
{"type":"assistant","message":{"role":"assistant","content":"done"},"sessionId":"s1","timestamp":"2026-05-07T02:00:05Z"}
"#,
    )
    .expect("write session");
    let cache = work.join("cache");
    std::fs::create_dir_all(&cache).expect("create sandbox cache dir");
    let dir = work.to_str().expect("temp dir is valid UTF-8");
    for lane in [vec!["--waste"], vec!["--diagnostics"]] {
        let mut base = vec!["-d", dir];
        base.extend_from_slice(&lane);
        let plain = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .env("HOME", &work)
            .env("XDG_CACHE_HOME", &cache)
            .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
            .args(&base)
            .output()
            .expect("run without gate flags");
        let gated = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .env("HOME", &work)
            .env("XDG_CACHE_HOME", &cache)
            .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
            .args({
                let mut with_flag = base.clone();
                with_flag.extend_from_slice(&["--fail-under-health", "100"]);
                with_flag
            })
            .output()
            .expect("run with gate flags");
        assert_eq!(
            plain.status.code(),
            Some(0),
            "carve-out lane must succeed: {lane:?}"
        );
        assert_eq!(plain.stdout, gated.stdout, "stdout byte-stable: {lane:?}");
        assert_eq!(plain.stderr, gated.stderr, "stderr byte-stable: {lane:?}");
        assert!(
            !String::from_utf8_lossy(&gated.stderr).contains("gates ignored"),
            "rm-486 carve-out views must not disclose: {lane:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&work);
}

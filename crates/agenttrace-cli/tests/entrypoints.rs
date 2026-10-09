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
fn project_filter_no_match_exits_loud() {
    // rm-779 review fix F2 (run 14954d7a independent_review): a
    // --project filter that matches nothing must be a loud rc!=0
    // error on stderr — never an empty success — matching the
    // contract documented in `--help` and the governance guide. The
    // matching/keeping semantics are pinned at the unit in
    // agenttrace-core (`project_matches_is_case_insensitive_substring_over_all_identities`).
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--overview",
            "-f",
            "json",
            "--project",
            "zzz-nomatch-pin",
            generated_fixture("detailed-tool-steps.jsonl")
                .to_str()
                .expect("fixture path is valid UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        !output.status.success(),
        "no-match --project must exit non-zero, got: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No sessions match the requested filters"),
        "expected the loud filter error, got: {stderr:?}"
    );
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
fn waste_json_honors_the_report_gates_but_the_text_view_stays_carved_out() {
    // rm-486 premise-refresh rider: rm-544 made `--waste -f json` a
    // versioned machine contract (`agenttrace.waste.v1`) documented for
    // CI, so the row's old "--waste ungated BY DESIGN" carve-out —
    // recorded when waste rendered a human text view — had to be
    // re-adjudicated for the machine form. The json arm evaluates the
    // same shared gate contract as `--compare` (write-then-gate: the
    // artifact is still emitted before the failing exit) across the
    // filtered session view the report is drawn from; the waste metrics
    // themselves are not gate inputs and carry no gate flags; the human
    // text view stays ungated by design.
    let work = std::env::temp_dir().join(format!(
        "agenttrace-waste-gate-{}-{:?}",
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

    // rm-301 style sandbox (see `compare_honors_the_fail_under_health_gate"):
    // the spawned CLI loads and saves the session cache, so without pinning
    // HOME/XDG_CACHE_HOME/AGENTTRACE_SESSION_CACHE_DIR this test would read
    // and REWRITE the operator's real session cache.
    let cache = work.join("cache");
    std::fs::create_dir_all(&cache).expect("create sandbox cache dir");
    let gated = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &work)
        .env("XDG_CACHE_HOME", &cache)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .args([
            "-d",
            work.to_str().expect("temp dir is valid UTF-8"),
            "--waste",
            "-f",
            "json",
            "--fail-under-health",
            "100",
        ])
        .output()
        .expect("run gated waste json");
    assert_eq!(
        gated.status.code(),
        Some(2),
        "--waste -f json must exit 2 when the gate fails, got {:?}",
        gated.status
    );
    let stderr = String::from_utf8_lossy(&gated.stderr);
    assert!(
        stderr.contains("Gate failed"),
        "expected gate failure evidence on stderr, got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&gated.stdout);
    assert!(
        stdout.contains("agenttrace.waste.v1"),
        "write-then-gate: the machine artifact must still be emitted before the failing exit, got: {stdout}"
    );

    // The ungated invocation is unchanged.
    let ungated = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &work)
        .env("XDG_CACHE_HOME", &cache)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .args([
            "-d",
            work.to_str().expect("temp dir is valid UTF-8"),
            "--waste",
            "-f",
            "json",
        ])
        .output()
        .expect("run ungated waste json");
    assert_eq!(
        ungated.status.code(),
        Some(0),
        "without gate flags --waste -f json must still exit 0, got {:?}",
        ungated.status
    );

    // The human text view stays carved out by design.
    let text = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &work)
        .env("XDG_CACHE_HOME", &cache)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .args([
            "-d",
            work.to_str().expect("temp dir is valid UTF-8"),
            "--waste",
            "--fail-under-health",
            "100",
        ])
        .output()
        .expect("run gated waste text view");
    assert_eq!(
        text.status.code(),
        Some(0),
        "the human waste view is ungated by design, got {:?}",
        text.status
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

#[test]
fn output_parent_creation_failure_names_the_dash_o_target() {
    // rm-704: `-o` under an uncreatable parent directory used to die
    // with a contextless raw io::Error ("Permission denied (os error
    // 13)" / "Not a directory") while adjacent -o failures carry the
    // phase and the path; the parent-creation phase must name the -o
    // target too.
    let work = std::env::temp_dir().join(format!(
        "agenttrace-o-parent-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&work).expect("create temp dir");
    // A FILE where a directory would be needed: create_dir_all fails
    // deterministically for any user (no root-only / probe).
    let blocker = work.join("blocker");
    std::fs::write(&blocker, "not a directory").expect("write blocker file");
    let target = blocker.join("rep.json");
    let target_arg = target.to_str().expect("target path is valid UTF-8");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--overview", "-f", "json", "-o", target_arg])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        !output.status.success(),
        "an uncreatable -o parent must fail the run"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("creating parent directory"),
        "phase missing from error: {stderr}"
    );
    assert!(
        stderr.contains(target_arg),
        "-o target missing from error: {stderr}"
    );
    assert!(
        stderr.contains("-o"),
        "flag attribution missing from error: {stderr}"
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
    // rm-382 residual (assess F2, 2026-10-05): the fixed pid-less path
    // made identical-tree sibling suite runs race this fixture on a
    // shared host (1-in-6 full-suite failures observed); qualify it
    // with the pid like the other temp-path tests.
    let work =
        std::env::temp_dir().join(format!("agenttrace-rm346b-baseline-{}", std::process::id()));
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
    // with `-h` parity. `mcp` joined the keyword set in rm-455.
    for keyword in ["statusline", "upstream", "mcp"] {
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
    for keyword in ["statusline", "upstream", "mcp"] {
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

    // markdown and svg cells: the guard rejects them loudly and names
    // the owning actions — the same contract the overview surfaces live
    // under. svg is pinned here beside markdown because it rides the
    // same outside-composable-set rejection on the waste lane, with its
    // own overview-only message (rm-576 review F1, integration review
    // 2026-10-09: the card is an --overview renderer; svg must bail on
    // every other lane instead of falling through to a silent text
    // render).
    for (format, message) in [
        (
            "markdown",
            "markdown and html formats require --overview or a governance report action",
        ),
        ("svg", "svg format requires --overview"),
    ] {
        let rejected = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .args(["--demo", "--waste", "-f", format])
            .output()
            .expect("run agenttrace CLI");
        assert_eq!(
            rejected.status.code(),
            Some(1),
            "--waste -f {format} is rejected, not silently rendered as text"
        );
        let stderr = String::from_utf8_lossy(&rejected.stderr);
        assert!(
            stderr.contains(message),
            "rejection names the format's owning actions: {stderr}"
        );
    }

    // rm-576 review F1 pin: a governance action with -f svg must bail
    // loudly too — before the fix `--demo --audit -f svg` exited 0 as
    // plain text and (with -o) wrote text into a .svg file.
    let governance_svg = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--audit", "-f", "svg"])
        .output()
        .expect("run agenttrace CLI");
    assert_eq!(
        governance_svg.status.code(),
        Some(1),
        "--audit -f svg is rejected, never a silent text render"
    );
    let stderr = String::from_utf8_lossy(&governance_svg.stderr);
    assert!(
        stderr.contains("svg format requires --overview"),
        "svg rejection names its owning lane: {stderr}"
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

#[test]
fn statusline_report_wins_over_the_mcp_keyword_e2e() {
    // Integration fix (rm-455 merge, 2026-10-07): the rm-573 dispatch-order
    // invariant covers every keyword host command, `mcp` included. Unguarded,
    // `--statusline-report mcp` starts the stdio server instead of rendering
    // the report — with stdin at EOF that is a silent rc0 with EMPTY stdout
    // (the report the user asked for never renders), and on a terminal it
    // looks hung. Guarded, the report lane runs and answers its own shape.
    let sandbox =
        std::env::temp_dir().join(format!("agenttrace-mcp-precedence-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox).expect("sandbox dir");

    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--statusline-report", "-f", "json", "mcp"])
        .env("HOME", &sandbox)
        .env("XDG_CACHE_HOME", sandbox.join("cache"))
        .env("XDG_CONFIG_HOME", sandbox.join("config"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", sandbox.join("cache"))
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: serde_json::Value = match serde_json::from_str(stdout.trim()) {
        Ok(value) => value,
        Err(error) => panic!(
            "--statusline-report must render the report, not the mcp server \
             (stdout was {:?}: {error})",
            stdout
        ),
    };
    assert!(
        report.get("journal").is_some(),
        "expected the statusline report document, got: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&sandbox);
}

#[cfg(unix)]
#[test]
fn output_to_dev_null_writes_no_file_and_exits_zero() {
    // rm-489: `-o /dev/null` used to fail with "writing report output
    // file" because the rm-250 temp-sibling staging cannot rename onto a
    // character device. A terminal sink is written through the open
    // handle instead: exit 0, Saved: banner, and no temp litter.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--overview", "-o", "/dev/null"])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "-o /dev/null must succeed: {:?}",
        output
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Saved: /dev/null"),
        "the Saved banner still names the sink: {stderr}"
    );
    // -o is a tee, not a redirect (documented contract): stdout still
    // carries the report while the sink receives its own copy.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "-o tees: stdout keeps carrying the report: {:?}",
        stdout
    );
}

#[cfg(unix)]
#[test]
fn output_to_dev_stdout_lands_the_report_on_stdout() {
    // rm-489: /dev/stdout is the same special-target class; the report
    // must arrive on the captured stdout pipe itself.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--overview", "-o", "/dev/stdout"])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "-o /dev/stdout must succeed: {:?}",
        output
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "the report rides the stdout device itself"
    );
}

#[cfg(unix)]
#[test]
fn output_to_fifo_is_refused_and_never_replaces_the_pipe() {
    // rm-489 (re-based at integration, conflict case
    // 8801a55fef4945a94942a29831feb4): the assess PoC watched
    // `-o <fifo>` silently REPLACE the named pipe with a regular file
    // (temp-sibling + rename). The landed write_output contract —
    // resolve_output_target + write_output_resolved from salvage run
    // e43bb8f3, a superset of this run's char-device/fifo write-through
    // arm — REFUSES fifo targets with a disclosed reason instead of
    // writing through (a readerless fifo would block the writer
    // forever, and a writer owning the node would replace it):
    // non-zero exit, the refusal names the node type on stderr, and
    // the pipe survives as a fifo — never materialized as a file.
    use std::fs;
    use std::os::unix::fs::FileTypeExt;

    let dir = std::env::temp_dir().join(format!("agenttrace-fifo-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create scratch dir");
    let fifo = dir.join("report.fifo");
    let _ = fs::remove_file(&fifo);
    let mkfifo = Command::new("mkfifo")
        .arg(&fifo)
        .output()
        .expect("run mkfifo");
    assert!(mkfifo.status.success(), "mkfifo failed: {mkfifo:?}");

    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--overview", "-o"])
        .arg(&fifo)
        .output()
        .expect("run agenttrace CLI");
    assert!(
        !output.status.success(),
        "-o <fifo> must be refused loudly, got: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("refusing to write"),
        "the refusal must name itself: {stderr}"
    );

    let metadata = fs::metadata(&fifo).expect("fifo still exists");
    assert!(
        metadata.file_type().is_fifo(),
        "the named pipe must never be replaced by a regular file"
    );
    let _ = fs::remove_file(&fifo);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn error_stderr_carries_the_anyhow_cause_chain() {
    // rm-610: the top-level handler printed `Error: {err}`, and
    // anyhow's Display shows only the outermost context layer — so
    // `--overview -o /dev/full` (ENOSPC) and `-o <existing-dir>`
    // (EISDIR) exited with byte-identical stderr through the SAME
    // "writing report output file" context site and the io error kind
    // was discarded. The handler must surface the full cause chain;
    // exit codes stay pinned at 1.
    let dir = std::env::temp_dir().join(format!("at-rm610-dir-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create -o directory target");
    let enospc = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--overview", "-o", "/dev/full"])
        .output()
        .expect("run agenttrace against /dev/full");
    let eisdir = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--overview", "-o", dir.to_str().expect("utf8")])
        .output()
        .expect("run agenttrace against a directory -o target");
    let _ = std::fs::remove_dir(&dir);

    let enospc_err = String::from_utf8_lossy(&enospc.stderr);
    let eisdir_err = String::from_utf8_lossy(&eisdir.stderr);
    for (label, code, stderr) in [
        ("enospc", enospc.status.code(), &enospc_err),
        ("eisdir", eisdir.status.code(), &eisdir_err),
    ] {
        assert_eq!(code, Some(1), "{label}: exit code unchanged: {stderr}");
        assert!(
            stderr.contains("Error: writing report output file"),
            "{label}: context site still leads the chain: {stderr}"
        );
    }
    assert!(
        enospc_err.contains("No space left on device")
            || enospc_err.contains("Permission denied")
            || enospc_err.contains("ENOSPC"),
        "the underlying write cause must surface through the context site \
         (on hosts where the atomic writer's temp sibling cannot be created \
         next to /dev/full, the honest cause is EACCES): {enospc_err}"
    );
    assert!(
        eisdir_err.contains("Is a directory"),
        "EISDIR cause must be visible through the context site: {eisdir_err}"
    );
    assert_ne!(
        enospc_err, eisdir_err,
        "a full disk and path misuse must be distinguishable in stderr"
    );
}

// rm-610, second leg: the cause-chain join must not change how
// single-layer (uncontextualized) errors render — the missing -d
// message keeps its plain form with no dangling separator glue.
// (The /dev/full and -o <directory> legs live in
// error_stderr_carries_the_anyhow_cause_chain above, written by the
// provider-dead prior attempt of this run and adopted after
// verification.)
#[test]
fn error_output_includes_the_underlying_cause_chain() {
    let missing = std::env::temp_dir().join(format!("at-rm610-missing-{}", std::process::id()));
    let plain = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--sessions", "-d", missing.to_str().expect("utf8")])
        .output()
        .expect("run agenttrace with missing -d");
    let stderr_plain = String::from_utf8_lossy(&plain.stderr);
    assert_eq!(plain.status.code(), Some(2), "bad request class");
    assert!(
        stderr_plain.contains("does not exist") && !stderr_plain.contains("writing report"),
        "unlayered errors keep their plain rendering: {stderr_plain}"
    );
    assert!(
        !stderr_plain.trim_end().ends_with(":"),
        "no dangling chain separator on a single-layer error: {stderr_plain}"
    );
}

#[test]
fn unreadable_agent_db_is_disclosed_not_claimed_absent() {
    // rm-753 (run ff0068ca, minted campaign-locally as rm-596,
    // rebound at integration 2026-10-07; assess P12/P14): a discovered
    // opencode
    // database of random bytes used to render as "No session files
    // found" with --doctor printing `parsed=0 failed=0` and
    // recommending --demo. The failure must now be disclosed on every
    // report path, and must survive the cache (no poisoned empty
    // snapshot from the failed load).
    let home = std::env::temp_dir().join(format!("at-rm753-home-rnd-{}", std::process::id()));
    let db = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(db.parent().expect("parent")).expect("create db dir");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../agenttrace-core/tests/fixtures/random-bytes/opencode.db");
    std::fs::copy(&fixture, &db).expect("plant hostile db");

    let run = || {
        Command::new(env!("CARGO_BIN_EXE_agenttrace"))
            .arg("--overview")
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("XDG_CACHE_HOME", home.join(".cache"))
            .env("XDG_DATA_HOME", home.join(".local/share"))
            .env("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"))
            .output()
            .expect("run agenttrace over the hostile home")
    };
    for run_index in 1..=2 {
        let out = run();
        assert_eq!(
            out.status.code(),
            Some(1),
            "run {run_index}: no sessions anywhere stays rc1"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("No session files found"),
            "run {run_index}: the empty-corpus message stays: {stderr}"
        );
        assert!(
            stderr.contains("found and could not be read"),
            "run {run_index}: the unreadable database is disclosed: {stderr}"
        );
        assert!(
            stderr.contains("opencode.db"),
            "run {run_index}: the disclosure names the file: {stderr}"
        );
    }

    let doctor = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--doctor")
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"))
        .output()
        .expect("run agenttrace --doctor over the hostile home");
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&doctor.stdout),
        String::from_utf8_lossy(&doctor.stderr)
    );
    assert!(
        rendered.contains("OpenCode (DB)"),
        "the database row appears in doctor output: {rendered}"
    );
    assert!(
        rendered.contains("failed=1"),
        "unreadable counts as a failure in the doctor counters: {rendered}"
    );
    assert!(
        rendered.contains("unreadable:"),
        "the doctor explains the failure: {rendered}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn dropped_sqlite_rows_are_disclosed_on_report_paths() {
    // rm-753 (run ff0068ca, minted campaign-locally as rm-596,
    // rebound at integration 2026-10-07; assess P11): five hostile
    // session rows of
    // which the NULL-id ghost used to vanish silently — the report
    // claimed "4 sessions" with no hint that a fifth was lost.
    let home = std::env::temp_dir().join(format!("at-rm753-home-oc-{}", std::process::id()));
    let db = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(db.parent().expect("parent")).expect("create db dir");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../agenttrace-core/tests/fixtures/opencode-hostile/opencode.db");
    std::fs::copy(&fixture, &db).expect("plant hostile db");

    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--overview")
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"))
        .output()
        .expect("run agenttrace over the hostile opencode db");
    assert_eq!(
        out.status.code(),
        Some(0),
        "the four decodable sessions are a usable corpus"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stdout.trim().is_empty(),
        "the report still renders the four sessions"
    );
    assert!(
        stderr.contains("1 session row(s) dropped as undecodable"),
        "the dropped row is disclosed: {stderr}"
    );
    assert!(
        stderr.contains("opencode.db"),
        "the disclosure names the file: {stderr}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn output_flag_creates_missing_parent_directories_recursively() {
    // rm-784 (run 24ec00eb, repository-maintenance cycle 1): `-o` keeps its
    // documented mkdir -p semantics — a deep missing path is created
    // recursively and the report lands at the leaf. Pinned so a future
    // "fix" cannot quietly drop the behavior fleet users rely on.
    let sandbox = std::env::temp_dir().join(format!(
        "at-rm784-mkdir-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let target = sandbox.join("reports/deep/nested/report.txt");
    let _ = std::fs::remove_dir_all(&sandbox);

    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--overview", "-o"])
        .arg(&target)
        .output()
        .expect("run agenttrace CLI (-o deep path)");

    assert_eq!(
        out.status.code(),
        Some(0),
        "mkdir -p must create the parent chain: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(target.is_file(), "report must exist at the requested leaf");
    let body = std::fs::read_to_string(&target).expect("report readable");
    assert!(!body.trim().is_empty(), "report has content");

    let _ = std::fs::remove_dir_all(&sandbox);
}

#[test]
fn output_flag_refusal_names_the_directory_and_target() {
    // rm-784: when a parent cannot be created (here: it exists as a regular
    // FILE, the NotADirectory shape), the error must name the directory it
    // tried to create and the requested -o target — not a bare
    // "File exists (os error 17)" with no path attached (rm-610 keeps the
    // full cause chain on stderr).
    let sandbox = std::env::temp_dir().join(format!(
        "at-rm784-refuse-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&sandbox).expect("create sandbox");
    let blocker = sandbox.join("blocker.txt");
    std::fs::write(&blocker, b"regular file").expect("seed blocking file");
    let target = blocker.join("x.txt");

    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--demo", "--overview", "-o"])
        .arg(&target)
        .output()
        .expect("run agenttrace CLI (-o under a file parent)");

    assert_eq!(out.status.code(), Some(1), "the refusal is an error exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("creating parent directory"),
        "error must name the mkdir failure: {stderr}"
    );
    assert!(
        stderr.contains(blocker.to_str().expect("path is UTF-8")),
        "error must name the directory that could not be created: {stderr}"
    );
    assert!(
        stderr.contains(target.to_str().expect("path is UTF-8")),
        "error must name the requested -o target: {stderr}"
    );
    assert!(
        stderr.contains("File exists"),
        "the OS cause stays disclosed (rm-610): {stderr}"
    );

    let _ = std::fs::remove_dir_all(&sandbox);
}

#[test]
fn waste_json_discloses_synthetic_loop_cost_basis() {
    // rm-857 e2e: a session whose only loop dollars are constant-derived
    // (synthetic basis — the parser's default when no priced retry cost is
    // recorded) must disclose that basis in the machine contract, the same
    // contract `--diagnostics` already carries via LoopCost.cost_basis.
    let work = std::env::temp_dir().join(format!(
        "agenttrace-waste-basis-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&work).expect("create temp dir");
    // Identical tool_use repeated -> a detected loop -> synthetic loop dollars.
    std::fs::write(
        work.join("loopy.jsonl"),
        r#"{"type":"user","message":{"role":"user","content":"run it"},"sessionId":"s1","timestamp":"2026-05-07T02:00:00Z"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tu1","name":"bash","input":{"command":"ls"}}]},"sessionId":"s1","timestamp":"2026-05-07T02:00:05Z"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tu2","name":"bash","input":{"command":"ls"}}]},"sessionId":"s1","timestamp":"2026-05-07T02:00:10Z"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tu3","name":"bash","input":{"command":"ls"}}]},"sessionId":"s1","timestamp":"2026-05-07T02:00:15Z"}
"#,
    )
    .expect("write loopy session");
    let cache = work.join("cache");
    std::fs::create_dir_all(&cache).expect("create sandbox cache dir");
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &work)
        .env("XDG_CACHE_HOME", &cache)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .args([
            "-d",
            work.to_str().expect("temp dir is valid UTF-8"),
            "--waste",
            "-f",
            "json",
        ])
        .output()
        .expect("run waste json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("agenttrace.waste.v1"),
        "waste.v1 schema banner expected, got: {stdout}"
    );
    assert!(
        stdout.contains("\"loop_cost_basis\": \"synthetic\""),
        "rm-857: the machine contract must carry the basis key, got: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&work);
}

#[test]
fn json_lanes_disclose_cache_coherence_fields() {
    // rm-504: -f json must disclose cache coherence in the data_health
    // object: `cache_schema_version` (the session-cache schema the report
    // was produced against — read-only const accessor, no schema bump) and
    // `parsed_with` (the agenttrace binary version). Machine consumers
    // need both to detect schema/version skew between reports.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args([
            "--overview",
            "--no-baseline-gate",
            "-f",
            "json",
            generated_fixture("detailed-tool-steps.jsonl")
                .to_str()
                .expect("fixture path is valid UTF-8"),
        ])
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "overview json must succeed: {:?}",
        output
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("overview lane emits one json object");
    let health = value
        .get("data_health")
        .expect("overview json carries a data_health object");
    let schema_version = health
        .get("cache_schema_version")
        .and_then(|v| v.as_u64())
        .expect("rm-504: data_health.cache_schema_version must be present and numeric");
    assert!(
        schema_version > 0,
        "cache_schema_version must be a real schema number, got {schema_version}"
    );
    let parsed_with = health
        .get("parsed_with")
        .and_then(|v| v.as_str())
        .expect("rm-504: data_health.parsed_with must be present and a string");
    assert_eq!(
        parsed_with,
        concat!("agenttrace v", env!("CARGO_PKG_VERSION")),
        "parsed_with must match the pinned --version identity exactly (skew detection)"
    );
}

#[test]
fn version_flag_prints_binary_identity() {
    // rm-088: `agenttrace --version` prints `agenttrace {CARGO_PKG_VERSION}`
    // to stdout and exits 0, without loading any session data.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--version")
        .output()
        .expect("run agenttrace CLI");
    assert_eq!(
        output.status.code(),
        Some(0),
        "--version must exit 0: {:?}",
        output
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        concat!("agenttrace v", env!("CARGO_PKG_VERSION")),
        "--version must print exactly the pinned binary identity (fleet 'v' format)"
    );
}

#[test]
fn completions_flag_prints_snippets_to_stdout() {
    // rm-088: `agenttrace --completions` prints the shell completion
    // snippets (checked-in under scripts/completions/) to stdout and
    // exits 0 — the default lane stays untouched when the flag is
    // absent. The emitted text must be non-empty and must name the
    // binary so downstream installers can verify what they captured.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--completions")
        .output()
        .expect("run agenttrace CLI");
    assert_eq!(
        output.status.code(),
        Some(0),
        "--completions must exit 0: {:?}",
        output
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("agenttrace"),
        "completions output must name the binary, got: {stdout}"
    );
    assert!(
        stdout.len() > 200,
        "completions output must carry real snippets, got {} bytes",
        stdout.len()
    );
}

#[test]
fn completions_word_list_covers_every_documented_flag() {
    // rm-088 no-drift pin: the snippets under scripts/completions/ are
    // static, so nothing stops them from falling behind --help when a
    // flag is added. Pin both directions mechanically: (a) every long
    // flag clap defines on a --help DEFINITION line (a line whose first
    // non-space character is `-` — backticked prose mentions never are)
    // must appear in the bash word list, which is generated from --help;
    // (b) every `--flag` token the completions emit must still be a real
    // flag, so removed flags cannot linger in the snippets either.
    let help = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--help")
        .output()
        .expect("run agenttrace --help");
    assert!(help.status.success(), "--help must exit 0");
    let help_text = String::from_utf8_lossy(&help.stdout);
    let defined: std::collections::BTreeSet<String> = help_text
        .lines()
        .map(|line| line.trim_start())
        .filter(|line| line.starts_with('-'))
        .flat_map(|line| line.split_whitespace())
        .filter_map(|token| token.strip_prefix("--"))
        .filter(|long| {
            !long.is_empty()
                && long
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .map(str::to_string)
        .collect();
    assert!(
        !defined.is_empty(),
        "--help must define long flags (found none — parser drift?)"
    );

    let completions = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("--completions")
        .output()
        .expect("run agenttrace --completions");
    assert!(completions.status.success(), "--completions must exit 0");
    let out = String::from_utf8_lossy(&completions.stdout).to_string();

    // (a) the bash word list (the `local opts="…"` line) carries every
    // documented long flag — it is the mechanically generated list.
    let opts_line = out
        .lines()
        .find(|line| line.trim_start().starts_with("local opts=\""))
        .expect("bash snippet must carry its `local opts=\"…\"` word list");
    let missing: Vec<&str> = defined
        .iter()
        .map(String::as_str)
        .filter(|long| !opts_line.contains(&format!("--{long}")))
        .collect();
    assert!(
        missing.is_empty(),
        "rm-088 drift: scripts/completions/agenttrace.bash word list is \
         missing flags --help defines: {missing:?} — regenerate the word \
         list from `agenttrace --help`"
    );

    // (b) no stale flags: every `--flag` token anywhere in the emitted
    // snippets must still exist in --help.
    let stale: Vec<String> = scan_long_flags(&out)
        .into_iter()
        .filter(|long| !defined.contains(long))
        .collect();
    assert!(
        stale.is_empty(),
        "rm-088 drift: completions emit flags --help no longer defines: \
         {stale:?} — regenerate the snippets"
    );
}

/// Collect maximal `--[a-z0-9-]+` tokens from `text` (the completions
/// emit flags as space-separated words, `--flag:VALUE` zsh pairs, and
/// comment mentions, so scan characters instead of splitting).
fn scan_long_flags(text: &str) -> std::collections::BTreeSet<String> {
    let mut flags = std::collections::BTreeSet::new();
    let mut current = String::new();
    let mut in_flag = false;
    for ch in text.chars() {
        if in_flag {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
                current.push(ch);
                continue;
            }
            if !current.is_empty() {
                flags.insert(current.clone());
            }
            current.clear();
            in_flag = false;
        } else if ch == '-' {
            // A candidate starts at the second consecutive dash.
            if !current.is_empty() && current == "-" {
                in_flag = true;
                current.clear();
            } else {
                current.clear();
                current.push(ch);
            }
        } else {
            current.clear();
        }
    }
    if in_flag && !current.is_empty() {
        flags.insert(current);
    }
    flags
}

use agenttrace_core::{
    add_baseline_comparison, average_health, compute_overview, context_trends, cost_audit,
    data_health, data_health_scoped, delivery_evidence_with_git, demo_sessions,
    evaluate_overview_gate, filter_sessions, fix_suggestions, inspect_first, list_pricing,
    load_sessions_with_options, lookup_price, mcp_governance, parse_file, predict_cost_anomaly,
    pricing_cache_path, pricing_source, recommendations, render_doctor_report,
    render_model_pricing_list, render_test_match, render_waste_report_with_language,
    report_compare_json, report_json_with_language, report_overview_html_with_context,
    report_overview_json_with_context, report_overview_markdown_with_context,
    report_overview_text_with_context, report_search_json, report_search_text,
    report_text_with_language, sanitize_line_segment, search_sessions, session_capability,
    tool_fail_rate, total_tokens, update_pricing, BaselineThresholds, LoadOptions, LoadReport,
    ReportLanguage, Session, TimeRange, VERSION,
};
use anyhow::{bail, Context};
use chrono::Utc;
use clap::Parser;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

mod upstream;

#[derive(Debug, Parser)]
#[command(name = "agenttrace")]
#[command(about = "TUI observability for AI coding agent sessions")]
struct Args {
    path: Option<String>,
    #[arg(
        short = 'f',
        long = "format",
        default_value = "text",
        value_parser = ["text", "json", "markdown", "md", "html"]
    )]
    format: String,
    /// Session directory to scan instead of auto-discovered agent homes
    #[arg(short = 'd', long = "dir")]
    dir: Option<String>,
    #[arg(long)]
    compare: bool,
    #[arg(long)]
    audit: bool,
    #[arg(long = "recommend")]
    recommend: bool,
    #[arg(long = "mcp-governance")]
    mcp_governance: bool,
    #[arg(long = "context-trends")]
    context_trends: bool,
    #[arg(long = "delivery-evidence")]
    delivery_evidence: bool,
    #[arg(long)]
    overview: bool,
    #[arg(long)]
    sessions: bool,
    #[arg(long)]
    diagnostics: bool,
    #[arg(long)]
    inspect: Option<usize>,
    #[arg(short = 'm', default_value = "default")]
    model: String,
    #[arg(short = 'o')]
    output: Option<PathBuf>,
    #[arg(long)]
    latest: bool,
    #[arg(long)]
    waste: bool,
    #[arg(long = "list-models")]
    list_models: bool,
    #[arg(long = "update-pricing")]
    update_pricing: bool,
    #[arg(long = "test-match")]
    test_match: bool,
    /// Report on the Claude Code statusline capture journal (candidate
    /// 53, cycle 7): limit-pressure windows, reset crossings, and
    /// per-session prompt-cache miss causes recorded by
    /// `agenttrace statusline`.
    #[arg(long = "statusline-report")]
    statusline_report: bool,
    /// `agenttrace upstream`: refresh the remote-tracking refs from the
    /// network via `git fetch` (and probe the npm registry) before
    /// reporting fork-vs-upstream drift. Without it, `agenttrace
    /// upstream` is fully offline (rm-024).
    #[arg(long)]
    fetch: bool,
    #[arg(long)]
    version: bool,
    #[arg(long)]
    demo: bool,
    #[arg(long)]
    doctor: bool,
    #[arg(long)]
    search: Option<String>,
    #[arg(long = "search-limit", default_value_t = 20)]
    search_limit: usize,
    #[arg(long = "fail-under-health", default_value_t = 0)]
    fail_under_health: i32,
    #[arg(long = "fail-on-critical")]
    fail_on_critical: bool,
    #[arg(long = "max-tool-fail-rate")]
    max_tool_fail_rate: Option<f64>,
    #[arg(long)]
    baseline: Option<String>,
    #[arg(long = "baseline-max-duration-delta-pct", default_value_t = 0.0)]
    baseline_max_duration_delta_pct: f64,
    #[arg(long = "baseline-max-cost-delta-pct", default_value_t = 0.0)]
    baseline_max_cost_delta_pct: f64,
    #[arg(long = "baseline-max-token-delta-pct", default_value_t = 0.0)]
    baseline_max_token_delta_pct: f64,
    /// Opt out of the baseline regression gate: keep the comparison in the
    /// report but do not fail the run (exit 2) on a threshold breach
    /// (pass-7 P7-3).
    #[arg(long = "no-baseline-gate")]
    no_baseline_gate: bool,
    #[arg(long = "lang", default_value = "en", value_name = "en|zh")]
    lang: String,
    #[arg(long, default_value = "all")]
    range: String,
    #[arg(long, default_value = "")]
    project: String,
    #[arg(long, default_value = "")]
    source: String,
    #[arg(long = "model-filter", default_value = "")]
    model_filter: String,
    #[arg(long, default_value = "")]
    query: String,
    #[arg(long, default_value = "")]
    health: String,
    #[arg(long, default_value = "")]
    cost: String,
    #[arg(long, default_value = "")]
    anomaly: String,
    #[arg(long, default_value = "recent")]
    sort: String,
    #[arg(long, default_value = "desc")]
    order: String,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    /// Explicitly bound governance reports to the newest N sessions.
    /// Governance reports audit every matching session by default;
    /// sampling is always disclosed via audited_sessions/total_sessions
    /// (pass-8 F8-1).
    #[arg(long)]
    sample: Option<usize>,
    #[arg(long = "clear-cache")]
    clear_cache: bool,
    #[arg(long = "preserve-history")]
    preserve_history: bool,
    #[arg(long = "include-history")]
    include_history: bool,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    // rm-247: flags after the positional session path used to be
    // silently dropped by the Go-flag shim; reject them loudly
    // instead (exit 2, the clap usage-error convention).
    let argv = go_flag_compatible_args(std::env::args_os()).unwrap_or_else(|error| {
        eprintln!("Error: {error}");
        std::process::exit(2);
    });
    let args = Args::parse_from(argv);
    // --version must win over argument validation, including action
    // validation: `--lang fr --version` used to fail in report_language()
    // and `--overview --version` in validate_primary_action() before the
    // early return ran (pass-6 P6-2).
    if args.version {
        write_stdout(&format!("agenttrace v{}\n", VERSION))?;
        return Ok(());
    }
    // `agenttrace statusline` is a Claude Code statusLine host command
    // (roadmap candidate 53, cycle 7), not a report action: it must run
    // before action validation and never fail the host — the status
    // line would otherwise hang or error on every prompt.
    if args.path.as_deref() == Some("statusline") {
        return agenttrace_core::run_statusline_host();
    }
    // `agenttrace upstream` is a repository status host command (rm-024,
    // cycle 1): fork-vs-upstream drift made visible. Like the statusline
    // host command it dispatches before action validation; unlike it,
    // this is an explicit user action that may fail loudly. It is fully
    // offline unless --fetch explicitly opts into the network.
    if args.path.as_deref() == Some("upstream") {
        let report = upstream::status_report(&args.format, args.fetch)?;
        write_stdout(&report)?;
        return Ok(());
    }
    validate_primary_action(&args)?;
    validate_gate_thresholds(&args)?;
    if args.sample == Some(0) {
        bail!("--sample must be at least 1");
    }
    if matches!(args.format.as_str(), "markdown" | "md" | "html")
        && !(args.overview
            || args.audit
            || args.recommend
            || args.mcp_governance
            || args.context_trends
            || args.delivery_evidence)
    {
        bail!("markdown and html formats require --overview or a governance report action");
    }
    validate_range_applicability(&args)?;

    let language = report_language(&args.lang)?;

    // rm-301: stdout purity under machine formats. The side-effect
    // announcements below are human progress chatter; with `-f json` the
    // stdout stream must stay a single parseable JSON document end to
    // end (a downstream `jq` breaks on any leading line), so they route
    // to stderr. The human path keeps them on stdout exactly as before.
    let announce: fn(&str) -> anyhow::Result<()> = if args.format == "json" {
        write_stderr
    } else {
        write_stdout
    };

    if args.clear_cache {
        agenttrace_core::clear_session_cache()?;
        announce("Session cache cleared.\n")?;
        if !has_session_action(&args) {
            return Ok(());
        }
    }

    if args.update_pricing {
        announce("Downloading pricing from LiteLLM...\n")?;
        let count = update_pricing()?;
        announce(&format!("Loaded {count} model prices\n"))?;
        announce(&format!(
            "Cache saved: {}\n",
            pricing_cache_path().display()
        ))?;
        if !has_post_pricing_action(&args) {
            return Ok(());
        }
    }

    if args.test_match {
        // rm-341: the early exits must honor -o and -f json like every
        // other report action instead of ignoring both.
        let out = if args.format == "json" {
            render_test_match_json()?
        } else {
            render_test_match()
        };
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }

    if args.doctor {
        let doctor_dir = args.dir.as_deref().map(PathBuf::from);
        let out = render_doctor_report(doctor_dir.as_deref(), args.demo, &args.format)?;
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }

    if args.list_models {
        // rm-341: honor -o and -f json (previously wrote text to stdout
        // regardless and never created the file).
        let out = if args.format == "json" {
            render_model_pricing_list_json()?
        } else {
            render_model_pricing_list()
        };
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }

    if args.statusline_report {
        let out = agenttrace_core::render_statusline_report(&args.format)?;
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }

    if !has_session_action(&args) {
        if args.demo {
            let sessions = demo_sessions()?;
            // rm-341: --demo honors --lang like the discovery TUI path
            // instead of silently dropping it.
            return agenttrace_tui::run_with_sessions_with_language(
                sessions,
                "demo",
                Some(&args.lang),
            );
        }
        return agenttrace_tui::run_with_language(
            args.dir.as_deref().unwrap_or(""),
            Some(&args.lang),
        );
    }
    if args.baseline.is_some() && !args.overview {
        bail!("--baseline requires --overview -f json");
    }

    if args.audit
        || args.recommend
        || args.mcp_governance
        || args.context_trends
        || args.delivery_evidence
    {
        // Governance reports audit every matching session by default;
        // bounded sampling is explicit (--sample) and always disclosed.
        // `--limit` is a display cap for list views only and no longer
        // filters aggregate data (pass-8 F8-1).
        if args.limit != 20 {
            eprintln!(
                "Note: --limit no longer bounds governance reports; it caps list views. Use --sample N for bounded, disclosed sampling."
            );
        }
        let matched = prepare_cli_view(load_sessions(&args)?, &args)?;
        if matched.is_empty() {
            bail!("No sessions match the requested filters");
        }
        let total_sessions = matched.len();
        let (sessions, excluded_reason) = match args.sample {
            Some(sample) if sample < total_sessions => (
                matched.into_iter().take(sample).collect::<Vec<_>>(),
                Some(format!(
                    "sampled first {sample} of {total_sessions} sessions in the --sort {} --order {} view (--sample {sample})", args.sort, args.order
                )),
            ),
            _ => (matched, None),
        };
        let value = if args.audit {
            serde_json::to_value(cost_audit(&sessions))?
        } else if args.recommend {
            serde_json::json!({
                "recommendations": serde_json::to_value(recommendations(&sessions))?,
            })
        } else if args.mcp_governance {
            serde_json::to_value(mcp_governance(&sessions))?
        } else if args.context_trends {
            serde_json::to_value(context_trends(&sessions))?
        } else {
            serde_json::to_value(delivery_evidence_with_git(&sessions))?
        };
        let value = attach_audit_coverage(value, sessions.len(), total_sessions, excluded_reason);
        let out = render_governance_report(&value, &args.format)?;
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        // rm-346 arm-a: the --fail-* gate flags are parsed on every report
        // path but used to be evaluated only under --overview — a gated
        // `--audit` (or sibling governance report) exited rc0 with 0-byte
        // stderr on a corpus the same flags condemn. Honor them here too:
        // the report still prints, the failures and evidence go to stderr,
        // and the process exits 2.
        let overview = compute_overview(&sessions);
        let failures = evaluate_overview_gate(
            &overview,
            &sessions,
            args.fail_under_health,
            args.fail_on_critical,
            args.max_tool_fail_rate,
        );
        if !failures.is_empty() {
            for failure in failures {
                eprintln!("Gate failed: {failure}");
            }
            eprintln!("Local evidence:");
            eprintln!("- avg health: {:.1}", average_health(&sessions));
            eprintln!("- critical sessions: {}", overview.critical);
            eprintln!("- tool fail rate: {:.1}%", tool_fail_rate(&sessions));
            if let Some(session) = sessions.iter().min_by(|left, right| {
                left.health
                    .cmp(&right.health)
                    .then_with(|| left.path.cmp(&right.path))
                    .then_with(|| left.name.cmp(&right.name))
            }) {
                eprintln!("- lowest-health session: {}", session.path);
            }
            let inspect = if args.demo {
                format!(
                    "agenttrace --demo {} -f json",
                    governance_inspect_flag(&args)
                )
            } else if let Some(dir) = args.dir.as_deref() {
                format!(
                    "agenttrace -d {:?} {} -f json",
                    dir,
                    governance_inspect_flag(&args)
                )
            } else {
                format!("agenttrace {} -f json", governance_inspect_flag(&args))
            };
            eprintln!("- inspect: `{inspect}`");
            std::process::exit(2);
        }
        return Ok(());
    }

    if args.compare {
        // Same coverage contract as the governance branch: --compare
        // audits every matching session unless --sample bounds it, and
        // the bound is disclosed (pass-8 F8-1 sibling at main.rs:249).
        if args.limit != 20 {
            eprintln!(
                "Note: --limit no longer bounds governance reports; it caps list views. Use --sample N for bounded, disclosed sampling."
            );
        }
        let matched = prepare_cli_view(load_sessions(&args)?, &args)?;
        if matched.is_empty() {
            bail!("No sessions match the requested filters");
        }
        let total_sessions = matched.len();
        let (sessions, excluded_reason) = match args.sample {
            Some(sample) if sample < total_sessions => (
                matched.into_iter().take(sample).collect::<Vec<_>>(),
                Some(format!(
                    "sampled first {sample} of {total_sessions} sessions in the --sort {} --order {} view (--sample {sample})", args.sort, args.order
                )),
            ),
            _ => (matched, None),
        };
        let out = if args.format == "json" {
            let compared =
                serde_json::from_str::<serde_json::Value>(&report_compare_json(&sessions))
                    .context("compare report serializes")?;
            let value = attach_audit_coverage(
                serde_json::json!({ "sessions": compared }),
                sessions.len(),
                total_sessions,
                excluded_reason,
            );
            serde_json::to_string_pretty(&value)?
        } else {
            let mut out = String::new();
            out.push_str(&format!(
                "(auditing {} of {} sessions)\n",
                sessions.len(),
                total_sessions
            ));
            out.push_str(&agenttrace_core::report_compare_with_language(
                &sessions,
                &args.model,
                language,
            ));
            out
        };
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        return Ok(());
    }

    if args.waste {
        let sessions = prepare_cli_view(load_sessions(&args)?, &args)?;
        let session =
            latest_session(&sessions).context("No sessions match the requested filters")?;
        let out = render_waste_report_with_language(session, language);
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        return Ok(());
    }

    if single_session_report_requested(&args) {
        let sessions = prepare_cli_view(load_sessions(&args)?, &args)?;
        let session =
            latest_session(&sessions).context("No sessions match the requested filters")?;
        let out = match args.format.as_str() {
            "json" => report_json_with_language(session, language),
            _ => report_text_with_language(session, language),
        };
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        return Ok(());
    }

    let (sessions, load_report) = load_sessions_report(&args)?;
    let sessions = prepare_cli_view(sessions, &args)?;
    if sessions.is_empty() {
        bail!("No sessions match the requested filters");
    }

    if args.sessions || args.diagnostics || args.inspect.is_some() {
        if args.sessions {
            let out = render_session_list(&sessions, &args.format, args.limit);
            write_output(&args.output, &(out.clone() + "\n"))?;
            write_stdout(&out)?;
            return Ok(());
        }
        let session = if let Some(rank) = args.inspect {
            if rank == 0 {
                bail!("--inspect rank starts at 1");
            }
            let item = inspect_first(&sessions)
                .get(rank - 1)
                .cloned()
                .context("inspect rank exceeds available priority sessions")?;
            &sessions[item.index]
        } else if args.latest {
            latest_session(&sessions).expect("sessions checked non-empty")
        } else {
            sessions.first().expect("sessions checked non-empty")
        };
        let out = render_diagnostics(session, &sessions, &args.format, language)?;
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        return Ok(());
    }

    if let Some(query) = args.search.as_deref() {
        let results = search_sessions(&sessions, query, args.search_limit);
        let out = if args.format == "json" {
            report_search_json(&results)
        } else {
            report_search_text(&results, query)
        };
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        return Ok(());
    }

    if args.overview {
        let overview = compute_overview(&sessions);
        // `discovered` comes from the loader (range-independent) and
        // parse failures stay separate from sessions excluded by the
        // range/filters, so "Parse coverage N/M" is true for every
        // range (pass-8 F8-2).
        let health = match load_report.as_ref() {
            Some(report) => data_health_scoped(
                &sessions,
                report.discovered,
                report.skipped,
                report.cache_hits,
            ),
            None => data_health(&sessions, sessions.len(), 0),
        };
        if args.limit < sessions.len() {
            eprintln!(
                "Note: --limit caps list views only; this overview's aggregates cover all {} sessions.",
                sessions.len()
            );
        }
        let range = parse_range(&args)?;
        let mut out = match args.format.as_str() {
            "json" => report_overview_json_with_context(
                &overview,
                &sessions,
                Some(&health),
                range,
                args.include_history,
                // Pinned epoch keeps --demo JSON byte-deterministic (CI
                // determinism check); real runs stamp the wall clock.
                args.demo.then_some(agenttrace_core::DEMO_REPORT_EPOCH),
                args.limit,
            ),
            "markdown" | "md" => report_overview_markdown_with_context(
                &overview,
                &sessions,
                &health,
                range,
                args.include_history,
            ),
            "html" => report_overview_html_with_context(
                &overview,
                &sessions,
                &health,
                range,
                args.include_history,
            ),
            _ => report_overview_text_with_context(
                &overview,
                &sessions,
                &health,
                range,
                args.include_history,
            ),
        };
        let mut baseline_breaches = None;
        if let Some(baseline) = args.baseline.as_deref() {
            if args.format != "json" {
                bail!("--baseline requires --overview -f json");
            }
            let (compared, breaches) = add_baseline_comparison(
                &out,
                baseline,
                BaselineThresholds {
                    max_duration_delta_pct: args.baseline_max_duration_delta_pct,
                    max_cost_delta_pct: args.baseline_max_cost_delta_pct,
                    max_token_delta_pct: args.baseline_max_token_delta_pct,
                },
            )?;
            out = compared;
            baseline_breaches = Some(breaches);
        }
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        let failures = evaluate_overview_gate(
            &overview,
            &sessions,
            args.fail_under_health,
            args.fail_on_critical,
            args.max_tool_fail_rate,
        );
        if !failures.is_empty() {
            for failure in failures {
                eprintln!("Gate failed: {failure}");
            }
            eprintln!("Local evidence:");
            eprintln!("- avg health: {:.1}", average_health(&sessions));
            eprintln!("- critical sessions: {}", overview.critical);
            eprintln!("- tool fail rate: {:.1}%", tool_fail_rate(&sessions));
            if let Some(session) = sessions.iter().min_by(|left, right| {
                left.health
                    .cmp(&right.health)
                    .then_with(|| left.path.cmp(&right.path))
                    .then_with(|| left.name.cmp(&right.name))
            }) {
                eprintln!("- lowest-health session: {}", session.path);
            }
            let inspect = if args.demo {
                "agenttrace --demo --overview -f json".to_string()
            } else if let Some(dir) = args.dir.as_deref() {
                format!("agenttrace -d {:?} --overview -f json", dir)
            } else {
                "agenttrace --overview -f json".to_string()
            };
            eprintln!("- inspect: `{inspect}`");
            std::process::exit(2);
        }
        // Pass-7 P7-3: a baseline threshold breach used to leave the
        // booleans buried in the JSON while the process exited 0 — the
        // opposite of what a gate promises. Breach now exits 2 (mirroring
        // the health gate) unless --no-baseline-gate opts out.
        if !args.no_baseline_gate {
            if let Some(breaches) = baseline_breaches.filter(|b| b.any()) {
                eprintln!("Gate failed: baseline regression above threshold");
                eprintln!("Local evidence:");
                if breaches.slower_than_baseline {
                    eprintln!("- duration delta above --baseline-max-duration-delta-pct");
                }
                if breaches.cost_above_threshold {
                    eprintln!("- cost delta above --baseline-max-cost-delta-pct");
                }
                if breaches.tokens_above_threshold {
                    eprintln!("- token delta above --baseline-max-token-delta-pct");
                }
                eprintln!("- inspect: `baseline_comparison` in the report JSON");
                eprintln!("- opt out: --no-baseline-gate");
                std::process::exit(2);
            }
        }
        return Ok(());
    }

    bail!("no report action selected")
}

fn write_stderr(value: &str) -> anyhow::Result<()> {
    match io::stderr().write_all(value.as_bytes()) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(Into::into),
    }
}

fn write_stdout(value: &str) -> anyhow::Result<()> {
    match io::stdout().write_all(value.as_bytes()) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(Into::into),
    }
}

fn render_governance_report(value: &serde_json::Value, format: &str) -> anyhow::Result<String> {
    let json = serde_json::to_string_pretty(value)?;
    // Every governance report discloses its coverage: "(auditing N of M
    // sessions)" in the human-readable formats, audited_sessions/
    // total_sessions/excluded_reason in the JSON (pass-8 F8-1).
    let disclosure = audit_coverage_phrase(value);
    Ok(match format {
        "json" => json,
        "markdown" | "md" => {
            if disclosure.is_empty() {
                format!("```json\n{json}\n```")
            } else {
                format!("{disclosure}```json\n{json}\n```")
            }
        }
        "html" => {
            if disclosure.is_empty() {
                format!("<pre>{}</pre>", escape_html(&json))
            } else {
                format!(
                    "<p>{}</p><pre>{}</pre>",
                    escape_html(disclosure.trim_end()),
                    escape_html(&json)
                )
            }
        }
        _ => {
            if disclosure.is_empty() {
                render_plain_value(value, 0)
            } else {
                let mut out = disclosure;
                out.push_str(&render_plain_value(value, 0));
                out
            }
        }
    })
}

fn audit_coverage_phrase(value: &serde_json::Value) -> String {
    let audited = value.get("audited_sessions").and_then(|item| item.as_u64());
    let total = value.get("total_sessions").and_then(|item| item.as_u64());
    match (audited, total) {
        (Some(audited), Some(total)) => {
            let mut phrase = format!("(auditing {audited} of {total} sessions)");
            if let Some(reason) = value.get("excluded_reason").and_then(|item| item.as_str()) {
                phrase.push_str(&format!("; {reason}"));
            }
            phrase.push('\n');
            phrase
        }
        _ => String::new(),
    }
}

fn attach_audit_coverage(
    mut value: serde_json::Value,
    audited_sessions: usize,
    total_sessions: usize,
    excluded_reason: Option<String>,
) -> serde_json::Value {
    let coverage = serde_json::json!({
        "audited_sessions": audited_sessions,
        "total_sessions": total_sessions,
        "excluded_reason": excluded_reason,
    });
    match &mut value {
        serde_json::Value::Object(map) => {
            if let serde_json::Value::Object(fields) = coverage {
                for (key, field) in fields {
                    map.insert(key, field);
                }
            }
            value
        }
        _ => {
            let mut wrapped = serde_json::json!({ "report": value });
            if let serde_json::Value::Object(map) = &mut wrapped {
                if let serde_json::Value::Object(fields) = coverage {
                    for (key, field) in fields {
                        map.insert(key, field);
                    }
                }
            }
            wrapped
        }
    }
}

fn render_plain_value(value: &serde_json::Value, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match value {
        serde_json::Value::Object(items) => items
            .iter()
            .map(|(key, value)| match value {
                serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                    format!("{indent}{key}:\n{}", render_plain_value(value, depth + 1))
                }
                _ => format!("{indent}{key}: {}", render_plain_value(value, 0)),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|value| {
                format!(
                    "{indent}- {}",
                    render_plain_value(value, depth + 1).trim_start()
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        // rm-383 review-fix: governance-family reports render
        // journal-derived strings (e.g. mcp_server_name inferred from a
        // crafted tool name) through this leaf; control bytes must not
        // reach the terminal (same contract as the TSV/search renderers).
        serde_json::Value::String(value) => sanitize_line_segment(value),
        _ => value.to_string(),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn go_flag_compatible_args<I>(args: I) -> anyhow::Result<Vec<OsString>>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let mut out = Vec::new();
    if let Some(program) = args.next() {
        out.push(program);
    }

    let mut expecting_value = false;
    while let Some(arg) = args.next() {
        if expecting_value {
            out.push(arg);
            expecting_value = false;
            continue;
        }
        if arg == "--" {
            out.push(arg);
            if let Some(path) = args.next() {
                out.push(path);
            }
            break;
        }
        if is_go_flag_positional(&arg) {
            out.push(arg);
            break;
        }
        expecting_value = flag_takes_value(&arg);
        out.push(arg);
    }

    // rm-247: this shim stops at the first positional, so anything left
    // on the command line is dropped without clap ever seeing it.
    // Dropping flags silently made documented invocations lie
    // (`sessions.jsonl -o out.txt` exited 0 without writing anything,
    // `sessions.jsonl --clear-cache` left the cache untouched), so name
    // the dropped arguments and fail loudly. Plain extra positionals
    // stay tolerated: they are not flags, and the Go-style contract
    // only ever documented flag handling.
    let dropped: Vec<OsString> = args.collect();
    if let Some(flag) = dropped.iter().find(|arg| !is_go_flag_positional(arg)) {
        let tail = dropped
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        bail!(
            "flag `{}` follows the positional session path and would be silently dropped \
             (dropped: `{tail}`); place flags before the positional path",
            flag.to_string_lossy()
        );
    }

    Ok(out)
}

fn is_go_flag_positional(arg: &OsString) -> bool {
    let text = arg.to_string_lossy();
    text == "-" || !text.starts_with('-')
}

fn flag_takes_value(arg: &OsString) -> bool {
    let text = arg.to_string_lossy();
    if text.contains('=') {
        return false;
    }
    matches!(
        text.as_ref(),
        "-f" | "--format"
            | "-d"
            | "--dir"
            | "-m"
            | "-o"
            | "--search"
            | "--search-limit"
            | "--fail-under-health"
            | "--max-tool-fail-rate"
            | "--baseline"
            | "--baseline-max-duration-delta-pct"
            | "--baseline-max-cost-delta-pct"
            | "--baseline-max-token-delta-pct"
            | "--lang"
            | "--range"
            | "--project"
            | "--source"
            | "--model-filter"
            | "--query"
            | "--health"
            | "--cost"
            | "--anomaly"
            | "--sort"
            | "--order"
            | "--limit"
            | "--sample"
            | "--inspect"
    )
}

fn latest_session(sessions: &[Session]) -> Option<&Session> {
    sessions.iter().max_by(|a, b| newer_session_order(a, b))
}

fn newer_session_order(a: &Session, b: &Session) -> std::cmp::Ordering {
    let a_has_session_time = !a.metrics.session_start.is_empty();
    let b_has_session_time = !b.metrics.session_start.is_empty();
    a_has_session_time
        .cmp(&b_has_session_time)
        .then_with(|| {
            if a_has_session_time && b_has_session_time {
                a.metrics.session_start.cmp(&b.metrics.session_start)
            } else {
                session_mod_time(a).cmp(&session_mod_time(b))
            }
        })
        .then_with(|| a.path.cmp(&b.path))
}

fn session_mod_time(session: &Session) -> SystemTime {
    fs::metadata(&session.path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn report_language(value: &str) -> anyhow::Result<ReportLanguage> {
    match value.to_ascii_lowercase().as_str() {
        "en" | "english" => Ok(ReportLanguage::En),
        "zh" | "zh-cn" | "zh_cn" | "chinese" => Ok(ReportLanguage::Zh),
        other => bail!("unsupported --lang value '{other}'; supported languages: en, zh"),
    }
}

fn load_sessions(args: &Args) -> anyhow::Result<Vec<Session>> {
    load_sessions_report(args).map(|(sessions, _)| sessions)
}

fn load_sessions_report(args: &Args) -> anyhow::Result<(Vec<Session>, Option<LoadReport>)> {
    if args.demo {
        return Ok((prepare_explicit_sessions(demo_sessions()?, args)?, None));
    }
    if let Some(path) = args.path.as_deref() {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok((
                prepare_explicit_sessions(vec![parse_file(&path)?], args)?,
                None,
            ));
        }
        if path.is_dir() {
            if is_cline_task_dir(&path) {
                return Ok((
                    prepare_explicit_sessions(vec![parse_file(&path)?], args)?,
                    None,
                ));
            }
            bail!(
                "Error loading {}: positional path must be a session file",
                path.display()
            );
        }
        bail!("session path does not exist: {}", path.display());
    }
    let dir = args.dir.as_deref().map(PathBuf::from);
    // Cycle-4 B2: a typo'd `-d` path used to produce the same
    // "No session files found in …" error as a genuinely empty
    // directory — the two need different messages and different exit
    // codes (2 = the request itself is wrong; 1 = nothing matched).
    if let Some(dir) = args.dir.as_deref() {
        let path = Path::new(dir);
        if !path.exists() {
            eprintln!("agenttrace: session directory does not exist: {dir}");
            eprintln!("- inspect: the -d/--dir value; drop -d to auto-discover agent homes");
            std::process::exit(2);
        }
        if !path.is_dir() {
            eprintln!("agenttrace: -d/--dir is not a directory: {dir}");
            std::process::exit(2);
        }
    }
    let range = parse_range(args)?;
    let report = load_sessions_with_options(
        dir.as_deref(),
        &LoadOptions {
            since: range.since(Utc::now()),
            project: args.project.clone(),
            source: args.source.clone(),
            model: args.model_filter.clone(),
            include_history: args.include_history,
            preserve_history: args.preserve_history,
        },
    );
    let sessions = report.sessions.clone();
    if sessions.is_empty() {
        if report.discovered == 0 {
            match args.dir.as_deref() {
                Some(dir) => bail!(
                    "No session files found in {dir} (directory exists but holds no session files)"
                ),
                None => bail!("No session files found in any auto-discovered agent home"),
            }
        }
        bail!("No sessions match the requested filters");
    }
    Ok((sessions, Some(report)))
}

fn parse_range(args: &Args) -> anyhow::Result<TimeRange> {
    TimeRange::parse(&args.range).context("range must be today, 7d, 30d, or all")
}

fn filter_cli_sessions(sessions: Vec<Session>, args: &Args) -> anyhow::Result<Vec<Session>> {
    Ok(filter_sessions(
        &sessions,
        parse_range(args)?,
        &args.project,
        &args.source,
        &args.model_filter,
        Utc::now(),
    ))
}

fn prepare_explicit_sessions(
    mut sessions: Vec<Session>,
    args: &Args,
) -> anyhow::Result<Vec<Session>> {
    if args.preserve_history {
        agenttrace_core::preserve_derived_history(&sessions)?;
    }
    if args.include_history {
        agenttrace_core::merge_preserved_history(&mut sessions);
    }
    filter_cli_sessions(sessions, args)
}

fn is_cline_task_dir(path: &std::path::Path) -> bool {
    path.join("api_conversation_history.json").is_file()
        || path.join("ui_messages.json").is_file()
        || path.join("task_metadata.json").is_file()
}

fn write_output(path: &Option<PathBuf>, content: &str) -> anyhow::Result<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // rm-250: stage through a unique temp sibling and rename into
        // place, so a crash or Ctrl-C mid-write never leaves a
        // truncated report at the destination (same pattern as the
        // session-cache and history persistence writers).
        let temp = unique_temp_sibling(path);
        let staged = fs::write(&temp, content).and_then(|()| fs::rename(&temp, path));
        if let Err(error) = staged {
            let _ = fs::remove_file(&temp);
            return Err(error).context("writing report output file");
        }
        eprintln!("Saved: {}", path.display());
    }
    Ok(())
}

fn render_test_match_json() -> anyhow::Result<String> {
    // rm-341: -f json support for the --test-match early exit; the same
    // probe models as render_test_match, one row per model.
    let models = [
        "claude-sonnet-4-5-20250929",
        "anthropic/claude-sonnet-4-6",
        "vertex_ai/claude-opus-4-5@20251101",
        "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "openai/gpt-4.1",
        "gpt-4.1-mini-2025-04-14",
        "deepseek-chat",
        "deepseek/deepseek-v3.2",
        "gemini-2.5-pro",
        "unknown-model-xyz",
    ];
    let rows: Vec<_> = models
        .iter()
        .map(|model| {
            let price = lookup_price(model);
            serde_json::json!({
                "model": model,
                "input_per_million": price.input,
                "output_per_million": price.output,
                "cache_write_per_million": price.cw,
                "cache_read_per_million": price.cr,
            })
        })
        .collect();
    let doc = serde_json::json!({
        "source": pricing_source(),
        "models": rows,
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

fn render_model_pricing_list_json() -> anyhow::Result<String> {
    // rm-341: -f json support for the --list-models early exit.
    let doc = serde_json::json!({
        "source": pricing_source(),
        "models": list_pricing(),
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

/// Unique per-process, per-call temp sibling of `path` for atomic `-o`
/// writes (rm-250). Mirrors `session_cache::unique_temp_path` (pass-6
/// P6-3); that helper is `pub(crate)`, and the core crate is owned by a
/// sibling lane, so the pattern is duplicated here instead of widened.
fn unique_temp_sibling(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("agenttrace-report");
    path.with_file_name(format!("{name}.tmp.{}.{}", std::process::id(), sequence))
}

fn prepare_cli_view(mut sessions: Vec<Session>, args: &Args) -> anyhow::Result<Vec<Session>> {
    validate_view_filters(args)?;
    sessions.retain(|session| {
        matches_text(session, &args.query)
            && matches_health(session.health, &args.health)
            && matches_number(session.metrics.cost_estimated, &args.cost)
            && (args.anomaly.is_empty()
                || session.anomalies.iter().any(|item| {
                    // rm-016 + review F1: anomaly kind/detail are user-facing
                    // text — fold Unicode-aware on both sides ("any" stays an
                    // ASCII keyword comparison).
                    args.anomaly.eq_ignore_ascii_case("any")
                        || item
                            .kind
                            .to_lowercase()
                            .contains(&args.anomaly.to_lowercase())
                        || item
                            .detail
                            .to_lowercase()
                            .contains(&args.anomaly.to_lowercase())
                }))
    });
    let descending = match args.order.as_str() {
        "asc" => false,
        "desc" => true,
        _ => bail!("--order must be asc or desc"),
    };
    sessions.sort_by(|left, right| {
        let ordering = match args.sort.as_str() {
            "recent" | "time" => left.metrics.session_start.cmp(&right.metrics.session_start),
            "health" => left.health.cmp(&right.health),
            "cost" => left
                .metrics
                .cost_estimated
                .total_cmp(&right.metrics.cost_estimated),
            "turns" => left
                .metrics
                .assistant_turns
                .cmp(&right.metrics.assistant_turns),
            "failures" => left
                .metrics
                .tool_calls_fail
                .cmp(&right.metrics.tool_calls_fail),
            "source" => left.metrics.source_tool.cmp(&right.metrics.source_tool),
            "name" => left.name.cmp(&right.name),
            "anomalies" => left.anomalies.len().cmp(&right.anomalies.len()),
            _ => std::cmp::Ordering::Equal,
        };
        let ordering = if descending {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| left.path.cmp(&right.path))
    });
    if !matches!(
        args.sort.as_str(),
        "recent"
            | "time"
            | "health"
            | "cost"
            | "turns"
            | "failures"
            | "source"
            | "name"
            | "anomalies"
    ) {
        bail!("unsupported --sort value: {}", args.sort);
    }
    Ok(sessions)
}

fn matches_text(session: &Session, query: &str) -> bool {
    // rm-016 + review F1: user text, so Unicode-aware folding (PR-5) —
    // `--query 'CAFÉ'` used to miss a session named 'café …' because both
    // sides were ASCII-folded while --search had already been fixed.
    let query = query.trim().to_lowercase();
    query.is_empty()
        || [
            session.name.as_str(),
            session.path.as_str(),
            session.cwd.as_str(),
            session.metrics.source_tool.as_str(),
            session.metrics.model_used.as_str(),
        ]
        .iter()
        .any(|value| value.to_lowercase().contains(&query))
        || session
            .metrics
            .tool_usage
            .keys()
            .chain(session.metrics.file_usage.keys())
            .any(|value| value.to_lowercase().contains(&query))
        || session.anomalies.iter().any(|item| {
            item.kind.to_lowercase().contains(&query) || item.detail.to_lowercase().contains(&query)
        })
}

fn matches_health(health: i32, filter: &str) -> bool {
    match filter.trim().to_ascii_lowercase().as_str() {
        "" => true,
        "good" | "healthy" => health >= 80,
        "warn" | "warning" => (50..80).contains(&health),
        "crit" | "critical" => health < 50,
        filter => matches_number(health as f64, filter),
    }
}

fn validate_view_filters(args: &Args) -> anyhow::Result<()> {
    if !args.health.is_empty()
        && !matches!(
            args.health.to_ascii_lowercase().as_str(),
            "good" | "healthy" | "warn" | "warning" | "crit" | "critical"
        )
        && !valid_number_filter(&args.health)
    {
        bail!("invalid --health filter: {}", args.health);
    }
    if !args.cost.is_empty() && !valid_number_filter(&args.cost) {
        bail!("invalid --cost filter: {}", args.cost);
    }
    Ok(())
}

fn valid_number_filter(filter: &str) -> bool {
    [">=", "<=", ">", "<", "="]
        .iter()
        .find_map(|prefix| filter.strip_prefix(prefix))
        .is_some_and(|value| value.parse::<f64>().is_ok())
}

fn matches_number(value: f64, filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }
    for (prefix, compare) in [
        (
            ">=",
            std::cmp::Ordering::is_ge as fn(std::cmp::Ordering) -> bool,
        ),
        ("<=", std::cmp::Ordering::is_le),
        (">", std::cmp::Ordering::is_gt),
        ("<", std::cmp::Ordering::is_lt),
        ("=", std::cmp::Ordering::is_eq),
    ] {
        if let Some(raw) = filter.strip_prefix(prefix) {
            return raw
                .parse::<f64>()
                .ok()
                .is_some_and(|target| compare(value.total_cmp(&target)));
        }
    }
    false
}

fn render_session_list(sessions: &[Session], format: &str, limit: usize) -> String {
    let sessions = sessions.iter().take(limit).collect::<Vec<_>>();
    if format == "json" {
        return serde_json::to_string_pretty(&sessions).expect("sessions serialize");
    }
    let mut lines =
        vec!["SESSION\tHEALTH\tDATA\tSOURCE\tMODEL\tCOST\tTOKENS\tFAIL\tANOMALIES".to_string()];
    lines.extend(sessions.into_iter().map(|session| {
        // rm-383: name, source tool, and model are
        // transcript-derived; sanitize them for terminal display and TSV
        // row integrity (assess PoC: a crafted session name carried a raw
        // OSC-52 clipboard-write byte-for-byte into the --sessions TSV).
        // JSON output escapes control bytes losslessly and stays untouched.
        format!(
            "{}\t{}\t{}\t{}\t{}\t{:.4}\t{}\t{}\t{}",
            sanitize_line_segment(&session.name),
            session.health,
            session_capability(session),
            sanitize_line_segment(&session.metrics.source_tool),
            sanitize_line_segment(&session.metrics.model_used),
            session.metrics.cost_estimated,
            total_tokens(session),
            session.metrics.tool_calls_fail,
            session.anomalies.len()
        )
    }));
    lines.join("\n")
}

fn render_diagnostics(
    session: &Session,
    history: &[Session],
    format: &str,
    language: ReportLanguage,
) -> anyhow::Result<String> {
    let alert = predict_cost_anomaly(history, session);
    let fixes = fix_suggestions(session);
    if format == "json" {
        return Ok(serde_json::to_string_pretty(&serde_json::json!({
            "session": session,
            "cost_alert": alert,
            "fix_suggestions": fixes,
        }))?);
    }
    let mut out = report_text_with_language(session, language);
    out.push_str("\nDiagnostics\n-----------\n");
    out.push_str(&serde_json::to_string_pretty(&session.diagnostics)?);
    if alert.triggered {
        out.push_str(&format!(
            "\nCost alert [{}]: {}",
            alert.level, alert.message
        ));
    }
    for fix in fixes {
        out.push_str(&format!("\nFix [{}]: {}", fix.severity, fix.action));
    }
    Ok(out)
}

fn has_post_pricing_action(args: &Args) -> bool {
    args.path.is_some()
        || args.list_models
        || args.test_match
        || args.doctor
        || args.latest
        || args.compare
        || args.audit
        || args.recommend
        || args.mcp_governance
        || args.context_trends
        || args.delivery_evidence
        || args.overview
        || args.sessions
        || args.diagnostics
        || args.inspect.is_some()
        || args.waste
        || args
            .search
            .as_deref()
            .map(|query| !query.trim().is_empty())
            .unwrap_or(false)
}

fn has_session_action(args: &Args) -> bool {
    args.path.is_some()
        || args.latest
        || args.compare
        || args.audit
        || args.recommend
        || args.mcp_governance
        || args.context_trends
        || args.delivery_evidence
        || args.overview
        || args.sessions
        || args.diagnostics
        || args.inspect.is_some()
        || args.waste
        || args.baseline.is_some()
        || args
            .search
            .as_deref()
            .map(|query| !query.trim().is_empty())
            .unwrap_or(false)
}

/// rm-244: `--range` only filters session reports. Every action that
/// consumes it is covered by `has_session_action`; on the interactive
/// and utility paths (`--demo` TUI, plain TUI, `--clear-cache`,
/// `--doctor`, `--list-models`, ...) the flag used to be accepted and
/// silently ignored, which looks exactly like a working filter. Reject
/// the combination loudly, in the same style as the format
/// applicability guard in `main`.
fn validate_range_applicability(args: &Args) -> anyhow::Result<()> {
    if args.range != "all" && !has_session_action(args) {
        bail!(
            "--range requires a session report action (for example --overview, --sessions, --diagnostics, --waste, --search QUERY, --compare, or --audit); the interactive and utility views ignore it"
        );
    }
    Ok(())
}

/// rm-246 (assess N1): the single-session report is the DEFAULT
/// action. It renders only when the invocation selects a session
/// without asking for any report action, because an explicit action
/// must win: routing a positional path (or `--latest`) into
/// `--overview`/`--search` lets those branches — and the `--fail-*`
/// gates — actually run, instead of silently rendering a
/// single-session report that skips the gate entirely
/// (`--overview --fail-under-health 100 <file>` used to exit 0).
/// Actions dispatched earlier in `run()` (--statusline, --compare,
/// --audit, --waste, …) never reach this decision point.
fn single_session_report_requested(args: &Args) -> bool {
    (args.latest || args.path.is_some())
        && !args.sessions
        && !args.diagnostics
        && args.inspect.is_none()
        && args.search.is_none()
        && !args.overview
}

fn validate_primary_action(args: &Args) -> anyhow::Result<()> {
    // --fetch only means something for the upstream command; staying
    // silent about it elsewhere would let a typo'd invocation report
    // stale data while looking refreshed.
    if args.fetch && args.path.as_deref() != Some("upstream") {
        bail!("--fetch applies only to the upstream command: agenttrace --fetch upstream");
    }
    // rm-341: --baseline gates --overview -f json; --compare never reads
    // it. Rejecting the pair up front replaces two errors that used to
    // contradict each other ("--baseline requires --overview -f json"
    // followed by "choose exactly one report action" once --overview was
    // added).
    if args.baseline.is_some() && args.compare {
        bail!(
            "--baseline cannot be combined with --compare: --baseline gates --overview -f json only"
        );
    }
    let actions = [
        args.compare,
        args.audit,
        args.recommend,
        args.mcp_governance,
        args.context_trends,
        args.delivery_evidence,
        args.overview,
        args.sessions,
        args.diagnostics || args.inspect.is_some(),
        args.waste,
        args.doctor,
        args.list_models,
        args.test_match,
        args.statusline_report,
        args.version,
        args.search
            .as_deref()
            .is_some_and(|query| !query.trim().is_empty()),
    ];
    if actions.into_iter().filter(|active| *active).count() > 1 {
        bail!("choose exactly one report action");
    }
    if args.latest
        && actions.into_iter().any(|active| active)
        && !args.diagnostics
        && args.inspect.is_none()
    {
        bail!("--latest can only be combined with --diagnostics or --inspect");
    }
    Ok(())
}

/// The report flag that selected the governance branch, for gate-evidence
/// `inspect` hints that reproduce the gated run (rm-346 arm-a).
fn governance_inspect_flag(args: &Args) -> &'static str {
    if args.audit {
        "--audit"
    } else if args.recommend {
        "--recommend"
    } else if args.mcp_governance {
        "--mcp-governance"
    } else if args.context_trends {
        "--context-trends"
    } else {
        "--delivery-evidence"
    }
}

fn validate_gate_thresholds(args: &Args) -> anyhow::Result<()> {
    if !(0..=100).contains(&args.fail_under_health) {
        bail!("--fail-under-health must be between 0 and 100");
    }
    if args
        .max_tool_fail_rate
        .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
    {
        bail!("--max-tool-fail-rate must be a finite number between 0 and 100");
    }
    if args.search.is_some() && args.search_limit == 0 {
        bail!("--search-limit must be at least 1");
    }
    // rm-346 arm-b: the --baseline-max-*-delta-pct thresholds used to reach
    // the comparison unvalidated — NaN made the bound vacuous (rc0, zero
    // stderr, on a corpus the baseline condemns) and a negative value
    // false-failed a byte-identical baseline. Reject both before any
    // comparison runs.
    for (flag, value) in [
        (
            "--baseline-max-duration-delta-pct",
            args.baseline_max_duration_delta_pct,
        ),
        (
            "--baseline-max-cost-delta-pct",
            args.baseline_max_cost_delta_pct,
        ),
        (
            "--baseline-max-token-delta-pct",
            args.baseline_max_token_delta_pct,
        ),
    ] {
        if !value.is_finite() || value < 0.0 {
            bail!("{flag} must be a finite number >= 0 (got {value})");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agenttrace_core::Metrics;
    use std::io::Write;

    #[test]
    fn report_language_accepts_supported_values_and_rejects_the_rest() {
        assert_eq!(report_language("en").unwrap(), ReportLanguage::En);
        assert_eq!(report_language("EN").unwrap(), ReportLanguage::En);
        assert_eq!(report_language("english").unwrap(), ReportLanguage::En);
        assert_eq!(report_language("zh").unwrap(), ReportLanguage::Zh);
        assert_eq!(report_language("zh-CN").unwrap(), ReportLanguage::Zh);
        assert_eq!(report_language("zh_cn").unwrap(), ReportLanguage::Zh);
        assert_eq!(report_language("chinese").unwrap(), ReportLanguage::Zh);
        // Unknown values previously fell back to English silently.
        assert!(report_language("fr").is_err());
        assert!(report_language("").is_err());
    }

    #[test]
    fn demo_overview_json_is_byte_deterministic() {
        // The CI determinism check compares --demo --overview -f json byte for
        // byte; the wall clock used to leak through scope.generated_at and
        // flake whenever two runs straddled a second boundary.
        let sessions = agenttrace_core::demo_sessions().expect("demo sessions");
        let overview = compute_overview(&sessions);
        let health = data_health(&sessions, sessions.len(), 0);
        let render = || {
            report_overview_json_with_context(
                &overview,
                &sessions,
                Some(&health),
                TimeRange::All,
                false,
                Some(agenttrace_core::DEMO_REPORT_EPOCH),
                // CLI default --limit (20): the recent_sessions list view
                // keeps its internal top-10 cap (pass-3 P3-5).
                20,
            )
        };
        let first = render();
        let second = render();
        assert_eq!(first, second);
        assert!(first.contains(agenttrace_core::DEMO_REPORT_EPOCH));
    }

    #[test]
    fn session_list_tsv_sanitizes_transcript_derived_control_bytes() {
        // rm-383: assess PoC — a crafted session name carried a raw OSC-52
        // clipboard-write sequence byte-for-byte into the --sessions TSV
        // (byte-verified in delegate/172562a8…-scratch/evidence.log). Text
        // renderers neutralize control bytes; JSON output escapes them
        // losslessly and is untouched.
        let session = Session {
            name: "osc\u{001b}]52;c;aGVsbG8=\u{0007}".to_string(),
            path: "/tmp/osc.jsonl".to_string(),
            cwd: String::new(),
            metrics: Metrics {
                source_tool: "pi\u{0007}".to_string(),
                model_used: "m\u{001b}[2J".to_string(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: agenttrace_core::Diagnostics::default(),
        };
        let tsv = render_session_list(std::slice::from_ref(&session), "tsv", 20);
        assert!(
            tsv.lines()
                // Tab is the TSV separator itself; no OTHER control byte
                // may survive in any cell.
                .all(|line| line.chars().all(|c| !c.is_control() || c == '\t')),
            "no raw control bytes in the TSV: {tsv:?}"
        );
        assert!(tsv.contains('\u{FFFD}'));
        assert!(tsv.contains("]52;c;aGVsbG8="));
        // JSON output keeps the bytes, escaped — data integrity over
        // terminal trust, by design.
        let json = render_session_list(&[session], "json", 20);
        assert!(json.contains("\\u001b"));
    }

    #[test]
    fn governance_text_render_sanitizes_report_string_leaves() {
        // rm-383 review-fix (ff2edfcf F1): the governance-family plain
        // renderer behind --mcp-governance (and the shared render_plain_value
        // used by --audit/--recommend/--context-trends/--delivery-evidence,
        // text being the DEFAULT format) printed report strings raw — a
        // crafted mcp_server_name derived from a tool name
        // `mcp__e<ESC>]52;c;aGVsbG8<BEL>vil__tool` carried a byte-for-byte
        // clipboard-write OSC into the terminal. Control bytes → U+FFFD;
        // printable CSI tails legitimately survive; JSON stays escaped.
        let report = serde_json::json!({
            "servers": [
                {
                    "server": "e\u{001b}]52;c;aGVsbG8\u{0007}vil",
                    "invoked_sessions": 1,
                }
            ],
            "audited_sessions": 1,
        });
        let text = render_plain_value(&report, 0);
        assert!(
            // Newline is the renderer's own layout byte; no OTHER control
            // byte may survive in any line (same idiom as the TSV test).
            text.lines()
                .all(|line| line.chars().all(|c| !c.is_control())),
            "no raw control bytes in governance text output: {text:?}"
        );
        assert!(text.contains('\u{FFFD}'));
        assert!(text.contains("]52;c;aGVsbG8"));
        // Non-string leaves render untouched.
        assert!(text.contains("invoked_sessions: 1"));
        // JSON output keeps the bytes escaped — data integrity over
        // terminal trust, by design.
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("\\u001b"));
    }

    #[test]
    fn latest_session_prefers_session_timestamp_over_mod_time() {
        let newer_file = temp_session_file("agenttrace-newer-mtime", "newer-mtime");
        let older_file = temp_session_file("agenttrace-older-mtime", "older-mtime");
        let older_mtime_session = session("older-time", &newer_file, "2026-01-01T00:00:00Z");
        let newer_session_time = session("newer-time", &older_file, "2026-01-02T00:00:00Z");

        assert_eq!(
            latest_session(&[older_mtime_session, newer_session_time])
                .map(|session| session.name.as_str()),
            Some("newer-time")
        );

        let _ = fs::remove_file(newer_file);
        let _ = fs::remove_file(older_file);
    }

    #[test]
    fn latest_session_uses_mod_time_when_timestamps_are_missing() {
        let older = temp_session_file("agenttrace-older-modtime", "older");
        std::thread::sleep(std::time::Duration::from_millis(5));
        let newer = temp_session_file("agenttrace-newer-modtime", "newer");
        let older_session = session("older", &older, "");
        let newer_session = session("newer", &newer, "");

        assert_eq!(
            latest_session(&[newer_session, older_session]).map(|session| session.name.as_str()),
            Some("newer")
        );

        let _ = fs::remove_file(older);
        let _ = fs::remove_file(newer);
    }

    #[test]
    fn latest_session_breaks_mod_time_ties_by_path() {
        let alpha = session_with_missing_file("alpha", "/tmp/agenttrace-alpha.jsonl");
        let omega = session_with_missing_file("omega", "/tmp/agenttrace-omega.jsonl");

        assert_eq!(
            latest_session(&[alpha, omega]).map(|session| session.name.as_str()),
            Some("omega")
        );
    }

    #[test]
    fn cli_view_filters_sorts_and_renders_diagnostics_json() {
        let mut args = compare_args(None);
        args.sessions = true;
        args.health = "crit".to_string();
        args.sort = "cost".to_string();
        args.order = "desc".to_string();
        let mut critical = session("critical", "/tmp/critical", "2026-01-01T00:00:00Z");
        critical.health = 40;
        critical.metrics.cost_estimated = 2.0;
        critical.metrics.tool_calls_fail = 2;
        let mut healthy = session("healthy", "/tmp/healthy", "2026-01-02T00:00:00Z");
        healthy.metrics.cost_estimated = 3.0;

        let filtered = prepare_cli_view(vec![healthy, critical], &args).expect("filter");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "critical");
        assert!(render_session_list(&filtered, "json", 20).contains("\"critical\""));
        let diagnostics = render_diagnostics(&filtered[0], &filtered, "json", ReportLanguage::En)
            .expect("diagnostics");
        assert!(diagnostics.contains("\"diagnostics\""));
        assert!(diagnostics.contains("\"cost_alert\""));

        args.cost = "expensive".to_string();
        assert!(prepare_cli_view(filtered, &args).is_err());
    }

    #[test]
    fn go_flag_compatible_args_rejects_flags_after_positional_path() {
        // rm-247 (was `ignore_flags_after_positional_path`): silent
        // deletion made documented invocations lie — `sessions.jsonl
        // -f json` "worked" while -f vanished, `sessions.jsonl -o
        // out.txt` exited 0 without writing anything, and gate flags
        // after the positional never reached the gate. The shim now
        // rejects the dropped tail loudly, naming the arguments.
        let error = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("session.jsonl"),
            OsString::from("-f"),
            OsString::from("json"),
        ])
        .expect_err("flags after the positional must be rejected");

        let message = error.to_string();
        assert!(
            message.contains("`-f`"),
            "names the dropped flag: {message}"
        );
        assert!(
            message.contains("`-f json`"),
            "names the dropped tail: {message}"
        );
    }

    #[test]
    fn go_flag_compatible_args_tolerates_extra_positionals_after_the_path() {
        // Only flags are rejected after the positional; plain extra
        // positionals keep the historical Go-style tolerance.
        let args = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("a.jsonl"),
            OsString::from("b.jsonl"),
        ])
        .expect("plain extra positionals are not flags");
        assert_eq!(
            args,
            vec![OsString::from("agenttrace"), OsString::from("a.jsonl"),]
        );
    }

    #[test]
    fn go_flag_compatible_args_rejects_flags_after_double_dash_path() {
        let error = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("--"),
            OsString::from("session.jsonl"),
            OsString::from("--clear-cache"),
        ])
        .expect_err("flags after a `--` path must be rejected too");
        assert!(error.to_string().contains("`--clear-cache`"));
    }

    #[test]
    fn go_flag_compatible_args_keep_flags_before_positional_path() {
        let args = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("-f"),
            OsString::from("json"),
            OsString::from("session.jsonl"),
        ])
        .expect("flags before the positional are kept");

        assert_eq!(
            args,
            vec![
                OsString::from("agenttrace"),
                OsString::from("-f"),
                OsString::from("json"),
                OsString::from("session.jsonl"),
            ]
        );
    }

    #[test]
    fn go_flag_shim_treats_sample_as_a_value_flag() {
        // CU-11: --sample takes a value; the Go-compatible shim must
        // consume it so later flags (like -f json) still parse instead
        // of being silently dropped as "positional".
        let args = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("--sample"),
            OsString::from("20"),
            OsString::from("-f"),
            OsString::from("json"),
        ])
        .expect("value flags keep parsing");
        assert_eq!(
            args,
            vec![
                OsString::from("agenttrace"),
                OsString::from("--sample"),
                OsString::from("20"),
                OsString::from("-f"),
                OsString::from("json"),
            ]
        );
    }

    #[test]
    fn go_flag_shim_keeps_boolean_no_baseline_gate_before_value_flags() {
        // A11-5 (cycle-4 review F2, found twice): --no-baseline-gate is a
        // boolean, but it sat in flag_takes_value, so the shim consumed
        // the next token and silently dropped the flags after it — the
        // documented CI recipe
        // (--no-baseline-gate --baseline X --overview -f json) died with
        // the misleading "--baseline requires --overview -f json".
        // Leading placement: every later flag must survive the shim.
        let args = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("--no-baseline-gate"),
            OsString::from("--baseline"),
            OsString::from("base.json"),
            OsString::from("--overview"),
            OsString::from("-f"),
            OsString::from("json"),
        ])
        .expect("leading placement");
        assert_eq!(
            args,
            vec![
                OsString::from("agenttrace"),
                OsString::from("--no-baseline-gate"),
                OsString::from("--baseline"),
                OsString::from("base.json"),
                OsString::from("--overview"),
                OsString::from("-f"),
                OsString::from("json"),
            ]
        );

        // Trailing placement must keep working: the boolean ends the
        // line without consuming anything.
        let args = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("--overview"),
            OsString::from("-f"),
            OsString::from("json"),
            OsString::from("--no-baseline-gate"),
        ])
        .expect("trailing placement before any positional");
        assert_eq!(
            args,
            vec![
                OsString::from("agenttrace"),
                OsString::from("--overview"),
                OsString::from("-f"),
                OsString::from("json"),
                OsString::from("--no-baseline-gate"),
            ]
        );
    }

    #[test]
    fn conflicting_report_actions_are_rejected() {
        let mut args = compare_args(None);
        args.sessions = true;
        assert!(validate_primary_action(&args).is_err());

        args.compare = false;
        args.sessions = false;
        args.latest = true;
        args.diagnostics = true;
        assert!(validate_primary_action(&args).is_ok());
    }

    #[test]
    fn single_session_report_yields_to_explicit_actions() {
        // rm-246 (assess N1): a positional session path (or --latest)
        // used to render the single-session report even when
        // --overview/--search was asked for, silently skipping the
        // overview quality gate (`--overview --fail-under-health 100
        // <file>` exited 0). Explicit actions now route the path into
        // the corpus loader instead. Actions dispatched before this
        // decision (--compare/--audit/--waste/…) never reach it.
        let mut args = compare_args(None);
        args.compare = false;
        args.path = Some("session.jsonl".into());
        assert!(single_session_report_requested(&args));

        args.overview = true;
        assert!(!single_session_report_requested(&args));
        args.overview = false;

        args.search = Some("tokens".into());
        assert!(!single_session_report_requested(&args));
        args.search = None;

        args.sessions = true;
        assert!(!single_session_report_requested(&args));
        args.sessions = false;

        args.latest = true;
        assert!(single_session_report_requested(&args));
        args.diagnostics = true;
        assert!(!single_session_report_requested(&args));
    }

    #[test]
    fn flag_takes_value_covers_every_value_taking_clap_flag() {
        // rm-247 canary: flag_takes_value is a hand-maintained list.
        // When a value-taking flag is missing from it, the shim treats
        // the flag's VALUE as the first positional and the truncation
        // point moves — exactly the A11-5 failure mode. Derive the
        // expected set from the clap definition so drift fails here
        // instead of in production argv.
        use clap::CommandFactory;

        let mut missing: Vec<String> = Vec::new();
        for arg in Args::command().get_arguments() {
            let takes_values = arg
                .get_num_args()
                .map(|range| range.takes_values())
                .unwrap_or(false);
            if !takes_values {
                continue;
            }
            if let Some(short) = arg.get_short() {
                let token = format!("-{short}");
                if !flag_takes_value(&OsString::from(&token)) {
                    missing.push(token);
                }
            }
            if let Some(long) = arg.get_long() {
                let token = format!("--{long}");
                if !flag_takes_value(&OsString::from(&token)) {
                    missing.push(token);
                }
            }
        }
        assert!(
            missing.is_empty(),
            "value-taking flags missing from flag_takes_value (their values would be \
             swallowed as positionals): {missing:?}"
        );
    }

    #[test]
    fn write_output_stages_atomically_and_leaves_no_temp_residue() {
        // rm-250: -o must stage through a temp sibling and rename, so a
        // crash mid-write never leaves a truncated report and no temp
        // files survive any exit path.
        let dir =
            std::env::temp_dir().join(format!("agenttrace-write-output-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("scratch dir");

        let target = dir.join("report.txt");
        write_output(&Some(target.clone()), "body\n").expect("write");
        assert_eq!(fs::read_to_string(&target).unwrap(), "body\n");

        // Failing rename (destination occupied by a directory) must
        // clean the staged temp file.
        let occupied = dir.join("occupied");
        fs::create_dir_all(&occupied).unwrap();
        assert!(write_output(&Some(occupied), "body\n").is_err());

        let residue: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp."))
            .collect();
        assert!(residue.is_empty(), "temp residue left behind: {residue:?}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn compare_uses_shared_filters_and_limit() {
        let root =
            std::env::temp_dir().join(format!("agenttrace-compare-cap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create compare temp dir");
        for idx in 0..16 {
            write_compare_session(&root.join(format!("{idx:02}.jsonl")), idx);
        }

        let mut args = compare_args(Some(root.to_string_lossy().to_string()));
        args.limit = 1;
        let sessions = prepare_cli_view(load_sessions(&args).unwrap(), &args).unwrap();
        assert_eq!(sessions.into_iter().take(args.limit).count(), 1);

        args.source = "missing-source".to_string();
        assert!(load_sessions(&args).is_err());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_sessions_reports_empty_after_parse_failures_like_go_overview() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-empty-after-parse-failure-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create parse failure dir");
        fs::write(root.join("storage.json"), r#"{"not":"#).expect("write bad json");

        let mut args = compare_args(Some(root.to_string_lossy().to_string()));
        args.compare = false;
        args.overview = true;
        let err = load_sessions(&args).expect_err("empty parseable sessions should fail");
        assert!(err.to_string().contains("No sessions match"));

        let _ = fs::remove_dir_all(root);
    }

    fn temp_session_file(prefix: &str, content: &str) -> String {
        let path = std::env::temp_dir().join(format!("{prefix}-{}.jsonl", std::process::id()));
        let mut file = fs::File::create(&path).expect("create temp session");
        writeln!(file, "{content}").expect("write temp session");
        path.to_string_lossy().to_string()
    }

    #[test]
    fn sessions_query_filter_folds_unicode() {
        // rm-016 + review F1: --query is user text (PR-5), so it must fold
        // Unicode-aware — 'CAFÉ' used to miss a session named 'café …'
        // while --search had already been fixed (live review PoC).
        let s = session("café résumé", "/tmp/utf8.jsonl", "2026-01-01T00:00:00Z");
        assert!(matches_text(&s, "CAFÉ"));
        assert!(matches_text(&s, "Résumé"));
        assert!(matches_text(&s, "résumé"));
        assert!(matches_text(&s, "  CAFÉ  "));
        assert!(!matches_text(&s, "nomatch"));
    }

    fn session(name: &str, path: &str, session_start: &str) -> Session {
        Session {
            name: name.to_string(),
            path: path.to_string(),
            cwd: String::new(),
            metrics: Metrics {
                session_start: session_start.to_string(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: agenttrace_core::Diagnostics::default(),
        }
    }

    fn session_with_missing_file(name: &str, path: &str) -> Session {
        session(name, path, "")
    }

    fn compare_args(dir: Option<String>) -> Args {
        Args {
            path: None,
            format: "json".to_string(),
            dir,
            compare: true,
            audit: false,
            recommend: false,
            mcp_governance: false,
            context_trends: false,
            delivery_evidence: false,
            overview: false,
            sessions: false,
            diagnostics: false,
            inspect: None,
            model: "default".to_string(),
            output: None,
            latest: false,
            waste: false,
            list_models: false,
            statusline_report: false,
            fetch: false,
            update_pricing: false,
            test_match: false,
            version: false,
            demo: false,
            doctor: false,
            search: None,
            search_limit: 20,
            fail_under_health: 0,
            fail_on_critical: false,
            max_tool_fail_rate: None,
            baseline: None,
            baseline_max_duration_delta_pct: 0.0,
            baseline_max_cost_delta_pct: 0.0,
            baseline_max_token_delta_pct: 0.0,
            no_baseline_gate: false,
            lang: "en".to_string(),
            range: "all".to_string(),
            project: String::new(),
            source: String::new(),
            model_filter: String::new(),
            query: String::new(),
            health: String::new(),
            cost: String::new(),
            anomaly: String::new(),
            sort: "recent".to_string(),
            order: "desc".to_string(),
            limit: 20,
            sample: None,
            clear_cache: false,
            preserve_history: false,
            include_history: false,
        }
    }

    #[test]
    fn governance_formats_and_gate_thresholds_keep_cli_contracts() {
        let value = serde_json::json!({"status": "ok"});
        assert!(render_governance_report(&value, "json")
            .expect("json")
            .starts_with('{'));
        assert!(render_governance_report(&value, "markdown")
            .expect("markdown")
            .starts_with("```json"));
        assert!(render_governance_report(&value, "html")
            .expect("html")
            .starts_with("<pre>"));
        assert!(render_governance_report(&value, "text")
            .expect("text")
            .starts_with("status:"));

        let mut args = compare_args(None);
        args.max_tool_fail_rate = Some(f64::NAN);
        assert!(validate_gate_thresholds(&args).is_err());
        args.max_tool_fail_rate = Some(101.0);
        assert!(validate_gate_thresholds(&args).is_err());
        args.max_tool_fail_rate = Some(100.0);
        assert!(validate_gate_thresholds(&args).is_ok());
    }

    fn write_compare_session(path: &std::path::Path, idx: usize) {
        let mut file = fs::File::create(path).expect("create compare session");
        writeln!(
            file,
            r#"{{"role":"user","content":"compare {idx}","timestamp":"2026-05-02T10:00:{idx:02}Z","ModelUsed":"gpt-4.1"}}"#
        )
        .expect("write compare user");
        writeln!(
            file,
            r#"{{"role":"assistant","content":"done","timestamp":"2026-05-02T10:01:{idx:02}Z","ModelUsed":"gpt-4.1"}}"#
        )
        .expect("write compare assistant");
    }

    #[test]
    fn range_without_a_consumer_is_rejected() {
        // rm-244: `agenttrace --range 30d` used to fall through to the
        // TUI with the filter silently ignored. It must exit rc!=0,
        // name the flag, and suggest composable actions.
        let mut args = compare_args(None);
        args.compare = false;
        args.range = "30d".to_string();
        let err = validate_range_applicability(&args)
            .expect_err("must reject --range without a consumer");
        let message = err.to_string();
        assert!(
            message.contains("--range"),
            "error must name the flag: {message}"
        );
        assert!(
            message.contains("--overview"),
            "error must suggest a report action: {message}"
        );
    }

    #[test]
    fn range_default_and_range_with_consumer_pass_the_guard() {
        let mut args = compare_args(None);
        args.compare = false;
        args.range = "all".to_string();
        assert!(validate_range_applicability(&args).is_ok());

        let mut args = compare_args(None);
        args.range = "7d".to_string();
        assert!(validate_range_applicability(&args).is_ok());

        let mut args = compare_args(None);
        args.compare = false;
        args.overview = true;
        args.range = "7d".to_string();
        assert!(validate_range_applicability(&args).is_ok());
    }
}

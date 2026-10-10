use agenttrace_core::{
    add_baseline_comparison, average_health, build_doctor_report, compute_overview,
    compute_waste_report, context_trends, cost_audit, data_health, data_health_scoped,
    delivery_evidence_with_git, demo_sessions, evaluate_overview_gate, filter_sessions,
    fix_suggestions, inspect_first, list_pricing, load_sessions_with_options, lookup_price,
    matches_numeric_filter, mcp_governance, parse_file, parse_numeric_filter, parse_stdin_bytes,
    predict_cost_anomaly, pricing_cache_path, pricing_source, recommendations,
    render_doctor_report, render_model_pricing_list, render_test_match,
    render_waste_report_with_language, report_compare_json, report_json_with_language,
    report_overview_html_with_context, report_overview_json_with_context,
    report_overview_markdown_with_context, report_overview_svg_with_context,
    report_overview_text_with_context, report_search_json, report_search_text,
    report_text_with_language, sanitize_line_segment, search_sessions, session_capability,
    session_start_cmp, tool_fail_rate, total_tokens, update_pricing, waste_report_json,
    BaselineThresholds, LoadOptions, LoadReport, ReportLanguage, Session, SvgCardTheme, TimeRange,
    VERSION,
};
use anyhow::{bail, Context};
use chrono::Utc;
use clap::Parser;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

mod csv_export;
mod mcp;
mod upstream;

mod config;

#[derive(Debug, Parser)]
#[command(name = "agenttrace")]
#[command(about = "TUI observability for AI coding agent sessions")]
struct Args {
    /// Session file to analyze (`-` reads one session stream from
    /// stdin through the same parser; stdin sessions are ephemeral and
    /// never touch the session cache)
    path: Option<String>,
    /// Output format for the requested view: text (default), json, csv,
    /// markdown/md, html, svg (the shareable usage card; requires
    /// --overview), or otel (the OTLP-JSON export of the whole
    /// corpus; requires --overview). Unsupported combinations fail
    /// loudly instead of falling back to text.
    #[arg(
        short = 'f',
        long = "format",
        default_value = "text",
        value_parser = ["text", "json", "csv", "markdown", "md", "html", "otel", "svg"]
    )]
    format: String,
    /// rm-576: color scheme of the SVG usage card (`-f svg`). `auto`
    /// ships the light palette plus a `prefers-color-scheme` override so
    /// the static file adapts to the viewer; bytes stay deterministic.
    #[arg(long = "card-theme", default_value = "auto")]
    card_theme: SvgCardTheme,
    /// Session directory to scan instead of auto-discovered agent homes
    #[arg(short = 'd', long = "dir")]
    dir: Option<String>,
    /// Compare the inspected session against the healthy-baseline
    /// summary (narrative framed for `--model`).
    #[arg(long)]
    compare: bool,
    /// Render the governance audit report across matching sessions
    /// (tool-authority drift and spend oversight; `--sample` bounds it).
    #[arg(long)]
    audit: bool,
    /// Render cost and efficiency recommendations derived from the
    /// session corpus.
    #[arg(long = "recommend")]
    recommend: bool,
    /// Audit MCP server governance across sessions: which servers were
    /// reachable, which tools they exposed, and allowlist drift.
    #[arg(long = "mcp-governance")]
    mcp_governance: bool,
    /// Render context-utilization trends over the session corpus
    /// (window pressure, cache reuse, growth by turn).
    #[arg(long = "context-trends")]
    context_trends: bool,
    /// Render the delivery-evidence report: verifiable outcome signals
    /// per session rather than effort metrics.
    #[arg(long = "delivery-evidence")]
    delivery_evidence: bool,
    /// Render the corpus overview report: totals, health mix, and
    /// model/provider/project breakdowns.
    #[arg(long)]
    overview: bool,
    /// List sessions as rows (TSV text by default; `--format` json/csv
    /// for machine use). The TSV ends with the subagent rollup columns
    /// SUBAGENTS and SUBAGENT_COST — attributed spawned work, kept
    /// separate from the session's own COST/TOKENS cells. Rollups are
    /// corpus-scope: they count every child in the loaded corpus, not
    /// only the rows surviving the active view filters (rm-798).
    /// Machine formats never let `--limit` read as the whole corpus:
    /// json wraps the rows with matched_sessions/returned_sessions/
    /// truncated/limit, and csv appends a `# truncated:` marker row
    /// when rows were dropped (rm-798); csv also carries the subagent
    /// parity columns subagents/subagent_cost/parent_session (rm-855).
    #[arg(long)]
    sessions: bool,
    /// Render per-session diagnostics: findings, evidence, fix
    /// suggestions, and next actions.
    #[arg(long)]
    diagnostics: bool,
    /// Inspect a single session by its `--sessions` list index
    /// (1-based) instead of the whole corpus.
    #[arg(long)]
    inspect: Option<usize>,
    /// Reference model for `--compare` framing (cost-rate attribution
    /// in the comparison narrative; not a session filter — see
    /// `--model-filter`).
    #[arg(short = 'm', default_value = "default")]
    model: String,
    /// Write the report to this path instead of stdout. Regular paths
    /// stage atomically through a temp sibling (rm-250); terminal
    /// sinks like /dev/null and /dev/stdout write through directly,
    /// while fifo/socket/block-device targets are refused with a
    /// disclosed reason instead of being replaced (rm-489). Missing
    /// parent directories are created automatically (mkdir -p
    /// semantics); when a parent cannot be created the error names
    /// the directory and the requested target (rm-784).
    #[arg(short = 'o')]
    output: Option<PathBuf>,
    /// Restrict the view to the single most recent session (works with
    /// `--waste` and report actions).
    #[arg(long)]
    latest: bool,
    /// Render the token-waste report: redundant context, loop cost,
    /// and unused tool output, with a per-reason breakdown.
    #[arg(long)]
    waste: bool,
    /// List the models the pricing catalog knows, with rate coverage;
    /// `--test-match` shows the resolution probes.
    #[arg(long = "list-models")]
    list_models: bool,
    /// Refresh the vendored pricing-catalog snapshots from their
    /// upstream sources, then report the drift.
    #[arg(long = "update-pricing")]
    update_pricing: bool,
    /// Print the catalog-resolution table for a probe set of model
    /// identifiers (alias and rate-match verification).
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
    #[arg(long = "fetch")]
    fetch: bool,
    /// Explicit configuration file (rm-384). Layered above the
    /// project and user config files; a missing file is an error.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    /// Override the history directory: highest precedence, then the
    /// config files, then `AGENTTRACE_HISTORY_DIR` (rm-384).
    #[arg(long, value_name = "PATH")]
    history_dir: Option<PathBuf>,
    /// Override the pricing override file: highest precedence, then
    /// the config files, then `AGENTTRACE_PRICING_FILE` (rm-384).
    #[arg(long, value_name = "PATH")]
    pricing_file: Option<PathBuf>,
    /// Weekly spend budget in USD for `--statusline-report` and
    /// `--budget`: highest precedence, then config
    /// `weekly_budget_usd` (rm-385).
    #[arg(long, value_name = "USD")]
    weekly_budget: Option<f64>,
    /// Show the weekly budget window-burn view: per-day spend from
    /// the statusline journal against the resolved weekly budget
    /// (rm-385).
    #[arg(long)]
    budget: bool,
    /// Print the version banner and exit 0; wins over action
    /// validation.
    #[arg(long)]
    version: bool,
    /// Use the built-in demo corpus instead of discovered agent homes
    /// (stable epoch-anchored sessions).
    #[arg(long)]
    demo: bool,
    /// Run environment self-checks (config paths, cache consistency,
    /// agent homes) and exit non-zero on failure.
    #[arg(long)]
    doctor: bool,
    /// Full-text search across discovered session transcripts for
    /// this query.
    #[arg(long)]
    search: Option<String>,
    /// Cap the number of `--search` hits reported (default 20).
    #[arg(long = "search-limit", default_value_t = 20)]
    search_limit: usize,
    /// CI gate: exit non-zero when corpus health drops below this
    /// score (0 disables the gate).
    #[arg(long = "fail-under-health", default_value_t = 0)]
    fail_under_health: i32,
    /// CI gate: exit non-zero when any critical-severity finding
    /// exists.
    #[arg(long = "fail-on-critical")]
    fail_on_critical: bool,
    /// CI gate: exit non-zero when the tool failure rate exceeds this
    /// fraction (0.0-1.0).
    #[arg(long = "max-tool-fail-rate")]
    max_tool_fail_rate: Option<f64>,
    /// Load a healthy-baseline summary JSON (path or id) to compare
    /// `--compare` runs against.
    #[arg(long)]
    baseline: Option<String>,
    /// Baseline gate: maximum allowed session duration drift, in
    /// percent.
    #[arg(long = "baseline-max-duration-delta-pct", default_value_t = 0.0)]
    baseline_max_duration_delta_pct: f64,
    /// Baseline gate: maximum allowed session cost drift, in percent.
    #[arg(long = "baseline-max-cost-delta-pct", default_value_t = 0.0)]
    baseline_max_cost_delta_pct: f64,
    /// Baseline gate: maximum allowed session token drift, in percent.
    #[arg(long = "baseline-max-token-delta-pct", default_value_t = 0.0)]
    baseline_max_token_delta_pct: f64,
    /// Opt out of the baseline regression gate: keep the comparison in the
    /// report but do not fail the run (exit 2) on a threshold breach
    /// (pass-7 P7-3).
    #[arg(long = "no-baseline-gate")]
    no_baseline_gate: bool,
    /// Report language for text and TUI surfaces: en (default) or zh.
    #[arg(long = "lang", default_value = "en", value_name = "en|zh")]
    lang: String,
    /// Time range filter for every report: today, 7d, 30d, or all
    /// (default all). A session counts when it had activity in the
    /// window, even one that started before it.
    #[arg(long, default_value = "all")]
    range: String,
    /// rm-779: keep only sessions whose resolved project matches this
    /// filter. Matching is a case-insensitive SUBSTRING test against
    /// any of the project's resolved identity, display name, and
    /// filesystem root, so `--project storefront` keeps both
    /// `/work/storefront` and `storefront-api`; use the encoded
    /// `-work-projects-storefront` spelling for claude-encoded session
    /// directories. The project resolves through the same identity
    /// machinery as the report's by_project rollup (git-root grouping
    /// when a `.git` is discoverable, else the normalized cwd, else
    /// the encoded/unattributed fallbacks).
    #[arg(long, default_value = "")]
    project: String,
    /// Filter sessions by source tool substring (e.g. claude-code,
    /// codex, pi).
    #[arg(long, default_value = "")]
    source: String,
    /// Filter sessions by model name substring.
    #[arg(long = "model-filter", default_value = "")]
    model_filter: String,
    /// Filter sessions by a free-text query over identity fields.
    #[arg(long, default_value = "")]
    query: String,
    /// Filter sessions by health tier (healthy/warning/critical) or a
    /// numeric comparison in the shared CLI/TUI dialect: an optional
    /// `>=`, `<=`, `>`, `<`, or `=` operator followed by a finite number,
    /// where a bare number means `>=` (e.g. `warn`, `>=80`, `80`).
    #[arg(long, default_value = "")]
    health: String,
    /// Filter sessions by a cost comparison in the shared CLI/TUI
    /// dialect: an optional `>=`, `<=`, `>`, `<`, or `=` operator followed
    /// by a finite number, where a bare number means `>=`
    /// (e.g. `>1.5`, `1.5`).
    #[arg(long, default_value = "")]
    cost: String,
    /// Filter sessions by anomaly kind.
    #[arg(long, default_value = "")]
    anomaly: String,
    /// Sort key for list views: recent (default), health, cost, turns,
    /// failures, source, or name.
    #[arg(long, default_value = "recent")]
    sort: String,
    /// Sort direction for list views: desc (default) or asc.
    #[arg(long, default_value = "desc")]
    order: String,
    /// Maximum number of rows a list view renders (default 20).
    #[arg(long, default_value_t = 20)]
    limit: usize,
    /// Explicitly bound governance reports to the newest N sessions.
    /// Governance reports audit every matching session by default;
    /// sampling is always disclosed via audited_sessions/total_sessions
    /// (pass-8 F8-1).
    #[arg(long)]
    sample: Option<usize>,
    /// Delete the session cache before running, forcing a full
    /// re-parse.
    #[arg(long = "clear-cache")]
    clear_cache: bool,
    /// Persist derived metrics to the history ledger so later
    /// `--include-history` runs can see them.
    #[arg(long = "preserve-history")]
    preserve_history: bool,
    /// Merge preserved-history sessions into the view (offline or
    /// retained-history analysis).
    #[arg(long = "include-history")]
    include_history: bool,
}

fn main() {
    if let Err(err) = run() {
        // rm-610: anyhow's Display prints only the outermost context
        // layer, so `--overview -o /dev/full` (ENOSPC) and `-o <dir>`
        // (EISDIR) used to exit with byte-identical stderr — the io
        // error kind underneath the context line was discarded at this
        // handler. Join the full cause chain onto one line instead;
        // single-layer errors (and therefore every exit code) render
        // exactly as before.
        let chain = err
            .chain()
            .map(|cause| cause.to_string())
            .collect::<Vec<_>>()
            .join(": ");
        eprintln!("Error: {chain}");
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
    // rm-573: an explicit --statusline-report wins over keyword host dispatch —
    // otherwise `agenttrace --statusline-report statusline` runs the host and
    // appends a bogus capture to the real journal before the report renders.
    if args.path.as_deref() == Some("statusline") && !args.statusline_report {
        return agenttrace_core::run_statusline_host();
    }
    // `agenttrace upstream` is a repository status host command (rm-024,
    // cycle 1): fork-vs-upstream drift made visible. Like the statusline
    // host command it dispatches before action validation; unlike it,
    // this is an explicit user action that may fail loudly. It is fully
    // offline unless --fetch explicitly opts into the network.
    // rm-573: --statusline-report also wins over the upstream keyword lane.
    if args.path.as_deref() == Some("upstream") && !args.statusline_report {
        let report = upstream::status_report(&args.format, args.fetch)?;
        // rm-625: the drift report rides the same dispatch choke point
        // as every other report lane (control bytes sanitized outside
        // -f json) — and rm-652: honor -o like every other report arm,
        // the report previously streamed to stdout only, silently
        // ignoring the flag. write_output is a no-op without -o, so
        // default stdout behavior is unchanged; the sanitized twin is
        // byte-identical on stdout and in the file (the report already
        // ends with a newline, no reformatting for the file lane).
        let out = dispatch_sanitize(&args.format, report);
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }
    // `agenttrace mcp` is the local MCP server host command (rm-455):
    // the coding agent asks "where did my tokens go" over the same
    // locally discovered sessions the CLI reports on. Like the other
    // host keywords it dispatches before action validation; the server
    // is fully read-only and offline — stdio only, no sockets, and no
    // writes outside the session cache the CLI itself maintains. It
    // serves until stdin closes (host disconnect = clean exit).
    // rm-573: an explicit --statusline-report wins over keyword host
    // dispatch here too — otherwise `--statusline-report mcp` silently
    // swaps the requested report for a stdin-blocked server (the report
    // never renders; the same keyword-shadowing class rm-573 closed for
    // statusline/upstream, caught for `mcp` at integration of rm-455).
    if args.path.as_deref() == Some("mcp") && !args.statusline_report {
        return mcp::serve();
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
    // rm-576 review F1 (integration review 2026-10-09): the SVG usage
    // card is an --overview renderer only — it consumes the overview
    // aggregation pass, and the governance actions have no svg arm, so
    // admitting them in the guard above let render_governance_report's
    // '_' fallback silently render plain text (rc=0; with -o, text was
    // written into a .svg file) — the rm-409 silent-fallthrough class
    // this guard family exists to prevent. Bail loudly instead,
    // mirroring the otel lane's overview-only gate below.
    if args.format == "svg" && !args.overview {
        bail!("svg format requires --overview (the usage card renders the overview aggregation)");
    }
    // rm-409 (review fix): csv's composable set is exactly --overview and
    // --sessions (the README column contract); every other report shape
    // renders text/json, so `-f csv` on them previously fell through to a
    // silent TEXT render — the same incoherence the guard above exists to
    // prevent for markdown/html. Bail loudly instead (--latest, <path>,
    // --doctor, --diagnostics, and the governance actions included).
    if args.format == "csv" && !(args.overview || args.sessions) {
        bail!("csv format requires --overview or --sessions");
    }
    // rm-599: `-f otel` renders the OTLP-JSON export of the whole
    // session corpus, so it rides the --overview lane (which loads
    // every session). On any other lane it would silently render text
    // — the same incoherence the guards above exist to prevent.
    if args.format == "otel" && !args.overview {
        bail!("otel format requires --overview");
    }
    validate_range_applicability(&args)?;
    // rm-735: --demo is itself a session source; an explicit source
    // beside it bails here — before any side effect and before any
    // lane dispatch — so every demo-reading lane is covered by one
    // guard instead of one per lane: the report actions, the
    // single-session report, the interactive TUI, and --doctor's
    // demo corpus (landed rm-596 made `--doctor --demo` render only
    // the bundled corpus, so a `-d` beside `--demo` would otherwise
    // be silently discarded there — the exact substitution this
    // guard exists to refuse).
    reject_demo_source_conflict(&args)?;

    let language = report_language(&args.lang)?;
    // rm-384: resolve the layered configuration and install the
    // winning knobs before any consumer (history persistence,
    // pricing override loading) initializes.
    let resolved = config::resolve(&args)?;
    agenttrace_core::set_runtime_config(agenttrace_core::RuntimeConfigOverrides {
        history_dir: resolved.history_dir.clone(),
        pricing_file: resolved.pricing_file.clone(),
        // rm-298 capacity arm: the config-file layer outranks the env
        // knob; when unset here core falls back to
        // AGENTTRACE_SESSION_CACHE_ENTRIES, then the built-in default.
        session_cache_entries: resolved.session_cache_entries,
    });

    // rm-301: stdout purity under machine formats. The side-effect
    // announcements below are human progress chatter; with `-f json` the
    // stdout stream must stay a single parseable JSON document end to
    // end (a downstream `jq` breaks on any leading line), so they route
    // to stderr. The human path keeps them on stdout exactly as before.
    // rm-912 rider (assess F3): the DOCUMENT lanes are machine-piped
    // too — `--clear-cache --overview -f svg > card.svg` (the README
    // share pattern) used to prefix "Session cache cleared." to the
    // SVG bytes and corrupt the file — so svg/markdown/md/html join
    // the stderr-routing set.
    let announce: fn(&str) -> anyhow::Result<()> = if matches!(
        args.format.as_str(),
        "json" | "csv" | "otel" | "svg" | "markdown" | "md" | "html"
    ) {
        write_stderr
    } else {
        write_stdout
    };

    if args.clear_cache {
        agenttrace_core::clear_session_cache()?;
        announce("Session cache cleared.\n")?;
        // rm-301 (cycle-3 extension, assess 7276 A1/A2): a requested
        // action must never be silently dropped by the clear-and-exit —
        // this guard and --update-pricing's below consult the ONE
        // shared action-introspection helper, not a hand-maintained
        // subset that drifts as arms are added.
        if !has_followup_action(&args) {
            return Ok(());
        }
    }

    if args.update_pricing {
        // Network-touch announcement, on stderr per PRIVACY.md ("announced on
        // stderr at the moment it happens"): stdout may be redirected to a
        // file or pipe and the announcement must still reach the terminal —
        // mirrors upstream.rs's `--fetch` disclosure (rm-404, review finding 1).
        eprintln!("Downloading pricing from LiteLLM...");
        let count = update_pricing()?;
        announce(&format!("Loaded {count} model prices\n"))?;
        announce(&format!(
            "Cache saved: {}\n",
            pricing_cache_path().display()
        ))?;
        if !has_followup_action(&args) {
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
        let out = dispatch_sanitize(&args.format, out);
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }

    if args.doctor {
        // rm-608: the doctor lane used to early-return before the -d
        // guard, so a typo'd or file-valued directory reported rc=0
        // "healthy". Route through the same validation as every other
        // `-d` consumer before rendering anything.
        validate_explicit_dir(args.dir.as_deref());
        let doctor_dir = args.dir.as_deref().map(PathBuf::from);
        // rm-384: disclose the configuration layers and the winning
        // source of every knob. JSON mode keeps stdout a single pure
        // object; the disclosure goes to stderr as its own document.
        if args.format == "json" {
            let disclosure = config::disclosure_json(&resolved);
            eprintln!("{disclosure}");
            // rm-656: build the report once and embed the SAME
            // disclosure payload into the `-o` artifact as a top-level
            // `config_disclosure` object — archiving a doctor run must
            // not lose the layer/knob provenance the terminal path
            // prints. The stdout document stays byte-identical to
            // render_doctor_report's json arm (the rm-384 terminal
            // contract); the embedded payload equals the stderr doc.
            let report = build_doctor_report(doctor_dir.as_deref(), args.demo);
            let out = serde_json::to_string_pretty(&report)? + "\n";
            let artifact = match &args.output {
                Some(_) => {
                    let mut doc = serde_json::to_value(&report)
                        .context("serializing the doctor report for -o disclosure embedding")?;
                    if let Some(object) = doc.as_object_mut() {
                        object.insert(
                            "config_disclosure".to_string(),
                            serde_json::from_str(disclosure.trim_end())
                                .context("parsing the config disclosure for -o embedding")?,
                        );
                    }
                    serde_json::to_string_pretty(&doc)? + "\n"
                }
                None => out.clone(),
            };
            write_output(&args.output, &artifact)?;
            write_stdout(&out)?;
            return Ok(());
        }
        // rm-625: the text doctor lane renders transcript-derived
        // samples, so it rides the same dispatch choke point as
        // every other text render.
        let out = format!(
            "{}{}",
            config::disclosure_text(&resolved),
            render_doctor_report(doctor_dir.as_deref(), args.demo, &args.format)?
        );
        let out = dispatch_sanitize(&args.format, out);
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
        let out = dispatch_sanitize(&args.format, out);
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }

    if args.statusline_report {
        let out = agenttrace_core::render_statusline_report(&args.format, resolved.weekly_budget)?;
        let out = dispatch_sanitize(&args.format, out);
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }

    if args.budget {
        // rm-385: the weekly window-burn view — journal-based, no
        // session discovery, mirroring --statusline-report.
        let out = agenttrace_core::render_budget_view(&args.format, resolved.weekly_budget)?;
        // rm-625: the budget lane renders journal-derived prose (model
        // names from statusline captures), so it rides the same
        // dispatch choke point as --statusline-report.
        let out = dispatch_sanitize(&args.format, out);
        write_output(&args.output, &out)?;
        write_stdout(&out)?;
        return Ok(());
    }

    if !has_session_action(&args) {
        if args.demo {
            // rm-735: the interactive demo TUI is the same silent
            // substitution arm as the report lane (`--demo -d /logs`
            // used to open the demo TUI and discard the flag); the
            // launch-guard at the top of main has already refused
            // the conflict, so the TUI never opens on a swallowed
            // source.
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
        let out = dispatch_sanitize(&args.format, out);
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        // rm-346 arm-a: the --fail-* gate flags are parsed on every report
        // path but used to be evaluated only under --overview — a gated
        // `--audit` (or sibling governance report) exited rc0 with 0-byte
        // stderr on a corpus the same flags condemn. Honor them here too:
        // the report still prints, the failures and evidence go to stderr,
        // and the process exits 2.
        enforce_report_gates(&args, &sessions);
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
        let out = dispatch_sanitize(&args.format, out);
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        // rm-486: the compare branch's own comment (above) claims the same
        // coverage contract as the governance branch, but it returned Ok(())
        // without ever evaluating the --fail-* gates — a gated `--compare`
        // exited rc0 on a corpus the same flags condemn on every sibling
        // report. Route it through the shared gate enforcement so a CI gate
        // built on --compare cannot pass silently.
        enforce_report_gates(&args, &sessions);
        return Ok(());
    }

    if args.waste {
        let sessions = prepare_cli_view(load_sessions(&args)?, &args)?;
        let session =
            latest_session(&sessions).context("No sessions match the requested filters")?;
        // rm-544: the format guard admits `-f json` for every action, so
        // every admitted machine format must be honored here too — the
        // json cell used to fall through to the text banner, handing
        // scripts prose where they expected data. markdown/html stay
        // rejected upstream (they are overview/governance surfaces).
        let out = match args.format.as_str() {
            "json" => waste_report_json(&compute_waste_report(session)),
            _ => render_waste_report_with_language(session, language),
        };
        let out = dispatch_sanitize(&args.format, out);
        write_output(&args.output, &(out.clone() + "\n"))?;
        write_stdout(&out)?;
        // rm-486 premise rider: rm-544 made `-f json` a versioned machine
        // contract (`agenttrace.waste.v1`) documented for CI, so the row's
        // old "--waste ungated BY DESIGN" carve-out — recorded when waste
        // rendered a human text view — no longer holds for the machine
        // form. The json arm evaluates the same shared gate contract as
        // `--overview`, `--governance` and `--compare`: write-then-gate,
        // so the artifact is still emitted before a failing exit. The
        // flags judge the filtered session view the report is drawn from;
        // the waste metrics themselves (waste_score, cache rating, tool
        // bloat) are not gate inputs and carry no gate flags. The human
        // text view stays ungated by design.
        if args.format.as_str() == "json" {
            enforce_report_gates(&args, &sessions);
        }
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
        let out = dispatch_sanitize(&args.format, out);
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
            // rm-798: out-of-band truncation note for every format —
            // json/csv carry the counts in-band too, but a human or a
            // TSV consumer should not have to go looking for them.
            if args.limit < sessions.len() {
                eprintln!(
                    "Note: --limit caps this list view only: showing {} of {} matching sessions.",
                    args.limit,
                    sessions.len()
                );
            }
            let out = render_session_list(&sessions, &args.format, args.limit);
            let out = dispatch_sanitize(&args.format, out);
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
        let out = dispatch_sanitize(&args.format, out);
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
        let out = dispatch_sanitize(&args.format, out);
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
                report.opencode_fork_excluded,
                report.sqlite.unreadable.clone(),
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
            "csv" => csv_export::overview_csv(&overview),
            // rm-599: the OTLP-JSON export lane — the CLI half of the
            // rm-493 renderer. Every session becomes one span with a
            // valid (non-zero) span id; see agenttrace_core::otel for
            // the recorded input-token basis and snapshot date.
            "otel" => agenttrace_core::report_otel_export(&sessions),
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
            "svg" => report_overview_svg_with_context(
                &overview,
                &sessions,
                &health,
                range,
                args.include_history,
                args.card_theme,
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
        let out = dispatch_sanitize(&args.format, out);
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

/// rm-486: the --fail-* report gate, evaluated identically by every report
/// path that documents it (governance branch, --overview, and --compare).
/// Prints the failures plus local evidence to stderr and exits 2; returns
/// normally when the corpus passes.
fn enforce_report_gates(args: &Args, sessions: &[Session]) {
    let overview = compute_overview(sessions);
    let failures = evaluate_overview_gate(
        &overview,
        sessions,
        args.fail_under_health,
        args.fail_on_critical,
        args.max_tool_fail_rate,
    );
    if failures.is_empty() {
        return;
    }
    for failure in failures {
        eprintln!("Gate failed: {failure}");
    }
    eprintln!("Local evidence:");
    eprintln!("- avg health: {:.1}", average_health(sessions));
    eprintln!("- critical sessions: {}", overview.critical);
    eprintln!("- tool fail rate: {:.1}%", tool_fail_rate(sessions));
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
            governance_inspect_flag(args)
        )
    } else if let Some(dir) = args.dir.as_deref() {
        format!(
            "agenttrace -d {:?} {} -f json",
            dir,
            governance_inspect_flag(args)
        )
    } else {
        format!("agenttrace {} -f json", governance_inspect_flag(args))
    };
    eprintln!("- inspect: `{inspect}`");
    std::process::exit(2);
}

fn write_stderr(value: &str) -> anyhow::Result<()> {
    match io::stderr().write_all(value.as_bytes()) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(Into::into),
    }
}

fn write_stdout(value: &str) -> anyhow::Result<()> {
    // rm-599 rider: text stdout lanes end with a trailing newline
    // exactly like their `-o` file twins (whose callers append "\n").
    // Idempotent — values that already end in a newline are written
    // verbatim, so machine documents gain at most the one standard
    // trailing newline.
    let out = if value.ends_with('\n') {
        std::borrow::Cow::Borrowed(value)
    } else {
        std::borrow::Cow::Owned(format!("{value}\n"))
    };
    match io::stdout().write_all(out.as_bytes()) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(Into::into),
    }
}

/// rm-625: the output-dispatch choke point. Every report emission for
/// non-JSON formats passes through here at the stdout / `-o` boundary —
/// one place where raw control bytes from transcript-derived names
/// (session names, model ids, tool names, message text) die, regardless
/// of which renderer produced the document. JSON is excluded by
/// design: serde escaping already encodes control bytes losslessly
/// (data integrity over terminal trust), and re-processing a serialized
/// document could corrupt payloads. Renderer-internal escapes (HTML
/// entities, the rm-540 CSV cell sanitizer) stay as defense-in-depth:
/// `sanitize_output_document` is idempotent, so composed output is
/// stable.
fn dispatch_sanitize(format: &str, out: String) -> String {
    if format == "json" {
        out
    } else {
        agenttrace_core::sanitize_output_document(&out)
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
        // rm-505: `statusline`, `upstream`, and `mcp` are keyword host
        // commands (dispatched in run()), not session paths. `--help`
        // following them used to die here as a "flag follows the
        // positional session path" error with no help route at all —
        // even though `--help` itself teaches the keywords. Give each
        // keyword a real help route, and a keyword-scoped usage error
        // instead of the mislabel for other dropped flags.
        let positional = out.last().map(|arg| arg.to_string_lossy().to_string());
        if positional.as_deref() == Some("statusline")
            || positional.as_deref() == Some("upstream")
            || positional.as_deref() == Some("mcp")
        {
            let keyword = positional.unwrap_or_default();
            if dropped.iter().any(|arg| arg == "--help" || arg == "-h") {
                write_stdout(&keyword_help_text(&keyword))?;
                std::process::exit(0);
            }
            bail!(
                "flag `{}` follows the `{keyword}` keyword and would be silently dropped \
                 (dropped: `{tail}`); `{keyword}` takes no flags after it — place flags \
                 before the keyword, or run `agenttrace {keyword} --help`",
                flag.to_string_lossy()
            );
        }
        bail!(
            "flag `{}` follows the positional session path and would be silently dropped \
             (dropped: `{tail}`); place flags before the positional path",
            flag.to_string_lossy()
        );
    }

    Ok(out)
}

/// Per-keyword help (rm-505): the host keywords dispatch as the
/// positional path slot, so clap never renders help for them. Keep the
/// text aligned with the flag doc comments in `Args` and the guides.
fn keyword_help_text(keyword: &str) -> String {
    match keyword {
        "statusline" => concat!(
            "`agenttrace statusline` — Claude Code statusLine host command\n",
            "\n",
            "Reads the Claude Code statusline payload from stdin, renders\n",
            "one status line for the host terminal, and appends a capture\n",
            "record to the statusline journal (report on that journal with\n",
            "--statusline-report). Designed to be wired into Claude Code's\n",
            "statusLine setting; it must never fail the host — on any problem\n",
            "it prints a minimal fallback line and exits 0 (diagnostics go to\n",
            "stderr only).\n",
            "\n",
            "This keyword takes no flags. Related surface:\n",
            "  --statusline-report   report on the capture journal instead of\n",
            "                        hosting the status line (place it BEFORE\n",
            "                        the keyword; flags after it are rejected)\n",
            "\n",
            "See docs/guides/statusline-capture.md.\n",
        ),
        "upstream" => concat!(
            "`agenttrace upstream` — repository status host command (rm-024)\n",
            "\n",
            "Reports fork-vs-upstream drift: ahead/behind/diverged counts,\n",
            "new upstream releases, and refs age. Fully offline by default;\n",
            "pass --fetch to refresh remote-tracking refs (git fetch) and\n",
            "probe the npm registry first. Place flags BEFORE the keyword;\n",
            "flags after it are rejected.\n",
            "\n",
            "Examples:\n",
            "  agenttrace upstream\n",
            "  agenttrace -f json upstream\n",
            "  agenttrace --fetch upstream\n",
            "\n",
            "See docs/guides/upstream-status.md.\n",
        ),
        "mcp" => concat!(
            "`agenttrace mcp` — local read-only MCP server (rm-455)\n",
            "\n",
            "Speaks newline-delimited JSON-RPC 2.0 over stdio so a coding\n",
            "agent can ask about its own token usage: initialize, ping,\n",
            "tools/list, tools/call. Two read-only tools render the same\n",
            "locally discovered sessions as the CLI — usage_overview (the\n",
            "--overview -f json document) and by_model_breakdown (cost and\n",
            "sessions per model). The server opens no sockets, performs no\n",
            "network probes, and writes nothing outside the agenttrace\n",
            "session cache; it serves until stdin closes. Note: this\n",
            "keyword is unrelated to the --mcp-governance flag, which\n",
            "filters MCP governance findings inside reports.\n",
            "\n",
            "This keyword takes no flags; flags after it are rejected.\n",
            "\n",
            "See docs/guides/mcp-server.md.\n",
        ),
        _ => "",
    }
    .to_string()
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
            | "--card-theme"
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
            // rm-384: the config-surface flags all take a PATH/value;
            // without them here their value is mistaken for the
            // positional session path and every following flag is
            // rejected as dropped (`--config x.toml --doctor` failed).
            | "--config"
            | "--history-dir"
            | "--pricing-file"
            | "--weekly-budget"
    )
}

fn latest_session(sessions: &[Session]) -> Option<&Session> {
    // rm-688: the mtime fallback stats the file on every comparison, so a
    // --latest selection over n mixed-presence sessions stat'ed each
    // fallback file up to 2·(n−1) times for one answer. One memo per
    // selection pass — every file is stat'ed at most once, and missing
    // files still resolve to the epoch fallback (their "stat" is cached
    // like any other).
    let mut mod_times: std::collections::HashMap<&Path, SystemTime> =
        std::collections::HashMap::with_capacity(sessions.len());
    sessions
        .iter()
        .max_by(|a, b| newer_session_order(a, b, &mut mod_times))
}

fn newer_session_order<'a>(
    a: &'a Session,
    b: &'a Session,
    mod_times: &mut std::collections::HashMap<&'a Path, SystemTime>,
) -> std::cmp::Ordering {
    // rm-785: recency is judged on the PARSED instant — the shared
    // session_start_cmp basis also used by `--sort recent` and the TUI
    // SortKey::Recent comparator — so offset-bearing spellings
    // (`…+09:00` vs `…Z`) order correctly instead of by raw byte value.
    // A session without a usable timestamp loses to any dated one (that
    // presence split is inside session_start_cmp, replacing the old
    // has-session_start pre-compare); two undated sessions fall back to
    // file mod time, then the path tie-break.
    //
    // rm-688 (composed at integration): the mtime fallback below is the
    // stat-storm site — both stats go through the ONE memo threaded by
    // the caller (`latest_session`), so each fallback file is stat'ed at
    // most once per selection pass, and missing files still resolve to
    // the epoch fallback (their "stat" is cached like any other).
    session_start_cmp(&a.metrics.session_start, &b.metrics.session_start)
        .then_with(|| {
            if a.metrics.session_start.is_empty() && b.metrics.session_start.is_empty() {
                // Sequential binds: the map is borrowed mutably per stat,
                // and both stats share the one memo.
                let a_mod_time = session_mod_time(a, mod_times);
                let b_mod_time = session_mod_time(b, mod_times);
                a_mod_time.cmp(&b_mod_time)
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .then_with(|| a.path.cmp(&b.path))
}

fn session_mod_time<'a>(
    session: &'a Session,
    mod_times: &mut std::collections::HashMap<&'a Path, SystemTime>,
) -> SystemTime {
    // One stat per distinct path per selection pass (rm-688); the raw
    // stat stays here so the memo and the epoch fallback are one code
    // path, not two that can drift.
    *mod_times
        .entry(Path::new(&session.path))
        .or_insert_with(|| {
            fs::metadata(&session.path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
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

/// rm-735: `--demo` is itself a session source — it substitutes the
/// bundled demo corpus for discovery. Combined with an explicit
/// session source (a positional path, `-d/--dir`, or a `--range`
/// other than the `all` default) it used to silently discard the
/// explicit source and serve the demo corpus instead (`--demo
/// --overview -d /real/logs` rendered demo numbers as if they were
/// the user's, assess F2, run 06cc5c7d). The combination now bails
/// loudly, naming every ignored source flag, so the substitution can
/// never be silent. Explicit `--range all` is indistinguishable from
/// the default (clap `default_value`) and stays legal — an explicit
/// "all" discards no distinct intent. `--doctor`'s demo lane is
/// covered too: since landed rm-596 `--doctor --demo` renders ONLY
/// the bundled demo corpus, so a `-d` beside `--demo` would be
/// silently discarded there — the launch-guard call at the top of
/// main fires before the doctor dispatch. The host commands
/// (`statusline`, `upstream`, `mcp`) dispatch before any session
/// loading and stay outside this guard.
fn reject_demo_source_conflict(args: &Args) -> anyhow::Result<()> {
    // Self-gated: the launch call site runs on every invocation, so
    // a run without --demo (e.g. plain `-d X --range 30d`) must pass
    // through untouched — only the demo substitution is in scope.
    if !args.demo {
        return Ok(());
    }
    let mut sources = Vec::new();
    if args.path.is_some() {
        sources.push("a positional path");
    }
    if args.dir.is_some() {
        sources.push("-d/--dir");
    }
    if args.range != "all" {
        sources.push("--range");
    }
    if sources.is_empty() {
        return Ok(());
    }
    bail!(
        "--demo ignores explicit session sources ({}): drop --demo to analyze the \
         explicit source, or drop the source flags to explore the demo corpus",
        sources.join(", ")
    );
}

/// Cycle-4 B2 guard, shared by the report lanes and (via rm-608)
/// `--doctor`: a typo'd path used to produce the same "No session
/// files found in …" error as a genuinely empty directory — the two
/// need different messages and different exit codes (2 = the request
/// itself is wrong; 1 = nothing matched). `--doctor` previously
/// skipped this entirely. The TUI launch path still consumes `-d`
/// without this guard (rm-608 review residual; next-cycle rider).
fn validate_explicit_dir(dir_arg: Option<&str>) {
    let Some(dir) = dir_arg else {
        return;
    };
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

fn load_sessions_report(args: &Args) -> anyhow::Result<(Vec<Session>, Option<LoadReport>)> {
    if args.demo {
        // rm-735: the demo substitution must never silently swallow an
        // explicit session source (see reject_demo_source_conflict).
        reject_demo_source_conflict(args)?;
        return Ok((prepare_explicit_sessions(demo_sessions()?, args)?, None));
    }
    if let Some(path) = args.path.as_deref() {
        // rm-503: `-` reads ONE session stream from stdin through the
        // same decode/parse path as a named file — only the byte source
        // differs. stdin sessions are ephemeral: like every explicit
        // single-file load they never touch the session cache (neither
        // read nor write), which `--help` discloses. `-d` stays a
        // separate multi-session lane and is out of scope here.
        if path == "-" {
            let mut raw = Vec::new();
            io::stdin()
                .read_to_end(&mut raw)
                .context("read session from stdin")?;
            return Ok((
                prepare_explicit_sessions(vec![parse_stdin_bytes(raw)?], args)?,
                None,
            ));
        }
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
        // rm-212 + review fix (2026-10-06): report the true stat class
        // instead of claiming the path is missing, and admit symlinks that
        // resolve to a readable file or directory (the discovery collectors
        // follow symlinks too) while naming a special-file target by its
        // real class instead of mislabeling every symlink "dangling".
        // rm-590 (run d65f72c7, landed at this merge) wanted the same
        // three-way split here via an inline exists()-but-not-regular arm;
        // this landed helper already provides it — and more (true stat
        // class, symlink resolution, dangling detection) — so the inline
        // arm is superseded and the distinct-words intent rides rm-212.
        admit_session_path(&path)?;
    }
    let dir = args.dir.as_deref().map(PathBuf::from);
    // Cycle-4 B2: a typo'd `-d` path used to produce the same
    // "No session files found in …" error as a genuinely empty
    // directory — the two need different messages and different exit
    // codes (2 = the request itself is wrong; 1 = nothing matched).
    validate_explicit_dir(args.dir.as_deref());
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
    // rm-753: SQLite-backed ingestion failures are disclosed, never
    // silently swallowed — a discovered-but-unreadable agent database
    // or a dropped session row prints one stderr note so report runs
    // cannot mistake partial data for the whole corpus. stdout (and
    // -f json) stays clean; exit codes are unchanged.
    disclose_sqlite_ingest(&report.sqlite);
    disclose_unlinked_subagents(report.unlinked_subagents);
    if sessions.is_empty() {
        if report.discovered == 0 {
            match args.dir.as_deref() {
                Some(dir) => bail!(
                    "No session files found in {dir} (directory exists but holds no session files)"
                ),
                None => {
                    // rm-753: a discovered-but-unreadable agent database
                    // must not be reported as "nothing found" — the
                    // corpus is unreadable, not absent (P12/P14 used to
                    // exit with the plain message here).
                    if !report.sqlite.unreadable.is_empty() {
                        let listed = report
                            .sqlite
                            .unreadable
                            .iter()
                            .map(|db| format!("{} ({})", db.path.display(), db.reason))
                            .collect::<Vec<_>>()
                            .join(", ");
                        bail!(
                            "No session files found in any auto-discovered agent home, \
                             but {} agent database(s) were found and could not be read: {}",
                            report.sqlite.unreadable.len(),
                            listed
                        );
                    }
                    bail!("No session files found in any auto-discovered agent home");
                }
            }
        }
        // rm-835: discovery used to swallow parse failures at `.ok()`, so
        // an all-corrupt directory bailed here as "no sessions match the
        // requested filters" — hiding both the failure and the parser's
        // own remediation hint (e.g. the zstd-compressed rollout hint the
        // explicit-file lane already prints). Differentiate the two
        // stories the user can act on: everything failed to parse, or
        // some parsed and the filters dropped them (failures still
        // disclosed so the blind spot never comes back).
        if let Some(first) = report
            .first_parse_failure
            .as_deref()
            .filter(|_| report.parse_failures > 0)
        {
            if report.parse_failures == report.discovered {
                bail!(
                    "No sessions parsed: all {} discovered session files failed to parse (first: {})",
                    report.discovered,
                    first
                );
            }
            bail!(
                "No sessions match the requested filters ({} of {} discovered files failed to parse; first: {})",
                report.parse_failures,
                report.discovered,
                first
            );
        }
        bail!("No sessions match the requested filters");
    }
    if let Some(advisory) = parse_failure_advisory(&report) {
        eprintln!("agenttrace: warning: {advisory}");
    }
    // rm-534: discovery/-d-walk arm of the input-journal membership
    // check (`-o <name>.jsonl` colliding with any journal a walk
    // admitted) — loud BEFORE any bytes are staged. rm-912: the walk
    // lane also contributes its attempted-but-unparseable journals —
    // overwriting one would destroy the user's only copy.
    ensure_output_is_not_an_input(args, &sessions, &report.parse_failure_paths)?;
    Ok((sessions, Some(report)))
}

/// rm-799: one stderr line when subagent transcripts could not be
/// linked to a parent session — an orphaned child renders as a
/// standalone row (its own usage only) and is rolled into no parent,
/// which reads as "no subagent work happened" unless the count is
/// disclosed. Report output on stdout is unaffected.
fn disclose_unlinked_subagents(count: usize) {
    if count > 0 {
        eprintln!(
            "agenttrace: warning: {count} subagent transcript(s) have no parent session in the loaded corpus; they render as standalone rows and roll into no parent"
        );
    }
}

/// rm-753: one stderr line per SQLite-backed ingestion failure — a
/// discovered-but-unreadable database, or a session row dropped as
/// undecodable. Report output on stdout is unaffected.
fn disclose_sqlite_ingest(ingest: &agenttrace_core::SqliteIngestReport) {
    for db in &ingest.unreadable {
        eprintln!(
            "agenttrace: warning: agent database found but unreadable: {} ({})",
            db.path.display(),
            db.reason
        );
    }
    for db in &ingest.dropped_rows {
        eprintln!(
            "agenttrace: warning: {} session row(s) dropped as undecodable in {}: {}",
            db.dropped,
            db.path.display(),
            db.sample
        );
    }
}

/// rm-835 residual arm: the stderr advisory text for a load that
/// produced sessions while some discovered files failed to parse
/// (None when every discovered file parsed). Kept as a pure function so
/// the exact wording ships with a pinned test.
fn parse_failure_advisory(report: &LoadReport) -> Option<String> {
    if report.parse_failures == 0 {
        return None;
    }
    report.first_parse_failure.as_deref().map(|first| {
        format!(
            "{} of {} discovered session files failed to parse; first: {first}",
            report.parse_failures, report.discovered
        )
    })
}

/// rm-534: `--output` must never be an input journal. write_output
/// stages through unique-temp + rename (rm-250), so a destination that
/// resolves onto a just-loaded transcript atomically replaces it with
/// the rendered report while the process exits 0 (assess b6c398bf PoC:
/// `--overview -o X.jsonl X.jsonl` => RC=0, journal replaced by the
/// 35-line report). Membership is checked against the LOADED session
/// set — explicit positional files and every journal admitted by a `-d`
/// walk — with both sides resolved, so relative paths and symlinks to
/// live journals are caught too.
///
/// rm-912 (assess F2): the membership set also covers every path the
/// loader ATTEMPTED and failed to parse — a discovered-but-unparseable
/// journal (e.g. a UTF-16LE PowerShell redirect) used to be invisible
/// to this guard, so `--overview -d dir -o dir/bad.jsonl` exited 0 and
/// silently overwrote the unfixable input (only a `.orig` twin left
/// behind). Attempted-but-failed paths join the refusal set and the
/// error names the transcript.
fn ensure_output_is_not_an_input(
    args: &Args,
    sessions: &[Session],
    attempted_parse_failures: &[PathBuf],
) -> anyhow::Result<()> {
    let Some(output) = args.output.as_deref() else {
        return Ok(());
    };
    let resolved = resolve_output_destination(output);
    for session in sessions {
        if let Ok(source) = fs::canonicalize(&session.path) {
            if source == resolved {
                bail!(
                    "--output {} is an input session journal ({}); refusing to overwrite a transcript — choose a different --output destination",
                    output.display(),
                    session.path
                );
            }
        }
    }
    for candidate in attempted_parse_failures {
        if let Ok(source) = fs::canonicalize(candidate) {
            if source == resolved {
                bail!(
                    "--output {} is an input session journal that failed to parse ({}); refusing to overwrite a transcript — convert or move the journal, or choose a different --output destination",
                    output.display(),
                    candidate.display()
                );
            }
        }
    }
    Ok(())
}

/// Resolve an --output destination the way the write will land it: the
/// canonical path when it already exists (the collision case — inputs
/// exist by definition), otherwise the canonical parent plus the file
/// name. A destination whose parent does not exist yet cannot collide
/// with a loaded journal, so the relative fallback is safe.
fn resolve_output_destination(output: &Path) -> PathBuf {
    if let Ok(existing) = fs::canonicalize(output) {
        return existing;
    }
    let file_name = output.file_name().map(PathBuf::from).unwrap_or_default();
    match output
        .parent()
        .and_then(|parent| fs::canonicalize(parent).ok())
    {
        Some(parent) => parent.join(file_name),
        None => PathBuf::from(output),
    }
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
    // rm-534: reject --output destinations that ARE an input journal
    // before any lane work (explicit positional file + demo paths).
    // rm-912: the explicit lane has no attempted-parse-failure set — an
    // explicit positional file that fails to parse bails at `parse_file`
    // (non-zero, before any write), so nothing attempted-and-failed
    // can reach this guard.
    ensure_output_is_not_an_input(args, &sessions, &[])?;
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

fn write_private(path: &Path, content: &str) -> std::io::Result<()> {
    // rm-525: sibling of agenttrace-core's session_cache::write_private
    // (the rm-208 family). -o artifacts are session-derived — --search
    // hits quote user prompts verbatim — so PRIVACY.md's blanket
    // owner-only sentence must hold at this choke point too. The mode
    // is set on the TEMP file because rm-250's rename gives the
    // destination a fresh inode on every write: a plain `fs::write`
    // there let the umask (0664 under the common 0002) survive the
    // rename, and a rewrite over an older artifact resurrected
    // group/other read. Non-Unix keeps the plain write, mirroring the
    // rm-208 helper's fallback.
    //
    // rm-693: creation is EXCLUSIVE (O_EXCL) — callers stage through a
    // fresh `unique_temp_sibling`, so a pre-existing path at the
    // staging name (a planted symlink) is refused with `AlreadyExists`
    // instead of opened and truncated through; pair with
    // `write_private_exclusive` for the sequence-bump retry and rename.
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(content.as_bytes())
    }
    #[cfg(not(unix))]
    {
        fs::write(path, content)
    }
}

/// How many consecutive temp names `write_private_exclusive` tries
/// before refusing (rm-693) — mirrors the agenttrace-core staging
/// helper's bound. A genuine collision (a stale same-name temp)
/// clears on the first bump; a symlink-poisoned directory fails
/// honestly after this many tries instead of ever truncating through
/// an occupied path.
const EXCLUSIVE_STAGING_ATTEMPTS: u32 = 16;

/// rm-693: stage `content` into a fresh owner-only temp sibling of
/// `path` (O_EXCL) and rename it into place. An `AlreadyExists` at the
/// predictable `{name}.tmp.{pid}.{seq}` name — a planted symlink, or a
/// stale temp — bumps the sequence instead of truncating through the
/// existing path; a fully poisoned window is refused loudly. The
/// rename still hands the destination a fresh inode atomically, so a
/// crash mid-write can never tear it. Mirrors
/// `agenttrace_core::session_cache::write_private_exclusive` (kept
/// local because the core helper is `pub(crate)`).
fn write_private_exclusive(path: &Path, content: &str) -> std::io::Result<()> {
    for _ in 0..EXCLUSIVE_STAGING_ATTEMPTS {
        let temp = unique_temp_sibling(path);
        match write_private(&temp, content) {
            Ok(()) => match fs::rename(&temp, path) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    // rm-250's no-residue invariant: a failed rename
                    // (e.g. the destination is a directory) must not
                    // leave the staged temp behind.
                    let _ = fs::remove_file(&temp);
                    return Err(err);
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                let _ = fs::remove_file(&temp);
                return Err(err);
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        format!(
            "refusing to stage {}: {} consecutive temp names were already occupied \
             (planted symlinks or stale temps); nothing was written or truncated",
            path.display(),
            EXCLUSIVE_STAGING_ATTEMPTS
        ),
    ))
}

fn write_output(path: &Option<PathBuf>, content: &str) -> anyhow::Result<()> {
    if let Some(path) = path {
        // rm-250 residual (cycle 1): resolve the destination honestly
        // before anything is written. The temp+rename dance used to
        // silently replace a symlink with a regular file (the link's
        // target kept its old bytes) and could not write special
        // files at all. rm-525 rides the same staging point below:
        // the resolved regular-file lane stages through `write_private`,
        // so the owner-only guarantee survives the refactor.
        let target = resolve_output_target(path)?;
        write_output_resolved(path, &target, content)?;
    }
    Ok(())
}

/// Follow symlinks to the final destination. A dangling link resolves
/// to its missing target, which then gets created — the link is
/// honored instead of replaced. Cycles fail loudly after 32 hops.
fn resolve_output_target(path: &Path) -> anyhow::Result<PathBuf> {
    let mut target = path.to_path_buf();
    for _ in 0..32 {
        match fs::symlink_metadata(&target) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let link = fs::read_link(&target)?;
                target = match target.parent() {
                    Some(parent) => parent.join(link),
                    None => link,
                };
            }
            _ => return Ok(target),
        }
    }
    bail!(
        "resolving {}: symlink cycle detected (32 hops)",
        path.display()
    );
}

/// Write to the resolved destination. Regular files (and new paths)
/// keep the rm-250 atomic stage-and-rename; character devices take a
/// direct stream; fifos, sockets, and block devices are refused with
/// a disclosed reason instead of silently becoming regular files.
/// `-o` targets that alias our own stdout/stderr: streamed directly.
/// `/dev/stdout` under a pipeline resolves through `/proc/self/fd/1`
/// to an anonymous pipe inode — neither a regular file nor a char
/// device — so the generic lanes would refuse it or try to stage a
/// temp file under `/proc`. The aliases are intercepted up front and
/// written through the descriptor instead (assess U3c: `-o
/// /dev/stdout` must stream the report instead of failing).
fn is_stdout_alias(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some("/dev/stdout")
            | Some("/dev/stderr")
            | Some("/dev/fd/1")
            | Some("/dev/fd/2")
            | Some("/proc/self/fd/1")
            | Some("/proc/self/fd/2")
    )
}

fn write_output_resolved(requested: &Path, target: &Path, content: &str) -> anyhow::Result<()> {
    if is_stdout_alias(requested) {
        let mut handle = fs::File::create(requested)
            .with_context(|| format!("opening {}", requested.display()))?;
        handle.write_all(content.as_bytes())?;
        eprintln!("Saved: {} (streamed)", requested.display());
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if let Ok(meta) = fs::symlink_metadata(target) {
            let kind = meta.file_type();
            if kind.is_fifo() || kind.is_socket() || kind.is_block_device() {
                bail!(
                    "refusing to write -o {}: it is a fifo/socket/block device; \
                     writing would block or silently replace it — write to a \
                     regular file instead",
                    requested.display()
                );
            }
            if kind.is_char_device() {
                let mut handle = fs::File::create(target)
                    .with_context(|| format!("opening {}", target.display()))?;
                // rm-610 (rebased at integration onto this rm-489
                // write-through lane): a failed streamed write must
                // surface its cause through the SAME context site as
                // the atomic regular-file lane, so `-o /dev/full`
                // renders "writing report output file: No space left
                // on device" instead of the bare io error.
                handle
                    .write_all(content.as_bytes())
                    .context("writing report output file")?;
                eprintln!("Saved: {}", target.display());
                return Ok(());
            }
        }
    }
    if let Some(parent) = target.parent() {
        // rm-784 (landing composed with the landed rm-704 wording — the
        // second lander names more, not less): mkdir -p semantics for -o
        // parents are KEPT (documented in --help and README; do not break
        // mkdir -p users) — but the error names the directory that could
        // not be created for the requested target instead of surfacing a
        // bare OS errno ("Permission denied" from mkdir at /) with no
        // path attached, riding the rm-610 full-cause-chain handler.
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "creating parent directory {} for -o {}",
                parent.display(),
                target.display()
            )
        })?;
    }
    // rm-250: stage through a unique temp sibling and rename into
    // place, so a crash or Ctrl-C mid-write never leaves a
    // truncated report at the destination (same pattern as the
    // session-cache and history persistence writers). rm-525 (rebased
    // at integration onto this resolved-target lane): the stage goes
    // through `write_private`, not a plain `fs::write`, so the temp is
    // 0600 before the rename — the rename hands the destination a
    // fresh inode every time, which is what re-tightens a rewrite over
    // an artifact aged to 0664 (PRIVACY.md's report-artifact sentence).
    // rm-693: the stage is O_EXCL with a sequence bump — a symlink
    // planted at the predictable temp name is skipped past (or, in a
    // fully poisoned window, refused loudly), never truncated through,
    // and the victim behind it is never touched.
    write_private_exclusive(target, content)
        .map_err(|error| anyhow::anyhow!("writing report output file: {error}"))?;
    eprintln!("Saved: {}", target.display());
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
                    args.anomaly.eq_ignore_ascii_case("any")
                        || item
                            .kind
                            .to_ascii_lowercase()
                            .contains(&args.anomaly.to_ascii_lowercase())
                        || item
                            .detail
                            .to_ascii_lowercase()
                            .contains(&args.anomaly.to_ascii_lowercase())
                }))
    });
    let descending = match args.order.as_str() {
        "asc" => false,
        "desc" => true,
        _ => bail!("--order must be asc or desc"),
    };
    sessions.sort_by(|left, right| {
        let ordering = match args.sort.as_str() {
            "recent" | "time" => {
                // rm-785: parsed-instant basis (shared session_start_cmp),
                // not raw string order — mixed-offset corpora sort honestly.
                session_start_cmp(&left.metrics.session_start, &right.metrics.session_start)
            }
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
    let query = query.trim().to_ascii_lowercase();
    query.is_empty()
        || [
            session.name.as_str(),
            session.path.as_str(),
            session.cwd.as_str(),
            session.metrics.source_tool.as_str(),
            session.metrics.model_used.as_str(),
        ]
        .iter()
        .any(|value| value.to_ascii_lowercase().contains(&query))
        || session
            .metrics
            .tool_usage
            .keys()
            .chain(session.metrics.file_usage.keys())
            .any(|value| value.to_ascii_lowercase().contains(&query))
        || session.anomalies.iter().any(|item| {
            item.kind.to_ascii_lowercase().contains(&query)
                || item.detail.to_ascii_lowercase().contains(&query)
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
        bail!(
            "invalid --health filter: {} (expected a health tier, or an optional >=, <=, >, <, or = operator followed by a finite number; a bare number means >=)",
            args.health
        );
    }
    if !args.cost.is_empty() && !valid_number_filter(&args.cost) {
        bail!(
            "invalid --cost filter: {} (expected an optional >=, <=, >, <, or = operator followed by a finite number; a bare number means >=)",
            args.cost
        );
    }
    Ok(())
}

fn valid_number_filter(filter: &str) -> bool {
    // rm-389 remaining scope / rm-786: the CLI and TUI share the ONE core
    // dialect (agenttrace_core::filters) — an optional comparison operator
    // followed by a FINITE number, where a bare number means >=. The CLI
    // gate previously required an operator prefix (rejecting `--cost 1.5`)
    // while accepting non-finite thresholds like `>=nan` and `=1e400`, which
    // then silently matched nothing (or everything) instead of erroring.
    parse_numeric_filter(filter).is_some()
}

fn matches_number(value: f64, filter: &str) -> bool {
    matches_numeric_filter(value, filter)
}

fn render_session_list(sessions: &[Session], format: &str, limit: usize) -> String {
    // rm-798: --limit is a DISPLAY cap, and a capped machine format
    // must never read as the whole corpus: the json arm wraps the rows
    // with matched/returned/truncated counts, and the csv arm gets a
    // `# truncated:` marker row from sessions_csv_bounded.
    let matched = sessions.len();
    let sessions = sessions.iter().take(limit).collect::<Vec<_>>();
    if format == "json" {
        return serde_json::to_string_pretty(&serde_json::json!({
            "matched_sessions": matched,
            "returned_sessions": sessions.len(),
            "truncated": sessions.len() < matched,
            "limit": limit,
            "sessions": sessions,
        }))
        .expect("sessions serialize");
    }
    if format == "csv" {
        // rm-409: RFC 4180 statement export. rm-383 sanitation composes
        // BEFORE quoting (control bytes never ride into a cell), and the
        // rm-408 disclosure column rides along so zeros never read clean.
        // rm-855: subagent parity columns mirror the TSV's
        // SUBAGENTS/SUBAGENT_COST and the JSON parent_session field.
        let rows = sessions
            .iter()
            .map(|session| csv_export::SessionCsvRow {
                session: sanitize_line_segment(&session.name),
                health: session.health,
                data: session_capability(session).to_string(),
                source: sanitize_line_segment(&session.metrics.source_tool),
                model: sanitize_line_segment(&session.metrics.model_used),
                cost: session.metrics.cost_estimated,
                tokens: total_tokens(session),
                fail: session.metrics.tool_calls_fail,
                anomalies: session.anomalies.len(),
                zero_usage_events: session.metrics.zero_usage_events,
                subagents: session.metrics.subagent_count,
                subagent_cost: session.metrics.subagent_cost,
                parent_session: sanitize_line_segment(&session.metrics.parent_session),
            })
            .collect::<Vec<_>>();
        return csv_export::sessions_csv_bounded(&rows, matched, limit);
    }
    let mut lines = vec![
        "SESSION\tHEALTH\tDATA\tSOURCE\tMODEL\tCOST\tTOKENS\tFAIL\tANOMALIES\tSUBAGENTS\tSUBAGENT_COST"
            .to_string(),
    ];
    lines.extend(sessions.into_iter().map(|session| {
        // rm-383: name, source tool, and model are
        // transcript-derived; sanitize them for terminal display and TSV
        // row integrity (assess PoC: a crafted session name carried a raw
        // OSC-52 clipboard-write byte-for-byte into the --sessions TSV).
        // JSON output escapes control bytes losslessly and stays untouched.
        // rm-545: subagent rollups ride the TSV as their own columns so
        // one session's spawned work stays visible without double
        // counting — the columns are internally derived (not
        // transcript-derived), so no sanitization is owed here.
        format!(
            "{}\t{}\t{}\t{}\t{}\t{:.4}\t{}\t{}\t{}\t{}\t{:.4}",
            sanitize_line_segment(&session.name),
            session.health,
            session_capability(session),
            sanitize_line_segment(&session.metrics.source_tool),
            sanitize_line_segment(&session.metrics.model_used),
            session.metrics.cost_estimated,
            total_tokens(session),
            session.metrics.tool_calls_fail,
            session.anomalies.len(),
            session.metrics.subagent_count,
            session.metrics.subagent_cost
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

/// rm-301 (cycle-3 acceptance extension, assess 7276 A1/A2): do the
/// side-effect early exits (`--clear-cache`, `--update-pricing`) owe
/// the user a follow-up action? The two guards used to consult two
/// hand-maintained predicates that each knew only part of the action
/// surface — `has_session_action` omitted `--doctor`, `--list-models`,
/// and `--test-match`, and neither knew `--statusline-report` or
/// `--budget` — so `--clear-cache --doctor` (and four sibling combos)
/// printed "Session cache cleared." and exited 0 without ever running
/// the co-requested action. One shared helper now answers for BOTH
/// guards: every action arm `run()` dispatches between the side-effect
/// exits and the interactive fallback must appear here, and the unit
/// matrix in `mod tests` pins the full flag set so a new action arm
/// cannot be registered on one guard and not the other.
fn has_followup_action(args: &Args) -> bool {
    has_session_action(args)
        || args.list_models
        || args.test_match
        || args.doctor
        || args.statusline_report
        || args.budget
}

/// Session-report actions only — the interactive-TUI fallback and the
/// `--range` applicability guard (rm-244). Deliberately NARROWER than
/// `has_followup_action`: `--doctor`, `--list-models`, `--test-match`,
/// `--statusline-report`, and `--budget` are utility arms that return
/// before either consumer runs and that consume neither sessions nor
/// `--range`. The side-effect early exits must NOT use this predicate
/// (rm-301: that drift is what swallowed co-requested actions).
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
    // rm-836 + rm-533 (combined at integration of run d80f6a25): --demo
    // sessions are synthetic samples pinned to a fixed epoch;
    // --preserve-history would bank them into the user's durable
    // history.json and permanently inflate every later
    // --include-history report (rm-836 dossier PoC F3: $0.8106 of
    // fiction after one demo run, and it survives forever — demo
    // reports are supposed to be ephemeral like their cache behavior;
    // the rm-533 assess b6c398bf PoC independently measured 4 rows /
    // $0.8950 of fabricated rows landing in history.json and repeated
    // by every later history-consuming report). Refuse the pair loudly
    // instead of writing fiction, mirroring the --baseline/--compare
    // rule above: the demo lane stays hermetic — no path may write
    // demo state to host stores.
    if args.demo && args.preserve_history {
        bail!(
            "--demo cannot be combined with --preserve-history: demo sessions are ephemeral samples and never enter durable history"
        );
    }
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
        // rm-385 (integration review ad24bcb1): `--budget` is a report
        // action like `--statusline-report`, but it was never added to
        // this table, so `--budget --overview` (or `--budget --doctor`,
        // `--budget --statusline-report`) silently rendered only the
        // first branch's output and exited 0 with the other action
        // dropped — exactly the action-hijack class rm-246 landed to
        // close. Registered here so the pair is a loud usage error.
        args.budget,
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

/// rm-212: describe a non-regular, non-directory filesystem object for the
/// truthful admission error.
///
/// Admit a positional session path, or fail with the path's true class.
/// Written once, platform-generic (rm-733): the round-2 review fix
/// initially split this into a cfg(unix)/cfg(not(unix)) pair, but the
/// two bodies converged byte-identical — the only unix-specific piece
/// is the `special_file_kind` classifier just below, which keeps its own
/// cfg pair (it imports FileTypeExt) — so the duplicate arms, a drift
/// factory that broke non-unix builds in that very round, are collapsed
/// back into one body. Zero behavioral change; the unix bin-suite pins
/// behavior.
fn admit_session_path(path: &std::path::Path) -> anyhow::Result<()> {
    if path.as_os_str() == "-" {
        bail!("invalid session path '-': read from stdin is not supported");
    }
    if path.is_file() || path.is_dir() {
        return Ok(());
    }
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => match std::fs::metadata(path) {
            Ok(target) if target.is_file() || target.is_dir() => Ok(()),
            Ok(target) => bail!(
                "session path is not a regular file or directory: {} (symbolic link to {})",
                path.display(),
                special_file_kind(&target)
            ),
            Err(_) => bail!("session path is a dangling symlink: {}", path.display()),
        },
        Ok(meta) => bail!(
            "session path is not a regular file or directory: {} ({})",
            path.display(),
            special_file_kind(&meta)
        ),
        Err(_) => bail!("session path does not exist: {}", path.display()),
    }
}

// Review fix (2026-10-07, round 2 MF1): this variant imports
// std::os::unix::fs::FileTypeExt, so it needs its own #[cfg(unix)] — an
// un-gated definition is an E0433 break on non-unix targets even with the
// stubs above in place.
#[cfg(unix)]
fn special_file_kind(meta: &std::fs::Metadata) -> &'static str {
    use std::os::unix::fs::FileTypeExt;
    let file_type = meta.file_type();
    if file_type.is_char_device() {
        "character device"
    } else if file_type.is_fifo() {
        "fifo"
    } else if file_type.is_socket() {
        "socket"
    } else if file_type.is_block_device() {
        "block device"
    } else {
        "special file"
    }
}

#[cfg(not(unix))]
fn special_file_kind(_meta: &std::fs::Metadata) -> &'static str {
    "special file"
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
    // rm-389: a zero limit is invalid regardless of which action runs —
    // --limit 0 used to render empty output rc0, and a bare --search-limit 0
    // skipped the guard when --search was not set.
    if args.search_limit == 0 {
        bail!("--search-limit must be at least 1");
    }
    if args.limit == 0 {
        bail!("--limit must be at least 1");
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

    // rm-389 (cycle 1): dispatch-time validation for numeric knobs
    // that used to degrade silently regardless of the action being
    // dispatched. Table-driven so every rule stays uniform and each
    // error names its flag. INTEGRATION (2026-10-07, conflict case
    // 2f1198de, merge of run de96d4cc candidate dfc3b36 into HEAD
    // 1c5edd1): this table's original --limit row duplicated and its
    // --search-limit row (conditional on --sessions/--search) was
    // superseded by rm-389's completion above — zero limits are now
    // invalid regardless of the dispatched action — so both rows are
    // retired here rather than kept as a second, weaker source of
    // truth; --weekly-budget keeps its table row.
    let rows: Vec<(&str, bool, String)> = vec![(
        "--weekly-budget",
        args.weekly_budget
            .is_none_or(|value| value.is_finite() && value > 0.0),
        format!(
            "--weekly-budget must be a positive finite USD amount (got {})",
            args.weekly_budget.unwrap_or(f64::NAN)
        ),
    )];
    for (_, valid, message) in rows {
        if !valid {
            bail!("{message}");
        }
    }
    Ok(())
}

/// Test-visible Args base: every knob at its neutral default, with
/// the non-zero CLI defaults real invocations carry (json format,
/// limit 20, `recent`/`desc` ordering, `en`/`all`). Shared by the
/// main.rs tests and the config.rs precedence matrix so both build
/// Args the same way.
#[cfg(test)]
pub(crate) fn test_args(dir: Option<String>) -> Args {
    Args {
        path: None,
        format: "json".to_string(),
        // rm-576: every test args literal carries the SVG card theme
        // default (the flag's clap default); the helper is the single
        // construction site since the test_args refactor.
        card_theme: SvgCardTheme::Auto,
        config: None,
        history_dir: None,
        pricing_file: None,
        weekly_budget: None,
        budget: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use agenttrace_core::Metrics;
    use std::io::Write;

    #[test]
    fn has_followup_action_covers_every_action_arm_for_both_side_effect_exits() {
        // rm-301 (cycle-3 extension, assess 7276 A1/A2): `--clear-cache`
        // and `--update-pricing` share this predicate as their
        // clear-and-exit guard. The --update-pricing half of the e2e
        // swallow matrix is network-bound (it downloads the LiteLLM
        // table), so both halves are pinned here at the shared
        // predicate: every action arm `run()` dispatches must register,
        // and the side-effect flags themselves must not.
        let base = || {
            let mut args = test_args(None);
            args.compare = false;
            args
        };
        assert!(
            !has_followup_action(&base()),
            "no action flag: the side-effect exits keep their clear-and-exit contract"
        );
        let variants: Vec<(&str, Args)> = vec![
            ("<path>", {
                let mut a = base();
                a.path = Some("session.jsonl".to_string());
                a
            }),
            ("--latest", {
                let mut a = base();
                a.latest = true;
                a
            }),
            ("--compare", {
                let mut a = base();
                a.compare = true;
                a
            }),
            ("--audit", {
                let mut a = base();
                a.audit = true;
                a
            }),
            ("--recommend", {
                let mut a = base();
                a.recommend = true;
                a
            }),
            ("--mcp-governance", {
                let mut a = base();
                a.mcp_governance = true;
                a
            }),
            ("--context-trends", {
                let mut a = base();
                a.context_trends = true;
                a
            }),
            ("--delivery-evidence", {
                let mut a = base();
                a.delivery_evidence = true;
                a
            }),
            ("--overview", {
                let mut a = base();
                a.overview = true;
                a
            }),
            ("--sessions", {
                let mut a = base();
                a.sessions = true;
                a
            }),
            ("--diagnostics", {
                let mut a = base();
                a.diagnostics = true;
                a
            }),
            ("--inspect", {
                let mut a = base();
                a.inspect = Some(3);
                a
            }),
            ("--waste", {
                let mut a = base();
                a.waste = true;
                a
            }),
            ("--baseline", {
                let mut a = base();
                a.baseline = Some("base".to_string());
                a
            }),
            ("--search QUERY", {
                let mut a = base();
                a.search = Some("query".to_string());
                a
            }),
            ("--doctor", {
                let mut a = base();
                a.doctor = true;
                a
            }),
            ("--list-models", {
                let mut a = base();
                a.list_models = true;
                a
            }),
            ("--test-match", {
                let mut a = base();
                a.test_match = true;
                a
            }),
            ("--statusline-report", {
                let mut a = base();
                a.statusline_report = true;
                a
            }),
            ("--budget", {
                let mut a = base();
                a.budget = true;
                a
            }),
        ];
        for (flag, args) in &variants {
            assert!(
                has_followup_action(args),
                "{flag} must count as a follow-up action for BOTH side-effect guards"
            );
        }
        for (flag, args) in [
            ("--clear-cache", {
                let mut a = base();
                a.clear_cache = true;
                a
            }),
            ("--update-pricing", {
                let mut a = base();
                a.update_pricing = true;
                a
            }),
        ] {
            assert!(
                !has_followup_action(&args),
                "{flag} is a side effect, not a follow-up action"
            );
        }
        // The search lane keeps the shared non-empty rule.
        let mut whitespace = base();
        whitespace.search = Some("   ".to_string());
        assert!(!has_followup_action(&whitespace));
    }

    #[test]
    fn pricing_download_announcement_goes_to_stderr() {
        // rm-404, review finding 1 (0145eeb5): PRIVACY.md promises every
        // network touch is "announced on stderr at the moment it happens".
        // The --update-pricing download announcement must be an eprintln, not
        // a write_stdout — piped/redirected stdout must not swallow it.
        let main_src = include_str!("main.rs");
        assert!(
            main_src.contains("eprintln!(\"Downloading pricing from LiteLLM...\")"),
            "the pricing download announcement must go to stderr (PRIVACY.md)"
        );
        assert!(
            !main_src.contains("write_stdout(\"Downloading pricing"),
            "the pricing download announcement must not write to stdout"
        );
    }

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
    fn session_list_tsv_carries_subagent_columns() {
        // rm-487/rm-545 column contract: the --sessions TSV exposes
        // SUBAGENTS and SUBAGENT_COST so spawned work stays visible in
        // the default text view, and the rollup never bleeds into the
        // session's own COST/TOKENS cells.
        let mut session = Session {
            name: "parent".to_string(),
            path: "/tmp/parent.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                cost_estimated: 0.0105,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: agenttrace_core::Diagnostics::default(),
        };
        session.metrics.subagent_count = 2;
        session.metrics.subagent_cost = 0.0053;
        let tsv = render_session_list(std::slice::from_ref(&session), "tsv", 20);
        let header = tsv.lines().next().expect("header");
        assert!(
            header.contains("SUBAGENTS") && header.contains("SUBAGENT_COST"),
            "TSV header must carry the subagent columns: {header}"
        );
        let row = tsv.lines().nth(1).expect("data row");
        let cells: Vec<&str> = row.split('\t').collect();
        // Column order is pinned: …FAIL, ANOMALIES, SUBAGENTS, SUBAGENT_COST.
        assert_eq!(cells[cells.len() - 2], "2");
        assert_eq!(cells[cells.len() - 1], "0.0053");
        assert!(
            cells.contains(&"0.0105"),
            "own cost stays its own cell: {row}"
        );
    }

    #[test]
    fn session_list_json_discloses_limit_truncation() {
        // rm-798: the json arm must never render a bare array — a
        // `jq '. | length' == 20` consumer cannot tell a capped list
        // from the whole corpus. The wrapper carries matched/returned/
        // truncated/limit and the rows under "sessions".
        let sessions: Vec<Session> = (0..5)
            .map(|i| Session {
                name: format!("s{i}"),
                path: format!("/tmp/s{i}.jsonl"),
                cwd: String::new(),
                branch: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: agenttrace_core::Diagnostics::default(),
            })
            .collect();
        let full = render_session_list(&sessions, "json", 20);
        let payload = serde_json::from_str::<serde_json::Value>(&full).expect("valid json");
        assert_eq!(payload["matched_sessions"], 5);
        assert_eq!(payload["returned_sessions"], 5);
        assert_eq!(payload["truncated"], false);
        assert_eq!(payload["limit"], 20);
        assert_eq!(payload["sessions"].as_array().map(Vec::len), Some(5));
        let capped = render_session_list(&sessions, "json", 2);
        let payload = serde_json::from_str::<serde_json::Value>(&capped).expect("valid json");
        assert_eq!(payload["matched_sessions"], 5);
        assert_eq!(payload["returned_sessions"], 2);
        assert_eq!(payload["truncated"], true);
        assert_eq!(payload["limit"], 2);
        assert_eq!(payload["sessions"].as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn session_list_csv_carries_subagent_parity_and_truncation() {
        // rm-855: csv parity — subagents/subagent_cost/parent_session
        // mirror the TSV rollup columns and the JSON parent_session
        // field, so no format hides what another shows. rm-798: a
        // capped csv gets a `# truncated:` marker row; an uncapped one
        // does not.
        let mut parent = Session {
            name: "parent".to_string(),
            path: "/tmp/parent.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                cost_estimated: 0.0105,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: agenttrace_core::Diagnostics::default(),
        };
        parent.metrics.subagent_count = 2;
        parent.metrics.subagent_cost = 0.0053;
        let mut child = parent.clone();
        child.name = "child".to_string();
        child.path = "/tmp/parent/subagents/agent-a1.jsonl".to_string();
        child.metrics.subagent_count = 0;
        child.metrics.subagent_cost = 0.0;
        child.metrics.parent_session = "/tmp/parent.jsonl".to_string();
        let csv = render_session_list(&[parent.clone(), child.clone()], "csv", 20);
        let header = csv.lines().nth(1).expect("header");
        assert!(
            header.ends_with("zero_usage_events,subagents,subagent_cost,parent_session"),
            "csv header must carry the rm-855 columns: {header}"
        );
        let parent_row = csv.lines().nth(2).expect("parent row");
        let cells: Vec<&str> = parent_row.split(',').collect();
        assert_eq!(cells[cells.len() - 3], "2");
        assert_eq!(cells[cells.len() - 2], "0.0053");
        assert_eq!(cells[cells.len() - 1], "");
        let child_row = csv.lines().nth(3).expect("child row");
        let cells: Vec<&str> = child_row.split(',').collect();
        assert_eq!(cells[cells.len() - 1], "/tmp/parent.jsonl");
        assert!(!csv.contains("# truncated"));
        let capped = render_session_list(&[parent, child], "csv", 1);
        assert!(
            capped
                .lines()
                .any(|line| line == "# truncated: showing 1 of 2 matching sessions (--limit 1)"),
            "capped csv must disclose: {capped}"
        );
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
            branch: String::new(),
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

    // rm-688: one stat per distinct fallback path per selection pass.
    // The pre-memo comparator stat'ed both operands on every comparison —
    // 2·C(n,2) stats over a full ordering — so a --latest selection over a
    // large mixed-presence history was a stat storm. The pin: drive the
    // comparator pairwise across every session pair with one shared memo
    // and require the memo to hold exactly one entry per fallback path,
    // each equal to a fresh stat.
    #[test]
    fn latest_session_selection_stats_each_fallback_file_once() {
        let mut files = Vec::new();
        for index in 0..6 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            files.push(temp_session_file(
                &format!("agenttrace-stat-pin-{index}"),
                "stat-pin",
            ));
        }
        // Mixed presence: two sessions carry session_start (no stat needed),
        // five fall back to mtime, one of which points at a missing file
        // (the epoch fallback is memoized like any other stat).
        let sessions = vec![
            session("stamped-a", &files[0], "2026-01-01T00:00:00Z"),
            session("stamped-b", &files[1], "2026-01-02T00:00:00Z"),
            session("file-2", &files[2], ""),
            session("file-3", &files[3], ""),
            session("file-4", &files[4], ""),
            session("file-5", &files[5], ""),
            session("shared-path", &files[2], ""),
            session_with_missing_file("missing", "/tmp/agenttrace-stat-pin-absent.jsonl"),
        ];
        let mut mod_times: std::collections::HashMap<&Path, SystemTime> =
            std::collections::HashMap::new();
        for a in &sessions {
            for b in &sessions {
                let _ = newer_session_order(a, b, &mut mod_times);
            }
        }
        // 4 distinct real fallback paths + 1 missing path, one entry each
        // — stamped paths never enter the memo, and the shared path is
        // stat'ed once, not once per pair that touches it.
        assert_eq!(mod_times.len(), 5, "one memo entry per fallback path");
        for (path, memoized) in &mod_times {
            let fresh = fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            assert_eq!(memoized, &fresh, "memo holds a live stat for {path:?}");
        }
        for file in files {
            let _ = fs::remove_file(file);
        }
    }

    // rm-688 parity: the memo changes how often files are stat'ed, never
    // which session wins. Mixed-presence selection must match an unmemoized
    // recomputation, including when a file disappears mid-pass (the memo
    // then keeps that session's earlier position instead of re-stating).
    #[test]
    fn latest_session_mixed_presence_selection_matches_unmemoized() {
        let mut files = Vec::new();
        for index in 0..4 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            files.push(temp_session_file(
                &format!("agenttrace-parity-{index}"),
                "parity",
            ));
        }
        let sessions = vec![
            session("file-0", &files[0], ""),
            session("file-1", &files[1], ""),
            session("stamped", &files[2], "2026-01-01T00:00:00Z"),
            session("file-3", &files[3], ""),
            session_with_missing_file("gone", "/tmp/agenttrace-parity-absent.jsonl"),
        ];
        let expected: Vec<String> = {
            // Unmemoized oracle: sort by the same precedence with a fresh
            // stat per comparison (the pre-rm-688 behavior).
            let mut ordered = sessions.clone();
            ordered.sort_by(|a, b| {
                let a_stamp = !a.metrics.session_start.is_empty();
                let b_stamp = !b.metrics.session_start.is_empty();
                a_stamp
                    .cmp(&b_stamp)
                    .then_with(|| {
                        if a_stamp && b_stamp {
                            a.metrics.session_start.cmp(&b.metrics.session_start)
                        } else {
                            let a_time = fs::metadata(&a.path)
                                .and_then(|metadata| metadata.modified())
                                .unwrap_or(SystemTime::UNIX_EPOCH);
                            let b_time = fs::metadata(&b.path)
                                .and_then(|metadata| metadata.modified())
                                .unwrap_or(SystemTime::UNIX_EPOCH);
                            a_time.cmp(&b_time)
                        }
                    })
                    .then_with(|| a.path.cmp(&b.path))
            });
            ordered.iter().rev().map(|s| s.name.clone()).collect()
        };
        // max_by returns the greatest by the same ordering the oracle sorts
        // ascending — assert the winner, and the full precedence (stamped
        // above all mtime sessions) via repeated selection.
        assert_eq!(
            latest_session(&sessions).map(|session| session.name.clone()),
            expected.first().cloned(),
            "memoized selection disagrees with the unmemoized oracle"
        );
        assert_eq!(
            latest_session(&sessions).map(|session| session.name.clone()),
            Some("stamped".to_string()),
            "stamped sessions outrank every mtime fallback"
        );
        for file in files {
            let _ = fs::remove_file(file);
        }
    }

    #[test]
    fn latest_session_orders_mixed_offset_timestamps_by_instant() {
        // rm-785: "2026-01-02T01:30:00Z" and "2026-01-02T10:30:00+09:00" are
        // the same instant, and a strictly-later Z timestamp must win the
        // --latest pick even though its raw STRING sorts before the +09:00
        // spelling (the raw comparator used to pick the earlier session).
        let later = session(
            "later",
            "/tmp/agenttrace-later-mixed",
            "2026-01-02T05:00:00Z",
        );
        let offset = session(
            "offset-spelling",
            "/tmp/agenttrace-offset-mixed",
            "2026-01-02T10:30:00+09:00",
        );

        assert_eq!(
            latest_session(&[later.clone(), offset.clone()]).map(|session| session.name.as_str()),
            Some("later")
        );
        assert_eq!(
            latest_session(&[offset, later]).map(|session| session.name.as_str()),
            Some("later")
        );
    }

    #[test]
    fn sort_recent_orders_mixed_offset_corpus_by_instant() {
        // rm-785: --sort recent uses the parsed-instant basis (shared
        // session_start_cmp), so the mixed-offset corpus orders newest-first
        // exactly like canonical_sessions regardless of +09:00 spelling;
        // equal instants keep stable input order, absent sorts last.
        let mut args = compare_args(None);
        args.sessions = true;
        args.sort = "recent".to_string();
        args.order = "desc".to_string();
        let sessions = vec![
            session("absent", "/tmp/agenttrace-absent", ""),
            session("z-morning", "/tmp/agenttrace-zm", "2026-01-02T01:30:00Z"),
            session(
                "plus-nine",
                "/tmp/agenttrace-pn",
                "2026-01-02T10:30:00+09:00",
            ),
            session("z-later", "/tmp/agenttrace-zl", "2026-01-02T05:00:00Z"),
        ];
        let ordered = prepare_cli_view(sessions, &args).expect("sort");
        let names: Vec<&str> = ordered.iter().map(|s| s.name.as_str()).collect();
        // z-later newest; z-morning and plus-nine are the SAME instant, and
        // the view's canonical pre-pass (name tie-break) orders the tie
        // deterministically regardless of input order; absent last.
        assert_eq!(names, vec!["z-later", "plus-nine", "z-morning", "absent"]);
    }

    #[test]
    fn number_filter_gate_accepts_bare_and_rejects_nonfinite() {
        // rm-389 remaining scope / rm-786: ONE finite-only dialect shared by
        // the CLI and the TUI. The CLI gate previously required an operator
        // prefix (rejecting `--cost 1.5`) while accepting `>=nan` / `=1e400`,
        // which then silently matched nothing or everything.
        assert!(valid_number_filter("1.5"));
        assert!(valid_number_filter(">=1.5"));
        assert!(valid_number_filter(">= 1.5"));
        for bad in [
            ">=nan", "<=inf", ">=inf", "=1e400", "1e400", "abc", "", ">=", "nan",
        ] {
            assert!(!valid_number_filter(bad), "{bad:?} must be rejected");
        }
        assert!(matches_number(1.5, "1.5"));
        assert!(matches_number(2.0, ">1.5"));
        assert!(!matches_number(2.0, ">=nan"));
        assert!(!matches_number(2.0, "<=inf"));
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
    fn budget_is_a_report_action_and_conflicts_loudly() {
        // rm-385 / integration review ad24bcb1: `--budget` dispatches as
        // its own report branch, so it must be a member of the
        // action-exclusivity table. Before it was registered,
        // `--budget --overview` (and `--budget --doctor`,
        // `--budget --statusline-report`) rendered only the branch that
        // ran first and exited 0 with the other action silently
        // dropped — the rm-246 action-hijack class.
        let mut args = test_args(None);
        args.compare = false;
        args.budget = true;
        assert!(validate_primary_action(&args).is_ok(), "--budget alone");

        for flag in [
            "--overview",
            "--doctor",
            "--statusline-report",
            "--sessions",
            "--waste",
        ] {
            let mut args = test_args(None);
            args.compare = false;
            args.budget = true;
            match flag {
                "--overview" => args.overview = true,
                "--doctor" => args.doctor = true,
                "--statusline-report" => args.statusline_report = true,
                "--sessions" => args.sessions = true,
                "--waste" => args.waste = true,
                _ => unreachable!("unhandled arm: {flag}"),
            }
            let error = validate_primary_action(&args)
                .expect_err("--budget must conflict with every other report action")
                .to_string();
            assert_eq!(error, "choose exactly one report action", "{flag}");
        }
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
    #[cfg(unix)]
    fn write_output_creates_owner_only_artifacts() {
        // rm-525: the -o artifact class is session-derived (--search
        // hits quote user prompts verbatim), so PRIVACY.md's blanket
        // 0600 sentence holds at the write_output choke point — fresh
        // writes AND rewrites: rm-250's rename replaces the inode, so
        // the temp must already be 0600 and a rewrite over a file aged
        // to 0664 (by an older build or a deliberate umask) must not
        // resurrect group/other read.
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("at-write-output-mode-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("scratch dir");
        let target = dir.join("report.json");
        write_output(&Some(target.clone()), "{}\n").expect("write -o artifact");
        let fresh = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            fresh, 0o600,
            "fresh -o artifact must be owner-only (got {fresh:o})"
        );
        fs::set_permissions(&target, fs::Permissions::from_mode(0o664)).expect("age the mode");
        write_output(&Some(target.clone()), "{}\n").expect("rewrite -o artifact");
        let rewritten = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            rewritten, 0o600,
            "a rewrite must not resurrect 0664 (got {rewritten:o})"
        );
        fs::remove_dir_all(&dir).ok();
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

    #[cfg(unix)]
    #[test]
    fn write_output_honors_symlinks_instead_of_replacing_them() {
        // rm-250 residual (cycle 1): -o through a symlink must update
        // the link's target and keep the link a link — the assess PoC
        // showed the link itself silently replaced by a regular file
        // while the target kept its original bytes. A dangling link
        // resolves to its missing target, which is created.
        let dir =
            std::env::temp_dir().join(format!("agenttrace-write-link-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("scratch dir");
        let real = dir.join("real");
        fs::write(&real, "original\n").unwrap();
        let link = dir.join("link.md");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        write_output(&Some(link.clone()), "new bytes\n").expect("write via link");
        assert_eq!(fs::read_to_string(&real).unwrap(), "new bytes\n");
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink(),
            "link must stay a link"
        );

        let dangling = dir.join("dangling.md");
        std::os::unix::fs::symlink(dir.join("missing.md"), &dangling).unwrap();
        write_output(&Some(dangling.clone()), "created\n").expect("via dangling link");
        assert_eq!(
            fs::read_to_string(dir.join("missing.md")).unwrap(),
            "created\n"
        );
        assert!(fs::symlink_metadata(&dangling)
            .unwrap()
            .file_type()
            .is_symlink());

        let a = dir.join("cycle-a");
        let b = dir.join("cycle-b");
        std::os::unix::fs::symlink(&a, &b).unwrap();
        std::os::unix::fs::symlink(&b, &a).unwrap();
        let error = write_output(&Some(a), "x\n").unwrap_err().to_string();
        assert!(error.contains("symlink cycle"), "{error}");

        fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn write_output_refuses_preplanted_symlinks_at_the_staged_temp() {
        // rm-693: the -o lane staged through create+truncate on a
        // predictable `{name}.tmp.{pid}.{seq}` sibling — a symlink
        // planted at that name was FOLLOWED (the victim clobbered
        // through the link) and the rename left the destination AS the
        // link. Staging must be O_EXCL with a sequence bump: one
        // planted name is skipped past, a fully poisoned window fails
        // loudly, and the victim survives both.
        let dir =
            std::env::temp_dir().join(format!("agenttrace-write-excl-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("scratch dir");
        let victim = dir.join("victim.txt");
        fs::write(&victim, "secret bytes\n").expect("write victim");

        // One planted link at the next temp name: the write bumps the
        // sequence and succeeds, leaving the victim intact.
        let target = dir.join("report.json");
        let planted = unique_temp_sibling(&target);
        std::os::unix::fs::symlink(&victim, &planted).expect("plant symlink");
        write_output(&Some(target.clone()), "body\n").expect("write past one planted link");
        assert_eq!(fs::read_to_string(&target).unwrap(), "body\n");
        assert_eq!(
            fs::read_to_string(&victim).unwrap(),
            "secret bytes\n",
            "victim behind the planted link must be untouched"
        );
        assert!(
            fs::symlink_metadata(&target).unwrap().file_type().is_file(),
            "the destination must be a regular file, never the link"
        );

        // Fully poisoned window: every candidate temp name is a link —
        // the write must fail loudly instead of following any of them.
        let poisoned = dir.join("poisoned.json");
        let seq_of = |name: &Path| {
            name.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.rsplit('.').next())
                .and_then(|name| name.parse::<u64>().ok())
                .expect("temp name carries a numeric sequence")
        };
        let mut last_seq = 0u64;
        for _ in 0..64 {
            let name = unique_temp_sibling(&poisoned);
            last_seq = seq_of(&name);
            std::os::unix::fs::symlink(&victim, &name).expect("plant probe symlink");
        }
        for seq in (last_seq + 1)..=(last_seq + 64) {
            let name = dir.join(format!("poisoned.json.tmp.{}.{}", std::process::id(), seq));
            std::os::unix::fs::symlink(&victim, &name).expect("plant forward symlink");
        }
        let error = write_output(&Some(poisoned.clone()), "body\n")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("writing report output file"),
            "the -o write context must lead: {error}"
        );
        assert!(
            error.contains("refusing to stage"),
            "the refusal must say so: {error}"
        );
        assert!(!poisoned.exists(), "no destination may materialize");
        assert_eq!(
            fs::read_to_string(&victim).unwrap(),
            "secret bytes\n",
            "victim behind every planted link must be untouched"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_output_refuses_special_files_instead_of_materializing_them() {
        // rm-250 residual (cycle 1): a fifo destination is refused with
        // a disclosed reason (the assess PoC showed "Saved:" rc0 while
        // the fifo silently became a regular file); character devices
        // like /dev/null take a direct stream.
        let dir =
            std::env::temp_dir().join(format!("agenttrace-write-fifo-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("scratch dir");
        let fifo = dir.join("fifo");
        assert!(std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo")
            .success());
        let error = write_output(&Some(fifo.clone()), "x\n")
            .unwrap_err()
            .to_string();
        assert!(error.contains("refusing to write"), "{error}");
        use std::os::unix::fs::FileTypeExt;
        assert!(
            fs::symlink_metadata(&fifo).unwrap().file_type().is_fifo(),
            "fifo must be untouched"
        );
        write_output(&Some(PathBuf::from("/dev/null")), "x\n").expect("char device streams");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_output_streams_stdout_aliases_even_under_a_pipe() {
        // Assess U3c + live PoC A4: `-o /dev/stdout` under a pipeline
        // resolves through /proc/self/fd/1 to an anonymous pipe inode,
        // which is neither a regular file nor a char device. The alias
        // set intercepts it before resolution and streams through the
        // descriptor, so piping `-o /dev/stdout` into jq keeps working.
        assert!(is_stdout_alias(Path::new("/dev/stdout")));
        assert!(is_stdout_alias(Path::new("/proc/self/fd/1")));
        assert!(is_stdout_alias(Path::new("/dev/fd/2")));
        assert!(!is_stdout_alias(Path::new("/tmp/out.md")));
        assert!(!is_stdout_alias(Path::new("/dev/null")));
    }

    #[test]
    fn go_flag_compatible_args_know_config_surface_flags_take_values() {
        // rm-384 regression: the Go-style shim's value-taking flag list
        // predates the config surface, so `--config x.toml --doctor`
        // mistook the config path for the positional session path and
        // rejected every following flag as dropped. All four new flags
        // must consume their value before the positional scan continues.
        for flag in ["--config", "--history-dir", "--pricing-file"] {
            let args = go_flag_compatible_args([
                OsString::from("agenttrace"),
                OsString::from(flag),
                OsString::from("conf.toml"),
                OsString::from("--doctor"),
                OsString::from("--demo"),
            ])
            .expect("value-taking flags do not end the positional scan");
            assert_eq!(args.len(), 5, "{flag} keeps the full argv");
        }
        let args = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("--weekly-budget"),
            OsString::from("25"),
            OsString::from("--budget"),
        ])
        .expect("--weekly-budget takes a value");
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn validate_gate_thresholds_rejects_silent_degradation_values() {
        // rm-389: --limit 0 and --search-limit 0 used to be accepted
        // silently at dispatch; --weekly-budget validates like every other
        // numeric knob. The 2026-10-06 completion (run de96d4cc) made both
        // zero limits invalid REGARDLESS of the action, superseding the
        // cycle-1 conditional search-limit rule this test originally pinned.
        let mut args = compare_args(None);
        args.limit = 0;
        let error = validate_gate_thresholds(&args).unwrap_err().to_string();
        assert!(error.contains("--limit must be at least 1"), "{error}");

        let mut args = compare_args(None);
        args.sessions = true;
        args.search_limit = 0;
        let error = validate_gate_thresholds(&args).unwrap_err().to_string();
        assert!(
            error.contains("--search-limit must be at least 1"),
            "{error}"
        );

        let mut args = compare_args(None);
        args.weekly_budget = Some(f64::NAN);
        let error = validate_gate_thresholds(&args).unwrap_err().to_string();
        assert!(
            error.contains("--weekly-budget must be a positive finite USD"),
            "{error}"
        );

        let mut args = compare_args(None);
        args.search_limit = 0;
        let error = validate_gate_thresholds(&args)
            .expect_err("bare --search-limit 0 fails too since rm-389's completion");
        assert!(error.to_string().contains("--search-limit"), "{error}");
        let mut args = compare_args(None);
        args.weekly_budget = Some(12.5);
        validate_gate_thresholds(&args).expect("positive budget accepted");
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
        // rm-835 flipped this message: the all-corrupt directory used to
        // bail as "No sessions match the requested filters" — the filters
        // were not the problem, every parse was failing silently — so the
        // exhaustion arm now says what actually happened.
        assert!(
            err.to_string().contains("No sessions parsed"),
            "exhaustion arm: {err}"
        );
        assert!(
            err.to_string().contains("failed to parse"),
            "failure disclosure: {err}"
        );
        assert!(
            err.to_string().contains("storage.json"),
            "first failure must name the offending file: {err}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn empty_session_bail_discloses_zstd_parse_failure_hint() {
        // rm-835: the parser's zstd-compressed rollout hint used to die
        // with the parse error at discovery's `.ok()`; the directory
        // overview then claimed "no sessions match the requested
        // filters" while the same file, passed explicitly, printed the
        // remediation hint. The bail must carry the hint through.
        let root = std::env::temp_dir().join(format!("agenttrace-zst-bail-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create zst bail dir");
        // zstd magic (28 b5 2f fd) followed by frame zeros: not JSONL.
        let mut bytes = vec![0x28, 0xb5, 0x2f, 0xfd];
        bytes.extend(std::iter::repeat_n(0u8, 100));
        fs::write(root.join("rollout-2026-10-06.jsonl"), bytes).expect("write zst fixture");

        let mut args = compare_args(Some(root.to_string_lossy().to_string()));
        args.compare = false;
        args.overview = true;
        let err = load_sessions(&args).expect_err("all-corrupt dir must fail, not filter-match");
        assert!(
            err.to_string().contains("zstd"),
            "the zstd remediation hint must surface: {err}"
        );
        assert!(
            err.to_string().contains("rollout-2026-10-06.jsonl"),
            "the offending file must be named: {err}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parse_failure_advisory_names_the_corrupt_file() {
        // rm-835 residual arm: a partially-unreadable corpus must carry a
        // load-time advisory even when enough sessions parse for reports
        // to render — otherwise the post-filter empty bails and every
        // report look complete over a corpus that is partially garbage.
        let report = agenttrace_core::LoadReport {
            discovered: 3,
            parse_failures: 1,
            first_parse_failure: Some(
                "read session file /x/rollout.jsonl: zstd-compressed".to_string(),
            ),
            ..Default::default()
        };
        let advisory = parse_failure_advisory(&report).expect("advisory for partial failure");
        assert!(
            advisory.starts_with("1 of 3 discovered session files failed to parse"),
            "{advisory}"
        );
        assert!(advisory.contains("rollout.jsonl"), "{advisory}");
        assert!(advisory.contains("zstd-compressed"), "{advisory}");

        // clean loads and countless failures must stay silent
        let report = agenttrace_core::LoadReport {
            discovered: 2,
            ..Default::default()
        };
        assert!(parse_failure_advisory(&report).is_none());
        let report = agenttrace_core::LoadReport {
            discovered: 2,
            parse_failures: 2, // counted, but no message retained
            ..Default::default()
        };
        assert!(
            parse_failure_advisory(&report).is_none(),
            "no message means nothing to print"
        );
    }

    #[test]
    fn demo_preserve_history_conflicts_loudly() {
        // rm-836: --demo --preserve-history used to write the synthetic
        // demo sessions into the user's durable history.json (dossier
        // PoC F3), permanently inflating every later --include-history
        // report. The pair must be refused before any load happens.
        let mut args = compare_args(None);
        args.demo = true;
        validate_primary_action(&args).expect("--demo alone is fine");

        let mut args = compare_args(None);
        args.preserve_history = true;
        validate_primary_action(&args).expect("--preserve-history alone is fine");

        let mut args = compare_args(None);
        args.demo = true;
        args.preserve_history = true;
        let error = validate_primary_action(&args)
            .expect_err("demo must never enter durable history")
            .to_string();
        assert!(
            error.contains("--demo cannot be combined with --preserve-history"),
            "{error}"
        );
        assert!(
            error.contains("ephemeral"),
            "the error must say why: {error}"
        );
    }

    fn temp_session_file(prefix: &str, content: &str) -> String {
        let path = std::env::temp_dir().join(format!("{prefix}-{}.jsonl", std::process::id()));
        let mut file = fs::File::create(&path).expect("create temp session");
        writeln!(file, "{content}").expect("write temp session");
        path.to_string_lossy().to_string()
    }

    fn session(name: &str, path: &str, session_start: &str) -> Session {
        Session {
            name: name.to_string(),
            path: path.to_string(),
            cwd: String::new(),
            branch: String::new(),
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
        crate::test_args(dir)
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

    #[test]
    fn gate_thresholds_reject_zero_limits_regardless_of_action() {
        // rm-389: --limit 0 used to render empty output rc0 and a bare
        // --search-limit 0 skipped the guard when --search was not set.
        let mut args = compare_args(None);
        args.overview = true;
        args.compare = false;
        args.limit = 0;
        let err = validate_gate_thresholds(&args).expect_err("--limit 0 must fail");
        assert!(err.to_string().contains("--limit"), "got: {err}");

        args.limit = 20;
        args.search_limit = 0;
        let err = validate_gate_thresholds(&args).expect_err("bare --search-limit 0 must fail");
        assert!(err.to_string().contains("--search-limit"), "got: {err}");
    }

    #[test]
    fn gate_thresholds_still_accept_valid_limits() {
        let mut args = compare_args(None);
        args.overview = true;
        args.compare = false;
        validate_gate_thresholds(&args).expect("valid limits pass");
    }

    #[test]
    #[cfg(unix)]
    fn special_file_kind_classifies_character_devices() {
        let meta = std::fs::symlink_metadata("/dev/null").expect("/dev/null metadata");
        assert_eq!(special_file_kind(&meta), "character device");
    }

    #[test]
    fn statusline_report_flag_wins_over_keyword_host_dispatch() {
        // rm-573: the dispatch-order guards must stay in main().
        let source = include_str!("main.rs");
        assert!(
            source.contains(r#"== Some("statusline") && !args.statusline_report"#),
            "rm-573: statusline keyword guard missing"
        );
        assert!(
            source.contains(r#"== Some("upstream") && !args.statusline_report"#),
            "rm-573: upstream keyword guard missing"
        );
        // rm-455 joined the keyword family at integration; the rm-573
        // precedence invariant covers every host keyword.
        assert!(
            source.contains(r#"== Some("mcp") && !args.statusline_report"#),
            "rm-573: mcp keyword guard missing"
        );
    }

    #[test]
    #[cfg(unix)]
    fn admission_names_symlink_targets_by_their_true_class() {
        // Review fix (rm-212, 2026-10-06): a symlink to /dev/null used to read
        // as "dangling symlink"; a symlink to a readable session file must be
        // admitted (discovery follows symlinks too).
        let dir = std::env::temp_dir().join(format!(
            "agenttrace-symlink-admission-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let link = dir.join("null-link");
        std::os::unix::fs::symlink("/dev/null", &link).expect("symlink to /dev/null");
        let err = admit_session_path(&link).expect_err("symlink to a device must fail");
        assert!(
            err.to_string()
                .contains("symbolic link to character device"),
            "got: {err}"
        );

        let dangling = dir.join("dangling");
        std::os::unix::fs::symlink(dir.join("nowhere"), &dangling).expect("dangling symlink");
        let err = admit_session_path(&dangling).expect_err("dangling must fail");
        assert!(err.to_string().contains("dangling symlink"), "got: {err}");

        let real = dir.join("real.jsonl");
        std::fs::write(&real, "{}").expect("real session file");
        let good = dir.join("good-link");
        std::os::unix::fs::symlink(&real, &good).expect("symlink to a real file");
        admit_session_path(&good).expect("symlink to a regular file is admitted");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

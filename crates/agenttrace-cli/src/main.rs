use agenttrace_core::{
    add_baseline_comparison, compute_overview, demo_sessions, evaluate_overview_gate,
    find_session_files, load_sessions_from_dir, parse_file, pricing_cache_path,
    render_doctor_report, render_model_pricing_list, render_test_match, render_waste_report,
    report_compare, report_compare_json, report_json_with_language, report_overview_html,
    report_overview_json, report_overview_markdown, report_overview_text, report_search_json,
    report_search_text, report_text, search_sessions, update_pricing, BaselineThresholds,
    ReportLanguage, Session, VERSION,
};
use anyhow::{bail, Context};
use chrono::{DateTime, Utc};
use clap::Parser;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Parser)]
#[command(name = "agenttrace")]
#[command(about = "TUI observability for AI coding agent sessions")]
struct Args {
    path: Option<String>,
    #[arg(short = 'f', long = "format", default_value = "text")]
    format: String,
    #[arg(short = 'd')]
    dir: Option<String>,
    #[arg(long)]
    compare: bool,
    #[arg(long)]
    overview: bool,
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
    #[arg(long = "lang", default_value = "en")]
    lang: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse_from(go_flag_compatible_args(std::env::args_os()));
    let language = report_language(&args.lang);

    if args.version {
        println!("agenttrace v{}", VERSION);
        return Ok(());
    }

    if args.update_pricing {
        println!("Downloading pricing from LiteLLM...");
        let count = update_pricing()?;
        println!("Loaded {} model prices", count);
        println!("Cache saved: {}", pricing_cache_path().display());
        if !has_post_pricing_action(&args) {
            return Ok(());
        }
    }

    if args.test_match {
        print!("{}", render_test_match());
        return Ok(());
    }

    if args.doctor {
        let doctor_dir = args.dir.as_deref().map(PathBuf::from);
        let out = render_doctor_report(doctor_dir.as_deref(), args.demo, &args.format)?;
        write_output(&args.output, &out)?;
        print!("{out}");
        return Ok(());
    }

    if args.list_models {
        print!("{}", render_model_pricing_list());
        return Ok(());
    }

    if !has_session_action(&args) {
        if args.demo {
            let sessions = demo_sessions()?;
            return agenttrace_tui::run_with_sessions(sessions, "demo");
        }
        return agenttrace_tui::run(args.dir.as_deref().unwrap_or(""));
    }
    if args.baseline.is_some() && !args.overview {
        bail!("--baseline requires --overview -f json");
    }

    if args.compare {
        let sessions = load_sessions_for_compare(&args)?;
        if sessions.is_empty() {
            bail!(
                "No session files found in {}",
                args.dir.as_deref().unwrap_or("")
            );
        }
        let out = if args.format == "json" {
            report_compare_json(&sessions)
        } else {
            report_compare(&sessions, &args.model)
        };
        write_output(&args.output, &(out.clone() + "\n"))?;
        print!("{out}");
        return Ok(());
    }

    if args.waste {
        let session = load_latest_session(&args)?;
        let out = render_waste_report(&session);
        write_output(&args.output, &(out.clone() + "\n"))?;
        print!("{out}");
        return Ok(());
    }

    if args.latest || args.path.is_some() {
        let session = load_latest_session(&args)?;
        let out = match args.format.as_str() {
            "json" => report_json_with_language(&session, language),
            _ => report_text(&session),
        };
        write_output(&args.output, &(out.clone() + "\n"))?;
        print!("{out}");
        return Ok(());
    }

    let sessions = load_sessions(&args)?;

    if let Some(query) = args.search.as_deref() {
        let results = search_sessions(&sessions, query, args.search_limit);
        let out = if args.format == "json" {
            report_search_json(&results)
        } else {
            report_search_text(&results, query)
        };
        write_output(&args.output, &(out.clone() + "\n"))?;
        print!("{out}");
        return Ok(());
    }

    if args.overview {
        let overview = compute_overview(&sessions);
        let mut out = match args.format.as_str() {
            "json" => report_overview_json(&overview, &sessions),
            "markdown" | "md" => report_overview_markdown(&overview, &sessions),
            "html" => report_overview_html(&overview, &sessions),
            _ => report_overview_text(&overview, &sessions),
        };
        if let Some(baseline) = args.baseline.as_deref() {
            if args.format != "json" {
                bail!("--baseline requires --overview -f json");
            }
            out = add_baseline_comparison(
                &out,
                baseline,
                BaselineThresholds {
                    max_duration_delta_pct: args.baseline_max_duration_delta_pct,
                    max_cost_delta_pct: args.baseline_max_cost_delta_pct,
                    max_token_delta_pct: args.baseline_max_token_delta_pct,
                },
            )?;
        }
        write_output(&args.output, &(out.clone() + "\n"))?;
        print!("{out}");
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
            eprintln!("Inspect demo data with: agenttrace --demo --overview -f json");
            std::process::exit(2);
        }
        return Ok(());
    }

    bail!("no report action selected")
}

fn go_flag_compatible_args<I>(args: I) -> Vec<OsString>
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

    out
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

fn report_language(value: &str) -> ReportLanguage {
    match value.to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" | "zh_cn" | "chinese" => ReportLanguage::Zh,
        _ => ReportLanguage::En,
    }
}

fn load_sessions(args: &Args) -> anyhow::Result<Vec<Session>> {
    if args.demo {
        return demo_sessions();
    }
    if let Some(path) = args.path.as_deref() {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(vec![parse_file(&path)?]);
        }
        if path.is_dir() {
            if is_cline_task_dir(&path) {
                return Ok(vec![parse_file(&path)?]);
            }
            bail!(
                "Error loading {}: positional path must be a session file",
                path.display()
            );
        }
        bail!("session path does not exist: {}", path.display());
    }
    let dir = args.dir.as_deref().map(PathBuf::from);
    let sessions = if dir.is_none() {
        load_sessions_from_dir(None)
    } else {
        load_parseable_sessions_from_files(dir.as_deref())?
    };
    if sessions.is_empty() {
        bail!(
            "No session files found in {}",
            args.dir.as_deref().unwrap_or("")
        );
    }
    Ok(sessions)
}

fn load_latest_session(args: &Args) -> anyhow::Result<Session> {
    if args.demo {
        let sessions = demo_sessions()?;
        return latest_session(&sessions)
            .cloned()
            .context("no demo sessions");
    }
    if let Some(path) = args.path.as_deref() {
        let path = PathBuf::from(path);
        if path.is_file() || is_cline_task_dir(&path) {
            return parse_file(&path);
        }
        if path.is_dir() {
            bail!(
                "Error loading {}: positional path must be a session file",
                path.display()
            );
        }
        bail!("session path does not exist: {}", path.display());
    }
    let dir = args.dir.as_deref().map(PathBuf::from);
    let files = find_session_files(dir.as_deref());
    if files.is_empty() {
        bail!(
            "No session files found in {}",
            args.dir.as_deref().unwrap_or("")
        );
    }
    let target = latest_session_file(&files).context("No session files found.")?;
    parse_file(target)
}

fn load_parseable_sessions_from_files(
    dir: Option<&std::path::Path>,
) -> anyhow::Result<Vec<Session>> {
    let files = find_session_files(dir);
    if files.is_empty() {
        return Ok(Vec::new());
    }
    Ok(files
        .iter()
        .filter_map(|path| parse_file(path).ok())
        .collect())
}

fn latest_session_file(files: &[PathBuf]) -> Option<&std::path::Path> {
    files
        .iter()
        .filter_map(|path| latest_session_candidate(path).map(|candidate| (path, candidate)))
        .max_by(|(_, a), (_, b)| newer_session_candidate_order(a, b))
        .map(|(path, _)| path.as_path())
}

struct LatestCandidate {
    session_time: Option<DateTime<Utc>>,
    mod_time: SystemTime,
    path: String,
}

fn latest_session_candidate(path: &std::path::Path) -> Option<LatestCandidate> {
    let mod_time = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()?;
    let session_time = parse_file(path)
        .ok()
        .and_then(|session| parse_session_time(&session.metrics.session_start));
    Some(LatestCandidate {
        session_time,
        mod_time,
        path: path.to_string_lossy().to_string(),
    })
}

fn parse_session_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

fn newer_session_candidate_order(a: &LatestCandidate, b: &LatestCandidate) -> std::cmp::Ordering {
    a.session_time
        .is_some()
        .cmp(&b.session_time.is_some())
        .then_with(|| match (a.session_time, b.session_time) {
            (Some(a_time), Some(b_time)) => a_time.cmp(&b_time),
            _ => a.mod_time.cmp(&b.mod_time),
        })
        .then_with(|| a.path.cmp(&b.path))
}

fn load_sessions_for_compare(args: &Args) -> anyhow::Result<Vec<Session>> {
    if args.demo {
        return load_sessions(args);
    }
    let dir = args.dir.as_deref().map(PathBuf::from);
    let mut files = find_session_files(dir.as_deref());
    if files.len() > 15 {
        eprintln!(
            "Found {} session files, showing the most recent 15. Use -d <dir> or remove old sessions to compare all.",
            files.len()
        );
        files.truncate(15);
    }
    Ok(files
        .iter()
        .filter_map(|path| parse_file(path).ok())
        .collect())
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
        fs::write(path, content)?;
        eprintln!("Saved: {}", path.display());
    }
    Ok(())
}

fn has_post_pricing_action(args: &Args) -> bool {
    args.path.is_some()
        || args.list_models
        || args.test_match
        || args.doctor
        || args.latest
        || args.compare
        || args.overview
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
        || args.overview
        || args.waste
        || args.baseline.is_some()
        || args
            .search
            .as_deref()
            .map(|query| !query.trim().is_empty())
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agenttrace_core::Metrics;
    use std::io::Write;

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
    fn go_flag_compatible_args_ignore_flags_after_positional_path() {
        let args = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("session.jsonl"),
            OsString::from("-f"),
            OsString::from("json"),
        ]);

        assert_eq!(
            args,
            vec![
                OsString::from("agenttrace"),
                OsString::from("session.jsonl")
            ]
        );
    }

    #[test]
    fn go_flag_compatible_args_keep_flags_before_positional_path() {
        let args = go_flag_compatible_args([
            OsString::from("agenttrace"),
            OsString::from("-f"),
            OsString::from("json"),
            OsString::from("session.jsonl"),
        ]);

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
    fn compare_loader_uses_go_file_cap() {
        let root =
            std::env::temp_dir().join(format!("agenttrace-compare-cap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create compare temp dir");
        for idx in 0..16 {
            write_compare_session(&root.join(format!("{idx:02}.jsonl")), idx);
        }

        let args = compare_args(Some(root.to_string_lossy().to_string()));
        let sessions = load_sessions_for_compare(&args).expect("load compare sessions");
        assert_eq!(sessions.len(), 15);

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
        assert!(err.to_string().contains("No session files found in"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn latest_session_file_can_select_unparseable_newest_file_like_go() {
        let root =
            std::env::temp_dir().join(format!("agenttrace-latest-bad-file-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create latest bad file dir");
        let older = root.join("older.jsonl");
        let newer = root.join("newer.json");
        fs::write(
            &older,
            r#"{"role":"user","content":"older no timestamp","SourceTool":"generic"}
{"role":"assistant","content":"older answer","SourceTool":"generic"}
"#,
        )
        .expect("write older session");
        std::thread::sleep(std::time::Duration::from_millis(5));
        fs::write(&newer, r#"{"not":"#).expect("write bad latest");

        let files = find_session_files(Some(&root));
        let latest = latest_session_file(&files).expect("latest file");
        assert_eq!(latest, newer.as_path());

        let mut args = compare_args(Some(root.to_string_lossy().to_string()));
        args.compare = false;
        args.latest = true;
        load_latest_session(&args).expect_err("bad latest should fail");

        let _ = fs::remove_dir_all(root);
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
            metrics: Metrics {
                session_start: session_start.to_string(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
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
            overview: false,
            model: "default".to_string(),
            output: None,
            latest: false,
            waste: false,
            list_models: false,
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
            lang: "en".to_string(),
        }
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
}

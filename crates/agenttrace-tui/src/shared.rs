//! Production TUI helpers (rm-014 unification, cycle 2).
//!
//! This module is the ONE implementation of the shared TUI helper set:
//! the production renderer (`explorer.rs`) and the test renderer
//! (`presentation.rs`) both consume these functions through `app`'s
//! `use shared::*` binding. Before rm-014 the test renderer carried its
//! own copies of all 21 helpers (12 of them drifted); tests exercised the
//! copies, not production code. `tests::presentation_defines_no_duplicate_helpers`
//! fails the suite if a copy is ever re-introduced, and
//! `tests::helpers_resolve_to_the_production_module` pins function-pointer
//! identity between the glob binding and `shared::`.
//!
//! Every helper here must have a production caller (directly or through
//! another helper) — there is deliberately no `allow(dead_code)`.

use super::*;

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct DriverItem {
    pub(super) label: String,
    pub(super) sessions: usize,
    pub(super) failures: usize,
    pub(super) tokens: i64,
    pub(super) cost: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct InspectFirstItem {
    pub(super) label: &'static str,
    pub(super) index: usize,
}

pub(super) fn cache_state_label() -> String {
    match agenttrace_core::session_cache_path().metadata() {
        Ok(metadata) if metadata.len() > 0 => "cache warm".to_string(),
        _ => "cache empty".to_string(),
    }
}

pub(super) fn source_counts(sessions: &[Session]) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for session in sessions {
        *counts.entry(driver_source(session)).or_default() += 1;
    }
    let mut items = counts.into_iter().collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items
}

pub(super) fn render_loading_status(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(app.t("Loading", "加载中"));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);
    let state = &app.load_state;
    let ratio = if state.discovered == 0 {
        0.0
    } else {
        state.processed.min(state.discovered) as f64 / state.discovered as f64
    };
    let label = if state.discovered == 0 {
        app.t("Discovering sessions…", "正在发现会话…").to_string()
    } else if state.processed >= state.discovered {
        app.t(
            "Files processed · loading databases and finishing…",
            "文件已处理 · 正在加载数据库并汇总…",
        )
        .to_string()
    } else {
        format!(
            "{}/{} · {:.0}%",
            state.processed,
            state.discovered,
            ratio * 100.0
        )
    };
    frame.render_widget(
        ratatui::widgets::Gauge::default()
            .ratio(ratio)
            .label(label)
            .gauge_style(Style::default().fg(Color::Cyan)),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(loading_status_lines(app)).wrap(Wrap { trim: true }),
        rows[1],
    );
}

/// Honest loading disclosure (adopted from the pre-unification test
/// renderer during rm-014): mode, progress bar, sources, parsed /
/// confidence / skipped / fallback-pricing / latest-session lines. The
/// pre-unification production copy rendered three lines and never
/// disclosed sources or parse confidence.
pub(super) fn loading_status_lines(app: &App) -> Vec<Line<'static>> {
    let state = &app.load_state;
    let health = &app.derived.health;
    let mode = if state.force {
        app.t("force reload", "强制重载")
    } else {
        app.t("normal load", "正常加载")
    };
    let processed = state.processed.min(state.discovered);
    let progress_width = 32;
    let filled = processed
        .saturating_mul(progress_width)
        .checked_div(state.discovered)
        .unwrap_or(0);
    let percent = processed
        .saturating_mul(100)
        .checked_div(state.discovered)
        .unwrap_or(0);
    let source_text = if state.sources.is_empty() {
        format!("{}={}", app.t("sources", "来源"), app.t("none", "无"))
    } else {
        format!(
            "{}={}",
            app.t("sources", "来源"),
            state
                .sources
                .iter()
                .take(4)
                .map(|(source, count)| format!("{}:{count}", short(source, 18)))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    vec![
        Line::from(format!(
            "{} - {} {} {}",
            load_phase_label(state.phase, app.language),
            mode,
            app.t("from", "来自"),
            short(&display_source_label(&state.source), 36)
        )),
        Line::from(format!(
            "{} {}/{} {}, {} {}, {}",
            app.t("loaded", "已加载"),
            format_count(processed as i64),
            format_count(state.discovered as i64),
            app.t("files processed", "个文件已处理"),
            format_count(state.cache_hits as i64),
            app.t("cache hits", "缓存命中"),
            cache_state_for_language(&state.cache_state, app.language)
        )),
        Line::from(vec![
            Span::raw("["),
            Span::styled("█".repeat(filled), Style::default().fg(Color::Green)),
            Span::styled(
                "░".repeat(progress_width - filled),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(format!("] {percent}%")),
        ]),
        Line::from(source_text),
        Line::from(format!(
            "{}={}  {}={}  {}={}  {}={}  {}={}",
            app.t("sessions parsed", "已解析会话"),
            format_count(state.parsed as i64),
            app.t("confidence", "可信度"),
            localized_level(&health.confidence, app.language),
            app.t("skipped", "跳过"),
            format_count(state.skipped as i64),
            app.t("pricing fallback", "价格回退"),
            format_count(health.fallback_pricing as i64),
            app.t("latest", "最新"),
            if health.latest_session_at.is_empty() {
                app.t("unknown", "未知").to_string()
            } else {
                short(&health.latest_session_at, 20)
            }
        )),
    ]
}

pub(super) fn load_summary_line(app: &App) -> String {
    if app.pending_load.is_some() && !app.sessions.is_empty() {
        let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let frame =
            frames[(app.last_auto_refresh.elapsed().as_millis() / 120) as usize % frames.len()];
        return format!(
            "{frame} {} {}/{}",
            app.t("Refreshing", "刷新中"),
            app.load_state.processed,
            app.load_state.discovered
        );
    }
    let state = &app.load_state;
    match state.phase {
        LoadPhase::Idle => app.t("idle", "空闲").to_string(),
        LoadPhase::Discovering => format!(
            "{} {} {}",
            app.t("discovering", "发现中"),
            format_count(state.discovered as i64),
            app.t("files", "个文件")
        ),
        LoadPhase::Parsing => format!(
            "{} {} {}, {} {}",
            app.t("loading", "加载中"),
            format_count(state.discovered as i64),
            app.t("files", "个文件"),
            format_count(state.cache_hits as i64),
            app.t("cache hits", "缓存命中")
        ),
        LoadPhase::Ready => {
            let source = state
                .sources
                .first()
                .map(|(source, count)| {
                    format!(
                        "{}:{}",
                        display_source_label(source),
                        format_count(*count as i64)
                    )
                })
                .unwrap_or_else(|| app.t("none", "无").to_string());
            format!(
                "{} {} {}, {} {}, {source}",
                app.t("loaded", "已加载"),
                format_count(state.parsed as i64),
                app.t("sessions", "个会话"),
                format_count(state.cache_hits as i64),
                app.t("cache hits", "缓存命中")
            )
        }
        LoadPhase::Failed => app.t("load failed", "加载失败").to_string(),
    }
}

pub(super) fn load_phase_label(phase: LoadPhase, language: Language) -> &'static str {
    match phase {
        LoadPhase::Idle => text(language, "Idle", "空闲"),
        LoadPhase::Discovering => text(language, "Finding sessions", "正在查找会话"),
        LoadPhase::Parsing => text(language, "Reading sessions", "正在读取会话"),
        LoadPhase::Ready => text(language, "Ready", "就绪"),
        LoadPhase::Failed => text(language, "Load failed", "加载失败"),
    }
}

pub(super) fn top_driver<T: Borrow<Session>>(
    sessions: &[T],
    label: fn(&Session) -> String,
) -> Option<DriverItem> {
    let mut groups: BTreeMap<String, DriverItem> = BTreeMap::new();
    for session in sessions {
        let session = session.borrow();
        let label = label(session);
        let entry = groups.entry(label.clone()).or_insert_with(|| DriverItem {
            label,
            ..DriverItem::default()
        });
        entry.sessions += 1;
        entry.failures += session.metrics.tool_calls_fail;
        entry.tokens = entry.tokens.saturating_add(total_tokens(session));
        entry.cost += session.metrics.cost_estimated;
    }
    groups.into_values().max_by(compare_driver_items)
}

/// Counts DISTINCT sessions per anomaly kind (adopted from the
/// pre-unification test renderer during rm-014): `DriverItem::sessions`
/// names sessions, not anomaly occurrences — a session carrying the same
/// kind three times must count once. The pre-unification production copy
/// counted occurrences, overstating the driver.
pub(super) fn top_anomaly_driver<T: Borrow<Session>>(sessions: &[T]) -> Option<DriverItem> {
    let mut groups: BTreeMap<String, DriverItem> = BTreeMap::new();
    for session in sessions {
        let session = session.borrow();
        let mut seen_kinds = std::collections::BTreeSet::new();
        for anomaly in &session.anomalies {
            if !seen_kinds.insert(anomaly.kind.clone()) {
                continue;
            }
            let entry = groups
                .entry(anomaly.kind.clone())
                .or_insert_with(|| DriverItem {
                    label: anomaly.kind.clone(),
                    ..DriverItem::default()
                });
            entry.sessions += 1;
            entry.failures += session.metrics.tool_calls_fail;
            entry.tokens = entry.tokens.saturating_add(total_tokens(session));
            entry.cost += session.metrics.cost_estimated;
        }
    }
    groups.into_values().max_by(compare_driver_items)
}

fn compare_driver_items(left: &DriverItem, right: &DriverItem) -> Ordering {
    left.sessions
        .cmp(&right.sessions)
        .then_with(|| left.failures.cmp(&right.failures))
        .then_with(|| left.cost.total_cmp(&right.cost))
        .then_with(|| right.label.cmp(&left.label))
}

pub(super) fn inspect_first_items_for_app(app: &App) -> Vec<InspectFirstItem> {
    let indices = app.filtered.clone();
    let sessions = indices
        .iter()
        .map(|index| app.sessions[*index].clone())
        .collect::<Vec<_>>();
    inspect_first(&sessions)
        .into_iter()
        .filter_map(|item| {
            indices.get(item.index).map(|index| InspectFirstItem {
                label: item.reason,
                index: *index,
            })
        })
        .collect()
}

pub(super) fn inspect_target_view(label: &str) -> View {
    match label {
        "cost" => View::Detail,
        _ => View::Diagnostics,
    }
}

pub(super) fn driver_source(session: &Session) -> String {
    if session.metrics.source_tool.is_empty() {
        "unknown".to_string()
    } else {
        display_source_label(&session.metrics.source_tool)
    }
}

pub(super) fn display_session_source(session: &Session) -> String {
    driver_source(session)
}

/// Recognizes BOTH slug ids and on-disk source paths (adopted from the
/// pre-unification test renderer during rm-014). The pre-unification
/// production copy mapped a path like `/home/u/.claude/projects` to its
/// last segment ("projects"), mislabeling the source column.
pub(super) fn display_source_label(source: &str) -> String {
    let source = source.trim();
    if source.is_empty() || source == "auto-discovery" {
        return "auto discovery".to_string();
    }
    if source == "pi" || source.ends_with("/.pi/agent/sessions") {
        return "Pi sessions".to_string();
    }
    if source == "oh_my_pi" || source.ends_with("/.omp/agent/sessions") {
        return "Oh My Pi sessions".to_string();
    }
    if source == "pi_senpi" {
        return "Pi (senpi) sessions".to_string();
    }
    if source == "pi_omo" {
        return "Pi (omo) sessions".to_string();
    }
    if source == "claude_code" || source.ends_with("/.claude/projects") {
        return "Claude Code".to_string();
    }
    if source == "codex_cli" || source.contains("/.codex/") {
        return "Codex".to_string();
    }
    if source == "hermes_db" || source.ends_with("/.hermes/state.db") {
        return "Hermes DB".to_string();
    }
    if source == "opencode_db" || source.ends_with("/opencode.db") {
        return "OpenCode DB".to_string();
    }
    if source.contains('/') {
        return source
            .rsplit('/')
            .find(|part| !part.is_empty())
            .unwrap_or(source)
            .to_string();
    }
    source.to_string()
}

pub(super) fn driver_model(session: &Session) -> String {
    if session.metrics.model_used.is_empty() {
        "unknown".to_string()
    } else {
        session.metrics.model_used.clone()
    }
}

pub(super) fn format_compact_cost(cost: f64) -> String {
    format_cost(cost)
}

pub(super) fn total_tokens_all<T: Borrow<Session>>(sessions: &[T]) -> i64 {
    // Saturating: cross-session totals must stay bounded when individual
    // sessions saturate (mirrors reports.rs overview_summary).
    sessions
        .iter()
        .map(|session| total_tokens(session.borrow()))
        .fold(0i64, i64::saturating_add)
}

pub(super) fn total_duration<T: Borrow<Session>>(sessions: &[T]) -> f64 {
    sessions
        .iter()
        .map(|session| session.borrow().metrics.duration_sec)
        .sum()
}

pub(super) fn p95_gap<T: Borrow<Session>>(sessions: &[T]) -> f64 {
    let mut gaps = sessions
        .iter()
        .flat_map(|session| session.borrow().metrics.gaps_sec.iter().copied())
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect::<Vec<_>>();
    if gaps.is_empty() {
        return 0.0;
    }
    gaps.sort_by(f64::total_cmp);
    let index = ((gaps.len() as f64) * 0.95) as usize;
    gaps[index.min(gaps.len() - 1)]
}

pub(super) fn health_color(health: i32) -> Color {
    match health {
        80.. => Color::Green,
        50..=79 => Color::Yellow,
        _ => Color::LightRed,
    }
}

pub(super) fn format_count(value: i64) -> String {
    format_tokens(value)
}

/// Durations of a year or more render in years (adopted from the
/// pre-unification test renderer during rm-014); the production copy
/// showed "1095.0d"-style output for multi-year durations.
pub(super) fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        "0s".to_string()
    } else if seconds < 60.0 {
        format!("{seconds:.0}s")
    } else if seconds < 3600.0 {
        format!("{:.1}m", seconds / 60.0)
    } else if seconds < 86_400.0 {
        format!("{:.1}h", seconds / 3600.0)
    } else if seconds < 365.0 * 86_400.0 {
        format!("{:.1}d", seconds / 86_400.0)
    } else {
        format!("{:.1}y", seconds / (365.0 * 86_400.0))
    }
}

pub(super) fn localized_level(value: &str, language: Language) -> String {
    if language == Language::En {
        return value.to_string();
    }
    match value {
        "critical" => "严重",
        "warning" => "警告",
        "high" => "高",
        "medium" => "中",
        "good" => "良好",
        "low" => "低",
        "info" => "提示",
        _ => value,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_tokens_all_saturates_across_sessions() {
        // Cross-session aggregation must saturate: two sessions whose totals
        // sit at i64::MAX previously wrapped the shared overview to -2.
        use agenttrace_core::{Metrics, Session};
        let session = Session {
            name: "adversarial".to_string(),
            path: "/tmp/adversarial.jsonl".to_string(),
            cwd: String::new(),
            metrics: Metrics {
                tokens_input: i64::MAX,
                tokens_output: i64::MAX,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Default::default(),
        };
        let sessions = vec![session.clone(), session];
        assert_eq!(total_tokens_all(&sessions), i64::MAX);
    }

    #[test]
    fn loading_status_lines_disclose_sources_and_confidence() {
        // Adopted semantics (rm-014): the loading panel is the honest
        // 5-line disclosure — phase+mode, files/cache/cache-state, a
        // progress bar, the source breakdown, and parse-confidence line.
        let app = App::new(Vec::new(), "test", None);
        let lines = loading_status_lines(&app);
        assert_eq!(lines.len(), 5);
        assert!(format!("{:?}", lines[0]).contains("Idle"));
        assert!(format!("{:?}", lines[0]).contains("normal load"));
        assert!(format!("{:?}", lines[1]).contains("cache hits"));
        assert!(format!("{:?}", lines[3]).contains("sources=none"));
    }

    #[test]
    fn display_source_label_maps_slugs_and_paths() {
        // Slug ids keep their pre-unification labels.
        assert_eq!(display_source_label("codex_cli"), "Codex");
        // rm-084 review follow-up: fork ids must render as labels, never
        // leak as raw slugs.
        assert_eq!(display_source_label("pi_senpi"), "Pi (senpi) sessions");
        assert_eq!(display_source_label("pi_omo"), "Pi (omo) sessions");
        // Adopted semantics (rm-014): known on-disk source paths label
        // their tool instead of leaking a trailing path segment.
        assert_eq!(
            display_source_label("/home/u/.claude/projects"),
            "Claude Code"
        );
        assert_eq!(display_source_label("/u/.pi/agent/sessions"), "Pi sessions");
        assert_eq!(
            display_source_label("/u/.omp/agent/sessions"),
            "Oh My Pi sessions"
        );
        assert_eq!(display_source_label("/u/.codex/sessions.jsonl"), "Codex");
        assert_eq!(display_source_label("/u/.hermes/state.db"), "Hermes DB");
        assert_eq!(display_source_label("/u/opencode.db"), "OpenCode DB");
        // Unknown paths still degrade to their last non-empty segment.
        assert_eq!(display_source_label("/tmp/custom"), "custom");
    }

    #[test]
    fn top_anomaly_driver_counts_distinct_sessions() {
        // Adopted semantics (rm-014): `sessions` counts sessions, not
        // anomaly occurrences — one session with three latency anomalies
        // counts once; a second session with the same kind makes two.
        use agenttrace_core::{Anomaly, Metrics, Session};
        let session = |anomalies: Vec<Anomaly>| Session {
            name: "s".to_string(),
            path: "/tmp/s.jsonl".to_string(),
            cwd: String::new(),
            metrics: Metrics {
                tokens_input: 10,
                tokens_output: 5,
                ..Metrics::default()
            },
            anomalies,
            health: 50,
            tool_warnings: Vec::new(),
            diagnostics: Default::default(),
        };
        let anomaly = |kind: &str| Anomaly {
            kind: kind.to_string(),
            severity: "medium".to_string(),
            detail: String::new(),
        };
        let sessions = vec![
            session(vec![
                anomaly("latency"),
                anomaly("latency"),
                anomaly("latency"),
            ]),
            session(vec![anomaly("latency")]),
            session(vec![anomaly("loop"), anomaly("loop")]),
        ];
        let top = top_anomaly_driver(&sessions).expect("a driver");
        assert_eq!(top.label, "latency");
        assert_eq!(top.sessions, 2);
        let loop_driver = {
            let only = &sessions[2..];
            top_anomaly_driver(only)
        };
        assert_eq!(loop_driver.expect("loop driver").sessions, 1);
    }

    #[test]
    fn format_duration_renders_years_above_a_year() {
        // Adopted semantics (rm-014): >= 365 days renders in years.
        assert_eq!(format_duration(364.0 * 86_400.0), "364.0d");
        assert_eq!(format_duration(365.0 * 86_400.0), "1.0y");
        assert_eq!(format_duration(3.0 * 365.0 * 86_400.0), "3.0y");
        assert_eq!(format_duration(125.0), "2.1m");
        assert_eq!(format_duration(0.0), "0s");
    }

    #[test]
    fn load_phase_labels_distinguish_idle_from_ready() {
        // Adopted semantics (rm-014): Idle no longer masquerades as
        // "Ready" — the Idle phase has its own label.
        assert_eq!(load_phase_label(LoadPhase::Idle, Language::En), "Idle");
        assert_eq!(load_phase_label(LoadPhase::Idle, Language::Zh), "空闲");
        assert_eq!(load_phase_label(LoadPhase::Ready, Language::En), "Ready");
        assert_eq!(
            load_phase_label(LoadPhase::Discovering, Language::En),
            "Finding sessions"
        );
        assert_eq!(
            load_phase_label(LoadPhase::Parsing, Language::En),
            "Reading sessions"
        );
        assert_eq!(
            load_phase_label(LoadPhase::Failed, Language::En),
            "Load failed"
        );
    }
}

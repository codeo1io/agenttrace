use crate::{
    average_health, canonical_sessions, classify_tool_authority, context_trends, cost_audit,
    delivery_evidence, fmt_duration, format_cost, format_count, format_tokens,
    highest_authority_for_metrics, is_high_authority_category, mcp_governance, parse_ts,
    recommendations, report_scope, round4, sanitize_line_segment, sorted_keys, sorted_set,
    total_tokens, Anomaly, GroupOverview, Overview, Session, TaskTypeOverview, ToolCall, VERSION,
};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

#[derive(Debug, Clone, Copy, Default)]
pub struct BaselineThresholds {
    pub max_duration_delta_pct: f64,
    pub max_cost_delta_pct: f64,
    pub max_token_delta_pct: f64,
}

/// Which baseline thresholds a run breached (pass-7 P7-3). The report
/// JSON has always carried these booleans; they now also gate the CLI
/// exit code (exit 2, mirroring `--fail-under-health`) unless
/// `--no-baseline-gate` opts out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BaselineBreaches {
    pub slower_than_baseline: bool,
    pub cost_above_threshold: bool,
    pub tokens_above_threshold: bool,
}

impl BaselineBreaches {
    pub fn any(&self) -> bool {
        self.slower_than_baseline || self.cost_above_threshold || self.tokens_above_threshold
    }
}

/// Renders a key=counter map for report surfaces: parse-line losses
/// (`unparseable_line=1, event_schema=2`, pass-7 P7-1) and journal
/// disclosures (`pi_branches=2, pi_usage_entry:cache_warm=1`,
/// rm-436/rm-437). Empty input renders empty so report bytes stay
/// unchanged for clean corpora.
/// rm-595/rm-594: keys route through the sanitize+cap helper here as
/// well — mint sites already cap+sanitize (rm-594 helper), and this
/// single choke point covers every current and future mint site (and
/// legacy cache entries) for the terminal, markdown and HTML surfaces
/// (defense-in-depth; idempotent — U+FFFD is not a control byte, keys
/// under the cap are byte-identical). JSON output deliberately skips
/// this sanitizer (lossless JSON escaping, rm-383 contract).
fn counts_cell(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return String::new();
    }
    // rm-594 residual (run 9ab0afad assess F4): bound how many entries
    // a single-line render carries — the first 40 (BTreeMap order is
    // deterministic) plus an explicit count of what is held back;
    // corpora under the cap render byte-identical.
    let mut parts: Vec<String> = counts
        .iter()
        .take(crate::parser::DISCLOSURE_RENDER_ENTRY_CAP)
        .map(|(reason, count)| {
            format!("{}={count}", crate::parser::capped_disclosure_value(reason))
        })
        .collect();
    if counts.len() > crate::parser::DISCLOSURE_RENDER_ENTRY_CAP {
        parts.push(format!(
            "+{} more distinct keys",
            counts.len() - crate::parser::DISCLOSURE_RENDER_ENTRY_CAP
        ));
    }
    let rendered = parts.join(", ");
    // rm-450 provenance: the workbuddy input-basis counters only carry
    // their meaning alongside the assumption that produced them; clean
    // corpora keep byte-identical report output.
    if counts
        .keys()
        .any(|reason| reason.starts_with("workbuddy_input_basis:"))
    {
        format!(
            "{rendered} (input basis assumed cache-inclusive; a zeroed input suggests the transcript reports a cache-exclusive basis — upstream luoyuctl/agenttrace#310)"
        )
    } else {
        rendered
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReportLanguage {
    #[default]
    En,
    Zh,
}

impl ReportLanguage {
    fn t(self, en: &'static str, zh: &'static str) -> &'static str {
        match self {
            Self::En => en,
            Self::Zh => zh,
        }
    }
}

pub fn report_json(session: &Session) -> String {
    report_json_with_language(session, ReportLanguage::En)
}

pub fn report_json_with_language(session: &Session, language: ReportLanguage) -> String {
    let metrics = &session.metrics;
    let total_tokens = total_tokens(session);
    let total_tools = metrics.tool_calls_ok + metrics.tool_calls_fail;
    let avg_reason = if metrics.reasoning_blocks > 0 {
        round4(metrics.reasoning_chars as f64 / metrics.reasoning_blocks as f64)
    } else {
        0.0
    };
    let mut gaps = metrics.gaps_sec.clone();
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let tool_rate = if total_tools > 0 {
        round4(metrics.tool_calls_ok as f64 / total_tools as f64 * 100.0)
    } else {
        0.0
    };
    let latency_avg = average(&gaps);
    let latency_max = gaps.last().copied().unwrap_or(0.0);
    let latency_median = crate::percentile(&gaps, 0.50);
    let latency_min = gaps.first().copied().unwrap_or(0.0);
    let latency_p95 = crate::percentile(&gaps, 0.95);

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"activity\": {\n");
    out.push_str(&format!(
        "    \"assistant_turns\": {},\n",
        metrics.assistant_turns
    ));
    out.push_str(&format!(
        "    \"tool_calls_fail\": {},\n",
        metrics.tool_calls_fail
    ));
    out.push_str(&format!(
        "    \"tool_calls_ok\": {},\n",
        metrics.tool_calls_ok
    ));
    out.push_str(&format!(
        "    \"tool_calls_total\": {},\n",
        metrics.tool_calls_total
    ));
    out.push_str(&format!(
        "    \"tool_success_rate\": {},\n",
        json_float(tool_rate)
    ));
    out.push_str(&format!(
        "    \"user_messages\": {}\n",
        metrics.user_messages
    ));
    out.push_str("  },\n");
    out.push_str("  \"anomalies\": ");
    write_anomalies_json(&mut out, &session.anomalies, language, 1);
    out.push_str(",\n");
    out.push_str("  \"cost\": {\n");
    out.push_str(&format!(
        "    \"estimated\": {},\n",
        json_float(metrics.cost_estimated)
    ));
    out.push_str(&format!(
        "    \"model\": {}\n",
        json_string(&metrics.model_used)
    ));
    out.push_str("  },\n");
    out.push_str(&format!("  \"health_score\": {},\n", session.health));
    out.push_str("  \"latency\": {\n");
    out.push_str(&format!("    \"avg\": {},\n", json_float(latency_avg)));
    out.push_str(&format!("    \"max\": {},\n", json_float(latency_max)));
    out.push_str(&format!(
        "    \"median\": {},\n",
        json_float(latency_median)
    ));
    out.push_str(&format!("    \"min\": {},\n", json_float(latency_min)));
    out.push_str(&format!("    \"p95\": {}\n", json_float(latency_p95)));
    out.push_str("  },\n");
    out.push_str(&format!(
        "  \"model_used\": {},\n",
        json_string(&metrics.model_used)
    ));
    out.push_str("  \"reasoning\": {\n");
    out.push_str(&format!("    \"avg_chars\": {},\n", json_float(avg_reason)));
    out.push_str(&format!("    \"blocks\": {},\n", metrics.reasoning_blocks));
    out.push_str(&format!(
        "    \"redacted\": {},\n",
        metrics.reasoning_redact
    ));
    out.push_str(&format!(
        "    \"total_chars\": {}\n",
        metrics.reasoning_chars
    ));
    out.push_str("  },\n");
    out.push_str("  \"session\": {\n");
    out.push_str(&format!(
        "    \"duration_human\": {},\n",
        json_string(&fmt_duration_for_language(metrics.duration_sec, language))
    ));
    out.push_str(&format!(
        "    \"duration_seconds\": {},\n",
        json_float(metrics.duration_sec)
    ));
    out.push_str(&format!(
        "    \"end\": {},\n",
        json_string(&metrics.session_end)
    ));
    out.push_str(&format!(
        "    \"start\": {}\n",
        json_string(&metrics.session_start)
    ));
    out.push_str("  },\n");
    out.push_str(&format!(
        "  \"source_tool\": {},\n",
        json_string(&metrics.source_tool)
    ));
    out.push_str("  \"tokens\": {\n");
    out.push_str(&format!(
        "    \"cache_read\": {},\n",
        metrics.tokens_cache_r
    ));
    out.push_str(&format!(
        "    \"cache_write\": {},\n",
        metrics.tokens_cache_w
    ));
    out.push_str(&format!("    \"input\": {},\n", metrics.tokens_input));
    out.push_str(&format!("    \"output\": {},\n", metrics.tokens_output));
    out.push_str(&format!("    \"total\": {}\n", total_tokens));
    out.push_str("  },\n");
    out.push_str("  \"tool_authority\": {\n");
    out.push_str("    \"counts\": ");
    write_usize_map_json(&mut out, &metrics.tool_authority, 2);
    out.push_str(",\n");
    out.push_str(&format!(
        "    \"highest\": {}\n",
        json_string(&highest_authority_for_metrics(metrics))
    ));
    out.push_str("  },\n");
    out.push_str("  \"tools_top\": ");
    write_usize_map_json(&mut out, &top_tools(&metrics.tool_usage), 1);
    out.push_str(",\n");
    out.push_str(&format!("  \"version\": {}\n", json_string(VERSION)));
    out.push('}');
    out
}

pub fn report_text(session: &Session) -> String {
    report_text_with_language(session, ReportLanguage::En)
}

pub fn report_text_with_language(session: &Session, language: ReportLanguage) -> String {
    let metrics = &session.metrics;
    let total_tokens = total_tokens(session);
    let total_tools = metrics.tool_calls_ok + metrics.tool_calls_fail;
    let success_rate = success_rate(metrics.tool_calls_ok, total_tools);
    let avg_reason = if metrics.reasoning_blocks > 0 {
        metrics.reasoning_chars as f64 / metrics.reasoning_blocks as f64
    } else {
        0.0
    };
    let mut gaps = metrics.gaps_sec.clone();
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let sep = "━".repeat(60);
    let sub = "─".repeat(40);
    let mut out = String::new();

    out.push_str(&sep);
    out.push('\n');
    out.push_str(&format!(
        "  AGENTTRACE v{} — {}\n",
        VERSION,
        language.t(
            "AI Agent Session Performance Report",
            "AI 智能体会话性能报告"
        )
    ));
    out.push_str(&sep);
    out.push_str("\n\n");

    out.push_str(language.t("💸 MONEY WASTE\n", "💸 成本与 Token\n"));
    out.push_str(&sub);
    out.push('\n');
    out.push_str(&format!(
        "  {}:       {:>10}  {}\n",
        language.t("Input", "输入"),
        format_tokens(metrics.tokens_input),
        language.t("tokens", "Token")
    ));
    out.push_str(&format!(
        "  {}:      {:>10}  {}\n",
        language.t("Output", "输出"),
        format_tokens(metrics.tokens_output),
        language.t("tokens", "Token")
    ));
    if metrics.tokens_cache_w > 0 || metrics.tokens_cache_r > 0 {
        out.push_str(&format!(
            "  {}: {:>10}  {}\n",
            language.t("Cache write", "缓存写入"),
            format_tokens(metrics.tokens_cache_w),
            language.t("tokens", "Token")
        ));
        out.push_str(&format!(
            "  {}:  {:>10}  {}\n",
            language.t("Cache read", "缓存读取"),
            format_tokens(metrics.tokens_cache_r),
            language.t("tokens", "Token")
        ));
    }
    out.push_str("  ────────────────────────────────────\n");
    out.push_str(&format!(
        "  {}: {:>10}\n",
        language.t("Total tokens", "Token 总数"),
        format_tokens(total_tokens)
    ));
    out.push_str(&format!(
        "  {}: {:>12}  ({}: {}{})\n\n",
        language.t("Estimated cost", "估算成本"),
        format_cost(metrics.cost_estimated),
        language.t("model", "模型"),
        // rm-383: model strings are transcript-derived; sanitize for the
        // terminal like every other text-renderer field.
        sanitize_line_segment(&metrics.model_used),
        // rm-449 F3 / rm-760: an estimate made while a usage block was
        // present but unusable (`+usage_unusable` provenance suffix) is
        // marked — "no usage recorded" and "usage present, unusable"
        // are different facts on the human surface too.
        if metrics.provenance.tokens.contains("+usage_unusable") {
            language.t(" — usage present but unusable", " — 用量存在但不可用")
        } else {
            ""
        }
    ));

    out.push_str(language.t("📊 ACTIVITY\n", "📊 活动\n"));
    out.push_str(&sub);
    out.push('\n');
    out.push_str(&format!(
        "  {}:    {} {}  |  {} {}\n",
        language.t("Messages", "消息"),
        metrics.user_messages,
        language.t("user", "用户"),
        metrics.assistant_turns,
        language.t("turns", "轮次")
    ));
    out.push_str(&format!(
        "  {}:  {}\n",
        language.t("Tool calls", "工具调用"),
        metrics.tool_calls_total
    ));
    if total_tools > 0 {
        let rate = metrics.tool_calls_ok as f64 / total_tools as f64;
        let success_emoji = if rate < 0.70 {
            "🔴"
        } else if rate < 0.85 {
            "🟡"
        } else {
            "🟢"
        };
        out.push_str(&format!(
            "  {}:     {} ({}/{}) {}\n",
            language.t("Success", "成功率"),
            success_rate,
            metrics.tool_calls_ok,
            total_tools,
            success_emoji
        ));
    }
    out.push('\n');

    out.push_str(language.t("⏱️  LATENCY\n", "⏱️  延迟\n"));
    out.push_str(&sub);
    out.push('\n');
    if gaps.is_empty() {
        out.push_str(language.t("  (no gap data)\n", "  （无间隔数据）\n"));
    } else {
        out.push_str(&format!(
            "  {}:     {:.1}s\n",
            language.t("min", "最小"),
            gaps[0]
        ));
        out.push_str(&format!(
            "  {}:  {:.1}s\n",
            language.t("median", "中位数"),
            crate::percentile(&gaps, 0.50)
        ));
        out.push_str(&format!(
            "  p95:     {:.1}s\n",
            crate::percentile(&gaps, 0.95)
        ));
        out.push_str(&format!(
            "  {}:     {:.1}s\n",
            language.t("max", "最大"),
            gaps[gaps.len() - 1]
        ));
        out.push_str(&format!(
            "  {}:     {:.1}s\n",
            language.t("avg", "平均"),
            average(&gaps)
        ));
    }
    out.push_str(&format!(
        "  {}: {}\n\n",
        language.t("Duration", "总耗时"),
        fmt_duration_for_language(metrics.duration_sec, language)
    ));

    if !metrics.tool_usage.is_empty() {
        out.push_str(language.t("🔧 TOP TOOLS\n", "🔧 高频工具\n"));
        out.push_str(&sub);
        out.push('\n');
        for (tool, count) in top_tool_rows(&metrics.tool_usage).into_iter().take(8) {
            // rm-383: tool names are journal/transcript-derived (assess PoC:
            // --latest -f text rendered a raw OSC-52 clipboard-write from a
            // crafted tool name); no control byte may reach the terminal.
            out.push_str(&format!(
                "  {:<35} {:>4}\n",
                sanitize_line_segment(&tool),
                count
            ));
        }
        out.push('\n');
    }

    out.push_str(language.t("🧠 THINKING / COT\n", "🧠 推理 / 思维链\n"));
    out.push_str(&sub);
    out.push('\n');
    if metrics.reasoning_blocks > 0 {
        let (quality_emoji, quality_label) = if avg_reason < 400.0 {
            ("🔴", language.t("shallow", "浅"))
        } else if avg_reason < 800.0 {
            ("🟡", language.t("moderate", "中等"))
        } else {
            ("🟢", language.t("deep", "深入"))
        };
        out.push_str(&format!(
            "  {}: {}\n",
            language.t("Blocks", "块数"),
            metrics.reasoning_blocks
        ));
        out.push_str(&format!(
            "  {}:    {:.0} {}\n",
            language.t("Avg", "平均"),
            avg_reason,
            language.t("chars", "字符")
        ));
        out.push_str(&format!(
            "  {}:  {} {}\n",
            language.t("Total", "总计"),
            metrics.reasoning_chars,
            language.t("chars", "字符")
        ));
        out.push_str(&format!(
            "  {}: {} {}\n",
            language.t("Quality", "质量"),
            quality_emoji,
            quality_label
        ));
        if metrics.reasoning_redact > 0 {
            out.push_str(&format!(
                "  ⚠️  {} {}\n",
                metrics.reasoning_redact,
                language.t("blocks REDACTED", "个块已脱敏")
            ));
        }
    } else {
        out.push_str(language.t("  (no thinking blocks)\n", "  （无推理块）\n"));
    }
    out.push('\n');

    out.push_str(language.t("🚨 ANOMALIES\n", "🚨 异常\n"));
    out.push_str(&sub);
    out.push('\n');
    if session.anomalies.is_empty() {
        out.push_str(language.t("  ✅ No anomalies detected\n", "  ✅ 未检测到异常\n"));
    } else {
        for anomaly in &session.anomalies {
            out.push_str(&format!(
                "  {} [{}] {}: {}\n",
                anomaly_emoji(&anomaly.severity),
                severity_label_for_language(&anomaly.severity, language),
                anomaly_type_label_for_language(&anomaly.kind, language),
                // rm-383: anomaly detail strings embed parsed transcript
                // values; sanitize like every other text-renderer field.
                sanitize_line_segment(&anomaly_detail_for_language(anomaly, language))
            ));
        }
    }
    out.push('\n');

    out.push_str(language.t("💯 HEALTH SCORE\n", "💯 健康评分\n"));
    out.push_str(&sub);
    out.push('\n');
    out.push_str(&format!(
        "  {}  {}/100  {}\n\n",
        health_emoji(session.health),
        session.health,
        health_bar(session.health)
    ));
    out.push_str(&sep);
    out.push('\n');
    out
}

/// Upper bound on the overview `recent_sessions` list view. `--limit`
/// can shrink it; nothing grows it past the internal cap.
const RECENT_SESSIONS_MAX: usize = 10;

pub fn report_overview_json(overview: &Overview, sessions: &[Session]) -> String {
    report_overview_json_with_health(overview, sessions, None)
}

pub fn report_overview_json_with_health(
    overview: &Overview,
    sessions: &[Session],
    data_health: Option<&crate::DataHealth>,
) -> String {
    let ordered = canonical_sessions(sessions);
    let summary = overview_summary(overview, &ordered);
    let agents = group_items(&overview.by_agent, true);
    let models = group_items(&overview.by_model, false);
    let projects = group_items(&overview.by_project, false);
    let providers = group_items(&overview.by_provider, false);
    // rm-585 (spend-by-branch, first cut): same GroupOverview shape as
    // by_provider; missing / detached-HEAD / no-branch lanes roll up
    // under the explicit "unknown" bucket (compute_overview_iter).
    let branches = group_items(&overview.by_branch, false);
    let task_types = task_type_items(&overview.by_task_type, overview.total_cost);
    let top_cost_drivers = top_cost_driver_rows(&ordered, 3)
        .into_iter()
        .map(|row| {
            json!({
                "session": row.session,
                "cost": round4(row.cost),
                "share_pct": (row.share_pct * 10.0).round() / 10.0,
                "possible_driver": row.note,
            })
        })
        .map(strip_nulls)
        .collect::<Vec<_>>();
    let recent_sessions: Vec<Value> = ordered
        .iter()
        .take(10)
        .map(|session| {
            json!({
                "name": session.name,
                "source_tool": session.metrics.source_tool,
                "model": session.metrics.model_used,
                "cwd": optional_string(&session.cwd),
                "turns": session.metrics.assistant_turns,
                "tools": session.metrics.tool_calls_ok + session.metrics.tool_calls_fail,
                "tokens": total_tokens(session),
                "cost": round4(session.metrics.cost_estimated),
                "health": session.health,
                "anomalies": session.anomalies.len(),
                "highest_tool_authority": highest_authority_for_metrics(&session.metrics),
                "possible_cost_driver": possible_cost_driver_note_strict(session),
            })
        })
        .map(strip_nulls)
        .collect();
    let anomalies: Vec<Value> = overview
        .anomalies_top
        .iter()
        .take(50)
        .map(|item| {
            json!({
                "Session": item.session,
                "Type": item.kind,
                "Age": item.age,
            })
        })
        .collect();
    let mut payload = json!({
        "version": VERSION,
        "summary": summary,
        "failure_families": failure_families(&ordered),
        "surfaces": surfaces(&ordered),
        "by_agent": agents,
        "by_model": models,
        "by_project": projects,
        "by_provider": providers,
        "by_branch": branches,
        "by_task_type": task_types,
        "top_cost_drivers": top_cost_drivers,
        "recent_sessions": recent_sessions,
        "incident_timelines": incident_timelines(&ordered),
        "anomalies": anomalies,
        "data_health": data_health,
    });
    if let Value::Object(obj) = &mut payload {
        let summary = obj.remove("summary").unwrap_or(Value::Null);
        obj.insert("summary".to_string(), strip_nulls(summary));
    }
    serde_json::to_string_pretty(&payload).expect("overview report serializes")
}

/// `generated_at_override` pins `scope.generated_at` (used for `--demo` so the
/// JSON is byte-deterministic); `None` stamps the real wall clock.
/// `display_limit` caps the recent_sessions list view only (`--limit`);
/// every aggregate in the report covers all sessions (pass-3 P3-5,
/// pass-8 F8-1).
pub fn report_overview_json_with_context(
    overview: &Overview,
    sessions: &[Session],
    data_health: Option<&crate::DataHealth>,
    range: crate::TimeRange,
    includes_preserved_history: bool,
    generated_at_override: Option<&str>,
    display_limit: usize,
) -> String {
    let base = report_overview_json_with_health(overview, sessions, data_health);
    let mut payload: Value = serde_json::from_str(&base).expect("overview JSON is valid");
    if let Some(list) = payload
        .get_mut("recent_sessions")
        .and_then(Value::as_array_mut)
    {
        list.truncate(display_limit.min(RECENT_SESSIONS_MAX));
    }
    let mut scope = report_scope(sessions, range, includes_preserved_history);
    if let Some(timestamp) = generated_at_override {
        scope.generated_at = timestamp.to_string();
    }
    let context = json!({
        "scope": scope,
        "cost_audit": cost_audit(sessions),
        "recommendations": recommendations(sessions),
        "mcp_governance": mcp_governance(sessions),
        "context_trends": context_trends(sessions),
        "delivery_evidence": delivery_evidence(sessions),
    });
    if let (Value::Object(payload), Value::Object(context)) = (&mut payload, context) {
        payload.extend(context);
    }
    serde_json::to_string_pretty(&payload).expect("overview context serializes")
}

pub fn report_overview_text_with_context(
    overview: &Overview,
    sessions: &[Session],
    data_health: &crate::DataHealth,
    range: crate::TimeRange,
    includes_preserved_history: bool,
) -> String {
    let scope = report_scope(sessions, range, includes_preserved_history);
    let audit = cost_audit(sessions);
    let mut out = report_overview_text(overview, sessions);
    out.push_str("\n── Scope and confidence ──\n");
    out.push_str(&format!(
        "  Range: {} | sessions: {} | {} to {}\n",
        scope.range, scope.sessions_in_scope, scope.earliest_session_at, scope.latest_session_at
    ));
    // Out-of-scope sessions are disclosed instead of shrinking the
    // `discovered` denominator (pass-8 F8-2).
    out.push_str(&format!(
        "  Parse: {} | confidence: {}\n",
        parse_coverage_phrase(data_health, ", "),
        data_health.confidence
    ));
    if !data_health.line_skips.is_empty() {
        out.push_str(&format!(
            "  Dropped lines: {}\n",
            counts_cell(&data_health.line_skips)
        ));
    }
    if !data_health.disclosures.is_empty() {
        out.push_str(&format!(
            "  Disclosed facts: {}\n",
            counts_cell(&data_health.disclosures)
        ));
    }
    // rm-734: an excluded corpus must be named, never silent.
    for failure in &data_health.sqlite_read_failures {
        out.push_str(&format!(
            "  Sqlite source unreadable: {} ({}) — {} — its sessions are EXCLUDED\n",
            failure.path.display(),
            failure.source,
            failure.reason
        ));
    }
    out.push_str(&format!(
        "  Pricing: {} | exact={} fallback={} unknown={}\n",
        audit.pricing_source,
        audit.pricing_coverage.priced_sessions,
        audit.pricing_coverage.fallback_priced_sessions,
        audit.pricing_coverage.unpriced_or_unknown_sessions
    ));
    render_recommendations_text(&mut out, &recommendations(sessions));
    out
}

pub fn report_overview_markdown_with_context(
    overview: &Overview,
    sessions: &[Session],
    data_health: &crate::DataHealth,
    range: crate::TimeRange,
    includes_preserved_history: bool,
) -> String {
    let scope = report_scope(sessions, range, includes_preserved_history);
    let audit = cost_audit(sessions);
    let mut out = report_overview_markdown(overview, sessions);
    out.push_str("\n## Scope and confidence\n\n| Field | Value |\n|---|---|\n");
    out.push_str(&format!("| Range | {} |\n| Session window | {} → {} |\n| Parse coverage | {} |\n| Confidence | {} |\n| Pricing | {} |\n| Pricing coverage | exact: {}; fallback: {}; unknown: {} |\n", markdown_cell(&scope.range), markdown_cell(&scope.earliest_session_at), markdown_cell(&scope.latest_session_at), markdown_cell(&parse_coverage_phrase(data_health, "; ")), markdown_cell(&data_health.confidence), markdown_cell(&audit.pricing_source), audit.pricing_coverage.priced_sessions, audit.pricing_coverage.fallback_priced_sessions, audit.pricing_coverage.unpriced_or_unknown_sessions));
    if !data_health.line_skips.is_empty() {
        out.push_str(&format!(
            "| Dropped lines | {} |\n",
            markdown_cell(&counts_cell(&data_health.line_skips))
        ));
    }
    if !data_health.disclosures.is_empty() {
        out.push_str(&format!(
            "| Disclosed facts | {} |\n",
            markdown_cell(&counts_cell(&data_health.disclosures))
        ));
    }
    // rm-734 riding the landed rm-753 records: name every excluded
    // sqlite corpus (path, lane, reason).
    for failure in &data_health.sqlite_read_failures {
        out.push_str(&format!(
            "| Sqlite source unreadable | {} ({}) — {} — its sessions are EXCLUDED |\n",
            markdown_cell(&failure.path.display().to_string()),
            failure.source,
            markdown_cell(&failure.reason)
        ));
    }
    render_recommendations_markdown(&mut out, &recommendations(sessions));
    out
}

pub fn report_overview_html_with_context(
    overview: &Overview,
    sessions: &[Session],
    data_health: &crate::DataHealth,
    range: crate::TimeRange,
    includes_preserved_history: bool,
) -> String {
    let scope = report_scope(sessions, range, includes_preserved_history);
    let audit = cost_audit(sessions);
    let recommendations = recommendations(sessions);
    let mut appendix = String::from("<section><h2>Scope and confidence</h2><table><tbody>");
    appendix.push_str(&format!("<tr><th>Range</th><td>{}</td></tr><tr><th>Session window</th><td>{} → {}</td></tr><tr><th>Parse coverage</th><td>{}</td></tr><tr><th>Confidence</th><td>{}</td></tr><tr><th>Pricing</th><td>{}</td></tr>", html_escape(&scope.range), html_escape(&scope.earliest_session_at), html_escape(&scope.latest_session_at), html_escape(&parse_coverage_phrase(data_health, "; ")), html_escape(&data_health.confidence), html_escape(&audit.pricing_source)));
    if !data_health.line_skips.is_empty() {
        appendix.push_str(&format!(
            "<tr><th>Dropped lines</th><td>{}</td></tr>",
            html_escape(&counts_cell(&data_health.line_skips))
        ));
    }
    if !data_health.disclosures.is_empty() {
        appendix.push_str(&format!(
            "<tr><th>Disclosed facts</th><td>{}</td></tr>",
            html_escape(&counts_cell(&data_health.disclosures))
        ));
    }
    // rm-734 riding the landed rm-753 records: name every excluded
    // sqlite corpus (path, lane, reason).
    for failure in &data_health.sqlite_read_failures {
        appendix.push_str(&format!(
            "<tr><th>Sqlite source unreadable</th><td>{} ({}) — {} — its sessions are EXCLUDED</td></tr>",
            html_escape(&failure.path.display().to_string()),
            failure.source,
            html_escape(&failure.reason)
        ));
    }
    appendix.push_str("</tbody></table></section>");
    appendix.push_str("<section><h2>Prioritized recommendations</h2><table><thead><tr><th>Priority</th><th>Finding</th><th>Impact</th><th>Action</th></tr></thead><tbody>");
    for item in recommendations.iter().take(12) {
        appendix.push_str(&format!(
            "<tr><td>{}</td><td>{}: {}</td><td>${:.4}; {} tokens; {}</td><td>{}</td></tr>",
            html_escape(&item.priority),
            html_escape(&item.category),
            html_escape(&item.rationale),
            item.estimated_savings_usd,
            item.estimated_savings_tokens,
            html_escape(&item.confidence),
            html_escape(&item.action)
        ));
    }
    appendix.push_str("</tbody></table></section>");
    report_overview_html(overview, sessions).replacen("</main>", &(appendix + "</main>"), 1)
}

// rm-576: shareable usage card (deterministic SVG, no network).
//
// The card is a static SVG document rendered from the same aggregation
// pass as every other `--overview` renderer. It is deliberately boring:
// no external references, no fonts, no scripts, no clock of its own —
// the same corpus + range + theme always produces the same bytes (pinned
// by tests/svg_card_contract.rs). Every session-derived string goes
// through `sanitize_line_segment` (raw control bytes would be illegal
// XML 1.0) and then `html_escape` (the renderer family's entity
// escaper), so a model/project name that looks like markup renders inert.

/// Color scheme of the usage card. `Auto` ships the light palette as
/// presentation attributes and embeds a `prefers-color-scheme: dark`
/// override block, so one static file adapts wherever SVG media queries
/// are honored — and degrades gracefully to light where they are not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgCardTheme {
    Light,
    Dark,
    Auto,
}

impl SvgCardTheme {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(SvgCardTheme::Light),
            "dark" => Some(SvgCardTheme::Dark),
            "auto" => Some(SvgCardTheme::Auto),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SvgCardTheme::Light => "light",
            SvgCardTheme::Dark => "dark",
            SvgCardTheme::Auto => "auto",
        }
    }
}

impl std::str::FromStr for SvgCardTheme {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
            .ok_or_else(|| format!("card theme must be auto, dark, or light (got {value:?})"))
    }
}

struct CardPalette {
    bg: &'static str,
    panel: &'static str,
    fg: &'static str,
    muted: &'static str,
    bar: &'static str,
    bar_alt: &'static str,
    spark: &'static str,
    sep: &'static str,
}

const CARD_PALETTE_LIGHT: CardPalette = CardPalette {
    bg: "#f8fafc",
    panel: "#ffffff",
    fg: "#0f172a",
    muted: "#64748b",
    bar: "#2563eb",
    bar_alt: "#0ea5e9",
    spark: "#2563eb",
    sep: "#e2e8f0",
};

const CARD_PALETTE_DARK: CardPalette = CardPalette {
    bg: "#0b1220",
    panel: "#111a2e",
    fg: "#e2e8f0",
    muted: "#8aa0b8",
    bar: "#38bdf8",
    bar_alt: "#22d3ee",
    spark: "#38bdf8",
    sep: "#24344d",
};

/// Disclosure lines shared by the context and demo renderings.
struct CardFooter {
    range: String,
    window: String,
    pricing: String,
    confidence: String,
}

fn svg_text(value: &str) -> String {
    html_escape(&sanitize_line_segment(value))
}

fn card_cost(value: f64) -> String {
    if !value.is_finite() || value <= 0.0 {
        "$0.00".to_string()
    } else if value >= 1000.0 {
        format!("${value:.0}")
    } else if value >= 1.0 {
        format!("${value:.2}")
    } else {
        format!("${value:.4}")
    }
}

fn card_count(value: u64) -> String {
    // Plain integer with US grouping — no locale, no drift.
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let lead = digits.len() % 3;
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && i % 3 == lead {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn card_label(value: &str, max_chars: usize) -> String {
    // Char-boundary-safe truncation so multi-byte model names cannot
    // panic the renderer or exceed the drawn width.
    if value.chars().count() <= max_chars {
        svg_text(value)
    } else {
        let mut out: String = value.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('\u{2026}');
        svg_text(&out)
    }
}

/// Daily cost series over the trailing `days` days of the corpus's own
/// clock (anchored at the latest event timestamp, never `Utc::now()`),
/// so a fixed corpus always yields the same series.
fn card_daily_series(sessions: &[Session], days: usize) -> Vec<(chrono::NaiveDate, f64)> {
    let mut latest: Option<chrono::NaiveDate> = None;
    let mut buckets: std::collections::BTreeMap<chrono::NaiveDate, f64> =
        std::collections::BTreeMap::new();
    for session in sessions {
        // rm-911 (arm b): the bucket day comes from the cache-surviving
        // session bounds first. `SessionMetrics::timestamps` is
        // #[serde(skip)], so a warm-cache replay carries it EMPTY and
        // the timestamps-only derivation collapsed the whole series
        // onto the 1970-01-01 flat window (assess F1 PoC: c6-warm.svg).
        // `session_start`/`session_end` round-trip the session cache,
        // and lib.rs derives them from the sorted timestamp list on
        // every lane that has one, so cold renders keep their exact
        // buckets while warm replays stop collapsing. The raw-stamp max
        // stays as the fallback for lanes whose bound string does not
        // parse through the shared lenient arm — the same precedence
        // rm-502 set for otel's `session_bounds`.
        let day = crate::parse_ts(&session.metrics.session_end)
            .map(|ts| ts.naive_utc().date())
            .or_else(|| {
                session
                    .metrics
                    .timestamps
                    .iter()
                    .map(|ts| ts.naive_utc().date())
                    .max()
            });
        let Some(day) = day else { continue };
        latest = Some(match latest {
            Some(current) if current >= day => current,
            _ => day,
        });
        *buckets.entry(day).or_insert(0.0) += session.metrics.cost_estimated;
    }
    let mut series = Vec::with_capacity(days);
    match latest {
        Some(anchor) => {
            for offset in (0..days).rev() {
                let day = anchor - chrono::Duration::days(offset as i64);
                series.push((day, buckets.get(&day).copied().unwrap_or(0.0)));
            }
        }
        None => {
            // Corpus with no timestamps at all: a flat, still-valid chart
            // rather than a panic or a wall-clock leak.
            for _ in 0..days {
                series.push((
                    chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date"),
                    0.0,
                ));
            }
        }
    }
    series
}

fn card_bars(
    out: &mut String,
    p: &CardPalette,
    x: i32,
    y: i32,
    title: &str,
    groups: &[(String, GroupOverview)],
    cls: &dyn Fn(&str) -> String,
) {
    out.push_str(&format!(
        "<text{} x=\"{x}\" y=\"{y}\" fill=\"{}\" font-size=\"13\" font-family=\"ui-monospace, Menlo, monospace\" letter-spacing=\"0.5\">{}</text>\n",
        cls("card-fg"),
        p.fg,
        svg_text(title)
    ));
    let rows = groups.iter().take(5).collect::<Vec<_>>();
    let max = rows.iter().map(|(_, g)| g.cost).fold(0.0_f64, f64::max);
    for (i, (name, group)) in rows.iter().enumerate() {
        let row_y = y + 18 + (i as i32) * 26;
        out.push_str(&format!(
            "<text{} x=\"{x}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
            cls("card-muted"),
            row_y + 11,
            p.muted,
            card_label(name, 22)
        ));
        let ratio = if max > 0.0 { group.cost / max } else { 0.0 };
        let bar_w = (ratio * 150.0).round();
        out.push_str(&format!(
            "<rect{} x=\"{}\" y=\"{row_y}\" width=\"{bar_w:.0}\" height=\"13\" rx=\"3\" fill=\"{}\"/>\n",
            cls(if i % 2 == 0 { "card-bar" } else { "card-bar-alt" }),
            x + 160,
            if i % 2 == 0 { p.bar } else { p.bar_alt }
        ));
        out.push_str(&format!(
            "<text{} x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
            cls("card-fg"),
            x + 160 + 156,
            row_y + 11,
            p.fg,
            card_cost(group.cost)
        ));
    }
}

fn render_usage_card(
    overview: &Overview,
    canonical: &[Session],
    theme: SvgCardTheme,
    footer: &CardFooter,
) -> String {
    let p = match theme {
        SvgCardTheme::Dark => &CARD_PALETTE_DARK,
        SvgCardTheme::Light | SvgCardTheme::Auto => &CARD_PALETTE_LIGHT,
    };
    // Auto theme: presentation attributes below stay light and these
    // classes restyle the card where the viewer prefers dark. A sanitizer
    // that strips <style> leaves a correct light card behind.
    let cls: Box<dyn Fn(&str) -> String> = if theme == SvgCardTheme::Auto {
        Box::new(|c: &str| format!(" class=\"{c}\""))
    } else {
        Box::new(|_: &str| String::new())
    };

    let total_tokens = canonical
        .iter()
        .map(|s| (s.metrics.tokens_input as i128).max(0) + (s.metrics.tokens_output as i128).max(0))
        .fold(0_i128, i128::saturating_add);
    let total_tokens = total_tokens.clamp(0, u64::MAX as i128) as u64;
    let tool_calls = canonical
        .iter()
        .map(|s| s.metrics.tool_calls_total)
        .fold(0_usize, usize::saturating_add);
    let tool_calls = tool_calls as u64;

    let mut out = String::with_capacity(12 * 1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"920\" height=\"560\" viewBox=\"0 0 920 560\" role=\"img\" aria-label=\"agenttrace usage card\">\n");
    if theme == SvgCardTheme::Auto {
        out.push_str(
            "<style>\n@media (prefers-color-scheme: dark) {\n.card-bg { fill: #0b1220; }\n.card-panel { fill: #111a2e; }\n.card-fg { fill: #e2e8f0; }\n.card-muted { fill: #8aa0b8; }\n.card-bar { fill: #38bdf8; }\n.card-bar-alt { fill: #22d3ee; }\n.card-spark { stroke: #38bdf8; }\n.card-sep { fill: #24344d; }\n}\n</style>\n",
        );
    }
    out.push_str(&format!(
        "<rect class=\"card-bg\" x=\"0\" y=\"0\" width=\"920\" height=\"560\" rx=\"12\" fill=\"{}\"/>\n",
        p.bg
    ));
    out.push_str(&format!(
        "<rect class=\"card-panel\" x=\"16\" y=\"16\" width=\"888\" height=\"528\" rx=\"10\" fill=\"{}\" fill-opacity=\"0.6\"/>\n",
        p.panel
    ));
    out.push_str(&format!(
        "<text{} x=\"40\" y=\"48\" fill=\"{}\" font-size=\"22\" font-weight=\"bold\" font-family=\"ui-monospace, Menlo, monospace\">agenttrace \u{b7} usage card</text>\n",
        cls("card-fg"),
        p.fg
    ));
    out.push_str(&format!(
        "<text{} x=\"880\" y=\"48\" text-anchor=\"end\" fill=\"{}\" font-size=\"12\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
        cls("card-muted"),
        p.muted,
        svg_text(&footer.range)
    ));
    out.push_str(&format!(
        "<text{} x=\"40\" y=\"70\" fill=\"{}\" font-size=\"12\" font-family=\"ui-monospace, Menlo, monospace\">{} sessions \u{b7} {} tool calls \u{b7} {} tokens (input+output)</text>\n",
        cls("card-muted"),
        p.muted,
        card_count(canonical.len() as u64),
        card_count(tool_calls),
        card_count(total_tokens)
    ));
    out.push_str(&format!(
        "<rect{} x=\"40\" y=\"86\" width=\"840\" height=\"1\" fill=\"{}\"/>\n",
        cls("card-sep"),
        p.sep
    ));

    // Stat row: total cost is the headline number.
    out.push_str(&format!(
        "<text{} x=\"40\" y=\"118\" fill=\"{}\" font-size=\"11\" letter-spacing=\"1\" font-family=\"ui-monospace, Menlo, monospace\">TOTAL COST</text>\n",
        cls("card-muted"),
        p.muted
    ));
    out.push_str(&format!(
        "<text{} x=\"40\" y=\"148\" fill=\"{}\" font-size=\"26\" font-weight=\"bold\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
        cls("card-fg"),
        p.fg,
        card_cost(overview.total_cost)
    ));
    out.push_str(&format!(
        "<text{} x=\"300\" y=\"118\" fill=\"{}\" font-size=\"11\" letter-spacing=\"1\" font-family=\"ui-monospace, Menlo, monospace\">SESSIONS</text>\n",
        cls("card-muted"),
        p.muted
    ));
    out.push_str(&format!(
        "<text{} x=\"300\" y=\"148\" fill=\"{}\" font-size=\"26\" font-weight=\"bold\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
        cls("card-fg"),
        p.fg,
        card_count(canonical.len() as u64)
    ));
    out.push_str(&format!(
        "<text{} x=\"520\" y=\"118\" fill=\"{}\" font-size=\"11\" letter-spacing=\"1\" font-family=\"ui-monospace, Menlo, monospace\">TOOL CALLS</text>\n",
        cls("card-muted"),
        p.muted
    ));
    out.push_str(&format!(
        "<text{} x=\"520\" y=\"148\" fill=\"{}\" font-size=\"26\" font-weight=\"bold\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
        cls("card-fg"),
        p.fg,
        card_count(tool_calls)
    ));
    out.push_str(&format!(
        "<text{} x=\"720\" y=\"118\" fill=\"{}\" font-size=\"11\" letter-spacing=\"1\" font-family=\"ui-monospace, Menlo, monospace\">TOKENS IN+OUT</text>\n",
        cls("card-muted"),
        p.muted
    ));
    out.push_str(&format!(
        "<text{} x=\"720\" y=\"148\" fill=\"{}\" font-size=\"26\" font-weight=\"bold\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
        cls("card-fg"),
        p.fg,
        card_count(total_tokens)
    ));

    // Bars: models left, projects right (top 5 by cost each).
    card_bars(
        &mut out,
        p,
        40,
        180,
        "Top models (cost)",
        &sorted_model_groups(&overview.by_model),
        &cls,
    );
    card_bars(
        &mut out,
        p,
        480,
        180,
        "Top projects (cost)",
        &sorted_model_groups(&overview.by_project),
        &cls,
    );

    // Sparkline: daily spend over the corpus's trailing 14 days.
    out.push_str(&format!(
        "<text{} x=\"40\" y=\"356\" fill=\"{}\" font-size=\"13\" font-family=\"ui-monospace, Menlo, monospace\">Daily spend \u{b7} last 14 days \u{b7} binned by session end date</text>\n",
        cls("card-fg"),
        p.fg
    ));
    let series = card_daily_series(canonical, 14);
    let peak = series.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
    let base_y = 448.0_f64;
    let chart_h = 56.0_f64;
    let step = 840.0 / (series.len().saturating_sub(1).max(1)) as f64;
    let mut points = Vec::with_capacity(series.len());
    for (i, (_, value)) in series.iter().enumerate() {
        let x = 40.0 + (i as f64) * step;
        let y = base_y
            - if peak > 0.0 {
                (value / peak) * chart_h
            } else {
                0.0
            };
        points.push(format!("{x:.1},{y:.1}"));
    }
    if let (Some(first), Some(last)) = (series.first(), series.last()) {
        out.push_str(&format!(
            "<text{} x=\"40\" y=\"466\" fill=\"{}\" font-size=\"10\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
            cls("card-muted"),
            p.muted,
            first.0.format("%Y-%m-%d")
        ));
        out.push_str(&format!(
            "<text{} x=\"880\" y=\"466\" text-anchor=\"end\" fill=\"{}\" font-size=\"10\" font-family=\"ui-monospace, Menlo, monospace\">{}</text>\n",
            cls("card-muted"),
            p.muted,
            last.0.format("%Y-%m-%d")
        ));
    }
    out.push_str(&format!(
        "<line{} x1=\"40\" y1=\"{base_y:.1}\" x2=\"880\" y2=\"{base_y:.1}\" stroke=\"{}\" stroke-width=\"1\"/>\n",
        cls("card-sep"),
        p.sep
    ));
    out.push_str(&format!(
        "<polyline{} points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>\n",
        cls("card-spark"),
        points.join(" "),
        p.spark
    ));

    // Honesty footer (the same disclosure fields the other renderers
    // carry: window, pricing source, data confidence).
    out.push_str(&format!(
        "<rect{} x=\"40\" y=\"488\" width=\"840\" height=\"1\" fill=\"{}\"/>\n",
        cls("card-sep"),
        p.sep
    ));
    out.push_str(&format!(
        "<text{} x=\"40\" y=\"512\" fill=\"{}\" font-size=\"11\" font-family=\"ui-monospace, Menlo, monospace\">window {} \u{b7} pricing {} \u{b7} data confidence {}</text>\n",
        cls("card-muted"),
        p.muted,
        svg_text(&footer.window),
        svg_text(&footer.pricing),
        svg_text(&footer.confidence)
    ));
    out.push_str(&format!(
        "<text{} x=\"40\" y=\"532\" fill=\"{}\" font-size=\"10\" font-family=\"ui-monospace, Menlo, monospace\">generated by agenttrace \u{b7} static file \u{b7} offline \u{b7} byte-deterministic for a fixed corpus+range+theme</text>\n",
        cls("card-muted"),
        p.muted
    ));
    out.push_str("</svg>");
    out
}

/// The shareable usage card over the full corpus (library entry point).
pub fn report_overview_svg(
    overview: &Overview,
    sessions: &[Session],
    theme: SvgCardTheme,
) -> String {
    let canonical = canonical_sessions(sessions);
    let scope = report_scope(sessions, crate::TimeRange::All, false);
    let audit = cost_audit(sessions);
    let footer = CardFooter {
        range: scope.range,
        window: format!(
            "{} \u{2192} {}",
            scope.earliest_session_at, scope.latest_session_at
        ),
        pricing: audit.pricing_source,
        confidence: "not assessed".to_string(),
    };
    render_usage_card(overview, &canonical, theme, &footer)
}

/// The usage card with the same context footer the HTML renderer carries
/// (selected range, window, pricing source, data-health confidence).
pub fn report_overview_svg_with_context(
    overview: &Overview,
    sessions: &[Session],
    data_health: &crate::DataHealth,
    range: crate::TimeRange,
    includes_preserved_history: bool,
    theme: SvgCardTheme,
) -> String {
    let canonical = canonical_sessions(sessions);
    let scope = report_scope(sessions, range, includes_preserved_history);
    let audit = cost_audit(sessions);
    let footer = CardFooter {
        range: scope.range,
        window: format!(
            "{} \u{2192} {}",
            scope.earliest_session_at, scope.latest_session_at
        ),
        pricing: audit.pricing_source,
        confidence: data_health.confidence.clone(),
    };
    render_usage_card(overview, &canonical, theme, &footer)
}

fn render_recommendations_text(out: &mut String, items: &[crate::Recommendation]) {
    if items.is_empty() {
        return;
    }
    out.push_str("\n── Prioritized recommendations ──\n");
    for item in items.iter().take(12) {
        out.push_str(&format!(
            "  [{}] {} — {} | ${:.4}, {} tokens | {}\n    Action: {}\n    Verify: {}\n",
            item.priority,
            item.title,
            item.rationale,
            item.estimated_savings_usd,
            item.estimated_savings_tokens,
            item.confidence,
            item.action,
            item.validation_command
        ));
    }
}

fn render_recommendations_markdown(out: &mut String, items: &[crate::Recommendation]) {
    if items.is_empty() {
        return;
    }
    out.push_str("\n## Prioritized recommendations\n\n| Priority | Finding | Estimated impact | Confidence | Action |\n|---|---|---:|---|---|\n");
    for item in items.iter().take(12) {
        out.push_str(&format!(
            "| {} | {} | ${:.4}; {} tokens | {} | {} |\n",
            item.priority,
            markdown_cell(&item.title),
            item.estimated_savings_usd,
            item.estimated_savings_tokens,
            markdown_cell(&item.confidence),
            markdown_cell(&item.action)
        ));
    }
}

/// rm-569: load a `--baseline` file with truthful, attributed errors.
///
/// The baseline gate is the CI flagship (docs/guides/ci-integration.md): a
/// wrong file must fail loudly instead of zero-filling into a fabricated
/// +100% regression. Input classes covered: unreadable (io error attributed
/// to the path), non-UTF-8, non-JSON, and valid JSON that is not an overview
/// report (missing `summary` object). The version cross-check against
/// the current report lives in `add_baseline_comparison`.
///
/// Review fix (2026-10-06): the `version` string and the summary's compared
/// numeric fields (`total_duration_seconds`, `total_cost`, `total_tokens`) are
/// required up front, so an empty or partial summary cannot zero-fill its way
/// into a fabricated +100% regression.
fn load_baseline_report(baseline_path: &str) -> anyhow::Result<Value> {
    let baseline_text = match fs::read_to_string(baseline_path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::InvalidData => {
            anyhow::bail!("--baseline: {baseline_path} is not valid UTF-8: {err}");
        }
        Err(err) => {
            anyhow::bail!("--baseline: failed to read baseline file {baseline_path}: {err}");
        }
    };
    let baseline: Value = serde_json::from_str(&baseline_text)
        .map_err(|err| anyhow::anyhow!("--baseline: {baseline_path} is not valid JSON: {err}"))?;
    let Some(summary) = baseline.get("summary").and_then(Value::as_object) else {
        anyhow::bail!(
            "--baseline: {baseline_path} is not an overview report: no \"summary\" object found; \
             expected the JSON written by `agenttrace --overview -f json -o <baseline>` \
             (an object with version and summary sections)"
        );
    };
    for key in ["total_duration_seconds", "total_cost", "total_tokens"] {
        if !summary.get(key).is_some_and(Value::is_number) {
            anyhow::bail!(
                "--baseline: {baseline_path} summary is missing the compared field \"{key}\"; \
                 the comparison would zero-fill it into a fabricated delta — regenerate the \
                 baseline with `agenttrace --overview -f json -o <baseline>`"
            );
        }
        // rm-697: `is_number` admits negatives, and `delta_pct` divides
        // by the baseline — total_cost = -100 sign-inverted a real
        // +$100 regression into a PASSING negative delta (live PoC:
        // delta_pct -100.036). Totals are magnitudes: admit only
        // finite non-negative numbers, with the same flag/path/
        // regenerate-hint shape as the rm-569 rejections above.
        if let Some(number) = summary.get(key).and_then(Value::as_f64) {
            if !number.is_finite() || number < 0.0 {
                anyhow::bail!(
                    "--baseline: {baseline_path} summary field \"{key}\" is negative or \
                     not finite ({number}); a negative baseline total sign-inverts the delta \
                     comparison into a pass — regenerate the baseline with \
                     `agenttrace --overview -f json -o <baseline>`"
                );
            }
        }
    }
    if baseline.get("version").and_then(Value::as_str).is_none() {
        anyhow::bail!(
            "--baseline: {baseline_path} is not an overview report: no \"version\" string found; \
             every overview JSON writes one — regenerate it with \
             `agenttrace --overview -f json -o <baseline>`"
        );
    }
    Ok(baseline)
}

pub fn add_baseline_comparison(
    report_json: &str,
    baseline_path: &str,
    thresholds: BaselineThresholds,
) -> anyhow::Result<(String, BaselineBreaches)> {
    let mut report: Value = serde_json::from_str(report_json)?;
    let baseline: Value = load_baseline_report(baseline_path)?;
    // rm-569/rm-389: a baseline from a different report version can zero-fill
    // into a fabricated regression; reject the mismatch loudly instead.
    if let (Some(current), Some(base)) = (report.get("version"), baseline.get("version")) {
        if current != base {
            anyhow::bail!(
                "--baseline: {baseline_path} version mismatch: baseline version {base} \
                 != current report version {current}; regenerate the baseline with the \
                 current agenttrace version (--overview -f json -o <baseline>)"
            );
        }
    }
    let summary = report
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let base_summary = baseline
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let duration_delta = delta_pct(
        number(&summary, "total_duration_seconds"),
        number(&base_summary, "total_duration_seconds"),
    );
    let cost_delta = delta_pct(
        number(&summary, "total_cost"),
        number(&base_summary, "total_cost"),
    );
    let token_delta = delta_pct(
        number(&summary, "total_tokens"),
        number(&base_summary, "total_tokens"),
    );
    let new_tools = diff_array(&report, &baseline, "/surfaces/tools");
    let new_files = diff_array(&report, &baseline, "/surfaces/files");

    let comparison = json!({
        "baseline_path": baseline_path,
        "thresholds": {
            "max_duration_delta_pct": thresholds.max_duration_delta_pct,
            "max_cost_delta_pct": thresholds.max_cost_delta_pct,
            "max_token_delta_pct": thresholds.max_token_delta_pct,
        },
        "current": baseline_snapshot(&report),
        "baseline": baseline_snapshot(&baseline),
        "duration_delta_pct": duration_delta,
        "cost_delta_pct": cost_delta,
        "token_delta_pct": token_delta,
        "slower_than_baseline": duration_delta > thresholds.max_duration_delta_pct,
        "cost_above_threshold": cost_delta > thresholds.max_cost_delta_pct,
        "tokens_above_threshold": token_delta > thresholds.max_token_delta_pct,
        "new_failure_families": diff_array(&report, &baseline, "/failure_families"),
        "broader_tool_surface": !new_tools.is_empty(),
        "new_tools": new_tools,
        "broader_file_surface": !new_files.is_empty(),
        "new_files": new_files,
        "new_tool_authority_categories": diff_array(&report, &baseline, "/surfaces/authority_categories"),
        "new_high_authority_tool_use": diff_array(&report, &baseline, "/surfaces/high_authority_tools"),
    });
    if let Value::Object(obj) = &mut report {
        obj.insert("baseline_comparison".to_string(), comparison);
    }
    let breaches = BaselineBreaches {
        slower_than_baseline: duration_delta > thresholds.max_duration_delta_pct,
        cost_above_threshold: cost_delta > thresholds.max_cost_delta_pct,
        tokens_above_threshold: token_delta > thresholds.max_token_delta_pct,
    };
    Ok((serde_json::to_string_pretty(&report)?, breaches))
}

pub fn report_overview_text(overview: &Overview, sessions: &[Session]) -> String {
    let ordered = canonical_sessions(sessions);
    let authority = overview_authority_summary(&ordered);
    let sep = "━".repeat(70);
    let mut out = String::new();

    out.push_str(&sep);
    out.push('\n');
    out.push_str(&format!(
        "  AGENTTRACE v{} — Global Overview  ({} Sessions)\n",
        VERSION, overview.total_sessions
    ));
    out.push_str(&sep);
    out.push_str("\n\n");

    let healthy_pct = (overview.healthy * 100)
        .checked_div(overview.total_sessions)
        .unwrap_or(0);
    let warning_pct = (overview.warning * 100)
        .checked_div(overview.total_sessions)
        .unwrap_or(0);
    let critical_pct = (overview.critical * 100)
        .checked_div(overview.total_sessions)
        .unwrap_or(0);
    out.push_str(&format!(
        "  Total Sessions:     {}\n",
        overview.total_sessions
    ));
    out.push_str(&format!(
        "  🟢 Healthy:   {} ({}%)\n",
        format_count(overview.healthy),
        healthy_pct
    ));
    out.push_str(&format!(
        "  🟡 Warning:   {} ({}%)\n",
        format_count(overview.warning),
        warning_pct
    ));
    out.push_str(&format!(
        "  🔴 Critical:   {} ({}%)\n",
        format_count(overview.critical),
        critical_pct
    ));
    out.push_str(&format!(
        "  💰 Total estimated cost:      {}\n\n",
        format_cost(overview.total_cost)
    ));

    let timelines = overview_incident_timelines(&ordered, 3);
    if !timelines.is_empty() {
        out.push_str("  ── Incident timeline ──\n");
        let mut rendered = 0;
        'timeline: for timeline in timelines {
            for item in timeline.items {
                out.push_str(&format!(
                    "    {:<30} {}: {}\n",
                    text_cell(&timeline.session, 30),
                    item.label,
                    text_cell(&item.detail, text_incident_detail_limit(&item.label))
                ));
                rendered += 1;
                if rendered >= 5 {
                    break 'timeline;
                }
            }
        }
        out.push('\n');
    }

    if authority.has_data {
        out.push_str("  ── Tool authority ──\n");
        if !authority.highest.is_empty() {
            out.push_str(&format!("    Highest category: {}\n", authority.highest));
        }
        if !authority.counts.is_empty() {
            for line in text_wrapped_key_values(
                "Authority category counts",
                &text_authority_count_values(&authority.counts),
                96,
            ) {
                out.push_str(&format!("    {line}\n"));
            }
        }
        if !authority.high_tools.is_empty() {
            for line in text_wrapped_key_values(
                "High-authority tools",
                &text_tool_values(&authority.top_high_tools()),
                96,
            ) {
                out.push_str(&format!("    {line}\n"));
            }
        }
        out.push('\n');
    }

    let notes = overview_cost_driver_notes(&ordered, 3);
    if !notes.is_empty() {
        out.push_str("  ── Possible cost drivers ──\n");
        for note in notes {
            out.push_str(&format!(
                "    {:<30} {}\n",
                text_cell(&note.session, 30),
                text_cell(&note.note, 80)
            ));
        }
        out.push('\n');
    }

    // rm-245: ranked top-cost sessions beside the cost-audit evidence -
    // what actually moved the bill, with each session's share of the
    // total and the strict driver note when one exists.
    let drivers = top_cost_driver_rows(&ordered, 3);
    if !drivers.is_empty() {
        out.push_str("  ── Top cost drivers (share of total) ──\n");
        for row in drivers {
            out.push_str(&format!(
                "    {:<30} {:>8} {:>5.1}%{}\n",
                text_cell(&row.session, 30),
                format_cost(row.cost),
                row.share_pct,
                row.note
                    .as_deref()
                    .map(|note| format!("  {note}"))
                    .unwrap_or_default()
            ));
        }
        out.push('\n');
    }

    out.push_str("  ── By Agent ──\n");
    push_text_group_family(&mut out, &overview.by_agent, 30, |name| {
        tool_display_name(name)
    });

    out.push_str("  ── By Model ──\n");
    push_text_group_family(&mut out, &overview.by_model, 25, |name| name.clone());

    // rm-245: vendor and task-type dimensions beside by-model. Providers
    // come from the pricing catalog row that prices each model;
    // unresolvable models land in the explicit "unknown" bucket.
    out.push_str("  ── By Provider ──\n");
    push_text_group_family(&mut out, &overview.by_provider, 25, |name| name.clone());

    // rm-585 (spend-by-branch, first cut): branch values come only
    // from the claude-code gitBranch envelope; missing, detached-HEAD
    // and other lanes share the explicit "unknown" bucket. The family
    // renders at width 40, not the by-model/by-provider 25: a branch
    // label is a git ref the operator must be able to IDENTIFY — the
    // rm-585 every-surface pin (by_branch_renders_on_every_surface)
    // holds the full `feature/rm585-spend-by-branch` name against this
    // lane, and cutting it at 25 (`feature/rm585-spend-by...`) was the
    // one behavior rm-895's shared-family refactor never pinned (its
    // contract is the GROUP_TAKE row cap + disclosed cut + shared
    // comparator — width stays a per-family parameter, agent=30).
    // text_cell still sanitizes (rm-239) and still truncates genuinely
    // over-long refs at the 40-rune bound, so the lane stays bounded
    // and control-byte-safe.
    out.push_str("  ── By Branch ──\n");
    push_text_group_family(&mut out, &overview.by_branch, 40, |name| name.clone());

    out.push_str("  ── By Task Type ──\n");
    for (task_type, group) in &overview.by_task_type {
        out.push_str(&format!(
            "    {:<15} {:>4} Sessions  {:>8}  in {:>9}  out {}\n",
            text_cell(task_type, 15),
            format_count(group.sessions),
            format_cost(group.cost),
            format_tokens(group.tokens_input),
            format_tokens(group.tokens_output)
        ));
    }
    out.push('\n');

    out.push_str("  ── Recent Anomalies ──\n");
    if overview.anomalies_top.is_empty() {
        out.push_str("    ✅ No anomalies\n");
    } else {
        for anomaly in overview.anomalies_top.iter().take(8) {
            out.push_str(&format!(
                "    ⚠️  {:<30} {}\n",
                text_cell(&anomaly.session, 30),
                anomaly_type_label(&anomaly.kind)
            ));
        }
    }
    out.push('\n');
    out.push_str(&sep);
    out.push('\n');
    out
}

pub fn report_overview_markdown(overview: &Overview, sessions: &[Session]) -> String {
    let ordered = canonical_sessions(sessions);
    let summary = overview_summary(overview, &ordered);
    let authority = overview_authority_summary(&ordered);
    let trend = analyze_health_trend(sessions);
    let mut out = String::new();

    out.push_str("# agenttrace overview\n\n");
    out.push_str("| Metric | Value |\n|---|---:|\n");
    out.push_str(&format!(
        "| Sessions | {} |\n",
        format_count(overview.total_sessions)
    ));
    out.push_str(&format!(
        "| Healthy / Warning / Critical | {} / {} / {} |\n",
        format_count(overview.healthy),
        format_count(overview.warning),
        format_count(overview.critical)
    ));
    out.push_str(&format!(
        "| Average health | {:.1} |\n",
        number_obj(&summary, "avg_health")
    ));
    out.push_str(&format!(
        "| Health Trend | {} |\n",
        markdown_cell(&trend.message)
    ));
    out.push_str(&format!(
        "| Total estimated cost | {} |\n",
        format_cost(overview.total_cost)
    ));
    out.push_str(&format!(
        "| Total tokens | {} |\n",
        format_tokens(number_obj(&summary, "total_tokens") as i64)
    ));
    out.push_str(&format!(
        "| Tool failures | {:.0} / {:.0} ({:.1}%) |\n\n",
        number_obj(&summary, "tool_failures"),
        number_obj(&summary, "tool_calls"),
        number_obj(&summary, "tool_fail_rate")
    ));

    if authority.has_data {
        out.push_str("## Tool authority\n\n");
        out.push_str("| Metric | Value |\n|---|---:|\n");
        if !authority.highest.is_empty() {
            out.push_str(&format!(
                "| Highest category | `{}` |\n",
                markdown_inline_code(&authority.highest)
            ));
        }
        if !authority.high_tools.is_empty() {
            out.push_str(&format!(
                "| High-authority tools | {} |\n",
                report_markdown_code_list(&authority.top_high_tools())
            ));
        }
        if !authority.counts.is_empty() {
            out.push_str("\n### Authority category counts\n\n");
            out.push_str("| Authority category | Count |\n|---|---:|\n");
            for item in &authority.counts {
                out.push_str(&format!(
                    "| `{}` | {} |\n",
                    markdown_inline_code(&item.category),
                    item.count
                ));
            }
            out.push('\n');
        }
    }

    let cost_notes = overview_cost_driver_notes(&ordered, 6);
    if !cost_notes.is_empty() {
        out.push_str("## Possible cost drivers\n\n");
        for note in cost_notes {
            out.push_str(&format!(
                "- **{}**: {}\n",
                markdown_cell(&note.session),
                markdown_cell(&note.note)
            ));
        }
        out.push('\n');
    }

    out.push_str("## Incident timeline\n\n");
    let timelines = overview_incident_timelines(&ordered, 6);
    if timelines.is_empty() {
        out.push_str("No incident timeline evidence yet.\n\n");
    } else {
        out.push_str("| Session | Signal | Evidence | Severity |\n|---|---|---|---|\n");
        for timeline in timelines {
            for item in timeline.items {
                out.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    markdown_cell(&timeline.session),
                    markdown_cell(&item.label),
                    markdown_cell(&item.detail),
                    markdown_cell(&severity_label(&item.severity))
                ));
            }
        }
        out.push('\n');
    }

    out.push_str("## By agent\n\n");
    out.push_str("| Agent | Sessions | Cost |\n|---|---:|---:|\n");
    for (agent, group) in sorted_group_overview(&overview.by_agent)
        .into_iter()
        .take(GROUP_TAKE)
    {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            markdown_cell(&tool_display_name(agent)),
            format_count(group.sessions),
            format_cost(group.cost)
        ));
    }
    if overview.by_agent.len() > GROUP_TAKE {
        out.push_str(&format!(
            "*… +{} more not shown*\n",
            overview.by_agent.len() - GROUP_TAKE
        ));
    }

    // rm-245: vendor and task-type dimensions beside the by-agent view.
    out.push_str("\n## By provider\n\n");
    out.push_str("| Provider | Sessions | Cost |\n|---|---:|---:|\n");
    for (provider, group) in sorted_group_overview(&overview.by_provider)
        .into_iter()
        .take(GROUP_TAKE)
    {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            markdown_cell(provider),
            format_count(group.sessions),
            format_cost(group.cost)
        ));
    }
    if overview.by_provider.len() > GROUP_TAKE {
        out.push_str(&format!(
            "*… +{} more not shown*\n",
            overview.by_provider.len() - GROUP_TAKE
        ));
    }

    // rm-585 (spend-by-branch, first cut): every session lands in
    // exactly one branch bucket; "unknown" covers missing,
    // detached-HEAD and lanes without a branch concept.
    out.push_str("\n## By branch\n\n");
    out.push_str("| Branch | Sessions | Cost |\n|---|---:|---:|\n");
    for (branch, group) in sorted_group_overview(&overview.by_branch)
        .into_iter()
        .take(GROUP_TAKE)
    {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            markdown_cell(branch),
            format_count(group.sessions),
            format_cost(group.cost)
        ));
    }
    if overview.by_branch.len() > GROUP_TAKE {
        out.push_str(&format!(
            "*… +{} more not shown*\n",
            overview.by_branch.len() - GROUP_TAKE
        ));
    }

    out.push_str("\n## By task type\n\n");
    out.push_str(
        "| Task type | Sessions | Tokens in | Tokens out | Cost |\n|---|---:|---:|---:|---:|\n",
    );
    for (task_type, group) in &overview.by_task_type {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            markdown_cell(task_type),
            format_count(group.sessions),
            format_tokens(group.tokens_input),
            format_tokens(group.tokens_output),
            format_cost(group.cost)
        ));
    }

    let drivers = top_cost_driver_rows(&ordered, 3);
    if !drivers.is_empty() {
        out.push_str("\n## Top cost drivers\n\n");
        out.push_str("| Session | Cost | Share | Possible driver |\n|---|---:|---:|---|\n");
        for row in drivers {
            out.push_str(&format!(
                "| {} | {} | {:.1}% | {} |\n",
                markdown_cell(&row.session),
                format_cost(row.cost),
                row.share_pct,
                markdown_cell(row.note.as_deref().unwrap_or(""))
            ));
        }
    }

    out.push_str("\n## Recent sessions\n\n");
    out.push_str(
        "| Session | Source | Model | Health | Cost | Anomalies |\n|---|---|---|---:|---:|---:|\n",
    );
    for session in ordered.iter().take(10) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            markdown_cell(&session.name),
            markdown_cell(&tool_display_name(&session.metrics.source_tool)),
            markdown_cell(&session.metrics.model_used),
            session.health,
            format_cost(session.metrics.cost_estimated),
            format_count(session.anomalies.len())
        ));
    }

    out.push_str("\n## Recent anomalies\n\n");
    if overview.anomalies_top.is_empty() {
        out.push_str("No anomalies detected.\n");
        return out;
    }
    out.push_str("| Session | Type | Age |\n|---|---|---|\n");
    for anomaly in overview.anomalies_top.iter().take(10) {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            markdown_cell(&anomaly.session),
            markdown_cell(&anomaly_type_label(&anomaly.kind)),
            markdown_cell(&anomaly.age)
        ));
    }
    out
}

pub fn report_overview_html(overview: &Overview, sessions: &[Session]) -> String {
    let ordered = canonical_sessions(sessions);
    let summary = overview_summary(overview, &ordered);
    let authority = overview_authority_summary(&ordered);
    let trend = analyze_health_trend(&ordered);
    let agents = sorted_group_overview(&overview.by_agent);
    let models = sorted_group_overview(&overview.by_model);
    let providers = sorted_group_overview(&overview.by_provider);
    let branches = sorted_group_overview(&overview.by_branch);

    let mut out = String::new();
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };

    w("<!doctype html>".to_string());
    w("<html lang=\"en\">".to_string());
    w("<head>".to_string());
    w("<meta charset=\"utf-8\">".to_string());
    w("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">".to_string());
    w("<title>agenttrace overview</title>".to_string());
    w("<link rel=\"icon\" href=\"data:,\">".to_string());
    w("<style>".to_string());
    w(":root{color-scheme:dark;--bg:#07090b;--panel:#101419;--line:#273039;--text:#f4f0dd;--muted:#a9a391;--green:#54ff00;--cyan:#00d8ff;--amber:#ffb000;--red:#ff4a4a}".to_string());
    w("*{box-sizing:border-box}body{margin:0;background:linear-gradient(180deg,#0b0f12,#050607);color:var(--text);font:15px/1.55 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}".to_string());
    w("main{max-width:1180px;margin:0 auto;padding:32px 18px 48px}header{display:flex;justify-content:space-between;gap:24px;align-items:flex-start;border-bottom:1px solid var(--line);padding-bottom:24px;margin-bottom:24px}".to_string());
    w("h1{font-size:clamp(42px,7vw,88px);line-height:.9;margin:0;letter-spacing:0}h2{margin:0 0 14px;font-size:20px;color:var(--cyan)}p{margin:10px 0 0;color:var(--muted)}".to_string());
    w(".brand{color:var(--green);font-weight:800}.meta{text-align:right;color:var(--muted)}.grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:1px;background:var(--line);border:1px solid var(--line);margin:24px 0}.metric{background:var(--panel);padding:18px;min-height:120px}.metric span{display:block;color:var(--muted);font-size:12px;text-transform:uppercase}.metric strong{display:block;margin-top:12px;font-size:30px;color:var(--green)}.warn strong{color:var(--amber)}.bad strong{color:var(--red)}".to_string());
    w("section{border:1px solid var(--line);background:rgba(16,20,25,.78);padding:20px;margin-top:20px}table{width:100%;border-collapse:collapse}th,td{padding:10px;border-bottom:1px solid var(--line);text-align:left;vertical-align:top}th{color:var(--muted);font-size:12px;text-transform:uppercase}td.num,th.num{text-align:right}.health-good{color:var(--green)}.health-warn{color:var(--amber)}.health-bad{color:var(--red)}code{color:var(--cyan)}@media(max-width:760px){header{display:block}.meta{text-align:left;margin-top:16px}.grid{grid-template-columns:1fr}table{font-size:13px}}".to_string());
    w("</style>".to_string());
    w("</head>".to_string());
    w("<body>".to_string());
    w("<main>".to_string());
    w("<header>".to_string());
    w("<div><div class=\"brand\">agenttrace</div><h1>AI agent session overview</h1><p>Static report generated from local coding-agent traces.</p></div>".to_string());
    w(format!(
        "<div class=\"meta\">v{}<br>{} Sessions<br><code>agenttrace --overview -f html</code></div>",
        html_escape(VERSION),
        overview.total_sessions
    ));
    w("</header>".to_string());
    w("<div class=\"grid\" aria-label=\"summary metrics\">".to_string());
    w(format!(
        "<div class=\"metric\"><span>Sessions</span><strong>{}</strong><p>{} Healthy / {} Warning / {} Critical</p></div>",
        overview.total_sessions, overview.healthy, overview.warning, overview.critical
    ));
    w(format!(
        "<div class=\"metric\"><span>Total tokens</span><strong>{}</strong><p>+ live</p></div>",
        format_tokens(number_obj(&summary, "total_tokens") as i64)
    ));
    w(format!(
        "<div class=\"metric\"><span>Average health</span><strong>{:.1}</strong><p>Fleet quality score</p></div>",
        number_obj(&summary, "avg_health")
    ));
    w(format!(
        "<div class=\"metric\"><span>Total estimated cost</span><strong>{}</strong><p>Estimated session cost</p></div>",
        format_cost(overview.total_cost)
    ));
    w(format!(
        "<div class=\"metric {}\"><span>Tool failures</span><strong>{:.0}/{:.0}</strong><p>{:.1}% failure rate</p></div>",
        html_escape(failure_class(number_obj(&summary, "tool_fail_rate"))),
        number_obj(&summary, "tool_failures"),
        number_obj(&summary, "tool_calls"),
        number_obj(&summary, "tool_fail_rate")
    ));
    w("</div>".to_string());

    if authority.has_data {
        w("<section><h2>Tool authority</h2>".to_string());
        if !authority.highest.is_empty() {
            w(format!(
                "<p><strong>Highest category</strong>: <code>{}</code></p>",
                html_escape(&authority.highest)
            ));
        }
        if !authority.counts.is_empty() {
            w("<table><caption>Authority category counts</caption><thead><tr><th>Authority category</th><th class=\"num\">Count</th></tr></thead><tbody>".to_string());
            for item in &authority.counts {
                w(format!(
                    "<tr><td><code>{}</code></td><td class=\"num\">{}</td></tr>",
                    html_escape(&item.category),
                    item.count
                ));
            }
            w("</tbody></table>".to_string());
        }
        if !authority.high_tools.is_empty() {
            w(format!(
                "<p><strong>High-authority tools</strong>: {}</p>",
                report_html_code_list(&authority.top_high_tools())
            ));
        }
        w("</section>".to_string());
    }

    let cost_notes = overview_cost_driver_notes(&ordered, 8);
    if !cost_notes.is_empty() {
        w("<section><h2>Possible cost drivers</h2><table><thead><tr><th>Session</th><th>Evidence</th></tr></thead><tbody>".to_string());
        for note in cost_notes {
            w(format!(
                "<tr><td>{}</td><td>{}</td></tr>",
                html_escape(&note.session),
                html_escape(&note.note)
            ));
        }
        w("</tbody></table></section>".to_string());
    }
    if ordered.len() > 1 {
        w(format!(
            "<section><h2>Health Trend</h2><p>{}</p></section>",
            html_escape(&trend.message)
        ));
    }

    w("<section><h2>Incident timeline</h2>".to_string());
    let timelines = overview_incident_timelines(&ordered, 8);
    if timelines.is_empty() {
        w("<p>No incident timeline evidence yet.</p>".to_string());
    } else {
        w("<table><thead><tr><th>Session</th><th>Signal</th><th>Evidence</th><th>Severity</th></tr></thead><tbody>".to_string());
        for timeline in timelines {
            for item in timeline.items {
                w(format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    html_escape(&timeline.session),
                    html_escape(&item.label),
                    html_escape(&item.detail),
                    html_escape(&severity_label(&item.severity))
                ));
            }
        }
        w("</tbody></table>".to_string());
    }
    w("</section>".to_string());

    w("<section><h2>Recent sessions</h2><table><thead><tr><th>Session</th><th>Source</th><th>Model</th><th class=\"num\">Total tokens</th><th class=\"num\">Cost</th><th class=\"num\">Health</th><th class=\"num\">Anomalies</th></tr></thead><tbody>".to_string());
    for session in ordered.iter().take(20) {
        w(format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num {}\">{}</td><td class=\"num\">{}</td></tr>",
            html_escape(&session.name),
            html_escape(&tool_display_name(&session.metrics.source_tool)),
            html_escape(&session.metrics.model_used),
            format_tokens(total_tokens(session)),
            format_cost(session.metrics.cost_estimated),
            html_escape(health_class(session.health)),
            session.health,
            format_count(session.anomalies.len())
        ));
    }
    w("</tbody></table></section>".to_string());

    w("<section><h2>By agent</h2><table><thead><tr><th>Agent</th><th class=\"num\">Sessions</th><th class=\"num\">Cost</th></tr></thead><tbody>".to_string());
    for (agent, group) in agents.iter().take(GROUP_TAKE) {
        w(format!(
            "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
            html_escape(&tool_display_name(agent)),
            format_count(group.sessions),
            format_cost(group.cost)
        ));
    }
    w(group_section_close(overview.by_agent.len()));

    w("<section><h2>By model</h2><table><thead><tr><th>Model</th><th class=\"num\">Sessions</th><th class=\"num\">Cost</th></tr></thead><tbody>".to_string());
    for (model, group) in models.iter().take(GROUP_TAKE) {
        w(format!(
            "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
            html_escape(model),
            format_count(group.sessions),
            format_cost(group.cost)
        ));
    }
    w(group_section_close(overview.by_model.len()));

    // rm-245: vendor and task-type dimensions beside by-model.
    w("<section><h2>By provider</h2><table><thead><tr><th>Provider</th><th class=\"num\">Sessions</th><th class=\"num\">Cost</th></tr></thead><tbody>".to_string());
    for (provider, group) in providers.iter().take(GROUP_TAKE) {
        w(format!(
            "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
            html_escape(provider),
            format_count(group.sessions),
            format_cost(group.cost)
        ));
    }
    w(group_section_close(overview.by_provider.len()));

    // rm-585 (spend-by-branch, first cut).
    w("<section><h2>By branch</h2><table><thead><tr><th>Branch</th><th class=\"num\">Sessions</th><th class=\"num\">Cost</th></tr></thead><tbody>".to_string());
    for (branch, group) in branches.iter().take(GROUP_TAKE) {
        w(format!(
            "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
            html_escape(branch),
            format_count(group.sessions),
            format_cost(group.cost)
        ));
    }
    w(group_section_close(overview.by_branch.len()));

    w("<section><h2>By task type</h2><table><thead><tr><th>Task type</th><th class=\"num\">Sessions</th><th class=\"num\">Tokens in</th><th class=\"num\">Tokens out</th><th class=\"num\">Cost</th></tr></thead><tbody>".to_string());
    for (task_type, group) in &overview.by_task_type {
        w(format!(
            "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
            html_escape(task_type),
            format_count(group.sessions),
            format_tokens(group.tokens_input),
            format_tokens(group.tokens_output),
            format_cost(group.cost)
        ));
    }
    w("</tbody></table></section>".to_string());

    let drivers = top_cost_driver_rows(&ordered, 3);
    if !drivers.is_empty() {
        w("<section><h2>Top cost drivers</h2><table><thead><tr><th>Session</th><th class=\"num\">Cost</th><th class=\"num\">Share</th><th>Possible driver</th></tr></thead><tbody>".to_string());
        for row in drivers {
            w(format!(
                "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{:.1}%</td><td>{}</td></tr>",
                html_escape(&row.session),
                format_cost(row.cost),
                row.share_pct,
                html_escape(row.note.as_deref().unwrap_or(""))
            ));
        }
        w("</tbody></table></section>".to_string());
    }

    w("<section><h2>Recent anomalies</h2>".to_string());
    if overview.anomalies_top.is_empty() {
        w("<p>No anomalies detected.</p>".to_string());
    } else {
        w(
            "<table><thead><tr><th>Session</th><th>Type</th><th>Age</th></tr></thead><tbody>"
                .to_string(),
        );
        for anomaly in overview.anomalies_top.iter().take(20) {
            w(format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&anomaly.session),
                html_escape(&anomaly_type_label(&anomaly.kind)),
                html_escape(&anomaly.age)
            ));
        }
        w("</tbody></table>".to_string());
    }
    w("</section>".to_string());
    w("</main>".to_string());
    w("</body>".to_string());
    w("</html>".to_string());
    out
}

pub fn report_compare(sessions: &[Session], model: &str) -> String {
    report_compare_with_language(sessions, model, ReportLanguage::En)
}

pub fn report_compare_with_language(
    sessions: &[Session],
    model: &str,
    language: ReportLanguage,
) -> String {
    let sep = "━".repeat(76);
    let mut out = String::new();
    out.push_str(&sep);
    out.push('\n');
    out.push_str(&format!(
        "  AGENTTRACE — {}  ({}: {})\n",
        language.t("Multi-Session Comparison", "多会话对比"),
        language.t("model", "模型"),
        model
    ));
    out.push_str(&sep);
    out.push('\n');
    out.push('\n');
    out.push_str(&format!(
        "  {:<28} {:>4} {:>5} {:>5} {:>5} {:>9} {:>7}\n",
        language.t("SESSION", "会话"),
        language.t("TURNS", "轮次"),
        language.t("TOOLS", "工具"),
        language.t("SUCC%", "成功%"),
        language.t("FAIL", "失败"),
        language.t("COST", "成本"),
        language.t("HEALTH", "健康")
    ));
    out.push_str(&format!("  {}\n", "─".repeat(70)));
    for session in sessions {
        let metrics = &session.metrics;
        let total_tools = metrics.tool_calls_ok + metrics.tool_calls_fail;
        let success_rate = if total_tools > 0 {
            format!(
                "{:.0}%",
                metrics.tool_calls_ok as f64 / total_tools as f64 * 100.0
            )
        } else {
            "N/A".to_string()
        };
        // rm-239 rider (d80f6a25): session names are transcript/filename-
        // derived; the compare text row is the last default text lane
        // that printed them raw (assess PoC: `--compare` over a hostile
        // journal emitted one OSC-52 clipboard-write sequence). Route
        // through the shared sanitizer before layout so the 27-rune
        // budget applies to sanitized text.
        let name = truncate_runes(&sanitize_line_segment(&session.name), 27);
        out.push_str(&format!(
            "  {:<28} {:>4} {:>5} {:>5} {:>5} {:>9} {} {}/100\n",
            name,
            format_count(metrics.assistant_turns),
            format_count(metrics.tool_calls_total),
            success_rate,
            format_count(metrics.tool_calls_fail),
            format_cost(metrics.cost_estimated),
            health_emoji(session.health),
            session.health
        ));
    }
    out.push_str(&sep);
    out.push('\n');
    out
}

pub fn report_compare_json(sessions: &[Session]) -> String {
    if sessions.is_empty() {
        return "[]".to_string();
    }
    let mut out = String::new();
    out.push_str("[\n");
    for (index, session) in sessions.iter().enumerate() {
        let metrics = &session.metrics;
        let total_tools = metrics.tool_calls_ok + metrics.tool_calls_fail;
        let success_rate = if total_tools > 0 {
            format!(
                "{:.0}%",
                metrics.tool_calls_ok as f64 / total_tools as f64 * 100.0
            )
        } else {
            "N/A".to_string()
        };
        out.push_str("  {\n");
        out.push_str(&format!("    \"name\": {},\n", json_string(&session.name)));
        out.push_str("    \"metrics\": {\n");
        out.push_str(&format!("      \"turns\": {},\n", metrics.assistant_turns));
        out.push_str(&format!("      \"tools\": {},\n", metrics.tool_calls_total));
        out.push_str(&format!(
            "      \"success_rate\": {},\n",
            json_string(&success_rate)
        ));
        out.push_str(&format!("      \"fail\": {},\n", metrics.tool_calls_fail));
        out.push_str(&format!(
            "      \"cost\": {}\n",
            json_float(metrics.cost_estimated)
        ));
        out.push_str("    },\n");
        out.push_str(&format!("    \"health\": {}\n", session.health));
        out.push_str("  }");
        if index + 1 < sessions.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push(']');
    out
}

fn health_emoji(health: i32) -> &'static str {
    if health >= 80 {
        "🟢"
    } else if health >= 50 {
        "🟡"
    } else {
        "🔴"
    }
}

fn health_bar(health: i32) -> String {
    let blocks = (health / 5).clamp(0, 20) as usize;
    let empty = 20usize.saturating_sub(blocks);
    format!("[{}{}]", "█".repeat(blocks), "░".repeat(empty))
}

fn success_rate(ok: usize, total: usize) -> String {
    if total == 0 {
        "N/A".to_string()
    } else {
        format!("{:.0}%", ok as f64 / total as f64 * 100.0)
    }
}

fn top_tool_rows(tools: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut items: Vec<_> = tools
        .iter()
        .map(|(key, value)| (key.clone(), *value))
        .collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    items
}

fn anomaly_emoji(severity: &str) -> &'static str {
    match severity {
        "high" => "🔴",
        "medium" => "🟡",
        "low" => "🟢",
        _ => "",
    }
}

fn severity_label(severity: &str) -> String {
    severity_label_for_language(severity, ReportLanguage::En)
}

fn severity_label_for_language(severity: &str, language: ReportLanguage) -> String {
    match severity.to_ascii_lowercase().as_str() {
        "critical" => language.t("CRITICAL", "严重").to_string(),
        "high" => language.t("HIGH", "高").to_string(),
        "warning" | "medium" => language.t("MEDIUM", "中").to_string(),
        "good" | "low" => language.t("LOW", "低").to_string(),
        _ => severity.to_ascii_uppercase(),
    }
}

fn anomaly_type_label(kind: &str) -> String {
    anomaly_type_label_for_language(kind, ReportLanguage::En)
}

fn anomaly_type_label_for_language(kind: &str, language: ReportLanguage) -> String {
    if language == ReportLanguage::Zh {
        return match kind {
            "hanging" => "卡顿".to_string(),
            "latency" => "延迟".to_string(),
            "tool_failures" => "工具失败".to_string(),
            "shallow_thinking" => "推理过浅".to_string(),
            "redacted" | "redaction" => "推理脱敏".to_string(),
            "no_tools" => "未使用工具".to_string(),
            other => other.replace('_', " "),
        };
    }
    match kind {
        "hanging" => "hanging".to_string(),
        "latency" => "latency".to_string(),
        "tool_failures" => "tool failures".to_string(),
        "shallow_thinking" => "shallow thinking".to_string(),
        "redacted" | "redaction" => "redacted thinking".to_string(),
        "no_tools" => "no tools".to_string(),
        other => other.replace('_', " "),
    }
}

pub(crate) fn json_string(value: &str) -> String {
    go_json_escape(serde_json::to_string(value).expect("string serializes"))
}

pub(crate) fn go_json_escape(value: String) -> String {
    value
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

pub(crate) fn json_float(value: f64) -> String {
    // Total by construction: non-finite values (reachable only through
    // hostile catalog data, guarded upstream in convert_litellm) render
    // as null instead of panicking, and data_health flags the session
    // via `non_finite_costs` (pass-8 F8-5).
    if !value.is_finite() {
        return "null".to_string();
    }
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string())
    }
}

fn write_usize_map_json(out: &mut String, values: &BTreeMap<String, usize>, base_indent: usize) {
    if values.is_empty() {
        out.push_str("{}");
        return;
    }
    let current_indent = "  ".repeat(base_indent);
    let item_indent = "  ".repeat(base_indent + 1);
    out.push_str("{\n");
    for (index, (key, value)) in values.iter().enumerate() {
        out.push_str(&item_indent);
        out.push_str(&json_string(key));
        out.push_str(": ");
        out.push_str(&value.to_string());
        if index + 1 < values.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&current_indent);
    out.push('}');
}

fn write_anomalies_json(
    out: &mut String,
    anomalies: &[Anomaly],
    language: ReportLanguage,
    base_indent: usize,
) {
    if anomalies.is_empty() {
        out.push_str("[]");
        return;
    }
    let array_indent = "  ".repeat(base_indent);
    let object_indent = "  ".repeat(base_indent + 1);
    let field_indent = "  ".repeat(base_indent + 2);
    out.push_str("[\n");
    for (index, anomaly) in anomalies.iter().enumerate() {
        out.push_str(&object_indent);
        out.push_str("{\n");
        out.push_str(&field_indent);
        out.push_str("\"detail\": ");
        out.push_str(&json_string(&anomaly_detail_for_language(
            anomaly, language,
        )));
        out.push_str(",\n");
        out.push_str(&field_indent);
        out.push_str("\"severity\": ");
        out.push_str(&json_string(&anomaly.severity));
        out.push_str(",\n");
        out.push_str(&field_indent);
        out.push_str("\"type\": ");
        out.push_str(&json_string(&anomaly.kind));
        out.push('\n');
        out.push_str(&object_indent);
        out.push('}');
        if index + 1 < anomalies.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&array_indent);
    out.push(']');
}

fn fmt_duration_for_language(seconds: f64, language: ReportLanguage) -> String {
    match language {
        ReportLanguage::En => fmt_duration(seconds),
        ReportLanguage::Zh => {
            if seconds < 60.0 {
                format!("{seconds:.0}秒")
            } else if seconds < 3600.0 {
                format!("{:.1}分钟", seconds / 60.0)
            } else {
                let hours = (seconds / 3600.0) as i64;
                let minutes = ((seconds as i64) % 3600) / 60;
                format!("{hours}小时{minutes}分钟")
            }
        }
    }
}

fn anomaly_detail_for_language(anomaly: &Anomaly, language: ReportLanguage) -> String {
    if language == ReportLanguage::En {
        return anomaly.detail.clone();
    }
    match anomaly.kind.as_str() {
        "shallow_thinking" => {
            if let Some(avg) = parse_avg_reasoning_chars(&anomaly.detail) {
                if anomaly.severity == "high" {
                    format!("平均推理 = {avg:.0} 字符 (极浅)")
                } else {
                    format!("平均推理 = {avg:.0} 字符")
                }
            } else {
                anomaly.detail.clone()
            }
        }
        "no_tools" => "无工具调用 — 纯对话会话".to_string(),
        "hanging" => anomaly
            .detail
            .strip_suffix('s')
            .map(|detail| detail.replace(" gap(s) >60s, max=", "个间隔 >60秒, 最长=") + "秒")
            .unwrap_or_else(|| anomaly.detail.clone()),
        "latency" => anomaly
            .detail
            .strip_prefix("p95 latency = ")
            .and_then(|value| value.strip_suffix('s'))
            .map(|value| format!("P95延迟 = {value}秒"))
            .unwrap_or_else(|| anomaly.detail.clone()),
        "tool_failures" => anomaly.detail.replace(" failed", " 失败"),
        "redaction" | "redacted" => anomaly
            .detail
            .strip_suffix(" block(s) redacted")
            .map(|count| format!("{count} 思维块已脱敏"))
            .unwrap_or_else(|| anomaly.detail.clone()),
        _ => anomaly.detail.clone(),
    }
}

fn parse_avg_reasoning_chars(detail: &str) -> Option<f64> {
    let value = detail.strip_prefix("avg reasoning = ")?;
    let value = value.split_whitespace().next()?;
    value.parse().ok()
}

fn overview_summary(overview: &Overview, sessions: &[Session]) -> Value {
    // Saturating: per-session totals can legitimately sit at i64::MAX, so a
    // plain `.sum()` overflowed across sessions (debug panic, release -2).
    let total_tokens: i64 = sessions
        .iter()
        .map(total_tokens)
        .fold(0i64, i64::saturating_add);
    let total_tools: usize = sessions
        .iter()
        .map(|s| s.metrics.tool_calls_ok + s.metrics.tool_calls_fail)
        .sum();
    let failed_tools: usize = sessions.iter().map(|s| s.metrics.tool_calls_fail).sum();
    let total_duration: f64 = sessions.iter().map(|s| s.metrics.duration_sec).sum();
    let anomalies_total = overview.anomalies_top.len();
    let authority_counts = authority_counts(sessions);
    let highest = highest_authority(sessions);
    let trend = analyze_health_trend_full(sessions);
    json!({
        "total_sessions": overview.total_sessions,
        "healthy": overview.healthy,
        "warning": overview.warning,
        "critical": overview.critical,
        "avg_health": round4(average_health(sessions)),
        "total_cost": round4(overview.total_cost),
        "total_duration_seconds": round4(total_duration),
        "total_tokens": total_tokens,
        "tool_calls": total_tools,
        "tool_failures": failed_tools,
        "tool_fail_rate": if total_tools > 0 { round4(failed_tools as f64 / total_tools as f64 * 100.0) } else { 0.0 },
        "anomalies_total": anomalies_total,
        "anomalies_returned": anomalies_total.min(50),
        "anomalies_truncated": anomalies_total > 50,
        "health_trend": {
            "direction": trend.direction,
            "regressing": trend.regressing,
            "avg_health": round4(trend.avg_health),
            "message": trend.message,
            "points": trend.points.iter().map(|point| json!({"name": point.name, "health": point.health, "cost": round4(point.cost)})).collect::<Vec<_>>(),
        },
        "tool_authority": {
            "highest": highest,
            "counts": authority_counts,
        },
    })
}

fn group_items(groups: &BTreeMap<String, GroupOverview>, agent_display: bool) -> Vec<Value> {
    let mut items: Vec<_> = groups
        .iter()
        .map(|(name, group)| {
            json!({
                "name": if agent_display { tool_display_name(name) } else { name.clone() },
                "sessions": group.sessions,
                "cost": round4(group.cost),
            })
        })
        .collect();
    if agent_display {
        items.sort_by(|a, b| {
            number_value(b, "sessions")
                .partial_cmp(&number_value(a, "sessions"))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    number_value(b, "cost")
                        .partial_cmp(&number_value(a, "cost"))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| string_value(a, "name").cmp(&string_value(b, "name")))
        });
    } else {
        items.sort_by(|a, b| {
            number_value(b, "cost")
                .partial_cmp(&number_value(a, "cost"))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    number_value(b, "sessions")
                        .partial_cmp(&number_value(a, "sessions"))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| string_value(a, "name").cmp(&string_value(b, "name")))
        });
    }
    items
}

/// rm-245: per-task-type rollup entries. Keys are the fixed taxonomy
/// (coding / debugging / planning) produced by `infer_task_type`;
/// tokens ride along because "what kind of work burned the tokens" is
/// the question this dimension answers.
fn task_type_items(groups: &BTreeMap<String, TaskTypeOverview>, total_cost: f64) -> Vec<Value> {
    let mut items: Vec<_> = groups
        .iter()
        .map(|(task_type, group)| {
            json!({
                "task_type": task_type,
                "sessions": group.sessions,
                "tokens": {
                    "input": group.tokens_input.max(0),
                    "output": group.tokens_output.max(0),
                },
                "cost": round4(group.cost),
                "share_pct": if total_cost > 0.0 {
                    (group.cost / total_cost * 1000.0).round() / 10.0
                } else {
                    0.0
                },
            })
        })
        .collect();
    items.sort_by(|a, b| {
        number_value(b, "cost")
            .partial_cmp(&number_value(a, "cost"))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| string_value(a, "task_type").cmp(&string_value(b, "task_type")))
    });
    items
}

/// rm-245: ranked top-cost sessions with their share of total spend and
/// the strict possible-driver note when one exists. Deterministic:
/// cost desc, then session name. Total comes from the same session set,
/// so shares always sum to 100% over the full ranking (the rendered
/// list is capped, the denominator is not).
struct CostDriverRow {
    session: String,
    cost: f64,
    share_pct: f64,
    note: Option<String>,
}

fn top_cost_driver_rows(sessions: &[Session], limit: usize) -> Vec<CostDriverRow> {
    if limit == 0 {
        return Vec::new();
    }
    let total: f64 = sessions
        .iter()
        .map(|session| session.metrics.cost_estimated)
        .sum();
    let mut ranked: Vec<&Session> = sessions.iter().collect();
    ranked.sort_by(|a, b| {
        b.metrics
            .cost_estimated
            .total_cmp(&a.metrics.cost_estimated)
            .then_with(|| a.name.cmp(&b.name))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|session| CostDriverRow {
            session: session.name.clone(),
            cost: session.metrics.cost_estimated,
            share_pct: if total > 0.0 {
                session.metrics.cost_estimated / total * 100.0
            } else {
                0.0
            },
            note: possible_cost_driver_note_strict(session),
        })
        .collect()
}

fn surfaces(sessions: &[Session]) -> Value {
    let mut tools = BTreeMap::new();
    let mut files = BTreeMap::new();
    let mut authority = BTreeMap::new();
    for session in sessions {
        for key in session.metrics.tool_usage.keys() {
            tools.insert(key.clone(), ());
        }
        for key in session.metrics.file_usage.keys() {
            files.insert(report_file_surface(key), ());
        }
        for (key, count) in &session.metrics.tool_authority {
            if *count > 0 {
                authority.insert(key.clone(), *count);
            }
        }
    }
    let tool_names = sorted_keys(&tools);
    json!({
        "tools": tool_names,
        "files": sorted_keys(&files),
        "authority_categories": sorted_keys(&authority),
        "high_authority_tools": high_authority_tools(&tool_names),
    })
}

fn failure_families(sessions: &[Session]) -> Vec<String> {
    let mut families = BTreeSet::new();
    for session in sessions {
        for anomaly in &session.anomalies {
            families.insert(anomaly.kind.clone());
        }
    }
    sorted_set(families)
}

fn report_file_surface(value: &str) -> String {
    serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|json| {
            json.get("path")
                .or_else(|| json.get("file_path"))
                .or_else(|| json.get("file"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| value.to_string())
}

fn incident_timelines(sessions: &[Session]) -> Vec<Value> {
    overview_incident_timelines(sessions, 10)
        .into_iter()
        .map(|timeline| {
            json!({
                "session": timeline.session,
                "items": timeline.items.into_iter().map(|item| json!({
                    "kind": item.kind,
                    "label": item.label,
                    "detail": item.detail,
                    "severity": item.severity,
                })).collect::<Vec<_>>(),
            })
        })
        .collect()
}

fn high_authority_tools(tools: &[String]) -> Vec<String> {
    tools
        .iter()
        .filter(|tool| is_high_authority_category(&classify_tool_authority_name(tool)))
        .cloned()
        .collect()
}

fn classify_tool_authority_name(name: &str) -> String {
    classify_tool_authority(&ToolCall {
        name: name.to_string(),
        ..ToolCall::default()
    })
}

fn authority_counts(sessions: &[Session]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for session in sessions {
        for (authority, count) in &session.metrics.tool_authority {
            if *count > 0 {
                *counts.entry(authority.clone()).or_insert(0) += count;
            }
        }
    }
    counts
}

fn highest_authority(sessions: &[Session]) -> String {
    let mut highest = String::new();
    for session in sessions {
        let current = highest_authority_for_metrics(&session.metrics);
        highest = crate::higher_tool_authority(&highest, &current);
    }
    highest
}

fn top_tools(tools: &BTreeMap<String, usize>) -> BTreeMap<String, usize> {
    let mut items: Vec<_> = tools.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    items
        .into_iter()
        .take(10)
        .map(|(key, value)| (key.clone(), *value))
        .collect()
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn delta_pct(current: f64, baseline: f64) -> f64 {
    if baseline == 0.0 {
        if current == 0.0 {
            0.0
        } else {
            100.0
        }
    } else {
        round4((current - baseline) / baseline * 100.0)
    }
}

fn diff_array(current: &Value, baseline: &Value, pointer: &str) -> Vec<String> {
    let current_items = current
        .pointer(pointer)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let baseline_items = baseline
        .pointer(pointer)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let baseline_set: BTreeSet<_> = baseline_items
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    current_items
        .iter()
        .filter_map(Value::as_str)
        .filter(|item| !baseline_set.contains(*item))
        .map(str::to_string)
        .collect()
}

fn baseline_snapshot(report: &Value) -> Value {
    json!({
        "duration_seconds": round4(report.pointer("/summary/total_duration_seconds").and_then(Value::as_f64).unwrap_or(0.0)),
        "cost": round4(report.pointer("/summary/total_cost").and_then(Value::as_f64).unwrap_or(0.0)),
        "tokens": report.pointer("/summary/total_tokens").and_then(Value::as_i64).unwrap_or(0),
        "failure_families": sorted_string_array(report.pointer("/failure_families")),
        "tools": sorted_string_array(report.pointer("/surfaces/tools")),
        "files": sorted_string_array(report.pointer("/surfaces/files")),
        "authority_categories": sorted_string_array(report.pointer("/surfaces/authority_categories")),
        "high_authority_tools": sorted_string_array(report.pointer("/surfaces/high_authority_tools")),
    })
}

fn sorted_string_array(value: Option<&Value>) -> Vec<String> {
    let mut items: Vec<String> = value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    items.sort();
    items.dedup();
    items
}

fn number(map: &Map<String, Value>, key: &str) -> f64 {
    map.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn number_obj(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn number_value(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn string_value(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn optional_string(value: &str) -> Value {
    if value.is_empty() {
        Value::Null
    } else {
        Value::String(value.to_string())
    }
}

fn strip_nulls(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter_map(|(key, value)| {
                    if value.is_null() {
                        None
                    } else {
                        Some((key, strip_nulls(value)))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(strip_nulls).collect()),
        other => other,
    }
}

fn tool_display_name(name: &str) -> String {
    match name {
        "hermes_jsonl" => "Hermes Agent (JSONL)".to_string(),
        "hermes_json" => "Hermes Agent (.json)".to_string(),
        "hermes_db" => "Hermes Agent (DB)".to_string(),
        "claude_code" => "Claude Code".to_string(),
        "claude_code_jsonl" => "Claude Code (JSONL)".to_string(),
        "codex_cli" => "Codex CLI".to_string(),
        "codex_rollout" => "Codex CLI (Rollout)".to_string(),
        "gemini_cli" => "Gemini CLI".to_string(),
        "qwen_code" => "Qwen Code".to_string(),
        "opencode" => "OpenCode".to_string(),
        "opencode_db" => "OpenCode (DB)".to_string(),
        "openclaw" => "OpenClaw".to_string(),
        "copilot_cli" => "Copilot CLI".to_string(),
        "kimi_cli" => "Kimi CLI".to_string(),
        "pi" => "Pi".to_string(),
        "pi_senpi" => "Pi (senpi)".to_string(),
        "pi_omo" => "Pi (omo)".to_string(),
        "oh_my_pi" => "Oh My Pi".to_string(),
        "aider" => "Aider".to_string(),
        "cursor" => "Cursor".to_string(),
        "cline" => "Cline".to_string(),
        "generic" => "Generic JSON/JSONL".to_string(),
        other => other.to_string(),
    }
}

#[derive(Debug)]
struct AuthorityCount {
    category: String,
    count: usize,
}

#[derive(Debug)]
struct OverviewAuthority {
    highest: String,
    counts: Vec<AuthorityCount>,
    high_tools: Vec<String>,
    has_data: bool,
}

const HIGH_TOOLS_DISPLAY_LIMIT: usize = 15;

impl OverviewAuthority {
    fn top_high_tools(&self) -> Vec<String> {
        let mut tools = self
            .high_tools
            .iter()
            .take(HIGH_TOOLS_DISPLAY_LIMIT)
            .cloned()
            .collect::<Vec<_>>();
        let hidden = self.high_tools.len().saturating_sub(tools.len());
        if hidden > 0 {
            tools.push(format!("+{hidden} more"));
        }
        tools
    }
}

#[derive(Debug)]
struct CostDriverNote {
    session: String,
    note: String,
}

#[derive(Debug)]
struct HealthTrend {
    message: String,
}

#[derive(Debug)]
struct FullHealthTrend {
    direction: String,
    regressing: bool,
    avg_health: f64,
    message: String,
    points: Vec<TrendPoint>,
}

#[derive(Debug)]
struct TrendPoint {
    name: String,
    health: i32,
    cost: f64,
}

#[derive(Debug)]
struct IncidentTimelineItem {
    kind: String,
    label: String,
    detail: String,
    severity: String,
}

#[derive(Debug)]
struct IncidentTimelineSummary {
    session: String,
    items: Vec<IncidentTimelineItem>,
}

fn overview_authority_summary(sessions: &[Session]) -> OverviewAuthority {
    let mut counts = BTreeMap::new();
    let mut tool_surface = BTreeMap::new();
    let mut highest = String::new();
    for session in sessions {
        for (tool, calls) in &session.metrics.tool_usage {
            *tool_surface.entry(tool.clone()).or_insert(0usize) += *calls;
        }
        for (category, count) in &session.metrics.tool_authority {
            if *count > 0 {
                *counts.entry(category.clone()).or_insert(0) += *count;
                highest = crate::higher_tool_authority(&highest, category);
            }
        }
        highest = crate::higher_tool_authority(&highest, &session.metrics.highest_authority);
    }
    let counts_vec = counts
        .iter()
        .filter(|(category, count)| !category.is_empty() && **count > 0)
        .map(|(category, count)| AuthorityCount {
            category: category.clone(),
            count: *count,
        })
        .collect::<Vec<_>>();
    let tool_names = sorted_keys(&tool_surface);
    let mut high_tools = high_authority_tools(&tool_names);
    high_tools.sort_by(|a, b| tool_surface[b].cmp(&tool_surface[a]).then_with(|| a.cmp(b)));
    let has_data = !highest.is_empty() || !counts_vec.is_empty() || !high_tools.is_empty();
    OverviewAuthority {
        highest,
        counts: counts_vec,
        high_tools,
        has_data,
    }
}

fn overview_cost_driver_notes(sessions: &[Session], limit: usize) -> Vec<CostDriverNote> {
    if limit == 0 {
        return Vec::new();
    }
    let mut notes = Vec::new();
    for session in sessions {
        if let Some(note) = possible_cost_driver_note_strict(session) {
            notes.push(CostDriverNote {
                session: session.name.clone(),
                note,
            });
            if notes.len() >= limit {
                break;
            }
        }
    }
    notes
}

fn analyze_health_trend(sessions: &[Session]) -> HealthTrend {
    HealthTrend {
        message: analyze_health_trend_full(sessions).message,
    }
}

fn analyze_health_trend_full(sessions: &[Session]) -> FullHealthTrend {
    if sessions.is_empty() {
        return FullHealthTrend {
            direction: String::new(),
            regressing: false,
            avg_health: 0.0,
            message: "No session data available".to_string(),
            points: Vec::new(),
        };
    }

    let mut ordered = sessions.to_vec();
    ordered.sort_by(|a, b| {
        match (
            parse_ts(&a.metrics.session_start),
            parse_ts(&b.metrics.session_start),
        ) {
            (Some(a_ts), Some(b_ts)) if a_ts != b_ts => a_ts.cmp(&b_ts),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    });

    let n = ordered.len().min(10);
    let recent = &ordered[ordered.len() - n..];
    let points = recent
        .iter()
        .map(|session| TrendPoint {
            name: session.name.clone(),
            health: session.health,
            cost: session.metrics.cost_estimated,
        })
        .collect::<Vec<_>>();
    let health_points: Vec<i32> = points.iter().map(|point| point.health).collect();
    let mut smoothed = Vec::with_capacity(health_points.len());
    for index in 0..health_points.len() {
        let start = index.saturating_sub(1);
        let end = (index + 1).min(health_points.len() - 1);
        let mut sum = 0;
        let mut count = 0;
        for health in &health_points[start..=end] {
            sum += *health;
            count += 1;
        }
        smoothed.push(sum as f64 / count as f64);
    }

    let avg_health = health_points
        .iter()
        .map(|health| *health as f64)
        .sum::<f64>()
        / health_points.len() as f64;
    let direction = if smoothed.len() >= 2 {
        let diff = smoothed[smoothed.len() - 1] - smoothed[0];
        if diff > 5.0 {
            "up"
        } else if diff < -5.0 {
            "down"
        } else {
            "stable"
        }
    } else {
        "stable"
    };

    let regressing = if smoothed.len() >= 3 {
        let last3 = &smoothed[smoothed.len() - 3..];
        last3.windows(2).all(|pair| pair[1] < pair[0]) && last3[last3.len() - 1] < avg_health
    } else {
        false
    };

    let start_index = health_points.len().saturating_sub(3);
    let last3_values = health_points[start_index..]
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>();
    let endpoint = if health_points.len() >= 2 {
        format!(
            "{}→{}",
            health_points[0],
            health_points[health_points.len() - 1]
        )
    } else {
        last3_values.join("→")
    };

    let message = if regressing {
        format!("Declining: {}", last3_values.join("→"))
    } else if direction == "down" && last3_values.len() >= 2 {
        format!("Declining: {endpoint}")
    } else if direction == "up" && last3_values.len() >= 2 {
        format!("Improving: {endpoint}")
    } else {
        format!("Health score stable at {avg_health:.0}")
    };

    FullHealthTrend {
        direction: direction.to_string(),
        regressing,
        avg_health,
        message,
        points,
    }
}

fn overview_incident_timelines(sessions: &[Session], limit: usize) -> Vec<IncidentTimelineSummary> {
    if limit == 0 {
        return Vec::new();
    }
    let mut items = Vec::new();
    for session in sessions {
        let timeline = build_incident_timeline(session);
        if timeline.items.is_empty() {
            continue;
        }
        items.push(timeline);
        if items.len() >= limit {
            break;
        }
    }
    items
}

fn build_incident_timeline(session: &Session) -> IncidentTimelineSummary {
    let metrics = &session.metrics;
    let mut items = Vec::new();
    let mut add = |kind: &str, label: &str, detail: String, severity: &str| {
        let detail = detail.trim().to_string();
        if detail.is_empty() {
            return;
        }
        items.push(IncidentTimelineItem {
            kind: kind.to_string(),
            label: label.to_string(),
            detail,
            severity: severity.to_string(),
        });
    };

    if metrics.assistant_turns > 0 {
        add(
            "milestone",
            "Last milestone",
            format!(
                "{} assistant turn(s) completed over {}",
                metrics.assistant_turns,
                fmt_duration(metrics.duration_sec)
            ),
            "low",
        );
    }

    if let Some(gap) = max_incident_gap(&metrics.gaps_sec) {
        if gap >= 30.0 {
            let severity = if gap >= 300.0 {
                "high"
            } else if gap >= 60.0 {
                "medium"
            } else {
                "low"
            };
            add(
                "idle_gap",
                "Longest idle gap",
                format!("{gap:.1}s gap between recorded events"),
                severity,
            );
        }
    }

    let total_tools = metrics.tool_calls_ok + metrics.tool_calls_fail;
    if total_tools > 0 && metrics.tool_calls_fail > 1 {
        let fail_rate = metrics.tool_calls_fail as f64 / total_tools as f64 * 100.0;
        let severity = if fail_rate >= 30.0 { "high" } else { "medium" };
        add(
            "failure_loop",
            "Failure loop",
            format!(
                "{} failed tool result(s) out of {} ({fail_rate:.1}%)",
                metrics.tool_calls_fail, total_tools
            ),
            severity,
        );
    }

    if total_tools > 0 && !metrics.tool_usage.is_empty() {
        if let Some((tool, count)) = top_incident_tool(&metrics.tool_usage) {
            add(
                "touched_surface",
                "Touched surface",
                format!(
                    "{} unique tool(s), {} total calls; top tool {} x{}",
                    metrics.tool_usage.len(),
                    total_tools,
                    incident_safe_name(&tool),
                    count
                ),
                "low",
            );
        }
    }

    let total_tokens = total_tokens(session);
    if total_tokens > 0 && metrics.assistant_turns > 0 {
        let tokens_per_turn = total_tokens / metrics.assistant_turns as i64;
        if tokens_per_turn >= 10000 {
            add(
                "burn_divergence",
                "Burn divergence",
                format!(
                    "{} tokens per assistant turn across {} turn(s)",
                    format_tokens(tokens_per_turn),
                    metrics.assistant_turns
                ),
                "medium",
            );
        }
    }

    IncidentTimelineSummary {
        session: session.name.clone(),
        items,
    }
}

/// rm-895: the ONE truncation policy for the overview by_* spend
/// families (agent / model / provider / branch). Human-facing
/// surfaces (text, html, markdown) render at most `GROUP_TAKE` groups
/// per family — sorted with [`sorted_group_overview`] — and disclose
/// the cut with a `+N more not shown` marker. Machine surfaces
/// (json, csv) never truncate and carry no marker: full data is their
/// contract. A policy change is a one-line edit here, and the golden
/// tests in `tests/by_group_truncation_contract.rs` pin the rendered
/// counts so surfaces can only disagree by documented policy.
pub const GROUP_TAKE: usize = 12;

/// rm-895: the shared comparator for the by_* spend families —
/// cost desc, then sessions desc, then name — so the group set that
/// survives a [`GROUP_TAKE`] cut is identical on every surface.
fn sorted_group_overview(
    groups: &BTreeMap<String, GroupOverview>,
) -> Vec<(&String, &GroupOverview)> {
    let mut items: Vec<_> = groups.iter().collect();
    items.sort_by(|(name_a, a), (name_b, b)| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.sessions.cmp(&a.sessions))
            .then_with(|| name_a.cmp(name_b))
    });
    items
}

/// Owned-group adapter over [`sorted_group_overview`] for the SVG usage
/// card's top-N bars: one sort policy everywhere, this just clones the
/// shared ranking into the owned `(String, GroupOverview)` pairs that
/// `card_bars` renders (the card cuts to its own labeled top 5).
fn sorted_model_groups(groups: &BTreeMap<String, GroupOverview>) -> Vec<(String, GroupOverview)> {
    sorted_group_overview(groups)
        .into_iter()
        .map(|(name, group)| (name.clone(), group.clone()))
        .collect()
}

/// rm-895: one row-shape for every by_* family on the text surface —
/// shared sort, shared cap, explicit disclosure of the cut.
/// rm-239 (2026-10-09, integration case a075eb22): the row label routes
/// through `text_cell` at the family width — the composition point of
/// run 7e00d9cbe20a review fix 95d74221 F1 (per-row
/// `text_cell(&model, 25)` / `text_cell(&provider, 25)`) onto the shared
/// family renderer that superseded those loops: the transcript-
/// controlled model key, the catalog-derived provider (with its
/// "unknown" fallback) and the git-branch bucket key all render
/// sanitized (U+FFFD substitution, rm-034/rm-540 contract) and cut to
/// the family width, and the tool-derived agent names ride the same
/// path for defense-in-depth. The json/md/html/csv group lanes never
/// pass through here and stay byte-identical.
fn push_text_group_family(
    out: &mut String,
    groups: &BTreeMap<String, GroupOverview>,
    width: usize,
    display_name: impl Fn(&String) -> String,
) {
    for (name, group) in sorted_group_overview(groups).into_iter().take(GROUP_TAKE) {
        out.push_str(&format!(
            "    {:<width$} {:>4} Sessions  {:>8}\n",
            text_cell(&display_name(name), width),
            format_count(group.sessions),
            format_cost(group.cost),
            width = width
        ));
    }
    if groups.len() > GROUP_TAKE {
        out.push_str(&format!(
            "    (+{} more not shown)\n",
            groups.len() - GROUP_TAKE
        ));
    }
    out.push('\n');
}

/// rm-895: shared close for a by_* section on the html surface —
/// table close, then the truncation marker when the family was cut.
fn group_section_close(total: usize) -> String {
    let mut close = String::from("</tbody></table>");
    if total > GROUP_TAKE {
        close.push_str(&format!(
            "<p class=\"truncated\">+{} more not shown</p>",
            total - GROUP_TAKE
        ));
    }
    close.push_str("</section>");
    close
}

fn health_class(health: i32) -> &'static str {
    if health >= 80 {
        "health-good"
    } else if health >= 50 {
        "health-warn"
    } else {
        "health-bad"
    }
}

fn failure_class(rate: f64) -> &'static str {
    if rate >= 25.0 {
        "bad"
    } else if rate >= 10.0 {
        "warn"
    } else {
        ""
    }
}

fn report_html_code_list(values: &[String]) -> String {
    values
        .iter()
        .filter(|value| !value.is_empty())
        .map(|value| format!("<code>{}</code>", html_escape(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn html_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&#34;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn text_authority_count_values(items: &[AuthorityCount]) -> Vec<String> {
    items
        .iter()
        .map(|item| format!("{}={}", item.category, item.count))
        .collect()
}

fn text_tool_values(items: &[String]) -> Vec<String> {
    items.iter().map(|item| text_cell(item, 40)).collect()
}

fn text_incident_detail_limit(label: &str) -> usize {
    const LINE_LIMIT: usize = 96;
    const INDENT_WIDTH: usize = 4;
    const SESSION_WIDTH: usize = 30;
    const SEPARATORS_WIDTH: usize = 3;

    let label_width = label.chars().count();
    let limit = LINE_LIMIT
        .saturating_sub(INDENT_WIDTH)
        .saturating_sub(SESSION_WIDTH)
        .saturating_sub(SEPARATORS_WIDTH)
        .saturating_sub(label_width);
    limit.max(24)
}

fn text_wrapped_key_values(label: &str, values: &[String], limit: usize) -> Vec<String> {
    if values.is_empty() {
        return vec![format!("{label}:")];
    }
    let prefix = format!("{label}: ");
    let continuation = " ".repeat(label.chars().count() + 2);
    let mut lines = Vec::with_capacity(1);
    let mut current = prefix.clone();
    for value in values {
        let separator = if current != prefix && current != continuation {
            ", "
        } else {
            ""
        };
        let mut next = format!("{separator}{value}");
        if rune_count(&current) + rune_count(&next) > limit
            && current != prefix
            && current != continuation
        {
            lines.push(current);
            current = format!("{continuation}{value}");
            continue;
        }
        if rune_count(&current) + rune_count(&next) > limit {
            let value_limit = limit.saturating_sub(rune_count(&current)).max(4);
            next = text_cell(value, value_limit);
        }
        current.push_str(&next);
    }
    lines.push(current);
    lines
}

fn text_cell(value: &str, limit: usize) -> String {
    // rm-239 (2026-10-07): text report cells route through the same
    // terminal-control sanitizer as the statement/statusline paths
    // (`sanitize_line_segment`, rm-034) — an OSC-52 clipboard-write or
    // CSI cursor payload carried by a session title, model name, or
    // MCP server name must never reach the terminal as raw escape
    // bytes. The sanitizer SUBSTITUTES every C0/C1 control byte with
    // U+FFFD (the rm-034/rm-540 house contract — nothing is stripped
    // or blanked), and the whitespace squash below then folds away
    // nothing of it; rune truncation happens after sanitization, so
    // the `...` tail of an over-long cell is clean too. The
    // json/md/html/csv lanes never pass through here and stay
    // byte-identical.
    let value = sanitize_line_segment(value);
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if limit > 3 {
        truncate_text_runes(&value, limit, "...")
    } else {
        value
    }
}

fn truncate_text_runes(value: &str, limit: usize, suffix: &str) -> String {
    if limit == 0 {
        return String::new();
    }
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let suffix_len = suffix.chars().count();
    let (cut, suffix) = if !suffix.is_empty() && suffix_len < limit {
        (limit - suffix_len, suffix)
    } else {
        (limit, "")
    };
    format!("{}{}", value.chars().take(cut).collect::<String>(), suffix)
}

fn rune_count(value: &str) -> usize {
    value.chars().count()
}

fn max_incident_gap(gaps: &[f64]) -> Option<f64> {
    let mut max_gap = 0.0;
    for gap in gaps {
        if !gap.is_finite() || *gap < 0.0 {
            continue;
        }
        if *gap > max_gap {
            max_gap = *gap;
        }
    }
    Some(max_gap)
}

fn top_incident_tool(items: &BTreeMap<String, usize>) -> Option<(String, usize)> {
    items
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(name, count)| (name.clone(), *count))
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
}

fn incident_safe_name(value: &str) -> String {
    let value = value.split_whitespace().next().unwrap_or("").trim();
    truncate_runes(value, 48)
}

fn truncate_runes(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    value.chars().take(limit).collect()
}

// NOTE (rm-502): the strict `parse_rfc3339` copy that used to live
// here was removed — report trend ordering routes through lib.rs
// `parse_ts` (the single source of timestamp truth), so naive-ISO
// sessions sort exactly like RFC 3339 ones instead of being ordered
// as unknown-time.

/// Human-readable parse-coverage phrase shared by the text, Markdown,
/// and HTML overview renderers (`sep` is ", " or "; ").
///
/// `discovered`/`skipped`/`out_of_scope` count source files while
/// `parsed` counts sessions — SQLite snapshot sources yield many
/// sessions per file, so when more sessions parsed than files were
/// discovered the phrase switches to "N sessions from M sources"
/// instead of a nonsensical "1410/364 parsed" (pass-8 F8-2).
/// Out-of-scope files are disclosed, never folded into the denominator;
/// when nothing is out of scope the phrase stays byte-identical to the
/// pre-cycle-5 rendering (the --demo golden depends on it). Sessions
/// carrying naive-ISO stamps (reinterpreted as UTC by the lenient
/// parse arm) are disclosed the same way (rm-502), so that UTC
/// assumption never hides behind a plain coverage number.
fn parse_coverage_phrase(health: &crate::DataHealth, sep: &str) -> String {
    let base = if health.parsed > health.discovered {
        format!(
            "{} sessions from {} sources",
            health.parsed, health.discovered
        )
    } else {
        format!("{}/{} parsed", health.parsed, health.discovered)
    };
    let mut phrase = format!("{base}{sep}{} skipped{sep}", health.skipped);
    if health.out_of_scope > 0 {
        phrase.push_str(&format!(
            "{} outside range/filters{sep}",
            health.out_of_scope
        ));
    }
    if health.naive_utc_sessions > 0 {
        phrase.push_str(&format!(
            "{} naive-UTC timestamp sessions{sep}",
            health.naive_utc_sessions
        ));
    }
    phrase.push_str(&format!("{} cache hits", health.cache_hits));
    phrase
}

/// Escapes a transcript-derived string for a plain GFM table cell (rm-403).
///
/// Policy: plain cells render as inline markdown, and inline markdown may
/// carry raw HTML — every HTML metacharacter (`&`, `<`, `>`) in a cell value
/// is entity-escaped, mirroring the HTML report arm's `html_escape` for text
/// nodes, before the pipe/newline table-safety escapes run. The `<br>` this
/// function emits itself is the only raw HTML a plain cell may contain.
///
/// Merge note: the control-byte sanitizer family (fc197c5e's lane) must run
/// BEFORE the entity escape at merge time — control bytes first, then
/// printable HTML — so both byte classes die in one pass.
///
/// rm-625 (resolved at the CLI boundary): the control-byte neutralization
/// now happens at the output-dispatch choke point
/// (`dispatch_sanitize` in agenttrace-cli/main.rs over
/// `sanitize_output_document`), which covers every non-JSON lane for
/// every report — including this markdown/html family. Entity escaping
/// here stays as defense-in-depth; the two layers compose because the
/// document sanitizer is idempotent.
/// rm-897: neutralizes spreadsheet-formula introducers in a markdown table
/// cell, mirroring the landed rm-540 CSV guard (agenttrace-cli
/// `csv_export.rs::guard_formula`) arm for arm. The numeric exemption is
/// judged FIRST on the whole trimmed cell (`-42.50`, `+1e3`, `" -1.5"` stay
/// bare so numeric columns import as values); then a cell whose first
/// NON-WHITESPACE character is an introducer — `=`, `+`, `@`, `-`, or their
/// fullwidth forms U+FF1D/U+FF0B/U+FF20/U+FF0D, which spreadsheet apps
/// normalize to the ASCII meaning — gets a leading `'`, the same paste-in
/// marker the CSV lane emits. A transcript-derived key (gitBranch under
/// rm-585, session name, model) can then never re-arm `=HYPERLINK`/`=cmd|'`
/// payloads when the markdown report is pasted into a spreadsheet; the
/// false-positive cost (a leading apostrophe on benign `-` bullets) is the
/// same trade the CSV lane accepted — guarding is the safe direction.
fn guard_formula_cell(value: &str) -> String {
    if value.trim().parse::<f64>().is_ok() {
        return value.to_string();
    }
    let starts_introducer = value
        .chars()
        .find(|ch| !ch.is_whitespace())
        .is_some_and(|first| {
            matches!(
                first,
                '=' | '+'
                    | '@'
                    | '-'
                    | '\u{FF1D}' // FULLWIDTH EQUALS SIGN
                    | '\u{FF0B}' // FULLWIDTH PLUS SIGN
                    | '\u{FF20}' // FULLWIDTH COMMERCIAL AT
                    | '\u{FF0D}' // FULLWIDTH HYPHEN-MINUS
            )
        });
    if starts_introducer {
        format!("'{value}")
    } else {
        value.to_string()
    }
}

fn markdown_cell(value: &str) -> String {
    guard_formula_cell(value)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "\\|")
        .replace('\n', "<br>")
}

/// Escapes a transcript-derived string for a GFM inline code span (rm-403).
///
/// Policy: code spans need no HTML entity escaping — spec-compliant markdown
/// renderers escape code-span contents themselves, and pre-escaping would
/// corrupt legitimate display (`&lt;img&gt;` would render literally instead of
/// `<img>`). Backticks are stripped to keep the span closed; pipe and newline
/// are made table-safe for spans emitted inside cells.
fn markdown_inline_code(value: &str) -> String {
    value
        .replace('`', "'")
        .replace('|', "\\|")
        .replace('\n', "<br>")
}

fn report_markdown_code_list(values: &[String]) -> String {
    values
        .iter()
        .filter(|value| !value.is_empty())
        .map(|value| format!("`{}`", markdown_inline_code(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn possible_cost_driver_note_strict(session: &Session) -> Option<String> {
    let metrics = &session.metrics;
    let total_tools = metrics.tool_calls_ok + metrics.tool_calls_fail;
    if total_tools > 0 {
        let fail_rate = metrics.tool_calls_fail as f64 / total_tools as f64 * 100.0;
        if fail_rate >= 25.0 {
            return Some(format!(
                "possible driver: {}/{} failed tool result(s) ({fail_rate:.1}%)",
                metrics.tool_calls_fail, total_tools
            ));
        }
    }
    if metrics.assistant_turns > 0 {
        let tokens_per_turn = total_tokens(session) / metrics.assistant_turns as i64;
        if tokens_per_turn >= 50000 {
            return Some(format!(
                "possible driver: {} tokens per assistant turn across {} turn(s)",
                format_tokens(tokens_per_turn),
                metrics.assistant_turns
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Metrics;

    #[test]
    fn text_cell_sanitizes_terminal_control_bytes_from_every_payload_class() {
        // rm-239 (2026-10-07) regression fixture: the payload classes
        // that reach text report lanes are session titles, model
        // names, and MCP server names. The pinned contract is the one
        // every render boundary already shares (rm-034 statusline,
        // rm-540 CSV): NO raw control byte survives — ESC from
        // OSC-52/CSI, BEL, DEL, C1 — each replaced by a visible
        // U+FFFD marker; the printable body around them stays
        // readable, and the OSC body left behind is inert once its
        // introducer byte is gone.
        let osc_52 = "\u{1b}]52;c;aGVsbG8=\u{1b}\\Evil Title";
        let csi = "\u{1b}[2J\u{1b}[Hgemini-2.5-pro";
        let bel = "context7\u{7}";
        for payload in [osc_52, csi, bel] {
            let cell = text_cell(payload, 80);
            assert!(
                !cell.chars().any(|c| c.is_control()),
                "raw control byte survived in {cell:?}"
            );
            assert!(
                cell.contains('\u{FFFD}'),
                "control bytes must be visibly replaced in {cell:?}"
            );
        }
        assert!(text_cell(osc_52, 80).contains("Evil Title"));
        assert!(text_cell(csi, 80).contains("gemini-2.5-pro"));
        assert_eq!(text_cell("plain title", 80), "plain title");
        // Truncation still applies after sanitization.
        let long = "x".repeat(90);
        assert_eq!(text_cell(&long, 40).chars().count(), 40);
    }

    #[test]
    fn overview_summary_totals_saturate_across_sessions() {
        // Two sessions whose usage saturates each session total to i64::MAX
        // previously overflowed the cross-session `.sum()` here: debug builds
        // panicked and release builds printed `total_tokens = -2` — the
        // original F1 symptom.
        let session = Session {
            name: "adversarial".to_string(),
            path: "/tmp/adversarial.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                tokens_input: i64::MAX,
                tokens_output: i64::MAX,
                tokens_cache_w: i64::MAX,
                tokens_cache_r: i64::MAX,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        };
        let sessions = vec![session.clone(), session];
        let summary = overview_summary(&crate::Overview::default(), &sessions);
        assert_eq!(summary["total_tokens"].as_i64(), Some(i64::MAX));
    }

    #[test]
    fn session_text_report_sanitizes_transcript_derived_control_bytes() {
        // rm-383: assess PoC (delegate/172562a8…-scratch/osc-latest.txt) —
        // `--latest -f text` rendered a raw OSC-52 clipboard-write sequence
        // from a crafted tool name in the TOP TOOLS table byte-for-byte.
        // Model strings and anomaly detail are transcript-derived the same
        // way; none may carry control bytes into the terminal. The report's
        // own newlines are the only control characters in the output.
        let mut metrics = Metrics {
            model_used: "m\u{001b}[2J".to_string(),
            ..Metrics::default()
        };
        metrics
            .tool_usage
            .insert("evil\u{001b}]52;c;aGVsbG8=\u{0007}".to_string(), 3);
        let session = Session {
            name: "osc\u{001b}]52;c;aGVsbG8=\u{0007}".to_string(),
            path: "/tmp/osc.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics,
            anomalies: vec![crate::Anomaly {
                kind: "tool_failures".to_string(),
                severity: "low".to_string(),
                detail: "2 failed, last: boom\u{001b}]52;c;aGVsbG8=\u{0007}".to_string(),
            }],
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        };
        let text = report_text(&session);
        assert!(
            !text.contains('\u{001b}') && !text.contains('\u{0007}'),
            "no raw ESC/BEL bytes may reach the terminal: {text:?}"
        );
        assert!(text.contains('\u{FFFD}'));
        assert!(text.contains("]52;c;aGVsbG8="));
    }

    #[test]
    fn compare_json_formats_zero_cost_like_go() {
        let session = Session {
            name: "session".to_string(),
            path: "/tmp/session.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                assistant_turns: 2,
                tool_calls_total: 4,
                tool_calls_ok: 4,
                cost_estimated: 0.0,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        };

        let report = report_compare_json(&[session]);
        assert!(report.contains("\"cost\": 0"));
        assert!(!report.contains("\"cost\": 0.0"));
    }

    #[test]
    fn compare_report_truncates_utf8_session_names_on_char_boundaries() {
        let session = Session {
            name: "打开中文文件并生成排查报告的长会话名称".to_string(),
            path: "/tmp/session.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                assistant_turns: 2,
                tool_calls_total: 4,
                tool_calls_ok: 4,
                cost_estimated: 0.0,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        };

        let report = report_compare(&[session], "default");
        assert!(report.contains("打开中文文件"));
    }

    #[test]
    fn text_and_compare_reports_support_chinese() {
        let session = Session {
            name: "会话".to_string(),
            path: "/tmp/session.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                assistant_turns: 2,
                tool_calls_total: 4,
                tool_calls_ok: 4,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        };

        let report = report_text_with_language(&session, ReportLanguage::Zh);
        assert!(report.contains("AI 智能体会话性能报告"));
        assert!(report.contains("成本与 Token"));
        assert!(!report.contains("MONEY WASTE"));

        let compare = report_compare_with_language(&[session], "default", ReportLanguage::Zh);
        assert!(compare.contains("多会话对比"));
        assert!(compare.contains("会话"));
    }

    #[test]
    fn parse_coverage_phrase_discloses_scope_and_handles_multi_session_sources() {
        // Pass-8 F8-2: ranged runs must disclose out-of-scope files, and
        // SQLite-backed corpora (many sessions per file) must not render
        // the nonsensical "1410/364 parsed". The no-scope case must stay
        // byte-identical to the pre-cycle-5 demo golden.
        let mut health = crate::DataHealth {
            discovered: 3,
            parsed: 3,
            ..Default::default()
        };
        assert_eq!(
            parse_coverage_phrase(&health, ", "),
            "3/3 parsed, 0 skipped, 0 cache hits",
            "golden demo phrase stays byte-identical"
        );
        health.discovered = 364;
        health.parsed = 73;
        health.out_of_scope = 291;
        assert_eq!(
            parse_coverage_phrase(&health, ", "),
            "73/364 parsed, 0 skipped, 291 outside range/filters, 330 cache hits"
                .replace("330", &health.cache_hits.to_string()),
        );
        health.cache_hits = 330;
        assert_eq!(
            parse_coverage_phrase(&health, ", "),
            "73/364 parsed, 0 skipped, 291 outside range/filters, 330 cache hits"
        );
        // rm-502: the naive-UTC disclosure is additive — zero keeps the
        // phrase byte-identical (demo golden), nonzero appends after the
        // out-of-scope segment and before cache hits.
        health.naive_utc_sessions = 0;
        assert_eq!(
            parse_coverage_phrase(&health, ", "),
            "73/364 parsed, 0 skipped, 291 outside range/filters, 330 cache hits"
        );
        health.naive_utc_sessions = 2;
        assert_eq!(
            parse_coverage_phrase(&health, ", "),
            "73/364 parsed, 0 skipped, 291 outside range/filters, \
             2 naive-UTC timestamp sessions, 330 cache hits"
        );
        health.naive_utc_sessions = 0;
        health.discovered = 364;
        health.parsed = 1410;
        health.out_of_scope = 0;
        assert_eq!(
            parse_coverage_phrase(&health, "; "),
            "1410 sessions from 364 sources; 0 skipped; 330 cache hits"
        );
    }

    #[test]
    fn json_float_renders_non_finite_values_as_null_instead_of_panicking() {
        // Pass-8 F8-5: a poisoned or overflowing catalog price used to
        // reach the report writer and panic on `.expect("float
        // serializes")`. Non-finite costs now render as null and data
        // health carries `non_finite_costs`, so the corruption is
        // visible instead of fatal.
        assert_eq!(json_float(f64::INFINITY), "null");
        assert_eq!(json_float(f64::NEG_INFINITY), "null");
        assert_eq!(json_float(f64::NAN), "null");
        assert_eq!(json_float(1.0), "1");
        assert_eq!(json_float(0.5), "0.5");
    }

    #[test]
    fn crate_percentile_is_the_only_percentile_and_matches_go() {
        // Pass-8 F8-6: reports.rs used to carry a second, divergent
        // percentile ((len-1)*p, rounded) next to the Go-pinned len*p
        // definition — p95 of 20 values differed (19 vs 20). The local
        // copy is gone; both call sites must land on the crate
        // definition pinned by `percentile_matches_go_index_rule`.
        let values: Vec<f64> = (1..=20).map(|v| v as f64).collect();
        assert_eq!(crate::percentile(&values, 0.95), 20.0);
        assert_eq!(crate::percentile(&values, 0.50), 11.0);
        assert_eq!(crate::percentile(&[], 0.95), 0.0);
        // rm-420: diagnostics.rs carried an UNSCANNED nearest-rank copy
        // (ceil-1) that read one rank low at every n ≡ 0 (mod 20) and
        // disarmed the slow-tool gates. The pin now scans every
        // percentile-consumer source in the crate, and also rejects the
        // inlined index arithmetic itself — any future local percentile
        // must route through crate::percentile. (Probes are concatenated
        // at runtime so this test's own source does not self-match.)
        let nearest_rank_probe = ["0.95", ").ceil()"].concat();
        let trunc_index_probe = ["* 0.95", ") as usize"].concat();
        for (name, source) in [
            ("reports.rs", include_str!("reports.rs")),
            ("diagnostics.rs", include_str!("diagnostics.rs")),
        ] {
            assert!(
                !source.contains("\nfn percentile("),
                "{name} must not re-declare percentile; use crate::percentile"
            );
            assert!(
                !source.contains(&nearest_rank_probe) && !source.contains(&trunc_index_probe),
                "{name} must not inline a percentile index; use crate::percentile"
            );
        }
    }

    #[test]
    fn counts_cell_appends_workbuddy_basis_provenance_only_when_present() {
        // rm-450: the workbuddy input-basis counters carry their meaning
        // only alongside the assumption that produced them; the note rides
        // the cell so every text-family overview format discloses it,
        // while clean corpora keep byte-identical output. (The cell was
        // generalized to counts_cell by run 6403d975's rm-436/437/438
        // disclosures column; the note ports onto it unchanged.)
        let mut skips = BTreeMap::new();
        assert_eq!(counts_cell(&skips), "");
        skips.insert("event_schema".to_string(), 2);
        assert_eq!(
            counts_cell(&skips),
            "event_schema=2",
            "no note without workbuddy keys"
        );
        skips.insert("workbuddy_input_basis:cache_subtracted".to_string(), 1);
        skips.insert(
            "workbuddy_input_basis:zeroed_suspected_mismatch".to_string(),
            1,
        );
        let cell = counts_cell(&skips);
        assert!(cell.contains("workbuddy_input_basis:zeroed_suspected_mismatch=1"));
        assert!(cell.contains("upstream luoyuctl/agenttrace#310"));
        assert!(cell.contains("cache-inclusive"));
    }
}

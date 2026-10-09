use crate::{
    parse_ts, pricing, project_name, resolve_project, round4, session_capability, total_tokens,
    Session,
};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct CostAudit {
    pub pricing_source: String,
    pub total_estimated_cost: f64,
    pub stored_estimated_cost_usd: f64,
    pub current_estimated_cost_usd: Option<f64>,
    pub pricing_coverage: PricingCoverage,
    pub by_provider_model: Vec<ModelCostAudit>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PricingCoverage {
    pub priced_sessions: usize,
    pub fallback_priced_sessions: usize,
    pub unpriced_or_unknown_sessions: usize,
    pub exact_pricing_pct: f64,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelCostAudit {
    pub provider: String,
    pub model: String,
    pub pricing_source: String,
    pub sessions: usize,
    pub tokens: TokenBreakdown,
    /// Thinking-token share of billed output tokens (0-100), only when
    /// the source reports a separate thinking count (pass-9 CU-20).
    pub reasoning_share_pct: Option<f64>,
    pub rates_per_million_usd: Option<PriceBreakdown>,
    pub component_cost_usd: Option<PriceBreakdown>,
    pub estimated_cost_usd: Option<f64>,
    pub stored_estimated_cost_usd: f64,
    pub pricing_status: String,
    pub pricing_note: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TokenBreakdown {
    pub input: i64,
    pub output: i64,
    /// Thinking tokens billed at the output rate and already included in
    /// `output` (pass-9 CU-20); zero when the source reports no separate
    /// thinking count.
    pub reasoning: i64,
    pub cache_write: i64,
    pub cache_read: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PriceBreakdown {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionCostAudit {
    pub pricing_source: String,
    pub stored_pricing_source: String,
    pub capability: &'static str,
    pub provider: String,
    pub model: String,
    pub tokens: TokenBreakdown,
    pub rates_per_million_usd: Option<PriceBreakdown>,
    pub component_cost_usd: Option<PriceBreakdown>,
    pub estimated_cost_usd: Option<f64>,
    pub stored_estimated_cost_usd: f64,
    pub pricing_status: String,
    pub pricing_note: String,
}

fn current_cost_estimate(
    model: &str,
    tokens: &TokenBreakdown,
) -> Option<(String, PriceBreakdown, PriceBreakdown)> {
    if model == "multiple" {
        return None;
    }
    let rate = pricing::lookup_price(model);
    let rates = PriceBreakdown {
        input: rate.input,
        output: rate.output,
        cache_write: rate.cw,
        cache_read: rate.cr,
        total: 0.0,
    };
    let mut components = PriceBreakdown {
        input: round4(tokens.input as f64 / 1e6 * rate.input),
        output: round4(tokens.output as f64 / 1e6 * rate.output),
        cache_write: round4(tokens.cache_write as f64 / 1e6 * rate.cw),
        cache_read: round4(tokens.cache_read as f64 / 1e6 * rate.cr),
        total: 0.0,
    };
    components.total = round4(
        components.input + components.output + components.cache_write + components.cache_read,
    );
    Some((pricing::pricing_source_for(model), rates, components))
}

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub id: String,
    pub priority: String,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub rationale: String,
    pub evidence: Vec<String>,
    pub estimated_savings_usd: f64,
    pub estimated_savings_tokens: i64,
    pub confidence: String,
    pub action: String,
    pub validation_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpGovernance {
    pub items: Vec<McpGovernanceItem>,
    pub methodology: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpGovernanceItem {
    pub server: String,
    pub loaded_sessions: Option<usize>,
    pub invoked_sessions: usize,
    pub tool_calls: usize,
    pub failed_calls: usize,
    pub coverage_pct: Option<f64>,
    pub estimated_schema_tokens: Option<usize>,
    pub recommendation: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextTrend {
    pub methodology: String,
    pub totals: ContextTrendTotals,
    pub projects: Vec<ProjectContextTrend>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContextTrendTotals {
    pub sessions: usize,
    pub context_warning_sessions: usize,
    pub context_critical_sessions: usize,
    pub repeated_file_reads: usize,
    pub cache_effectiveness_pct: f64,
    pub read_to_write_ratio: f64,
    pub output_cost_per_million_tokens: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectContextTrend {
    pub project: String,
    pub sessions: usize,
    pub avg_context_utilization_pct: f64,
    pub cache_effectiveness_pct: f64,
    pub repeated_file_reads: usize,
    pub read_to_write_ratio: f64,
    pub output_cost_per_million_tokens: f64,
}

#[derive(Default)]
struct ContextAggregate {
    sessions: usize,
    context: f64,
    warnings: usize,
    critical: usize,
    cache_read: i64,
    input: i64,
    output: i64,
    cost: f64,
    reads: usize,
    writes: usize,
    files: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeliveryEvidence {
    pub methodology: String,
    /// rm-714: the per-row confidence disclaimer hoisted to document
    /// level. Rows carry the bare confidence word; the "time-window
    /// heuristic; Git commits are correlated, not attributable proof of
    /// main-merge or business value" explanation is stated once here
    /// instead of being re-serialized into every row (a 24,501-row
    /// --overview -o report.json grew its sessions block by ~3.8 MB of
    /// 155-byte repeats).
    pub confidence_note: String,
    pub summary: DeliverySummary,
    pub sessions: Vec<SessionDeliveryEvidence>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DeliverySummary {
    pub strong: usize,
    pub medium: usize,
    pub weak: usize,
    pub none: usize,
    pub non_code: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionDeliveryEvidence {
    pub session: String,
    pub project: String,
    pub level: String,
    pub evidence: Vec<String>,
    pub confidence: String,
}

pub fn cost_audit(sessions: &[Session]) -> CostAudit {
    #[derive(Default)]
    struct Aggregate {
        sessions: usize,
        tokens: TokenBreakdown,
        cost: f64,
        specific: usize,
        fallback: usize,
        unknown: usize,
    }
    let mut rows: BTreeMap<(String, String), Aggregate> = BTreeMap::new();
    let mut coverage = PricingCoverage::default();
    for session in sessions {
        let provider = session.metrics.source_tool.clone();
        let model = normalized_model(&session.metrics.model_used);
        let row = rows.entry((provider, model.clone())).or_default();
        row.sessions += 1;
        row.tokens.input = row
            .tokens
            .input
            .saturating_add(session.metrics.tokens_input);
        row.tokens.output = row
            .tokens
            .output
            .saturating_add(session.metrics.tokens_output);
        row.tokens.reasoning = row
            .tokens
            .reasoning
            .saturating_add(session.metrics.tokens_reasoning);
        row.tokens.cache_write = row
            .tokens
            .cache_write
            .saturating_add(session.metrics.tokens_cache_w);
        row.tokens.cache_read = row
            .tokens
            .cache_read
            .saturating_add(session.metrics.tokens_cache_r);
        row.tokens.total = row.tokens.total.saturating_add(total_tokens(session));
        row.cost += session.metrics.cost_estimated;
        if matches!(model.as_str(), "default" | "unknown" | "multiple") {
            row.unknown += 1;
            coverage.unpriced_or_unknown_sessions += 1;
        } else if pricing::has_specific_price(&model) {
            row.specific += 1;
            coverage.priced_sessions += 1;
        } else {
            row.fallback += 1;
            coverage.fallback_priced_sessions += 1;
        }
    }
    let negative_components = sessions
        .iter()
        .filter(|session| {
            session.metrics.tokens_input < 0
                || session.metrics.tokens_output < 0
                || session.metrics.tokens_cache_r < 0
                || session.metrics.tokens_cache_w < 0
                || session.metrics.cost_estimated < 0.0
        })
        .count();
    coverage.confidence = if coverage.unpriced_or_unknown_sessions > 0 || negative_components > 0 {
        "low"
    } else if coverage.fallback_priced_sessions > 0 {
        "medium"
    } else {
        "high"
    }
    .to_string();
    coverage.exact_pricing_pct = pct(
        coverage.priced_sessions as i64,
        (coverage.priced_sessions
            + coverage.fallback_priced_sessions
            + coverage.unpriced_or_unknown_sessions) as i64,
    );
    let mut by_provider_model = rows
        .into_iter()
        .map(|((provider, model), row)| {
            let current = current_cost_estimate(&model, &row.tokens);
            let pricing_source = current
                .as_ref()
                .map(|(source, _, _)| source.clone())
                .unwrap_or_else(|| "unavailable: multiple models".to_string());
            let (pricing_status, pricing_note) = if model == "multiple" {
                (
                    "aggregate_estimate",
                    "SQLite aggregated multiple model IDs; no single-model exact price applies",
                )
            } else if row.unknown > 0 {
                ("unpriced_or_unknown", "model name is missing or generic")
            } else if row.fallback > 0 {
                (
                    "fallback_estimate",
                    "no exact catalog match; built-in fallback rate used",
                )
            } else {
                (
                    "catalog_estimate",
                    "exact normalized model match in pricing catalog",
                )
            };
            let pricing_note = if current
                .as_ref()
                .is_some_and(|(_, _, components)| (components.total - row.cost).abs() > 0.0001)
            {
                format!(
                    "{pricing_note}; current rates recalculate a different total than the stored estimate"
                )
            } else {
                pricing_note.to_string()
            };
            let (rates_per_million_usd, component_cost_usd, estimated_cost_usd) = current
                .map(|(_, rates, components)| {
                    let total = components.total;
                    (Some(rates), Some(components), Some(total))
                })
                .unwrap_or((None, None, None));
            let reasoning_share_pct = if row.tokens.reasoning > 0 && row.tokens.output > 0 {
                Some(round4(
                    row.tokens.reasoning as f64 / row.tokens.output as f64 * 100.0,
                ))
            } else {
                None
            };
            ModelCostAudit {
                provider,
                model,
                pricing_source,
                sessions: row.sessions,
                tokens: row.tokens,
                reasoning_share_pct,
                rates_per_million_usd,
                component_cost_usd,
                estimated_cost_usd,
                stored_estimated_cost_usd: round4(row.cost),
                pricing_status: pricing_status.to_string(),
                pricing_note,
            }
        })
        .collect::<Vec<_>>();
    let stored_estimated_cost_usd = round4(
        sessions
            .iter()
            .map(|session| session.metrics.cost_estimated)
            .sum(),
    );
    let current_estimated_cost_usd = by_provider_model
        .iter()
        .try_fold(0.0, |total, row| {
            row.estimated_cost_usd.map(|cost| total + cost)
        })
        .map(round4);
    by_provider_model.sort_by(|left, right| {
        let left_cost = left
            .estimated_cost_usd
            .unwrap_or(left.stored_estimated_cost_usd);
        let right_cost = right
            .estimated_cost_usd
            .unwrap_or(right.stored_estimated_cost_usd);
        right_cost
            .total_cmp(&left_cost)
            .then_with(|| left.provider.cmp(&right.provider))
            .then_with(|| left.model.cmp(&right.model))
    });
    CostAudit {
        pricing_source: pricing::pricing_source(),
        total_estimated_cost: stored_estimated_cost_usd,
        stored_estimated_cost_usd,
        current_estimated_cost_usd,
        pricing_coverage: coverage,
        by_provider_model,
    }
}

pub fn session_cost_audit(session: &Session) -> SessionCostAudit {
    let model = normalized_model(&session.metrics.model_used);
    let tokens = TokenBreakdown {
        input: session.metrics.tokens_input,
        output: session.metrics.tokens_output,
        reasoning: session.metrics.tokens_reasoning,
        cache_write: session.metrics.tokens_cache_w,
        cache_read: session.metrics.tokens_cache_r,
        total: total_tokens(session),
    };
    let current = current_cost_estimate(&model, &tokens);
    let (pricing_status, pricing_note) = if model == "multiple" {
        (
            "aggregate_estimate",
            "SQLite aggregated multiple model IDs; no single-model exact price applies",
        )
    } else if matches!(model.as_str(), "default" | "unknown") {
        (
            "unpriced_or_unknown",
            "model name is missing or generic; cost is not a catalog match",
        )
    } else if pricing::has_specific_price(&model) {
        (
            "catalog_estimate",
            "exact normalized model match in pricing catalog",
        )
    } else {
        (
            "fallback_estimate",
            "no exact catalog match; built-in fallback rate used",
        )
    };
    let pricing_note = if current.as_ref().is_some_and(|(_, _, components)| {
        (components.total - session.metrics.cost_estimated).abs() > 0.0001
    }) {
        format!(
            "{pricing_note}; current rates recalculate a different total than the stored estimate"
        )
    } else {
        pricing_note.to_string()
    };
    let pricing_source = current
        .as_ref()
        .map(|(source, _, _)| source.clone())
        .unwrap_or_else(|| "unavailable: multiple models".to_string());
    let (rates_per_million_usd, component_cost_usd, estimated_cost_usd) = current
        .map(|(_, rates, components)| {
            let total = components.total;
            (Some(rates), Some(components), Some(total))
        })
        .unwrap_or((None, None, None));
    SessionCostAudit {
        pricing_source,
        stored_pricing_source: if session.metrics.provenance.pricing_source.is_empty() {
            "not recorded".to_string()
        } else {
            session.metrics.provenance.pricing_source.clone()
        },
        capability: session_capability(session),
        provider: session.metrics.source_tool.clone(),
        model,
        tokens,
        rates_per_million_usd,
        component_cost_usd,
        estimated_cost_usd,
        stored_estimated_cost_usd: round4(session.metrics.cost_estimated),
        pricing_status: pricing_status.to_string(),
        pricing_note,
    }
}

pub fn recommendations(sessions: &[Session]) -> Vec<Recommendation> {
    let mut items = Vec::new();
    for session in sessions {
        let loop_cost = session.diagnostics.loop_cost.total_loop_cost;
        if loop_cost > 0.0 {
            items.push(recommendation(
                "retry-loop",
                severity_from_cost(loop_cost),
                "retry_loop",
                "Bound repeated tool retries",
                format!(
                    "{} repeated tool events or loop group(s) were detected.",
                    session.diagnostics.loop_cost.retry_events
                        + session.diagnostics.loop_cost.loop_groups
                ),
                vec![
                    format!("session={}", session.name),
                    // rm-754: the dollar figure names its basis -- a
                    // constant-derived synthetic estimate can no longer
                    // wear a plain $ label in the advice.
                    format!(
                        "loop_cost=${loop_cost:.4} ({})",
                        session.diagnostics.loop_cost.cost_basis.disclosure()
                    ),
                ],
                loop_cost,
                0,
                "medium",
                "Stop after two unchanged failures; inspect the failure boundary before retrying.",
                "agenttrace --diagnostics --inspect 1 -f json",
            ));
        }
        if session.metrics.tool_calls_fail > 0 {
            items.push(recommendation(
                "tool-failures",
                if session.metrics.tool_calls_fail >= 3 { "high" } else { "medium" },
                "tool_failure",
                "Reduce failing tool calls",
                "Tool failures increase wall time and often precede repeated work.".to_string(),
                vec![format!("session={}", session.name), format!("failed_calls={}", session.metrics.tool_calls_fail)],
                session.metrics.cost_estimated * session.metrics.tool_calls_fail as f64
                    / session.metrics.tool_calls_total.max(1) as f64,
                0,
                "high",
                "Inspect arguments and results, then change the approach instead of retrying unchanged calls.",
                "agenttrace --sessions --sort failures --limit 20",
            ));
        }
        let context = &session.diagnostics.context_utilization;
        if matches!(context.risk_level.as_str(), "warning" | "critical") {
            items.push(recommendation(
                "context-pressure",
                &context.risk_level,
                "context",
                "Start a narrower follow-up session",
                "Conversation and tool context leave little room for the active task.".to_string(),
                vec![format!("session={}", session.name), format!("utilization={:.1}%", context.utilization_pct)],
                session.metrics.cost_estimated * 0.2,
                (context.conversation_history / 5) as i64,
                "medium",
                "Carry only the current goal, relevant files, and failing output into a fresh session.",
                "agenttrace --context-trends --project <project> -f json",
            ));
        }
        // rm-528: severity follows measured latency only. An unmatched
        // tool_use is a pairing defect (torn tail, crashed session, call
        // still in flight — rm-004's taxonomy, upstream #296 keeps them
        // separate the same way), not timeout evidence: rating it high
        // let a zero-latency phantom outrank — and via the
        // (category, title) dedupe, displace — a measured p95 > 30s row
        // from another session. High requires `is_slow` (the same > 30s
        // gate the diagnostics feed uses); unmatched-only sessions stay
        // disclosed at medium. Selection prefers a measured slow tool
        // over the first unmatched row when both exist.
        if let Some(slow) = session
            .diagnostics
            .tool_latencies
            .iter()
            // rm-755: cite the WORST slow tool, not whichever entry
            // the diagnostics max_sec sort happens to surface first. A
            // max_sec leader with a lower p95 used to displace the
            // max-p95 entry, so the advice cited the wrong percentile.
            .filter(|item| item.is_slow)
            .min_by(|a, b| {
                b.p95_sec
                    .total_cmp(&a.p95_sec)
                    .then_with(|| a.tool_name.cmp(&b.tool_name))
            })
            .or_else(|| {
                session
                    .diagnostics
                    .tool_latencies
                    .iter()
                    .find(|item| item.unmatched > 0)
            })
        {
            items.push(recommendation(
                "slow-tool",
                if slow.is_slow { "high" } else { "medium" },
                "latency",
                "Bound slow tool execution",
                "A tool exceeded the latency threshold or returned without a result.".to_string(),
                vec![
                    format!("session={}", session.name),
                    format!(
                        "tool={} p95={:.1}s unmatched={}",
                        slow.tool_name, slow.p95_sec, slow.unmatched
                    ),
                ],
                session.metrics.cost_estimated * 0.1,
                0,
                "high",
                "Set a timeout and batch independent work instead of serial retries.",
                "agenttrace --diagnostics --inspect 1 -f json",
            ));
        }
    }
    let mut deduped: BTreeMap<(String, String), Recommendation> = BTreeMap::new();
    for item in items {
        let key = (item.category.clone(), item.title.clone());
        match deduped.get(&key) {
            Some(existing) if recommendation_rank(existing) >= recommendation_rank(&item) => {}
            _ => {
                deduped.insert(key, item);
            }
        }
    }
    let mut items = deduped.into_values().collect::<Vec<_>>();
    items.sort_by(|left, right| {
        recommendation_rank(right)
            .cmp(&recommendation_rank(left))
            .then_with(|| left.id.cmp(&right.id))
    });
    items
}

pub fn mcp_governance(sessions: &[Session]) -> McpGovernance {
    #[derive(Default)]
    struct Aggregate {
        invoked: BTreeSet<String>,
        calls: usize,
        failed: usize,
    }
    let mut rows: BTreeMap<String, Aggregate> = BTreeMap::new();
    for session in sessions {
        let session_key = format!("{}:{}", session.path, session.metrics.session_start);
        for (tool, count) in &session.metrics.tool_usage {
            let Some(server) = mcp_server_name(tool) else {
                continue;
            };
            let row = rows.entry(server).or_default();
            row.invoked.insert(session_key.clone());
            row.calls += count;
        }
        for warning in &session.tool_warnings {
            if let Some(server) = mcp_server_name(&warning.tool_name) {
                rows.entry(server).or_default().failed += warning.count;
            }
        }
    }
    let items = rows.into_iter().map(|(server, row)| {
        let invoked_sessions = row.invoked.len();
        let recommendation = if row.failed > 0 {
            "investigate failed calls before changing server scope"
        } else {
            "observed usage is material; loading coverage cannot be inferred from invocation-only logs"
        };
        McpGovernanceItem {
            server,
            loaded_sessions: None,
            invoked_sessions,
            tool_calls: row.calls,
            failed_calls: row.failed,
            coverage_pct: None,
            estimated_schema_tokens: None,
            recommendation: recommendation.to_string(),
            confidence: "low: loaded-session counts and schema tokens are unavailable because these logs expose invocations, not complete MCP inventories".to_string(),
        }
    }).collect();
    McpGovernance {
        items,
        methodology: "MCP server names are inferred from tool-name prefixes. Invocation coverage is reported only among observed calls; loaded-server inventory and schema-token cost are intentionally left unmeasured.".to_string(),
    }
}

pub fn context_trends(sessions: &[Session]) -> ContextTrend {
    let mut totals = ContextAggregate::default();
    let mut projects: BTreeMap<String, ContextAggregate> = BTreeMap::new();
    for session in sessions {
        let project = project_name(session);
        add_context_session(&mut totals, session);
        add_context_session(projects.entry(project).or_default(), session);
    }
    let totals_view = context_totals(&totals);
    let mut projects = projects
        .into_iter()
        .map(|(project, value)| ProjectContextTrend {
            project,
            sessions: value.sessions,
            avg_context_utilization_pct: if value.sessions == 0 {
                0.0
            } else {
                round4(value.context / value.sessions as f64)
            },
            cache_effectiveness_pct: pct(
                value.cache_read,
                value.input.saturating_add(value.cache_read),
            ),
            repeated_file_reads: value
                .files
                .values()
                .map(|count| count.saturating_sub(1))
                .sum(),
            read_to_write_ratio: ratio(value.reads, value.writes),
            output_cost_per_million_tokens: per_million_output_cost(&value),
        })
        .collect::<Vec<_>>();
    projects.sort_by(|left, right| {
        right
            .sessions
            .cmp(&left.sessions)
            .then_with(|| left.project.cmp(&right.project))
    });
    ContextTrend {
        methodology: "Cross-session aggregate. Repeated reads are file surface occurrences, and cache effectiveness uses cache-read / (input + cache-read).".to_string(),
        totals: totals_view,
        projects,
    }
}

pub fn delivery_evidence(sessions: &[Session]) -> DeliveryEvidence {
    delivery_evidence_inner(sessions, false)
}

pub fn delivery_evidence_with_git(sessions: &[Session]) -> DeliveryEvidence {
    delivery_evidence_inner(sessions, true)
}

fn delivery_evidence_inner(sessions: &[Session], inspect_git: bool) -> DeliveryEvidence {
    let commits = if inspect_git {
        git_commits_by_root(sessions)
    } else {
        Default::default()
    };
    let mut summary = DeliverySummary::default();
    let mut records = Vec::new();
    for session in sessions {
        let project = resolve_project(session);
        let authority = &session.metrics.tool_authority;
        let matching_commits = commits
            .get(&project.root)
            .map(|commits| commits_for_session(commits, session))
            .unwrap_or_default();
        let (level, mut evidence, confidence) = if !matching_commits.is_empty() {
            (
                "strong",
                vec![format!(
                    "{} local Git commit(s) overlap the session time window",
                    matching_commits.len()
                )],
                "medium",
            )
        } else if authority.get("external_publish").copied().unwrap_or(0) > 0 {
            (
                "medium",
                vec!["observed external publish command category".to_string()],
                "medium",
            )
        } else if authority.get("git_write").copied().unwrap_or(0) > 0 {
            (
                "medium",
                vec!["observed git write command category".to_string()],
                "medium",
            )
        } else if authority.get("write_files").copied().unwrap_or(0) > 0 {
            (
                "weak",
                vec!["observed file write/edit command category".to_string()],
                "medium",
            )
        } else if authority.get("network_access").copied().unwrap_or(0) > 0
            || session.metrics.tool_calls_total > 0
        {
            (
                "non_code",
                vec!["tool activity observed without code-delivery evidence".to_string()],
                "low",
            )
        } else {
            (
                "none",
                vec!["no write, Git, publish, or tool evidence observed".to_string()],
                "low",
            )
        };
        if inspect_git && matching_commits.is_empty() && !project.root.is_empty() {
            evidence.push("no overlapping local commit found; this does not rule out uncommitted, remote, non-code, or later-delivered work".to_string());
        }
        match level {
            "strong" => summary.strong += 1,
            "medium" => summary.medium += 1,
            "weak" => summary.weak += 1,
            "non_code" => summary.non_code += 1,
            _ => summary.none += 1,
        }
        records.push(SessionDeliveryEvidence {
            session: session.name.clone(),
            project: project.display_name,
            level: level.to_string(),
            evidence,
            // rm-714: bare word — the disclaimer lives once at document
            // level (DeliveryEvidence.confidence_note).
            confidence: confidence.to_string(),
        });
    }
    sort_delivery_records(&mut records);
    DeliveryEvidence {
        methodology: if inspect_git {
            format!(
                "Read-only local Git heuristic: commit timestamps are matched to session start/end with a 2-minute lead and 5-minute tail. It does not prove authorship, merge-to-main, or business value. The local git log probe is bounded at {} seconds per repository root; a probe that overruns the bound is treated as unavailable and its root degrades to the tool-authority heuristic.",
                GIT_PROBE_TIMEOUT.as_secs()
            )
        } else {
            "Lightweight heuristic based on observed tool authority only. Run --delivery-evidence for read-only local Git timestamp correlation.".to_string()
        },
        confidence_note: "Confidence is a time-window heuristic; Git commits are correlated, not attributable proof of main-merge or business value".to_string(),
        summary,
        sessions: records,
    }
}

#[derive(Clone)]
struct GitCommit {
    timestamp: DateTime<Utc>,
}

fn git_commits_by_root(sessions: &[Session]) -> BTreeMap<String, Vec<GitCommit>> {
    let roots = sessions
        .iter()
        .map(resolve_project)
        .map(|project| project.root)
        .filter(|root| !root.is_empty())
        .collect::<BTreeSet<_>>();
    roots
        .into_iter()
        .filter_map(|root| git_commits(&root).map(|commits| (root, commits)))
        .collect()
}

/// Wall-clock bound for the read-only `git log` probe (rm-242): a
/// wedged or pathologically slow git binary must degrade the report to
/// the tool-authority heuristic instead of stalling it indefinitely.
/// Same cap class as the network pricing fetch (pricing.rs
/// `download_pricing`), sized for a local repository walk rather than a
/// network round trip. The bound is disclosed in the report's
/// `methodology` string.
const GIT_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Wait for a spawned child with a wall-clock deadline. Returns the
/// exit status when the child finishes inside the bound; kills the
/// child and returns `None` when the deadline passes. Callers that
/// piped stdout must keep draining it on another thread so a full
/// pipe can never turn a fast child into a spurious timeout.
fn wait_child_bounded(
    child: &mut Child,
    bound: std::time::Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + bound;
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return Some(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

fn git_commits(root: &str) -> Option<Vec<GitCommit>> {
    // rm-242: the probe used `Command::output()`, which waits without a
    // bound. Here stdout is drained on a helper thread while the main
    // thread polls `try_wait` against `GIT_PROBE_TIMEOUT`, so a hung
    // git degrades after ten seconds instead of hanging the report.
    let mut child = Command::new("git")
        .args(["-C", root, "log", "--all", "--format=%ct"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take();
    let drain = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(stream) = stdout.as_mut() {
            let _ = stream.read_to_end(&mut bytes);
        }
        bytes
    });
    let status = wait_child_bounded(&mut child, GIT_PROBE_TIMEOUT)?;
    if !status.success() {
        return None;
    }
    let stdout = drain.join().ok()?;
    Some(
        String::from_utf8_lossy(&stdout)
            .lines()
            .filter_map(|line| line.parse::<i64>().ok())
            .filter_map(|timestamp| DateTime::from_timestamp(timestamp, 0))
            .map(|timestamp| GitCommit { timestamp })
            .collect(),
    )
}

/// Evidence-strength ordering for the delivery-evidence session list
/// (rm-243). The previous lexicographic level sort rendered the labels
/// alphabetically — `medium`, `non_code`, `none`, `strong`, `weak` — so
/// the weakest evidence could lead the report. The contract is ranked:
/// strong > medium > weak > non_code > none, with the session name as
/// the deterministic tie-break inside a level.
fn sort_delivery_records(records: &mut [SessionDeliveryEvidence]) {
    records.sort_by(|left, right| {
        delivery_level_rank(&left.level)
            .cmp(&delivery_level_rank(&right.level))
            .then_with(|| left.session.cmp(&right.session))
    });
}

/// Smaller rank is stronger evidence; unrecognized labels sort last so
/// future levels cannot silently resurrect the alphabet bug.
fn delivery_level_rank(level: &str) -> u8 {
    match level {
        "strong" => 0,
        "medium" => 1,
        "weak" => 2,
        "non_code" => 3,
        _ => 4,
    }
}

fn commits_for_session<'a>(commits: &'a [GitCommit], session: &Session) -> Vec<&'a GitCommit> {
    let Some(start) = parse_ts(&session.metrics.session_start) else {
        return Vec::new();
    };
    let end = parse_ts(&session.metrics.session_end).unwrap_or(start);
    let start = start - Duration::minutes(2);
    let end = end + Duration::minutes(5);
    commits
        .iter()
        .filter(|commit| commit.timestamp >= start && commit.timestamp <= end)
        .collect()
}

// NOTE (rm-502): the strict `parse_timestamp` copy that used to live
// here was removed — governance commit windows route through lib.rs
// `parse_ts` (the single source of timestamp truth), so naive-ISO
// sessions match their commits exactly like RFC 3339 ones.

fn normalized_model(model: &str) -> String {
    if model.trim().is_empty() {
        "unknown".to_string()
    } else {
        model.to_string()
    }
}

#[allow(clippy::too_many_arguments)]
fn recommendation(
    id: &str,
    severity: &str,
    category: &str,
    title: &str,
    rationale: String,
    evidence: Vec<String>,
    estimated_savings_usd: f64,
    estimated_savings_tokens: i64,
    confidence: &str,
    action: &str,
    validation_command: &str,
) -> Recommendation {
    Recommendation {
        id: id.to_string(),
        priority: priority(severity).to_string(),
        severity: severity.to_string(),
        category: category.to_string(),
        title: title.to_string(),
        rationale,
        evidence,
        estimated_savings_usd: round4(estimated_savings_usd.max(0.0)),
        estimated_savings_tokens: estimated_savings_tokens.max(0),
        confidence: confidence.to_string(),
        action: action.to_string(),
        validation_command: validation_command.to_string(),
    }
}

fn recommendation_rank(item: &Recommendation) -> (u8, i64, i64, String) {
    (
        severity_rank(&item.severity),
        (item.estimated_savings_usd * 10_000.0) as i64,
        item.estimated_savings_tokens,
        item.title.clone(),
    )
}
fn severity_rank(value: &str) -> u8 {
    match value {
        "critical" => 4,
        "high" => 3,
        "warning" | "medium" => 2,
        _ => 1,
    }
}
fn priority(value: &str) -> &'static str {
    match severity_rank(value) {
        4 => "P0",
        3 => "P1",
        2 => "P2",
        _ => "P3",
    }
}
fn severity_from_cost(value: f64) -> &'static str {
    if value >= 10.0 {
        "high"
    } else {
        "medium"
    }
}
fn mcp_server_name(tool: &str) -> Option<String> {
    let tool = tool.trim();
    if let Some(value) = tool.strip_prefix("mcp__") {
        return value
            .split("__")
            .next()
            .filter(|value| !value.is_empty())
            .map(str::to_string);
    }
    if tool.starts_with("mcp_rca_mcp_") {
        return Some("rca-mcp".to_string());
    }
    None
}
fn pct(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        0.0
    } else {
        round4(numerator as f64 / denominator as f64 * 100.0)
    }
}
fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        round4(numerator as f64 / denominator as f64)
    }
}
fn add_context_session(aggregate: &mut ContextAggregate, session: &Session) {
    aggregate.sessions += 1;
    aggregate.context += session.diagnostics.context_utilization.utilization_pct;
    match session.diagnostics.context_utilization.risk_level.as_str() {
        "critical" => aggregate.critical += 1,
        "warning" => aggregate.warnings += 1,
        _ => {}
    }
    // Saturating: per-session token totals can sit at i64::MAX, so plain
    // `+=` overflowed across sessions in the context aggregate.
    aggregate.cache_read = aggregate
        .cache_read
        .saturating_add(session.metrics.tokens_cache_r);
    aggregate.input = aggregate.input.saturating_add(session.metrics.tokens_input);
    aggregate.output = aggregate
        .output
        .saturating_add(session.metrics.tokens_output);
    aggregate.cost += session.metrics.cost_estimated;
    aggregate.reads += session
        .metrics
        .tool_usage
        .iter()
        .filter(|(tool, _)| is_read_tool(tool))
        .map(|(_, count)| count)
        .sum::<usize>();
    aggregate.writes += session
        .metrics
        .tool_usage
        .iter()
        .filter(|(tool, _)| is_write_tool(tool))
        .map(|(_, count)| count)
        .sum::<usize>();
    for (file, count) in &session.metrics.file_usage {
        *aggregate.files.entry(file.clone()).or_default() += count;
    }
}
fn context_totals(value: &ContextAggregate) -> ContextTrendTotals {
    ContextTrendTotals {
        sessions: value.sessions,
        context_warning_sessions: value.warnings,
        context_critical_sessions: value.critical,
        repeated_file_reads: value
            .files
            .values()
            .map(|count| count.saturating_sub(1))
            .sum(),
        cache_effectiveness_pct: pct(
            value.cache_read,
            value.input.saturating_add(value.cache_read),
        ),
        read_to_write_ratio: ratio(value.reads, value.writes),
        output_cost_per_million_tokens: per_million_output_cost(value),
    }
}

/// Cost of one million output tokens in USD — the same derivation for the
/// per-project rows and the totals row so the two can never drift apart
/// (rm-532: the old project field divided dollars by single tokens and read
/// 0.0 everywhere while the totals twin carried the real rate).
fn per_million_output_cost(value: &ContextAggregate) -> f64 {
    if value.output == 0 {
        0.0
    } else {
        round4(value.cost / value.output as f64 * 1e6)
    }
}
fn is_read_tool(tool: &str) -> bool {
    let tool = tool.to_ascii_lowercase();
    ["read", "glob", "grep", "find", "list", "view"]
        .iter()
        .any(|token| tool.contains(token))
}
fn is_write_tool(tool: &str) -> bool {
    let tool = tool.to_ascii_lowercase();
    ["write", "edit", "patch", "replace", "delete", "create"]
        .iter()
        .any(|token| tool.contains(token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{demo_sessions, Diagnostics, Metrics, ToolWarning};

    fn session(name: &str) -> Session {
        Session {
            name: name.to_string(),
            path: format!("/tmp/{name}.jsonl"),
            cwd: "/tmp/project".to_string(),
            metrics: Metrics {
                model_used: "unknown".to_string(),
                tokens_input: 100,
                tokens_output: 20,
                cost_estimated: 1.0,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 80,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        }
    }

    #[test]
    fn audit_marks_unknown_models_without_claiming_exact_pricing() {
        let audit = cost_audit(&[session("unknown-model")]);
        assert_eq!(audit.pricing_coverage.unpriced_or_unknown_sessions, 1);
        assert_eq!(audit.pricing_coverage.exact_pricing_pct, 0.0);
        assert_eq!(
            audit.by_provider_model[0].pricing_status,
            "unpriced_or_unknown"
        );
        let audit_json = serde_json::to_value(&audit).expect("cost audit json");
        assert_eq!(audit_json["stored_estimated_cost_usd"], 1.0);
        assert!(audit_json["current_estimated_cost_usd"].as_f64().is_some());
        assert!(
            audit_json["by_provider_model"][0]["stored_estimated_cost_usd"]
                .as_f64()
                .is_some()
        );
        let mut historical = session("unknown-model");
        historical.metrics.provenance.pricing_source = "historic price source".to_string();
        let session_audit = session_cost_audit(&historical);
        assert_eq!(session_audit.pricing_status, "unpriced_or_unknown");
        assert_eq!(session_audit.capability, "aggregate");
        assert_eq!(
            session_audit.pricing_source,
            pricing::pricing_source_for("unknown")
        );
        assert_eq!(session_audit.stored_pricing_source, "historic price source");
        assert!(session_audit.pricing_note.contains("generic"));
        let json = serde_json::to_value(&session_audit).expect("session audit json");
        assert_eq!(json["stored_estimated_cost_usd"], 1.0);
        assert!(json["estimated_cost_usd"].as_f64().is_some());
    }

    #[test]
    fn audit_marks_sqlite_multi_model_aggregates_without_exact_pricing() {
        let mut value = session("aggregate");
        value.metrics.model_used = "multiple".to_string();
        value.metrics.source_tool = "opencode_db".to_string();
        value.metrics.provenance.pricing_source = "SQLite aggregate: multiple models".to_string();
        let audit = session_cost_audit(&value);
        assert_eq!(audit.pricing_status, "aggregate_estimate");
        assert_eq!(audit.pricing_source, "unavailable: multiple models");
        assert_eq!(
            audit.stored_pricing_source,
            "SQLite aggregate: multiple models"
        );
        assert!(audit.rates_per_million_usd.is_none());
        assert!(audit.component_cost_usd.is_none());
        assert!(audit.estimated_cost_usd.is_none());
        assert_eq!(audit.stored_estimated_cost_usd, 1.0);
        assert!(audit.pricing_note.contains("multiple model IDs"));

        let aggregate = cost_audit(&[value]);
        assert!(aggregate.current_estimated_cost_usd.is_none());
        assert!(aggregate.by_provider_model[0].estimated_cost_usd.is_none());
        assert!(aggregate.by_provider_model[0].component_cost_usd.is_none());
    }

    #[test]
    fn recommendations_rank_context_pressure_before_low_severity_findings() {
        let mut pressured = session("pressured");
        pressured.diagnostics.context_utilization.risk_level = "critical".to_string();
        pressured.diagnostics.context_utilization.utilization_pct = 110.0;
        pressured
            .diagnostics
            .context_utilization
            .conversation_history = 1_000;
        let mut looping = session("looping");
        looping.diagnostics.loop_cost.total_loop_cost = 0.5;
        looping.diagnostics.loop_cost.loop_groups = 1;
        let items = recommendations(&[looping, pressured]);
        assert_eq!(items[0].category, "context");
        assert_eq!(items[0].priority, "P0");
    }

    #[test]
    fn context_pressure_gate_and_trends_follow_the_usage_derived_numerator() {
        // rm-871(d): governance reacts to the vendor-MEASURED numerator,
        // not the bytes/2 heuristic. A session reporting 800k input
        // tokens against the 1M catalog window is ~80% occupied and must
        // trip the context-pressure recommendation and the warning
        // aggregate; the 40.17% assess-probe shape stays `caution` —
        // disclosed in the totals but firing no gate (display tier).
        let event = |input: i64, output: i64| {
            serde_json::json!({
                "type": "assistant",
                "timestamp": "2026-01-01T00:00:00Z",
                "message": {
                    "id": "msg",
                    "model": "claude-sonnet-4-5",
                    "usage": {
                        "input_tokens": input,
                        "output_tokens": output
                    }
                }
            })
            .to_string()
                + "\n"
        };
        let probe_event = || {
            serde_json::json!({
                "type": "assistant",
                "timestamp": "2026-01-01T00:00:00Z",
                "message": {
                    "id": "msg",
                    "model": "claude-sonnet-4-5",
                    "usage": {
                        "input_tokens": 1_000,
                        "cache_creation_input_tokens": 500,
                        "cache_read_input_tokens": 400_000,
                        "output_tokens": 191
                    }
                }
            })
            .to_string()
                + "\n"
        };
        let dir = std::env::temp_dir().join(format!("at-rm871-gov-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let heavy_path = dir.join("heavy.jsonl");
        std::fs::write(&heavy_path, event(800_000, 1_000)).expect("write");
        let probe_path = dir.join("probe.jsonl");
        std::fs::write(&probe_path, probe_event()).expect("write");
        let heavy = crate::parse_file(&heavy_path).expect("parse heavy");
        let probe = crate::parse_file(&probe_path).expect("parse probe");
        let _ = std::fs::remove_dir_all(&dir);

        for session in [&heavy, &probe] {
            assert_eq!(
                session.diagnostics.context_utilization.numerator_source, "usage",
                "both corpora must be measured, not estimated"
            );
        }
        assert_eq!(heavy.diagnostics.context_utilization.risk_level, "warning");
        assert_eq!(probe.diagnostics.context_utilization.risk_level, "caution");

        let items = recommendations(std::slice::from_ref(&heavy));
        assert!(
            items.iter().any(|item| item.category == "context"),
            "80% occupancy from vendor usage must produce a context rec"
        );
        let quiet = recommendations(std::slice::from_ref(&probe));
        assert!(
            !quiet.iter().any(|item| item.category == "context"),
            "caution is display-only: the gate still fires from warning up"
        );

        let trends = context_trends(&[probe, heavy]);
        assert_eq!(trends.totals.sessions, 2);
        assert_eq!(trends.totals.context_warning_sessions, 1);
        assert_eq!(trends.totals.context_critical_sessions, 0);
    }

    #[test]
    fn unmatched_tool_evidence_never_outranks_measured_slow_tool_latency() {
        // rm-528: severity used to follow `unmatched > 0`, so a torn
        // tail with p95=0.0s (no latency data at all) rated P1/high
        // while a measured p95 > 30s tool rated P2/medium — and the
        // (category, title) dedupe let the phantom displace the
        // measured row. Measured latency is now the only path to high;
        // unmatched evidence stays disclosed at medium.
        let torn = |name: &str| {
            let mut session = session(name);
            session.diagnostics.tool_latencies = vec![crate::diagnostics::ToolLatency {
                tool_name: "Bash".to_string(),
                count: 1,
                avg_sec: 0.0,
                p95_sec: 0.0,
                max_sec: 0.0,
                min_sec: 0.0,
                unmatched: 1,
                is_slow: false,
            }];
            session
        };
        let measured = |name: &str| {
            let mut session = session(name);
            session.diagnostics.tool_latencies = vec![crate::diagnostics::ToolLatency {
                tool_name: "Mcp__deploy__release".to_string(),
                count: 5,
                avg_sec: 31.0,
                p95_sec: 35.0,
                max_sec: 40.0,
                min_sec: 29.0,
                unmatched: 0,
                is_slow: true,
            }];
            session
        };
        let items = recommendations(&[torn("torn")]);
        let item = items
            .iter()
            .find(|item| item.id == "slow-tool")
            .expect("phantom still gets a slow-tool row");
        assert_eq!(item.severity, "medium");
        assert_eq!(item.priority, "P2");
        assert!(
            item.evidence
                .iter()
                .any(|line| line.contains("unmatched=1")),
            "the phantom stays disclosed: {item:?}"
        );
        // Both sessions share (category, title): the measured row must
        // win the dedupe, not the phantom, and hold P1/high with its own
        // evidence line.
        let items = recommendations(&[torn("torn"), measured("measured")]);
        let item = items
            .iter()
            .find(|item| item.id == "slow-tool")
            .expect("paired slow-tool row");
        assert_eq!(item.severity, "high");
        assert_eq!(item.priority, "P1");
        assert!(
            item.evidence
                .iter()
                .any(|line| line.contains("p95=35.0s") && line.contains("unmatched=0")),
            "evidence names the measured row: {item:?}"
        );
        let items = recommendations(&[measured("measured")]);
        let item = items
            .iter()
            .find(|item| item.id == "slow-tool")
            .expect("measured-only slow-tool row");
        assert_eq!(item.severity, "high");
        assert_eq!(item.priority, "P1");
    }

    #[test]
    fn cost_audit_breaks_out_reasoning_tokens_and_their_share() {
        // CU-20: thinking tokens are billed at the output rate; the audit
        // must fold them into output, break them out as tokens.reasoning,
        // and expose their share of billed output tokens.
        let mut thinker = session("thinker");
        thinker.metrics.tokens_input = 1_000;
        thinker.metrics.tokens_output = 600;
        thinker.metrics.tokens_reasoning = 400;
        let audit = cost_audit(&[thinker]);
        assert_eq!(audit.by_provider_model.len(), 1);
        let row = &audit.by_provider_model[0];
        assert_eq!(row.tokens.output, 600);
        assert_eq!(row.tokens.reasoning, 400);
        let share = row.reasoning_share_pct.expect("share computed");
        assert!((share - 66.6667).abs() < 0.01, "share is 400/600: {share}");

        // Corpora without thinking models keep the old shape: no
        // reasoning key gymnastics, share stays null.
        let plain = session("plain");
        let audit = cost_audit(&[plain]);
        let row = &audit.by_provider_model[0];
        assert_eq!(row.tokens.reasoning, 0);
        assert!(row.reasoning_share_pct.is_none());
    }

    #[test]
    fn mcp_governance_never_invents_loaded_coverage() {
        let mut value = session("mcp");
        value
            .metrics
            .tool_usage
            .insert("mcp__demo__lookup".to_string(), 2);
        value.tool_warnings.push(ToolWarning {
            tool_name: "mcp__demo__lookup".to_string(),
            pattern: "fail_retry_chain".to_string(),
            count: 1,
            detail: String::new(),
            severity: "high".to_string(),
        });
        let item = &mcp_governance(&[value]).items[0];
        assert_eq!(item.server, "demo");
        assert_eq!(item.invoked_sessions, 1);
        assert_eq!(item.loaded_sessions, None);
        assert_eq!(item.coverage_pct, None);
        assert_eq!(item.failed_calls, 1);
    }

    #[test]
    fn delivery_without_git_keeps_authority_evidence_heuristic() {
        let mut value = session("delivery");
        value
            .metrics
            .tool_authority
            .insert("git_write".to_string(), 1);
        let report = delivery_evidence(&[value]);
        assert_eq!(report.summary.medium, 1);
        assert!(report.methodology.contains("heuristic"));
        // rm-714: rows carry the bare confidence word; the disclaimer is
        // hoisted to the document-level confidence_note.
        assert_eq!(report.sessions[0].confidence, "medium");
        assert!(report.confidence_note.contains("not attributable"));
        assert!(report.confidence_note.contains("time-window heuristic"));
    }

    #[test]
    fn delivery_evidence_orders_levels_by_strength_not_alphabet() {
        // rm-243: the old lexicographic sort rendered the labels
        // alphabetically - medium, non_code, none, strong, weak - so the
        // weakest evidence could lead the report. One record per level
        // must come out ranked strongest-first instead.
        let mut records: Vec<SessionDeliveryEvidence> =
            ["weak", "strong", "none", "non_code", "medium"]
                .into_iter()
                .map(|level| SessionDeliveryEvidence {
                    session: format!("session-{level}"),
                    project: "demo".to_string(),
                    level: level.to_string(),
                    evidence: Vec::new(),
                    confidence: String::new(),
                })
                .collect();
        sort_delivery_records(&mut records);
        let levels: Vec<&str> = records.iter().map(|r| r.level.as_str()).collect();
        assert_eq!(levels, ["strong", "medium", "weak", "non_code", "none"]);
        assert_ne!(
            levels,
            ["medium", "non_code", "none", "strong", "weak"],
            "must not regress to the alphabetical order"
        );
    }

    #[test]
    fn delivery_evidence_rank_ties_break_on_session_name() {
        let mut records: Vec<SessionDeliveryEvidence> = ["zeta", "alpha"]
            .into_iter()
            .map(|name| SessionDeliveryEvidence {
                session: name.to_string(),
                project: "demo".to_string(),
                level: "weak".to_string(),
                evidence: Vec::new(),
                confidence: String::new(),
            })
            .collect();
        sort_delivery_records(&mut records);
        let names: Vec<&str> = records.iter().map(|r| r.session.as_str()).collect();
        assert_eq!(names, ["alpha", "zeta"]);
    }

    #[test]
    fn delivery_evidence_unknown_level_sorts_weakest() {
        // A future level literal must land after "none" rather than
        // resurrecting the alphabet bug somewhere in the middle.
        let mut records: Vec<SessionDeliveryEvidence> =
            [("none", "a"), ("new_level", "b"), ("strong", "c")]
                .into_iter()
                .map(|(level, name)| SessionDeliveryEvidence {
                    session: name.to_string(),
                    project: "demo".to_string(),
                    level: level.to_string(),
                    evidence: Vec::new(),
                    confidence: String::new(),
                })
                .collect();
        sort_delivery_records(&mut records);
        let levels: Vec<&str> = records.iter().map(|r| r.level.as_str()).collect();
        assert_eq!(levels, ["strong", "none", "new_level"]);
    }

    #[test]
    fn delivery_evidence_demo_levels_are_ranked_not_alphabetized() {
        // End to end on the shipped demo corpus: whatever levels the
        // heuristic assigns, the rendered list must be non-increasing in
        // evidence rank.
        let report = delivery_evidence_with_git(&demo_sessions().expect("demo corpus"));
        assert!(!report.sessions.is_empty());
        let ranks: Vec<u8> = report
            .sessions
            .iter()
            .map(|record| delivery_level_rank(&record.level))
            .collect();
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        assert_eq!(
            ranks,
            sorted,
            "levels must be ranked, got {:?}",
            report
                .sessions
                .iter()
                .map(|r| r.level.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn wait_child_bounded_kills_a_sleeping_child_at_the_deadline() {
        // rm-242: the sleeping-git stand-in from the assessment PoC (a
        // git that never exits) must be capped by the bound, not waited
        // on forever.
        let started = Instant::now();
        let mut child = Command::new("sleep")
            .arg("30")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sleep");
        let status = wait_child_bounded(&mut child, std::time::Duration::from_millis(300));
        assert!(
            status.is_none(),
            "a child that outlives the bound must time out"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "the bound must actually cap the wait, took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn wait_child_bounded_returns_status_for_a_fast_child() {
        let mut child = Command::new("true")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn true");
        let status = wait_child_bounded(&mut child, std::time::Duration::from_secs(5));
        assert!(status.expect("fast child finishes").success());
    }

    #[test]
    fn git_probe_bound_is_disclosed_in_the_methodology() {
        let report = delivery_evidence_with_git(&demo_sessions().expect("demo corpus"));
        assert!(
            report
                .methodology
                .contains(&GIT_PROBE_TIMEOUT.as_secs().to_string()),
            "methodology must disclose the probe bound: {}",
            report.methodology
        );
    }
    #[test]
    fn slow_tool_recommendation_cites_max_p95_entry() {
        // rm-755: tool_latencies sorts by max_sec, so the first
        // `is_slow` entry is the max_sec leader -- but the advice must
        // cite the WORST p95 among slow tools, not the leader's
        // percentile. Here the leader's p95 (31s) is well under the
        // second slow entry's (45s): the evidence must name the 45s
        // tool. The pre-fix `.find(is_slow)` cited 31s.
        let mut s = session("rm-755");
        s.diagnostics.tool_latencies = vec![
            crate::diagnostics::ToolLatency {
                tool_name: "WebSearch".to_string(),
                count: 10,
                avg_sec: 20.0,
                p95_sec: 31.0,
                max_sec: 60.0,
                min_sec: 1.0,
                unmatched: 0,
                is_slow: true,
            },
            crate::diagnostics::ToolLatency {
                tool_name: "Mcp__deploy__release".to_string(),
                count: 8,
                avg_sec: 30.0,
                p95_sec: 45.0,
                max_sec: 45.0,
                min_sec: 10.0,
                unmatched: 0,
                is_slow: true,
            },
        ];
        let items = recommendations(&[s]);
        let item = items
            .iter()
            .find(|item| item.id == "slow-tool")
            .expect("slow-tool recommendation present");
        assert!(
            item.evidence.iter().any(
                |line| line.contains("tool=Mcp__deploy__release") && line.contains("p95=45.0s")
            ),
            "evidence must cite the max-p95 slow entry: {item:?}"
        );
    }

    #[test]
    fn retry_loop_recommendation_discloses_cost_basis() {
        // rm-754: a loop-cost dollar figure may no longer wear a plain
        // $ label -- the evidence line names its basis in both arms.
        // (Separate recommendations() calls: the (category, title)
        // dedupe would collapse two retry-loop rows from one batch.)
        let mut priced = session("rm-754-priced");
        priced.diagnostics.loop_cost.total_loop_cost = 0.5;
        priced.diagnostics.loop_cost.loop_groups = 1;
        priced.diagnostics.loop_cost.cost_basis = crate::diagnostics::LoopCostBasis::Priced;
        let items = recommendations(&[priced]);
        let item = items
            .iter()
            .find(|item| item.id == "retry-loop")
            .expect("retry-loop recommendation present");
        assert!(
            item.evidence
                .iter()
                .any(|line| line.contains("loop_cost=$0.5000 (priced at session model rates)")),
            "priced arm must disclose its basis: {item:?}"
        );

        let mut synthetic = session("rm-754-synthetic");
        synthetic.diagnostics.loop_cost.total_loop_cost = 0.5;
        synthetic.diagnostics.loop_cost.loop_groups = 1;
        synthetic.diagnostics.loop_cost.cost_basis = crate::diagnostics::LoopCostBasis::Synthetic;
        let items = recommendations(&[synthetic]);
        let item = items
            .iter()
            .find(|item| item.id == "retry-loop")
            .expect("retry-loop recommendation present");
        assert!(
            item.evidence.iter().any(|line| line
                .contains("synthetic estimate -- no token mass recorded, not price-derived")),
            "synthetic arm must disclose its basis: {item:?}"
        );
    }
}

use crate::{
    pricing, project_name, resolve_project, round4, session_capability, total_tokens, Session,
};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

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
    pub cost_per_output_token: f64,
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
                    format!("loop_cost=${loop_cost:.4}"),
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
        if let Some(slow) = session
            .diagnostics
            .tool_latencies
            .iter()
            .find(|item| item.is_slow || item.unmatched > 0)
        {
            items.push(recommendation(
                "slow-tool",
                if slow.unmatched > 0 { "high" } else { "medium" },
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
            cost_per_output_token: if value.output == 0 {
                0.0
            } else {
                round4(value.cost / value.output as f64)
            },
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
    let git = if inspect_git {
        git_commits_by_root(sessions)
    } else {
        GitEvidence::default()
    };
    let mut summary = DeliverySummary::default();
    let mut records = Vec::new();
    for session in sessions {
        let project = resolve_project(session);
        let authority = &session.metrics.tool_authority;
        let matching_commits = git
            .commits_by_root
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
            if let Some(reason) = git.refused_roots.get(&project.root) {
                // rm-225: say the inspection was skipped and why — the
                // root failed the trust policy (transcript-shaped path);
                // a silent "no overlapping commit" would overstate what
                // was checked.
                evidence.push(format!(
                    "local Git inspection skipped: the derived project root was refused by the trust policy ({reason}); set AGENTTRACE_DELIVERY_EVIDENCE_ROOTS to inspect it explicitly"
                ));
            } else {
                evidence.push("no overlapping local commit found; this does not rule out uncommitted, remote, non-code, or later-delivered work".to_string());
            }
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
            confidence: format!("{confidence}: time-window heuristic; Git commits are correlated, not attributable proof of main-merge or business value"),
        });
    }
    records.sort_by(|left, right| {
        left.level
            .cmp(&right.level)
            .then_with(|| left.session.cmp(&right.session))
    });
    DeliveryEvidence {
        methodology: if inspect_git {
            "Read-only local Git heuristic: commit timestamps are matched to session start/end with a 2-minute lead and 5-minute tail. It does not prove authorship, merge-to-main, or business value."
        } else {
            "Lightweight heuristic based on observed tool authority only. Run --delivery-evidence for read-only local Git timestamp correlation."
        }.to_string(),
        summary,
        sessions: records,
    }
}

#[derive(Clone)]
struct GitCommit {
    timestamp: DateTime<Utc>,
}

/// rm-225: wall-clock ceiling for one delivery-evidence `git log`.
/// Mirrors the npm-probe curl cap (crates/agenttrace-cli/src/upstream.rs
/// `NPM_PROBE_TIMEOUT_SECS`); a contrast test pins the pair so the two
/// subprocess disciplines cannot drift apart.
pub const GIT_INSPECT_TIMEOUT_SECS: u64 = 15;

/// rm-225: `--max-count` bound on the `git log --all` query, on top of
/// the per-root `--since` floor (earliest session window per root). A
/// session cannot match a commit older than its own window, so the
/// floor is exact; the cap bounds even a pathological repository that
/// holds millions of commits inside the window.
pub const GIT_INSPECT_MAX_COMMITS: usize = 10_000;

/// rm-225: explicit allowlist (path list, `:`-separated) for the git
/// roots delivery evidence may inspect. When set it REPLACES the
/// default trust policy: only roots at or under an entry are queried,
/// and an entry may explicitly trust a path the default policy refuses
/// (e.g. a repository kept under the system temp directory).
pub const GIT_INSPECT_ROOT_ALLOWLIST_VAR: &str = "AGENTTRACE_DELIVERY_EVIDENCE_ROOTS";

/// rm-225 verdict of the delivery-evidence root trust policy. The
/// derived git root comes from session metadata, and a hostile
/// transcript controls that metadata (the session directory name
/// decodes into an arbitrary path via `decode_agent_project_dir`), so
/// the spawn site — not the decoder — is the trust boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootTrust {
    Trusted,
    Refused(&'static str),
}

/// Pure core of the root trust policy (rm-225), parameterized by the
/// environment so tests pin it without mutating process state:
///
/// * an allowlist entry covering the root (after canonicalizing both)
///   is trusted outright — explicit user configuration wins, even for
///   a path the default policy would refuse;
/// * with an allowlist set but no entry covering it, the root is
///   refused — the allowlist replaces, not extends, the defaults;
/// * otherwise a root under the system temp directory is refused:
///   that is exactly the shape of the transcript-decoded hostile path;
/// * otherwise the default policy trusts the user's own territory —
///   anything at or under `$HOME`, at or under the working directory,
///   or an ancestor of it (the audit running inside the repository);
/// * anything else (e.g. `/etc`, an unrelated mount) is refused.
fn root_trust_decision(
    root: &str,
    home: Option<&std::path::Path>,
    cwd: Option<&std::path::Path>,
    temp: Option<&std::path::Path>,
    allowlist: Option<&str>,
) -> RootTrust {
    if root.trim().is_empty() {
        return RootTrust::Refused("empty root");
    }
    let Some(root_canon) = std::fs::canonicalize(root).ok() else {
        return RootTrust::Refused("path does not resolve on this host");
    };
    let canonical = |path: &std::path::Path| std::fs::canonicalize(path).ok();
    if let Some(list) = allowlist {
        let covered = list
            .split(':')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .filter_map(|entry| canonical(std::path::Path::new(entry)))
            .any(|entry| root_canon.starts_with(&entry));
        if covered {
            return RootTrust::Trusted;
        }
        return RootTrust::Refused("not under any allowlist entry");
    }
    if let Some(temp) = temp.and_then(canonical) {
        if root_canon.starts_with(&temp) {
            return RootTrust::Refused("inside the system temp directory");
        }
    }
    if home
        .and_then(canonical)
        .is_some_and(|home| root_canon.starts_with(&home))
    {
        return RootTrust::Trusted;
    }
    if let Some(cwd) = cwd.and_then(canonical) {
        if root_canon.starts_with(&cwd) || cwd.starts_with(&root_canon) {
            return RootTrust::Trusted;
        }
    }
    RootTrust::Refused("outside the user home and the working directory")
}

/// Env-reading wrapper over [`root_trust_decision`] (rm-225).
fn trusted_root(root: &str) -> RootTrust {
    let allowlist = std::env::var(GIT_INSPECT_ROOT_ALLOWLIST_VAR).ok();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from);
    let cwd = std::env::current_dir().ok();
    let temp = std::env::temp_dir();
    root_trust_decision(
        root,
        home.as_deref(),
        cwd.as_deref(),
        Some(temp.as_path()),
        allowlist.as_deref(),
    )
}

/// Builds the bounded, hardened `git log` for one root (rm-225):
/// fsmonitor off (no side processes), prompts and system config off,
/// `%ct` output, `--since` pinned to the earliest session window that
/// could match, `--max-count` as a hard cap.
fn git_log_command(root: &str, floor: Option<DateTime<Utc>>, cap: usize) -> Command {
    let mut command = Command::new("git");
    command.args([
        "-c",
        "core.fsmonitor=false",
        "-C",
        root,
        "log",
        "--all",
        "--format=%ct",
    ]);
    if let Some(floor) = floor {
        command.arg(format!("--since={}", floor.timestamp()));
    }
    command.arg(format!("--max-count={cap}"));
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    command
}

/// Polls a child to completion and kills it at the deadline (rm-225).
/// Returns `None` for a timeout or an unrecoverable wait error.
fn wait_with_deadline(
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Err(_) => return None,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    None
}

/// Commits reachable in one root's repository, bounded and hardened
/// (rm-225). Stdout is redirected to a scratch file under the system
/// temp dir rather than a pipe: the capped 10k-line `%ct` output can
/// exceed the pipe buffer (notably on macOS) and would deadlock the
/// deadline poll while git blocks on write.
fn git_commits(command: &mut Command, timeout: std::time::Duration) -> Option<Vec<GitCommit>> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    let out_path = std::env::temp_dir().join(format!(
        "agenttrace-git-log-{}-{nanos}",
        std::process::id()
    ));
    let stdout = std::fs::File::create(&out_path).ok()?;
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(stdout))
        .stderr(std::process::Stdio::null());
    let read = (|| {
        let mut child = command.spawn().ok()?;
        let status = wait_with_deadline(&mut child, timeout)?;
        if !status.success() {
            return None;
        }
        std::fs::read_to_string(&out_path).ok()
    })();
    let _ = std::fs::remove_file(&out_path);
    Some(
        read?
            .lines()
            .filter_map(|line| line.parse::<i64>().ok())
            .filter_map(|timestamp| DateTime::from_timestamp(timestamp, 0))
            .map(|timestamp| GitCommit { timestamp })
            .collect(),
    )
}

/// Git evidence for the whole corpus (rm-225): commits per trusted
/// root, plus the roots the trust policy refused — so per-session
/// evidence can say the inspection was skipped instead of silently
/// claiming "no overlapping commit".
#[derive(Default)]
struct GitEvidence {
    commits_by_root: BTreeMap<String, Vec<GitCommit>>,
    refused_roots: BTreeMap<String, &'static str>,
}

fn git_commits_by_root(sessions: &[Session]) -> GitEvidence {
    let mut sessions_by_root: BTreeMap<String, Vec<&Session>> = BTreeMap::new();
    for session in sessions {
        let root = resolve_project(session).root;
        if !root.is_empty() {
            sessions_by_root.entry(root).or_default().push(session);
        }
    }
    let mut evidence = GitEvidence::default();
    for (root, root_sessions) in sessions_by_root {
        // Commits can only match inside [start - 2m, end + 5m]; a root
        // whose sessions have no parseable window can never produce a
        // match, so it never spawns git at all. The `--since` floor is
        // the earliest possible match across the root's sessions.
        let Some(floor) = root_sessions
            .iter()
            .filter_map(|session| parse_timestamp(&session.metrics.session_start))
            .map(|start| start - Duration::minutes(2))
            .min()
        else {
            continue;
        };
        match trusted_root(&root) {
            RootTrust::Trusted => {
                let mut command = git_log_command(&root, Some(floor), GIT_INSPECT_MAX_COMMITS);
                if let Some(commits) = git_commits(
                    &mut command,
                    std::time::Duration::from_secs(GIT_INSPECT_TIMEOUT_SECS),
                ) {
                    evidence.commits_by_root.insert(root, commits);
                }
            }
            RootTrust::Refused(reason) => {
                evidence.refused_roots.insert(root, reason);
            }
        }
    }
    evidence
}

fn commits_for_session<'a>(commits: &'a [GitCommit], session: &Session) -> Vec<&'a GitCommit> {
    let Some(start) = parse_timestamp(&session.metrics.session_start) else {
        return Vec::new();
    };
    let end = parse_timestamp(&session.metrics.session_end).unwrap_or(start);
    let start = start - Duration::minutes(2);
    let end = end + Duration::minutes(5);
    commits
        .iter()
        .filter(|commit| commit.timestamp >= start && commit.timestamp <= end)
        .collect()
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

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
        output_cost_per_million_tokens: if value.output == 0 {
            0.0
        } else {
            round4(value.cost / value.output as f64 * 1e6)
        },
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
    use crate::{Diagnostics, Metrics, ToolWarning};

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
        assert!(report.sessions[0].confidence.contains("not attributable"));
    }

    // ---- rm-225: bounded, hardened delivery-evidence git inspection ----

    #[test]
    fn git_log_args_are_bounded_and_hardened() {
        let floor = DateTime::parse_from_rfc3339("2026-05-03T10:00:00Z")
            .expect("fixed floor parses")
            .with_timezone(&Utc);
        let command = git_log_command("/tmp/probe", Some(floor), 500);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        for expected in [
            "-c",
            "core.fsmonitor=false",
            "-C",
            "/tmp/probe",
            "log",
            "--all",
            "--format=%ct",
            &format!("--since={}", floor.timestamp()),
            "--max-count=500",
        ] {
            assert!(
                args.iter().any(|arg| arg == expected),
                "git log args must include {expected}: {args:?}"
            );
        }
        let env: Vec<(&std::ffi::OsStr, &std::ffi::OsStr)> = command
            .get_envs()
            .filter_map(|(key, value)| value.map(|value| (key, value)))
            .collect();
        assert!(
            env.contains(&("GIT_TERMINAL_PROMPT".as_ref(), "0".as_ref())),
            "git must not be allowed to prompt: {env:?}"
        );
        assert!(
            env.contains(&("GIT_CONFIG_NOSYSTEM".as_ref(), "1".as_ref())),
            "system git config must be disabled: {env:?}"
        );
    }

    #[test]
    fn git_inspection_defaults_stay_bounded() {
        assert_eq!(GIT_INSPECT_TIMEOUT_SECS, 15, "mirrors the npm-probe curl cap");
        assert_eq!(GIT_INSPECT_MAX_COMMITS, 10_000);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_subprocess_is_killed_at_the_deadline() {
        let started = std::time::Instant::now();
        let mut child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("sleep spawns on unix");
        let status = wait_with_deadline(&mut child, std::time::Duration::from_millis(150));
        assert!(status.is_none(), "deadline must return None, got {status:?}");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "the deadline must fire near 150ms, not after the child's 30s"
        );
        assert!(child.try_wait().expect("killed child reaps").is_some());
    }

    fn temp_repo(kind: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "agenttrace-rm225-policy-{kind}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create policy fixture root");
        dir
    }

    #[test]
    fn root_trust_policy_pins_the_default_and_allowlist_verdicts() {
        let root = temp_repo("cases");
        let home = root.join("home");
        let cwd = root.join("cwd");
        let trusted = root.join("home").join("project");
        let outside = root.join("elsewhere").join("project");
        let temp = root.join("tmpdir");
        let temp_repo = temp.join("repo");
        let deep_repo = cwd.join("deep").join("repo");
        for dir in [&home, &cwd, &trusted, &outside, &temp, &temp_repo, &deep_repo] {
            std::fs::create_dir_all(dir).expect("create fixture dir");
        }
        // Home containment, cwd subtree, and cwd ancestors are the
        // user's own territory.
        assert_eq!(
            root_trust_decision(
                trusted.to_str().expect("utf-8"),
                Some(&home),
                Some(&cwd),
                Some(&temp),
                None
            ),
            RootTrust::Trusted
        );
        // Cwd subtree containment (production single-repo case).
        assert_eq!(
            root_trust_decision(
                deep_repo.to_str().expect("utf-8"),
                Some(&home),
                Some(&cwd),
                Some(&temp),
                None
            ),
            RootTrust::Trusted
        );
        // Running the audit from inside the repository trusts the
        // repository (cwd is under the root).
        assert_eq!(
            root_trust_decision(
                cwd.to_str().expect("utf-8"),
                Some(&home),
                Some(&deep_repo),
                Some(&temp),
                None
            ),
            RootTrust::Trusted
        );
        // Outside home and the working tree: refused.
        assert_eq!(
            root_trust_decision(
                outside.to_str().expect("utf-8"),
                Some(&home),
                Some(&cwd),
                Some(&temp),
                None
            ),
            RootTrust::Refused("outside the user home and the working directory")
        );
        // Temp-dir roots are the transcript-decoded hostile shape.
        assert_eq!(
            root_trust_decision(
                temp_repo.to_str().expect("utf-8"),
                Some(&home),
                Some(&cwd),
                Some(&temp),
                None
            ),
            RootTrust::Refused("inside the system temp directory")
        );
        // An explicit allowlist overrides — and replaces — the default
        // policy: it may trust a temp path ...
        let allowlist = temp_repo.to_str().expect("utf-8");
        assert_eq!(
            root_trust_decision(
                allowlist,
                Some(&home),
                Some(&cwd),
                Some(&temp),
                Some(allowlist)
            ),
            RootTrust::Trusted
        );
        // ... and it refuses paths that were trusted by default.
        assert_eq!(
            root_trust_decision(
                trusted.to_str().expect("utf-8"),
                Some(&home),
                Some(&cwd),
                Some(&temp),
                Some(allowlist)
            ),
            RootTrust::Refused("not under any allowlist entry")
        );
        // Unresolvable and empty roots never reach the spawn site.
        assert_eq!(
            root_trust_decision("", Some(&home), Some(&cwd), Some(&temp), None),
            RootTrust::Refused("empty root")
        );
        assert_eq!(
            root_trust_decision(
                "/definitely/not/a/path-rm225",
                Some(&home),
                Some(&cwd),
                Some(&temp),
                None
            ),
            RootTrust::Refused("path does not resolve on this host")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn delivery_evidence_refuses_transcript_derived_temp_roots() {
        // Rebuilds the assess F1 hostile shape end to end: a session
        // whose directory NAME decodes (empty cwd) into a repository
        // planted under /tmp, with one commit inside the session
        // window. Pre-rm-225 this scored "strong" from a repository
        // the user never chose; now the root is refused and the note
        // says the inspection was skipped. With an explicit allowlist
        // the same fixture scores strong again (positive control: the
        // git fixture itself is sound).
        let scratch = std::env::temp_dir().join(format!(
            "agenttrace-rm225-e2e-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let pid = std::process::id();
        // The decoder probes literal `/tmp`, so the planted repository
        // must live there (not under TMPDIR, which may point elsewhere).
        let repo = std::path::Path::new("/tmp")
            .join(pid.to_string())
            .join("repo");
        std::fs::create_dir_all(&repo).expect("create hostile repo dir");
        let git = || {
            let mut command = Command::new("git");
            command
                .arg("-C")
                .arg(repo.to_str().expect("utf-8 repo path"))
                .env("GIT_AUTHOR_NAME", "fixture")
                .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
                .env("GIT_COMMITTER_NAME", "fixture")
                .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
                .env("GIT_AUTHOR_DATE", "2026-05-03T10:01:30Z")
                .env("GIT_COMMITTER_DATE", "2026-05-03T10:01:30Z")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1");
            command
        };
        let init = git()
            .args(["init", "-q"])
            .output()
            .expect("git init");
        assert!(init.status.success(), "git init must succeed");
        let commit = git()
            .args(["commit", "-q", "--allow-empty", "-m", "fixture"])
            .output()
            .expect("git commit");
        assert!(commit.status.success(), "git commit must succeed");
        let mut hostile = session("hostile");
        // Empty cwd forces project resolution through the session
        // DIRECTORY NAME decoder: "-<pid>-repo" inside a projects/
        // tree decodes to /tmp/<pid>/repo.
        hostile.cwd = String::new();
        hostile.path = scratch
            .join("projects")
            .join(format!("-tmp-{pid}-repo"))
            .join("session.jsonl")
            .to_string_lossy()
            .to_string();
        hostile.metrics.session_start = "2026-05-03T10:01:00.000Z".to_string();
        hostile.metrics.session_end = "2026-05-03T10:04:00.000Z".to_string();
        hostile.metrics.tool_calls_total = 1;

        let report = delivery_evidence_with_git(&[hostile.clone()]);
        assert_eq!(
            report.summary.strong, 0,
            "a transcript-shaped temp root must not produce git evidence"
        );
        assert_eq!(report.summary.non_code, 1);
        assert!(
            report.sessions[0]
                .evidence
                .iter()
                .any(|line| line.contains("was refused by the trust policy")),
            "the evidence must say the inspection was skipped: {:?}",
            report.sessions[0].evidence
        );

        // Positive control: an explicit allowlist entry for exactly
        // this repository turns the git path back on.
        let _env = crate::test_env::lock_env();
        let previous = std::env::var_os(GIT_INSPECT_ROOT_ALLOWLIST_VAR);
        std::env::set_var(
            GIT_INSPECT_ROOT_ALLOWLIST_VAR,
            repo.to_str().expect("utf-8 repo path"),
        );
        let allowed = delivery_evidence_with_git(&[hostile]);
        match previous {
            Some(value) => std::env::set_var(GIT_INSPECT_ROOT_ALLOWLIST_VAR, value),
            None => std::env::remove_var(GIT_INSPECT_ROOT_ALLOWLIST_VAR),
        }
        assert_eq!(
            allowed.summary.strong, 1,
            "the allowlist must restore the (real) git match — fixture control"
        );

        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&scratch);
        let _ = std::fs::remove_dir_all(std::path::Path::new("/tmp").join(pid.to_string()));
    }
}

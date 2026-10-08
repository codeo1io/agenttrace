use crate::{parse_ts, Event, Metrics, Session};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Diagnostics {
    pub loop_cost: LoopCost,
    pub loop_fingerprints: Vec<LoopFingerprint>,
    pub tool_latencies: Vec<ToolLatency>,
    pub context_utilization: ContextUtilization,
    pub large_params: Vec<LargeParam>,
    pub unused_tools: Vec<UnusedTool>,
    pub stuck_patterns: Vec<StuckPattern>,
    pub steps: Vec<TraceStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub kind: String,
    pub name: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_sec: f64,
    pub status: String,
    pub tokens: i64,
    pub call_id: String,
    pub parent_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoopCost {
    pub retry_cost: f64,
    pub tool_loop_cost: f64,
    pub total_loop_cost: f64,
    pub retry_events: usize,
    pub loop_groups: usize,
    pub loop_type: String,
    pub turns: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopFingerprint {
    pub tool_name: String,
    pub result_hash: String,
    pub count: usize,
    pub first_index: usize,
    pub last_index: usize,
    pub severity: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLatency {
    pub tool_name: String,
    pub count: usize,
    pub avg_sec: f64,
    pub p95_sec: f64,
    pub max_sec: f64,
    pub min_sec: f64,
    /// Tool calls whose matching result event is absent from the trace
    /// (truncated log, crashed session, or call still in flight). The
    /// trace carries no timeout marker, so these are reported as
    /// unmatched instead of being mislabeled as timeouts (rm-004).
    pub unmatched: usize,
    pub is_slow: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextUtilization {
    pub estimated_total: usize,
    pub tool_definitions: usize,
    pub conversation_history: usize,
    pub system_prompt: usize,
    pub available_for_task: usize,
    pub utilization_pct: f64,
    pub risk_level: String,
    pub suggestion: String,
    /// Where the context-window denominator came from (rm-231):
    /// `"catalog"` — the model's vendor window (LiteLLM
    /// `max_input_tokens`) from the active pricing catalog;
    /// `"fallback"` — the documented name-substring ladder, used only
    /// when the catalog cannot resolve the model, in which case the
    /// utilization is an estimate, never a measured fraction. Empty on
    /// sessions diagnosed before this field existed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub window_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeParam {
    pub tool_name: String,
    pub size: usize,
    pub risk: String,
    pub timestamp: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedTool {
    pub tool_name: String,
    pub call_count: usize,
    pub level: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StuckPattern {
    pub pattern: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixSuggestion {
    pub title: String,
    pub description: String,
    pub action: String,
    pub severity: String,
    pub category: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CostAlert {
    pub triggered: bool,
    pub level: String,
    pub message: String,
    pub current: f64,
    pub baseline: f64,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindingEvidence {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionFinding {
    pub kind: String,
    pub value: f64,
    pub detail: String,
    pub severity: String,
    pub evidence: Vec<FindingEvidence>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectFirst {
    pub reason: &'static str,
    pub index: usize,
}

const ATTENTION_FAIL_MIN: usize = 3;
const ATTENTION_FAIL_RATE: f64 = 0.2;
const ATTENTION_COST_USD: f64 = 10.0;
const ATTENTION_P95_GAP_SEC: f64 = 120.0;

// Ordering doubles as triage priority: the first matching signal is the one shown.
// Context risk is intentionally excluded because its estimate is a heuristic.
pub fn inspect_reason(session: &Session) -> &'static str {
    let metrics = &session.metrics;
    let fail_rate = metrics.tool_calls_fail as f64 / metrics.tool_calls_total.max(1) as f64;
    if session.health < 50 {
        "critical"
    } else if metrics.tool_calls_fail >= ATTENTION_FAIL_MIN && fail_rate >= ATTENTION_FAIL_RATE {
        "failures"
    } else if session
        .anomalies
        .iter()
        .any(|anomaly| anomaly.severity == "high" && anomaly.kind != "hanging")
    {
        "anomaly"
    } else if session
        .diagnostics
        .loop_fingerprints
        .iter()
        .any(|loop_| matches!(loop_.severity.as_str(), "high" | "critical"))
    {
        "loops"
    } else if metrics.cost_estimated >= ATTENTION_COST_USD {
        "cost"
    } else if p95_gap(session) >= ATTENTION_P95_GAP_SEC {
        "latency"
    } else {
        "ok"
    }
}

pub fn needs_attention(session: &Session) -> bool {
    inspect_reason(session) != "ok"
}

pub fn attention_priority(session: &Session) -> u8 {
    match inspect_reason(session) {
        "critical" | "anomaly" | "failures" | "loops" => 1,
        "latency" | "cost" => 2,
        _ => 3,
    }
}

pub fn attention_rank(
    session: &Session,
) -> (u8, i32, std::cmp::Reverse<usize>, std::cmp::Reverse<u64>) {
    (
        attention_priority(session),
        session.health,
        std::cmp::Reverse(session.anomalies.len() + session.metrics.tool_calls_fail),
        std::cmp::Reverse(session.metrics.cost_estimated.to_bits()),
    )
}

pub fn inspect_first(sessions: &[Session]) -> Vec<InspectFirst> {
    let mut items = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let candidates = [
        (
            "critical",
            inspect_by(
                sessions,
                |session| session.health < 50,
                |left, right| {
                    right
                        .health
                        .cmp(&left.health)
                        .then_with(|| cmp_cost(left, right))
                },
            ),
        ),
        (
            "anomaly",
            inspect_by(
                sessions,
                |session| !session.anomalies.is_empty(),
                |left, right| {
                    left.anomalies
                        .len()
                        .cmp(&right.anomalies.len())
                        .then_with(|| right.health.cmp(&left.health))
                        .then_with(|| cmp_cost(left, right))
                },
            ),
        ),
        (
            "failures",
            inspect_by(
                sessions,
                |session| session.metrics.tool_calls_fail > 0,
                |left, right| {
                    left.metrics
                        .tool_calls_fail
                        .cmp(&right.metrics.tool_calls_fail)
                        .then_with(|| right.health.cmp(&left.health))
                        .then_with(|| cmp_cost(left, right))
                },
            ),
        ),
        ("cost", inspect_by(sessions, |_| true, cmp_cost)),
        (
            "latency",
            inspect_by(
                sessions,
                |session| session.metrics.duration_sec > 0.0 || p95_gap(session) > 0.0,
                |left, right| {
                    left.metrics
                        .duration_sec
                        .total_cmp(&right.metrics.duration_sec)
                        .then_with(|| p95_gap(left).total_cmp(&p95_gap(right)))
                        .then_with(|| cmp_cost(left, right))
                },
            ),
        ),
    ];
    for (reason, index) in candidates {
        if let Some(index) = index.filter(|index| seen.insert(*index)) {
            items.push(InspectFirst { reason, index });
        }
    }
    items
}

fn inspect_by(
    sessions: &[Session],
    include: fn(&Session) -> bool,
    compare: fn(&Session, &Session) -> std::cmp::Ordering,
) -> Option<usize> {
    sessions
        .iter()
        .enumerate()
        .filter(|(_, session)| include(session))
        .max_by(|(left_index, left), (right_index, right)| {
            compare(left, right).then_with(|| right_index.cmp(left_index))
        })
        .map(|(index, _)| index)
}

fn cmp_cost(left: &Session, right: &Session) -> std::cmp::Ordering {
    left.metrics
        .cost_estimated
        .total_cmp(&right.metrics.cost_estimated)
        .then_with(|| crate::total_tokens(left).cmp(&crate::total_tokens(right)))
}

fn p95_gap(session: &Session) -> f64 {
    let mut gaps = session
        .metrics
        .gaps_sec
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect::<Vec<_>>();
    gaps.sort_by(f64::total_cmp);
    // House percentile definition only (rm-420): this used to be an
    // inline trunc+clamp copy — semantically right, but a literal
    // duplicate that could drift from the pinned crate definition.
    crate::percentile(&gaps, 0.95)
}

pub(crate) fn analyze_diagnostics(events: &[Event], metrics: &Metrics) -> Diagnostics {
    Diagnostics {
        loop_cost: loop_cost(events, metrics.cost_estimated),
        loop_fingerprints: loop_fingerprints(events),
        tool_latencies: tool_latencies(events),
        context_utilization: context_utilization(events, &metrics.model_used),
        large_params: large_params(events),
        unused_tools: unused_tools(events),
        stuck_patterns: stuck_patterns(events, metrics),
        steps: trace_steps(events, &metrics.model_used),
    }
}

fn trace_steps(events: &[Event], _model: &str) -> Vec<TraceStep> {
    let results = events
        .iter()
        .filter(|event| event.role == "tool" && !event.tool_call_id.is_empty())
        .map(|event| (event.tool_call_id.as_str(), event))
        .collect::<HashMap<_, _>>();
    let mut steps = events
        .iter()
        .flat_map(|event| event.tool_calls.iter().map(move |call| (event, call)))
        .map(|(event, call)| {
            let result = results.get(call.id.as_str());
            let duration_sec = parse_ts(&event.timestamp)
                .zip(result.and_then(|event| parse_ts(&event.timestamp)))
                .map(|(start, end)| (end - start).num_milliseconds() as f64 / 1000.0)
                // rm-004 residual arm: this used to clamp at 3600s,
                // collapsing a 2h call to 0.0 while tool_latencies
                // kept it (7199.0s) — same 24h sanity bound now, so
                // steps and latencies agree on one report.
                .filter(|duration| *duration > 0.0 && *duration <= 86_400.0)
                .unwrap_or(0.0);
            TraceStep {
                kind: "tool".to_string(),
                name: call.name.clone(),
                started_at: event.timestamp.clone(),
                ended_at: result
                    .map(|event| event.timestamp.clone())
                    .unwrap_or_default(),
                duration_sec,
                status: match result {
                    Some(event) if event.is_error => "error",
                    Some(_) => "ok",
                    None => "missing",
                }
                .to_string(),
                tokens: 0,
                call_id: call.id.clone(),
                parent_id: String::new(),
            }
        })
        .collect::<Vec<_>>();
    // ponytail: keep the failure boundary, add pagination only if users need full local traces.
    if steps.len() > 200 {
        let omitted = steps.len() - 200;
        steps = steps.split_off(omitted);
        steps.insert(
            0,
            TraceStep {
                kind: "meta".to_string(),
                name: format!("{omitted} earlier tool steps omitted"),
                started_at: String::new(),
                ended_at: String::new(),
                duration_sec: 0.0,
                status: "truncated".to_string(),
                tokens: 0,
                call_id: String::new(),
                parent_id: String::new(),
            },
        );
    }
    steps
}

pub fn fix_suggestions(session: &Session) -> Vec<FixSuggestion> {
    let mut fixes = Vec::new();
    let total = session.metrics.tool_calls_total;
    for anomaly in &session.anomalies {
        let (title, description, action, category) = match anomaly.kind.as_str() {
            "hanging" => (
                "Add tool timeout",
                "Long gaps indicate an unbounded operation.",
                "Add cancellation and a bounded timeout to long-running tools.",
                "hanging",
            ),
            "tool_failures" if total > 0 => (
                "Reduce tool failures",
                "Repeated failures increase latency and cost.",
                "Inspect failed arguments and stop retrying unchanged calls.",
                "tool_failure",
            ),
            "shallow_thinking" => (
                "Add an explicit plan",
                "The session executed risky steps with little planning evidence.",
                "Plan and verify the risky steps before execution.",
                "thinking",
            ),
            "redaction" => (
                "Review redacted reasoning",
                "Redacted reasoning limits failure attribution.",
                "Check whether missing reasoning hides the failure boundary.",
                "redaction",
            ),
            "no_tools" => (
                "Use available tools",
                "The session did not inspect concrete artifacts.",
                "Inspect concrete artifacts instead of relying on chat-only reasoning.",
                "no_tools",
            ),
            _ => continue,
        };
        fixes.push(FixSuggestion {
            title: title.to_string(),
            description: description.to_string(),
            action: action.to_string(),
            severity: anomaly.severity.clone(),
            category: category.to_string(),
        });
    }
    fixes
}

pub fn predict_cost_anomaly(history: &[Session], current: &Session) -> CostAlert {
    let costs = history
        .iter()
        // Only sessions with a real (non-zero) cost estimate belong in
        // the baseline: zero-cost sessions are unpriced, not free, and
        // dragging them into the mean fabricates a cheap baseline that
        // inflates every anomaly ratio (rm-004).
        .filter(|session| {
            session.path != current.path
                && session.metrics.assistant_turns > 0
                && session.metrics.cost_estimated > 0.0
        })
        .map(|session| session.metrics.cost_estimated / session.metrics.assistant_turns as f64)
        .collect::<Vec<_>>();
    if costs.is_empty() || current.metrics.assistant_turns == 0 {
        return CostAlert {
            level: "info".to_string(),
            message: "No comparable cost history.".to_string(),
            ..CostAlert::default()
        };
    }
    let baseline = costs.iter().sum::<f64>() / costs.len() as f64;
    let value = current.metrics.cost_estimated / current.metrics.assistant_turns as f64;
    let ratio = if baseline > 0.0 {
        value / baseline
    } else {
        0.0
    };
    let loop_pct = loop_waste_percent(
        current.diagnostics.loop_cost.total_loop_cost,
        current.metrics.cost_estimated,
    );
    let (triggered, level, message) = if ratio > 3.0 {
        (
            true,
            "critical",
            format!("Cost/turn is {ratio:.1}x the session baseline."),
        )
    } else if ratio > 2.0 {
        (
            true,
            "warning",
            format!("Cost/turn is {ratio:.1}x the session baseline."),
        )
    } else if loop_pct > 50.0 {
        (
            true,
            "critical",
            format!("Loop waste is {loop_pct:.0}% of session cost."),
        )
    } else if loop_pct > 30.0 {
        (
            true,
            "warning",
            format!("Loop waste is {loop_pct:.0}% of session cost."),
        )
    } else {
        (
            false,
            "info",
            format!("Cost/turn is {ratio:.1}x the session baseline."),
        )
    };
    CostAlert {
        triggered,
        level: level.to_string(),
        message,
        current: value,
        baseline,
        ratio,
    }
}

pub fn session_findings(session: &Session, history: &[Session]) -> Vec<SessionFinding> {
    let diagnostics = &session.diagnostics;
    let mut findings = Vec::new();
    if diagnostics.loop_cost.loop_groups > 0 {
        findings.push(finding(
            "loop",
            diagnostics.loop_cost.total_loop_cost,
            "",
            "high",
            "repeated_groups",
            diagnostics.loop_cost.loop_groups.to_string(),
        ));
    }
    if session.metrics.tool_calls_fail > 0 {
        findings.push(finding(
            "retry",
            session.metrics.tool_calls_fail as f64,
            "",
            if session.metrics.tool_calls_fail >= 3 {
                "high"
            } else {
                "medium"
            },
            "failed_calls",
            session.metrics.tool_calls_fail.to_string(),
        ));
    }
    if let Some(latency) = diagnostics
        .tool_latencies
        .iter()
        .filter(|item| item.max_sec >= 5.0 || item.unmatched > 0)
        .max_by(|left, right| left.max_sec.total_cmp(&right.max_sec))
    {
        findings.push(finding(
            "latency",
            latency.max_sec,
            &latency.tool_name,
            if latency.unmatched > 0 {
                "high"
            } else {
                "medium"
            },
            "slowest_tool",
            latency.tool_name.clone(),
        ));
    }
    if matches!(
        diagnostics.context_utilization.risk_level.as_str(),
        "warning" | "critical"
    ) {
        findings.push(finding(
            "context",
            diagnostics.context_utilization.utilization_pct,
            "",
            &diagnostics.context_utilization.risk_level,
            "context_used",
            format!("{:.0}%", diagnostics.context_utilization.utilization_pct),
        ));
    }
    if let Some(largest) = diagnostics.large_params.iter().max_by_key(|item| item.size) {
        findings.push(finding(
            "large_params",
            (largest.size / 1024) as f64,
            &largest.tool_name,
            &largest.risk,
            "largest_input",
            format!("{} KB", largest.size / 1024),
        ));
    }
    if let Some(pattern) = diagnostics.stuck_patterns.first() {
        findings.push(finding(
            "stuck",
            0.0,
            &pattern.description,
            &pattern.severity,
            "pattern",
            pattern.pattern.replace('_', " "),
        ));
    }
    let alert = predict_cost_anomaly(history, session);
    if alert.triggered {
        findings.push(finding(
            "cost",
            session.metrics.cost_estimated,
            &alert.message,
            &alert.level,
            "session_cost",
            format!("${:.2}", session.metrics.cost_estimated),
        ));
    }
    findings
}

fn finding(
    kind: &str,
    value: f64,
    detail: &str,
    severity: &str,
    evidence_kind: &str,
    evidence_value: String,
) -> SessionFinding {
    SessionFinding {
        kind: kind.to_string(),
        value,
        detail: detail.to_string(),
        severity: severity.to_string(),
        evidence: vec![FindingEvidence {
            kind: evidence_kind.to_string(),
            value: evidence_value,
        }],
    }
}

pub fn loop_waste_percent(loop_cost: f64, total_cost: f64) -> f64 {
    if !total_cost.is_finite() || total_cost <= 0.0 {
        return 0.0;
    }
    loop_cost.clamp(0.0, total_cost) / total_cost * 100.0
}

// Review F7 (rm-233, campaign-local rm-028): two argument objects that
// differ only in key order are the same arguments -- producers differ in
// key order across calls of one retry run, and raw string equality let
// those runs escape the loop key. serde_json is built with
// preserve_order, so parse -> serialize alone does not canonicalize;
// sort object keys explicitly (arrays keep order, scalars are
// unchanged). Non-JSON argument text falls back to comparing as raw
// text, as before.
fn canonical_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut pairs: Vec<(String, serde_json::Value)> = map
                .iter()
                .map(|(key, value)| (key.clone(), canonical_json(value)))
                .collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            pairs
                .into_iter()
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into()
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonical_json).collect())
        }
        other => other.clone(),
    }
}

fn canonical_args(args: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(args) {
        Ok(value) => {
            serde_json::to_string(&canonical_json(&value)).unwrap_or_else(|_| args.to_string())
        }
        Err(_) => args.to_string(),
    }
    // Residual (round-2 review R2-7): numbers are NOT normalized, so
    // 5 and 5.0 (and floats formatted differently by the producer)
    // remain different keys. Normalizing here would have to round-trip
    // through f64 and risks losing integer precision on large values;
    // real retry loops re-issue byte-identical argument text, so the
    // residual is accepted and recorded rather than traded for a new
    // precision defect. Key-order differences, the observed escape,
    // are canonicalized above.
}

fn loop_cost(events: &[Event], total_cost: f64) -> LoopCost {
    // rm-233 (campaign-local rm-028): a retry is the same call
    // re-issued -- same tool AND same
    // arguments. Keying on the tool name alone counted a parallel batch
    // of distinct-argument calls in one assistant turn as a retry loop
    // and priced it into loop_cost (the `Bash_loop` false positive).
    let mut last: Option<(&str, String)> = None;
    let mut consecutive = 0;
    let mut max_consecutive = 0;
    let mut max_tool = "";
    let mut retries = 0;
    let mut groups = 0;
    for call in events.iter().flat_map(|event| &event.tool_calls) {
        let call_args = canonical_args(&call.args);
        let same_call =
            matches!(&last, Some((name, args)) if *name == call.name && *args == call_args);
        if same_call {
            consecutive += 1;
            if consecutive >= 3 {
                retries += 1;
            }
        } else {
            if consecutive > max_consecutive {
                max_consecutive = consecutive;
                max_tool = last.map(|(name, _)| name).unwrap_or("");
            }
            if consecutive >= 3 {
                groups += 1;
            }
            consecutive = 1;
            last = Some((call.name.as_str(), call_args));
        }
    }
    if consecutive > max_consecutive {
        max_consecutive = consecutive;
        max_tool = last.map(|(name, _)| name).unwrap_or("");
    }
    if consecutive >= 3 {
        groups += 1;
    }
    let tool = if max_consecutive >= 3 {
        max_consecutive as f64 * 0.015
    } else {
        0.0
    };
    let retry = retries as f64 * 0.0075;
    let raw_total = tool + retry;
    let total = if total_cost.is_finite() && total_cost > 0.0 {
        raw_total.min(total_cost)
    } else {
        0.0
    };
    let scale = if raw_total > 0.0 {
        total / raw_total
    } else {
        0.0
    };
    LoopCost {
        retry_cost: retry * scale,
        tool_loop_cost: tool * scale,
        total_loop_cost: total,
        retry_events: retries,
        loop_groups: groups,
        loop_type: if max_consecutive >= 3 {
            format!("{max_tool}_loop")
        } else {
            String::new()
        },
        turns: max_consecutive,
    }
}

fn loop_fingerprints(events: &[Event]) -> Vec<LoopFingerprint> {
    let results = events
        .iter()
        .filter(|event| event.role == "tool" && !event.tool_call_id.is_empty())
        .map(|event| (event.tool_call_id.as_str(), hash(&event.content)))
        .collect::<HashMap<_, _>>();
    let pairs = events
        .iter()
        .flat_map(|event| &event.tool_calls)
        .filter_map(|call| {
            results
                .get(call.id.as_str())
                .map(|hash| (call.name.as_str(), *hash))
        })
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut start = 0;
    while start < pairs.len() {
        let mut end = start + 1;
        while end < pairs.len() && pairs[end] == pairs[start] {
            end += 1;
        }
        let count = end - start;
        if count >= 3 {
            out.push(LoopFingerprint {
                tool_name: pairs[start].0.to_string(),
                result_hash: format!("{:x}", pairs[start].1),
                count,
                first_index: start,
                last_index: end - 1,
                severity: if count >= 5 { "critical" } else { "high" }.to_string(),
                detail: format!(
                    "Tool '{}' returned the same result {count} times",
                    pairs[start].0
                ),
            });
        }
        start = end;
    }
    out
}

fn tool_latencies(events: &[Event]) -> Vec<ToolLatency> {
    let results = events
        .iter()
        .filter_map(|event| {
            parse_ts(&event.timestamp).map(|time| (event.tool_call_id.as_str(), time))
        })
        .filter(|(id, _)| !id.is_empty())
        .collect::<HashMap<_, _>>();
    let mut values: BTreeMap<String, (Vec<f64>, usize)> = BTreeMap::new();
    for event in events {
        let Some(start) = parse_ts(&event.timestamp) else {
            continue;
        };
        for call in &event.tool_calls {
            let entry = values.entry(call.name.clone()).or_default();
            if let Some(end) = results.get(call.id.as_str()) {
                let seconds = (*end - start).num_milliseconds() as f64 / 1000.0;
                // Non-positive durations mean disordered or identical
                // timestamps and carry no latency signal. Genuine calls
                // longer than an hour used to be silently dropped by a
                // 3600s cap; they are now kept up to a 24h sanity bound
                // on clock skew (rm-004).
                if seconds > 0.0 && seconds <= 86_400.0 {
                    entry.0.push(seconds);
                }
            } else {
                // No result event for this call id: the trace cannot
                // say it timed out — report it as unmatched (rm-004).
                entry.1 += 1;
            }
        }
    }
    let mut out = values
        .into_iter()
        .map(|(tool_name, (mut values, unmatched))| {
            values.sort_by(f64::total_cmp);
            let count = values.len() + unmatched;
            let avg_sec = if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            };
            // rm-420: a nearest-rank (ceil-1) copy used to live here and
            // read one rank low at every n ≡ 0 (mod 20) — a 19×2s+1×31s
            // corpus reported p95=2.0/is_slow=false beside max=31.0.
            // House definition only (crate::percentile, values already
            // sorted above via total_cmp).
            let p95_sec = crate::percentile(&values, 0.95);
            ToolLatency {
                tool_name,
                count,
                avg_sec,
                p95_sec,
                max_sec: values.last().copied().unwrap_or(0.0),
                min_sec: values.first().copied().unwrap_or(0.0),
                unmatched,
                is_slow: p95_sec > 30.0,
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| {
        b.max_sec
            .total_cmp(&a.max_sec)
            .then_with(|| a.tool_name.cmp(&b.tool_name))
    });
    out
}

fn context_utilization(events: &[Event], model: &str) -> ContextUtilization {
    // Denominator ladder (rm-231): (1) the model's vendor context window
    // from the active pricing catalog (LiteLLM `max_input_tokens`, carried
    // since the 2026-10-04 snapshot — 1M-window Claude models used to be
    // divided by a name-substring 200k guess); (2) the legacy
    // name-substring ladder for models the catalog cannot resolve,
    // labeled `window_source: "fallback"` so the estimate is disclosed
    // instead of silently asserted. The numerator is deliberately
    // untouched (see rm-436 for the separate numerator defect).
    let (total, window_source) = match crate::pricing::lookup_context_window(model) {
        Some(window) => (window as usize, "catalog"),
        None => (fallback_context_window(model), "fallback"),
    };
    context_utilization_with(events, total, window_source)
}

/// Documented fallback ladder for models the active pricing catalog
/// cannot resolve (rm-231). Order matters: gemini first (the only
/// 1M-token class reachable by substring), then claude, then the
/// gpt/deepseek 128k class, else the conservative default.
fn fallback_context_window(model: &str) -> usize {
    let lower = model.to_ascii_lowercase();
    if lower.contains("gemini") {
        1_048_576
    } else if lower.contains("claude") {
        200_000
    } else if lower.contains("gpt") || lower.contains("deepseek") {
        128_000
    } else {
        131_072
    }
}

fn context_utilization_with(
    events: &[Event],
    total: usize,
    window_source: &str,
) -> ContextUtilization {
    let tools = events
        .iter()
        .flat_map(|event| &event.tool_calls)
        .map(|call| call.name.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        .max(8)
        * 300;
    let history = events
        .iter()
        .map(|event| event.content.len() + event.reasoning.len())
        .sum::<usize>()
        / 2;
    let system = 12_000;
    let used = tools + history + system;
    let available = total.saturating_sub(used);
    ContextUtilization {
        estimated_total: total,
        tool_definitions: tools,
        conversation_history: history,
        system_prompt: system,
        available_for_task: available,
        utilization_pct: used as f64 / total as f64 * 100.0,
        risk_level: if available < 20_000 {
            "critical"
        } else if available < 50_000 {
            "warning"
        } else {
            "good"
        }
        .to_string(),
        suggestion: if available < 50_000 {
            "Reduce conversation or tool context before continuing.".to_string()
        } else {
            String::new()
        },
        window_source: window_source.to_string(),
    }
}

fn large_params(events: &[Event]) -> Vec<LargeParam> {
    events
        .iter()
        .flat_map(|event| event.tool_calls.iter().map(move |call| (event, call)))
        .filter_map(|(event, call)| {
            let size = call.args.len();
            (size > 10_000).then(|| LargeParam {
                tool_name: call.name.clone(),
                size,
                risk: if size > 50_000 { "high" } else { "medium" }.to_string(),
                timestamp: event.timestamp.clone(),
                detail: format!("Tool '{}' received {size} bytes of arguments", call.name),
            })
        })
        .collect()
}

fn unused_tools(events: &[Event]) -> Vec<UnusedTool> {
    let mut usage = BTreeMap::new();
    for call in events.iter().flat_map(|event| &event.tool_calls) {
        *usage.entry(call.name.clone()).or_insert(0) += 1;
    }
    usage
        .into_iter()
        .filter(|(_, count)| *count <= 2)
        .map(|(tool_name, call_count)| UnusedTool {
            tool_name,
            call_count,
            level: "rare".to_string(),
            detail: format!("Tool was used {call_count} time(s)."),
        })
        .collect()
}

fn stuck_patterns(events: &[Event], metrics: &Metrics) -> Vec<StuckPattern> {
    let mut out = Vec::new();
    let long_gaps = metrics.gaps_sec.iter().filter(|gap| **gap > 120.0).count();
    if long_gaps >= 3 {
        out.push(StuckPattern {
            pattern: "long_gaps".to_string(),
            description: format!("{long_gaps} gaps exceed 120s"),
            severity: "critical".to_string(),
        });
    }
    let mut content = BTreeMap::new();
    for event in events
        .iter()
        .filter(|event| event.role == "assistant" && event.content.len() > 50)
    {
        // Key on the full response: the old 50-char prefix key flagged
        // distinct long responses that merely shared an opening (rm-004).
        *content.entry(event.content.clone()).or_insert(0) += 1;
    }
    for count in content.into_values().filter(|count| *count >= 4) {
        out.push(StuckPattern {
            pattern: "repeated_response".to_string(),
            description: format!("Repeated assistant response {count} times"),
            severity: "warning".to_string(),
        });
    }
    let result_ids = events
        .iter()
        .filter(|event| event.role == "tool")
        .map(|event| event.tool_call_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let zombies = events
        .iter()
        .flat_map(|event| &event.tool_calls)
        .filter(|call| !call.id.is_empty() && !result_ids.contains(call.id.as_str()))
        .count();
    if zombies > 0 {
        out.push(StuckPattern {
            pattern: "zombie_tool_calls".to_string(),
            description: format!("{zombies} tool calls have no result"),
            severity: "warning".to_string(),
        });
    }
    out
}

// NOTE (rm-502): the strict `parse_time` copy that used to live here
// was removed — every timestamp parse in this module routes through
// lib.rs `parse_ts` (the single source of timestamp truth), so naive
// ISO stamps derive latencies and step durations exactly like
// RFC 3339 ones instead of vanishing.

fn hash(value: &str) -> u32 {
    value.bytes().take(200).fold(5381_u32, |hash, byte| {
        hash.wrapping_mul(33).wrapping_add(byte as u32)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCall;

    #[test]
    fn detects_loop_latency_large_params_and_stuck_signals() {
        let mut events = vec![Event {
            role: "user".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            ..Event::default()
        }];
        for index in 0..4 {
            events.push(Event {
                role: "assistant".to_string(),
                timestamp: format!("2026-01-01T00:00:0{}Z", index * 2 + 1),
                content: "same response long enough to qualify as repeated assistant output"
                    .to_string(),
                tool_calls: vec![ToolCall {
                    id: index.to_string(),
                    name: "bash".to_string(),
                    args: "x".repeat(10_001),
                }],
                ..Event::default()
            });
            events.push(Event {
                role: "tool".to_string(),
                tool_call_id: index.to_string(),
                content: "same failure".to_string(),
                is_error: true,
                timestamp: format!("2026-01-01T00:00:0{}Z", index * 2 + 2),
                ..Event::default()
            });
        }
        let metrics = Metrics {
            model_used: "gpt-5.1".to_string(),
            cost_estimated: 0.1,
            ..Metrics::default()
        };
        let diagnostics = analyze_diagnostics(&events, &metrics);
        assert_eq!(diagnostics.loop_fingerprints[0].count, 4);
        assert_eq!(diagnostics.loop_fingerprints[0].first_index, 0);
        assert_eq!(diagnostics.loop_fingerprints[0].last_index, 3);
        assert!(!diagnostics.loop_fingerprints[0].result_hash.is_empty());
        assert!(diagnostics.loop_cost.total_loop_cost > 0.0);
        assert_eq!(diagnostics.loop_cost.loop_type, "bash_loop");
        assert_eq!(diagnostics.loop_cost.turns, 4);
        assert_eq!(diagnostics.tool_latencies[0].count, 4);
        assert_eq!(diagnostics.tool_latencies[0].min_sec, 1.0);
        assert_eq!(diagnostics.large_params.len(), 4);
        assert!(!diagnostics.large_params[0].timestamp.is_empty());
        assert!(!diagnostics.large_params[0].detail.is_empty());
        assert!(diagnostics
            .stuck_patterns
            .iter()
            .any(|item| item.pattern == "repeated_response"));
    }

    #[test]
    fn trace_steps_keep_metadata_without_content_or_args() {
        let events = vec![
            Event {
                role: "assistant".to_string(),
                content: "private response".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                tool_calls: vec![crate::ToolCall {
                    id: "call-1".to_string(),
                    name: "shell".to_string(),
                    args: "secret=true".to_string(),
                }],
                ..Event::default()
            },
            Event {
                role: "tool".to_string(),
                content: "private result".to_string(),
                timestamp: "2026-01-01T00:00:02Z".to_string(),
                tool_call_id: "call-1".to_string(),
                ..Event::default()
            },
        ];
        let steps = trace_steps(&events, "gpt-test");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].name, "shell");
        assert_eq!(steps[0].duration_sec, 2.0);
        let json = serde_json::to_string(&steps).unwrap();
        assert!(!json.contains("private"));
        assert!(!json.contains("secret"));
    }

    #[test]
    fn loop_groups_counts_distinct_runs_not_a_flag() {
        // rm-004 golden: loop_groups reports the number of consecutive-tool
        // runs of length >= 3, not a 0/1 "any loop" flag.
        let mut events = Vec::new();
        let mut seq = 0usize;
        for (tool, count) in [("read", 4), ("write", 3), ("search", 2)] {
            for _ in 0..count {
                events.push(Event {
                    role: "assistant".to_string(),
                    timestamp: "2026-01-01T00:00:00Z".to_string(),
                    tool_calls: vec![ToolCall {
                        id: format!("call-{seq}"),
                        name: tool.to_string(),
                        args: "{}".to_string(),
                    }],
                    ..Event::default()
                });
                seq += 1;
            }
        }
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        assert_eq!(diagnostics.loop_cost.loop_groups, 2);
        assert_eq!(diagnostics.loop_cost.turns, 4);
        assert_eq!(diagnostics.loop_cost.retry_events, 3);
        assert_eq!(diagnostics.loop_cost.loop_type, "read_loop");
    }

    #[test]
    fn parallel_batch_of_distinct_argument_calls_is_not_a_loop() {
        // rm-233 + rm-230 golden (campaign-local rm-028/rm-025, assess
        // probe, run cbe30a9c): one assistant turn issuing three
        // same-name calls with DISTINCT arguments is a parallel batch --
        // keying retries on the tool name alone reported Bash_loop
        // (retry_events 1, loop_groups 1) and priced it into loop_cost,
        // while the dropped call/result join key made all three paired
        // results count as unmatched.
        let mut events = Vec::new();
        for (seq, args) in ["{\"path\":\"a\"}", "{\"path\":\"b\"}", "{\"path\":\"c\"}"]
            .into_iter()
            .enumerate()
        {
            let call_id = format!("call-{seq}");
            events.push(Event {
                role: "assistant".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                tool_calls: vec![ToolCall {
                    id: call_id.clone(),
                    name: "Bash".to_string(),
                    args: args.to_string(),
                }],
                ..Event::default()
            });
            events.push(Event {
                role: "tool".to_string(),
                timestamp: "2026-01-01T00:00:01Z".to_string(),
                content: "done".to_string(),
                tool_call_id: call_id,
                ..Event::default()
            });
        }
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        assert_eq!(diagnostics.loop_cost.retry_events, 0);
        assert_eq!(diagnostics.loop_cost.loop_groups, 0);
        assert_eq!(diagnostics.loop_cost.loop_type, "");
        assert_eq!(diagnostics.tool_latencies[0].count, 3);
        assert_eq!(diagnostics.tool_latencies[0].unmatched, 0);
    }

    #[test]
    fn identical_argument_retries_are_still_a_loop() {
        // rm-233 negative control (campaign-local rm-028): re-issuing
        // the same call -- same tool
        // AND same arguments -- is still a retry loop after the (name,
        // args) keying change.
        let mut events = Vec::new();
        for seq in 0..3 {
            let call_id = format!("call-{seq}");
            events.push(Event {
                role: "assistant".to_string(),
                timestamp: format!("2026-01-01T00:00:0{seq}Z"),
                tool_calls: vec![ToolCall {
                    id: call_id.clone(),
                    name: "Bash".to_string(),
                    args: "{\"path\":\"same\"}".to_string(),
                }],
                ..Event::default()
            });
            events.push(Event {
                role: "tool".to_string(),
                timestamp: format!("2026-01-01T00:00:0{}Z", seq + 1),
                content: "done again".to_string(),
                tool_call_id: call_id,
                ..Event::default()
            });
        }
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        assert_eq!(diagnostics.loop_cost.retry_events, 1);
        assert_eq!(diagnostics.loop_cost.loop_groups, 1);
        assert_eq!(diagnostics.loop_cost.turns, 3);
        assert_eq!(diagnostics.loop_cost.loop_type, "Bash_loop");
        assert_eq!(diagnostics.tool_latencies[0].unmatched, 0);
    }

    #[test]
    fn reordered_argument_keys_are_still_the_same_call_for_loop_keying() {
        // Review F7 (rm-233, campaign-local rm-028): producers may
        // serialize the same argument
        // object with keys in different order across calls of one retry
        // run. Raw string equality let that run escape the (name, args)
        // loop key -- the retry loop went undetected. Arguments that
        // differ only in key order are the same call and must still be
        // keyed together.
        let mut events = Vec::new();
        let arg_variants = [
            "{\"path\":\"same\",\"mode\":\"r\"}",
            "{\"mode\":\"r\",\"path\":\"same\"}",
            "{\"path\":\"same\",\"mode\":\"r\"}",
        ];
        for (seq, args) in arg_variants.iter().enumerate() {
            let call_id = format!("call-{seq}");
            events.push(Event {
                role: "assistant".to_string(),
                timestamp: format!("2026-01-01T00:00:0{seq}Z"),
                tool_calls: vec![ToolCall {
                    id: call_id.clone(),
                    name: "Read".to_string(),
                    args: args.to_string(),
                }],
                ..Event::default()
            });
            events.push(Event {
                role: "tool".to_string(),
                timestamp: format!("2026-01-01T00:00:0{}Z", seq + 1),
                content: "done".to_string(),
                tool_call_id: call_id,
                ..Event::default()
            });
        }
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        assert_eq!(
            diagnostics.loop_cost.loop_type, "Read_loop",
            "key-order-only argument differences must not break the retry key"
        );
        assert_eq!(diagnostics.loop_cost.retry_events, 1);
        assert_eq!(diagnostics.loop_cost.turns, 3);
    }

    #[test]
    fn cost_anomaly_baseline_ignores_unpriced_sessions() {
        // rm-004 golden: zero-cost sessions are excluded from the baseline
        // so they cannot dilute it. Old behavior put this at "critical"
        // (poisoned baseline 1.0 -> ratio 7.5); priced baseline 3.0/turn vs
        // current 7.5/turn is a 2.5x drift -> warning.
        let priced = |name: &str, cost: f64| Session {
            name: name.to_string(),
            path: format!("/tmp/{name}"),
            cwd: String::new(),
            metrics: Metrics {
                assistant_turns: 10,
                cost_estimated: cost,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        let current = priced("current", 75.0);
        let history = vec![priced("a", 30.0), priced("b", 0.0), priced("c", 0.0)];
        let alert = predict_cost_anomaly(&history, &current);
        assert_eq!(alert.level, "warning");
        assert!(alert.message.contains("2.5x"));
    }

    #[test]
    fn tool_latencies_p95_uses_house_percentile_at_mod_20_boundary() {
        // rm-420 golden: 19 x 2s + 1 x 31s. A nearest-rank (ceil-1)
        // percentile reads one rank low at exactly n=20 and reported
        // p95=2.0 / is_slow=false beside max_sec=31.0; the house
        // definition (crate::percentile) must report the 31s tail.
        let mut events = Vec::new();
        let mut t = 0;
        for index in 0..20 {
            let duration = if index == 0 { 31 } else { 2 };
            events.push(Event {
                role: "assistant".to_string(),
                timestamp: format!("2026-01-01T00:{:02}:{:02}Z", (t / 60) % 60, t % 60),
                tool_calls: vec![ToolCall {
                    id: format!("c{index}"),
                    name: "bash".to_string(),
                    args: "{}".to_string(),
                }],
                ..Event::default()
            });
            t += duration;
            events.push(Event {
                role: "tool".to_string(),
                timestamp: format!("2026-01-01T00:{:02}:{:02}Z", (t / 60) % 60, t % 60),
                tool_call_id: format!("c{index}"),
                content: "done".to_string(),
                ..Event::default()
            });
            t += 1;
        }
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        let latency = &diagnostics.tool_latencies[0];
        assert_eq!(latency.count, 20);
        assert!(
            (latency.p95_sec - 31.0).abs() < 1e-6,
            "p95 must be the 31s tail, got {}",
            latency.p95_sec
        );
        assert!((latency.max_sec - 31.0).abs() < 1e-6);
        assert!(latency.is_slow, "the 31s tail must trip the >30s slow gate");
    }

    #[test]
    fn p95_gap_uses_house_percentile_at_mod_20_boundary() {
        // rm-420 golden: gaps 1..=20s. House trunc(len*p) picks index 19
        // -> 20.0; a nearest-rank re-copy would pick 19.0. Pins the
        // p95_gap routing through crate::percentile.
        let mut session = session_with_cost("gaps", 1, 1.0);
        session.metrics.gaps_sec = (1..=20).map(|value| value as f64).collect();
        assert!((p95_gap(&session) - 20.0).abs() < 1e-6);
    }

    #[test]
    fn trace_steps_agree_with_tool_latencies_beyond_one_hour() {
        // rm-004 residual-arm golden: a 2h call must not collapse to
        // dur=0.0 in steps while tool_latencies keeps 7199s — one
        // report, one number.
        let events = vec![
            Event {
                role: "assistant".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                tool_calls: vec![ToolCall {
                    id: "long".to_string(),
                    name: "train".to_string(),
                    args: "{}".to_string(),
                }],
                ..Event::default()
            },
            Event {
                role: "tool".to_string(),
                timestamp: "2026-01-01T01:59:59Z".to_string(),
                tool_call_id: "long".to_string(),
                content: "done".to_string(),
                ..Event::default()
            },
        ];
        let steps = trace_steps(&events, "test-model");
        assert_eq!(steps.len(), 1);
        assert!(
            (steps[0].duration_sec - 7199.0).abs() < 1e-6,
            "2h call must report 7199s, got {}",
            steps[0].duration_sec
        );
        assert_eq!(steps[0].status, "ok");
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        assert!((diagnostics.tool_latencies[0].p95_sec - 7199.0).abs() < 1e-6);
    }

    #[test]
    fn tool_latencies_keep_long_calls_and_report_unmatched() {
        // rm-004 golden: calls over the old 3600s cap stay in the
        // distribution (a 4000s call is slow, not invisible), and calls
        // whose result event is absent count as unmatched, never as
        // "timeouts" the trace never recorded.
        let events = vec![
            Event {
                role: "assistant".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                tool_calls: vec![
                    ToolCall {
                        id: "call-long".to_string(),
                        name: "bash".to_string(),
                        args: "{}".to_string(),
                    },
                    ToolCall {
                        id: "call-never".to_string(),
                        name: "bash".to_string(),
                        args: "{}".to_string(),
                    },
                ],
                ..Event::default()
            },
            Event {
                role: "tool".to_string(),
                timestamp: "2026-01-01T01:06:40Z".to_string(),
                tool_call_id: "call-long".to_string(),
                content: "done".to_string(),
                ..Event::default()
            },
        ];
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        let latency = &diagnostics.tool_latencies[0];
        assert_eq!(latency.tool_name, "bash");
        assert_eq!(latency.count, 2);
        assert_eq!(latency.unmatched, 1);
        assert!((latency.max_sec - 4000.0).abs() < 1e-6);
        assert!(latency.avg_sec > 0.0);
    }

    #[test]
    fn repeated_response_requires_identical_full_content() {
        // rm-004 golden: two DISTINCT long responses sharing a 50-char
        // opening are not "the same response repeated". With the old
        // prefix key, 3+3 such responses fused into one count of 6 and
        // crossed the >=4 threshold as a false positive.
        let prefix = "p".repeat(50);
        let mut events = Vec::new();
        for (suffix, count) in [("a", 3), ("b", 3)] {
            for _ in 0..count {
                events.push(Event {
                    role: "assistant".to_string(),
                    timestamp: "2026-01-01T00:00:00Z".to_string(),
                    content: format!("{prefix}{suffix}"),
                    ..Event::default()
                });
            }
        }
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        assert!(!diagnostics
            .stuck_patterns
            .iter()
            .any(|pattern| pattern.pattern == "repeated_response"));

        // Four byte-identical responses still report exactly once, with
        // the true repeat count.
        for _ in 0..4 {
            events.push(Event {
                role: "assistant".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                content: format!("{prefix}c"),
                ..Event::default()
            });
        }
        let diagnostics = analyze_diagnostics(&events, &Metrics::default());
        let repeated: Vec<_> = diagnostics
            .stuck_patterns
            .iter()
            .filter(|pattern| pattern.pattern == "repeated_response")
            .collect();
        assert_eq!(repeated.len(), 1);
        assert_eq!(
            repeated[0].description,
            "Repeated assistant response 4 times"
        );
    }

    #[test]
    fn session_findings_reuse_diagnostic_rules() {
        let mut session = Session {
            name: "failed".to_string(),
            path: "failed".to_string(),
            metrics: Metrics {
                tool_calls_fail: 3,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 70,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
            cwd: String::new(),
        };
        session.diagnostics.loop_cost.loop_groups = 1;
        let findings = session_findings(&session, &[]);
        assert!(findings.iter().any(|finding| finding.kind == "loop"));
        assert!(findings
            .iter()
            .any(|finding| finding.kind == "retry" && finding.severity == "high"));
    }

    #[test]
    fn cost_alert_exposes_structured_baseline() {
        let historical = session_with_cost("old", 2, 0.2);
        let current = session_with_cost("new", 2, 0.8);
        let alert = predict_cost_anomaly(&[historical], &current);
        assert!(alert.triggered);
        assert_eq!(alert.current, 0.4);
        assert_eq!(alert.baseline, 0.1);
        assert_eq!(alert.ratio, 4.0);
    }

    #[test]
    fn attention_membership_excludes_ordinary_sessions() {
        let ordinary = session_with_cost("ordinary", 1, 0.05);
        let mut long_but_fine = session_with_cost("long", 1, 0.05);
        long_but_fine.metrics.duration_sec = 3_600.0;
        long_but_fine.metrics.gaps_sec = [vec![5.0; 40], vec![900.0]].concat();
        let mut one_failure = session_with_cost("one-failure", 1, 0.05);
        one_failure.metrics.tool_calls_total = 40;
        one_failure.metrics.tool_calls_fail = 1;
        let mut delayed = session_with_cost("delayed", 1, 0.05);
        delayed.metrics.gaps_sec = vec![150.0];
        let mut unhealthy = session_with_cost("unhealthy", 1, 0.05);
        unhealthy.health = 30;
        assert!(!needs_attention(&ordinary));
        assert!(!needs_attention(&long_but_fine));
        assert!(!needs_attention(&one_failure));
        assert_eq!(inspect_reason(&delayed), "latency");
        assert!(needs_attention(&delayed));
        assert!(needs_attention(&unhealthy));
    }

    #[test]
    fn inspect_reason_matches_inspect_first_priority() {
        let mut critical = session_with_cost("critical", 1, 0.1);
        critical.health = 20;
        let mut failing = session_with_cost("failing", 1, 5.0);
        failing.metrics.tool_calls_total = 10;
        failing.metrics.tool_calls_fail = 4;
        let costly = session_with_cost("costly", 1, 19.0);
        assert_eq!(inspect_reason(&critical), "critical");
        assert_eq!(inspect_reason(&failing), "failures");
        assert_eq!(inspect_reason(&costly), "cost");
        assert!(attention_rank(&critical) < attention_rank(&failing));
        assert!(attention_rank(&failing) < attention_rank(&costly));
        let ranked = inspect_first(&[costly, failing, critical]);
        assert_eq!(ranked[0].reason, "critical");
        assert_eq!(ranked[1].reason, "failures");
    }

    fn session_with_cost(path: &str, turns: usize, cost: f64) -> Session {
        Session {
            name: path.to_string(),
            path: path.to_string(),
            metrics: Metrics {
                assistant_turns: turns,
                cost_estimated: cost,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
            cwd: String::new(),
        }
    }

    #[test]
    fn context_utilization_divides_by_the_1m_vendor_window() {
        // rm-231 golden: a 1M-window Claude model (the fork's default
        // class since CC 2.1.284) used to be divided by the
        // name-substring 200k guess, overstating utilization ~5x. With
        // the catalog window the same history lands far from the
        // critical threshold.
        let events = vec![Event {
            role: "assistant".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            content: "x".repeat(40_000),
            ..Event::default()
        }];
        let utilization = context_utilization_with(&events, 1_000_000, "catalog");
        // tools = max(unique tools, 8) * 300 = 2_400; history = 40_000/2;
        // system = 12_000.
        assert_eq!(utilization.tool_definitions, 2_400);
        assert_eq!(utilization.conversation_history, 20_000);
        assert_eq!(utilization.system_prompt, 12_000);
        let used = 2_400 + 20_000 + 12_000;
        assert_eq!(utilization.estimated_total, 1_000_000);
        assert_eq!(utilization.available_for_task, 1_000_000 - used);
        assert!((utilization.utilization_pct - used as f64 / 1_000_000.0 * 100.0).abs() < 1e-9);
        assert_eq!(utilization.risk_level, "good");
        assert_eq!(utilization.window_source, "catalog");
    }

    #[test]
    fn context_utilization_labels_the_substring_ladder_as_fallback() {
        // rm-231: models the catalog cannot resolve keep the documented
        // ladder, but the estimate is disclosed via `window_source`
        // instead of being silently asserted as the real window.
        let events = vec![Event::default()];
        let utilization = context_utilization_with(&events, 131_072, "fallback");
        assert_eq!(utilization.estimated_total, 131_072);
        assert_eq!(utilization.window_source, "fallback");
    }

    #[test]
    fn fallback_context_window_ladder_is_unchanged_for_offline_models() {
        // The legacy ladder itself must stay stable: it is now the
        // documented fallback, not the primary denominator.
        assert_eq!(fallback_context_window("gemini-2.5-pro"), 1_048_576);
        assert_eq!(fallback_context_window("claude-opus-4-6"), 200_000);
        assert_eq!(fallback_context_window("gpt-5.2"), 128_000);
        assert_eq!(fallback_context_window("deepseek-v4"), 128_000);
        assert_eq!(fallback_context_window("totally-unknown"), 131_072);
    }
}

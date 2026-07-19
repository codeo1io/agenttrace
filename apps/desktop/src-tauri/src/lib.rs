use agenttrace_core::{
    average_health, compare_session_outcome, compute_overview, data_health, known_session_dirs,
    load_sessions_with_options, project_name, session_capability, session_findings, total_tokens,
    DataHealth, LoadOptions, LoadReport, Session, TraceStep,
};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Default)]
struct AppState {
    report: Mutex<Option<Arc<LoadReport>>>,
}

impl AppState {
    fn get_or_load(
        &self,
        load: impl FnOnce() -> anyhow::Result<LoadReport>,
    ) -> Result<Arc<LoadReport>, String> {
        let mut report = self.report.lock().map_err(|error| error.to_string())?;
        if report.is_none() {
            *report = Some(Arc::new(load().map_err(|error| error.to_string())?));
        }
        Ok(report.clone().unwrap_or_default())
    }

    fn replace(&self, report: LoadReport) -> Result<(), String> {
        *self.report.lock().map_err(|error| error.to_string())? = Some(Arc::new(report));
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceDto {
    name: String,
    detected: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceStateDto {
    sources: Vec<SourceDto>,
    local_only: bool,
    session_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionSummaryDto {
    id: String,
    name: String,
    project: String,
    source: String,
    model: String,
    started_at: String,
    duration_sec: f64,
    cost: f64,
    tokens: i64,
    health: i32,
    status: &'static str,
    tool_failures: usize,
    loop_cost: f64,
    capability: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AttentionDto {
    session_id: String,
    kind: String,
    value: f64,
    detail: String,
    severity: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HomeDto {
    total_sessions: usize,
    smooth_sessions: usize,
    total_cost: f64,
    potential_savings: f64,
    average_health: f64,
    attention: Vec<AttentionDto>,
    recent_sessions: Vec<SessionSummaryDto>,
    data_health: DataHealth,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionListDto {
    sessions: Vec<SessionSummaryDto>,
    total: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceDto {
    kind: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FindingDto {
    id: String,
    session_id: String,
    session_name: String,
    kind: String,
    value: f64,
    detail: String,
    severity: String,
    evidence: Vec<EvidenceDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionDetailDto {
    session: SessionSummaryDto,
    anomalies: Vec<EvidenceDto>,
    warnings: Vec<EvidenceDto>,
    findings: Vec<FindingDto>,
    context_utilization: f64,
    max_tool_latency_sec: f64,
    steps: Vec<TraceStep>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompareMetricDto {
    kind: &'static str,
    current: f64,
    previous: f64,
    unit: &'static str,
    lower_is_better: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompareDto {
    current: SessionSummaryDto,
    previous: SessionSummaryDto,
    outcome: &'static str,
    reasons: Vec<&'static str>,
    metrics: Vec<CompareMetricDto>,
}

fn load_sessions() -> anyhow::Result<LoadReport> {
    let mut seen = HashSet::new();
    let mut report = load_sessions_with_options(None, &LoadOptions::default());
    report.sessions.retain(|session| {
        seen.insert((
            session.path.clone(),
            session.name.clone(),
            session.metrics.session_start.clone(),
        ))
    });
    report.parsed = report.sessions.len();
    Ok(report)
}

fn sessions(state: &State<'_, AppState>) -> Result<Arc<LoadReport>, String> {
    state.get_or_load(load_sessions)
}

#[tauri::command]
fn refresh_sessions(state: State<'_, AppState>) -> Result<(), String> {
    let loaded = load_sessions().map_err(|error| error.to_string())?;
    state.replace(loaded)
}

fn session_id(session: &Session) -> String {
    let mut hasher = DefaultHasher::new();
    session.path.hash(&mut hasher);
    session.name.hash(&mut hasher);
    session.metrics.session_start.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn status(health: i32) -> &'static str {
    if health >= 80 {
        "smooth"
    } else {
        "attention"
    }
}

fn summary(session: &Session) -> SessionSummaryDto {
    SessionSummaryDto {
        id: session_id(session),
        name: session.name.clone(),
        project: project_name(session),
        source: display_source(&session.metrics.source_tool),
        model: session.metrics.model_used.clone(),
        started_at: session.metrics.session_start.clone(),
        duration_sec: session.metrics.duration_sec,
        cost: session.metrics.cost_estimated,
        tokens: total_tokens(session),
        health: session.health,
        status: status(session.health),
        tool_failures: session.metrics.tool_calls_fail,
        loop_cost: session.diagnostics.loop_cost.total_loop_cost,
        capability: session_capability(session),
    }
}

fn display_source(source: &str) -> String {
    match source {
        "codex_cli" => "Codex".to_string(),
        "claude_code" => "Claude Code".to_string(),
        "antigravity_cli" => "Antigravity".to_string(),
        "hermes" | "hermes_agent" | "hermes_db" | "hermes_jsonl" => "Hermes".to_string(),
        "qwen_code" => "Qwen Code".to_string(),
        "opencode" | "opencode_db" => "OpenCode".to_string(),
        "workbuddy" => "WorkBuddy".to_string(),
        "" | "unknown" => "Unknown".to_string(),
        value => value.replace('_', " "),
    }
}

fn findings_for(session: &Session, history: &[Session]) -> Vec<FindingDto> {
    session_findings(session, history)
        .into_iter()
        .map(|finding| FindingDto {
            id: format!("{}-{}", session_id(session), finding.kind),
            session_id: session_id(session),
            session_name: session.name.clone(),
            kind: finding.kind,
            value: finding.value,
            detail: finding.detail,
            severity: finding.severity,
            evidence: finding
                .evidence
                .into_iter()
                .map(|evidence| EvidenceDto {
                    kind: evidence.kind,
                    value: evidence.value,
                })
                .collect(),
        })
        .collect()
}

fn severity_rank(value: &str) -> u8 {
    match value {
        "critical" => 0,
        "high" => 1,
        "warning" | "medium" => 2,
        _ => 3,
    }
}

#[tauri::command]
fn detect_sources(state: State<'_, AppState>) -> anyhow::Result<SourceStateDto, String> {
    let sessions = sessions(&state)?;
    Ok(SourceStateDto {
        sources: known_session_dirs()
            .into_iter()
            .map(|source| SourceDto {
                name: source.name,
                detected: source.path.exists(),
            })
            .collect(),
        local_only: true,
        session_count: sessions.sessions.len(),
    })
}

#[tauri::command]
fn load_home(state: State<'_, AppState>) -> anyhow::Result<HomeDto, String> {
    let report = state.get_or_load(load_sessions)?;
    let sessions = &report.sessions;
    let overview = compute_overview(sessions);
    let total_sessions = sessions.len();
    let mut findings = sessions
        .iter()
        .flat_map(|session| findings_for(session, sessions))
        .collect::<Vec<_>>();
    findings.sort_by_key(|finding| severity_rank(&finding.severity));
    let attention = findings
        .into_iter()
        .take(3)
        .map(|finding| AttentionDto {
            session_id: finding.session_id,
            kind: finding.kind,
            value: finding.value,
            detail: finding.detail,
            severity: finding.severity,
        })
        .collect();
    Ok(HomeDto {
        total_sessions,
        smooth_sessions: overview.healthy,
        total_cost: overview.total_cost,
        potential_savings: sessions
            .iter()
            .map(|session| session.diagnostics.loop_cost.total_loop_cost)
            .sum(),
        average_health: average_health(sessions),
        attention,
        recent_sessions: sessions.iter().take(6).map(summary).collect(),
        data_health: data_health(sessions, sessions.len() + report.skipped, report.cache_hits),
    })
}

#[tauri::command]
fn list_sessions(
    state: State<'_, AppState>,
    query: String,
    status_filter: String,
) -> anyhow::Result<SessionListDto, String> {
    let sessions = sessions(&state)?;
    let sessions = &sessions.sessions;
    let query = query.trim().to_ascii_lowercase();
    let filtered = sessions
        .iter()
        .filter(|session| {
            let item = summary(session);
            (query.is_empty()
                || item.name.to_ascii_lowercase().contains(&query)
                || item.project.to_ascii_lowercase().contains(&query)
                || item.source.to_ascii_lowercase().contains(&query))
                && (status_filter == "all" || item.status == status_filter)
        })
        .map(summary)
        .collect::<Vec<_>>();
    Ok(SessionListDto {
        total: filtered.len(),
        sessions: filtered,
    })
}

#[tauri::command]
fn get_session(state: State<'_, AppState>, id: String) -> anyhow::Result<SessionDetailDto, String> {
    let sessions = sessions(&state)?;
    let sessions = &sessions.sessions;
    let session = sessions
        .iter()
        .find(|session| session_id(session) == id)
        .ok_or_else(|| "Session not found".to_string())?;
    Ok(SessionDetailDto {
        session: summary(session),
        anomalies: session
            .anomalies
            .iter()
            .map(|item| EvidenceDto {
                kind: item.kind.clone(),
                value: item.detail.clone(),
            })
            .collect(),
        warnings: session
            .tool_warnings
            .iter()
            .map(|item| EvidenceDto {
                kind: item.tool_name.clone(),
                value: item.detail.clone(),
            })
            .collect(),
        findings: findings_for(session, &sessions),
        context_utilization: session.diagnostics.context_utilization.utilization_pct,
        max_tool_latency_sec: session
            .diagnostics
            .tool_latencies
            .iter()
            .map(|item| item.max_sec)
            .fold(0.0, f64::max),
        steps: session.diagnostics.steps.clone(),
    })
}

#[tauri::command]
fn list_findings(state: State<'_, AppState>) -> anyhow::Result<Vec<FindingDto>, String> {
    let sessions = sessions(&state)?;
    let sessions = &sessions.sessions;
    let mut findings = sessions
        .iter()
        .flat_map(|session| findings_for(session, &sessions))
        .collect::<Vec<_>>();
    findings.sort_by_key(|finding| severity_rank(&finding.severity));
    Ok(findings)
}

#[tauri::command]
fn compare_sessions(
    state: State<'_, AppState>,
    id: String,
    previous_id: Option<String>,
) -> anyhow::Result<CompareDto, String> {
    let sessions = sessions(&state)?;
    let sessions = &sessions.sessions;
    let index = sessions
        .iter()
        .position(|session| session_id(session) == id)
        .ok_or_else(|| "Session not found".to_string())?;
    let previous_index = previous_id
        .and_then(|value| {
            sessions
                .iter()
                .position(|session| session_id(session) == value)
        })
        .filter(|candidate| *candidate != index)
        .or_else(|| (index + 1 < sessions.len()).then_some(index + 1))
        .or_else(|| (index > 0).then_some(index - 1))
        .ok_or_else(|| "Another session is required for comparison".to_string())?;
    let current = &sessions[index];
    let previous = &sessions[previous_index];
    let comparison = compare_session_outcome(current, previous);
    Ok(CompareDto {
        current: summary(current),
        previous: summary(previous),
        outcome: comparison.outcome,
        reasons: comparison.reasons,
        metrics: vec![
            CompareMetricDto {
                kind: "duration",
                current: current.metrics.duration_sec,
                previous: previous.metrics.duration_sec,
                unit: "sec",
                lower_is_better: true,
            },
            CompareMetricDto {
                kind: "cost",
                current: current.metrics.cost_estimated,
                previous: previous.metrics.cost_estimated,
                unit: "usd",
                lower_is_better: true,
            },
            CompareMetricDto {
                kind: "tool_failures",
                current: current.metrics.tool_calls_fail as f64,
                previous: previous.metrics.tool_calls_fail as f64,
                unit: "count",
                lower_is_better: true,
            },
            CompareMetricDto {
                kind: "repeated_work",
                current: current.diagnostics.loop_cost.total_loop_cost,
                previous: previous.diagnostics.loop_cost.total_loop_cost,
                unit: "usd",
                lower_is_better: true,
            },
        ],
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            refresh_sessions,
            detect_sources,
            load_home,
            list_sessions,
            get_session,
            list_findings,
            compare_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running AgentTrace desktop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_findings_are_plain_language_and_evidence_backed() {
        let sessions = agenttrace_core::demo_sessions().expect("demo sessions");
        let findings = sessions
            .iter()
            .flat_map(|session| findings_for(session, &sessions))
            .collect::<Vec<_>>();
        assert!(findings.iter().any(|item| item.kind == "retry"));
        assert!(findings.iter().all(|item| !item.evidence.is_empty()));
    }

    #[test]
    fn stable_summary_does_not_expose_local_path() {
        let sessions = agenttrace_core::demo_sessions().expect("demo sessions");
        let json = serde_json::to_string(&summary(&sessions[0])).expect("serialize summary");
        assert!(!json.contains("demo://"));
    }

    #[test]
    fn desktop_state_loads_once_until_explicit_refresh() {
        let state = AppState::default();
        let loads = std::cell::Cell::new(0);
        let first = state
            .get_or_load(|| {
                loads.set(loads.get() + 1);
                Ok(LoadReport {
                    sessions: agenttrace_core::demo_sessions()?,
                    ..LoadReport::default()
                })
            })
            .unwrap();
        let second = state
            .get_or_load(|| {
                loads.set(loads.get() + 1);
                Ok(LoadReport {
                    sessions: agenttrace_core::demo_sessions()?,
                    ..LoadReport::default()
                })
            })
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(loads.get(), 1);
        state.replace(LoadReport::default()).unwrap();
        assert!(state
            .get_or_load(|| unreachable!())
            .unwrap()
            .sessions
            .is_empty());
    }

    #[test]
    fn session_id_distinguishes_sessions_from_one_database() {
        let mut sessions = agenttrace_core::demo_sessions().unwrap();
        sessions[1].path = sessions[0].path.clone();
        assert_ne!(session_id(&sessions[0]), session_id(&sessions[1]));
    }

    #[test]
    fn desktop_dtos_match_frontend_contract_fields() {
        let sessions = agenttrace_core::demo_sessions().unwrap();
        let summary_json = serde_json::to_value(summary(&sessions[0])).unwrap();
        for key in [
            "id",
            "startedAt",
            "durationSec",
            "toolFailures",
            "loopCost",
            "capability",
        ] {
            assert!(
                summary_json.get(key).is_some(),
                "missing SessionSummary.{key}"
            );
        }
        let detail = SessionDetailDto {
            session: summary(&sessions[0]),
            anomalies: Vec::new(),
            warnings: Vec::new(),
            findings: findings_for(&sessions[0], &sessions),
            context_utilization: 0.0,
            max_tool_latency_sec: 0.0,
            steps: sessions[0].diagnostics.steps.clone(),
        };
        let detail = serde_json::to_value(detail).unwrap();
        for key in [
            "session",
            "findings",
            "contextUtilization",
            "maxToolLatencySec",
            "steps",
        ] {
            assert!(detail.get(key).is_some(), "missing SessionDetail.{key}");
        }
    }
}

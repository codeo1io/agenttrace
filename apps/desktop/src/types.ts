export type View = "home" | "sessions" | "discover" | "compare" | "settings";
export type FindingKind = "loop" | "retry" | "latency" | "context" | "large_params" | "stuck" | "cost" | string;
export type MetricKind = "duration" | "cost" | "tool_failures" | "repeated_work";

export interface SourceState { sources: { name: string; detected: boolean }[]; localOnly: boolean; sessionCount: number }
export interface SessionSummary {
  id: string; name: string; project: string; source: string; model: string; startedAt: string;
  durationSec: number; cost: number; tokens: number; health: number; status: "smooth" | "attention";
  toolFailures: number; loopCost: number;
  capability: "detailed" | "aggregate" | "limited";
}
export interface Finding {
  id: string; sessionId: string; sessionName: string; kind: FindingKind; value: number; detail: string;
  severity: string; evidence: { kind: string; value: string }[];
}
export interface HomeData {
  totalSessions: number; smoothSessions: number; totalCost: number; potentialSavings: number; averageHealth: number;
  attention: Pick<Finding, "sessionId" | "kind" | "value" | "detail" | "severity">[];
  recentSessions: SessionSummary[];
  dataHealth: { discovered: number; parsed: number; skipped: number; cache_hits: number; unknown_sources: number; unknown_models: number; fallback_pricing: number; latest_session_at: string; confidence: string; with_tokens: number; with_duration: number; with_tools: number; with_event_timing: number; with_diagnostics: number };
}
export interface SessionDetail {
  session: SessionSummary; anomalies: { kind: string; value: string }[]; warnings: { kind: string; value: string }[];
  findings: Finding[]; contextUtilization: number; maxToolLatencySec: number;
  steps: { kind: string; name: string; started_at: string; ended_at: string; duration_sec: number; status: string; tokens: number; call_id: string; parent_id: string }[];
}
export interface CompareData {
  current: SessionSummary; previous: SessionSummary; outcome: string; reasons: string[];
  metrics: { kind: MetricKind; current: number; previous: number; unit: string; lowerIsBetter: boolean }[];
}

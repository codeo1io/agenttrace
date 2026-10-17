mod cache_dir;
mod codex_replay;
mod demo;
mod diagnostics;
mod discovery;
mod doctor;
mod filters;
mod governance;
mod history;
mod insights;
mod otel;
mod parser;
mod pricing;
mod reports;
mod runtime_config;
mod search;
mod session_cache;
mod sqlite_sessions;
mod statusline;
mod subagents;
mod waste;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub use demo::{demo_sessions, DEMO_REPORT_EPOCH};
pub use diagnostics::{
    attention_priority, attention_rank, fix_suggestions, inspect_first, inspect_reason,
    loop_waste_percent, needs_attention, predict_cost_anomaly, session_findings,
    ContextUtilization, CostAlert, Diagnostics, FindingEvidence, FixSuggestion, InspectFirst,
    LargeParam, LoopCost, LoopCostBasis, LoopFingerprint, SessionFinding, StuckPattern,
    ToolLatency, TraceStep, UnusedTool,
};
pub use filters::{
    matches_numeric_filter, parse_finite_f64, parse_numeric_filter, parse_numeric_filter_i32,
    NumericFilterOp,
};

pub use codex_replay::{reconcile_codex_multiagent_replays, CodexReplayAdjustment};
pub use discovery::{
    collect_session_files, discover_session_dirs, find_session_files, known_session_dirs,
    load_sessions_from_dir, load_sessions_with_options, load_sessions_with_progress,
    load_sessions_with_progress_from_cache, load_sessions_with_progress_from_cache_mode,
    KnownSessionDir, LoadOptions, LoadProgress, LoadReport,
};
pub use doctor::{
    build_doctor_report, doctor_report_text, render_doctor_report, DoctorDirReport, DoctorReport,
};
pub use governance::{
    context_trends, cost_audit, delivery_evidence, delivery_evidence_with_git, mcp_governance,
    recommendations, session_cost_audit, ContextTrend, ContextTrendTotals, CostAudit,
    DeliveryEvidence, DeliverySummary, McpGovernance, McpGovernanceItem, ModelCostAudit,
    PriceBreakdown, PricingCoverage, ProjectContextTrend, Recommendation, SessionCostAudit,
    SessionDeliveryEvidence, TokenBreakdown,
};
pub use history::{history_path, merge_preserved_history, preserve_derived_history};
pub use insights::{
    compare_session_outcome, data_health, data_health_scoped, filter_sessions,
    future_admission_bound, project_decode_status, project_name, report_scope, resolve_project,
    session_admitted_since, session_beyond_future_grace, session_capability,
    session_matches_time_range, DataHealth, ProjectDecodeStatus, ProjectIdentity, ReportScope,
    SessionComparison, SourceScope, TimeRange, FUTURE_ADMISSION_GRACE_SECS,
};
pub use otel::{gen_ai_system_for_path, report_otel_export, SEMCONV_SNAPSHOT_DATE};
pub use parser::{
    parse_file, parse_raw_session, parse_stdin_bytes, ReadCapRefused, FILE_READ_CAP_BYTES,
};
pub use pricing::{
    cache_rate_census, catalog_identity, effort_tier_of, has_specific_price, list_pricing,
    lookup_context_window, lookup_price, lookup_price_tiered, pricing_cache_path, pricing_source,
    pricing_source_for, provider_for, render_model_pricing_list, render_test_match,
    specific_base_for_unpriced, update_pricing, CacheRateCensus, Price,
};
pub use reports::{
    add_baseline_comparison, guard_formula_json_leaves, report_compare, report_compare_json,
    report_compare_with_language, report_json, report_json_with_language, report_overview_html,
    report_overview_html_with_context, report_overview_json, report_overview_json_with_context,
    report_overview_json_with_health, report_overview_markdown,
    report_overview_markdown_with_context, report_overview_svg, report_overview_svg_with_context,
    report_overview_text, report_overview_text_with_context, report_text,
    report_text_with_language, BaselineBreaches, BaselineThresholds, ReportLanguage, SvgCardTheme,
    GROUP_TAKE,
};
pub use runtime_config::{
    get as runtime_config_overrides, set as set_runtime_config, RuntimeConfigOverrides,
};
pub use search::{
    report_search_json, report_search_json_detailed, report_search_text,
    report_search_text_detailed, search_sessions, search_sessions_detailed, SearchReport,
    TranscriptScanSummary, TRANSCRIPT_SCAN_FILE_BYTE_CAP, TRANSCRIPT_SCAN_TOTAL_BYTE_BUDGET,
};
pub use session_cache::{
    cached_session, clear_session_cache, effective_session_cache_entries, load_cached_sessions,
    load_cached_sessions_from_cache, load_session_cache, save_session_cache,
    session_cache_entries_source, session_cache_path, store_session, SessionCache,
    MAX_CONFIGURABLE_SESSION_CACHE_ENTRIES, MAX_SESSION_CACHE_ENTRIES,
    MIN_CONFIGURED_SESSION_CACHE_ENTRIES, SESSION_CACHE_SCHEMA_VERSION,
};
pub use sqlite_sessions::{
    load_sqlite_backed_sessions, load_sqlite_backed_sessions_reported,
    load_sqlite_backed_sessions_reported_bounded, skip_sqlite_backed_file_dir, SqliteDroppedRows,
    SqliteDuplicateDb, SqliteIngestReport, SqlitePartialDb, SqliteScanBudget, SqliteUnreadableDb,
};
pub use statusline::{
    correlated_prompt_ids, escape_spoofing_json, is_bidi_format_control,
    is_invisible_spoofing_control, is_spoofing_format_control, load_statusline_insights,
    render_budget_view, render_statusline_report, run_statusline_host, sanitize_line_segment,
    sanitize_output_document, statusline_budget_series, statusline_capture_path,
    statusline_insights, statusline_journal_stats, CapturedStatusline, StatuslineBudgetSeries,
    StatuslineContextUsage, StatuslineContextWindow, StatuslineInsights, StatuslineJournalStats,
    StatuslinePromptCache, StatuslineRateLimitState, StatuslineSessionContextWindow,
    StatuslineSessionPrompt, StatuslineSpendLimit,
};
pub use subagents::attribute_subagents;
pub use waste::{
    compute_waste_report, render_waste_report, render_waste_report_with_language,
    waste_report_json, WasteReport,
};

pub const VERSION: &str = match option_env!("AGENTTRACE_RELEASE_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Event {
    #[serde(
        default,
        rename = "role",
        alias = "Role",
        deserialize_with = "deserialize_string"
    )]
    pub role: String,
    #[serde(
        default,
        rename = "content",
        alias = "Content",
        deserialize_with = "deserialize_string"
    )]
    pub content: String,
    #[serde(
        default,
        rename = "timestamp",
        alias = "Timestamp",
        deserialize_with = "deserialize_string"
    )]
    pub timestamp: String,
    #[serde(
        default,
        rename = "reasoning",
        alias = "Reasoning",
        deserialize_with = "deserialize_string"
    )]
    pub reasoning: String,
    #[serde(default, rename = "redacted", alias = "Redacted")]
    pub redacted: bool,
    #[serde(
        default,
        rename = "cwd",
        alias = "CWD",
        deserialize_with = "deserialize_string"
    )]
    pub cwd: String,
    /// rm-585 (spend-by-branch, first cut): the branch the session ran
    /// on, captured from the claude-code lane's `gitBranch` envelope
    /// field onto the `session_meta` event beside `cwd`. Other lanes
    /// leave it empty and the `by_branch` rollup buckets them under
    /// "unknown"; the detached-HEAD literal "HEAD" is preserved here
    /// verbatim and normalized to the same bucket only in the rollup.
    #[serde(
        default,
        rename = "branch",
        alias = "Branch",
        deserialize_with = "deserialize_string"
    )]
    pub branch: String,
    /// rm-1405 reap (run 4f2af62e5b7e, "Claude org-dimension truth"):
    /// org identity captured from the claude-code lane's
    /// `$.session.identity` envelope block (`orgIdentifier` /
    /// `loginEmail`, claude-code 2.1.29x+) onto the session's meta
    /// event beside `branch`. Empty when the journal carries no
    /// identity; the `by_org` rollup leaves such sessions out
    /// entirely (no "unknown" bucket — zero-noise facet) instead of
    /// inventing an org.
    #[serde(
        default,
        rename = "org",
        alias = "Org",
        deserialize_with = "deserialize_string"
    )]
    pub org: String,
    /// Login email riding the same identity block. Provenance only —
    /// bucketing uses `org` — and empty when the journal carries no
    /// identity.
    #[serde(
        default,
        rename = "org_email",
        alias = "OrgEmail",
        deserialize_with = "deserialize_string"
    )]
    pub org_email: String,
    #[serde(
        default,
        rename = "tool_calls",
        alias = "ToolCalls",
        deserialize_with = "deserialize_tool_calls"
    )]
    pub tool_calls: Vec<ToolCall>,
    #[serde(
        default,
        rename = "tool_call_id",
        alias = "ToolCallID",
        deserialize_with = "deserialize_string"
    )]
    pub tool_call_id: String,
    #[serde(default, rename = "is_error", alias = "IsError")]
    pub is_error: bool,
    #[serde(
        default,
        rename = "Usage",
        alias = "usage",
        deserialize_with = "deserialize_usage_map"
    )]
    pub usage: BTreeMap<String, i64>,
    // rm-718: the sibling scalar fields accept both spellings; the
    // generic fallback lane is the only lane that deserializes foreign
    // JSON into `Event` (the dedicated parsers construct events by
    // hand), so accepting the lowercase spelling here is exactly the
    // case-normalization that lane was missing — journals carrying
    // `model_used` attributed to "default" and priced as unknown.
    // (The alias itself also landed via rm-616's generic-lane batch on
    // this branch — the two rows share the intake; rm-718 adds the
    // cache-spelling normalization in `lenient_usage_map` below.)
    #[serde(
        default,
        rename = "ModelUsed",
        alias = "model_used",
        deserialize_with = "deserialize_string"
    )]
    pub model_used: String,
    /// rm-610: the service tier the turn ran on, when the source
    /// journal discloses it (codex rollouts carry it as
    /// `payload.thread_settings.service_tier`, e.g. "flex" or
    /// "priority"). Flex turns price at the catalog row's flex slots
    /// (`Price::input_flex` ...). Absent on every other family and on
    /// turns whose journal is silent; `skip` keeps serialized events
    /// byte-identical to pre-rm-610 output.
    #[serde(
        default,
        rename = "ServiceTier",
        alias = "service_tier",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_tier: Option<String>,
    /// rm-485: cost-only credit attribution in USD. Carried by meta events
    /// whose adapter truth is a session-wide credit counter (Copilot's
    /// `totalNanoAiu`, 1 AIU = 1 AI credit = $0.01) rather than tokens the
    /// pricing table can price. Never summed across events: the session
    /// keeps the maximum, so repeated snapshots (checkpoints, shutdown after
    /// checkpoints) never double-count.
    #[serde(default, rename = "CreditUsd")]
    pub credit_usd: f64,
    #[serde(
        default,
        rename = "SourceTool",
        deserialize_with = "deserialize_string"
    )]
    pub source_tool: String,
    /// Upstream-recorded USD cost for THIS event's usage block, when the
    /// source journal recorded it (pi `type:"usage"` entries carry
    /// `usage.cost.total`, rm-436). Beats the catalog estimate: the
    /// recorded tokens are excluded from the catalog formula and
    /// priced at this value instead. Set programmatically by the pi
    /// parser only; never deserialized from foreign journals.
    #[serde(
        skip_deserializing,
        rename = "RecordedCostUSD",
        skip_serializing_if = "Option::is_none"
    )]
    pub recorded_cost_usd: Option<f64>,
    /// Parse-time disclosure counters riding on this event (pi-family
    /// journals, rm-436/rm-437): `pi_usage_entry:<kind>`,
    /// `pi_branches`, `pi_entry_skipped:<type>`,
    /// `pi_message_role:<role>`. Aggregated onto session metrics and
    /// data_health so documented-but-uncounted journal facts stay
    /// visible. Never conversation content.
    #[serde(
        skip_deserializing,
        rename = "DisclosureCounters",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub disclosure_counters: BTreeMap<String, i64>,
    /// rm-880: session-level wire facts the accounting reads but does
    /// not consume (codex 0.160.1 identity/lineage/quota wire:
    /// creator ids, fork/parent lineage, thread_name, service_tier,
    /// plan_type, …). Set programmatically by parsers only; never
    /// deserialized from foreign journals. Aggregated onto session
    /// metrics (last observation wins) so reports and the session
    /// cache disclose the hidden wire instead of dropping it unread.
    #[serde(
        skip_deserializing,
        rename = "WireMetadata",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub wire_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: String,
}

impl<'de> Deserialize<'de> for ToolCall {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let Some(obj) = value.as_object() else {
            return Ok(Self::default());
        };
        let function = obj.get("function").and_then(Value::as_object);
        Ok(Self {
            id: string_from_value(
                obj.get("id")
                    .or_else(|| obj.get("ID"))
                    .or_else(|| obj.get("call_id")),
            ),
            name: string_from_value(
                obj.get("name")
                    .or_else(|| obj.get("Name"))
                    .or_else(|| function.and_then(|function| function.get("name"))),
            ),
            args: json_value_string(
                obj.get("args")
                    .or_else(|| obj.get("Args"))
                    .or_else(|| obj.get("arguments"))
                    .or_else(|| obj.get("input"))
                    .or_else(|| function.and_then(|function| function.get("arguments"))),
            ),
        })
    }
}

/// Usage maps arrive from untrusted logs in several shapes (pass-7
/// P7-1): flat integers (the strict contract), numeric strings, and
/// Event-typed nested objects whose numeric leaves are worth keeping.
/// Coerce leniently instead of letting the whole event fail to
/// deserialize and drop the line.
fn deserialize_usage_map<'de, D>(deserializer: D) -> Result<BTreeMap<String, i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(lenient_usage_map(&value))
}

fn lenient_usage_map(value: &Value) -> BTreeMap<String, i64> {
    let mut usage = BTreeMap::new();
    // rm-718 F4 sweep: cross-provider spellings of the cache-read count
    // normalize onto the canonical key below — the canonical spelling
    // wins if both appear, deterministically in either key order and
    // never summed.
    let mut aliased_cache_read: Option<i64> = None;
    let Some(object) = value.as_object() else {
        return usage;
    };
    for (key, entry) in object {
        match entry {
            // Event-typed usage (e.g. `"input": {"tokens": 5}`): keep the
            // numeric leaves under a joined name.
            Value::Object(nested) => {
                for (leaf_key, leaf) in nested {
                    if let Some(number) = parser::number_as_i64(leaf) {
                        usage.insert(format!("{key}_{leaf_key}"), number);
                    }
                }
            }
            other => {
                let Some(number) = parser::number_as_i64(other) else {
                    continue;
                };
                match key.as_str() {
                    // Gemini wire (`cachedContentTokenCount`, the spelling
                    // the dedicated gemini lane already maps) and the
                    // OpenAI-compatible camelCase spelling.
                    "cachedContentTokenCount" | "cacheReadInputTokens" => {
                        aliased_cache_read = Some(number);
                    }
                    _ => {
                        usage.insert(key.clone(), number);
                    }
                }
            }
        }
    }
    if let Some(number) = aliased_cache_read {
        usage
            .entry("cache_read_input_tokens".to_string())
            .or_insert(number);
    }
    usage
}

fn deserialize_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Option::<Value>::deserialize(deserializer)? {
        Some(Value::String(text)) => text,
        Some(Value::Null) | None => String::new(),
        Some(value) => serde_json::to_string(&value).unwrap_or_default(),
    })
}

/// The canonical/alias spelling pairs on `Event` (serde `rename` +
/// `alias`). A foreign doc carrying BOTH spellings of a pair trips
/// serde's duplicate-field error, so the whole line drops as
/// `event_schema` — these are the pairs `Event`'s strict parse needs
/// pre-normalized (rm-860).
const EVENT_FIELD_SPELLING_PAIRS: &[(&str, &str)] = &[
    ("role", "Role"),
    ("content", "Content"),
    ("timestamp", "Timestamp"),
    ("reasoning", "Reasoning"),
    ("redacted", "Redacted"),
    ("cwd", "CWD"),
    ("tool_calls", "ToolCalls"),
    ("tool_call_id", "ToolCallID"),
    ("is_error", "IsError"),
    ("Usage", "usage"),
    ("ModelUsed", "model_used"),
];

/// rm-860: collapse duplicate case-variant spellings of `Event`
/// fields before the strict deserialize, so a doc carrying BOTH
/// `ModelUsed` and `model_used` (any [`EVENT_FIELD_SPELLING_PAIRS`]
/// pair) parses instead of dropping its content + usage as an
/// `event_schema` skip. Canonical-winner rule, disclosed as
/// `duplicate_spelling_normalized` on the informational channel:
/// the canonical (serde `rename`) spelling wins when it carries a
/// value; a null canonical yields to the alias. Returns how many
/// pairs were normalized. Docs carrying a single spelling are
/// untouched (byte-identical parse).
pub(crate) fn normalize_duplicate_field_spellings(value: &mut serde_json::Value) -> usize {
    let Some(object) = value.as_object_mut() else {
        return 0;
    };
    let mut normalized = 0;
    for (canonical, alias) in EVENT_FIELD_SPELLING_PAIRS {
        if object.contains_key(*canonical) && object.contains_key(*alias) {
            if object
                .get(*canonical)
                .is_some_and(serde_json::Value::is_null)
            {
                // A null canonical contributes nothing to the strict
                // parse; keep whichever spelling carries a value.
                object.remove(*canonical);
            } else {
                object.remove(*alias);
            }
            normalized += 1;
        }
    }
    normalized
}

fn deserialize_tool_calls<'de, D>(deserializer: D) -> Result<Vec<ToolCall>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?.unwrap_or(Value::Null);
    let Some(items) = value.as_array() else {
        return Ok(Vec::new());
    };
    Ok(items.iter().map(tool_call_from_value).collect())
}

fn tool_call_from_value(value: &Value) -> ToolCall {
    let Some(obj) = value.as_object() else {
        return ToolCall::default();
    };
    let function = obj.get("function").and_then(Value::as_object);
    ToolCall {
        id: string_from_value(
            obj.get("id")
                .or_else(|| obj.get("ID"))
                .or_else(|| obj.get("call_id")),
        ),
        name: string_from_value(
            obj.get("name")
                .or_else(|| obj.get("Name"))
                .or_else(|| function.and_then(|function| function.get("name"))),
        ),
        args: json_value_string(
            obj.get("args")
                .or_else(|| obj.get("Args"))
                .or_else(|| obj.get("arguments"))
                .or_else(|| obj.get("input"))
                .or_else(|| function.and_then(|function| function.get("arguments"))),
        ),
    }
}

fn string_from_value(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or("").to_string()
}

fn json_value_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(value) => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// rm-1509 (cycle 2, run b01c2b6a): one entry of the OpenCode v2
/// `session.summary_diffs` payload — upstream type
/// `FileDiff.LegacyInfo` (anomalyco/opencode
/// packages/schema/src/file-diff.ts): the writer's per-file diff
/// accounting beside the integer summary columns. The upstream
/// `patch` body is deliberately not mirrored; the delivery lane
/// reports counts and paths, not patch text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionFileDiff {
    /// The touched path, when the writer recorded one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub additions: i64,
    pub deletions: i64,
    /// Upstream: "added" | "deleted" | "modified" — recorded as
    /// free text, never enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// rm-1509 (cycle 2, run b01c2b6a): the `summary_diffs` per-file sums vs the
/// integer `summary_*` columns of the same session row. Constructed only when
/// both sides are present and they disagree — divergence is disclosed, never
/// silently reconciled (same class as ccusage #1883: exact-match assumptions
/// between recorded and derived totals pick wrong sides).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryDiffDivergence {
    pub per_file_additions: i64,
    pub per_file_deletions: i64,
    pub summary_additions: i64,
    pub summary_deletions: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricProvenance {
    #[serde(rename = "Tokens")]
    pub tokens: String,
    #[serde(rename = "Duration")]
    pub duration: String,
    #[serde(rename = "ToolResults")]
    pub tool_results: String,
    #[serde(rename = "Files")]
    pub files: String,
    #[serde(rename = "Cost")]
    pub cost: String,
    #[serde(rename = "PricingSource")]
    pub pricing_source: String,
    #[serde(rename = "Naming")]
    pub naming: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Metrics {
    pub events_total: usize,
    pub user_messages: usize,
    pub assistant_turns: usize,
    pub tool_results: usize,
    pub tool_calls_total: usize,
    pub tool_calls_ok: usize,
    pub tool_calls_fail: usize,
    pub tool_usage: BTreeMap<String, usize>,
    pub file_usage: BTreeMap<String, usize>,
    pub tool_arg_usage: BTreeMap<String, usize>,
    pub tool_authority: BTreeMap<String, usize>,
    pub highest_authority: String,
    pub reasoning_blocks: usize,
    /// Characters (Unicode scalar values), not bytes (pass-6 P6-4).
    pub reasoning_chars: usize,
    pub reasoning_lens: Vec<usize>,
    pub reasoning_redact: usize,
    pub tokens_input: i64,
    pub tokens_output: i64,
    /// Thinking tokens reported separately by the source (Gemini
    /// `thoughtsTokenCount` and OpenAI-compatible `reasoning_tokens`),
    /// billed at the output rate and already included in
    /// `tokens_output` (pass-9 CU-20).
    pub tokens_reasoning: i64,
    pub tokens_cache_w: i64,
    pub tokens_cache_r: i64,
    #[serde(skip)]
    pub timestamps: Vec<DateTime<Utc>>,
    pub gaps_sec: Vec<f64>,
    pub model_used: String,
    /// rm-610: the service tier the session's usage blocks all agree on
    /// (codex `thread_settings.service_tier`, e.g. "flex"). `None` when
    /// the journal is silent or the tiers mix — exactly the value the
    /// totals were billed on; consumed by the audit/waste recomputes and
    /// persisted by the session cache so warm hits replay tier-priced
    /// costs. `skip` keeps tier-less output byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// rm-991 (cycle 2, run b5affead): the agent name the opencode
    /// session DB records on its `session` table (`agent` column;
    /// live census 2026-10-12: 302/310 rows carry one, the rest are
    /// parent/subagent lanes). Empty when the lane carries no name —
    /// the `by_agent` rollup then falls back to the source-tool
    /// bucket exactly as before. `skip` keeps unnamed output
    /// byte-identical.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent_name: String,
    /// rm-991: the writer-reported diff summary opencode stores beside
    /// the session (`summary_additions` / `summary_deletions` /
    /// `summary_files`), each read leniently — `None` when the column
    /// or the value is absent, and the by_agent split never depends on
    /// them. `reported_by_agent` in `provenance.files` is the only
    /// consumer that upgrades on `summary_files`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_additions: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_deletions: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_files: Option<i64>,
    /// rm-1509 (cycle 2, run b01c2b6a): the OpenCode v2
    /// `session.summary_diffs` json column — the writer's per-file
    /// diff list (`FileDiff.LegacyInfo[]` upstream,
    /// packages/schema/src/file-diff.ts), read behind the same
    /// column-presence probe as the `summary_*` integers so 1.x
    /// stores parse identically. `None` when the column is absent
    /// or the row's value is NULL. The `patch` body is deliberately
    /// not carried: delivery evidence reports counts and paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_diffs: Option<Vec<SessionFileDiff>>,
    /// rm-1509: rows whose `summary_diffs` payload failed to decode
    /// (not json / wrong shape). Counted, never silently dropped.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub summary_diffs_dropped: u32,
    /// rm-1509: per-file sums vs the integer `summary_*` columns on
    /// the same row — `Some` only when both sides are known and they
    /// disagree. Divergence is disclosed, never silently reconciled
    /// (same class as ccusage #1883: recorded vs derived totals).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_diffs_divergence: Option<SummaryDiffDivergence>,
    pub source_tool: String,
    pub session_start: String,
    pub session_end: String,
    /// Event timestamps accepted by the lenient naive-ISO arm and
    /// reinterpreted as UTC (no offset in the source log; rm-502).
    /// Zero unless the session's log carried naive stamps.
    pub naive_utc_stamps: usize,
    pub duration_sec: f64,
    pub cost_estimated: f64,
    /// rm-485: session-wide cost-only credit total (USD), folded from meta
    /// events with max semantics. Zero for every non-credit adapter, so the
    /// field stays out of their serialized output.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub credit_usd: f64,
    /// How far message-derived token aggregation drifted from the
    /// authoritative totals stored on the session row (SQLite sources).
    /// Zero unless stored totals were applied.
    pub stored_totals_delta: i64,
    /// Parse lines lost inside this session's source file, by reason
    /// (pass-7 P7-1): `unparseable_line`, `event_schema`, `non_event`.
    /// Parse loss and lost-accounting disclosures. Assumption
    /// disclosures — including the rm-450 workbuddy basis counters
    /// (`workbuddy_input_basis:*`) — live in `disclosure_counters`
    /// (rm-538) so they no longer degrade confidence like a lost line
    /// would. The rm-616 accounting-drop disclosures stay on this
    /// channel because they mark lost accounting, not an assumption:
    /// `model_or_usage_dropped` (a rejected line that itself carried
    /// model identity or a usage block) and
    /// `usage_present_not_counted` (a conversation line whose usage fell
    /// outside its family's counted lane while the session estimated).
    /// Empty for clean parses.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub line_skips: BTreeMap<String, usize>,
    /// rm-408: usage blocks the transcript REPORTED as all zero
    /// (recognized keys, no positive values). Counted as measured
    /// zeros and disclosed — never silently clean, never rerouted to
    /// the text-estimate fallback (absent usage keeps that path).
    #[serde(skip_serializing_if = "usize_is_zero")]
    pub zero_usage_events: usize,
    /// USD cost the source journal recorded for usage blocks (pi
    /// `type:"usage"` entries carrying `usage.cost.total`, rm-436).
    /// Those tokens are excluded from the catalog formula and added at
    /// face value instead of being estimated at the session model's
    /// rate.
    #[serde(skip_serializing_if = "zero_usd")]
    pub upstream_cost_usd: f64,
    /// Token classes already carrying an upstream-recorded cost
    /// (counted in the class totals above for visibility, excluded
    /// from the catalog formula — rm-436). Persisted so downstream
    /// audits can recompute a stored estimate under rm-436 semantics
    /// (catalog × unpriced tokens + recorded USD) instead of comparing
    /// a catalog-only total against a recorded-cost-inclusive one and
    /// false-tripping a drift note on every recorded-cost session
    /// (rm-520). Cache rows written before the schema 41 bump surface
    /// with these all-zero beside a positive `upstream_cost_usd` —
    /// that shape is a basis that cannot be reproduced, never
    /// evidence of zero upstream-priced tokens (rm-578 withholds the
    /// re-price there).
    #[serde(skip_serializing_if = "zero_i64")]
    pub upstream_priced_input: i64,
    #[serde(skip_serializing_if = "zero_i64")]
    pub upstream_priced_output: i64,
    #[serde(skip_serializing_if = "zero_i64")]
    pub upstream_priced_cache_w: i64,
    #[serde(skip_serializing_if = "zero_i64")]
    pub upstream_priced_cache_r: i64,
    /// Parse-time disclosure counters: journal facts the accounting
    /// keeps visible without treating them as loss (pi-family,
    /// rm-436/rm-437): `pi_usage_entry:<kind>` per usage-entry kind,
    /// `pi_branches` when a v2/v3 tree journal has more than one
    /// branch end, `pi_entry_skipped:<type>` /
    /// `pi_message_role:<role>` for documented shapes the accounting
    /// intentionally does not count; plus the workbuddy input-basis
    /// assumption notes (rm-450 minted, rm-538 moved here):
    /// `workbuddy_input_basis:cache_subtracted` when a priced journal
    /// already subtracted cache from input, and
    /// `workbuddy_input_basis:zeroed_suspected_mismatch` when a usage
    /// block reports cache but zeroes input — assumptions, not lost
    /// lines, so they never degrade `data_health.confidence` (rm-538).
    /// The same non-loss channel carries the parse-time siblings:
    /// `workbuddy_input_basis:cache_clamped` (rm-600, upstream #316 —
    /// a cache count clamped down to the remaining input) and
    /// `workbuddy_usage_dropped:function_call_result` (rm-497 residual,
    /// run 1f12309a — a result-record usage echo kept visibly
    /// uncounted because counting it would double-bill the request
    /// its message/function_call record already summed).
    /// rm-1508 (run 5c144613 cycle 4): `cache_rate_fallback` — pricing
    /// billed cache tokens at the absent-rate fallback (the catalog
    /// row publishes no cache rate; cr bills at the input rate, cw at
    /// 1.25× input). Counted per application (session-rate arm or
    /// per-block arm); suppressed counterfactuals do not fire it.
    /// Empty for journals without disclosures.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub disclosure_counters: BTreeMap<String, usize>,
    /// rm-880: wire facts read off the journal that the accounting
    /// does not consume — codex 0.160.1 session_meta identity/lineage
    /// (creator_user_id/creator_account_id, forked_from_id/
    /// forked_from_ordinal_exclusive, parent_thread_id, history_base,
    /// agent persona), session_configured (thread_name,
    /// model_provider_id, service_tier), and the rate_limits quota
    /// snapshot (plan_type, limit id/name). Values are sanitized and
    /// capped through the rm-594 treatment; last observation wins.
    /// Empty for journals without wire metadata.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub wire_metadata: BTreeMap<String, String>,
    /// rm-406 (dated 2026-10-07 arm): per-model token/cost attribution
    /// for sessions whose usage blocks name more than one model
    /// (Claude Code advisor turns ride message.usage.iterations[] with
    /// their own model; pi model_change journals switch mid-session).
    /// Populated only when the multi-model pricing arm runs —
    /// single-model sessions keep their exact pre-existing serialized
    /// shape — and consumed by the overview's by_model rollup so a
    /// session's advisor spend lands on the advisor's model instead of
    /// vanishing under the "multiple" label. `sessions` in a
    /// by_model bucket counts sessions that USED the model, so a
    /// multi-model session appears in more than one bucket.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub model_attribution: BTreeMap<String, ModelAttribution>,
    pub provenance: MetricProvenance,
    /// rm-545: subagent transcripts attributed to this session. Kept
    /// SEPARATE from the totals above so fleet rollups still count each
    /// transcript exactly once; linked after load, never cached.
    #[serde(skip_serializing_if = "usize_is_zero")]
    pub subagent_count: usize,
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub subagent_cost: f64,
    #[serde(skip_serializing_if = "i64_is_zero")]
    pub subagent_tokens: i64,
    /// rm-545: parent transcript path when this session is itself a
    /// Claude Code subagent transcript (`<session>/subagents/agent-*`),
    /// or — rm-791 — the source parent row id for a sqlite-backed
    /// subagent child (opencode `parent_id`), resolved and rolled up by
    /// `attribute_subagents` after load.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parent_session: String,
    /// rm-790: per-source row identity for DB-resident lanes (opencode
    /// `session.id`, hermes sessions `id`); empty for transcript lanes.
    /// Joins the derived-history session id preimage so two sessions
    /// from the same database file that start in the same second no
    /// longer fold onto one history row, and anchors rm-791's parent
    /// linkage for sqlite-backed children.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session_key: String,
}

fn i64_is_zero(value: &i64) -> bool {
    *value == 0
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

/// rm-406: one model's slice of a multi-model session's usage, priced
/// at that model's own catalog rate by the same per-block arithmetic
/// that prices the session total.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelAttribution {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_w: i64,
    pub cache_r: i64,
    pub cost_usd: f64,
}

fn usize_is_zero(value: &usize) -> bool {
    *value == 0
}

fn zero_usd(value: &f64) -> bool {
    *value == 0.0
}

fn zero_i64(value: &i64) -> bool {
    *value == 0
}

/// rm-408: a present usage block whose recognized values are all zero.
/// Every parser attaches zeros through `usage_from_value_with_keys`
/// (which inserts explicit `0` values), so a non-empty all-zero map is
/// exactly the client-reported-zero population — the one stated rule
/// across formats instead of per-format accidents.
pub fn usage_is_all_zero(usage: &BTreeMap<String, i64>) -> bool {
    !usage.is_empty() && usage.values().all(|value| *value == 0)
}

#[derive(Debug, Clone, Serialize)]
pub struct Anomaly {
    #[serde(rename = "type")]
    pub kind: String,
    pub severity: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolWarning {
    pub tool_name: String,
    pub pattern: String,
    pub count: usize,
    pub detail: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cwd: String,
    /// rm-585: branch the session ran on (claude-code `gitBranch`,
    /// first cut). Empty when the lane carries no branch; the overview
    /// `by_branch` rollup buckets empty and detached-HEAD ("HEAD")
    /// values under "unknown" instead of inventing a value.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub branch: String,
    /// rm-1405 reap (run 4f2af62e5b7e, "Claude org-dimension truth"):
    /// org identifier (`$.session.identity.orgIdentifier`) when the
    /// claude-code journal carries it; empty otherwise. The `by_org`
    /// rollup only buckets sessions with a non-empty org — no
    /// "unknown" bucket — so corpora without org data render
    /// identically to before (zero-noise facet).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub org: String,
    /// Login email riding the same identity block; provenance only
    /// (bucketing uses `org`), serialized only when present.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub org_email: String,
    pub metrics: Metrics,
    pub anomalies: Vec<Anomaly>,
    pub health: i32,
    pub tool_warnings: Vec<ToolWarning>,
    pub diagnostics: Diagnostics,
}

#[derive(Debug, Clone, Default)]
pub struct Overview {
    pub total_sessions: usize,
    pub total_cost: f64,
    /// rm-726: count of implausible-magnitude clamps disclosed by the
    /// member sessions (`magnitude_clamped:*` counters) — rendered
    /// beside the total cost so a clamped $42.7T cannot look clean.
    pub clamped_values: usize,
    /// rm-1500: count of member sessions whose `cost_estimated` is
    /// non-finite (poisoned or overflowing pricing inputs — e.g. a NaN
    /// REAL column read back from a sqlite lane). Their cost is
    /// EXCLUDED from every aggregate fold below (total, agent, model,
    /// provider, task-type, project, branch) and disclosed here and
    /// beside the rendered total, using the same `non_finite_costs`
    /// vocabulary as the insights data-health census (pass-8 F8-5) so
    /// `--overview` and the insights report agree on the exclusion —
    /// a poisoned session must never read as clean zero-spend.
    pub non_finite_costs: usize,
    pub healthy: usize,
    pub warning: usize,
    pub critical: usize,
    pub by_agent: BTreeMap<String, GroupOverview>,
    pub by_model: BTreeMap<String, GroupOverview>,
    pub by_project: BTreeMap<String, GroupOverview>,
    /// rm-245: vendor attribution from the pricing catalog (never
    /// invented from name prefixes). Models the catalog cannot resolve
    /// bucket explicitly under "unknown" instead of being dropped.
    pub by_provider: BTreeMap<String, GroupOverview>,
    /// rm-585 (spend-by-branch, first cut): branch attribution from the
    /// claude-code lane's `gitBranch` envelope. Every session lands in
    /// exactly one bucket: a branch name when carried, "unknown" when
    /// missing, detached-HEAD ("HEAD"), or from a lane that has no
    /// branch concept yet (codex/qwen/copilot — gap disclosed in the
    /// governance guide). Sessions and cost only, like by_provider.
    pub by_branch: BTreeMap<String, GroupOverview>,
    /// rm-1405 reap (run 4f2af62e5b7e, "Claude org-dimension truth"):
    /// org-attributed rollup from the claude-code
    /// `$.session.identity.orgIdentifier` dimension. Zero-noise by
    /// contract: only sessions that carry a non-empty org are
    /// bucketed — there is no "unknown" bucket, so corpora without
    /// org data leave the map empty and render identically to before
    /// (no facet key in report JSON, no section on text surfaces).
    /// Sessions and cost only, like by_branch.
    pub by_org: BTreeMap<String, GroupOverview>,
    /// rm-245: heuristic task-type attribution. Correlation over parsed
    /// aggregates, not ground truth; see `infer_task_type`.
    pub by_task_type: BTreeMap<String, TaskTypeOverview>,
    pub anomalies_top: Vec<AnomalyTop>,
}

/// Task-type rollup (rm-245). Carries token totals alongside sessions
/// and cost because "what kind of work burned the tokens" is the
/// question this dimension answers.
#[derive(Debug, Clone, Default)]
pub struct TaskTypeOverview {
    pub sessions: usize,
    pub cost: f64,
    pub tokens_input: i64,
    pub tokens_output: i64,
}

#[derive(Debug, Clone, Default)]
pub struct GroupOverview {
    pub sessions: usize,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnomalyTop {
    pub session: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub age: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cwd: String,
    pub source_tool: String,
    pub model: String,
    pub health: i32,
    pub cost: f64,
    pub tokens: i64,
    pub matches: Vec<String>,
    /// rm-1415: disclosure for this row's bounded transcript re-read lane
    /// ("scanned (N bytes)" or "skipped: <reason>"); the hand-rolled JSON
    /// writers carry it per row.
    pub transcript_scan: String,
}

/// rm-449 F4 (review fix bd6e4a50 F1): the hermes/generic lane consumes
/// ONLY the five canonical names from the TOP-LEVEL `usage` field
/// (`Event` deserializes `usage`; `session_from_events` reads
/// input/output/reasoning/cache-creation/cache-read there —
/// [`parser::USAGE_KEYS_CONSUMED`]). A wire key is DISCLOSED, minted
/// through the shared capped+sanitized helper (rm-594 composition),
/// whenever this lane's accounting cannot consume it at the location it
/// appears — never silently zeroed:
/// - `usage_unknown_key:<name>` — not in the shared alias vocabulary
///   at all (PoC p4-kimi's 300,190-byte hostile key);
/// - `usage_alias_unmapped:<name>` — a KNOWN wire synonym from another
///   tool's format that this lane does not map, wherever it appears
///   (top-level `prompt_tokens`/`completion_tokens`): tokens were
///   silently zeroed before this fix;
/// - `usage_unconsumed_location:<name>` — a CANONICAL key at a place
///   the accounting never reads: `message.usage`, `providerData.usage`,
///   or the top-level `usage` of a NON-meta line (the accumulator reads
///   usage only on `session_meta`/`meta` events).
///
/// Precedence is name-first: a known synonym reports as an alias even
/// at an unread location (the actionable fact is that this lane could
/// map it); a canonical name reports as relocated. Canonical keys at
/// the top-level `usage` field of a `session_meta`/`meta` line stay
/// silent — they are actually accounted. Nested objects flatten numeric
/// leaves to `{key}_{leaf}` first (mirroring `lenient_usage_map`), so a
/// nested `"input": {"tokens": 7}` on a meta line is consumed, not
/// disclosed.
///
/// rm-718 (integration of run 5417681937ae): the Event intake's
/// `lenient_usage_map` normalizes the two cross-provider spellings of
/// the cache-read count (`cachedContentTokenCount`, the Gemini wire
/// spelling, and `cacheReadInputTokens`, the OpenAI-compatible one)
/// onto the canonical `cache_read_input_tokens` key. At the location
/// the accounting reads (top-level `usage` of a meta line) those
/// spellings are therefore CONSUMED and stay silent like canonical
/// keys — disclosing them as unmapped there would be the exact
/// dishonesty this helper exists to prevent (a counted token labeled
/// as dropped, degrading `data_health.confidence`). At every other
/// location the accounting still never reads them, so they keep the
/// truthful `usage_alias_unmapped` tier below. When a block carries
/// both spellings the canonical value wins and the alias value is
/// discarded (never summed) — pinned by `disclosure_plane_honesty`'s
/// canonical-wins pair; the discarded alias stays silent there too.
fn usage_field_case_folded<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<&'a serde_json::Value> {
    if let Some(value) = object.get(key) {
        return Some(value);
    }
    let mut chars = key.chars();
    let first = chars.next()?;
    let mut folded = String::with_capacity(key.len());
    folded.push(first.to_ascii_uppercase());
    folded.push_str(chars.as_str());
    object.get(&folded)
}

pub(crate) fn unrecognized_usage_keys(value: &serde_json::Value) -> Vec<String> {
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    // rm-756 (run 9ab0afad assess F1): `Event` deserializes the usage
    // field from BOTH wire spellings (`rename = "Usage", alias =
    // "usage"`), so the probes below fold key case the same way serde
    // folds the field — the wire spelling first, then its
    // first-letter-capitalized twin (the same two-spelling fold the
    // `role`/`Role` probe applies). A line carrying BOTH spellings
    // fails Event's duplicate-field check and lands on `event_schema`,
    // so the lowercase probe is the only block a parsed line can owe.
    let message_usage = usage_field_case_folded(object, "message")
        .and_then(serde_json::Value::as_object)
        .and_then(|message| usage_field_case_folded(message, "usage"));
    let provider_usage = usage_field_case_folded(object, "providerData")
        .and_then(serde_json::Value::as_object)
        .and_then(|data| usage_field_case_folded(data, "usage"));
    // The accounting reads usage only on `session_meta`/`meta` events
    // (`session_from_events` below): a top-level `usage` on any other
    // role's line is exactly as unread as `message.usage`, so it gets
    // the same `usage_unconsumed_location` disclosure instead of the
    // silent zero it had before (role shape found while fixing review
    // bd6e4a50 F1 — real hermes journals carry usage on meta lines
    // only, so this fires on hostile/evolved shapes, not benign ones).
    let role = object
        .get("role")
        .or_else(|| object.get("Role"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let meta_line = matches!(role, "session_meta" | "meta");
    let mut keys = Vec::new();
    // (usage value, is this the place the lane's accounting reads?)
    for (usage, consumed_location) in [
        (usage_field_case_folded(object, "usage"), meta_line),
        (message_usage, false),
        (provider_usage, false),
    ]
    .into_iter()
    .filter_map(|(usage, consumed_location)| usage.map(|usage| (usage, consumed_location)))
    {
        let Some(map) = usage.as_object() else {
            continue;
        };
        for (wire_key, wire_value) in map {
            // Does this entry flatten to a canonical / recognized name
            // with a numeric value the accounting could take?
            let (canonical, recognized) = match wire_value {
                serde_json::Value::Object(nested) => {
                    let mut canonical = false;
                    let mut recognized = false;
                    for (leaf, leaf_value) in nested {
                        if parser::number_as_i64(leaf_value).is_some() {
                            let flat = format!("{wire_key}_{leaf}");
                            canonical |= parser::usage_key_is_consumed(&flat);
                            recognized |= parser::usage_wire_key_is_recognized(&flat);
                        }
                    }
                    (canonical, recognized)
                }
                other => {
                    let numeric = parser::number_as_i64(other).is_some();
                    (
                        numeric && parser::usage_key_is_consumed(wire_key),
                        numeric && parser::usage_wire_key_is_recognized(wire_key),
                    )
                }
            };
            // rm-718: the intake normalization (see the helper's doc
            // above) consumes the two cache-read alias spellings at
            // the read location — `cachedContentTokenCount` /
            // `cacheReadInputTokens` on a meta line's top-level
            // `usage` fold onto the canonical key, so they are
            // accounted there and must stay silent like a canonical
            // key. Everywhere else they fall through to the alias
            // tier unchanged.
            let normalized_cache_read_spelling = matches!(
                wire_key.as_str(),
                "cachedContentTokenCount" | "cacheReadInputTokens",
            );
            let prefix = if consumed_location && (canonical || normalized_cache_read_spelling) {
                continue; // actually accounted: canonical at the read
                          // place, or an rm-718-normalized spelling
                          // folded onto it
            } else if canonical {
                "usage_unconsumed_location"
            } else if recognized {
                "usage_alias_unmapped"
            } else {
                "usage_unknown_key"
            };
            keys.push(parser::disclosure_key(prefix, wire_key));
        }
    }
    keys
}

/// rm-594 residual (run 9ab0afad assess F4): the per-KEY sanitizer
/// ([`parser::disclosure_key`]) bounds one hostile key's length, but
/// nothing bounded how many DISTINCT keys the wire-controlled
/// disclosure channel could mint — a 4.6MB journal of unique `zz_*`
/// usage keys minted 50,000 `line_skips` entries, a 2.7MB diagnostics
/// JSON and megabyte single-line renders. Distinct wire-controlled
/// disclosure keys are now capped per session at
/// [`USAGE_DISCLOSURE_DISTINCT_KEY_CAP`]; once the cap is reached new
/// distinct keys stop entering the map and every suppressed occurrence
/// counts into [`DISCLOSURE_KEYS_CAPPED`], so the suppression itself is
/// disclosed instead of silent.
const USAGE_DISCLOSURE_DISTINCT_KEY_CAP: usize = 1_000;
const DISCLOSURE_KEYS_CAPPED: &str = "disclosure_keys_capped";

fn counts_toward_disclosure_cap(key: &str) -> bool {
    key.starts_with("usage_unknown_key:")
        || key.starts_with("usage_alias_unmapped:")
        || key.starts_with("usage_unconsumed_location:")
        || key.starts_with("kimi_usage_alias:")
}

/// Mint one disclosure key (with its occurrence count) into a
/// session's skip map under the rm-594 distinct-key cap. Non-disclosure
/// skip reasons (parse loss, schema skips) never pass through here and
/// stay uncapped.
pub(crate) fn mint_disclosure(
    skips: &mut BTreeMap<String, usize>,
    key: String,
    occurrences: usize,
) {
    if skips.contains_key(&key) {
        *skips.get_mut(&key).expect("checked contains_key") += occurrences;
        return;
    }
    let distinct = skips
        .keys()
        .filter(|key| counts_toward_disclosure_cap(key))
        .count();
    if distinct >= USAGE_DISCLOSURE_DISTINCT_KEY_CAP {
        *skips.entry(DISCLOSURE_KEYS_CAPPED.to_string()).or_insert(0) += occurrences;
        return;
    }
    skips.insert(key, occurrences);
}

pub fn parse_jsonl_session(name: &str, path: &str, raw: &str) -> anyhow::Result<Session> {
    let mut events = Vec::new();
    let mut line_objects = Vec::new();
    // Pass-7 P7-1: this fallback used to strict-parse each line and
    // silently `continue` past recoverable ones (lone surrogates,
    // Event-typed usage), losing whole lines with no health signal.
    // Lines now go through the same lenient machinery the format
    // detectors use, and every loss is counted with a reason.
    let mut line_skips: BTreeMap<String, usize> = BTreeMap::new();
    // rm-860: informational counter for duplicate-spelling
    // normalizations, merged into metrics.disclosure_counters (never
    // into line_skips — normalizing is not a loss).
    let mut normalized_spellings = 0usize;
    let count_skip = |reason: &str, skips: &mut BTreeMap<String, usize>| {
        *skips.entry(reason.to_string()).or_insert(0) += 1;
    };
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(mut value) = parser::parse_jsonl_value_lenient(line) else {
            count_skip("unparseable_line", &mut line_skips);
            // rm-616: a rejected line that itself carries model identity
            // or a usage block is not just a lost message — it is lost
            // ACCOUNTING. Count it separately so the drop is disclosed,
            // never silent. Unparseable lines get a conservative
            // substring check (quoted key forms for usage to avoid
            // `tool_usage`-shaped false positives).
            if line.contains("model_used")
                || line.contains("ModelUsed")
                || line.contains("\"usage\"")
                || line.contains("\"Usage\"")
            {
                count_skip("model_or_usage_dropped", &mut line_skips);
            }
            continue;
        };
        if line_objects.len() < 20 {
            if let Some(object) = value.as_object() {
                line_objects.push(object.clone());
            }
        }
        let has_event_type = value
            .as_object()
            .is_some_and(|object| object.contains_key("type"));
        // rm-449 F4: disclose every unconsumed usage wire key before the
        // event consume below — a usage entry this lane's accounting
        // cannot take (unknown name, known-but-unmapped alias, or a
        // recognized key at an ignored location) used to silently
        // produce zero tokens with no health signal (PoC p4-kimi; review
        // bd6e4a50 F1 counterexamples f4-alias/f4-loc). rm-594 residual:
        // the mint rides the distinct-key cap so wire-controlled
        // cardinality cannot balloon the disclosure map.
        for disclosure in unrecognized_usage_keys(&value) {
            mint_disclosure(&mut line_skips, disclosure, 1);
        }
        // rm-616: computed before `value` is consumed by the strict
        // Event parse — the same key census the drop markers use.
        let carries_model_or_usage = value.as_object().is_some_and(|object| {
            object.contains_key("model_used")
                || object.contains_key("ModelUsed")
                || object.contains_key("usage")
                || object.contains_key("Usage")
        });
        // rm-860: collapse duplicate case-variant spellings before the
        // strict Event parse — a doc carrying BOTH `ModelUsed` and
        // `model_used` (any rename+alias pair) used to trip serde's
        // duplicate-field error and vanish from every surface as an
        // `event_schema` drop. Canonical spelling wins; the
        // normalization is disclosed on the informational channel,
        // never counted as a drop.
        normalized_spellings += normalize_duplicate_field_spellings(&mut value);
        let Ok(mut event) = serde_json::from_value::<Event>(value) else {
            count_skip("event_schema", &mut line_skips);
            if carries_model_or_usage {
                count_skip("model_or_usage_dropped", &mut line_skips);
            }
            continue;
        };
        if event.role.is_empty() && !has_event_type {
            count_skip("non_event", &mut line_skips);
            if carries_model_or_usage {
                count_skip("model_or_usage_dropped", &mut line_skips);
            }
            continue;
        }
        if event.source_tool.is_empty() {
            event.source_tool = "generic".to_string();
        }
        events.push(event);
    }
    if events.is_empty() {
        anyhow::bail!("generic: unable to parse as JSONL or JSON array");
    }
    let source_tool = jsonl_source_tool(&line_objects);
    for event in &mut events {
        event.source_tool = source_tool.to_string();
    }
    let mut session = session_from_events(name, path, events)?;
    // rm-616: merge, never overwrite — analyze() may have put its own
    // disclosure counters (rm-450 workbuddy basis, rm-616
    // usage_present_not_counted) into metrics.line_skips, and clobbering
    // them here would silence exactly the disclosures that matter.
    for (reason, count) in line_skips {
        *session.metrics.line_skips.entry(reason).or_insert(0) += count;
    }
    if normalized_spellings > 0 {
        *session
            .metrics
            .disclosure_counters
            .entry("duplicate_spelling_normalized".to_string())
            .or_insert(0) += normalized_spellings;
    }
    Ok(session)
}

fn jsonl_source_tool(objects: &[serde_json::Map<String, Value>]) -> &'static str {
    for object in objects {
        let role = object
            .get("role")
            .or_else(|| object.get("Role"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if role == "session_meta" {
            return "hermes_jsonl";
        }
        if matches!(role, "user" | "assistant" | "tool")
            && (object.contains_key("timestamp") || object.contains_key("Timestamp"))
        {
            return "hermes_jsonl";
        }
    }
    "generic"
}

pub fn session_from_events(
    name: &str,
    path: &str,
    mut events: Vec<Event>,
) -> anyhow::Result<Session> {
    // rm-778: last-line containment at the ONE choke point every
    // format passes through — including the generic Vec<Event> lane
    // and deserialized journals, which never touch the five capped
    // read sites. Parser lanes cap at read time, so their events are
    // already within [`parser::SESSION_CWD_BYTE_CAP`] bytes and pass
    // through untouched; anything longer is capped HERE with the same
    // `cwd_truncated:<lane>` disclosure the read sites mint.
    for event in &mut events {
        let (capped, truncated) = crate::parser::cap_session_cwd(&event.cwd);
        if truncated {
            let lane = if event.source_tool.is_empty() {
                "generic"
            } else {
                event.source_tool.as_str()
            };
            event.cwd = capped;
            for (key, value) in crate::parser::cwd_truncation_disclosure(lane, true) {
                *event.disclosure_counters.entry(key).or_insert(0) += value;
            }
        }
    }
    let mut model = "default".to_string();
    let mut cwd = String::new();
    let mut branch = String::new();
    // rm-1405 reap (run 4f2af62e5b7e): org identity locals, captured
    // from the claude-code lane's meta-event carrier below.
    let mut org = String::new();
    let mut org_email = String::new();
    // rm-861: the session model label is only writable by rows that
    // speak for the session's model — assistant turns,
    // session/meta identity rows, and usage-bearing rows (a row that
    // carries usage declares which model the counted tokens belong
    // to). The scan used to be last-wins over EVERY role, so a
    // terminal user row naming a different model relabeled the whole
    // session (the label feeds by_model and pricing for sessions
    // whose usage is unconsumed). A non-authoritative row may still
    // SEED the label off "default" — lanes that stamp the model only
    // on tool rows keep their attribution — but once named, its
    // differing label is ignored and disclosed as
    // `model_label_non_assistant_ignored`.
    let mut non_assistant_model_ignored = 0usize;
    for event in &events {
        if !event.model_used.is_empty() && event.model_used != "unknown" {
            let authoritative =
                matches!(event.role.as_str(), "assistant" | "session_meta" | "meta")
                    || !event.usage.is_empty();
            if authoritative || model == "default" {
                model = event.model_used.clone();
            } else if event.model_used != model {
                non_assistant_model_ignored += 1;
            }
        }
        if cwd.is_empty() && !event.cwd.is_empty() {
            cwd = event.cwd.clone();
        }
        // rm-585: the branch rides the same session_meta/meta event as
        // cwd (claude-code writes both envelope fields on every line).
        if branch.is_empty() && !event.branch.is_empty() {
            branch = event.branch.clone();
        }
        // rm-1405 reap (run 4f2af62e5b7e): org identity rides the same
        // meta event — first non-empty capture wins; the parser only
        // sets the fields on one carrier per session.
        if org.is_empty() && !event.org.is_empty() {
            org = event.org.clone();
        }
        if org_email.is_empty() && !event.org_email.is_empty() {
            org_email = event.org_email.clone();
        }
        if model != "default" && !cwd.is_empty() {
            break;
        }
    }
    let mut metrics = analyze(&events, &model);
    if non_assistant_model_ignored > 0 {
        // rm-861 disclosure: ignored relabeling attempts stay visible
        // on the informational channel (a healthy corpus never fires
        // it — clean journals carry the model on authoritative rows
        // or not at all).
        *metrics
            .disclosure_counters
            .entry("model_label_non_assistant_ignored".to_string())
            .or_insert(0) += non_assistant_model_ignored;
    }
    // rm-726: drain the parser's thread-local magnitude-clamp census
    // (number_as_i64 clamps: u64 payloads above i64::MAX, saturating
    // f64s and strings) onto the disclosure channel at the one choke
    // point every format passes through — before anomaly detection, so
    // the cost truth carries its clamp count instead of rendering a
    // clean $42.7T from clamped bounds.
    let magnitude_clamped_above_i64 = crate::parser::take_magnitude_clamp_count() as usize;
    if magnitude_clamped_above_i64 > 0 {
        *metrics
            .disclosure_counters
            .entry("magnitude_clamped:above_i64".to_string())
            .or_default() += magnitude_clamped_above_i64;
    }
    // rm-726 x rm-831 reconciliation (conflict 606aefa4): the rm-831
    // sign gate (landed after rm-726 was authored) refuses negative
    // usage classes at intake, so most negatives never reach the
    // aggregator loop's `.max(0)` clamps below — the gate's refusals
    // are drained here onto the same negative_to_zero key so the
    // refusal stays a disclosed implausible-magnitude fact (the
    // in-loop census below remains as the backstop for lanes that
    // construct usage maps directly).
    let negative_usage_refused = crate::parser::take_negative_usage_refusal_count() as usize;
    if negative_usage_refused > 0 {
        *metrics
            .disclosure_counters
            .entry("magnitude_clamped:negative_to_zero".to_string())
            .or_default() += negative_usage_refused;
    }
    // rm-408 disclosure (arm a): count present-but-zero usage blocks at
    // the one choke point every format passes through, and flag them in
    // the provenance string so no surface reports clean zeros silently.
    let zero_usage_events = events
        .iter()
        .filter(|event| usage_is_all_zero(&event.usage))
        .count();
    if zero_usage_events > 0 {
        metrics.zero_usage_events = zero_usage_events;
        metrics.provenance.tokens = format!(
            "{}+zero_usage_reported:{}",
            metrics.provenance.tokens, zero_usage_events
        );
    }
    let display_name = session_display_name(name, &events);
    metrics.provenance.naming = if display_name != name {
        "first_user_request"
    } else {
        "file_name"
    }
    .to_string();
    let anomalies = detect_anomalies(&metrics);
    let health = health_score(&anomalies);
    let tool_warnings = validate_tool_warnings(&events);
    let diagnostics = diagnostics::analyze_diagnostics(&events, &metrics);
    Ok(Session {
        name: display_name,
        path: path.to_string(),
        cwd,
        branch,
        org,
        org_email,
        metrics,
        anomalies,
        health,
        tool_warnings,
        diagnostics,
    })
}

/// Single-source text→title transform for message-derived session names.
/// Shared by the JSONL path (per-event) and the SQLite path (first user
/// message text) so both name sessions identically.
pub(crate) fn display_title_from_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.starts_with("# AGENTS.md instructions")
        || text.starts_with("<environment_context>")
        || text.starts_with("Another language model started")
    {
        return None;
    }
    // Upstream v0.9.0 (#286) title cleaning: strip injected wrapper tags
    // (goal_context/user_query/...), "My request for Codex" wrappers, and
    // boilerplate prefixes before rendering. Composed here on top of the
    // fork's suppression list so both lineages' fixes hold.
    let text = clean_user_request(text)?;
    let title = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!title.is_empty()).then(|| {
        let mut chars = title.chars();
        let short = chars.by_ref().take(60).collect::<String>();
        if chars.next().is_some() {
            format!("{short}…")
        } else {
            short
        }
    })
}

fn session_display_name(fallback: &str, events: &[Event]) -> String {
    events
        .iter()
        .filter(|event| event.role == "user")
        .filter_map(|event| display_title_from_text(&event.content))
        .next()
        .unwrap_or_else(|| fallback.to_string())
}

/// Fallback token estimate, used only when no usage block exists and named
/// `estimated_from_text` in `provenance.tokens`. CJK-aware: ASCII text keeps
/// the classic 4-characters-per-token rate while every non-ASCII character
/// counts as roughly one token (pass-6 P6-4 — the previous bytes/4 heuristic
/// under-counted CJK by ~40–60% because CJK is ≥1 token per character but
/// only ~3/4 token per byte).
fn estimate_tokens_from_text(text: &str) -> i64 {
    // Scaled by 4 so an ASCII character contributes 1 and a non-ASCII
    // character contributes 4 (one whole token).
    let mut scaled: u64 = 0;
    for ch in text.chars() {
        scaled = scaled.saturating_add(if ch.is_ascii() { 1 } else { 4 });
    }
    (scaled / 4) as i64
}

const BOILERPLATE_USER_PREFIXES: &[&str] = &[
    "# AGENTS.md instructions",
    "Another language model started",
    "Recommended: install companion packages",
    "Caveat: The messages below were generated",
];

// Harness-injected wrappers carry no user intent; the goal wrapper is the exception.
const INJECTED_TAGS: &[&str] = &[
    "system-reminder",
    "user_action",
    "codex_internal_context",
    "environment_context",
    "turn_aborted",
    "recommended_plugins",
    "user_instructions",
    "image",
    "local-command-stdout",
    "local-command-caveat",
    "command-name",
    "command-message",
    "command-args",
];

const REQUEST_WRAPPER_TAGS: &[&str] = &["user_query", "USER_REQUEST", "teammate-message"];

fn clean_user_request(content: &str) -> Option<&str> {
    let mut text = content.trim();
    if let Some((_, request)) = text.split_once("## My request for Codex:") {
        text = request.trim();
    }
    while let Some(tag) = leading_tag_name(text) {
        let close = format!("</{tag}>");
        if tag == "goal_context" {
            return tag_body(text, "objective").map(str::trim);
        }
        if REQUEST_WRAPPER_TAGS.contains(&tag) {
            let body = text.find('>').map_or("", |end| &text[end + 1..]);
            text = body.split(close.as_str()).next().unwrap_or_default().trim();
            continue;
        }
        if !INJECTED_TAGS.contains(&tag) {
            break;
        }
        text = match text.find(&close) {
            Some(end) => text[end + close.len()..].trim_start(),
            None => text
                .find('>')
                .map_or("", |end| text[end + 1..].trim_start()),
        };
    }
    if text.is_empty()
        || BOILERPLATE_USER_PREFIXES
            .iter()
            .any(|prefix| text.starts_with(prefix))
    {
        return None;
    }
    Some(text)
}

fn leading_tag_name(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('<')?;
    let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))?;
    (end > 0).then(|| &rest[..end])
}

fn tag_body<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(&text[start..end])
}

/// Marker substring embedded in `provenance.pricing_source` whenever a
/// session's usage blocks were priced exactly per usage block (rm-438).
/// The mint in `analyze` and `governance::is_per_block_multiple` both go
/// through this const so the marker cannot be reworded on one side only.
pub(crate) const PER_BLOCK_PRICING_MARKER: &str = "priced per usage block";

/// rm-726: documented plausibility bound for a single usage value —
/// no real per-message token magnitude approaches it (largest model
/// contexts are ~2M tokens), so values at/above it are priced as
/// parsed but flagged implausible on the disclosure channel rather
/// than rendered as clean truth.
const IMPLAUSIBLE_TOKEN_MAGNITUDE: i64 = 1_000_000_000_000;

pub fn analyze(events: &[Event], model: &str) -> Metrics {
    let price = pricing::lookup_price(model);
    let mut metrics = Metrics {
        model_used: model.to_string(),
        tool_usage: BTreeMap::new(),
        file_usage: BTreeMap::new(),
        tool_arg_usage: BTreeMap::new(),
        tool_authority: BTreeMap::new(),
        ..Metrics::default()
    };
    let has_meta_usage = events.iter().any(|event| {
        matches!(event.role.as_str(), "session_meta" | "meta") && !event.usage.is_empty()
    });
    // rm-616: generic-lane (serde passthrough) sessions carry usage on
    // conversation lines, not synthetic meta events. Reported usage
    // exists for them too, so text estimation must stand down. Scoped to
    // the serde-origin classification (`source_tool == "generic"`, set
    // by the parser.rs `Vec<Event>` arm and the per-line fallback):
    // native families fold usage through meta events, and folding their
    // conversation lines on top would double-count.
    let has_reported_usage = has_meta_usage
        || events.iter().any(|event| {
            !matches!(event.role.as_str(), "session_meta" | "meta")
                && event.source_tool == "generic"
                && !event.usage.is_empty()
        });
    metrics.provenance.tokens = if has_reported_usage {
        "reported_by_agent"
    } else {
        "estimated_from_text"
    }
    .to_string();
    metrics.provenance.duration = "unavailable".to_string();
    metrics.provenance.tool_results = "unavailable".to_string();
    metrics.provenance.files = "unavailable".to_string();
    metrics.provenance.pricing_source = pricing::pricing_source_for(model);
    // rm-436/rm-438 pricing state consumed by the cost formula below:
    // which models produced usage blocks, and which token classes
    // already carry an upstream-recorded cost.
    let mut usage_models: BTreeSet<String> = BTreeSet::new();
    let mut usage_tiers: BTreeSet<Option<String>> = BTreeSet::new();
    let mut upstream_priced_input = 0i64;
    let mut upstream_priced_output = 0i64;
    let mut upstream_priced_cache_w = 0i64;
    let mut upstream_priced_cache_r = 0i64;
    // rm-450 first minted the workbuddy basis disclosures into the
    // line_skips family so they degraded confidence; rm-538 moves them
    // to metrics.disclosure_counters — the non-loss disclosure channel
    // — so they stay visible under the disclosure row without tanking
    // confidence for an otherwise-exact parse.
    //
    // rm-490: capture a family fallback before the loop consumes `events` —
    // meta-only sessions (codex usage journals, copilot metric records)
    // otherwise end with an empty source_tool.
    let meta_fallback_source_tool = events
        .iter()
        .find(|event| !event.source_tool.is_empty())
        .map(|event| event.source_tool.clone());
    // rm-726: implausible usage magnitudes are disclosed, never
    // silently priced. Since the rm-831 sign gate (conflict 606aefa4
    // reconciliation) negatives are refused at intake and counted on
    // the parser's census before this loop runs; the counts here are
    // the backstop for lanes that construct usage maps directly —
    // negatives that DO reach the loop are zeroed by the `.max(0)`
    // clamps below, and values at/above the documented plausibility
    // bound are priced as parsed — all counted so the disclosure
    // channel (and the implausible_magnitude anomaly) carries the
    // facts.
    let mut negative_usage_values = 0usize;
    let mut implausible_usage_values = 0usize;

    for event in events {
        // rm-726 census: negatives are zeroed below (`.max(0)`), values
        // at/above the plausibility bound are priced as parsed — both
        // counted on the disclosure channel, never silent.
        for value in event.usage.values() {
            if *value < 0 {
                negative_usage_values += 1;
            } else if *value >= IMPLAUSIBLE_TOKEN_MAGNITUDE {
                implausible_usage_values += 1;
            }
        }
        if metrics.source_tool.is_empty()
            && !event.source_tool.is_empty()
            && event.role != "meta"
            && event.role != "session_meta"
        {
            metrics.source_tool = event.source_tool.clone();
        }

        if let Some((ts, form)) = parse_ts_form(&event.timestamp) {
            metrics.timestamps.push(ts);
            if matches!(form, TimestampForm::NaiveUtc) {
                metrics.naive_utc_stamps += 1;
            }
        }

        // Usage values arrive from untrusted session logs; clamp negatives
        // (corrupt data) and accumulate with saturation so adversarial or
        // corrupt token counts can never wrap into negative totals. The
        // same clamp-plus-saturate contract governs every downstream
        // consumer of these totals (by_model/by_task_type rollups
        // included, rm-541) — a new sum of untrusted counts must use
        // saturating_add, never `+=`.
        let usage_tokens = |key: &str| -> i64 { event.usage.get(key).copied().unwrap_or(0).max(0) };

        // rm-436/rm-437: aggregate parse-time disclosure counters onto
        // the session (pi-family journals). Counters are facts about
        // the journal, never conversation content.
        for (key, value) in &event.disclosure_counters {
            *metrics.disclosure_counters.entry(key.clone()).or_insert(0) += *value as usize;
        }

        // rm-616: fold reported usage from generic-lane conversation
        // lines with the meta arm's clamp-plus-saturate contract — this
        // is the counted lane for serde-origin sessions, whose journals
        // put usage on the messages themselves. Events carrying model
        // attribution join the per-model pricing state below so a
        // switched session prices per block like any other family.
        let generic_reported_usage = !matches!(event.role.as_str(), "session_meta" | "meta")
            && event.source_tool == "generic"
            && !event.usage.is_empty();
        if generic_reported_usage {
            metrics.tokens_input = metrics
                .tokens_input
                .saturating_add(usage_tokens("input_tokens"));
            metrics.tokens_output = metrics
                .tokens_output
                .saturating_add(usage_tokens("output_tokens"));
            metrics.tokens_reasoning = metrics
                .tokens_reasoning
                .saturating_add(usage_tokens("reasoning_tokens"));
            metrics.tokens_cache_w = metrics
                .tokens_cache_w
                .saturating_add(usage_tokens("cache_creation_input_tokens"));
            metrics.tokens_cache_r = metrics
                .tokens_cache_r
                .saturating_add(usage_tokens("cache_read_input_tokens"));
            if !event.model_used.is_empty() && event.model_used != "unknown" {
                usage_models.insert(event.model_used.clone());
            }
        }
        // rm-616 disclosure: a conversation line that REPORTS usage but
        // whose family's counted lane is elsewhere (native families
        // fold usage through meta events; the generic lane folds above)
        // must not be silently swallowed — disclose it on the pass-7
        // line_skips channel instead. Review fix F3 (review f6b98bc3):
        // the counter now also fires BESIDE meta usage — a session with
        // meta totals plus stray conversation-line usage previously
        // dropped the stray silently, narrower than the batch's own
        // prevention rule. Still cannot fire for the meta arm (role
        // gate) or the generic lane (folded above).
        if !matches!(event.role.as_str(), "session_meta" | "meta")
            && !generic_reported_usage
            && !event.usage.is_empty()
        {
            *metrics
                .line_skips
                .entry("usage_present_not_counted".to_string())
                .or_insert(0) += 1;
        }

        // rm-880: wire metadata rides the same fold — last observation
        // wins so a re-configured session reports its latest wire
        // (session_configured can follow session_meta, and rate-limit
        // snapshots refresh on every token_count line).
        for (key, value) in &event.wire_metadata {
            metrics.wire_metadata.insert(key.clone(), value.clone());
        }

        match event.role.as_str() {
            "session_meta" | "meta" => {
                if !event.usage.is_empty() {
                    metrics.tokens_input = metrics
                        .tokens_input
                        .saturating_add(usage_tokens("input_tokens"));
                    metrics.tokens_output = metrics
                        .tokens_output
                        .saturating_add(usage_tokens("output_tokens"));
                    metrics.tokens_reasoning = metrics
                        .tokens_reasoning
                        .saturating_add(usage_tokens("reasoning_tokens"));
                    metrics.tokens_cache_w = metrics
                        .tokens_cache_w
                        .saturating_add(usage_tokens("cache_creation_input_tokens"));
                    metrics.tokens_cache_r = metrics
                        .tokens_cache_r
                        .saturating_add(usage_tokens("cache_read_input_tokens"));
                    if !event.model_used.is_empty() && event.model_used != "unknown" {
                        usage_models.insert(event.model_used.clone());
                    }
                    // rm-610: every usage block also records the tier its
                    // turn ran on, so the cost formula below can bill the
                    // session on the tier it actually used.
                    usage_tiers.insert(event.service_tier.clone());
                    // Workbuddy basis disclosure (upstream #310): the
                    // workbuddy lane subtracts cache_read_input_tokens from
                    // input_tokens assuming a cache-INCLUSIVE input basis.
                    // Disclose every subtraction; when the subtraction
                    // zeroes the input, flag the suspected basis mismatch
                    // instead of silently reporting 0.
                    if event.source_tool == "workbuddy"
                        && usage_tokens("cache_read_input_tokens") > 0
                    {
                        *metrics
                            .disclosure_counters
                            .entry("workbuddy_input_basis:cache_subtracted".to_string())
                            .or_insert(0) += 1;
                        if usage_tokens("input_tokens") == 0 {
                            *metrics
                                .disclosure_counters
                                .entry(
                                    "workbuddy_input_basis:zeroed_suspected_mismatch".to_string(),
                                )
                                .or_insert(0) += 1;
                        }
                    }
                }
                // rm-436 (review 06e542d5 F-C): an upstream-recorded
                // cost is honored even when the usage block carries
                // no token class at all (pi cache_warm with zeroed
                // classes) — a PRESENT cost must not be silently
                // zeroed for lacking tokens. The token exclusions
                // below add zero for an empty map.
                if let Some(cost) = event
                    .recorded_cost_usd
                    .filter(|cost| cost.is_finite() && *cost >= 0.0)
                {
                    metrics.upstream_cost_usd += cost;
                    upstream_priced_input =
                        upstream_priced_input.saturating_add(usage_tokens("input_tokens"));
                    upstream_priced_output =
                        upstream_priced_output.saturating_add(usage_tokens("output_tokens"));
                    upstream_priced_cache_w = upstream_priced_cache_w
                        .saturating_add(usage_tokens("cache_creation_input_tokens"));
                    upstream_priced_cache_r = upstream_priced_cache_r
                        .saturating_add(usage_tokens("cache_read_input_tokens"));
                }
                // rm-485: fold cost-only credit attribution with max
                // semantics — repeated snapshots never double-count.
                if event.credit_usd > metrics.credit_usd {
                    metrics.credit_usd = event.credit_usd;
                }
            }
            "user" => {
                metrics.user_messages += 1;
                if !event.content.is_empty() && !has_reported_usage {
                    metrics.tokens_input = metrics.tokens_input.saturating_add(std::cmp::max(
                        1,
                        estimate_tokens_from_text(&event.content),
                    ));
                }
            }
            "assistant" => {
                metrics.assistant_turns += 1;
                if !event.reasoning.is_empty() {
                    metrics.reasoning_blocks += 1;
                    // Characters, not bytes — the field name promises the
                    // unit (pass-6 P6-4).
                    let chars = event.reasoning.chars().count();
                    metrics.reasoning_chars += chars;
                    metrics.reasoning_lens.push(chars);
                    if event.redacted {
                        metrics.reasoning_redact += 1;
                    }
                    if !has_reported_usage {
                        metrics.tokens_output = metrics.tokens_output.saturating_add(
                            std::cmp::max(1, estimate_tokens_from_text(&event.reasoning)),
                        );
                    }
                }
                if !event.content.is_empty() && !has_reported_usage {
                    metrics.tokens_output = metrics.tokens_output.saturating_add(std::cmp::max(
                        1,
                        estimate_tokens_from_text(&event.content),
                    ));
                }
                metrics.tool_calls_total += event.tool_calls.len();
                for tool_call in &event.tool_calls {
                    let name = if tool_call.name.is_empty() {
                        "unknown"
                    } else {
                        &tool_call.name
                    };
                    *metrics.tool_usage.entry(name.to_string()).or_insert(0) += 1;
                    for arg in searchable_tool_args(&tool_call.args) {
                        *metrics.tool_arg_usage.entry(arg).or_insert(0) += 1;
                    }
                    let authority = classify_tool_authority(tool_call);
                    *metrics.tool_authority.entry(authority.clone()).or_insert(0) += 1;
                    metrics.highest_authority =
                        higher_tool_authority(&metrics.highest_authority, &authority);
                    for file in extract_tool_call_files(&tool_call.args) {
                        *metrics.file_usage.entry(file).or_insert(0) += 1;
                        metrics.provenance.files = "tool_arguments".to_string();
                    }
                }
            }
            "tool" => {
                metrics.tool_results += 1;
                let mut is_error = event.is_error;
                if !is_error && !event.content.is_empty() {
                    if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&event.content) {
                        if obj.get("success") == Some(&Value::Bool(false))
                            || obj
                                .get("error")
                                .is_some_and(|value| !matches!(value, Value::Null))
                        {
                            is_error = true;
                        }
                    }
                }
                if is_error {
                    metrics.tool_calls_fail += 1;
                } else {
                    metrics.tool_calls_ok += 1;
                }
                metrics.provenance.tool_results = if event.is_error {
                    "reported_by_agent"
                } else {
                    "reported_or_inferred"
                }
                .to_string();
            }
            _ => {}
        }
    }

    metrics.events_total = events.len();
    metrics.timestamps.sort();
    if let (Some(first), Some(last)) = (metrics.timestamps.first(), metrics.timestamps.last()) {
        metrics.session_start = first.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        metrics.session_end = last.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        metrics.duration_sec = (*last - *first).num_milliseconds() as f64 / 1000.0;
        metrics.provenance.duration = "timestamp_span".to_string();
    }
    for pair in metrics.timestamps.windows(2) {
        let gap = (pair[1] - pair[0]).num_milliseconds() as f64 / 1000.0;
        if gap > 0.0 {
            metrics.gaps_sec.push(gap);
        }
    }
    let max_ok = metrics
        .tool_calls_total
        .saturating_sub(metrics.tool_calls_fail);
    if metrics.tool_calls_ok > max_ok {
        metrics.tool_calls_ok = max_ok;
    }
    // rm-490: apply the family fallback after the loop — only when no
    // non-meta event ever named a tool (see capture above).
    if metrics.source_tool.is_empty() {
        if let Some(fallback) = meta_fallback_source_tool {
            metrics.source_tool = fallback;
        }
    }
    // rm-610: a single-model session still bills on the tier its turns
    // ran on. All usage events agreeing on one tier (the codex flex
    // case) prices the session totals at that tier's row; a tier mix
    // falls through to per-block pricing exactly like a model mix.
    let usage_blocks_multi_tier = usage_tiers.len() > 1;
    let session_service_tier = if usage_tiers.len() == 1 {
        usage_tiers.iter().next().cloned().flatten()
    } else {
        None
    };
    let catalog_price = match &session_service_tier {
        Some(tier) => pricing::lookup_price_tiered(model, Some(tier.as_str())),
        None => price.clone(),
    };
    // rm-610: record the billed tier on the metrics so every consumer
    // that re-prices from (model, tokens) — the governance audit's
    // "current rates" recompute, waste's unpriced-input cost — bills
    // the tier the session actually ran on instead of the standard row.
    metrics.service_tier = session_service_tier.clone();
    // rm-436: tokens whose cost the source journal recorded (pi
    // `type:"usage"` entries carrying `usage.cost.total`) were already
    // added to the class totals above for visibility; exclude them from
    // the catalog formula so they are priced exactly once, at the
    // recorded value, instead of being estimated at the session
    // model's rate on top of the recorded cost.
    // rm-1508: cache rates resolve with the absent-rate fallback
    // (cr = input rate, cw = input × 1.25) when the catalog row
    // publishes none — never a silent $0. Term order is unchanged so
    // published-rate sessions price byte-identically.
    let (catalog_cw, _) = catalog_price.cache_write_billed();
    let (catalog_cr, _) = catalog_price.cache_read_billed();
    let catalog_cache_w = metrics
        .tokens_cache_w
        .saturating_sub(upstream_priced_cache_w);
    let catalog_cache_r = metrics
        .tokens_cache_r
        .saturating_sub(upstream_priced_cache_r);
    let catalog_cost = round4(
        metrics.tokens_input.saturating_sub(upstream_priced_input) as f64 / 1e6
            * catalog_price.input
            + metrics.tokens_output.saturating_sub(upstream_priced_output) as f64 / 1e6
                * catalog_price.output
            + catalog_cache_w as f64 / 1e6 * catalog_cw
            + catalog_cache_r as f64 / 1e6 * catalog_cr,
    );
    // rm-1508: the fallback is a disclosure, not a guess — count it on
    // the session's non-loss disclosure channel so the governance
    // census and the data-health fold surface it.
    if pricing::cache_fallback_fired(&catalog_price, catalog_cache_w, catalog_cache_r) {
        *metrics
            .disclosure_counters
            .entry("cache_rate_fallback".to_string())
            .or_insert(0) += 1;
    }
    // rm-520: persist which token classes the journal priced upstream
    // so audits can reproduce the stored estimate on the same basis
    // (the classes above are already excluded from the catalog
    // formula; the audit needs the split to avoid re-estimating those
    // tokens at the session model's rate against a stored number that
    // already includes their recorded cost).
    metrics.upstream_priced_input = upstream_priced_input;
    metrics.upstream_priced_output = upstream_priced_output;
    metrics.upstream_priced_cache_w = upstream_priced_cache_w;
    metrics.upstream_priced_cache_r = upstream_priced_cache_r;
    // rm-438: when the usage blocks themselves disagree about which
    // model produced them (pi model_change journals), pricing the whole
    // session at the first model's rate silently misprices every
    // post-switch block. Price each block with the model that produced
    // it instead, mirroring the SQLite aggregate: per-block pricing,
    // model_used "multiple", and a disclosure instead of a silent
    // single-model estimate. Integration of run 6403d975 widened the
    // trigger to usage-vs-session disagreement: a switched session
    // whose usage blocks all name the pre-switch model (post-switch
    // turns without usage) is a mixed session too — billing its one
    // block at the post-switch session rate would misprice it the
    // same way, per the parser-agnostic rule the batch itself states
    // ("any journal family whose usage-bearing events carry model
    // attribution prices each block at its own model").
    let session_model_is_real = !matches!(model, "" | "default" | "unknown");
    let usage_blocks_multi_model = usage_models.len() > 1
        || (!usage_models.is_empty()
            && session_model_is_real
            && !usage_models.iter().any(|usage_model| usage_model == model));
    // rm-406 (dated 2026-10-07 arm): per-model attribution for the
    // multi-model session — each usage block's tokens and per-block
    // cost land on the model that produced them (the advisor-model
    // case: claude-fable advisor turns ride message.usage.iterations[]
    // with their own model while the session's headline model hides
    // them). Single-model sessions never populate this map, so their
    // serialized metrics shape is byte-identical to the pre-rm-406
    // form.
    let mut model_attribution: BTreeMap<String, ModelAttribution> = BTreeMap::new();
    // rm-610: one model on several tiers prices per usage block too —
    // same per-block arm, tier-aware rates (see the loop below).
    // rm-1052: recorded-wins must not be silent, in EITHER pricing arm.
    // The recorded guard admits a well-formed 0.0 — at the wire, a
    // gateway passthrough that never priced the call is
    // indistinguishable from a subscription flat rate — so before the
    // per-block/session-rate split, every recorded-cost block gets its
    // suppressed catalog estimate computed and disclosed:
    // `recorded_cost_suppressed_estimate_cents` carries the suppressed
    // amount (cents), `recorded_cost_zero_blocks` counts
    // gateway-unpriced blocks (recorded $0 on priced tokens). The
    // recorded number stays authoritative (the rm-419 discipline) —
    // disclosure, never repricing — mirroring rm-984's
    // recorded-vs-estimated stance for the Cursor lane onto every
    // recorded-cost arm.
    // rm-1052 rider (run 80e8beb6 cycle 1): the counter is a LOWER
    // BOUND on suppressed spend, and the docs must say so. The
    // suppressed estimate prices the same four buckets as the
    // estimate formula (input/output/cache-write/cache-read) —
    // Claude's 1h-tier cache writes (`ephemeral_1h_input_tokens`)
    // have no catalog rate yet (tier-aware pricing is rm-176), so a
    // 1h-dominated gateway block suppresses an estimate that never
    // counted its 1h bytes. The recorded number stays authoritative;
    // the gap is disclosed in README's estimates paragraph, not
    // hidden behind a tidy counter.
    for event in events.iter().filter(|event| {
        event
            .recorded_cost_usd
            .is_some_and(|cost| cost.is_finite() && cost >= 0.0)
    }) {
        let recorded = event.recorded_cost_usd.unwrap_or(0.0);
        let suppressed_price = if event.model_used.is_empty() || event.model_used == "unknown" {
            // F5 (review 37f970af): price unattributed blocks at the
            // same tier-aware session rate the session-rate arm bills
            // at (catalog_price, rm-610) — never the untiered row, so
            // the suppressed estimate cannot understate a single-tier
            // session's real catalog price.
            catalog_price.clone()
        } else {
            // rm-610: mirror the tier the turn actually ran on.
            pricing::lookup_price_tiered(&event.model_used, event.service_tier.as_deref())
        };
        // rm-1508: the counterfactual prices at the same fallback
        // rates real billing would use, but does NOT fire the
        // disclosure counter — the counter tracks billed estimates,
        // not suppressed counterfactuals.
        let (suppressed_cw, _) = suppressed_price.cache_write_billed();
        let (suppressed_cr, _) = suppressed_price.cache_read_billed();
        let suppressed_estimate =
            event.usage.get("input_tokens").copied().unwrap_or(0).max(0) as f64 / 1e6
                * suppressed_price.input
                + event
                    .usage
                    .get("output_tokens")
                    .copied()
                    .unwrap_or(0)
                    .max(0) as f64
                    / 1e6
                    * suppressed_price.output
                + event
                    .usage
                    .get("cache_creation_input_tokens")
                    .copied()
                    .unwrap_or(0)
                    .max(0) as f64
                    / 1e6
                    * suppressed_cw
                + event
                    .usage
                    .get("cache_read_input_tokens")
                    .copied()
                    .unwrap_or(0)
                    .max(0) as f64
                    / 1e6
                    * suppressed_cr;
        if suppressed_estimate - recorded > 0.005 {
            *metrics
                .disclosure_counters
                .entry("recorded_cost_suppressed_estimate_cents".to_string())
                .or_insert(0) += (round4(suppressed_estimate - recorded) * 100.0).round() as usize;
            if recorded == 0.0 {
                *metrics
                    .disclosure_counters
                    .entry("recorded_cost_zero_blocks".to_string())
                    .or_insert(0) += 1;
            }
        }
    }
    let token_priced = if usage_blocks_multi_model || usage_blocks_multi_tier {
        // Recorded costs were accumulated into upstream_cost_usd above;
        // start from them and add per-block catalog pricing for the rest.
        let mut per_block_cost = metrics.upstream_cost_usd;
        // rm-616: generic-lane usage blocks join this loop — the rule is
        // parser-agnostic ("any journal family whose usage-bearing events
        // carry model attribution prices each block at its own model"),
        // and serde-origin blocks carry usage on conversation lines.
        for event in events
            .iter()
            .filter(|event| {
                matches!(event.role.as_str(), "session_meta" | "meta")
                    || event.source_tool == "generic"
            })
            .filter(|event| !event.usage.is_empty())
        {
            let block_model = if event.model_used.is_empty() || event.model_used == "unknown" {
                model.to_string()
            } else {
                event.model_used.clone()
            };
            let entry = model_attribution.entry(block_model).or_default();
            entry.input_tokens = entry
                .input_tokens
                .saturating_add(event.usage.get("input_tokens").copied().unwrap_or(0).max(0));
            entry.output_tokens = entry.output_tokens.saturating_add(
                event
                    .usage
                    .get("output_tokens")
                    .copied()
                    .unwrap_or(0)
                    .max(0),
            );
            entry.cache_w = entry.cache_w.saturating_add(
                event
                    .usage
                    .get("cache_creation_input_tokens")
                    .copied()
                    .unwrap_or(0)
                    .max(0),
            );
            entry.cache_r = entry.cache_r.saturating_add(
                event
                    .usage
                    .get("cache_read_input_tokens")
                    .copied()
                    .unwrap_or(0)
                    .max(0),
            );
            if event
                .recorded_cost_usd
                .is_some_and(|cost| cost.is_finite() && cost >= 0.0)
            {
                // Upstream-recorded cost already sits in the seed; credit
                // the block's model with it so attribution sums to the
                // session total.
                entry.cost_usd = round4(entry.cost_usd + event.recorded_cost_usd.unwrap_or(0.0));
                continue;
            }
            let block_price = if event.model_used.is_empty() || event.model_used == "unknown" {
                price.clone()
            } else {
                // rm-610: price the tier the turn actually ran on —
                // flex-disclosing turns use the row's flex slots.
                pricing::lookup_price_tiered(&event.model_used, event.service_tier.as_deref())
            };
            // rm-1508: per-block cache rates resolve with the same
            // absent-rate fallback as the session-rate arm, and the
            // fallback here DOES fire the disclosure counter — this is
            // a billed estimate.
            let (block_cw, _) = block_price.cache_write_billed();
            let (block_cr, _) = block_price.cache_read_billed();
            let block_cache_w = event
                .usage
                .get("cache_creation_input_tokens")
                .copied()
                .unwrap_or(0)
                .max(0);
            let block_cache_r = event
                .usage
                .get("cache_read_input_tokens")
                .copied()
                .unwrap_or(0)
                .max(0);
            if pricing::cache_fallback_fired(&block_price, block_cache_w, block_cache_r) {
                *metrics
                    .disclosure_counters
                    .entry("cache_rate_fallback".to_string())
                    .or_insert(0) += 1;
            }
            let block_cost = event.usage.get("input_tokens").copied().unwrap_or(0).max(0) as f64
                / 1e6
                * block_price.input
                + event
                    .usage
                    .get("output_tokens")
                    .copied()
                    .unwrap_or(0)
                    .max(0) as f64
                    / 1e6
                    * block_price.output
                + block_cache_w as f64 / 1e6 * block_cw
                + block_cache_r as f64 / 1e6 * block_cr;
            per_block_cost += block_cost;
            entry.cost_usd = round4(entry.cost_usd + block_cost);
        }
        round4(per_block_cost)
    } else {
        round4(catalog_cost + metrics.upstream_cost_usd)
    };
    // rm-485: a session-wide credit counter is billing truth — never
    // report below it, never stack it on top of the token-priced
    // estimate. (Integration of run 32192d92: `token_priced` is the
    // rm-436/rm-438-refined token estimate above — upstream-recorded
    // costs excluded from the catalog formula and per-block pricing
    // for multi-model sessions — so the credit floor composes with,
    // rather than replaces, the landed pricing rules.)
    metrics.cost_estimated = if metrics.credit_usd > 0.0 {
        round4(token_priced.max(metrics.credit_usd))
    } else {
        round4(token_priced)
    };
    // rm-046: a class pinned at i64::MAX means accumulation saturated —
    // the total is a clamp, not an exact sum, and the provenance must
    // say so instead of claiming precise arithmetic.
    metrics.provenance.cost = if metrics.credit_usd > 0.0 && metrics.credit_usd >= token_priced {
        // rm-485: the credit counter decided the total (or matched it) —
        // say so instead of implying token arithmetic produced it.
        "calculated_from_copilot_credits"
    } else if [
        metrics.tokens_input,
        metrics.tokens_output,
        metrics.tokens_reasoning,
        metrics.tokens_cache_w,
        metrics.tokens_cache_r,
    ]
    .contains(&i64::MAX)
    {
        "calculated_from_tokens_clamped"
    } else if usage_blocks_multi_model || usage_blocks_multi_tier {
        "calculated_per_message_tokens"
    } else if metrics.upstream_cost_usd > 0.0 {
        "calculated_from_tokens_with_recorded_cost"
    } else {
        "calculated_from_tokens"
    }
    .to_string();
    if usage_blocks_multi_model {
        metrics.model_used = "multiple".to_string();
        // rm-406: keep the split that produced the per-block total on
        // the metrics — by_model reads it so the advisor/model-switch
        // spend lands on each producing model's bucket.
        metrics.model_attribution = model_attribution;
        // Per-block cost seeds from the upstream-recorded total, so the
        // mixed-model row keeps the recorded-cost hint the single-model
        // row gets (review fix F3).
        metrics.provenance.pricing_source = if metrics.upstream_cost_usd > 0.0 {
            format!("multiple models ({PER_BLOCK_PRICING_MARKER}) + recorded cost")
        } else {
            format!("multiple models ({PER_BLOCK_PRICING_MARKER})")
        }
        .to_string();
    } else if usage_blocks_multi_tier {
        // rm-610: one model, several tiers — the session priced each
        // block on the tier that produced it, and says so instead of
        // presenting a single-rate estimate.
        metrics.provenance.pricing_source = if metrics.upstream_cost_usd > 0.0 {
            "mixed service tiers (priced per usage block) + recorded cost"
        } else {
            "mixed service tiers (priced per usage block)"
        }
        .to_string();
    } else if let Some(tier) = &session_service_tier {
        // rm-610: the whole session ran one tier that is not the
        // standard row — the cost carries the tier it billed on.
        metrics.provenance.pricing_source = if metrics.upstream_cost_usd > 0.0 {
            format!(
                "{} + recorded cost; service_tier: {}",
                metrics.provenance.pricing_source, tier
            )
        } else {
            format!(
                "{}; service_tier: {}",
                metrics.provenance.pricing_source, tier
            )
        };
    } else if metrics.upstream_cost_usd > 0.0 {
        metrics.provenance.pricing_source =
            format!("{} + recorded cost", metrics.provenance.pricing_source);
    }
    // rm-726: fold the usage-magnitude census onto the non-loss
    // disclosure channel — same channel as the workbuddy/kimi facts,
    // visible in every disclosure fold (doctor, diagnostics, markdown
    // overview) without touching confidence.
    if negative_usage_values > 0 {
        *metrics
            .disclosure_counters
            .entry("magnitude_clamped:negative_to_zero".to_string())
            .or_default() += negative_usage_values;
    }
    if implausible_usage_values > 0 {
        *metrics
            .disclosure_counters
            .entry("magnitude_clamped:above_plausibility_bound".to_string())
            .or_default() += implausible_usage_values;
    }
    metrics
}

/// rm-1534: render a whole-second span as a human phrase for the
/// `future_timestamp` anomaly detail ("8000 years", "3 days") —
/// exact numbers, no precision theater beyond whole units.
fn humantime_span(seconds: i64) -> String {
    if seconds >= 365 * 24 * 3600 {
        format!("~{} years", seconds / (365 * 24 * 3600))
    } else if seconds >= 24 * 3600 {
        format!("~{} days", seconds / (24 * 3600))
    } else if seconds >= 3600 {
        format!("~{} hours", seconds / 3600)
    } else if seconds >= 60 {
        format!("~{} minutes", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

pub fn detect_anomalies(metrics: &Metrics) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();
    // rm-726: implausible token magnitudes are a first-class anomaly
    // class. The parser clamps u64 payloads above i64::MAX (and
    // saturating f64s/strings) onto the i64 bounds and the aggregator
    // zeroes negative usage — pricing then multiplies the clamped
    // values at real rates, so a hostile or corrupted journal renders
    // a $42.7T total that otherwise sails through as HEALTH 100 with
    // "No anomalies detected". The clamp census rides the
    // disclosure_counters; surface it as a high-severity anomaly so
    // health reflects the corrupted cost truth.
    let mut magnitude_facts: Vec<String> = Vec::new();
    let mut magnitude_clamped_total = 0usize;
    for (key, count) in &metrics.disclosure_counters {
        if !key.starts_with("magnitude_clamped:") {
            continue;
        }
        magnitude_clamped_total += *count;
        match key.as_str() {
            "magnitude_clamped:above_i64" => {
                magnitude_facts.push(format!("{count} value(s) clamped at the i64 bounds"))
            }
            "magnitude_clamped:negative_to_zero" => {
                magnitude_facts.push(format!("{count} negative usage value(s) zeroed"))
            }
            "magnitude_clamped:above_plausibility_bound" => magnitude_facts.push(format!(
                "{count} usage value(s) ≥ {IMPLAUSIBLE_TOKEN_MAGNITUDE} tokens"
            )),
            _ => {}
        }
    }
    if magnitude_clamped_total > 0 {
        anomalies.push(Anomaly {
            kind: "implausible_magnitude".to_string(),
            severity: "high".to_string(),
            detail: magnitude_facts.join("; "),
        });
    }
    // rm-1534: far-future timestamps are a first-class anomaly class.
    // A single corrupted clock (the year-9999 PoC) used to sail
    // through as HEALTH 100 with zero anomalies AND pollute every
    // forward --range window via the lower-bound-only admission
    // predicate (insights::session_admitted_since). Admission now
    // carries the same upper bound (now + the 24h grace — see
    // insights::FUTURE_ADMISSION_GRACE_SECS), and this class makes
    // the condition visible on the session itself wherever it is
    // still listed (--range all): same anchor the admission predicate
    // uses (session_end where known, else start — the shared
    // session_last_activity basis), so the anomaly and the exclusion
    // can never disagree about which timestamp spoke.
    let future_anchor = {
        let start = parse_ts(&metrics.session_start);
        let end = parse_ts(&metrics.session_end);
        match (start, end) {
            (Some(start), Some(end)) => Some(start.max(end)),
            (Some(start), None) => Some(start),
            (None, Some(end)) => Some(end),
            (None, None) => None,
        }
    };
    if let Some(anchor) = future_anchor {
        let bound = crate::insights::future_admission_bound(Utc::now());
        if anchor > bound {
            anomalies.push(Anomaly {
                kind: "future_timestamp".to_string(),
                severity: "high".to_string(),
                detail: format!(
                    "session activity reaches {} — {} beyond now; ranged views exclude it (bound: now + {}h)",
                    anchor.to_rfc3339(),
                    humantime_span((anchor.timestamp() - Utc::now().timestamp()).max(0)),
                    crate::insights::FUTURE_ADMISSION_GRACE_SECS / 3600
                ),
            });
        }
    }
    if !metrics.gaps_sec.is_empty() {
        let mut gaps = metrics.gaps_sec.clone();
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let long_gaps = gaps.iter().filter(|gap| **gap > 60.0).count();
        let max_gap = *gaps.last().unwrap_or(&0.0);
        let has_super_long = gaps.iter().any(|gap| *gap > 300.0);
        if has_super_long {
            anomalies.push(Anomaly {
                kind: "hanging".to_string(),
                severity: "high".to_string(),
                detail: format!("{long_gaps} gap(s) >60s, max={max_gap:.0}s"),
            });
        } else if long_gaps > 0 {
            anomalies.push(Anomaly {
                kind: "hanging".to_string(),
                severity: "medium".to_string(),
                detail: format!("{long_gaps} gap(s) >60s, max={max_gap:.0}s"),
            });
        } else if percentile(&gaps, 0.95) > 30.0 {
            anomalies.push(Anomaly {
                kind: "latency".to_string(),
                severity: "low".to_string(),
                detail: format!("p95 latency = {:.1}s", percentile(&gaps, 0.95)),
            });
        }
    }

    let total_tools = metrics.tool_calls_ok + metrics.tool_calls_fail;
    if total_tools > 0 {
        let fail_rate = metrics.tool_calls_fail as f64 / total_tools as f64;
        if fail_rate > 0.30 {
            anomalies.push(Anomaly {
                kind: "tool_failures".to_string(),
                severity: "high".to_string(),
                detail: format!(
                    "{}/{} failed ({:.0}%)",
                    metrics.tool_calls_fail,
                    total_tools,
                    fail_rate * 100.0
                ),
            });
        } else if fail_rate > 0.15 {
            anomalies.push(Anomaly {
                kind: "tool_failures".to_string(),
                severity: "medium".to_string(),
                detail: format!(
                    "{}/{} failed ({:.0}%)",
                    metrics.tool_calls_fail,
                    total_tools,
                    fail_rate * 100.0
                ),
            });
        }
    }

    if !metrics.reasoning_lens.is_empty() && metrics.reasoning_blocks > 0 {
        let avg_reason = metrics.reasoning_chars as f64 / metrics.reasoning_blocks as f64;
        if avg_reason < 200.0 {
            anomalies.push(Anomaly {
                kind: "shallow_thinking".to_string(),
                severity: "high".to_string(),
                detail: format!("avg reasoning = {avg_reason:.0} chars (very shallow)"),
            });
        } else if avg_reason < 500.0 {
            anomalies.push(Anomaly {
                kind: "shallow_thinking".to_string(),
                severity: "medium".to_string(),
                detail: format!("avg reasoning = {avg_reason:.0} chars"),
            });
        }
    }

    if metrics.reasoning_redact > 0 {
        anomalies.push(Anomaly {
            kind: "redaction".to_string(),
            severity: "medium".to_string(),
            detail: format!("{} block(s) redacted", metrics.reasoning_redact),
        });
    }

    if metrics.tool_calls_total == 0 && metrics.assistant_turns > 2 {
        anomalies.push(Anomaly {
            kind: "no_tools".to_string(),
            severity: "low".to_string(),
            detail: "no tool calls — chat-only session".to_string(),
        });
    }

    anomalies
}

pub fn health_score(anomalies: &[Anomaly]) -> i32 {
    let mut score = 100;
    for anomaly in anomalies {
        match anomaly.severity.as_str() {
            "high" => score -= 30,
            "medium" => score -= 12,
            "low" => score -= 4,
            _ => {}
        }
    }
    score.clamp(0, 100)
}

fn validate_tool_warnings(events: &[Event]) -> Vec<ToolWarning> {
    let mut empty_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut invalid_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut last_key = String::new();
    let mut last_tool = String::new();
    let mut consecutive = 0;
    let mut calls_by_id = BTreeMap::new();
    let mut redundant: BTreeMap<String, (String, usize, BTreeSet<usize>)> = BTreeMap::new();
    let mut turn = 0;
    for event in events {
        if event.role == "user" {
            if consecutive >= 4 {
                warnings.push(tool_warning(&last_tool, "dead_loop", consecutive, "high"));
            }
            last_key.clear();
            last_tool.clear();
            consecutive = 0;
            turn += 1;
        }
        if event.role != "assistant" || event.tool_calls.is_empty() {
            continue;
        }
        for tool_call in &event.tool_calls {
            let name = tool_call.name.trim();
            let args = tool_call.args.trim();
            if !tool_call.id.is_empty() {
                calls_by_id.insert(tool_call.id.clone(), name.to_string());
            }
            let normalized = normalize_tool_args(args);
            let key = format!("{name}\0{normalized}");
            if key == last_key {
                consecutive += 1;
            } else {
                if consecutive >= 4 {
                    warnings.push(tool_warning(&last_tool, "dead_loop", consecutive, "high"));
                }
                last_key = key.clone();
                last_tool = name.to_string();
                consecutive = 1;
            }
            if !normalized.is_empty() {
                let entry = redundant
                    .entry(key)
                    .or_insert_with(|| (name.to_string(), 0, BTreeSet::new()));
                entry.1 += 1;
                entry.2.insert(turn);
            }
            if (args.is_empty() || args == "{}") && tool_requires_args(name) {
                *empty_counts.entry(name.to_string()).or_insert(0) += 1;
                continue;
            }
            if looks_like_structured_args(args) && serde_json::from_str::<Value>(args).is_err() {
                *invalid_counts.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }
    if consecutive >= 4 {
        warnings.push(tool_warning(&last_tool, "dead_loop", consecutive, "high"));
    }
    for (name, count) in empty_counts {
        warnings.push(tool_warning(&name, "empty_args", count, "medium"));
    }
    for (name, count) in invalid_counts {
        warnings.push(tool_warning(&name, "invalid_args", count, "medium"));
    }
    let mut last_failed = String::new();
    let mut failure_chain = 0;
    for event in events {
        if event.role == "user" {
            if failure_chain >= 3 {
                warnings.push(tool_warning(
                    &last_failed,
                    "fail_retry_chain",
                    failure_chain,
                    "high",
                ));
            }
            last_failed.clear();
            failure_chain = 0;
        } else if event.role == "tool" && event.is_error {
            let name = calls_by_id
                .get(&event.tool_call_id)
                .cloned()
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            if name == last_failed {
                failure_chain += 1;
            } else {
                if failure_chain >= 3 {
                    warnings.push(tool_warning(
                        &last_failed,
                        "fail_retry_chain",
                        failure_chain,
                        "high",
                    ));
                }
                last_failed = name;
                failure_chain = 1;
            }
        }
    }
    if failure_chain >= 3 {
        warnings.push(tool_warning(
            &last_failed,
            "fail_retry_chain",
            failure_chain,
            "high",
        ));
    }
    for (_, (name, count, turns)) in redundant {
        if count >= 4 && turns.len() >= 3 {
            warnings.push(tool_warning(&name, "redundant", count, "low"));
        }
    }
    warnings.sort_by(|left, right| {
        warning_rank(&right.severity)
            .cmp(&warning_rank(&left.severity))
            .then_with(|| left.tool_name.cmp(&right.tool_name))
            .then_with(|| left.pattern.cmp(&right.pattern))
    });
    warnings
}

fn tool_warning(name: &str, pattern: &str, count: usize, severity: &str) -> ToolWarning {
    let detail = match pattern {
        "empty_args" => format!("Tool '{name}' had {count} call(s) with empty arguments"),
        "invalid_args" => format!("Tool '{name}' had {count} call(s) with malformed arguments"),
        "dead_loop" => format!("Tool '{name}' repeated the same call {count} times"),
        "fail_retry_chain" => format!("Tool '{name}' failed {count} consecutive times"),
        "redundant" => format!("Tool '{name}' repeated the same call {count} times across turns"),
        _ => String::new(),
    };
    ToolWarning {
        tool_name: name.to_string(),
        pattern: pattern.to_string(),
        count,
        detail,
        severity: severity.to_string(),
    }
}

fn normalize_tool_args(args: &str) -> String {
    let args = args.trim();
    serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|value| serde_json::to_string(&value).ok())
        .unwrap_or_else(|| args.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn warning_rank(severity: &str) -> u8 {
    match severity {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn tool_requires_args(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() {
        return false;
    }
    [
        "apply_patch",
        "bash",
        "click",
        "edit",
        "exec_command",
        "fetch",
        "find",
        "grep",
        "image",
        "open",
        "patch",
        "read",
        "replace",
        "rg",
        "run_command",
        "scrape",
        "search",
        "shell",
        "terminal",
        "view",
        "web",
        "write",
    ]
    .iter()
    .any(|token| name.contains(token))
}

fn looks_like_structured_args(args: &str) -> bool {
    let args = args.trim();
    args.starts_with('{') || args.starts_with('[')
}

pub fn compute_overview(sessions: &[Session]) -> Overview {
    compute_overview_iter(sessions.iter())
}

pub fn compute_overview_iter<'a>(sessions: impl Iterator<Item = &'a Session>) -> Overview {
    let mut overview = Overview::default();
    for session in sessions {
        overview.total_sessions += 1;
        // rm-1500: skip-while-counting at every cost fold below — one
        // poisoned session must never read as clean zero-spend (and never
        // silently propagate through the attribution split either).
        let session_cost = session.metrics.cost_estimated;
        let cost_finite = session_cost.is_finite();
        if !cost_finite {
            overview.non_finite_costs += 1;
        }
        if cost_finite {
            overview.total_cost += session_cost;
        }
        if session.health >= 80 {
            overview.healthy += 1;
        } else if session.health >= 50 {
            overview.warning += 1;
        } else {
            overview.critical += 1;
        }
        let agent = if session.metrics.agent_name.is_empty() {
            session.metrics.source_tool.clone()
        } else {
            session.metrics.agent_name.clone()
        };
        // rm-991 (cycle 2, run b5affead — the landed lane this
        // integration kept over run b5d391ec's stranded parallel
        // implementation of the same unit): the opencode `session.agent`
        // column names WHO ran the session (live census 2026-10-12:
        // 302/310 rows). Keying on it splits the one `opencode_db`
        // bucket every opencode session used to collapse into; every
        // other lane never sets `agent_name`, so their buckets are
        // byte-identical to before (same source_tool fallback).
        let agent_entry = overview.by_agent.entry(agent).or_default();
        agent_entry.sessions += 1;
        // rm-1500: the agent rollup skips poisoned costs like every
        // other fold (counted in overview.non_finite_costs).
        if cost_finite {
            agent_entry.cost += session_cost;
        }

        let model = if session.metrics.model_used.is_empty() {
            "unknown".to_string()
        } else {
            session.metrics.model_used.clone()
        };
        let model_entry = overview.by_model.entry(model).or_default();
        model_entry.sessions += 1;
        // Review fix F1 (run 14954d7a independent_review, blocking):
        // when the session carries a per-model attribution split, its
        // full spend is distributed among the attributed buckets below,
        // so the headline bucket (multi-model sessions collapse under
        // "multiple") keeps the session count but contributes no cost —
        // sum(by_model[].cost) stays exactly the true total, which the
        // pre-batch rollup guaranteed and agenttrace.overview.v1 machine
        // consumers sum against. Unattributed sessions (single- or
        // multi-model) keep the one-bucket full-cost behavior.
        if session.metrics.model_attribution.is_empty() && cost_finite {
            model_entry.cost += session_cost;
        }

        // rm-949 (cycle 3): the per-model attribution costs are priced
        // independently of the session's billed `cost_estimated` (per-block
        // catalog pricing with per-add round4, recorded-cost seeds, credit
        // floors), so raw sums drifted away from the billed total and broke
        // sum(by_model.cost) == total_cost for machine consumers summing
        // agenttrace.overview.v1 buckets (assess PoC: billed 0.0002 vs
        // attribution Σ 0.0004). Distribute the billed total across the
        // buckets proportionally: the attribution proportions survive,
        // and the sum is the billed total exactly, by construction.
        // The apportionment runs in integer 4-dp units via the
        // largest-remainder method, so every bucket is exact at the
        // precision every lane renders (json/csv/markdown all print
        // costs at 4 dp) — the invariant holds in raw f64 AND as
        // rendered, which the naive proportional division broke
        // (0.0002 total vs rendered buckets summing 0.0003).
        //
        // rm-406 (dated 2026-10-07 arm): the session-level by_model
        // bucket above is the headline split (multi-model sessions
        // collapse under "multiple"); additionally credit each
        // producing model with its own share of the session's spend so
        // an advisor turn (or a mid-session model switch) lands on the
        // model that actually burned the tokens instead of vanishing.
        // `sessions` in an attribution bucket counts sessions that USED
        // the model — a multi-model session therefore appears in more
        // than one bucket (by_task_type, by contrast, assigns each
        // session exactly one bucket and never overlaps).
        let attributed_total: f64 = session
            .metrics
            .model_attribution
            .values()
            .map(|attribution| attribution.cost_usd)
            .sum();
        if attributed_total > 0.0 && cost_finite {
            let scale = 1.0e4_f64;
            let total_units = (session.metrics.cost_estimated * scale).round() as i64;
            let attribution_items: Vec<_> = session.metrics.model_attribution.iter().collect();
            let mut shares: Vec<i64> = Vec::with_capacity(attribution_items.len());
            let mut remainders: Vec<(usize, f64)> = Vec::with_capacity(attribution_items.len());
            let mut allocated: i64 = 0;
            for (idx, (_, attribution)) in attribution_items.iter().enumerate() {
                let exact = attribution.cost_usd / attributed_total * total_units as f64;
                let floor = exact.floor() as i64;
                shares.push(floor);
                allocated += floor;
                remainders.push((idx, exact - floor as f64));
            }
            let mut leftover = total_units - allocated;
            // Largest remainder first; ties break on attribution weight
            // (bigger share wins the extra unit) then model name — both
            // independent of the map's iteration order (HashMap order is
            // per-process), so the split is deterministic across runs.
            remainders.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(
                        attribution_items[b.0]
                            .1
                            .cost_usd
                            .partial_cmp(&attribution_items[a.0].1.cost_usd)
                            .unwrap_or(std::cmp::Ordering::Equal),
                    )
                    .then(attribution_items[a.0].0.cmp(attribution_items[b.0].0))
            });
            let mut ri = 0;
            while leftover > 0 && !remainders.is_empty() {
                let (idx, _) = remainders[ri % remainders.len()];
                shares[idx] += 1;
                leftover -= 1;
                ri += 1;
            }
            for ((attributed_model, _), units) in attribution_items.iter().zip(&shares) {
                let entry = overview
                    .by_model
                    .entry((*attributed_model).clone())
                    .or_default();
                entry.sessions += 1;
                entry.cost += *units as f64 / scale;
            }
        } else {
            for attributed_model in session.metrics.model_attribution.keys() {
                let entry = overview
                    .by_model
                    .entry(attributed_model.clone())
                    .or_default();
                entry.sessions += 1;
            }
        }

        // rm-245: attribution dimensions. The provider comes from the
        // same catalog row that prices the model, so vendor claims and
        // price claims can never disagree; unresolvable models go to an
        // explicit "unknown" bucket rather than vanishing.
        let provider = pricing::provider_for(&session.metrics.model_used)
            .unwrap_or_else(|| "unknown".to_string());
        let provider_entry = overview.by_provider.entry(provider).or_default();
        provider_entry.sessions += 1;
        if cost_finite {
            provider_entry.cost += session_cost;
        }

        // rm-726: surface the magnitude-clamp census beside the total.
        overview.clamped_values += session
            .metrics
            .disclosure_counters
            .iter()
            .filter(|(key, _)| key.starts_with("magnitude_clamped:"))
            .map(|(_, count)| count)
            .sum::<usize>();

        let task_type_entry = overview
            .by_task_type
            .entry(infer_task_type(session).to_string())
            .or_default();
        task_type_entry.sessions += 1;
        if cost_finite {
            task_type_entry.cost += session_cost;
        }
        // rm-541: token sums saturate, never `+=`. These per-session totals
        // arrive from untrusted logs (a hostile journal pins i64::MAX), so
        // a bare `+=` aborted debug builds with `attempt to add with
        // overflow` and wrapped negative in release — where the renderer's
        // `.max(0)` mask printed a 0-token bucket beside a summary claiming
        // 9,223,372,036,854,775,807 total tokens. Same treatment as every
        // other token total (the governance.rs add_context_session comment
        // is the precedent).
        // Independently re-derived by run 933058 (campaign-local
        // rm-529, retired at integration as this row's title-twin;
        // its distinct deliverable, the
        // tests/report_numeric_truthfulness.rs suite, pins the
        // committed adversarial corpus).
        task_type_entry.tokens_input = task_type_entry
            .tokens_input
            .saturating_add(session.metrics.tokens_input.max(0));
        task_type_entry.tokens_output = task_type_entry
            .tokens_output
            .saturating_add(session.metrics.tokens_output.max(0));

        let project_entry = overview
            .by_project
            .entry(project_name(session))
            .or_default();
        project_entry.sessions += 1;
        if cost_finite {
            project_entry.cost += session_cost;
        }

        // rm-585 (spend-by-branch, first cut): branch values come only
        // from the claude-code lane's gitBranch envelope. Missing,
        // detached-HEAD ("HEAD") and no-branch-concept lanes all roll
        // up under one stable "unknown" bucket, so every session lands
        // in exactly one bucket and the sum over buckets always equals
        // total_sessions. No token totals ride this dimension
        // (GroupOverview, like by_provider), so nothing here needs the
        // clamp+saturate contract the token-bearing dimensions obey.
        let branch_bucket = if session.branch.is_empty() || session.branch == "HEAD" {
            "unknown".to_string()
        } else {
            session.branch.clone()
        };
        let branch_entry = overview.by_branch.entry(branch_bucket).or_default();
        branch_entry.sessions += 1;
        if cost_finite {
            branch_entry.cost += session_cost;
        }

        // rm-1405 reap (run 4f2af62e5b7e, "Claude org-dimension
        // truth"): org attribution from the claude-code
        // `$.session.identity.orgIdentifier` dimension. Zero-noise:
        // sessions without a carried org are left OUT of the facet
        // entirely — no "unknown" bucket, deliberately unlike
        // by_branch, because a missing identity is the normal state
        // of pre-2.1.29x journals and single-account hosts, not a
        // disclosure-worthy gap; the disclosure counter
        // `claude_org_identity:*` (journal basis vs config-dir
        // fallback) carries that signal instead. Sessions and cost
        // only, like by_branch — including the rm-1500 skip-while-counting
        // discipline: a non-finite session cost is counted in
        // non_finite_costs and skipped, never folded into the org bucket
        // (rm-1549).
        if !session.org.is_empty() {
            let org_entry = overview.by_org.entry(session.org.clone()).or_default();
            org_entry.sessions += 1;
            if cost_finite {
                org_entry.cost += session_cost;
            }
        }

        for anomaly in &session.anomalies {
            overview.anomalies_top.push(AnomalyTop {
                session: session.name.clone(),
                kind: anomaly.kind.clone(),
                age: "now".to_string(),
                severity: anomaly.severity.clone(),
            });
        }
    }
    overview.anomalies_top.sort_by(|a, b| {
        severity_rank(&a.severity)
            .cmp(&severity_rank(&b.severity))
            .then_with(|| a.session.cmp(&b.session))
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.age.cmp(&b.age))
    });
    overview
}

/// Parse-instant comparison for `session_start` strings (rm-785).
///
/// Orders by the parsed instant ([`parse_ts`]), not the raw string, so any
/// two RFC 3339 spellings of the same timeline — `…Z` and `…+09:00` — order
/// correctly. A session with a missing or unparseable `session_start` counts
/// as the OLDEST (it sorts last in newest-first views); two such sessions
/// compare `Equal` so caller tie-breaks (name, path, mtime) decide. This is
/// the one recency basis shared by `canonical_sessions`, the CLI
/// `--sort recent` arm and `--latest` selection, and the TUI
/// `SortKey::Recent` comparator.
pub fn session_start_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (parse_ts(a), parse_ts(b)) {
        (Some(a_ts), Some(b_ts)) => a_ts.cmp(&b_ts),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

pub fn canonical_sessions(sessions: &[Session]) -> Vec<Session> {
    let mut out = sessions.to_vec();
    out.sort_by(|a, b| {
        session_start_cmp(&a.metrics.session_start, &b.metrics.session_start)
            .reverse()
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.path.cmp(&b.path))
    });
    out
}

// ---- rm-201 (run 12a73275 cycle 4): the --quota plan-window view -------

/// One rolling plan window of the `--quota` view: a trailing span that
/// ends at the corpus cutoff, with the TTL-correct spend and the
/// sessions that contributed to it.
struct QuotaWindow {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    spend_usd: f64,
    sessions_contributing: usize,
}

/// The corpus cutoff: the newest instant any loaded session can place
/// a cost on — the last event timestamp when the parse carried one
/// (fresh parses), else the parsed `session_end` (warm-cache rows,
/// which lose the event-timestamp vector to its serde skip).
///
/// rm-1402 (run f2fccaa9 cycle 3, assess 53af6c9e F3): the cutoff is
/// the newest PLAUSIBLE instant, never an unbounded max(). A
/// future-dated last event or `session_end` (hostile meter, clock
/// skew, one fabricated journal line) used to anchor every rolling
/// window in the future, pushing all real events outside the
/// trailing spans — the assess PoC corpus billed $0.0045 against an
/// honest $1.52, hiding 99.7% of the spend. Instants after `now`
/// are discarded at a disclosed clock-skew guard: the anchor falls
/// back to the newest at-or-before-now instant the session still
/// carries (an earlier event timestamp, or a warm-cache
/// `session_start`), and sessions with no plausible instant are
/// counted as discarded, never silently dropped. Returns
/// `(cutoff, future_discarded_sessions)`.
fn quota_cutoff(sessions: &[Session]) -> (Option<DateTime<Utc>>, usize) {
    let now = Utc::now();
    let mut cutoff: Option<DateTime<Utc>> = None;
    let mut future_discarded = 0usize;
    for session in sessions {
        let anchor = match session
            .metrics
            .timestamps
            .last()
            .copied()
            .or_else(|| parse_ts(&session.metrics.session_end))
        {
            Some(ts) => ts,
            None => continue,
        };
        if anchor <= now {
            cutoff = Some(cutoff.map_or(anchor, |c: DateTime<Utc>| c.max(anchor)));
            continue;
        }
        future_discarded += 1;
        let rescued = session
            .metrics
            .timestamps
            .iter()
            .rev()
            .find(|ts| **ts <= now)
            .copied()
            .or_else(|| parse_ts(&session.metrics.session_start).filter(|ts| *ts <= now));
        if let Some(ts) = rescued {
            cutoff = Some(cutoff.map_or(ts, |c: DateTime<Utc>| c.max(ts)));
        }
    }
    (cutoff, future_discarded)
}

/// The share of a session's cost that sits inside `[start, end]`.
/// Fresh parses carry the full event-timestamp vector, so the share is
/// the fraction of events inside the window; warm-cache rows keep only
/// `session_start`/`session_end`, so the share is the fraction of the
/// session's span inside the window (an instant session counts fully
/// when its instant is inside). `None` when the row carries no
/// placeable time at all — the caller counts it as unplaced and bills
/// it at zero rather than guessing.
fn quota_cost_share(metrics: &Metrics, start: &DateTime<Utc>, end: &DateTime<Utc>) -> Option<f64> {
    if !metrics.timestamps.is_empty() {
        let total = metrics.timestamps.len();
        let inside = metrics
            .timestamps
            .iter()
            .filter(|ts| **ts >= *start && **ts <= *end)
            .count();
        return Some(inside as f64 / total as f64);
    }
    let (Some(first), Some(last)) = (
        parse_ts(&metrics.session_start),
        parse_ts(&metrics.session_end),
    ) else {
        return None;
    };
    let span_ms = (last - first).num_milliseconds();
    if span_ms <= 0 {
        // Instant (or inverted) span: the whole cost rides the single
        // instant the row does carry.
        return Some(if first >= *start && first <= *end {
            1.0
        } else {
            0.0
        });
    }
    let overlap_ms = (last.min(*end) - first.max(*start)).num_milliseconds();
    Some(overlap_ms.clamp(0, span_ms) as f64 / span_ms as f64)
}

/// Fold one rolling window: every session contributes its cost times
/// its [`quota_cost_share`] inside the window, so spend is TTL-correct
/// — a session's cost leaves the window as its events age out, which
/// is the rolling analog of a plan-window reset.
fn quota_window(
    sessions: &[Session],
    cutoff: DateTime<Utc>,
    span: chrono::Duration,
) -> QuotaWindow {
    let start = cutoff - span;
    let mut spend_usd = 0.0;
    let mut sessions_contributing = 0;
    for session in sessions {
        if let Some(share) = quota_cost_share(&session.metrics, &start, &cutoff) {
            if share > 0.0 {
                sessions_contributing += 1;
            }
            // rm-1500: share distribution skips poisoned costs; the
            // session is already counted in overview.non_finite_costs.
            if session.metrics.cost_estimated.is_finite() {
                spend_usd += session.metrics.cost_estimated * share;
            }
        }
    }
    QuotaWindow {
        start,
        end: cutoff,
        spend_usd: round4(spend_usd),
        sessions_contributing,
    }
}

/// Census of the attribution bases across the corpus — which sessions
/// the event-timestamp share covers, which the warm-cache span share,
/// and which carry no placeable time at all (counted, disclosed, billed
/// at zero).
fn quota_attribution_census(sessions: &[Session]) -> (usize, usize, usize) {
    let mut event_basis = 0;
    let mut span_basis = 0;
    let mut unplaced = 0;
    for session in sessions {
        if !session.metrics.timestamps.is_empty() {
            event_basis += 1;
        } else if parse_ts(&session.metrics.session_start).is_some()
            && parse_ts(&session.metrics.session_end).is_some()
        {
            span_basis += 1;
        } else {
            unplaced += 1;
        }
    }
    (event_basis, span_basis, unplaced)
}

fn quota_ts(ts: &DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn quota_window_json(window: &QuotaWindow, span: chrono::Duration) -> serde_json::Value {
    serde_json::json!({
        "window_start_utc": quota_ts(&window.start),
        "window_end_utc": quota_ts(&window.end),
        "spend_usd": window.spend_usd,
        "sessions_contributing": window.sessions_contributing,
        // Rolling analog of a plan reset: the instant after which every
        // event currently counted has aged out of the trailing window.
        "resets_at_utc": quota_ts(&(window.end + span)),
    })
}

/// rm-201 (run 12a73275 cycle 4): the `--quota` report value —
/// plan-window truth from the local corpus alone. The two plan
/// windows (5h, 7d) are ROLLING spans that end at the corpus cutoff,
/// because the provider's own anchors (a first-use 5h start, a weekly
/// 7am-UTC Monday reset) are not knowable without credentials, which
/// agenttrace never reads; the JSON says so in `corpus.window_anchor`
/// instead of inventing them. Spend is TTL-correct per session (see
/// [`quota_cost_share`]), limits are disclosed-unknown (`limits` is
/// null with its basis — a config-provided limit is a later extension
/// arm), and the cutoff itself is disclosed so a stale corpus cannot
/// masquerade as a live quota. Schema `agenttrace.quota/1` is stable:
/// the README "Quota view" section documents every key.
pub fn quota_report(sessions: &[Session]) -> serde_json::Value {
    let (cutoff, future_discarded) = quota_cutoff(sessions);
    let (event_basis, span_basis, unplaced) = quota_attribution_census(sessions);
    let (five_hour, weekly) = match cutoff {
        Some(cutoff) => (
            quota_window(sessions, cutoff, chrono::Duration::hours(5)),
            quota_window(sessions, cutoff, chrono::Duration::days(7)),
        ),
        None => {
            return serde_json::json!({
                "schema": "agenttrace.quota/1",
                "five_hour": serde_json::Value::Null,
                "weekly": serde_json::Value::Null,
                "limits": {
                    "five_hour_usd": null,
                    "weekly_usd": null,
                    "basis": "unknown: agenttrace reads no credentials and no plan data; config-provided limits are a later extension",
                },
                "corpus": {
                    "sessions": sessions.len(),
                    "cutoff_utc": serde_json::Value::Null,
                    "cutoff_basis": "newest event timestamp across the loaded corpus (cached sessions: newest session_end); null when no session carries placeable time; rm-1402: timestamps after now are discarded at a clock-skew guard, so the cutoff is always the newest at-or-before-now instant",
                    "future_discarded_sessions": future_discarded,
                    "window_anchor": "rolling: both windows end at the corpus cutoff; provider plan anchors (first-use 5h start, weekly 7am UTC Monday) are not knowable without credentials",
                    "attribution": {
                        "event_timestamps": event_basis,
                        "session_span": span_basis,
                        "unplaced": unplaced,
                    },
                    "attribution_basis": "a session's cost enters a window pro rata: the share of its event timestamps inside the window (fresh parses), or the share of its start-end span inside the window (warm-cache rows, which do not carry event timestamps); unplaced rows are counted and disclosed but billed at zero",
                },
            })
        }
    };
    serde_json::json!({
        "schema": "agenttrace.quota/1",
        "five_hour": quota_window_json(&five_hour, chrono::Duration::hours(5)),
        "weekly": quota_window_json(&weekly, chrono::Duration::days(7)),
        "limits": {
            "five_hour_usd": null,
            "weekly_usd": null,
            "basis": "unknown: agenttrace reads no credentials and no plan data; config-provided limits are a later extension",
        },
        "corpus": {
            "sessions": sessions.len(),
            "cutoff_utc": quota_ts(&five_hour.end),
            "cutoff_basis": "newest event timestamp across the loaded corpus (cached sessions: newest session_end); rm-1402: timestamps after now are discarded at a clock-skew guard, so the cutoff is always the newest at-or-before-now instant",
            "future_discarded_sessions": future_discarded,
            "window_anchor": "rolling: both windows end at the corpus cutoff; provider plan anchors (first-use 5h start, weekly 7am UTC Monday) are not knowable without credentials",
            "attribution": {
                "event_timestamps": event_basis,
                "session_span": span_basis,
                "unplaced": unplaced,
            },
            "attribution_basis": "a session's cost enters a window pro rata: the share of its event timestamps inside the window (fresh parses), or the share of its start-end span inside the window (warm-cache rows, which do not carry event timestamps); unplaced rows are counted and disclosed but billed at zero",
        },
    })
}

/// Render the `--quota` view (rm-201). `-f json` emits the stable
/// `agenttrace.quota/1` schema; the default text lane emits the same
/// numbers as a compact human summary. Any other format is rejected
/// loudly — the quota view has no csv/markdown/html rendering, and a
/// silent text fallback would look like a supported one.
pub fn render_quota_view(format: &str, sessions: &[Session]) -> anyhow::Result<String> {
    let report = quota_report(sessions);
    match format {
        "json" => Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&report)?
        )),
        "text" => Ok(render_quota_text(&report)),
        other => anyhow::bail!(
            "--quota supports -f text and -f json only (got {other:?}); the quota view has no {other} rendering"
        ),
    }
}

fn render_quota_text(report: &serde_json::Value) -> String {
    let corpus = &report["corpus"];
    let mut out =
        String::from("Quota view — rolling plan windows from the local corpus (rm-201)\n");
    match corpus["cutoff_utc"].as_str() {
        Some(cutoff) => {
            out.push_str(&format!(
                "  corpus cutoff  {cutoff}  ({} session(s) loaded)\n",
                corpus["sessions"]
            ));
            for (label, key) in [("5h window", "five_hour"), ("7d window", "weekly")] {
                match report[key].as_object() {
                    Some(window) => out.push_str(&format!(
                        "  {label}      {} → {}  spend ${:.4} across {} session(s); ages out fully {}\n",
                        window["window_start_utc"].as_str().unwrap_or("?"),
                        window["window_end_utc"].as_str().unwrap_or("?"),
                        window["spend_usd"].as_f64().unwrap_or(0.0),
                        window["sessions_contributing"],
                        window["resets_at_utc"].as_str().unwrap_or("?"),
                    )),
                    None => out.push_str(&format!("  {label}      undefined (no cutoff)\n")),
                }
            }
        }
        None => out.push_str(&format!(
            "  no session carries placeable time — windows and spend are undefined ({} session(s) loaded)\n",
            corpus["sessions"]
        )),
    }
    let attribution = &corpus["attribution"];
    out.push_str(&format!(
        "  attribution     event-timestamp share ({}), start/end span share ({} warm-cache), unplaced ({})\n",
        attribution["event_timestamps"], attribution["session_span"], attribution["unplaced"],
    ));
    // rm-1402: the clock-skew guard must be visible in the text lane
    // too — a discarded future timestamp is a disclosure, not a
    // silent edit of the corpus.
    if let Some(discarded) = corpus["future_discarded_sessions"].as_u64() {
        if discarded > 0 {
            out.push_str(&format!(
                "  clock skew       {discarded} session(s) carry timestamps after now — discarded; the cutoff is the newest at-or-before-now instant\n"
            ));
        }
    }
    out.push_str(
        "  limits          unknown — agenttrace reads no credentials and knows no plan limits\n",
    );
    out
}

pub fn evaluate_overview_gate(
    overview: &Overview,
    sessions: &[Session],
    fail_under_health: i32,
    fail_on_critical: bool,
    max_tool_fail_rate: Option<f64>,
) -> Vec<String> {
    let mut failures = Vec::new();
    let avg_health = average_health(sessions);
    let tool_fail_rate = tool_fail_rate(sessions);
    if fail_under_health > 0 && avg_health < fail_under_health as f64 {
        failures.push(format!(
            "average health {:.1} is below {}",
            avg_health, fail_under_health
        ));
    }
    if fail_on_critical && overview.critical > 0 {
        failures.push(format!("{} critical sessions found", overview.critical));
    }
    if let Some(max_tool_fail_rate) = max_tool_fail_rate {
        if tool_fail_rate > max_tool_fail_rate {
            failures.push(format!(
                "tool failure rate {:.1}% exceeds {:.1}%",
                tool_fail_rate, max_tool_fail_rate
            ));
        }
    }
    failures
}

pub fn average_health<T: std::borrow::Borrow<Session>>(sessions: &[T]) -> f64 {
    if sessions.is_empty() {
        return 0.0;
    }
    let total: i32 = sessions.iter().map(|session| session.borrow().health).sum();
    total as f64 / sessions.len() as f64
}

pub fn tool_fail_rate(sessions: &[Session]) -> f64 {
    let ok: usize = sessions
        .iter()
        .map(|session| session.metrics.tool_calls_ok)
        .sum();
    let fail: usize = sessions
        .iter()
        .map(|session| session.metrics.tool_calls_fail)
        .sum();
    let total = ok + fail;
    if total == 0 {
        0.0
    } else {
        fail as f64 / total as f64 * 100.0
    }
}

pub fn total_tokens(session: &Session) -> i64 {
    session
        .metrics
        .tokens_input
        .saturating_add(session.metrics.tokens_output)
        .saturating_add(session.metrics.tokens_cache_w)
        .saturating_add(session.metrics.tokens_cache_r)
}

pub fn format_tokens(value: i64) -> String {
    let (divisor, suffix) = match value.unsigned_abs() {
        999_950_000_000.. => (1_000_000_000_000.0, "T"),
        999_950_000.. => (1_000_000_000.0, "B"),
        999_950.. => (1_000_000.0, "M"),
        1_000.. => (1_000.0, "K"),
        _ => return value.to_string(),
    };
    format!("{:.1}{suffix}", value as f64 / divisor)
}

pub fn format_count(value: usize) -> String {
    format_tokens(value as i64)
}

pub fn format_cost(value: f64) -> String {
    if !value.is_finite() {
        // rm-1500: non-finite is a null marker on human lanes — never
        // "$0.0000", which reads as a clean zero-spend session. The JSON
        // lanes serialize the raw f64 (null) per the insights contract.
        return "—".to_string();
    }
    let value = value + 0.0;
    let abs = value.abs();
    if abs >= 999_950_000_000.0 {
        format!("${:.1}T", value / 1_000_000_000_000.0)
    } else if abs >= 999_950_000.0 {
        format!("${:.1}B", value / 1_000_000_000.0)
    } else if abs >= 999_950.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if abs >= 999.95 {
        format!("${:.1}K", value / 1_000.0)
    } else if abs >= 10.0 {
        format!("${value:.2}")
    } else {
        format!("${value:.4}")
    }
}

fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}

/// rm-949 rounding discipline: `round4` defines the 4-decimal quantum every
/// cost lane reports through - half-away-from-zero on the scaled integer,
/// with a 1e-9 epsilon that absorbs f64 representation noise so values
/// already at the quantum never re-round. The by_model attribution
/// apportionment (build_overview's rm-949 block) works in integer units of
/// this quantum via largest-remainder, so every bucket is exact at the
/// precision every lane renders and sum(by_model.cost) equals the billed
/// total by construction, not by luck.
pub fn round4(value: f64) -> f64 {
    let scaled = value * 10000.0;
    let rounded = if scaled.fract().abs() >= 0.5 - 1e-9 {
        scaled.trunc() + scaled.signum()
    } else {
        scaled.trunc()
    };
    rounded / 10000.0
}

pub(crate) use pricing::token_cost;

pub fn fmt_duration(seconds: f64) -> String {
    // Unit-carry discipline (rm-953): a rendered value must never sit at the
    // next unit's boundary — no "60s", no "60.0m". Promote to the next unit
    // when the rounded render would reach it (59.5s renders "1.0m", 3597s
    // renders "0h 59m" rather than "60.0m").
    let seconds = if seconds < 0.0 { 0.0 } else { seconds };
    if seconds < 59.5 {
        format!("{seconds:.0}s")
    } else if seconds < 3597.0 {
        format!("{:.1}m", seconds / 60.0)
    } else {
        let hours = (seconds / 3600.0) as i64;
        let minutes = ((seconds as i64) % 3600) / 60;
        format!("{hours}h {minutes}m")
    }
}

pub fn highest_authority_for_metrics(metrics: &Metrics) -> String {
    let mut highest = metrics.highest_authority.clone();
    for (authority, count) in &metrics.tool_authority {
        if *count > 0 {
            highest = higher_tool_authority(&highest, authority);
        }
    }
    highest
}

pub fn higher_tool_authority(a: &str, b: &str) -> String {
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    if authority_rank(b) > authority_rank(a) {
        b.to_string()
    } else {
        a.to_string()
    }
}

pub fn is_high_authority_category(category: &str) -> bool {
    authority_rank(category) >= authority_rank("write_files")
}

pub fn sorted_keys<T>(map: &BTreeMap<String, T>) -> Vec<String> {
    map.keys().cloned().collect()
}

pub fn sorted_set(set: BTreeSet<String>) -> Vec<String> {
    set.into_iter().collect()
}

/// How [`parse_ts`] accepted a timestamp string (rm-502).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimestampForm {
    /// RFC 3339 with an explicit UTC offset (`Z`, `+00:00`, `-0700`, …).
    Rfc3339,
    /// Naive ISO `YYYY-MM-DDTHH:MM:SS[.f]` with no offset — reinterpreted
    /// as UTC by the lenient arm, the Go-parity contract pinned by
    /// `naive_iso_timestamps_match_go_utc_gap_rules`.
    NaiveUtc,
}

/// The single source of timestamp truth (rm-502).
///
/// Every consumer that needs a `DateTime<Utc>` from a session-log
/// timestamp string parses through here — ingest (`analyze`), session
/// ordering, report trend windows, the discovery and SQLite
/// since-filters, governance commit windows, diagnostics latencies and
/// trace steps, and the TUI step timeline. Strict RFC-3339-only copies
/// previously rejected the naive-ISO stamps this arm accepts, so one
/// corpus produced full `--overview` totals while `--diagnostics`
/// reported empty tool latencies and zero-second trace steps.
pub fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    parse_ts_form(value).map(|(ts, _)| ts)
}

fn parse_ts_form(value: &str) -> Option<(DateTime<Utc>, TimestampForm)> {
    if value.is_empty() {
        return None;
    }
    let normalized = value.replace('Z', "+00:00");
    if let Ok(ts) = DateTime::parse_from_rfc3339(&normalized) {
        return Some((ts.with_timezone(&Utc), TimestampForm::Rfc3339));
    }
    NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S"))
        .ok()
        .map(|ts| (ts.and_utc(), TimestampForm::NaiveUtc))
}

pub(crate) fn percentile(sorted: &[f64], p: f64) -> f64 {
    // The single percentile definition for the whole crate (pass-8
    // F8-6): truncating at len*p, pinned as Go-compatible by
    // `percentile_matches_go_index_rule`. reports.rs used to carry a
    // divergent (len-1)*p-rounding copy (20 vs 19 at p=0.95).
    if sorted.is_empty() {
        return 0.0;
    }
    let mut idx = (sorted.len() as f64 * p) as isize;
    if idx >= sorted.len() as isize {
        idx = sorted.len() as isize - 1;
    }
    if idx < 0 {
        idx = 0;
    }
    sorted[idx as usize]
}

fn searchable_tool_args(args: &str) -> Vec<String> {
    let args = args.trim();
    if args.is_empty() || args.contains('\n') || args.len() > 512 {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(args) else {
        return vec![args.to_string()];
    };
    let mut seen = BTreeSet::new();
    collect_searchable_tool_args(&value, &mut seen);
    seen.into_iter().collect()
}

fn collect_searchable_tool_args(value: &Value, seen: &mut BTreeSet<String>) {
    match value {
        Value::String(s) => {
            let item = s.trim();
            if !item.is_empty() && !item.contains('\n') && item.len() <= 512 {
                seen.insert(item.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_searchable_tool_args(item, seen);
            }
        }
        Value::Object(items) => {
            for item in items.values() {
                collect_searchable_tool_args(item, seen);
            }
        }
        _ => {}
    }
}

/// Task-type heuristic (rm-245): a documented, deterministic, three-way
/// cascade over aggregates the parser already produced - no new
/// parsing, no message-content inspection. This is correlation over
/// tool-mix and failure evidence, not ground truth; the inference
/// limits are part of the overview contract (docs/guides/
/// governance-reports.md). The cascade is ordered:
/// - `debugging`: failure-driven work - at least one failed tool call
///   with a >=25% session fail rate (the same strictness the cost-driver
///   note uses), or a `tool_failures` anomaly on the session;
/// - `coding`: execution evidence - write-capable or test/build tool
///   authority observed with at least one tool call;
/// - `planning`: everything else (read-only, reasoning-only, zero-tool
///   sessions).
pub fn infer_task_type(session: &Session) -> &'static str {
    let metrics = &session.metrics;
    let total_calls = metrics.tool_calls_ok + metrics.tool_calls_fail;
    let fail_rate = if total_calls == 0 {
        0.0
    } else {
        metrics.tool_calls_fail as f64 / total_calls as f64
    };
    let failure_driven = (metrics.tool_calls_fail > 0 && fail_rate >= 0.25)
        || session
            .anomalies
            .iter()
            .any(|anomaly| anomaly.kind == "tool_failures");
    if failure_driven {
        return "debugging";
    }
    let execution_evidence = total_calls > 0
        && metrics.tool_authority.iter().any(|(authority, count)| {
            *count > 0
                && matches!(
                    authority.as_str(),
                    "external_publish"
                        | "git_write"
                        | "write_files"
                        | "package_install"
                        | "test_or_build"
                )
        });
    if execution_evidence {
        return "coding";
    }
    "planning"
}

pub(crate) fn classify_tool_authority(tool_call: &ToolCall) -> String {
    let name = tool_call.name.trim().to_ascii_lowercase();
    let args = tool_call.args.trim().to_ascii_lowercase();
    let combined = format!("{} {}", name, args).trim().to_string();
    if combined.is_empty() || name == "unknown" {
        return "unknown_authority".to_string();
    }
    if contains_any(
        &combined,
        &[
            "npm publish",
            "pnpm publish",
            "yarn publish",
            "twine upload",
            "docker push",
            "gh release create",
            "goreleaser release",
        ],
    ) {
        "external_publish".to_string()
    } else if contains_any(
        &combined,
        &[
            "git push",
            "git commit",
            "git tag",
            "git merge",
            "git rebase",
            "git cherry-pick",
            "git reset",
            "git checkout",
            "git switch",
        ],
    ) {
        "git_write".to_string()
    } else if contains_any(
        &combined,
        &[
            "npm install",
            "npm i ",
            "pnpm install",
            "pnpm add",
            "yarn add",
            "pip install",
            "uv add",
            "go get",
            "cargo add",
            "brew install",
        ],
    ) {
        "package_install".to_string()
    } else if contains_any(&combined, &["curl ", "wget ", "http://", "https://"])
        || contains_any(&name, &["web_fetch", "web_search", "network"])
    {
        "network_access".to_string()
    } else if contains_any(
        &combined,
        &[
            "go test",
            "go build",
            "npm test",
            "npm run test",
            "pytest",
            "cargo test",
            "cargo build",
            "make test",
            "make build",
            "mvn test",
            "gradle test",
        ],
    ) {
        "test_or_build".to_string()
    } else if contains_any(
        &name,
        &[
            "write",
            "edit",
            "patch",
            "delete",
            "remove",
            "create_file",
            "replace",
        ],
    ) {
        "write_files".to_string()
    } else if contains_any(
        &name,
        &[
            "bash",
            "shell",
            "terminal",
            "exec",
            "run_command",
            "command",
        ],
    ) {
        "shell_exec".to_string()
    } else if contains_any(&name, &["go_test", "test", "build"]) {
        "test_or_build".to_string()
    } else if contains_any(
        &name,
        &[
            "read", "view", "list", "grep", "rg", "find", "cat", "ls", "glob", "search",
        ],
    ) {
        "read_only_files".to_string()
    } else {
        "unknown_authority".to_string()
    }
}

fn extract_tool_call_files(args: &str) -> Vec<String> {
    if args.trim().is_empty() {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(args) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    collect_tool_call_files(&value, "", &mut seen);
    seen.into_iter().collect()
}

fn collect_tool_call_files(value: &Value, key: &str, seen: &mut BTreeSet<String>) {
    match value {
        Value::Object(items) => {
            for (child_key, child) in items {
                collect_tool_call_files(child, &child_key.to_ascii_lowercase(), seen);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_tool_call_files(child, key, seen);
            }
        }
        Value::String(text) if is_file_surface_key(key) => {
            if let Some(file) = normalize_tool_call_file(text) {
                seen.insert(file);
            }
        }
        _ => {}
    }
}

fn is_file_surface_key(key: &str) -> bool {
    let key = key.replace('-', "_");
    matches!(
        key.as_str(),
        "path"
            | "file"
            | "files"
            | "filename"
            | "file_name"
            | "filepath"
            | "file_path"
            | "target"
            | "target_file"
            | "uri"
    ) || key.contains("file")
        || key.contains("path")
}

fn normalize_tool_call_file(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.contains('\n')
        || value.starts_with("http://")
        || value.starts_with("https://")
    {
        return None;
    }
    let value = value.strip_prefix("file://").unwrap_or(value);
    let mut parts = Vec::new();
    for component in std::path::Path::new(value).components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::RootDir => parts.clear(),
            std::path::Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            std::path::Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().to_string())
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn authority_rank(category: &str) -> i32 {
    match category {
        "unknown_authority" => 1,
        "read_only_files" => 2,
        "test_or_build" => 3,
        "write_files" => 4,
        "package_install" => 5,
        "network_access" => 6,
        "shell_exec" => 7,
        "git_write" => 8,
        "external_publish" => 9,
        _ => 0,
    }
}

fn severity_rank(severity: &str) -> i32 {
    match severity {
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

#[cfg(test)]
pub(crate) mod test_env {
    /// Serializes every unit test that mutates process environment
    /// variables (`HOME`, `XDG_*`, `AGENTTRACE_SESSION_CACHE_DIR`).
    /// Per-module locks previously let the pricing, session-cache, and
    /// statusline tests re-point each other's cache roots mid-flight
    /// (observed as an intermittent `journal_roundtrip…` open failure
    /// under the combined workspace run); one shared lock closes the
    /// class. Poisoning is tolerated so one flaky assertion cannot
    /// cascade.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    pub(crate) fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        match ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recency_session(name: &str, session_start: &str) -> Session {
        Session {
            name: name.to_string(),
            path: format!("/{name}.jsonl"),
            cwd: String::new(),
            branch: String::new(),
            org: String::new(),
            org_email: String::new(),
            metrics: Metrics {
                session_start: session_start.to_string(),
                ..Default::default()
            },
            anomalies: vec![],
            health: 0,
            tool_warnings: vec![],
            diagnostics: Diagnostics::default(),
        }
    }

    #[test]
    fn session_start_cmp_orders_parsed_instants_across_offset_spellings() {
        // rm-785: "2026-01-02T01:30:00Z" and "2026-01-02T10:30:00+09:00"
        // are the same instant in two spellings and must compare Equal; both
        // must order against other instants identically. The raw-string
        // comparators this replaces ordered by byte value, so any
        // non-normalizing producer mis-sorted the whole corpus.
        assert_eq!(session_start_cmp("", ""), std::cmp::Ordering::Equal);
        assert_eq!(
            session_start_cmp("2026-01-02T01:30:00Z", "2026-01-02T10:30:00+09:00"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            session_start_cmp("2026-01-02T10:30:00+09:00", "2026-01-02T01:29:59Z"),
            std::cmp::Ordering::Greater
        );
        // Naive timestamps parse as UTC on the same basis.
        assert_eq!(
            session_start_cmp("2026-01-02T01:30:00", "2026-01-02T01:30:01Z"),
            std::cmp::Ordering::Less
        );
        // Missing or unparseable timestamps count as the OLDEST instant, so
        // newest-first views keep them last and --latest never picks them
        // over a dated session.
        assert_eq!(
            session_start_cmp("", "2000-01-01T00:00:00Z"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            session_start_cmp("bogus", "2000-01-01T00:00:00Z"),
            std::cmp::Ordering::Less
        );
        assert_eq!(session_start_cmp("bogus", ""), std::cmp::Ordering::Equal);
    }

    #[test]
    fn canonical_sessions_orders_mixed_offset_corpus_by_instant() {
        // rm-785 regression fixture: a mixed-offset corpus (+09:00 vs Z vs
        // naive vs absent) orders by parsed instant, newest first, absent
        // last — any future non-normalizing producer fails this pin loudly.
        let sessions = vec![
            recency_session("absent", ""),
            recency_session("z-morning", "2026-01-02T01:30:00Z"),
            recency_session("plus-nine", "2026-01-02T10:30:00+09:00"),
            recency_session("z-later", "2026-01-02T05:00:00Z"),
            recency_session("naive-utc", "2026-01-02T00:00:00"),
        ];
        let ordered = canonical_sessions(&sessions);
        let names: Vec<&str> = ordered.iter().map(|s| s.name.as_str()).collect();
        // z-later (05:00Z) newest; z-morning and plus-nine are the same
        // instant and fall to the name tie-break; naive-utc (00:00Z) is the
        // oldest present session; absent last.
        assert_eq!(
            names,
            vec!["z-later", "plus-nine", "z-morning", "naive-utc", "absent"]
        );
    }

    #[test]
    fn all_three_installers_verify_sha256_sidecars() {
        // rm-005: every distribution channel (shell, PowerShell, npm)
        // must verify the .sha256 sidecar before placing a binary --
        // parity across channels is the acceptance criterion, not just
        // the PowerShell fix in isolation.
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
        let ps1 =
            std::fs::read_to_string(format!("{root}install.ps1")).expect("install.ps1 readable");
        let sh = std::fs::read_to_string(format!("{root}install.sh")).expect("install.sh readable");
        let js = std::fs::read_to_string(format!("{root}npm/scripts/install.js"))
            .expect("npm/scripts/install.js readable");
        assert!(
            ps1.contains(".sha256"),
            "install.ps1 must fetch the sidecar"
        );
        assert!(
            ps1.contains("Get-FileHash"),
            "install.ps1 must hash the downloaded artifact"
        );
        assert!(
            ps1.contains("SHA-256 mismatch"),
            "install.ps1 must refuse to install on mismatch"
        );
        assert!(
            sh.contains(".sha256") && sh.contains("sha256sum"),
            "install.sh sidecar verification must stay in place"
        );
        assert!(
            js.contains(".sha256") && js.contains("createHash"),
            "npm installer sidecar verification must stay in place"
        );
    }

    #[test]
    fn all_three_installers_refuse_when_the_sidecar_cannot_be_verified() {
        // rm-234 (campaign-local rm-029): parity is about refusal
        // semantics, not just sidecar naming. install.ps1's IWR throws
        // on a missing sidecar and the script refuses; npm's download()
        // rejects HTTP != 200 and main() exits 1. install.sh used to warn
        // and proceed -- the gate below pins the refusal so the
        // degradation lane cannot silently return.
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
        let sh = std::fs::read_to_string(format!("{root}install.sh")).expect("install.sh readable");
        assert!(
            !sh.contains("skipping checksum verification"),
            "install.sh must not degrade to skipping verification when the sidecar is missing or no hashing tool exists"
        );
        assert!(
            sh.contains("No checksum sidecar at"),
            "install.sh must diagnose a missing sidecar explicitly"
        );
        assert!(
            sh.contains("No sha256 tool found"),
            "install.sh must diagnose a missing hashing tool explicitly"
        );
        // The refusal must be the terminal path for both conditions:
        // each diagnosis is followed by an abort before the binary is
        // placed (the messages below only print on the way to exit 1).
        let missing = sh
            .find("No checksum sidecar at")
            .and_then(|at| sh[at..].find("exit 1").map(|end| at + end))
            .expect("missing-sidecar diagnosis must be followed by exit 1");
        let no_tool = sh
            .find("No sha256 tool found")
            .and_then(|at| sh[at..].find("exit 1").map(|end| at + end))
            .expect("missing-hashing-tool diagnosis must be followed by exit 1");
        assert!(
            missing < sh.find("Binary size").unwrap_or(usize::MAX),
            "the missing-sidecar refusal must abort before the size check continues the install"
        );
        assert!(
            no_tool < sh.find("Binary size").unwrap_or(usize::MAX),
            "the missing-hashing-tool refusal must abort before the size check continues the install"
        );
    }

    #[test]
    fn event_typed_and_string_usage_coerce_instead_of_dropping_the_event() {
        // Pass-7 P7-1: strict `BTreeMap<String, i64>` usage rejected the
        // shapes real agents write, and the generic fallback then dropped
        // the whole line with no signal.
        let event: Event = serde_json::from_str(
            r#"{"role":"user","content":"x","usage":{"input":{"tokens":5},"output":"2"}}"#,
        )
        .expect("lenient usage deserializes");
        assert_eq!(
            event.usage,
            BTreeMap::from([("input_tokens".to_string(), 5), ("output".to_string(), 2),])
        );
        // Lowercase `usage` used to be invisible next to rename="Usage".
        let lower: Event =
            serde_json::from_str(r#"{"role":"user","content":"y","usage":{"input_tokens":"5"}}"#)
                .expect("lowercase usage key deserializes");
        assert_eq!(lower.usage.get("input_tokens"), Some(&5));
    }

    #[test]
    fn parse_jsonl_session_recovers_recoverable_lines_and_counts_losses() {
        // Pass-7 P7-1 reproducer: a lone-surrogate line and an Event-typed
        // usage line used to vanish silently; an unparseable line must be
        // counted, not ignored.
        let raw = concat!(
            "{\"role\":\"user\",\"content\":\"clean\"}\n",
            "{\"role\":\"user\",\"content\":\"lone \\ud800 tail\"}\n",
            "{\"role\":\"meta\",\"content\":\"usage\",\"usage\":{\"input\":{\"tokens\":7}}}\n",
            "{\"role\":\"user\",\"content\":\"broken \\uzzzz }\n",
        );
        let session = parse_jsonl_session("generic-loss", "generic-loss.jsonl", raw)
            .expect("generic fallback parses");
        assert_eq!(session.metrics.user_messages, 2);
        assert_eq!(
            session.metrics.line_skips,
            BTreeMap::from([("unparseable_line".to_string(), 1)])
        );
        assert_eq!(session.metrics.tokens_input, 7);
    }

    #[test]
    fn adversarial_token_counts_stay_bounded_and_non_negative() {
        let events = vec![
            Event {
                role: "session_meta".to_string(),
                usage: BTreeMap::from([
                    ("input_tokens".to_string(), i64::MAX),
                    ("output_tokens".to_string(), i64::MAX),
                    ("cache_creation_input_tokens".to_string(), i64::MAX),
                    ("cache_read_input_tokens".to_string(), i64::MAX),
                ]),
                ..Event::default()
            },
            Event {
                role: "session_meta".to_string(),
                usage: BTreeMap::from([
                    ("input_tokens".to_string(), 7),
                    ("output_tokens".to_string(), -9),
                    ("cache_read_input_tokens".to_string(), -1_000),
                ]),
                ..Event::default()
            },
        ];
        let session = session_from_events("adversarial", "adversarial.jsonl", events)
            .expect("session from adversarial events");
        assert_eq!(session.metrics.tokens_input, i64::MAX);
        assert_eq!(session.metrics.tokens_output, i64::MAX);
        assert_eq!(session.metrics.tokens_cache_w, i64::MAX);
        assert_eq!(session.metrics.tokens_cache_r, i64::MAX);
        assert_eq!(total_tokens(&session), i64::MAX);
        assert!(session.metrics.cost_estimated.is_finite());
        assert!(session.metrics.cost_estimated >= 0.0);
    }

    #[test]
    fn clamped_token_totals_flag_the_cost_provenance() {
        // rm-046: a class pinned at i64::MAX means accumulation
        // saturated — the total is a clamp, not an exact sum, and the
        // cost provenance must say so instead of claiming precise
        // arithmetic.
        let saturated = session_from_events(
            "clamp",
            "clamp.jsonl",
            vec![
                Event {
                    role: "session_meta".to_string(),
                    usage: BTreeMap::from([("input_tokens".to_string(), i64::MAX)]),
                    ..Event::default()
                },
                Event {
                    role: "session_meta".to_string(),
                    usage: BTreeMap::from([("input_tokens".to_string(), i64::MAX)]),
                    ..Event::default()
                },
            ],
        )
        .expect("session");
        assert_eq!(saturated.metrics.tokens_input, i64::MAX);
        assert_eq!(
            saturated.metrics.provenance.cost, "calculated_from_tokens_clamped",
            "a clamped total must flag itself in cost provenance"
        );
        assert!(saturated.metrics.cost_estimated.is_finite());

        let exact = session_from_events(
            "exact",
            "exact.jsonl",
            vec![Event {
                role: "session_meta".to_string(),
                usage: BTreeMap::from([("input_tokens".to_string(), 7)]),
                ..Event::default()
            }],
        )
        .expect("session");
        assert_eq!(exact.metrics.provenance.cost, "calculated_from_tokens");
    }

    #[test]
    fn implausible_magnitudes_bill_as_parsed_with_disclosure() {
        // rm-600 verify-arm (run bbf61fbd cycle 3): a usage value above
        // IMPLAUSIBLE_TOKEN_MAGNITUDE but inside i64 — the assess F7
        // hostile-corpus shape — is priced AS PARSED (rm-726 census:
        // clamping the value would fabricate a cost the transcript
        // does not contain), and the compound behavior is exactly:
        // cost = value × catalog rate, finite, provenance
        // un-clamped, and the magnitude census disclosed on both the
        // session's disclosure channel and the overview's
        // clamped_values counter. This pin makes every leg of that
        // compound a deliberate change: if a future cycle clamps or
        // rejects the value instead, this test is the one that turns
        // red first.
        let tokens = 4_219_692_706_861_134i64; // 4.2e15: > 1e12 bound, ≪ i64::MAX
        let session = session_from_events(
            "hostile",
            "hostile.jsonl",
            vec![Event {
                role: "session_meta".to_string(),
                model_used: "claude-sonnet-4-5".to_string(),
                usage: BTreeMap::from([("input_tokens".to_string(), tokens)]),
                ..Event::default()
            }],
        )
        .expect("session");
        // The value survives arithmetic un-clamped and exact.
        assert_eq!(session.metrics.tokens_input, tokens);
        assert_eq!(
            session.metrics.provenance.cost, "calculated_from_tokens",
            "accumulation did not saturate, so the provenance must not claim a clamp"
        );
        let price = pricing::lookup_price_tiered("claude-sonnet-4-5", None);
        assert_eq!(
            session.metrics.cost_estimated,
            round4(tokens as f64 / 1e6 * price.input),
            "cost is exactly the parsed magnitude × the catalog rate, on the
             same round4 the accumulation applies (the assess F7 figure's mechanism)"
        );
        assert!(session.metrics.cost_estimated.is_finite());
        // And the magnitude census discloses it on both channels.
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("magnitude_clamped:above_plausibility_bound"),
            Some(&1)
        );
        let overview = compute_overview(std::slice::from_ref(&session));
        assert_eq!(overview.clamped_values, 1);
    }

    #[test]
    fn total_tokens_saturates_instead_of_wrapping_negative() {
        let session = session_from_events("saturate", "saturate.jsonl", vec![Event::default()])
            .expect("session");
        let mut session = session;
        session.metrics.tokens_input = i64::MAX;
        session.metrics.tokens_output = i64::MAX;
        session.metrics.tokens_cache_w = 5;
        session.metrics.tokens_cache_r = 5;
        assert_eq!(total_tokens(&session), i64::MAX);
    }

    #[test]
    fn session_name_uses_first_real_user_request() {
        let session = session_from_events(
            "rollout-2026-07-19-random",
            "session.jsonl",
            vec![
                Event {
                    role: "user".to_string(),
                    content: "# AGENTS.md instructions\n<INSTRUCTIONS>...</INSTRUCTIONS>".to_string(),
                    ..Event::default()
                },
                Event {
                    role: "user".to_string(),
                    content: "# Files mentioned by the user:\n\n## My request for Codex:\n修复全局 Token 展示格式".to_string(),
                    ..Event::default()
                },
            ],
        )
        .expect("build session");

        assert_eq!(session.name, "修复全局 Token 展示格式");
    }

    #[test]
    fn session_name_skips_injected_wrappers() {
        let user = |content: &str| Event {
            role: "user".to_string(),
            content: content.to_string(),
            ..Event::default()
        };
        let cases = [
            (
                vec![user("<system-reminder>\nworktree\n</system-reminder>\n\n搜索最近3天ai新闻")],
                "搜索最近3天ai新闻",
            ),
            (
                vec![
                    user("<turn_aborted>\nThe user interrupted.\n</turn_aborted>"),
                    user("继续修复"),
                ],
                "继续修复",
            ),
            (
                vec![user("<system-reminder data-role=\"user-context\">\nOS\n</system-reminder>\n<user_query>帮我领个优惠券</user_query>")],
                "帮我领个优惠券",
            ),
            (
                vec![user("<teammate-message teammate_id=\"lead\" summary=\"x\">\n第二轮分析\n</teammate-message>")],
                "第二轮分析",
            ),
            (
                vec![user("<goal_context>\nContinue.\n<objective>\n回测最近任务\n</objective>\nmore\n</goal_context>")],
                "回测最近任务",
            ),
            (
                vec![
                    user("<image name=[Image #1] path=\"/tmp/a.png\">"),
                    user("Recommended: install companion packages for pi-subagents."),
                    user("看下这个报错"),
                ],
                "看下这个报错",
            ),
        ];
        for (events, want) in cases {
            let session =
                session_from_events("fallback", "s.jsonl", events).expect("build session");
            assert_eq!(session.name, want);
        }
    }

    #[test]
    fn parsers_preserve_codex_and_pi_workspaces() {
        let codex = parse_raw_session(
            "codex",
            "rollout.jsonl",
            r#"{"type":"session_meta","timestamp":"2026-07-19T00:00:00Z","payload":{"cwd":"/work/codex"}}
{"type":"response_item","timestamp":"2026-07-19T00:00:01Z","payload":{"type":"message","role":"user","content":"task"}}"#,
        )
        .expect("parse codex");
        assert_eq!(codex.cwd, "/work/codex");

        let pi = parse_raw_session(
            "pi",
            "/home/user/.pi/agent/sessions/session.jsonl",
            r#"{"type":"session","version":3,"id":"1","cwd":"/work/pi"}
{"type":"message","timestamp":"2026-07-19T00:00:00Z","message":{"role":"user","content":"task"}}"#,
        )
        .expect("parse pi");
        assert_eq!(pi.cwd, "/work/pi");
    }

    #[test]
    fn tool_file_surfaces_follow_go_structured_key_rules() {
        let files = extract_tool_call_files(
            r#"{"path":"./README.md","command":"go test ./...","nested":{"file_path":"file://src/lib.rs"},"url":"https://example.com/file.txt","body":"a\nb"}"#,
        );

        assert_eq!(
            files,
            vec!["README.md".to_string(), "src/lib.rs".to_string()]
        );
    }

    #[test]
    fn searchable_tool_args_match_go_leaf_string_rules() {
        assert_eq!(
            searchable_tool_args(
                r#"{"cmd":"read_file","path":"src/lib.rs","body":"line one\nline two","repeat":"src/lib.rs","nested":[" cargo test "]}"#,
            ),
            vec![
                "cargo test".to_string(),
                "read_file".to_string(),
                "src/lib.rs".to_string()
            ]
        );
        assert_eq!(
            searchable_tool_args("  read_file src/lib.rs  "),
            vec!["read_file src/lib.rs".to_string()]
        );
        assert!(searchable_tool_args("{\"cmd\":\"read_file\"}\n{\"cmd\":\"rg\"}").is_empty());
        assert!(searchable_tool_args(&"x".repeat(513)).is_empty());
    }

    #[test]
    fn percentile_matches_go_index_rule() {
        let sorted = [1.0, 2.0, 3.0, 31.0, 32.0];
        assert_eq!(percentile(&sorted, 0.95), 32.0);
        assert_eq!(percentile(&sorted, 1.0), 32.0);
        assert_eq!(percentile(&sorted, -1.0), 1.0);
    }

    #[test]
    fn naive_iso_timestamps_match_go_utc_gap_rules() {
        let metrics = analyze(
            &[
                Event {
                    role: "user".to_string(),
                    timestamp: "2026-06-21T01:00:00.100000".to_string(),
                    ..Event::default()
                },
                Event {
                    role: "assistant".to_string(),
                    timestamp: "2026-06-21T01:06:01.100000".to_string(),
                    ..Event::default()
                },
            ],
            "default",
        );

        assert_eq!(metrics.session_start, "2026-06-21T01:00:00Z");
        assert_eq!(metrics.session_end, "2026-06-21T01:06:01Z");
        assert_eq!(metrics.gaps_sec, vec![361.0]);
        assert_eq!(detect_anomalies(&metrics)[0].kind, "hanging");
    }

    #[test]
    fn parse_ts_reports_which_arm_accepted_each_form() {
        // rm-502 acceptance (c): the shared parser is the single source
        // of timestamp truth, and it reports which arm accepted a value
        // so ingest can count naive-UTC reinterpretations.
        let form = |value: &str| parse_ts_form(value).map(|(_, form)| form);
        assert_eq!(form("2026-10-05T03:00:00Z"), Some(TimestampForm::Rfc3339));
        assert_eq!(
            form("2026-10-05T03:00:00+02:00"),
            Some(TimestampForm::Rfc3339)
        );
        assert_eq!(
            form("2026-10-05T03:00:00.500000"),
            Some(TimestampForm::NaiveUtc)
        );
        assert_eq!(form("2026-10-05T03:00:00"), Some(TimestampForm::NaiveUtc));
        assert_eq!(form(""), None);
        assert_eq!(form("not a timestamp"), None);
        assert_eq!(form("1735689600"), None);
    }

    #[test]
    fn fmt_duration_matches_go_shape() {
        assert_eq!(fmt_duration(40.0), "40s");
        assert_eq!(fmt_duration(90.0), "1.5m");
        assert_eq!(fmt_duration(6180.0), "1h 43m");
    }

    #[test]
    fn fmt_duration_never_renders_unit_boundaries() {
        // rm-953: the rendered value must never round up onto the next
        // unit's boundary (no "60s", no "60.0m"); promote instead.
        assert_eq!(fmt_duration(-3.0), "0s");
        assert_eq!(fmt_duration(0.4), "0s");
        assert_eq!(fmt_duration(59.4), "59s");
        assert_eq!(fmt_duration(59.6), "1.0m");
        assert_eq!(fmt_duration(59.5), "1.0m");
        assert_eq!(fmt_duration(3594.0), "59.9m"); // 59.9 minutes exactly
        assert_eq!(fmt_duration(3596.9), "59.9m"); // just under the hour
        assert_eq!(fmt_duration(3597.0), "0h 59m"); // would render "60.0m"
        assert_eq!(fmt_duration(3599.8), "0h 59m"); // the reported case
        assert_eq!(fmt_duration(3600.0), "1h 0m");
        assert_eq!(fmt_duration(3603.0), "1h 0m"); // 60.05 minutes
    }

    #[test]
    fn token_counts_use_compact_units() {
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1_000), "1.0K");
        assert_eq!(format_tokens(1_250_000), "1.2M");
        assert_eq!(format_tokens(43_719_584_174), "43.7B");
        assert_eq!(format_tokens(2_500_000_000_000), "2.5T");
        assert_eq!(format_tokens(999_950), "1.0M");
        assert_eq!(format_cost(35_240.741_6), "$35.2K");
    }

    #[test]
    fn tool_result_error_null_matches_go_success_rules() {
        let metrics = analyze(
            &[
                Event {
                    role: "assistant".to_string(),
                    tool_calls: vec![
                        ToolCall {
                            name: "ok_null_error".to_string(),
                            ..ToolCall::default()
                        },
                        ToolCall {
                            name: "fail_error_object".to_string(),
                            ..ToolCall::default()
                        },
                        ToolCall {
                            name: "fail_success_false".to_string(),
                            ..ToolCall::default()
                        },
                    ],
                    ..Event::default()
                },
                Event {
                    role: "tool".to_string(),
                    content: r#"{"success":true,"error":null}"#.to_string(),
                    ..Event::default()
                },
                Event {
                    role: "tool".to_string(),
                    content: r#"{"error":{"message":"failed"}}"#.to_string(),
                    ..Event::default()
                },
                Event {
                    role: "tool".to_string(),
                    content: r#"{"success":false,"error":null}"#.to_string(),
                    ..Event::default()
                },
            ],
            "default",
        );

        assert_eq!(metrics.tool_calls_total, 3);
        assert_eq!(metrics.tool_calls_ok, 1);
        assert_eq!(metrics.tool_calls_fail, 2);
    }

    #[test]
    fn metric_provenance_distinguishes_reported_and_estimated_values() {
        let estimated = analyze(
            &[
                Event {
                    role: "user".to_string(),
                    content: "hello world".to_string(),
                    timestamp: "2026-01-01T00:00:00Z".to_string(),
                    ..Event::default()
                },
                Event {
                    role: "assistant".to_string(),
                    content: "answer".to_string(),
                    timestamp: "2026-01-01T00:00:01Z".to_string(),
                    ..Event::default()
                },
            ],
            "gpt-5",
        );
        assert_eq!(estimated.provenance.tokens, "estimated_from_text");
        assert_eq!(estimated.provenance.duration, "timestamp_span");

        let mut usage = BTreeMap::new();
        usage.insert("input_tokens".to_string(), 100);
        usage.insert("output_tokens".to_string(), 20);
        let reported = analyze(
            &[
                Event {
                    role: "user".to_string(),
                    content: "this text must not be estimated".to_string(),
                    ..Event::default()
                },
                Event {
                    role: "meta".to_string(),
                    usage,
                    ..Event::default()
                },
            ],
            "gpt-5",
        );
        assert_eq!(reported.tokens_input, 100);
        assert_eq!(reported.tokens_output, 20);
        assert_eq!(reported.provenance.tokens, "reported_by_agent");
    }

    #[test]
    fn fallback_token_estimate_is_cjk_aware() {
        // Pass-6 P6-4: the old bytes/4 heuristic estimated 8 CJK
        // characters (24 bytes) at 6 tokens when real tokenizers use
        // roughly one token per CJK character (~8). Stated tolerance:
        // ±25% of the one-token-per-CJK-character model.
        let metrics = analyze(
            &[Event {
                role: "user".to_string(),
                content: "中文测试中文测试".to_string(),
                ..Event::default()
            }],
            "gpt-5",
        );
        assert_eq!(metrics.tokens_input, 8);

        // ASCII keeps the classic 4-characters-per-token rate.
        let metrics = analyze(
            &[Event {
                role: "user".to_string(),
                content: "12345678".to_string(),
                ..Event::default()
            }],
            "gpt-5",
        );
        assert_eq!(metrics.tokens_input, 2);

        // Mixed text: "café 中文" = 5 ASCII + 3 non-ASCII → (5 + 12)/4 = 4.
        let metrics = analyze(
            &[Event {
                role: "user".to_string(),
                content: "café 中文".to_string(),
                ..Event::default()
            }],
            "gpt-5",
        );
        assert_eq!(metrics.tokens_input, 4);
    }

    #[test]
    fn reasoning_chars_counts_characters_not_bytes() {
        // Pins the unit the field name promises: 4 characters of CJK are
        // 12 UTF-8 bytes, and reasoning_chars must report characters.
        let metrics = analyze(
            &[Event {
                role: "assistant".to_string(),
                content: "ok".to_string(),
                reasoning: "中文测试".to_string(),
                ..Event::default()
            }],
            "gpt-5",
        );
        assert_eq!(metrics.reasoning_chars, 4);
        assert_eq!(metrics.reasoning_lens, vec![4]);
    }

    #[test]
    fn naming_provenance_marks_fallback_names() {
        // Without a usable user request the file name survives and the
        // provenance must say so.
        let session = session_from_events(
            "file-stem",
            "path",
            vec![Event {
                role: "assistant".to_string(),
                content: "answer only".to_string(),
                ..Event::default()
            }],
        )
        .expect("session builds");
        assert_eq!(session.name, "file-stem");
        assert_eq!(session.metrics.provenance.naming, "file_name");

        // With a usable user request the derived title wins and the
        // provenance says where it came from.
        let session = session_from_events(
            "file-stem",
            "path",
            vec![
                Event {
                    role: "user".to_string(),
                    content: "please audit the billing export".to_string(),
                    ..Event::default()
                },
                Event {
                    role: "assistant".to_string(),
                    content: "answer".to_string(),
                    ..Event::default()
                },
            ],
        )
        .expect("session builds");
        assert_eq!(session.name, "please audit the billing export");
        assert_eq!(session.metrics.provenance.naming, "first_user_request");
    }

    #[test]
    fn token_provenance_scans_usage_before_and_after_messages_without_text_double_counting() {
        let mut first_usage = BTreeMap::new();
        first_usage.insert("input_tokens".to_string(), 100);
        first_usage.insert("output_tokens".to_string(), 20);
        let mut second_usage = BTreeMap::new();
        second_usage.insert("input_tokens".to_string(), 3);
        let metrics = analyze(
            &[
                Event {
                    role: "meta".to_string(),
                    usage: first_usage,
                    ..Event::default()
                },
                Event {
                    role: "user".to_string(),
                    content: "a very long message that would otherwise be estimated".to_string(),
                    ..Event::default()
                },
                Event {
                    role: "assistant".to_string(),
                    content: "a very long answer that would otherwise be estimated".to_string(),
                    ..Event::default()
                },
                Event {
                    role: "session_meta".to_string(),
                    usage: second_usage,
                    ..Event::default()
                },
            ],
            "gpt-5",
        );
        assert_eq!(metrics.tokens_input, 103);
        assert_eq!(metrics.tokens_output, 20);
        assert_eq!(metrics.tokens_cache_r, 0);
        assert_eq!(metrics.tokens_cache_w, 0);
        assert_eq!(metrics.provenance.tokens, "reported_by_agent");
    }

    #[test]
    fn detects_dead_loop_failure_chain_and_redundant_tool_calls() {
        let mut events = vec![Event {
            role: "user".to_string(),
            ..Event::default()
        }];
        for turn in 0..4 {
            events.push(Event {
                role: "assistant".to_string(),
                tool_calls: vec![ToolCall {
                    id: turn.to_string(),
                    name: "bash".to_string(),
                    args: r#"{"cmd":"test"}"#.to_string(),
                }],
                ..Event::default()
            });
            events.push(Event {
                role: "tool".to_string(),
                tool_call_id: turn.to_string(),
                is_error: true,
                ..Event::default()
            });
        }
        let warnings = validate_tool_warnings(&events);
        assert!(warnings
            .iter()
            .any(|item| item.pattern == "fail_retry_chain"));

        let redundant = (0..4)
            .flat_map(|index| {
                [
                    Event {
                        role: "user".to_string(),
                        ..Event::default()
                    },
                    Event {
                        role: "assistant".to_string(),
                        tool_calls: vec![ToolCall {
                            id: index.to_string(),
                            name: "bash".to_string(),
                            args: r#"{"cmd":"test"}"#.to_string(),
                        }],
                        ..Event::default()
                    },
                ]
            })
            .collect::<Vec<_>>();
        assert!(validate_tool_warnings(&redundant)
            .iter()
            .any(|item| item.pattern == "redundant"));

        let repeated = (0..4)
            .map(|index| Event {
                role: "assistant".to_string(),
                tool_calls: vec![ToolCall {
                    id: index.to_string(),
                    name: "bash".to_string(),
                    args: "same".to_string(),
                }],
                ..Event::default()
            })
            .collect::<Vec<_>>();
        assert!(validate_tool_warnings(&repeated)
            .iter()
            .any(|item| item.pattern == "dead_loop"));
    }

    #[test]
    fn tool_authority_classification_matches_go_rules() {
        let cases = [
            (
                ToolCall {
                    name: "read_file".to_string(),
                    args: r#"{"path":"README.md"}"#.to_string(),
                    ..ToolCall::default()
                },
                "read_only_files",
            ),
            (
                ToolCall {
                    name: "write_file".to_string(),
                    args: r#"{"path":"README.md"}"#.to_string(),
                    ..ToolCall::default()
                },
                "write_files",
            ),
            (
                ToolCall {
                    name: "terminal".to_string(),
                    args: r#"{"cmd":"go test ./..."}"#.to_string(),
                    ..ToolCall::default()
                },
                "test_or_build",
            ),
            (
                ToolCall {
                    name: "bash".to_string(),
                    args: r#"{"cmd":"npm install"}"#.to_string(),
                    ..ToolCall::default()
                },
                "package_install",
            ),
            (
                ToolCall {
                    name: "terminal".to_string(),
                    args: r#"{"cmd":"sed -n '1,20p' main.go"}"#.to_string(),
                    ..ToolCall::default()
                },
                "shell_exec",
            ),
            (
                ToolCall {
                    name: "bash".to_string(),
                    args: r#"{"cmd":"git push origin HEAD"}"#.to_string(),
                    ..ToolCall::default()
                },
                "git_write",
            ),
            (
                ToolCall {
                    name: "bash".to_string(),
                    args: r#"{"cmd":"curl https://example.com"}"#.to_string(),
                    ..ToolCall::default()
                },
                "network_access",
            ),
            (
                ToolCall {
                    name: "bash".to_string(),
                    args: r#"{"cmd":"npm publish"}"#.to_string(),
                    ..ToolCall::default()
                },
                "external_publish",
            ),
            (
                ToolCall {
                    name: "mystery".to_string(),
                    args: r#"{"value":"x"}"#.to_string(),
                    ..ToolCall::default()
                },
                "unknown_authority",
            ),
        ];

        for (call, want) in cases {
            assert_eq!(classify_tool_authority(&call), want);
        }
        assert_eq!(
            higher_tool_authority("shell_exec", "unknown_authority"),
            "shell_exec"
        );
        assert!(is_high_authority_category("write_files"));
        assert!(!is_high_authority_category("read_only_files"));
    }

    #[test]
    fn session_capability_and_coverage_degrade_with_available_data() {
        let detailed = Session {
            metrics: Metrics {
                tokens_input: 10,
                duration_sec: 1.0,
                gaps_sec: vec![1.0],
                ..Metrics::default()
            },
            ..test_session()
        };
        let aggregate = Session {
            metrics: Metrics {
                tokens_input: 10,
                duration_sec: 1.0,
                ..Metrics::default()
            },
            ..test_session()
        };
        let limited = test_session();
        assert_eq!(session_capability(&detailed), "detailed");
        assert_eq!(session_capability(&aggregate), "aggregate");
        assert_eq!(session_capability(&limited), "limited");
        let health = data_health(&[detailed, aggregate, limited], 3, 0);
        assert_eq!(health.with_tokens, 2);
        assert_eq!(health.with_duration, 2);
        assert_eq!(health.with_event_timing, 1);
        assert_eq!(health.with_diagnostics, 1);
    }

    #[test]
    fn session_comparison_reuses_shared_outcome_rules() {
        let mut current = test_session();
        current.metrics.duration_sec = 5.0;
        current.metrics.cost_estimated = 1.0;
        current.metrics.tool_calls_fail = 0;
        let mut previous = test_session();
        previous.metrics.duration_sec = 10.0;
        previous.metrics.cost_estimated = 2.0;
        previous.metrics.tool_calls_fail = 2;
        let comparison = compare_session_outcome(&current, &previous);
        assert_eq!(comparison.outcome, "faster_cheaper");
        assert_eq!(comparison.reasons, vec!["fewer_failures"]);
    }

    fn test_session() -> Session {
        Session {
            org: String::new(),
            org_email: String::new(),
            name: "test".to_string(),
            path: "test".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        }
    }

    // ---- rm-450: workbuddy input-basis disclosure (upstream #310) ----

    fn wb_meta_event(usage: BTreeMap<String, i64>) -> Event {
        Event {
            role: "meta".to_string(),
            source_tool: "workbuddy".to_string(),
            usage,
            ..Event::default()
        }
    }

    #[test]
    fn workbuddy_basis_disclosure_flags_suspected_mismatch() {
        // Basis mismatch as analyze() sees it: a raw input_tokens 1000
        // with cache_read 1500 subtracts/clamps the input to 0 (post
        // rm-529 the parser also clamps the cache count itself, to 1000;
        // this map keeps 1500 to pin the disclosure logic, which keys on
        // the zeroed input) -- the silent zero this batch exists
        // to disclose.
        let usage: BTreeMap<String, i64> = [
            ("input_tokens", 0),
            ("output_tokens", 50),
            ("cache_read_input_tokens", 1500),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        let metrics = analyze(&[wb_meta_event(usage)], "gpt-5");
        assert_eq!(
            metrics
                .disclosure_counters
                .get("workbuddy_input_basis:cache_subtracted"),
            Some(&1)
        );
        assert_eq!(
            metrics
                .disclosure_counters
                .get("workbuddy_input_basis:zeroed_suspected_mismatch"),
            Some(&1)
        );
        assert_eq!(metrics.tokens_input, 0);
        assert_eq!(metrics.tokens_cache_r, 1500);
    }

    #[test]
    fn workbuddy_basis_disclosure_counts_plain_subtraction() {
        // Cache-inclusive basis: the subtraction fires but leaves a
        // positive input -- disclose the subtraction, no mismatch flag.
        let usage: BTreeMap<String, i64> = [
            ("input_tokens", 500),
            ("output_tokens", 50),
            ("cache_read_input_tokens", 1500),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        let metrics = analyze(&[wb_meta_event(usage)], "gpt-5");
        assert_eq!(
            metrics
                .disclosure_counters
                .get("workbuddy_input_basis:cache_subtracted"),
            Some(&1)
        );
        assert!(!metrics
            .disclosure_counters
            .contains_key("workbuddy_input_basis:zeroed_suspected_mismatch"));
    }

    #[test]
    fn workbuddy_basis_disclosure_ignores_other_sources_and_no_cache() {
        // cache_read on a non-workbuddy meta event is the normal Claude
        // shape and must NOT trip the workbuddy disclosure; a workbuddy
        // usage without cache_read leaves both channels empty entirely.
        let claude_usage: BTreeMap<String, i64> =
            [("input_tokens", 500), ("cache_read_input_tokens", 1500)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();
        let mut e = wb_meta_event(claude_usage);
        e.source_tool = "claude_code".to_string();
        let metrics = analyze(&[e], "gpt-5");
        assert!(metrics.line_skips.is_empty());
        assert!(metrics.disclosure_counters.is_empty());

        let plain_usage: BTreeMap<String, i64> = [("input_tokens", 500)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let metrics = analyze(&[wb_meta_event(plain_usage)], "gpt-5");
        assert!(metrics.line_skips.is_empty());
        assert!(metrics.disclosure_counters.is_empty());
    }

    #[test]
    fn workbuddy_basis_disclosure_end_to_end_from_fixtures() {
        // Fixture shape mirrors real workbuddy transcripts: message-level
        // usage under the detector's type/sessionId/cwd keys.
        let mismatch = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/workbuddy/usage-basis-mismatch.jsonl"
        ))
        .expect("fixture");
        let session = parse_raw_session("wb-mismatch", "wb-mismatch.jsonl", &mismatch)
            .expect("parses as workbuddy");
        assert_eq!(session.metrics.source_tool, "workbuddy");
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("workbuddy_input_basis:zeroed_suspected_mismatch"),
            Some(&1)
        );
        // rm-600 (upstream #316): cache_read is now clamped to the
        // source-recorded input, so a record claiming 1500 cached
        // against 1000 input reports cache_r 1000 (was 1500) with the
        // clamp disclosed by the parser-side counter below.
        assert_eq!(session.metrics.tokens_input, 0);
        // rm-529 clamp (integration 2026-10-06, conflict case
        // 44cf98cc, re-mint of 4d35cd1e): cache_read clamps to the
        // reported input (1500 -> 1000 on this fixture), so the
        // disclosed cache total is the clamped 1000; both
        // basis-disclosure counters still fire.
        assert_eq!(session.metrics.tokens_cache_r, 1000);
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("workbuddy_input_basis:cache_clamped"),
            Some(&1),
            "rm-600: the 1500>1000 clamp must be disclosed"
        );

        let inclusive = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/workbuddy/usage-basis-inclusive.jsonl"
        ))
        .expect("fixture");
        let session = parse_raw_session("wb-inclusive", "wb-inclusive.jsonl", &inclusive)
            .expect("parses as workbuddy");
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("workbuddy_input_basis:cache_subtracted"),
            Some(&1)
        );
        assert!(!session
            .metrics
            .disclosure_counters
            .contains_key("workbuddy_input_basis:zeroed_suspected_mismatch"));
        assert_eq!(session.metrics.tokens_input, 500);
    }
}

#[test]
fn detect_anomalies_flags_implausible_magnitudes_from_the_disclosure_channel() {
    // rm-726: the magnitude_clamped:* disclosure family is a
    // first-class anomaly (severity high → health 70), while clean
    // metrics stay at zero anomalies of this class.
    let mut metrics = Metrics::default();
    metrics
        .disclosure_counters
        .insert("magnitude_clamped:above_i64".to_string(), 1);
    metrics
        .disclosure_counters
        .insert("magnitude_clamped:above_plausibility_bound".to_string(), 2);
    metrics
        .disclosure_counters
        .insert("magnitude_clamped:negative_to_zero".to_string(), 2);
    let anomaly = detect_anomalies(&metrics)
        .into_iter()
        .find(|anomaly| anomaly.kind == "implausible_magnitude")
        .expect("clamp census must produce the anomaly");
    assert_eq!(anomaly.severity, "high");
    assert!(anomaly.detail.contains("i64 bounds"), "{}", anomaly.detail);
    assert!(anomaly.detail.contains("negative"), "{}", anomaly.detail);
    assert!(
        anomaly.detail.contains("1000000000000"),
        "{}",
        anomaly.detail
    );

    let clean = detect_anomalies(&Metrics::default());
    assert!(
        !clean
            .iter()
            .any(|anomaly| anomaly.kind == "implausible_magnitude"),
        "clean metrics must stay clean"
    );
}

// ---- rm-201 (run 12a73275 cycle 4): --quota plan-window view -----------

#[cfg(test)]
mod quota_tests {
    use super::*;

    fn quota_session(name: &str, stamps: &[&str], cost: f64) -> Session {
        let timestamps = stamps
            .iter()
            .map(|stamp| parse_ts(stamp).expect("fixture timestamp parses"))
            .collect::<Vec<_>>();
        Session {
            name: name.to_string(),
            path: format!("/{name}.jsonl"),
            cwd: String::new(),
            branch: String::new(),
            org: String::new(),
            org_email: String::new(),
            metrics: Metrics {
                timestamps,
                cost_estimated: cost,
                session_start: stamps.first().copied().unwrap_or("").to_string(),
                session_end: stamps.last().copied().unwrap_or("").to_string(),
                ..Default::default()
            },
            anomalies: vec![],
            health: 0,
            tool_warnings: vec![],
            diagnostics: Diagnostics::default(),
        }
    }

    fn quota_span_session(name: &str, start: &str, end: &str, cost: f64) -> Session {
        // The warm-cache shape: `timestamps` is serde-skipped, only the
        // session_start/session_end strings round-trip.
        Session {
            org: String::new(),
            org_email: String::new(),
            name: name.to_string(),
            path: format!("/{name}.jsonl"),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                timestamps: Vec::new(),
                cost_estimated: cost,
                session_start: start.to_string(),
                session_end: end.to_string(),
                ..Default::default()
            },
            anomalies: vec![],
            health: 0,
            tool_warnings: vec![],
            diagnostics: Diagnostics::default(),
        }
    }

    #[test]
    fn quota_windows_bill_ttl_correctly() {
        // Cutoff is the newest event: 2026-10-12T10:30:00Z. The 5h
        // window is [05:30, 10:30]; the 7d window starts 2026-10-05.
        let sessions = vec![
            // Both events inside the 5h window: full $1.00.
            quota_session(
                "fresh",
                &["2026-10-12T10:00:00Z", "2026-10-12T10:30:00Z"],
                1.0,
            ),
            // Half the events inside (05:00 is out, 10:30 is in): $2.00 x 0.5.
            quota_session(
                "straddler",
                &["2026-10-12T05:00:00Z", "2026-10-12T10:30:00Z"],
                2.0,
            ),
        ];
        let report = quota_report(&sessions);
        assert_eq!(report["schema"], "agenttrace.quota/1");
        assert_eq!(report["corpus"]["cutoff_utc"], "2026-10-12T10:30:00Z");
        let five = &report["five_hour"];
        assert_eq!(five["window_start_utc"], "2026-10-12T05:30:00Z");
        assert_eq!(five["spend_usd"], 2.0, "1.0 x 1.0 + 2.0 x 0.5");
        assert_eq!(five["sessions_contributing"], 2);
        assert_eq!(five["resets_at_utc"], "2026-10-12T15:30:00Z");
        let weekly = &report["weekly"];
        assert_eq!(weekly["window_start_utc"], "2026-10-05T10:30:00Z");
        assert_eq!(weekly["spend_usd"], 3.0, "every event is inside the week");
        assert_eq!(weekly["resets_at_utc"], "2026-10-19T10:30:00Z");
        // Limits are disclosed-unknown, never invented.
        assert!(report["limits"]["five_hour_usd"].is_null());
        assert!(report["limits"]["weekly_usd"].is_null());
        assert_eq!(report["corpus"]["attribution"]["event_timestamps"], 2);
        assert_eq!(report["corpus"]["attribution"]["unplaced"], 0);
    }

    #[test]
    fn quota_warm_cache_sessions_bill_by_span() {
        // 6h span [04:30, 10:30], 5h window [05:30, 10:30]: overlap 5h of
        // 6h -> $6.00 x 5/6 = $5.00, plus the instant session's full
        // $0.50 -> $5.50 (rounded to 4dp).
        let sessions = vec![
            quota_span_session(
                "cached",
                "2026-10-12T04:30:00Z",
                "2026-10-12T10:30:00Z",
                6.0,
            ),
            // Instant span inside the window: full cost.
            quota_span_session(
                "instant",
                "2026-10-12T08:00:00Z",
                "2026-10-12T08:00:00Z",
                0.5,
            ),
        ];
        let report = quota_report(&sessions);
        assert_eq!(report["corpus"]["cutoff_utc"], "2026-10-12T10:30:00Z");
        assert_eq!(report["five_hour"]["spend_usd"], 5.5);
        assert_eq!(report["five_hour"]["sessions_contributing"], 2);
        assert_eq!(report["weekly"]["spend_usd"], 6.5);
        let attribution = &report["corpus"]["attribution"];
        assert_eq!(attribution["session_span"], 2);
        assert_eq!(attribution["event_timestamps"], 0);
        assert_eq!(attribution["unplaced"], 0);
    }

    #[test]
    fn quota_empty_and_unplaced_corpus_discloses_instead_of_guessing() {
        // Empty corpus: no cutoff, undefined windows — never a zero-cost
        // window that would masquerade as a measured quota.
        let report = quota_report(&[]);
        assert!(report["corpus"]["cutoff_utc"].is_null());
        assert!(report["five_hour"].is_null());
        assert!(report["weekly"].is_null());
        let text = render_quota_view("text", &[]).expect("text lane renders");
        assert!(
            text.contains("no session carries placeable time"),
            "empty render says so: {text}"
        );

        // A session with cost but no placeable time at all: counted,
        // disclosed, billed at zero.
        let unplaced = vec![quota_session("nowhen", &[], 9.0)];
        let report = quota_report(&unplaced);
        assert_eq!(report["corpus"]["sessions"], 1);
        assert_eq!(report["corpus"]["attribution"]["unplaced"], 1);
        assert!(report["five_hour"].is_null(), "cutoff stays null");
    }

    #[test]
    fn quota_render_lanes_and_schema_stability() {
        let sessions = vec![quota_session("solo", &["2026-10-12T10:30:00Z"], 0.0105)];
        let json = render_quota_view("json", &sessions).expect("json lane renders");
        let parsed: serde_json::Value =
            serde_json::from_str(json.trim()).expect("json lane is valid JSON");
        assert_eq!(parsed["schema"], "agenttrace.quota/1");
        for key in ["five_hour", "weekly", "limits", "corpus"] {
            assert!(
                parsed.as_object().expect("object").contains_key(key),
                "key {key}"
            );
        }
        for key in [
            "window_start_utc",
            "window_end_utc",
            "spend_usd",
            "sessions_contributing",
            "resets_at_utc",
        ] {
            assert!(
                parsed["five_hour"]
                    .as_object()
                    .expect("window object")
                    .contains_key(key),
                "window key {key}"
            );
        }
        let text = render_quota_view("text", &sessions).expect("text lane renders");
        assert!(text.starts_with("Quota view"), "text lane: {text}");
        assert!(
            text.contains("$0.0105"),
            "spend carries sub-cent precision: {text}"
        );
        assert!(
            text.contains("unknown"),
            "limits stay disclosed-unknown: {text}"
        );
        // Unsupported formats are loud, not silent text fallbacks.
        let err = render_quota_view("csv", &sessions).unwrap_err();
        assert!(
            err.to_string().contains("-f text and -f json only"),
            "format guard names the supported lanes: {err}"
        );
    }

    #[test]
    fn quota_cutoff_clamps_future_skewed_timestamps() {
        // rm-1402 (run f2fccaa9 cycle 3, assess 53af6c9e F3): the
        // cutoff used to be an unbounded max() over every last-event /
        // session_end instant, so ONE future-dated journal line
        // (clock skew or a hostile meter) anchored the rolling
        // windows in the future and every real event fell OUTSIDE the
        // trailing spans — the PoC corpus billed $0.0045 against an
        // honest $1.52 (99.7% of spend hidden). The cutoff is now the
        // newest PLAUSIBLE instant: timestamps after now are
        // discarded at a disclosed clock-skew guard, and the discard
        // count rides `corpus.future_discarded_sessions`.
        //
        // 2099 stays implausibly future for the life of this pin; the
        // real events use fixed past stamps so the corpus is
        // deterministic regardless of when the test runs.
        let sessions = vec![
            quota_session(
                "real",
                &["2026-10-12T10:00:00Z", "2026-10-12T10:30:00Z"],
                1.52,
            ),
            quota_session(
                "skewed",
                &["2099-06-01T00:00:00Z", "2099-06-01T01:00:00Z"],
                0.0045,
            ),
        ];
        let report = quota_report(&sessions);
        assert_eq!(
            report["corpus"]["cutoff_utc"], "2026-10-12T10:30:00Z",
            "the cutoff is the newest timestamp at-or-before now, never the 2099 skew"
        );
        assert_eq!(
            report["corpus"]["future_discarded_sessions"], 1,
            "the clamp firing is disclosed in the corpus block"
        );
        assert!(
            report["corpus"]["cutoff_basis"]
                .as_str()
                .expect("cutoff basis string")
                .contains("clock-skew guard"),
            "the basis names the clock-skew guard"
        );
        assert_eq!(
            report["five_hour"]["spend_usd"], 1.52,
            "both real events sit inside the 5h window; the skewed line bills 0"
        );
        assert_eq!(
            report["weekly"]["spend_usd"], 1.52,
            "the honest weekly number is restored (pre-fix: $0.0045, 99.7% hidden)"
        );
        assert_eq!(
            report["weekly"]["sessions_contributing"], 1,
            "only the plausible session contributes"
        );
    }

    #[test]
    fn quota_future_only_corpus_discloses_null_cutoff() {
        // Every session in the future: the clamp discards them all,
        // so the corpus has NO plausible cutoff — undefined windows
        // with a discard count, never a future-anchored zero-spend
        // masquerade and never a silent empty report.
        let sessions = vec![
            quota_session("hostile", &["2099-06-01T00:00:00Z"], 5.0),
            quota_span_session(
                "hostile-span",
                "2099-06-02T00:00:00Z",
                "2099-06-02T06:00:00Z",
                7.0,
            ),
        ];
        let report = quota_report(&sessions);
        assert!(
            report["corpus"]["cutoff_utc"].is_null(),
            "no plausible cutoff"
        );
        assert_eq!(report["corpus"]["future_discarded_sessions"], 2);
        assert!(report["five_hour"].is_null() && report["weekly"].is_null());
        let text = render_quota_view("text", &sessions).expect("text lane renders");
        assert!(
            text.contains("no session carries placeable time"),
            "fully future corpus renders the undefined-cutoff lane: {text}"
        );
    }

    #[test]
    fn quota_span_sessions_honor_the_future_clamp() {
        // Warm-cache sessions place by session_end; a future session_end
        // must hit the same clock-skew guard as event timestamps, while
        // a real span keeps its TTL-correct share beside it.
        let sessions = vec![
            quota_span_session(
                "real-span",
                "2026-10-12T04:30:00Z",
                "2026-10-12T10:30:00Z",
                6.0,
            ),
            quota_span_session(
                "future-span",
                "2099-06-01T00:00:00Z",
                "2099-06-01T06:00:00Z",
                1.0,
            ),
        ];
        let report = quota_report(&sessions);
        assert_eq!(report["corpus"]["cutoff_utc"], "2026-10-12T10:30:00Z");
        assert_eq!(report["corpus"]["future_discarded_sessions"], 1);
        // 6h span, 5h window overlap: $6.00 x 5/6 = $5.00, weekly $6.00.
        assert_eq!(report["five_hour"]["spend_usd"], 5.0);
        assert_eq!(report["weekly"]["spend_usd"], 6.0);
    }
}

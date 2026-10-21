use crate::{
    parse_ts, pricing, project_name, resolve_project, round4, session_capability, total_tokens,
    Session, PER_BLOCK_PRICING_MARKER,
};
// rm-985 (adopted) / rm-897 rider: the --recommend evidence strings are a text
// lane the CLI dispatch sanitizer only strips control bytes from, so a session
// or tool name that begins with a spreadsheet formula introducer would paste
// straight through. Guard at the source, like every other rm-985 text lane.
use crate::reports::guard_formula_cell;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

/// rm-726: CostAudit serde guard — clamp count only appears when
/// nonzero, so clean audits keep their exact prior JSON shape.
fn usize_is_zero(value: &usize) -> bool {
    *value == 0
}

#[derive(Debug, Clone, Serialize)]
pub struct CostAudit {
    pub pricing_source: String,
    pub total_estimated_cost: f64,
    /// rm-726: how many implausible-magnitude values were clamped
    /// (i64 bounds, negative-to-zero, above the plausibility bound)
    /// across the audited sessions — zero means the totals price
    /// untouched values.
    #[serde(skip_serializing_if = "usize_is_zero")]
    pub clamped_values: usize,
    /// rm-990 (cycle 2, run b5affead): sessions the pi lane marked as
    /// cancelled — `pi_run_aborted` disclosures minted from either the
    /// message-level `stopReason:"aborted"` arm (the one that fires
    /// on real journals) or the pi >=1.1.0 `agent_settled{aborted}`
    /// entry. Zero (or an absent field once serialized) means no
    /// cancelled run muddies the finished-run totals the audit
    /// reasons about.
    #[serde(skip_serializing_if = "usize_is_zero")]
    pub aborted_sessions: usize,
    pub stored_estimated_cost_usd: f64,
    pub current_estimated_cost_usd: Option<f64>,
    pub pricing_coverage: PricingCoverage,
    pub by_provider_model: Vec<ModelCostAudit>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PricingCoverage {
    pub priced_sessions: usize,
    pub fallback_priced_sessions: usize,
    /// rm-1508: sessions whose cache tokens billed at the absent-rate
    /// fallback (catalog row publishes no cache rate; cr at the input
    /// rate, cw at 1.25× input) — counted from the
    /// `cache_rate_fallback` disclosure counter. Zero (absent once
    /// serialized) means no session in scope hit the fallback.
    #[serde(skip_serializing_if = "usize_is_zero")]
    pub cache_rate_fallback_sessions: usize,
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
    /// Vendor-recorded cost carried by these sessions (upstream
    /// `usage` cost totals), when any (rm-570). The audit's
    /// comparison carries these dollars at face value on the rm-520
    /// same-basis re-price — or withholds the re-price entirely when
    /// the recorded token split is not reproducible (rm-578); the
    /// `pricing_note` names the recorded/estimate-only split.
    pub recorded_cost_usd: Option<f64>,
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
    /// rm-164 arm 3 (run d2775a7e): the >1h-tier cache-write class —
    /// priced at its own rate, never inside `cache_write`.
    pub cache_write_1h: i64,
    pub cache_read: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PriceBreakdown {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    /// rm-164 arm 3 (run d2775a7e): the billed >1h-tier cache-write
    /// rate — the row's published `cw_1h`, or the 5m billed rate when
    /// the row publishes none (disclosed in the source arm).
    pub cache_write_1h: f64,
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
    /// Vendor-recorded cost this session carries, when any (rm-570).
    /// It rides the current-rate comparison at face value on the
    /// rm-520 same-basis re-price — or withholds the re-price when
    /// the recorded token split is not reproducible (rm-578).
    pub recorded_cost_usd: Option<f64>,
    pub pricing_status: String,
    pub pricing_note: String,
}

/// rm-520: the recorded-cost half of an rm-436 stored estimate — the
/// token classes the source journal already priced (excluded from the
/// catalog formula so they are charged exactly once, at face value)
/// and the recorded USD total. The audit's "current" recompute must
/// add this basis back; a catalog-only total compared against a
/// recorded-cost-inclusive stored number false-trips the drift note
/// on every recorded-cost session.
#[derive(Debug, Clone, Default)]
struct RecordedCostBasis {
    input: i64,
    output: i64,
    cache_write: i64,
    cache_write_1h: i64,
    cache_read: i64,
    usd: f64,
}

impl RecordedCostBasis {
    fn of(metrics: &crate::Metrics) -> Self {
        Self {
            input: metrics.upstream_priced_input,
            output: metrics.upstream_priced_output,
            cache_write: metrics.upstream_priced_cache_w,
            cache_write_1h: metrics.upstream_priced_cache_w_1h,
            cache_read: metrics.upstream_priced_cache_r,
            usd: metrics.upstream_cost_usd,
        }
    }
}

/// rm-578: the drift arm of the pricing note. The current-rate total
/// is compared against the stored estimate on the stored estimate's
/// own basis — lib.rs prices it as "catalog rates on the tokens the
/// journal did not record a cost for, plus the recorded dollars at
/// face value" — so a difference really does mean the catalog rates
/// moved. The comparison the audit used to make re-priced every
/// token at catalog rates and read every single-model recorded-cost
/// session as drift, because recorded dollars and catalog dollars
/// legitimately differ (the false-drift defect this row fixes).
///
/// No suffix when the totals agree on that basis, when there is no
/// single-rate re-price (aggregates), or when the session carries
/// recorded dollars whose token split is unavailable: a session with
/// positive `upstream_cost_usd` but all-zero `upstream_priced_*`
/// counters cannot have its basis reproduced. `rm-520` persists the
/// split through the session cache (schema 40 → 41), but rows
/// written before that bump — and any other lossy path — still
/// surface as recorded dollars without a split, and a basis the
/// audit cannot reproduce must stay silent instead of being
/// reported as drift — the same-basis re-price is withheld entirely
/// rather than double-count the recorded tokens into a fresh
/// catalog total. The withheld-basis disclosure is cause-neutral by
/// design (the run's independent-review finding): at audit time a
/// pre-bump cache row and a journal that recorded cost without any
/// token classes present the identical all-zero-split shape, so the
/// note must not name a cause — the cache — it cannot prove.
fn append_drift_note(
    pricing_note: &str,
    current_total: Option<f64>,
    stored: f64,
    recorded_usd: f64,
    split_reproducible: bool,
) -> String {
    let Some(current_total) = current_total else {
        if recorded_usd > 0.0 && !split_reproducible {
            return format!(
                "{pricing_note}; no same-basis re-price available — \
                 the recorded-cost token split is unavailable"
            );
        }
        return pricing_note.to_string();
    };
    if recorded_usd > 0.0 && !split_reproducible {
        return pricing_note.to_string();
    }
    if (current_total - stored).abs() > 0.0001 {
        format!(
            "{pricing_note}; current rates recalculate a different total than the stored estimate"
        )
    } else {
        pricing_note.to_string()
    }
}

fn current_cost_estimate(
    model: &str,
    service_tier: Option<&str>,
    tokens: &TokenBreakdown,
    recorded: &RecordedCostBasis,
) -> Option<(String, PriceBreakdown, PriceBreakdown)> {
    if model == "multiple" {
        return None;
    }
    // rm-610: the "current rates" recompute must bill the tier the
    // session(s) actually ran on — an untiered recompute priced every
    // flex session at the standard row and flagged a false drift
    // (current 48 vs stored 36 on a flex+standard corpus).
    let rate = pricing::lookup_price_tiered(model, service_tier);
    // rm-1508: the recompute bills the same absent-rate fallback the
    // stored estimate did (cr = input rate, cw = 1.25× input), so the
    // drift check never flags a false mismatch on fallback-priced
    // sessions; the rates render with the fallback disclosed in the
    // source arm so the census carries the decision.
    let (cw_billed, cw_absent) = rate.cache_write_billed();
    let (cr_billed, cr_absent) = rate.cache_read_billed();
    // rm-164 arm 3: the 1h tier re-prices at its own billed class —
    // the drift check must not fold 1h tokens into the 5m rate
    // (that would false-flag every 1h-heavy session after the
    // estimate formula gained the class).
    let (cw_1h_billed, cw_1h_assumed) = rate.cache_write_1h_billed();
    let rates = PriceBreakdown {
        input: rate.input,
        output: rate.output,
        cache_write: cw_billed,
        cache_write_1h: cw_1h_billed,
        cache_read: cr_billed,
        total: 0.0,
    };
    let mut components = PriceBreakdown {
        input: round4(tokens.input.saturating_sub(recorded.input) as f64 / 1e6 * rate.input),
        output: round4(tokens.output.saturating_sub(recorded.output) as f64 / 1e6 * rate.output),
        cache_write: round4(
            tokens.cache_write.saturating_sub(recorded.cache_write) as f64 / 1e6 * cw_billed,
        ),
        cache_write_1h: round4(
            tokens
                .cache_write_1h
                .saturating_sub(recorded.cache_write_1h) as f64
                / 1e6
                * cw_1h_billed,
        ),
        cache_read: round4(
            tokens.cache_read.saturating_sub(recorded.cache_read) as f64 / 1e6 * cr_billed,
        ),
        total: 0.0,
    };
    // The recorded USD rides in the total at face value: by the time
    // an audit sees the session it has no token-class attribution,
    // but it is part of the rm-436 basis the stored estimate was
    // computed on, so any comparison against that stored number must
    // include it.
    components.total = round4(
        components.input
            + components.output
            + components.cache_write
            + components.cache_write_1h
            + components.cache_read
            + recorded.usd,
    );
    // rm-610: disclose the tier the recompute billed on, mirroring the
    // provenance label `analyze` stamps on tier-priced sessions.
    let source = match service_tier {
        Some(tier) => format!(
            "{}; service_tier: {tier}",
            pricing::pricing_source_for(model)
        ),
        None => pricing::pricing_source_for(model),
    };
    // rm-1508: carry the absent-rate fallback in the same disclosure
    // string — only when the row actually prices cache tokens at the
    // fallback, so published-rate rows render unchanged.
    let source = if cw_absent || cr_absent {
        format!("{source}; cache rates not published — fallback (cr=input, cw=1.25×input)")
    } else {
        source
    };
    // rm-164 arm 3: carry the 5m-assumed 1h tier the same way — only
    // when the row publishes no >1h rate AND the session actually
    // billed 1h-tier tokens does the note say so (the estimate-side
    // `cache_write_1h_5m_assumed` disclosure is gated identically);
    // token-less rows keep their source string unchanged.
    let source = if cw_1h_assumed && tokens.cache_write_1h > 0 {
        format!("{source}; 1h cache-write rate not published — 1h tier billed at the 5m rate")
    } else {
        source
    };
    Some((source, rates, components))
}

/// rm-521: `model_used == "multiple"` has two producers with opposite
/// exactness. Per-block pricing (rm-438, pi-family journals) prices
/// every usage block at the model that produced it — the stored total
/// is exact. SQLite aggregates (sqlite_sessions.rs) merge sessions
/// across models with no per-model split — "no single-model exact
/// price applies" is true only there. The audit note must claim the
/// mechanism that actually produced the stored number. The marker this
/// matches on is the shared PER_BLOCK_PRICING_MARKER const (lib.rs),
/// the same symbol `analyze` mints the provenance with.
fn is_per_block_multiple(session: &Session) -> bool {
    session
        .metrics
        .provenance
        .pricing_source
        .contains(PER_BLOCK_PRICING_MARKER)
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
        /// rm-578: some session in the row carries recorded dollars
        /// without their token split (rows written before the rm-520
        /// schema bump, or any other lossy path, drop the counters) —
        /// the row's basis cannot be reproduced, so the drift note
        /// stays off and the current-rate column is withheld for the
        /// whole row.
        split_lost: bool,
        specific: usize,
        fallback: usize,
        unknown: usize,
        recorded: RecordedCostBasis,
        per_block_multiple: usize,
        // rm-610: the tiers the row's sessions ran on. One agreed tier
        // re-prices tier-aware; a mix cannot carry one rate row and is
        // disclosed as such instead of silently pricing standard.
        tiers: BTreeSet<Option<String>>,
        // rm-570 (run e88f2da2 cycle 3): how many sessions carry
        // vendor-recorded dollars — the disclosure counter for the F1
        // split note below. The dollars themselves ride
        // `recorded.usd` (rm-520's basis) so the two can never drift.
        recorded_sessions: usize,
    }
    let mut rows: BTreeMap<(String, String), Aggregate> = BTreeMap::new();
    let mut coverage = PricingCoverage::default();
    // rm-726: the audit discloses the magnitude-clamp census beside the
    // total — clamped values are priced at the i64 bounds, and the
    // summary must say how many there were instead of implying clean
    // arithmetic.
    let clamped_values: usize = sessions
        .iter()
        .map(|session| {
            session
                .metrics
                .disclosure_counters
                .iter()
                .filter(|(key, _)| key.starts_with("magnitude_clamped:"))
                .map(|(_, count)| count)
                .sum::<usize>()
        })
        .sum();
    // rm-990: the cancelled-vs-finished split. Counted per SESSION
    // (any abort signal present), not per signal — the audit reasons
    // about runs, and one run can legitimately carry several abort
    // signals (retry after escape).
    let aborted_sessions: usize = sessions
        .iter()
        .filter(|session| {
            session
                .metrics
                .disclosure_counters
                .contains_key("pi_run_aborted")
        })
        .count();
    for session in sessions {
        let provider = session.metrics.source_tool.clone();
        let model = normalized_model(&session.metrics.model_used);
        let row = rows.entry((provider, model.clone())).or_default();
        row.sessions += 1;
        row.tiers.insert(session.metrics.service_tier.clone());
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
        // rm-1500 (cycle 5, run e4209f0c): governance rows fold cost_estimated;
        // gate it like the lib.rs overview arms (the "every aggregate fold"
        // acceptance named this row and it was missed). Poisoned sessions
        // contribute 0.0 to the row and stay counted by overview.non_finite_costs.
        row.cost += if session.metrics.cost_estimated.is_finite() {
            session.metrics.cost_estimated
        } else {
            0.0
        };
        // rm-520: carry each session's rm-436 recorded-cost basis into
        // the row so the drift comparison prices recorded classes
        // exactly once instead of re-estimating them at catalog rates.
        row.recorded.input = row
            .recorded
            .input
            .saturating_add(session.metrics.upstream_priced_input);
        row.recorded.output = row
            .recorded
            .output
            .saturating_add(session.metrics.upstream_priced_output);
        row.recorded.cache_write = row
            .recorded
            .cache_write
            .saturating_add(session.metrics.upstream_priced_cache_w);
        row.recorded.cache_read = row
            .recorded
            .cache_read
            .saturating_add(session.metrics.upstream_priced_cache_r);
        row.recorded.usd += session.metrics.upstream_cost_usd;
        // rm-570 (run e88f2da2 cycle 3): count the sessions carrying
        // vendor-recorded dollars — the F1 split note below needs the
        // recorded/estimate-only session split.
        if session.metrics.upstream_cost_usd > 0.0 {
            row.recorded_sessions += 1;
        }
        if model == "multiple" && is_per_block_multiple(session) {
            row.per_block_multiple += 1;
        }
        // rm-578: any session presenting recorded dollars without
        // their token split poisons the whole row — re-pricing the row
        // would price that session's tokens at catalog rates AND add
        // its recorded dollars back, charging them twice.
        row.split_lost |= session.metrics.upstream_cost_usd > 0.0
            && session.metrics.upstream_priced_input == 0
            && session.metrics.upstream_priced_output == 0
            && session.metrics.upstream_priced_cache_w == 0
            && session.metrics.upstream_priced_cache_r == 0;
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
        // rm-1508: count fallback-cache-priced sessions — the census
        // arm that makes the fallback population visible in the
        // report overview (pattern: fallback_pricing).
        if session
            .metrics
            .disclosure_counters
            .contains_key("cache_rate_fallback")
        {
            coverage.cache_rate_fallback_sessions += 1;
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
            let mixed_tiers = row.tiers.len() > 1;
            let row_tier = row.tiers.iter().next().and_then(|tier| tier.as_deref());
            // rm-520/rm-578: re-price on the stored estimate's basis —
            // `current_cost_estimate` applies the catalog rates only
            // to the tokens the journal did not already price and
            // adds the recorded dollars back at face value,
            // mirroring the lib.rs cost formula. rm-610: the recompute
            // bills the tier the row's sessions ran on; a mixed-tier
            // row cannot carry one rate row.
            let current = if row.split_lost {
                // rm-578: a recorded-cost basis whose token split was
                // lost cannot be re-priced without double-counting
                // the recorded dollars — withhold the current-rate
                // column for the row.
                None
            } else if mixed_tiers {
                None
            } else {
                current_cost_estimate(&model, row_tier, &row.tokens, &row.recorded)
            };
            let pricing_source = if row.split_lost {
                // rm-578: withheld for the same reason as the estimate
                // — the split the re-price needs is gone. Cause-neutral
                // wording: the shape arises both from pre-schema-41
                // cache rows and from cost-only usage entries no cache
                // ever touched.
                "unavailable: recorded-cost token split not reproducible".to_string()
            } else if mixed_tiers {
                "unavailable: mixed service tiers".to_string()
            } else {
                current
                    .as_ref()
                    .map(|(source, _, _)| source.clone())
                    .unwrap_or_else(|| "unavailable: multiple models".to_string())
            };
            let (pricing_status, pricing_note) = if model == "multiple" {
                if row.per_block_multiple == row.sessions {
                    // rm-521: every session in this row was priced per
                    // usage block (rm-438) — the stored total is exact.
                    // Only genuinely aggregated rows keep the
                    // no-exact-price claim below.
                    (
                        "aggregate_estimate",
                        "multiple models, priced per usage block; per-model totals exact",
                    )
                } else {
                    (
                        "aggregate_estimate",
                        "SQLite aggregated multiple model IDs; no single-model exact price applies",
                    )
                }
            } else if mixed_tiers {
                (
                    "aggregate_estimate",
                    "sessions under this model ran on mixed service tiers; each session's cost prices the tier it ran on, so no single-tier rate row applies",
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
            let pricing_note = append_drift_note(
                pricing_note,
                current
                    .as_ref()
                    .map(|(_, _, components)| round4(components.total)),
                row.cost,
                row.recorded.usd,
                !row.split_lost,
            );
            // rm-570 (run e88f2da2 cycle 3, review 9af53c3f F1): name
            // the recorded/estimate-only split on every row that
            // carries vendor-recorded dollars — how many sessions
            // recorded cost, and how much of the stored total is an
            // ingest estimate — so the aggregate basis is disclosed
            // rather than implied ("$0.0700 vendor-recorded" alone
            // would imply the whole stored total is vendor-recorded).
            // Informational only: the comparison mechanics stay
            // rm-520/578's — same-basis re-price when the split
            // survives, withholding when it does not.
            let pricing_note = if row.recorded.usd > 0.0 {
                let estimate_only_sessions = row.sessions - row.recorded_sessions;
                let estimate_only_usd = (row.cost - row.recorded.usd).max(0.0);
                let mut split: Vec<String> = Vec::new();
                if estimate_only_sessions > 0 {
                    split.push(format!("{} estimate-only session(s)", estimate_only_sessions));
                }
                if estimate_only_usd > 0.005 {
                    split.push(format!(
                        "${:.4} of the stored total estimated at ingest",
                        estimate_only_usd
                    ));
                }
                if split.is_empty() {
                    format!(
                        "{pricing_note}; {} of {} session(s) carry ${:.4} vendor-recorded cost",
                        row.recorded_sessions, row.sessions, row.recorded.usd
                    )
                } else {
                    format!(
                        "{pricing_note}; {} of {} session(s) carry ${:.4} vendor-recorded cost ({})",
                        row.recorded_sessions,
                        row.sessions,
                        row.recorded.usd,
                        split.join(" and ")
                    )
                }
            } else {
                pricing_note
            };
            let (rates_per_million_usd, component_cost_usd, estimated_cost_usd) = current
                .map(|(_, rates, components)| {
                    // The current-rate total carries the recorded
                    // dollars at face value (folded into
                    // `components.total` by `current_cost_estimate`)
                    // so it shares the stored estimate's basis
                    // (rm-520/rm-578).
                    let total = round4(components.total);
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
                recorded_cost_usd: if row.recorded.usd > 0.0 {
                    Some(round4(row.recorded.usd))
                } else {
                    None
                },
                pricing_status: pricing_status.to_string(),
                pricing_note,
            }
        })
        .collect::<Vec<_>>();
    // rm-1500 (review b346cf8b fix round): the stored-side basis of the
    // anomaly ratios is itself an aggregate fold — a poisoned session
    // contributes 0.0 here instead of turning the audit totals (and every
    // ratio downstream) into NaN; the session itself stays counted in its
    // model-group row.
    let stored_estimated_cost_usd = round4(
        sessions
            .iter()
            .map(|session| {
                if session.metrics.cost_estimated.is_finite() {
                    session.metrics.cost_estimated
                } else {
                    0.0
                }
            })
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
        clamped_values,
        aborted_sessions,
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
        cache_write_1h: session.metrics.tokens_cache_w_1h,
        cache_read: session.metrics.tokens_cache_r,
        total: total_tokens(session),
    };
    // rm-520/rm-578: re-price on the stored estimate's basis — see
    // `append_drift_note`. `current_cost_estimate` applies catalog
    // rates only to the tokens the journal did not record a cost
    // for; recorded dollars ride the current total at face value.
    // rm-610: the recompute bills the tier the session ran on.
    let recorded = RecordedCostBasis::of(&session.metrics);
    let recorded_usd = session.metrics.upstream_cost_usd;
    let split_reproducible = recorded_usd <= 0.0
        || session.metrics.upstream_priced_input > 0
        || session.metrics.upstream_priced_output > 0
        || session.metrics.upstream_priced_cache_w > 0
        || session.metrics.upstream_priced_cache_w_1h > 0
        || session.metrics.upstream_priced_cache_r > 0;
    // rm-610: a single-model session that mixed service tiers priced
    // each usage block on the tier that produced it (the analyze()
    // widening) stores an exact per-block total, but the per-tier token
    // split does not survive on the metrics — so the current-rate
    // column is withheld with the reason stated, the same honest
    // withholding a mixed-tier row gets, instead of re-pricing every
    // token at the standard row and flagging a false drift against the
    // tier-aware stored total.
    let mixed_tier_session = model != "multiple"
        && session
            .metrics
            .provenance
            .pricing_source
            .contains("mixed service tiers");
    let current = if !split_reproducible {
        // rm-578: same withholding at session level — re-pricing a
        // split-lost recorded-cost basis would price the recorded
        // tokens twice (catalog + face value).
        None
    } else if mixed_tier_session {
        None
    } else {
        current_cost_estimate(
            &model,
            session.metrics.service_tier.as_deref(),
            &tokens,
            &recorded,
        )
    };
    // rm-570 (run e88f2da2 cycle 3): mark vendor-recorded dollars on
    // the audit contract. The comparison mechanics stay rm-520/578's
    // — same-basis re-price when the split survives, withholding when
    // it does not — the marker only discloses the recorded dollars
    // (supersedes the candidate's "never re-price a recorded row"
    // rule, which would also hide genuine catalog drift on the
    // unrecorded half of a mixed session).
    let recorded_basis = recorded_usd > 0.0;
    let (pricing_status, pricing_note) = if model == "multiple" {
        if is_per_block_multiple(session) {
            // rm-521: per-block pricing (rm-438) priced every usage
            // block at its own model — the stored total is exact, and
            // the note must not claim SQLite aggregation.
            (
                "aggregate_estimate",
                "multiple models, priced per usage block; per-model totals exact",
            )
        } else {
            (
                "aggregate_estimate",
                "SQLite aggregated multiple model IDs; no single-model exact price applies",
            )
        }
    } else if mixed_tier_session {
        // rm-610: the session-level mirror of the mixed-tier row rule
        // and of rm-521's per-block disclosure — the stored total is
        // exact per block, and no single rate row reproduces it.
        (
            "aggregate_estimate",
            "single model, mixed service tiers, priced per usage block; per-tier totals exact",
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
    let pricing_note = append_drift_note(
        pricing_note,
        current
            .as_ref()
            .map(|(_, _, components)| round4(components.total)),
        session.metrics.cost_estimated,
        recorded_usd,
        split_reproducible,
    );
    // rm-570 (run e88f2da2 cycle 3): name the recorded dollars on the
    // note so a recorded-cost session's contract is disclosed — they
    // ride the comparison at face value on the same-basis re-price
    // (or the re-price is withheld when the split was lost, which
    // `append_drift_note` already says).
    let pricing_note = if recorded_basis {
        if split_reproducible {
            format!(
                "{pricing_note}; ${:.4} vendor-recorded cost \
                 (rides the comparison at face value)",
                recorded_usd
            )
        } else {
            format!("{pricing_note}; ${:.4} vendor-recorded cost", recorded_usd)
        }
    } else {
        pricing_note
    };
    let pricing_source = if !split_reproducible {
        // rm-578: cause-neutral for the same reason as the row-level
        // string — the cache is one cause of the shape, not the only
        // one (cost-only usage entries never had a split to lose).
        "unavailable: recorded-cost token split not reproducible".to_string()
    } else {
        current
            .as_ref()
            .map(|(source, _, _)| source.clone())
            .unwrap_or_else(|| "unavailable: multiple models".to_string())
    };
    let (rates_per_million_usd, component_cost_usd, estimated_cost_usd) = current
        .map(|(_, rates, components)| {
            // rm-520: `components.total` already carries the recorded
            // dollars at face value, so the current-rate total shares
            // the stored estimate's basis.
            let total = round4(components.total);
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
        recorded_cost_usd: if recorded_basis {
            Some(round4(session.metrics.upstream_cost_usd))
        } else {
            None
        },
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
                    format!("session={}", guard_formula_cell(&session.name)),
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
                vec![format!("session={}", guard_formula_cell(&session.name)), format!("failed_calls={}", session.metrics.tool_calls_fail)],
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
                vec![format!("session={}", guard_formula_cell(&session.name)), format!("utilization={:.1}%", context.utilization_pct)],
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
                    format!("session={}", guard_formula_cell(&session.name)),
                    format!(
                        "tool={} p95={:.1}s unmatched={}",
                        guard_formula_cell(&slow.tool_name),
                        slow.p95_sec,
                        slow.unmatched
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
        // rm-1509 (cycle 2, run b01c2b6a): the OpenCode v2
        // `session.summary_diffs` per-file list is writer-reported
        // delivery evidence — surfaced as counts and paths (no patch
        // bodies). When the per-file sums disagree with the integer
        // `summary_*` columns on the same row, the divergence is
        // disclosed on its own line, never silently reconciled into
        // either side.
        if let Some(diffs) = session
            .metrics
            .summary_diffs
            .as_ref()
            .filter(|diffs| !diffs.is_empty())
        {
            // Review fix F1 (attempt 03ec687a): the evidence line
            // sums SATURATE, exactly like `summary_diffs_divergence`
            // in sqlite_sessions.rs — a hostile payload of i64::MAX
            // per-file entries must disclose pinned maxima, never
            // overflow the disclosure arithmetic (the plain `.sum()`
            // here panicked debug builds and wrapped negative in
            // release). Keeping both sites on the same saturating
            // fold also means the evidence line and the divergence
            // disclosure can never disagree with each other.
            let (additions, deletions) =
                diffs
                    .iter()
                    .fold((0i64, 0i64), |(additions, deletions), diff| {
                        (
                            additions.saturating_add(diff.additions),
                            deletions.saturating_add(diff.deletions),
                        )
                    });
            evidence.push(format!(
                "writer-reported per-file diff summary: {} files (+{}/-{})",
                diffs.len(),
                additions,
                deletions
            ));
        }
        if let Some(divergence) = &session.metrics.summary_diffs_divergence {
            evidence.push(format!(
                "per-file diff sums diverge from the row's summary columns: +{} vs +{} additions, -{} vs -{} deletions",
                divergence.per_file_additions,
                divergence.summary_additions,
                divergence.per_file_deletions,
                divergence.summary_deletions
            ));
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
                "Read-only local Git heuristic: commit timestamps are matched to session start/end with a 2-minute lead and 5-minute tail. It does not prove authorship, merge-to-main, or business value. The local git log probe is bounded at {} seconds per repository root and drains at most {} KiB of log output; a probe that overruns either bound is treated as unavailable and its root degrades to the tool-authority heuristic.",
                GIT_PROBE_TIMEOUT.as_secs(),
                GIT_PROBE_DRAIN_MAX_BYTES / 1024
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

/// rm-731 drain arm (run 80e8beb6 cycle 1, batch "bounded inputs,
/// honest docs"): the probe's helper thread used to `read_to_end`
/// the WHOLE `git log --all` stream — the timeout bounded the wait,
/// never the buffer, so a hostile or pathological repository (a log
/// with millions of lines) grew the drain without bound. The drain
/// now stops at [`GIT_PROBE_DRAIN_MAX_BYTES`] and an overrun degrades
/// the root to the documented heuristic (a truncated commit census
/// would silently bias the recency heuristic, so overflow means
/// unavailable, never partial). ~6k commits fit under the cap — one
/// timestamp per `git log --format=%ct` line — which covers every
/// real project root; the bound is disclosed in the report's
/// `methodology` string alongside the timeout.
const GIT_PROBE_DRAIN_MAX_BYTES: usize = 64 * 1024;

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
    // rm-731 drain arm: the drain thread is bounded twice — read at
    // most GIT_PROBE_DRAIN_MAX_BYTES + 1 bytes (the extra byte detects
    // overflow in one read; bytes past the cap die with the pipe), and
    // the recv below only gets the remainder of the probe budget.
    let (drain_tx, drain_rx) = std::sync::mpsc::channel::<(Vec<u8>, bool)>();
    // The handle is deliberately not joined: the reader exits once its
    // pipe EOFs (or with the process), and the bounded recv below is the
    // only wait this probe performs on it.
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut overflow = false;
        if let Some(stream) = stdout.as_mut() {
            let mut capped = stream.take(GIT_PROBE_DRAIN_MAX_BYTES as u64 + 1);
            let _ = capped.read_to_end(&mut bytes);
            overflow = bytes.len() > GIT_PROBE_DRAIN_MAX_BYTES;
        }
        let _ = drain_tx.send((bytes, overflow));
    });
    let started = std::time::Instant::now();
    let status = wait_child_bounded(&mut child, GIT_PROBE_TIMEOUT)?;
    if !status.success() {
        return None;
    }
    // rm-731: `wait_child_bounded` returns the moment the child exits,
    // but a forked helper (git-remote-https, a credential helper, an
    // exec'd hook) can inherit the stdout write-end and keep
    // `read_to_end` from ever seeing EOF — the old unbounded join wedged
    // the whole report for the helper's lifetime. The drain gets only
    // what remains of the probe's ten-second budget and degrades to the
    // documented heuristic instead of hanging.
    let remaining = GIT_PROBE_TIMEOUT.saturating_sub(started.elapsed());
    let (stdout, overflow) = drain_rx.recv_timeout(remaining).ok()?;
    if overflow {
        // A log that overran the drain cap cannot yield a complete
        // commit census — degrade to the documented heuristic rather
        // than price a truncated one (the cap is disclosed in the
        // report's methodology string).
        return None;
    }
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
    // rm-1500 (review b346cf8b fix round): context aggregates are aggregate
    // folds too — skip-while-counting like the lib.rs arms: a non-finite
    // session still counts in aggregate.sessions (and the overview's
    // non_finite_costs) but contributes 0.0 cost, keeping
    // output_cost_per_million_tokens finite on the totals row and every
    // project row.
    if session.metrics.cost_estimated.is_finite() {
        aggregate.cost += session.metrics.cost_estimated;
    }
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
            branch: String::new(),
            org: String::new(),
            org_email: String::new(),
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
    fn audit_totals_skip_non_finite_cost() {
        // rm-1500 (review b346cf8b fix round): the audit-level stored-side
        // basis — CostAudit.stored_estimated_cost_usd / total_estimated_cost —
        // folded cost_estimated with a bare `.sum()`, so one poisoned session
        // NaN'd the totals every per-model anomaly ratio is measured against,
        // even though the per-model rows themselves were already gated.
        let mut a = session("finite");
        a.metrics.model_used = "gpt-x".to_string();
        a.metrics.cost_estimated = 1.0;
        let mut b = session("poisoned");
        b.metrics.model_used = "gpt-x".to_string();
        b.metrics.cost_estimated = f64::NAN;
        let audit = cost_audit(&[a, b]);
        assert!(
            audit.stored_estimated_cost_usd.is_finite(),
            "NaN poisoned the audit stored-side total: {}",
            audit.stored_estimated_cost_usd
        );
        assert!(
            audit.total_estimated_cost.is_finite(),
            "NaN poisoned the audit total: {}",
            audit.total_estimated_cost
        );
        assert_eq!(
            audit.stored_estimated_cost_usd, 1.0,
            "the finite member is folded, the NaN is not"
        );
    }

    #[test]
    fn context_aggregate_folds_skip_non_finite_cost() {
        // rm-1500 (review b346cf8b fix round): add_context_session folded
        // cost_estimated into ContextAggregate.cost ungated, so
        // output_cost_per_million_tokens — the context-trends lane's only
        // cost surface — went NaN on the totals row and every project row.
        let mut a = session("finite");
        a.metrics.cost_estimated = 1.0;
        let mut b = session("poisoned");
        b.metrics.cost_estimated = f64::NAN;
        let trend = context_trends(&[a, b]);
        assert_eq!(trend.totals.sessions, 2, "both sessions stay counted");
        assert!(
            trend.totals.output_cost_per_million_tokens.is_finite(),
            "NaN poisoned the context-trend total: {}",
            trend.totals.output_cost_per_million_tokens
        );
        for project in &trend.projects {
            assert!(
                project.output_cost_per_million_tokens.is_finite(),
                "NaN poisoned the {} context row: {}",
                project.project,
                project.output_cost_per_million_tokens
            );
        }
    }

    #[test]
    fn stored_side_group_fold_skips_non_finite_cost() {
        // rm-1500 (cycle 5 implement + review b346cf8b fix round): TWO
        // sessions in the SAME model group, one poisoned — pins the
        // implement-phase row gate (the cost_by_model fold) with a MIXED
        // group, which the implement pin (a lone NaN in its own row) never
        // exercised: only a sibling makes the fold actually sum against a
        // non-finite member.
        let mut a = session("finite");
        a.metrics.model_used = "gpt-x".to_string();
        a.metrics.cost_estimated = 1.0;
        let mut b = session("poisoned");
        b.metrics.model_used = "gpt-x".to_string();
        b.metrics.cost_estimated = f64::NAN;
        let audit = cost_audit(&[a, b]);
        assert!(
            !audit.by_provider_model.is_empty(),
            "the gpt-x group must be represented"
        );
        for m in &audit.by_provider_model {
            assert!(
                m.stored_estimated_cost_usd.is_finite(),
                "NaN poisoned the stored-side group fold: {}",
                m.stored_estimated_cost_usd
            );
        }
    }

    #[test]
    fn cost_by_model_folds_skip_non_finite_cost() {
        // rm-1500 (cycle 5, run e4209f0c assess F2): cost_by_model folded
        // cost_estimated without the finiteness gate the overview arms
        // carry — one NaN journal poisoned the CSV models lane.
        let mut poisoned = session("nan-cost");
        poisoned.metrics.cost_estimated = f64::NAN;
        poisoned.metrics.model_used = "poison-model".to_string();
        let audit = cost_audit(&[poisoned]);
        assert!(
            !audit.by_provider_model.is_empty(),
            "the poisoned session's model row must still exist (sessions counted)"
        );
        for m in &audit.by_provider_model {
            assert!(
                m.stored_estimated_cost_usd.is_finite(),
                "non-finite cost leaked into the provider/model cost fold"
            );
        }
    }

    #[test]
    fn audit_reprices_recorded_cost_sessions_on_their_own_basis() {
        // rm-578: the assess PoC shape (pi v3 usage entry carrying
        // usage.cost.total 0.0075 on 31.5k sonnet-4 tokens): the
        // stored total rests on a recorded-cost basis, so the
        // current-rate column must compare on that basis — catalog
        // rates on the unrecorded tokens (none here) plus the
        // recorded dollars — instead of re-pricing every token and
        // reporting legitimate basis differences as rate drift.
        let mut session = session("recorded");
        session.metrics.model_used = "claude-sonnet-4-20250514".to_string();
        session.metrics.tokens_input = 500;
        session.metrics.tokens_output = 1_000;
        session.metrics.tokens_cache_w = 5_000;
        session.metrics.tokens_cache_r = 25_000;
        session.metrics.upstream_cost_usd = 0.0075;
        session.metrics.upstream_priced_input = 500;
        session.metrics.upstream_priced_output = 1_000;
        session.metrics.upstream_priced_cache_w = 5_000;
        session.metrics.upstream_priced_cache_r = 25_000;
        session.metrics.cost_estimated = 0.0075;
        let audit = session_cost_audit(&session);
        assert!(!audit.pricing_note.contains("current rates recalculate"));
        assert!((audit.estimated_cost_usd.expect("same-basis re-price") - 0.0075).abs() < 1e-6);
        let report = cost_audit(&[session]);
        let row = &report.by_provider_model[0];
        assert!(!row.pricing_note.contains("current rates recalculate"));
        assert!((row.estimated_cost_usd.expect("row re-price") - 0.0075).abs() < 1e-6);
    }

    #[test]
    fn audit_still_reports_genuine_catalog_rate_drift() {
        // No recorded dollars: a stored total priced under different
        // rates must still re-price to a different total and say so
        // (the note's honest meaning survives rm-578).
        let mut session = session("drift");
        session.metrics.model_used = "claude-sonnet-4-20250514".to_string();
        session.metrics.tokens_input = 1_000_000;
        session.metrics.cost_estimated = 4.5; // catalog says 3.0
        let audit = session_cost_audit(&session);
        assert!(audit.pricing_note.contains("current rates recalculate"));
    }

    #[test]
    fn audit_stays_silent_when_the_recorded_split_was_lost() {
        // Pre-schema-41 cache-row shape: the recorded dollars
        // survive (`upstream_cost_usd` round-trips through
        // `GoMetrics`) but the token split does not — since rm-520
        // the split round-trips too, so the zero-split shape is
        // constructed directly here; a cost-only usage entry presents
        // the same shape with no cache involved. The basis cannot be
        // reproduced and must not be reported as drift.
        let mut session = session("cached");
        session.metrics.model_used = "claude-sonnet-4-20250514".to_string();
        session.metrics.tokens_input = 500;
        session.metrics.tokens_output = 1_000;
        session.metrics.tokens_cache_w = 5_000;
        session.metrics.tokens_cache_r = 25_000;
        session.metrics.upstream_cost_usd = 0.0075;
        session.metrics.cost_estimated = 0.0075;
        let audit = session_cost_audit(&session);
        assert!(!audit.pricing_note.contains("current rates recalculate"));
        assert!(audit.pricing_note.contains("no same-basis re-price"));
        assert_eq!(audit.estimated_cost_usd, None);
    }

    #[test]
    fn cost_audit_recorded_row_reprices_on_the_recorded_basis() {
        // rm-570 (run e88f2da2 cycle 3) red-first, re-anchored on the
        // landed rm-520/578 contract: the journal recorded this
        // session's cost (usage cost.total → upstream_cost_usd), so
        // pricing the whole token breakdown at current catalog rates
        // and alarming about the difference is phantom drift (the
        // 0.07 recorded / ~0.13 catalog PoC). The landed fix compares
        // on the same basis — catalog rates only on unrecorded tokens
        // (none here) plus the recorded dollars at face value — and
        // the row discloses its recorded dollars via
        // recorded_cost_usd and the note's recorded split.
        let mut recorded = session("recorded");
        recorded.metrics.model_used = "claude-sonnet-4-5".to_string();
        recorded.metrics.tokens_input = 38_500;
        recorded.metrics.tokens_output = 950;
        recorded.metrics.cost_estimated = 0.07;
        recorded.metrics.upstream_cost_usd = 0.07;
        recorded.metrics.upstream_priced_input = 38_500;
        recorded.metrics.upstream_priced_output = 950;
        let audit = cost_audit(&[recorded]);
        let row = &audit.by_provider_model[0];
        assert!((row.stored_estimated_cost_usd - 0.07).abs() < 1e-9);
        assert!((row.recorded_cost_usd.expect("recorded cost disclosed") - 0.07).abs() < 1e-9);
        let current = row.estimated_cost_usd.expect("comparison present");
        assert!(
            (current - 0.07).abs() < 1e-4,
            "recorded dollars ride the comparison at face value, not a full catalog re-price: {current}"
        );
        assert!(
            row.pricing_note
                .contains("1 of 1 session(s) carry $0.0700 vendor-recorded cost"),
            "the note names the recorded dollars: {}",
            row.pricing_note
        );
        assert!(
            !row.pricing_note.contains("recalculate a different total"),
            "no phantom drift on a recorded-basis row: {}",
            row.pricing_note
        );
        let top = audit.current_estimated_cost_usd.expect("sum present");
        assert!((top - 0.07).abs() < 1e-4, "top level agrees: {top}");
    }

    #[test]
    fn cost_audit_estimate_only_rows_still_reprice_at_current_rates() {
        // rm-570 regression arm: estimate-only sessions keep the
        // current-rates comparison and the drift note.
        let mut plain = session("estimate-only");
        plain.metrics.model_used = "claude-sonnet-4-5".to_string();
        plain.metrics.tokens_input = 38_500;
        plain.metrics.tokens_output = 950;
        plain.metrics.cost_estimated = 0.05;
        let audit = cost_audit(&[plain]);
        let row = &audit.by_provider_model[0];
        let current = row.estimated_cost_usd.expect("re-priced");
        assert!(
            (current - 0.05).abs() > 0.01,
            "estimate-only keeps the catalog comparison: {current}"
        );
        assert!(row.pricing_note.contains("recalculate a different total"));
        assert!(
            row.recorded_cost_usd.is_none(),
            "no recorded dollars, no recorded marker"
        );
    }

    #[test]
    fn cost_audit_mixed_row_names_the_split_and_withholds_the_reprice() {
        // rm-570 mixed-row arm (review 9af53c3f F1), re-anchored on
        // the landed contract: one recorded session ($0.07, its
        // usage entry carried no token classes — a split-lost
        // recorded basis, rm-578) + one estimate-only session on the
        // same (provider, model) row. The landed rule withholds the
        // current-rate column for the whole row (re-pricing would
        // double-count the recorded dollars) — the phantom-drift
        // alarm rm-570 chased is absent the honest way — and the
        // note must NAME the split so the semantics are disclosed
        // rather than implied: how many sessions carry recorded
        // dollars and how much of the stored total is an ingest
        // estimate. The live shape: the parser drops pi
        // message-inline usage cost, manufacturing estimate-only
        // sessions that co-locate with recorded ones under the same
        // provider+model.
        let mut recorded = session("mixed-recorded");
        recorded.metrics.model_used = "claude-sonnet-4-5".to_string();
        recorded.metrics.tokens_input = 38_500;
        recorded.metrics.tokens_output = 950;
        recorded.metrics.cost_estimated = 0.07;
        recorded.metrics.upstream_cost_usd = 0.07;
        let mut estimate_only = session("mixed-estimate-only");
        estimate_only.metrics.model_used = "claude-sonnet-4-5".to_string();
        estimate_only.metrics.tokens_input = 38_500;
        estimate_only.metrics.tokens_output = 950;
        estimate_only.metrics.cost_estimated = 0.05;
        let audit = cost_audit(&[recorded, estimate_only]);
        assert_eq!(audit.by_provider_model.len(), 1, "same row");
        let row = &audit.by_provider_model[0];
        assert_eq!(row.sessions, 2);
        assert!((row.stored_estimated_cost_usd - 0.12).abs() < 1e-9);
        assert!((row.recorded_cost_usd.expect("recorded cost") - 0.07).abs() < 1e-9);
        // rm-578: the recorded member's token split was lost, so the
        // row's current-rate column is withheld — a catalog re-price
        // of the estimate-only member would double-count the
        // recorded dollars (which `cost_audit_estimate_only_rows_`
        // `still_reprice_at_current_rates` proves happens on
        // estimate-ONLY rows).
        assert_eq!(
            row.estimated_cost_usd, None,
            "split-lost recorded member withholds the row re-price"
        );
        assert_eq!(
            row.pricing_source,
            "unavailable: recorded-cost token split not reproducible"
        );
        assert!(
            row.pricing_note.contains("1 of 2 session(s)"),
            "the note names the session split: {}",
            row.pricing_note
        );
        assert!(
            row.pricing_note.contains("1 estimate-only session(s)"),
            "estimate-only sessions are disclosed: {}",
            row.pricing_note
        );
        assert!(
            row.pricing_note
                .contains("$0.0500 of the stored total estimated at ingest"),
            "estimate-at-ingest dollars are disclosed: {}",
            row.pricing_note
        );
        assert!(
            !row.pricing_note.contains("recalculate a different total"),
            "no drift note on a withheld row: {}",
            row.pricing_note
        );
        assert_eq!(
            audit.current_estimated_cost_usd, None,
            "top level stays silent with a withheld row"
        );
    }

    #[test]
    fn cost_audit_recorded_row_with_estimate_at_ingest_dollars_names_the_split() {
        // rm-570 mixed shape as it manifests live (review 9af53c3f
        // F1's mixed-row-poc.jsonl), re-anchored on the landed
        // contract: the pi journal parses to ONE session carrying a
        // $0.07 vendor-recorded usage entry (no token classes —
        // split lost, rm-578) AND ~$13.50 of tokens with no recorded
        // cost, so the recorded dollars and the estimate-at-ingest
        // dollars share a single (provider, model) row and a single
        // stored total. The row's re-price is withheld, and the note
        // must name the DOLLAR split too — "1 of 1 session(s) carry
        // $0.0700" alone would imply the whole $13.57 stored total
        // is vendor-recorded.
        let mut mixed = session("recorded-plus-estimate");
        mixed.metrics.model_used = "claude-sonnet-4-5".to_string();
        mixed.metrics.tokens_input = 4_038_500;
        mixed.metrics.tokens_output = 100_950;
        mixed.metrics.cost_estimated = 13.57;
        mixed.metrics.upstream_cost_usd = 0.07;
        let audit = cost_audit(&[mixed]);
        assert_eq!(audit.by_provider_model.len(), 1);
        let row = &audit.by_provider_model[0];
        assert_eq!(row.sessions, 1);
        assert_eq!(row.estimated_cost_usd, None, "split lost: withheld");
        assert!(
            row.pricing_note.contains(
                "no same-basis re-price available — the recorded-cost token split is unavailable"
            ),
            "withholding is explained: {}",
            row.pricing_note
        );
        assert!(
            row.pricing_note.contains(
                "1 of 1 session(s) carry $0.0700 vendor-recorded cost ($13.5000 of the stored total estimated at ingest)"
            ),
            "the note names the dollar split: {}",
            row.pricing_note
        );
        assert!(
            !row.pricing_note.contains("recalculate a different total"),
            "no drift note on a recorded-basis row: {}",
            row.pricing_note
        );
    }

    #[test]
    fn session_cost_audit_recorded_cost_rides_the_comparison_at_face_value() {
        // rm-570 session-level arm, re-anchored on the landed
        // contract: same recorded-cost rule for --audit <session> —
        // the recorded dollars ride the current-rate comparison at
        // face value (rm-520 basis), are disclosed on the note and
        // via recorded_cost_usd, and never false-trip the drift note.
        let mut recorded = session("recorded-session");
        recorded.metrics.model_used = "claude-sonnet-4-5".to_string();
        recorded.metrics.tokens_input = 38_500;
        recorded.metrics.tokens_output = 950;
        recorded.metrics.cost_estimated = 0.07;
        recorded.metrics.upstream_cost_usd = 0.07;
        recorded.metrics.upstream_priced_input = 38_500;
        recorded.metrics.upstream_priced_output = 950;
        let audit = session_cost_audit(&recorded);
        let current = audit.estimated_cost_usd.expect("comparison present");
        assert!(
            (current - 0.07).abs() < 1e-4,
            "recorded dollars ride the comparison at face value: {current}"
        );
        assert!(
            audit
                .pricing_note
                .contains("$0.0700 vendor-recorded cost (rides the comparison at face value)"),
            "the note discloses the recorded dollars: {}",
            audit.pricing_note
        );
        assert!(
            !audit.pricing_note.contains("recalculate a different total"),
            "no phantom drift on a recorded-basis session: {}",
            audit.pricing_note
        );
        assert!((audit.recorded_cost_usd.expect("recorded marker") - 0.07).abs() < 1e-9);
    }

    #[test]
    fn audit_counts_cache_rate_fallback_disclosures_as_coverage() {
        // rm-1508 (run 5c144613 cycle 4): a session whose pricing fired
        // the absent-rate fallback carries the disclosure counter, and
        // the coverage census must surface it — once per session, not
        // per application, and only for that key. The census renders
        // as the `; cache-rate fallback: N` report arm, and only when
        // nonzero, so fallback-free audits stay byte-identical.
        let mut fired_twice = session("fallback-a");
        fired_twice.metrics.model_used = "claude-sonnet-4-20250514".to_string();
        fired_twice
            .metrics
            .disclosure_counters
            .insert("cache_rate_fallback".to_string(), 2);
        let mut fired_once_other_key_too = session("fallback-b");
        fired_once_other_key_too.metrics.model_used = "claude-sonnet-4-20250514".to_string();
        fired_once_other_key_too
            .metrics
            .disclosure_counters
            .insert("cache_rate_fallback".to_string(), 1);
        fired_once_other_key_too
            .metrics
            .disclosure_counters
            .insert("unrelated_disclosure".to_string(), 7);
        let mut clean = session("clean");
        clean.metrics.model_used = "claude-sonnet-4-20250514".to_string();
        let audit = cost_audit(&[fired_twice, fired_once_other_key_too, clean]);
        assert_eq!(
            audit.pricing_coverage.cache_rate_fallback_sessions, 2,
            "one per session that fired the fallback, not per application"
        );
        let audit_json = serde_json::to_value(&audit).expect("cost audit json");
        assert_eq!(
            audit_json["pricing_coverage"]["cache_rate_fallback_sessions"],
            serde_json::json!(2),
            "the census rides the serialized audit the governance/report surfaces use"
        );
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
    fn audit_recomputes_recorded_cost_sessions_on_the_rm436_basis() {
        // rm-520: a recorded-cost session's stored estimate is
        // catalog × unpriced tokens + recorded USD (rm-436). The
        // audit's "current" recompute must use the same basis — the
        // pre-fix catalog-only total reported $1.25 against a recorded
        // $5.00 and false-tripped the drift note (assess PoC F1:
        // /tmp/at-ac14/corpus2/usage-entry.jsonl, 1M gpt-5 input
        // tokens at $1.25/M catalog, $5.00 recorded).
        let rate = pricing::lookup_price("gpt-5");
        assert!((rate.input - 1.25).abs() < 1e-9, "golden premise");
        let mut recorded = session("recorded");
        recorded.metrics.model_used = "gpt-5".to_string();
        recorded.metrics.tokens_input = 1_000_000;
        recorded.metrics.tokens_output = 0;
        recorded.metrics.upstream_priced_input = 1_000_000;
        recorded.metrics.upstream_cost_usd = 5.0;
        recorded.metrics.cost_estimated = 5.0;
        let audit = session_cost_audit(&recorded);
        assert_eq!(audit.stored_estimated_cost_usd, 5.0);
        let current = audit.estimated_cost_usd.expect("current computed");
        assert!((current - 5.0).abs() < 0.0001, "rm-436 basis: {current}");
        assert!(
            !audit
                .pricing_note
                .contains("current rates recalculate a different total"),
            "recorded-cost session must not false-trip: {}",
            audit.pricing_note
        );

        // A genuinely stale stored estimate (no recorded cost) still
        // trips — the drift mechanism survives the fix.
        let mut stale = session("stale");
        stale.metrics.model_used = "gpt-5".to_string();
        stale.metrics.tokens_input = 1_000_000;
        stale.metrics.tokens_output = 0;
        stale.metrics.cost_estimated = 5.0;
        let audit = session_cost_audit(&stale);
        let current = audit.estimated_cost_usd.expect("current computed");
        assert!((current - 1.25).abs() < 0.0001, "catalog-only: {current}");
        assert!(
            audit
                .pricing_note
                .contains("current rates recalculate a different total"),
            "stale catalog comparison must keep tripping: {}",
            audit.pricing_note
        );

        // Corpus rows inherit the corrected basis: two clean
        // recorded-cost sessions in one row produce no trip...
        let clean = cost_audit(&[recorded.clone(), recorded.clone()]);
        assert_eq!(clean.by_provider_model.len(), 1);
        let row = &clean.by_provider_model[0];
        assert_eq!(row.sessions, 2);
        let current = row.estimated_cost_usd.expect("row current computed");
        assert!((current - 10.0).abs() < 0.0001, "2 × recorded: {current}");
        assert!(!row.pricing_note.contains("current rates recalculate"));

        // ...while a row holding one genuinely stale session still
        // reports the difference — the trip here is truthful.
        let mixed = cost_audit(&[recorded, stale]);
        let row = &mixed.by_provider_model[0];
        let current = row.estimated_cost_usd.expect("row current computed");
        assert!(
            (current - 6.25).abs() < 0.0001,
            "1.25 catalog + 5.0 recorded: {current}"
        );
        assert!(row.pricing_note.contains("current rates recalculate"));
    }

    #[test]
    fn audit_labels_per_block_multi_model_rows_by_their_real_mechanism() {
        // rm-521: "multiple" has two producers — per-block pricing
        // (rm-438, exact) and SQLite aggregation (no per-model split).
        // The note must claim the mechanism that produced the number.
        let mut per_block = session("per-block");
        per_block.metrics.model_used = "multiple".to_string();
        per_block.metrics.source_tool = "pi".to_string();
        per_block.metrics.provenance.pricing_source =
            "multiple models (priced per usage block) + recorded cost".to_string();
        let audit = session_cost_audit(&per_block);
        assert_eq!(
            audit.pricing_note,
            "multiple models, priced per usage block; per-model totals exact"
        );
        assert_eq!(audit.pricing_status, "aggregate_estimate");
        assert!(audit.rates_per_million_usd.is_none());

        // The SQLite boundary is unchanged (rm-521 keeps the legacy
        // note where aggregation is genuinely what happened).
        let mut sqlite = session("aggregate");
        sqlite.metrics.model_used = "multiple".to_string();
        sqlite.metrics.source_tool = "opencode_db".to_string();
        sqlite.metrics.provenance.pricing_source = "SQLite aggregate: multiple models".to_string();
        let audit = session_cost_audit(&sqlite);
        assert_eq!(
            audit.pricing_note,
            "SQLite aggregated multiple model IDs; no single-model exact price applies"
        );

        // Corpus arm: a row whose every session is per-block-priced
        // gets the exact label; a mixed row stays conservative.
        let rows = cost_audit(&[per_block.clone(), per_block.clone()]);
        assert_eq!(rows.by_provider_model.len(), 1);
        assert_eq!(
            rows.by_provider_model[0].pricing_note,
            "multiple models, priced per usage block; per-model totals exact"
        );
        let mut mixed = per_block.clone();
        mixed.name = "mixed".to_string();
        mixed.metrics.provenance.pricing_source = "SQLite aggregate: multiple models".to_string();
        let rows = cost_audit(&[per_block, mixed]);
        assert_eq!(rows.by_provider_model.len(), 1);
        assert_eq!(
            rows.by_provider_model[0].pricing_note,
            "SQLite aggregated multiple model IDs; no single-model exact price applies"
        );
    }

    #[test]
    fn audit_reprices_the_service_tier_the_sessions_ran_on() {
        // rm-610: the "current rates" recompute must be tier-aware. The
        // live PoC shape: a codex flex turn of 1M in / 1M out on gpt-5.6
        // costs $12 on the flex row — an untiered recompute claimed $24
        // "today" and flagged a false drift against the stored $12.
        let mut flex = session("flex");
        flex.metrics.model_used = "gpt-5.6".to_string();
        flex.metrics.source_tool = "codex_cli".to_string();
        flex.metrics.tokens_input = 1_000_000;
        flex.metrics.tokens_output = 1_000_000;
        flex.metrics.cost_estimated = 12.0;
        flex.metrics.service_tier = Some("flex".to_string());

        let single = session_cost_audit(&flex);
        assert_eq!(single.estimated_cost_usd, Some(12.0));
        assert_eq!(single.stored_estimated_cost_usd, 12.0);
        assert!(single.pricing_source.contains("service_tier: flex"));
        assert!(
            !single
                .pricing_note
                .contains("recalculate a different total"),
            "a tier-aware recompute must not flag drift against the tier-priced store: {}",
            single.pricing_note
        );

        // One model, two tiers under one (provider, model) row: no
        // single rate row applies — disclosed, not silently standard.
        let mut standard = flex.clone();
        standard.metrics.service_tier = Some("default".to_string());
        standard.metrics.cost_estimated = 24.0;
        let audit = cost_audit(&[flex.clone(), standard]);
        let row = &audit.by_provider_model[0];
        assert_eq!(row.pricing_status, "aggregate_estimate");
        assert_eq!(row.pricing_source, "unavailable: mixed service tiers");
        assert!(row.estimated_cost_usd.is_none());
        assert!(row.rates_per_million_usd.is_none());
        assert_eq!(audit.stored_estimated_cost_usd, 36.0);
        assert!(audit.current_estimated_cost_usd.is_none());

        // All sessions on one tier: the row re-prices tier-aware and
        // agrees with the stored total.
        let audit = cost_audit(&[flex]);
        let row = &audit.by_provider_model[0];
        assert_eq!(row.estimated_cost_usd, Some(12.0));
        assert_eq!(audit.current_estimated_cost_usd, Some(12.0));
        assert!(row.pricing_source.contains("service_tier: flex"));
    }

    #[test]
    fn session_audit_withholds_current_rates_for_mixed_tier_sessions() {
        // rm-610: a single-model session that mixed tiers priced per
        // usage block (the analyze() widening) stores an exact
        // per-block total ($12 flex + $24 standard = $36 on 2M/2M
        // gpt-5.6); recomputing it at the standard row would claim $48
        // "today" and flag a false drift, so the current-rate column
        // is withheld with the reason stated — the session-level
        // mirror of the mixed-tier row rule.
        let mut mixed = session("mixed");
        mixed.metrics.model_used = "gpt-5.6".to_string();
        mixed.metrics.source_tool = "codex_cli".to_string();
        mixed.metrics.tokens_input = 2_000_000;
        mixed.metrics.tokens_output = 2_000_000;
        mixed.metrics.cost_estimated = 36.0;
        mixed.metrics.service_tier = None; // tiers mixed: no single tier
        mixed.metrics.provenance.pricing_source =
            "mixed service tiers (priced per usage block)".to_string();

        let audit = session_cost_audit(&mixed);
        assert_eq!(audit.stored_estimated_cost_usd, 36.0);
        assert_eq!(
            audit.estimated_cost_usd, None,
            "no single rate row reproduces a per-block mixed-tier total"
        );
        assert!(audit.rates_per_million_usd.is_none());
        assert_eq!(audit.pricing_status, "aggregate_estimate");
        assert!(
            audit.pricing_note.contains("mixed service tiers"),
            "the note must name the tier mix: {}",
            audit.pricing_note
        );
        assert!(
            !audit.pricing_note.contains("recalculate a different total"),
            "a withheld recompute must not claim drift: {}",
            audit.pricing_note
        );
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
    fn delivery_evidence_saturates_hostile_per_file_sums() {
        // Review fix F1 (attempt 03ec687a): the per-file evidence sums
        // must SATURATE. Before the fix this exact shape panicked debug
        // builds ('attempt to add with overflow', rc 101 — the live CLI
        // PoC planted two i64::MAX additions) and wrapped to -2 in
        // release, contradicting the CHANGELOG's saturation claim. The
        // pinned expectation: both maxima disclose as i64::MAX and the
        // deletions sum normally alongside them.
        let mut value = session("hostile");
        value.metrics.summary_diffs = Some(vec![
            crate::SessionFileDiff {
                file: Some("src/a.rs".to_string()),
                additions: i64::MAX,
                deletions: 1,
                status: None,
            },
            crate::SessionFileDiff {
                file: Some("src/b.rs".to_string()),
                additions: i64::MAX,
                deletions: 1,
                status: None,
            },
        ]);
        let report = delivery_evidence(&[value]);
        let line = report.sessions[0]
            .evidence
            .iter()
            .find(|line| line.contains("writer-reported per-file diff summary"))
            .expect("per-file summary evidence line");
        assert_eq!(
            line.as_str(),
            format!(
                "writer-reported per-file diff summary: 2 files (+{}/-2)",
                i64::MAX
            ),
            "saturating sums must pin at i64::MAX, never overflow or wrap"
        );
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
    #[cfg(unix)]
    fn git_commits_probe_survives_a_helper_holding_the_pipe_past_exit() {
        let _env_lock = crate::test_env::lock_env();
        // rm-731: `wait_child_bounded` returns the moment the child
        // exits, but a forked helper inheriting the stdout write-end used
        // to wedge the probe's unbounded drain for the helper's whole
        // lifetime (the upstream.rs fast-exit arm's sibling — same
        // assess PoC family). The drain now gets only the remainder of
        // the ten-second probe budget and the report degrades to its
        // documented heuristic instead of hanging. The shim delegates
        // every other invocation to the real git so concurrent probes
        // in this test binary are unaffected.
        use std::os::unix::fs::PermissionsExt;
        let marker = "agenttrace-gov-fast-exit-holder";
        let root = std::env::temp_dir().join(marker);
        let shim_dir = root.join("shim");
        std::fs::create_dir_all(&shim_dir).expect("create shim dir");
        std::fs::write(
            shim_dir.join("git"),
            format!(
                "#!/bin/sh\ncase \"$2\" in\n  *{marker}*) sleep 60 &\n    exec true ;;\n  *) exec /usr/bin/git \"$@\" ;;\nesac\n"
            ),
        )
        .expect("write git shim");
        std::fs::set_permissions(shim_dir.join("git"), std::fs::Permissions::from_mode(0o755))
            .expect("chmod git shim");
        let previous_path = std::env::var_os("PATH");
        std::env::set_var(
            "PATH",
            format!("{}:{}", shim_dir.display(), std::env::var("PATH").unwrap()),
        );
        let started = std::time::Instant::now();
        let commits = git_commits(root.to_str().expect("utf-8 root"));
        if let Some(path) = previous_path {
            std::env::set_var("PATH", path);
        }
        assert!(
            commits.is_none(),
            "the shim prints nothing and the drained bytes are unrecoverable in budget: the probe must degrade to the heuristic, got {} commits",
            commits.map(|drained| drained.len()).unwrap_or(0)
        );
        assert!(
            started.elapsed() < GIT_PROBE_TIMEOUT + std::time::Duration::from_secs(5),
            "the probe must stay inside its bound even when a helper holds the pipe (took {:?})",
            started.elapsed()
        );
        let _ = std::fs::remove_dir_all(&root);
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
    #[cfg(unix)]
    fn git_probe_degrades_when_the_log_overruns_the_drain_cap() {
        // rm-731 drain arm (run 80e8beb6 cycle 1, batch "bounded
        // inputs, honest docs"): a helper whose `git log` output
        // overruns the drain cap must degrade the root to the
        // documented heuristic instead of buffering an unbounded log
        // — a truncated census would silently bias the recency
        // heuristic. Control arm: the same shim delegates ordinary
        // roots to real git, which still drains under the cap and
        // yields commits. Mirrors the pipe-holder sibling's shim
        // pattern (marker keyed on the -C root arg).
        use std::os::unix::fs::PermissionsExt;
        let _env_lock = crate::test_env::lock_env();
        let marker = "agenttrace-gov-drain-cap";
        let root = std::env::temp_dir().join(format!("{marker}-root"));
        let shim_dir = root.join("shim");
        std::fs::create_dir_all(&shim_dir).expect("create shim dir");
        // 7,000 lines of 11-byte timestamps = ~77 KiB > the 64 KiB
        // drain cap; one line per commit mirrors `git log --format=%ct`.
        std::fs::write(
            shim_dir.join("git"),
            format!(
                "#!/bin/sh\ncase \"$2\" in\n  *{marker}*) i=0\n    while [ $i -lt 7000 ]; do\n      echo 1000000000\n      i=$((i + 1))\n    done ;;\n  *) exec /usr/bin/git \"$@\" ;;\nesac\n"
            ),
        )
        .expect("write git shim");
        std::fs::set_permissions(shim_dir.join("git"), std::fs::Permissions::from_mode(0o755))
            .expect("chmod git shim");
        let previous_path = std::env::var_os("PATH");
        std::env::set_var(
            "PATH",
            format!("{}:{}", shim_dir.display(), std::env::var("PATH").unwrap()),
        );
        let started = std::time::Instant::now();
        let overflow = git_commits(root.to_str().expect("utf-8 root"));
        let overflow_elapsed = started.elapsed();
        // Control: the crate root is a real git repository, and the
        // shim delegates it to /usr/bin/git — it must still yield
        // commits under the cap (also proves the shim only diverts
        // the marker root).
        let control = git_commits(env!("CARGO_MANIFEST_DIR"));
        if let Some(path) = previous_path {
            std::env::set_var("PATH", path);
        }
        assert!(
            overflow.is_none(),
            "an over-cap git log must degrade to the heuristic, got {} commits",
            overflow.map(|drained| drained.len()).unwrap_or(0)
        );
        assert!(
            overflow_elapsed < GIT_PROBE_TIMEOUT + std::time::Duration::from_secs(5),
            "the overflow arm must stay inside the probe bounds (took {overflow_elapsed:?})"
        );
        assert!(
            control.is_some(),
            "the control root must still yield commits through real git"
        );
        let _ = std::fs::remove_dir_all(&root);
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

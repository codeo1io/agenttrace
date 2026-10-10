//! rm-956 arm 2: cross-root Copilot usage reconcile.
//!
//! Copilot writes ONE conversation's accounting to TWO roots. The
//! CLI writes cumulative per-model snapshots (`session.shutdown` /
//! `session.usage_checkpoint` records under
//! `~/.copilot/session-state/<conversation>/events.jsonl`), while the
//! OTel exporter writes per-call spans (`chat.completion` with usage)
//! under `~/.copilot/otel/`. When both exist for the same
//! conversation, both lanes count the covered calls — ccusage #1857
//! (and PR #1858's merged reference) observed 2–3× inflation in the
//! wild. The OTel lane also captures calls the session-state root
//! never saw: a process killed before shutdown leaves the CLI's last
//! checkpoint short, while the exporter already has the span.
//!
//! This pass reconciles the two roots per `(conversation id, model)`:
//!
//! * no OTel calls for the pair → the session-state root is the only
//!   witness; nothing moves (the pre-rm-956 behavior, exactly);
//! * OTel calls exist and the shutdown snapshots' reported request
//!   deltas cover them → the OTel calls inside the covered range are
//!   DOUBLE-COUNTED: they are dropped from the OTel session and
//!   disclosed as [`DROPPED_COVERED_COUNTER`];
//! * the OTel window holds MORE calls than the snapshots reported
//!   (killed process, export lagging in) → from the boundary snapshot
//!   on, the OTel export is the better witness: the OTel session keeps
//!   every call after the boundary, the session-state session's
//!   post-boundary delta is subtracted, and both sides disclose
//!   ([`DROPPED_COVERED_COUNTER`] on the OTel session,
//!   [`REPLACED_SHUTDOWN_COUNTER`] on the session-state session).
//!
//! The boundary walk is the fork's port of ccusage #1858's
//! `otel_replacement_start` with one deliberate divergence, documented
//! on [`replacement_start`]: the reference fires replacement on the
//! first interval that contains ANY OTel rows because it compares a
//! millisecond duration against a call count; the fork fires it only
//! when the OTel window holds strictly more calls than the snapshots
//! reported for that window, which is the rule the reference's README
//! states and the conservative one for credit-floored bills.
//!
//! Consumption is idempotent: dropped calls and replaced snapshots are
//! REMOVED from the metrics' fact vectors as they are applied, so a
//! second pass over the same sessions (e.g. a cached reload) finds a
//! smaller, consistent world and adjusts nothing further.

use crate::parser::CopilotOtelCall;
use crate::{round4, Session};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Unbounded};
use std::path::Path;
/// Disclosure key on the OTel session: number of its deduped chat
/// calls that the session-state root of the same conversation already
/// counted (dropped there, never re-added).
pub const DROPPED_COVERED_COUNTER: &str = "copilot_cross_root_calls_dropped";

/// Disclosure key on the session-state session: number of its
/// cumulative snapshots that the OTel export replaced as the better
/// witness (their post-boundary usage was subtracted).
pub const REPLACED_SHUTDOWN_COUNTER: &str = "copilot_cross_root_shutdowns_replaced";

/// One `(conversation id, model)` pair's interval history, sorted by
/// timestamp (cumulative counters, not deltas).
type IntervalHistory = BTreeMap<i64, (i64, BTreeMap<String, i64>)>;

/// One `(session_index, call_index, timestamp_ns)` triple locating a
/// deduped OTel chat call for the reconcile walk.
type OtelSite = (usize, usize, i64);

/// Cross-root reconcile over the fully assembled session list. Returns
/// the number of distinct sessions whose accounting or disclosure
/// moved. Sessions without Copilot facts (every non-Copilot session)
/// are never touched — the pass is a no-op for them by construction.
pub fn reconcile_copilot_cross_root(sessions: &mut [Session]) -> usize {
    // ---- index 1: OTel call sites per (conversation, model) ----
    let mut otel_sites: BTreeMap<(String, String), Vec<OtelSite>> = BTreeMap::new();
    for (session_index, session) in sessions.iter().enumerate() {
        for (call_index, call) in session.metrics.copilot_otel_calls.iter().enumerate() {
            if call.conversation_id.is_empty() {
                continue;
            }
            otel_sites
                .entry((call.conversation_id.clone(), call.model.clone()))
                .or_default()
                .push((session_index, call_index, call.timestamp_ns));
        }
    }
    if otel_sites.is_empty() {
        return 0;
    }
    // ---- index 2: session-state interval history per (conversation, model) ----
    // The conversation id of a session-state session IS the name of the
    // directory holding its events file (ccusage's paths.rs derives the
    // same id the same way, so exports pair across implementations).
    let mut state_groups: BTreeMap<(String, String), IntervalHistory> = BTreeMap::new();
    // (conversation, model, snapshot ts) -> owning session, so both
    // the consumption pass and pass 2's final-snapshot delta owner
    // key on the EXACT session that carries a given interval (a
    // conversation's events may span more than one session-state
    // file).
    let mut interval_owners: BTreeMap<(String, String, i64), usize> = BTreeMap::new();
    for (session_index, session) in sessions.iter().enumerate() {
        if session.metrics.copilot_shutdown_intervals.is_empty() {
            continue;
        }
        let Some(conversation_id) = session_state_conversation_id(&session.path) else {
            continue;
        };
        for interval in &session.metrics.copilot_shutdown_intervals {
            let key = (conversation_id.clone(), interval.model.clone());
            state_groups.entry(key.clone()).or_default().insert(
                interval.timestamp_ns,
                (interval.requests_count, interval.usage.clone()),
            );
            interval_owners.insert((key.0, key.1, interval.timestamp_ns), session_index);
        }
    }

    // ---- pass 2: decide per pair ----
    struct OtelDrop {
        session_index: usize,
        call_index: usize,
    }
    struct StateAdjust {
        session_index: usize,
        model: String,
        usage_delta: BTreeMap<String, i64>,
        replaced_snapshots: usize,
        // Consumption: which interval range this replacement consumed
        // (the owner lookup below keys on these).
        conversation_id: String,
        boundary_ts: i64,
        final_ts: i64,
    }
    let mut otel_drops: Vec<OtelDrop> = Vec::new();
    let mut state_adjustments: Vec<StateAdjust> = Vec::new();
    for (key, sites) in &otel_sites {
        let Some(history) = state_groups.get(key) else {
            continue;
        };
        let otel_ts: Vec<i64> = sites.iter().map(|(_, _, ts)| *ts).collect();
        let (boundary_ts, replaced) = replacement_start(history, &otel_ts);
        // OTel side: calls the state root already covered (or, under
        // replacement, everything up to the boundary) are dropped.
        for &(session_index, call_index, ts) in sites {
            if ts <= boundary_ts {
                otel_drops.push(OtelDrop {
                    session_index,
                    call_index,
                });
            }
        }
        if !replaced {
            continue;
        }
        // State side: subtract everything the snapshots counted AFTER
        // the boundary — the OTel export is the witness from there on.
        let boundary_usage = if boundary_ts == i64::MIN {
            BTreeMap::new()
        } else {
            history
                .get(&boundary_ts)
                .map(|(_, usage)| usage.clone())
                .unwrap_or_default()
        };
        let Some((&final_ts, _)) = history.iter().next_back() else {
            continue;
        };
        if final_ts <= boundary_ts {
            continue;
        }
        let final_usage = history
            .get(&final_ts)
            .map(|(_, usage)| usage.clone())
            .unwrap_or_default();
        let mut usage_delta = BTreeMap::new();
        for (class, final_value) in &final_usage {
            let boundary_value = boundary_usage.get(class).copied().unwrap_or(0);
            let delta = final_value.saturating_sub(boundary_value);
            if delta > 0 {
                usage_delta.insert(class.clone(), delta);
            }
        }
        // Snapshots strictly after the boundary are the replaced ones
        // (the boundary snapshot itself stays the state root's word).
        let replaced_snapshots = history.range((Excluded(boundary_ts), Unbounded)).count();
        // The delta is subtracted from the session owning the FINAL
        // snapshot — its per-model fold already surfaced that
        // cumulative total as this session's metrics. Review 23701305
        // F3: the owner is the EXACT interval owner at final_ts, not
        // the highest-indexed state session of the conversation —
        // when one conversation spans several session-state files and
        // the final snapshot is not in the last-discovered one, the
        // subtraction must not land on a session whose metrics never
        // carried that cumulative total.
        let final_owner = interval_owners
            .get(&(key.0.clone(), key.1.clone(), final_ts))
            .copied()
            .unwrap_or(usize::MAX);
        if final_owner == usize::MAX || usage_delta.is_empty() {
            continue;
        }
        state_adjustments.push(StateAdjust {
            session_index: final_owner,
            model: key.1.clone(),
            usage_delta,
            replaced_snapshots,
            conversation_id: key.0.clone(),
            boundary_ts,
            final_ts,
        });
    }
    if otel_drops.is_empty() && state_adjustments.is_empty() {
        return 0;
    }

    // ---- pass 3: apply ----
    // Otel drops, grouped per (session, model) so cost/model_attribution
    // adjust once per model block.
    struct Delta {
        input: i64,
        output: i64,
        cache_w: i64,
        cache_r: i64,
        cost: f64,
        dropped_calls: usize,
    }
    let mut otel_deltas: BTreeMap<(usize, String), Delta> = BTreeMap::new();
    for drop in &otel_drops {
        let call = &sessions[drop.session_index].metrics.copilot_otel_calls[drop.call_index];
        let entry = otel_deltas
            .entry((drop.session_index, call.model.clone()))
            .or_insert(Delta {
                input: 0,
                output: 0,
                cache_w: 0,
                cache_r: 0,
                cost: 0.0,
                dropped_calls: 0,
            });
        entry.input = entry
            .input
            .saturating_add(call.usage.get("input_tokens").copied().unwrap_or(0));
        entry.output = entry
            .output
            .saturating_add(call.usage.get("output_tokens").copied().unwrap_or(0));
        entry.cache_w = entry.cache_w.saturating_add(
            call.usage
                .get("cache_creation_input_tokens")
                .copied()
                .unwrap_or(0),
        );
        entry.cache_r = entry.cache_r.saturating_add(
            call.usage
                .get("cache_read_input_tokens")
                .copied()
                .unwrap_or(0),
        );
        entry.cost += block_cost(&call.model, &call.usage);
        entry.dropped_calls += 1;
    }
    let mut adjusted: BTreeSet<usize> = BTreeSet::new();
    for ((session_index, model), delta) in otel_deltas {
        let metrics = &mut sessions[session_index].metrics;
        metrics.tokens_input = metrics.tokens_input.saturating_sub(delta.input);
        metrics.tokens_output = metrics.tokens_output.saturating_sub(delta.output);
        metrics.tokens_cache_w = metrics.tokens_cache_w.saturating_sub(delta.cache_w);
        metrics.tokens_cache_r = metrics.tokens_cache_r.saturating_sub(delta.cache_r);
        // The credit floor never applies here (OTel sessions carry no
        // AIU credit counter), but keep the same clamp shape as analyze
        // so a subtracted estimate can never go negative.
        metrics.cost_estimated = round4((metrics.cost_estimated - delta.cost).max(0.0));
        if let Some(entry) = metrics.model_attribution.get_mut(&model) {
            entry.input_tokens = entry.input_tokens.saturating_sub(delta.input);
            entry.output_tokens = entry.output_tokens.saturating_sub(delta.output);
            entry.cache_w = entry.cache_w.saturating_sub(delta.cache_w);
            entry.cache_r = entry.cache_r.saturating_sub(delta.cache_r);
            entry.cost_usd = round4((entry.cost_usd - delta.cost).max(0.0));
        }
        *metrics
            .disclosure_counters
            .entry(DROPPED_COVERED_COUNTER.to_string())
            .or_insert(0) += delta.dropped_calls;
        adjusted.insert(session_index);
    }
    for adjustment in &state_adjustments {
        let metrics = &mut sessions[adjustment.session_index].metrics;
        let input = adjustment
            .usage_delta
            .get("input_tokens")
            .copied()
            .unwrap_or(0);
        let output = adjustment
            .usage_delta
            .get("output_tokens")
            .copied()
            .unwrap_or(0);
        let cache_w = adjustment
            .usage_delta
            .get("cache_creation_input_tokens")
            .copied()
            .unwrap_or(0);
        let cache_r = adjustment
            .usage_delta
            .get("cache_read_input_tokens")
            .copied()
            .unwrap_or(0);
        metrics.tokens_input = metrics.tokens_input.saturating_sub(input);
        metrics.tokens_output = metrics.tokens_output.saturating_sub(output);
        metrics.tokens_cache_w = metrics.tokens_cache_w.saturating_sub(cache_w);
        metrics.tokens_cache_r = metrics.tokens_cache_r.saturating_sub(cache_r);
        // rm-485 composition: the credit counter is a floor on the
        // session's bill, so the reconciled estimate clamps at it — a
        // session whose credits covered the calls keeps reporting the
        // credit total, never less.
        let cost_delta = block_cost(&adjustment.model, &adjustment.usage_delta);
        let floor = metrics.credit_usd.max(0.0);
        metrics.cost_estimated = round4((metrics.cost_estimated - cost_delta).max(floor));
        if let Some(entry) = metrics.model_attribution.get_mut(&adjustment.model) {
            entry.input_tokens = entry.input_tokens.saturating_sub(input);
            entry.output_tokens = entry.output_tokens.saturating_sub(output);
            entry.cache_w = entry.cache_w.saturating_sub(cache_w);
            entry.cache_r = entry.cache_r.saturating_sub(cache_r);
            entry.cost_usd = round4((entry.cost_usd - cost_delta).max(0.0));
        }
        *metrics
            .disclosure_counters
            .entry(REPLACED_SHUTDOWN_COUNTER.to_string())
            .or_insert(0) += adjustment.replaced_snapshots;
        adjusted.insert(adjustment.session_index);
    }

    // ---- pass 4: consume the facts (idempotency) ----
    // Remove dropped OTel calls from the fact vectors; the remaining
    // calls sit strictly after every boundary, so a re-run finds no
    // covered window and adjusts nothing.
    let mut drop_set: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for drop in otel_drops {
        drop_set
            .entry(drop.session_index)
            .or_default()
            .insert(drop.call_index);
    }
    for (session_index, call_indices) in drop_set {
        let calls = &mut sessions[session_index].metrics.copilot_otel_calls;
        let mut kept: Vec<CopilotOtelCall> = Vec::with_capacity(calls.len());
        for (call_index, call) in calls.drain(..).enumerate() {
            if !call_indices.contains(&call_index) {
                kept.push(call);
            }
        }
        sessions[session_index].metrics.copilot_otel_calls = kept;
    }
    // Remove the REPLACED intervals from the sessions that carry them
    // (ts strictly after the boundary). A re-run then finds a history
    // that ends at the boundary — no window can overflow again — and
    // with the OTel calls all past it, the pair settles permanently.
    for adjustment in &state_adjustments {
        let mut touched: BTreeSet<usize> = BTreeSet::new();
        for ((conversation_id, model, ts), owner) in &interval_owners {
            if *conversation_id == adjustment.conversation_id
                && *model == adjustment.model
                && *ts > adjustment.boundary_ts
                && *ts <= adjustment.final_ts
            {
                touched.insert(*owner);
            }
        }
        for owner in touched {
            let intervals = &mut sessions[owner].metrics.copilot_shutdown_intervals;
            intervals.retain(|interval| {
                !(interval.model == adjustment.model
                    && interval.timestamp_ns > adjustment.boundary_ts
                    && interval.timestamp_ns <= adjustment.final_ts)
            });
        }
    }
    adjusted.len()
}

/// The conversation id of a session-state session: the name of the
/// directory holding its events file (`<id>/events.jsonl`). Mirrors
/// ccusage's `session-state` id derivation, so the join works across
/// implementations; a flat file under `session-state/` itself yields
/// the directory name `session-state`, which never matches a span
/// conversation id and simply stays unpaired.
fn session_state_conversation_id(path: &str) -> Option<String> {
    let parent = Path::new(path).parent()?;
    let name = parent.file_name()?.to_str()?;
    (!name.is_empty()).then(|| name.to_string())
}

/// rm-956 arm 2 boundary walk (fork port of ccusage #1858's
/// `otel_replacement_start`).
///
/// Walks the cumulative snapshot history and decides where (if
/// anywhere) the OTel export takes over:
///
/// * returns `(boundary, false)` — "keep the intervals, drop OTel
///   calls at or before `boundary`" — when every window the OTel
///   export touches was fully reported by the snapshots (`boundary`
///   = the LAST snapshot's timestamp, i.e. everything the state root
///   counted stays counted);
/// * returns `(boundary, true)` — "replace from `boundary` on" — at
///   the first window where the OTel export holds MORE calls than the
///   snapshots reported (`boundary` = the timestamp of the snapshot
///   BEFORE that window; `i64::MIN` when the very first window is
///   short).
///
/// A snapshot without `requests.count` (the older CLI) makes the
/// window it closes incomparable: replacement cannot fire at or after
/// it until a counted snapshot re-establishes the baseline. This is
/// the documented divergence from the reference, which fires
/// replacement at the first interval containing any OTel row because
/// it compares a millisecond duration against a call count — here the
/// comparison is calls-vs-reported-calls, and no replacement happens
/// while the snapshots already account for every exported call.
fn replacement_start(history: &IntervalHistory, otel_ts: &[i64]) -> (i64, bool) {
    let mut last_ts: i64 = i64::MIN;
    // ccusage #1858's walk starts the baseline at 0: the FIRST
    // snapshot's requests.count is already the conversation's total
    // from its beginning, so its window compares against zero. A
    // snapshot without `requests.count` resets the baseline to
    // incomparable (None) until a counted snapshot re-establishes it.
    let mut last_count: Option<i64> = Some(0);
    let mut otel_after: i64 = i64::MIN;
    let mut replaced = false;
    for (&ts, &(count, _)) in history {
        let interval_calls = otel_ts
            .iter()
            .filter(|call_ts| *call_ts > &last_ts && *call_ts <= &ts)
            .count();
        if !replaced && interval_calls > 0 {
            if let Some(prev) = last_count {
                if count > 0 {
                    let reported = count.saturating_sub(prev).max(0) as usize;
                    if interval_calls > reported {
                        otel_after = last_ts;
                        replaced = true;
                    }
                }
            }
        }
        last_ts = ts;
        last_count = (count > 0).then_some(count);
    }
    if !replaced {
        otel_after = last_ts;
    }
    (otel_after, replaced)
}

/// Catalog pricing of one usage block, replicating the per-class
/// formula `analyze` prices the same tokens with (round-4 of the
/// linear /1e6 sum at the block's own model, so subtracting this from
/// a total priced the same way removes exactly the block's share).
fn block_cost(model: &str, usage: &BTreeMap<String, i64>) -> f64 {
    let price = crate::pricing::lookup_price(model);
    round4(
        usage.get("input_tokens").copied().unwrap_or(0) as f64 / 1e6 * price.input
            + usage.get("output_tokens").copied().unwrap_or(0) as f64 / 1e6 * price.output
            + usage
                .get("cache_creation_input_tokens")
                .copied()
                .unwrap_or(0) as f64
                / 1e6
                * price.cw
            + usage.get("cache_read_input_tokens").copied().unwrap_or(0) as f64 / 1e6 * price.cr,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(ts: i64, requests: i64, input: i64) -> (i64, (i64, BTreeMap<String, i64>)) {
        (
            ts,
            (
                requests,
                BTreeMap::from([("input_tokens".to_string(), input)]),
            ),
        )
    }

    #[test]
    fn replacement_start_fires_on_the_first_window_when_otel_overflows_it() {
        // Single snapshot (killed process): the checkpoint claims 1
        // request but the export carries 2 calls before its timestamp.
        let history = IntervalHistory::from([interval(1_000, 1, 100)]);
        let otel = [400, 420, 500];
        let (boundary, replaced) = replacement_start(&history, &otel);
        assert_eq!(boundary, i64::MIN);
        assert!(replaced);
    }

    #[test]
    fn replacement_start_reports_last_interval_when_shutdowns_cover_otel() {
        // Two snapshots: after 2 requests and after 5. The OTel export
        // carries exactly the 5 covered calls — nothing to replace.
        let history = IntervalHistory::from([interval(1_000, 2, 20), interval(2_000, 5, 50)]);
        let otel = [100, 200, 1_100, 1_500, 1_900];
        let (boundary, replaced) = replacement_start(&history, &otel);
        assert_eq!(boundary, 2_000);
        assert!(!replaced);
    }

    #[test]
    fn replacement_start_fires_when_otel_holds_unreported_calls() {
        // First snapshot says 2 requests but the export carries 4 in
        // its window — the first window overflows, so replacement
        // fires with an open boundary (the export is the witness from
        // the conversation's start).
        let history = IntervalHistory::from([interval(1_000, 2, 20), interval(2_000, 5, 50)]);
        let otel = [100, 200, 400, 800, 1_600, 1_700, 1_800];
        let (boundary, replaced) = replacement_start(&history, &otel);
        assert_eq!(boundary, i64::MIN);
        assert!(replaced);
    }

    #[test]
    fn replacement_start_never_fires_when_counts_are_missing() {
        // Older CLI: no requests.count on any snapshot — the windows are
        // incomparable, so the state root stays the witness.
        let history = IntervalHistory::from([interval(1_000, 0, 20), interval(2_000, 0, 50)]);
        let otel = [100, 200, 400, 800, 1_600, 1_700];
        let (boundary, replaced) = replacement_start(&history, &otel);
        assert_eq!(boundary, 2_000);
        assert!(!replaced);
    }

    #[test]
    fn replacement_start_ignores_pre_export_intervals() {
        // The export was enabled mid-conversation: calls before the
        // first covered window are the state root's alone, and a window
        // with no OTel rows can never fire the replacement.
        let history = IntervalHistory::from([
            interval(1_000, 2, 20),
            interval(2_000, 5, 50),
            interval(3_000, 7, 70),
        ]);
        let otel = [2_100, 2_200];
        let (boundary, replaced) = replacement_start(&history, &otel);
        // Window (2_000, 3_000]: 2 otel calls vs 7-5=2 reported —
        // covered, no replacement, boundary = the last snapshot.
        assert_eq!(boundary, 3_000);
        assert!(!replaced);
    }

    #[test]
    fn conversation_id_derives_from_parent_directory() {
        assert_eq!(
            session_state_conversation_id("/home/u/.copilot/session-state/conv-abc/events.jsonl")
                .as_deref(),
            Some("conv-abc")
        );
        assert_eq!(session_state_conversation_id("events.jsonl"), None);
    }

    #[test]
    fn block_cost_matches_the_linear_catalog_formula() {
        // gpt-4o family rates come from the bundled catalog; assert the
        // formula shape rather than the exact price table by pricing a
        // block against lookup_price directly.
        let usage = BTreeMap::from([
            ("input_tokens".to_string(), 1_000_000),
            ("output_tokens".to_string(), 1_000_000),
        ]);
        let price = crate::pricing::lookup_price("gpt-4o");
        assert_eq!(
            block_cost("gpt-4o", &usage),
            round4(price.input + price.output)
        );
    }
}

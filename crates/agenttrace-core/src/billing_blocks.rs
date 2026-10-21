//! rm-042: transcript-derived five-hour billing blocks.
//!
//! The provider's five-hour billing window opens at FIRST USE and closes
//! five hours later, or earlier when the provider resets it after
//! inactivity — a reset that transcripts cannot observe and this view
//! therefore cannot claim. Before this module the only five-hour truth
//! in the fork was statusline-captured (`Session::insights.five_hour`),
//! so corpora parsed without a statusline configured carried no window
//! truth at all, and `--quota` deliberately rendered rolling windows
//! because provider anchors were disclosed as "not knowable without
//! credentials" (rm-201).
//!
//! This module derives first-use-aligned blocks from the corpus itself:
//!
//! * per session, the first parsable event timestamp opens a block
//!   `[t0, t0 + 5h)`; every later event inside the window extends it;
//!   the first event at or past the window end opens the next block at
//!   its own timestamp (the same first-use alignment the provider
//!   applies, replayed from the transcript);
//! * overlapping per-session blocks merge across sessions, because
//!   concurrent sessions share the provider window they opened together;
//! * attribution follows the quota census discipline (rm-201): sessions
//!   whose parse carried per-event stamps (`metrics.timestamps`, fed by
//!   the shared recorder for every parsable event timestamp) are
//!   event-basis and split their totals pro-rata across the blocks their
//!   stamps land in; sessions without stamps fall back to a span anchor
//!   at their session start and are censused, never silently blended;
//! * the active block is the one whose window is still open at `now`,
//!   with a burn rate over the elapsed part of the window only.
//!
//! Parity contract (roadmap acceptance): for a corpus with a captured
//! `insights.five_hour.resets_at`, the derived active block satisfies
//! `start == resets_at - 5h` and `end == resets_at` — pinned red-first in
//! `tests/five_hour_blocks_contract.rs`, replayed from a plain journal
//! with no statusline environment in play.

use crate::Session;
use chrono::{DateTime, Duration, Utc};

/// Length of the provider billing window, in seconds.
pub const BLOCK_WINDOW_SECS: i64 = 5 * 3600;

struct WindowedUsage {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    sessions_contributing: usize,
    events: usize,
    tokens_input: i64,
    tokens_output: i64,
    tokens_cache_r: i64,
    tokens_cache_w: i64,
    cost_usd: f64,
}

/// Partition one session's event stamps into first-use-aligned windows.
/// An event exactly at a window end belongs to the NEXT window (the
/// provider closes the old one at that instant).
fn session_windows(stamps: &[DateTime<Utc>]) -> Vec<(DateTime<Utc>, DateTime<Utc>, usize)> {
    let mut windows: Vec<(DateTime<Utc>, DateTime<Utc>, usize)> = Vec::new();
    for ts in stamps.iter().copied() {
        match windows.last_mut() {
            Some((_, end, count)) if ts < *end => *count += 1,
            _ => windows.push((ts, ts + Duration::seconds(BLOCK_WINDOW_SECS), 1)),
        }
    }
    windows
}

fn utc_json(ts: DateTime<Utc>) -> serde_json::Value {
    serde_json::json!(ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

fn epoch_json(ts: DateTime<Utc>) -> serde_json::Value {
    serde_json::json!(ts.timestamp())
}

/// Derive the five-hour billing-block report for a parsed corpus.
///
/// The report never reads `Session::insights`: the derivation is
/// independent of statusline captures by construction (parity with
/// captures is asserted by the contract tests, not consumed here).
pub fn billing_blocks(sessions: &[Session], now: DateTime<Utc>) -> serde_json::Value {
    let mut windows: Vec<WindowedUsage> = Vec::new();
    let mut event_basis = 0usize;
    let mut span_basis = 0usize;
    let mut unplaced = 0usize;
    let mut considered = 0usize;
    let mut cutoff: Option<DateTime<Utc>> = None;

    for session in sessions.iter() {
        considered += 1;
        if let Some(end) = crate::parse_ts(&session.metrics.session_end) {
            cutoff = Some(match cutoff {
                Some(c) if c >= end => c,
                _ => end,
            });
        }

        let stamps = &session.metrics.timestamps;
        let spans: Vec<(DateTime<Utc>, DateTime<Utc>, usize)> = if !stamps.is_empty() {
            event_basis += 1;
            session_windows(stamps)
        } else if let Some(anchor) = crate::parse_ts(&session.metrics.session_start) {
            span_basis += 1;
            vec![(anchor, anchor + Duration::seconds(BLOCK_WINDOW_SECS), 0)]
        } else {
            unplaced += 1;
            continue;
        };

        let total_events = stamps.len().max(1) as f64;
        let total_cost: f64 = session
            .metrics
            .model_attribution
            .values()
            .map(|attribution| attribution.cost_usd)
            .sum::<f64>()
            .max(0.0);
        for (start, end, events_in_window) in spans.iter().copied() {
            // Pro-rata share by stamp membership; the span fallback
            // attributes the whole session to its single anchor window.
            let share = if stamps.is_empty() {
                1.0
            } else {
                events_in_window as f64 / total_events
            };
            let entry = match windows.iter_mut().find(|w| start < w.end && w.start < end) {
                Some(w) => {
                    if start < w.start {
                        w.start = start;
                    }
                    if end > w.end {
                        w.end = end;
                    }
                    w
                }
                None => {
                    windows.push(WindowedUsage {
                        start,
                        end,
                        sessions_contributing: 0,
                        events: 0,
                        tokens_input: 0,
                        tokens_output: 0,
                        tokens_cache_r: 0,
                        tokens_cache_w: 0,
                        cost_usd: 0.0,
                    });
                    windows.last_mut().expect("just pushed")
                }
            };
            entry.sessions_contributing += 1;
            entry.events += events_in_window;
            entry.tokens_input = entry
                .tokens_input
                .saturating_add((session.metrics.tokens_input as f64 * share) as i64);
            entry.tokens_output = entry
                .tokens_output
                .saturating_add((session.metrics.tokens_output as f64 * share) as i64);
            entry.tokens_cache_r = entry
                .tokens_cache_r
                .saturating_add((session.metrics.tokens_cache_r as f64 * share) as i64);
            entry.tokens_cache_w = entry
                .tokens_cache_w
                .saturating_add((session.metrics.tokens_cache_w as f64 * share) as i64);
            entry.cost_usd += total_cost * share;
        }
    }

    windows.sort_by_key(|w| w.start);

    let mut active_index: Option<usize> = None;
    let blocks: Vec<serde_json::Value> = windows
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let active = now >= w.start && now < w.end;
            if active && active_index.is_none() {
                active_index = Some(i);
            }
            let mut block = serde_json::json!({
                "start_utc": utc_json(w.start),
                "end_utc": utc_json(w.end),
                "start_epoch": epoch_json(w.start),
                "end_epoch": epoch_json(w.end),
                "sessions_contributing": w.sessions_contributing,
                "events": w.events,
                "input_tokens": w.tokens_input,
                "output_tokens": w.tokens_output,
                "cache_read_tokens": w.tokens_cache_r,
                "cache_write_tokens": w.tokens_cache_w,
                "cost_usd": (w.cost_usd * 10_000.0).round() / 10_000.0,
                "active": active,
            });
            if active {
                let elapsed_hours = (now - w.start).num_seconds().max(0) as f64 / 3600.0;
                if elapsed_hours > 0.0 {
                    block["burn_rate_usd_per_hour"] =
                        serde_json::json!((w.cost_usd / elapsed_hours * 100.0).round() / 100.0);
                }
            }
            block
        })
        .collect();

    serde_json::json!({
        "schema": "agenttrace.blocks/1",
        "corpus": {
            "cutoff_utc": cutoff.map(utc_json).unwrap_or(serde_json::Value::Null),
            "sessions_considered": considered,
            "attribution": {
                "event_basis": event_basis,
                "span_basis": span_basis,
                "unplaced": unplaced,
            },
        },
        "disclosure": "First-use-aligned 5-hour billing windows derived from transcript event \
    timestamps; the provider's early inactivity reset is NOT derivable from transcripts and is not \
    claimed. Sessions with per-event stamps split totals pro-rata across the windows their stamps \
    land in (event basis); sessions without stamps anchor at session start (span basis, censused). \
    Token and cost attribution is pro-rata by stamp count, mirroring the quota attribution census.",
        "active_block_index": active_index.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null),
        "blocks": blocks,
    })
}

/// Render the five-hour billing-block view. Only `json` and `text` are
/// offered; every other format is refused loudly (the `--quota`
/// precedent).
pub fn render_blocks_view(
    format: &str,
    sessions: &[Session],
    now: DateTime<Utc>,
) -> Result<String, String> {
    let report = billing_blocks(sessions, now);
    match format {
        "json" => Ok(serde_json::to_string_pretty(&report)
            .map_err(|e| format!("blocks serialization failed: {e}"))?),
        "text" => {
            let mut out = String::from("5-hour billing blocks (transcript-derived)\n");
            let attr = &report["corpus"]["attribution"];
            out.push_str(&format!(
                "  sessions: {} (event-basis {}, span-basis {}, unplaced {})\n",
                report["corpus"]["sessions_considered"],
                attr["event_basis"],
                attr["span_basis"],
                attr["unplaced"]
            ));
            let blocks = report["blocks"].as_array().cloned().unwrap_or_default();
            if blocks.is_empty() {
                out.push_str("  no blocks: no placeable sessions\n");
            }
            let recent: Vec<&serde_json::Value> =
                blocks.iter().skip(blocks.len().saturating_sub(5)).collect();
            for block in recent {
                let active = if block["active"].as_bool().unwrap_or(false) {
                    " <== active"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "  {} .. {}  ${:.4}  {} sessions, {} events{}\n",
                    block["start_utc"].as_str().unwrap_or("?"),
                    block["end_utc"].as_str().unwrap_or("?"),
                    block["cost_usd"].as_f64().unwrap_or(0.0),
                    block["sessions_contributing"],
                    block["events"],
                    active
                ));
            }
            Ok(out)
        }
        other => Err(format!(
            "format '{other}' is not available for --blocks; only json and text are offered"
        )),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 15, h, m, 0).unwrap()
    }

    #[test]
    fn boundary_events_open_the_next_window() {
        let windows = session_windows(&[ts(10, 0), ts(14, 59), ts(15, 0), ts(15, 1)]);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].0, ts(10, 0));
        assert_eq!(windows[0].1, ts(15, 0));
        assert_eq!(windows[0].2, 2, "10:00 and 14:59 share the first window");
        assert_eq!(
            windows[1].0,
            ts(15, 0),
            "an event AT the boundary opens the next window"
        );
        assert_eq!(windows[1].2, 2, "15:00 and 15:01 share the second window");
    }

    #[test]
    fn empty_and_single_stamps() {
        assert!(session_windows(&[]).is_empty());
        let windows = session_windows(&[ts(9, 30)]);
        assert_eq!(windows.len(), 1);
        assert_eq!(
            windows[0].1,
            ts(9, 30) + Duration::seconds(BLOCK_WINDOW_SECS)
        );
    }
}

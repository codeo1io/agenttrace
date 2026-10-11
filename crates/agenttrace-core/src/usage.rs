//! rm-967: calendar and 5-hour-block usage rollups, ported from
//! upstream PR #306 (luoyuctl/agenttrace, merged 2026-10-04). The
//! adoption merge a9c18c4 resolved the CLI 100% fork-side and dropped
//! this surface; the port is deliberate — the bucketing reads the
//! fork's `Metrics.usage_points` (per-record `[ts, tokens, cost]`
//! triples filled in `analyze`) and falls back to a single
//! session-start point when a lane reports no per-record usage, so
//! every discovery lane is reportable without per-lane surgery.
//!
//! Community-pain evidence for the flag surface (from the upstream PR
//! body): 5-hour window / burn-rate tracking (ccusage #483, #146,
//! CCUM #106) and timezone day-boundary bugs (ccusage #778, codeburn
//! #425, CCUM #121).

use chrono::{
    DateTime, Datelike, Duration, FixedOffset, Local, NaiveDate, TimeZone, Timelike, Utc,
};
use std::collections::{BTreeMap, BTreeSet};

use crate::{round4, total_tokens, Session};

/// Length of one usage block in seconds (5 hours, mirroring the
/// Claude subscription window approximation).
const BLOCK_SECS: i64 = 5 * 60 * 60;
/// Floor for the rate window of an active block, so a single early
/// record cannot extrapolate a per-minute rate from seconds of
/// elapsed time (upstream #306 guards the same explosion).
const MIN_RATE_SECS: i64 = 15 * 60;

/// Tokens and estimated cost reported by one timestamped usage record.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UsagePoint {
    /// Unix seconds (UTC).
    pub ts: i64,
    pub tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsagePeriod {
    Day,
    Week,
    Month,
}

impl UsagePeriod {
    pub fn as_str(&self) -> &'static str {
        match self {
            UsagePeriod::Day => "day",
            UsagePeriod::Week => "week",
            UsagePeriod::Month => "month",
        }
    }
}

/// Timezone for bucket boundaries: `local`, `UTC`, or a fixed numeric
/// offset (`+08:00`, `-05:30`, `+8`). Named IANA zones are out of
/// scope for the port (the fork carries no tz database dependency);
/// `--tz` documents the accepted forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageTz {
    Local,
    Fixed(FixedOffset),
}

impl UsageTz {
    pub fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("local") {
            return Some(UsageTz::Local);
        }
        if value.eq_ignore_ascii_case("utc") || value.eq_ignore_ascii_case("z") {
            return Some(UsageTz::Fixed(FixedOffset::east_opt(0)?));
        }
        let rest = value
            .strip_prefix('+')
            .or_else(|| value.strip_prefix('-'))?;
        let negative = value.starts_with('-');
        let (hours, minutes) = match rest.split_once(':') {
            Some((h, m)) => (h.parse::<i64>().ok()?, m.parse::<i64>().ok()?),
            None if rest.len() == 4 && rest.bytes().all(|b| b.is_ascii_digit()) => (
                rest[..2].parse::<i64>().ok()?,
                rest[2..].parse::<i64>().ok()?,
            ),
            None => (rest.parse::<i64>().ok()?, 0),
        };
        if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
            return None;
        }
        let total = hours * 3600 + minutes * 60;
        let seconds = if negative { -total } else { total };
        FixedOffset::east_opt(seconds as i32).map(UsageTz::Fixed)
    }

    fn local_date(&self, ts: i64) -> Option<NaiveDate> {
        match self {
            UsageTz::Local => Local
                .timestamp_opt(ts, 0)
                .single()
                .map(|dt| dt.date_naive()),
            UsageTz::Fixed(offset) => offset
                .timestamp_opt(ts, 0)
                .single()
                .map(|dt| dt.date_naive()),
        }
    }

    fn floor_hour(&self, ts: i64) -> i64 {
        match self {
            UsageTz::Local => Local.timestamp_opt(ts, 0).single().map_or(ts, |dt| {
                dt.with_minute(0)
                    .and_then(|dt| dt.with_second(0))
                    .and_then(|dt| dt.with_nanosecond(0))
                    .map_or(ts, |dt| dt.timestamp())
            }),
            UsageTz::Fixed(offset) => offset
                .timestamp_opt(ts, 0)
                .single()
                .and_then(|dt| dt.with_minute(0))
                .and_then(|dt| dt.with_second(0))
                .and_then(|dt| dt.with_nanosecond(0))
                .map_or(ts, |dt| dt.timestamp()),
        }
    }

    fn format(&self, ts: i64) -> String {
        match self {
            UsageTz::Local => Local
                .timestamp_opt(ts, 0)
                .single()
                .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
                .unwrap_or_else(|| ts.to_string()),
            UsageTz::Fixed(offset) => offset
                .timestamp_opt(ts, 0)
                .single()
                .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
                .unwrap_or_else(|| ts.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct UsageBucket {
    pub period: String,
    pub sessions: usize,
    pub tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct UsageBlock {
    pub start: String,
    pub end: String,
    pub first_activity: String,
    pub last_activity: String,
    pub active: bool,
    pub sessions: usize,
    pub tokens: i64,
    pub cost: f64,
    pub tokens_per_minute: f64,
    pub cost_per_hour: f64,
    pub projected_tokens: i64,
    pub projected_cost: f64,
}

/// Points for one session: the per-record series when the lane filled
/// it, otherwise a single point at session start carrying the session
/// totals (documented fallback — aggregate lanes stay reportable).
pub fn session_points(session: &Session) -> Vec<UsagePoint> {
    if !session.metrics.usage_points.is_empty() {
        return session.metrics.usage_points.clone();
    }
    let Ok(start) = DateTime::parse_from_rfc3339(&session.metrics.session_start) else {
        return Vec::new();
    };
    vec![UsagePoint {
        ts: start.timestamp(),
        tokens: total_tokens(session),
        cost: session.metrics.cost_estimated,
    }]
}

fn bucket_key(date: NaiveDate, period: UsagePeriod) -> String {
    match period {
        UsagePeriod::Day => date.format("%Y-%m-%d").to_string(),
        UsagePeriod::Week => {
            let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
            monday.format("%Y-%m-%d").to_string()
        }
        UsagePeriod::Month => date.format("%Y-%m").to_string(),
    }
}

/// Groups usage into calendar buckets, newest first.
pub fn usage_by_period(sessions: &[Session], period: UsagePeriod, tz: UsageTz) -> Vec<UsageBucket> {
    let mut buckets: BTreeMap<String, (BTreeSet<usize>, i64, f64)> = BTreeMap::new();
    for (index, session) in sessions.iter().enumerate() {
        for point in session_points(session) {
            let Some(date) = tz.local_date(point.ts) else {
                continue;
            };
            let entry = buckets.entry(bucket_key(date, period)).or_default();
            entry.0.insert(index);
            entry.1 = entry.1.saturating_add(point.tokens);
            entry.2 += point.cost;
        }
    }
    buckets
        .into_iter()
        .rev()
        .map(|(period, (sessions, tokens, cost))| UsageBucket {
            period,
            sessions: sessions.len(),
            tokens,
            cost: round4(cost),
        })
        .collect()
}

/// Splits usage into 5-hour blocks, newest first.
///
/// A block starts at the top of the hour (in `tz`) of its first record
/// and lasts five hours; the next record at or after the block end
/// opens a new block. This mirrors how Claude subscription windows
/// are commonly approximated from local logs; it is an estimate, not
/// the provider's server-side accounting.
pub fn usage_blocks(sessions: &[Session], now: DateTime<Utc>, tz: UsageTz) -> Vec<UsageBlock> {
    let mut points = sessions
        .iter()
        .enumerate()
        .flat_map(|(index, session)| {
            session_points(session)
                .into_iter()
                .map(move |point| (point, index))
        })
        .collect::<Vec<_>>();
    points.sort_by_key(|(point, _)| point.ts);

    struct Acc {
        start: i64,
        first: i64,
        last: i64,
        sessions: BTreeSet<usize>,
        tokens: i64,
        cost: f64,
    }
    let mut blocks: Vec<Acc> = Vec::new();
    for (point, index) in points {
        let open_new = blocks
            .last()
            .is_none_or(|block| point.ts >= block.start + BLOCK_SECS);
        if open_new {
            blocks.push(Acc {
                start: tz.floor_hour(point.ts),
                first: point.ts,
                last: point.ts,
                sessions: BTreeSet::new(),
                tokens: 0,
                cost: 0.0,
            });
        }
        let block = blocks.last_mut().expect("block opened above");
        block.last = point.ts;
        block.sessions.insert(index);
        block.tokens = block.tokens.saturating_add(point.tokens);
        block.cost += point.cost;
    }

    let now = now.timestamp();
    blocks
        .into_iter()
        .rev()
        .map(|block| {
            let end = block.start + BLOCK_SECS;
            let active = now >= block.start && now < end;
            let measured_until = if active { now } else { block.last };
            let minutes = (measured_until - block.start).max(MIN_RATE_SECS) as f64 / 60.0;
            let tokens_per_minute = block.tokens as f64 / minutes;
            let cost_per_minute = block.cost / minutes;
            let remaining = if active {
                (end - now) as f64 / 60.0
            } else {
                0.0
            };
            UsageBlock {
                start: tz.format(block.start),
                end: tz.format(end),
                first_activity: tz.format(block.first),
                last_activity: tz.format(block.last),
                active,
                sessions: block.sessions.len(),
                tokens: block.tokens,
                cost: round4(block.cost),
                tokens_per_minute: (tokens_per_minute * 10.0).round() / 10.0,
                cost_per_hour: round4(cost_per_minute * 60.0),
                projected_cost: round4(block.cost + cost_per_minute * remaining),
                projected_tokens: block
                    .tokens
                    .saturating_add((tokens_per_minute * remaining) as i64),
            }
        })
        .collect()
}

/// Returns the timezone offset label used in report headers.
pub fn usage_tz_label(tz: UsageTz, now: DateTime<Utc>) -> String {
    match tz {
        UsageTz::Local => Local
            .from_utc_datetime(&now.naive_utc())
            .format("%:z")
            .to_string(),
        UsageTz::Fixed(offset) => offset.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Diagnostics, Metrics};

    fn ts(value: &str) -> i64 {
        DateTime::parse_from_rfc3339(value).unwrap().timestamp()
    }

    fn session(points: &[(&str, i64, f64)]) -> Session {
        Session {
            name: "s".to_string(),
            path: "s".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                usage_points: points
                    .iter()
                    .map(|(at, tokens, cost)| UsagePoint {
                        ts: ts(at),
                        tokens: *tokens,
                        cost: *cost,
                    })
                    .collect(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        }
    }

    #[test]
    fn parses_timezones() {
        assert_eq!(UsageTz::parse("local"), Some(UsageTz::Local));
        let utc = FixedOffset::east_opt(0).unwrap();
        assert_eq!(UsageTz::parse("UTC"), Some(UsageTz::Fixed(utc)));
        let cst = FixedOffset::east_opt(8 * 3600).unwrap();
        for value in ["+08:00", "+0800", "+8"] {
            assert_eq!(UsageTz::parse(value), Some(UsageTz::Fixed(cst)), "{value}");
        }
        let est = FixedOffset::west_opt(5 * 3600 + 1800).unwrap();
        assert_eq!(UsageTz::parse("-05:30"), Some(UsageTz::Fixed(est)));
        assert_eq!(UsageTz::parse("Asia/Shanghai"), None);
        assert_eq!(UsageTz::parse("+25"), None);
    }

    #[test]
    fn splits_a_session_across_midnight_in_the_requested_timezone() {
        let sessions = [session(&[
            ("2026-10-01T15:30:00Z", 100, 1.0),
            ("2026-10-01T16:30:00Z", 50, 0.5),
        ])];
        let utc = usage_by_period(&sessions, UsagePeriod::Day, UsageTz::parse("utc").unwrap());
        assert_eq!(utc.len(), 1);
        assert_eq!(utc[0].tokens, 150);
        // 16:30Z is 00:30 the next day in +08:00.
        let cst = usage_by_period(
            &sessions,
            UsagePeriod::Day,
            UsageTz::parse("+08:00").unwrap(),
        );
        assert_eq!(
            cst.iter()
                .map(|b| (b.period.as_str(), b.tokens))
                .collect::<Vec<_>>(),
            [("2026-10-02", 50), ("2026-10-01", 100)]
        );
        assert!(cst.iter().all(|bucket| bucket.sessions == 1));
    }

    #[test]
    fn groups_weeks_by_monday_and_months() {
        let sessions = [
            session(&[("2026-09-28T10:00:00Z", 1, 0.0)]), // Monday
            session(&[("2026-10-04T10:00:00Z", 2, 0.0)]), // Sunday, same ISO week
            session(&[("2026-10-05T10:00:00Z", 4, 0.0)]), // next Monday
        ];
        let utc = UsageTz::parse("utc").unwrap();
        let weeks = usage_by_period(&sessions, UsagePeriod::Week, utc);
        assert_eq!(
            weeks
                .iter()
                .map(|b| (b.period.as_str(), b.tokens, b.sessions))
                .collect::<Vec<_>>(),
            [("2026-10-05", 4, 1), ("2026-09-28", 3, 2)]
        );
        let months = usage_by_period(&sessions, UsagePeriod::Month, utc);
        assert_eq!(
            months
                .iter()
                .map(|b| (b.period.as_str(), b.tokens))
                .collect::<Vec<_>>(),
            [("2026-10", 6), ("2026-09", 1)]
        );
    }

    #[test]
    fn builds_five_hour_blocks_and_projects_the_active_one() {
        let sessions = [
            session(&[
                ("2026-10-04T08:10:00Z", 1000, 1.0),
                ("2026-10-04T09:10:00Z", 1000, 1.0),
            ]),
            session(&[
                ("2026-10-04T13:05:00Z", 600, 0.6), // past 08:00 + 5h -> new block
                ("2026-10-04T13:35:00Z", 600, 0.6),
            ]),
        ];
        let utc = UsageTz::parse("utc").unwrap();
        let now = DateTime::parse_from_rfc3339("2026-10-04T14:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let blocks = usage_blocks(&sessions, now, utc);
        assert_eq!(blocks.len(), 2);

        let active = &blocks[0];
        assert!(active.active);
        assert_eq!(active.start, "2026-10-04T13:00:00+00:00");
        assert_eq!(active.end, "2026-10-04T18:00:00+00:00");
        assert_eq!(active.tokens, 1200);
        // 1200 tokens over 60 minutes since the 13:00 block start.
        assert_eq!(active.tokens_per_minute, 20.0);
        assert_eq!(active.cost_per_hour, 1.2);
        // 4h remaining at $0.02/min.
        assert_eq!(active.projected_cost, 6.0);
        assert_eq!(active.projected_tokens, 1200 + 20 * 240);

        let done = &blocks[1];
        assert!(!done.active);
        assert_eq!(done.start, "2026-10-04T08:00:00+00:00");
        assert_eq!(done.tokens, 2000);
        assert_eq!(done.projected_cost, done.cost);
    }

    #[test]
    fn blocks_start_on_the_hour_of_the_requested_timezone() {
        let sessions = [session(&[("2026-10-04T04:45:00Z", 100, 0.1)])];
        let now = DateTime::parse_from_rfc3339("2026-10-05T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        // 04:45Z is 10:15 in +05:30, so the block starts at 10:00 local.
        let ist = UsageTz::parse("+05:30").unwrap();
        let blocks = usage_blocks(&sessions, now, ist);
        assert_eq!(blocks[0].start, "2026-10-04T10:00:00+05:30");
        assert_eq!(blocks[0].end, "2026-10-04T15:00:00+05:30");
        let west = UsageTz::parse("-03:30").unwrap();
        assert_eq!(
            usage_blocks(&sessions, now, west)[0].start,
            "2026-10-04T01:00:00-03:30"
        );
    }

    #[test]
    fn single_early_record_does_not_explode_the_projection() {
        let sessions = [session(&[("2026-10-04T13:00:30Z", 100_000, 1.0)])];
        let now = DateTime::parse_from_rfc3339("2026-10-04T13:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let block = &usage_blocks(&sessions, now, UsageTz::parse("utc").unwrap())[0];
        assert!(block.active);
        // Rate is floored to a 15-minute window, not one minute.
        assert!((block.tokens_per_minute - 100_000.0 / 15.0).abs() < 0.1);
        assert!(
            block.projected_tokens < 2_100_000,
            "{}",
            block.projected_tokens
        );
    }

    #[test]
    fn falls_back_to_session_start_without_usage_points() {
        let mut aggregate = session(&[]);
        aggregate.metrics.session_start = "2026-10-03T23:00:00Z".to_string();
        aggregate.metrics.tokens_input = 70;
        aggregate.metrics.cost_estimated = 0.7;
        let days = usage_by_period(
            &[aggregate],
            UsagePeriod::Day,
            UsageTz::parse("utc").unwrap(),
        );
        assert_eq!(days.len(), 1);
        assert_eq!(
            (days[0].period.as_str(), days[0].tokens, days[0].cost),
            ("2026-10-03", 70, 0.7)
        );
    }
}

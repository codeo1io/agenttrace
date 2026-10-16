---
title: Admission predicates need an upper bound, not just a floor
date: 2026-10-16
category: reliability
module: agenttrace-core
problem_type: silent-admission
component: crates/agenttrace-core/src/insights.rs
severity: high
tags: [time-ranges, admission, anomalies, disclosure, far-future]
related_issues:
  - ROADMAP rm-1534 (this fix, run 92ea7949 cycle 4)
  - ROADMAP rm-1402 (the snapshot clamp whose pattern this generalizes)
  - ROADMAP rm-890 (the one-predicate discipline this extends)
  - ROADMAP rm-727 (the no-silent-drop accounting discipline)
---

## Problem

A session whose only timestamps sit in the year 9999 rendered as
HEALTH 100 with zero anomalies AND was admitted into `--range today`
— because the range-admission predicate (`session_admitted_since`)
checked only `time >= since`. The floor kept old sessions out; nothing
kept impossible future sessions out of forward windows. One corrupted
clock (a laptop set to year 9999, a timezone parsed as an offset
catastrophe, a fuzzed corpus) laundered a session into every forward
window and drove `latest_session_at` to 9999.

## Symptoms

- `--overview --range today` includes sessions whose last activity is
  millennia ahead, with `latest_session_at` reporting the far-future
  stamp.
- `--sessions` shows HEALTH 100 / anomalies 0 for the future session.
- `--doctor` census never mentions them.

## Root cause

The admission predicate was a half-open interval with only a lower
bound, while the corpus's timestamps are UNTRUSTED INPUT. A floor-only
filter trusts that "later than the window start" implies "plausibly
near now" — which no corrupted clock honors. The anomaly taxonomy had
no future class either, so the session itself disclosed nothing.

## Fix

Admission is bounded above: `now + FUTURE_ADMISSION_GRACE_SECS`
(24h — tight enough to exclude fabrications, loose enough for a
timezone-offset mistake or small skew). The bound lives in ONE place
(`insights::future_admission_bound`) and drives three surfaces that
cannot disagree:

1. the admission predicate (excludes when a range is active;
   `--range all` keeps the session VISIBLE — the disclosure there is
   the anomaly, not a vanishing session);
2. a new `future_timestamp` anomaly (high severity, health drops),
   anchored on the same last-activity basis as the predicate;
3. a loader counter (`LoadReport::future_excluded_sessions`) that
   flows into the data-health `future_timestamp_sessions` disclosure
   and the doctor census/recommendation — the exclusion is counted and
   disclosed, never a silent drop.

The sqlite pre-filter stays a superset (floor-only) so the merged
retain owns the bound and its count — every lane (file, hermes sqlite,
opencode sqlite) filters through the same predicate, preserving the
rm-890 one-predicate discipline.

Unknown-time sessions keep the N7 bucket contract: visible, never
"future". In-grace sessions (now+12h) are admitted unchanged and
anomaly-free — grace tolerance is a contract, pinned by test.

## Verification

- `cargo test -p agenttrace-core --test future_timestamp_bound_contract`
  — 8 pins: 24h grace value, exclusion, `--range all` visibility,
  in-grace admission, past/unknown contracts, anomaly class + silence
  for unknown-time, loader counting end-to-end, in-grace e2e.
- End-to-end on the year-9999 fixture: `--overview --range today`
  excludes it, data_health shows `future_timestamp_sessions: 1` (json)
  and a "Future-dated sessions" text row; `--range all` shows it at
  health 70 with the `future_timestamp` anomaly; `--doctor` counts it
  under "future-dated session timestamps" with a recommendation.

## Lesson

Every window predicate over untrusted timestamps needs both edges. A
floor alone silently admits the impossible; a ceiling alone silently
drops the merely-late. And whenever a filter EXCLUDES, the exclusion
must surface as a number somewhere a human reads — the same
no-silent-drop discipline as copy accounting. The anomaly class, the
admission bound, and the disclosure counter must share one constant
and one anchor, or they drift and disagree about which timestamp
spoke.

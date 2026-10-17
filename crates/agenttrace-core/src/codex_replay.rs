//! rm-402 — corpus-level reconciliation of codex MultiAgent V2 replay families.
//!
//! A multi-agent child rollout opens by replaying the parent's transcript as
//! its own context. Per file that is honest usage (the API really billed the
//! replay as input), but a corpus that sums per-rollout usage bills the
//! replayed prefix once in the parent and again inside the child. The
//! acceptance PoC: parent final 1,400 + child-from-zero final 1,700 = 3,100
//! reported vs 1,700 true (+82%).
//!
//! Detection is content-based and conservative (the wire carries no session
//! ids or replay markers — recorded deviation): two rollouts belong to the
//! same replay family when their opening `response_item` payloads are
//! byte-identical across at least two items AND the shared opening carries
//! at least `MIN_SHARED_OPENING_BYTES` of serialized transcript (review
//! af521003 F4: a shared two-item opening of a few hundred bytes is the
//! signature of a prompt retry or templated first turns, not of a replayed
//! parent transcript — merging those dropped a real session's usage).
//! The earliest-starting member — by the rollout FILE's own first
//! timestamp, re-derived from disk so the session cache's serde-dropped
//! `metrics.timestamps` cannot invert the roles on a warm run (review
//! af521003 F1), with the canonical path as a deterministic tie-break —
//! is the parent; every other member that continues past the shared
//! opening inherits the parent's final `total_token_usage` snapshot as
//! its baseline, saturating per field: the child keeps only the tokens
//! it added, and the replayed prefix stays counted once, attributed to
//! the parent.
//!
//! A child whose final snapshot is below the parent's on ANY field is NOT
//! rebased (review af521003 F3): that is the concurrent-spawn shape — the
//! child spawned mid-parent and replayed the parent's state at spawn time,
//! so the parent's final snapshot is not a valid baseline and a saturating
//! delta would silently erase the child's real usage while mislabeling it
//! as replayed tokens. Such children keep their full parsed usage and
//! carry a distinct `codex_multiagent_replay_skipped_below_baseline`
//! disclosure; the family total then overcounts (conservative) rather
//! than destroying data.
//!
//! Guarantees:
//! * A corpus with no family (e.g. any single-file codex corpus) is
//!   untouched — byte-identical output before/after.
//! * Sessions priced from upstream-recorded costs are never adjusted here
//!   (their token-derived cost would not recompute); they are disclosed as
//!   skipped instead.
//! * Adjusted children carry disclosure counters through the existing
//!   `metrics.disclosure_counters` channel
//!   (`codex_multiagent_replayed_child_adjusted`,
//!   `codex_multiagent_replayed_tokens_dropped`), mirroring the
//!   `codex_token_count_skipped` family.
//! * Rebased costs use the exact per-Mtok catalog formula the `analyze`
//!   pass applies (each class / 1e6 at its own rate — review af521003 F2:
//!   the previous per-token multiplication inflated a rebased child's
//!   cost by x1,000,000).
//! * zstd rollouts — the format Codex ≥ 0.152 actually writes (landed
//!   rm-992, after this run's base) — scan through the same bounded
//!   decoder `parse_session_bytes` uses, so every session the parser
//!   accepts is eligible for family detection; a file that fails to
//!   decode yields no evidence (never a second loud failure), and family
//!   identity is content-based, so a zstd parent and an uncompressed
//!   child of the same rollout still tie (integration case 4404ce94).
//!
//! rm-1907 (run 0779d26b, cycle 6): shape alone does not make a parent.
//! Two SIBLINGS that share a long onboarding template pass every shape
//! gate — identical ≥384-byte 2+-item opening, both continue past it,
//! the later one's final covers the earlier's — and the rebase then
//! silently destroyed the later sibling's real usage under a FALSE
//! replay disclosure (assess PoC: corpus reported 900 in / $0.0027
//! against a true 1,400 / $0.0042, −36%, "500 tokens dropped"). The
//! wire carries no pointer/turn-id linkage, so positive parent evidence
//! is measured from the two files' own content: the presumed parent's
//! billed usage must already be COMPLETE inside the byte-identical
//! prefix (its cumulative snapshot at the divergence boundary equals
//! its final — it billed nothing the child did not replay), and the
//! child's FIRST cumulative snapshot must already cover that boundary
//! input (it really billed the replayed prefix, not merely started from
//! a template it billed itself). A family without that evidence is
//! UNRESOLVED: no member's usage moves, and the would-be child discloses
//! `codex_multiagent_replay_family_unresolved` — numbers never drift
//! silently in either direction.
//!
//! rm-1908 (run 0779d26b, cycle 6, disclosure arm): reconciliation
//! re-reads rollout files, and a corpus whose rollouts turned
//! unreadable between runs (permission flips, mid-delete, vanished
//! paths on a warm session cache) used to lose its families SILENTLY —
//! reported totals flipped between runs with no disclosure. Every
//! unreadable rollout now marks its own session with
//! `codex_multiagent_replay_scan_unreadable` and counts in
//! `CodexReplayAdjustment::scans_unreadable`; the flip stays visible
//! instead of silent.

use crate::{lookup_price_tiered, round4, Metrics, Session};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Aggregate outcome of one reconciliation pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CodexReplayAdjustment {
    /// Replay families found (fingerprint groups with 2+ codex sessions).
    pub families: usize,
    /// Child rollouts whose usage was rebased onto the parent's final snapshot.
    pub children_adjusted: usize,
    /// Family children left untouched because they carry upstream-recorded
    /// costs (token-derived recompute would be dishonest for them).
    pub children_skipped_upstream: usize,
    /// Family children left untouched because the parent's final snapshot
    /// exceeds theirs on some field (concurrent-spawn shape) — a saturating
    /// rebase would erase real usage (review af521003 F3).
    pub children_below_baseline: usize,
    /// rm-1907: family children left untouched because the family is
    /// UNRESOLVED — no positive parent evidence (the presumed parent
    /// billed usage past the shared prefix, or the child's first
    /// cumulative snapshot does not cover it), so the rebase would eat
    /// real usage. Disclosed per child via
    /// `codex_multiagent_replay_family_unresolved`; usage never moves.
    pub children_unresolved: usize,
    /// rm-1908: codex rollouts that could not be READ during
    /// reconciliation (vanished path, permission flip, undecodable) —
    /// each one silently removed its session from every family and
    /// flipped reported totals. Disclosed per session via
    /// `codex_multiagent_replay_scan_unreadable`.
    pub scans_unreadable: usize,
    /// Replayed tokens removed from children, per class.
    pub tokens_input_dropped: i64,
    pub tokens_cache_read_dropped: i64,
    pub tokens_cache_write_dropped: i64,
    pub tokens_output_dropped: i64,
    pub tokens_reasoning_dropped: i64,
}

/// Cumulative `total_token_usage` snapshot of one rollout, per wire field.
#[derive(Debug, Default, Clone, Copy)]
struct TotalSnapshot {
    input: i64,
    cached: i64,
    cache_creation: i64,
    output: i64,
    reasoning: i64,
}

impl TotalSnapshot {
    fn saturating_delta(child: &TotalSnapshot, parent: &TotalSnapshot) -> TotalSnapshot {
        TotalSnapshot {
            input: child.input.saturating_sub(parent.input),
            cached: child.cached.saturating_sub(parent.cached),
            cache_creation: child.cache_creation.saturating_sub(parent.cache_creation),
            output: child.output.saturating_sub(parent.output),
            reasoning: child.reasoning.saturating_sub(parent.reasoning),
        }
    }

    /// Review af521003 F3: the parent's final snapshot is a valid baseline
    /// only if the child's cumulative usage never fell below it on any
    /// field — the sequential-replay shape. Any violation means the child
    /// spawned mid-parent (concurrent) and replayed the parent's EARLIER
    /// state, so per-field subtraction would silently zero real usage.
    fn is_plausible_baseline(parent: &TotalSnapshot, child: &TotalSnapshot) -> bool {
        child.input >= parent.input
            && child.cached >= parent.cached
            && child.cache_creation >= parent.cache_creation
            && child.output >= parent.output
            && child.reasoning >= parent.reasoning
    }
}

/// Opening-context fingerprint of one rollout file: chained payload hashes
/// of the first response_items, the item count it rests on, and the
/// serialized volume of that shared opening. `None` when the file offers
/// too little stable content to tie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OpeningFingerprint {
    hash: u64,
    items: usize,
    bytes: usize,
}

/// Everything the reconciliation pass needs from one rollout file.
struct RolloutScan {
    fingerprint: Option<OpeningFingerprint>,
    /// Final cumulative `total_token_usage` snapshot (the file's last one).
    total: TotalSnapshot,
    /// FIRST cumulative `total_token_usage` snapshot — rm-1907's
    /// child-side parent evidence: a true replay child bills the
    /// replayed prefix on its first token event, so that snapshot
    /// already covers the prefix's usage (a sibling that merely shares
    /// a template opening also starts low, but the parent-side boundary
    /// check is what separates the two).
    first_total: Option<TotalSnapshot>,
    /// First RFC3339 `timestamp` seen on any line of the file — the
    /// session's true start, re-derived from disk so it survives the
    /// session-cache round-trip that drops `metrics.timestamps`
    /// (`#[serde(skip)]`, review af521003 F1).
    first_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Total `response_item` lines (not just fingerprinted ones) — a
    /// replay child by definition continues past the shared opening.
    response_items: usize,
    /// rm-1908: the file could not be read, zstd-decoded or interpreted
    /// as UTF-8 — distinct from "scanned clean, no evidence": the
    /// caller must disclose it, never let the failure silently hide a
    /// family.
    read_failed: bool,
}

/// Stream a rollout file once: hash the opening `response_item` payloads,
/// remember the last `token_count` cumulative snapshot, the first line
/// timestamp and the total item count. Cheap and strictly read-only;
/// malformed lines are ignored exactly like the parser ignores them, so
/// fingerprint and parse see the same file.
///
/// zstd rollouts (Codex ≥ 0.152 writes the `.jsonl` file as a zstd frame,
/// landed rm-992 after this run's base): the file is read whole, and a
/// zstd-magic file goes through the SAME bounded decoder `parse_session_bytes`
/// uses, so a session the parser accepted as first-class can never be
/// invisible to family detection — the acceptance (replay counted once,
/// attributed to the parent, disclosed) holds on the wire format Codex
/// actually writes. A decode failure here yields no evidence, not an error:
/// the parser has already failed loudly for that file at parse time, and
/// reconciliation must never turn one parse-level refusal into a second one.
fn scan_rollout(path: &Path) -> RolloutScan {
    let mut scan = RolloutScan {
        fingerprint: None,
        total: TotalSnapshot::default(),
        first_total: None,
        first_timestamp: None,
        response_items: 0,
        read_failed: false,
    };
    let Some(text) = rollout_text(path) else {
        scan.read_failed = true;
        return scan;
    };
    let mut hasher = DefaultHasher::new();
    let mut items = 0usize;
    let mut bytes = 0usize;
    const MAX_FINGERPRINT_ITEMS: usize = 4;
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if scan.first_timestamp.is_none() {
            if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                scan.first_timestamp = chrono::DateTime::parse_from_rfc3339(ts)
                    .ok()
                    .map(|t| t.with_timezone(&chrono::Utc));
            }
        }
        let kind = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let payload = v.get("payload").unwrap_or(&serde_json::Value::Null);
        match kind {
            "response_item" => {
                scan.response_items += 1;
                if items < MAX_FINGERPRINT_ITEMS {
                    // Stable textual form of (type, payload) — byte-identical
                    // openings hash identically regardless of key order.
                    let mut item_hasher = DefaultHasher::new();
                    payload.hash(&mut item_hasher);
                    item_hasher.finish().hash(&mut hasher);
                    bytes += serde_json::to_string(payload)
                        .map(|serialized| serialized.len())
                        .unwrap_or(0);
                    items += 1;
                }
            }
            "event_msg" => {
                let payload_type = payload.get("type").and_then(|t| t.as_str());
                if payload_type == Some("token_count") {
                    if let Some(info) = payload.get("info") {
                        if let Some(usage) = info.get("total_token_usage") {
                            let get =
                                |key: &str| usage.get(key).and_then(|x| x.as_i64()).unwrap_or(0);
                            let snapshot = TotalSnapshot {
                                input: get("input_tokens"),
                                cached: get("cached_input_tokens"),
                                cache_creation: get("cache_creation_input_tokens"),
                                output: get("output_tokens"),
                                reasoning: get("reasoning_output_tokens"),
                            };
                            // rm-1907: keep the FIRST cumulative snapshot —
                            // the child-side half of the parent-evidence
                            // check below.
                            if scan.first_total.is_none() {
                                scan.first_total = Some(snapshot);
                            }
                            scan.total = snapshot;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // Review af521003 F4: an opening qualifies as replay evidence only
    // with two or more shared items AND enough serialized volume —
    // short identical openings are prompt retries / templated first
    // turns, not replayed transcripts, and merging them dropped a real
    // session's usage (probe: family 200 vs true 300).
    const MIN_SHARED_OPENING_BYTES: usize = 384;
    if items >= 2 && bytes >= MIN_SHARED_OPENING_BYTES {
        scan.fingerprint = Some(OpeningFingerprint {
            hash: hasher.finish(),
            items,
            bytes,
        });
    }
    scan
}

/// rm-1908 (run 0779d26b, cycle 6): a rollout's text through the shared
/// container handling — `None` when the file cannot be read, decoded or
/// interpreted as UTF-8. Both the pass-1 scan and the pass-2 corroboration
/// read through here so they can never disagree about what a file says.
fn rollout_text(path: &Path) -> Option<String> {
    let mut raw = std::fs::read(path).ok()?;
    // The zstd magic `28 B5 2F FD` — same probe as `parse_session_bytes`
    // (rm-992). Plain rollouts take the direct path byte-for-byte.
    if raw.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        let label = path.display().to_string();
        raw = crate::parser::decode_zstd_rollout_bounded(raw, &label).ok()?;
    }
    String::from_utf8(raw).ok()
}

/// rm-1907 (run 0779d26b, cycle 6): stream both rollouts in lockstep over
/// their `response_item` payloads and measure how deep the byte-identical
/// opening actually goes. Returns the shared item count and the PARENT's
/// cumulative `total_token_usage` at the divergence point — the parent's
/// billed state as of the last item the child replays. `None` when either
/// file cannot be read; the caller treats that as no evidence (a file that
/// was readable in pass 1 failing here is a mid-run race — the resulting
/// unresolved-family disclosure still covers the totals).
fn scan_shared_prefix(parent: &Path, child: &Path) -> Option<(usize, TotalSnapshot)> {
    let parent_text = rollout_text(parent)?;
    let child_text = rollout_text(child)?;
    let mut parent_lines = parent_text.lines();
    let mut child_lines = child_text.lines();
    let mut parent_usage = TotalSnapshot::default();
    let mut child_usage = TotalSnapshot::default();
    let mut shared = 0usize;
    loop {
        match (
            next_response_hash(&mut parent_lines, &mut parent_usage),
            next_response_hash(&mut child_lines, &mut child_usage),
        ) {
            (Some(parent_hash), Some(child_hash)) if parent_hash == child_hash => shared += 1,
            _ => break,
        }
    }
    Some((shared, parent_usage))
}

/// Advance to the next `response_item` line, folding `token_count`
/// cumulative snapshots seen on the way into `usage`. Returns the hashed
/// payload — the SAME stable per-item hash the opening fingerprint chains,
/// so "shared item" here means byte-identical payload exactly as family
/// detection means it.
fn next_response_hash(lines: &mut std::str::Lines, usage: &mut TotalSnapshot) -> Option<u64> {
    for line in lines {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let kind = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let payload = v.get("payload").unwrap_or(&serde_json::Value::Null);
        match kind {
            "response_item" => {
                let mut item_hasher = DefaultHasher::new();
                payload.hash(&mut item_hasher);
                return Some(item_hasher.finish());
            }
            "event_msg" => {
                let payload_type = payload.get("type").and_then(|t| t.as_str());
                if payload_type == Some("token_count") {
                    if let Some(info) = payload.get("info") {
                        if let Some(usage_value) = info.get("total_token_usage") {
                            let get = |key: &str| {
                                usage_value.get(key).and_then(|x| x.as_i64()).unwrap_or(0)
                            };
                            *usage = TotalSnapshot {
                                input: get("input_tokens"),
                                cached: get("cached_input_tokens"),
                                cache_creation: get("cache_creation_input_tokens"),
                                output: get("output_tokens"),
                                reasoning: get("reasoning_output_tokens"),
                            };
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// rm-1907: POSITIVE parent evidence for rebasing this child onto the
/// promoted member's FINAL snapshot. The byte-identical opening only
/// proves the two files share a prefix; a replay family and two siblings
/// with the same onboarding template are indistinguishable by shape. Two
/// wire facts separate them, both measurable from the files themselves:
///
/// 1. the presumed parent's billed usage was already COMPLETE inside the
///    shared prefix — its cumulative snapshot at the divergence boundary
///    equals its final (cumulative snapshots are monotone, so equality
///    means it billed nothing the child did not replay; the parent-final
///    baseline is then exactly what the child replayed);
/// 2. the child's FIRST cumulative snapshot already covers the boundary's
///    input — it really billed the replayed prefix on its first turn, not
///    merely started from a template it billed itself.
///
/// Anything else (a parent that billed past the boundary — the sibling and
/// mid-spawn shapes alike) leaves the family unresolved: no rebase, no
/// drop, disclosure instead.
fn parent_final_is_evidenced(
    shared: usize,
    parent_at_boundary: &TotalSnapshot,
    parent_final: &TotalSnapshot,
    child_first: Option<TotalSnapshot>,
) -> bool {
    shared > 0
        && parent_at_boundary.input == parent_final.input
        && parent_at_boundary.cached == parent_final.cached
        && parent_at_boundary.cache_creation == parent_final.cache_creation
        && parent_at_boundary.output == parent_final.output
        && parent_at_boundary.reasoning == parent_final.reasoning
        && child_first.is_some_and(|first| first.input >= parent_at_boundary.input)
}

/// Recompute a child's estimated cost from its rebased token classes with
/// the exact per-Mtok catalog formula the `analyze` pass applies
/// (lib.rs `catalog_cost`: every class divided by 1e6 at its own rate;
/// `reasoning` is a breakdown of `output` on this wire — rm-603 — so it
/// carries no separate term, exactly like the canonical formula).
fn rebase_cost(metrics: &Metrics, uncached_input: i64, rebased: &TotalSnapshot) -> f64 {
    let price = lookup_price_tiered(&metrics.model_used, metrics.service_tier.as_deref());
    round4(
        uncached_input as f64 / 1e6 * price.input
            + rebased.output as f64 / 1e6 * price.output
            + rebased.cache_creation as f64 / 1e6 * price.cw
            + rebased.cached as f64 / 1e6 * price.cr,
    )
}

/// Reconcile codex MultiAgent V2 replay families in a discovered corpus.
///
/// Runs after all sessions are assembled and before any view is prepared.
/// Mutates only children of detected families; everything else — including
/// every single-file corpus — is left byte-identical.
pub fn reconcile_codex_multiagent_replays(sessions: &mut [Session]) -> CodexReplayAdjustment {
    let mut stats = CodexReplayAdjustment::default();

    // Pass 1: scan codex sessions for family evidence.
    struct Evidence {
        session_idx: usize,
        started_at: chrono::DateTime<chrono::Utc>,
        path: PathBuf,
        total: TotalSnapshot,
        first_total: Option<TotalSnapshot>,
        fingerprint_items: usize,
        response_items: usize,
    }
    let mut families: HashMap<OpeningFingerprint, Vec<Evidence>> = HashMap::new();
    for (idx, session) in sessions.iter_mut().enumerate() {
        if session.metrics.source_tool != "codex_cli" {
            continue;
        }
        let Ok(path) = Path::new(&session.path).canonicalize() else {
            // rm-1908: the rollout vanished between the parse that
            // produced this session and the reconciliation that anchors
            // families (warm cache, moved/deleted corpus). The family it
            // would anchor is gone and reported totals flip — that flip
            // must be visible, never silent.
            *session
                .metrics
                .disclosure_counters
                .entry("codex_multiagent_replay_scan_unreadable".to_string())
                .or_insert(0) += 1;
            stats.scans_unreadable += 1;
            continue;
        };
        let scan = scan_rollout(&path);
        if scan.read_failed {
            // rm-1908: unreadable at reconcile time (permission flip,
            // mid-delete race, undecodable container) — same silent-flip
            // class as the vanished path above: the session still renders
            // from its parse, but its family membership is gone and must
            // be disclosed, never silently dropped.
            *session
                .metrics
                .disclosure_counters
                .entry("codex_multiagent_replay_scan_unreadable".to_string())
                .or_insert(0) += 1;
            stats.scans_unreadable += 1;
            continue;
        }
        let Some(fingerprint) = scan.fingerprint else {
            continue;
        };
        // Review af521003 F1: prefer the rollout FILE's own first
        // timestamp — the session cache's serde round-trip drops
        // `metrics.timestamps`, so every warm-run member used to tie at
        // MIN_UTC and the stable sort degenerated to directory-walk
        // order, inverting parent/child on the shipped fixture itself.
        let started_at = scan
            .first_timestamp
            .or_else(|| session.metrics.timestamps.first().copied())
            .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC);
        families.entry(fingerprint).or_default().push(Evidence {
            session_idx: idx,
            started_at,
            path,
            total: scan.total,
            first_total: scan.first_total,
            fingerprint_items: fingerprint.items,
            response_items: scan.response_items,
        });
    }

    // Pass 2: per family, earliest start is the parent; rebase the rest.
    for (_fp, mut members) in families {
        if members.len() < 2 {
            continue;
        }
        stats.families += 1;
        // Earliest real start wins; the canonical path tie-break keeps
        // the order deterministic when timestamps tie or are absent,
        // independent of the directory walk order.
        members.sort_by(|a, b| {
            a.started_at
                .cmp(&b.started_at)
                .then_with(|| a.path.cmp(&b.path))
        });
        let parent_total = members[0].total;
        for child in &members[1..] {
            let session = &mut sessions[child.session_idx];
            if session.metrics.upstream_cost_usd > 0.0 {
                session
                    .metrics
                    .disclosure_counters
                    .entry("codex_multiagent_replay_skipped_upstream_priced".to_string())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
                stats.children_skipped_upstream += 1;
                continue;
            }
            // Review af521003 F4 (second discriminator): a session whose
            // whole transcript IS the shared opening continued past
            // nothing — it is the template/retry shape, not a replay
            // child doing its own work. Non-membership is silent and
            // conservative: its usage stays fully counted.
            if child.response_items <= child.fingerprint_items {
                continue;
            }
            // Review af521003 F3: concurrent-spawn shape — the child's
            // final snapshot is below the parent's on some field, so the
            // parent-final baseline is invalid. Keep the child's usage
            // intact and disclose distinctly instead of saturating real
            // usage to zero under a replay label.
            if !TotalSnapshot::is_plausible_baseline(&parent_total, &child.total) {
                session
                    .metrics
                    .disclosure_counters
                    .entry("codex_multiagent_replay_skipped_below_baseline".to_string())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
                stats.children_below_baseline += 1;
                continue;
            }
            // rm-1907 (run 0779d26b, cycle 6): shape alone does not make a
            // parent — rebase only on positive parent evidence (see
            // `parent_final_is_evidenced`). A family without it is
            // UNRESOLVED: no member's usage moves, the would-be child
            // discloses. This is the sibling-shape guard: two independent
            // sessions sharing a long onboarding template used to lose
            // the later one's real usage here (−36% in the assess PoC).
            let corroborated = scan_shared_prefix(&members[0].path, &child.path).is_some_and(
                |(shared, parent_at_boundary)| {
                    parent_final_is_evidenced(
                        shared,
                        &parent_at_boundary,
                        &parent_total,
                        child.first_total,
                    )
                },
            );
            if !corroborated {
                session
                    .metrics
                    .disclosure_counters
                    .entry("codex_multiagent_replay_family_unresolved".to_string())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
                stats.children_unresolved += 1;
                continue;
            }
            let before_input = session.metrics.tokens_input;
            let before_cache_r = session.metrics.tokens_cache_r;
            let before_cache_w = session.metrics.tokens_cache_w;
            let before_output = session.metrics.tokens_output;
            let before_reasoning = session.metrics.tokens_reasoning;

            let rebased = TotalSnapshot::saturating_delta(&child.total, &parent_total);
            // Mirror the codex parser's class mapping (parser.rs):
            // `tokens_input` is UNCACHED input — the wire's
            // `input_tokens` includes the cached read.
            let uncached_input = rebased.input.saturating_sub(rebased.cached).max(0);
            session.metrics.tokens_input = uncached_input;
            session.metrics.tokens_cache_r = rebased.cached;
            session.metrics.tokens_cache_w = rebased.cache_creation;
            session.metrics.tokens_output = rebased.output;
            session.metrics.tokens_reasoning = rebased.reasoning;
            session.metrics.cost_estimated =
                rebase_cost(&session.metrics, uncached_input, &rebased);

            let dropped_input = before_input
                .saturating_sub(session.metrics.tokens_input)
                .max(0);
            let dropped = dropped_input
                + before_cache_r
                    .saturating_sub(session.metrics.tokens_cache_r)
                    .max(0)
                + before_cache_w
                    .saturating_sub(session.metrics.tokens_cache_w)
                    .max(0)
                + before_output
                    .saturating_sub(session.metrics.tokens_output)
                    .max(0)
                + before_reasoning
                    .saturating_sub(session.metrics.tokens_reasoning)
                    .max(0);
            let dropped_usize = u64::try_from(dropped.max(0))
                .map(|v| v.min(usize::MAX as u64) as usize)
                .unwrap_or(usize::MAX);
            session
                .metrics
                .disclosure_counters
                .entry("codex_multiagent_replayed_child_adjusted".to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            session
                .metrics
                .disclosure_counters
                .entry("codex_multiagent_replayed_tokens_dropped".to_string())
                .and_modify(|c| *c = c.saturating_add(dropped_usize))
                .or_insert(dropped_usize);

            stats.children_adjusted += 1;
            stats.tokens_input_dropped += dropped_input;
            stats.tokens_cache_read_dropped += before_cache_r
                .saturating_sub(session.metrics.tokens_cache_r)
                .max(0);
            stats.tokens_cache_write_dropped += before_cache_w
                .saturating_sub(session.metrics.tokens_cache_w)
                .max(0);
            stats.tokens_output_dropped += before_output
                .saturating_sub(session.metrics.tokens_output)
                .max(0);
            stats.tokens_reasoning_dropped += before_reasoning
                .saturating_sub(session.metrics.tokens_reasoning)
                .max(0);
        }
    }
    stats
}

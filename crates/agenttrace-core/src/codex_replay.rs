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
//! Within a family, each member's DIRECT parent is resolved by snapshot
//! evidence (rm-402): a replay child's opening billing — its FIRST
//! cumulative `total_token_usage` snapshot — contains the state it spawned
//! from, so a candidate that started earlier (by the rollout FILE's own
//! first timestamp, re-derived from disk so the session cache's
//! serde-dropped `metrics.timestamps` cannot invert the roles on a warm
//! run — review af521003 F1 — with the canonical path as the
//! deterministic tie-break) and whose FINAL snapshot is component-wise ≤
//! the child's opening billing is a plausible parent, and the LARGEST
//! such candidate is the closest true ancestor. The pre-fix pass instead
//! rebased every member onto the family-earliest: correct for one parent
//! with same-generation children, but a chain A→B→C kept A's replayed
//! prefix inside C's total (+25% on the acceptance chain) and a template
//! sibling became a fake parent that zeroed its siblings (−67%). A
//! member with no plausible parent — no candidate's final fits under its
//! opening billing — is not a replay child of anyone present: it stands
//! alone, keeping its full usage (conservative), and a child whose
//! opening sits below every candidate's final is the concurrent-spawn
//! shape below. Equal-final stubs (totals alone cannot separate a
//! zero-own-work replay child from a sibling stub) additionally require
//! content containment — the child's opening must still hash-match the
//! candidate's FULL item sequence, which byte-identical duplicates do
//! and distinct-content stubs do not. A rebased child inherits its
//! direct parent's final snapshot as its baseline, saturating per field:
//! the child keeps only the tokens it added, and the replayed prefix
//! stays counted once, attributed to the parent.
//!
//! A child whose final snapshot is below the parent's on ANY field is NOT
//! rebased (review af521003 F3): that is the concurrent-spawn shape — the
//! child spawned mid-parent and replayed the parent's state at spawn time,
//! so the parent's final snapshot is not a valid baseline and a saturating
//! delta would silently erase the child's real usage while mislabeling it
//! as replayed tokens. Such children keep their full parsed usage and
//! carry a distinct `codex_multiagent_replay_skipped_below_baseline`
//! disclosure; the family total then overcounts (conservative) rather
//! than destroying data. A member whose file cannot be re-read for
//! evidence at all — a non-UTF8 rollout name arrives lossy through the
//! parser's path seam and no longer resolves on disk (rm-1916) — used to
//! leave its family SILENTLY, double-counting whatever prefix it
//! contains with no disclosure; it now carries
//! `codex_multiagent_replay_member_unreadable` and keeps its full usage.
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
    /// Family members whose stored path could not be resolved back to a
    /// file during evidence scanning (rm-1916: non-UTF8 rollout names
    /// arrive lossy through the parser's path seam, and a file deleted
    /// between discovery and reconciliation arrives dangling). Such a
    /// member used to leave its family SILENTLY — the replay prefix it
    /// contains (or that is replayed from it) can double-count with no
    /// disclosure. It is now disclosed on its own session and kept fully
    /// counted (conservative).
    pub members_unreadable: usize,
    /// Child rollouts whose usage was rebased onto the parent's final snapshot.
    pub children_adjusted: usize,
    /// Family children left untouched because they carry upstream-recorded
    /// costs (token-derived recompute would be dishonest for them).
    pub children_skipped_upstream: usize,
    /// Family children left untouched because the parent's final snapshot
    /// exceeds theirs on some field (concurrent-spawn shape) — a saturating
    /// rebase would erase real usage (review af521003 F3).
    pub children_below_baseline: usize,
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

    /// Component-wise `<=`: every field at most the other's (rm-402 —
    /// plausibility of one snapshot sitting inside another).
    fn le_all(self, other: Self) -> bool {
        self.input <= other.input
            && self.cached <= other.cached
            && self.cache_creation <= other.cache_creation
            && self.output <= other.output
            && self.reasoning <= other.reasoning
    }

    /// Component-wise equality on every wire field (rm-402 stub-guard).
    fn eq_all(self, other: Self) -> bool {
        self.input == other.input
            && self.cached == other.cached
            && self.cache_creation == other.cache_creation
            && self.output == other.output
            && self.reasoning == other.reasoning
    }

    /// Deterministic maximality key for choosing among qualifying
    /// parent candidates: input-first lexicographic over the five
    /// classes (rm-402 — the closest ancestor subsumes the others).
    fn lex_key(self) -> (i64, i64, i64, i64, i64) {
        (
            self.input,
            self.cached,
            self.cache_creation,
            self.output,
            self.reasoning,
        )
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
    /// FIRST cumulative `total_token_usage` snapshot — the child's opening
    /// billing, i.e. the state it spawned from (rm-402 direct-parent
    /// evidence). `None` when the file carries no total snapshot at all.
    first_total: Option<TotalSnapshot>,
    /// Chained hash after EVERY `response_item` (index n-1 = fingerprint of
    /// the first n items) — content containment for the rm-402 stub-guard:
    /// a candidate's full sequence is inside a child's opening iff the
    /// child's chain at the candidate's length equals the candidate's last
    /// chain value.
    chain: Vec<u64>,
    /// First RFC3339 `timestamp` seen on any line of the file — the
    /// session's true start, re-derived from disk so it survives the
    /// session-cache round-trip that drops `metrics.timestamps`
    /// (`#[serde(skip)]`, review af521003 F1).
    first_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Total `response_item` lines (not just fingerprinted ones) — a
    /// replay child by definition continues past the shared opening.
    response_items: usize,
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
        chain: Vec::new(),
        first_timestamp: None,
        response_items: 0,
    };
    let Ok(mut raw) = std::fs::read(path) else {
        return scan;
    };
    // The zstd magic `28 B5 2F FD` — same probe as `parse_session_bytes`
    // (rm-992). Plain rollouts take the direct path byte-for-byte.
    if raw.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        let label = path.display().to_string();
        match crate::parser::decode_zstd_rollout_bounded(raw, &label) {
            Ok(decoded) => raw = decoded,
            Err(_) => return scan,
        }
    }
    let Ok(text) = String::from_utf8(raw) else {
        return scan;
    };
    let mut hasher = DefaultHasher::new();
    let mut chain_hasher = DefaultHasher::new();
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
                // Stable textual form of (type, payload) — byte-identical
                // openings hash identically regardless of key order.
                let mut item_hasher = DefaultHasher::new();
                payload.hash(&mut item_hasher);
                let payload_hash = item_hasher.finish();
                // rm-402: chain over the WHOLE sequence, not just the
                // opening — prefix chains feed the stub-guard's content
                // containment check.
                payload_hash.hash(&mut chain_hasher);
                scan.chain.push(chain_hasher.finish());
                if items < MAX_FINGERPRINT_ITEMS {
                    payload_hash.hash(&mut hasher);
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
                            scan.total = TotalSnapshot {
                                input: get("input_tokens"),
                                cached: get("cached_input_tokens"),
                                cache_creation: get("cache_creation_input_tokens"),
                                output: get("output_tokens"),
                                reasoning: get("reasoning_output_tokens"),
                            };
                            // rm-402: the first cumulative snapshot is the
                            // child's opening billing (what it spawned from).
                            if scan.first_total.is_none() {
                                scan.first_total = Some(scan.total);
                            }
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
        /// Opening billing — the FIRST cumulative snapshot (rm-402).
        first_total: TotalSnapshot,
        /// Chained per-item prefix hashes over the whole sequence (rm-402
        /// stub-guard: content containment).
        chain: Vec<u64>,
        fingerprint_items: usize,
        response_items: usize,
    }
    let mut families: HashMap<OpeningFingerprint, Vec<Evidence>> = HashMap::new();
    for (idx, session) in sessions.iter_mut().enumerate() {
        if session.metrics.source_tool != "codex_cli" {
            continue;
        }
        let Ok(path) = Path::new(&session.path).canonicalize() else {
            // rm-1916: the stored path does not resolve back to a file —
            // a non-UTF8 rollout name arrives lossy through the parser's
            // `to_string_lossy` seam, and a file deleted between discovery
            // and reconciliation arrives dangling. Either way this member
            // used to leave its replay family SILENTLY, so the prefix it
            // replays (or that is replayed from it) could double-count
            // with no disclosure. Disclose on the session itself and keep
            // its usage fully counted (conservative).
            session
                .metrics
                .disclosure_counters
                .entry("codex_multiagent_replay_member_unreadable".to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            stats.members_unreadable += 1;
            continue;
        };
        let scan = scan_rollout(&path);
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
            first_total: scan.first_total.unwrap_or(scan.total),
            chain: scan.chain,
            fingerprint_items: fingerprint.items,
            response_items: scan.response_items,
        });
    }

    // Pass 2: per family, resolve each member's DIRECT parent and rebase
    // onto it (rm-402). The pre-fix pass treated the earliest-starting
    // member as the parent of everyone — correct for one parent with
    // same-generation children, wrong for chains (A→B→C rebased C onto
    // A, keeping A's replayed prefix inside C's total: +25% on the
    // acceptance chain) and destructive for template siblings (the
    // earliest sibling became a fake parent that zeroed the rest).
    //
    // Direct-parent evidence, per child: a replay child's opening billing
    // — its FIRST cumulative snapshot — contains the state it spawned
    // from, so a candidate that started earlier and whose FINAL snapshot
    // sits component-wise under that opening billing is a plausible
    // parent, and the LARGEST such candidate is the closest true ancestor
    // (in a chain both A and B fit under C's opening, and B's final
    // subsumes A's). No plausible parent ⇒ the member stands alone
    // (template sibling / missing-parent shape — full usage kept); every
    // candidate's final exceeding the opening billing ⇒ concurrent-spawn
    // shape (af521003 F3 — kept intact and disclosed).
    enum Role {
        Standalone,
        Child { parent: usize },
        BelowBaseline,
        UpstreamPriced,
    }
    // rm-402 stub-guard: does `child`'s transcript open with `parent`'s
    // FULL item sequence? The chained prefix hashes make this one
    // comparison per pair.
    let opening_contains = |child: &Evidence, parent: &Evidence| -> bool {
        match (
            parent.chain.last(),
            child.chain.get(parent.chain.len().wrapping_sub(1)),
        ) {
            (None, _) => true,
            (Some(&full), Some(&at_len)) => full == at_len,
            (Some(_), None) => false,
        }
    };
    for (_fp, mut members) in families {
        if members.len() < 2 {
            continue;
        }
        stats.families += 1;
        // Deterministic iteration order; parent choice itself is
        // evidence-based, not order-based.
        members.sort_by(|a, b| {
            a.started_at
                .cmp(&b.started_at)
                .then_with(|| a.path.cmp(&b.path))
        });
        let mut roles: Vec<Role> = Vec::with_capacity(members.len());
        for i in 0..members.len() {
            let child = &members[i];
            // The landed upstream/F4 arms keep their pre-fix breadth
            // (every member but the family root); the root has no earlier
            // candidate and stands alone by construction.
            if i > 0 {
                if sessions[child.session_idx].metrics.upstream_cost_usd > 0.0 {
                    roles.push(Role::UpstreamPriced);
                    continue;
                }
                // Review af521003 F4 (second discriminator): a member whose
                // whole transcript IS the shared opening continued past
                // nothing — it is the template/retry shape, not a replay
                // child doing its own work. Non-membership is silent and
                // conservative: its usage stays fully counted.
                if child.response_items <= child.fingerprint_items {
                    roles.push(Role::Standalone);
                    continue;
                }
            }
            let mut best: Option<usize> = None;
            let mut exceeded = false;
            for j in 0..members.len() {
                if j == i {
                    continue;
                }
                let candidate = &members[j];
                // A replay parent necessarily started before its child
                // (the child replays a transcript that already existed);
                // the tuple comparison also breaks timestamp ties
                // deterministically and rejects mutual-parent cycles.
                if (candidate.started_at, &candidate.path) >= (child.started_at, &child.path) {
                    continue;
                }
                if candidate.total.le_all(child.first_total) {
                    let better = match best {
                        None => true,
                        Some(b) => {
                            let incumbent = members[b].total.lex_key();
                            let challenger = candidate.total.lex_key();
                            challenger > incumbent
                                || (challenger == incumbent
                                    && (candidate.started_at, &candidate.path)
                                        < (members[b].started_at, &members[b].path))
                        }
                    };
                    if better {
                        best = Some(j);
                    }
                } else {
                    exceeded = true;
                }
            }
            roles.push(match best {
                Some(parent) => {
                    // Stub-guard: when the child's FINAL equals the
                    // candidate's final on every field, the totals alone
                    // cannot separate a zero-own-work replay child from a
                    // sibling stub — accept only on content evidence (the
                    // child's opening must still contain the candidate's
                    // full item sequence; byte-identical duplicates pass,
                    // distinct-content stubs stand alone).
                    if members[parent].total.eq_all(child.total)
                        && !opening_contains(child, &members[parent])
                    {
                        Role::Standalone
                    } else {
                        Role::Child { parent }
                    }
                }
                None if exceeded => Role::BelowBaseline,
                None => Role::Standalone,
            });
        }
        for (i, role) in roles.into_iter().enumerate() {
            let child = &members[i];
            let session = &mut sessions[child.session_idx];
            match role {
                Role::UpstreamPriced => {
                    session
                        .metrics
                        .disclosure_counters
                        .entry("codex_multiagent_replay_skipped_upstream_priced".to_string())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                    stats.children_skipped_upstream += 1;
                }
                Role::BelowBaseline => {
                    // Review af521003 F3: concurrent-spawn shape — no
                    // candidate's final fits under this child's opening
                    // billing, and at least one exceeds it: the child
                    // spawned mid-parent and replayed the parent's state
                    // at spawn time. Keep the child's usage intact and
                    // disclose distinctly instead of saturating real
                    // usage to zero under a replay label.
                    session
                        .metrics
                        .disclosure_counters
                        .entry("codex_multiagent_replay_skipped_below_baseline".to_string())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                    stats.children_below_baseline += 1;
                }
                Role::Standalone => {}
                Role::Child { parent } => {
                    let parent_total = members[parent].total;
                    // af521003 F3 defense in depth: qualification used
                    // the child's OPENING billing; non-monotonic wire data
                    // can still leave the child's final below the chosen
                    // parent's state — keep the usage intact and disclose.
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
        }
    }
    stats
}

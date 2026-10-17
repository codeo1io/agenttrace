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
//! * Family evidence additionally carries a `session_meta` id trace key
//!   (the id's uuid groups minus the final node group — the MultiAgent V2
//!   family stem: `rollout-<ts>-01a0d1bb-0000-7000-8000-000000000001` →
//!   `01a0d1bb-0000-7000-8000`; the leading rollout timestamp and the
//!   trailing node sequence vary per file). The key is a
//!   VETO, never a tie-breaker: members sharing an opening but carrying
//!   two or more distinct stems are the independent-rerun shape (a fresh
//!   rollout re-executing the same deterministic opening) and are left
//!   fully counted with a disclosure instead of being rebased — a
//!   rebase there would erase the rerun's real usage (rm-1980).
//!   Keyless files (no session_meta id) keep the fingerprint-only
//!   fallback, disclosed. Codex 0.164 fork events (history-prefix forks,
//!   #52535) replay recorded lines into a fresh file, so they meet this
//!   pass as either shape; the veto keeps the destructive arm closed.
//! * Scans are memoized in an on-disk `RolloutScanStore` beside the
//!   session cache (rm-1979): every warm run used to re-read and
//!   re-parse the whole codex corpus even when every session came from
//!   the session cache (measured: the entire warm-run residual on a
//!   49 MiB corpus). Entries re-validate per file by size+mtime, so a
//!   stale store can only cost a re-read — never a wrong decision.
//! * Plain (non-zstd) rollouts are read under a byte cap like every
//!   other bounded intake read; an over-cap file is skipped with a
//!   disclosure, never partially read.

use crate::{lookup_price_tiered, round4, Metrics, Session};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
    /// Families vetoed before rebasing: members share an opening but carry
    /// two or more distinct `session_meta` id stems — the independent-rerun
    /// shape. Their usage is preserved and disclosed (rm-1980).
    pub divergent_trace_families: usize,
    /// Families reconciled with no `session_meta` id evidence at all
    /// (truncated/degenerate rollouts): fingerprint-only fallback, disclosed.
    pub unkeyed_families: usize,
    /// Rollout files the scanner could not read at reconcile time.
    pub scans_unreadable: usize,
    /// Rollout files skipped for exceeding `ROLLOUT_PLAIN_SCAN_MAX_BYTES`.
    pub scans_skipped_oversized: usize,
    /// Scans served from the on-disk `RolloutScanStore` (warm runs).
    pub scan_cache_hits: usize,
    /// Scans re-read from the rollout files (cold, changed, or over-cap).
    pub scan_cache_misses: usize,
    /// Replayed tokens removed from children, per class.
    pub tokens_input_dropped: i64,
    pub tokens_cache_read_dropped: i64,
    pub tokens_cache_write_dropped: i64,
    pub tokens_output_dropped: i64,
    pub tokens_reasoning_dropped: i64,
}

/// Cumulative `total_token_usage` snapshot of one rollout, per wire field.
/// Serialized into the `RolloutScanStore` (rm-1979).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct OpeningFingerprint {
    hash: u64,
    items: usize,
    bytes: usize,
}

/// Why a scan ended without full evidence. `Scanned` is the happy path;
/// the other two are disclosed on the session and counted in
/// `CodexReplayAdjustment` (rm-1980 F3/F2).
#[derive(Debug, Clone, PartialEq, Eq)]
enum ScanStatus {
    Scanned,
    /// `fs::read` or zstd decode failed at reconcile time (file deleted,
    /// permissions, corrupt frame). The parser already failed loudly at
    /// parse time; this discloses the reconcile-side gap instead of
    /// silently rendering the corpus unreconciled.
    Unreadable,
    /// Plain rollout larger than `ROLLOUT_PLAIN_SCAN_MAX_BYTES` — skipped
    /// whole, never partially read.
    Oversized {
        size: u64,
    },
}

/// Upper bound on the raw bytes a plain (uncompressed) rollout may
/// occupy before the replay scanner refuses to read it. Same discipline
/// as `ZSTD_ROLLOUT_DECODE_MAX_BYTES` (decompressed) and
/// `MCP_INPUT_MAX_BYTES`: caps are consts, changes ride their own review
/// — never a hand-written literal at a read site.
pub const ROLLOUT_PLAIN_SCAN_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Everything the reconciliation pass needs from one rollout file.
pub(crate) struct RolloutScan {
    fingerprint: Option<OpeningFingerprint>,
    /// Final cumulative `total_token_usage` snapshot (the file's last one).
    total: TotalSnapshot,
    /// First RFC3339 `timestamp` seen on any line of the file — the
    /// session's true start, re-derived from disk so it survives the
    /// session-cache round-trip that drops `metrics.timestamps`
    /// (`#[serde(skip)]`, review af521003 F1).
    first_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Total `response_item` lines (not just fingerprinted ones) — a
    /// replay child by definition continues past the shared opening.
    response_items: usize,
    /// `session_meta` id trace key (id minus its final `-` segment):
    /// the family stem. `None` when the file carries no usable id.
    trace_key: Option<String>,
    /// How the scan ended — drives disclosure of unreadable/oversized
    /// rollouts instead of the old silent no-evidence path.
    status: ScanStatus,
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
    scan_rollout_with_cap(path, ROLLOUT_PLAIN_SCAN_MAX_BYTES)
}

fn scan_rollout_with_cap(path: &Path, max_plain_bytes: u64) -> RolloutScan {
    let mut scan = RolloutScan {
        fingerprint: None,
        total: TotalSnapshot::default(),
        first_timestamp: None,
        response_items: 0,
        trace_key: None,
        status: ScanStatus::Unreadable,
    };
    // rm-1979: refuse over-cap plain rollouts BEFORE reading — a bounded
    // partial read would fingerprint a truncated opening and could tie
    // two over-cap files on their shared prefix. Skip whole, disclose.
    let file_len = match std::fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(_) => return scan,
    };
    if file_len > max_plain_bytes {
        scan.status = ScanStatus::Oversized { size: file_len };
        return scan;
    }
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
            "session_meta" => {
                // rm-1980: the rollout's own id is the family stem once its
                // trailing sequence group is stripped (`…-8000-0001` →
                // `…-8000`). Used only as a rebase veto, never a tie-breaker.
                if scan.trace_key.is_none() {
                    if let Some(id) = payload.get("id").and_then(|v| v.as_str()) {
                        scan.trace_key = session_meta_trace_key(id);
                    }
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
    scan.status = ScanStatus::Scanned;
    scan
}

/// Path of the on-disk rollout-scan memo: beside the session cache so it
/// shares its directory lifecycle (`~/.cache/agenttrace/codex-rollout-scans.json`).
pub fn rollout_scan_store_path() -> PathBuf {
    crate::session_cache::session_cache_path()
        .parent()
        .map(|dir| dir.join("codex-rollout-scans.json"))
        .unwrap_or_else(|| PathBuf::from("codex-rollout-scans.json"))
}

/// On-disk memo of `RolloutScan`s keyed by canonical rollout path
/// (rm-1979). Warm runs re-read the whole codex corpus on every pass —
/// the only full-corpus I/O left after a session-cache hit — so scans
/// persist here and re-validate per file via size+mtime before use. A
/// stale, lost, or partially-written store costs a re-read, never a
/// wrong family decision; correctness lives in the fingerprint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct StoredRolloutScan {
    size: u64,
    mtime_secs: i64,
    mtime_nanos: u32,
    scan: PersistedScan,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct PersistedScan {
    fingerprint: Option<OpeningFingerprint>,
    total: TotalSnapshot,
    first_timestamp: Option<String>,
    response_items: usize,
    trace_key: Option<String>,
}

impl From<&RolloutScan> for PersistedScan {
    fn from(scan: &RolloutScan) -> Self {
        PersistedScan {
            fingerprint: scan.fingerprint,
            total: scan.total,
            first_timestamp: scan.first_timestamp.map(|ts| ts.to_rfc3339()),
            response_items: scan.response_items,
            trace_key: scan.trace_key.clone(),
        }
    }
}

impl PersistedScan {
    fn hydrate(&self) -> RolloutScan {
        RolloutScan {
            fingerprint: self.fingerprint,
            total: self.total,
            first_timestamp: self.first_timestamp.as_ref().and_then(|ts| {
                chrono::DateTime::parse_from_rfc3339(ts)
                    .ok()
                    .map(|t| t.with_timezone(&chrono::Utc))
            }),
            response_items: self.response_items,
            trace_key: self.trace_key.clone(),
            status: ScanStatus::Scanned,
        }
    }
}

/// Bounded so the memo cannot grow unbounded on rotating corpora.
const ROLLOUT_SCAN_STORE_MAX_ENTRIES: usize = 4096;
pub const ROLLOUT_SCAN_STORE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Default)]
pub struct RolloutScanStore {
    schema_version: u32,
    entries: BTreeMap<String, StoredRolloutScan>,
    dirty: bool,
}

impl RolloutScanStore {
    /// Fresh empty memo (also the fallback for a missing, unreadable, or
    /// version-mismatched file).
    pub fn new() -> Self {
        Self {
            schema_version: ROLLOUT_SCAN_STORE_SCHEMA_VERSION,
            entries: BTreeMap::new(),
            dirty: false,
        }
    }

    pub fn load(path: &Path) -> Self {
        let Ok(raw) = std::fs::read(path) else {
            return Self::new();
        };
        match serde_json::from_slice::<RolloutScanStoreDocument>(&raw) {
            Ok(doc) if doc.schema_version == ROLLOUT_SCAN_STORE_SCHEMA_VERSION => {
                let mut store = Self {
                    schema_version: doc.schema_version,
                    entries: doc.entries,
                    dirty: false,
                };
                store.prune_missing_files();
                store
            }
            _ => Self::new(),
        }
    }

    /// Cached scan for `path`, but only when the file on disk still
    /// matches the recorded size+mtime — anything else re-reads.
    pub(crate) fn lookup(&self, path: &Path) -> Option<RolloutScan> {
        let entry = self.entries.get(path.to_string_lossy().as_ref())?;
        let meta = std::fs::metadata(path).ok()?;
        if meta.len() != entry.size {
            return None;
        }
        let (secs, nanos) = mtime_parts(meta.modified().ok()?);
        if secs != entry.mtime_secs || nanos != entry.mtime_nanos {
            return None;
        }
        Some(entry.scan.hydrate())
    }

    /// Memoize one scan. Only complete (`Scanned`) evidence persists —
    /// unreadable/oversized results re-derive cheaply and stay disclosed.
    pub(crate) fn record(&mut self, path: &Path, scan: &RolloutScan) {
        if scan.status != ScanStatus::Scanned {
            return;
        }
        let Ok(meta) = std::fs::metadata(path) else {
            return;
        };
        let (secs, nanos) = match meta.modified() {
            Ok(mtime) => mtime_parts(mtime),
            Err(_) => (0, 0),
        };
        self.entries.insert(
            path.to_string_lossy().into_owned(),
            StoredRolloutScan {
                size: meta.len(),
                mtime_secs: secs,
                mtime_nanos: nanos,
                scan: PersistedScan::from(scan),
            },
        );
        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn prune_missing_files(&mut self) {
        let before = self.entries.len();
        self.entries.retain(|path, _| Path::new(path).exists());
        if self.entries.len() != before {
            self.dirty = true;
        }
    }

    /// Persist atomically (tempfile + rename) if anything changed. The
    /// memo is advisory: concurrent writers race last-writer-wins and a
    /// lost write only costs re-reads.
    pub fn save_if_dirty(&mut self, path: &Path) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if self.entries.len() > ROLLOUT_SCAN_STORE_MAX_ENTRIES {
            let mut order: Vec<(String, (i64, u32))> = self
                .entries
                .iter()
                .map(|(p, e)| (p.clone(), (e.mtime_secs, e.mtime_nanos)))
                .collect();
            order.sort_by_key(|(_, m)| *m);
            let excess = self.entries.len() - ROLLOUT_SCAN_STORE_MAX_ENTRIES;
            for (p, _) in order.into_iter().take(excess) {
                self.entries.remove(&p);
            }
        }
        let doc = RolloutScanStoreDocument {
            schema_version: ROLLOUT_SCAN_STORE_SCHEMA_VERSION,
            entries: std::mem::take(&mut self.entries),
        };
        let bytes = serde_json::to_vec(&doc)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, path)?;
        self.entries = doc.entries;
        self.schema_version = ROLLOUT_SCAN_STORE_SCHEMA_VERSION;
        self.dirty = false;
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RolloutScanStoreDocument {
    schema_version: u32,
    entries: BTreeMap<String, StoredRolloutScan>,
}

/// Family identity from a `session_meta` id: the id's uuid groups minus
/// the final node group. `rollout-<timestamp>-<uuid4>-<node>` carries the
/// family stem in the uuid's first four groups — a replay child re-mints
/// the same uuid with the next node, while an independent rerun mints a
/// fresh uuid (rm-1980).
fn session_meta_trace_key(id: &str) -> Option<String> {
    let groups: Vec<&str> = id.split('-').collect();
    if groups.len() < 5 {
        return None;
    }
    Some(groups[groups.len() - 5..groups.len() - 1].join("-"))
}

fn mtime_parts(mtime: SystemTime) -> (i64, u32) {
    match mtime.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i64, d.subsec_nanos()),
        Err(e) => {
            let d = e.duration();
            (-(d.as_secs() as i64), d.subsec_nanos())
        }
    }
}

/// Recompute a child's estimated cost from its rebased token classes with
/// the exact per-Mtok catalog formula the `analyze` pass applies
/// (lib.rs `catalog_cost`: every class divided by 1e6 at its own rate;
/// `reasoning` is a breakdown of `output` on this wire — rm-603 — so it
/// carries no separate term, exactly like the canonical formula).
fn rebase_cost(metrics: &Metrics, uncached_input: i64, rebased: &TotalSnapshot) -> f64 {
    let price = lookup_price_tiered(&metrics.model_used, metrics.service_tier.as_deref());
    // rm-1508 seam at the 2026-10-17 integration (case 7f566ee8): this
    // master-side replay formula was written when cw/cr were plain f64;
    // the landed absence-preserving catalog made them Option, so an
    // unpublished rate here must bill at the disclosed fallback ladder
    // (cr = input rate, cw = input x the vendor-convention multiple)
    // exactly like the canonical `catalog_cost` — never a silent $0.
    let (cw, _) = price.cache_write_billed();
    let (cr, _) = price.cache_read_billed();
    round4(
        uncached_input as f64 / 1e6 * price.input
            + rebased.output as f64 / 1e6 * price.output
            + rebased.cache_creation as f64 / 1e6 * cw
            + rebased.cached as f64 / 1e6 * cr,
    )
}

/// Reconcile codex MultiAgent V2 replay families in a discovered corpus.
///
/// Runs after all sessions are assembled and before any view is prepared.
/// Mutates only children of detected families; everything else — including
/// every single-file corpus — is left byte-identical.
pub fn reconcile_codex_multiagent_replays(sessions: &mut [Session]) -> CodexReplayAdjustment {
    reconcile_codex_multiagent_replays_with_store(sessions, None)
}

/// `reconcile_codex_multiagent_replays` with an optional on-disk scan
/// memo (rm-1979): `None` re-reads every rollout (tests, single-shot
/// paths); `Some` hydrates unchanged files from the store and records
/// fresh scans back into it. The store never changes a decision — see
/// `RolloutScanStore`.
pub fn reconcile_codex_multiagent_replays_with_store(
    sessions: &mut [Session],
    mut scan_store: Option<&mut RolloutScanStore>,
) -> CodexReplayAdjustment {
    let mut stats = CodexReplayAdjustment::default();

    // Pass 1: scan codex sessions for family evidence.
    struct Evidence {
        session_idx: usize,
        started_at: chrono::DateTime<chrono::Utc>,
        path: PathBuf,
        total: TotalSnapshot,
        fingerprint_items: usize,
        response_items: usize,
        trace_key: Option<String>,
    }
    let mut families: HashMap<OpeningFingerprint, Vec<Evidence>> = HashMap::new();
    let mut pending_scan_disclosures: Vec<(usize, &'static str)> = Vec::new();
    for (idx, session) in sessions.iter().enumerate() {
        if session.metrics.source_tool != "codex_cli" {
            continue;
        }
        // rm-1980 F3: a rollout that parsed but no longer resolves is the
        // silent-unreconciled shape — disclosed here, not swallowed.
        let path = match Path::new(&session.path).canonicalize() {
            Ok(path) => path,
            Err(_) => {
                stats.scans_unreadable += 1;
                pending_scan_disclosures.push((idx, "codex_replay_scan_unreadable"));
                continue;
            }
        };
        // rm-1979: unchanged files hydrate from the scan memo; only real
        // misses pay the read+decode+parse.
        let mut scan = scan_store.as_ref().and_then(|st| st.lookup(&path));
        if scan.is_some() {
            stats.scan_cache_hits += 1;
        } else {
            stats.scan_cache_misses += 1;
            scan = Some(scan_rollout(&path));
            if let Some(store) = scan_store.as_deref_mut() {
                store.record(&path, scan.as_ref().expect("just scanned"));
            }
        }
        let scan = scan.expect("scan present on both paths");
        match &scan.status {
            ScanStatus::Scanned => {}
            ScanStatus::Unreadable => {
                stats.scans_unreadable += 1;
                pending_scan_disclosures.push((idx, "codex_replay_scan_unreadable"));
            }
            ScanStatus::Oversized { .. } => {
                stats.scans_skipped_oversized += 1;
                pending_scan_disclosures.push((idx, "codex_replay_scan_skipped_oversized"));
            }
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
            fingerprint_items: fingerprint.items,
            response_items: scan.response_items,
            trace_key: scan.trace_key,
        });
    }
    for (idx, key) in pending_scan_disclosures {
        bump_disclosure(sessions, idx, key, 1);
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
        // rm-1980 rerun veto: a shared opening across two or more DISTINCT
        // id stems is an independent rerun of the same task, not a replay
        // child — rebasing it onto the earliest run would erase the rerun's
        // real usage. Keep every member fully counted and disclose.
        let mut stems: Vec<&str> = members
            .iter()
            .filter_map(|m| m.trace_key.as_deref())
            .collect();
        stems.sort_unstable();
        stems.dedup();
        if stems.len() > 1 {
            for member in &members {
                bump_disclosure(
                    sessions,
                    member.session_idx,
                    "codex_multiagent_replay_skipped_divergent_trace",
                    1,
                );
            }
            stats.divergent_trace_families += 1;
            continue;
        }
        if stems.is_empty() {
            // No id evidence anywhere (truncated/degenerate rollouts):
            // fingerprint-only fallback — the shipped behavior — but no
            // longer silent.
            for member in &members {
                bump_disclosure(
                    sessions,
                    member.session_idx,
                    "codex_multiagent_replay_family_unkeyed",
                    1,
                );
            }
            stats.unkeyed_families += 1;
        }
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

fn bump_disclosure(sessions: &mut [Session], idx: usize, key: &str, add: usize) {
    if let Some(session) = sessions.get_mut(idx) {
        session
            .metrics
            .disclosure_counters
            .entry(key.to_string())
            .and_modify(|c| *c += add)
            .or_insert(add);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("codex-replay-impl-{}-{}", name, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{}.jsonl", name));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn oversized_plain_rollout_is_skipped_whole_with_status() {
        let payload = vec![b'a'; 4096];
        let path = write_tmp("oversized", &payload);
        let scan = scan_rollout_with_cap(&path, 1024);
        assert_eq!(
            scan.status,
            ScanStatus::Oversized { size: 4096 },
            "an over-cap plain rollout must be refused before any read"
        );
        assert!(scan.fingerprint.is_none());
        // And the production cap admits it (same file, real cap):
        let scan = scan_rollout(&path);
        assert_eq!(scan.status, ScanStatus::Scanned);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_rollout_reports_unreadable_status() {
        let path = std::env::temp_dir().join(format!(
            "codex-replay-impl-absent-{}.jsonl",
            std::process::id()
        ));
        let scan = scan_rollout(&path);
        assert_eq!(scan.status, ScanStatus::Unreadable);
        assert!(scan.fingerprint.is_none());
    }

    #[test]
    fn trace_key_strips_the_trailing_sequence_group() {
        let body = concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"rollout-2026-09-24T04-44-49-01a0d1bb-0000-7000-8000-000000000001\"}}\n",
            "{\"timestamp\":\"2026-09-24T04:44:49.000Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"hello\"}]}}\n"
        );
        let path = write_tmp("trace-key", body.as_bytes());
        let scan = scan_rollout(&path);
        assert_eq!(
            scan.trace_key.as_deref(),
            Some("01a0d1bb-0000-7000-8000"),
            "the leading rollout timestamp and trailing node sequence must              both be stripped; only the uuid's family stem remains"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn scan_store_roundtrip_and_invalidation() {
        let body = concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"rollout-2026-09-24T04-44-49-01a0d1bb-0000-7000-8000-000000000001\"}}\n",
            "{\"timestamp\":\"2026-09-24T04:44:49.000Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"warm-cache probe body that is long enough to fingerprint\"}]}}\n"
        );
        let path = write_tmp("store-roundtrip", body.as_bytes());
        let store_path = std::env::temp_dir().join(format!(
            "codex-replay-impl-store-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&store_path);

        let first = scan_rollout(&path);
        let mut store = RolloutScanStore::new();
        assert!(store.lookup(&path).is_none(), "empty store misses");
        store.record(&path, &first);
        assert!(store.is_dirty());
        store.save_if_dirty(&store_path).unwrap();
        assert!(!store.is_dirty());

        let loaded = RolloutScanStore::load(&store_path);
        assert_eq!(loaded.len(), 1, "persisted entry survives the round-trip");
        let hydrated = loaded.lookup(&path).expect("size+mtime match → hit");
        assert_eq!(hydrated.trace_key, first.trace_key);
        assert_eq!(hydrated.response_items, first.response_items);
        assert_eq!(hydrated.first_timestamp, first.first_timestamp);
        assert_eq!(hydrated.status, ScanStatus::Scanned);

        // Touching the file (new size) must invalidate the entry.
        let touched = body.replace("hello", "hello again");
        std::fs::write(&path, touched).unwrap();
        assert!(
            loaded.lookup(&path).is_none(),
            "a rewritten file must invalidate the memo entry"
        );

        // A version-mismatched document loads as empty, never panics.
        std::fs::write(&store_path, br#"{"schema_version":9999,"entries":{}}"#).unwrap();
        assert_eq!(RolloutScanStore::load(&store_path).len(), 0);

        let _ = std::fs::remove_file(&store_path);
        let _ = std::fs::remove_file(&path);
    }
}

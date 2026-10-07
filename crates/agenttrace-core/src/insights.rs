use crate::{parse_ts, pricing, total_tokens, Session};
use chrono::{DateTime, FixedOffset, Local, Offset, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct SessionComparison {
    pub outcome: &'static str,
    pub reasons: Vec<&'static str>,
}

pub fn compare_session_outcome(current: &Session, previous: &Session) -> SessionComparison {
    let outcome = match (
        current.metrics.duration_sec <= previous.metrics.duration_sec,
        current.metrics.cost_estimated <= previous.metrics.cost_estimated,
    ) {
        (true, true) => "faster_cheaper",
        (true, false) => "faster_costlier",
        (false, true) => "slower_cheaper",
        (false, false) => "slower_costlier",
    };
    let mut reasons = Vec::new();
    if current.metrics.tool_calls_fail < previous.metrics.tool_calls_fail {
        reasons.push("fewer_failures");
    }
    if current.diagnostics.loop_cost.total_loop_cost
        < previous.diagnostics.loop_cost.total_loop_cost
    {
        reasons.push("less_repeated_work");
    }
    if reasons.is_empty() {
        reasons.push("review_metric_changes");
    }
    SessionComparison { outcome, reasons }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TimeRange {
    Today,
    Days7,
    Days30,
    #[default]
    All,
}

/// Start of the calendar day containing `now` under a fixed UTC `offset`,
/// expressed as a UTC instant (rm-040). Fixed offsets carry no DST
/// ambiguity, so the mapping is total; on zone-transition days the anchor
/// can sit an hour off true local midnight, which is strictly better than
/// anchoring every non-UTC user to UTC midnight every day.
fn day_start_for_offset(now: DateTime<Utc>, offset: FixedOffset) -> DateTime<Utc> {
    let midnight = now
        .with_timezone(&offset)
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is always representable");
    midnight
        .and_local_timezone(offset)
        .single()
        .expect("fixed offsets have no DST ambiguity")
        .with_timezone(&Utc)
}

impl TimeRange {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "today" | "day" | "1d" => Some(Self::Today),
            "7d" | "week" | "weekly" => Some(Self::Days7),
            "30d" | "month" | "monthly" => Some(Self::Days30),
            "all" | "" => Some(Self::All),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Days7 => "7d",
            Self::Days30 => "30d",
            Self::All => "all",
        }
    }
    pub fn since(self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Today => {
                // rm-040: "today" is the reporting user's calendar day, not
                // the UTC one. UTC+9 users lost their local morning from
                // `--range today` (a 23:00Z session is 08:00 JST the next
                // day), and west-of-UTC users had yesterday evening counted
                // in. Anchor to local midnight, expressed as a UTC instant
                // for the comparison in session_matches_time_range.
                let offset = now.with_timezone(&Local).offset().fix();
                Some(day_start_for_offset(now, offset))
            }
            Self::Days7 => Some(now - chrono::Duration::days(7)),
            Self::Days30 => Some(now - chrono::Duration::days(30)),
            Self::All => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectIdentity {
    pub id: String,
    pub display_name: String,
    pub root: String,
    pub resolution: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReportScope {
    pub generated_at: String,
    pub range: String,
    pub earliest_session_at: String,
    pub latest_session_at: String,
    pub sessions_in_scope: usize,
    pub sources: Vec<SourceScope>,
    pub includes_sqlite_derived_sessions: bool,
    pub includes_preserved_history: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SourceScope {
    pub source: String,
    pub sessions: usize,
    pub tokens: i64,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DataHealth {
    pub discovered: usize,
    pub parsed: usize,
    /// Sessions that failed to parse (loader `skipped`); never counts
    /// sessions merely excluded by range/filters.
    pub skipped: usize,
    /// Sessions the loader discovered but the report scope excludes
    /// (time range, project/source/model filters). Separate from
    /// `skipped` so "Parse coverage N/M" is true for every range
    /// (pass-8 F8-2).
    pub out_of_scope: usize,
    pub cache_hits: usize,
    pub unknown_sources: usize,
    pub unknown_models: usize,
    pub fallback_pricing: usize,
    pub latest_session_at: String,
    pub confidence: String,
    pub with_tokens: usize,
    pub with_duration: usize,
    pub with_tools: usize,
    pub with_event_timing: usize,
    pub with_diagnostics: usize,
    /// Sessions whose token totals came from the authoritative columns
    /// stored on the session row instead of message aggregation.
    pub stored_totals_sessions: usize,
    /// Aggregate magnitude of stored-versus-derived token drift across
    /// those sessions (saturating, never negative).
    pub stored_totals_delta_tokens: i64,
    /// Sessions whose start time is unknown (empty or unparseable). They
    /// stay visible in time-ranged views instead of being dropped.
    pub unknown_time_sessions: usize,
    /// Parse lines lost inside otherwise-parsed sessions, by reason
    /// (pass-7 P7-1): `unparseable_line`, `event_schema`, `non_event`.
    /// Pure parse loss only — assumption disclosures live in
    /// `disclosures` (rm-538 moved the rm-450 workbuddy input-basis
    /// counters there), so nothing in this map is an assumption note
    /// and nothing here is "kept" rather than lost.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub line_skips: BTreeMap<String, usize>,
    /// Parse-time disclosure counters aggregated across sessions —
    /// journal facts and assumptions the accounting keeps visible
    /// without treating them as loss (rm-436/rm-437, pi-family
    /// journals): `pi_usage_entry:<kind>`, `pi_branches`,
    /// `pi_entry_skipped:<type>`, `pi_message_role:<role>`; plus the
    /// workbuddy input-basis notes (rm-450 minted, rm-538 moved):
    /// `workbuddy_input_basis:cache_subtracted`,
    /// `workbuddy_input_basis:zeroed_suspected_mismatch`. Facts the
    /// journal documents that the accounting deliberately does not
    /// count (or counts across all branches), plus input-basis
    /// assumptions, kept visible instead of silently dropped — and
    /// never degrading `confidence`, which keys on real parse loss
    /// only (rm-538). Empty for corpora without such journals, so
    /// clean report bytes are unchanged.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub disclosures: BTreeMap<String, usize>,
    /// Sessions whose estimated cost is not a finite number (poisoned
    /// or overflowing pricing inputs). Their costs render as null in
    /// reports; the count keeps the corruption visible (pass-8 F8-5).
    pub non_finite_costs: usize,
    /// SQLite-backed stores (hermes/opencode) whose read failed
    /// (rm-722): a transient SQLITE_BUSY/locked or unreadable database
    /// must be disclosed — those sessions are EXCLUDED from this report,
    /// never silently reported as a zero-session answer. Detail strings
    /// carry `<database>: <error>`. Populated by the CLI from the
    /// loader's failure registry; empty for clean runs, so report bytes
    /// are unchanged.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sqlite_store_failures: Vec<String>,
}

pub fn session_capability(session: &Session) -> &'static str {
    let m = &session.metrics;
    if !m.gaps_sec.is_empty()
        || !session.diagnostics.tool_latencies.is_empty()
        || session.diagnostics.context_utilization.estimated_total > 0
    {
        "detailed"
    } else if m.duration_sec > 0.0
        || m.tokens_input.saturating_add(m.tokens_output) > 0
        || m.tool_calls_total > 0
        || !matches!(m.model_used.as_str(), "" | "default" | "unknown")
    {
        "aggregate"
    } else {
        "limited"
    }
}

pub fn resolve_project(session: &Session) -> ProjectIdentity {
    let decoded;
    let raw = if !session.cwd.trim().is_empty() {
        session.cwd.trim()
    } else {
        match decode_agent_project_dir_detailed(&session.path) {
            DecodeOutcome::Verified(path) => {
                decoded = path;
                decoded.as_str()
            }
            // rm-240: the official >200-char form truncates the encoding
            // and appends a hash of the full path. The hash tail is
            // opaque-but-stable, so the identity keys on the raw encoded
            // name (unique per source path) while `root` exposes the
            // deepest VERIFIED prefix of the truncation. No git-root walk:
            // the prefix is only a partial reconstruction, and a repo
            // above the cut would mis-merge distinct long-named projects.
            DecodeOutcome::Truncated { prefix, encoded } => {
                let display_name = truncated_display_name(&prefix, &encoded);
                return ProjectIdentity {
                    id: encoded,
                    display_name,
                    root: prefix,
                    resolution: "encoded_truncated".to_string(),
                };
            }
            // rm-240: CLAUDE_CODE_PROJECT_DIR_NAME (v2.1.234+) files
            // transcripts under `projects/<custom-name>/` where the name
            // is an opaque label naming no path at all — it IS the stable
            // identity, not `unknown`.
            DecodeOutcome::Opaque { name } => {
                return ProjectIdentity {
                    id: name.clone(),
                    display_name: name,
                    root: String::new(),
                    resolution: "opaque_project_dir_name".to_string(),
                };
            }
            DecodeOutcome::None => "",
        }
    };
    if raw.is_empty() || raw.starts_with("history:") {
        return ProjectIdentity {
            id: "unknown".to_string(),
            display_name: "unknown".to_string(),
            root: String::new(),
            resolution: "unattributed".to_string(),
        };
    }
    let path = lexical_normalize(Path::new(raw));
    let (root, resolution) = git_root(&path)
        .map(|root| (root, "git_root".to_string()))
        .unwrap_or_else(|| (path.clone(), "cwd".to_string()));
    let display_name = root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown")
        .to_string();
    let root = root.to_string_lossy().to_string();
    ProjectIdentity {
        id: root.clone(),
        display_name,
        root,
        resolution,
    }
}

// Claude Code and Cursor name project folders after the workspace path with
// separators replaced by '-'. The encoding is not injective — a literal '-',
// '/', '.' or '_' inside a directory name all encode to '-' — so
// `-x-my-repo` may equally decode to `/x/my-repo`, `/x/my/repo`,
// `/x/my.repo` or `/x/my_repo`, and only host state can distinguish them.
//
// rm-381: the decode is deterministic and host-state-independent in its
// tie-break. The walk consumes the encoded components left to right and at
// each step takes the LONGEST run of remaining components that verifies as
// a single existing directory, with separator variants tried in the fixed
// order '-', '.', '_'. Both consequences are pinned by tests:
//   * a decoy that is merely a more-split spelling of the encoded name
//     (`/x/my/repo` planted next to the true `/x/my-repo`) can no longer
//     capture attribution the way the old greedy first-match decoder let
//     it, and
//   * an unrelated intermediate directory (`/<tmp>/agenttrace` shadowing
//     `/<tmp>/agenttrace-project-<pid>/…` fixtures) can no longer truncate
//     the walk into "unknown".
// `resolve_project` still prefers the transcript's own `cwd` when present
// (cwd-first), so this decoder only runs for sessions without one.
// Residual ambiguity — a verified alternative was shadowed by the
// longest-run rule at some step — is disclosed through
// `project_decode_status` / `--doctor` instead of being decided silently.
/// Outcome of reversing a transcript's encoded project directory
/// (rm-240): a complete verified decode, the official truncated+hash
/// form, an opaque custom name — or nothing.
enum DecodeOutcome {
    /// A complete verified decode of a '-'-encoded name.
    Verified(String),
    /// The official form for names over
    /// [`OFFICIAL_ENCODED_NAME_TRUNCATION`] chars: the encoding is cut to
    /// that length and a hash of the full path is appended. `prefix` is
    /// the deepest VERIFIED decode of the truncation; `encoded` is the
    /// raw on-disk name (the stable identity, unique per source path).
    Truncated { prefix: String, encoded: String },
    /// A CLAUDE_CODE_PROJECT_DIR_NAME-style opaque custom name: it names
    /// no path, so the name itself is the identity.
    Opaque { name: String },
    /// No attribution form recognized (not under `…/projects/<encoded>/`),
    /// or a path-shaped name that simply does not verify.
    None,
}

/// Official encoded-name truncation length: encoded project dir names
/// over 200 chars are cut to 200 and suffixed with a hash of the full
/// path (code.claude.com/docs/en/sessions), so everything past 200 is
/// opaque — stable per source path, never recoverable.
const OFFICIAL_ENCODED_NAME_TRUNCATION: usize = 200;

/// CLAUDE_CODE_PROJECT_DIR_NAME opaque custom names (v2.1.234+, used
/// with CLAUDE_CONFIG_DIR): 1-64 chars of `[a-zA-Z0-9_-]` that name no
/// path at all.
fn opaque_project_dir_name(name: &str) -> bool {
    let mut len = 0usize;
    name.chars().all(|c| {
        len += 1;
        c.is_ascii_alphanumeric() || c == '-' || c == '_'
    }) && (1..=64).contains(&len)
}

fn decode_agent_project_dir_detailed(session_path: &str) -> DecodeOutcome {
    let Some(parts) = encoded_project_components(session_path) else {
        return DecodeOutcome::None;
    };
    if let Some(walk) = decode_encoded_components(&parts, false) {
        return DecodeOutcome::Verified(walk.path);
    }
    let Some(raw) = encoded_project_dir_name(session_path) else {
        return DecodeOutcome::None;
    };
    if raw.len() > OFFICIAL_ENCODED_NAME_TRUNCATION
        && raw.is_char_boundary(OFFICIAL_ENCODED_NAME_TRUNCATION)
    {
        let truncation = &raw[..OFFICIAL_ENCODED_NAME_TRUNCATION];
        let truncated_parts: Vec<&str> = truncation.trim_start_matches('-').split('-').collect();
        return DecodeOutcome::Truncated {
            prefix: deepest_verified_prefix(&truncated_parts),
            encoded: raw.to_string(),
        };
    }
    // Path-shaped encodings carry a leading '-' (the encoded root
    // slash); a leading-dash-free label that matches the opaque-name
    // pattern is a CLAUDE_CODE_PROJECT_DIR_NAME custom name, not a
    // failed decode.
    if !raw.starts_with('-') && opaque_project_dir_name(raw) {
        return DecodeOutcome::Opaque {
            name: raw.to_string(),
        };
    }
    DecodeOutcome::None
}

/// Display name for the truncated+hash form: the deepest verified
/// component of the truncation when one exists, else the final
/// non-empty component of the truncation itself (the cut-off tail
/// fragment — an honest reconstruction of what the encoder kept).
fn truncated_display_name(prefix: &str, encoded: &str) -> String {
    if let Some(name) = prefix.rsplit('/').find(|component| !component.is_empty()) {
        return name.to_string();
    }
    encoded
        .get(..OFFICIAL_ENCODED_NAME_TRUNCATION)
        .unwrap_or(encoded)
        .rsplit('-')
        .find(|component| !component.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

/// The raw encoded project directory name of a transcript path
/// (`…/projects/<encoded>/…`), exactly as the encoder wrote it (leading
/// '-' included when the source was an absolute path).
fn encoded_project_dir_name(session_path: &str) -> Option<&str> {
    let path = Path::new(session_path);
    let dir = path.ancestors().find_map(|ancestor| {
        let parent = ancestor.parent()?;
        matches!(parent.file_name()?.to_str()?, "projects")
            .then(|| ancestor.file_name()?.to_str())
            .flatten()
    })?;
    (!dir.is_empty()).then_some(dir)
}

/// The `-`-encoded project directory components of a transcript path
/// (`…/projects/<encoded>/…`), with the leading '-' (the encoded root
/// slash) stripped the way the encoders write it.
fn encoded_project_components(session_path: &str) -> Option<Vec<&str>> {
    let encoded = encoded_project_dir_name(session_path)?.trim_start_matches('-');
    (!encoded.is_empty()).then(|| encoded.split('-').collect())
}

struct EncodedWalk {
    path: String,
    /// Other COMPLETE decodes of the same encoded name (in the fixed search
    /// order), recorded for disclosure (`--doctor`), never chosen.
    shadowed: Vec<String>,
}

/// Probe budget for one decode: caps worst-case host filesystem work while
/// leaving realistic corpora (a dozen components, a few ambiguous steps)
/// fully explored. The unambiguous walk alone costs ~3 probes per component
/// pair (O(n^2) stat() calls, microseconds each); the budget exists to
/// bound combinatorial ambiguity blowups, not linear scans. Deterministic —
/// the search order is fixed, so a budget-truncated scan reports the same
/// chosen path every run.
const DECODE_PROBE_BUDGET: usize = 1024;

/// Enumerates complete decodes of the encoded components in a fixed order —
/// at every step the longest run of remaining components is tried first,
/// then separator variants '-', '.', '_', then consuming an empty component
/// without movement — and records them into `out`. The first entry of `out`
/// is therefore the deterministic choice: it is exactly the path the greedy
/// longest-run walk yields whenever that walk completes, and when the greedy
/// walk dead-ends the search still finds a decode instead of falling back
/// to "unknown". `disclose` keeps scanning after the first hit so ambiguity
/// can be reported; without it the search stops at the first complete
/// decode, which yields the identical chosen path at a fraction of the
/// probes.
fn decode_encoded_components(parts: &[&str], disclose: bool) -> Option<EncodedWalk> {
    let mut out = Vec::new();
    let mut probes = 0usize;
    explore_encoded_components(parts, 0, Path::new("/"), &mut out, &mut probes, disclose);
    let mut iter = out.into_iter();
    let path = iter.next()?;
    Some(EncodedWalk {
        path,
        shadowed: iter.collect(),
    })
}

fn explore_encoded_components(
    parts: &[&str],
    idx: usize,
    current: &Path,
    out: &mut Vec<String>,
    probes: &mut usize,
    disclose: bool,
) {
    if out.len() > if disclose { 8 } else { 0 } || *probes > DECODE_PROBE_BUDGET {
        return;
    }
    if idx == parts.len() {
        out.push(current.to_string_lossy().to_string());
        return;
    }
    for len in (1..=parts.len() - idx).rev() {
        let joined = parts[idx..idx + len].join("-");
        // Separator variants '-', '.', '_' of the run. A dash-free run (a
        // single component, or consecutive empties) spells identically in
        // all three; dedupe so each distinct spelling is probed — and
        // recursed from — exactly once.
        let mut variants = Vec::with_capacity(3);
        for candidate in [
            joined.clone(),
            joined.replace('-', "."),
            joined.replace('-', "_"),
        ] {
            // An empty component (consecutive dashes) encodes either an
            // empty path segment or a literal dash inside a longer name;
            // on its own it moves nowhere, so it is consumed after the
            // runs have been tried, never probed.
            if !candidate.is_empty() && !variants.contains(&candidate) {
                variants.push(candidate);
            }
        }
        for candidate in variants {
            *probes += 1;
            let next = current.join(&candidate);
            if next.is_dir() {
                explore_encoded_components(parts, idx + len, &next, out, probes, disclose);
                if !disclose && !out.is_empty() {
                    return;
                }
            }
        }
    }
    // Consecutive dashes: consume the empty component without moving. Only
    // meaningful as the tail of the walk (a leading/middle empty inside a
    // run was already covered by the joined spellings above).
    if parts[idx].is_empty() {
        explore_encoded_components(parts, idx + 1, current, out, probes, disclose);
    }
}

/// Deepest VERIFIED decode of a truncated encoded name's components
/// (rm-240): the official encoder cuts names over
/// [`OFFICIAL_ENCODED_NAME_TRUNCATION`] chars, so only a prefix of the
/// component list is real. Walks with the same longest-run-first,
/// separator-variant order as `explore_encoded_components`, keeping the
/// deepest directory reached on any branch even when no branch completes
/// — the unrecoverable tail is disclosed through the
/// `encoded_truncated` resolution instead of collapsing to `unknown`.
/// Same probe budget and determinism guarantees as the complete walk.
fn deepest_verified_prefix(parts: &[&str]) -> String {
    struct Best {
        depth: usize,
        path: String,
    }
    fn explore(
        parts: &[&str],
        idx: usize,
        current: &Path,
        depth: usize,
        probes: &mut usize,
        best: &mut Best,
    ) {
        if depth > best.depth {
            best.depth = depth;
            best.path = current.to_string_lossy().to_string();
        }
        if idx == parts.len() || *probes > DECODE_PROBE_BUDGET {
            return;
        }
        for len in (1..=parts.len() - idx).rev() {
            let joined = parts[idx..idx + len].join("-");
            let mut variants = Vec::with_capacity(3);
            for candidate in [
                joined.clone(),
                joined.replace('-', "."),
                joined.replace('-', "_"),
            ] {
                if !candidate.is_empty() && !variants.contains(&candidate) {
                    variants.push(candidate);
                }
            }
            for candidate in variants {
                *probes += 1;
                let next = current.join(&candidate);
                if next.is_dir() {
                    explore(parts, idx + len, &next, depth + len, probes, best);
                }
            }
        }
        if parts[idx].is_empty() {
            explore(parts, idx + 1, current, depth, probes, best);
        }
    }
    let mut best = Best {
        depth: 0,
        path: "/".to_string(),
    };
    let mut probes = 0usize;
    explore(parts, 0, Path::new("/"), 0, &mut probes, &mut best);
    best.path
}

/// Outcome of reversing a transcript's `-`-encoded agent project
/// directory, for `--doctor` disclosure (rm-381).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectDecodeStatus {
    /// The decoder was not consulted: the session records a `cwd`, or the
    /// transcript does not live under a `…/projects/<encoded>/` directory.
    NotConsulted,
    /// Every step of the walk verified exactly one candidate.
    Resolved { path: String },
    /// The deterministic longest-run rule chose `path`, but at least one
    /// other COMPLETE decode of the same encoded name verified
    /// (`shadowed`): the attribution is defensible, not proven.
    Ambiguous { path: String, shadowed: Vec<String> },
    /// No complete verified decode: attribution falls back to "unknown".
    Unresolved { encoded: String },
    /// The official >200-char truncated+hash form (rm-240): no complete
    /// decode exists (the tail past the truncation is a hash of the full
    /// path), so attribution keys on the raw `encoded` name while
    /// `prefix` carries the deepest VERIFIED prefix of the truncation.
    Truncated { prefix: String, encoded: String },
    /// A CLAUDE_CODE_PROJECT_DIR_NAME-style opaque custom name (rm-240):
    /// the name labels no path, so it is the stable identity.
    Opaque { name: String },
}

/// Reports how a session without a `cwd` was attributed when its project
/// directory name is '-'-encoded — deterministic decode plus ambiguity
/// disclosure (rm-381). See `decode_agent_project_dir` for the rule and
/// its tie-break.
pub fn project_decode_status(session: &Session) -> ProjectDecodeStatus {
    if !session.cwd.trim().is_empty() {
        return ProjectDecodeStatus::NotConsulted;
    }
    let Some(parts) = encoded_project_components(&session.path) else {
        return ProjectDecodeStatus::NotConsulted;
    };
    match decode_agent_project_dir_detailed(&session.path) {
        DecodeOutcome::None => ProjectDecodeStatus::Unresolved {
            encoded: parts.join("-"),
        },
        DecodeOutcome::Verified(_) => {
            // Re-walk with disclosure on: the chosen path is identical
            // (same fixed search order), but the shadowed alternatives
            // are collected for the ambiguity report.
            match decode_encoded_components(&parts, true) {
                Some(EncodedWalk { path, shadowed }) if shadowed.is_empty() => {
                    ProjectDecodeStatus::Resolved { path }
                }
                Some(EncodedWalk { path, shadowed }) => {
                    ProjectDecodeStatus::Ambiguous { path, shadowed }
                }
                None => ProjectDecodeStatus::Unresolved {
                    encoded: parts.join("-"),
                },
            }
        }
        DecodeOutcome::Truncated { prefix, encoded } => {
            ProjectDecodeStatus::Truncated { prefix, encoded }
        }
        DecodeOutcome::Opaque { name } => ProjectDecodeStatus::Opaque { name },
    }
}

pub fn project_name(session: &Session) -> String {
    resolve_project(session).display_name
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::RootDir => out.push(Path::new("/")),
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::Normal(value) => out.push(value),
        }
    }
    out
}

// Project views resolve thousands of sessions per frame; the filesystem walk is memoized per cwd.
fn git_root(path: &Path) -> Option<PathBuf> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<PathBuf>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().ok().and_then(|map| map.get(path).cloned()) {
        return hit;
    }
    let found = find_git_root(path);
    if let Ok(mut map) = cache.lock() {
        map.insert(path.to_path_buf(), found.clone());
    }
    found
}

fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_dir() { path } else { path.parent()? };
    loop {
        let marker = current.join(".git");
        if marker.is_dir() {
            return Some(current.to_path_buf());
        }
        if marker.is_file() {
            return Some(worktree_main_root(&marker).unwrap_or_else(|| current.to_path_buf()));
        }
        current = current.parent()?;
    }
}

// Linked worktrees carry a `.git` file ("gitdir: <repo>/.git/worktrees/<name>"); group them
// under the main checkout so every agent worktree counts as the same project.
fn worktree_main_root(marker: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(marker).ok()?;
    let gitdir = PathBuf::from(content.strip_prefix("gitdir:")?.trim());
    let git = gitdir.parent()?.parent()?;
    (git.file_name()? == ".git").then(|| git.parent().map(Path::to_path_buf))?
}

pub fn filter_sessions(
    sessions: &[Session],
    range: TimeRange,
    project: &str,
    source: &str,
    model: &str,
    now: DateTime<Utc>,
) -> Vec<Session> {
    sessions
        .iter()
        .filter(|session| {
            session_matches_time_range(session, range, now)
                && project_matches(session, project)
                && contains(&session.metrics.source_tool, source)
                && contains(&session.metrics.model_used, model)
        })
        .cloned()
        .collect()
}
pub fn session_matches_time_range(session: &Session, range: TimeRange, now: DateTime<Utc>) -> bool {
    range.since(now).is_none_or(|since| {
        // Unknown start times stay visible (N7 unknown-time bucket);
        // only sessions with a known start before the cutoff drop out.
        parse_ts(&session.metrics.session_start)
            .map(|time| time >= since)
            .unwrap_or(true)
    })
}

/// Saturating f64 addition for per-source cost totals (rm-723): the
/// token arm of the same accumulator saturates since rm-529, but the
/// cost arm used plain `+=`, so a hostile journal pushing
/// `cost_estimated` values past the f64 boundary surfaced `inf` in
/// reports (per-session renderers null non-finite costs via
/// `non_finite_costs`; the aggregated source total had no such guard).
/// A finite sum above `f64::MAX` clamps to `f64::MAX`; a NaN addend
/// keeps the sum NaN — clamping it would silently "repair" poisoned
/// pricing inputs that the `non_finite_costs` disclosure exists to
/// flag.
fn saturating_add_cost(total: f64, addend: f64) -> f64 {
    let sum = total + addend;
    if sum.is_sign_positive() && sum > f64::MAX {
        f64::MAX
    } else {
        sum
    }
}

pub fn report_scope(
    sessions: &[Session],
    range: TimeRange,
    includes_preserved_history: bool,
) -> ReportScope {
    let mut sources: BTreeMap<String, SourceScope> = BTreeMap::new();
    let mut earliest = None;
    let mut latest = None;
    for session in sessions {
        if let Some(time) = parse_ts(&session.metrics.session_start) {
            earliest = Some(earliest.map_or(time, |current: DateTime<Utc>| current.min(time)));
            latest = Some(latest.map_or(time, |current: DateTime<Utc>| current.max(time)));
        }
        let entry = sources
            .entry(session.metrics.source_tool.clone())
            .or_insert_with(|| SourceScope {
                source: session.metrics.source_tool.clone(),
                ..SourceScope::default()
            });
        entry.sessions += 1;
        entry.tokens = entry.tokens.saturating_add(total_tokens(session));
        // rm-723: the cost arm joins the rm-529 token arms in saturating —
        // see `saturating_add_cost`.
        entry.estimated_cost =
            saturating_add_cost(entry.estimated_cost, session.metrics.cost_estimated);
    }
    ReportScope {
        generated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        range: range.label().to_string(),
        earliest_session_at: earliest
            .map(|time: DateTime<Utc>| time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .unwrap_or_default(),
        latest_session_at: latest
            .map(|time: DateTime<Utc>| time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .unwrap_or_default(),
        sessions_in_scope: sessions.len(),
        sources: sources.into_values().collect(),
        includes_sqlite_derived_sessions: sessions.iter().any(|session| {
            matches!(
                session.metrics.source_tool.as_str(),
                "hermes_db" | "opencode_db"
            )
        }),
        includes_preserved_history,
    }
}

pub fn data_health(sessions: &[Session], discovered: usize, cache_hits: usize) -> DataHealth {
    // Legacy semantics kept for callers without a LoadReport (TUI view,
    // explicit-path runs): parse failures and scope exclusions cannot be
    // separated, so they fold into `skipped` and out_of_scope stays 0.
    let parsed = sessions.len();
    let skipped = discovered.saturating_sub(parsed);
    data_health_from_parts(sessions, discovered, skipped, 0, cache_hits, Vec::new())
}

/// Truthful coverage accounting from the discovery loader (pass-8
/// F8-2): `discovered` is the loader's range-independent file-scan
/// count, `parse_failures` is the loader's `skipped`, and everything
/// else the loader discovered but the report excludes lands in
/// `out_of_scope` instead of shrinking the denominator.
pub fn data_health_scoped(
    sessions: &[Session],
    discovered: usize,
    parse_failures: usize,
    cache_hits: usize,
    sqlite_store_failures: Vec<String>,
) -> DataHealth {
    let parsed = sessions.len();
    let out_of_scope = discovered.saturating_sub(parsed + parse_failures);
    data_health_from_parts(
        sessions,
        discovered,
        parse_failures,
        out_of_scope,
        cache_hits,
        sqlite_store_failures,
    )
}

fn data_health_from_parts(
    sessions: &[Session],
    discovered: usize,
    skipped: usize,
    out_of_scope: usize,
    cache_hits: usize,
    sqlite_store_failures: Vec<String>,
) -> DataHealth {
    let parsed = sessions.len();
    let unknown_sources = sessions
        .iter()
        .filter(|s| matches!(s.metrics.source_tool.as_str(), "" | "generic" | "unknown"))
        .count();
    let unknown_models = sessions
        .iter()
        .filter(|s| matches!(s.metrics.model_used.as_str(), "" | "default" | "unknown"))
        .count();
    let fallback_pricing = sessions
        .iter()
        .filter(|s| !pricing::has_specific_price(&s.metrics.model_used))
        .count();
    let stored_totals_sessions = sessions
        .iter()
        .filter(|s| s.metrics.provenance.tokens == "stored_session_totals")
        .count();
    let stored_totals_delta_tokens = sessions.iter().fold(0i64, |acc, s| {
        acc.saturating_add(s.metrics.stored_totals_delta.saturating_abs())
    });
    let unknown_time_sessions = sessions
        .iter()
        .filter(|s| parse_ts(&s.metrics.session_start).is_none())
        .count();
    let mut line_skips = BTreeMap::new();
    for session in sessions {
        for (reason, count) in &session.metrics.line_skips {
            *line_skips.entry(reason.clone()).or_insert(0) += count;
        }
    }
    // rm-436/rm-437: aggregate per-session disclosure counters the same
    // way line_skips aggregates, so journal-level facts surface once per
    // corpus instead of per session.
    let mut disclosures = BTreeMap::new();
    for session in sessions {
        for (key, count) in &session.metrics.disclosure_counters {
            *disclosures.entry(key.clone()).or_insert(0) += count;
        }
    }
    let non_finite_costs = sessions
        .iter()
        .filter(|s| !s.metrics.cost_estimated.is_finite())
        .count();
    DataHealth {
        discovered,
        parsed,
        skipped,
        out_of_scope,
        cache_hits,
        unknown_sources,
        unknown_models,
        fallback_pricing,
        latest_session_at: sessions
            .iter()
            .filter_map(|s| parse_ts(&s.metrics.session_start))
            .max()
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .unwrap_or_default(),
        confidence: if parsed == 0
            || skipped > 0
            || unknown_sources > 0
            || unknown_models > 0
            || non_finite_costs > 0
            || !line_skips.is_empty()
        {
            "low"
        } else if fallback_pricing > 0 {
            "medium"
        } else {
            "high"
        }
        .to_string(),
        with_tokens: sessions
            .iter()
            .filter(|s| {
                s.metrics
                    .tokens_input
                    .saturating_add(s.metrics.tokens_output)
                    > 0
            })
            .count(),
        stored_totals_sessions,
        stored_totals_delta_tokens,
        unknown_time_sessions,
        line_skips,
        disclosures,
        non_finite_costs,
        sqlite_store_failures,
        with_duration: sessions
            .iter()
            .filter(|s| s.metrics.duration_sec > 0.0)
            .count(),
        with_tools: sessions
            .iter()
            .filter(|s| s.metrics.tool_calls_total > 0)
            .count(),
        with_event_timing: sessions
            .iter()
            .filter(|s| !s.metrics.gaps_sec.is_empty())
            .count(),
        with_diagnostics: sessions
            .iter()
            .filter(|s| session_capability(s) == "detailed")
            .count(),
    }
}

fn project_matches(session: &Session, filter: &str) -> bool {
    if filter.trim().is_empty() {
        return true;
    }
    let project = resolve_project(session);
    contains(&project.id, filter)
        || contains(&project.display_name, filter)
        || contains(&project.root, filter)
}

fn contains(value: &str, filter: &str) -> bool {
    filter.trim().is_empty()
        || value
            .to_ascii_lowercase()
            .contains(&filter.trim().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn session_at(cwd: &str, path: &str) -> Session {
        Session {
            name: "s".to_string(),
            path: path.to_string(),
            cwd: cwd.to_string(),
            metrics: crate::Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        }
    }

    /// Unique fixture root for decode/project tests (rm-382): pid + thread
    /// id + sequence make collisions impossible, and the `at-…` prefix means
    /// no plausible host ancestor or sibling directory can shadow a
    /// component of the encoded path. The old `…/agenttrace-project-<pid>`
    /// fixtures were flipped to "unknown" whenever the host TMPDIR contained
    /// a directory literally named `agenttrace` (as hermes delegate hosts
    /// do), failing the suite host-dependently.
    fn unique_decode_root(label: &str) -> std::path::PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "at-{}-{:?}-{}-{label}",
            std::process::id(),
            std::thread::current().id(),
            seq
        ))
    }

    #[test]
    fn projects_group_worktrees_and_decode_agent_dirs() {
        // rm-382: hermetic fixture — unique root, no `agenttrace` component
        // for host TMPDIR shadows to collide with.
        let root = unique_decode_root("grouping");
        let repo = root.join("src").join("my-repo");
        let worktree = root.join("worktrees").join("a1b2").join("my-repo");
        fs::create_dir_all(repo.join(".git").join("worktrees").join("wt")).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", repo.join(".git/worktrees/wt").display()),
        )
        .unwrap();

        let main = resolve_project(&session_at(&repo.to_string_lossy(), "/x.jsonl"));
        let linked = resolve_project(&session_at(&worktree.to_string_lossy(), "/x.jsonl"));
        assert_eq!(linked.id, main.id);
        assert_eq!(linked.display_name, "my-repo");

        let encoded = repo.to_string_lossy().replace('/', "-");
        let transcript = root.join("projects").join(&encoded).join("session.jsonl");
        let decoded = resolve_project(&session_at("", &transcript.to_string_lossy()));
        assert_eq!(decoded.id, main.id);

        let missing = resolve_project(&session_at("", "/nowhere/projects/-gone-dir/s.jsonl"));
        assert_eq!(missing.display_name, "unknown");
        assert_eq!(
            project_decode_status(&session_at("", "/nowhere/projects/-gone-dir/s.jsonl")),
            ProjectDecodeStatus::Unresolved {
                encoded: "gone-dir".to_string()
            }
        );
        // A cwd-bearing session never consults the decoder.
        assert_eq!(
            project_decode_status(&session_at("/somewhere", "/any/projects/-x/s.jsonl")),
            ProjectDecodeStatus::NotConsulted
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn decode_ignores_shadow_directories_at_plausible_ancestors() {
        // rm-382 regression (assess N2): with TMPDIR=/home/agent/.hermes/
        // tmp/delegate and a leftover sibling directory literally named
        // `agenttrace`, the old greedy decoder consumed it as the first
        // component of the fixture path `…/<tmp>/agenttrace-project-<pid>/
        // src/my-repo` and the walk never recovered — the suite failed only
        // on hosts with that shadow (release-local gate rc101 cascade). The
        // shadow is planted INSIDE the unique root, so the fixture never
        // mutates shared host state while reproducing the exact trap.
        let root = unique_decode_root("shadow");
        let fixture = root.join(format!("agenttrace-project-{}", std::process::id()));
        let repo = fixture.join("src").join("my-repo");
        fs::create_dir_all(&repo).expect("create repo");
        fs::create_dir_all(root.join("agenttrace")).expect("plant shadow ancestor");
        let encoded = repo.to_string_lossy().replace('/', "-");
        let transcript = root.join("projects").join(&encoded).join("session.jsonl");
        fs::create_dir_all(&transcript).unwrap();
        let decoded = resolve_project(&session_at(
            "",
            &transcript.join("s.jsonl").to_string_lossy(),
        ));
        assert_eq!(decoded.display_name, "my-repo");
        assert_eq!(
            decoded.root,
            lexical_normalize(&repo).to_string_lossy().to_string()
        );
        // The shadow never even becomes a disclosed candidate.
        assert!(matches!(
            project_decode_status(&session_at(
                "",
                &transcript.join("s.jsonl").to_string_lossy()
            )),
            ProjectDecodeStatus::Resolved { .. }
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn decode_prefers_the_longest_verified_path_over_a_decoy_split() {
        // rm-381 regression, assess PoC triple (delegate/172562a8…-scratch/
        // poc-misattr-{decoy,true,nodecoy}.json): with the true project
        // `<root>/misattr/my-repo` and an unrelated decoy
        // `<root>/misattr/my/repo`, the old greedy first-match decoder
        // attributed the session to the decoy while it existed (`--project
        // my-repo` rc1) and flipped back after it was removed. Attribution
        // must be byte-identical in both host states, and the ambiguity must
        // be disclosed, not decided silently.
        let root = unique_decode_root("misattr");
        let base = root.join("misattr");
        let true_repo = base.join("my-repo");
        fs::create_dir_all(&true_repo).expect("create true repo");
        let decoy = base.join("my").join("repo");
        fs::create_dir_all(&decoy).expect("plant decoy");
        let encoded = true_repo.to_string_lossy().replace('/', "-");
        let transcript = root
            .join("projects")
            .join(&encoded)
            .join("session.jsonl")
            .to_string_lossy()
            .to_string();
        let session = session_at("", &transcript);

        // Decoy present: attributed to the true project, not the decoy.
        let with_decoy = resolve_project(&session);
        assert_eq!(with_decoy.display_name, "my-repo");
        assert_eq!(
            with_decoy.root,
            lexical_normalize(&true_repo).to_string_lossy().to_string()
        );
        // The alternative the old decoder would have descended into is
        // disclosed as shadowed.
        match project_decode_status(&session) {
            ProjectDecodeStatus::Ambiguous { path, shadowed } => {
                assert!(path.ends_with("misattr/my-repo"), "true path: {path}");
                assert!(
                    shadowed.iter().any(|alt| alt.ends_with("misattr/my/repo")),
                    "decoy decode disclosed: {shadowed:?}"
                );
            }
            other => panic!("expected ambiguity disclosure, got {other:?}"),
        }

        // Decoy removed: a fresh walk (bypassing the resolve memo) yields the
        // byte-identical attribution — host state cannot flip it.
        fs::remove_dir_all(&decoy).unwrap();
        let parts = encoded_project_components(&transcript).expect("encoded components");
        let walk = decode_encoded_components(&parts, false).expect("decode without decoy");
        assert_eq!(walk.path, true_repo.to_string_lossy().to_string());
        assert!(walk.shadowed.is_empty());
        assert!(matches!(
            project_decode_status(&session),
            ProjectDecodeStatus::Resolved { .. }
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn decode_tolerates_literal_dashes_and_empty_components() {
        // A literal '-' inside a component name encodes exactly like a path
        // separator, so the longest-run rule prefers the joined spelling;
        // consecutive dashes (empty components) consume no movement.
        let root = unique_decode_root("dashes");
        let dashed = root.join("a--b");
        fs::create_dir_all(&dashed).unwrap();
        // `…-e-` is the encoding of both `/…/e-` and `/…/e/`: the dashed
        // directory wins as the longer run when it exists, and the trailing
        // empty component is tolerated when it does not.
        let trailing_dash = root.join("e-");
        fs::create_dir_all(&trailing_dash).unwrap();
        for repo in [&dashed, &trailing_dash] {
            let encoded = repo.to_string_lossy().replace('/', "-");
            let transcript = root
                .join("projects")
                .join(&encoded)
                .join("s.jsonl")
                .to_string_lossy()
                .to_string();
            let parts = encoded_project_components(&transcript).expect("encoded components");
            let walk = decode_encoded_components(&parts, false).expect("decode dashed name");
            assert_eq!(walk.path, repo.to_string_lossy().to_string());
        }
        // Without a dashed sibling on disk, the same trailing-dash encoding
        // decodes to `/…/f` and the trailing empty component is consumed
        // harmlessly (the tolerance the old decoder provided by accident).
        fs::create_dir_all(root.join("f")).unwrap();
        let mut encoded = root.join("f").to_string_lossy().replace('/', "-");
        encoded.push('-');
        let parts: Vec<&str> = encoded.split('-').collect();
        let walk = decode_encoded_components(&parts, false).expect("decode trailing empty");
        assert_eq!(walk.path, root.join("f").to_string_lossy().to_string());
        let _ = fs::remove_dir_all(root);
    }

    fn at(instant: &str) -> DateTime<Utc> {
        instant.parse().expect("rfc3339 fixture instant")
    }

    #[test]
    fn range_today_anchors_to_the_users_local_midnight() {
        // rm-040, east-of-UTC probe (the assess PoC corpus): at 05:00Z a
        // UTC+9 user is at 14:00 on their Oct 2, so "today" starts at
        // Oct 2 00:00 JST = Oct 1 15:00Z — and the 23:00Z session (08:00
        // JST that morning) is INSIDE today. UTC-midnight anchoring
        // excluded it.
        let now = at("2026-10-02T05:00:00Z");
        let east = day_start_for_offset(now, FixedOffset::east_opt(9 * 3600).unwrap());
        assert_eq!(east, at("2026-10-01T15:00:00Z"));
        assert!(at("2026-10-01T23:00:00Z") >= east);

        // West-of-UTC probe: the same instant is 00:00 Oct 2 for a UTC-5
        // user, so "today" starts at 05:00Z and yesterday 23:30 local
        // (04:30Z) is OUTSIDE today. UTC-midnight anchoring counted it in.
        let west = day_start_for_offset(now, FixedOffset::west_opt(5 * 3600).unwrap());
        assert_eq!(west, at("2026-10-02T05:00:00Z"));
        assert!(at("2026-10-02T04:30:00Z") < west);

        // UTC users keep the previous boundary exactly.
        assert_eq!(
            day_start_for_offset(now, FixedOffset::east_opt(0).unwrap()),
            at("2026-10-02T00:00:00Z")
        );
    }

    #[test]
    fn range_today_uses_local_midnight_whatever_the_host_zone() {
        // since() must agree with the helper under the live host offset,
        // in any deployment timezone, and never start after `now`.
        let now = Utc::now();
        let offset = now.with_timezone(&Local).offset().fix();
        assert_eq!(
            TimeRange::Today.since(now),
            Some(day_start_for_offset(now, offset))
        );
        assert!(TimeRange::Today
            .since(now)
            .is_some_and(|start| start <= now));
    }

    #[test]
    fn scope_cost_totals_saturate_instead_of_overflowing_to_infinity() {
        // rm-723 red-first: the token arm of the per-source accumulator
        // saturates (rm-529), but the cost arm used plain `+=` — two
        // near-f64::MAX `cost_estimated` values summed to `inf`, which
        // renderers pass through instead of the disclosed clamp. The cost
        // arm must saturate at f64::MAX too; a NaN cost must stay NaN so
        // `non_finite_costs` keeps flagging it rather than the saturating
        // clamp silently "fixing" poisoned pricing inputs.
        let mut a = session_at("", "/a.jsonl");
        a.metrics.source_tool = "claude_code".to_string();
        a.metrics.cost_estimated = f64::MAX;
        let mut b = session_at("", "/b.jsonl");
        b.metrics.source_tool = "claude_code".to_string();
        b.metrics.cost_estimated = f64::MAX;
        let scope = report_scope(&[a, b], TimeRange::All, false);
        let source = scope
            .sources
            .iter()
            .find(|entry| entry.source == "claude_code")
            .expect("per-source scope entry");
        assert_eq!(source.sessions, 2);
        assert_eq!(source.estimated_cost, f64::MAX);

        let mut nan = session_at("", "/c.jsonl");
        nan.metrics.source_tool = "claude_code".to_string();
        nan.metrics.cost_estimated = f64::NAN;
        let scope = report_scope(&[nan], TimeRange::All, false);
        let source = scope
            .sources
            .iter()
            .find(|entry| entry.source == "claude_code")
            .expect("per-source scope entry");
        assert!(source.estimated_cost.is_nan(), "NaN passes through");
    }

    #[test]
    fn opaque_project_dir_name_yields_a_stable_identity() {
        // rm-240 residual red-first: CLAUDE_CODE_PROJECT_DIR_NAME (v2.1.234+)
        // files transcripts under `projects/<custom-name>/` where the name
        // is an opaque [a-zA-Z0-9_-]{1,64} label — no leading '-', nothing
        // path-shaped to decode. The old decoder dash-split it, failed the
        // filesystem walk, and dumped every such session into `unknown`.
        // The name must attribute stably instead.
        let session = session_at("", "/cfg/projects/my-custom-app/s.jsonl");
        let identity = resolve_project(&session);
        assert_eq!(identity.display_name, "my-custom-app");
        assert_eq!(identity.id, "my-custom-app");
        assert_eq!(identity.resolution, "opaque_project_dir_name");
        // Two sessions share the identity (stable grouping, no `unknown`
        // merger), and a cwd-bearing session is never consulted.
        let other = resolve_project(&session_at("", "/cfg/projects/my-custom-app/t.jsonl"));
        assert_eq!(other.id, identity.id);
        assert!(
            resolve_project(&session_at(
                "/somewhere",
                "/cfg/projects/my-custom-app/s.jsonl"
            ))
            .resolution
                != "opaque_project_dir_name"
        );
    }

    #[test]
    fn truncated_encoded_project_name_decodes_the_verified_prefix() {
        // rm-240 residual red-first: the official encoder cuts names over
        // 200 chars to 200 and appends a hash of the full path, so the tail
        // past 200 is opaque-but-stable. The old decoder ran the walk over
        // truncation+hash, dead-ended on the hash fragment, and answered
        // `unknown`. The decode must instead reconstruct the truncation:
        // attribute by the raw encoded name (unique per source path thanks
        // to the hash) and expose the deepest VERIFIED prefix as the root.
        let root = unique_decode_root("truncated");
        let mut repo = root.clone();
        for i in 0..18 {
            repo = repo.join(format!("lvl{i:02}a1b2c3d4"));
        }
        repo = repo.join("final-project");
        fs::create_dir_all(&repo).expect("create deep repo");
        let encoded = repo.to_string_lossy().replace('/', "-");
        assert!(
            encoded.len() > 200,
            "fixture must exercise the >200-char form: {}",
            encoded.len()
        );
        let hashed = format!("{}9f8e7d6c", &encoded[..200]);
        let transcript = root
            .join("projects")
            .join(&hashed)
            .join("s.jsonl")
            .to_string_lossy()
            .to_string();
        let identity = resolve_project(&session_at("", &transcript));
        assert_eq!(identity.id, hashed, "identity keys on the raw encoded name");
        assert_eq!(identity.resolution, "encoded_truncated");
        assert_ne!(identity.display_name, "unknown");
        // Deepest verified prefix of the truncation: the walk follows the
        // real fixture directories and stops where the cut makes the tail
        // fragment unverifiable — so the root is a strict prefix of the
        // true repo path, never a misattribution to some other project.
        let repo_normalized = lexical_normalize(&repo).to_string_lossy().to_string();
        assert!(
            repo_normalized.starts_with(&identity.root) && !identity.root.is_empty(),
            "root {root:?} must be a verified prefix of {repo_normalized:?}"
        );
        let _ = fs::remove_dir_all(root);
    }
}

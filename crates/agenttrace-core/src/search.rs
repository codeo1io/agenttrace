use crate::reports::{json_float, json_string};
use crate::{
    canonical_sessions, format_cost, format_tokens, highest_authority_for_metrics, round4,
    total_tokens, SearchResult, Session, VERSION,
};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;

/// rm-1415: per-file byte cap for the bounded transcript re-read arm of
/// `--search`. Files larger than this are skipped and disclosed (counted in
/// the scan summary) rather than read — a hostile corpus of giant journals
/// cannot make one search unbounded.
pub const TRANSCRIPT_SCAN_FILE_BYTE_CAP: u64 = 8 * 1024 * 1024;
/// rm-1415: total read budget across a whole `--search` invocation. The
/// canonical (newest-first) session order is scanned until the budget is
/// exhausted; every file past the line is skipped and disclosed.
pub const TRANSCRIPT_SCAN_TOTAL_BYTE_BUDGET: u64 = 64 * 1024 * 1024;
/// rm-1415: transcript evidence items a single session may contribute; the
/// row-level `matches` cap (8) is applied after metadata + transcript merge.
const TRANSCRIPT_MATCHES_PER_SESSION: usize = 4;

/// Corpus-level disclosure for the rm-1415 transcript re-read lane: every
/// candidate session file is accounted for exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptScanSummary {
    pub files_considered: usize,
    pub scanned_files: usize,
    pub scanned_bytes: u64,
    pub skipped_missing: usize,
    pub skipped_unreadable: usize,
    pub skipped_oversized: usize,
    pub skipped_non_file: usize,
    pub skipped_budget: usize,
    /// rm-1872 residual-c (run 7eb6138c cycle 3): zstd-framed files
    /// whose bounded decode refused (corrupt frame, over-cap output) —
    /// compressed content that could not be honestly searched, so it
    /// was disclosed instead of charged to the budget.
    pub skipped_zstd_decode: usize,
    pub file_byte_cap: u64,
    pub total_byte_budget: u64,
}

impl TranscriptScanSummary {
    fn new(file_byte_cap: u64, total_byte_budget: u64) -> Self {
        Self {
            files_considered: 0,
            scanned_files: 0,
            scanned_bytes: 0,
            skipped_missing: 0,
            skipped_unreadable: 0,
            skipped_oversized: 0,
            skipped_non_file: 0,
            skipped_budget: 0,
            skipped_zstd_decode: 0,
            file_byte_cap,
            total_byte_budget,
        }
    }

    /// Human sentence for the text report tail; "none considered" renders
    /// nothing (the blank-query guard returns before any file is touched).
    fn describe(&self) -> Option<String> {
        if self.files_considered == 0 {
            return None;
        }
        let mut parts = vec![format!(
            "{} file(s) scanned ({} bytes)",
            self.scanned_files, self.scanned_bytes
        )];
        for (label, count) in [
            ("missing", self.skipped_missing),
            ("unreadable", self.skipped_unreadable),
            ("over byte cap", self.skipped_oversized),
            ("non-file", self.skipped_non_file),
            ("read budget exhausted", self.skipped_budget),
            ("zstd decode failed", self.skipped_zstd_decode),
        ] {
            if count > 0 {
                parts.push(format!("{count} skipped: {label}"));
            }
        }
        Some(format!(
            "Transcript scan: {}; caps: {} bytes/file, {} bytes total.",
            parts.join(", "),
            self.file_byte_cap,
            self.total_byte_budget
        ))
    }
}

/// rm-1415: `--search` outcome with the transcript-scan disclosure attached.
#[derive(Debug, Clone)]
pub struct SearchReport {
    pub results: Vec<SearchResult>,
    pub scan: TranscriptScanSummary,
}

pub fn search_sessions(sessions: &[Session], query: &str, limit: usize) -> Vec<SearchResult> {
    search_sessions_detailed(
        sessions,
        query,
        limit,
        TRANSCRIPT_SCAN_FILE_BYTE_CAP,
        TRANSCRIPT_SCAN_TOTAL_BYTE_BUDGET,
    )
    .results
}

/// rm-1415: full-text search across discovered session transcripts —
/// metadata evidence first (unchanged field list and ordering), then a
/// bounded re-read of each session file under explicit byte caps. Every
/// candidate file is accounted for in the returned scan summary; a body-only
/// query hit is the contract this restores (README.md / `--help` wording
/// predates the metadata-only implementation gap).
pub fn search_sessions_detailed(
    sessions: &[Session],
    query: &str,
    limit: usize,
    file_byte_cap: u64,
    total_byte_budget: u64,
) -> SearchReport {
    let query = query.trim().to_ascii_lowercase();
    let mut scan = TranscriptScanSummary::new(file_byte_cap, total_byte_budget);
    if query.is_empty() {
        // A blank query matches nothing, so the CLI rejects it loudly
        // before ever calling this (agenttrace-cli main.rs, the landed
        // rm-988 guard in validate_gate_thresholds — run b5d391ec's
        // parallel in-run() duplicate of that check was superseded at
        // integration, conflict case bd70117e). This defensive empty
        // return keeps the library function total for any other
        // caller; the CLI guard is the rejection surface, never this
        // arm.
        // rm-988: a blank query matches NOTHING (never everything) — and
        // never touches the filesystem.
        return SearchReport {
            results: Vec::new(),
            scan,
        };
    }
    let mut results = Vec::new();
    let mut budget_left = total_byte_budget;

    for session in canonical_sessions(sessions) {
        let mut matches = search_session_evidence(&session, &query);
        let (transcript_matches, transcript_scan) = transcript_evidence(
            &session.path,
            &query,
            file_byte_cap,
            &mut budget_left,
            &mut scan,
        );
        let mut seen: BTreeSet<String> = matches.iter().cloned().collect();
        for item in transcript_matches {
            if seen.insert(item.clone()) {
                matches.push(item);
            }
        }
        if matches.is_empty() {
            continue;
        }
        matches.truncate(8);
        results.push(SearchResult {
            name: session.name.clone(),
            path: session.path.clone(),
            cwd: session.cwd.clone(),
            source_tool: session.metrics.source_tool.clone(),
            model: session.metrics.model_used.clone(),
            health: session.health,
            cost: round4(session.metrics.cost_estimated),
            tokens: total_tokens(&session),
            matches,
            transcript_scan,
        });
        if results.len() >= limit {
            break;
        }
    }

    SearchReport { results, scan }
}

pub fn report_search_json(results: &[SearchResult]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"count\": {},\n", results.len()));
    out.push_str("  \"results\": ");
    if results.is_empty() {
        out.push_str("[],\n");
    } else {
        write_search_results_json(&mut out, results);
        out.push_str(",\n");
    }
    out.push_str(&format!(
        "  \"version\": {}\n",
        serde_json::to_string(VERSION).expect("version serializes")
    ));
    out.push('}');
    out
}

fn write_search_results_json(out: &mut String, results: &[SearchResult]) {
    write_search_results_json_inner(out, results, false)
}

/// `include_transcript_scan` gates the rm-1415 per-row disclosure so the
/// stable [`report_search_json`] lane stays byte-identical to the pre-1415
/// schema while the detailed lane carries the new key.
fn write_search_results_json_inner(
    out: &mut String,
    results: &[SearchResult],
    include_transcript_scan: bool,
) {
    out.push_str("[\n");
    for (index, result) in results.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!("      \"name\": {},\n", json_string(&result.name)));
        out.push_str(&format!("      \"path\": {},\n", json_string(&result.path)));
        if !result.cwd.is_empty() {
            out.push_str(&format!("      \"cwd\": {},\n", json_string(&result.cwd)));
        }
        out.push_str(&format!(
            "      \"source_tool\": {},\n",
            json_string(&result.source_tool)
        ));
        out.push_str(&format!(
            "      \"model\": {},\n",
            json_string(&result.model)
        ));
        out.push_str(&format!("      \"health\": {},\n", result.health));
        out.push_str(&format!("      \"cost\": {},\n", json_float(result.cost)));
        out.push_str(&format!("      \"tokens\": {},\n", result.tokens));
        out.push_str("      \"matches\": ");
        write_string_array_json(out, &result.matches, 3);
        out.push('\n');
        if include_transcript_scan {
            // `matches` was the final key pre-1415; the detailed lane appends
            // one key after it, so the array needs its comma back.
            out.pop();
            out.push_str(",\n");
            out.push_str(&format!(
                "      \"transcript_scan\": {}\n",
                json_string(&result.transcript_scan)
            ));
        }
        out.push_str("    }");
        if index + 1 < results.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]");
}

fn write_string_array_json(out: &mut String, values: &[String], base_indent: usize) {
    if values.is_empty() {
        out.push_str("[]");
        return;
    }
    let item_indent = "  ".repeat(base_indent + 1);
    let close_indent = "  ".repeat(base_indent);
    out.push_str("[\n");
    for (index, value) in values.iter().enumerate() {
        out.push_str(&item_indent);
        out.push_str(&json_string(value));
        if index + 1 < values.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&close_indent);
    out.push(']');
}

/// rm-985: the transcript-derived fields of the search text renderer are
/// guarded at the render boundary — control bytes first (rm-383's
/// sanitizer), then the spreadsheet-formula introducer neutralizer
/// (`reports::guard_formula_cell`, the same family the CSV/markdown/
/// sessions-TSV lanes use) — so a session named `=HYPERLINK(…)` or a
/// match-evidence line beginning with an introducer carries the leading
/// `'` paste-marker instead of re-arming when the pasted output lands in
/// a spreadsheet. JSON output stays lossless by design.
fn guarded_segment(value: &str) -> String {
    crate::reports::guard_formula_cell(&crate::statusline::sanitize_line_segment(value))
}

/// rm-540 residual (run 7fd83ab2 cycle 2, assess cbf457f4 F1): evidence
/// items are `label: value` pairs joined in `add_match`, and
/// `guard_formula_cell` only neutralizes an introducer at the FIRST
/// non-whitespace character — so guarding the JOINED item at the render
/// loop could never fire behind the label, and every evidence row
/// (`name:`, `tool:`, `tool argument:`, …) rendered its payload raw.
/// Split the fixed label off and guard the VALUE: text-to-columns and
/// `": "`-aware paste targets make the value a cell of its own. The
/// value passes through the same rm-985 stack, and JSON output remains
/// lossless (the guard is a text-render concern; the stored match item
/// keeps its raw value).
fn guarded_evidence_item(item: &str) -> String {
    match item.split_once(": ") {
        Some((label, value)) => format!("{}: {}", label, guarded_segment(value)),
        None => guarded_segment(item),
    }
}

/// rm-1415: detailed JSON lane — per-row `transcript_scan` disclosure plus
/// the corpus-level scan summary object (additive to the stable schema of
/// [`report_search_json`], which is kept byte-stable for library consumers).
pub fn report_search_json_detailed(report: &SearchReport) -> String {
    let results = &report.results;
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"count\": {},\n", results.len()));
    out.push_str("  \"results\": ");
    if results.is_empty() {
        out.push_str("[],\n");
    } else {
        write_search_results_json_inner(&mut out, results, true);
        out.push_str(",\n");
    }
    out.push_str("  \"transcript_scan\": ");
    write_transcript_summary_json(&mut out, &report.scan);
    out.push_str(",\n");
    out.push_str(&format!(
        "  \"version\": {}\n",
        serde_json::to_string(VERSION).expect("version serializes")
    ));
    out.push('}');
    out
}

fn write_transcript_summary_json(out: &mut String, scan: &TranscriptScanSummary) {
    out.push_str("{\n");
    let fields = [
        ("files_considered", scan.files_considered.to_string()),
        ("scanned_files", scan.scanned_files.to_string()),
        ("scanned_bytes", scan.scanned_bytes.to_string()),
        ("skipped_missing", scan.skipped_missing.to_string()),
        ("skipped_unreadable", scan.skipped_unreadable.to_string()),
        ("skipped_oversized", scan.skipped_oversized.to_string()),
        ("skipped_non_file", scan.skipped_non_file.to_string()),
        ("skipped_budget", scan.skipped_budget.to_string()),
        ("skipped_zstd_decode", scan.skipped_zstd_decode.to_string()),
        ("file_byte_cap", scan.file_byte_cap.to_string()),
        ("total_byte_budget", scan.total_byte_budget.to_string()),
    ];
    for (index, (key, value)) in fields.iter().enumerate() {
        out.push_str(&format!("    \"{key}\": {value}"));
        if index + 1 < fields.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  }");
}

/// rm-1415: detailed text lane — the stable text report plus a per-row
/// `transcript:` disclosure line and the scan summary tail.
pub fn report_search_text_detailed(report: &SearchReport, query: &str) -> String {
    let mut out = report_search_text(&report.results, query);
    if let Some(summary) = report.scan.describe() {
        out.push('\n');
        out.push_str(&summary);
        out.push('\n');
    }
    out
}

pub fn report_search_text(results: &[SearchResult], query: &str) -> String {
    let mut out = format!("Search results: {:?} ({})\n", query, results.len());
    if results.is_empty() {
        out.push_str("No matching session metadata found.\n");
        return out;
    }
    for result in results {
        // rm-383: names, tool/model strings, cwd/path and match evidence are
        // transcript-derived; a raw ESC/OSC-52 in any of them must not reach
        // the terminal (assess PoC: --search -f text emitted a verbatim
        // clipboard-write sequence from a crafted tool name). JSON output
        // escapes control bytes losslessly and is intentionally untouched.
        // rm-985 rider: the same fields then take the formula-introducer
        // guard (`guarded_segment`) so the text lane is paste-safe too.
        out.push_str(&format!(
            "\n{}  {}  {}  health={}  {}  {} TOKENS\n",
            guarded_segment(&result.name),
            guarded_segment(&result.source_tool),
            guarded_segment(&result.model),
            result.health,
            format_cost(result.cost),
            format_tokens(result.tokens)
        ));
        if !result.cwd.is_empty() {
            out.push_str(&format!("  cwd: {}\n", guarded_segment(&result.cwd)));
        }
        if !result.path.is_empty() {
            out.push_str(&format!("  path: {}\n", guarded_segment(&result.path)));
        }
        for item in &result.matches {
            out.push_str(&format!("  - {}\n", guarded_evidence_item(item)));
        }
    }
    out
}

fn search_session_evidence(session: &Session, query: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut matches = Vec::new();
    add_match(&mut matches, &mut seen, "name", &session.name, query);
    add_match(&mut matches, &mut seen, "path", &session.path, query);
    add_match(&mut matches, &mut seen, "cwd", &session.cwd, query);
    add_match(
        &mut matches,
        &mut seen,
        "source",
        &session.metrics.source_tool,
        query,
    );
    add_match(
        &mut matches,
        &mut seen,
        "model",
        &session.metrics.model_used,
        query,
    );
    add_match(
        &mut matches,
        &mut seen,
        "authority",
        &highest_authority_for_metrics(&session.metrics),
        query,
    );

    for tool in session.metrics.tool_usage.keys() {
        add_match(&mut matches, &mut seen, "tool", tool, query);
    }
    for arg in session.metrics.tool_arg_usage.keys() {
        add_match(&mut matches, &mut seen, "tool argument", arg, query);
    }
    for file in session.metrics.file_usage.keys() {
        add_match(&mut matches, &mut seen, "file", file, query);
    }
    for anomaly in &session.anomalies {
        add_match(&mut matches, &mut seen, "anomaly", &anomaly.kind, query);
        add_match(&mut matches, &mut seen, "anomaly", &anomaly.detail, query);
    }
    for warning in &session.tool_warnings {
        add_match(
            &mut matches,
            &mut seen,
            "tool warning",
            &warning.tool_name,
            query,
        );
        add_match(
            &mut matches,
            &mut seen,
            "tool warning",
            &warning.pattern,
            query,
        );
        add_match(
            &mut matches,
            &mut seen,
            "tool warning",
            &warning.detail,
            query,
        );
    }
    matches.truncate(8);
    matches
}

fn add_match(
    matches: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    label: &str,
    value: &str,
    query: &str,
) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    if value.to_ascii_lowercase().contains(query) {
        let item = format!("{}: {}", label, value);
        if seen.insert(item.clone()) {
            matches.push(item);
        }
    }
}

/// rm-1415: bounded transcript re-read for one session file. Returns the
/// evidence snippets ("transcript: …" items, at most
/// [`TRANSCRIPT_MATCHES_PER_SESSION`]) plus the per-row disclosure string;
/// every outcome — including skips — is counted in `scan` so the corpus-level
/// summary can disclose what the lane did NOT read. Unreadable, missing,
/// oversized, non-file (`demo://`) and budget-exhausted files are disclosed,
/// never fatal: the metadata lanes above keep working verbatim. zstd-framed
/// rollouts (rm-1872) take the decompressed arm in
/// [`zstd_transcript_evidence`]: decoded through the parser's bounded lane,
/// searched as text, budgeted on DECOMPRESSED bytes, with decode refusals
/// disclosed instead of charged.
#[allow(clippy::too_many_arguments)]
fn transcript_evidence(
    path: &str,
    query: &str,
    file_byte_cap: u64,
    budget_left: &mut u64,
    scan: &mut TranscriptScanSummary,
) -> (Vec<String>, String) {
    scan.files_considered += 1;
    if path.is_empty() || path.contains("://") {
        scan.skipped_non_file += 1;
        return (Vec::new(), "skipped: non-file path".to_string());
    }
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            scan.skipped_missing += 1;
            return (Vec::new(), "skipped: file missing".to_string());
        }
    };
    if !metadata.is_file() {
        scan.skipped_non_file += 1;
        return (Vec::new(), "skipped: non-file path".to_string());
    }
    let len = metadata.len();
    if len > file_byte_cap {
        scan.skipped_oversized += 1;
        return (
            Vec::new(),
            format!("skipped: over byte cap ({len} > {file_byte_cap})"),
        );
    }
    if len > *budget_left {
        scan.skipped_budget += 1;
        return (Vec::new(), "skipped: read budget exhausted".to_string());
    }
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            scan.skipped_unreadable += 1;
            return (Vec::new(), "skipped: unreadable".to_string());
        }
    };
    let mut bytes = Vec::with_capacity(len as usize);
    if file.take(file_byte_cap).read_to_end(&mut bytes).is_err() {
        scan.skipped_unreadable += 1;
        return (Vec::new(), "skipped: unreadable".to_string());
    }
    let read = bytes.len() as u64;
    // rm-1872 residual-c (run 7eb6138c cycle 3): content-based zstd
    // probe, exactly like the parse lane (parser.rs sniffs the same
    // magic 28 B5 2F FD on bytes, never on the file name). A compressed
    // rollout is not searchable text: the old arm lossy-decoded the RAW
    // FRAME, charged its compressed size to the budget, and reported
    // "scanned (849 bytes)" over bytes that could never match — a
    // body-only needle inside the decompressed journal was silently
    // invisible (assess A1 live PoC: identical content under a plain
    // `.jsonl` name hit; its `.jsonl.zst` twin did not).
    if bytes.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        return zstd_transcript_evidence(bytes, read, query, budget_left, scan, path);
    }
    // rm-1872: the metadata-length gate above and this subtraction
    // straddle the actual read — a file growing between them makes
    // `read` exceed the checked `len`, and the old `-=` would wrap the
    // u64 budget upward (panicking under debug builds, silently
    // DISABLING the cap under release). Saturate: an over-read file
    // exhausts the budget honestly, never resets it.
    *budget_left = budget_left.saturating_sub(read);
    scan.scanned_files += 1;
    scan.scanned_bytes += read;

    // Lossy decode: a torn multibyte tail must not abort the scan.
    let content = String::from_utf8_lossy(&bytes);
    (
        transcript_hits(&content, query),
        format!("scanned ({read} bytes)"),
    )
}

/// rm-1415/rm-1872: the per-line body scan shared by the plain and the
/// decompressed arms — at most [`TRANSCRIPT_MATCHES_PER_SESSION`]
/// `transcript: …` snippet evidence items.
fn transcript_hits(content: &str, query: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        if out.len() >= TRANSCRIPT_MATCHES_PER_SESSION {
            break;
        }
        if let Some(snippet) = snippet_around(line, query, 96) {
            out.push(format!("transcript: {snippet}"));
        }
    }
    out
}

/// rm-1872 residual-c (run 7eb6138c cycle 3): the compressed arm of
/// [`transcript_evidence`]. The frame decodes through the parser's own
/// [`decode_zstd_rollout_bounded`](crate::parser::decode_zstd_rollout_bounded)
/// — one decoder, one 64 MiB total-output cap, one refusal wording, so
/// the search lane and the parse lane can never disagree about what a
/// rollout file contains. The budget meters DECOMPRESSED bytes: those
/// are the bytes actually searched, and the row note discloses the
/// compressed↔decompressed relationship at scan time. Every skip is
/// disclosed BEFORE any budget accounting — a compressed file that
/// cannot be honestly searched charges nothing, so a corpus of
/// undecodable frames cannot burn down the budget it refuses to
/// search.
fn zstd_transcript_evidence(
    frame: Vec<u8>,
    compressed: u64,
    query: &str,
    budget_left: &mut u64,
    scan: &mut TranscriptScanSummary,
    label: &str,
) -> (Vec<String>, String) {
    let decoded = match crate::parser::decode_zstd_rollout_bounded(frame, label) {
        Ok(decoded) => decoded,
        Err(error) => {
            scan.skipped_zstd_decode += 1;
            return (Vec::new(), format!("skipped: zstd decode failed ({error})"));
        }
    };
    let decompressed = decoded.len() as u64;
    if decompressed > *budget_left {
        scan.skipped_budget += 1;
        return (
            Vec::new(),
            format!(
                "skipped: decompressed {decompressed} bytes exceed the read budget ({compressed}-byte zstd frame)"
            ),
        );
    }
    *budget_left = budget_left.saturating_sub(decompressed);
    scan.scanned_files += 1;
    scan.scanned_bytes += decompressed;
    let content = String::from_utf8_lossy(&decoded);
    (
        transcript_hits(&content, query),
        format!("scanned ({decompressed} bytes decompressed from a {compressed}-byte zstd frame)"),
    )
}

/// A `…window…` snippet centered on the first case-insensitive hit of
/// `query` inside `line` (indices align: `to_ascii_lowercase` preserves byte
/// length). `None` when the line does not contain the query. Window edges
/// are walked to char boundaries so a multibyte hit cannot panic slicing.
fn snippet_around(line: &str, query: &str, width: usize) -> Option<String> {
    let haystack = line.to_ascii_lowercase();
    let idx = haystack.find(query)?;
    let qlen = query.len();
    let mut start = idx.saturating_sub(width / 2);
    while !line.is_char_boundary(start) {
        start += 1;
    }
    let mut end = (idx + qlen + width / 2).min(line.len());
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    let mut snippet = String::new();
    if start > 0 {
        snippet.push('…');
    }
    snippet.push_str(&line[start..end]);
    if end < line.len() {
        snippet.push('…');
    }
    Some(snippet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Metrics, Session};

    #[test]
    fn search_evidence_uses_go_labels_dedupes_and_caps_matches() {
        let mut metrics = Metrics {
            source_tool: "codex_cli".to_string(),
            model_used: "gpt-5".to_string(),
            ..Metrics::default()
        };
        metrics
            .tool_arg_usage
            .insert("go test ./internal/billing".to_string(), 2);
        metrics
            .tool_arg_usage
            .insert("go test ./internal/billing".to_string(), 1);
        for idx in 0..12 {
            metrics
                .file_usage
                .insert(format!("internal/billing/file-{idx}.go"), 1);
        }
        let session = Session {
            org: String::new(),
            org_email: String::new(),
            name: "billing".to_string(),
            path: "/tmp/billing.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics,
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        };

        let matches = search_session_evidence(&session, "billing");
        assert!(matches.contains(&"tool argument: go test ./internal/billing".to_string()));
        assert!(matches.len() <= 8);
        assert!(!matches.iter().any(|item| item.starts_with("tool_arg:")));
    }

    #[test]
    fn search_json_formats_zero_cost_like_go() {
        let result = SearchResult {
            name: "session".to_string(),
            path: "/tmp/session.jsonl".to_string(),
            cwd: String::new(),
            source_tool: "pi".to_string(),
            model: "mimo-v2.5-pro".to_string(),
            health: 88,
            cost: 0.0,
            tokens: 42,
            matches: vec!["tool argument: error".to_string()],
            transcript_scan: "scanned (0 bytes)".to_string(),
        };

        let report = report_search_json(&[result]);
        assert!(report.contains("\"cost\": 0,"));
        assert!(!report.contains("\"cost\": 0.0,"));
    }

    #[test]
    fn search_text_sanitizes_transcript_derived_control_bytes() {
        // rm-383: assess PoC (delegate/172562a8…-scratch/osc-search.txt) —
        // `--search -f text` rendered a raw OSC-52 clipboard-write sequence
        // from a crafted tool name byte-for-byte into the terminal. Every
        // transcript-derived field of the text renderer must come out free
        // of control bytes; the printable tail of the sequence survives as
        // text (only the ESC/BEL bytes are neutralized).
        let result = SearchResult {
            name: "osc\u{001b}]52;c;aGVsbG8=\u{0007}".to_string(),
            path: "/tmp/s.jsonl".to_string(),
            cwd: "/tmp\u{001b}[31mred".to_string(),
            source_tool: "pi\u{0007}".to_string(),
            model: "m\u{001b}[2J".to_string(),
            health: 88,
            cost: 0.0,
            tokens: 42,
            matches: vec!["tool argument: x\u{001b}]52;c;aGVsbG8=\u{0007}".to_string()],
            transcript_scan: "scanned (0 bytes)".to_string(),
        };
        let text = report_search_text(&[result], "probe");
        assert!(
            text.lines()
                .all(|line| line.chars().all(|c| !c.is_control())),
            "no raw control bytes may reach the terminal: {text:?}"
        );
        assert!(text.contains('\u{FFFD}'));
        assert!(text.contains("]52;c;aGVsbG8="));
    }

    /// rm-988 (cycle 2, run b5affead): the LIBRARY-side guard the CLI
    /// now front-stops. A blank or whitespace-only query must match
    /// NOTHING (never everything) — this is the semantic the CLI
    /// rejection leans on: had the guard instead returned all
    /// sessions, the loud rejection would be masking a wildcard, and a
    /// future caller that bypasses the CLI (library consumer, MCP
    /// tool) would inherit the wildcard. Pinned red-first at mint:
    /// the same session matches a real query, so the empty results
    /// below are the guard, not the fixture.
    #[test]
    fn blank_and_whitespace_queries_match_nothing() {
        let session = Session {
            org: String::new(),
            org_email: String::new(),
            name: "01-healthy-ship".to_string(),
            path: "demo://01-healthy-ship.jsonl".to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                source_tool: "pi".to_string(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: crate::Diagnostics::default(),
        };
        let sessions = [session];

        assert_eq!(
            search_sessions(&sessions, "healthy", 20).len(),
            1,
            "the fixture must match a real query, or the blank pins below are vacuous"
        );
        for blank in ["", "   ", "\t"] {
            assert!(
                search_sessions(&sessions, blank, 20).is_empty(),
                "blank query {blank:?} must match nothing (not everything)"
            );
        }
    }

    #[test]
    fn search_text_neutralizes_formula_introducers_in_every_field() {
        // rm-985: assess fdaf8b39 F1 family — the search text lane renders
        // session names, tool/model strings, cwd/path and match evidence;
        // every one of them must arrive paste-safe (leading `'` marker on
        // an introducer) while numeric-looking fields stay bare.
        let result = SearchResult {
            name: "=HYPERLINK(\"http://evil.example\",\"click-me\") please review".to_string(),
            path: "/tmp/s.jsonl".to_string(),
            cwd: "/tmp".to_string(),
            source_tool: "pi".to_string(),
            model: "@import('x')".to_string(),
            health: 88,
            cost: 0.0,
            tokens: 42,
            matches: vec!["+SUM(9)".to_string()],
            transcript_scan: "scanned (0 bytes)".to_string(),
        };
        let text = report_search_text(&[result], "probe");
        assert!(
            text.contains("\n'=HYPERLINK("),
            "session name must carry the paste-marker: {text:?}"
        );
        assert!(text.contains("'@import"));
        // The matches list is routed through the same guard: evidence
        // items are `field: snippet`-shaped today (introducer can never
        // sit at cell start), but the render boundary is pinned for any
        // future item shape that leads with the payload itself.
        assert!(text.contains("- '+SUM(9)"));
        // The JSON lane stays lossless by design.
        let json = report_search_json(&[SearchResult {
            name: "=2+5|cmd".to_string(),
            path: String::new(),
            cwd: String::new(),
            source_tool: String::new(),
            model: String::new(),
            health: 88,
            cost: 0.0,
            tokens: 42,
            matches: Vec::new(),
            transcript_scan: "scanned (0 bytes)".to_string(),
        }]);
        assert!(json.contains("\"=2+5|cmd\""));
    }
}

use crate::reports::{json_float, json_string};
use crate::{
    canonical_sessions, format_cost, format_tokens, highest_authority_for_metrics, round4,
    total_tokens, SearchResult, Session, VERSION,
};
use std::collections::BTreeSet;

pub fn search_sessions(sessions: &[Session], query: &str, limit: usize) -> Vec<SearchResult> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();

    for session in canonical_sessions(sessions) {
        let matches = search_session_evidence(&session, &query);
        if matches.is_empty() {
            continue;
        }
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
        });
        if results.len() >= limit {
            break;
        }
    }

    results
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
        // rm-992: the same fields ride `sanitize_rendered_text` (control
        // bytes + spreadsheet-formula shape) — search results get pasted
        // into spreadsheets exactly like the CSV/TSV exports, and a model
        // id `=2+5|cmd` used to render verbatim here while every export
        // lane guarded it (assess PoC 0c03d32e F3, fourth live lane).
        out.push_str(&format!(
            "\n{}  {}  {}  health={}  {}  {} TOKENS\n",
            crate::statusline::sanitize_rendered_text(&result.name),
            crate::statusline::sanitize_rendered_text(&result.source_tool),
            crate::statusline::sanitize_rendered_text(&result.model),
            result.health,
            format_cost(result.cost),
            format_tokens(result.tokens)
        ));
        if !result.cwd.is_empty() {
            out.push_str(&format!(
                "  cwd: {}\n",
                crate::statusline::sanitize_rendered_text(&result.cwd)
            ));
        }
        if !result.path.is_empty() {
            out.push_str(&format!(
                "  path: {}\n",
                crate::statusline::sanitize_rendered_text(&result.path)
            ));
        }
        for item in &result.matches {
            out.push_str(&format!(
                "  - {}\n",
                crate::statusline::sanitize_rendered_text(item)
            ));
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

    #[test]
    fn search_text_guards_spreadsheet_formula_shapes() {
        // rm-992: the text search lane is one of the four live formula
        // lanes (assess 0c03d32e F3) — name/model/tool strings rendered
        // `=HYPERLINK(…)` and `=2+5|cmd` verbatim while the CSV/TSV
        // export lanes guarded the identical strings. Every
        // transcript-derived field now routes through
        // sanitize_rendered_text, so an introducer-led value renders
        // apostrophe-prefixed (inert when pasted into a spreadsheet),
        // and the raw form never appears un-prefixed.
        let result = SearchResult {
            name: "=SUM(9+9)".to_string(),
            path: "/tmp/=IMPORT(x).jsonl".to_string(),
            cwd: "/tmp".to_string(),
            source_tool: "+cmd|'/C calc'!A0".to_string(),
            model: "=2+5|cmd".to_string(),
            health: 88,
            cost: 0.0,
            tokens: 42,
            matches: vec!["model: =2+5|cmd".to_string()],
        };
        let text = report_search_text(&[result], "probe");
        for guarded in ["'=SUM(9+9)", "'+cmd|'/C calc'!A0", "'=2+5|cmd"] {
            assert!(
                text.contains(guarded),
                "guarded form {guarded:?} missing from the search text lane"
            );
        }
        // Shape contract, pinned: the guard prefixes only CELL-LEADING
        // introducers, exactly like the CSV/TSV lanes. A path embeds
        // the same characters MID-string (the cell begins with
        // `/tmp/…`), so it stays raw — pasted whole it is not a
        // formula. Pinning this keeps the text lane's semantics
        // arm-for-arm with the exports instead of drifting stricter.
        assert!(
            text.contains("path: /tmp/=IMPORT(x).jsonl"),
            "mid-string introducers are not cell-leading and stay raw: {text:?}"
        );
        assert!(
            text.contains("  - model: =2+5|cmd"),
            "match-evidence composites embed values mid-string (not cell-leading): {text:?}"
        );
        // Every RENDERED FIELD must be guarded where it leads: the
        // result row separates fields with the renderer's double
        // space, and fields 0..=2 are name / tool / model.
        let row = text
            .lines()
            .find(|line| line.contains("TOKENS"))
            .expect("result row renders");
        let fields: Vec<&str> = row.split("  ").collect();
        for field in fields.iter().take(3) {
            assert!(
                field.starts_with('\''),
                "result-row field {field:?} must render guarded (rm-992)"
            );
        }
        // And no line may LEAD with a bare introducer after the
        // renderer's own label prefixes (cwd:/path:/- evidence).
        for line in text.lines() {
            let trimmed = line.trim_start();
            let body = trimmed
                .strip_prefix("cwd: ")
                .or_else(|| trimmed.strip_prefix("path: "))
                .or_else(|| trimmed.strip_prefix("- "))
                .map(|rest| rest.split(':').next().unwrap_or(rest))
                .unwrap_or(trimmed.split_whitespace().next().unwrap_or(trimmed));
            let Some(first) = body.chars().next() else {
                continue;
            };
            if [
                '=', '+', '@', '\u{ff1d}', '\u{ff0b}', '\u{ff20}', '\u{ff0d}',
            ]
            .contains(&first)
            {
                assert!(
                    body.starts_with('\''),
                    "un-guarded formula-shaped line rendered: {line:?}"
                );
            }
        }
    }
}

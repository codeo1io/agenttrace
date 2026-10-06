//! RFC 4180 CSV export (rm-409): a statement you can hand to accounting.
//!
//! Scope: `--sessions -f csv` (one row per session) and
//! `--overview -f csv` (blank-line-separated tables, each named by a
//! single-cell `# <name>` row). Row builders live in `main.rs` next to
//! the TSV/JSON renderers so CLI-local helpers (`session_capability`,
//! `total_tokens`) stay where they are; this module owns quoting,
//! cell hardening, and the documented column contract.
//!
//! Conventions (shared with the fork's table/TSV lanes — this module
//! adopts, it does not redefine):
//! - RFC 4180: cells containing comma/quote/newline/CR are
//!   double-quoted with inner quotes doubled; rows end with CRLF.
//! - rm-383 control-byte sanitation composes BEFORE quoting, so
//!   transcript-derived cells never carry OSC/ANSI escapes.
//! - Spreadsheet formula injection: a cell starting with `=`, `+`, `@`,
//!   or a non-numeric `-` gets a leading `'` (the `'` is preserved by
//!   Excel/LibreOffice as a text marker, not cell content). Plain
//!   numbers stay bare so cost/token columns import as values.

use agenttrace_core::Overview;

/// One `--sessions -f csv` row; mirrors the TSV columns plus the
/// rm-408 disclosure column, so the statement never reports clean
/// zeros silently.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionCsvRow {
    pub session: String,
    /// Health score 0-100 (integer, as everywhere else in the CLI).
    pub health: i32,
    pub data: String,
    pub source: String,
    pub model: String,
    pub cost: f64,
    pub tokens: i64,
    pub fail: usize,
    pub anomalies: usize,
    /// rm-408: all-zero usage blocks the transcript reported, counted
    /// as measured and disclosed.
    pub zero_usage_events: usize,
}

pub fn sessions_csv(rows: &[SessionCsvRow]) -> String {
    let mut out = table(
        "sessions",
        &[
            "session",
            "health",
            "data",
            "source",
            "model",
            "cost",
            "tokens",
            "fail",
            "anomalies",
            "zero_usage_events",
        ],
    );
    for row in rows {
        out.push_str(&csv_row(&[
            &row.session,
            &row.health.to_string(),
            &row.data,
            &row.source,
            &row.model,
            &format!("{:.4}", row.cost),
            &row.tokens.to_string(),
            &row.fail.to_string(),
            &row.anomalies.to_string(),
            &row.zero_usage_events.to_string(),
        ]));
    }
    out
}

pub fn overview_csv(overview: &Overview) -> String {
    let mut out = table(
        "summary",
        &["sessions", "healthy", "warning", "critical", "cost"],
    );
    out.push_str(&csv_row(&[
        &overview.total_sessions.to_string(),
        &overview.healthy.to_string(),
        &overview.warning.to_string(),
        &overview.critical.to_string(),
        &format!("{:.4}", overview.total_cost),
    ]));
    for (name, rows) in [
        ("by_model", &overview.by_model),
        ("by_provider", &overview.by_provider),
    ] {
        if rows.is_empty() {
            continue;
        }
        out.push_str(&table(name, &["name", "sessions", "cost"]));
        for (group, entry) in rows {
            out.push_str(&csv_row(&[
                group,
                &entry.sessions.to_string(),
                &format!("{:.4}", entry.cost),
            ]));
        }
    }
    if !overview.by_task_type.is_empty() {
        out.push_str(&table(
            "by_task_type",
            &["name", "sessions", "cost", "tokens_input", "tokens_output"],
        ));
        for (task_type, entry) in &overview.by_task_type {
            out.push_str(&csv_row(&[
                task_type,
                &entry.sessions.to_string(),
                &format!("{:.4}", entry.cost),
                &entry.tokens_input.to_string(),
                &entry.tokens_output.to_string(),
            ]));
        }
    }
    out
}

/// Opens a named table: `# <name>` marker row, header row. Callers
/// append data rows via [`csv_row`].
fn table(name: &str, header: &[&str]) -> String {
    let mut out = csv_row(&[format!("# {name}").as_str()]);
    out.push_str(&csv_row(header));
    out
}

/// One RFC 4180 data row (quoted as needed, CRLF-terminated).
fn csv_row(cells: &[&str]) -> String {
    let mut row = cells
        .iter()
        .map(|cell| csv_cell(cell))
        .collect::<Vec<_>>()
        .join(",");
    row.push_str("\r\n");
    row
}

fn csv_cell(raw: &str) -> String {
    let hardened = guard_formula(raw);
    let needs_quotes = hardened.contains(',')
        || hardened.contains('"')
        || hardened.contains('\n')
        || hardened.contains('\r');
    if !needs_quotes {
        return hardened;
    }
    let mut quoted = String::with_capacity(hardened.len() + 2);
    quoted.push('"');
    for ch in hardened.chars() {
        if ch == '"' {
            quoted.push('"');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    quoted
}

/// Prefixes spreadsheet-formula-looking cells with `'`. Bare numbers
/// (including negatives such as `-1.5`) are exempt so numeric columns
/// still import as values.
fn guard_formula(raw: &str) -> String {
    let looks_numeric = raw.parse::<f64>().is_ok();
    if looks_numeric {
        return raw.to_string();
    }
    let starts_formula = raw
        .chars()
        .next()
        .is_some_and(|first| matches!(first, '=' | '+' | '@' | '-'));
    if starts_formula {
        format!("'{raw}")
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_embedded_commas_quotes_and_newlines() {
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_cell("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(csv_cell("line1\r\nline2"), "\"line1\r\nline2\"");
    }

    #[test]
    fn cjk_and_unicode_cells_pass_through() {
        assert_eq!(csv_cell("会話セッション"), "会話セッション");
        assert_eq!(csv_cell("emoji 🤖 session"), "emoji 🤖 session");
    }

    #[test]
    fn formula_cells_get_a_guard_quote_but_numbers_do_not() {
        assert_eq!(csv_cell("=1+1"), "'=1+1");
        assert_eq!(csv_cell("+SUM(A1)"), "'+SUM(A1)");
        assert_eq!(csv_cell("@cmd"), "'@cmd");
        assert_eq!(csv_cell("-1.5"), "-1.5");
        assert_eq!(csv_cell("0"), "0");
        assert_eq!(csv_cell("- formula-ish"), "'- formula-ish");
    }

    #[test]
    fn sessions_csv_has_header_and_crlf_rows() {
        let out = sessions_csv(&[SessionCsvRow {
            session: "fix the \"bug\", today".to_string(),
            health: 98,
            data: "green".to_string(),
            source: "claude_code".to_string(),
            model: "claude-sonnet-4-5".to_string(),
            cost: 1.25,
            tokens: 900,
            fail: 0,
            anomalies: 0,
            zero_usage_events: 2,
        }]);
        let lines: Vec<&str> = out.split("\r\n").collect();
        assert_eq!(lines[0], "# sessions");
        assert_eq!(
            lines[1],
            "session,health,data,source,model,cost,tokens,fail,anomalies,zero_usage_events"
        );
        assert_eq!(
            lines[2],
            "\"fix the \"\"bug\"\", today\",98,green,claude_code,claude-sonnet-4-5,1.2500,900,0,0,2"
        );
        assert!(out.ends_with("\r\n"));
    }

    #[test]
    fn overview_csv_emits_named_tables_in_order() {
        let mut overview = Overview {
            total_sessions: 3,
            total_cost: 2.5,
            ..Default::default()
        };
        overview.by_model.insert(
            "claude-sonnet-4-5".to_string(),
            agenttrace_core::GroupOverview {
                sessions: 2,
                cost: 2.0,
                estimated_cost: false,
            },
        );
        let out = overview_csv(&overview);
        let lines: Vec<&str> = out.split("\r\n").collect();
        assert_eq!(lines[0], "# summary");
        assert_eq!(lines[1], "sessions,healthy,warning,critical,cost");
        assert_eq!(lines[2], "3,0,0,0,2.5000");
        assert!(out.contains("# by_model"));
        assert!(out.contains("claude-sonnet-4-5,2,2.0000"));
        assert!(!out.contains("# by_provider")); // empty sections omitted
    }
}

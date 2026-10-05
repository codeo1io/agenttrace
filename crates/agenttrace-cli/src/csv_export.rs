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
//!   transcript-derived cells never carry OSC/ANSI escapes. Session
//!   rows are sanitized by their builders in `main.rs`; the rm-506
//!   rule extends the same guarantee to the OVERVIEW GROUP KEYS this
//!   module renders itself (`by_model` / `by_provider` /
//!   `by_task_type` names are transcript-derived model ids), so every
//!   cell this module emits has passed `sanitize_line_segment`.
//! - Spreadsheet formula injection: a cell starting with `=`, `+`, `@`,
//!   a non-numeric `-`, or ANY leading whitespace (space/TAB/CR/LF,
//!   NBSP, full-width U+3000, U+FEFF — spreadsheet importers trim
//!   these before formula parsing; the OWASP CSV-injection set) gets a
//!   leading `'` (preserved by Excel/LibreOffice as a text marker, not
//!   cell content). Only FINITE bare numbers stay unguarded: Rust's
//!   f64 parser accepts `inf`/`NaN`, which are formula operands, not
//!   values (rm-506 closed that hole in the numeric exemption).
//! - rm-409 residual: csv output begins with a UTF-8 BOM (U+FEFF) so
//!   Excel detects UTF-8 and the CJK model/session names import
//!   instead of mojibaking through ANSI decode. JSON and TSV lanes are
//!   byte-identical to before — the BOM belongs to the csv statement
//!   contract only.

use agenttrace_core::{sanitize_line_segment, Overview};

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
    // rm-409 residual: BOM first so Excel decodes UTF-8 (CJK names).
    let mut out = String::from("\u{feff}");
    out.push_str(&table(
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
    ));
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
    // rm-409 residual: BOM first so Excel decodes UTF-8 (CJK model ids).
    let mut out = String::from("\u{feff}");
    out.push_str(&table(
        "summary",
        &["sessions", "healthy", "warning", "critical", "cost"],
    ));
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
                // rm-506: group keys are transcript-derived model ids —
                // sanitize at the render boundary, exactly like the
                // text/markdown/html arms (session rows are already
                // sanitized by their builders in main.rs).
                &sanitize_line_segment(group),
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
                &sanitize_line_segment(task_type),
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

/// Prefixes spreadsheet-formula-looking cells with `'`. Bare FINITE
/// numbers (including negatives such as `-1.5`) are exempt so numeric
/// columns import as values; `inf`/`NaN` parse as f64 in Rust but are
/// operands, not values, so the exemption is finite-only (rm-506).
fn guard_formula(raw: &str) -> String {
    let looks_finite_numeric = raw.parse::<f64>().is_ok_and(f64::is_finite);
    if looks_finite_numeric {
        return raw.to_string();
    }
    let starts_formula = raw.chars().next().is_some_and(|first| {
        matches!(first, '=' | '+' | '@' | '-' | '\u{feff}') || first.is_whitespace()
    });
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
    fn guard_covers_whitespace_led_and_non_finite_cells() {
        // rm-506: spreadsheet importers TRIM leading whitespace before
        // formula parsing, so a space/TAB/NBSP/full-width-space-led cell
        // is as injectable as a `=`-led one; and Rust's f64 parser
        // accepts inf/NaN, which used to slip through the numeric
        // exemption. Both now get the `'` guard.
        assert_eq!(csv_cell(" =cmd|/C calc!A0"), "' =cmd|/C calc!A0");
        assert_eq!(csv_cell("\t=cmd|/C calc!A0"), "'\t=cmd|/C calc!A0");
        assert_eq!(csv_cell("\u{00a0}=SUM(A1)"), "'\u{00a0}=SUM(A1)");
        assert_eq!(csv_cell("\u{3000}=SUM(A1)"), "'\u{3000}=SUM(A1)");
        assert_eq!(csv_cell("\u{feff}=SUM(A1)"), "'\u{feff}=SUM(A1)");
        // Non-finite f64 parses no longer take the numeric fast-path: a
        // leading operator now reaches the formula guard. Bare `inf`/`NaN`
        // carry no leading operator, so they stay inert text.
        assert_eq!(csv_cell("-inf"), "'-inf");
        assert_eq!(csv_cell("+inf"), "'+inf");
        assert_eq!(csv_cell("inf"), "inf");
        assert_eq!(csv_cell("NaN"), "NaN");
        // The finite exemption still holds for real numeric columns.
        assert_eq!(csv_cell("-1.5"), "-1.5");
        assert_eq!(csv_cell("900"), "900");
        assert_eq!(csv_cell("0.0"), "0.0");
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
        // rm-409 residual: BOM leads the csv statement so Excel detects UTF-8.
        assert!(out.starts_with('\u{feff}'));
        assert_eq!(lines[0], "\u{feff}# sessions");
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
            },
        );
        let out = overview_csv(&overview);
        let lines: Vec<&str> = out.split("\r\n").collect();
        assert!(out.starts_with('\u{feff}'));
        assert_eq!(lines[0], "\u{feff}# summary");
        assert_eq!(lines[1], "sessions,healthy,warning,critical,cost");
        assert_eq!(lines[2], "3,0,0,0,2.5000");
        assert!(out.contains("# by_model"));
        assert!(out.contains("claude-sonnet-4-5,2,2.0000"));
        assert!(!out.contains("# by_provider")); // empty sections omitted
    }

    #[test]
    fn overview_csv_sanitizes_transcript_derived_group_keys() {
        // rm-506: group keys are model ids — transcript-controlled. The
        // assess PoC (\t=cmd|/C calc!A0) must arrive as U+FFFD-led and
        // OSC-52 must never ride the csv statement into Excel.
        let mut overview = Overview::default();
        overview.by_model.insert(
            "\t=cmd|/C calc!A0".to_string(),
            agenttrace_core::GroupOverview {
                sessions: 1,
                cost: 0.0,
            },
        );
        overview.by_provider.insert(
            "\u{1b}]52;c;aGVsbG8=\u{0007}".to_string(),
            agenttrace_core::GroupOverview {
                sessions: 1,
                cost: 0.0,
            },
        );
        let out = overview_csv(&overview);
        assert!(
            out.contains("\u{fffd}=cmd|/C calc!A0,1,0.0000"),
            "tab-DDE model key sanitized then rendered: {out:?}"
        );
        assert!(!out.contains("\t="), "no raw TAB-led payload: {out:?}");
        assert!(!out.contains('\u{1b}') && !out.contains('\u{0007}'));
        assert!(
            out.contains("]52;c;aGVsbG8="),
            "payload stays visible, inert"
        );
    }
}

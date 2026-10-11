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
//! - rm-506: the overview lane's transcript-derived group keys
//!   (`by_model`/`by_provider`/`by_task_type` labels) are sanitized
//!   HERE, at this lane's render entry (`sanitize_line_segment`, the
//!   rm-383 set: control bytes → U+FFFD), BEFORE quoting — callers do
//!   not pre-sanitize this lane. (The rm-409 claim that caller-side
//!   rm-383 passes compose before quoting here was falsified by the
//!   assess `8ee739a8` PoC: `--overview -f csv` rendered a raw
//!   `\t=HYPERLINK(…)` group label on the release binary.)
//! - Spreadsheet formula injection (OWASP guard set): a cell starting
//!   with `=`, `+`, `@`, a non-numeric `-`, a cell delimiter (TAB/CR/
//!   LF), or any of their full-width double-byte variants gets a
//!   leading `'` (the `'` is preserved by Excel/LibreOffice as a text
//!   marker, not cell content). Plain numbers stay bare so
//!   cost/token columns import as values.

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
            // rm-506: sanitize the group key at this lane's render entry.
            let group = sanitize_line_segment(group);
            out.push_str(&csv_row(&[
                &group,
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
            // rm-506: sanitize the group key at this lane's render entry.
            let task_type = sanitize_line_segment(task_type);
            out.push_str(&csv_row(&[
                &task_type,
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
/// still import as values. rm-506: the guard set is the full OWASP
/// CSV-injection set — besides `= + @ -`, a cell led by a delimiter
/// Excel honors inside quotes (TAB 0x09, CR 0x0D, LF 0x0A) or by a
/// full-width double-byte variant of an operator (`＝＋－＠`) is
/// prefixed too; quoting alone does not neutralize those.
fn guard_formula(raw: &str) -> String {
    let looks_numeric = raw.parse::<f64>().is_ok();
    if looks_numeric {
        return raw.to_string();
    }
    let starts_formula = raw.chars().next().is_some_and(|first| {
        matches!(
            first,
            '=' | '+' | '@' | '-' | '\t' | '\r' | '\n' | '＝' | '＋' | '－' | '＠'
        )
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

    #[test]
    fn formula_guard_covers_the_owasp_leading_set() {
        // rm-506 (run d6432dd5, assess 8ee739a8 F1 + OWASP CSV
        // Injection): the guard used to cover only a leading `= + - @`.
        // A cell led by TAB/CR/LF — spreadsheet cell delimiters that
        // Excel honors even inside quotes — sailed through raw, and
        // so did the full-width double-byte variants. All of them get
        // the apostrophe prefix now; the numeric exemption stays.
        assert_eq!(
            csv_cell("\t=HYPERLINK(\"http://evil.example/pwn\", \"click\")"),
            // quoted because of the embedded quotes/comma; the guard
            // apostrophe rides inside the quotes
            "\"'\t=HYPERLINK(\"\"http://evil.example/pwn\"\", \"\"click\"\")\""
        );
        assert_eq!(csv_cell("\r=cmd"), "\"'\r=cmd\"");
        assert_eq!(csv_cell("\n=cmd"), "\"'\n=cmd\"");
        assert_eq!(csv_cell("＝SUM(A1)"), "'＝SUM(A1)"); // full-width =
        assert_eq!(csv_cell("＋1"), "'＋1"); // full-width +
        assert_eq!(csv_cell("－cmd"), "'－cmd"); // full-width -
        assert_eq!(csv_cell("＠evil"), "'＠evil"); // full-width @
                                                   // Interior tab in a non-formula cell stays untouched.
        assert_eq!(csv_cell("model\tx"), "model\tx");
        // The numeric exemption survives the wider guard set.
        assert_eq!(csv_cell("-1.5"), "-1.5");
        assert_eq!(csv_cell("42"), "42");
    }

    #[test]
    fn overview_csv_sanitizes_group_keys() {
        // rm-506: the overview group keys are raw `model_used` strings,
        // so the csv lane sanitizes them at render entry (rm-383
        // semantics: control bytes → U+FFFD) before quoting — the
        // falsified module claim used to be that the callers' rm-383
        // pass already composed before quoting on this lane.
        let mut overview = Overview {
            total_sessions: 1,
            ..Default::default()
        };
        overview.by_model.insert(
            "\u{1b}]0;pwned-title\u{7}vis".to_string(),
            agenttrace_core::GroupOverview {
                sessions: 1,
                cost: 0.0,
            },
        );
        overview.by_task_type.insert(
            "\t=HYPERLINK(\"http://evil.example/pwn\", \"click\")".to_string(),
            agenttrace_core::TaskTypeOverview {
                sessions: 1,
                cost: 0.0,
                tokens_input: 0,
                tokens_output: 0,
            },
        );
        let out = overview_csv(&overview);
        assert!(
            !out.contains('\u{1b}') && !out.contains('\u{7}'),
            "OSC/BEL must not reach the csv group labels"
        );
        assert!(
            out.contains('\u{FFFD}'),
            "stripped control bytes are disclosed as U+FFFD"
        );
        assert!(
            !out.contains("\t=HYPERLINK"),
            "a tab-led formula cell must not reach a csv group label raw"
        );
    }
}

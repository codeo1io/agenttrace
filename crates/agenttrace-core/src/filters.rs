//! Shared numeric-filter dialect for `--cost` / `--health` style filters.
//!
//! One dialect is shared by every numeric filter surface (CLI view filters and
//! TUI structured filters): an optional comparison operator (`>=`, `<=`, `>`,
//! `<`, `=`) followed by a finite number, where a bare number means `>=`.
//!
//! Non-finite thresholds — `NaN`, `inf`, `-inf`, and overflow literals such as
//! `1e400`, which Rust's `f64` parser maps to non-finite values — are invalid
//! filters. They used to parse successfully and then silently match nothing
//! (`>=NaN`) or everything (`<=inf`), which is indistinguishable from an
//! honest empty or full result; rejecting them at parse time keeps one class
//! of typo from producing three different failure modes.
//!
//! Integer-scored surfaces (the TUI `:health` filter) use the `i32` variant,
//! which rejects non-finite thresholds by construction while keeping the same
//! operator and bare-number rules.
//!
//! Adopted (not reinvented) from sibling run e97ae6c9's unmerged rm-364
//! design — conductor-salvage commit `a5daa6b`, verified absent from this
//! lineage at adoption time — landed here as the rm-389 remaining-scope
//! rider / rm-786 shared dialect (run 24ec00eb cycle 1, 2026-10-08).

use std::cmp::Ordering;

/// Comparison operator accepted by numeric filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericFilterOp {
    Gte,
    Lte,
    Gt,
    Lt,
    Eq,
}

impl NumericFilterOp {
    /// Split `filter` into its operator and raw value text.
    ///
    /// A bare number carries no operator and defaults to `>=` — the one
    /// bare-number rule shared by the CLI and the TUI.
    pub fn split(filter: &str) -> (Self, &str) {
        for (prefix, op) in [
            (">=", Self::Gte),
            ("<=", Self::Lte),
            (">", Self::Gt),
            ("<", Self::Lt),
            ("=", Self::Eq),
        ] {
            if let Some(value) = filter.strip_prefix(prefix) {
                return (op, value);
            }
        }
        (Self::Gte, filter)
    }
}

/// Parse a finite `f64`, rejecting `NaN`, `±inf`, and overflow literals.
///
/// Rust's `f64` parser accepts `NaN`, `inf`, `-inf`, and `infinity`
/// (case-insensitively) and maps overflow such as `1e400` to `inf`; none of
/// those are meaningful filter thresholds, so they parse to `None`.
pub fn parse_finite_f64(value: &str) -> Option<f64> {
    let parsed: f64 = value.trim().parse().ok()?;
    parsed.is_finite().then_some(parsed)
}

/// Parse a numeric filter into its operator and a finite threshold.
///
/// Returns `None` for non-numeric text and for non-finite thresholds — both
/// are invalid filters, never silent no-ops.
pub fn parse_numeric_filter(filter: &str) -> Option<(NumericFilterOp, f64)> {
    let (op, raw) = NumericFilterOp::split(filter.trim());
    let value = parse_finite_f64(raw)?;
    Some((op, value))
}

/// Parse a numeric filter whose threshold must be an `i32`.
///
/// Used for integer-scored surfaces; `i32` parsing rejects non-finite values
/// and fractional thresholds by construction.
pub fn parse_numeric_filter_i32(filter: &str) -> Option<(NumericFilterOp, i32)> {
    let (op, raw) = NumericFilterOp::split(filter.trim());
    let value: i32 = raw.trim().parse().ok()?;
    Some((op, value))
}

/// Whether `value` matches `filter`.
///
/// An empty filter matches everything and an invalid or non-finite filter
/// matches nothing (surfaces validate at parse time and reject those);
/// comparison is [`f64::total_cmp`]-based, so behavior for finite thresholds
/// is identical to the historical CLI matcher.
pub fn matches_numeric_filter(value: f64, filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }
    let Some((op, target)) = parse_numeric_filter(filter) else {
        return false;
    };
    let ordering: Ordering = value.total_cmp(&target);
    match op {
        NumericFilterOp::Gte => ordering.is_ge(),
        NumericFilterOp::Lte => ordering.is_le(),
        NumericFilterOp::Gt => ordering.is_gt(),
        NumericFilterOp::Lt => ordering.is_lt(),
        NumericFilterOp::Eq => ordering.is_eq(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonfinite_thresholds_are_invalid_filters() {
        for filter in [
            ">=NaN",
            "<=NaN",
            ">NaN",
            "<NaN",
            "=NaN",
            ">=nan",
            "=NAN",
            "<=inf",
            ">=inf",
            ">inf",
            "<inf",
            "=inf",
            ">=-inf",
            ">-inf",
            "<-inf",
            "=-inf",
            ">=1e400",
            "<=1e400",
            "=1e400",
            "1e400",
            "-1e400",
            ">=infinity",
            "<=INFINITY",
        ] {
            assert_eq!(
                parse_numeric_filter(filter),
                None,
                "filter {filter:?} must be rejected as non-finite"
            );
        }
    }

    #[test]
    fn nonfinite_filters_match_nothing() {
        // The old silent behaviors: `>=NaN` matched nothing, `<=inf` matched
        // everything. Both now match nothing at the matcher level, while
        // validation rejects them outright before any session is scanned.
        for filter in [">=NaN", "<=inf", ">=1e400"] {
            assert!(!matches_numeric_filter(0.2, filter), "{filter:?}");
        }
    }

    #[test]
    fn split_strips_operators_in_prefix_order() {
        assert_eq!(NumericFilterOp::split(">=5"), (NumericFilterOp::Gte, "5"));
        assert_eq!(NumericFilterOp::split("<=5"), (NumericFilterOp::Lte, "5"));
        assert_eq!(NumericFilterOp::split(">5"), (NumericFilterOp::Gt, "5"));
        assert_eq!(NumericFilterOp::split("<5"), (NumericFilterOp::Lt, "5"));
        assert_eq!(NumericFilterOp::split("=2.5"), (NumericFilterOp::Eq, "2.5"));
        assert_eq!(NumericFilterOp::split(">= 5"), (NumericFilterOp::Gte, " 5"));
        // Bare number defaults to `>=`.
        assert_eq!(NumericFilterOp::split("5"), (NumericFilterOp::Gte, "5"));
        // No number at all: the caller's value parse decides.
        assert_eq!(NumericFilterOp::split("<"), (NumericFilterOp::Lt, ""));
    }

    #[test]
    fn finite_controls_parse() {
        assert_eq!(
            parse_numeric_filter(">=0.01"),
            Some((NumericFilterOp::Gte, 0.01))
        );
        assert_eq!(
            parse_numeric_filter(">=79.5"),
            Some((NumericFilterOp::Gte, 79.5))
        );
        assert_eq!(
            parse_numeric_filter(">=-0.5"),
            Some((NumericFilterOp::Gte, -0.5))
        );
        assert_eq!(parse_numeric_filter("5"), Some((NumericFilterOp::Gte, 5.0)));
        assert_eq!(
            parse_numeric_filter(">= 5"),
            Some((NumericFilterOp::Gte, 5.0))
        );
        assert_eq!(
            parse_numeric_filter("  >=5  "),
            Some((NumericFilterOp::Gte, 5.0))
        );
        for bad in [">=abc", "", " ", "abc", ">=", "= "] {
            assert_eq!(parse_numeric_filter(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn matcher_keeps_historical_semantics_for_finite_filters() {
        assert!(matches_numeric_filter(0.2, ""));
        assert!(matches_numeric_filter(0.2, ">=0.1"));
        assert!(!matches_numeric_filter(0.05, ">=0.1"));
        assert!(matches_numeric_filter(0.1, "<=0.1"));
        assert!(matches_numeric_filter(0.1, "=0.1"));
        assert!(!matches_numeric_filter(0.10001, "=0.1"));
        assert!(!matches_numeric_filter(0.2, ">0.2"));
        assert!(matches_numeric_filter(0.2, ">=0.2"));
        // Bare number means `>=` in both dialects.
        assert!(matches_numeric_filter(5.0, "5"));
        assert!(!matches_numeric_filter(4.9, "5"));
        assert_eq!(
            matches_numeric_filter(4.9, "5"),
            matches_numeric_filter(4.9, ">=5")
        );
        // Whitespace-tolerant forms of the same filter agree.
        assert_eq!(
            matches_numeric_filter(0.2, ">= 0.1"),
            matches_numeric_filter(0.2, ">=0.1")
        );
        // Unparseable text matches nothing (validation rejects it first).
        assert!(!matches_numeric_filter(0.2, ">=abc"));
    }

    #[test]
    fn i32_variant_keeps_integer_semantics() {
        assert_eq!(
            parse_numeric_filter_i32("<80"),
            Some((NumericFilterOp::Lt, 80))
        );
        assert_eq!(
            parse_numeric_filter_i32("80"),
            Some((NumericFilterOp::Gte, 80))
        );
        assert_eq!(
            parse_numeric_filter_i32(" 90 "),
            Some((NumericFilterOp::Gte, 90))
        );
        for bad in [
            ">=79.5", "5.5", ">=nan", "<=inf", ">=1e400", "1e400", "abc", "",
        ] {
            assert_eq!(parse_numeric_filter_i32(bad), None, "{bad:?}");
        }
    }
}

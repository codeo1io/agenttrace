//! Render-safety boundary for session-derived strings (rm-034).
//!
//! Every string agenttrace prints that originates from a session file —
//! model names, session names, agent labels, cost-driver notes — is
//! attacker-controllable input: a poisoned session file in a shared
//! workspace or a cloned repo decides its content. The statusline path
//! learned this in cycle 7 (rm-012: OSC/CSI injection via the journal
//! payload); the text and markdown report paths carried the same
//! exposure until this module became the single choke point.
//!
//! Policy: control characters (C0, DEL, C1 — exactly `char::is_control`)
//! are replaced with U+FFFD. That single rule neutralizes every
//! terminal-control sequence — ESC, OSC-52 clipboard writes, SGR color
//! and cursor addressing, BEL — because their bytes are all control
//! characters; what survives renders as inert text. The replacement
//! keeps the string one line and preserves its visible width, so
//! column alignment in report tables survives hostile input.

/// Neutralizes terminal control sequences in one session-derived line
/// or cell value before it reaches a report, statusline, or TUI sink.
///
/// Every control character becomes U+FFFD; all other characters pass
/// through unchanged. Apply at the interpolation boundary (or inside
/// the cell helper that wraps the value), never at parse time — the
/// stored session data stays faithful, only the rendering is made safe.
pub fn sanitize_line_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|c| if c.is_control() { '\u{FFFD}' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sanitize_line_segment;

    #[test]
    fn control_characters_are_replaced_but_visible_text_survives() {
        let hostile = "EVIL\u{1b}]52;c;aGVsbG8=\u{7}\u{1b}[31mRED";
        let safe = sanitize_line_segment(hostile);
        assert!(!safe.contains('\u{1b}'));
        assert!(!safe.contains('\u{7}'));
        assert!(safe.contains("EVIL"));
        assert!(safe.contains("aGVsbG8="));
        assert!(safe.contains("RED"));
        // Both the ESC introducers and the OSC terminator are neutralized,
        // so no terminal can reassemble the sequence from the output.
        assert_eq!(safe.matches('\u{FFFD}').count(), 3);
    }

    #[test]
    fn c1_and_del_are_replaced_too() {
        assert_eq!(sanitize_line_segment("a\u{9b}b"), "a\u{FFFD}b"); // CSI as C1
        assert_eq!(sanitize_line_segment("a\u{7f}b"), "a\u{FFFD}b"); // DEL
    }

    #[test]
    fn ordinary_text_is_untouched() {
        assert_eq!(
            sanitize_line_segment("claude-sonnet-4-5"),
            "claude-sonnet-4-5"
        );
        // CJK and box-drawing characters are not control characters.
        assert_eq!(sanitize_line_segment("成本─"), "成本─");
    }
}

//! rm-933 slice-3 cross-format invariant (run ff7615be,
//! repository-maintenance 7f0a7958 cycle 3, implement attempt 43e5c129).
//!
//! The kitchen-sink corpus carries every spoofing class we defend
//! against — terminal-control sequences (OSC-52 clipboard-write, the
//! OSC-7501 program-status class pi 1.1.0 introduced, DCS), the
//! directional Cf set (RLO/PDF/LRI/PDI), the invisible-spoofing Cf set
//! (BOM/ZWNBSP, word joiner, Mongolian vowel separator, and — since
//! the review-6f85e2c9 F1 fix — the plane-14 tag block U+E0000..U+E007F,
//! DEFENDED, previously unlabeled), the deliberately-EXCLUDED adjacent
//! Cf (soft hyphen, ZWSP/ZWNJ/ZWJ; the plane-14 Variation Selectors
//! Supplement U+E0080+ is likewise excluded-and-bounds-pinned but not
//! carried in the corpus), and the preserved-verbatim neighbors
//! (NBSP, LS/PS). This contract pins ONE invariant across EVERY output
//! boundary composition:
//!
//! * render/document lanes substitute U+FFFD and keep layout controls;
//! * the serialized-JSON lane escapes `\uXXXX` and is lossless;
//! * no lane ever emits a raw spoofing character;
//! * every composition is idempotent (the layers compose, not corrupt).

use agenttrace_core::{
    escape_spoofing_json, is_invisible_spoofing_control, is_spoofing_format_control,
    sanitize_line_segment, sanitize_output_document,
};

const KITCHEN_SINK: &str = concat!(
    "a\u{1b}]52;c;base64\u{07}osc52\u{1b}\\", // OSC-52 clipboard-write (BEL + ST variants)
    "b\u{1b}]7501;prog-status\u{1b}\\",       // OSC-7501 program-status (pi 1.1.0 class)
    "c\u{1b}P+q\u{1b}\\",                     // DCS
    "d\u{202e}RLO\u{202c}PDF\u{2066}LRI\u{2069}PDI", // directional Cf
    "e\u{feff}zwnbsp\u{2060}wj\u{180e}mvs",   // invisible spoofing Cf
    "f\u{00ad}soft\u{200b}zwsp\u{200c}zwnj\u{200d}zwj", // excluded-but-adjacent Cf
    "g\u{e0000}tag\u{e0001}",                 // plane-14 tag characters
    "h\u{a0}nbsp i\u{2028}sep j\u{2029}par",  // preserved neighbors
);

#[test]
fn no_lane_emits_raw_spoofing_characters() {
    for (lane, out) in [
        ("render", sanitize_line_segment(KITCHEN_SINK)),
        ("document", sanitize_output_document(KITCHEN_SINK)),
    ] {
        let raw_spoofing: Vec<char> = out
            .chars()
            .filter(|c| is_spoofing_format_control(*c))
            .collect();
        assert!(
            raw_spoofing.is_empty(),
            "{lane} lane emitted raw spoofing chars: {raw_spoofing:?}"
        );
        let raw_ctl: Vec<char> = out
            .chars()
            .filter(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            .collect();
        assert!(
            raw_ctl.is_empty(),
            "{lane} lane emitted raw control bytes: {raw_ctl:?}"
        );
    }

    // Serialized-JSON lane: serde first (structural escapers pass Cf raw),
    // then the escape pass — the exact composition main.rs dispatch uses.
    let serialized = serde_json::to_string(&serde_json::Value::String(KITCHEN_SINK.to_string()))
        .expect("serializes");
    let escaped = escape_spoofing_json(&serialized);
    assert!(
        !escaped.chars().any(is_spoofing_format_control),
        "json lane emitted raw spoofing chars"
    );
    assert!(
        !escaped.contains('\u{1b}'),
        "json lane emitted a raw ESC byte"
    );
    // Plane-14 tags ride the JSON lane as a UTF-16 surrogate pair
    // (review 6f85e2c9 F1) — asserted here so the generic union walk
    // above can never silently narrow back.
    assert!(
        escaped.contains("\\uDB40\\uDC01"),
        "U+E0001 must escape as its surrogate pair, got: {escaped}"
    );
    assert!(!escaped.contains("\\uE0001"));
}

#[test]
fn excluded_cf_and_neighbors_survive_verbatim_in_data_lanes() {
    let doc = sanitize_output_document(KITCHEN_SINK);
    for keep in [
        "\u{00ad}soft",
        "\u{200b}zwsp",
        "\u{200c}zwnj",
        "\u{200d}zwj",
        "\u{a0}nbsp",
        "sep",
        "par",
    ] {
        assert!(
            doc.contains(keep),
            "document lane must keep {keep:?} verbatim"
        );
    }
    // The invisible set is disclosed, not silently dropped: substitution
    // markers sit where the payload was.
    assert!(doc.contains('\u{fffd}'));
    // Render lane keeps neither layout controls nor the spoofing set.
    let seg = sanitize_line_segment(KITCHEN_SINK);
    assert!(seg.contains("\u{00ad}soft") && seg.contains("\u{200b}zwsp"));
    assert!(!seg.contains('\n') && !seg.contains('\t') && !seg.contains('\r'));
}

#[test]
fn json_lane_is_lossless_end_to_end() {
    let serialized = serde_json::to_string(&serde_json::Value::String(KITCHEN_SINK.to_string()))
        .expect("serializes");
    let escaped = escape_spoofing_json(&serialized);
    // The escaped stream must still be valid JSON whose value round-trips
    // to the exact original corpus — no data loss in the machine lane.
    let decoded: String = serde_json::from_str(&escaped).expect("escaped stream is valid JSON");
    assert_eq!(decoded, KITCHEN_SINK, "json lane must be lossless");
}

#[test]
fn every_composition_is_idempotent() {
    let once = sanitize_line_segment(KITCHEN_SINK);
    assert_eq!(sanitize_line_segment(&once), once);
    let doc_once = sanitize_output_document(KITCHEN_SINK);
    assert_eq!(sanitize_output_document(&doc_once), doc_once);
    // The two human-readable layers compose: document-of-segment is
    // stable, and segment-of-document is stable too (rm-625 composition
    // property preserved under the widened set).
    let seg_then_doc = sanitize_output_document(&once);
    assert_eq!(sanitize_output_document(&seg_then_doc), seg_then_doc);
    let serialized = serde_json::to_string(&serde_json::Value::String(KITCHEN_SINK.to_string()))
        .expect("serializes");
    let esc_once = escape_spoofing_json(&serialized);
    assert_eq!(escape_spoofing_json(&esc_once), esc_once);
}

#[test]
fn invisible_set_bounds_are_pinned() {
    for c in [
        '\u{feff}',
        '\u{2060}',
        '\u{180e}',
        '\u{e0000}',
        '\u{e0001}',
        '\u{e007f}',
    ] {
        assert!(
            is_invisible_spoofing_control(c),
            "U+{:X} must be covered",
            c as u32
        );
    }
    // The excluded-but-adjacent family must stay excluded so the data
    // lanes above can assert verbatim survival.
    for c in ['\u{00ad}', '\u{200b}', '\u{200c}', '\u{200d}'] {
        assert!(!is_spoofing_format_control(c), "{c:?} wrongly flagged");
    }
    // The Variation Selectors Supplement sharing plane 14 (Mn, not Cf)
    // is functional orthography — the tag-block range ends at U+E007F.
    for c in ['\u{e0080}', '\u{e0100}', '\u{e01ef}'] {
        assert!(
            !is_spoofing_format_control(c),
            "U+{:X} wrongly flagged",
            c as u32
        );
    }
}

#[test]
fn plane14_tag_block_is_neutralized_and_json_escapes_as_surrogate_pair() {
    // review-6f85e2c9 F1: the acceptance's "U+E0000-tagged ASCII" arm,
    // now executed — render/document lanes substitute U+FFFD, the JSON
    // lane escapes losslessly.
    let tagged = "g\u{e0000}tag\u{e0001}\u{e007f}";
    assert_eq!(
        sanitize_line_segment(tagged),
        "g\u{fffd}tag\u{fffd}\u{fffd}"
    );
    assert_eq!(
        sanitize_output_document(tagged),
        "g\u{fffd}tag\u{fffd}\u{fffd}"
    );
    let serialized =
        serde_json::to_string(&serde_json::Value::String(tagged.to_string())).expect("serializes");
    let escaped = escape_spoofing_json(&serialized);
    // A JSON \u escape is exactly four hex digits — a non-BMP char MUST
    // ride as its surrogate pair. A five-digit \uE0001 would decode to
    // U+E000 plus a literal "1", silently corrupting the machine lane.
    assert!(escaped.contains("\\uDB40\\uDC01"), "got: {escaped}");
    assert!(!escaped.contains("\\uE0001"));
    let decoded: String = serde_json::from_str(&escaped).expect("escaped stream is valid JSON");
    assert_eq!(
        decoded, tagged,
        "json lane must be lossless for the tag block"
    );
    assert_eq!(escape_spoofing_json(&escaped), escaped, "idempotent");
}

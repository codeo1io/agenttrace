//! rm-760 — usage-truth disclosure on every claiming lane; rm-449 F3/F4
//! fold arms (usage_non_object key + estimated-token marker;
//! usage_key_non_numeric label).
//!
//! Corpus: the run-ec762a618372 assess F1-F4 PoC mirror (attempt
//! 984bcea3, base 92149bd — `delegate/984bcea3…-scratch/poc-mirror/`),
//! redacted into fixtures. Every fixture here measured RED on the
//! pre-batch build at 92149bd; each test names the old wrong behavior
//! so flipping back is the regression gate:
//!
//! 1. `alias-single-object.jsonl` — one hermes event line as a
//!    single-JSON-object document was claimed by the gemini lane
//!    (`usage` container probe) and re-accounted 100/50 under
//!    `gemini_cli` with ZERO disclosures, while the byte-identical
//!    journal split across two lines reported 0/0 +
//!    `usage_alias_unmapped` on `hermes_jsonl`. Token truth and tool
//!    attribution must not flip on line count (rm-760 arm 1).
//! 2. `unknown-meta-single-object.jsonl` — same swallowing; a usage
//!    block of only unknown keys surfaced no `usage_unknown_key`
//!    disclosure (rm-760 arm 2).
//! 3. `caps-*.jsonl` / `msgcaps.jsonl` — the classifier probed only
//!    lowercase `usage` while `Event` deserializes both `Usage` and
//!    `usage` (lib.rs rename+alias): capital-spelled containers were
//!    invisible to the disclosure pass (silently zeroed or silently
//!    estimated), and an all-noncanonical NONZERO map skipped rm-408's
//!    `zero_usage_reported` rider (rm-760 arm 3).
//! 4. `hermes-whole.json` — whole-JSON hermes documents accounted the
//!    root usage map but silently dropped both the unknown root key
//!    and every per-message usage container (rm-760 arm 4).
//! 5. `nonobj-*.jsonl` / `lonesurr-key.jsonl` — a usage value that is
//!    not a container (`"usage":5`, `"usage":[]`) rode lenient repair
//!    to a dropped field and fabricated estimated tokens with no
//!    marker, and a canonical key with a non-numeric value was
//!    mislabeled `usage_unknown_key` instead of naming the value
//!    defect (rm-449 F3/F4).
//! 6. `reasoning-fold-*.json` / `gemini-shadowed-alias.json` /
//!    `gemini-nested-usage.json` — review e747789a fixes F1-F3: the
//!    reasoning fold consumed its winning wire key without marking it
//!    (standing false `usage_unconsumed_location:reasoning_tokens` /
//!    `usage_alias_unmapped:thoughtsTokenCount` on a fully accounted
//!    container), the gemini scan marked the whole alias vocabulary
//!    consumed while `gemini_usage` reads only the first present key
//!    per class (a shadowed duplicate alias's value vanished with zero
//!    disclosure), and the scan skipped the nested
//!    checkpoint/session/chat wrappers the parser recurses into (an
//!    unknown usage key inside `checkpoint.usage` disclosed nowhere).

use agenttrace_core::{parse_file, report_text};
use std::path::PathBuf;

fn fixture(name: &str) -> agenttrace_core::Session {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/usage-truth")
        .join(name);
    parse_file(&path).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn alias_pair_accounts_identically_single_object_vs_multiline() {
    // rm-760 arm 1. Old behavior: single-object variant was claimed by
    // the gemini lane — source `gemini_cli`, 100/50 tokens via alias
    // mapping, zero disclosures; the multiline variant was
    // `hermes_jsonl` 0/0 with the alias disclosures. Line count must
    // not decide the accounting.
    let single = fixture("alias-single-object.jsonl");
    let multi = fixture("alias-multiline.jsonl");

    assert_eq!(single.metrics.source_tool, "hermes_jsonl");
    assert_eq!(multi.metrics.source_tool, "hermes_jsonl");

    // Token truth: the alias names map to no canonical class on this
    // lane — 0/0 in both containers.
    assert_eq!(single.metrics.tokens_input, 0);
    assert_eq!(single.metrics.tokens_output, 0);
    assert_eq!(multi.metrics.tokens_input, 0);
    assert_eq!(multi.metrics.tokens_output, 0);

    // Disclosures: identical alias-unmapped keys in both containers.
    // Integration re-base (conflict case a042df3e): rm-616 (landed on
    // HEAD after this batch's base 92149bd) adds its line-level rider
    // `usage_present_not_counted` for the multi variant — its usage
    // rides an assistant line the meta-lane fold never reads, exactly
    // the shape that rider exists for. The per-key alias truth stays
    // identical; the single-object variant (usage on the meta line
    // itself) keeps the bare map.
    let expected = std::collections::BTreeMap::from([
        ("usage_alias_unmapped:completion_tokens".to_string(), 1),
        ("usage_alias_unmapped:prompt_tokens".to_string(), 1),
    ]);
    let mut expected_with_rider = expected.clone();
    expected_with_rider.insert("usage_present_not_counted".to_string(), 1);
    assert_eq!(single.metrics.line_skips, expected);
    assert_eq!(multi.metrics.line_skips, expected_with_rider);

    // The rider: an all-alias map carries no consumed class, so it
    // reports zero usable tokens in both containers (rm-760 arm 3
    // honesty applied consistently).
    assert!(single
        .metrics
        .provenance
        .tokens
        .contains("+zero_usage_reported:1"));
    assert!(multi
        .metrics
        .provenance
        .tokens
        .contains("+zero_usage_reported:1"));
}

#[test]
fn single_object_unknown_usage_key_discloses() {
    // rm-760 arm 2. Old behavior: claimed as `gemini_cli` 0/0 with ZERO
    // disclosures cold and warm (assess fz/ctl probes).
    let session = fixture("unknown-meta-single-object.jsonl");
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(session.metrics.tokens_input, 0);
    assert_eq!(
        session.metrics.line_skips,
        std::collections::BTreeMap::from([("usage_unknown_key:bogus_metric".to_string(), 1)])
    );
}

#[test]
fn case_matched_usage_discloses_with_zero_rider() {
    // rm-760 arm 3. Old behavior: capital `Usage` containers were
    // invisible to the classifier (caps-assistant silently 0/0 with
    // estimated_from_text fabrication, msgcaps silently dropped) and
    // caps-meta-bogus reported_by_agent WITHOUT rm-408's rider.
    let assistant = fixture("caps-assistant.jsonl");
    assert_eq!(assistant.metrics.source_tool, "hermes_jsonl");
    assert_eq!(assistant.metrics.tokens_input, 0);
    assert_eq!(assistant.metrics.tokens_output, 0);
    // The usage block existed at a place this lane's accounting never
    // reads — disclosed, not silently fabricated.
    // Integration re-base (conflict case a042df3e): rm-616's landed
    // line-level rider fires beside the per-key truth for a
    // conversation line carrying usage (see alias_pair note above).
    assert_eq!(
        assistant.metrics.line_skips,
        std::collections::BTreeMap::from([
            ("usage_present_not_counted".to_string(), 1),
            ("usage_unconsumed_location:input_tokens".to_string(), 1),
            ("usage_unconsumed_location:output_tokens".to_string(), 1),
        ])
    );
    assert_eq!(assistant.metrics.provenance.tokens, "estimated_from_text");

    let bogus = fixture("caps-meta-bogus.jsonl");
    assert_eq!(
        bogus.metrics.line_skips,
        std::collections::BTreeMap::from([("usage_unknown_key:bogus_metric".to_string(), 1)])
    );
    // An all-noncanonical NONZERO map reports no usable tokens — the
    // rm-408 rider fires instead of silently upgrading to clean
    // reported_by_agent.
    assert_eq!(
        bogus.metrics.provenance.tokens,
        "reported_by_agent+zero_usage_reported:1"
    );

    let msgcaps = fixture("msgcaps.jsonl");
    assert_eq!(
        msgcaps.metrics.line_skips,
        std::collections::BTreeMap::from([(
            "usage_unconsumed_location:input_tokens".to_string(),
            1
        )])
    );
}

#[test]
fn case_matched_meta_usage_accounts_cleanly() {
    // rm-760 arm 3 control: `Usage` on the meta line deserializes into
    // Event.usage (rename+alias) and is consumed — accounted 100/50,
    // no disclosure, no rider.
    let session = fixture("caps-meta.jsonl");
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 50);
    assert!(session.metrics.line_skips.is_empty());
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
}

#[test]
fn hermes_whole_json_accounts_root_and_discloses_rest() {
    // rm-760 arm 4. Old behavior: root usage accounted (input 10) but
    // the unknown root key AND the per-message usage container were
    // both silent.
    let session = fixture("hermes-whole.json");
    assert_eq!(session.metrics.source_tool, "hermes_json");
    assert_eq!(session.metrics.tokens_input, 10);
    assert_eq!(
        session.metrics.line_skips,
        std::collections::BTreeMap::from([
            // messages[0].usage is not added to the root map (that
            // would double-count a session-total) — it discloses as
            // present-at-an-unread place instead of vanishing.
            ("usage_unconsumed_location:input_tokens".to_string(), 1),
            ("usage_unknown_key:bogus_metric".to_string(), 1),
        ])
    );
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
}

#[test]
fn non_object_usage_discloses_and_marks_estimates() {
    // rm-449 F3. Old behavior: `"usage":5` / `"usage":[]` rode lenient
    // repair to a dropped field — estimated tokens with no marker that
    // a usage block was present at all.
    for (name, kind) in [
        ("nonobj-int.jsonl", "number"),
        ("nonobj-arr.jsonl", "array"),
    ] {
        let session = fixture(name);
        assert_eq!(
            session.metrics.line_skips,
            std::collections::BTreeMap::from([(format!("usage_non_object:{kind}"), 1)]),
            "{name}"
        );
        // The provenance suffix distinguishes "estimated because no
        // usage" from "estimated although a usage block existed".
        assert_eq!(
            session.metrics.provenance.tokens, "estimated_from_text+usage_unusable:1",
            "{name}"
        );
        // The human text renderer carries the marker (rm-449 F3 seam).
        let text = report_text(&session);
        assert!(
            text.contains("usage present but unusable"),
            "{name}: marker missing from report_text"
        );
    }
}

#[test]
fn non_numeric_canonical_key_labels_non_numeric() {
    // rm-449 F4. Old behavior: `input_tokens:"\ud800"` (a lone
    // surrogate string) was labeled `usage_unknown_key:input_tokens`,
    // contradicting the unknown-key semantics — the name IS canonical;
    // the value is the defect.
    let session = fixture("lonesurr-key.jsonl");
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(
        session.metrics.line_skips,
        std::collections::BTreeMap::from([("usage_key_non_numeric:input_tokens".to_string(), 1)])
    );
    // The lenient map drops the unusable value, so no usable tokens —
    // but the disclosure names why.
    assert_eq!(session.metrics.provenance.tokens, "estimated_from_text");
}

#[test]
fn assistant_line_usage_no_longer_gemini_claimed() {
    // rm-760 lane honesty companion: a lone assistant event line with
    // lowercase usage was claimed by the gemini lane as 100/50
    // `gemini_cli`. It is a hermes journal fragment: the honest
    // accounting keeps 0/0 with the usage disclosed at its unread
    // place (the gemini lane's alias mapping does not get to
    // re-account a foreign format's fragment).
    let session = fixture("lc-assistant.jsonl");
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(session.metrics.tokens_input, 0);
    assert_eq!(session.metrics.tokens_output, 0);
    // Integration re-base (conflict case a042df3e): rm-616's landed
    // line-level rider fires beside the per-key truth (see the
    // alias_pair note above) — the usage is disclosed twice over,
    // once per key and once for the line, never silently.
    assert_eq!(
        session.metrics.line_skips,
        std::collections::BTreeMap::from([
            ("usage_present_not_counted".to_string(), 1),
            ("usage_unconsumed_location:input_tokens".to_string(), 1),
            ("usage_unconsumed_location:output_tokens".to_string(), 1),
        ])
    );
}

#[test]
fn reasoning_fold_consumed_keys_disclose_nothing() {
    // Review e747789a F1. Old behavior: the reasoning fold added the
    // value to output and the breakdown but never pushed the winning
    // wire key onto `matched_keys`, so a FULLY accounted root usage
    // container minted a standing false disclosure —
    // `usage_unconsumed_location:reasoning_tokens` on
    // `reasoning-fold-root.json`, `usage_alias_unmapped:thoughtsTokenCount`
    // on the synonym variant. Tokens stay identical in both cases;
    // only the false disclosure disappears.
    let canonical = fixture("reasoning-fold-root.json");
    assert_eq!(canonical.metrics.source_tool, "hermes_json");
    assert_eq!(canonical.metrics.tokens_input, 100);
    assert_eq!(canonical.metrics.tokens_output, 55);
    assert_eq!(canonical.metrics.tokens_reasoning, 5);
    assert!(
        canonical.metrics.line_skips.is_empty(),
        "consumed reasoning key disclosed: {:?}",
        canonical.metrics.line_skips
    );
    assert_eq!(canonical.metrics.provenance.tokens, "reported_by_agent");

    let synonym = fixture("reasoning-fold-synonym.json");
    assert_eq!(synonym.metrics.source_tool, "hermes_json");
    assert_eq!(synonym.metrics.tokens_input, 100);
    assert_eq!(synonym.metrics.tokens_output, 5);
    assert_eq!(synonym.metrics.tokens_reasoning, 5);
    assert!(
        synonym.metrics.line_skips.is_empty(),
        "consumed reasoning synonym disclosed: {:?}",
        synonym.metrics.line_skips
    );
    assert_eq!(synonym.metrics.provenance.tokens, "reported_by_agent");

    // F1 completion (fix attempt dfbb49b2): consumption is numeracy-based
    // like the class winners, so an EXPLICIT ZERO — a standard member of
    // OpenAI-compatible usage objects — is consumed, not disclosed. Old
    // behavior of the first fix cut: the `> 0` filter also gated the
    // matched_keys push, minting a standing
    // `usage_unconsumed_location:reasoning_tokens` on
    // `usage:{prompt_tokens:100, completion_tokens:50,
    // reasoning_tokens:0}` and disagreeing with the JSONL lane
    // (USAGE_KEYS_CONSUMED lists reasoning_tokens).
    let zero = fixture("reasoning-fold-zero.json");
    assert_eq!(zero.metrics.source_tool, "hermes_json");
    assert_eq!(zero.metrics.tokens_input, 100);
    assert_eq!(zero.metrics.tokens_output, 50);
    assert_eq!(zero.metrics.tokens_reasoning, 0);
    assert!(
        zero.metrics.line_skips.is_empty(),
        "zero-valued reasoning key disclosed: {:?}",
        zero.metrics.line_skips
    );
    assert_eq!(zero.metrics.provenance.tokens, "reported_by_agent");
}

#[test]
fn gemini_shadowed_duplicate_alias_discloses_not_drops() {
    // Review e747789a F2. Old behavior: the doc scan marked the entire
    // GEMINI_*_KEYS vocabulary consumed while `gemini_usage` reads only
    // the FIRST present key per class — `input_tokens:50` shadowed by
    // `promptTokenCount:100` vanished with zero disclosure. The
    // shadowed duplicate must disclose at its unread place instead.
    let session = fixture("gemini-shadowed-alias.json");
    assert_eq!(session.metrics.source_tool, "gemini_cli");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 0);
    assert_eq!(
        session.metrics.line_skips,
        std::collections::BTreeMap::from([(
            "usage_unconsumed_location:input_tokens".to_string(),
            1
        )])
    );
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
}

#[test]
fn gemini_nested_wrapper_usage_is_scanned() {
    // Review e747789a F3. Old behavior: `parse_gemini_object` recurses
    // into `checkpoint`/`session`/`chat` and claims the usage container
    // there, but the doc scan stayed top-level — `bogus_metric:7`
    // inside `checkpoint.usage` disclosed nowhere. The scan must
    // mirror the recursion at every depth.
    let session = fixture("gemini-nested-usage.json");
    assert_eq!(session.metrics.source_tool, "gemini_cli");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 0);
    assert_eq!(
        session.metrics.line_skips,
        std::collections::BTreeMap::from([("usage_unknown_key:bogus_metric".to_string(), 1)])
    );
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
}

/// One whole-JSON hermes document whose 1,500 messages each carry a
/// DISTINCT unknown usage key — the rm-594 cardinality shape pressed
/// onto the rm-760 document lanes (integration review 865c4bc6: the
/// batch minted its document-lane disclosures straight into
/// `line_skips`, bypassing the capped mint every other wire-named
/// site rides; the landed cap contract — `disclosure_distinct_keys_
/// are_capped_and_the_cap_discloses_itself` — must hold on every
/// claiming lane).
fn cardinality_hermes_document(entries: usize) -> String {
    let mut messages = String::new();
    for i in 0..entries {
        if i > 0 {
            messages.push(',');
        }
        messages.push_str(&format!(
            r#"{{"role":"user","content":"x","usage":{{"zz_{i}":1}}}}"#
        ));
    }
    format!(
        r#"{{"platform":"pi","session_id":"cap","model":"m","session_start":"2026-10-07T01:00:00Z","last_updated":"2026-10-07T01:05:00Z","usage":{{"input_tokens":10}},"messages":[{messages}]}}"#
    )
}

/// Same shape on the gemini document lane: one claimed usage
/// container carrying 1,500 distinct unknown keys beside a consumed
/// canonical alias.
fn cardinality_gemini_document(entries: usize) -> String {
    let mut keys = String::from("\"promptTokenCount\":100");
    for i in 0..entries {
        keys.push_str(&format!(",\"zz_{i}\":1"));
    }
    format!(r#"{{"checkpoint":{{"modelVersion":"gemini-2.0-flash","usage":{{{keys}}}}}}}"#)
}

#[test]
fn document_lane_disclosures_ride_the_distinct_key_cap() {
    // Integration review 865c4bc6 (rm-760 × the landed rm-594
    // residual): the run's document-lane scans mint wire-named usage
    // keys (`usage_unknown_key:<name>`) of their own, and a hostile
    // document presses exactly the cardinality hazard the landed cap
    // closed for the JSONL/array lanes — 1,500 distinct keys entered
    // `line_skips` unbounded on the merged tree with nothing naming
    // the growth. Both document lanes must ride the SAME capped mint:
    // 1,000 distinct keys enter, `disclosure_keys_capped` counts the
    // suppression, and the cap is a disclosure-channel bound only
    // (events still all parse).
    let dir = std::env::temp_dir().join(format!("rm760-doccap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tempdir");
    for (name, doc) in [
        ("hermes-doccap.json", cardinality_hermes_document(1_500)),
        ("gemini-doccap.json", cardinality_gemini_document(1_500)),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, doc).expect("fixture write");
        let session = parse_file(&path).unwrap_or_else(|error| panic!("{name}: {error}"));
        let distinct = session
            .metrics
            .line_skips
            .keys()
            .filter(|key| key.starts_with("usage_"))
            .count();
        assert_eq!(
            distinct, 1_000,
            "{name}: distinct wire-controlled disclosure keys must cap at 1,000"
        );
        assert_eq!(
            session.metrics.line_skips.get("disclosure_keys_capped"),
            Some(&500),
            "{name}: the cap must count its own suppression"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

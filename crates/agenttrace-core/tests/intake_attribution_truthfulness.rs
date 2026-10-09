//! Intake → attribution → disclosure truthfulness (cycle 1 batch
//! rm-863 + rm-860 + rm-861, run 5eb82325ebea; rider rm-862's doctor
//! vocabulary is pinned inside `doctor.rs`'s internal tests).
//!
//! - **rm-863 (LEAD): reasoning-token producer semantics.** The
//!   usage folds added EVERY reasoning alias to the output bucket
//!   ("billed at the output rate", pass-9 CU-20). That bills
//!   subset-shaped producers — whose reported completion bucket
//!   already CONTAINS the thinking span — twice. The folds now follow
//!   the matched wire key's semantics: `thoughtsTokenCount` /
//!   `thinkingTokenCount` / antigravity `reasoningTokens` are SIBLING
//!   buckets beside the reported output and still add; the flat
//!   OpenAI-compatible spellings `reasoning_tokens` /
//!   `thinking_tokens` are the documented breakdown INSIDE
//!   completion_tokens and only record the breakdown. The live repro
//!   was the hermes agent's own session dump (research ef1ca6d9 PoC
//!   `hermes-flat-reasoning.json`: completion 400 + reasoning 150
//!   reported out 550).
//! - **rm-860: duplicate-spelling intake.** A generic-lane journal
//!   row carrying BOTH spellings of an `Event` field (`ModelUsed` +
//!   `model_used`, any serde rename+alias pair) tripped serde's
//!   duplicate-field error and dropped the whole line as
//!   `event_schema` — its content and usage vanished from every
//!   surface. The strict parse now retries against pre-normalized
//!   elements under the disclosed canonical-winner rule (canonical
//!   spelling wins unless it is null), counting
//!   `duplicate_spelling_normalized` on the informational channel —
//!   never as parse loss. The equivalence contract: a both-spellings
//!   doc behaves exactly like its single-spelling controls plus that
//!   one counter.
//! - **rm-861: model-label authority.** The session model label scan
//!   was last-wins over every role, so a terminal user row naming a
//!   different model relabeled the whole session (the label feeds
//!   by_model and pricing). Only authoritative rows — assistant
//!   turns, session/meta identity rows, and usage-bearing rows — may
//!   (re)write a named label; a non-authoritative row may still SEED
//!   the label off `"default"`, and its differing relabeling attempt
//!   is ignored and disclosed as
//!   `model_label_non_assistant_ignored`.
//!
//! Lane notes used below: role+timestamp JSONL classifies as
//! `hermes_jsonl` (`jsonl_source_tool`), whose counted usage lane is
//! the session-meta arm — conversation-row usage there is disclosed
//! (`usage_present_not_counted`), not counted (the rm-616 boundary
//! pinned by `usage_on_non_meta_rows_stays_estimated_from_text`).
//! Role-only JSONL (no timestamps) and JSON-array docs classify as
//! `generic`, whose conversation-row usage IS the counted lane. Both
//! shapes are exercised.
//!
//! Fixtures live under `tests/fixtures/generic/` and
//! `tests/fixtures/hermes-hostile/` (the research PoCs copied
//! verbatim).

use agenttrace_core::{parse_file, parse_raw_session, Session};
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

fn parse(rel: &str) -> Session {
    let path = fixture(rel);
    parse_file(&path).unwrap_or_else(|error| panic!("parse {path:?}: {error:#}"))
}

// ---------------------------------------------------------------------
// rm-863 — reasoning-token producer semantics
// ---------------------------------------------------------------------

/// The research PoC corpus, verbatim: a hermes session dump whose
/// usage object carries the flat OpenAI-compatible spelling
/// (`completion_tokens` 400 + `reasoning_tokens` 150). Pre-fix the
/// lane reported out 550 — the thinking span billed twice.
#[test]
fn flat_reasoning_subset_does_not_double_bill_output() {
    let session = parse("hermes-hostile/flat-reasoning-subset.json");
    assert_eq!(session.metrics.source_tool, "hermes_json");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(
        session.metrics.tokens_output, 400,
        "rm-863: reasoning_tokens is a breakdown INSIDE completion_tokens — \
         the pre-fix fold reported 550"
    );
    assert_eq!(
        session.metrics.tokens_reasoning, 150,
        "the matched span stays visible as the reasoning breakdown"
    );
}

/// Sibling control (research ef1ca6d9, verbatim): the same session
/// shape with NO reasoning key reports the plain completion count —
/// the subset fix changed nothing for docs without the alias.
#[test]
fn reasoning_absent_control_reports_plain_completion() {
    let session = parse("hermes-hostile/flat-reasoning-absent.json");
    assert_eq!(session.metrics.source_tool, "hermes_json");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(session.metrics.tokens_output, 400);
    assert_eq!(session.metrics.tokens_reasoning, 0);
}

/// The camelCase Gemini-style metadata (`thoughtsTokenCount`) keeps
/// the sibling fold on the hermes session-dump lane: the reported
/// completion bucket EXCLUDES the thoughts span, so it still adds
/// (completion 400 + thoughts 150 → out 550). The same rule on the
/// Gemini checkpoint lane is pinned end-to-end by the discovery
/// contract tests.
#[test]
fn camelcase_thoughts_stay_a_sibling_bucket_on_the_dump_lane() {
    let session = parse("hermes-hostile/camelcase-thoughts-sibling.json");
    assert_eq!(session.metrics.source_tool, "hermes_json");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(session.metrics.tokens_output, 550, "400 + 150 sibling add");
    assert_eq!(session.metrics.tokens_reasoning, 150);
}

/// The flat spelling on the same generic lane: recorded as the
/// breakdown WITHOUT adding — the qwen-shaped subset contract.
#[test]
fn flat_reasoning_records_the_breakdown_without_adding() {
    let session = parse_raw_session(
        "subset.jsonl",
        "subset.jsonl",
        concat!(
            "{\"role\":\"user\",\"content\":\"hi\"}\n",
            "{\"role\":\"assistant\",\"model_used\":\"qwen3-thinking\",",
            "\"usage\":{\"input_tokens\":100,\"output_tokens\":50,\"reasoning_tokens\":40}}\n"
        ),
    )
    .expect("parse");
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(
        session.metrics.tokens_output, 50,
        "rm-863: flat reasoning_tokens never adds to the output bucket"
    );
    assert_eq!(session.metrics.tokens_reasoning, 40);
}

// ---------------------------------------------------------------------
// rm-860 — duplicate-spelling intake
// ---------------------------------------------------------------------

/// The rm-860 PoC corpus: one assistant row carrying BOTH `ModelUsed`
/// and `model_used` (with a plain user row after it). Pre-fix the
/// assistant row dropped as `event_schema=1` + `model_or_usage_dropped=1`
/// and the session attributed to `"default"`. Post-fix it behaves
/// exactly like its single-spelling controls (hermes_jsonl lane:
/// conversation usage disclosed, not counted — see the module notes)
/// plus one informational counter.
#[test]
fn both_spellings_parse_under_the_canonical_winner_rule() {
    let session = parse("generic/model-used-both-spellings.jsonl");
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(
        session.metrics.model_used, "claude-sonnet-4-5-20250929",
        "canonical ModelUsed spelling wins (both spellings agree here)"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("duplicate_spelling_normalized"),
        Some(&1),
        "the normalization is disclosed on the informational channel: {:#?}",
        session.metrics.disclosure_counters
    );
    assert!(
        !session.metrics.line_skips.contains_key("event_schema"),
        "a normalized spelling is not parse loss: line_skips {:#?}",
        session.metrics.line_skips
    );
    assert!(
        !session
            .metrics
            .line_skips
            .contains_key("model_or_usage_dropped"),
        "the usage no longer drops with the line: line_skips {:#?}",
        session.metrics.line_skips
    );
    // The equivalence contract: same lane semantics as the
    // single-spelling controls — conversation usage disclosed, not
    // counted (rm-616 boundary), text-estimated token provenance.
    assert_eq!(
        session.metrics.provenance.tokens, "estimated_from_text",
        "identical to the pascal-only/lower-only control class"
    );
    assert_eq!(
        session.metrics.line_skips.get("usage_present_not_counted"),
        Some(&1),
        "the assistant row's usage is disclosed exactly like a \
         single-spelling doc of this shape: {:#?}",
        session.metrics.line_skips
    );
}

/// The JSON-array arm of the same intake (a `.json` doc of `Event`
/// arrays rather than JSONL lines) takes the same retry: pre-fix the
/// whole ARRAY dropped to the hermes fallback and lost the Event
/// lane's usage. Array docs classify as `generic`, so their
/// conversation-row usage is the counted lane.
#[test]
fn both_spellings_parse_in_the_array_arm_too() {
    let raw = concat!(
        "[\n",
        " {\"role\":\"assistant\",",
        "\"ModelUsed\":\"claude-sonnet-4-5-20250929\",\"model_used\":\"claude-sonnet-4-5-20250929\",",
        "\"usage\":{\"input_tokens\":100,\"output_tokens\":50}},\n",
        " {\"role\":\"user\",\"content\":\"hello\"}\n",
        "]"
    );
    let session = parse_raw_session("array.json", "array.json", raw).expect("parse array doc");
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(session.metrics.model_used, "claude-sonnet-4-5-20250929");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 50);
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("duplicate_spelling_normalized"),
        Some(&1),
        "array arm discloses the normalization too: {:#?}",
        session.metrics.disclosure_counters
    );
}

/// The canonical-winner rule, not the alias: when the two spellings
/// DISAGREE, the canonical (serde `rename`) spelling decides the
/// attribution — a hostile alias cannot override it.
#[test]
fn canonical_spelling_wins_when_the_pair_disagrees() {
    let session = parse_raw_session(
        "disagree.jsonl",
        "disagree.jsonl",
        concat!(
            "{\"role\":\"user\",\"content\":\"hi\"}\n",
            "{\"role\":\"assistant\",",
            "\"ModelUsed\":\"claude-sonnet-4-5-20250929\",\"model_used\":\"hostile-alias\",",
            "\"usage\":{\"input_tokens\":100,\"output_tokens\":50}}\n"
        ),
    )
    .expect("parse");
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(
        session.metrics.model_used, "claude-sonnet-4-5-20250929",
        "canonical wins over a disagreeing alias"
    );
    assert_eq!(session.metrics.tokens_output, 50);
}

/// A null canonical yields to the alias: the pair collapses to the
/// spelling that actually carries a value.
#[test]
fn null_canonical_yields_to_the_alias() {
    let session = parse_raw_session(
        "null.jsonl",
        "null.jsonl",
        concat!(
            "{\"role\":\"user\",\"content\":\"hi\"}\n",
            "{\"role\":\"assistant\",",
            "\"ModelUsed\":null,\"model_used\":\"claude-haiku-4-5-20251001\",",
            "\"usage\":{\"input_tokens\":100,\"output_tokens\":50}}\n"
        ),
    )
    .expect("parse");
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(
        session.metrics.model_used, "claude-haiku-4-5-20251001",
        "a null canonical contributes nothing — the alias carries the value"
    );
    assert_eq!(session.metrics.tokens_output, 50);
}

/// Single-spelling docs never take the retry path: the assess-era
/// positive controls (pascal-only, lower-only) parse byte-identically
/// with NO normalization counter.
#[test]
fn single_spelling_docs_stay_untouched() {
    let pascal = parse_raw_session(
        "pascal.jsonl",
        "pascal.jsonl",
        concat!(
            "{\"role\":\"user\",\"content\":\"hi\"}\n",
            "{\"role\":\"assistant\",",
            "\"ModelUsed\":\"claude-sonnet-4-5-20250929\",",
            "\"usage\":{\"input_tokens\":100,\"output_tokens\":50}}\n"
        ),
    )
    .expect("parse pascal");
    assert_eq!(pascal.metrics.model_used, "claude-sonnet-4-5-20250929");
    assert_eq!(pascal.metrics.tokens_output, 50);
    assert!(
        !pascal
            .metrics
            .disclosure_counters
            .contains_key("duplicate_spelling_normalized"),
        "no counter for single-spelling docs: {:#?}",
        pascal.metrics.disclosure_counters
    );

    let lower = parse_raw_session(
        "lower.jsonl",
        "lower.jsonl",
        concat!(
            "{\"role\":\"user\",\"content\":\"hi\"}\n",
            "{\"role\":\"assistant\",",
            "\"model_used\":\"claude-sonnet-4-5-20250929\",",
            "\"usage\":{\"input_tokens\":100,\"output_tokens\":50}}\n"
        ),
    )
    .expect("parse lower");
    assert_eq!(lower.metrics.model_used, "claude-sonnet-4-5-20250929");
    assert_eq!(lower.metrics.tokens_output, 50);
    assert!(
        !lower
            .metrics
            .disclosure_counters
            .contains_key("duplicate_spelling_normalized"),
        "no counter for single-spelling docs: {:#?}",
        lower.metrics.disclosure_counters
    );
}

// ---------------------------------------------------------------------
// rm-861 — model-label authority
// ---------------------------------------------------------------------

/// The assess F3 PoC corpus: an assistant row names opus (with
/// usage), then a terminal user row names haiku with NO usage. The
/// last-wins scan attributed the session to haiku; opus's spend rode
/// a mispriced label. (Timestamped lines classify as hermes_jsonl —
/// usage stays disclosed-not-counted there; the LABEL is what this
/// test pins, and it is lane-independent.)
#[test]
fn user_row_cannot_relabel_the_session_model() {
    let session = parse("generic/model-label-user-hijack.jsonl");
    assert_eq!(
        session.metrics.model_used, "claude-opus-4-1-20250805",
        "the usage-bearing assistant row keeps the label"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("model_label_non_assistant_ignored"),
        Some(&1),
        "the ignored relabeling attempt is disclosed: {:#?}",
        session.metrics.disclosure_counters
    );
    assert!(
        !session.metrics.line_skips.contains_key("event_schema"),
        "an ignored relabel is not parse loss: {:#?}",
        session.metrics.line_skips
    );
}

/// The generic counted lane shows the pricing-side effect: opus's
/// usage prices under the opus label (pre-fix the last-wins scan
/// relabeled to haiku AFTER the usage rows, mispricing the session).
#[test]
fn generic_lane_usage_prices_under_the_authoritative_label() {
    let session = parse_raw_session(
        "hijack.jsonl",
        "hijack.jsonl",
        concat!(
            "{\"role\":\"user\",\"content\":\"hi\"}\n",
            "{\"role\":\"assistant\",\"model_used\":\"claude-opus-4-1-20250805\",",
            "\"usage\":{\"input_tokens\":100,\"output_tokens\":50}}\n",
            "{\"role\":\"user\",\"content\":\"stop\",\"model_used\":\"claude-haiku-4-5-20251001\"}\n"
        ),
    )
    .expect("parse");
    assert_eq!(session.metrics.source_tool, "generic");
    assert_eq!(session.metrics.model_used, "claude-opus-4-1-20250805");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 50);
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("model_label_non_assistant_ignored"),
        Some(&1)
    );
    assert_eq!(
        session.metrics.cost_estimated, 0.0053,
        "priced under opus (15/75 per 1M → 0.00525 → round4): the \
         pre-fix haiku label would price 0.0004"
    );
}

/// A non-authoritative row may still SEED the label off `"default"`:
/// lanes that stamp the model only on tool rows keep their
/// attribution, and no counter fires when the seed AGREES with the
/// standing label.
#[test]
fn non_assistant_row_may_seed_the_label_off_default() {
    let session = parse_raw_session(
        "seed.jsonl",
        "seed.jsonl",
        concat!(
            "{\"role\":\"user\",\"content\":\"hi\",",
            "\"model_used\":\"claude-haiku-4-5-20251001\"}\n",
            "{\"role\":\"user\",\"content\":\"again\",",
            "\"model_used\":\"claude-haiku-4-5-20251001\"}\n"
        ),
    )
    .expect("parse");
    assert_eq!(
        session.metrics.model_used, "claude-haiku-4-5-20251001",
        "the first non-assistant row seeds the label off default"
    );
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("model_label_non_assistant_ignored"),
        "an agreeing re-stamp is not an ignored relabel: {:#?}",
        session.metrics.disclosure_counters
    );
}

/// Assistant turns stay authoritative end-to-end: a later assistant
/// row relabels the session. No usage blocks here, so the rm-541
/// multi-model marker (usage blocks naming a model the label does
/// not match — pinned separately by the usage-occurrence contract
/// tests) does not fire and the relabel stays visible as the label.
#[test]
fn assistant_relabels_stay_authoritative() {
    let session = parse_raw_session(
        "switch.jsonl",
        "switch.jsonl",
        concat!(
            "{\"role\":\"user\",\"content\":\"hi\"}\n",
            "{\"role\":\"assistant\",\"model_used\":\"claude-sonnet-4-5-20250929\",",
            "\"content\":\"first\"}\n",
            "{\"role\":\"assistant\",\"model_used\":\"claude-opus-4-1-20250805\",",
            "\"content\":\"second\"}\n"
        ),
    )
    .expect("parse");
    assert_eq!(
        session.metrics.model_used, "claude-opus-4-1-20250805",
        "the later assistant turn relabels (both are authoritative rows)"
    );
    assert!(!session
        .metrics
        .disclosure_counters
        .contains_key("model_label_non_assistant_ignored"));
    assert_ne!(session.metrics.model_used, "multiple");
}

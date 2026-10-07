//! Disclosure-plane honesty — attribution, confidence, absence
//! (cycle 3 batch rm-718 + rm-719 + rm-716, run 5417681937ae).
//!
//! Three invariants about what the reports SAY rather than what they
//! count:
//!
//! - **Attribution (rm-718).** The generic fallback lane deserializes
//!   journal rows into `Event`, whose `model_used` field accepted only
//!   the PascalCase `ModelUsed` spelling (its sibling fields accept
//!   both spellings — `Event`'s serde aliases, `lib.rs`). A journal
//!   carrying the lowercase `model_used` key attributed to `"default"`
//!   and priced as unknown. The alias half closes that: the carried
//!   model name reports with catalog-or-unknown provenance, never
//!   `"default"`. The OTHER half of the same assess finding — folding
//!   a usage block that rides a non-meta role into the token totals
//!   (the `$22.05` PoC number) — is sibling row rm-694 (unlanded
//!   4c34ecaa band, `analyze` meta-arm fold) and is deliberately NOT
//!   implemented here; the boundary is pinned by
//!   `usage_on_non_meta_rows_stays_estimated_from_text` below.
//! - **Cache-read aliases (rm-718 F4 sweep arm).** The generic lane's
//!   usage intake accepted any scalar keys, so cross-provider
//!   spellings of the cache-read count (`cachedContentTokenCount`,
//!   `cacheReadInputTokens`) mapped nowhere: cache reads silently
//!   fell out of the cost fold. The sweep normalizes exactly those
//!   two spellings onto the canonical `cache_read_input_tokens` key
//!   at Event intake — the single deserialization choke point, which
//!   only the generic lane uses.
//! - **Confidence truthfulness (rm-719).** `kimi_usage_alias:<key>`
//!   counters are informational (the vendor's real wire keys were
//!   matched), yet they rode `Metrics.line_skips` — the parse-loss
//!   channel that degrades `data_health.confidence` and renders under
//!   "Dropped lines". They now ride `Metrics.disclosure_counters`
//!   (the rm-538 non-loss channel): visible in --doctor's "Disclosed
//!   facts", never degrading confidence. Genuine parse loss
//!   (unparseable lines) still degrades. `rm-400`'s
//!   every-alias-ships-a-counter rule is unchanged.
//! - **Absence (rm-716).** Pre-Sept-2026 codex rollouts (tokscale
//!   #1405 envelope) carry no `token_count` events and no
//!   `token_usage_record` rows, so the codex lane parsed them green
//!   with zero usage and no verdict — a silent zero. Those rollouts
//!   now carry a distinct `codex_rollout_no_usage_rows` disclosure
//!   on the non-loss channel, usage is never fabricated, and
//!   rollouts that DO carry usage rows report no verdict.
//!
//! Fixtures live under `tests/fixtures/generic/` and
//! `tests/fixtures/codex-legacy/` (see their READMEs).

use agenttrace_core::{
    compute_overview, data_health, parse_file, parse_raw_session, pricing_source_for,
    render_doctor_report, report_overview_markdown_with_context, Session, TimeRange,
};
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
// rm-718 — attribution
// ---------------------------------------------------------------------

/// The rm-718 PoC corpus: multi-line role-only JSONL (no `type` keys, so
/// the generic fallback lane parses it) whose assistant row carries the
/// lowercase `model_used` key. Pre-fix the session attributed to
/// `"default"` ($0.00, unpriced 1, by_model `default`).
#[test]
fn lowercase_model_used_attributes_the_named_model() {
    let session = parse("generic/model-used-lowercase.jsonl");
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(
        session.metrics.model_used, "claude-sonnet-4-5-20250929",
        "the lowercase model_used key must attribute the carried model, not default"
    );
    // Provenance: the name must be resolvable to a price — catalog
    // knowledge, not the built-in fallback that "default" implies.
    assert!(
        pricing_source_for(&session.metrics.model_used).contains("LiteLLM"),
        "carried model must price from the catalog: {}",
        pricing_source_for(&session.metrics.model_used)
    );
}

/// The same attribution flows through the report dimensions: the model
/// bucket and the vendor bucket carry the real name, and no `"default"`
/// or `"unknown"` bucket appears.
#[test]
fn lowercase_model_used_flows_into_the_report_dimensions() {
    let session = parse("generic/model-used-lowercase.jsonl");
    let overview = compute_overview(std::slice::from_ref(&session));
    assert!(
        overview.by_model.contains_key("claude-sonnet-4-5-20250929"),
        "by_model: {:#?}",
        overview.by_model
    );
    assert!(
        !overview.by_model.contains_key("default"),
        "by_model: {:#?}",
        overview.by_model
    );
    assert!(
        overview.by_provider.contains_key("anthropic"),
        "by_provider: {:#?}",
        overview.by_provider
    );
    assert!(
        !overview.by_provider.contains_key("unknown"),
        "by_provider: {:#?}",
        overview.by_provider
    );
}

/// Boundary pin for the title-twin split (must_remain_separate #1): the
/// usage block riding the NON-meta `assistant` row is sibling row
/// rm-694's half (the analyze meta-arm fold). This batch implements the
/// alias half only, so token provenance on that corpus stays
/// text-estimated — asserted here so rm-694's landing flips this test
/// consciously rather than drifting.
#[test]
fn usage_on_non_meta_rows_stays_estimated_from_text() {
    let session = parse("generic/model-used-lowercase.jsonl");
    assert_eq!(session.metrics.provenance.tokens, "estimated_from_text");
}

// ---------------------------------------------------------------------
// rm-718 F4 sweep arm — cache-read alias spellings
// ---------------------------------------------------------------------

/// A generic-lane usage block carrying the Gemini wire spelling of the
/// cache-read count. Pre-fix `cachedContentTokenCount` mapped nowhere:
/// tokens_cache_r stayed 0 and the cost fold billed every cached token
/// at the full input rate.
#[test]
fn camelcase_cache_read_maps_onto_the_canonical_key() {
    let session = parse("generic/cache-alias-camel.jsonl");
    assert_eq!(session.metrics.source_tool, "hermes_jsonl");
    assert_eq!(
        session.metrics.model_used, "claude-sonnet-4-5",
        "lowercase model_used alias applies to every generic-lane row"
    );
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(session.metrics.tokens_output, 100);
    assert_eq!(
        session.metrics.tokens_cache_r, 500,
        "cachedContentTokenCount is the cache-read count"
    );
}

/// When a block carries BOTH the canonical spelling and an alias
/// spelling, the canonical value wins — deterministic regardless of key
/// order, never summed.
#[test]
fn canonical_cache_read_wins_over_the_alias_spelling() {
    let raw_alias_first = concat!(
        r#"{"role":"session_meta","timestamp":"2026-10-07T12:00:00Z","model_used":"gpt-4o","#,
        r#""usage":{"input_tokens":10,"cachedContentTokenCount":999,"cache_read_input_tokens":7}}"#,
        "\n",
        r#"{"role":"user","timestamp":"2026-10-07T12:00:01Z","content":"q"}"#,
        "\n"
    );
    let session =
        parse_raw_session("alias-first.jsonl", "alias-first.jsonl", raw_alias_first).unwrap();
    assert_eq!(session.metrics.tokens_cache_r, 7);
    let raw_canonical_first = concat!(
        r#"{"role":"session_meta","timestamp":"2026-10-07T12:00:00Z","model_used":"gpt-4o","#,
        r#""usage":{"input_tokens":10,"cache_read_input_tokens":7,"cachedContentTokenCount":999}}"#,
        "\n",
        r#"{"role":"user","timestamp":"2026-10-07T12:00:01Z","content":"q"}"#,
        "\n"
    );
    let session = parse_raw_session(
        "canonical-first.jsonl",
        "canonical-first.jsonl",
        raw_canonical_first,
    )
    .unwrap();
    assert_eq!(session.metrics.tokens_cache_r, 7);
}

/// The other OpenAI-compatible camelCase spelling of the same count.
#[test]
fn openai_spelling_cache_read_maps_onto_the_canonical_key() {
    let raw = concat!(
        r#"{"role":"session_meta","timestamp":"2026-10-07T12:00:00Z","model_used":"gpt-4o","#,
        r#""usage":{"input_tokens":10,"cacheReadInputTokens":4}}"#,
        "\n",
        r#"{"role":"user","timestamp":"2026-10-07T12:00:01Z","content":"q"}"#,
        "\n"
    );
    let session = parse_raw_session("openai-spelling.jsonl", "openai-spelling.jsonl", raw).unwrap();
    assert_eq!(session.metrics.tokens_cache_r, 4);
}

// ---------------------------------------------------------------------
// rm-526 census pin (guards the F4 intake change against regressions
// in the counted-loss channel it shares with the generic lane)
// ---------------------------------------------------------------------

#[test]
fn flat_mixed_corpus_keeps_its_counted_skip_census() {
    let session = parse("generic/claude-flat-mixed.jsonl");
    assert_eq!(
        session.metrics.line_skips.get("non_object_line"),
        Some(&1),
        "line_skips: {:#?}",
        session.metrics.line_skips
    );
    assert!(
        session.metrics.line_skips.contains_key("unparseable_line"),
        "line_skips: {:#?}",
        session.metrics.line_skips
    );
}

// ---------------------------------------------------------------------
// rm-716 — codex absence disclosure
// ---------------------------------------------------------------------

/// The tokscale #1405 envelope (pre-Sept-2026): no `token_count`
/// events, no compaction rows. The session must carry a DISTINCT
/// absence verdict on the non-loss channel, must not fabricate usage,
/// and must keep the rollout's own attribution (model from
/// turn_context).
#[test]
fn legacy_rollout_reports_the_absence_verdict() {
    let session = parse("codex-legacy/rollout-no-usage.jsonl");
    assert_eq!(session.metrics.source_tool, "codex_cli");
    assert_eq!(
        session.metrics.model_used, "gpt-5-codex",
        "the rollout's own model attribution survives the absence verdict"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("codex_rollout_no_usage_rows"),
        Some(&1),
        "disclosure_counters: {:#?}",
        session.metrics.disclosure_counters
    );
    assert!(
        !session
            .metrics
            .line_skips
            .contains_key("codex_rollout_no_usage_rows"),
        "the verdict is an absence disclosure, not parse loss: line_skips {:#?}",
        session.metrics.line_skips
    );
    // Usage never fabricated: no usage rows exist in this rollout, so
    // token provenance stays text-estimated and the totals stay at the
    // estimator's scale, not a plausible-looking invented number.
    assert_eq!(session.metrics.provenance.tokens, "estimated_from_text");
    assert!(
        session.metrics.tokens_input < 10,
        "tokens_input {}",
        session.metrics.tokens_input
    );
    assert!(
        session.metrics.tokens_output < 10,
        "tokens_output {}",
        session.metrics.tokens_output
    );
}

/// The control: the identical envelope plus one `token_count` snapshot
/// counts real usage and reports NO absence verdict.
#[test]
fn modern_rollout_with_usage_reports_no_absence_verdict() {
    let session = parse("codex-legacy/rollout-with-usage.jsonl");
    assert_eq!(session.metrics.source_tool, "codex_cli");
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("codex_rollout_no_usage_rows"),
        "disclosure_counters: {:#?}",
        session.metrics.disclosure_counters
    );
    assert!(!session
        .metrics
        .line_skips
        .contains_key("codex_rollout_no_usage_rows"));
    assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    assert_eq!(
        session.metrics.tokens_input, 400,
        "1200 total minus 800 cached"
    );
    assert_eq!(session.metrics.tokens_cache_r, 800);
    assert_eq!(session.metrics.tokens_output, 300);
    assert_eq!(session.metrics.tokens_reasoning, 100);
}

/// The verdict renders where users look for it: the overview's
/// "Disclosed facts" row and --doctor's "Disclosed facts" line.
#[test]
fn absence_verdict_renders_in_overview_and_doctor() {
    let session = parse("codex-legacy/rollout-no-usage.jsonl");
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(
        health.disclosures.get("codex_rollout_no_usage_rows"),
        Some(&1),
        "health.disclosures: {:#?}",
        health.disclosures
    );
    let overview = compute_overview(std::slice::from_ref(&session));
    let markdown = report_overview_markdown_with_context(
        &overview,
        std::slice::from_ref(&session),
        &health,
        TimeRange::All,
        false,
    );
    assert!(
        markdown.contains("Disclosed facts"),
        "markdown:\n{markdown}"
    );
    assert!(
        markdown.contains("codex_rollout_no_usage_rows"),
        "markdown:\n{markdown}"
    );

    let dir = std::env::temp_dir().join(format!("agenttrace-codex-legacy-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create doctor scratch dir");
    std::fs::copy(
        fixture("codex-legacy/rollout-no-usage.jsonl"),
        dir.join("rollout-no-usage.jsonl"),
    )
    .expect("copy fixture for doctor scan");
    let text = render_doctor_report(Some(&dir), false, "text").expect("render doctor");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(text.contains("Disclosed facts"), "doctor:\n{text}");
    assert!(
        text.contains("codex_rollout_no_usage_rows"),
        "doctor:\n{text}"
    );
}

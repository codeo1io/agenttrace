//! rm-436 / rm-437 / rm-438 — pi-family journal accounting correctness.
//!
//! Fixtures are schema-faithful to the public versioned session-format
//! spec (packages/coding-agent/docs/session-format.md, pi 1.0.2) and to
//! the writer's wire keys confirmed in the @earendil-works/pi-coding-agent
//! 1.0.2 dist: `appendUsage(kind, provider, model, usage)` emits
//! `type:"usage"` entries whose model rides `modelId` (and whose spend
//! "contributes to session token and cost totals" per the spec), and
//! `appendModelChange(provider, modelId)` emits `type:"model_change"`
//! with the same `modelId` key — `cache-warmer.js` emits
//! `kind:"cache_warm"`. They are synthetic, not recordings of a real
//! host. Expected costs are computed through the same public pricing
//! lookup the crate uses, so catalog drift cannot rot these tests.

use std::fs;
use std::path::{Path, PathBuf};

use agenttrace_core::{
    build_doctor_report, data_health, lookup_price, parse_file, render_doctor_report, round4,
    Session,
};

/// A pi v3 journal whose cache-warm usage entry was invisible before
/// rm-436: the parser's `_ => {}` catch-all dropped `type:"usage"`
/// entirely, so 50,000 cache-read tokens and the $0.015 the journal
/// itself records never reached any total (research PoC: 150 tokens,
/// $0.00 reported).
const USAGE_CACHE_WARM: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50, "cacheRead": 0, "cacheWrite": 0}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "usage", "id": "31", "parentId": "21", "timestamp": "2026-10-04T10:00:05.000Z", "kind": "cache_warm", "provider": "anthropic", "model": "claude-sonnet-4-5", "usage": {"input": 0, "output": 0, "cacheRead": 50000, "cacheWrite": 0, "totalTokens": 50000, "cost": {"input": 0, "output": 0, "cacheRead": 0.015, "cacheWrite": 0, "total": 0.015}}}"#,
);

/// Same journal with the header's `cwd` removed: the header is still
/// detected (a `version` key suffices) but there is no meta event with
/// a cwd to carry the disclosure counters. Before the carrier fix this
/// shape panicked inside the parser (`expect` on a lookup that can
/// never hit) instead of parsing.
const USAGE_CACHE_WARM_NO_CWD: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50, "cacheRead": 0, "cacheWrite": 0}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "usage", "id": "31", "parentId": "21", "timestamp": "2026-10-04T10:00:05.000Z", "kind": "cache_warm", "provider": "anthropic", "model": "claude-sonnet-4-5", "usage": {"input": 0, "output": 0, "cacheRead": 50000, "cacheWrite": 0, "totalTokens": 50000, "cost": {"input": 0, "output": 0, "cacheRead": 0.015, "cacheWrite": 0, "total": 0.015}}}"#,
);

/// A usage entry of a kind this build does not specifically know, with
/// NO cost block: rm-436's rule is count-the-tokens-under-the-kind —
/// unknown kinds must not regress to the old silent drop.
const USAGE_UNKNOWN_KIND: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "usage", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "kind": "hypothetical_kind", "provider": "anthropic", "model": "claude-sonnet-4-5", "usage": {"input": 10, "output": 5, "cacheRead": 0, "cacheWrite": 0}}"#,
);

/// A pi v3 tree journal with sibling branches: entries 21 and 22 both
/// parent to 11. Both branches are counted (first cut); the shape is
/// disclosed as `pi_branches`.
const BRANCH_SIBLINGS: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "attempt A"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "message", "id": "22", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "attempt B"}], "timestamp": 1733234401000, "usage": {"input": 200, "output": 60}, "model": "claude-sonnet-4-5"}}"#,
);

/// A pi v1-style journal: no entry ids (only the header carries
/// one, as the parser requires), so no tree shape exists to
/// disclose.
const LINEAR_V1: &str = concat!(
    r#"{"type": "session", "version": 1, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50}, "model": "claude-sonnet-4-5"}}"#,
);

/// The pi wire key for model switches is `modelId` (dist writer
/// `appendModelChange(provider, modelId)`); the parser previously read
/// `model`, so the switch was dead and the post-switch usage was
/// priced (and attributed) as the pre-switch model.
const MODEL_CHANGE: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "on claude"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50, "cacheRead": 0, "cacheWrite": 0}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "model_change", "id": "31", "parentId": "21", "timestamp": "2026-10-04T10:00:03.000Z", "provider": "openai", "modelId": "gpt-4o"}"#,
    "\n",
    r#"{"type": "message", "id": "41", "parentId": "31", "timestamp": "2026-10-04T10:00:04.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "on gpt"}], "timestamp": 1733234401000, "usage": {"input": 300, "output": 70}}}"#,
);

/// The legacy `model` spelling of a model change: shipped by older pi
/// writers and still honored, so the switch must keep working.
const MODEL_CHANGE_LEGACY_KEY: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "on claude"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50, "cacheRead": 0, "cacheWrite": 0}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "model_change", "id": "31", "parentId": "21", "timestamp": "2026-10-04T10:00:03.000Z", "provider": "openai", "model": "legacy-model"}"#,
    "\n",
    r#"{"type": "message", "id": "41", "parentId": "31", "timestamp": "2026-10-04T10:00:04.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "on gpt"}], "timestamp": 1733234401000, "usage": {"input": 300, "output": 70}}}"#,
);

/// Entry types and message roles with no accounting arm stay visible
/// as counters instead of vanishing (rm-436's disclosure family).
const SKIPPED_AND_UNKNOWN_ROLES: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "label", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "label": "fix-the-bug"}"#,
    "\n",
    r#"{"type": "thinking_level_change", "id": "12", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "level": "high"}"#,
    "\n",
    r#"{"type": "message", "id": "13", "parentId": "12", "timestamp": "2026-10-04T10:00:03.000Z", "message": {"role": "system", "content": "banner", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "14", "parentId": "13", "timestamp": "2026-10-04T10:00:04.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "15", "parentId": "14", "timestamp": "2026-10-04T10:00:05.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}], "timestamp": 1733234401000, "usage": {"input": 10, "output": 5}, "model": "claude-sonnet-4-5"}}"#,
);

/// Scratch dir per call (atomic sequence, not `line!()`, which is
/// macro-site invariant): the doctor test scans its whole dir, so a
/// shared dir would fold other tests' fixtures into its aggregate.
fn scratch_dir() -> PathBuf {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("at-pi-accounting-{}-{seq}", std::process::id()));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn journal(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).expect("write fixture");
    path
}

fn parsed(name: &str, body: &str) -> Session {
    let dir = scratch_dir();
    let path = journal(&dir, name, body);
    parse_file(&path).unwrap_or_else(|error| panic!("{name} must parse as pi journal: {error}"))
}

#[test]
fn usage_entries_count_cache_warm_and_recorded_cost() {
    // rm-436: `type:"usage"` entries contribute to token and cost
    // totals (spec) instead of dying in the catch-all. The $0.015 the
    // journal records for the cache-warm block is passed through as
    // recorded cost and the block's cache-read tokens are NOT charged
    // again by the catalog.
    let session = parsed("usage-cache-warm.jsonl", USAGE_CACHE_WARM);
    let metrics = &session.metrics;
    assert_eq!(metrics.tokens_input, 100, "message input tokens");
    assert_eq!(metrics.tokens_output, 50, "message output tokens");
    assert_eq!(
        metrics.tokens_cache_r, 50000,
        "cache_warm cache-read tokens"
    );
    assert_eq!(metrics.tokens_cache_w, 0);
    assert_eq!(
        metrics.disclosure_counters.get("pi_usage_entry:cache_warm"),
        Some(&1),
        "one cache_warm entry, disclosed under its kind"
    );
    assert!(
        (metrics.upstream_cost_usd - 0.015).abs() < 1e-12,
        "journal-recorded cost passes through, got {}",
        metrics.upstream_cost_usd
    );
    let price = lookup_price("claude-sonnet-4-5");
    let expected = round4(100.0 / 1e6 * price.input + 50.0 / 1e6 * price.output + 0.015);
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "catalog prices the unrecorded block, recorded cost rides along: got {} want {}",
        metrics.cost_estimated,
        expected
    );
    assert_eq!(metrics.provenance.tokens, "reported_by_agent");
    assert!(
        metrics
            .provenance
            .pricing_source
            .ends_with(" + recorded cost"),
        "pricing source discloses the recorded component: {}",
        metrics.provenance.pricing_source
    );
    assert_eq!(
        metrics.provenance.cost,
        "calculated_from_tokens_with_recorded_cost"
    );
    assert_eq!(metrics.assistant_turns, 1);
    assert_eq!(metrics.user_messages, 1);
}

#[test]
fn unknown_usage_kinds_count_their_tokens_under_their_own_kind() {
    // rm-436 rule: a usage kind this build does not specifically know
    // still counts — under its own disclosed kind. No cost block means
    // no recorded-cost passthrough; the catalog prices the tokens.
    let session = parsed("usage-unknown-kind.jsonl", USAGE_UNKNOWN_KIND);
    let metrics = &session.metrics;
    assert_eq!(metrics.tokens_input, 10);
    assert_eq!(metrics.tokens_output, 5);
    assert_eq!(
        metrics
            .disclosure_counters
            .get("pi_usage_entry:hypothetical_kind"),
        Some(&1)
    );
    assert_eq!(metrics.upstream_cost_usd, 0.0);
    let price = lookup_price("claude-sonnet-4-5");
    let expected = round4(10.0 / 1e6 * price.input + 5.0 / 1e6 * price.output);
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "got {} want {}",
        metrics.cost_estimated,
        expected
    );
}

#[test]
fn sibling_branches_disclose_two_ends_without_counting_the_header() {
    // rm-437 first cut: entries 21 and 22 both parent to 11, so the
    // journal has TWO branch ends among body entries. The session
    // header's own id (s-1) is a root, never a branch end — the
    // pre-fix code counted it and reported 3.
    let session = parsed("branch-siblings.jsonl", BRANCH_SIBLINGS);
    let metrics = &session.metrics;
    assert_eq!(
        metrics.disclosure_counters.get("pi_branches"),
        Some(&2),
        "two branch ends disclosed, header not counted: {:?}",
        metrics.disclosure_counters
    );
    // Both branches are still counted in the totals (disclosed, not
    // silently deduplicated — active-branch replay is a later cut).
    assert_eq!(metrics.tokens_input, 300);
    assert_eq!(metrics.tokens_output, 110);
    assert_eq!(metrics.assistant_turns, 2);
}

#[test]
fn linear_journals_have_no_branch_disclosure() {
    // Linear v2/v3 chains and id-less v1 journals have exactly one
    // leaf, so `pi_branches` must not appear at all.
    let linear = parsed("linear-v3.jsonl", USAGE_CACHE_WARM);
    assert!(
        !linear
            .metrics
            .disclosure_counters
            .contains_key("pi_branches"),
        "{:?}",
        linear.metrics.disclosure_counters
    );
    let v1 = parsed("linear-v1.jsonl", LINEAR_V1);
    assert!(
        !v1.metrics.disclosure_counters.contains_key("pi_branches"),
        "{:?}",
        v1.metrics.disclosure_counters
    );
    assert_eq!(v1.metrics.tokens_input, 100);
}

#[test]
fn model_change_wire_key_reattributes_and_reprices_per_block() {
    // rm-438: the wire key is `modelId`; with the handler live, the
    // post-switch block is attributed to gpt-4o and priced with
    // gpt-4o's catalog price instead of silently billing 370 tokens to
    // the pre-switch claude model.
    let session = parsed("model-change.jsonl", MODEL_CHANGE);
    let metrics = &session.metrics;
    assert_eq!(metrics.tokens_input, 400, "100 claude + 300 gpt-4o");
    assert_eq!(metrics.tokens_output, 120, "50 claude + 70 gpt-4o");
    assert_eq!(metrics.assistant_turns, 2);
    let claude = lookup_price("claude-sonnet-4-5");
    let gpt = lookup_price("gpt-4o");
    let expected = round4(
        100.0 / 1e6 * claude.input
            + 50.0 / 1e6 * claude.output
            + 300.0 / 1e6 * gpt.input
            + 70.0 / 1e6 * gpt.output,
    );
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "per-block pricing: got {} want {}",
        metrics.cost_estimated,
        expected
    );
    // The session row discloses the mix instead of naming one model.
    assert_eq!(metrics.model_used, "multiple");
    assert_eq!(
        metrics.provenance.pricing_source,
        "multiple models (priced per usage block)"
    );
    assert_eq!(metrics.provenance.cost, "calculated_per_message_tokens");
}

#[test]
fn legacy_model_key_still_switches_the_tracked_model() {
    // rm-438 must not regress the legacy `model` spelling: the switch
    // applies, the post-switch block is priced as legacy-model, and
    // the session discloses the model mix.
    let session = parsed("model-change-legacy.jsonl", MODEL_CHANGE_LEGACY_KEY);
    let metrics = &session.metrics;
    assert_eq!(metrics.model_used, "multiple");
    let claude = lookup_price("claude-sonnet-4-5");
    let legacy = lookup_price("legacy-model");
    let expected = round4(
        100.0 / 1e6 * claude.input
            + 50.0 / 1e6 * claude.output
            + 300.0 / 1e6 * legacy.input
            + 70.0 / 1e6 * legacy.output,
    );
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "got {} want {}",
        metrics.cost_estimated,
        expected
    );
}

#[test]
fn header_without_cwd_does_not_panic_and_still_discloses() {
    // Regression for the carrier fix: a header without `cwd` has no
    // meta event for the disclosure counters to ride. The parser must
    // synthesize one carrier instead of panicking, and the disclosures
    // and usage totals must survive.
    let session = parsed("usage-no-cwd.jsonl", USAGE_CACHE_WARM_NO_CWD);
    let metrics = &session.metrics;
    assert_eq!(metrics.tokens_cache_r, 50000);
    assert!(
        (metrics.upstream_cost_usd - 0.015).abs() < 1e-12,
        "got {}",
        metrics.upstream_cost_usd
    );
    assert_eq!(
        metrics.disclosure_counters.get("pi_usage_entry:cache_warm"),
        Some(&1),
        "counters ride the synthetic carrier: {:?}",
        metrics.disclosure_counters
    );
}

#[test]
fn skipped_entry_types_and_unhandled_message_roles_disclose() {
    // rm-436's counter family: entry types with no accounting arm
    // (`label`, `thinking_level_change`) and message roles without an
    // arm (`system`) surface as counters instead of silently
    // vanishing.
    let session = parsed("skipped-and-roles.jsonl", SKIPPED_AND_UNKNOWN_ROLES);
    let metrics = &session.metrics;
    assert_eq!(
        metrics.disclosure_counters.get("pi_entry_skipped:label"),
        Some(&1),
        "{:?}",
        metrics.disclosure_counters
    );
    assert_eq!(
        metrics
            .disclosure_counters
            .get("pi_entry_skipped:thinking_level_change"),
        Some(&1),
        "{:?}",
        metrics.disclosure_counters
    );
    assert_eq!(
        metrics.disclosure_counters.get("pi_message_role:system"),
        Some(&1),
        "{:?}",
        metrics.disclosure_counters
    );
    assert_eq!(metrics.user_messages, 1);
    assert_eq!(metrics.assistant_turns, 1);
}

#[test]
fn data_health_and_doctor_aggregate_disclosures() {
    // The acceptance surface: per-session counters aggregate once per
    // corpus in `data_health` (JSON + report rows) and in `--doctor`,
    // so the facts are visible without per-session digging.
    let dir = scratch_dir();
    journal(&dir, "usage-cache-warm.jsonl", USAGE_CACHE_WARM);
    journal(&dir, "branch-siblings.jsonl", BRANCH_SIBLINGS);
    let first = parse_file(&dir.join("usage-cache-warm.jsonl")).expect("parse first");
    let second = parse_file(&dir.join("branch-siblings.jsonl")).expect("parse second");
    let sessions = [first, second];
    let health = data_health(&sessions, sessions.len(), 0);
    assert_eq!(
        health.disclosures.get("pi_usage_entry:cache_warm"),
        Some(&1),
        "{:?}",
        health.disclosures
    );
    assert_eq!(
        health.disclosures.get("pi_branches"),
        Some(&2),
        "{:?}",
        health.disclosures
    );

    let report = build_doctor_report(Some(&dir), false);
    assert_eq!(
        report.disclosures.get("pi_branches"),
        Some(&2),
        "doctor aggregates the same counters: {:?}",
        report.disclosures
    );
    assert_eq!(
        report.disclosures.get("pi_usage_entry:cache_warm"),
        Some(&1)
    );
    let text = render_doctor_report(Some(&dir), false, "text").expect("doctor text");
    assert!(
        text.contains("Journal disclosures:")
            && text.contains("pi_branches=2")
            && text.contains("pi_usage_entry:cache_warm=1"),
        "text render shows the disclosures:\n{text}"
    );
    let json = render_doctor_report(Some(&dir), false, "json").expect("doctor json");
    assert!(
        json.contains("\"disclosures\""),
        "doctor JSON carries the map (skip-if-empty keeps clean corpora unchanged):\n{json}"
    );
}

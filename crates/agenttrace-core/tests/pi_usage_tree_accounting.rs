//! rm-436 / rm-437 / rm-438 — pi-family journal accounting correctness.
//!
//! Fixtures are schema-faithful to the public versioned session-format
//! spec (packages/coding-agent/docs/session-format.md, pi 1.0.2) and to
//! the writer's wire keys confirmed in the @earendil-works/pi-coding-agent
//! 1.0.2 dist: `appendUsage(kind, provider, model, usage)` emits
//! `type:"usage"` entries whose model rides `model` (the usage-entry
//! key — distinct from `model_change`'s `modelId` below; the spec's
//! own usage example agrees) and whose spend "contributes to session
//! token and cost totals" per the spec, and
//! `appendModelChange(provider, modelId)` emits `type:"model_change"`
//! with the same `modelId` key — `cache-warmer.js` emits
//! `kind:"cache_warm"`. They are synthetic, not recordings of a real
//! host. Expected costs are computed through the same public pricing
//! lookup the crate uses, so catalog drift cannot rot these tests.

use std::fs;
use std::path::{Path, PathBuf};

use agenttrace_core::{
    build_doctor_report, compute_overview, data_health, lookup_price, parse_file,
    render_doctor_report, report_overview_markdown_with_context, round4, Session, TimeRange,
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

/// A forked pi v3 journal with branch ends at MIXED depths (review
/// fix F8): 31 ends the 11→21 subtree while 32 and 33 both end under
/// 22 — the rm-437 acceptance promised a forked-session shape, not
/// only same-parent siblings.
const BRANCH_FORK: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "depth one"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "message", "id": "22", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "sibling"}], "timestamp": 1733234401000, "usage": {"input": 200, "output": 60}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "message", "id": "31", "parentId": "21", "timestamp": "2026-10-04T10:00:03.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "end under 21"}], "timestamp": 1733234401000, "usage": {"input": 10, "output": 5}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "message", "id": "32", "parentId": "22", "timestamp": "2026-10-04T10:00:04.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "end under 22 a"}], "timestamp": 1733234401000, "usage": {"input": 20, "output": 6}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "message", "id": "33", "parentId": "22", "timestamp": "2026-10-04T10:00:05.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "end under 22 b"}], "timestamp": 1733234401000, "usage": {"input": 30, "output": 7}, "model": "claude-sonnet-4-5"}}"#,
);

/// A generic (non-pi) JSONL session whose usage-bearing events carry
/// per-event model attribution (review fix F2): per-block multi-model
/// pricing is parser-agnostic, so this shape must price each block at
/// its own model's catalog price — not bill every token at the
/// session's last-seen model the way the pre-batch code did.
const GENERIC_TWO_MODELS: &str = concat!(
    r#"{"role":"meta","timestamp":"2024-12-03T14:00:01Z","cwd":"/tmp/x","ModelUsed":"claude-sonnet-4-5","Usage":{"input_tokens":1000,"output_tokens":100}}"#,
    "\n",
    r#"{"role":"meta","timestamp":"2024-12-03T14:00:02Z","cwd":"/tmp/x","ModelUsed":"gpt-4o","Usage":{"input_tokens":2000,"output_tokens":200}}"#,
    "\n",
);

/// A generic journal line trying to inject pi-parser-set fields
/// (review fix F5): `RecordedCostUSD` and `DisclosureCounters` are
/// never deserialized from foreign journals, so a minted $999 cost
/// and a fabricated `pi_branches` must both be ignored — the block
/// prices from the catalog and no disclosure appears.
const GENERIC_INJECTION: &str = concat!(
    r#"{"role":"meta","timestamp":"2024-12-03T14:00:01Z","cwd":"/tmp/x","ModelUsed":"claude-sonnet-4-5","Usage":{"input_tokens":1000,"output_tokens":100},"RecordedCostUSD":999,"DisclosureCounters":{"pi_branches":99}}"#,
    "\n",
);

/// rm-438's model-change journal plus a recorded-cost usage entry
/// (review fix F3): a mixed-model session whose spend is partly
/// journal-recorded must keep the " + recorded cost" hint on its
/// pricing source, exactly like the single-model row.
const MODEL_CHANGE_RECORDED: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "on claude"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50, "cacheRead": 0, "cacheWrite": 0}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "model_change", "id": "31", "parentId": "21", "timestamp": "2026-10-04T10:00:03.000Z", "provider": "openai", "modelId": "gpt-4o"}"#,
    "\n",
    r#"{"type": "message", "id": "41", "parentId": "31", "timestamp": "2026-10-04T10:00:04.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "on gpt"}], "timestamp": 1733234401000, "usage": {"input": 300, "output": 70}}}"#,
    "\n",
    r#"{"type": "usage", "id": "51", "parentId": "41", "timestamp": "2026-10-04T10:00:05.000Z", "kind": "cache_warm", "provider": "openai", "model": "gpt-4o", "usage": {"input": 0, "output": 0, "cacheRead": 1000, "cacheWrite": 0, "totalTokens": 1000, "cost": {"input": 0, "output": 0, "cacheRead": 0.02, "cacheWrite": 0, "total": 0.02}}}"#,
);

/// Hostile disclosure strings (review 06e542d5 F-A): usage `kind`,
/// entry `type`, and message `role` are journal-authored and embed
/// directly into counter keys that reach --doctor/--overview text and
/// markdown. The fixtures carry an OSC-52 clipboard-write sequence
/// (ESC ]52;c;… BEL — proven live pre-fix to reach stdout
/// byte-exact), a bare ESC, a `\n` (proven to inject a second doctor
/// line), and a typeless line probing the empty-name guard.
const HOSTILE_DISCLOSURES: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "12", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "sys\u001b]52;c;aGVsbG8=\u0007tem", "content": "banner", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "la\u001bbel", "id": "13", "parentId": "12", "timestamp": "2026-10-04T10:00:03.000Z", "label": "x"}"#,
    "\n",
    r#"{"no_type": true, "id": "14", "parentId": "13", "timestamp": "2026-10-04T10:00:04.000Z"}"#,
    "\n",
    r#"{"type": "usage", "id": "15", "parentId": "14", "timestamp": "2026-10-04T10:00:05.000Z", "kind": "k\u001b]52;c;aGVsbG8=\u0007end", "provider": "anthropic", "model": "claude-sonnet-4-5", "usage": {"input": 10, "output": 5, "cacheRead": 0, "cacheWrite": 0}}"#,
    "\n",
    r#"{"type": "usage", "id": "16", "parentId": "15", "timestamp": "2026-10-04T10:00:06.000Z", "kind": "bad\nkind", "provider": "anthropic", "model": "claude-sonnet-4-5", "usage": {"input": 1, "output": 1}}"#,
    "\n",
);

/// An all-zero-token usage entry whose only signal is its recorded
/// cost (review 06e542d5 F-C, proven live pre-fix at $0.0011 where
/// the honest total was $0.0161): a PRESENT cost must not be silently
/// zeroed for lacking token classes — the inverse of the batch's own
/// "a missing or corrupt cost must not silently zero real spend".
const ZERO_TOKEN_RECORDED_COST: &str = concat!(
    r#"{"type": "session", "version": 3, "id": "s-1", "timestamp": "2026-10-04T10:00:00.000Z", "cwd": "/tmp/x"}"#,
    "\n",
    r#"{"type": "message", "id": "11", "parentId": null, "timestamp": "2026-10-04T10:00:01.000Z", "message": {"role": "user", "content": "hi", "timestamp": 1733234401000}}"#,
    "\n",
    r#"{"type": "message", "id": "21", "parentId": "11", "timestamp": "2026-10-04T10:00:02.000Z", "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}], "timestamp": 1733234401000, "usage": {"input": 100, "output": 50, "cacheRead": 0, "cacheWrite": 0}, "model": "claude-sonnet-4-5"}}"#,
    "\n",
    r#"{"type": "usage", "id": "31", "parentId": "21", "timestamp": "2026-10-04T10:00:05.000Z", "kind": "cache_warm", "provider": "anthropic", "model": "claude-sonnet-4-5", "usage": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 0, "cost": {"input": 0, "output": 0, "cacheRead": 0.015, "cacheWrite": 0, "total": 0.015}}}"#,
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

#[test]
fn forked_subtrees_disclose_all_ends() {
    // rm-437 (review fix F8): branch ends at MIXED depths — 31 ends
    // the 11→21 subtree, 32 and 33 both end under 22 — so the journal
    // has three leaves. Referenced parents (11, 21, 22) and the header
    // root never count; spend stays all-branches (first cut).
    let session = parsed("branch-fork.jsonl", BRANCH_FORK);
    let metrics = &session.metrics;
    assert_eq!(
        metrics.disclosure_counters.get("pi_branches"),
        Some(&3),
        "three leaves across two levels: {:?}",
        metrics.disclosure_counters
    );
    assert_eq!(metrics.tokens_input, 360);
    assert_eq!(metrics.tokens_output, 128);
    assert_eq!(metrics.assistant_turns, 5);
}

#[test]
fn multi_model_pricing_is_parser_agnostic_generic_jsonl() {
    // Review fix F2: per-block multi-model pricing is not pi-only —
    // any parser whose usage-bearing events carry model attribution
    // (here the generic JSONL arm) prices each block at its own
    // model's catalog price and reports the model mix, instead of
    // billing every token to the session's last-seen model.
    let session = parsed("generic-two-models.jsonl", GENERIC_TWO_MODELS);
    let metrics = &session.metrics;
    assert_eq!(metrics.model_used, "multiple");
    assert_eq!(
        metrics.provenance.pricing_source,
        "multiple models (priced per usage block)"
    );
    let claude = lookup_price("claude-sonnet-4-5");
    let gpt = lookup_price("gpt-4o");
    let expected = round4(
        1000.0 / 1e6 * claude.input
            + 100.0 / 1e6 * claude.output
            + 2000.0 / 1e6 * gpt.input
            + 200.0 / 1e6 * gpt.output,
    );
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "got {} want {}",
        metrics.cost_estimated,
        expected
    );
}

#[test]
fn foreign_journal_cost_and_counter_injection_is_ignored() {
    // Review fix F5: `RecordedCostUSD` and `DisclosureCounters` are
    // set by the pi parser only and are serde `skip_deserializing` on
    // `Event`, so a foreign JSONL line cannot mint $999 of cost or a
    // fabricated `pi_branches` disclosure — the review proved both
    // injections live on the pre-fix binary.
    let session = parsed("generic-injection.jsonl", GENERIC_INJECTION);
    let metrics = &session.metrics;
    assert_eq!(
        metrics.upstream_cost_usd, 0.0,
        "foreign recorded cost must not pass through"
    );
    assert!(
        metrics.disclosure_counters.is_empty(),
        "foreign counters must not appear: {:?}",
        metrics.disclosure_counters
    );
    let price = lookup_price("claude-sonnet-4-5");
    let expected = round4(1000.0 / 1e6 * price.input + 100.0 / 1e6 * price.output);
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "catalog prices the block: got {} want {}",
        metrics.cost_estimated,
        expected
    );
}

#[test]
fn model_change_with_recorded_cost_keeps_provenance_hint() {
    // Review fix F3: the mixed-model pricing source keeps the
    // " + recorded cost" suffix when journal-recorded spend
    // contributed — the single-model row always had the hint, and the
    // multi-model cost seeds from the same recorded total.
    let session = parsed("model-change-recorded.jsonl", MODEL_CHANGE_RECORDED);
    let metrics = &session.metrics;
    assert_eq!(metrics.model_used, "multiple");
    assert_eq!(
        metrics.provenance.pricing_source,
        "multiple models (priced per usage block) + recorded cost"
    );
    assert_eq!(metrics.tokens_cache_r, 1000);
    assert_eq!(
        metrics.disclosure_counters.get("pi_usage_entry:cache_warm"),
        Some(&1)
    );
    let claude = lookup_price("claude-sonnet-4-5");
    let gpt = lookup_price("gpt-4o");
    let expected = round4(
        100.0 / 1e6 * claude.input
            + 50.0 / 1e6 * claude.output
            + 300.0 / 1e6 * gpt.input
            + 70.0 / 1e6 * gpt.output
            + 0.02,
    );
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "per-block catalog + recorded $0.02: got {} want {}",
        metrics.cost_estimated,
        expected
    );
}

#[test]
fn hostile_disclosure_kinds_and_roles_are_sanitized_at_mint() {
    // Review 06e542d5 F-A: counter keys embed journal-authored strings
    // and reach every report surface. The sanitizer's contract is
    // control-bytes-only (printable CSI/OSC tails legitimately
    // survive), so the pin targets the control bytes: no C0/C1/DEL in
    // any key, and the doctor text line stays ONE line. Proven live
    // pre-fix: the raw ESC/BEL pair reached stdout byte-exact in
    // --doctor and --overview (an OSC-52 clipboard-write sequence),
    // and a `\n` kind injected a second line.
    let session = parsed("hostile-disclosures.jsonl", HOSTILE_DISCLOSURES);
    let metrics = &session.metrics;
    assert!(
        !metrics
            .disclosure_counters
            .keys()
            .any(|key| key.chars().any(char::is_control)),
        "counter keys carry no control bytes: {:?}",
        metrics.disclosure_counters
    );
    let osc_kind = metrics
        .disclosure_counters
        .keys()
        .find(|key| key.starts_with("pi_usage_entry:k"))
        .expect("the OSC-bearing kind counter exists");
    assert!(
        osc_kind.contains('\u{FFFD}'),
        "ESC/BEL became U+FFFD: {osc_kind:?}"
    );
    assert!(
        osc_kind.contains("]52;c;aGVsbG8="),
        "printable OSC tail survives per the sanitizer contract: {osc_kind:?}"
    );
    assert!(
        metrics
            .disclosure_counters
            .contains_key("pi_entry_skipped:la\u{FFFD}bel"),
        "hostile entry type sanitized into its counter: {:?}",
        metrics.disclosure_counters
    );
    assert!(
        metrics
            .disclosure_counters
            .keys()
            .any(|key| key.starts_with("pi_message_role:sys\u{FFFD}")),
        "hostile message role sanitized into its counter: {:?}",
        metrics.disclosure_counters
    );
    assert!(
        !metrics
            .disclosure_counters
            .contains_key("pi_entry_skipped:"),
        "a typeless line mints no empty-name counter: {:?}",
        metrics.disclosure_counters
    );

    // Terminal-facing surfaces render the sanitized keys on one line
    // with no raw control bytes anywhere in the report.
    let dir = scratch_dir();
    journal(&dir, "hostile-disclosures.jsonl", HOSTILE_DISCLOSURES);
    let text = render_doctor_report(Some(&dir), false, "text").expect("doctor text");
    let disclosure_lines: Vec<&str> = text
        .lines()
        .filter(|line| {
            line.contains("pi_usage_entry:")
                || line.contains("pi_entry_skipped:")
                || line.contains("pi_message_role:")
        })
        .collect();
    assert_eq!(
        disclosure_lines.len(),
        1,
        "one-line contract (newline kinds cannot inject lines):\n{text}"
    );
    assert!(
        !text.contains('\u{1b}') && !text.contains('\u{7}'),
        "no raw ESC/BEL reaches the terminal:\n{text}"
    );

    let session = parse_file(&dir.join("hostile-disclosures.jsonl")).expect("re-parse");
    let sessions = [session];
    let health = data_health(&sessions, sessions.len(), 0);
    assert_eq!(
        health.disclosures.keys().count(),
        metrics.disclosure_counters.len(),
        "data_health carries the same sanitized keys"
    );
    let markdown = report_overview_markdown_with_context(
        &compute_overview(&sessions),
        &sessions,
        &health,
        TimeRange::All,
        false,
    );
    let markdown_rows: Vec<&str> = markdown
        .lines()
        .filter(|line| line.contains("pi_usage_entry:") || line.contains("pi_message_role:"))
        .collect();
    assert_eq!(
        markdown_rows.len(),
        1,
        "markdown one-row contract:\n{markdown}"
    );
    assert!(
        !markdown_rows[0].chars().any(char::is_control),
        "no control bytes in the markdown row"
    );
}

#[test]
fn zero_token_usage_entry_still_passes_recorded_cost() {
    // Review 06e542d5 F-C: the meta event is emitted whenever EITHER
    // token classes or a recorded cost exist, and analyze() honors the
    // recorded cost outside the non-empty-usage gate — an all-zero
    // usage block with $0.015 recorded spend must land in the session
    // cost (pre-fix it was silently dropped: $0.0011 vs $0.0161
    // honest).
    let session = parsed("zero-token-recorded.jsonl", ZERO_TOKEN_RECORDED_COST);
    let metrics = &session.metrics;
    assert_eq!(metrics.tokens_input, 100, "message tokens counted");
    assert_eq!(metrics.tokens_output, 50);
    assert_eq!(
        metrics.tokens_cache_r, 0,
        "the entry claims no token classes"
    );
    assert!(
        (metrics.upstream_cost_usd - 0.015).abs() < 1e-12,
        "recorded cost passes through despite zero tokens: got {}",
        metrics.upstream_cost_usd
    );
    let price = lookup_price("claude-sonnet-4-5");
    let expected = round4(100.0 / 1e6 * price.input + 50.0 / 1e6 * price.output + 0.015);
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "catalog + recorded cost: got {} want {}",
        metrics.cost_estimated,
        expected
    );
    assert!(
        metrics
            .provenance
            .pricing_source
            .ends_with(" + recorded cost"),
        "provenance keeps the recorded-cost hint: {}",
        metrics.provenance.pricing_source
    );
    assert_eq!(
        metrics.provenance.cost,
        "calculated_from_tokens_with_recorded_cost"
    );
    assert_eq!(
        metrics.disclosure_counters.get("pi_usage_entry:cache_warm"),
        Some(&1),
        "the entry is still disclosed: {:?}",
        metrics.disclosure_counters
    );
}

/// rm-551 — the assess PoC shape: one exchange plus one compaction
/// call whose inline usage block was the whole-context call's real
/// spend (38000 in / 900 out / $0.0123 upstream-recorded). Schema-
/// faithful to pi dist `appendCompaction` (type, summary,
/// tokensBefore, systemMessage, firstKeptEntryId, usage).
const COMPACTION_SENTINEL: &str = concat!(
    "{\"id\":\"s1\",\"timestamp\":\"2026-01-04T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp/a\"}\n",
    "{\"id\":\"a1\",\"parentId\":\"s1\",\"type\":\"message\",\"timestamp\":\"2026-01-04T10:00:01.000Z\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"continue\"}]}}\n",
    "{\"id\":\"a2\",\"parentId\":\"a1\",\"type\":\"message\",\"timestamp\":\"2026-01-04T10:00:02.000Z\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"okay\"}],\"timestamp\":1766625901000,\"usage\":{\"input\":500,\"output\":1000,\"cacheRead\":25000,\"cacheWrite\":5000},\"model\":\"claude-sonnet-4-5\"}}\n",
    "{\"id\":\"a3\",\"parentId\":\"a2\",\"type\":\"compaction\",\"timestamp\":\"2026-01-04T10:00:03.000Z\",\"summary\":\"compacted the context\",\"tokensBefore\":123,\"systemMessage\":\"sys\",\"firstKeptEntryId\":\"a2\",\"usage\":{\"input\":38000,\"output\":900,\"cacheRead\":0,\"cacheWrite\":0,\"cost\":{\"input\":0.0123,\"output\":0,\"cacheRead\":0,\"cacheWrite\":0,\"total\":0.0123}}}\n"
);

/// rm-551 — branch_summary entries fold through the same arm; this one
/// carries usage with NO recorded cost, so the catalog must price its
/// tokens (nothing is upstream-priced to exclude).
const BRANCH_SUMMARY_USAGE: &str = concat!(
    "{\"id\":\"s1\",\"timestamp\":\"2026-01-04T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp/b\"}\n",
    "{\"id\":\"a1\",\"parentId\":\"s1\",\"type\":\"branch_summary\",\"timestamp\":\"2026-01-04T10:00:01.000Z\",\"summary\":\"branched from s1\",\"usage\":{\"input\":700,\"output\":80,\"cacheRead\":0,\"cacheWrite\":0}}\n"
);

/// rm-551 — a compaction entry with an empty usage block must fold
/// NOTHING (no counter, no meta event) while the summary still
/// lands as a turn: the arm's fold is conditional, not unconditional.
const COMPACTION_EMPTY_USAGE: &str = concat!(
    "{\"id\":\"s1\",\"timestamp\":\"2026-01-04T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp/c\"}\n",
    "{\"id\":\"a1\",\"parentId\":\"s1\",\"type\":\"compaction\",\"timestamp\":\"2026-01-04T10:00:01.000Z\",\"summary\":\"compacted with no usage\",\"tokensBefore\":50,\"usage\":{}}\n"
);

#[test]
fn compaction_usage_composes_with_catalog() {
    // rm-551: the compaction call's inline usage block folds into the
    // session exactly like the rm-436 standalone usage arm — tokens
    // counted, upstream-recorded cost riding along, the block's own
    // tokens excluded from the catalog estimate (upstream-priced) so
    // nothing is double-charged. Pre-fix this journal reported
    // cost_estimated ~= the catalog of the exchange alone (the assess
    // PoC: $0.0023 reported vs ~$0.0143 spec-true).
    let session = parsed("compaction-sentinel.jsonl", COMPACTION_SENTINEL);
    let metrics = &session.metrics;
    assert_eq!(
        metrics.tokens_input,
        500 + 38000,
        "exchange + compaction input"
    );
    assert_eq!(metrics.tokens_output, 1000 + 900);
    assert_eq!(metrics.tokens_cache_r, 25000);
    assert_eq!(metrics.tokens_cache_w, 5000);
    // The upstream-priced exclusion is asserted through the catalog
    // formula itself: expected prices ONLY the exchange's 500/1000/
    // 25000/5000 — the 38000/900 block is carried at its recorded
    // $0.0123 instead of being estimated (or double-charged).
    assert!((metrics.upstream_cost_usd - 0.0123).abs() < 1e-12);
    let price = lookup_price("claude-sonnet-4-5");
    let expected = round4(
        price.input / 1e6 * 500.0
            + price.output / 1e6 * 1000.0
            + price.cr / 1e6 * 25000.0
            + price.cw / 1e6 * 5000.0
            + 0.0123,
    );
    assert!(
        (metrics.cost_estimated - expected).abs() < 1e-9,
        "catalog(exchange) + recorded compaction cost: got {} want {}",
        metrics.cost_estimated,
        expected
    );
    assert_eq!(
        metrics
            .disclosure_counters
            .get("pi_compaction_usage_counted:compaction"),
        Some(&1),
        "the fold is disclosed per kind: {:?}",
        metrics.disclosure_counters
    );
    assert_eq!(
        metrics
            .disclosure_counters
            .get("pi_entry_skipped:compaction"),
        None,
        "handled types never take the skip channel"
    );
    assert_eq!(metrics.assistant_turns, 2, "exchange + compaction summary");
}

#[test]
fn branch_summary_usage_catalogs_when_no_recorded_cost() {
    // rm-551: branch_summary usage without a recorded cost is plain
    // counted spend — catalog-priced (nothing upstream-priced to
    // exclude) and disclosed under its own kind.
    let session = parsed("branch-summary-usage.jsonl", BRANCH_SUMMARY_USAGE);
    let metrics = &session.metrics;
    assert_eq!(metrics.tokens_input, 700);
    assert_eq!(metrics.tokens_output, 80);
    assert_eq!(metrics.upstream_cost_usd, 0.0);
    let price = lookup_price("claude-sonnet-4-5");
    let expected = round4(price.input / 1e6 * 700.0 + price.output / 1e6 * 80.0);
    assert!((metrics.cost_estimated - expected).abs() < 1e-9);
    assert_eq!(
        metrics
            .disclosure_counters
            .get("pi_compaction_usage_counted:branch_summary"),
        Some(&1)
    );
    assert_eq!(metrics.assistant_turns, 1, "the summary itself is a turn");
}

#[test]
fn compaction_without_usage_folds_nothing() {
    // rm-551 guard: an empty usage block must not mint a counter or a
    // zero-token meta event — the fold fires only when usage or a
    // recorded cost is actually present.
    let session = parsed("compaction-empty-usage.jsonl", COMPACTION_EMPTY_USAGE);
    let metrics = &session.metrics;
    // No usage block anywhere in this journal, so the pre-existing
    // absent-usage semantics apply to the summary turn: assistant
    // content takes the text-estimate fallback on tokens_output
    // (lib.rs estimate_tokens_from_text — 23-char summary / 4 = 5)
    // while tokens_input stays 0 (no user message). The fold's
    // absence must not mint a counter or a zero-token meta event.
    assert_eq!(
        metrics.tokens_input, 0,
        "no usage entry and no user message"
    );
    assert_eq!(
        metrics.tokens_output, 5,
        "assistant-content text-estimate, pre-existing absent-usage path"
    );
    assert_eq!(metrics.upstream_cost_usd, 0.0);
    assert!(!metrics
        .disclosure_counters
        .keys()
        .any(|k| k.starts_with("pi_compaction_usage_counted")));
    assert_eq!(metrics.assistant_turns, 1, "the summary still lands");
}

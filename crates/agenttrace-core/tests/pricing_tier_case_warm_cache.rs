//! rm-164 tier-case warm-cache regression (integration of run
//! 96287e100a56469babf0aae93fd0e9ed cycle 1 "pricing-drift svg-golden
//! red + installer unbounded", independent review, conflict case
//! b277d011cb77415eb0b83126f274404a): `lookup_price_tiered` now folds
//! `service_tier` to lowercase before the flex/ultrafast/priority
//! match, so a codex journal carrying `thread_settings.service_tier:
//! "Flex"` (config casing, recorded verbatim for disclosure) bills the
//! flex row — 1M in + 1M out on gpt-5.6 is $12, not the $24 the
//! standard rates charge — but `pricing_snapshot.json` is
//! byte-identical (the fold is pure code), so the rm-196
//! catalog-identity freshness gate keys an UNCHANGED digest and cannot
//! self-heal a warm store: without the 72 → 73 rung a v72 entry keeps
//! replaying the standard-rate `cost_estimated` for unchanged files
//! forever (the rm-485 class; the same shape as the rm-1511
//! effort-alias rung two rungs below).
//!
//! The rung must invalidate that store so the tier price regenerates.
//! This is the batch's warm-schema-72 pin of the sweep doc's oracle
//! family (`session-cache-schema-bump-sweeps-all-pinned-surfaces.md`).
//!
//! Own test binary on purpose (the rm-1522 precedent,
//! `claude_v2_1_300_warm_cache.rs`): cache paths load/save the session
//! cache in-process, so an env-touching warm-cache regression cannot
//! share a binary with anything else without a cache-file race — the
//! only test here touches `AGENTTRACE_SESSION_CACHE_DIR`.

use agenttrace_core::{
    cached_session, load_session_cache, lookup_price, lookup_price_tiered, parse_file,
    save_session_cache, session_cache_path, store_session, SESSION_CACHE_SCHEMA_VERSION,
};
use serde_json::Value;
use std::fs;

/// The recorded shape (mixed-case twin of the CLI lane's
/// `warm_cache_preserves_the_flex_service_tier_on_hits`): a codex
/// rollout whose turn_context discloses
/// `thread_settings.service_tier = "Flex"` and one token_count
/// snapshot of exactly 1M in / 1M out — $12 on the gpt-5.6 flex row,
/// $24 at the standard rates the pre-fold exact-match arm fell back to.
fn write_journal(root: &std::path::Path) -> std::path::PathBuf {
    let journal = root.join("flex-case.jsonl");
    fs::write(
        &journal,
        concat!(
            "{\"timestamp\":\"2026-10-01T09:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"s1\",\"cwd\":\"/tmp/x\",\"model\":\"gpt-5.6\"}}\n",
            "{\"timestamp\":\"2026-10-01T09:00:05Z\",\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-5.6\",\"cwd\":\"/tmp/x\",\"thread_settings\":{\"service_tier\":\"Flex\"}}}\n",
            "{\"timestamp\":\"2026-10-01T09:00:10Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"input_tokens\":1000000,\"output_tokens\":1000000}}}}\n"
        ),
    )
    .expect("journal write");
    journal
}

/// Cold prices at the folded tier row, warm replays it, and a v72
/// store (what a pre-rung build writes) is invalidated — never served —
/// so the standard-rate entry cannot survive the rung, with the discard
/// recorded in the `schema_invalidations` ledger, and the tier price
/// regenerates once on the next scan.
#[test]
fn warm_schema_72_store_is_invalidated_and_the_tier_price_regenerates() {
    let root = std::env::temp_dir().join(format!("rm164-warm72-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("tempdir");
    let journal = write_journal(&root);

    // The only env-touching test in this binary.
    let cache_dir = root.join("cache");
    fs::create_dir_all(&cache_dir).expect("cache dir");
    let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache_dir);

    // Bind the fixture to the catalog the way the CLI lane pins it: the
    // flex row bills $12 for 1M/1M on gpt-5.6 and the standard row
    // differs — that difference is the red-first property (a pre-fold
    // parse returns the standard cost, so any regression here flips the
    // cold assert, not just the warm one).
    let tier_cost = {
        let flex = lookup_price_tiered("gpt-5.6", Some("flex"));
        flex.input + flex.output
    };
    assert_eq!(
        tier_cost, 12.0,
        "the bundled gpt-5.6 flex row prices 1M/1M at $12 (CLI lane twin)"
    );
    let standard_cost = {
        let standard = lookup_price("gpt-5.6");
        standard.input + standard.output
    };
    assert_ne!(
        standard_cost, tier_cost,
        "the standard row must differ or the fold has nothing to reprice"
    );

    // Cold: the case-folded match resolves the flex row, and the entry
    // persists.
    let mut cache = load_session_cache();
    assert!(
        cached_session(&journal, &mut cache).is_none(),
        "cold: no entry yet"
    );
    let cold = parse_file(&journal).expect("journal parses");
    assert_eq!(
        cold.metrics.cost_estimated, tier_cost,
        "cold parse must price the mixed-case tier journal at the tier's row
        (pre-fold: the standard-rate silent reprice, {standard_cost})"
    );
    store_session(&journal, &cold, &mut cache).expect("store_session");
    save_session_cache(&mut cache).expect("save_session_cache");

    // Warm at the live schema: the served entry replays the tier price.
    let mut warm_cache = load_session_cache();
    let warm = cached_session(&journal, &mut warm_cache).expect("warm hit at the current schema");
    assert_eq!(
        warm.metrics.cost_estimated, cold.metrics.cost_estimated,
        "warm replay must keep the tier-priced cost"
    );

    // The rm-485-class scenario: surgically degrade the store to schema
    // 72 (what a pre-rung build writes). It must NOT be served — the
    // entry would replay the standard-rate reprice — and the discard
    // must be recorded in the schema_invalidations ledger, not silent.
    let cache_file = session_cache_path();
    let raw = fs::read_to_string(&cache_file).expect("cache file exists");
    let mut degraded: Value = serde_json::from_str(&raw).expect("cache file is JSON");
    degraded["schema_version"] = Value::from(72);
    fs::write(&cache_file, serde_json::to_string(&degraded).unwrap()).unwrap();
    let mut stale_cache = load_session_cache();
    assert!(
        cached_session(&journal, &mut stale_cache).is_none(),
        "a warm v72 entry must not serve the standard-rate cost_estimated"
    );
    let ledger = stale_cache.schema_invalidations();
    let record = ledger
        .last()
        .expect("the invalidation must be recorded, not silent");
    assert_eq!(record.get("reason").and_then(Value::as_str), Some("schema"));
    assert_eq!(
        record
            .get("observed_schema_version")
            .and_then(Value::as_i64),
        Some(72)
    );
    assert_eq!(
        record.get("schema_version").and_then(Value::as_i64),
        Some(SESSION_CACHE_SCHEMA_VERSION)
    );
    assert!(
        record
            .get("dropped_entries")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1,
        "the discarded warm entry must be counted: {record}"
    );

    // And the tier price regenerates once on next scan.
    let mut regenerated = load_session_cache();
    let fresh = parse_file(&journal).expect("journal still parses");
    assert_eq!(fresh.metrics.cost_estimated, tier_cost);
    store_session(&journal, &fresh, &mut regenerated).expect("re-store_session");
    save_session_cache(&mut regenerated).expect("re-save_session_cache");
    let mut healed_cache = load_session_cache();
    let healed =
        cached_session(&journal, &mut healed_cache).expect("regenerated warm hit at the new rung");
    assert_eq!(
        healed.metrics.cost_estimated, tier_cost,
        "the regenerated entry must serve the tier-priced cost"
    );

    match prior_cache {
        Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
        None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
    }
    let _ = fs::remove_dir_all(&root);
}

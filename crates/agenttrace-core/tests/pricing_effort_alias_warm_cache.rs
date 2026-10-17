//! rm-1511 warm-cache regression (integration of run 0915f736 cycle 4,
//! conflict case ab19f050f02e4aa882c9c659a9a089b8): the effort-alias
//! pricing fix reprices reasoning-effort-suffixed model ids
//! (`claude-sonnet-5-5-medium` and kin) at their in-catalog base — the
//! recorded PoC flips $0.0105 → $0.007 — but `pricing_snapshot.json`
//! is byte-identical, so the rm-196 catalog-identity freshness gate
//! keys an UNCHANGED digest and cannot self-heal a warm store: without
//! the 70 → 71 rung a v70 entry keeps replaying the over-billed
//! `cost_estimated` for unchanged files forever (the rm-485 class).
//!
//! The rung must invalidate that store so the corrected cost
//! regenerates. This is the batch's warm-schema-70 pin of the sweep
//! doc's oracle family
//! (`session-cache-schema-bump-sweeps-all-pinned-surfaces.md`).
//!
//! Own test binary on purpose (the rm-1522 precedent,
//! `claude_v2_1_300_warm_cache.rs`): cache paths load/save the session
//! cache in-process, so an env-touching warm-cache regression cannot
//! share a binary with anything else without a cache-file race — the
//! only test here touches `AGENTTRACE_SESSION_CACHE_DIR`.

use agenttrace_core::{
    cached_session, load_session_cache, parse_file, save_session_cache, session_cache_path,
    store_session, SESSION_CACHE_SCHEMA_VERSION,
};
use serde_json::Value;
use std::fs;

/// The recorded PoC journal (contract twin of
/// `recorded_poc_flips_from_default_to_base_rates`): 1000in/500out on
/// `claude-sonnet-5-5-medium` — $0.007 at the base's 2e-6/1e-5 rates,
/// $0.0105 under the pre-fix builtin default.
fn write_journal(root: &std::path::Path) -> std::path::PathBuf {
    let journal = root.join("medium.jsonl");
    fs::write(
        &journal,
        concat!(
            r#"{"type":"assistant","message":{"id":"m1","model":"claude-sonnet-5-5-medium","usage":{"input_tokens":1000,"output_tokens":500}},"timestamp":"2026-10-14T10:00:00Z"}"#,
            "\n"
        ),
    )
    .expect("journal write");
    journal
}

/// Cold prices at the base rate, warm replays it, and a v70 store
/// (what a pre-rung build writes) is invalidated — never served — so
/// the over-billed entry cannot survive the rung, with the discard
/// recorded in the `schema_invalidations` ledger.
#[test]
fn warm_schema_70_store_is_invalidated_and_the_overbill_regenerates() {
    let root = std::env::temp_dir().join(format!("rm1511-warm70-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("tempdir");
    let journal = write_journal(&root);

    // The only env-touching test in this binary.
    let cache_dir = root.join("cache");
    fs::create_dir_all(&cache_dir).expect("cache dir");
    let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache_dir);

    // Cold: the effort-alias arm resolves the in-catalog base — $0.007,
    // the recorded PoC's corrected price — and the entry persists.
    let mut cache = load_session_cache();
    assert!(
        cached_session(&journal, &mut cache).is_none(),
        "cold: no entry yet"
    );
    let cold = parse_file(&journal).expect("journal parses");
    let base_cost = 1000.0 * 2.0e-6 + 500.0 * 1.0e-5; // $0.007 at base rates
    assert_eq!(
        cold.metrics.cost_estimated, base_cost,
        "cold parse must price the suffixed id at its in-catalog base"
    );
    store_session(&journal, &cold, &mut cache).expect("store_session");
    save_session_cache(&mut cache).expect("save_session_cache");

    // Warm at the live schema: the served entry replays the SAME price.
    let mut warm_cache = load_session_cache();
    let warm = cached_session(&journal, &mut warm_cache).expect("warm hit at the current schema");
    assert_eq!(
        warm.metrics.cost_estimated, cold.metrics.cost_estimated,
        "warm replay must keep the base-rate price"
    );

    // The rm-485-class scenario: surgically degrade the store to schema
    // 70 (what a pre-rung build writes). It must NOT be served — the
    // entry would replay the $0.0105 over-bill — and the discard must
    // be recorded in the schema_invalidations ledger, not silent.
    let cache_file = session_cache_path();
    let raw = fs::read_to_string(&cache_file).expect("cache file exists");
    let mut degraded: Value = serde_json::from_str(&raw).expect("cache file is JSON");
    degraded["schema_version"] = Value::from(70);
    fs::write(&cache_file, serde_json::to_string(&degraded).unwrap()).unwrap();
    let mut stale_cache = load_session_cache();
    assert!(
        cached_session(&journal, &mut stale_cache).is_none(),
        "a warm v70 entry must not serve the over-billed cost_estimated"
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
        Some(70)
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

    // And the corrected price regenerates once on next scan.
    let mut regenerated = load_session_cache();
    let fresh = parse_file(&journal).expect("journal still parses");
    assert_eq!(fresh.metrics.cost_estimated, base_cost);
    store_session(&journal, &fresh, &mut regenerated).expect("re-store_session");
    save_session_cache(&mut regenerated).expect("re-save_session_cache");
    let mut healed_cache = load_session_cache();
    let healed =
        cached_session(&journal, &mut healed_cache).expect("regenerated warm hit at the new rung");
    assert_eq!(
        healed.metrics.cost_estimated, base_cost,
        "the regenerated entry must serve the base-rate price"
    );

    match prior_cache {
        Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
        None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
    }
    let _ = fs::remove_dir_all(&root);
}

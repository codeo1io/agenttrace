//! rm-1522 warm-cache regression (review 3ea62c4d F1 → fix 282d5474):
//! the claude-lane unmatched-kind disclosure — `claude_unmatched_type:
//! {kind}` counters riding the lane's one-carrier meta event — shipped
//! at the implement round WITHOUT a SESSION_CACHE_SCHEMA_VERSION rung,
//! so a warm v64 store served the PRE-change parse and `--doctor`
//! never materialized the counter for an upgraded fleet. The review
//! proved it live: the old binary warms the store over this fixture,
//! the new binary's `--doctor` over the warm store discloses nothing
//! (0 hits) while the cold parse discloses.
//!
//! The 64 → 65 rung must invalidate that store so the disclosure
//! regenerates. This is the batch's warm-schema-64 pin of the sweep
//! doc's oracle family
//! (`session-cache-schema-bump-sweeps-all-pinned-surfaces.md`).
//!
//! Own test binary on purpose: `render_doctor_report` loads/saves the
//! session cache in-process, so an env-touching warm-cache regression
//! cannot share a binary with the doctor test without a cache-file
//! race — the only test here touches `AGENTTRACE_SESSION_CACHE_DIR`.

use agenttrace_core::{
    cached_session, load_session_cache, parse_file, save_session_cache, session_cache_path,
    store_session, SESSION_CACHE_SCHEMA_VERSION,
};
use serde_json::Value;
use std::fs;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-code-2-1-300/request-reproduction.jsonl"
);

/// Cold discloses, warm replays the disclosure, and a v64 store (what
/// a pre-rung build writes) is invalidated — never served — with the
/// discard recorded in the `schema_invalidations` ledger.
#[test]
fn warm_schema_64_store_is_invalidated_and_the_disclosure_regenerates() {
    let root = std::env::temp_dir().join(format!("rm1522-warm64-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("tempdir");
    let journal = root.join("request-reproduction.jsonl");
    fs::copy(FIXTURE, &journal).expect("fixture copy");

    // The only env-touching test in this binary.
    let cache_dir = root.join("cache");
    fs::create_dir_all(&cache_dir).expect("cache dir");
    let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache_dir);

    // Cold: the disclosure lands in metrics.disclosure_counters — the
    // channel --doctor folds and the cache entry persists.
    let mut cache = load_session_cache();
    assert!(
        cached_session(&journal, &mut cache).is_none(),
        "cold: no entry yet"
    );
    let cold = parse_file(&journal).expect("vendored fixture parses");
    assert_eq!(
        cold.metrics
            .disclosure_counters
            .get("claude_unmatched_type:request_reproduction"),
        Some(&3),
        "cold parse must disclose the unmatched kind"
    );
    store_session(&journal, &cold, &mut cache).expect("store_session");
    save_session_cache(&mut cache).expect("save_session_cache");

    // Warm at the live schema: the served entry replays the SAME
    // disclosure (metrics.disclosure_counters round-trips through the
    // Go-mirror cache entry).
    let mut warm_cache = load_session_cache();
    let warm = cached_session(&journal, &mut warm_cache).expect("warm hit at the current schema");
    assert_eq!(
        warm.metrics.disclosure_counters, cold.metrics.disclosure_counters,
        "warm replay must keep the claude_unmatched_type disclosure"
    );

    // The F1 scenario: surgically degrade the store to schema 64
    // (what a pre-rung build writes). It must NOT be served — the
    // entry replays a parse without the disclosure — and the discard
    // must be recorded in the schema_invalidations ledger, not silent.
    let cache_file = session_cache_path();
    let raw = fs::read_to_string(&cache_file).expect("cache file exists");
    let mut degraded: Value = serde_json::from_str(&raw).expect("cache file is JSON");
    degraded["schema_version"] = Value::from(64);
    fs::write(&cache_file, serde_json::to_string(&degraded).unwrap()).unwrap();
    let mut stale_cache = load_session_cache();
    assert!(
        cached_session(&journal, &mut stale_cache).is_none(),
        "review 3ea62c4d F1: a warm v64 entry must not serve the pre-change parse"
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
        Some(64)
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

    match prior_cache {
        Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
        None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
    }
    let _ = fs::remove_dir_all(&root);
}

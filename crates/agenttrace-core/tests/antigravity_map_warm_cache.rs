//! rm-1533 completion warm-cache regression (run 8a5372ee cycle 7):
//! the completed antigravity numeric table (ccusage #1948 / v20.0.41
//! parity) changes DERIVED IDENTITY for unchanged source files — a
//! trajectory whose `modelConfigId` is the number 1402 attributed to
//! the stringified "1402" pre-completion and now attributes to
//! claude-opus-5-5 — but the change is code-only (no
//! `pricing_snapshot.json` byte moves), so the rm-196 catalog-identity
//! freshness gate keys an UNCHANGED digest and cannot self-heal a warm
//! store: without the 73 → 74 rung a v73 entry keeps replaying the
//! pre-completion attribution for unchanged files forever (the rm-485
//! class — the same disease the rm-1511 effort-alias rung cured).
//!
//! Own test binary on purpose (the rm-1511/rm-1522 precedent): cache
//! paths load/save the session cache in-process, so an env-touching
//! warm-cache regression cannot share a binary with anything else
//! without a cache-file race — the only test here touches
//! `AGENTTRACE_SESSION_CACHE_DIR`.

use agenttrace_core::{
    cached_session, load_session_cache, parse_file, save_session_cache, session_cache_path,
    store_session, SESSION_CACHE_SCHEMA_VERSION,
};
use serde_json::Value;
use std::fs;

/// The completion's flagship journal: 1000in/500out on numeric
/// `modelConfigId` 1402 — attributed "1402" by every pre-completion
/// build, claude-opus-5-5 by this one, priced at the bare key's own
/// rates (present in both the 2026-10-08 and 2026-10-17 bundles).
fn write_trajectory(root: &std::path::Path) -> std::path::PathBuf {
    let trajectory = root.join("55555555-6666-7777-8888-999999999999.trajectory.json");
    fs::write(
        &trajectory,
        concat!(
            r#"{"trajectoryId":"55555555-6666-7777-8888-999999999999","steps":[{"type":"CORTEX_STEP_TYPE_PLANNER_RESPONSE","status":"COMPLETED","metadata":{"createdAt":"2026-10-22T10:00:05Z"},"plannerResponse":{"response":"Opus generation.","usageStats":{"inputTokens":1000,"outputTokens":500},"modelConfigId":1402}}]}"#,
            "\n"
        ),
    )
    .expect("trajectory write");
    trajectory
}

/// Cold attributes to claude-opus-5-5, warm replays it, and a v73
/// store (what the pre-rung build at the wall writes) is invalidated —
/// never served — so the stringified "1402" attribution cannot survive
/// the rung, with the discard recorded in the `schema_invalidations`
/// ledger.
#[test]
fn warm_schema_73_store_is_invalidated_and_the_identity_regenerates() {
    let root = std::env::temp_dir().join(format!("rm1533-warm73-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("tempdir");
    let trajectory = write_trajectory(&root);

    // The only env-touching test in this binary.
    let cache_dir = root.join("cache");
    fs::create_dir_all(&cache_dir).expect("cache dir");
    let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache_dir);

    // Cold: the completed table attributes 1402 to claude-opus-5-5 —
    // the pre-completion build served the stringified "1402" here.
    let mut cache = load_session_cache();
    assert!(
        cached_session(&trajectory, &mut cache).is_none(),
        "cold: no entry yet"
    );
    let cold = parse_file(&trajectory).expect("trajectory parses");
    assert_eq!(
        cold.metrics.model_used, "claude-opus-5-5",
        "cold parse must attribute the completed mapping, never the stringified id"
    );
    store_session(&trajectory, &cold, &mut cache).expect("store_session");
    save_session_cache(&mut cache).expect("save_session_cache");

    // Warm at the live schema: the served entry replays the SAME
    // identity.
    let mut warm_cache = load_session_cache();
    let warm = cached_session(&trajectory, &mut warm_cache).expect("warm hit at the current schema");
    assert_eq!(
        warm.metrics.model_used, cold.metrics.model_used,
        "warm replay must keep the completed attribution"
    );

    // The rm-485-class scenario: surgically degrade the store to
    // schema 73 (what the pre-rung build at the wall writes — its
    // table predates the completion, so its entries hold the
    // stringified "1402"). It must NOT be served, and the discard must
    // be recorded in the schema_invalidations ledger, not silent.
    let cache_file = session_cache_path();
    let raw = fs::read_to_string(&cache_file).expect("cache file exists");
    let mut degraded: Value = serde_json::from_str(&raw).expect("cache file is JSON");
    degraded["schema_version"] = Value::from(73);
    fs::write(&cache_file, serde_json::to_string(&degraded).unwrap()).unwrap();
    let mut stale_cache = load_session_cache();
    assert!(
        cached_session(&trajectory, &mut stale_cache).is_none(),
        "a warm v73 entry must not serve the pre-completion attribution"
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
        Some(73)
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

    // And the completed identity regenerates once on next scan.
    let mut regenerated = load_session_cache();
    let fresh = parse_file(&trajectory).expect("trajectory still parses");
    assert_eq!(fresh.metrics.model_used, "claude-opus-5-5");
    store_session(&trajectory, &fresh, &mut regenerated).expect("re-store_session");
    save_session_cache(&mut regenerated).expect("re-save_session_cache");
    let mut healed_cache = load_session_cache();
    let healed = cached_session(&trajectory, &mut healed_cache)
        .expect("regenerated warm hit at the new rung");
    assert_eq!(
        healed.metrics.model_used, "claude-opus-5-5",
        "the regenerated entry must serve the completed attribution"
    );

    match prior_cache {
        Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
        None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
    }
    let _ = fs::remove_dir_all(&root);
}

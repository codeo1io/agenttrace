//! rm-1426 raw-origin pin: `is_locally_defined` judges slugs against the
//! RAW keys of the ACTIVE catalog origin — a downloaded cache when one is
//! served, the bundled snapshot otherwise — never the bundle alone. Own
//! test binary (own process): the catalog singleton and its raw-key set
//! are `OnceLock`-cached, so the cache-relocation env must be set before
//! the first pricing call (same shape as the override-file test).

#[test]
fn cache_catalog_raw_keys_decide_what_is_named() {
    let dir = std::env::temp_dir().join(format!("at-rm1426-cache-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    // Raw LiteLLM envelope with ONE prefixed chat row for the
    // deployment's own slug. AGENTTRACE_SESSION_CACHE_DIR relocates
    // pricing.json here (rm-575), so this becomes the catalog the
    // process serves entries AND raw keys from.
    std::fs::write(
        dir.join("pricing.json"),
        r#"{"z.ai/glm-5.2":{"mode":"chat","litellm_provider":"zai","input_cost_per_token":0.00000152,"output_cost_per_token":0.000012,"max_input_tokens":272000}}"#,
    )
    .expect("write cache");
    std::env::remove_var("AGENTTRACE_PRICING_FILE");
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &dir);

    // The cache's OWN raw key names the slug as recorded → priced, not
    // locally-defined — even though the BUNDLED snapshot never carried
    // "z.ai/glm-5.2" (it carries a different carrier's prefix).
    assert!(
        !agenttrace_core::is_locally_defined("z.ai/glm-5.2"),
        "a raw key of the ACTIVE cache names the slug"
    );
    // Sanity: the active origin really is the cache — the rate comes
    // from the hand-written row, not from any bundled carrier.
    let price = agenttrace_core::lookup_price_tiered("z.ai/glm-5.2", None);
    assert!(
        (price.input - 1.52).abs() < 1e-9,
        "cache row prices it: {price:?}"
    );
    // Converse arm: a carrier prefix the ACTIVE cache does NOT carry
    // ("zai/glm-5.2" is only a bundled-snapshot raw key) prices by a
    // borrowed collapsed name → locally-defined. Judging by the bundle
    // instead of the active origin would call this one named.
    assert!(
        agenttrace_core::is_locally_defined("zai/glm-5.2"),
        "the active origin's raw keys decide, not the bundled snapshot's"
    );

    std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
    let _ = std::fs::remove_dir_all(&dir);
}

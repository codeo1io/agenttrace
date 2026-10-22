//! rm-1426 escape-hatch arm: an `AGENTTRACE_PRICING_FILE` row resolves a
//! locally-defined slug, flipping the disclosure off. This lives in its own
//! test binary (each integration test file is its own process) because the
//! runtime config that carries the env knob is `OnceLock`-cached: the env
//! must be set before the first pricing call in the process.

#[test]
fn override_file_resolves_locally_defined_slug() {
    let dir = std::env::temp_dir().join("at-rm1426-override");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("pricing.json");
    // PricingOverrideMaps envelope: `prices` -> slug -> Price row in the
    // struct's serde names (input/output/cw/cr — all required fields).
    std::fs::write(
        &path,
        r#"{"prices":{"glm-5.2":{"input":0.0000014,"output":0.0000044,"cw":0.0,"cr":0.0}}}"#,
    )
    .expect("write override");
    std::env::set_var("AGENTTRACE_PRICING_FILE", &path);

    // First pricing call in this process happens with the override live.
    assert!(
        !agenttrace_core::is_locally_defined("glm-5.2"),
        "operator override resolves the locally-defined slug"
    );
    let price = agenttrace_core::lookup_price_tiered("glm-5.2", None);
    assert_eq!(price.input, 0.0000014);
    assert_eq!(price.output, 0.0000044);
    // rm-1426 acceptance (4): the rate row CARRIES operator provenance —
    // "user override", not a catalog/fallback claim (rm-231 family).
    assert_eq!(
        agenttrace_core::pricing_source_for("glm-5.2"),
        "user override",
        "operator-bound price is disclosed as such"
    );

    std::env::remove_var("AGENTTRACE_PRICING_FILE");
    let _ = std::fs::remove_file(&path);
}

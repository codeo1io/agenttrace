//! rm-1511 contract: reasoning-effort-suffixed model ids (the class
//! Antigravity CLI emits — `claude-sonnet-5-5-medium`/-low) price at their
//! in-catalog base instead of the builtin Claude default, the effort tier
//! stays derivable from the raw id, and the fallback disclosure names models
//! whose exact base was available instead of only counting them.
//!
//! Recorded pre-fix behavior (research PoC, HEAD 8bc0587): the suffixed id
//! billed $0.0105 at 200k-ctx builtin-default rates for volumes the exact
//! base prices at $0.007 with 1M context, while only the aggregate
//! `fallback_pricing` counter moved and `cost_basis` rendered "priced".

use std::io::Write;

use agenttrace_core::{
    data_health, effort_tier_of, has_specific_price, lookup_price, parse_raw_session,
    specific_base_for_unpriced,
};

fn rates(model: &str) -> (f64, f64) {
    let price = lookup_price(model);
    (price.input, price.output)
}

fn write_fixture(dir: &std::path::Path, name: &str, model: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(
        file,
        r#"{{"type":"assistant","message":{{"id":"m1","model":"{model}","usage":{{"input_tokens":1000,"output_tokens":500}}}},"timestamp":"2026-10-14T10:00:00Z"}}"#
    )
    .unwrap();
    path
}

fn parse_fixture(dir: &std::path::Path, name: &str, model: &str) -> agenttrace_core::Session {
    let path = write_fixture(dir, name, model);
    let raw = std::fs::read_to_string(&path).unwrap();
    parse_raw_session(name, path.to_str().unwrap(), &raw).unwrap_or_else(|e| panic!("{e}"))
}

/// Review 1a615b64 F6 (the rm-369 /tmp-collision class, cycle-3 csv_export
/// precedent): fixture dirs are unique per invocation — a fixed shared dir
/// races concurrent runs of this suite.
fn fixture_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "pricing_effort_alias_contract-{}-{tag}",
        std::process::id()
    ))
}

#[test]
fn effort_suffixed_ids_price_at_their_in_catalog_base() {
    let base = rates("claude-sonnet-5-5");
    assert!(base.0 > 0.0, "base key must be in the bundled catalog");
    for model in [
        "claude-sonnet-5-5-low",
        "claude-sonnet-5-5-medium",
        "claude-sonnet-5-5-high",
        "claude-sonnet-5-5-thinking",
        "claude-sonnet-5-5:thinking",
    ] {
        assert_eq!(rates(model), base, "expected base rates for {model}");
    }
    // Case-insensitivity rides normalize_model like every other arm.
    assert_eq!(rates("Claude-Sonnet-5-5-Medium"), base);
}

#[test]
fn effort_suffix_arm_depends_on_no_suffixed_catalog_keys() {
    // The alias arm resolves the base; if a suffixed key ever lands in the
    // catalog (exact match wins first), this pin forces a conscious update.
    let snapshot: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("../src/pricing_snapshot.json"))
            .expect("bundled snapshot parses");
    for key in [
        "claude-sonnet-5-5-low",
        "claude-sonnet-5-5-medium",
        "claude-sonnet-5-5-high",
        "claude-sonnet-5-5-thinking",
    ] {
        assert!(
            !snapshot.contains_key(key),
            "suffixed key {key} is now catalogued; update this contract"
        );
    }
}

#[test]
fn effort_tier_stays_derivable_after_resolution() {
    assert_eq!(effort_tier_of("claude-sonnet-5-5-low"), Some("low"));
    assert_eq!(effort_tier_of("claude-sonnet-5-5-medium"), Some("medium"));
    assert_eq!(effort_tier_of("claude-sonnet-5-5-high"), Some("high"));
    assert_eq!(
        effort_tier_of("claude-sonnet-5-5-thinking"),
        Some("thinking")
    );
    assert_eq!(
        effort_tier_of("claude-sonnet-5-5:thinking"),
        Some("thinking")
    );
    assert_eq!(effort_tier_of("claude-sonnet-5-5"), None);
    assert_eq!(effort_tier_of("gpt-5.6"), None);
}

#[test]
fn known_effort_suffix_makes_the_price_specific() {
    // The aggregate fallback counter drops: the session now bills at an
    // exact catalog row instead of the builtin default.
    assert!(has_specific_price("claude-sonnet-5-5-medium"));
    assert!(has_specific_price("claude-opus-5-5:thinking"));
    // And the disclosure helper no longer flags it — it is priced.
    assert_eq!(specific_base_for_unpriced("claude-sonnet-5-5-medium"), None);
}

#[test]
fn unknown_model_with_unknown_suffix_stays_at_default() {
    let default = rates("totally-unknown-model");
    assert_eq!(default, rates("totally-unknown-model-medium"));
    assert!(!has_specific_price("totally-unknown-model-medium"));
    assert_eq!(
        specific_base_for_unpriced("totally-unknown-model-medium"),
        None
    );
}

#[test]
fn disclosure_names_unrecognized_suffixes_with_available_base() {
    // `-ultra` is not in the effort table: the session still prices at the
    // default, but the exact base sits in the catalog — the rm-1511
    // disclosure must say so instead of only counting fallback sessions.
    let model = "claude-sonnet-5-5-ultra";
    assert!(!has_specific_price(model));
    assert_eq!(
        specific_base_for_unpriced(model),
        Some("claude-sonnet-5-5".to_string())
    );
    // A priced model never names a base, whatever its tail.
    assert_eq!(specific_base_for_unpriced("claude-sonnet-5-5"), None);
    // A tail that is not a plain short token stays silent: date-suffixed
    // style ids (`-2026`) are their own model family, not a hidden base.
    assert_eq!(specific_base_for_unpriced("claude-sonnet-5-5-2026"), None);
    assert_eq!(specific_base_for_unpriced("claude-sonnet-5-5-x9"), None);
    // And the short-token rule is bounded: any ≤10-char alpha tail on a
    // priced base discloses (`-ultra`), a longer one does not.
    assert_eq!(
        specific_base_for_unpriced("claude-sonnet-5-5-abcdefghij"),
        Some("claude-sonnet-5-5".to_string())
    );
    assert_eq!(
        specific_base_for_unpriced("claude-sonnet-5-5-abcdefghijk"),
        None
    );
}

#[test]
fn data_health_names_models_with_available_base() {
    let dir = fixture_dir("health");
    std::fs::create_dir_all(&dir).unwrap();
    let session = parse_fixture(&dir, "ultra.jsonl", "claude-sonnet-5-5-ultra");
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(health.fallback_pricing, 1);
    assert_eq!(
        health.fallback_with_base_available,
        vec!["claude-sonnet-5-5".to_string()]
    );
    // And the priced suffix class no longer appears anywhere in the lane.
    let session = parse_fixture(&dir, "medium2.jsonl", "claude-sonnet-5-5-medium");
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(health.fallback_pricing, 0);
    assert!(health.fallback_with_base_available.is_empty());
}

#[test]
fn recorded_poc_flips_from_default_to_base_rates() {
    // Pre-fix (research PoC at 8bc0587): $0.0105 — the builtin default
    // (3e-6/1.5e-5) applied while the exact base (2e-6/1e-5) was catalogued.
    let dir = fixture_dir("poc-medium");
    std::fs::create_dir_all(&dir).unwrap();
    let session = parse_fixture(&dir, "medium.jsonl", "claude-sonnet-5-5-medium");
    let expected = 1000.0 * 2.0e-6 + 500.0 * 1.0e-5; // $0.007 at base rates
    assert_eq!(session.metrics.cost_estimated, expected);
}

#[test]
fn poc_baseline_unaffected_by_the_alias_arm() {
    let dir = fixture_dir("poc-base");
    std::fs::create_dir_all(&dir).unwrap();
    let session = parse_fixture(&dir, "base.jsonl", "claude-sonnet-5-5");
    let expected = 1000.0 * 2.0e-6 + 500.0 * 1.0e-5;
    assert_eq!(session.metrics.cost_estimated, expected);
}

/// Review 1a615b64 F2 — rm-1511 acceptance arm (3): an Antigravity-shaped
/// fixture (the rm-947 lane: `PLANNER_RESPONSE` entries carrying
/// `step_index`/`created_at`, usage via the `usageStats` RPC names, model
/// via `modelConfigId`) exercises the alias end-to-end through the real
/// parse path — the exact recorded PoC volumes flip from the $0.0105
/// builtin default to the $0.007 base rate on the antigravity parser lane.
#[test]
fn antigravity_lane_prices_effort_alias_at_base() {
    let dir = fixture_dir("antigravity");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("conversations.jsonl");
    let mut file = std::fs::File::create(&path).unwrap();
    use std::io::Write as _;
    file.write_all(
        br#"{"type":"USER_INPUT","step_index":0,"created_at":"2026-10-14T10:00:00Z","content":"plan"}"#,
    )
    .unwrap();
    file.write_all(b"\n").unwrap();
    file.write_all(
        br#"{"type":"PLANNER_RESPONSE","step_index":1,"created_at":"2026-10-14T10:00:05Z","content":"done","modelConfigId":"claude-sonnet-5-5-medium","usageStats":{"inputTokens":1000,"outputTokens":500}}"#,
    )
    .unwrap();
    file.write_all(b"\n").unwrap();
    drop(file);
    let raw = std::fs::read_to_string(&path).unwrap();
    let session = parse_raw_session("conversations.jsonl", path.to_str().unwrap(), &raw)
        .unwrap_or_else(|e| panic!("{e}"));
    // It rode the antigravity lane, not a generic fallback shape.
    assert_eq!(session.metrics.source_tool, "antigravity_cli");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(session.metrics.tokens_output, 500);
    assert_eq!(session.metrics.model_used, "claude-sonnet-5-5-medium");
    let expected = 1000.0 * 2.0e-6 + 500.0 * 1.0e-5; // $0.007 at base rates
    assert_eq!(session.metrics.cost_estimated, expected);
    // The priced class is no longer fallback, and the tier stays derivable.
    assert!(has_specific_price(&session.metrics.model_used));
    assert_eq!(effort_tier_of(&session.metrics.model_used), Some("medium"));
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(health.fallback_pricing, 0);
    assert!(health.fallback_with_base_available.is_empty());
}

/// Review 1a615b64 F4 — pin the second-order expansion (variants of
/// variants) so the catalog loop in `lookup_price_in` is a contract, not an
/// accident: `claude-sonnet-5-5-pro-high`'s first-order candidate list is
/// [raw, normalized, effort-stripped `claude-sonnet-5-5-pro`] and misses
/// (that mid-form is not catalogued); only the loop's expansion of the
/// mid-form's own minor arm (`pro` is a len<=3 minor token) reaches the
/// in-catalog `claude-sonnet-5-5`.
#[test]
fn second_order_variant_expansion_resolves_via_the_catalog_loop() {
    let snapshot: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("../src/pricing_snapshot.json"))
            .expect("bundled snapshot parses");
    assert!(
        !snapshot.contains_key("claude-sonnet-5-5-pro"),
        "pin premise: the mid-form must stay uncatalogued — update this contract \
         alongside the effort arm if that ever changes"
    );
    let base = rates("claude-sonnet-5-5");
    assert_eq!(rates("claude-sonnet-5-5-pro-high"), base);
    // The mid-form itself resolves through its own minor arm — same shared
    // basis, same answer (there was never a second truth function).
    assert!(has_specific_price("claude-sonnet-5-5-pro"));
    assert_eq!(effort_tier_of("claude-sonnet-5-5-pro-high"), Some("high"));
}

/// Review 1a615b64 F5 — the lexical tier marker on a catalogued SKU:
/// `aihubmix/gpt-5.4-high` is a real catalog key whose name ends in a tier
/// marker. Pricing is unaffected (the exact key wins at $2.5/$15 per
/// million — not the $3/$15 builtin default, so no stripping re-priced it);
/// only `effort_tier_of` reports a marker, which the doc contract now
/// discloses.
#[test]
fn effort_tier_marker_on_a_catalogued_sku_prices_exactly() {
    let sku = rates("aihubmix/gpt-5.4-high");
    assert_eq!(sku, (2.5, 15.0));
    assert_ne!(sku, rates("totally-unknown-model"));
    assert!(has_specific_price("aihubmix/gpt-5.4-high"));
    assert_eq!(effort_tier_of("aihubmix/gpt-5.4-high"), Some("high"));
    assert_eq!(specific_base_for_unpriced("aihubmix/gpt-5.4-high"), None);
}

/// rm-1533 red-first (cost half): ccusage v20.0.36 widened the numeric
/// map (1050 → gemini-3.1-flash-lite PR #1902, 1418/1419 → gemini-4-argon
/// PR #1905) and the 2026-10-20 snapshot refresh pinned both targets
/// (gemini/gemini-3.1-flash-lite, gemini/gemini-4-argon) — the build
/// order that made this arm landable same-day. Pre-fix, a numeric-1418
/// corpus attributed to raw digits and priced through the fallback lane
/// (no specific price, diluted unknown-model census). Post-fix the arm
/// prices SPECIFICALLY at the catalog row, and a still-unmapped id
/// (1277, unmapped upstream as of v20.0.37) keeps the WARN +
/// cost-exclusion contract: fallback pricing, disclosed — never a
/// silent $0.
#[test]
fn antigravity_numeric_id_arms_price_from_the_pinned_snapshot() {
    for (id, model) in [
        (1418, "gemini-4-argon"),
        (1419, "gemini-4-argon"),
        (1050, "gemini-3.1-flash-lite"),
    ] {
        let dir = fixture_dir("antigravity-numeric");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("conversations.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        use std::io::Write as _;
        file.write_all(
            br#"{"type":"USER_INPUT","step_index":0,"created_at":"2026-10-18T10:00:00Z","content":"plan"}"#,
        )
        .unwrap();
        file.write_all(b"\n").unwrap();
        let line = format!(
            r#"{{"type":"PLANNER_RESPONSE","step_index":1,"created_at":"2026-10-18T10:00:05Z","content":"done","modelConfigId":{id},"usageStats":{{"inputTokens":1000,"outputTokens":500}}}}"#
        );
        file.write_all(line.as_bytes()).unwrap();
        file.write_all(b"\n").unwrap();
        drop(file);
        let raw = std::fs::read_to_string(&path).unwrap();
        let session = parse_raw_session("conversations.jsonl", path.to_str().unwrap(), &raw)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(session.metrics.source_tool, "antigravity_cli");
        assert_eq!(
            session.metrics.model_used, model,
            "numeric id {id} attributes through the proven ccusage arm"
        );
        // `rates` returns per-million dollars; cost is per token.
        let (input, output) = rates(model);
        let expected = (1000.0 * input + 500.0 * output) / 1_000_000.0;
        assert_eq!(
            session.metrics.cost_estimated, expected,
            "numeric id {id} prices at the pinned catalog row, not a fallback"
        );
        assert!(
            has_specific_price(&session.metrics.model_used),
            "the mapped arm is a specifically priced model"
        );
        let health = data_health(std::slice::from_ref(&session), 1, 0);
        assert_eq!(health.fallback_pricing, 0);
        assert_eq!(health.unknown_models, 0);
    }

    // The unmapped arm: excluded from specific pricing, priced through
    // the DISCLOSED fallback lane (unknown-model WARN), never silently $0.
    let dir = fixture_dir("antigravity-numeric-unmapped");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("conversations.jsonl");
    let mut file = std::fs::File::create(&path).unwrap();
    use std::io::Write as _;
    file.write_all(
        br#"{"type":"USER_INPUT","step_index":0,"created_at":"2026-10-18T11:00:00Z","content":"plan"}"#,
    )
    .unwrap();
    file.write_all(b"\n").unwrap();
    file.write_all(
        br#"{"type":"PLANNER_RESPONSE","step_index":1,"created_at":"2026-10-18T11:00:05Z","content":"done","modelConfigId":1277,"usageStats":{"inputTokens":1000,"outputTokens":500}}"#,
    )
    .unwrap();
    file.write_all(b"\n").unwrap();
    drop(file);
    let raw = std::fs::read_to_string(&path).unwrap();
    let session = parse_raw_session("conversations.jsonl", path.to_str().unwrap(), &raw)
        .unwrap_or_else(|e| panic!("1277: {e}"));
    assert_eq!(session.metrics.model_used, "1277");
    assert!(!has_specific_price(&session.metrics.model_used));
    let (input, output) = rates("totally-unknown-model");
    assert_eq!(
        session.metrics.cost_estimated,
        (1000.0 * input + 500.0 * output) / 1_000_000.0,
        "the unmapped id bills at the disclosed default fallback, not a silent $0"
    );
    let health = data_health(std::slice::from_ref(&session), 1, 0);
    assert_eq!(
        health.fallback_pricing, 1,
        "the fallback estimate is WARNED"
    );
    assert_eq!(
        health.fallback_with_base_available,
        Vec::<String>::new(),
        "a raw numeric spelling must never masquerade as an available base"
    );
}

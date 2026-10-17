//! rm-1532 (run 92ea7949 cycle 4, assess 7fb648a0 F4 + research b1e8b617):
//! first-party pricing truth for the dated Anthropic ids Claude Code
//! actually records. The bundled LiteLLM snapshot carries NO row in
//! any form for `claude-sonnet-4-20250514`,
//! `claude-3-7-sonnet-20250219`, and `claude-opus-4-1-20250805`
//! (verified against LIVE LiteLLM, 4,161 keys: only
//! reseller-qualified rows — `openrouter/`, `vercel_ai_gateway/`,
//! `github_copilot/` — end in those ids), so the raw-first ladder's
//! base-model collapse served the reseller winner: a first-party
//! session rendered `By Provider: openrouter` / `vercel_ai_gateway`
//! at reseller-forwarded list rates. A snapshot refresh cannot fix
//! upstream absence, so the second catalog (the embedded models.dev
//! first-party extract, `models_dev_snapshot.json`) serves those ids
//! — consulted only when the LiteLLM ladder finds no first-party row.
//!
//! These pins hold the fixed contract:
//! 1. every extract id resolves at the vendor's LIST rates (rate,
//!    provider, and pricing-source all agree the extract served);
//! 2. a session naming a RESELLER keeps the reseller row — the
//!    extract speaks only for first-party (bare) ids, never for a
//!    qualified id the session itself scoped;
//! 3. a first-party id the LiteLLM catalog DOES carry (a current
//!    dated sonnet) still prices byte-identically at LiteLLM — the
//!    extract is a gap-filler, not a shadow catalog;
//! 4. unknown ids still fall through to the built-in default and
//!    report no specific price;
//! 5. `catalog_identity` folds the extract bytes in, so a changed
//!    extract (new binary) reprices cached sessions.

use agenttrace_core::{
    catalog_identity, has_specific_price, lookup_price, pricing_source_for, provider_for,
};
use std::sync::OnceLock;

/// The three dated ids the extract carries, with their published list
/// rates in $/M (input, output, cache_read, cache_write) — the vendor
/// list prices models.dev publishes, identical to what the reseller
/// rows forward.
const EXTRACT_ROWS: &[(&str, f64, f64, f64, f64)] = &[
    ("claude-sonnet-4-20250514", 3.0, 15.0, 0.3, 3.75),
    ("claude-3-7-sonnet-20250219", 3.0, 15.0, 0.3, 3.75),
    ("claude-opus-4-1-20250805", 15.0, 75.0, 1.5, 18.75),
];

fn expect_price(price: agenttrace_core::Price, input: f64, output: f64, cr: f64, cw: f64) {
    let assert_close = |name: &str, got: f64, want: f64| {
        assert!(
            (got - want).abs() < 1e-9,
            "{name} must be the vendor list rate {want}, got {got}"
        );
    };
    assert_close("input", price.input, input);
    assert_close("output", price.output, output);
    // rm-1508 made the cache slots `Option<f64>`: an absent rate is
    // `None` (disclosed fallback), a published number — explicit $0
    // included — is `Some`. Every extract row publishes both rates,
    // so a `None` here is a regression in the extract or its ingest.
    assert_close(
        "cache_read",
        price.cr.expect("extract row publishes cache_read"),
        cr,
    );
    assert_close(
        "cache_write",
        price.cw.expect("extract row publishes cache_write"),
        cw,
    );
}

#[test]
fn extract_ids_bill_at_first_party_list_rates() {
    for (id, input, output, cr, cw) in EXTRACT_ROWS {
        expect_price(lookup_price(id), *input, *output, *cr, *cw);
    }
}

#[test]
fn extract_ids_attribute_to_their_vendor() {
    for (id, ..) in EXTRACT_ROWS {
        assert_eq!(
            provider_for(id).as_deref(),
            Some("anthropic"),
            "{id}: first-party ids must attribute to anthropic, not a reseller"
        );
    }
}

#[test]
fn extract_service_is_disclosed_at_the_provenance_line() {
    for (id, ..) in EXTRACT_ROWS {
        let source = pricing_source_for(id);
        assert!(
            source.starts_with("models.dev snapshot "),
            "{id}: the serving catalog must be disclosed as the models.dev extract, got {source:?}"
        );
        assert!(
            source.contains("LiteLLM lacks a first-party row"),
            "{id}: the disclosure must name WHY the extract served, got {source:?}"
        );
    }
}

#[test]
fn extract_ids_are_specific_prices_not_fallbacks() {
    for (id, ..) in EXTRACT_ROWS {
        assert!(
            has_specific_price(id),
            "{id}: served by the extract, so it is a specific price"
        );
    }
}

#[test]
fn reseller_qualified_ids_keep_their_reseller_rows() {
    // The session itself scoped these ids to a reseller lane: the
    // extract must NOT override an explicit qualification (it speaks
    // only for the bare first-party form).
    let openrouter = lookup_price("openrouter/claude-sonnet-4-20250514");
    assert!(
        (openrouter.input - 3.0).abs() < 1e-9,
        "the openrouter row forwards the same list input rate"
    );
    assert_eq!(
        provider_for("openrouter/claude-sonnet-4-20250514").as_deref(),
        Some("openrouter"),
        "a reseller-qualified id attributes to the reseller"
    );
    assert!(
        !pricing_source_for("openrouter/claude-sonnet-4-20250514")
            .starts_with("models.dev snapshot"),
        "a reseller-qualified id prices through the LiteLLM catalog, disclosed as such"
    );
}

#[test]
fn first_party_ids_the_primary_catalog_carries_still_price_at_litellm() {
    // The extract is a gap-filler: an id LiteLLM prices first-party
    // (a current dated sonnet) must be byte-identical to the
    // pre-extract behavior — LiteLLM rates, LiteLLM source.
    let current = lookup_price("claude-sonnet-4-5-20250929");
    assert!(
        (current.input - 3.0).abs() < 1e-9 && (current.output - 15.0).abs() < 1e-9,
        "the current sonnet prices at its own LiteLLM row"
    );
    assert_eq!(
        provider_for("claude-sonnet-4-5-20250929").as_deref(),
        Some("anthropic"),
        "the current sonnet attributes through LiteLLM's first-party row"
    );
    assert!(
        pricing_source_for("claude-sonnet-4-5-20250929").starts_with("LiteLLM snapshot"),
        "the current sonnet's provenance names the LiteLLM catalog, not the extract"
    );
}

#[test]
fn unknown_ids_still_fall_through_to_the_default() {
    let unknown = "totally-unknown-model-9000";
    assert!(
        !has_specific_price(unknown),
        "an id neither catalog carries has no specific price"
    );
    assert_eq!(provider_for(unknown), None);
    assert_eq!(
        pricing_source_for(unknown),
        "built-in fallback",
        "unknown ids keep the built-in fallback provenance"
    );
}

#[test]
fn catalog_identity_folds_the_extract_bytes() {
    // rm-1532: the extract can reprice and re-attribute ids, so its
    // bytes must ride the identity the session cache freshness-key
    // consumes — a changed extract (new binary) invalidates cached
    // per-session costs even when the primary catalog is identical.
    static FIRST: OnceLock<String> = OnceLock::new();
    let identity = FIRST.get_or_init(|| catalog_identity().to_string());
    assert_eq!(
        catalog_identity(),
        identity.as_str(),
        "identity is stable within a build"
    );
    let digest = identity
        .rsplit("-d")
        .next()
        .unwrap_or_default()
        .trim_start_matches('d');
    assert!(
        identity.contains("-d") && !digest.is_empty() && digest.len() == 16,
        "identity must carry the extract digest suffix (-dXXXXXXXXXXXXXXXX), got {identity:?}"
    );
}

#[test]
fn extract_json_carries_its_provenance_note() {
    // The file documents WHERE its rates came from and the retirement
    // condition (LiteLLM shipping a first-party row ends the search).
    // If the note is gone the file was machine-edited blind — fail
    // loudly instead of serving unattributable rates.
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/models_dev_snapshot.json"
    ))
    .expect("models_dev_snapshot.json readable from the test");
    assert!(
        raw.contains("self-retire") && raw.contains("\"date\": \"2026-10-16\""),
        "the extract must carry its source note and pinned date"
    );
    for (id, ..) in EXTRACT_ROWS {
        let qualified = format!("anthropic/{id}");
        assert!(
            raw.contains(&qualified),
            "the extract must carry the qualified row {qualified}"
        );
    }
}

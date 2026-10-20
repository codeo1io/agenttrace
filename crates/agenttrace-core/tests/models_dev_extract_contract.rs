//! rm-1532 rider guard (run ab7a73576cd144e9b873ade127d9abbb cycle 5, batch
//! "pricing-truth cure v2"): the vendored models.dev extract can never be
//! silently superseded or stale-shipped by the sanctioned refresh. The full
//! first-party behavior contract (vendor list rates, precedence ladder,
//! disclosure strings) lives in tests/first_party_pricing_extract_contract.rs;
//! this file pins the GUARD surfaces the refresh machinery itself depends on:
//!
//! (a) the three dated first-party ids still resolve through the extract —
//!     and nothing in the bundled snapshot shadows them. The rm-176
//!     2026-10-17 refresh retired every reseller row that used to carry
//!     those model names (the 459-key `openrouter/*` family plus the
//!     github_copilot straggler), so the extract is now their ONLY pricing
//!     lane; a refresh that quietly reintroduced a first-party or reseller
//!     row for one of these ids would shift attribution without any test
//!     failing elsewhere;
//! (b) extract provenance is single-sourced: pricing.rs's
//!     MODELS_DEV_SNAPSHOT_DATE const and the extract's own _snapshot.date
//!     agree with the pinned date — a re-extract cannot ship behind a
//!     stale const, and a const bump cannot ship without the extract;
//! (c) the refresh-time assertion arm is pinned in the refresh machinery
//!     itself, not just prose: scripts/pricing/update-snapshot.sh must
//!     carry the three fail-closed refusals (date desync, first-party
//!     supersession, frozen-extract md5 identity), so deleting an arm
//!     from the script reddens THIS test instead of the next live
//!     refresh.

use agenttrace_core::{has_specific_price, lookup_price, pricing_source_for};

const PRICING_RS: &str = include_str!("../src/pricing.rs");
const EXTRACT: &str = include_str!("../src/models_dev_snapshot.json");
const BUNDLE: &str = include_str!("../src/pricing_snapshot.json");
const REFRESH_SCRIPT: &str = include_str!("../../../scripts/pricing/update-snapshot.sh");

/// rm-1532 arm (b): moving the extract without the const (or the const
/// without the extract) is a conscious, loud change — the pinned literal
/// makes both directions red here.
const PINNED_EXTRACT_DATE: &str = "2026-10-16";

/// The dated Anthropic ids the extract owns (see its `_snapshot.source`
/// note). Kept in lockstep with first_party_pricing_extract_contract.rs.
const EXTRACT_IDS: [&str; 3] = [
    "claude-3-7-sonnet-20250219",
    "claude-opus-4-1-20250805",
    "claude-sonnet-4-20250514",
];

#[test]
fn dated_first_party_ids_resolve_through_the_extract_unshadowed() {
    for id in EXTRACT_IDS {
        assert!(
            has_specific_price(id),
            "{id} must resolve at extract rates — the extract is its only \
             pricing lane after the rm-176 2026-10-17 refresh"
        );
        let priced = lookup_price(id);
        assert!(
            priced.input > 0.0 && priced.output > 0.0,
            "{id} resolved without real rates: {priced:?}"
        );
        let source = pricing_source_for(id);
        assert!(
            source.starts_with("models.dev snapshot"),
            "{id} must disclose the extract as its pricing source, got {source:?}"
        );
        // Unshadowed, exact-key-tail semantics: the id must appear as no
        // bundled key's tail — neither a bare first-party row nor any
        // reseller-qualified row (e.g. "openrouter/{id}"). The `"id":`
        // anchor does not match a longer key that merely contains the id
        // as a prefix ("{id}-v2" stays legal).
        let key_tail = format!("{id}\":");
        assert!(
            !BUNDLE.contains(&key_tail),
            "{id} is shadowed by a bundled pricing_snapshot.json row — the \
             extract must stay its only lane (first-party supersession is \
             the refresh script's REFUSED case)"
        );
    }
}

#[test]
fn extract_provenance_is_single_sourced() {
    let needle = "MODELS_DEV_SNAPSHOT_DATE: &str = \"";
    let i = PRICING_RS
        .find(needle)
        .expect("pricing.rs must carry the MODELS_DEV_SNAPSHOT_DATE const");
    let rest = &PRICING_RS[i + needle.len()..];
    let const_date = &rest[..rest.find('"').expect("const date must close its quote")];
    assert_eq!(
        const_date, PINNED_EXTRACT_DATE,
        "MODELS_DEV_SNAPSHOT_DATE moved without updating this pin — \
         re-extract models_dev_snapshot.json in the same change"
    );
    assert!(
        EXTRACT.contains(&format!("\"date\": \"{PINNED_EXTRACT_DATE}\"")),
        "models_dev_snapshot.json's _snapshot.date must equal the pinned \
         const date — a re-extract cannot ship behind a stale const"
    );
}

#[test]
fn refresh_script_carries_the_fail_closed_guard_arms() {
    for arm in [
        // arm 1a: the const is missing entirely
        "REFUSED: MODELS_DEV_SNAPSHOT_DATE declaration not found in ",
        // arm 1b: const and extract dates disagree
        "REFUSED: pricing.rs MODELS_DEV_SNAPSHOT_DATE (",
        // arm 2: live prices a first-party row the extract also serves
        "REFUSED: live LiteLLM now prices first-party row(s) the models.dev ",
        // arm 3: the refresh must leave the frozen extract byte-identical
        "REFUSED: $extract changed during the refresh",
    ] {
        assert!(
            REFRESH_SCRIPT.contains(arm),
            "update-snapshot.sh lost its fail-closed guard arm: {arm:?} \
             (rm-1532 rider — arms are pinned here, not just documented)"
        );
    }
    // The desync arm must compare against the pinned const, not a
    // free-floating string duplicated in the script.
    assert!(
        REFRESH_SCRIPT.contains("MODELS_DEV_SNAPSHOT_DATE"),
        "the desync guard must read MODELS_DEV_SNAPSHOT_DATE from pricing.rs"
    );
}

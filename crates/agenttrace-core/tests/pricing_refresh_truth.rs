//! rm-176 / rm-2348 / rm-1533 discharge pins for the 2026-10-22 pricing
//! snapshot refresh ("pricing-truth discharge + scan-cap fairness" batch,
//! run e90edd6c cycle 1).
//!
//! The 2026-10-08 bundle priced zero rows for the antigravity
//! gemini-4-argon family: the live source carries the model ONLY under
//! vendor prefixes (`gemini/`, `vertex_ai/`, `github_copilot/`), while
//! the antigravity parser's numeric map (ANTIGRAVITY_NUMERIC_MODEL_MAP,
//! identity-only) emits the BARE id `gemini-4-argon` — so mapped spend
//! degraded through the disclosed unknown-model fallback (research
//! 9d7cb7a2: the ladder never synthesizes prefixes, rejected arm (b)).
//!
//! The cure is refresh-side (rm-2348 arm (a)): update-snapshot.sh injects
//! a bare-key copy of the canonical first-party vendor row beside it and
//! discloses every injection in `_snapshot.bare_aliases`. These pins hold
//! the money truth that follows:
//! 1. the bare id prices at exactly the vendor's first-party rates
//!    ($2/$10 per Mtok — the row's 2e-06/1e-05 $/tok, which `Price`
//!    stores after the ingest's ×1e6 conversion — cache_read
//!    $0.1/Mtok published, cache_write ABSENT — the rm-1508 absence
//!    distinction rides the vendor row);
//! 2. the pricing never claims a provider or rate the bundle does not
//!    carry (first-party `gemini`, priced through the bundled snapshot);
//! 3. the disclosure is machine-checkable and the desync gate's pair
//!    (const date == `_snapshot.date`) holds at the test level;
//! 4. the bundle's self-count stays honest (`_snapshot.models` equals
//!    the row count).
//!
//! The lookup arms run inside `support::with_isolated_pricing_env` so
//! the once-per-process catalog deterministically serves the BUNDLED
//! snapshot, never an ambient machine cache (rm-1954 discipline).

mod support;

fn bundled_raw() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/pricing_snapshot.json"
    )))
    .expect("vendored pricing snapshot must parse")
}

#[test]
fn bare_gemini_4_argon_alias_prices_at_first_party_rates() {
    support::with_isolated_pricing_env(|| {
        use agenttrace_core::{has_specific_price, lookup_price, provider_for};
        assert!(
            has_specific_price("gemini-4-argon"),
            "the bare id must be a priced catalog key, not an unknown-model fallback \
             (rm-2348 red-first: the 2026-10-08 bundle carried zero argon rows)"
        );
        let price = lookup_price("gemini-4-argon");
        // Price stores $/Mtok: the vendor row's 2e-06/1e-05 per-token
        // rates ingest as 2.0/10.0 (pricing.rs multiplies by 1e6).
        assert_eq!(price.input, 2.0, "first-party gemini input rate ($2/Mtok)");
        assert_eq!(
            price.output, 10.0,
            "first-party gemini output rate ($10/Mtok)"
        );
        // rm-1508: the vendor row PUBLISHES cache_read ($0.1/Mtok, the
        // row's 1e-07 per-token) and publishes NO cache_write — the
        // alias copy must carry both facts, not a zero-fill and not an
        // invented rate.
        let cr = price
            .cr
            .expect("the gemini/gemini-4-argon row publishes cache_read $0.1/Mtok");
        // The rm-1508 cache_read pin: the row publishes 1e-07 per
        // token, which the ingest's ×1e6 conversion carries to
        // 0.09999999999999999 — pin with tolerance, not ==, so the
        // pin states the rate, not the float's last bit.
        assert!((cr - 0.1).abs() < 1e-12, "cache_read $0.1/Mtok, got {cr}");
        assert!(
            price.cw.is_none(),
            "cache_write is absent upstream and must stay absent (got {:?})",
            price.cw
        );
        assert_eq!(
            provider_for("gemini-4-argon").as_deref(),
            Some("gemini"),
            "the alias copies the first-party vendor row, so the provider \
             attribution must say gemini (never a reseller)"
        );
    });
}

#[test]
fn antigravity_numeric_ids_price_through_the_bare_alias() {
    // The parser map is identity-only (rm-1864 discipline: the table
    // never invents a price), so 1417 — and its 1418/1419 siblings —
    // price exactly when the bare key is a catalog key. This is the
    // pricing half of the composition; parser.rs pins the mapping half.
    support::with_isolated_pricing_env(|| {
        use agenttrace_core::lookup_context_window;
        let price = agenttrace_core::lookup_price("gemini-4-argon");
        assert_eq!(price.input, 2.0, "$2/Mtok (2e-06/tok)");
        assert_eq!(price.output, 10.0, "$10/Mtok (1e-05/tok)");
        assert_eq!(
            lookup_context_window("gemini-4-argon"),
            Some(1_048_576),
            "the vendor row's max_input_tokens must ride the alias copy"
        );
    });
}

#[test]
fn bare_alias_injection_is_disclosed_in_the_snapshot_meta() {
    let raw = bundled_raw();
    let meta = &raw["_snapshot"];
    let aliases = meta["bare_aliases"]
        .as_object()
        .expect("_snapshot.bare_aliases must exist and be an object");
    assert_eq!(
        aliases
            .get("gemini-4-argon")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "gemini/gemini-4-argon",
        "the injected bare key must disclose its canonical vendor source row"
    );
    // Every disclosed alias must be a real priced row in the bundle —
    // a disclosure naming a key the bundle does not carry is a lie.
    for bare in aliases.keys() {
        assert!(
            raw.get(bare).map(|row| row.is_object()).unwrap_or(false),
            "disclosed bare alias {bare:?} must be a priced bundle row"
        );
        let vendor = aliases[bare].as_str().expect("vendor source is a string");
        assert!(
            raw.get(vendor).map(|row| row.is_object()).unwrap_or(false),
            "disclosed vendor source {vendor:?} must itself be a priced bundle row"
        );
    }
}

#[test]
fn snapshot_meta_stays_desync_free_and_self_consistent() {
    let raw = bundled_raw();
    let meta = raw["_snapshot"]
        .as_object()
        .expect("_snapshot must be an object");
    // The drift gate's desync pair (assess A1), pinned at the test
    // level: the const must equal the stamped date.
    assert_eq!(
        meta.get("date").and_then(|d| d.as_str()).unwrap_or(""),
        agenttrace_core::bundled_snapshot_date(),
        "PRICING_SNAPSHOT_DATE must equal _snapshot.date (desync gate)"
    );
    let rows = raw
        .as_object()
        .expect("snapshot is a JSON object")
        .keys()
        .filter(|k| k.as_str() != "_snapshot")
        .count();
    assert_eq!(
        meta.get("models").and_then(|m| m.as_u64()).unwrap_or(0) as usize,
        rows,
        "_snapshot.models must equal the number of priced rows"
    );
}

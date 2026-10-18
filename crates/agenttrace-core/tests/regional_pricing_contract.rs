//! rm-1053 (run f2fccaa9 cycle 3, prioritize ca5a4697): regional-twin
//! pricing truth. The bundled LiteLLM snapshot carries regional twins
//! as distinct SKUs with distinct prices (`us.anthropic.…`,
//! `us-gov.anthropic.…`, `bedrock/us-gov-east-1/…`, `azure/us-gov/…`),
//! but the load-time key collapse keyed every row on
//! `normalize_model(raw)` and deduped by provider priority — one
//! winner per base model — so a session naming the qualified form was
//! billed at whichever regional sibling sorted first (assess 53af6c9e
//! PoC: global billed $3.30 vs the row's own $3.00, us-gov billed
//! $3.30 vs $3.60), and the provider/pricing-source lookups resolved
//! through a DIFFERENT (double-normalized) key than the rate.
//!
//! These pins hold the fixed contract:
//! 1. every region/vendor-qualified id bills at ITS OWN snapshot row
//!    (the assess $ table, per regional prefix family);
//! 2. provider attribution rides the same row the rate came from;
//! 3. base-model resolution is byte-identical to the pre-split
//!    behavior (the collapse still serves plain session ids);
//! 4. a full-catalog sweep: every snapshot key bills at a real
//!    snapshot row — no invented prices, no collateral collapse.

use agenttrace_core::{
    lookup_context_window, lookup_price, pricing_source_for, provider_for, rate_history_rows,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

fn snapshot_json() -> &'static Value {
    static SNAPSHOT: OnceLock<Value> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/pricing_snapshot.json"
        ))
        .expect("bundled pricing_snapshot.json readable from the test");
        serde_json::from_str(&raw).expect("bundled snapshot parses as JSON")
    })
}

/// (input $/M, output $/M, litellm_provider) for a raw snapshot key,
/// computed with the same *1e6 scaling the converter applies.
fn snapshot_row(key: &str) -> (f64, f64, String) {
    let row = snapshot_json()
        .get(key)
        .unwrap_or_else(|| panic!("snapshot must carry key {key:?}"))
        .as_object()
        .expect("snapshot rows are objects");
    let input = row
        .get("input_cost_per_token")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        * 1e6;
    let output = row
        .get("output_cost_per_token")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        * 1e6;
    let provider = row
        .get("litellm_provider")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    (input, output, provider)
}

/// True when the raw LiteLLM key carries a region/vendor qualifier
/// that `normalize_model` strips — a '/'-prefixed deployment id or a
/// dotted regional prefix in front of a vendor marker. These keys must
/// keep their own catalog row.
fn is_qualified_key(key: &str) -> bool {
    if key.contains('/') {
        return true;
    }
    for marker in [".anthropic.", ".google.", ".meta.", ".amazon."] {
        if let Some(idx) = key.find(marker) {
            if idx > 0 {
                return true;
            }
        }
    }
    false
}

/// Is the snapshot row chat-priced (the population the converter
/// ingests)? Non-chat modes and zero-rate rows never reach the
/// catalog.
fn is_chat_priced(key: &str) -> bool {
    let row = match snapshot_json().get(key).and_then(Value::as_object) {
        Some(row) => row,
        None => return false,
    };
    let mode = row.get("mode").and_then(Value::as_str).unwrap_or("chat");
    let input = row
        .get("input_cost_per_token")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let output = row
        .get("output_cost_per_token")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    mode == "chat" && (input > 0.0 || output > 0.0) && input.is_finite() && output.is_finite()
}

#[test]
fn regional_twin_families_bill_at_their_own_rows() {
    // The assess 53af6c9e $ table, one probe per regional prefix
    // family present in the bundled snapshot. Each probe must resolve
    // to ITS OWN snapshot row — pre-fix, every probe collapsed onto
    // the au.anthropic sibling ($3.30/$16.50) regardless of region.
    let claude_sonnet_4_5 = "claude-sonnet-4-5-20250929-v1:0";
    let probes = [
        "us.anthropic.",
        "eu.anthropic.",
        "global.anthropic.",
        "au.anthropic.",
        "jp.anthropic.",
        "us-gov.anthropic.",
    ]
    .iter()
    .map(|prefix| format!("{prefix}{claude_sonnet_4_5}"))
    .chain([
        // apac has no sonnet-4-5 twin; probe its sonnet-4-6 row.
        "apac.anthropic.claude-sonnet-4-6".to_string(),
        // slashed regional ids: bedrock GovCloud (both spellings of
        // the model segment) and the Azure Gov deployment family.
        "bedrock/us-gov-east-1/anthropic.claude-sonnet-4-5-20250929-v1:0".to_string(),
        "bedrock/us-gov-west-1/claude-sonnet-4-5-20250929-v1:0".to_string(),
        "azure/us-gov/gpt-5.1".to_string(),
    ]);

    for probe in probes {
        let price = lookup_price(&probe);
        let (want_input, want_output, want_provider) = snapshot_row(&probe);
        assert_eq!(
            (price.input, price.output),
            (want_input, want_output),
            "regional twin {probe:?} must bill at its own snapshot row \
             ({want_input:.2}/{want_output:.2} $/M), got ({:.2}/{:.2})",
            price.input,
            price.output
        );
        assert_eq!(
            provider_for(&probe).as_deref(),
            Some(want_provider.as_str()),
            "regional twin {probe:?} must attribute to its own row's provider"
        );
    }

    // The hard $ anchors from the assess PoC, so a future snapshot
    // refresh cannot silently blur the regional premium back into the
    // base row without failing a named number:
    let us = lookup_price(&format!("us.anthropic.{claude_sonnet_4_5}"));
    assert!(
        (us.input - 3.30).abs() < 1e-6 && (us.output - 16.50).abs() < 1e-6,
        "us.anthropic row (got {:.4}/{:.4})",
        us.input,
        us.output
    );
    let global = lookup_price(&format!("global.anthropic.{claude_sonnet_4_5}"));
    assert!(
        (global.input - 3.00).abs() < 1e-6 && (global.output - 15.00).abs() < 1e-6,
        "global.anthropic row (pre-fix it over-billed to the us row's 3.30; got {:.4}/{:.4})",
        global.input,
        global.output
    );
    let gov = lookup_price(&format!("us-gov.anthropic.{claude_sonnet_4_5}"));
    assert!(
        (gov.input - 3.60).abs() < 1e-6 && (gov.output - 18.00).abs() < 1e-6,
        "us-gov.anthropic row (pre-fix it under-billed to 3.30; got {:.4}/{:.4})",
        gov.input,
        gov.output
    );
    let bedrock_gov =
        lookup_price("bedrock/us-gov-east-1/anthropic.claude-sonnet-4-5-20250929-v1:0");
    assert!(
        (bedrock_gov.input - 3.60).abs() < 1e-6 && (bedrock_gov.output - 18.00).abs() < 1e-6,
        "bedrock GovCloud row (got {:.4}/{:.4})",
        bedrock_gov.input,
        bedrock_gov.output
    );
}

#[test]
fn regional_provider_attribution_rides_the_rate_row() {
    // rm-1053's core invariant: provider and pricing-source lookups
    // derive from the SAME resolved row the rate came from. Pre-fix,
    // the provider path double-normalized (normalize of a normalize),
    // landing "us.anthropic.claude-sonnet-4-5-20250929-v1:0" on the
    // undated base row's provider while the rate came from the
    // regional winner — by_provider rendered the wrong vendor.
    let probe = "us.anthropic.claude-sonnet-4-5-20250929-v1:0";
    let price = lookup_price(probe);
    let (want_input, _, want_provider) = snapshot_row(probe);
    assert_eq!(price.input, want_input);
    assert_eq!(
        provider_for(probe).as_deref(),
        Some(want_provider.as_str()),
        "provider must ride the row that served the rate (want {want_provider})"
    );
    assert!(
        pricing_source_for(probe).contains("LiteLLM"),
        "a catalog-served regional id must not report a built-in fallback source: {}",
        pricing_source_for(probe)
    );

    // Slashed regional ids attribute to their own vendor too.
    let slashed = "bedrock/us-gov-east-1/anthropic.claude-sonnet-4-5-20250929-v1:0";
    assert_eq!(
        provider_for(slashed).as_deref(),
        Some("bedrock"),
        "slashed bedrock id must attribute to bedrock, not the base winner"
    );
}

#[test]
fn base_model_pricing_is_unchanged_by_the_regional_split() {
    // The collapse keeps serving plain session ids exactly as before:
    // the undated base row for the sonnet-4-5 family is the plain
    // `claude-sonnet-4-5` key ($3.00/$15.00), and unrelated models are
    // untouched. This is the no-collateral-regression half of the
    // contract — regional rows are ADDED, base resolution is not
    // re-won.
    let dated = lookup_price("claude-sonnet-4-5-20250929");
    assert_eq!(
        (dated.input, dated.output),
        (3.0, 15.0),
        "plain session ids keep resolving to the base-model row"
    );
    assert_eq!(
        (lookup_price("gpt-4o").input, lookup_price("gpt-4o").output),
        (2.5, 10.0)
    );
    assert_eq!(
        (
            lookup_price("deepseek-chat").input,
            lookup_price("deepseek-chat").output
        ),
        (0.28, 0.42)
    );
    assert_eq!(
        provider_for("gpt-4o").as_deref(),
        Some("openai"),
        "plain-id provider attribution is unchanged"
    );
}

/// The bundled snapshot's own `max_input_tokens` for a raw key — the
/// value the window lane must serve for that key's sessions.
fn snapshot_window(key: &str) -> Option<u64> {
    let row = snapshot_json().get(key)?.as_object()?;
    let window = row.get("max_input_tokens")?.as_u64()?;
    (window > 0).then_some(window)
}

#[test]
fn window_pins_regional_twins_resolve_to_their_own_rows() {
    // rm-1881: the window lane must resolve through the same rm-1053
    // raw-first ladder the rate lane uses. Pre-fix it was the one
    // resolver still on the plain matcher, so a mixed-case deployment
    // id (assess 532cfddb PoC: "azure/eu/GPT-5.5") billed at its OWN
    // row while the utilization denominator came from the BASE row
    // (1,050,000 labeled window_source "catalog"; the own row pins
    // 922,000) — and the mixed-case dotted regional form missed
    // entirely (None) instead of hitting its own row.
    let probes = [
        // (session id, own snapshot key)
        ("azure/eu/gpt-5.5", "azure/eu/gpt-5.5"), // raw exact
        ("azure/eu/GPT-5.5", "azure/eu/gpt-5.5"), // lowercased-raw ladder arm
        (
            "US.Anthropic.Claude-Opus-4-6-V1",
            "us.anthropic.claude-opus-4-6-v1",
        ), // dotted regional, mixed case
    ];
    for (model, own_key) in probes {
        let own = snapshot_window(own_key)
            .unwrap_or_else(|| panic!("snapshot must carry a window for {own_key:?}"));
        assert_eq!(
            lookup_context_window(model),
            Some(own),
            "{model:?} must resolve to its OWN row's window ({own}), not a twin's"
        );
    }

    // Controls: the plain base id and a variant-path id (suffix-strip
    // onto the base row) are unchanged by the ladder reroute.
    assert_eq!(lookup_context_window("gpt-5.5"), snapshot_window("gpt-5.5"));
    assert_eq!(
        lookup_context_window("gpt-5.5-2026-04"),
        snapshot_window("gpt-5.5")
    );
}

#[test]
fn bundled_catalog_window_sweep_serves_own_rows() {
    // rm-1881 full-catalog sweep, mirroring the rate sweep: every
    // chat-priced qualified key that carries a window must resolve its
    // window to its OWN row's value through the public lane.
    let mut checked = 0usize;
    for key in snapshot_json()
        .as_object()
        .expect("snapshot root is an object")
        .keys()
    {
        if !is_chat_priced(key) || !is_qualified_key(key) {
            continue;
        }
        if let Some(own) = snapshot_window(key) {
            assert_eq!(
                lookup_context_window(key),
                Some(own),
                "window lane must serve {key:?}'s own row"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 100,
        "window sweep must cover the qualified population (checked {checked})"
    );
}

#[test]
fn unresolvable_window_is_none_not_a_twin() {
    // None is the honest answer for an unresolvable id: the
    // diagnostics caller then labels the utilization a fallback
    // estimate. The window lane must never reach for a base-twin
    // window through the back door.
    assert_eq!(lookup_context_window("no/such/model-anywhere"), None);
    assert_eq!(
        lookup_context_window("azure/eu/definitely-not-a-model"),
        None
    );
}

/// rm-1965: newest observed state per rate-history key, in the sweep's
/// cent-rounded units. Table order is date-ascending per key (pinned by
/// rate_basis_contract), so the LAST row per key is its newest state —
/// the rate a current-time (`at: None`) lookup bills.
fn newest_observed_state() -> &'static HashMap<&'static str, (i64, i64)> {
    static NEWEST: OnceLock<HashMap<&'static str, (i64, i64)>> = OnceLock::new();
    NEWEST.get_or_init(|| {
        let mut map = HashMap::new();
        for row in rate_history_rows() {
            map.insert(
                row.key,
                (
                    (row.input * 1e2).round() as i64,
                    (row.output * 1e2).round() as i64,
                ),
            );
        }
        map
    })
}

#[test]
fn bundled_catalog_sweep_bills_every_key_at_a_snapshot_row() {
    // Full-catalog sweep: every chat-priced snapshot key must bill at
    // a REAL snapshot row (its own row when the key is
    // region/vendor-qualified; some same-collapse row otherwise) and,
    // for qualified keys, attribute to its own row's provider. This
    // is the load-bearing no-collapse regression proof — it fails the
    // moment any regional family is silently re-collapsed (pre-fix it
    // failed on global/us-gov/bedrock/azure keys).
    let mut pairs: HashSet<(i64, i64)> = HashSet::new();
    for (key, value) in snapshot_json()
        .as_object()
        .expect("snapshot root is an object")
    {
        if key == "_snapshot" || !is_chat_priced(key) {
            continue;
        }
        let row = value.as_object().expect("rows are objects");
        let snap_input = row
            .get("input_cost_per_token")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            * 1e6;
        let snap_output = row
            .get("output_cost_per_token")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            * 1e6;
        // Bit-level rounding of the converter's *1e6 scaling so the
        // sweep compares the same numbers the catalog stores.
        pairs.insert((
            (snap_input * 1e2).round() as i64,
            (snap_output * 1e2).round() as i64,
        ));
    }
    let mut qualified = 0usize;
    let mut plain = 0usize;
    for (key, value) in snapshot_json()
        .as_object()
        .expect("snapshot root is an object")
    {
        if key == "_snapshot" {
            continue;
        }
        if !is_chat_priced(key) {
            continue;
        }
        let row = value.as_object().expect("rows are objects");
        let snap_input = row
            .get("input_cost_per_token")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            * 1e6;
        let snap_output = row
            .get("output_cost_per_token")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            * 1e6;

        let price = lookup_price(key);
        let billed = (
            (price.input * 1e2).round() as i64,
            (price.output * 1e2).round() as i64,
        );
        // rm-1965: a key with recorded rate history bills its NEWEST
        // OBSERVED state at a current-time lookup — an evidence-backed
        // supersession of the bundled row (models.dev edits rows in
        // place; the snapshot is one state on a timeline), so the rate
        // exists in no snapshot row BY DESIGN. Pin that arm explicitly
        // and keep the no-invented-rates proof for every key without
        // history.
        if let Some(&observed) = newest_observed_state().get(key.as_str()) {
            assert_eq!(
                billed, observed,
                "rm-1965: {key:?} must bill its newest observed state at a current-time lookup"
            );
            if is_qualified_key(key) {
                qualified += 1;
                let want_provider = row
                    .get("litellm_provider")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                assert_eq!(
                    provider_for(key).as_deref(),
                    Some(want_provider),
                    "qualified history key {key:?} still attributes to its own row's provider"
                );
            } else {
                plain += 1;
            }
            continue;
        }
        assert!(
            pairs.contains(&billed),
            "{key:?} bills ({:.2}/{:.2}) at a rate that exists in NO snapshot row",
            price.input,
            price.output
        );
        if is_qualified_key(key) {
            qualified += 1;
            let want = (
                (snap_input * 1e2).round() as i64,
                (snap_output * 1e2).round() as i64,
            );
            assert_eq!(
                billed, want,
                "qualified key {key:?} must bill at its OWN row, not the collapse winner"
            );
            let want_provider = row
                .get("litellm_provider")
                .and_then(Value::as_str)
                .unwrap_or_default();
            assert_eq!(
                provider_for(key).as_deref(),
                Some(want_provider),
                "qualified key {key:?} must attribute to its own provider"
            );
        } else {
            plain += 1;
        }
    }
    assert!(
        qualified > 2_000,
        "expected the regional/vendor-qualified population to survive the conversion, got {qualified}"
    );
    assert!(plain > 100, "plain keys still populate the base collapse");
}

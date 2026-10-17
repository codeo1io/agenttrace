//! rm-1532 guard: the embedded models.dev extract is APPEND-ONLY.
//!
//! The extract (`crates/agenttrace-core/src/models_dev_snapshot.json`) is the
//! second pricing catalog. It serves the dated Anthropic ids the LiteLLM
//! bundle carries NO first-party row for — and since 2026-10-17 it is the
//! ONLY price source for them anywhere upstream: models.dev's api.json
//! returns `cost: null` for exactly these rows and live LiteLLM has zero
//! rows (no first-party, no reseller; research run 939c4681, cycle 3).
//!
//! Contract (stewardship fc289ec9, cycle 3):
//! - a regeneration of the extract from live data would silently ZERO these
//!   rates (cost:null upstream) — so the extract is pinned, never
//!   regenerated; edits may only APPEND rows;
//! - every row must carry nonzero input/output/cache rates — a zero or
//!   missing rate is the silent-zero-pricing failure mode this guards;
//! - the current row set is pinned by id: removing or zeroing a row
//!   requires editing this contract deliberately, with the roadmap delta
//!   that explains why (the speed bump is the point).

use serde_json::Value;

const EXTRACT_JSON: &str = include_str!("../src/models_dev_snapshot.json");

/// The dated first-party Anthropic ids this extract pinned on 2026-10-16 —
/// orphaned upstream (models.dev cost:null, LiteLLM zero rows) on 2026-10-17.
const PINNED_ROWS: &[&str] = &[
    "anthropic/claude-3-7-sonnet-20250219",
    "anthropic/claude-sonnet-4-20250514",
    "anthropic/claude-opus-4-1-20250805",
];

#[test]
fn every_extract_row_carries_nonzero_rates() {
    let doc: Value = serde_json::from_str(EXTRACT_JSON).expect("models_dev_snapshot.json parses");
    let map = doc.as_object().expect("extract is a JSON object");
    assert!(
        map.contains_key("_snapshot"),
        "extract must keep its _snapshot provenance header"
    );

    let mut row_count = 0usize;
    for (key, row) in map {
        if key == "_snapshot" {
            continue;
        }
        row_count += 1;
        let row = row
            .as_object()
            .unwrap_or_else(|| panic!("row {key} must be a JSON object, got: {row}"));
        let provider = row
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("row {key} must carry a provider string"));
        assert!(
            key.starts_with(&format!("{provider}/")),
            "row {key}: provider prefix must match the key"
        );
        for field in ["input", "output", "cache_read", "cache_write"] {
            let rate = row
                .get(field)
                .and_then(Value::as_f64)
                .unwrap_or_else(|| panic!("row {key} must carry a numeric {field} rate"));
            assert!(
                rate > 0.0,
                "row {key}: {field} rate must be nonzero — a zero/null rate \
                 is the silent-zero-pricing failure (models.dev serves \
                 cost:null for these ids since 2026-10-17; this extract is \
                 their only price source)"
            );
        }
    }
    assert!(row_count >= PINNED_ROWS.len(), "extract lost rows");
}

#[test]
fn pinned_orphaned_rows_remain_present() {
    let doc: Value = serde_json::from_str(EXTRACT_JSON).expect("models_dev_snapshot.json parses");
    let map = doc.as_object().expect("extract is a JSON object");
    for id in PINNED_ROWS {
        assert!(
            map.contains_key(*id),
            "pinned row {id} is gone from the extract — since 2026-10-17 \
             models.dev (cost:null) and LiteLLM (zero rows) no longer price \
             it, so removing the row silently zeroes real session costs. If \
             this removal is deliberate, update this contract and the rm-1532 \
             roadmap row in the same change"
        );
    }
}

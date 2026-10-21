//! rm-949 contract: by_model buckets distribute the session's billed
//! total exactly. The per-model attribution costs are priced
//! independently of the session's billed `cost_estimated` (per-block
//! catalog pricing with per-add round4, recorded-cost seeds, credit
//! floors), so summing them raw drifted away from the billed total and
//! broke `sum(by_model.cost) == total_cost` for machine consumers
//! summing agenttrace.overview.v1 buckets.
//!
//! Red-first fixture multi.json is the PoC corpus from the cycle-3
//! adversarial assessment, preserved verbatim (pre-fix: billed
//! total_cost 0.0002 vs by_model Σ 0.0004 — fable 0.0003 + opus
//! 0.0001). Post-fix the buckets hold proportional shares of the
//! billed total and the sum is exact by construction.

use agenttrace_core::{compute_overview, parse_file};
use std::fs;
use std::path::PathBuf;

const MULTI: &str = include_str!("fixtures/rm-949-by-model/multi.jsonl");

// Each test runs on its own parallel harness thread and materializes the
// fixture at a distinct temp path: a shared pid-keyed path let one test's
// fs::write/fs::remove_file truncate or delete the file under another
// test's parse_file ("empty session" flake, caught in targeted validation).
fn session(tag: &str) -> agenttrace_core::Session {
    let path: PathBuf =
        std::env::temp_dir().join(format!("at-rm949-{}-{tag}.jsonl", std::process::id()));
    fs::write(&path, MULTI).expect("write fixture");
    let parsed = parse_file(&path).expect("session parses");
    let _ = fs::remove_file(&path);
    parsed
}

#[test]
fn by_model_buckets_sum_to_the_billed_total_exactly() {
    let overview = compute_overview(&[session("sum")]);
    let sum: f64 = overview.by_model.values().map(|bucket| bucket.cost).sum();
    assert!(
        (sum - overview.total_cost).abs() < 1e-9,
        "by_model Σ {sum:.6} must equal billed total_cost {:.6}",
        overview.total_cost
    );
}

#[test]
fn by_model_buckets_hold_proportional_shares_of_the_billed_total() {
    let overview = compute_overview(&[session("proportional")]);
    // Attributed proportions fable 0.0003 / opus 0.0001 of billed 0.0002.
    let fable = overview
        .by_model
        .get("claude-fable-5-1")
        .expect("fable bucket exists");
    let opus = overview
        .by_model
        .get("claude-opus-5-5")
        .expect("opus bucket exists");
    // The apportionment works in 4-dp units (what every render lane
    // prints): 2 units split by weight 0.75/0.25 → exact 1.5/0.5, floors
    // 1/0, the leftover unit breaks the 0.5/0.5 tie toward the larger
    // weight → fable 0.0002 / opus 0.0000. Proportionality holds within
    // one quantum per bucket, the sum is exact, the order survives.
    let quantum = 1.0e-4;
    for (name, bucket) in &overview.by_model {
        if name == "multiple" {
            continue;
        }
        let units = bucket.cost / quantum;
        assert!(
            (units - units.round()).abs() < 1e-9,
            "bucket {name} cost {} is not a 4-dp quantum",
            bucket.cost
        );
    }
    // With 2 units and weights 0.75/0.25 the exact split is 1.5/0.5 —
    // floors 1/0, one leftover unit goes to the larger remainder. Which
    // bucket wins that unit is decided at f64 noise level (both remainders
    // sit at ~0.5), so the test pins the CONTRACT, not the coin flip:
    // quantum-multiple buckets, exact sum, order never inverted, and each
    // bucket within one quantum of its proportional share (1.5 / 0.5 units).
    assert!((fable.cost - 0.00015).abs() <= quantum + 1e-12);
    assert!((opus.cost - 0.00005).abs() <= quantum + 1e-12);
    assert!(
        fable.cost >= opus.cost,
        "attribution order inverted: fable {} < opus {}",
        fable.cost,
        opus.cost
    );
    assert_eq!(fable.sessions, 1);
    assert_eq!(opus.sessions, 1);
}

#[test]
fn headline_multiple_bucket_keeps_session_count_without_cost() {
    let overview = compute_overview(&[session("headline")]);
    let headline = overview
        .by_model
        .get("multiple")
        .expect("multi-model sessions collapse under the headline bucket");
    assert_eq!(headline.sessions, 1);
    assert!((headline.cost - 0.0).abs() < f64::EPSILON);
}

#[test]
fn overflow_band_buckets_sum_to_the_billed_total_not_i64_saturation() {
    // rm-2249 red-first: cost_estimated * 1e4 saturates the `as i64`
    // cast above ~9.2e14 — a hostile recorded cost (1.7e308) used to
    // collapse every by_model bucket onto the saturated unit lattice
    // while the billed total rendered at full magnitude. The f64
    // fallback must keep the buckets at full magnitude and sum to the
    // billed total (relative exactness).
    let mut s = session("overflow-band");
    s.metrics.cost_estimated = 1.7e308;
    s.metrics.model_attribution.clear();
    for (model, weight) in [("model-a", 1.2e308), ("model-b", 0.5e308)] {
        s.metrics.model_attribution.insert(
            model.to_string(),
            agenttrace_core::ModelAttribution {
                input_tokens: 0,
                output_tokens: 0,
                cache_w: 0,
                cache_r: 0,
                cost_usd: weight,
            },
        );
    }
    let overview = compute_overview(&[s]);
    assert!(
        overview
            .by_model
            .values()
            .all(|b| b.cost == 0.0 || (b.cost.is_finite() && b.cost > 1.0e300)),
        "weight-carrying buckets must hold the full finite magnitude (zero-weight
         marker buckets are fine), got {:?}",
        overview.by_model
    );
    let sum: f64 = overview.by_model.values().map(|b| b.cost).sum();
    assert!(
        sum.is_finite() && (sum - overview.total_cost).abs() <= overview.total_cost.abs() * 1e-12,
        "buckets must sum to the billed total {} (±1e-12 relative), got {sum:e}",
        overview.total_cost
    );
}

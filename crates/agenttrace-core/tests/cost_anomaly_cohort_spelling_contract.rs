//! rm-1900 contract: anomaly-cohort pooling through pricing
//! normalization, over the real intake chain (parse_file → model
//! attribution → catalog pricing → predict_cost_anomaly).
//!
//! The cost-anomaly baseline pools same-model history (rm-1438) while
//! the pricing layer folds dated, `/`-pathed and provider-qualified
//! spellings of one model onto a single rate row (pricing.rs
//! `normalize_model`). Pre-fix the cohort keyed on the RAW
//! `model_used` string, so a mixed-spelling corpus — the common
//! dated/undated and bedrock-alias mix — silently skipped the check
//! with "no same-model cost history" although every session priced
//! identically (adversarial-assessment F2 PoC: the undated arm
//! skipped while the identical-spelling control fired 3.0x). The
//! contract pins three things:
//!
//! 1. A mixed-spelling corpus pools into ONE baseline and the check
//!    evaluates (fires) instead of skipping — and the dated and
//!    undated spellings price to the same dollar total, pinning the
//!    pricing-fold premise itself.
//! 2. The identical-spelling control still fires at exactly the same
//!    ratio: normalization is a no-op when spellings already agree.
//! 3. A genuinely different model's history never enters the
//!    baseline: the fold collapses spellings of ONE model, never two.

use agenttrace_core::{parse_file, predict_cost_anomaly, Session};
use std::fs;
use std::path::{Path, PathBuf};

/// One generic-lane session file: a user row plus `turns` assistant
/// rows, all attributed to `model`, each reporting `input_tokens`.
/// The session prices at the catalog row the pricing ladder resolves
/// for `model` (input-only, so the total is exactly
/// turns × input_tokens × input_rate / 1e6).
fn session_file(tag: &str, model: &str, turns: usize, input_tokens: i64) -> PathBuf {
    let mut raw = String::from("[{\"role\":\"user\",\"content\":\"go\"}");
    for _ in 0..turns {
        raw.push_str(&format!(
            ",{{\"role\":\"assistant\",\"content\":\"ok\",\"model_used\":\"{model}\",\"usage\":{{\"input_tokens\":{input_tokens},\"output_tokens\":0}}}}"
        ));
    }
    raw.push(']');
    // Fleet precedent (rm-616 review F4): pid-qualify scratch paths so
    // overlapping test invocations never race on the same files.
    let path = std::env::temp_dir().join(format!("at-rm1900-{}-{tag}.json", std::process::id()));
    fs::write(&path, raw).expect("write fixture");
    path
}

fn parsed(path: &Path) -> Session {
    let session = parse_file(path).expect("session parses");
    let _ = fs::remove_file(path);
    session
}

/// History at $0.30/turn, current at $0.912/turn → ratio 3.04 (fires
/// critical, renders "3.0x"). Three history sessions keep the basis
/// off the thin-history disclosure. `prefix` namespaces the scratch
/// files per TEST (rm-616 review F4 fleet precedent: pid-qualified
/// paths alone still race across this binary's own parallel tests).
fn corpus(prefix: &str, history_model: &str, current_model: &str) -> Vec<Session> {
    let mut sessions: Vec<Session> = ["a", "b", "c"]
        .iter()
        .map(|name| {
            parsed(&session_file(
                &format!("{prefix}-{name}"),
                history_model,
                10,
                100_000,
            ))
        })
        .collect();
    sessions.push(parsed(&session_file(
        &format!("{prefix}-current"),
        current_model,
        10,
        304_000,
    )));
    sessions
}

#[test]
fn mixed_spelling_history_pools_instead_of_skipping() {
    let sessions = corpus("mixed", "claude-sonnet-4-5-20250929", "claude-sonnet-4-5");
    let current = sessions.last().expect("current session");
    let history = &sessions[..sessions.len() - 1];
    // The premise: both spellings resolve to the same catalog row, so
    // the history is comparable exactly because pricing folds them.
    let undated_reference = parsed(&session_file("mixed-ref", "claude-sonnet-4-5", 10, 100_000));
    assert!(
        history.iter().all(|s| (s.metrics.cost_estimated
            - undated_reference.metrics.cost_estimated)
            .abs()
            < 1e-9),
        "dated history must price at the same total as the undated spelling"
    );
    let alert = predict_cost_anomaly(history, current);
    assert!(
        alert.triggered,
        "mixed-spelling corpus must evaluate, not skip: {}",
        alert.message
    );
    assert_eq!(alert.level, "critical");
    assert!((alert.ratio - 3.04).abs() < 1e-6, "ratio {}", alert.ratio);
    assert_eq!(alert.basis, "'claude-sonnet-4-5' sessions");
    assert!(
        !alert.message.to_lowercase().contains("skip"),
        "skip disclosure must be gone: {}",
        alert.message
    );
}

#[test]
fn identical_spelling_control_fires_unchanged() {
    let sessions = corpus("ctl", "claude-sonnet-4-5", "claude-sonnet-4-5");
    let current = sessions.last().expect("current session");
    let history = &sessions[..sessions.len() - 1];
    let control = predict_cost_anomaly(history, current);
    assert!(control.triggered);
    assert_eq!(control.level, "critical");
    assert!((control.ratio - 3.04).abs() < 1e-6);

    // The mixed corpus must land on the SAME pooled baseline the
    // control sees: normalization joins cohorts, never moves them.
    let mixed = corpus(
        "ctl-mixed",
        "claude-sonnet-4-5-20250929",
        "claude-sonnet-4-5",
    );
    let mixed_alert = predict_cost_anomaly(&mixed[..mixed.len() - 1], mixed.last().unwrap());
    assert!((mixed_alert.baseline - control.baseline).abs() < 1e-9);
    assert!((mixed_alert.ratio - control.ratio).abs() < 1e-9);
}

#[test]
fn different_model_history_still_skips_on_same_model_basis() {
    let sessions = corpus("distinct", "claude-opus-4-6", "claude-sonnet-4-5");
    let current = sessions.last().expect("current session");
    let history = &sessions[..sessions.len() - 1];
    assert!(
        history.iter().all(|s| s.metrics.cost_estimated > 0.0),
        "different-model history must still be priced cohort candidates"
    );
    let alert = predict_cost_anomaly(history, current);
    assert!(!alert.triggered);
    assert_eq!(alert.level, "info");
    assert!(
        alert.message.to_lowercase().contains("same-model"),
        "distinct-model history must keep the same-model skip: {}",
        alert.message
    );
}

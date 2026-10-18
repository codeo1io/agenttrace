//! rm-1435 + rm-1975 contracts: the cline CLI persisted store and the
//! started/finished pair fold.
//!
//! rm-1435: the stable cline CLI persisted store (cli-v3.0.73+,
//! 2026-10-17) lives at `~/.cline/data/sessions/<id>/` with a
//! `session.json` v1 manifest and a `<id>.messages.json` document whose
//! assistant messages carry sibling `metrics` objects. Fixtures derive
//! from the upstream SDK fixtures (sdk/packages/core/fixtures/session/).
//!
//! rm-1975: upstream `combineApiRequests.ts:66-77` removes every
//! `api_req_finished` row and override-merges it into its immediately
//! preceding `api_req_started` row (finished wins key-by-key), and
//! `getApiMetrics.ts:49-53` sums only the merged vocabulary. These tests
//! pin that fold so a pair is counted once, an orphan finished row counts
//! nothing, and the plain VS Code shape is unchanged.

use agenttrace_core::{parse_file, parse_raw_session};
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(rel)
}

/// rm-1435: the session dir (manifest + messages file) parses as one
/// session with the manifest's model and the sibling-metrics usage.
#[test]
fn cli_store_dir_session_parses_with_manifest_model() {
    let session = parse_file(&fixture("cline-cli-store")).expect("cli store dir parses");
    assert_eq!(session.metrics.source_tool, "cline");
    assert_eq!(session.metrics.model_used, "claude-opus-4.8");
    // Recorded by identity: the sibling metrics are one measurement.
    assert_eq!(session.metrics.tokens_input, 34334);
    assert_eq!(session.metrics.tokens_output, 741);
    assert_eq!(session.metrics.tokens_cache_r, 0);
    assert_eq!(session.metrics.tokens_cache_w, 0);
}

/// rm-1435: a store document handed to the generic parser without its
/// directory is claimed by the conservative store probe (model from the
/// last `modelInfo.id`, identity basis).
#[test]
fn cli_store_standalone_document_parses_via_probe() {
    let raw = std::fs::read_to_string(
        fixture("cline-cli-store/1736805123-a1b2c3d4.messages.json"),
    )
    .expect("store fixture readable");
    let session =
        parse_raw_session(&raw, "1736805123-a1b2c3d4.messages.json", "1736805123-a1b2c3d4")
            .expect("store document parses via the probe");
    assert_eq!(session.metrics.source_tool, "cline");
    assert_eq!(session.metrics.model_used, "anthropic/claude-opus-4.8-fast");
    assert_eq!(session.metrics.tokens_input, 34334);
    assert_eq!(session.metrics.tokens_output, 741);
}

/// rm-1975: an adjacent started/finished pair with metrics JSON in both
/// texts folds to ONE measurement — finished wins key-by-key — instead of
/// being summed additively (which would double-count the request).
#[test]
fn say_pair_folds_started_finished_upstream_exact() {
    let session = parse_file(&fixture("cline-vscode-pair/ui_messages.json"))
        .expect("pair fixture parses");
    assert_eq!(session.metrics.source_tool, "cline");
    assert_eq!(session.metrics.tokens_input, 100, "pair folded once, not summed");
    assert_eq!(session.metrics.tokens_output, 200, "pair folded once, not summed");
}

/// rm-1975 regression: the plain VS Code shape (started carries no metrics
/// JSON; finished does) still counts exactly once, with the finished row's
/// values.
#[test]
fn vs_code_started_without_metrics_still_single_counted() {
    let session = parse_file(&fixture("cline-vscode-single/ui_messages.json"))
        .expect("single fixture parses");
    assert_eq!(session.metrics.tokens_input, 50);
    assert_eq!(session.metrics.tokens_output, 60);
}

/// rm-1975 parity: an `api_req_finished` row with no preceding started row
/// folds nothing (upstream `getApiMetrics` sums only the merged started
/// vocabulary), so it contributes no usage.
#[test]
fn orphan_finished_row_contributes_nothing() {
    let session = parse_file(&fixture("cline-vscode-orphan/ui_messages.json"))
        .expect("orphan fixture parses");
    assert_eq!(session.metrics.tokens_input, 0);
    assert_eq!(session.metrics.tokens_output, 0);
}

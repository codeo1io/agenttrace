//! rm-1426 journal-parse contract: codex sessions on locally-defined
//! slugs (provider-local `model_catalog_json` / private-proxy class)
//! disclose the fabricated-rate dollars they are silently priced at today
//! (`locally_defined_model_pricing:{slug}` occurrence +
//! `locally_defined_priced_cents`), mirroring rm-1052's discipline; a
//! priced model keeps zero-delta output.
//!
//! rm-1954 composition (wall, post-base): every arm below classifies
//! through the pricing catalog (`is_locally_defined` judges the ACTIVE
//! origin's raw keys; the controls price through real catalog rows), so a
//! machine pricing cache at the default location would flip them. Each
//! body runs inside `support::with_isolated_pricing_env` — deterministic
//! battery truth, never host state.

mod support;

use agenttrace_core::{
    build_doctor_report, cost_audit, data_health, parse_raw_session, render_doctor_report,
    session_cost_audit,
};

const CODEX_META: &str = r#"{"timestamp":"2026-10-14T08:00:00Z","type":"session_meta","payload":{"id":"rm1426","cwd":"/tmp/rm1426","originator":"codex_cli_rs","cli_version":"0.162.1"}}"#;

/// Real codex rollouts stamp turn_context as a TOP-LEVEL line
/// (host-corpus ground truth, ~/.codex/sessions rollout journals).
fn codex_journal(model: &str) -> String {
    [
        CODEX_META,
        &format!(
            r#"{{"timestamp":"2026-10-14T08:00:01Z","type":"turn_context","payload":{{"turn_id":"rm1426-turn","cwd":"/tmp/rm1426","model":"{model}"}}}}"#
        ),
        r#"{"timestamp":"2026-10-14T08:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000000,"cached_input_tokens":0,"output_tokens":1000000}}}}"#,
    ]
    .join("\n")
}

fn counter(s: &agenttrace_core::Session, key: &str) -> Option<usize> {
    s.metrics.disclosure_counters.get(key).copied()
}

#[test]
fn foreign_carrier_priced_session_discloses() {
    support::with_isolated_pricing_env(foreign_carrier_priced_session_discloses_body);
}

fn foreign_carrier_priced_session_discloses_body() {
    let s = parse_raw_session(
        "rm1426-glm.jsonl",
        "rm1426-glm.jsonl",
        &codex_journal("glm-5.2"),
    )
    .expect("codex journal parses");
    assert_eq!(
        counter(&s, "locally_defined_model_pricing:glm-5.2"),
        Some(1),
        "one usage-bearing event on the locally-defined slug; got {:?}",
        s.metrics.disclosure_counters
    );
    let cents =
        counter(&s, "locally_defined_priced_cents").expect("fabricated-rate cents counter minted");
    assert!(cents > 0, "fabricated dollars disclosed: {cents} cents");
}

#[test]
fn builtin_default_priced_session_discloses_exact_dollars() {
    support::with_isolated_pricing_env(builtin_default_priced_session_discloses_exact_dollars_body);
}

fn builtin_default_priced_session_discloses_exact_dollars_body() {
    let s = parse_raw_session(
        "rm1426-default.jsonl",
        "rm1426-default.jsonl",
        &codex_journal("glm-9.9-not-a-sku"),
    )
    .expect("codex journal parses");
    // 1M uncached input at $3.00/Mtok + 1M output at $15.00/Mtok =
    // exactly $18.00 of builtin-default dollars.
    assert_eq!(
        counter(&s, "locally_defined_priced_cents"),
        Some(1800),
        "exact default-fallback dollars; got {:?}",
        s.metrics.disclosure_counters
    );
}

#[test]
fn priced_model_sessions_stay_zero_delta() {
    support::with_isolated_pricing_env(priced_model_sessions_stay_zero_delta_body);
}

fn priced_model_sessions_stay_zero_delta_body() {
    let s = parse_raw_session(
        "rm1426-control.jsonl",
        "rm1426-control.jsonl",
        &codex_journal("claude-sonnet-4-5"),
    )
    .expect("control journal parses");
    assert!(
        s.metrics
            .disclosure_counters
            .keys()
            .all(|k| !k.starts_with("locally_defined_model_pricing:")
                && k != "locally_defined_priced_cents"),
        "no locally-defined disclosure for a fully priced model: {:?}",
        s.metrics.disclosure_counters
    );
}

/// rm-1426 CU-B: the guard families agree with the disclosure — arm (b)
/// (`glm-5.2` name-collides with collapsed `zai/glm-5.2` carrier rows, so
/// `has_specific_price` alone says "specific") must classify as fallback
/// everywhere, while a genuinely priced model keeps `catalog_estimate`.
#[test]
fn guard_families_count_locally_defined_as_fallback() {
    support::with_isolated_pricing_env(guard_families_count_locally_defined_as_fallback_body);
}

fn guard_families_count_locally_defined_as_fallback_body() {
    let glm = parse_raw_session(
        "rm1426-guards-glm.jsonl",
        "rm1426-guards-glm.jsonl",
        &codex_journal("glm-5.2"),
    )
    .expect("glm journal parses");
    let priced = parse_raw_session(
        "rm1426-guards-sonnet.jsonl",
        "rm1426-guards-sonnet.jsonl",
        &codex_journal("claude-sonnet-4-5"),
    )
    .expect("sonnet journal parses");

    let health = data_health(&[glm.clone(), priced.clone()], 2, 0);
    assert_eq!(
        health.fallback_pricing, 1,
        "insights counts the locally-defined session as fallback pricing"
    );

    let audit = cost_audit(&[glm.clone(), priced.clone()]);
    assert_eq!(
        audit.pricing_coverage.fallback_priced_sessions, 1,
        "governance coverage: locally-defined is fallback, not specific"
    );
    assert_eq!(
        audit.pricing_coverage.priced_sessions, 1,
        "the genuinely priced model keeps its specific classification"
    );

    assert_eq!(
        session_cost_audit(&glm).pricing_status,
        "fallback_estimate",
        "arm (b) no longer masquerades as catalog_estimate"
    );
    assert_eq!(
        session_cost_audit(&priced).pricing_status,
        "catalog_estimate",
        "a named row is still an exact catalog match"
    );
}

/// rm-1426 CU-A end-to-end: doctor's disclosure fold renders the counter
/// for a scanned corpus (the rm-1052 `pi-present-zero` doctor precedent
/// shape).
#[test]
fn doctor_report_renders_the_disclosure() {
    support::with_isolated_pricing_env(doctor_report_renders_the_disclosure_body);
}

fn doctor_report_renders_the_disclosure_body() {
    let dir = std::env::temp_dir().join(format!("at-rm1426-doctor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    std::fs::write(dir.join("rm1426-doc.jsonl"), codex_journal("glm-5.2")).expect("write fixture");
    let report = build_doctor_report(Some(&dir), false);
    assert_eq!(
        report
            .disclosures
            .get("locally_defined_model_pricing:glm-5.2"),
        Some(&1),
        "doctor disclosure fold carries the counter: {:?}",
        report.disclosures
    );
    let text = render_doctor_report(Some(&dir), false, "text").expect("doctor text");
    assert!(
        text.contains("locally_defined_model_pricing:glm-5.2"),
        "doctor renders the disclosure counter:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

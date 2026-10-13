//! rm-1426 journal-parse contract: codex sessions on locally-defined
//! slugs (provider-local `model_catalog_json` / private-proxy class)
//! disclose the fabricated-rate dollars they are silently priced at today
//! (`locally_defined_model_pricing:{slug}` occurrence +
//! `locally_defined_priced_cents`), mirroring rm-1052's discipline; a
//! priced model keeps zero-delta output.

use agenttrace_core::parse_raw_session;

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

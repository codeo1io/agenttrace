//! rm-1907 (cycle 1, run 65afcf57): the antigravity numeric-ID
//! resolution ladder, asserted against a checked-in fixture corpus
//! spanning BOTH ecosystem-default roots from ccusage #1897
//! (`~/.gemini/antigravity` and `~/.gemini/antigravity-ide`).
//!
//! Before rm-1907 the model carrier read was string-only, so a JSON
//! NUMBER `modelConfigId` (e.g. 1277, 1417) folded to `None` and the
//! generation priced under no identity at all — silent. Placeholder
//! carriers (`MODEL_PLACEHOLDER_M418`) passed through verbatim, so
//! ccusage #1904's m418/m419 buckets never reached a catalog row.
//! The tests pin: numeric carriers are accepted, the ladder resolves
//! both families (m417/m418/m419 bucket onto the single gemini-4-argon
//! family row), an unmapped id stays verbatim with a
//! `no_catalog_row` disclosure, and BOTH roots are discovered by
//! default.

use agenttrace_core::{find_session_files, parse_file};
use std::fs;
use std::path::{Path, PathBuf};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/antigravity/numeric-ids")
}

/// Plants the corpus into a synthesized HOME, exercising the REAL
/// discovery roots: `~/.gemini/antigravity/conversations/` (the ag-*
/// trajectory sidecars) and `~/.gemini/antigravity-ide/conversations/`
/// (the standalone jsonl rows).
fn plant_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("ag-numeric-ids-{}-{tag}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let root_a = home
        .join(".gemini")
        .join("antigravity")
        .join("conversations");
    let root_b = home
        .join(".gemini")
        .join("antigravity-ide")
        .join("conversations");
    fs::create_dir_all(&root_a).expect("plant root A");
    fs::create_dir_all(&root_b).expect("expect root B");
    for entry in fs::read_dir(corpus()).expect("corpus readable") {
        let entry = entry.expect("corpus entry");
        let name = entry.file_name().to_string_lossy().to_string();
        let dst = if name.starts_with("ag-") {
            root_a.join(format!("a10c0a11-0000-4000-8000-00{name}"))
        } else {
            root_b.join(format!("b10c0a11-0000-4000-8000-00{name}"))
        };
        fs::copy(entry.path(), dst).expect("plant fixture");
    }
    home
}

fn session_for(name: &str) -> agenttrace_core::Session {
    let path = corpus().join(name);
    parse_file(&path).unwrap_or_else(|err| panic!("{name} parses: {err}"))
}

#[test]
fn both_ecosystem_roots_are_discovered_by_default() {
    let home = plant_home("discovery");
    let files = find_session_files(Some(&home));
    let found_roots = files
        .iter()
        .filter(|path| {
            let text = path.to_string_lossy();
            text.contains("antigravity") && text.contains("conversations")
        })
        .count();
    assert_eq!(
        found_roots, 5,
        "all five corpus sessions across BOTH roots are discovered"
    );
    assert!(
        files.iter().any(|p| p
            .to_string_lossy()
            .contains(".gemini/antigravity/conversations")),
        "root A (~/.gemini/antigravity) is scanned"
    );
    assert!(
        files.iter().any(|p| p
            .to_string_lossy()
            .contains(".gemini/antigravity-ide/conversations")),
        "root B (~/.gemini/antigravity-ide) is scanned"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn numeric_config_id_1417_resolves_to_named_model() {
    // ccusage's live table: 1417 = gemini-4-argon. Pre-rm-1907 the
    // JSON number folded to None and the session priced under
    // "default" with no disclosure.
    let session = session_for("ag-config-1417.trajectory.json");
    assert_eq!(session.metrics.tokens_input, 1000, "usage folds");
    assert_eq!(
        session.metrics.model_used, "gemini-4-argon",
        "numeric carrier resolves through the ladder"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("antigravity_model:id_resolved:1417"),
        Some(&1),
        "the resolution itself is disclosed, never silent"
    );
}

#[test]
fn numeric_config_id_246_resolves_to_a_priced_catalog_row() {
    let session = session_for("ag-config-246.trajectory.json");
    assert_eq!(
        session.metrics.model_used, "gemini-2.5-pro",
        "246 maps through the numeric table"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("antigravity_model:no_catalog_row:gemini-2.5-pro"),
        None,
        "a resolved id with a catalog row raises no gap counter"
    );
}

#[test]
fn unmapped_numeric_id_1277_stays_verbatim_and_disclosed() {
    // 1277 is one of ccusage #1897's unmapped ids: no table entry, no
    // catalog row. It must stay VERBATIM (cost_audit's coverage channel
    // keeps reporting it) and the ladder gap must be named.
    let session = session_for("ag-config-1277.trajectory.json");
    assert_eq!(
        session.metrics.model_used, "1277",
        "an unmapped numeric id is never silently dropped or coerced"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("antigravity_model:no_catalog_row:1277"),
        Some(&1),
        "the not-resolvable-to-a-catalog-row case is disclosed in json"
    );
}

#[test]
fn placeholder_family_buckets_onto_the_named_family_row() {
    // ccusage #1904: m418 / m419 name gemini-4-argon at medium / low
    // reasoning effort — one pricing family, one bucketed fallback.
    let session = session_for("ide-m418-m419.jsonl");
    assert_eq!(session.metrics.tokens_input, 900, "both generations fold");
    assert_eq!(
        session.metrics.model_used, "gemini-4-argon",
        "last-wins after both placeholders resolve onto the family row"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("antigravity_model:id_resolved:model_placeholder_m418"),
        Some(&1)
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("antigravity_model:id_resolved:model_placeholder_m419"),
        Some(&1)
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("antigravity_model:multi_model_last_wins"),
        Some(&2),
        "the multi-id merge stays disclosed (never silent last-wins)"
    );
}

#[test]
fn m16_placeholder_resolves_through_the_table() {
    let session = session_for("ide-m16.jsonl");
    assert_eq!(
        session.metrics.model_used, "gemini-3.1-pro",
        "m16 maps through the placeholder table (mirrors ccusage)"
    );
}

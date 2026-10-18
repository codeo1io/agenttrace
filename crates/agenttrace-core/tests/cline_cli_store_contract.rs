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
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
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
    let raw = std::fs::read_to_string(fixture("cline-cli-store/1736805123-a1b2c3d4.messages.json"))
        .expect("store fixture readable");
    let session = parse_raw_session(
        "1736805123-a1b2c3d4.messages.json",
        "1736805123-a1b2c3d4",
        &raw,
    )
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
    let session =
        parse_file(&fixture("cline-vscode-pair/ui_messages.json")).expect("pair fixture parses");
    assert_eq!(session.metrics.source_tool, "cline");
    assert_eq!(
        session.metrics.tokens_input, 100,
        "pair folded once, not summed"
    );
    assert_eq!(
        session.metrics.tokens_output, 200,
        "pair folded once, not summed"
    );
}

/// rm-1975 leg (rm-1885 vocabulary gate stays the admission rule): a
/// say row of a NON-usage type carrying token-shaped JSON in its text
/// is an ECHO — content, never a usage carrier. Only the say types
/// upstream meters carry usage (api_req_started/api_req_finished/
/// deleted_api_reqs/subagent_usage); anything else must not run the
/// carrier fold.
#[test]
fn non_usage_say_rows_are_content_not_carriers() {
    // A say:"text" row whose text is exactly a metrics JSON object.
    let document = r#"{"messages":[{"ts":1736805123000,"type":"say","say":"text","text":"{\"tokensIn\":10,\"tokensOut\":20,\"cacheReads\":30,\"cacheWrites\":5}"}]}"#;
    let dir = std::env::temp_dir().join(format!("at-rm1975-vocab-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("ui_messages.json"), document).expect("fixture write");
    let session = match parse_file(&dir) {
        Ok(session) => session,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&dir);
            panic!("echo document must parse as a conversation: {error}");
        }
    };
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("cline_metrics_carriers_parsed"),
        "an echo say row must not run the carrier fold"
    );
    // Estimation-proof fold invariant: the cache classes only ever
    // come from a folded carrier (text estimation writes input/output
    // only), so the echo's cacheReads/cacheWrites must land nowhere.
    assert_eq!(session.metrics.tokens_cache_r, 0);
    assert_eq!(session.metrics.tokens_cache_w, 0);
    assert_eq!(
        session.metrics.provenance.tokens, "estimated_from_text",
        "the echo row is content: it estimates, never folds usage"
    );
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
/// folds NO usage (upstream `getApiMetrics.ts:49-53` sums only the merged
/// started vocabulary) — the drop is disclosed on
/// `cline_orphan_finished_rows` and no carrier is parsed. The row's raw
/// JSON text still surfaces as a conversation event (the rm-1430 text
/// stream is additive-untouched), so a usage-less session keeps its
/// rm-616 text estimation — which never writes the cache classes. The
/// cache assertions are therefore the estimation-proof fold invariant:
/// any leak of the row's cacheRead/cacheWrite into usage lands here.
#[test]
fn orphan_finished_row_folds_no_usage_and_discloses_the_drop() {
    let session = parse_file(&fixture("cline-vscode-orphan/ui_messages.json"))
        .expect("orphan fixture parses");
    let counters = &session.metrics.disclosure_counters;
    assert_eq!(
        counters.get("cline_orphan_finished_rows"),
        Some(&1),
        "the orphan drop must be disclosed, not silent"
    );
    assert_eq!(
        counters.get("cline_metrics_carriers_parsed"),
        Some(&0),
        "an orphan finished row parses no carrier"
    );
    assert_eq!(session.metrics.tokens_cache_r, 0);
    assert_eq!(session.metrics.tokens_cache_w, 0);
    assert_eq!(
        session.metrics.provenance.tokens, "estimated_from_text",
        "the totals present are text estimation, never folded usage"
    );
}

/// rm-1435 admission truth: the standalone store probe is
/// content-shaped, never filename-shaped. A hostile corpus of generic
/// `.messages.json` documents — conversation shapes without the store
/// envelope — must never gain CLI-store USAGE: a plain conversation
/// document may fall to a conversation lane (its right, including the
/// pre-existing Roo chat-export arm), but nothing in the corpus may
/// parse through the rm-1435 store probe — no store disclosure
/// counters, and no folded carrier (the cache classes are the
/// estimation-proof invariant: text estimation writes input/output
/// only, so any store-fold leak lands there).
#[test]
fn generic_messages_documents_are_not_claimed_as_the_cli_store() {
    let hostile = [
        // Plain conversation document, no envelope.
        r#"{"messages":[{"role":"user","content":"hi"},{"role":"assistant","content":"hello"}]}"#,
        // Token-shaped JSON nested as CONTENT, not as a sibling
        // metrics carrier — an echo, not usage evidence (rm-1885
        // vocabulary gate: only a sibling object counts).
        r#"{"messages":[{"role":"assistant","content":"{\"tokensIn\":10,\"tokensOut\":20,\"cacheReads\":30}"}]}"#,
        // Carrier-shaped text with NO envelope fields and NO
        // modelInfo/metrics sibling: token keys in a bare message body
        // are shape-adjacent, not the store — nothing may fold.
        r#"{"messages":[{"text":"{\"tokensIn\":10,\"tokensOut\":20,\"cacheReads\":30}"}]}"#,
        // An envelope with an EMPTY messages array is not a store claim
        // (the probe requires a carrier-bearing message).
        r#"{"sessionId":"x","agent":"primary","messages":[]}"#,
    ];
    for document in hostile {
        // Both outcomes are honest: clean rejection (no lane claims it)
        // or a parse through some other lane — the invariant is only
        // that the rm-1435 store fold never runs.
        let session = match parse_raw_session("hostile.messages.json", "hostile", document) {
            Ok(session) => session,
            Err(error) if error.to_string().contains("unsupported session format") => continue,
            Err(error) => panic!("unexpected failure on hostile document: {error}"),
        };
        let counters = &session.metrics.disclosure_counters;
        assert!(
            !counters.contains_key("cline_metrics_carriers_parsed"),
            "hostile document hit the rm-1435 store fold: {document}"
        );
        assert!(
            !counters.contains_key("cline_orphan_finished_rows"),
            "hostile document hit the rm-1975 pair machinery: {document}"
        );
        assert_eq!(
            session.metrics.tokens_cache_r, 0,
            "token-shaped text leaked into store usage: {document}"
        );
        assert_eq!(
            session.metrics.tokens_cache_w, 0,
            "token-shaped text leaked into store usage: {document}"
        );
    }
}

/// rm-1435 discovery truth: the store directory is enumerated as ONE
/// session root — both directly (`collect_session_files` on the store
/// dir) and through a parent-tree walk (the walker admits the store
/// dir at its boundary instead of listing its manifest/messages files
/// as separate sessions).
#[test]
fn discovery_enumerates_the_cli_store_as_one_session_root() {
    use agenttrace_core::collect_session_files;
    let store = fixture("cline-cli-store");
    let direct = collect_session_files(&store);
    assert_eq!(
        direct,
        vec![store.clone()],
        "the store dir itself is the session root"
    );
    // A parent tree holding the store dir yields the DIR (one session),
    // never session.json or the messages document separately.
    let parent = store.parent().expect("fixtures dir exists").to_path_buf();
    let walked = collect_session_files(&parent);
    assert!(
        walked.contains(&store),
        "parent walk must admit the store dir as a session root"
    );
    for path in &walked {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        assert_ne!(
            name, "session.json",
            "the manifest must never enumerate as its own session"
        );
    }
}

//! rm-1984 (lead) + rm-1435 (companion) contract pins — Cline CLI
//! attribution truth against messages contract v1.
//!
//! Upstream authorities this corpus is schema-faithful to (md5-pinned
//! here so a drift in the upstream contract docs is caught at review
//! time; mirrored under the research cache for this cycle):
//! - messages-contract-v1.md  md5 810cada4048d6f074a152d6c89ec79c1
//!   (modelInfo{id,provider} REQUIRED on every assistant row; metrics
//!   inputTokens/outputTokens/cacheReadTokens/cacheWriteTokens/cost;
//!   hooks.jsonl is observability-only)
//! - success.messages.json     md5 4e0d965513974ef0d9c0450a39b0e2e4
//!   (upstream golden: envelope model id "claude-sonnet-4-6",
//!   provider "anthropic", metrics 21/8/3/1/0.13)
//! - session-manifest.ts       md5 c56ce18717005e092f54ae964af75493
//!   (session.json: version/session_id/source/pid/started_at/status/
//!   interactive/provider/model/cwd/workspace_root/enable_* +
//!   optional metadata record — the manifest `model` is the
//!   authoritative fast path, `metadata.model` the fallback)
//!
//! Red-first history: on the pre-rm-1984 tree every attribution
//! assertion below failed — the whole Cline CLI lane stamped
//! `model_used: "default"` (assess F2: unknown_models 2,
//! fallback_pricing 2), the manifest was never read, and a
//! manifest-less session dir silently vanished from discovery
//! (assess F3: sess-beta-nomanifest returned 2/skipped 0 with no
//! disclosure). The disclosure assertions pin the read-time counter
//! so the drop can never go silent again.

use agenttrace_core::{
    build_doctor_report, collect_session_files, compute_overview, data_health_scoped,
    has_specific_price, load_sessions_with_options, parse_file, LoadOptions,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cline-cli");

fn fixture(name: &str) -> PathBuf {
    Path::new(FIXTURES).join(name)
}

/// Serializes env surgery — the same discipline discovery_contract.rs
/// uses. The lock guard is dropped before any caught panic resumes so
/// a flaky assertion cannot poison the mutex for every later test.
fn lock_env() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    if let Some(value) = previous {
        std::env::set_var(key, value);
    } else {
        std::env::remove_var(key);
    }
}

/// Copies the named fixture session dirs into a fresh store root and
/// points `CLINE_SESSION_DATA_DIR` (+ a scratch `HOME`) at the staged
/// tree for the duration of `f`. Only the named dirs are staged so
/// each test controls exactly which sessions the walk can see.
fn with_cline_store(dirs: &[&str], f: impl FnOnce(&Path)) {
    let root = std::env::temp_dir().join(format!(
        "cline-contract-{}-{}",
        std::process::id(),
        dirs.join("+")
    ));
    let _ = fs::remove_dir_all(&root);
    for dir in dirs {
        fs::create_dir_all(root.join(dir)).expect("stage session dir");
        for entry in fs::read_dir(fixture(dir)).expect("fixture dir") {
            let entry = entry.expect("fixture entry");
            let to = root.join(dir).join(entry.file_name());
            fs::copy(entry.path(), &to).expect("stage fixture file");
        }
    }
    let result = {
        let _guard = lock_env();
        let previous_home = std::env::var_os("HOME");
        let previous_store = std::env::var_os("CLINE_SESSION_DATA_DIR");
        let previous_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        let scratch_home = root.join("home");
        fs::create_dir_all(&scratch_home).expect("scratch home");
        std::env::set_var("HOME", &scratch_home);
        std::env::set_var("CLINE_SESSION_DATA_DIR", &root);
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", root.join("no-cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&root)));
        restore_env("HOME", previous_home);
        restore_env("CLINE_SESSION_DATA_DIR", previous_store);
        restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_cache);
        result
    };
    let _ = fs::remove_dir_all(&root);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

/// rm-1984 T1: an in-contract CLI session (manifest + per-row
/// modelInfo, both naming claude-sonnet-4-6) attributes its session
/// label, its per-model rollup, and its usage totals to the REAL
/// model id — never "default"/"unknown" — and prices from the
/// catalog instead of the fallback row. This is the assess-F2 PoC
/// turning green end to end through the discovery lane.
#[test]
fn manifest_and_modelinfo_attribute_the_session_end_to_end() {
    with_cline_store(&["sess-attribution"], |root| {
        let report = load_sessions_with_options(Some(root), &LoadOptions::default());
        assert_eq!(report.sessions.len(), 1, "one manifested session loads");
        assert_eq!(
            report.cline_manifestless, 0,
            "a manifested store owes no refusal count"
        );
        let metrics = &report.sessions[0].metrics;
        assert_eq!(
            metrics.model_used, "claude-sonnet-4-6",
            "session label must be the resolved model, not 'default'"
        );
        assert!(
            has_specific_price(&metrics.model_used),
            "the real id must price from the catalog, not the fallback row"
        );
        // The contract's REQUIRED metrics block (21/8/3/1/0.13 in the
        // upstream golden) folds into the session's token classes and
        // the upstream cost channel.
        assert_eq!(metrics.tokens_input, 21);
        assert_eq!(metrics.tokens_output, 8);
        assert_eq!(metrics.tokens_cache_r, 3);
        assert_eq!(metrics.tokens_cache_w, 1);
        assert_eq!(metrics.upstream_cost_usd, 0.13);
        // 3 assistant-classified events from the golden's two
        // assistant rows (the thinking+tool_use row splits); the
        // attribution subject is the MODEL, so pin the floor.
        assert!(
            metrics.assistant_turns >= 2,
            "both assistant rows survive into the session (got {})",
            metrics.assistant_turns
        );
        assert_eq!(
            metrics
                .wire_metadata
                .get("cline_modelinfo_provider")
                .map(String::as_str),
            Some("anthropic"),
            "modelInfo.provider rides as capped provenance metadata"
        );
        assert_eq!(
            metrics
                .disclosure_counters
                .get("cline_metrics_carriers_parsed"),
            Some(&1),
            "the carrier disclosure folds into session metrics"
        );
        let overview = compute_overview(std::slice::from_ref(&report.sessions[0]));
        assert!(
            overview.by_model.contains_key("claude-sonnet-4-6"),
            "the by_model rollup keys on the real id: {:?}",
            overview.by_model.keys().collect::<Vec<_>>()
        );
        assert!(
            !overview.by_model.contains_key("default")
                && !overview.by_model.contains_key("unknown"),
            "no fabricated model bucket survives"
        );
    });
}

/// rm-1984 T2: when the manifest and the envelope's modelInfo
/// disagree, the manifest wins (it speaks for the session the
/// operator launched) and the disagreement is DISCLOSED, not silently
/// resolved. The envelope row keeps its OWN id in the per-model
/// attribution rollup.
#[test]
fn manifest_model_wins_over_envelope_disagreement_and_discloses() {
    with_cline_store(&["sess-disagree"], |root| {
        let report = load_sessions_with_options(Some(root), &LoadOptions::default());
        assert_eq!(report.sessions.len(), 1);
        let metrics = &report.sessions[0].metrics;
        assert_eq!(
            metrics.model_used, "claude-sonnet-4-6",
            "the manifest model wins over the envelope's gpt-5.2-codex"
        );
        assert_eq!(
            metrics
                .disclosure_counters
                .get("cline_manifest_model_disagreement"),
            Some(&1),
            "a manifest/envelope model disagreement must be disclosed"
        );
        assert!(
            metrics.assistant_turns >= 1,
            "the assistant row survives into the session"
        );
        assert_eq!(
            metrics.upstream_cost_usd, 0.05,
            "the envelope row's own metrics still count"
        );
        assert_eq!(
            metrics
                .wire_metadata
                .get("cline_modelinfo_provider")
                .map(String::as_str),
            Some("openai"),
            "the envelope row keeps its own provider as provenance — the manifest only owns the session label"
        );
    });
}

/// rm-1435 T3: a manifest-less session dir is REFUSED by discovery
/// (messages contract v1 writes session.json beside every transcript)
/// and the refusal is counted and disclosed at every read-time
/// surface — load report, data health, doctor — instead of the
/// session vanishing silently (assess F3).
#[test]
fn manifestless_session_is_refused_counted_and_disclosed() {
    with_cline_store(&["sess-attribution", "sess-beta-nomanifest"], |root| {
        let report = load_sessions_with_options(Some(root), &LoadOptions::default());
        assert_eq!(
            report.sessions.len(),
            1,
            "only the manifested session parses"
        );
        assert!(
            report
                .sessions
                .iter()
                .all(|session| !session.path.ends_with("sess-beta-nomanifest")),
            "the manifest-less transcript must not become a session"
        );
        assert_eq!(
            report.cline_manifestless, 1,
            "the load report counts the refused session dir"
        );
        // data health rides the same key the loader inserts.
        let health = data_health_scoped(
            &report.sessions,
            report.discovered,
            report.sessions.len(),
            report.cache_hits,
            0,
            0,
            Vec::new(),
            report.cline_manifestless,
        );
        assert_eq!(
            health.disclosures.get("cline_manifestless_sessions"),
            Some(&1),
            "data health discloses the refused session count"
        );
        // The doctor's disclosure fold agrees (explicit --dir arm).
        let doctor = build_doctor_report(Some(root), false);
        assert_eq!(
            doctor.disclosures.get("cline_manifestless_sessions"),
            Some(&1),
            "the doctor discloses the refused session count"
        );
    });
}

/// rm-1984 T4: parsing a manifest-less transcript DIRECTLY (explicit
/// file argument) still attributes via the envelope's modelInfo — the
/// fallback ladder never synthesizes a model the corpus did not name.
#[test]
fn manifestless_transcript_direct_parse_still_attributes() {
    let session = parse_file(&fixture(
        "sess-beta-nomanifest/sess-beta-nomanifest.messages.json",
    ))
    .expect("parse manifest-less transcript");
    assert_eq!(
        session.metrics.model_used, "claude-sonnet-4-6",
        "envelope modelInfo alone attributes the session"
    );
    assert!(
        has_specific_price(&session.metrics.model_used),
        "the attributed id prices from the catalog"
    );
}

/// rm-1984 T5: honest degradation — a corpus that names NO model
/// anywhere (no modelInfo, empty manifest model) stays "unknown" per
/// row; the session label is never fabricated and prices at the
/// fallback row, disclosed.
#[test]
fn model_free_corpus_stays_unknown_never_synthesized() {
    let session = parse_file(&fixture("sess-nomodel/sess-nomodel.messages.json"))
        .expect("parse model-free transcript");
    let metrics = &session.metrics;
    assert_eq!(
        metrics.model_used, "default",
        "session label degrades to default, not a fabricated id"
    );
    assert!(
        metrics.model_attribution.keys().all(|key| key != "default"),
        "no fabricated id may join the per-model rollup: {:?}",
        metrics.model_attribution.keys().collect::<Vec<_>>()
    );
    assert!(
        !has_specific_price(&metrics.model_used),
        "the unknown lane still prices as fallback — disclosed, not hidden"
    );
    assert_eq!(
        metrics.tokens_input, 3,
        "the metrics block still counts even without a model"
    );
}

/// rm-1435 T6 (non-perturbation): the session.json manifest and the
/// hooks.jsonl observability log are never admitted as sessions, and
/// the walk returns only the contract-named transcripts.
#[test]
fn manifest_and_hooks_siblings_are_never_admitted() {
    with_cline_store(&["sess-attribution", "sess-disagree"], |root| {
        let files = collect_session_files(root);
        let names: Vec<String> = files
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names.len(),
            2,
            "exactly the two contract-named transcripts: {names:?}"
        );
        assert!(names.iter().all(|name| name.ends_with(".messages.json")));
        assert!(
            !names.iter().any(|name| name.contains("session.json")),
            "the manifest is not a session"
        );
        assert!(
            !names.iter().any(|name| name.contains("hooks")),
            "hooks.jsonl is observability-only, never a session"
        );
    });
}

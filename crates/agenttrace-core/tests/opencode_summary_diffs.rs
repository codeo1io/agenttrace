//! opencode v2 `session.summary_diffs` consumption contract
//! (rm-1509, cycle 2 of run b01c2b6a, repository-maintenance 1bf8dd62).
//!
//! The staged opencode v2 schema (anomalyco/opencode, dev branch
//! `packages/core/src/session/sql.ts:43`) adds a `summary_diffs` json
//! column beside the landed `summary_*` integers, carrying
//! `FileDiff.LegacyInfo[]` (`packages/schema/src/file-diff.ts`:
//! `{file?, patch?, additions, deletions, status?}`). v2 is not on npm
//! yet (the package was renamed to unscoped `opencode-ai`), so this
//! contract pins the lane against a synthetic v2-shape fixture — the
//! same readiness-before-corpus posture rm-991 and the gemini 0.63
//! chats lane shipped with.
//!
//! Fixture `opencode-summary-diffs/opencode.db` rows:
//! - `ses-diff-matched`   — per-file sums equal the summary columns;
//! - `ses-diff-divergent` — per-file sums DISAGREE (40 vs 500 / 4 vs 50);
//! - `ses-diff-malformed` — payload is not JSON (must count, not drop);
//! - `ses-diff-empty`     — `[]` decodes to an empty list (still Some).
//!
//! Fixture `opencode-summary-diffs/hostile.db` rows (review fix F1/F5,
//! attempt 03ec687a):
//! - `ses-diff-hostile` — two i64::MAX per-file additions: every sum
//!   (divergence fold AND the delivery-evidence line) must saturate,
//!   never overflow (debug panic) or wrap (release negative);
//! - `ses-diff-blob`    — the payload is present but stored in BLOB
//!   storage class: counted as dropped, never silently absent.
//!
//! The rm-991 contract (`opencode_agent_summary.rs`) stays the
//! authority for the legacy column set; the fixtures there are
//! untouched and must keep parsing byte-identically.

use agenttrace_core::{
    compute_overview, delivery_evidence, load_sqlite_backed_sessions_reported, Session,
};
use std::path::{Path, PathBuf};

fn temp_home(tag: &str) -> PathBuf {
    // The suite's tests run in parallel and several load the SAME
    // fixture, so the path must be unique per CALL, not per fixture —
    // a shared path lets one test's `remove_dir_all` teardown race
    // another's plant/load (rm-991's suite avoids this by giving every
    // test its own tag; a counter is immune to call-site mistakes).
    static HOMES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let home = std::env::temp_dir().join(format!(
        "agenttrace-rm1509-contract-{}-{tag}-{}",
        std::process::id(),
        HOMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&home).expect("temp home");
    home
}

/// Scoped env override (HOME + XDG + session-cache dir) so the sqlite
/// discovery lanes read only the fixture home and never touch the
/// ambient session cache. Mirrors opencode_agent_summary.rs's
/// `with_home`, including the guard mutex that serializes
/// env-swapping tests and the catch_unwind so a resumed panic never
/// unwinds through a held lock.
fn with_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
    let result = {
        let _guard = lock_env();
        let saved: Vec<(&str, Option<std::ffi::OsString>)> = [
            "HOME",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "XDG_DATA_HOME",
            "AGENTTRACE_SESSION_CACHE_DIR",
        ]
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect();
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        for (key, value) in saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        result
    };
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    match LOCK.get_or_init(|| std::sync::Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn plant(home: &Path, fixture: &str) {
    let target = home.join(".local/share/opencode/opencode.db");
    std::fs::create_dir_all(target.parent().unwrap()).expect("mkdir opencode home");
    std::fs::copy(
        format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), fixture),
        &target,
    )
    .expect("copy fixture db");
}

fn session_named<'a>(sessions: &'a [Session], key: &str) -> &'a Session {
    sessions
        .iter()
        .find(|session| session.metrics.session_key.contains(key))
        .unwrap_or_else(|| {
            let mut keys: Vec<String> = sessions
                .iter()
                .map(|session| session.metrics.session_key.clone())
                .collect();
            keys.sort();
            panic!("no session matching {key:?}; got keys {keys:?}")
        })
}

fn loaded(fixture: &str) -> Vec<Session> {
    let home = temp_home(fixture);
    let sessions = with_home(&home, || {
        plant(&home, fixture);
        let (sessions, report) = load_sqlite_backed_sessions_reported(None);
        assert!(
            report.unreadable.is_empty(),
            "fixture database must be readable: {:?}",
            report.unreadable
        );
        sessions
    });
    std::fs::remove_dir_all(&home).ok();
    sessions
}

#[test]
fn per_file_diffs_surface_with_path_and_counts() {
    let sessions = loaded("opencode-summary-diffs/opencode.db");
    let matched = session_named(&sessions, "ses-diff-matched");
    let diffs = matched
        .metrics
        .summary_diffs
        .as_ref()
        .expect("summary_diffs decodes for the matched row");
    assert_eq!(diffs.len(), 2);
    assert_eq!(diffs[0].file.as_deref(), Some("src/a.rs"));
    assert_eq!(diffs[0].additions, 120);
    assert_eq!(diffs[0].deletions, 8);
    assert_eq!(diffs[0].status.as_deref(), Some("modified"));
    assert_eq!(diffs[1].file.as_deref(), Some("README.md"));
    assert_eq!(diffs[1].additions, 10);
    assert_eq!(diffs[1].deletions, 2);
    // Patch bodies are deliberately not carried onto Metrics — the
    // delivery lane reports counts and paths, not patch text.
    assert!(matched.metrics.summary_diffs_dropped == 0);
}

#[test]
fn matched_per_file_sums_do_not_flag_divergence() {
    let sessions = loaded("opencode-summary-diffs/opencode.db");
    let matched = session_named(&sessions, "ses-diff-matched");
    assert!(
        matched.metrics.summary_diffs_divergence.is_none(),
        "40/4-style per-file sums equal to the summary columns must NOT flag"
    );
}

#[test]
fn divergent_per_file_sums_are_disclosed_not_reconciled() {
    let sessions = loaded("opencode-summary-diffs/opencode.db");
    let divergent = session_named(&sessions, "ses-diff-divergent");
    let divergence = divergent
        .metrics
        .summary_diffs_divergence
        .expect("per-file 40/4 vs summary 500/50 must flag divergence");
    assert_eq!(divergence.per_file_additions, 40);
    assert_eq!(divergence.per_file_deletions, 4);
    assert_eq!(divergence.summary_additions, 500);
    assert_eq!(divergence.summary_deletions, 50);
    // The integer columns keep the WRITER's numbers — divergence is
    // disclosed alongside, never silently reconciled into either side.
    assert_eq!(divergent.metrics.summary_additions, Some(500));
    assert_eq!(divergent.metrics.summary_deletions, Some(50));
}

#[test]
fn malformed_payload_counts_instead_of_dropping_the_session() {
    let sessions = loaded("opencode-summary-diffs/opencode.db");
    let malformed = session_named(&sessions, "ses-diff-malformed");
    assert!(
        malformed.metrics.summary_diffs.is_none(),
        "undecodable payload degrades to None"
    );
    assert_eq!(
        malformed.metrics.summary_diffs_dropped, 1,
        "the undecodable payload is COUNTED"
    );
    // The session itself survives with its estimated cost intact.
    assert!((malformed.metrics.cost_estimated - 0.04).abs() < 1e-9);
}

#[test]
fn empty_diff_list_decodes_to_some_empty() {
    let sessions = loaded("opencode-summary-diffs/opencode.db");
    let empty = session_named(&sessions, "ses-diff-empty");
    let diffs = empty
        .metrics
        .summary_diffs
        .as_ref()
        .expect("'[]' decodes to Some(vec![])");
    assert!(diffs.is_empty());
    assert!(empty.metrics.summary_diffs_dropped == 0);
    // Empty list sums (0/0) vs summary 5/1: no false divergence for a
    // payload the writer explicitly marked empty — divergence requires
    // at least one per-file entry.
    assert!(empty.metrics.summary_diffs_divergence.is_none());
}

#[test]
fn delivery_evidence_carries_per_file_and_divergence_lines() {
    let sessions = loaded("opencode-summary-diffs/opencode.db");
    let report = delivery_evidence(&sessions);
    // Delivery records key on the derived display name (the fixture
    // titles are non-placeholder), not on the session_key.
    let matched = report
        .sessions
        .iter()
        .find(|record| record.session.contains("builder run"))
        .expect("matched record");
    assert!(
        matched
            .evidence
            .iter()
            .any(|line| line.contains("writer-reported per-file diff summary: 2 files (+130/-10)")),
        "per-file summary line missing: {:?}",
        matched.evidence
    );
    let divergent = report
        .sessions
        .iter()
        .find(|record| record.session.contains("builder diverges"))
        .expect("divergent record");
    assert!(
        divergent
            .evidence
            .iter()
            .any(|line| line.contains("diverge") && line.contains("+40 vs +500")),
        "divergence disclosure line missing: {:?}",
        divergent.evidence
    );
}

#[test]
fn hostile_magnitudes_saturate_across_both_sums() {
    // Review fix F1: the live CLI PoC planted exactly this shape (two
    // i64::MAX additions) and `--delivery-evidence` panicked debug
    // builds with 'attempt to add with overflow' (release wrapped to
    // -2). Both disclosure surfaces must saturate, and they must
    // agree with each other: the divergence fold saturates to
    // i64::MAX, the integer column records i64::MAX, so equality
    // holds and no divergence is flagged.
    let sessions = loaded("opencode-summary-diffs/hostile.db");
    let hostile = session_named(&sessions, "ses-diff-hostile");
    assert_eq!(hostile.metrics.summary_diffs.as_ref().unwrap().len(), 2);
    assert!(
        hostile.metrics.summary_diffs_divergence.is_none(),
        "saturated per-file sums must equal the i64::MAX summary column"
    );
    let report = delivery_evidence(&sessions);
    let record = report
        .sessions
        .iter()
        .find(|record| record.session.contains("hostile saturates"))
        .expect("hostile record");
    assert!(
        record.evidence.iter().any(|line| line
            == "writer-reported per-file diff summary: 2 files (+9223372036854775807/-2)"),
        "evidence sums must pin at i64::MAX: {:?}",
        record.evidence
    );
}

#[test]
fn blob_storage_payload_counts_as_dropped() {
    // Review fix F5: SQLite columns are dynamically typed — a payload
    // present in BLOB storage (valid JSON bytes, wrong storage class)
    // must be COUNTED as dropped, not read as absent. Only SQL NULL
    // is absence.
    let sessions = loaded("opencode-summary-diffs/hostile.db");
    let blob = session_named(&sessions, "ses-diff-blob");
    assert!(
        blob.metrics.summary_diffs.is_none(),
        "non-TEXT payload does not decode"
    );
    assert_eq!(
        blob.metrics.summary_diffs_dropped, 1,
        "present-but-BLOB payload is COUNTED"
    );
    assert!(blob.metrics.summary_diffs_divergence.is_none());
    // The session itself survives.
    assert!((blob.metrics.cost_estimated - 0.0).abs() < 1e-9);
}

#[test]
fn legacy_store_without_the_column_loads_identically() {
    // The rm-991 fixture (schema WITHOUT summary_diffs) must keep
    // parsing exactly as before: no diffs, no dropped, no divergence —
    // the column-presence probe stays the gate.
    let sessions = loaded("opencode-agent-summary/opencode.db");
    assert!(sessions
        .iter()
        .all(|session| session.metrics.summary_diffs.is_none()));
    assert!(sessions
        .iter()
        .all(|session| session.metrics.summary_diffs_dropped == 0));
    assert!(sessions
        .iter()
        .all(|session| session.metrics.summary_diffs_divergence.is_none()));
    let build = session_named(&sessions, "ses-build");
    assert_eq!(build.metrics.summary_additions, Some(120));
    assert_eq!(build.metrics.agent_name, "build");
}

#[test]
fn overview_by_agent_still_keys_on_the_agent_column() {
    let sessions = loaded("opencode-summary-diffs/opencode.db");
    let overview = compute_overview(&sessions);
    let builder = overview
        .by_agent
        .iter()
        .find(|(agent, _)| agent.as_str() == "builder")
        .map(|(_, group)| group)
        .expect("builder bucket");
    assert_eq!(builder.sessions, 2);
}

/// rm-1913 (run 0779d26b, cycle 6 — superseded at integration by the
/// landed rm-1926 arm, which owns this defect with a richer
/// disclosure; the run's fixture and per-leg shapes ride on and pin
/// the landed counter): the rm-1509 cross-check silently skipped
/// when either integer `summary_*` leg was NULL or non-numeric
/// — even with the per-file list present and healthy (sums 130/10) —
/// so a writer that recorded diffs but lost a summary column was
/// never checked AND never disclosed. The skip is per-session
/// visible and names the broken column and storage class; the
/// numbers themselves still never move, and the healthy path (both
/// legs present, sums matching) stays counter-free.
#[test]
fn null_or_non_numeric_summary_leg_discloses_the_skipped_crosscheck() {
    let sessions = loaded("opencode-summary-diffs-nulllegs/opencode.db");
    const COUNTER: &str = "opencode_summary_cross_check_unreadable";
    let expected = [
        ("ses-null-add", "summary_additions", "stored as NULL"),
        ("ses-null-del", "summary_deletions", "stored as NULL"),
        ("ses-text-leg", "summary_additions", "non-numeric text"),
    ];
    for (key, column, reason) in expected {
        let session = session_named(&sessions, key);
        // The per-file payload is present and healthy — the check had
        // something to compare and could not run.
        assert_eq!(
            session.metrics.summary_diffs.as_ref().unwrap().len(),
            2,
            "{key}: per-file list present"
        );
        assert_eq!(session.metrics.summary_diffs_dropped, 0);
        assert!(
            session.metrics.summary_diffs_divergence.is_none(),
            "{key}: no divergence verdict without both legs"
        );
        // The landed rm-1926 disclosure names the broken column and
        // its storage class — the run's counter spelling is retired.
        let detail = session
            .metrics
            .summary_cross_check_unreadable
            .as_ref()
            .unwrap_or_else(|| panic!("{key}: unreadable detail missing"));
        assert_eq!(detail.len(), 1, "{key}: one broken leg");
        assert_eq!(detail[0].column, column, "{key}: broken column named");
        assert_eq!(detail[0].reason, reason, "{key}: storage class named");
        assert_eq!(
            session.metrics.disclosure_counters.get(COUNTER),
            Some(&1),
            "{key}: the skipped cross-check is disclosed"
        );
    }

    // Healthy control: both legs present and matching the per-file
    // sums — no divergence, and no skip disclosure.
    let healthy = session_named(&sessions, "ses-healthy");
    assert_eq!(healthy.metrics.summary_additions, Some(130));
    assert_eq!(healthy.metrics.summary_deletions, Some(10));
    assert!(healthy.metrics.summary_diffs_divergence.is_none());
    assert!(healthy.metrics.summary_cross_check_unreadable.is_none());
    assert!(!healthy.metrics.disclosure_counters.contains_key(COUNTER));

    // The rm-991 contract (opencode_agent_summary.rs) stays the
    // authority for the legacy column set: its fixture is untouched.
}

//! Count-honesty ledger admission tests (cycle 5, run 77ca5de0).
//!
//! Three ledger arms landed together because they are the same
//! honesty failure seen from three sides — the report's counts are
//! only as honest as admission, pricing, and dedup:
//!
//! - rm-1887: pi subagent artifact mirrors under a
//!   `subagent-artifacts/` path segment are child-agent output, not
//!   independent sessions. They used to be admitted as phantom
//!   sessions (double-banked spend the operator never approved).
//!   Admission now skips them and discloses the count
//!   (`subagent_artifact_files`), while the `-d` inspection lane
//!   keeps them (rm-548 stance: explicit dirs are for looking).
//! - rm-1886: byte-identical journals at two paths are ONE session;
//!   the copy is suppressed at admission and disclosed
//!   (`duplicate_session_files`, `duplicate_session_file_pairs`
//!   naming survivor <- suppressed), without touching the `-d` lane.
//! - rm-1888: a negative `cost.total` on a pi usage block is a
//!   provider bug, not a refund. It is rejected and counted
//!   (`recorded_costs_negative_rejected`), mirroring cline's clamp
//!   disclosure; a well-formed 0.0 stays honored (gateway flat rate)
//!   and a positive recorded cost still wins over the estimate.
//!
//! The F1/F2 numbers pin the same red-first PoCs the assess phase
//! proved on this corpus shape: F1 was "2 sessions / $0.0076 for one
//! real session" (artifact mirror), F2 was "-5.0 silently folded".
//! Every HOME/XDG/cache env var is pinned per-test so the suite never
//! reads or writes the host's real session state or the shared
//! default session cache.

use agenttrace_core::{
    analyze, build_doctor_report, find_session_files, load_session_cache,
    load_sessions_with_progress, load_sessions_with_progress_from_cache, parse_raw_session,
    save_session_cache, Event, LoadOptions,
};
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within
/// this binary (same pattern as pi_family_discovery.rs): discovery
/// reads env at call time, so two tests pinning HOME concurrently
/// would race. Each call gets its own temp home AND its own session
/// cache dir — the host default cache is a shared fleet surface.
fn with_pinned_home(home: &Path, f: impl FnOnce()) {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = GUARD.get_or_init(|| Mutex::new(())).lock().unwrap();
    let previous_home = std::env::var_os("HOME");
    let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
    let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
    let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
    let previous_session_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("HOME", home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
    std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("session-cache"));
    f();
    restore_env("HOME", previous_home);
    restore_env("XDG_CONFIG_HOME", previous_xdg_config);
    restore_env("XDG_CACHE_HOME", previous_xdg_cache);
    restore_env("XDG_DATA_HOME", previous_xdg_data);
    restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_session_cache);
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        None => std::env::remove_var(key),
        Some(value) => std::env::set_var(key, value),
    }
}

fn temp_home(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("at-count-honesty-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp home");
    dir
}

/// A pi v3 journal whose single usage block records `cost.total` =
/// `total`, mirroring the pi-pi-v3 fixture shape (same wire keys, ids
/// synthesized) so it parses through the pi lane end to end.
fn pi_journal_with_cost(total: &str) -> String {
    [
        r#"{"id": "00000000-0000-4000-8000-000000000001", "timestamp": "2026-10-04T07:16:46.634Z", "type": "session", "version": 3, "cwd": "/redacted/project"}"#,
        r#"{"id": "00000000-0000-4000-8000-000000000002", "parentId": "00000000-0000-4000-8000-000000000001", "timestamp": "2026-10-04T07:16:46.700Z", "type": "session_info", "name": "redacted-session"}"#,
        r#"{"id": "00000000-0000-4000-8000-000000000003", "timestamp": "2026-10-04T07:16:47.000Z", "message": {"role": "user", "content": [{"type": "text", "text": "hello"}], "timestamp": "2026-10-04T07:16:47.000Z"}, "type": "message"}"#,
        &format!(
            r#"{{"id": "00000000-0000-4000-8000-000000000004", "timestamp": "2026-10-04T07:16:48.000Z", "message": {{"role": "assistant", "content": [{{"type": "text", "text": "hi there"}}], "timestamp": "2026-10-04T07:16:48.000Z", "usage": {{"input": 500, "output": 1000, "cacheRead": 25000, "cacheWrite": 5000, "totalTokens": 31500, "cost": {{"input": 0.0015, "output": 0.006, "cacheRead": 0.0, "cacheWrite": 0.0, "total": {total}}}}}}}, "model": "claude-sonnet-4-20250514", "type": "message"}}"#
        ),
    ]
    .join("\n")
        + "\n"
}

/// Generic role-based journal for cross-lane (cursor/hermes) fixture
/// files: parses everywhere, content varies by `seed` so the dedup
/// ledger never sees these as copies.
fn generic_journal(seed: u32) -> String {
    format!(
        concat!(
            r#"{{"role":"session_meta","timestamp":"2026-05-02T10:00:0{}Z","ModelUsed":"claude-sonnet-4"}}"#,
            "\n",
            r#"{{"role":"meta","ModelUsed":"claude-sonnet-4","Usage":{{"input_tokens":{},"output_tokens":5}}}}"#,
            "\n"
        ),
        seed,
        10 + seed
    )
}

fn write(path: &Path, contents: &str) -> PathBuf {
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, contents).expect("write fixture");
    path.to_path_buf()
}

// ------------------------------------------------------------------
// rm-1888 — negative recorded costs are rejected and counted (F2
// matrix: {-5.0 rejected, +0.0045 wins, 0.0 honored}).
// ------------------------------------------------------------------

#[test]
fn f2_negative_recorded_cost_is_rejected_and_counted() {
    // Red-first flip: assess F2 proved -5.0 folded silently. Now the
    // block is rejected (tokens fall back to the catalog estimate —
    // never a negative total) and the rejection is NAMED.
    let session = parse_raw_session("pi-neg", "pi-neg", &pi_journal_with_cost("-5.0"))
        .expect("negative-cost pi journal still parses");
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("recorded_costs_negative_rejected"),
        Some(&1),
        "the rejected negative recorded cost must be counted, not silently dropped"
    );
    assert!(
        (session.metrics.upstream_cost_usd - 0.0).abs() < 1e-12,
        "negative recorded cost must not fold into upstream cost: got {}",
        session.metrics.upstream_cost_usd
    );
    assert!(
        session.metrics.cost_estimated > 0.0005 && session.metrics.cost_estimated < 0.05,
        "with the recorded block rejected the tokens price from the catalog: got {}",
        session.metrics.cost_estimated
    );
    assert!(
        session.metrics.cost_estimated.is_finite() && session.metrics.cost_estimated >= 0.0,
        "session total can never be negative: got {}",
        session.metrics.cost_estimated
    );
}

#[test]
fn f2_positive_recorded_cost_still_wins() {
    let session = parse_raw_session("pi-pos", "pi-pos", &pi_journal_with_cost("0.0045"))
        .expect("positive recorded cost parses");
    assert!(
        (session.metrics.upstream_cost_usd - 0.0045).abs() < 1e-12,
        "recorded +0.0045 folds verbatim: got {}",
        session.metrics.upstream_cost_usd
    );
    assert!(
        (session.metrics.cost_estimated - 0.0045).abs() < 1e-9,
        "recorded wins over the catalog estimate: got {}",
        session.metrics.cost_estimated
    );
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("recorded_costs_negative_rejected"),
        "a positive recorded cost must not mint a rejection"
    );
}

#[test]
fn f2_zero_recorded_cost_is_honored_not_rejected() {
    // A well-formed 0.0 is a gateway flat rate, not a bug (rm-1052
    // discipline): it stays authoritative and mints no rejection.
    let session = parse_raw_session("pi-zero", "pi-zero", &pi_journal_with_cost("0.0"))
        .expect("zero recorded cost parses");
    assert!(
        !session
            .metrics
            .disclosure_counters
            .contains_key("recorded_costs_negative_rejected"),
        "0.0 is well-formed — no rejection counter"
    );
    assert!(
        (session.metrics.cost_estimated - 0.0).abs() < 1e-12,
        "recorded 0.0 stays authoritative: got {}",
        session.metrics.cost_estimated
    );
}

#[test]
fn analyze_counts_negative_recorded_costs_from_any_lane() {
    // The parser gate covers the pi wire shape; the analyze-level
    // mint is the cross-lane net (serde/generic events carrying a
    // negative recorded cost reach metrics with the same key).
    let usage = || {
        BTreeMap::from([
            ("input_tokens".to_string(), 1000i64),
            ("output_tokens".to_string(), 500),
        ])
    };
    let negative = Event {
        role: "meta".to_string(),
        model_used: "claude-sonnet-4-20250514".to_string(),
        usage: usage(),
        recorded_cost_usd: Some(-1.0),
        ..Event::default()
    };
    let metrics = analyze(std::slice::from_ref(&negative), "claude-sonnet-4-20250514");
    assert_eq!(
        metrics
            .disclosure_counters
            .get("recorded_costs_negative_rejected"),
        Some(&1),
        "analyze mints the shared rejection key for non-pi lanes"
    );
    assert!(
        (metrics.upstream_cost_usd - 0.0).abs() < 1e-12,
        "negative recorded cost never folds into upstream cost"
    );

    for well_formed in [0.0045f64, 0.0] {
        let event = Event {
            role: "meta".to_string(),
            model_used: "claude-sonnet-4-20250514".to_string(),
            usage: usage(),
            recorded_cost_usd: Some(well_formed),
            ..Event::default()
        };
        let metrics = analyze(std::slice::from_ref(&event), "claude-sonnet-4-20250514");
        assert!(
            !metrics
                .disclosure_counters
                .contains_key("recorded_costs_negative_rejected"),
            "well-formed recorded cost {well_formed} must not mint a rejection"
        );
    }
}

// ------------------------------------------------------------------
// rm-1887 — subagent artifact shapes are skipped and disclosed; the
// filename lookalike is NOT mislabeled (segment, not suffix).
// ------------------------------------------------------------------

#[test]
fn subagent_artifact_shapes_are_skipped_and_counted() {
    let home = temp_home("artifacts");
    let proj = home.join(".pi/agent/sessions/proj");
    let real = write(&proj.join("session.jsonl"), &pi_journal_with_cost("0.0038"));
    let shapes = [
        proj.join("subagent-artifacts/run1_worker/_transcript.jsonl"),
        proj.join("subagent-artifacts/run1_worker/mixed.jsonl"),
        proj.join("subagent-artifacts/mirror/event-stream.jsonl"),
        proj.join("subagent-artifacts/outputs/call_123/events.jsonl"),
    ];
    for shape in &shapes {
        write(shape, &generic_journal(7));
    }
    with_pinned_home(&home, || {
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        assert_eq!(
            report.subagent_artifact_files, 4,
            "all four artifact shapes are skipped at admission"
        );
        assert_eq!(
            report.sessions.len(),
            1,
            "only the real session is admitted — artifact mirrors are not phantom sessions"
        );
        assert_eq!(
            report.sessions[0].path,
            real.to_string_lossy(),
            "the surviving session is the real journal"
        );
        assert_eq!(
            report.duplicate_session_files, 0,
            "skipped artifacts are not also counted as duplicates"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn artifact_guard_does_not_mislabel_lookalike_names() {
    // The guard matches a PATH SEGMENT, never a filename suffix: a
    // journal legitimately NAMED subagent-artifacts.jsonl in the
    // cursor, hermes, and pi lanes stays admitted.
    let home = temp_home("lookalike");
    // Cursor admits only agent-transcripts/ subtrees (pre-existing
    // `is_special_session_file` gate), so its lookalike lives there —
    // the most adversarial spelling the lane can host.
    let cursor = write(
        &home.join(".cursor/projects/p1/agent-transcripts/subagent-artifacts.jsonl"),
        &generic_journal(1),
    );
    let hermes = write(
        &home.join(".hermes/sessions/p1/subagent-artifacts.jsonl"),
        &generic_journal(2),
    );
    let pi = write(
        &home.join(".pi/agent/sessions/p1/subagent-artifacts.jsonl"),
        &generic_journal(3),
    );
    with_pinned_home(&home, || {
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        assert_eq!(
            report.subagent_artifact_files, 0,
            "filename lookalikes must never be skipped"
        );
        assert_eq!(
            report.sessions.len(),
            3,
            "cursor/hermes/pi lookalike journals all stay admitted"
        );
        let paths: Vec<&str> = report
            .sessions
            .iter()
            .map(|session| session.path.as_str())
            .collect();
        for expected in [
            cursor.to_string_lossy(),
            hermes.to_string_lossy(),
            pi.to_string_lossy(),
        ] {
            assert!(
                paths.contains(&expected.as_ref()),
                "admitted set must contain {expected}; got {paths:?}"
            );
        }
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn explicit_dir_lane_keeps_subagent_artifacts() {
    // rm-548 stance: `-d` is the inspection lane — the operator named
    // the directory, so its files (artifact mirrors included) load.
    let home = temp_home("artifacts-explicit");
    let proj = home.join(".pi/agent/sessions/proj");
    write(&proj.join("session.jsonl"), &pi_journal_with_cost("0.0038"));
    let mirror = write(
        &proj.join("subagent-artifacts/run1_worker/_transcript.jsonl"),
        &generic_journal(7),
    );
    with_pinned_home(&home, || {
        let admitted = find_session_files(Some(&proj));
        assert!(
            admitted.iter().any(|path| path == &mirror),
            "the -d lane keeps artifact files: got {admitted:?}"
        );
        let report = load_sessions_with_progress(Some(&proj), &LoadOptions::default(), |_| {});
        assert_eq!(
            report.subagent_artifact_files, 0,
            "the -d lane does not mint artifact counters"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

// ------------------------------------------------------------------
// rm-1886 — content-signature duplicate suppression with disclosure.
// ------------------------------------------------------------------

#[test]
fn byte_identical_copy_is_suppressed_with_disclosure() {
    // Red-first flip of assess F1: two byte-identical copies of one
    // journal used to count as 2 sessions / $0.0076 (double-banked).
    // One session is admitted, the copy is disclosed by name, and the
    // total is the single-copy $0.0038.
    let home = temp_home("dup");
    let journal = pi_journal_with_cost("0.0038");
    let alpha = write(&home.join(".pi/agent/sessions/alpha/a.jsonl"), &journal);
    let beta = write(&home.join(".pi/agent/sessions/beta/b.jsonl"), &journal);
    with_pinned_home(&home, || {
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        assert_eq!(report.sessions.len(), 1, "copies collapse to one session");
        assert_eq!(
            report.duplicate_session_files, 1,
            "the suppressed copy is counted"
        );
        assert_eq!(report.duplicate_session_file_pairs.len(), 1);
        let (survivor, suppressed) = &report.duplicate_session_file_pairs[0];
        let mut got: Vec<&Path> = vec![survivor.as_path(), suppressed.as_path()];
        let mut expected: Vec<&Path> = vec![alpha.as_path(), beta.as_path()];
        got.sort();
        expected.sort();
        assert_eq!(
            got, expected,
            "the pair names survivor and suppressed by path"
        );
        assert_eq!(
            Path::new(&report.sessions[0].path),
            survivor.as_path(),
            "the admitted session is the named survivor"
        );
        assert!(
            (report.sessions[0].metrics.cost_estimated - 0.0038).abs() < 1e-9,
            "total is the single-copy cost, not double-banked: got {}",
            report.sessions[0].metrics.cost_estimated
        );
        assert!(
            (report.sessions[0].metrics.upstream_cost_usd - 0.0038).abs() < 1e-12,
            "recorded cost counted once"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn duplicate_survivor_is_the_lexicographic_min_regardless_of_mtime() {
    // rm-1888 residual (cycle 1, 2026-10-21): the survivor among
    // byte-identical copies used to be whichever copy the mtime sort
    // visited first — newest-mtime wins — so the winner followed the
    // clock (the HOME probe flipped survivors by touching one copy)
    // and flipped on mtime-coincident writes (fork master CI red,
    // run 39394440216: the CLI count-honesty lane's survivor flipped
    // on macOS). The rule no longer consults the clock: the
    // LEXICOGRAPHICALLY SMALLEST path survives, under BOTH mtime
    // orderings and on a warm-cache rerun.
    let home = temp_home("lexmin");
    let journal = pi_journal_with_cost("0.0038");
    let alpha = write(&home.join(".pi/agent/sessions/alpha/a.jsonl"), &journal);
    let beta = write(&home.join(".pi/agent/sessions/beta/b.jsonl"), &journal);
    let older = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
    let newer = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
    let set_mtime = |path: &Path, mtime: std::time::SystemTime| {
        let file = fs::File::options()
            .write(true)
            .open(path)
            .expect("open for set_modified");
        file.set_modified(mtime).expect("set_modified");
    };
    let assert_alpha_survives = |tag: &str| {
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        assert_eq!(
            report.sessions.len(),
            1,
            "{tag}: copies collapse to one session"
        );
        assert_eq!(
            report.duplicate_session_files, 1,
            "{tag}: the copy is counted"
        );
        assert_eq!(
            report.duplicate_session_file_pairs.len(),
            1,
            "{tag}: the pair is named"
        );
        let (survivor, suppressed) = &report.duplicate_session_file_pairs[0];
        assert_eq!(
            survivor, &alpha,
            "{tag}: the lexicographically smallest path survives"
        );
        assert_eq!(suppressed, &beta, "{tag}: the other copy is suppressed");
        assert_eq!(
            Path::new(&report.sessions[0].path),
            alpha.as_path(),
            "{tag}: the admitted session is the deterministic survivor"
        );
    };
    with_pinned_home(&home, || {
        // Regime 1: natural order — beta (written second) is newest.
        set_mtime(&alpha, older);
        set_mtime(&beta, newer);
        assert_alpha_survives("beta-newest");
        // Regime 2: flipped — alpha is the newest copy. The survivor
        // must not move: that movement is exactly the macOS CI flip.
        set_mtime(&alpha, newer);
        set_mtime(&beta, older);
        assert_alpha_survives("alpha-newest");
        // Warm-cache rerun under the flipped regime stays on the same
        // survivor — the content-signature memo must not resurrect an
        // mtime-order winner from an earlier cache generation.
        let mut cache = load_session_cache();
        let warm = load_sessions_with_progress_from_cache(
            None,
            &LoadOptions::default(),
            &mut cache,
            |_| {},
        );
        assert_eq!(
            Path::new(&warm.sessions[0].path),
            alpha.as_path(),
            "warm rerun stays on the lexicographic-min survivor"
        );
        assert_eq!(warm.duplicate_session_files, 1);
        assert_eq!(warm.sessions.len(), 1);
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn distinct_content_of_the_same_size_is_not_suppressed() {
    // The ledger keys content, not size: same byte-length journals
    // with different content are two real sessions.
    let home = temp_home("distinct");
    let mut left = pi_journal_with_cost("0.0038");
    let mut right = pi_journal_with_cost("0.0038");
    // Same length, different bytes: swap one digit pair.
    right = right.replace("total\": 0.0038", "total\": 0.0048");
    assert_eq!(left.len(), right.len(), "fixture must stay same-size");
    assert_ne!(left, right);
    left.push('\n');
    right.push('\n');
    write(&home.join(".pi/agent/sessions/alpha/a.jsonl"), &left);
    write(&home.join(".pi/agent/sessions/beta/b.jsonl"), &right);
    with_pinned_home(&home, || {
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        assert_eq!(
            report.sessions.len(),
            2,
            "distinct-content journals are both admitted"
        );
        assert_eq!(
            report.duplicate_session_files, 0,
            "no duplicate disclosure for distinct content"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn explicit_dir_lane_keeps_duplicate_copies() {
    // rm-548 stance parity: `-d` loads both copies — suppression is
    // an aggregation-lane rule, never an inspection-lane rule.
    let home = temp_home("dup-explicit");
    let journal = pi_journal_with_cost("0.0038");
    let sessions_dir = home.join(".pi/agent/sessions");
    let alpha = write(&sessions_dir.join("alpha/a.jsonl"), &journal);
    let beta = write(&sessions_dir.join("beta/b.jsonl"), &journal);
    with_pinned_home(&home, || {
        let admitted = find_session_files(Some(&sessions_dir));
        assert!(
            admitted.contains(&alpha) && admitted.contains(&beta),
            "the -d lane keeps both copies: got {admitted:?}"
        );
        let report =
            load_sessions_with_progress(Some(&sessions_dir), &LoadOptions::default(), |_| {});
        assert_eq!(
            report.sessions.len(),
            2,
            "explicit dir loads both copies as sessions"
        );
        assert_eq!(
            report.duplicate_session_files, 0,
            "the -d lane does not suppress"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

#[cfg(unix)]
// from_mode chmod arm has no Windows equivalent (fork
// master CI red, run 39419593000: E0433 :38 / E0599 :488/:502)
#[test]
fn unreadable_journal_is_kept_and_never_a_duplicate() {
    // A journal whose bytes cannot be read (permissions) has no
    // content signature: it must stay ADMITTED (its parse failure is
    // disclosed downstream) and must not be counted as a duplicate of
    // anything — suppression without evidence would hide the file.
    let home = temp_home("unreadable");
    let proj = home.join(".pi/agent/sessions/proj");
    write(&proj.join("a.jsonl"), &pi_journal_with_cost("0.0038"));
    let sealed = write(&proj.join("b.jsonl"), &pi_journal_with_cost("0.0048"));
    fs::set_permissions(&sealed, fs::Permissions::from_mode(0o000)).expect("seal journal");
    with_pinned_home(&home, || {
        let admitted = find_session_files(None);
        assert!(
            admitted.contains(&sealed),
            "an unreadable journal stays admitted: got {admitted:?}"
        );
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        assert_eq!(
            report.duplicate_session_files, 0,
            "no content signature means no duplicate claim"
        );
        assert_eq!(report.sessions.len(), 1, "only the readable journal parses");
    });
    let _ = fs::set_permissions(&sealed, fs::Permissions::from_mode(0o644));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn over_cap_journals_are_not_deduped_and_refuse_loudly_at_parse() {
    // Integration seam (conflict case 72b88975, rm-451 composition):
    // the content-signature read rides the named-file read cap, so an
    // over-cap journal signs as None — ADMITTED (never deduped, never
    // silently dropped) and refused loudly by the parse lane's
    // ReadCapRefused instead of being fully read just to prove it is
    // a copy. Two byte-identical over-cap journals therefore surface
    // as two disclosed refusals, not one swallowed duplicate.
    let home = temp_home("overcap-dup");
    let journal = pi_journal_with_cost("0.0038");
    assert!(
        journal.len() > 16,
        "fixture must sit over the pinned test cap"
    );
    let alpha = write(&home.join(".pi/agent/sessions/alpha/a.jsonl"), &journal);
    let beta = write(&home.join(".pi/agent/sessions/beta/b.jsonl"), &journal);
    with_pinned_home(&home, || {
        let previous_cap = std::env::var_os("AGENTTRACE_FILE_CAP_BYTES");
        std::env::set_var("AGENTTRACE_FILE_CAP_BYTES", "16");
        let admitted = find_session_files(None);
        std::env::remove_var("AGENTTRACE_FILE_CAP_BYTES");
        restore_env("AGENTTRACE_FILE_CAP_BYTES", previous_cap);
        assert!(
            admitted.contains(&alpha) && admitted.contains(&beta),
            "over-cap journals stay admitted for the parse lane: got {admitted:?}"
        );
    });
    with_pinned_home(&home, || {
        let previous_cap = std::env::var_os("AGENTTRACE_FILE_CAP_BYTES");
        std::env::set_var("AGENTTRACE_FILE_CAP_BYTES", "16");
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        std::env::remove_var("AGENTTRACE_FILE_CAP_BYTES");
        restore_env("AGENTTRACE_FILE_CAP_BYTES", previous_cap);
        assert_eq!(
            report.duplicate_session_files, 0,
            "no signature means no duplicate claim — the cap outranks dedup"
        );
        assert_eq!(report.sessions.len(), 0, "neither over-cap copy banks");
        assert_eq!(report.parse_failures, 2, "both copies refuse, loudly");
        assert_eq!(
            report.read_cap_refused, 2,
            "the refusals classify as read-cap, not generic parse loss"
        );
        let first = report.first_parse_failure.as_deref().unwrap_or_default();
        assert!(
            first.contains("read cap"),
            "the refusal names the cap class: {first}"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn duplicate_suppression_converges_across_warm_cache_runs() {
    // rm-1886 cache-leg acceptance: the SAME counts on a warm rerun
    // with NO wipe — the content-signature memo and the admission
    // ledger must agree across cache generations.
    let home = temp_home("warm");
    let journal = pi_journal_with_cost("0.0038");
    write(&home.join(".pi/agent/sessions/alpha/a.jsonl"), &journal);
    write(&home.join(".pi/agent/sessions/beta/b.jsonl"), &journal);
    write(
        &home.join(".pi/agent/sessions/proj/subagent-artifacts/run1_w/_transcript.jsonl"),
        &generic_journal(7),
    );
    with_pinned_home(&home, || {
        let cold = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        assert_eq!(cold.sessions.len(), 1);
        assert_eq!(cold.duplicate_session_files, 1);
        assert_eq!(cold.subagent_artifact_files, 1);

        // Warm rerun: load the persisted cache explicitly, serve the
        // corpus through it, and require the SAME admission ledger.
        let mut cache = load_session_cache();
        let warm = load_sessions_with_progress_from_cache(
            None,
            &LoadOptions::default(),
            &mut cache,
            |_| {},
        );
        let _ = save_session_cache(&mut cache);
        assert!(
            warm.cache_hits >= 1,
            "the warm leg actually served from cache: {} hits",
            warm.cache_hits
        );
        assert_eq!(
            warm.sessions.len(),
            cold.sessions.len(),
            "session count converges without a cache wipe"
        );
        assert_eq!(
            warm.duplicate_session_files, cold.duplicate_session_files,
            "duplicate disclosure converges without a cache wipe"
        );
        assert_eq!(
            warm.subagent_artifact_files, cold.subagent_artifact_files,
            "artifact disclosure converges without a cache wipe"
        );
        for (cold_session, warm_session) in cold.sessions.iter().zip(warm.sessions.iter()) {
            assert_eq!(cold_session.path, warm_session.path);
            assert!(
                (cold_session.metrics.cost_estimated - warm_session.metrics.cost_estimated).abs()
                    < 1e-9,
                "cost converges: {} vs {}",
                cold_session.metrics.cost_estimated,
                warm_session.metrics.cost_estimated
            );
        }
    });
    let _ = fs::remove_dir_all(&home);
}

// ------------------------------------------------------------------
// Doctor + audit disclosure surface.
// ------------------------------------------------------------------

#[test]
fn doctor_and_audit_disclose_the_admission_ledger() {
    let home = temp_home("doctor");
    let journal = pi_journal_with_cost("0.0038");
    write(&home.join(".pi/agent/sessions/alpha/a.jsonl"), &journal);
    write(&home.join(".pi/agent/sessions/beta/b.jsonl"), &journal);
    write(
        &home.join(".pi/agent/sessions/proj/subagent-artifacts/run1_w/_transcript.jsonl"),
        &generic_journal(7),
    );
    // rm-1888 arm: a pi journal whose recorded cost is negative must
    // surface its rejection in the same doctor render.
    write(
        &home.join(".pi/agent/sessions/gamma/neg.jsonl"),
        &pi_journal_with_cost("-5.0"),
    );
    with_pinned_home(&home, || {
        let report = build_doctor_report(None, false);
        assert_eq!(
            report.disclosures.get("subagent_artifact_files"),
            Some(&1),
            "doctor carries the artifact-skip count"
        );
        assert_eq!(
            report.disclosures.get("duplicate_session_files"),
            Some(&1),
            "doctor carries the duplicate-suppression count"
        );
        assert_eq!(
            report.disclosures.get("recorded_costs_negative_rejected"),
            Some(&1),
            "doctor carries the negative-recorded-cost rejection"
        );
        let text = agenttrace_core::doctor_report_text(&report);
        assert!(
            text.contains("subagent_artifact_files"),
            "rendered doctor text names the artifact counter"
        );
        assert!(
            text.contains("duplicate_session_files"),
            "rendered doctor text names the duplicate counter"
        );
        assert!(
            text.contains("a.jsonl") && text.contains("b.jsonl"),
            "the duplicate pair renders survivor <- suppressed by name"
        );
        assert!(
            text.contains("recorded_costs_negative_rejected"),
            "the negative-cost rejection family renders in doctor"
        );
        // The --cost-audit JSON injection of these two counters (the
        // CLI-layer seam in main.rs) is pinned end to end by
        // `count_honesty_disclosures_in_cli_lanes` in
        // agenttrace-cli/tests/entrypoints.rs on this same fixture
        // shape.
    });
    let _ = fs::remove_dir_all(&home);
}

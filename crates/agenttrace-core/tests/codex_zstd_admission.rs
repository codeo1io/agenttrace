//! rm-992 + rm-1872 residual arm: compressed codex rollouts must be TRUE
//! end-to-end — discovery admits the `rollout-*.jsonl.zst` migration name,
//! parses identical to the plain twin, doctor discloses the compressed share,
//! and raw-byte search decodes before scanning (counting decoded bytes).
//!
//! The falsification this closes (cycle-3 live PoC at 40425749): a 4-rollout
//! corpus showed `--sessions` 2-of-4 and `--doctor found=2 failed=0` — the
//! `.zst`-named pair invisible with zero disclosure while the decode lane was
//! fine. Fixture trio per tests/fixtures/codex-zstd/README.md.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use agenttrace_core::{
    build_doctor_report, find_session_files, parse_file, search_sessions_detailed,
};

/// rm-451 residual arm: the replay scan raw read is bounded; an
/// over-bound rollout is refused loudly — disclosed on the session
/// (doctor folds `disclosure_counters`), counted in the stats — instead
/// of being read whole. Never a crash, never fabricated evidence.
#[test]
fn replay_scan_refuses_oversized_rollouts_loudly() {
    let dir = std::env::temp_dir().join(format!("agenttrace-rm451-overcap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let giant = dir.join("rollout-giant.jsonl");
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&giant).unwrap();
        // Real opening bytes + `set_len(64 MiB + 1)`: the stat-size
        // refusal fires without reading the over-bound body — the scan
        // must refuse on metadata alone, never ingest the giant.
        file.write_all(b"{\"type\":\"response_item\",\"payload\":{}}\n")
            .unwrap();
        file.set_len(64 * 1024 * 1024 + 1).unwrap();
    }

    // A real parsed codex session repointed at the over-bound rollout:
    // the lane gate (`source_tool == "codex_cli"`) passes — it is the
    // scan read that must refuse.
    let mut session = parse_file(&fixture(&[
        "tests",
        "fixtures",
        "codex-zstd",
        "rollout.jsonl",
    ]))
    .expect("fixture rollout parses");
    session.path = giant.to_string_lossy().to_string();
    let before = session.metrics.clone();
    let mut sessions = vec![session];
    let stats = agenttrace_core::reconcile_codex_multiagent_replays(&mut sessions);
    assert_eq!(
        stats.scans_refused_oversized, 1,
        "the over-bound read must be refused AND counted: {stats:?}"
    );
    assert_eq!(
        sessions[0]
            .metrics
            .disclosure_counters
            .get("codex_replay_scan_refused_oversized"),
        Some(&1),
        "the refusal must be doctor-visible on the session"
    );
    assert_eq!(
        stats.families, 0,
        "no evidence fabricated from the refused file"
    );
    // Zero behavior change for everything except the disclosure channel:
    // usage fields on the refused session are byte-identical.
    let after = &sessions[0].metrics;
    assert_eq!(after.tokens_input, before.tokens_input);
    assert_eq!(after.tokens_output, before.tokens_output);
    assert_eq!(after.cost_estimated, before.cost_estimated);

    let _ = std::fs::remove_dir_all(&dir);
}

fn fixture(parts: &[&str]) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.extend(parts);
    p
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn sessions_dir(home: &Path) -> PathBuf {
    home.join(".codex/sessions/2026/10/18")
}

/// Hermetic HOME (discovery_contract pattern) with a populated codex lane:
/// `.codex/sessions/2026/10/18/` holding the compressed trio plus a bare
/// `.zst` decoy that must stay unadmitted.
fn populate(home: &Path) {
    let sessions = sessions_dir(home);
    std::fs::create_dir_all(&sessions).unwrap();
    for (src, dst) in [
        (
            fixture(&["tests", "fixtures", "codex-zstd", "rollout.jsonl"]),
            "rollout.jsonl",
        ),
        (
            fixture(&[
                "tests",
                "fixtures",
                "codex-zstd",
                "rollout-migrated.jsonl.zst",
            ]),
            "rollout-migrated.jsonl.zst",
        ),
        (
            fixture(&["tests", "fixtures", "codex-custom-tools", "rollout.jsonl"]),
            "rollout-plain.jsonl",
        ),
    ] {
        std::fs::copy(&src, sessions.join(dst)).unwrap();
    }
    // Bare `.zst` (no `.jsonl` stem): no agent ships this shape — stays out.
    std::fs::copy(
        fixture(&["tests", "fixtures", "codex-zstd", "rollout.jsonl"]),
        sessions.join("rollout-bare.zst"),
    )
    .unwrap();
}

fn with_hermetic_home<F: FnOnce(&Path)>(f: F) {
    let _guard = env_lock().lock().unwrap();
    let home = std::env::temp_dir().join(format!(
        "agenttrace-codex-zstd-admission-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&home);
    populate(&home);
    let restore = |k: &str, v: Option<String>| match v {
        Some(v) => std::env::set_var(k, v),
        None => std::env::remove_var(k),
    };
    let old = [
        ("HOME", std::env::var("HOME").ok()),
        ("XDG_DATA_HOME", std::env::var("XDG_DATA_HOME").ok()),
        ("XDG_STATE_HOME", std::env::var("XDG_STATE_HOME").ok()),
        ("CODEX_HOME", std::env::var("CODEX_HOME").ok()),
    ];
    std::env::set_var("HOME", &home);
    std::env::remove_var("XDG_DATA_HOME");
    std::env::remove_var("XDG_STATE_HOME");
    std::env::remove_var("CODEX_HOME");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&home)));
    for (k, v) in old {
        restore(k, v);
    }
    let _ = std::fs::remove_dir_all(&home);
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// Independent-review fix F3 (review 80dcb0be, run 7e61b07e cycle 3): the
/// `.jsonl.zst` name-gate arm is LANE-GLOBAL — `is_session_file_name`
/// (discovery.rs) is the shared mechanism for every walk root (codex,
/// claude, opencode), so a compressed journal under a NON-codex lane
/// root is admitted too. Decision of record: kept lane-global rather
/// than codex-root-scoped, because the CONTENT lane was already
/// lane-global before this batch (zstd bytes under a `.jsonl` name
/// decode in any lane via the magic sniff + bounded decode); scoping
/// only the name gate would make admission lane-dependent while
/// decoding stays content-driven. This pin holds the decision on the
/// claude lane root. Fixture bytes reuse the codex-zstd pair — the gate
/// under test is name-based, and parse parity needs a plain twin.
#[test]
fn name_gate_is_lane_global_claude_root_admits_jsonl_zst() {
    let dir = std::env::temp_dir().join(format!(
        "agenttrace-rm992-lane-global-{}-{}",
        std::process::id(),
        thread_name_or_id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    // A claude lane root shape: `.claude/projects/<project>/` holding a
    // plain journal plus its compressed migration twin.
    let claude = dir.join(".claude/projects/demo");
    std::fs::create_dir_all(&claude).unwrap();
    for (src, dst) in [
        (
            fixture(&["tests", "fixtures", "codex-custom-tools", "rollout.jsonl"]),
            "session-plain.jsonl",
        ),
        (
            fixture(&[
                "tests",
                "fixtures",
                "codex-zstd",
                "rollout-migrated.jsonl.zst",
            ]),
            "session-migrated.jsonl.zst",
        ),
    ] {
        std::fs::copy(&src, claude.join(dst)).unwrap();
    }

    let found = find_session_files(Some(&claude));
    let names: Vec<String> = found
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(
        names.iter().any(|n| n == "session-migrated.jsonl.zst"),
        "the .jsonl.zst name gate must admit under a non-codex lane root \
         (the mechanism is lane-global, discovery.rs is_session_file_name): {names:?}"
    );

    // Decode-by-content is lane-global as well: the compressed twin
    // parses identical to its plain sibling regardless of the root.
    let plain = parse_file(&claude.join("session-plain.jsonl")).unwrap();
    let migrated = parse_file(&claude.join("session-migrated.jsonl.zst")).unwrap();
    assert_eq!(migrated.name, plain.name);
    assert_eq!(
        migrated.metrics.tokens_input, plain.metrics.tokens_input,
        "decode parity must not depend on the lane root"
    );
    assert_eq!(migrated.metrics.tokens_output, plain.metrics.tokens_output);

    let _ = std::fs::remove_dir_all(&dir);
}

fn thread_name_or_id() -> String {
    format!("{:?}", std::thread::current().id())
}

#[test]
fn name_gate_admits_migration_shape_and_excludes_bare_zst() {
    // `find_session_files(None)` resolves the hermetic HOME itself; the
    // closure parameter is intentionally unused here (underscore).
    with_hermetic_home(|_home| {
        let found = find_session_files(None);
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        for expected in [
            "rollout.jsonl",
            "rollout-migrated.jsonl.zst",
            "rollout-plain.jsonl",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "admitted set missing {expected}: {names:?}"
            );
        }
        assert!(
            !names.iter().any(|n| n.ends_with("rollout-bare.zst")),
            "bare .zst must stay unadmitted: {names:?}"
        );
    });
}

#[test]
fn migrated_rollout_parses_identical_to_plain_twin() {
    with_hermetic_home(|home| {
        let dir = sessions_dir(home);
        let plain = parse_file(&dir.join("rollout-plain.jsonl")).unwrap();
        let zstd_named = parse_file(&dir.join("rollout.jsonl")).unwrap();
        let migrated = parse_file(&dir.join("rollout-migrated.jsonl.zst")).unwrap();
        for decoded in [zstd_named, migrated] {
            assert_eq!(decoded.name, plain.name);
            assert_eq!(decoded.metrics.events_total, plain.metrics.events_total);
            assert_eq!(decoded.metrics.tokens_input, plain.metrics.tokens_input);
            assert_eq!(decoded.metrics.tokens_output, plain.metrics.tokens_output);
        }
    });
}

#[test]
fn doctor_discloses_compressed_share_of_codex_lane() {
    with_hermetic_home(|home| {
        let report = build_doctor_report(Some(&sessions_dir(home)), false);
        let codex = report
            .directories
            .iter()
            .find(|d| d.path.contains(".codex"))
            .expect("codex lane dir in doctor report");
        assert_eq!(codex.files, 3, "3 admitted journal files: {:?}", codex);
        assert_eq!(codex.parsed, 3, "all three must parse: {:?}", codex);
        assert_eq!(
            codex.compressed, 1,
            "exactly the .jsonl.zst migration name counts as compressed: {:?}",
            codex
        );
    });
}

#[test]
fn search_decodes_compressed_bytes_before_scanning() {
    with_hermetic_home(|home| {
        let dir = sessions_dir(home);
        // A needle that exists only in the DECODED journal text — the raw
        // compressed bytes never contain it (proves decode-before-scan).
        let sessions = vec![
            parse_file(&dir.join("rollout-plain.jsonl")).unwrap(),
            parse_file(&dir.join("rollout.jsonl")).unwrap(),
            parse_file(&dir.join("rollout-migrated.jsonl.zst")).unwrap(),
        ];
        let report = search_sessions_detailed(&sessions, "token_count", 20, 1 << 20, 8 << 20);
        let hit_paths: Vec<&str> = report
            .results
            .iter()
            .map(|r| r.path.rsplit('/').next().unwrap_or(&r.path))
            .collect();
        assert!(
            hit_paths
                .iter()
                .any(|p| p.ends_with("rollout-migrated.jsonl.zst")),
            "compressed migration twin must yield hits: {hit_paths:?}"
        );
        assert!(
            hit_paths.iter().any(|p| p.ends_with("rollout.jsonl")),
            "zstd-content .jsonl must yield hits: {hit_paths:?}"
        );
        assert!(
            report.scan.scanned_compressed >= 2,
            "decode-share counted: {:?}",
            report.scan
        );
    });
}

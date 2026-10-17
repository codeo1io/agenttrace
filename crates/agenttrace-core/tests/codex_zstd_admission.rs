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
        (fixture(&["tests", "fixtures", "codex-zstd", "rollout.jsonl"]), "rollout.jsonl"),
        (
            fixture(&["tests", "fixtures", "codex-zstd", "rollout-migrated.jsonl.zst"]),
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

#[test]
fn name_gate_admits_migration_shape_and_excludes_bare_zst() {
    with_hermetic_home(|home| {
        let found = find_session_files(None).unwrap();
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
        let report = build_doctor_report(Some(home), false).unwrap();
        let codex = report
            .dirs
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
        let report =
            search_sessions_detailed(&sessions, "token_count", 20, 1 << 20, 8 << 20);
        let hit_paths: Vec<&str> = report
            .results
            .iter()
            .map(|r| r.path.rsplit('/').next().unwrap_or(&r.path))
            .collect();
        assert!(
            hit_paths.iter().any(|p| p.ends_with("rollout-migrated.jsonl.zst")),
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

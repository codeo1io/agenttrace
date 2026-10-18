//! rm-992 + rm-249 integration: codex `.jsonl.zst` admission end-to-end
//! and discovery-walk refusal disclosure.
//!
//! Fleet falsification context (run e7b57840 cycle 3, binary 40425749): a
//! 4-rollout corpus admitted only 2-of-4 — the `.jsonl.zst` pair was
//! invisible to discovery (`discovery.rs` name gate) while the decode lane
//! already existed. These tests pin the cured behavior: admission by name,
//! end-to-end parse identity, decompressed byte accounting in `--search`,
//! loud failure on truncated frames, and walk-refusal disclosure.

use agenttrace_core::discovery::{collect_session_files_with_disclosure, is_session_file_name};
use agenttrace_core::parser::parse_file;
use agenttrace_core::search::search_sessions_detailed;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex-zst-admission")
}

fn temp_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "at-zst-admit-{}-{}",
        tag,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn name_gate_admits_jsonl_zst_and_only_that_zst_shape() {
    assert!(is_session_file_name(
        "rollout-2026-10-18T01-00-00-x.jsonl.zst"
    ));
    assert!(is_session_file_name("journal.jsonl"));
    // A bare `.zst` is not a codex rollout shape — it must stay rejected
    // (the gate names the codex writer convention, not a compression
    // heuristic).
    assert!(!is_session_file_name("archive.zst"));
    assert!(!is_session_file_name("rollout.json.zst"));
}

#[test]
fn discovery_admits_zst_named_rollouts_four_of_four() {
    let root = temp_root("walk");
    for name in [
        "plain-a.jsonl",
        "plain-b.jsonl",
        "zst-c.jsonl.zst",
        "zst-bad.jsonl.zst",
    ] {
        fs::copy(fixture_dir().join(name), root.join(name)).unwrap();
    }
    let (files, disclosure) = collect_session_files_with_disclosure(&root);
    let mut names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "plain-a.jsonl",
            "plain-b.jsonl",
            "zst-bad.jsonl.zst",
            "zst-c.jsonl.zst",
        ],
        "rm-992 A1: .jsonl.zst rollouts must be admitted by name — the fleet falsification saw 2-of-4"
    );
    assert!(disclosure.is_clean());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn zst_rollout_parses_end_to_end_with_identity() {
    let session = parse_file(&fixture_dir().join("zst-c.jsonl.zst"))
        .expect("zstd rollout decodes and parses (decode lane pre-existed)");
    assert!(
        session.id.contains("000000000003"),
        "session id carried out of the compressed rollout: {}",
        session.id
    );
    let plain = parse_file(&fixture_dir().join("plain-a.jsonl"))
        .expect("plain rollout parses");
    assert!(plain.id.contains("000000000001"));
}

#[test]
fn truncated_zst_frame_fails_loudly_not_silently() {
    let result = parse_file(&fixture_dir().join("zst-bad.jsonl.zst"));
    assert!(
        result.is_err(),
        "rm-992 A2: a truncated zstd frame must surface a parse error (doctor's failure_samples lane carries the reason)"
    );
}

#[test]
fn search_reports_decompressed_bytes_for_zst_journals() {
    let session = parse_file(&fixture_dir().join("zst-c.jsonl.zst")).unwrap();
    let report = search_sessions_detailed(&[session], "probe", 10, 1 << 20, 1 << 20);
    assert_eq!(report.scan.zstd_decoded_files, 1);
    assert_eq!(report.scan.skipped_zstd_decode, 0);
    // The fixture decompresses to exactly 861 bytes; the on-disk frame is
    // 417. The scan must count what it READS (decompressed), not what the
    // directory listing says (compressed) — the fleet falsification caught
    // the compressed-byte lie (rm-1872 arm).
    assert_eq!(
        report.scan.scanned_bytes, 861,
        "rm-992 A3: scanned bytes must count decompressed size for zst journals"
    );
}

#[cfg(unix)]
#[test]
fn unreadable_directory_is_disclosed_not_silent() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp_root("unreadable");
    let hidden = root.join("locked");
    fs::create_dir_all(&hidden).unwrap();
    fs::write(hidden.join("inside.jsonl"), "{}\n").unwrap();
    fs::set_permissions(&hidden, fs::Permissions::from_mode(0o000)).unwrap();
    // Self-detecting privilege probe: when the environment ignores mode
    // bits (root, or a permissive container runtime), the refusal cannot
    // be reproduced — restore and skip rather than lie about coverage.
    if fs::read_dir(&hidden).is_ok() {
        fs::set_permissions(&hidden, fs::Permissions::from_mode(0o755)).unwrap();
        let _ = fs::remove_dir_all(&root);
        return;
    }
    let (files, disclosure) = collect_session_files_with_disclosure(&root);
    assert!(
        files.is_empty(),
        "the locked subtree must yield no files (unchanged behavior)"
    );
    assert!(
        !disclosure.is_clean(),
        "rm-249 B1: the walk refusal must be disclosed, never silently dropped"
    );
    assert!(
        disclosure
            .unreadable_dirs
            .iter()
            .any(|p| p.contains("locked")),
        "the sample must name the refused directory: {:?}",
        disclosure.unreadable_dirs
    );
    fs::set_permissions(&hidden, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(&root);
}

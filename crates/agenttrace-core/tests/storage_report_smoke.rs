//! rm-692 storage-report smoke fixtures.
//!
//! The in-module unit tests (`crates/agenttrace-core/src/storage.rs`) pin
//! the walk primitives (sidecar naming, symlink skips). These integration
//! tests drive the assembled `storage_report_with_roots` end to end over a
//! synthetic provider/cache fixture — symlinked root, overlapping roots
//! that dedup, WAL/SHM sidecars, nested directory symlinks — and pin the
//! two contract properties operators care about: the report is READ-ONLY
//! (the walked tree is byte-identical before and after — SQLite sidecars
//! are counted by name and never opened, which matters because OPENING a
//! database is what creates `-wal`/`-shm` files) and its JSON is stable.

use agenttrace_core::{storage_report, storage_report_with_roots, KnownSessionDir};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "agenttrace-rm692-smoke-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Recursive path → (is_symlink, size) snapshot used to prove the walk
/// mutated nothing (no new files, no size changes, no sidecar creation).
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (bool, u64)> {
    let mut map = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).unwrap();
            let is_symlink = meta.file_type().is_symlink();
            map.insert(
                path.strip_prefix(root).unwrap().to_path_buf(),
                (is_symlink, meta.len()),
            );
            if !is_symlink && meta.is_dir() {
                stack.push(path);
            }
        }
    }
    map
}

fn known(name: &str, path: &Path) -> KnownSessionDir {
    KnownSessionDir {
        name: name.to_string(),
        path: path.to_path_buf(),
    }
}

#[cfg(unix)]
#[test]
fn synthetic_roots_count_sidecars_dedup_and_stay_read_only() {
    let base = scratch("full");
    // Provider tree A: five files incl. both SQLite sidecars, one nested.
    let tree_a = base.join("real-a");
    fs::create_dir_all(tree_a.join("nested")).unwrap();
    fs::write(tree_a.join("a.jsonl"), "0123456789").unwrap(); // 10 B
    fs::write(tree_a.join("big.jsonl"), vec![b'x'; 2048]).unwrap();
    fs::write(tree_a.join("nested").join("two.jsonl"), vec![b'y'; 32]).unwrap();
    fs::write(tree_a.join("opencode.db-wal"), vec![b'w'; 128]).unwrap();
    fs::write(tree_a.join("opencode.db-shm"), vec![b's'; 64]).unwrap();

    // Provider tree B: one real file, one self-referential directory
    // symlink (loop safety — must be skipped, never followed) and one
    // dangling file symlink (disclosed).
    let tree_b = base.join("real-b");
    fs::create_dir_all(&tree_b).unwrap();
    fs::write(tree_b.join("h.jsonl"), vec![b'h'; 100]).unwrap();
    std::os::unix::fs::symlink(&tree_b, tree_b.join("loop")).unwrap();
    std::os::unix::fs::symlink(base.join("no-such-target"), tree_b.join("dang.jsonl")).unwrap();

    // A symlinked ROOT: the root itself is a symlink, resolved exactly
    // once before walking; a second root naming the same underlying tree
    // must dedup into a note, not double-count; a missing root is a
    // normal absent provider, not a skip.
    let link_to_a = base.join("link-to-a");
    std::os::unix::fs::symlink(&tree_a, &link_to_a).unwrap();
    let cache = base.join("cache");
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("sessions.json"), "{}{}").unwrap(); // 4 B
    fs::write(cache.join("journal.bin"), vec![b'j'; 8]).unwrap();

    let roots = vec![
        known("Claude Code", &link_to_a),
        known("Claude Code duplicate", &tree_a),
        known("Hermes Agent", &tree_b),
        known("Ghost provider", &base.join("missing-root")),
    ];

    let before = snapshot(&base);
    let report = storage_report_with_roots(&roots, &cache);
    let after = snapshot(&base);
    assert_eq!(
        before, after,
        "the storage walk must be read-only: no new files, no size or type changes anywhere under the fixture"
    );

    // Provider rows: deduped (A counted once via its symlink) and the
    // missing root absent.
    assert_eq!(report.providers.len(), 2, "three live roots, one deduped");
    let claude = &report.providers[0];
    assert_eq!(claude.name, "Claude Code");
    assert_eq!(claude.root, link_to_a, "row carries the configured root");
    assert_eq!(
        claude.resolved_root,
        fs::canonicalize(&tree_a).unwrap(),
        "symlinked root resolves exactly once"
    );
    assert_eq!(claude.files, 5, "five regular files under tree A");
    assert_eq!(claude.logical_bytes, 10 + 2048 + 32 + 128 + 64);
    assert_eq!(claude.sqlite_sidecars, 2, "WAL and SHM counted by name");
    assert_eq!(claude.sqlite_sidecar_bytes, 128 + 64);

    let hermes = &report.providers[1];
    assert_eq!(hermes.name, "Hermes Agent");
    assert_eq!(
        hermes.files, 1,
        "the dir symlink and dangling link are not files"
    );
    assert_eq!(hermes.logical_bytes, 100);
    assert_eq!(hermes.sqlite_sidecars, 0);

    // Skips are disclosed and flip coverage to partial.
    assert_eq!(report.coverage, "partial");
    let reasons: Vec<&str> = report.skipped.iter().map(|s| s.reason.as_str()).collect();
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("nested directory symlink")),
        "self-referential dir symlink disclosed: {reasons:?}"
    );
    assert!(
        reasons.iter().any(|r| r.contains("dangling symlink")),
        "dangling symlink disclosed: {reasons:?}"
    );
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("canonicalizes into an already-walked root")),
        "overlapping root dedup is disclosed in notes: {:?}",
        report.notes
    );

    // Cache footprint: the cache dir itself is not a counted file.
    assert!(report.cache.exists);
    assert_eq!(report.cache.dir, cache);
    assert_eq!(report.cache.files, 2);
    assert_eq!(report.cache.bytes, 4 + 8);

    // Ten largest: all six files, biggest first, sorted descending.
    assert_eq!(report.ten_largest.len(), 6);
    assert_eq!(report.ten_largest[0].path, tree_a.join("big.jsonl"));
    assert_eq!(report.ten_largest[0].bytes, 2048);
    let sizes: Vec<u64> = report.ten_largest.iter().map(|f| f.bytes).collect();
    let mut sorted = sizes.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(sizes, sorted, "ten_largest must be sorted descending");

    // One filesystem row per distinct device: all fixture roots share one.
    assert_eq!(
        report.filesystems.len(),
        1,
        "same-device roots must collapse: {:?}",
        report.filesystems
    );
    assert!(
        report.filesystems[0].available_bytes.is_some(),
        "statfs on a live fixture directory must succeed"
    );

    // The report serializes; the coverage verdict is part of the schema.
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("\"coverage\":\"partial\""));
    assert!(json.contains("\"sqlite_sidecar_bytes\":192"));

    fs::remove_dir_all(&base).ok();
}

#[test]
fn clean_roots_report_full_coverage() {
    let base = scratch("clean");
    let tree = base.join("provider");
    fs::create_dir_all(&tree).unwrap();
    fs::write(tree.join("one.jsonl"), "data").unwrap();
    let cache = base.join("cache");
    fs::create_dir_all(&cache).unwrap(); // exists, empty

    let report = storage_report_with_roots(&[known("Clean provider", &tree)], &cache);
    assert_eq!(report.coverage, "full", "no skips on a clean fixture");
    assert!(report.skipped.is_empty());
    assert_eq!(report.providers.len(), 1);
    assert_eq!(report.providers[0].files, 1);
    assert!(report.cache.exists);
    assert_eq!(report.cache.files, 0, "empty cache dir counts zero files");
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("\"coverage\":\"full\""));

    fs::remove_dir_all(&base).ok();
}

#[test]
fn discovery_driven_report_holds_invariants_on_any_host() {
    // `storage_report` itself (HOME-driven discovery) cannot be pinned to
    // exact numbers on a shared host, so pin the host-independent contract:
    // the cache row is passed through verbatim, coverage always matches
    // the skip ledger, and every provider row reports a canonical root.
    let base = scratch("realhome");
    let cache = base.join("cache");
    fs::create_dir_all(&cache).unwrap();

    let report = storage_report(&cache);
    assert_eq!(report.cache.dir, cache);
    assert!(report.cache.exists);
    let expected_coverage = if report.skipped.is_empty() {
        "full"
    } else {
        "partial"
    };
    assert_eq!(
        report.coverage, expected_coverage,
        "coverage must agree with the skip ledger"
    );
    for provider in &report.providers {
        assert!(
            provider.resolved_root.is_absolute(),
            "resolved roots are canonicalized: {}",
            provider.resolved_root.display()
        );
    }

    fs::remove_dir_all(&base).ok();
}

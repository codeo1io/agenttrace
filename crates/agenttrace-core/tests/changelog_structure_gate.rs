//! Changelog structure-gate contract (rm-1928, run 9d3b927c cycle 1,
//! repository-maintenance 14f74994).
//!
//! `scripts/ci/check-changelog-structure.sh` guards CHANGELOG.md shape
//! in CI (`.github/workflows/ci.yml:83`). It validated version-heading
//! hygiene but accepted unresolved merge-conflict fences: the standing
//! tree's CHANGELOG.md has carried committed `<<<<<<<`/`=======`/
//! `>>>>>>>` markers since the 2026-10-14 fog (anchors :7/:106/:108 at
//! base e67d123) while `bash scripts/ci/check-changelog-structure.sh`
//! returned rc0 on it.
//!
//! The gate now REJECTS conflict fences BEFORE any version-heading
//! check, so a half-merged changelog can never pass CI. This suite
//! pins the gate against fixtures in a temp repo-shaped tree (the
//! script derives its repo root from its own location), never against
//! the live CHANGELOG.md — the standing markers are the commit gate's
//! to resolve, in gate order.
//!
//! Fence shapes rejected, mirroring git's conflict output exactly:
//! - `<<<<<<<` (label follows on the same line, e.g. `<<<<<<< HEAD`);
//! - `>>>>>>> ` (label follows after the space);
//! - `=======` (bare, exactly seven equals — a setext underline of
//!   three or four `=` stays legal).

use std::path::{Path, PathBuf};
use std::process::Command;

const HEALTHY: &str = "# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- a thing

### Fixed

- another thing
";

const RESOLVED_MERGE: &str = "# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- merged entry from HEAD

- merged entry from the branch

### Fixed

- resolved fix
";

fn temp_repo(tag: &str, changelog: &str) -> PathBuf {
    static REPOS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "agenttrace-rm1928-gate-{}-{tag}-{}",
        std::process::id(),
        REPOS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(root.join("scripts/ci")).expect("mkdir scripts/ci");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/ci/check-changelog-structure.sh"
        ),
        root.join("scripts/ci/check-changelog-structure.sh"),
    )
    .expect("copy gate script");
    std::fs::write(root.join("CHANGELOG.md"), changelog).expect("write fixture changelog");
    root
}

fn run_gate(root: &Path) -> (bool, String, String) {
    let output = Command::new("bash")
        .arg(root.join("scripts/ci/check-changelog-structure.sh"))
        .output()
        .expect("run gate script");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn healthy_changelog_passes_the_gate() {
    let root = temp_repo("healthy", HEALTHY);
    let (ok, _out, err) = run_gate(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(ok, "healthy fixture must pass: {err}");
}

#[test]
fn resolved_merge_passes_the_gate() {
    // The same entries as the conflicted fixtures, fences removed —
    // the gate must pass the RESOLVED file so the check cannot be
    // satisfied by dropping legitimate entries.
    let root = temp_repo("resolved", RESOLVED_MERGE);
    let (ok, _out, err) = run_gate(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(ok, "resolved fixture must pass: {err}");
}

#[test]
fn ours_fence_reds_the_gate() {
    let conflicted = HEALTHY.replace(
        "### Fixed\n\n- another thing\n",
        "### Fixed\n\n<<<<<<< HEAD\n- another thing\n=======\n- the branch's thing\n>>>>>>> feature/fog-reconcile\n",
    );
    let root = temp_repo("ours", &conflicted);
    let (ok, _out, err) = run_gate(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(!ok, "unresolved '<<<<<<<' fence must fail the gate");
    assert!(
        err.contains("conflict"),
        "failure must name the conflict fence class, got: {err}"
    );
}

#[test]
fn theirs_fence_reds_the_gate() {
    let conflicted = HEALTHY.replace(
        "### Added\n\n- a thing\n",
        "### Added\n\n>>>>>>> feature/fog-reconcile\n- a thing\n",
    );
    let root = temp_repo("theirs", &conflicted);
    let (ok, _out, err) = run_gate(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(!ok, "unresolved '>>>>>>> ' fence must fail the gate");
    assert!(
        err.contains("conflict"),
        "failure must name the conflict fence class, got: {err}"
    );
}

#[test]
fn bare_separator_fence_reds_the_gate() {
    let conflicted = HEALTHY.replace(
        "### Fixed\n\n- another thing\n",
        "### Fixed\n\n- one side\n=======\n- other side\n",
    );
    let root = temp_repo("separator", &conflicted);
    let (ok, _out, err) = run_gate(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(!ok, "bare '=======' fence must fail the gate");
    assert!(
        err.contains("conflict"),
        "failure must name the conflict fence class, got: {err}"
    );
}

#[test]
fn setext_underline_stays_legal() {
    // Three `=` under a title line is a markdown setext H2 underline,
    // not a git fence: exactly seven equals is the fence shape.
    let underlined = "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\n## [Unreleased]\n\n### Added\n\n- a thing\n\nSection\n===\n";
    let root = temp_repo("setext", underlined);
    let (ok, _out, err) = run_gate(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(
        ok,
        "3-'=' setext underline must not trip the fence check: {err}"
    );
}

/// Verbatim-duplicated entry lines inside one section are re-doubled
/// landings (rm-2338 class: 39 duplicated entry blocks / 132 duplicated
/// long lines found in the live [Unreleased] on 2026-10-22, deduplicated
/// by run 252064e2 cycle 4): the gate must fail and name the class.
#[test]
fn duplicate_entry_line_reds_the_gate() {
    let entry =
        "- SQLite snapshot cache: failed database reads never bank a stale snapshot into the cache.";
    let doubled = HEALTHY.replace(
        "### Fixed\n\n- another thing\n",
        format!("### Fixed\n\n{entry}\n\n{entry}\n").as_str(),
    );
    let root = temp_repo("entry-dup", &doubled);
    let (ok, _out, err) = run_gate(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(!ok, "verbatim-duplicated entry line must fail the gate");
    assert!(
        err.contains("duplicate entry line"),
        "failure must name the entry-duplication class, got: {err}"
    );
}

/// Class boundary: short bullets under the significance floor and repeated
/// `Integration seam (...)` continuation annotations inside DIFFERENT entries
/// are legitimate shapes (the live CHANGELOG carries two of the latter after
/// the 2026-10-22 dedup) and must NOT trip the arm.
#[test]
fn short_bullets_and_seam_annotations_stay_legal() {
    let legal = HEALTHY.replace(
        "### Fixed\n\n- another thing\n",
        "### Fixed\n\n- fixed.\n\n- fixed.\n\n\
- parser: keeps transcript replay stable across cold caches (2026-10-21, fleet merge).\n  Integration seam (2026-10-21, conflict case 0f0f0f0f): seam annotation repeated.\n\n\
- discovery: keeps the roots census stable across cold caches (2026-10-21, fleet merge).\n  Integration seam (2026-10-21, conflict case 0f0f0f0f): seam annotation repeated.\n",
    );
    let root = temp_repo("entry-dup-boundary", &legal);
    let (ok, _out, err) = run_gate(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(
        ok,
        "short bullets and repeated seam annotations must not trip the arm: {err}"
    );
}

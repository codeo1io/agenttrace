//! rm-1892 (run 77ca5de0 cycle 5): the relative-link scan gate,
//! pinned by fixture both ways.
//!
//! docs/guides/usage-card.md:47 pointed at ../pricing.md, which has
//! never existed in this repository's history — the docs scan read
//! "99 links / 1 broken" for as long as the card shipped. The rider
//! retargets the link to the README pricing anchor and lands
//! scripts/ci/check-docs-links.sh so the broken-count can never
//! quietly regress: the gate fails the build on any tracked *.md
//! relative link whose target (minus anchor) does not resolve on
//! disk.
//!
//! Both legs run the real gate script end to end:
//! - green: the shipped tree resolves every relative link
//!   (broken-count 1 -> 0, asserted here rather than trusted);
//! - red-first: a fixture repository whose tracked markdown holds a
//!   dangling relative link fails the gate, naming the file and the
//!   missing target.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ci/check-docs-links.sh")
}

#[test]
fn gate_passes_on_the_shipped_tree() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("bash")
        .arg(script_path())
        .current_dir(&repo_root)
        .output()
        .expect("run check-docs-links.sh");
    assert!(
        output.status.success(),
        "the shipped tree must resolve every tracked relative link\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("all relative links resolve"),
        "the green leg must be the gate's own success line"
    );
    // The rider's specific cure: the usage-card link now points at
    // the README pricing anchor, which exists.
    let card = fs::read_to_string(repo_root.join("docs/guides/usage-card.md"))
        .expect("read usage-card.md");
    assert!(
        !card.contains("../pricing.md"),
        "the dead ../pricing.md link must stay gone"
    );
    let readme = fs::read_to_string(repo_root.join("README.md")).expect("read README.md");
    assert!(
        readme.contains("<a id=\"pricing\"></a>") || readme.contains("id=\"pricing\""),
        "the README pricing anchor the card retargets to must exist"
    );
}

#[test]
fn gate_fails_on_a_dangling_relative_link_fixture() {
    // Red-first pin: a tracked markdown link to a missing file fails
    // the gate and names the culprit. Built as its own throwaway git
    // repository because the gate scans TRACKED files only.
    let repo = std::env::temp_dir().join(format!("at-docs-links-fixture-{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    fs::create_dir_all(repo.join("docs/guides")).expect("create fixture repo");
    fs::write(
        repo.join("docs/guides/gone.md"),
        "# Guide\n\nSee [the pricing page](../pricing.md) for rates.\n",
    )
    .expect("write fixture markdown");
    fs::write(
        repo.join("README.md"),
        "# Fixture\n\n[ok](docs/guides/gone.md)\n",
    )
    .expect("write fixture readme");
    let git = |args: &[&str]| {
        Command::new("git")
            .current_dir(&repo)
            .args(["-c", "user.name=fixture"])
            .args(["-c", "user.email=fixture@example.invalid"])
            .args(args)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    };
    assert!(
        git(&["init", "-q"]) && git(&["add", "."]) && git(&["commit", "-q", "-m", "fixture"]),
        "fixture repo must commit its tracked markdown"
    );
    let output = Command::new("bash")
        .arg(script_path())
        .current_dir(&repo)
        .output()
        .expect("run gate against fixture");
    assert!(
        !output.status.success(),
        "a dangling relative link must fail the gate"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("gone.md") && stderr.contains("pricing.md"),
        "the failure must name the file and the missing target: {stderr}"
    );
    let _ = fs::remove_dir_all(&repo);
}

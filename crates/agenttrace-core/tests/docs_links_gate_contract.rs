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

/// rm-1894 (run 5cbd75e3 cycle 2): both rider legs, fixture-pinned.
/// The scan gap was syntactic — reference-style definitions were
/// invisible to it — and the existence test was `-f`, which reads a
/// DIRECTORY target as broken. Each leg builds its own throwaway
/// repository because the gate scans TRACKED files only.
fn fixture_repo(tag: &str) -> std::path::PathBuf {
    let repo = std::env::temp_dir().join(format!("at-docs-links-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    fs::create_dir_all(repo.join("docs")).expect("create fixture repo");
    // A placeholder tracked file: an empty tree (bare directory) has
    // nothing to commit and the fixture assert would misfire.
    fs::write(repo.join("README.md"), "# Fixture\n").expect("write placeholder readme");
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
    repo
}

#[test]
fn gate_fails_on_a_dangling_reference_style_link_fixture() {
    // Red-first for the rm-1894 form gap: every INLINE link resolves,
    // so the pre-rider gate left this tree green — only the
    // `[label]: destination` definition dangles.
    let repo = std::env::temp_dir().join(format!("at-docs-links-refstyle-{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    fs::create_dir_all(repo.join("docs")).expect("create fixture repo");
    fs::write(
        repo.join("README.md"),
        "# Fixture\n\nSee [the guide][guide].\n\n[guide]: docs/gone.md\n",
    )
    .expect("write fixture readme");
    fs::write(repo.join("docs/here.md"), "# Here\n").expect("write a real target");
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
        .expect("run gate against reference-style fixture");
    assert!(
        !output.status.success(),
        "a dangling reference-style definition must fail the gate"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("README.md") && stderr.contains("docs/gone.md"),
        "the failure must name the file and the missing reference-style target: {stderr}"
    );
    let _ = fs::remove_dir_all(&repo);
}

#[test]
fn gate_accepts_directory_targets() {
    // Red-first for the rm-1894 `-f` misfire: a link to an existing
    // DIRECTORY (`](docs)`) is legitimate — the pre-rider gate read it
    // as broken and would have failed any tree that used one.
    let repo = fixture_repo("dir-target");
    fs::write(
        repo.join("docs/here.md"),
        "# Here\n\nBack to [the docs root](..).\n",
    )
    .expect("write directory-target link");
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
        git(&["add", "."]) && git(&["commit", "-q", "-m", "dir target"]),
        "fixture must commit the directory-target link"
    );
    let output = Command::new("bash")
        .arg(script_path())
        .current_dir(&repo)
        .output()
        .expect("run gate against directory-target fixture");
    assert!(
        output.status.success(),
        "a directory target is a valid relative link: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(&repo);
}

#[test]
fn gate_lints_the_standing_fixture_for_every_covered_form() {
    // The standing fixture doc (docs/quality-gates/relative-link-
    // forms.md) exercises every form in the TREE the gate lints. It
    // only joins `git ls-files` at commit time, so this leg pins the
    // doc's SHAPE by direct containment: both syntaxes and all three
    // semantics must be present in the file the gate will lint once
    // the batch lands. The behavioral legs above prove the gate's
    // handling; this leg keeps the fixture from silently shedding a
    // covered form in a future edit.
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc = fs::read_to_string(repo_root.join("docs/quality-gates/relative-link-forms.md"))
        .expect("read the standing fixture doc");
    // Inline file target.
    assert!(doc.contains("(../../README.md)"), "inline file target");
    // Inline DIRECTORY target — the `-e` semantics.
    assert!(
        doc.contains("(../quality-gates)"),
        "inline directory target"
    );
    // Reference-style definitions, including a fragment target — the
    // syntax and the stripped-anchor semantics.
    assert!(doc.contains("[contributing]: ../../CONTRIBUTING.md"));
    assert!(doc.contains("[changelog head]: ../../CHANGELOG.md#unreleased"));
    assert!(doc.contains("[quality-gates dir]: ./"));
}

//! rm-931: directories the walker cannot read must be REPORTED, never
//! silently conflated with an empty corpus.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use agenttrace_core::{find_session_files, UnreadableDir};

/// Restores `0o700` on drop so the tempdir cleanup succeeds even when a
/// test panics while the directory is still sealed.
struct Unsealed(std::path::PathBuf);
impl Drop for Unsealed {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
    }
}

fn seal(dir: &Path) -> Unsealed {
    fs::set_permissions(dir, fs::Permissions::from_mode(0o000)).expect("seal dir");
    Unsealed(dir.to_path_buf())
}

/// True when the test process can read through a 0o000 directory
/// (i.e. we are root); the probe below is then meaningless.
fn running_as_root() -> bool {
    let probe = std::env::temp_dir().join(format!("rm931-root-probe-{}", std::process::id()));
    fs::create_dir_all(&probe).expect("create probe dir");
    let _guard = seal(&probe);
    fs::read_dir(&probe).is_ok()
}

fn write_session(dir: &Path, name: &str) {
    fs::create_dir_all(dir).expect("create session dir");
    fs::write(
        dir.join(name),
        concat!(
            r#"{"type":"user","timestamp":"2026-10-10T10:00:00Z","message":{"role":"user","content":"hi"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-10-10T10:00:01Z","message":{"id":"m1","model":"claude-sonnet-4-5","content":[{"type":"text","text":"ok"}],"usage":{"input_tokens":10,"output_tokens":1}}}"#,
            "\n",
        ),
    )
    .expect("write session file");
}

fn tmp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rm931-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create root");
    dir
}

fn reasons_mention_dir(unreadable: &[UnreadableDir], name: &str) -> bool {
    unreadable
        .iter()
        .any(|d| d.path.file_name().map(|f| f == name).unwrap_or(false))
}

#[test]
fn unreadable_subdir_is_reported_alongside_discovered_files() {
    if running_as_root() {
        eprintln!("skipping: running as root, chmod 0o000 is not enforced");
        return;
    }
    let root = tmp_root("subdir");
    write_session(&root.join("projects/alpha"), "a.jsonl");
    fs::create_dir_all(root.join("projects/sealed-inner")).expect("create sealed");
    write_session(&root.join("projects/sealed-inner"), "hidden.jsonl");
    let _guard = seal(&root.join("projects/sealed-inner"));

    let (files, unreadable) = find_session_files(Some(&root));
    assert!(
        files.iter().any(|f| f.ends_with("a.jsonl")),
        "readable session must still be discovered: {files:?}"
    );
    assert!(
        !files.iter().any(|f| f.ends_with("hidden.jsonl")),
        "sealed file must not be discovered"
    );
    assert_eq!(
        unreadable.len(),
        1,
        "sealed dir must be reported: {unreadable:?}"
    );
    assert!(reasons_mention_dir(&unreadable, "sealed-inner"));
    assert!(
        unreadable[0].reason.contains("failed to read directory"),
        "reason carries the OS context: {}",
        unreadable[0].reason
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn fully_unreadable_root_reports_itself_not_silence() {
    if running_as_root() {
        eprintln!("skipping: running as root, chmod 0o000 is not enforced");
        return;
    }
    let root = tmp_root("all");
    write_session(&root.join("inner"), "a.jsonl");
    let _guard = seal(&root);

    let (files, unreadable) = find_session_files(Some(&root));
    assert!(files.is_empty(), "nothing readable under a sealed root");
    assert_eq!(
        unreadable.len(),
        1,
        "the sealed root itself must be reported"
    );
    assert_eq!(unreadable[0].path, root);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn fully_readable_tree_reports_no_unreadable_dirs() {
    let root = tmp_root("clean");
    write_session(&root.join("projects/alpha"), "a.jsonl");
    write_session(&root.join("projects/beta"), "b.jsonl");

    let (files, unreadable) = find_session_files(Some(&root));
    assert_eq!(files.len(), 2, "both sessions discovered: {files:?}");
    assert!(
        unreadable.is_empty(),
        "clean tree must not report unreadable dirs: {unreadable:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

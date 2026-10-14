//! rm-1374 acceptance (binary-level): a broken HOME never plants history
//! under the CWD. `HOME= agenttrace --overview --preserve-history` used to
//! write `./.local/share/agenttrace/history.json` into whatever directory
//! the tool ran from (control-proven at 904f0f3), because the data lane's
//! HOME fallback joined `.local/share` onto an empty path while the XDG
//! arm already refused relative values (rm-248). Empty and relative HOME
//! are now treated as unset: the history root falls back to the system
//! temp root — the spec leaves `$HOME/.local/share` undefined without a
//! usable HOME — and the CWD stays byte-clean.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PLANTED_SESSION: &str = concat!(
    "{\"type\":\"user\",\"timestamp\":\"2026-10-04T01:00:00Z\",\"cwd\":\"/tmp/guard\",\"message\":{\"role\":\"user\",\"content\":\"check the billing please\"}}\n",
    "{\"type\":\"assistant\",\"timestamp\":\"2026-10-04T01:00:05Z\",\"message\":{\"id\":\"msg_5\",\"model\":\"claude-sonnet-4-5-20250929\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"on it\"}],\"usage\":{\"input_tokens\":10,\"output_tokens\":5,\"cache_creation_input_tokens\":2,\"cache_read_input_tokens\":3}}}\n"
);

fn scratch(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("at-rm1374-{label}-{}", std::process::id()))
}

/// Runs the built binary with HOME set to `home`, every other host store
/// scoped under the scratch (TMPDIR included, so the broken-HOME temp
/// fallback lands somewhere probe-able), cwd pinned to an isolated empty
/// directory.
fn run_with_home(home: &str, args: &[&str], root: &Path) -> (i32, String, String) {
    for dir in ["cwd", "tmp", "xdgcache", "xdgconfig", "cache"] {
        std::fs::create_dir_all(root.join(dir)).expect("mkdir scratch dir");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(args)
        .current_dir(root.join("cwd"))
        .env("HOME", home)
        .env_remove("XDG_DATA_HOME")
        .env("XDG_CACHE_HOME", root.join("xdgcache"))
        .env("XDG_CONFIG_HOME", root.join("xdgconfig"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", root.join("cache"))
        .env_remove("AGENTTRACE_HISTORY_DIR")
        .env("TMPDIR", root.join("tmp"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn agenttrace");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// An agent-home-shaped explicit corpus with one real claude-format
/// session, so the overview lane completes and --preserve-history fires.
fn plant_corpus(root: &Path) -> PathBuf {
    let corpus = root.join("corpus");
    let projects = corpus.join(".claude").join("projects").join("tmp-guard");
    std::fs::create_dir_all(&projects).expect("create corpus projects dir");
    std::fs::write(projects.join("session-1.jsonl"), PLANTED_SESSION).expect("plant session file");
    corpus
}

fn assert_cwd_stays_clean(root: &Path, context: &str) {
    let cwd = root.join("cwd");
    let entries: Vec<String> = std::fs::read_dir(&cwd)
        .expect("cwd survives")
        .map(|entry| {
            entry
                .expect("read cwd entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert!(
        entries.is_empty(),
        "{context}: the CWD must stay byte-clean under a broken HOME, found: {entries:?}"
    );
}

#[test]
fn empty_home_preserve_history_never_writes_under_the_cwd() {
    let root = scratch("empty-home");
    let _ = std::fs::remove_dir_all(&root);
    let corpus = plant_corpus(&root);
    let (code, _stdout, stderr) = run_with_home(
        "",
        &[
            "--overview",
            "--preserve-history",
            "-d",
            corpus.to_str().expect("utf8 corpus path"),
        ],
        &root,
    );
    assert_eq!(
        code, 0,
        "overview over the planted corpus, stderr: {stderr}"
    );
    assert_cwd_stays_clean(&root, "HOME=");
    let fallback = root.join("tmp").join("agenttrace").join("history.json");
    assert!(
        fallback.is_file(),
        "history must land in the temp fallback root under an empty HOME: {}",
        fallback.display()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn relative_home_preserve_history_never_writes_under_the_cwd() {
    // The relative arm of the same admission: `HOME=relative-home` used to
    // resolve the data root against the cwd the same way the empty value
    // did. Treated as unset → same temp fallback.
    let root = scratch("relative-home");
    let _ = std::fs::remove_dir_all(&root);
    let corpus = plant_corpus(&root);
    let (code, _stdout, stderr) = run_with_home(
        "relative-home",
        &[
            "--overview",
            "--preserve-history",
            "-d",
            corpus.to_str().expect("utf8 corpus path"),
        ],
        &root,
    );
    assert_eq!(
        code, 0,
        "overview over the planted corpus, stderr: {stderr}"
    );
    assert_cwd_stays_clean(&root, "HOME=relative-home");
    let fallback = root.join("tmp").join("agenttrace").join("history.json");
    assert!(
        fallback.is_file(),
        "history must land in the temp fallback root under a relative HOME: {}",
        fallback.display()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn absolute_home_still_owns_the_data_root() {
    // Control arm: a usable absolute HOME keeps the spec default —
    // history lands under <home>/.local/share/agenttrace, never in the
    // temp fallback or the CWD.
    let root = scratch("absolute-home");
    let _ = std::fs::remove_dir_all(&root);
    let corpus = plant_corpus(&root);
    let home = root.join("home");
    std::fs::create_dir_all(&home).expect("mkdir home");
    let (code, _stdout, stderr) = run_with_home(
        home.to_str().expect("utf8 home path"),
        &[
            "--overview",
            "--preserve-history",
            "-d",
            corpus.to_str().expect("utf8 corpus path"),
        ],
        &root,
    );
    assert_eq!(
        code, 0,
        "overview over the planted corpus, stderr: {stderr}"
    );
    assert_cwd_stays_clean(&root, "absolute HOME control");
    let owned = home
        .join(".local")
        .join("share")
        .join("agenttrace")
        .join("history.json");
    assert!(
        owned.is_file(),
        "history must land under the absolute HOME data root: {}",
        owned.display()
    );
    assert!(
        !root
            .join("tmp")
            .join("agenttrace")
            .join("history.json")
            .exists(),
        "temp fallback must stay unused under a usable HOME"
    );
    let _ = std::fs::remove_dir_all(&root);
}

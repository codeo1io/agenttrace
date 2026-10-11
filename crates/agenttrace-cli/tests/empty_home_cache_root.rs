//! rm-985 poison-proof (run 6e476825 cycle 1): a set-but-empty `HOME`
//! (with `XDG_CACHE_HOME=` too) must never resolve the agenttrace cache
//! against the CWD. The assess PoC reproduced it live: `cd <scratch> &&
//! HOME= XDG_CACHE_HOME= agenttrace --overview` CREATED the relative
//! `./.cache/agenttrace/sessions.json` (1939 bytes) because three of
//! the four `user_cache_dir()` copies treated an empty HOME as set.
//! The resolver now lives once in `agenttrace-core`'s `cache_root`
//! (the guarded statusline model) and lands on the system temp-dir
//! ladder end instead. This is the end-to-end pin, driven through the
//! real binary from a scratch cwd that carries a PLANTED
//! `./.cache/agenttrace` tree:
//!
//! - nothing under the CWD may be created, modified, or consumed
//!   (the planted sentinel survives byte-for-byte and never leaks
//!   into output);
//! - with no plant at all, `./.cache` is never even created;
//! - the temp ladder end is where the artifacts land (TMPDIR is
//!   pinned to the test sandbox so that is directly observable).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("at-empty-home-{tag}-{}", std::process::id()))
        .join(format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            tag
        ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_corpus(corpus: &Path) {
    fs::create_dir_all(corpus).unwrap();
    fs::write(
        corpus.join("20261001T100000+0000-empty-home-probe.jsonl"),
        concat!(
            "{\"type\": \"user\", \"uuid\": \"u1\", \"timestamp\": \"2026-10-01T10:00:00Z\", \"message\": {\"role\": \"user\", \"content\": \"empty home probe\"}, \"cwd\": \"/tmp/at-empty-home\"}\n",
            "{\"type\": \"assistant\", \"uuid\": \"a1\", \"timestamp\": \"2026-10-01T10:00:05Z\", \"message\": {\"role\": \"assistant\", \"model\": \"claude-sonnet-4-5\", \"content\": [{\"type\": \"text\", \"text\": \"done\"}], \"usage\": {\"input_tokens\": 10, \"output_tokens\": 5}}}\n",
        ),
    )
    .unwrap();
}

/// Recursive snapshot of file paths + bytes under `root`.
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .map(|rd| rd.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let bytes = fs::read(&path).unwrap_or_default();
                out.push((path, bytes));
            }
        }
    }
    out
}

fn run_empty_env(report: &str, corpus: &Path, cwd: &Path, tmpdir: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .arg("-d")
        .arg(corpus)
        .arg(report)
        .env("HOME", "")
        .env("XDG_CACHE_HOME", "")
        .env_remove("AGENTTRACE_SESSION_CACHE_DIR")
        .env("TMPDIR", tmpdir)
        .current_dir(cwd)
        .output()
        .expect("agenttrace runs");
    assert!(
        out.status.success(),
        "{report} failed under empty HOME: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

const SENTINEL: &[u8] = b"PLANTED-SENTINEL-must-not-be-read-or-rewritten";

#[test]
fn empty_home_environment_leaves_a_planted_dot_cache_untouched() {
    let tmp = tempdir("planted");
    let corpus = tmp.join("corpus");
    write_corpus(&corpus);
    let sandbox = tmp.join("tmp"); // the child's TMPDIR ladder end
    fs::create_dir_all(&sandbox).unwrap();

    let cwd = tmp.join("cwd");
    let planted_dir = cwd.join(".cache").join("agenttrace");
    fs::create_dir_all(&planted_dir).unwrap();
    let planted = planted_dir.join("sessions.json");
    fs::write(&planted, SENTINEL).unwrap();
    let before = snapshot(&cwd);

    for report in ["--overview", "--doctor"] {
        let combined = run_empty_env(report, &corpus, &cwd, &sandbox);
        // (a) the planted tree is byte-for-byte untouched — never
        // rewritten, never consumed.
        assert_eq!(
            fs::read(&planted).unwrap(),
            SENTINEL,
            "{report} rewrote the planted sessions.json"
        );
        // (b) nothing NEW appeared anywhere under the cwd.
        assert_eq!(
            snapshot(&cwd),
            before,
            "{report} created or modified files under the CWD"
        );
        // (c) the sentinel never leaked into output (a read would
        // surface it or a corruption diagnostic).
        assert!(
            !combined.contains("PLANTED-SENTINEL") && !combined.contains("corrupt"),
            "{report} output betrays a read of the planted cache:\n{combined}"
        );
    }
}

#[test]
fn empty_home_environment_creates_no_dot_cache_at_all() {
    // The exact assess PoC shape: a clean scratch cwd, no plant.
    // Pre-fix, --overview created ./.cache/agenttrace/sessions.json.
    let tmp = tempdir("clean");
    let corpus = tmp.join("corpus");
    write_corpus(&corpus);
    let sandbox = tmp.join("tmp");
    fs::create_dir_all(&sandbox).unwrap();
    let cwd = tmp.join("cwd");
    fs::create_dir_all(&cwd).unwrap();

    for report in ["--overview", "--doctor"] {
        run_empty_env(report, &corpus, &cwd, &sandbox);
        assert!(
            !cwd.join(".cache").exists(),
            "{report} created a relative ./.cache under an empty HOME"
        );
        // And the ladder end engaged: the artifacts landed in the
        // pinned TMPDIR sandbox, not anywhere near the cwd.
        assert!(
            sandbox.join("agenttrace").is_dir() || sandbox.join("agenttrace").exists(),
            "{report} wrote no cache artifact into the TMPDIR sandbox — fixture vacuity"
        );
        let dirs: Vec<_> = fs::read_dir(&cwd)
            .map(|rd| rd.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        assert!(
            dirs.is_empty(),
            "{report} left entries in the scratch cwd: {:?}",
            dirs.iter().map(|e| e.file_name()).collect::<Vec<_>>()
        );
    }
}

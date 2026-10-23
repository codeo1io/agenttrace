//! opencode v2.1.x storage-currency contract (rm-507 / rm-1021).
//!
//! Banks a v2.1.2-shaped fixture store (see
//! `tests/fixtures/opencode-2-1-x/PROVENANCE.md`; field renames per
//! anomalyco/opencode PR #57260 — `state`/`providerState` → `native`,
//! compaction `recent` → `metadata.keepID`, `session.compacted`
//! removed) and locks FULL attribution through the storage-session
//! parser (`parse_opencode_storage_session`, which reads only
//! id/message/tokens/meta — the renames touch none of those, so v2.1
//! corpora parse identically; the fixture proves it rather than
//! assuming it).
//!
//! Attribution locked per session: project directory (both arms —
//! session-doc `directory` and `storage/project/<projectID>`
//! resolution), model (`modelID` on messages), message bodies
//! (user/assistant text, reasoning, tool round-trip), and every token
//! class including the two rm-619 folds: message-level tokens are
//! authoritative over step-finish parts (a poisoned 9 999 block on
//! `msg_a2` must never fold), and a reasoning-only message lets its
//! step-finish part's billing through exactly once (`msg_a4`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use agenttrace_core::{find_session_files, load_sessions_from_dir, parse_file};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/opencode-2-1-x")
        .join(name)
}

/// Session A totals (see PROVENANCE.md): message-level authoritative
/// folds plus the `msg_a4` step-finish rescue.
#[test]
fn v21_alpha_locks_full_attribution_and_token_folds() {
    let parsed = parse_file(&fixture(
        "opencode/storage/session/ses_v21_alpha/ses_v21_alpha.json",
    ))
    .expect("v2.1 alpha session doc must parse");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "opencode");

    // Project attribution, arm 1: `directory` on the session doc.
    assert_eq!(parsed.cwd, "/work/demo/orbit");

    // Every token class. input/output prove BOTH folds: 1 200 + 700
    // (rescue) and 340 + 210 (rescue); a folded `msg_a2` step block
    // would read 11 899 / 10 339.
    assert_eq!(metrics.tokens_input, 1900, "input: 1200 + 700 rescue once");
    assert_eq!(metrics.tokens_output, 550, "output: 340 + 210 rescue once");
    assert_eq!(metrics.tokens_cache_r, 800);
    assert_eq!(metrics.tokens_cache_w, 100);
    assert_eq!(metrics.tokens_reasoning, 112, "reasoning: 64 + 48, folded once");

    // Message bodies survived the renamed `native` shapes.
    assert_eq!(metrics.reasoning_blocks, 1);
    assert!(metrics.reasoning_chars > 0, "reasoning text must be read");
    assert_eq!(metrics.tool_calls_total, 1);
    assert_eq!(metrics.tool_calls_ok, 1);
    assert_eq!(metrics.tool_calls_fail, 0);
    assert_eq!(
        metrics.tool_usage.get("read").copied(),
        Some(1),
        "the completed `read` round-trip must attribute to its tool"
    );

    // Model attribution rides the message `modelID`s.
    let haiku = metrics
        .model_attribution
        .get("anthropic/claude-haiku-4-5")
        .unwrap_or_else(|| panic!("haiku must own the usage: {:?}", metrics.model_attribution));
    assert_eq!(haiku.input_tokens, 1900);
    assert_eq!(haiku.output_tokens, 550);
    assert_eq!(haiku.cache_r, 800);
    assert_eq!(haiku.cache_w, 100);
}

/// Session B resolves its directory through
/// `storage/project/<projectID>.json` — the second attribution arm,
/// also under the v2.1 shapes.
#[test]
fn v21_beta_resolves_directory_through_project_doc() {
    let parsed = parse_file(&fixture(
        "opencode/storage/session/ses_v21_beta/ses_v21_beta.json",
    ))
    .expect("v2.1 beta session doc must parse");
    assert_eq!(parsed.cwd, "/work/demo/northgate");
    assert_eq!(parsed.metrics.tokens_input, 500);
    assert_eq!(parsed.metrics.tokens_output, 150);
    let sonnet = parsed
        .metrics
        .model_attribution
        .get("anthropic/claude-sonnet-4-5")
        .unwrap_or_else(|| panic!("sonnet must own the usage"));
    assert_eq!(sonnet.input_tokens, 500);
    assert_eq!(sonnet.output_tokens, 150);
}

/// Auto-discovery: a v2.1.x store living at the real OpenCode data
/// root surfaces both sessions with attribution intact — the fixture
/// is not a parse-only curiosity.
#[test]
fn v21_store_auto_discovers_with_attribution() {
    let root = temp_root("agenttrace-opencode-v21-discovery");
    let home = root.join("home");
    let data = home.join(".local").join("share").join("opencode");
    fs::create_dir_all(&data).expect("create opencode data root");
    copy_tree(&fixture("opencode"), &data);

    with_home(&home, || {
        let session_docs: Vec<String> = find_session_files(None)
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .filter(|path| path.contains("ses_v21_"))
            .collect();
        assert_eq!(
            session_docs.len(),
            2,
            "both v2.1 session docs must be discovered: {session_docs:?}"
        );

        let mut by_cwd: Vec<(String, String, i64)> = Vec::new();
        for session in load_sessions_from_dir(None) {
            if session.cwd == "/work/demo/orbit" || session.cwd == "/work/demo/northgate" {
                by_cwd.push((
                    session.cwd.clone(),
                    session.metrics.source_tool.clone(),
                    session.metrics.tokens_input,
                ));
            }
        }
        by_cwd.sort();
        assert_eq!(
            by_cwd,
            vec![
                ("/work/demo/northgate".to_string(), "opencode".to_string(), 500),
                ("/work/demo/orbit".to_string(), "opencode".to_string(), 1900),
            ],
            "both v2.1 sessions must load with their directory and token attribution"
        );
    });

    let _ = fs::remove_dir_all(root);
}

fn copy_tree(src: &Path, dst: &Path) {
    for entry in fs::read_dir(src).expect("read fixture tree") {
        let entry = entry.expect("dir entry");
        let target = dst.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            fs::create_dir_all(&target).expect("create dir");
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy fixture file");
        }
    }
}

fn temp_root(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within
/// this test binary (same pattern as `pi_family_discovery.rs`; the
/// catch-unwind keeps a failing assertion from poisoning the lock).
fn with_home(home: &Path, f: impl FnOnce()) {
    let result = {
        let _guard = lock_env();
        let previous_home = std::env::var_os("HOME");
        let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
        let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
        let previous_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore_env("HOME", previous_home);
        restore_env("XDG_CONFIG_HOME", previous_xdg_config);
        restore_env("XDG_CACHE_HOME", previous_xdg_cache);
        restore_env("XDG_DATA_HOME", previous_xdg_data);
        restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_cache);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

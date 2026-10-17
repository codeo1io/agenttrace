//! rm-1921 environment-scope truth contract.
//!
//! Every discovery-lane env var and the AGENTTRACE_SESSION_CACHE_DIR
//! override must treat RELATIVE values as inert, so corpus admission and
//! cache placement are CWD-independent. Mirrors the rm-248 cache_dir guard
//! family at the discovery surface (fleet PoC 2026-10-16: a relative
//! AGENTTRACE_SESSION_CACHE_DIR planted `rel-cache/sessions.json` inside
//! the CWD and `--doctor -f json` reported CWD-relative roots).

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

static SEQ: OnceLock<Mutex<()>> = OnceLock::new();

/// Serializes env mutation against this binary's other tests and snapshots
/// / restores every var this contract touches (plus the CWD). The mutex
/// guard is a FIELD dropped after `Drop::drop` restores the environment —
/// holding it only inside `lock()` would release the lock on return and
/// let the two contract tests mutate the process env concurrently (this
/// exact flake: a battery run where each test observed the other's
/// mid-mutation env).
struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<OsString>)>,
    cwd: PathBuf,
}

impl EnvGuard {
    fn lock() -> Self {
        let mutex = SEQ.get_or_init(|| Mutex::new(()));
        let _lock = mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut saved = Vec::new();
        for name in GUARDED_VARS.iter().copied().chain([
            "HOME",
            "XDG_CACHE_HOME",
            "AGENTTRACE_SESSION_CACHE_DIR",
        ]) {
            saved.push((name, std::env::var_os(name)));
        }
        EnvGuard {
            _lock,
            saved,
            cwd: std::env::current_dir().expect("current dir"),
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, value) in self.saved.drain(..) {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        let _ = std::env::set_current_dir(&self.cwd);
    }
}

/// The discovery lanes whose env vars are guarded by rm-1921.
/// OPENCODE_CONFIG_HOME is reserved (read nowhere in-tree yet): a
/// relative value must stay inert by construction when the lane lands.
const GUARDED_VARS: [&str; 7] = [
    "CODEX_HOME",
    "CLAUDE_CONFIG_DIR",
    "OPENCLAW_STATE_DIR",
    "XDG_CONFIG_HOME",
    "OPENCODE_DATA_DIR",
    "XDG_DATA_HOME",
    "OPENCODE_CONFIG_HOME",
];

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rm1921-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp root");
    dir
}

#[test]
fn discovery_and_cache_are_cwd_independent_when_env_values_are_relative() {
    let _env = EnvGuard::lock();

    // Seed an absolute HOME corpus: two session files that MUST be admitted
    // identically from any CWD while every lane var holds a relative value.
    let home = temp_root("home");
    let sessions_dir = home.join(".codex").join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("seed codex sessions dir");
    std::fs::write(sessions_dir.join("rollout-a.jsonl"), "{}\n").expect("seed session a");
    std::fs::write(sessions_dir.join("rollout-b.jsonl"), "{}\n").expect("seed session b");
    let xdg_cache = temp_root("xdg-cache");

    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_CACHE_HOME", &xdg_cache);
    for name in GUARDED_VARS {
        std::env::set_var(name, format!("rel-{name}"));
    }
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", "rel-cache");

    let cwd_a = temp_root("cwd-a");
    let cwd_b = temp_root("cwd-b");

    // Seed the relative lane values as REAL directories (and one session
    // file) inside cwd_a only: before the rm-1921 guards, the raw
    // OPENCODE_DATA_DIR / XDG_DATA_HOME / XDG_CONFIG_HOME reads admitted
    // these CWD-relative roots whenever the CLI happened to run there
    // (discover_session_dirs filters is_dir, so existence is what made the
    // defect real). Post-guard they must not be admitted from any CWD.
    std::fs::create_dir_all(cwd_a.join("rel-OPENCODE_DATA_DIR"))
        .expect("seed relative opencode data dir");
    std::fs::write(
        cwd_a.join("rel-OPENCODE_DATA_DIR").join("session-c.jsonl"),
        "{}\n",
    )
    .expect("seed CWD-planted session file");
    std::fs::create_dir_all(
        cwd_a
            .join("rel-XDG_DATA_HOME")
            .join("opencode")
            .join("storage"),
    )
    .expect("seed relative xdg data storage");
    std::fs::create_dir_all(
        cwd_a
            .join("rel-XDG_CONFIG_HOME")
            .join("Code")
            .join("User")
            .join("globalStorage")
            .join("saoudrizwan.claude-dev")
            .join("tasks"),
    )
    .expect("seed relative cline tasks dir");

    std::env::set_current_dir(&cwd_a).expect("chdir cwd a");
    let dirs_a = agenttrace_core::discover_session_dirs();
    let files_a = agenttrace_core::find_session_files(None);
    let sessions_json_a = agenttrace_core::session_cache_path();
    let pricing_json_a = agenttrace_core::pricing_cache_path();
    let statusline_a = agenttrace_core::statusline_capture_path();

    std::env::set_current_dir(&cwd_b).expect("chdir cwd b");
    let dirs_b = agenttrace_core::discover_session_dirs();
    let files_b = agenttrace_core::find_session_files(None);
    let sessions_json_b = agenttrace_core::session_cache_path();
    let pricing_json_b = agenttrace_core::pricing_cache_path();
    let statusline_b = agenttrace_core::statusline_capture_path();

    assert_eq!(dirs_a, dirs_b, "discovery dirs must not depend on the CWD");
    assert_eq!(files_a, files_b, "corpus totals must not depend on the CWD");
    assert_eq!(sessions_json_a, sessions_json_b);
    assert_eq!(pricing_json_a, pricing_json_b);
    assert_eq!(statusline_a, statusline_b);

    for dir in &dirs_a {
        assert!(
            dir.is_absolute(),
            "discovered dir must be absolute: {dir:?}"
        );
        assert!(
            !dir.starts_with(&cwd_a) && !dir.starts_with(&cwd_b),
            "rm-1921: no artifact may resolve under either CWD: {dir:?}"
        );
    }
    assert_eq!(
        files_a.len(),
        2,
        "exactly the two seeded session files may be admitted: {files_a:?}"
    );
    assert!(
        files_a.iter().all(|file| file.starts_with(&home)),
        "the seeded absolute corpus must be the admitted corpus: {files_a:?}"
    );
    // The relative override is inert; the XDG_CACHE_HOME ladder (absolute)
    // supplies the user cache root instead of the CWD.
    assert_eq!(
        sessions_json_a,
        xdg_cache.join("agenttrace").join("sessions.json")
    );
    assert_eq!(
        pricing_json_a,
        xdg_cache.join("agenttrace").join("pricing.json")
    );
    assert_eq!(
        statusline_a,
        xdg_cache.join("agenttrace").join("statusline.jsonl")
    );
}

#[test]
fn absolute_env_values_still_bind_their_lanes() {
    // The guard must not overreach: an ABSOLUTE CODEX_HOME still binds the
    // codex lane (guard-join, not ignore-join).
    let _env = EnvGuard::lock();
    let home = temp_root("abs-home");
    let codex = temp_root("abs-codex");
    std::fs::create_dir_all(codex.join("sessions")).expect("seed codex home");
    std::env::set_var("HOME", &home);
    std::env::remove_var("XDG_CACHE_HOME");
    for name in GUARDED_VARS {
        if name != "CODEX_HOME" {
            std::env::remove_var(name);
        }
    }
    std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("CODEX_HOME", &codex);

    let dirs = agenttrace_core::discover_session_dirs();
    assert!(
        dirs.iter().any(|dir| dir == &codex.join("sessions")),
        "an absolute CODEX_HOME must bind its lane: {dirs:?}"
    );
}

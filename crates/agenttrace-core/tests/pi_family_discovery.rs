//! Golden coverage for pi-family session-home discovery (rm-084).
//!
//! pi stores transcripts under `<agent-dir>/sessions/<project>/*.jsonl`,
//! where the agent dir defaults to `~/.pi/agent`. Forks relocate the
//! whole root (`~/.senpi`, `~/.omo`, `~/.omp`) and pi itself relocates
//! the agent dir inside a root (`~/.pi/agent-cliproxy-only` — isolated
//! agent state / profiles). Default discovery used to register exactly
//! three homes (`~/.pi/agent`, `~/.config/pi/agent`, `~/.omp/agent`),
//! so every fork and agent-dir corpus was invisible to `--overview`
//! unless the operator passed `-d`. The fixture below mirrors the real
//! host shapes this defect was first proven on (1,404 jsonl under
//! `~/.senpi/agent-cliproxy-only/sessions`, 8,187 under
//! `~/.pi/agent-cliproxy-only/sessions`, and `~/.omo/sessions` holding
//! project dirs directly) and pins the discovery set, the doctor-visible
//! registry names, and the per-home source labels.

use agenttrace_core::{find_session_files, known_session_dirs, load_sessions_from_dir, parse_file};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const PI_SESSION: &str = r#"{"type":"session","version":3,"id":"pi-family-fixture","cwd":"/work/fixture"}
{"type":"message","id":"u1","timestamp":"2026-09-30T10:00:00.000Z","message":{"role":"user","content":"fixture turn"}}
"#;

/// A parseable pi transcript under `<root>/sessions/<proj>/<name>`.
fn pi_transcript(root: &Path, proj: &str, name: &str) -> PathBuf {
    let dir = root.join("sessions").join(proj);
    fs::create_dir_all(&dir).expect("create fixture session dir");
    let path = dir.join(name);
    fs::write(&path, PI_SESSION).expect("write fixture transcript");
    path
}

/// Builds the multi-home, multi-profile fixture corpus and returns the
/// exact set of session files default discovery must find.
fn write_pi_family_fixture(home: &Path) -> Vec<PathBuf> {
    let files = vec![
        // ~/.pi: default agent dir plus a relocated agent dir.
        pi_transcript(&home.join(".pi").join("agent"), "proj", "pi-default.jsonl"),
        pi_transcript(
            &home.join(".pi").join("agent-cliproxy-only"),
            "proj",
            "pi-agent-dir.jsonl",
        ),
        // XDG pi root: still pi, not a fork.
        pi_transcript(
            &home.join(".config").join("pi").join("agent"),
            "proj",
            "pi-xdg.jsonl",
        ),
        // ~/.omp fork: default agent dir plus an agent-dir variant.
        pi_transcript(
            &home.join(".omp").join("agent"),
            "proj",
            "omp-default.jsonl",
        ),
        pi_transcript(
            &home.join(".omp").join("agent-beta"),
            "proj",
            "omp-agent-dir.jsonl",
        ),
        // ~/.senpi fork (both shapes seen live).
        pi_transcript(
            &home.join(".senpi").join("agent"),
            "proj",
            "senpi-default.jsonl",
        ),
        pi_transcript(
            &home.join(".senpi").join("agent-cliproxy-only"),
            "proj",
            "senpi-agent-dir.jsonl",
        ),
        // ~/.omo fork: agent dir, agent-dir variant, and project dirs
        // directly under `<root>/sessions` (the omo home itself is the
        // agent dir on real hosts).
        pi_transcript(
            &home.join(".omo").join("agent"),
            "proj",
            "omo-default.jsonl",
        ),
        pi_transcript(
            &home.join(".omo").join("agent-cliproxy-only"),
            "proj",
            "omo-agent-dir.jsonl",
        ),
        pi_transcript(&home.join(".omo"), "proj", "omo-root-sessions.jsonl"),
    ];

    // Negative shapes: fork state that is NOT a session home. Both hold
    // parseable pi content, so they would load as sessions if any
    // future enumeration walked them by mistake.
    let memory = home.join(".omo").join("memory");
    fs::create_dir_all(&memory).expect("create memory dir");
    fs::write(memory.join("notes.jsonl"), PI_SESSION).expect("write memory fixture");
    let pi_cache = home.join(".pi").join("cache");
    fs::create_dir_all(&pi_cache).expect("create pi cache dir");
    fs::write(pi_cache.join("state.jsonl"), PI_SESSION).expect("write cache fixture");

    files
}

#[test]
fn default_discovery_covers_pi_forks_and_agent_dir_variants() {
    let root = temp_root("agenttrace-pi-family-golden");
    let home = root.join("home");
    let expected = write_pi_family_fixture(&home);

    with_home(&home, || {
        let files = find_session_files(None);
        let discovered: HashSet<String> = files
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();
        let expected_set: HashSet<String> = expected
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            discovered, expected_set,
            "default discovery must find every pi-family home (forks, agent-dir \
             variants, root-level sessions) and never the non-session state dirs"
        );

        // The doctor-visible registry names the fork and agent-dir
        // variants instead of collapsing them into three hardcoded rows.
        let registry: Vec<(String, PathBuf)> = known_session_dirs()
            .into_iter()
            .map(|dir| (dir.name, dir.path))
            .collect();
        let paths: HashSet<String> = registry
            .iter()
            .map(|(_, path)| path.to_string_lossy().to_string())
            .collect();
        for expected_path in [
            home.join(".pi").join("agent").join("sessions"),
            home.join(".pi")
                .join("agent-cliproxy-only")
                .join("sessions"),
            home.join(".config")
                .join("pi")
                .join("agent")
                .join("sessions"),
            home.join(".omp").join("agent").join("sessions"),
            home.join(".omp").join("agent-beta").join("sessions"),
            home.join(".senpi").join("agent").join("sessions"),
            home.join(".senpi")
                .join("agent-cliproxy-only")
                .join("sessions"),
            home.join(".omo").join("agent").join("sessions"),
            home.join(".omo")
                .join("agent-cliproxy-only")
                .join("sessions"),
            home.join(".omo").join("sessions"),
        ] {
            assert!(
                paths.contains(&expected_path.to_string_lossy().to_string()),
                "registry must include {}: {registry:?}",
                expected_path.display()
            );
        }
        let names: Vec<&str> = registry.iter().map(|(name, _)| name.as_str()).collect();
        for expected_name in ["Senpi", "Senpi (agent-cliproxy-only)", "Omo"] {
            assert!(
                names.contains(&expected_name),
                "registry must name {expected_name}: {names:?}"
            );
        }

        // Loading through the default surface attributes every corpus
        // to its actual home (the source label must not claim a
        // different agent).
        let sessions = load_sessions_from_dir(None);
        let by_source = |stem: &str| {
            sessions
                .iter()
                .find(|session| {
                    Path::new(&session.path)
                        .file_stem()
                        .is_some_and(|found| found == stem)
                })
                .unwrap_or_else(|| panic!("session {stem} must load: {sessions:?}"))
                .metrics
                .source_tool
                .clone()
        };
        assert_eq!(by_source("pi-default"), "pi");
        assert_eq!(by_source("pi-agent-dir"), "pi");
        assert_eq!(by_source("pi-xdg"), "pi");
        assert_eq!(by_source("omp-default"), "oh_my_pi");
        assert_eq!(by_source("omp-agent-dir"), "oh_my_pi");
        assert_eq!(by_source("senpi-default"), "pi_senpi");
        assert_eq!(by_source("senpi-agent-dir"), "pi_senpi");
        assert_eq!(by_source("omo-default"), "pi_omo");
        assert_eq!(by_source("omo-agent-dir"), "pi_omo");
        assert_eq!(by_source("omo-root-sessions"), "pi_omo");
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn pi_source_labels_match_the_actual_home_root() {
    // Pure parser-level pin (no discovery, no env): the source label is
    // derived from the home root that actually owns the file. The XDG
    // root is pi (it used to be mislabeled oh_my_pi), fork roots keep
    // their own identity, and a pi-family transcript outside every
    // known root — the `-d <dir>` escape for an agent dir relocated
    // elsewhere — keeps the historical oh_my_pi family label.
    let root = temp_root("agenttrace-pi-family-labels");
    let files = write_pi_family_fixture(&root);

    let source_of = |stem: &str| {
        let path = files
            .iter()
            .find(|path| {
                path.file_stem()
                    .is_some_and(|found| found.to_string_lossy() == stem)
            })
            .unwrap_or_else(|| panic!("fixture {stem} missing"));
        parse_file(path)
            .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
            .metrics
            .source_tool
    };

    assert_eq!(source_of("pi-default"), "pi");
    assert_eq!(source_of("pi-agent-dir"), "pi");
    assert_eq!(source_of("pi-xdg"), "pi");
    assert_eq!(source_of("omp-default"), "oh_my_pi");
    assert_eq!(source_of("senpi-default"), "pi_senpi");
    assert_eq!(source_of("omo-default"), "pi_omo");
    assert_eq!(source_of("omo-root-sessions"), "pi_omo");

    let relocated = pi_transcript(&root.join("elsewhere-agent"), "proj", "relocated.jsonl");
    let parsed = parse_file(&relocated).expect("parse relocated pi transcript");
    assert_eq!(
        parsed.metrics.source_tool, "oh_my_pi",
        "unknown-root pi-family transcripts keep the family fallback label"
    );

    let _ = fs::remove_dir_all(root);
}

fn temp_root(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within this
/// test binary (see discovery_contract.rs for the same pattern; the
/// catch-unwind keeps a failing assertion from poisoning the lock).
/// XDG_DATA_HOME is pinned too (rm-084 review follow-up, Rule 6 of
/// docs/maintainers/test-flake-prevention.md): discovery reads it for
/// opencode storage, so an inherited runner value would leak foreign
/// artifacts into the golden set.
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

//! rm-367 stage (a) + rm-904 contract: the doctor's file lane persists
//! fresh parses through the session cache (an immediate rerun reuses
//! them instead of re-parsing every session), and orphaned subagent
//! transcripts surface through the doctor's disclosures map —
//! 0-count-clean (a corpus without orphans carries no key).
//!
//! Stage (a) makes the doctor PERSIST, so every doctor test here (and
//! the rm-596 control arm in doctor_demo_contract.rs, re-sandboxed for
//! this batch) pins HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR to a temp
//! home first — the pi_family_discovery `with_home` pattern — so no
//! fixture entry can ever reach the operator's real cache.

use agenttrace_core::{build_doctor_report, render_doctor_report};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within this
/// test binary (pi_family_discovery.rs pattern; the catch-unwind keeps a
/// failing assertion from poisoning the lock).
fn lock_env() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

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

fn temp_home(marker: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("at-rm367a-{marker}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).expect("create sandbox home");
    home
}

/// The vendored corpus with exactly one orphaned subagent
/// (`lonely/subagents/agent-orphan.jsonl` with no parent transcript —
/// `subagent_attribution.rs` pins the loader side of the same corpus).
fn subagent_corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/subagent-corpus")
}

const PLANTED_SESSION: &str = concat!(
    "{\"role\":\"session_meta\",\"timestamp\":\"2026-05-01T09:00:00Z\",\"ModelUsed\":\"claude-sonnet-4-5\"}\n",
    "{\"role\":\"meta\",\"ModelUsed\":\"claude-sonnet-4-5\",\"Usage\":{\"input_tokens\":120,\"output_tokens\":30}}\n",
);

fn planted_clean_corpus(marker: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("at-rm904-clean-{marker}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let project = root.join("projects").join("home-agent-clean");
    fs::create_dir_all(&project).expect("create clean project dir");
    fs::write(project.join("session.jsonl"), PLANTED_SESSION).expect("plant session file");
    root
}

#[test]
fn doctor_persists_fresh_parses_for_the_immediate_rerun() {
    // rm-367 stage (a): the cold doctor walk parses the corpus and
    // PERSISTS it through the session cache; the immediate rerun on the
    // same directory reuses those parses (per-directory cache_hits > 0,
    // reparsed_this_scan < session_files) instead of re-parsing every
    // session on every run.
    let home = temp_home("persist");
    with_home(&home, || {
        let corpus = subagent_corpus();
        let cold = build_doctor_report(Some(&corpus), false);
        assert!(cold.session_files >= 6, "fixture corpus must be walked");
        let cold_hits: usize = cold.directories.iter().map(|d| d.cache_hits).sum();
        assert_eq!(
            cold_hits, 0,
            "cold run against an empty cache must take the parse arm"
        );
        // The persist itself: the cache file exists and is non-empty
        // after the cold walk (red before stage (a): the doctor read
        // the cache but never wrote it).
        let cache_file = home.join("cache").join("sessions.json");
        assert!(
            cache_file.is_file(),
            "cold doctor walk must persist the session cache at {}",
            cache_file.display()
        );
        assert!(
            fs::metadata(&cache_file)
                .map(|m| m.len() > 0)
                .unwrap_or(false),
            "persisted session cache must be non-empty"
        );

        let warm = build_doctor_report(Some(&corpus), false);
        let warm_hits: usize = warm.directories.iter().map(|d| d.cache_hits).sum();
        assert!(
            warm_hits >= 6,
            "immediate rerun must reuse the persisted parses: cache_hits={warm_hits}"
        );
        assert!(
            warm.reparsed_this_scan < warm.session_files,
            "warm rerun must not re-parse every file: reparsed_this_scan={} of {}",
            warm.reparsed_this_scan,
            warm.session_files
        );
        // Parsed accounting is mode-invariant: the rerun sees the same
        // session count either way.
        assert_eq!(warm.sessions, cold.sessions);
        assert_eq!(warm.session_files, cold.session_files);
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn doctor_discloses_orphaned_subagents_zero_count_clean() {
    // rm-904: a subagent transcript whose parent is not part of the
    // scanned corpus surfaces through the doctor's disclosures map —
    // the same map the CLI's every-report disclosure (main.rs
    // disclose_unlinked_subagents) feeds — instead of vanishing from
    // --doctor entirely. 0-count-clean: a corpus without orphans
    // carries no key.
    let home = temp_home("orphan");
    with_home(&home, || {
        let corpus = subagent_corpus();
        let report = build_doctor_report(Some(&corpus), false);
        assert_eq!(
            report.disclosures.get("unlinked_subagents"),
            Some(&1),
            "the fixture's single orphan (lonely/subagents/agent-orphan.jsonl) must disclose"
        );

        let json = render_doctor_report(Some(&corpus), false, "json").expect("render json");
        assert!(
            json.contains("\"unlinked_subagents\""),
            "json doctor report must carry the disclosure: {json}"
        );
        let text = render_doctor_report(Some(&corpus), false, "text").expect("render text");
        assert!(
            text.contains("unlinked_subagents=1"),
            "text doctor report must render the disclosure: {text}"
        );

        let clean_root = planted_clean_corpus("rm904");
        let clean = build_doctor_report(Some(&clean_root), false);
        assert!(
            !clean.disclosures.contains_key("unlinked_subagents"),
            "0-count-clean: a corpus without orphans must not carry the key"
        );
        let _ = fs::remove_dir_all(&clean_root);
    });
    let _ = fs::remove_dir_all(&home);
}

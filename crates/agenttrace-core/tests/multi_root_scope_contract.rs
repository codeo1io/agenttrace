//! rm-1063 + rm-1061-min contracts (run 62250c06, batch
//! "corpus-scope truth"):
//!
//! - rm-1063: a `-d`-scoped run whose scope holds zero readable
//!   sources must never parse (or rewrite) the global session-cache
//!   store — scope resolution happens before the store load.
//! - rm-1061 (minimal arm): `-d` accepts multiple roots; the union
//!   dedupes a session file that appears in more than one root (the
//!   multi-device synced-roots case) by (source, file name), and the
//!   union's totals match scanning the deduped file set once.
//!
//! The env-scoped tests serialize on one mutex because
//! `AGENTTRACE_SESSION_CACHE_DIR` is process-global state.

use agenttrace_core::{load_sessions_from_roots, load_sessions_with_options, LoadOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "agenttrace-mr-{}-{}-{tag}",
        std::process::id(),
        scratch_seq()
    ));
    fs::create_dir_all(&dir).expect("mkdir scratch");
    dir
}

fn scratch_seq() -> u32 {
    static SEQ: OnceLock<std::sync::atomic::AtomicU32> = OnceLock::new();
    SEQ.get_or_init(Default::default)
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

/// A minimal claude-format journal (zero_usage_contract line shapes)
/// whose token totals differ per file so double-counting is visible.
fn journal(tag: &str, input: u64, output: u64) -> String {
    let user = format!(
        r#"{{"type":"user","timestamp":"2026-10-04T01:00:00Z","cwd":"/tmp/{tag}","message":{{"role":"user","content":"check the billing please"}}}}"#
    );
    let assistant = format!(
        r#"{{"type":"assistant","timestamp":"2026-10-04T01:00:05Z","message":{{"id":"msg_{tag}","model":"claude-sonnet-4-5-20250929","role":"assistant","content":[{{"type":"text","text":"on it"}}],"usage":{{"input_tokens":{input},"output_tokens":{output},"cache_creation_input_tokens":2,"cache_read_input_tokens":3}}}}}}"#
    );
    format!("{user}\n{assistant}\n")
}

fn write_session(dir: &Path, name: &str, body: &str) {
    fs::write(dir.join(name), body).expect("write fixture session");
}

/// Isolate the global cache store for the duration of the closure and
/// pre-seed it with recognizable bytes so "the store was rewritten"
/// is observable as a byte change.
fn with_isolated_store<F>(tag: &str, f: F) -> PathBuf
where
    F: FnOnce(),
{
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let cache_dir = scratch(&format!("{tag}-cache"));
    let store = cache_dir.join("sessions.json");
    fs::write(
        &store,
        b"{\"schema_version\":1,\"marker\":\"rm-1063-probe\"}",
    )
    .expect("seed store");
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &cache_dir);
    f();
    std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
    cache_dir
}

#[test]
fn empty_scope_leaves_global_store_untouched() {
    let empty_scope = scratch("empty-scope");
    let cache_dir = with_isolated_store("defer", || {
        let report = load_sessions_with_options(Some(&empty_scope), &LoadOptions::default());
        assert!(
            report.sessions.is_empty(),
            "empty scope must yield zero sessions"
        );
        assert_eq!(report.discovered, 0, "empty scope must discover nothing");
    });
    let after = fs::read(cache_dir.join("sessions.json")).expect("store readable after run");
    assert_eq!(
        String::from_utf8_lossy(&after),
        "{\"schema_version\":1,\"marker\":\"rm-1063-probe\"}",
        "rm-1063: an empty -d scope must not rewrite the global store"
    );
}

#[test]
fn multi_root_union_dedupes_shared_session_file() {
    let root_a = scratch("root-a");
    let root_b = scratch("root-b");
    let union_dir = scratch("union");
    let shared = journal("shared", 100, 50);
    write_session(&root_a, "s-a-only.jsonl", &journal("a-only", 10, 5));
    write_session(&root_a, "s-shared.jsonl", &shared);
    write_session(&root_b, "s-shared.jsonl", &shared);
    write_session(&root_b, "s-b-only.jsonl", &journal("b-only", 7, 3));
    write_session(&union_dir, "s-a-only.jsonl", &journal("a-only", 10, 5));
    write_session(&union_dir, "s-shared.jsonl", &shared);
    write_session(&union_dir, "s-b-only.jsonl", &journal("b-only", 7, 3));

    let cache_dir = with_isolated_store("multi", || {
        let multi = load_sessions_from_roots(&[&root_a, &root_b], &LoadOptions::default());
        let single = load_sessions_with_options(Some(&union_dir), &LoadOptions::default());
        assert_eq!(
            multi.sessions.len(),
            3,
            "rm-1061: the shared session file must count once across roots"
        );
        assert_eq!(
            multi.discovered, 3,
            "rm-1061: discovery counters reflect the deduped union"
        );
        assert_eq!(multi.parsed, 3, "rm-1061: parsed tracks the deduped union");
        assert_eq!(single.sessions.len(), 3);

        assert_eq!(single.sessions.len(), 3);
        // Byte-identical files parse deterministically, so comparing
        // the (file name, raw parsed journal) pairs between the
        // multi-root union and the single scan of the deduped union
        // pins full token/cost parity without reaching into parser
        // internals.
        let fingerprint = |sessions: &[agenttrace_core::Session]| {
            let mut pairs: Vec<(String, String)> = sessions
                .iter()
                .map(|session| {
                    (
                        Path::new(&session.path)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        serde_json::to_string(&session.metrics).expect("serializable"),
                    )
                })
                .collect();
            pairs.sort();
            pairs
        };
        assert_eq!(
            fingerprint(&multi.sessions),
            fingerprint(&single.sessions),
            "rm-1061: multi-root union must equal the single scan of the union's distinct files"
        );
    });
    // the multi-root run may legitimately warm the store; only the
    // defer contracts pin store immutability
    let _ = cache_dir;
}

#[test]
fn multi_root_all_empty_scopes_defer_the_store() {
    let root_a = scratch("root-a-empty");
    let root_b = scratch("root-b-empty");
    let cache_dir = with_isolated_store("multi-empty", || {
        let report = load_sessions_from_roots(&[&root_a, &root_b], &LoadOptions::default());
        assert!(report.sessions.is_empty());
        assert_eq!(report.discovered, 0);
    });
    let after = fs::read(cache_dir.join("sessions.json")).expect("store readable after run");
    assert_eq!(
        String::from_utf8_lossy(&after),
        "{\"schema_version\":1,\"marker\":\"rm-1063-probe\"}",
        "rm-1063: an all-empty multi-root scope must not rewrite the global store"
    );
}

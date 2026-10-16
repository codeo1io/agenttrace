//! rm-299 (assess F1, batch 'warm-path truth'): the opencode storage
//! warm-cache fingerprint must cover the `storage/message/<id>/` +
//! `storage/part/<msg>/` sibling fan-out, not just the session doc.
//!
//! The parser derives every event from sibling files the session doc
//! never names (parser.rs `parse_opencode_storage_session` reads
//! `message/<session_id>/*.json` and, per message, `part/<msg_id>/*.json`),
//! so a post-session write to a sibling left the warm entry's
//! session-doc fingerprint matched and the cache served stale
//! token/cost totals silently — PoC (assess 43e5489f, poc1): cold
//! `$0.0060`, append `storage/message/sess_1/msg_2.json` +
//! `storage/part/msg_2/part_1.json` with the session doc untouched →
//! warm still `$0.0060`, truth `$0.0240`.
//!
//! These tests pin the acceptance arms red-first:
//!  1. a sibling APPEND with the session doc untouched must re-parse
//!     (warm totals equal the fresh-parse truth),
//!  2. an in-place sibling PART mutation must re-parse,
//!  3. a pre-fix entry (no sibling fingerprint) self-heals through
//!     exactly one re-parse — no schema rung, no store torch,
//!  4. non-opencode entries keep warm-serving (the None==None arm),
//!  5. a corpus whose message ids do not match their file names
//!     (path-only enumeration cannot see the parser's real fan-out)
//!     never warm-serves — it re-parses every scan instead.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use agenttrace_core::{
    cached_session, load_session_cache, load_sessions_from_dir, parse_file, store_session,
    SessionCache,
};

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within this
/// test binary (doctor_honesty_contract.rs pattern; the catch-unwind
/// keeps a failing assertion from poisoning the lock).
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

/// Pins the sandbox env (cache dir, HOME, XDG, OPENCODE_DATA_DIR) so
/// cache writes land in the temp home and corpus paths resolve through
/// the `/opencode/storage` marker instead of any live data dir.
fn with_home(home: &Path, f: impl FnOnce()) {
    let result = {
        let _guard = lock_env();
        let previous_home = std::env::var_os("HOME");
        let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
        let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
        let previous_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        let previous_opencode = std::env::var_os("OPENCODE_DATA_DIR");
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        std::env::remove_var("OPENCODE_DATA_DIR");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore_env("HOME", previous_home);
        restore_env("XDG_CONFIG_HOME", previous_xdg_config);
        restore_env("XDG_CACHE_HOME", previous_xdg_cache);
        restore_env("XDG_DATA_HOME", previous_xdg_data);
        restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_cache);
        restore_env("OPENCODE_DATA_DIR", previous_opencode);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn sandbox(marker: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "at-rm299-{marker}-{}-{}",
        std::process::id(),
        marker
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create sandbox");
    root
}

/// The PoC corpus shape: one opencode storage session whose usage rides
/// a step-finish part (message-level tokens stay absent so the part is
/// the only usage carrier — the exact shape the sibling-append PoC
/// exercised). `include_m2` controls whether the second message — the
/// one the warm-cache bug appended AFTER the cold scan — exists yet.
/// `m1_doc_id` lets the unverifiable-scope arm rename the message id
/// away from its file name (defaults to the honest "m1").
fn write_corpus(root: &Path, include_m2: bool, m1_doc_id: &str) -> PathBuf {
    let session_id = "sess_1";
    for dir in [
        format!("opencode/storage/session/{session_id}"),
        format!("opencode/storage/message/{session_id}"),
        "opencode/storage/part/m1".to_string(),
        "opencode/storage/part/m2".to_string(),
        // The unverifiable arm points m1's parts at part/mX/.
        format!("opencode/storage/part/{m1_doc_id}"),
    ] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    let session_file = root.join(format!(
        "opencode/storage/session/{session_id}/{session_id}.json"
    ));
    fs::write(
        &session_file,
        r#"{
  "id": "sess_1",
  "slug": "warm-path-truth",
  "projectID": "prj_1",
  "title": "Warm path truth",
  "model": {"id": "claude-sonnet-4-5"},
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    )
    .unwrap();

    fs::write(
        root.join(format!("opencode/storage/message/{session_id}/m1.json")),
        format!(r#"{{"id": "{m1_doc_id}", "sessionID": "sess_1", "role": "user", "modelID": "claude-sonnet-4-5", "time": {{"created": 1764750001000}}}}"#),
    )
    .unwrap();
    // Parts live under part/<m1's doc id>/ — the parser keys them by
    // the message doc's id, not the file name.
    fs::write(
        root.join(format!("opencode/storage/part/{m1_doc_id}/p_text.json")),
        r#"{"id": "p_text", "type": "text", "text": "hello world", "time": {"created": 1764750001500}}"#,
    )
    .unwrap();
    fs::write(
        root.join(format!("opencode/storage/part/{m1_doc_id}/p_fin.json")),
        r#"{"id": "p_fin", "type": "step-finish", "time": {"created": 1764750001600}, "tokens": {"input": 100, "output": 20}}"#,
    )
    .unwrap();

    if include_m2 {
        append_m2(root);
    }
    session_file
}

/// The post-session sibling write the PoC performed: a new message doc
/// plus its parts, with the session doc never touched.
fn append_m2(root: &Path) {
    fs::write(
        root.join("opencode/storage/message/sess_1/m2.json"),
        r#"{"id": "m2", "sessionID": "sess_1", "role": "assistant", "modelID": "claude-sonnet-4-5", "time": {"created": 1764750002000}}"#,
    )
    .unwrap();
    fs::write(
        root.join("opencode/storage/part/m2/p_text2.json"),
        r#"{"id": "p_text2", "type": "text", "text": "the answer", "time": {"created": 1764750002500}}"#,
    )
    .unwrap();
    fs::write(
        root.join("opencode/storage/part/m2/p_fin2.json"),
        r#"{"id": "p_fin2", "type": "step-finish", "time": {"created": 1764750002600}, "tokens": {"input": 150, "output": 90}}"#,
    )
    .unwrap();
}

fn opencode_session(sessions: &[agenttrace_core::Session]) -> &agenttrace_core::Session {
    sessions
        .iter()
        .find(|session| session.path.ends_with("sess_1.json"))
        .unwrap_or_else(|| panic!("opencode session missing from {sessions:?}"))
}

/// Acceptance 1 (the PoC): a sibling append with the session doc
/// untouched must invalidate the warm entry — the warm totals must
/// equal the fresh-parse truth (input 100+150, output 20+90).
#[test]
fn sibling_append_with_untouched_session_doc_reparses() {
    let root = sandbox("append");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    write_corpus(&root, false, "m1");

    with_home(&home, || {
        let cold = load_sessions_from_dir(Some(&root));
        let cold = opencode_session(&cold);
        assert_eq!(cold.metrics.tokens_input, 100, "cold: m1 usage only");
        assert_eq!(cold.metrics.tokens_output, 20, "cold: m1 usage only");

        // The PoC's mutation: siblings only, session doc untouched.
        append_m2(&root);

        let warm = load_sessions_from_dir(Some(&root));
        let warm = opencode_session(&warm);
        assert_eq!(
            warm.metrics.tokens_input, 250,
            "warm must re-parse the appended message (100 + 150), not serve the stale 100"
        );
        assert_eq!(
            warm.metrics.tokens_output, 110,
            "warm must re-parse the appended part (20 + 90), not serve the stale 20"
        );
        assert_eq!(warm.metrics.user_messages, 1);
        assert_eq!(warm.metrics.assistant_turns, 1, "the appended m2 turn");

        // And the equality target: warm == a fresh parse of the same tree.
        let session_file = root.join("opencode/storage/session/sess_1/sess_1.json");
        let fresh = parse_file(&session_file).expect("fresh parse after append");
        assert_eq!(warm.metrics.tokens_input, fresh.metrics.tokens_input);
        assert_eq!(warm.metrics.tokens_output, fresh.metrics.tokens_output);
        assert_eq!(warm.metrics.events_total, fresh.metrics.events_total);
    });
    let _ = fs::remove_dir_all(&root);
}

/// Acceptance 2: an in-place mutation of an EXISTING sibling part (no
/// new files, message docs untouched) must invalidate the warm entry.
#[test]
fn sibling_part_mutation_with_untouched_session_doc_reparses() {
    let root = sandbox("mutate");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    write_corpus(&root, true, "m1");

    with_home(&home, || {
        let cold = load_sessions_from_dir(Some(&root));
        let cold = opencode_session(&cold);
        assert_eq!(cold.metrics.tokens_input, 250, "cold: m1 + m2 usage");

        // Rewrite one existing part in place — larger payload so the
        // size fingerprint moves even on same-tick filesystems.
        fs::write(
            root.join("opencode/storage/part/m1/p_fin.json"),
            r#"{"id": "p_fin", "type": "step-finish", "time": {"created": 1764750001600}, "tokens": {"input": 3000, "output": 600}}"#,
        )
        .unwrap();

        let warm = load_sessions_from_dir(Some(&root));
        let warm = opencode_session(&warm);
        assert_eq!(
            warm.metrics.tokens_input, 3150,
            "warm must re-parse the mutated part (3000 + 150), not serve the stale 250"
        );
        assert_eq!(
            warm.metrics.tokens_output, 690,
            "warm must re-parse the mutated part (600 + 90), not serve the stale 110"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

/// Acceptance 3: a pre-fix entry (written before the sibling
/// fingerprint existed) self-heals through exactly one re-parse. No
/// schema rung fires (the rm-230 convention: bumps are for what
/// fingerprints CANNOT see — here the new fingerprint dimension
/// self-invalidates via None != Some), dir listings survive, and the
/// next store re-banks the sibling fingerprint.
#[test]
fn prefix_entry_self_heals_without_store_torch() {
    let root = sandbox("selfheal");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let session_file = write_corpus(&root, false, "m1");

    with_home(&home, || {
        // Cold load stores the entry (with sibling fingerprints).
        let cold = load_sessions_from_dir(Some(&root));
        assert_eq!(opencode_session(&cold).metrics.tokens_input, 100);

        // Surgically rewind the stored entry to its pre-fix shape:
        // strip the sibling fingerprint, keep everything else fresh.
        let cache_path = home.join("cache").join("sessions.json");
        let raw = fs::read_to_string(&cache_path).expect("cache written by the cold load");
        let mut doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let entries = doc
            .get_mut("entries")
            .and_then(|entries| entries.as_object_mut())
            .expect("entries map");
        let key = session_file.to_str().expect("utf-8 corpus path");
        let entry = entries
            .get_mut(key)
            .expect("the opencode session has a cache entry");
        let stripped = entry
            .as_object_mut()
            .expect("entry object")
            .remove("opencode_siblings");
        assert!(
            stripped.is_some(),
            "post-fix stores must bank the sibling fingerprint"
        );
        fs::write(&cache_path, serde_json::to_string(&doc).unwrap()).unwrap();

        // The pre-fix entry must MISS (None vs the fresh Some) —
        // self-healing re-parse, disclosed as a plain freshness miss.
        let mut cache = load_session_cache();
        assert!(
            cached_session(&session_file, &mut cache).is_none(),
            "a pre-fix (sibling-less) entry must not warm-serve"
        );
        assert!(
            cache.schema_invalidations().is_empty(),
            "no schema rung: the sibling fingerprint self-invalidates"
        );
        assert!(
            cache.dir_count() >= 1,
            "self-heal must not torch the dir listings"
        );

        // Re-load: the session is re-parsed, served truthfully, and the
        // next store re-banks the fingerprint (one-time heal).
        let healed = load_sessions_from_dir(Some(&root));
        assert_eq!(opencode_session(&healed).metrics.tokens_input, 100);
        let raw = fs::read_to_string(&cache_path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let entry = doc
            .get("entries")
            .and_then(|entries| entries.as_object())
            .expect("entries map")
            .get(key)
            .expect("entry re-stored after heal");
        assert!(
            entry.get("opencode_siblings").is_some(),
            "the healed store must re-bank the sibling fingerprint"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

/// Control arm: non-opencode entries keep warm-serving (the composite
/// check must be a no-op for lanes without a sibling fan-out).
#[test]
fn non_opencode_entries_still_warm_serve() {
    let root = sandbox("control");
    let journal = root.join("sess.jsonl");
    fs::write(
        &journal,
        concat!(
            "{\"role\":\"session_meta\",\"timestamp\":\"2026-05-01T09:00:00Z\",\"ModelUsed\":\"claude-sonnet-4-5\"}\n",
            "{\"role\":\"meta\",\"ModelUsed\":\"claude-sonnet-4-5\",\"Usage\":{\"input_tokens\":120,\"output_tokens\":30}}\n",
        ),
    )
    .unwrap();

    let session = parse_file(&journal).expect("jsonl session parses");
    let mut cache = SessionCache::default();
    store_session(&journal, &session, &mut cache).expect("store");
    let warm = cached_session(&journal, &mut cache)
        .expect("non-opencode entries warm-serve (None == None arm)");
    assert_eq!(warm.metrics.tokens_input, 120);
    assert_eq!(warm.metrics.tokens_output, 30);
    assert!(
        cached_session(&journal, &mut cache).is_some(),
        "stable warm hits for untouched non-opencode journals"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Acceptance 6 (independent-review finding 1, PoC
/// poc-nonutf8-sibling.txt): a sibling whose FILE NAME is not valid
/// UTF-8 is admitted by the parser (`read_opencode_records` checks
/// is_dir/extension/content and never stringifies the name), so its
/// usage is counted — but a path-STRING-keyed fingerprint map can
/// never see it. The enumeration must DEGRADE to unbounded (and the
/// scope verifier to unverifiable), never skip-and-trust: such a
/// corpus re-parses every scan, so an in-place mutation of the
/// non-UTF8-named message's parts is always served fresh. Pre-fix,
/// the enumerator skipped the name, the fan-out below it was never
/// walked, and warm served stale totals across scans.
#[test]
#[cfg(unix)]
fn non_utf8_named_sibling_degrades_to_reparse_not_stale_serve() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let root = sandbox("nonutf8");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    write_corpus(&root, false, "m1");

    // The post-session sibling write with a non-UTF8-named message
    // doc (doc id "m2", parts under part/m2/ — the parser keys parts
    // by the doc id, which is plain ASCII inside the file).
    let m2_name = OsStr::from_bytes(b"m\xff2.json");
    fs::write(
        root.join("opencode/storage/message/sess_1").join(m2_name),
        r#"{"id": "m2", "sessionID": "sess_1", "role": "assistant", "modelID": "claude-sonnet-4-5", "time": {"created": 1764750002000}}"#,
    )
    .unwrap();
    fs::write(
        root.join("opencode/storage/part/m2/p_text2.json"),
        r#"{"id": "p_text2", "type": "text", "text": "the answer", "time": {"created": 1764750002500}}"#,
    )
    .unwrap();
    fs::write(
        root.join("opencode/storage/part/m2/p_fin2.json"),
        r#"{"id": "p_fin2", "type": "step-finish", "time": {"created": 1764750002600}, "tokens": {"input": 150, "output": 90}}"#,
    )
    .unwrap();

    with_home(&home, || {
        let session_file = root.join("opencode/storage/session/sess_1/sess_1.json");
        // The parser admits the non-UTF8-named sibling: cold totals
        // already count it (100 + 150 / 20 + 90).
        let cold = load_sessions_from_dir(Some(&root));
        let cold = opencode_session(&cold);
        assert_eq!(
            cold.metrics.tokens_input, 250,
            "cold parse must count the non-UTF8-named message (100 + 150)"
        );
        assert_eq!(cold.metrics.tokens_output, 110);

        // The scope containing a non-UTF8 name is unbounded/unverifiable
        // — it must never warm-serve.
        let mut cache = load_session_cache();
        assert!(
            cached_session(&session_file, &mut cache).is_none(),
            "a non-UTF8-named sibling scope must not warm-serve"
        );

        // The review PoC's mutation: in-place rewrite of that
        // message's part (session doc, m1 docs, and m1 parts all
        // untouched) — the exact write that used to be invisible.
        fs::write(
            root.join("opencode/storage/part/m2/p_fin2.json"),
            r#"{"id": "p_fin2", "type": "step-finish", "time": {"created": 1764750002600}, "tokens": {"input": 3000, "output": 600}}"#,
        )
        .unwrap();

        let warm = load_sessions_from_dir(Some(&root));
        let warm = opencode_session(&warm);
        assert_eq!(
            warm.metrics.tokens_input, 3100,
            "warm must serve the fresh truth (3000 + 100), not the stale 250"
        );
        assert_eq!(
            warm.metrics.tokens_output, 620,
            "warm must serve the fresh truth (600 + 20), not the stale 110"
        );

        // The miss persists: the next scan re-parses too (bounded
        // scopes only — this one never banks).
        let warm2 = load_sessions_from_dir(Some(&root));
        assert_eq!(
            opencode_session(&warm2).metrics.tokens_input,
            3100,
            "an unbounded scope re-parses every scan"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

/// Acceptance 5 (safety valve): when a message doc's id does not match
/// its file name, the parser's real part fan-out (part/<doc id>/) is
/// invisible to path-only enumeration — such a corpus must NEVER
/// warm-serve; it re-parses every scan instead of risking stale totals.
#[test]
fn unverifiable_sibling_scope_never_warm_serves() {
    let root = sandbox("unverifiable");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let session_file = write_corpus(&root, false, "mX");

    with_home(&home, || {
        // The corpus parses fine (the parser reads part/mX/ via the
        // doc id) and is served with truthful totals...
        let loaded = load_sessions_from_dir(Some(&root));
        let loaded = opencode_session(&loaded);
        assert_eq!(loaded.metrics.tokens_input, 100, "parse stays truthful");

        // ...but its warm entry can never be trusted: two consecutive
        // freshness checks must both miss.
        let mut cache = load_session_cache();
        assert!(
            cached_session(&session_file, &mut cache).is_none(),
            "unverifiable scope must not warm-serve (attempt 1)"
        );
        // Re-parse + re-store, then miss again — every scan.
        let fresh = parse_file(&session_file).expect("re-parse");
        store_session(&session_file, &fresh, &mut cache).expect("re-store");
        assert!(
            cached_session(&session_file, &mut cache).is_none(),
            "unverifiable scope must not warm-serve (attempt 2)"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

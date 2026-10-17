//! rm-1865: opencode session docs carry `directory` (and `projectID`) that
//! the parser never decoded — every opencode session rolled up as
//! by_project "unknown", even docs with an explicit directory (the in-tree
//! project_alpha fixture carries `"directory": "/repo"`).
//!
//! The session's working directory is decoded from the doc's `directory`,
//! falling back to the project doc referenced by `projectID`
//! (`storage/project/<id>.json`, v1 store layout), and rides the meta
//! event's cwd so `session_from_events` attributes the session to its
//! project (`resolve_project` prefers cwd). When neither is present the
//! empty cwd keeps the disclosed unknown-project fallback.
//!
//! rm-381 fence: a `directory` that is not ABSOLUTE (session doc or
//! project doc) is dropped to the disclosed unknown fallback with a
//! `cwd_relative:opencode` counter instead of riding the cwd —
//! pre-fence, attribution of a relative spelling depended on the
//! PROCESS cwd the scan happened to run from. rm-299 fence: the
//! project doc is a cache-fingerprint sidecar — editing it must
//! invalidate the warm cache entry (see
//! `project_doc_edits_invalidate_the_warm_cache_entry`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use agenttrace_core::{
    cached_session, load_session_cache, parse_file, resolve_project, store_session, Diagnostics,
    Metrics, Session,
};

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

/// Pin the ambient identity the session cache derives its path from
/// (the `doctor_honesty_contract` sandbox pattern) so cache-layer
/// tests never read or write the operator's real cache.
fn with_home(home: &Path, f: impl FnOnce()) {
    let _guard = env_lock();
    let keys = [
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "AGENTTRACE_SESSION_CACHE_DIR",
    ];
    let saved: Vec<(String, Option<std::ffi::OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), std::env::var_os(key)))
        .collect();
    fs::create_dir_all(home).unwrap();
    for (key, value) in &saved {
        if value.is_some() {
            std::env::remove_var(key);
        }
    }
    std::env::set_var("HOME", home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
    std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("session-cache"));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    for (key, value) in &saved {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

/// Run `f` with the PROCESS cwd moved into `dir` (serialized behind
/// the same lock as the env pin — both are process-global state that
/// parallel test threads must not interleave with each other).
fn with_chdir(dir: &Path, f: impl FnOnce()) {
    let _guard = env_lock();
    let previous = std::env::current_dir().expect("current dir");
    std::env::set_current_dir(dir).expect("chdir into the fixture repo");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::env::set_current_dir(&previous).expect("restore cwd");
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn write_session_doc(root: &Path, session_id: &str, extra: &str) -> PathBuf {
    fs::create_dir_all(root.join("opencode/storage/session").join(session_id)).unwrap();
    let path = root.join(format!(
        "opencode/storage/session/{session_id}/{session_id}.json"
    ));
    fs::write(&path, extra).unwrap();
    path
}

fn write_message(root: &Path, session_id: &str) {
    fs::create_dir_all(root.join(format!("opencode/storage/message/{session_id}"))).unwrap();
    fs::write(
        root.join(format!("opencode/storage/message/{session_id}/m1.json")),
        r#"{"id": "m1", "sessionID": "sess_1", "role": "assistant", "time": {"created": 1764750002000}}"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("opencode/storage/part/m1")).unwrap();
    fs::write(
        root.join("opencode/storage/part/m1/p1.json"),
        r#"{"id": "p1", "type": "text", "text": "answer text", "time": {"created": 1764750002500}}"#,
    )
    .unwrap();
}

/// Same fixture as [`write_message`] but the assistant message carries a
/// model — the rm-381 fence tests assert a disclosure counter that
/// rides the meta event, and that event is only emitted when the
/// session has a model, usage, or a usable directory (the dropped
/// relative spelling contributes none of those).
fn write_modeled_message(root: &Path, session_id: &str) {
    fs::create_dir_all(root.join(format!("opencode/storage/message/{session_id}"))).unwrap();
    fs::write(
        root.join(format!("opencode/storage/message/{session_id}/m1.json")),
        r#"{"id": "m1", "sessionID": "sess_1", "role": "assistant", "model": "claude-sonnet-4-5", "time": {"created": 1764750002000}}"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("opencode/storage/part/m1")).unwrap();
    fs::write(
        root.join("opencode/storage/part/m1/p1.json"),
        r#"{"id": "p1", "type": "text", "text": "answer text", "time": {"created": 1764750002500}}"#,
    )
    .unwrap();
}

#[test]
fn session_doc_directory_feeds_cwd_attribution() {
    let root = std::env::temp_dir().join(format!("at_rm1865_docdir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "attribution",
  "projectID": "prj_alpha",
  "title": "Attribution",
  "directory": "/home/dev/project-alpha",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_1");

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "/home/dev/project-alpha",
        "the doc's directory must ride the session's cwd so by_project resolves"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn project_doc_fallback_feeds_cwd_attribution() {
    // The doc omits `directory`, but its projectID references
    // storage/project/prj_beta.json — the project doc's directory is the
    // store's canonical home for that mapping.
    let root = std::env::temp_dir().join(format!("at_rm1865_projdoc_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "attribution",
  "projectID": "prj_beta",
  "title": "Attribution",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/project")).unwrap();
    fs::write(
        root.join("opencode/storage/project/prj_beta.json"),
        r#"{"id": "prj_beta", "directory": "/srv/work/beta", "time": {"created": 1764750000000}}"#,
    )
    .unwrap();

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "/srv/work/beta",
        "the referenced project doc's directory must serve as the session cwd"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn missing_directory_keeps_the_disclosed_unknown_fallback() {
    // Neither a doc directory nor a resolvable project doc: cwd stays
    // empty and the session still parses (by_project keeps its existing
    // unknown fallback — honest, not invented).
    let root = std::env::temp_dir().join(format!("at_rm1865_none_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "no-directory",
  "projectID": "prj_missing",
  "title": "No directory",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_1");
    // No storage/project/prj_missing.json on purpose.

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "an unresolvable project must degrade to the disclosed unknown fallback"
    );
    assert!(session.metrics.events_total > 0);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn relative_directory_spellings_degrade_to_the_disclosed_unknown_fallback() {
    // rm-381 fence: a relative `directory` has no anchor the parse
    // knows — pre-fence it rode the cwd verbatim and attribution
    // depended on the PROCESS cwd the scan ran from (live-proven: the
    // same `src` doc attributed `unknown` from a git-root cwd and
    // `src` from a plain dir). Plain, dot-prefixed and traversal
    // spellings all degrade to the disclosed unknown fallback and
    // count the drop.
    for (index, spelling) in ["src", "./src", "../../vault/stolen"].iter().enumerate() {
        let root =
            std::env::temp_dir().join(format!("at_rm381_doc_{}_{index}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let session_file = write_session_doc(
            &root,
            "sess_1",
            &format!(
                r#"{{"id": "sess_1", "slug": "relative", "projectID": "prj_absent", "title": "Relative", "directory": "{spelling}", "time": {{"created": 1764750000000, "updated": 1764750004000}}}}"#
            ),
        );
        write_modeled_message(&root, "sess_1");

        let session = parse_file(&session_file).unwrap();
        assert_eq!(
            session.cwd, "",
            "relative spelling {spelling:?} must not ride the session cwd"
        );
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("cwd_relative:opencode"),
            Some(&1),
            "the dropped spelling {spelling:?} must be disclosed"
        );

        let _ = fs::remove_dir_all(&root);
    }
}

#[test]
fn relative_project_doc_directory_degrades_and_absolute_falls_through() {
    // rm-381 fence, project-doc arm. A relative directory inside the
    // project doc is the same unanchored spelling the doc arm refuses
    // — drop it (disclosed) rather than attribute through the process
    // cwd. And when the session doc's own `directory` was relative but
    // the project doc carries an absolute one, the project doc still
    // serves, with the doc-side drop still disclosed.
    let root = std::env::temp_dir().join(format!("at_rm381_projdoc_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    // Arm 1: only the project doc exists, and its directory is relative.
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "relative-project",
  "projectID": "prj_rel",
  "title": "Relative project",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_modeled_message(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/project")).unwrap();
    fs::write(
        root.join("opencode/storage/project/prj_rel.json"),
        r#"{"id": "prj_rel", "directory": "beta/relative", "time": {"created": 1764750000000}}"#,
    )
    .unwrap();

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "a relative project-doc directory must not ride the session cwd: {}",
        session.cwd
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("cwd_relative:opencode"),
        Some(&1),
        "the dropped project-doc spelling must be disclosed"
    );

    // Arm 2: relative doc `directory` + absolute project-doc directory.
    let session_file = write_session_doc(
        &root,
        "sess_2",
        r#"{
  "id": "sess_2",
  "slug": "mixed",
  "projectID": "prj_abs",
  "title": "Mixed",
  "directory": "src",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_modeled_message(&root, "sess_2");
    fs::write(
        root.join("opencode/storage/project/prj_abs.json"),
        r#"{"id": "prj_abs", "directory": "/srv/absolute-beta", "time": {"created": 1764750000000}}"#,
    )
    .unwrap();

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "/srv/absolute-beta",
        "the absolute project doc must serve after the relative doc spelling drops"
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("cwd_relative:opencode"),
        Some(&1),
        "the dropped doc-side spelling must still be disclosed"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn project_doc_edits_invalidate_the_warm_cache_entry() {
    // rm-299 fence: the project doc is a SIDECAR of the session doc —
    // its edits must invalidate the warm entry exactly like an edit of
    // the session doc itself. Pre-fence the stale project was served
    // forever (live-proven: cold /srv/alpha → sidecar edited → warm
    // scans still alpha until the cache was wiped). The replacement
    // directory is a different LENGTH, so the size arm of the
    // fingerprint moves regardless of mtime granularity.
    let home = std::env::temp_dir().join(format!("at_rm299_home_{}", std::process::id()));
    let root = std::env::temp_dir().join(format!("at_rm299_warm_{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&root);
    with_home(&home, || {
        let session_file = write_session_doc(
            &root,
            "sess_1",
            r#"{
  "id": "sess_1",
  "slug": "warm",
  "projectID": "prj_alpha",
  "title": "Warm",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
        );
        write_message(&root, "sess_1");
        let project_doc = root.join("opencode/storage/project/prj_alpha.json");
        fs::create_dir_all(project_doc.parent().unwrap()).unwrap();
        fs::write(
            &project_doc,
            r#"{"id": "prj_alpha", "directory": "/srv/alpha", "time": {"created": 1764750000000}}"#,
        )
        .unwrap();

        let mut cache = load_session_cache();
        let cold = parse_file(&session_file).unwrap();
        assert_eq!(cold.cwd, "/srv/alpha");
        store_session(&session_file, &cold, &mut cache).unwrap();
        let warm = cached_session(&session_file, &mut cache)
            .expect("an unchanged sidecar must stay a warm hit");
        assert_eq!(warm.cwd, "/srv/alpha");

        // Edit ONLY the sidecar — the session doc's own stat never moves.
        fs::write(
            &project_doc,
            r#"{"id": "prj_alpha", "directory": "/srv/renamed-project-beta", "time": {"created": 1764750000000}}"#,
        )
        .unwrap();
        assert!(
            cached_session(&session_file, &mut cache).is_none(),
            "a sidecar edit must invalidate the warm entry"
        );

        let reparsed = parse_file(&session_file).unwrap();
        assert_eq!(reparsed.cwd, "/srv/renamed-project-beta");
        store_session(&session_file, &reparsed, &mut cache).unwrap();
        let warm_again =
            cached_session(&session_file, &mut cache).expect("the re-stored entry is warm again");
        assert_eq!(warm_again.cwd, "/srv/renamed-project-beta");
    });
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn relative_cwd_never_resolves_through_the_process_cwd() {
    // rm-381 fence belt: even a relative cwd that reaches
    // resolve_project — a lane without its own intake fence, or a
    // stale cache row — must resolve to its lexical self. Pre-fence,
    // with the process sitting in a git root that contains `src/`, the
    // ancestor walk probed `src/.git`, then the EMPTY path's bare
    // `.git` marker (the process cwd's own repo) and returned an empty
    // root that displayed as `unknown` — the same session attributed
    // differently depending on where the operator ran the scan.
    let tmp = std::env::temp_dir().join(format!("at_rm381_walk_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::create_dir_all(tmp.join(".git")).unwrap();

    with_chdir(&tmp, || {
        let session = Session {
            name: "walk".to_string(),
            path: "/opt/store/opencode/storage/session/s/s.json".to_string(),
            cwd: "src".to_string(),
            branch: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 0,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        let identity = resolve_project(&session);
        assert_eq!(identity.resolution, "cwd");
        assert_eq!(identity.root, "src");
        assert_eq!(identity.display_name, "src");
        assert_eq!(identity.id, "src");
    });

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn hostile_project_ids_cannot_escape_the_store() {
    // Review F1 (2026-10-16): projectID is joined into a filesystem
    // path, and `Path::join` is not a confinement — an ABSOLUTE id
    // replaces the base path outright (the joined path becomes
    // <payload>/evil.json outside the store entirely) and a `../` id
    // walks out of `storage/project`. Both spellings used to read an
    // out-of-store doc and import its `directory` as this session's cwd
    // (live-proven pre-fix); both must now degrade to the disclosed
    // unknown fallback even with a payload doc sitting at the escaped
    // location.
    let tmp = std::env::temp_dir();
    let root = tmp.join(format!("at_rm1865f1_hostile_{}", std::process::id()));
    let payload = tmp.join(format!("at_rm1865f1_payload_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&payload);
    fs::create_dir_all(&payload).unwrap();
    fs::write(
        payload.join("evil.json"),
        r#"{"directory": "/zzz-pwned-marker/absolute"}"#,
    )
    .unwrap();

    // Arm 1: absolute id — pre-fix the joined path replaced the store
    // base and read the payload doc.
    let absolute_id = payload.join("evil").to_string_lossy().to_string();
    let session_file = write_session_doc(
        &root,
        "sess_1",
        &format!(
            r#"{{"id": "sess_1", "slug": "hostile", "projectID": "{absolute_id}", "title": "Hostile", "time": {{"created": 1764750000000, "updated": 1764750004000}}}}"#
        ),
    );
    write_message(&root, "sess_1");
    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "an absolute projectID must not read outside the store: {}",
        session.cwd
    );

    // Arm 2: traversal id — storage/project/../../../escape/evil.json
    // resolves inside root but OUTSIDE the opencode store (root/opencode
    // is the store base); pre-fix it read the payload doc too.
    fs::create_dir_all(root.join("escape")).unwrap();
    fs::write(
        root.join("escape/evil.json"),
        r#"{"directory": "/zzz-pwned-marker/traversal"}"#,
    )
    .unwrap();
    let session_file = write_session_doc(
        &root,
        "sess_2",
        r#"{
  "id": "sess_2",
  "slug": "hostile",
  "projectID": "../../../escape/evil",
  "title": "Hostile",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_2");
    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "a ../-spelled projectID must not walk out of the store: {}",
        session.cwd
    );
    assert!(
        session.metrics.events_total > 0,
        "the session still parses — a rejected id is the disclosed unknown, not an error"
    );

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&payload);
}

#[test]
fn oversize_project_doc_is_capped_before_read() {
    // Review F1 (rm-1437 discipline): the project doc is stat-capped
    // before the read — one byte over the transcript cap degrades to
    // the disclosed unknown fallback instead of admitting the file
    // into memory. The doc is VALID JSON whose directory sits at the
    // front, so only the cap can explain an empty cwd.
    const CAP_BYTES: usize = 10 * 1024 * 1024;
    let root = std::env::temp_dir().join(format!("at_rm1865f1_cap_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_file = write_session_doc(
        &root,
        "sess_1",
        r#"{
  "id": "sess_1",
  "slug": "cap",
  "projectID": "prj_huge",
  "title": "Cap",
  "time": {"created": 1764750000000, "updated": 1764750004000}
}"#,
    );
    write_message(&root, "sess_1");
    fs::create_dir_all(root.join("opencode/storage/project")).unwrap();
    let mut body = String::from(r#"{"directory": "/zzz-oversize-marker", "pad": ""#);
    body.push_str(&"x".repeat(CAP_BYTES));
    body.push_str("\"}");
    fs::write(root.join("opencode/storage/project/prj_huge.json"), body).unwrap();

    let session = parse_file(&session_file).unwrap();
    assert_eq!(
        session.cwd, "",
        "an over-cap project doc must be skipped before the read: {}",
        session.cwd
    );
    assert!(session.metrics.events_total > 0);

    let _ = fs::remove_dir_all(&root);
}

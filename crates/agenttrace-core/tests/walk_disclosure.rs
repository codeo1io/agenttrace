//! Walk-lane disclosure tests — run 82a64f5a cycle 2, batch of
//! record "Nothing read-side is silent: disclosure completion across
//! the discovery/intake lanes" (rm-2126 CORE, rm-2127, rm-1888 arm,
//! rm-367 arm, rm-2128).
//!
//! Five arms, one pipeline: every class of read-side suppression the
//! walk/parse/cache lanes perform is disclosed on a named counter —
//! never silently dropped. All five were proven red-first by this
//! run's assess probes (p2/p3 depth + explicit `-d`, p5b mode-000
//! subtree, p10 hardlink/symlink aliases, p7 read-only cache dir) or
//! sourced by research (C8: ccusage 20.0.37 requestless resp_ rows,
//! issue #1935 — the claude-lane id-less usage emission is the same
//! count-once hole).
//!
//! Contracts pinned here:
//! - rm-2126: a depth-bound clip that loses content counts EXACTLY
//!   (a clip whose peek finds nothing stays 0), on BOTH the uncached
//!   walk (`find_session_files_reported`) and the cached loader lane
//!   (`LoadReport::depth_clipped_dirs`), auto and explicit `-d`; the
//!   `-d` escape hatch (point at the deepest level) recovers the
//!   file — the bound changes, the silence does not.
//! - rm-2127: an unreadable subtree counts and its content is
//!   excluded while siblings survive; NotFound stays silent.
//! - rm-1888 arm: same-inode aliases (hardlink AND symlink) bank
//!   exactly once and the suppressed alias count is exact.
//! - rm-367 arm: a cache persist that cannot write (read-only cache
//!   dir) never fails the read and never panics; the wrapper
//!   `persist_session_cache_disclosed` owns the stderr note (format
//!   pinned in its doc comment; the one-line/no-noise shape is
//!   verified binary-side in the phase evidence).
//! - rm-2128: a claude usage emission without a message id banks its
//!   own meta event (legacy behavior, unchanged — a fixture pins it)
//!   AND is counted on the journal's disclosure carrier
//!   (`claude_unkeyed_usage_emissions`); a healthy keyed corpus
//!   emits zero.
//!
//! Every HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR env var is pinned
//! per-test (same pattern as count_honesty_admission.rs): the suite
//! never reads or writes the host's real session state or the
//! shared default session cache.

use agenttrace_core::{
    find_session_files_reported, load_session_cache, load_sessions_with_progress_from_cache,
    parse_file, persist_session_cache_disclosed, store_session, LoadOptions, SessionCache,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within
/// this binary (discovery reads env at call time). Each call gets
/// its own temp home AND its own session cache dir — the host
/// default cache is a shared fleet surface.
fn with_pinned_home(home: &Path, f: impl FnOnce()) {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = GUARD.get_or_init(|| Mutex::new(())).lock().unwrap();
    let previous_home = std::env::var_os("HOME");
    let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
    let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
    let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
    let previous_session_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
    std::env::set_var("HOME", home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
    std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
    std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("session-cache"));
    f();
    restore_env("HOME", previous_home);
    restore_env("XDG_CONFIG_HOME", previous_xdg_config);
    restore_env("XDG_CACHE_HOME", previous_xdg_cache);
    restore_env("XDG_DATA_HOME", previous_xdg_data);
    restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_session_cache);
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        None => std::env::remove_var(key),
        Some(value) => std::env::set_var(key, value),
    }
}

fn temp_home(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("at-walk-disclosure-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp home");
    dir
}

/// A minimal valid claude-code journal: one user message + one
/// assistant message with usage. Parses through the claude lane
/// end to end.
fn claude_journal() -> String {
    [
        r#"{"type":"user","timestamp":"2026-10-04T07:16:47.000Z","message":{"role":"user","content":[{"type":"text","text":"hello"}]}}"#.to_string(),
        r#"{"type":"assistant","timestamp":"2026-10-04T07:16:48.000Z","message":{"id":"msg_0001","usage":{"input_tokens":100,"output_tokens":50}}}"#.to_string(),
    ]
    .join("\n")
        + "\n"
}

fn write_journal(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
    fs::write(path, body).expect("write journal");
}

// ---------------------------------------------------------------------------
// rm-2126 — depth-bound clip disclosed, exact, on both lanes
// ---------------------------------------------------------------------------

/// A session file nested one level past the claude depth bound
/// (root `.claude/projects` = depth 0, max 3; `d3` sits at depth 4)
/// used to vanish with zero output on the auto lane AND the explicit
/// `-d` lane (assess probe p3). Now the clip is counted, and the
/// `-d` escape hatch (point at the deepest level) recovers the file.
#[test]
fn depth_clip_disclosed_on_both_lanes() {
    let home = temp_home("depth-clip");
    let projects = home.join(".claude").join("projects");
    // d1 (2) -> d2 (3) -> d3 (4 > 3, clipped): the deepest file is
    // invisible to a walk rooted at `projects`.
    let deep_file = projects
        .join("-proj")
        .join("d1")
        .join("d2")
        .join("d3")
        .join("session-00000000-0000-4000-8000-000000000001.jsonl");
    write_journal(&deep_file, &claude_journal());

    with_pinned_home(&home, || {
        // Auto lane, uncached walk: one clipped dir, file invisible.
        let (files, walk) = find_session_files_reported(None);
        assert_eq!(walk.depth_clipped_dirs, 1, "auto lane must disclose the clip");
        assert!(
            !files.iter().any(|path| path.ends_with(&deep_file)),
            "clipped file stays out of the inventory (bound unchanged)"
        );
        assert_eq!(walk.unreadable_dirs, 0);
        assert_eq!(walk.same_inode_files_suppressed, 0);

        // Explicit -d lane rooted at `projects`: same bound, same clip,
        // same disclosure — the explicit lane must not be the lane that
        // hides it (assess probe p2/p3 divergence).
        let (files, walk) = find_session_files_reported(Some(&projects));
        assert_eq!(
            walk.depth_clipped_dirs, 1,
            "explicit -d lane must disclose the clip too"
        );
        assert!(!files.iter().any(|path| path.ends_with(&deep_file)));

        // Cached loader lane (the TUI/overview path): the flat
        // LoadReport field carries the same number, cold AND warm —
        // a replayed listing must not resurrect or double the clip.
        let mut cache = SessionCache::default();
        let options = LoadOptions::default();
        let cold = load_sessions_with_progress_from_cache(None, &options, &mut cache, |_| {});
        assert_eq!(
            cold.depth_clipped_dirs, 1,
            "LoadReport must carry the clip count (cold)"
        );
        let warm = load_sessions_with_progress_from_cache(None, &options, &mut cache, |_| {});
        assert_eq!(
            warm.depth_clipped_dirs, 1,
            "warm replay must agree with the cold walk"
        );
        assert_eq!(
            cold.depth_clipped_dirs, warm.depth_clipped_dirs,
            "warm == cold"
        );

        // Escape hatch: -d at the deepest level re-roots the walk
        // (bound 4 from there), recovers the file, discloses nothing.
        let (files, walk) = find_session_files_reported(Some(&projects.join("-proj").join("d1")));
        assert_eq!(walk.depth_clipped_dirs, 0, "deeper -d root walks it in full");
        assert!(
            files.iter().any(|path| path.ends_with(&deep_file)),
            "the escape hatch recovers the clipped file"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

/// The counter is EXACT, not paranoid: a clip whose peek finds no
/// eligible content (an empty deepest dir) stays zero, so a healthy
/// corpus carries an all-zero disclosure and its serialized reports
/// stay byte-identical to pre-disclosure binaries (no counter noise).
#[test]
fn depth_clip_silent_when_nothing_lost() {
    let home = temp_home("clip-silent");
    let projects = home.join(".claude").join("projects");
    // d3 exists but is EMPTY: the peek finds nothing, no loss, no
    // counter. A sibling valid session sits at the legal depth.
    fs::create_dir_all(projects.join("-proj").join("d1").join("d2").join("d3"))
        .expect("create empty deep dir");
    let legal = projects
        .join("-proj")
        .join("session-00000000-0000-4000-8000-000000000002.jsonl");
    write_journal(&legal, &claude_journal());

    with_pinned_home(&home, || {
        let (files, walk) = find_session_files_reported(None);
        assert_eq!(walk.depth_clipped_dirs, 0, "empty clip is not a loss");
        assert_eq!(walk.unreadable_dirs, 0);
        assert_eq!(walk.same_inode_files_suppressed, 0);
        assert!(files.iter().any(|path| path.ends_with(&legal)));
    });
    let _ = fs::remove_dir_all(&home);
}

/// A corpus entirely inside the bound discloses nothing anywhere:
/// disclosure-only means zero counters, zero report deltas, zero
/// behavior change on valid corpora.
#[test]
fn valid_corpus_discloses_nothing() {
    let home = temp_home("valid-corpus");
    let projects = home.join(".claude").join("projects");
    let legal = projects
        .join("-proj")
        .join("session-00000000-0000-4000-8000-000000000003.jsonl");
    write_journal(&legal, &claude_journal());

    with_pinned_home(&home, || {
        let (files, walk) = find_session_files_reported(None);
        assert_eq!(walk.depth_clipped_dirs, 0);
        assert_eq!(walk.unreadable_dirs, 0);
        assert_eq!(walk.same_inode_files_suppressed, 0);
        assert_eq!(files.len(), 1);

        let mut cache = load_session_cache();
        let options = LoadOptions::default();
        let report =
            load_sessions_with_progress_from_cache(None, &options, &mut cache, |_| {});
        assert_eq!(report.depth_clipped_dirs, 0);
        assert_eq!(report.unreadable_dirs, 0);
        assert_eq!(report.same_inode_files_suppressed, 0);
        assert_eq!(report.parsed, 1);
        assert_eq!(report.parse_failures, 0);
    });
    let _ = fs::remove_dir_all(&home);
}

// ---------------------------------------------------------------------------
// rm-2127 — unreadable subtree disclosed, NotFound stays silent
// ---------------------------------------------------------------------------

/// A mode-000 subdir (EACCES under uid 1000) hid its whole subtree
/// with rc 0 and zero stderr (assess probe p5b). Now it counts as
/// `unreadable_dirs`, its files stay out, and its SIBLINGS survive.
#[test]
#[cfg(unix)]
fn unreadable_subdir_disclosed_sibling_survives() {
    use std::os::unix::fs::PermissionsExt;

    let home = temp_home("unreadable");
    let project = home.join(".claude").join("projects").join("-proj");
    let sibling = project.join("session-00000000-0000-4000-8000-000000000004.jsonl");
    write_journal(&sibling, &claude_journal());
    let locked_file = project.join("locked").join("session.jsonl");
    write_journal(&locked_file, &claude_journal());
    fs::set_permissions(project.join("locked"), fs::Permissions::from_mode(0o000))
        .expect("chmod 000");

    // Root cannot bypass mode bits: if it can, the assertion would be
    // vacuous — skip instead of lying.
    let meaningful = fs::read(&locked_file).is_err();
    with_pinned_home(&home, || {
        let (files, walk) = find_session_files_reported(None);
        if meaningful {
            assert_eq!(walk.depth_clipped_dirs, 0);
            assert_eq!(
                walk.unreadable_dirs, 1,
                "the mode-000 subtree must be disclosed"
            );
            assert!(
                !files.iter().any(|path| path.ends_with("locked/session.jsonl")),
                "unreadable content stays out of the inventory"
            );
        }
        assert!(
            files.iter().any(|path| path.ends_with(&sibling)),
            "the readable sibling survives the unreadable subtree"
        );
    });
    // Cleanup needs the bit back.
    let _ = fs::set_permissions(project.join("locked"), fs::Permissions::from_mode(0o700));
    let _ = fs::remove_dir_all(&home);
}

/// NotFound is the normal missing-root race (an agent home without a
/// `.claude` tree) and stays silent — only surprising failures count.
#[test]
fn missing_root_stays_silent() {
    let home = temp_home("notfound");
    with_pinned_home(&home, || {
        let (files, walk) = find_session_files_reported(None);
        assert!(files.is_empty());
        assert_eq!(walk.depth_clipped_dirs, 0);
        assert_eq!(walk.unreadable_dirs, 0);
        assert_eq!(walk.same_inode_files_suppressed, 0);

        // An explicit -d at a path that does not exist: same rule.
        let (files, walk) =
            find_session_files_reported(Some(&home.join("no-such-dir")));
        assert!(files.is_empty());
        assert_eq!(walk.unreadable_dirs, 0);
    });
    let _ = fs::remove_dir_all(&home);
}

// ---------------------------------------------------------------------------
// rm-1888 arm — same-inode aliases bank once, count exactly
// ---------------------------------------------------------------------------

/// A hardlink and a symlink alias of one physical transcript are ONE
/// session (rm-597 same-file admission; assess probe p10 proved the
/// suppression but no counter existed anywhere). The suppressed
/// alias count is exact: 2 aliases, 2 counted, 1 banked file.
#[test]
#[cfg(unix)]
fn same_inode_aliases_suppressed_exactly() {
    let home = temp_home("aliases");
    let project = home.join(".claude").join("projects").join("-proj");
    let real = project.join("session-00000000-0000-4000-8000-000000000005.jsonl");
    write_journal(&real, &claude_journal());
    let hardlink = project.join("alias-hardlink.jsonl");
    fs::hard_link(&real, &hardlink).expect("hard link");
    let symlink = project.join("alias-symlink.jsonl");
    std::os::unix::fs::symlink(&real, &symlink).expect("symlink");

    with_pinned_home(&home, || {
        // Auto lane (aggregation): one physical transcript banks once.
        let (files, walk) = find_session_files_reported(None);
        let banked = files
            .iter()
            .filter(|path| path.ends_with("000000000005.jsonl") || path.ends_with("alias-"))
            .count();
        assert_eq!(banked, 1, "aliases bank exactly once: {files:?}");
        assert_eq!(
            walk.same_inode_files_suppressed, 2,
            "both aliases are counted (hardlink + symlink)"
        );
        assert_eq!(walk.depth_clipped_dirs, 0);
        assert_eq!(walk.unreadable_dirs, 0);

        // Cached lane agrees on a warm replay (rm-597: the replayed
        // listing re-applies the same-file admission).
        let mut cache = load_session_cache();
        let options = LoadOptions::default();
        let report = load_sessions_with_progress_from_cache(None, &options, &mut cache, |_| {});
        assert_eq!(report.same_inode_files_suppressed, 2);
        assert_eq!(report.parsed, 1);
    });
    let _ = fs::remove_dir_all(&home);
}

// ---------------------------------------------------------------------------
// rm-367 arm — a cache persist that cannot write never fails the read
// ---------------------------------------------------------------------------

/// The read lane's `let _ = save_session_cache(&mut cache)` (assess
/// probe p7: read-only cache dir, rc 0, silent cold start) now rides
/// `persist_session_cache_disclosed`: the read still succeeds, the
/// wrapper never panics, and a writable cache still saves (zero
/// noise). The one-line stderr note's exact bytes are the wrapper's
/// documented contract (verified binary-side in the phase evidence).
#[test]
fn read_only_cache_dir_never_fails_the_read() {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let home = temp_home("ro-cache");
    let project = home.join(".claude").join("projects").join("-proj");
    let journal = project.join("session-00000000-0000-4000-8000-000000000006.jsonl");
    write_journal(&journal, &claude_journal());
    let cache_dir = home.join("session-cache");
    fs::create_dir_all(&cache_dir).expect("create cache dir");

    #[cfg(unix)]
    fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o555)).expect("chmod 555");

    with_pinned_home(&home, || {
        // The anchor site itself: the load path parses the corpus and
        // the failed save neither fails the read nor panics. On unix
        // with mode 555 the save inside the wrapper fails; the read
        // legs below must all pass regardless.
        let mut cache = load_session_cache();
        let session = parse_file(&journal).expect("parse journal");
        store_session(&journal, session, &mut cache);
        // The disclosed wrapper: must return normally whether or not
        // the save fails (both branches exercised across platforms).\n        persist_session_cache_disclosed(&mut cache);

        // And the corpus is still fully readable through the loader.
        let mut cache = load_session_cache();
        let options = LoadOptions::default();
        let report = load_sessions_with_progress_from_cache(None, &options, &mut cache, |_| {});
        assert_eq!(report.parsed, 1, "the read lane still parses the corpus");
        assert_eq!(report.parse_failures, 0);
    });

    #[cfg(unix)]
    let _ = fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o700));
    let _ = fs::remove_dir_all(&home);
}

/// Zero noise on a healthy cache: the wrapper saves normally and
/// nothing about the read path changes when the write succeeds.
#[test]
fn writable_cache_dir_persists_normally() {
    let home = temp_home("rw-cache");
    let project = home.join(".claude").join("projects").join("-proj");
    let journal = project.join("session-00000000-0000-4000-8000-000000000007.jsonl");
    write_journal(&journal, &claude_journal());

    with_pinned_home(&home, || {
        let mut cache = load_session_cache();
        let session = parse_file(&journal).expect("parse journal");
        store_session(&journal, session, &mut cache);
        persist_session_cache_disclosed(&mut cache);
        assert!(
            cache.path.is_file(),
            "the cache file landed (path: {})",
            cache.path.display()
        );
        let options = LoadOptions::default();
        let warm = load_sessions_with_progress_from_cache(None, &options, &mut cache, |_| {});
        assert!(warm.cache_hits >= 1, "the persisted entry is reused");
    });
    let _ = fs::remove_dir_all(&home);
}

// ---------------------------------------------------------------------------
// rm-2128 — claude id-less usage emission disclosed, banking unchanged
// ---------------------------------------------------------------------------

/// A claude usage emission without a message id cannot ride the
/// rm-601 fold: each id-less block banks its own meta event (the
/// exact inflation class ccusage fixed for requestless resp_ rows,
/// 20.0.37 / issue #1935). Banking is UNCHANGED (a fixture pins it);
/// the hole is now counted on the journal's disclosure carrier.
#[test]
fn unkeyed_usage_emission_counted_banking_unchanged() {
    let home = temp_home("unkeyed-usage");
    let journal = home
        .join(".claude")
        .join("projects")
        .join("-proj")
        .join("session-00000000-0000-4000-8000-000000000008.jsonl");
    let body = [
        r#"{"type":"user","timestamp":"2026-10-04T07:16:47.000Z","message":{"role":"user","content":[{"type":"text","text":"hello"}]}}"#,
        // One keyed message, two streamed blocks: folds to ONE meta.
        r#"{"type":"assistant","timestamp":"2026-10-04T07:16:48.000Z","message":{"id":"msg_0001","usage":{"input_tokens":10,"output_tokens":5}}}"#,
        r#"{"type":"assistant","timestamp":"2026-10-04T07:16:49.000Z","message":{"id":"msg_0001","usage":{"input_tokens":20,"output_tokens":6}}}"#,
        // Two id-less emissions: each banks its own meta (legacy).
        r#"{"type":"assistant","timestamp":"2026-10-04T07:16:50.000Z","message":{"usage":{"input_tokens":7,"output_tokens":3}}}"#,
        r#"{"type":"assistant","timestamp":"2026-10-04T07:16:51.000Z","message":{"usage":{"input_tokens":8,"output_tokens":4}}}"#,
    ]
    .join("\n")
        + "\n";
    write_journal(&journal, &body);

    let session = parse_file(&journal).expect("parse journal");
    // Banking unchanged: the keyed pair folds to one meta (input
    // 20 = running max), the two id-less emissions stay separate.
    let usage_metas: Vec<_> = session
        .events
        .iter()
        .filter(|event| event.role == "meta" && !event.usage.is_empty())
        .collect();
    assert_eq!(usage_metas.len(), 3, "1 folded + 2 unkeyed metas");
    let folded = usage_metas
        .iter()
        .find(|event| event.usage.get("input_tokens") == Some(&20))
        .expect("the keyed pair folded to one running-max meta");
    assert_eq!(folded.usage.get("output_tokens"), Some(&6));
    // The disclosure carrier states the hole exactly.
    let carrier = session
        .events
        .iter()
        .find(|event| event.role == "meta" && event.usage.is_empty())
        .expect("the disclosure carrier meta exists");
    assert_eq!(
        carrier.disclosure_counters.get("claude_unkeyed_usage_emissions"),
        Some(&2),
        "both id-less emissions are counted"
    );
    let _ = home;
}

/// A healthy keyed corpus emits zero: the counter only fires on the
/// hole, so serialized sessions stay byte-identical when nothing was
/// suppressed (disclosure-only).
#[test]
fn healthy_journal_counts_no_unkeyed_emissions() {
    let home = temp_home("healthy-usage");
    let journal = home
        .join(".claude")
        .join("projects")
        .join("-proj")
        .join("session-00000000-0000-4000-8000-000000000009.jsonl");
    let body = [
        r#"{"type":"user","timestamp":"2026-10-04T07:16:47.000Z","message":{"role":"user","content":[{"type":"text","text":"hello"}]}}"#,
        r#"{"type":"assistant","timestamp":"2026-10-04T07:16:48.000Z","message":{"id":"msg_0001","usage":{"input_tokens":10,"output_tokens":5}}}"#,
        r#"{"type":"assistant","timestamp":"2026-10-04T07:16:49.000Z","message":{"id":"msg_0002","usage":{"input_tokens":11,"output_tokens":6}}}"#,
    ]
    .join("\n")
        + "\n";
    write_journal(&journal, &body);

    let session = parse_file(&journal).expect("parse journal");
    for event in &session.events {
        assert_ne!(
            event
                .disclosure_counters
                .get("claude_unkeyed_usage_emissions"),
            Some(&2),
            "no counter on a healthy journal"
        );
        assert!(
            !event
                .disclosure_counters
                .contains_key("claude_unkeyed_usage_emissions")
                || event.disclosure_counters["claude_unkeyed_usage_emissions"] == 0,
            "healthy corpus emits zero unkeyed-usage counters"
        );
    }
    let _ = home;
}

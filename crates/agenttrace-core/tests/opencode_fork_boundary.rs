// rm-805: opencode fork boundary-aware accounting — the refinement of
// rm-548's whole-drop exclusion (docs/guides/opencode-fork-marker.md).
// A fork copy that continues past its fork point keeps its session but
// counts only post-boundary work; an orphaned fork (parent doc gone)
// counts fully; a pure replay still drops whole — every drop or trim
// disclosed with its token and estimated-cost magnitude.
use agenttrace_core::{
    data_health_scoped, load_sessions_from_dir, load_sessions_with_progress, LoadOptions,
};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn temp_root(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    if let Some(value) = previous {
        std::env::set_var(key, value);
    } else {
        std::env::remove_var(key);
    }
}

fn with_home(home: &Path, f: impl FnOnce()) {
    // Mirrors discovery_contract::with_home — the env guard is dropped
    // before a caught panic resumes so a mutex cannot cascade.
    let result = {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_home = std::env::var_os("HOME");
        let previous_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_cache = std::env::var_os("XDG_CACHE_HOME");
        let previous_data = std::env::var_os("XDG_DATA_HOME");
        let previous_session_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore_env("HOME", previous_home);
        restore_env("XDG_CONFIG_HOME", previous_config);
        restore_env("XDG_CACHE_HOME", previous_cache);
        restore_env("XDG_DATA_HOME", previous_data);
        restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_session_cache);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

struct StorageFixture {
    storage: PathBuf,
}

/// One message record + its text part under `storage`.
fn opencode_message(home: &Path, session: &str, id: &str, created_millis: i64, tokens: &str) {
    let root = home.join(".local/share/opencode/storage");
    let dir = root.join("message").join(session);
    fs::create_dir_all(&dir).expect("create message dir");
    fs::write(
        dir.join(format!("{id}.json")),
        format!(
            r#"{{"id":"{id}","sessionID":"{session}","role":"assistant","modelID":"claude-sonnet-4-5","time":{{"created":{created_millis}}},"tokens":{tokens}}}"#
        ),
    )
    .expect("write message record");
    let part_dir = root.join("part").join(id);
    fs::create_dir_all(&part_dir).expect("create part dir");
    fs::write(
        part_dir.join("p1.json"),
        r#"{"id":"p1","type":"text","text":"fork lane"}"#,
    )
    .expect("write part record");
}

fn opencode_session_doc(home: &Path, id: &str, parent: Option<&str>, created_millis: i64) {
    let info = home.join(".local/share/opencode/storage/session/info");
    fs::create_dir_all(&info).expect("create info dir");
    let parent_field = parent
        .map(|parent| format!(r#","parentID":"{parent}""#))
        .unwrap_or_default();
    fs::write(
        info.join(format!("{id}.json")),
        format!(
            r#"{{"id":"{id}","projectID":"proj-test","title":"{id}","timeCreated":{created_millis}{parent_field}}}"#
        ),
    )
    .expect("write session doc");
}

/// Parent with one 100/50 message; fork at +100s whose messages straddle
/// the boundary (replay 300/100 at boundary-50s, continuation 40/20 at
/// boundary+60s); plus an orphaned fork of a deleted parent (70/30).
fn seed_boundary_corpus(home: &Path) -> StorageFixture {
    opencode_session_doc(home, "ses-parent", None, 1_780_000_000_000);
    opencode_session_doc(home, "ses-fork", Some("ses-parent"), 1_780_000_100_000);
    opencode_session_doc(home, "ses-orphan", Some("ses-gone"), 1_780_000_200_000);

    opencode_message(
        home,
        "ses-parent",
        "m1",
        1_780_000_000_000,
        r#"{"input":100,"output":50}"#,
    );
    opencode_message(
        home,
        "ses-fork",
        "m3",
        1_780_000_050_000,
        r#"{"input":300,"output":100}"#,
    );
    opencode_message(
        home,
        "ses-fork",
        "m4",
        1_780_000_160_000,
        r#"{"input":40,"output":20}"#,
    );
    opencode_message(
        home,
        "ses-orphan",
        "m5",
        1_780_000_200_000,
        r#"{"input":70,"output":30}"#,
    );
    StorageFixture {
        storage: home.join(".local/share/opencode/storage"),
    }
}

fn session_by_stem<'a>(
    sessions: &'a [agenttrace_core::Session],
    stem: &str,
) -> &'a agenttrace_core::Session {
    sessions
        .iter()
        .find(|session| {
            session
                .path
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches(".json")
                == stem
        })
        .unwrap_or_else(|| panic!("session {stem} missing from {:?}", stems(sessions)))
}

fn stems(sessions: &[agenttrace_core::Session]) -> Vec<String> {
    let mut names: Vec<String> = sessions
        .iter()
        .map(|session| {
            session
                .path
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches(".json")
                .to_string()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn continuation_fork_counts_only_post_boundary_work() {
    let root = temp_root("agenttrace-rm805-continuation");
    let home = root.join("home");
    seed_boundary_corpus(&home);

    with_home(&home, || {
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});

        // The continuation fork is retained (not whole-dropped) and the
        // orphan is counted: parent + fork + orphan, nothing dropped.
        assert_eq!(
            stems(&report.sessions),
            vec!["ses-fork", "ses-orphan", "ses-parent"]
        );
        assert_eq!(
            report.opencode_fork_excluded, 0,
            "no whole-drop remains: the fork continues past its boundary"
        );

        // The fork's replayed prefix (m3, 300/100) is excluded at parse
        // time; only m4 (40/20) counts.
        let fork = session_by_stem(&report.sessions, "ses-fork");
        assert_eq!(fork.metrics.tokens_input, 40);
        assert_eq!(fork.metrics.tokens_output, 20);

        // The orphaned fork counts fully (70/30): its history lives
        // nowhere else.
        let orphan = session_by_stem(&report.sessions, "ses-orphan");
        assert_eq!(orphan.metrics.tokens_input, 70);
        assert_eq!(orphan.metrics.tokens_output, 30);

        // The parent is untouched by the fork's accounting.
        let parent = session_by_stem(&report.sessions, "ses-parent");
        assert_eq!(parent.metrics.tokens_input, 100);
        assert_eq!(parent.metrics.tokens_output, 50);

        // Everything that was excluded or kept-by-exception is disclosed
        // with its magnitude — never only a silent count.
        let health = data_health_scoped(
            &report.sessions,
            report.discovered,
            report.skipped,
            report.cache_hits,
            &report.opencode_forks,
        );
        assert_eq!(
            health
                .disclosures
                .get("opencode_fork_prefix_excluded_tokens"),
            Some(&400),
            "the replayed prefix (300 input + 100 output) must be disclosed"
        );
        assert!(
            health
                .disclosures
                .get("opencode_fork_prefix_excluded_cost_estimated_microusd")
                .copied()
                .unwrap_or(0)
                > 0,
            "the catalog-priced magnitude of the trimmed prefix must be disclosed"
        );
        assert_eq!(
            health.disclosures.get("opencode_fork_orphans_counted"),
            Some(&1),
            "the counted orphan is disclosed as an exception, not dropped"
        );
        assert_eq!(
            health
                .disclosures
                .get("opencode_fork_excluded_sessions")
                .copied()
                .unwrap_or(0),
            0
        );
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn boundary_aware_fork_bypasses_the_warm_cache_and_does_not_poison_it() {
    let root = temp_root("agenttrace-rm805-warm");
    let home = root.join("home");
    let fixture = seed_boundary_corpus(&home);

    with_home(&home, || {
        // Cold aggregation load — populates the session cache.
        let first = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        let fork_first = session_by_stem(&first.sessions, "ses-fork");
        assert_eq!(
            (
                fork_first.metrics.tokens_input,
                fork_first.metrics.tokens_output
            ),
            (40, 20)
        );

        // Warm aggregation load — the boundary fork must bypass the
        // cached entry (a full-fidelity parse would resurrect the
        // replayed prefix) and stay 40/20, not 340/120.
        let second = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        let fork_warm = session_by_stem(&second.sessions, "ses-fork");
        assert_eq!(
            (
                fork_warm.metrics.tokens_input,
                fork_warm.metrics.tokens_output
            ),
            (40, 20),
            "warm reload must not double-apply or undo the prefix exclusion"
        );

        // Explicit full-fidelity load after a warm filtered entry:
        // renders the fork whole (m3 + m4 = 340/120) and must not have
        // been poisoned by the filtered view the cache would otherwise
        // serve.
        let explicit = load_sessions_from_dir(Some(&fixture.storage));
        let fork_full = session_by_stem(&explicit, "ses-fork");
        assert_eq!(
            (
                fork_full.metrics.tokens_input,
                fork_full.metrics.tokens_output
            ),
            (340, 120),
            "explicit -d keeps full fidelity: replay + continuation"
        );
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn sqlite_orphaned_fork_rows_counted_with_magnitude_disclosed() {
    let root = temp_root("agenttrace-rm805-sqlite-orphans");
    let home = root.join("home");
    let db_path = home.join(".local/share/opencode/opencode.db");
    fs::create_dir_all(db_path.parent().expect("db parent")).expect("create db parent");
    let db = Connection::open(&db_path).expect("open opencode db");
    db.execute_batch(
        r#"
        create table session (
            id text primary key,
            title text,
            time_created integer,
            time_updated integer,
            cost real,
            tokens_input integer,
            tokens_output integer,
            tokens_reasoning integer,
            tokens_cache_read integer,
            tokens_cache_write integer,
            parent_id text
        );
        create table message (session_id text, data text);
        create table part (session_id text, data text);
        insert into session values
            ('ses_db_parent', 'Parent', 1764750000000, 1764750004000, 0.10, 500, 100, 0, 0, 0, null);
        insert into session values
            ('ses_db_fork', 'Fork of live parent', 1764750100000, 1764750104000, 0.25, 200, 80, 0, 0, 0, 'ses_db_parent');
        insert into session values
            ('ses_db_orphan', 'Orphaned fork', 1764750200000, 1764750204000, 0.40, 70, 30, 0, 0, 0, 'ses_db_gone');
        insert into session values
            ('ses_db_fork_nostored', 'Fork without stored usage', 1764750300000, 1764750304000, null, null, null, null, null, null, 'ses_db_parent');
        insert into message values
            ('ses_db_fork_nostored', '{"modelID":"claude-sonnet-4-5","tokens":{"input":50,"output":25,"reasoning":5,"cache":{"read":30,"write":10}}}');
        insert into message values
            ('ses_db_fork_nostored', '{"modelID":"claude-sonnet-4-5","tokens":{"input":10,"output":15,"reasoning":0,"cache":{"read":5,"write":5}}}');
        "#,
    )
    .expect("seed fork-boundary opencode db");
    drop(db);

    with_home(&home, || {
        let report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});

        // The live-parent fork row drops (rm-548); the orphan row counts.
        // Sqlite sessions carry the database as their path, so assert on
        // names, not path stems.
        let mut names: Vec<String> = report.sessions.iter().map(|s| s.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["Orphaned fork", "Parent"]);
        assert_eq!(
            report.opencode_fork_excluded, 2,
            "both live-parent forks drop (stored + no-stored-columns); the orphan counts"
        );

        // Orphan counts fully — stored totals win for it. Its session
        // rides the database path, so look it up by name.
        let orphan = report
            .sessions
            .iter()
            .find(|session| session.name == "Orphaned fork")
            .expect("orphaned fork session missing");
        assert_eq!(orphan.metrics.tokens_input, 70);
        assert_eq!(orphan.metrics.tokens_output, 30);

        let health = data_health_scoped(
            &report.sessions,
            report.discovered,
            report.skipped,
            report.cache_hits,
            &report.opencode_forks,
        );
        assert_eq!(
            health.disclosures.get("opencode_fork_excluded_sessions"),
            Some(&2),
            "both dropped fork rows are counted (rm-548 behavior): one with              stored usage columns, one without"
        );
        assert_eq!(
            health.disclosures.get("opencode_fork_excluded_tokens"),
            Some(&435),
            "dropped row magnitude: stored 200 input + 80 output, plus the              derived no-stored-columns fork (review-fix F2: 60 input + 45              output (30 output + 15 reasoning folded) + 35 cache read + 15              cache write from its message rows)"
        );
        assert!(
            health
                .disclosures
                .get("opencode_fork_excluded_cost_estimated_microusd")
                >= Some(&250_000),
            "stored fork cost 0.25 USD as micro-USD, plus the derived fork's              message-priced cost (review-fix F2 arm)"
        );
        assert_eq!(
            health.disclosures.get("opencode_fork_orphans_counted"),
            Some(&1),
            "the counted orphan row is disclosed"
        );

        // Review fix F5: the fork-boundary disclosure must survive the warm
        // path — reload (the first pass populated the session cache) and
        // require every fork disclosure key to report the identical value.
        // The snapshot-schema bumps (sqlite 9, dir-cache 34) guarantee a
        // pre-cycle snapshot cannot serve stale whole-drop totals here.
        let warm_report = load_sessions_with_progress(None, &LoadOptions::default(), |_| {});
        let warm_health = data_health_scoped(
            &warm_report.sessions,
            warm_report.discovered,
            warm_report.skipped,
            warm_report.cache_hits,
            &warm_report.opencode_forks,
        );
        let fork_keys = [
            "opencode_fork_excluded_sessions",
            "opencode_fork_excluded_tokens",
            "opencode_fork_excluded_cost_estimated_microusd",
            "opencode_fork_prefix_excluded_tokens",
            "opencode_fork_prefix_excluded_cost_estimated_microusd",
            "opencode_fork_orphans_counted",
        ];
        for key in fork_keys {
            assert_eq!(
                health.disclosures.get(key),
                warm_health.disclosures.get(key),
                "warm reload must disclose the same fork accounting for {key}"
            );
        }
    });

    let _ = fs::remove_dir_all(root);
}

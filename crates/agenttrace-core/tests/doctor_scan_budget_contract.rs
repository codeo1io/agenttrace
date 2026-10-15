//! rm-367 stages (b)/(c)/(d) + rm-1425 contract: the doctor scan is
//! BOUNDED and HONEST about truncation.
//!
//! Stage (b): a per-root wall-clock budget keeps `--doctor` from
//! stalling silently on a decade-scale HOME (the cycle-4 assess F1
//! red: rc124 at BOTH timeout 240 and 900 with 0 bytes of stdout).
//! When the budget truncates a root, the row reports the unscanned
//! files as `skipped_budget` — never as parse failures — and the
//! report carries a `scan_partial` disclosure ('scanned N of M roots
//! in Ts'), so the output stays valid and the truncation stays
//! visible instead of presenting partial counts as complete ones.
//!
//! Stage (d): a size-capped pass for oversized roots — once the
//! parsed bytes of a root cross the cap, the remaining files are
//! counted as `skipped_size` rather than parsed.
//!
//! Byte-shape guard (the rm-1425 CI-gate pairing): a scan that never
//! truncates must serialize WITHOUT any of the new keys, so the
//! `--demo`/clean-env gate outputs stay byte-identical.
//!
//! Every test pins HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR to a temp
//! home first (doctor_honesty_contract.rs `with_home` pattern), so no
//! fixture entry can reach the operator's real cache.

use agenttrace_core::build_doctor_report;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Serializes HOME/XDG/cache/budget-env pinning within this test
/// binary (doctor_honesty_contract.rs pattern; catch-unwind keeps a
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

fn with_home_and_budget(home: &Path, budget: &[(&str, &str)], f: impl FnOnce()) {
    let keys = [
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "AGENTTRACE_SESSION_CACHE_DIR",
        "AGENTTRACE_DOCTOR_ROOT_BUDGET_MS",
        "AGENTTRACE_DOCTOR_ROOT_MAX_BYTES",
        "AGENTTRACE_DOCTOR_DB_BUDGET_MS",
        "AGENTTRACE_DOCTOR_SQLITE_TOTAL_BUDGET_MS",
    ];
    let result = {
        let _guard = lock_env();
        let previous: Vec<_> = keys.iter().map(|k| (*k, std::env::var_os(k))).collect();
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        for (key, value) in budget {
            std::env::set_var(key, value);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        for (key, value) in previous {
            restore_env(key, value);
        }
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn temp_home(marker: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("at-rm367bcd-{marker}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).expect("create sandbox home");
    home
}

const SESSION: &str = concat!(
    "{\"role\":\"session_meta\",\"timestamp\":\"2026-05-01T09:00:00Z\",\"ModelUsed\":\"claude-sonnet-4-5\"}\n",
    "{\"role\":\"meta\",\"ModelUsed\":\"claude-sonnet-4-5\",\"Usage\":{\"input_tokens\":120,\"output_tokens\":30}}\n",
);

/// A claude-shaped custom directory with two parseable session files
/// in one project directory (doctor_honesty_contract.rs planted
/// corpus shape).
fn planted_two_file_corpus(marker: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "at-rm367bcd-corpus-{marker}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let project = root.join("projects").join("home-agent-budget");
    fs::create_dir_all(&project).expect("create project dir");
    fs::write(project.join("session-a.jsonl"), SESSION).expect("plant session a");
    fs::write(project.join("session-b.jsonl"), SESSION).expect("plant session b");
    root
}

#[test]
fn zero_root_budget_truncates_with_honest_partial_counts() {
    // rm-367(b) regression pin, verbatim intent: a budget-interrupted
    // run still reports honest partial counts — every file the budget
    // skipped shows up as `skipped_budget` (never as failed), the
    // per-file identity files == parsed + failed + cache_hits +
    // skipped_budget + skipped_size holds, and the top-level
    // `scan_partial` disclosure exists in the serialized report.
    let home = temp_home("budget0");
    let corpus = planted_two_file_corpus("budget0");
    with_home_and_budget(&home, &[("AGENTTRACE_DOCTOR_ROOT_BUDGET_MS", "0")], || {
        let report = build_doctor_report(Some(&corpus), false);
        let row = report
            .directories
            .iter()
            .find(|d| d.files >= 2)
            .expect("the planted project row is walked");
        assert_eq!(row.skipped_budget, 2, "zero budget must skip both files");
        assert_eq!(row.failed, 0, "budget skips must never count as failures");
        assert_eq!(row.parsed + row.failed + row.cache_hits + row.skipped_budget + row.skipped_size, row.files,
            "honest-count identity: every file is parsed, failed, cached, budget-skipped, or size-skipped");
        let partial = report
            .scan_partial
            .as_ref()
            .expect("scan_partial disclosure");
        assert_eq!(partial.files_skipped_budget, 2);
        assert_eq!(partial.roots_truncated, 1);
        assert!(
            partial.note.contains("budget"),
            "note names the bound: {partial:?}"
        );
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(
            json.contains("\"skipped_budget\":2"),
            "row carries the skip count in JSON"
        );
        assert!(
            json.contains("\"scan_partial\""),
            "JSON carries the disclosure"
        );
    });
}

#[test]
fn default_budget_parses_everything_and_adds_no_json_keys() {
    // Byte-shape guard (rm-1425 pairing): a scan that never truncates
    // must serialize WITHOUT the new keys, so clean-env/`--demo` gate
    // outputs stay byte-identical.
    let home = temp_home("default");
    let corpus = planted_two_file_corpus("default");
    with_home_and_budget(&home, &[], || {
        let report = build_doctor_report(Some(&corpus), false);
        let row = report
            .directories
            .iter()
            .find(|d| d.files >= 2)
            .expect("the planted project row is walked");
        assert_eq!(
            row.parsed + row.cache_hits,
            2,
            "both files parse under defaults"
        );
        assert_eq!(row.skipped_budget + row.skipped_size, 0);
        assert!(report.scan_partial.is_none());
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(!json.contains("skipped_budget"));
        assert!(!json.contains("skipped_size"));
        assert!(!json.contains("scan_partial"));
    });
}

#[test]
fn root_size_cap_keeps_first_file_and_discloses_the_rest() {
    // rm-367(d): a root whose parsed bytes cross the cap keeps the
    // files it parsed and counts the rest as `skipped_size` — a
    // count-sampled pass for oversized roots, disclosed honestly.
    let home = temp_home("sizecap");
    let corpus = planted_two_file_corpus("sizecap");
    with_home_and_budget(&home, &[("AGENTTRACE_DOCTOR_ROOT_MAX_BYTES", "1")], || {
        let report = build_doctor_report(Some(&corpus), false);
        let row = report
            .directories
            .iter()
            .find(|d| d.files >= 2)
            .expect("the planted project row is walked");
        assert_eq!(
            row.parsed + row.cache_hits,
            1,
            "the first file parses before the cap bites"
        );
        assert_eq!(row.skipped_size, 1, "the second file is size-skipped");
        assert_eq!(row.skipped_budget, 0);
        let partial = report
            .scan_partial
            .as_ref()
            .expect("scan_partial disclosure");
        assert_eq!(partial.files_skipped_size, 1);
    });
}

#[test]
fn zero_sqlite_budget_discloses_partial_db_and_never_snapshots() {
    // rm-367(b) sqlite lane: an exhausted database budget is disclosed
    // as a partial database entry (kept session count) and must never
    // persist a snapshot for the truncated pass — the rm-753
    // no-poison rule extended to budget truncation.
    let home = temp_home("sqlite0");
    let hermes = home.join(".hermes");
    fs::create_dir_all(&hermes).expect("create .hermes");
    fs::write(hermes.join("state.db"), b"garbage-not-a-real-db-but").expect("plant state.db");
    with_home_and_budget(
        &home,
        &[
            ("AGENTTRACE_DOCTOR_DB_BUDGET_MS", "0"),
            ("AGENTTRACE_DOCTOR_SQLITE_TOTAL_BUDGET_MS", "0"),
        ],
        || {
            let report = build_doctor_report(None, false);
            let partial = report
                .scan_partial
                .as_ref()
                .expect("scan_partial disclosure");
            assert!(
                partial.sqlite_dbs_partial >= 1,
                "the budget-exhausted db is disclosed"
            );
            assert!(!report.sqlite_partial_dbs.is_empty());
            let snapshot = home.join("cache").join("hermes-sqlite.json");
            assert!(
                !snapshot.exists(),
                "a budget-truncated pass must not persist a snapshot"
            );
        },
    );
}

#[test]
fn warm_cache_hits_stay_exempt_from_a_zero_budget() {
    // rm-367(b) warm-exemption pin: cache hits are checked BEFORE the
    // budget, so a rerun whose corpus is entirely warm counts every
    // file as parsed (cache_hits) with zero budget-skips — while a
    // file that missed the cache under the same zero budget is
    // disclosed as skipped_budget. Warm homes never lose their
    // doctor numbers to a spent budget.
    let home = temp_home("warm0");
    let corpus = planted_two_file_corpus("warm0");
    // Cold pass: default budgets parse both files and persist them
    // into the home-pinned cache (rm-367 stage (a)).
    with_home_and_budget(&home, &[], || {
        let report = build_doctor_report(Some(&corpus), false);
        let row = report
            .directories
            .iter()
            .find(|d| d.files >= 2)
            .expect("the planted project row is walked");
        assert_eq!(row.parsed, 2, "cold pass parses both files");
        assert_eq!(row.cache_hits, 0);
    });
    // Warm pass: ZERO budget on the same corpus — both files are
    // warm now and must still be counted, never budget-skipped.
    with_home_and_budget(&home, &[("AGENTTRACE_DOCTOR_ROOT_BUDGET_MS", "0")], || {
        let report = build_doctor_report(Some(&corpus), false);
        let row = report
            .directories
            .iter()
            .find(|d| d.files >= 2)
            .expect("the planted project row is walked");
        assert_eq!(row.cache_hits, 2, "warm hits are exempt from the budget");
        assert_eq!(row.parsed, 2, "cache hits count as parsed");
        assert_eq!(row.skipped_budget, 0);
    });
    // Cold newcomer under the same zero budget: disclosed as exactly
    // one budget-skip while the warm pair keeps its numbers.
    let project = corpus.join("projects").join("home-agent-budget");
    fs::write(project.join("session-c.jsonl"), SESSION).expect("plant session c");
    with_home_and_budget(&home, &[("AGENTTRACE_DOCTOR_ROOT_BUDGET_MS", "0")], || {
        let report = build_doctor_report(Some(&corpus), false);
        let row = report
            .directories
            .iter()
            .find(|d| d.files >= 3)
            .expect("the row now carries three files");
        assert_eq!(row.cache_hits, 2, "the warm pair still counts");
        assert_eq!(
            row.skipped_budget, 1,
            "the cold newcomer alone is budget-skipped"
        );
        assert_eq!(row.failed, 0);
    });
}

#[test]
fn demo_report_stays_fully_unbounded_and_keyless() {
    // The bundled demo corpus must never truncate and must serialize
    // without the new keys (check-output-contract.sh byte identity).
    let home = temp_home("demo");
    with_home_and_budget(&home, &[], || {
        let report = build_doctor_report(None, true);
        assert!(report.scan_partial.is_none());
        assert!(report.sqlite_partial_dbs.is_empty());
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(!json.contains("scan_partial"));
        assert!(!json.contains("skipped_budget"));
        assert!(!json.contains("skipped_size"));
    });
}

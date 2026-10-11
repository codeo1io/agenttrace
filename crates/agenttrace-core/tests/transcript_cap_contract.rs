//! rm-1003 contract: whole-file transcript reads are size-capped.
//!
//! Pre-fix, `parse_file` read any transcript into memory with an
//! unbounded `fs::read` — a multi-GB rollout silently stalled every
//! lane that touched it (reports, cache build, doctor). The cap is a
//! hard default of 2 GiB, overridable with `AGENTTRACE_MAX_TRANSCRIPT_BYTES`
//! (tests here pin it to 1 MiB so the fixtures stay small), and the
//! refusal is LOUD: the error names the file, its size, the active
//! cap, and the override knob, and the discovery/doctor lanes count it
//! as a budget skip — never as a parse failure.
//!
//! Per docs/quality-gates/byte-caps-must-test-the-eof-edge.md, the cap
//! carries BOTH boundary arms: over-cap WITH a terminator (mid-stream
//! refusal) and over-cap at EOF with no terminator at all.

use agenttrace_core::{
    build_doctor_report, load_sessions_with_options, parse_file, LoadOptions,
    DEFAULT_TRANSCRIPT_BYTE_CAP,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

const TEST_CAP: u64 = 1024 * 1024; // 1 MiB

/// Serializes AGENTTRACE_MAX_TRANSCRIPT_BYTES (and the HOME/XDG pins the
/// doctor arm needs) within this test binary — the
/// doctor_honesty_contract.rs `with_home` pattern; env mutation is
/// process-global, so every env-touching arm runs under this lock.
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

fn with_test_cap(f: impl FnOnce()) {
    let result = {
        let _guard = lock_env();
        let previous_cap = std::env::var_os("AGENTTRACE_MAX_TRANSCRIPT_BYTES");
        std::env::set_var("AGENTTRACE_MAX_TRANSCRIPT_BYTES", TEST_CAP.to_string());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore_env("AGENTTRACE_MAX_TRANSCRIPT_BYTES", previous_cap);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

/// Doctor arm variant: cap pin PLUS the home/cache pins so the report
/// scan never touches the operator's real cache (doctor persists fresh
/// parses, rm-367 stage (a)).
fn with_test_cap_and_home(home: &Path, f: impl FnOnce()) {
    let result = {
        let _guard = lock_env();
        let previous = |keys: &[&str]| keys.iter().map(std::env::var_os).collect::<Vec<_>>();
        let keys = [
            "AGENTTRACE_MAX_TRANSCRIPT_BYTES",
            "HOME",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "XDG_DATA_HOME",
            "AGENTTRACE_SESSION_CACHE_DIR",
        ];
        let saved = previous(&keys);
        std::env::set_var("AGENTTRACE_MAX_TRANSCRIPT_BYTES", TEST_CAP.to_string());
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        for (key, value) in keys.iter().zip(saved) {
            restore_env(key, value);
        }
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("agenttrace-cap-{}-{}", std::process::id(), tag));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// A valid two-line Go-style JSONL session (the discovery_contract
/// SAMPLE_JSONL / write_compare_session family — a shape the format
/// probes really claim, verified on the pre-fix tree) sized so the
/// WHOLE file is exactly `total` bytes, optional trailing newline
/// included in the count. Exact sizes matter at the cap boundary:
/// cap+1 must refuse, cap must parse.
fn sized_session(total: usize, terminator: bool) -> String {
    let head = r#"{"role":"user","content":""#;
    let tail = r#"","timestamp":"2026-05-02T10:00:00Z","ModelUsed":"gpt-4.1"}"#;
    let second = r#"{"role":"assistant","content":"done","timestamp":"2026-05-02T10:00:01Z","ModelUsed":"gpt-4.1"}"#;
    let overhead = head.len() + tail.len() + second.len() + 1 + usize::from(terminator);
    assert!(
        total > overhead,
        "fixture too small ({total}) for the two-line body ({overhead})"
    );
    let padding = total - overhead;
    let mut body = String::with_capacity(total);
    body.push_str(head);
    body.push_str(&"a".repeat(padding));
    body.push_str(tail);
    body.push('\n');
    body.push_str(second);
    if terminator {
        body.push('\n');
    }
    assert_eq!(body.len(), total);
    body
}

fn write_fixture(path: &Path, total: usize, terminator: bool) {
    fs::write(path, sized_session(total, terminator)).expect("write sized fixture");
}

#[test]
fn default_cap_is_two_gibibytes() {
    assert_eq!(DEFAULT_TRANSCRIPT_BYTE_CAP, 2 * 1024 * 1024 * 1024);
}

#[test]
fn over_cap_with_terminator_is_refused_loudly() {
    with_test_cap(|| {
        let dir = scratch_dir("mid");
        let path = dir.join("big.jsonl");
        write_fixture(&path, (TEST_CAP + 1) as usize, true);
        let error = parse_file(&path)
            .expect_err("cap+1 bytes must be refused under a 1 MiB cap")
            .to_string();
        assert!(
            error.contains("over the transcript cap"),
            "error must name the cap, got: {error}"
        );
        assert!(
            error.contains("big.jsonl"),
            "error must name the file, got: {error}"
        );
        assert!(
            error.contains("1.0 MiB"),
            "error must name the ACTIVE (env-overridden) cap size, got: {error}"
        );
        assert!(
            error.contains("AGENTTRACE_MAX_TRANSCRIPT_BYTES"),
            "error must name the override knob, got: {error}"
        );
        let _ = fs::remove_dir_all(&dir);
    });
}

#[test]
fn over_cap_at_eof_without_terminator_is_refused_not_read() {
    // byte-caps-must-test-the-eof-edge: the cap-crossing bytes are the
    // stream's final act, no terminator after them. The refusal must
    // fire on SIZE alone — content shape must not matter.
    with_test_cap(|| {
        let dir = scratch_dir("eof");
        let path = dir.join("eof.jsonl");
        write_fixture(&path, (TEST_CAP + 1) as usize, false);
        let error = parse_file(&path)
            .expect_err("cap+1 bytes at EOF must be refused under a 1 MiB cap")
            .to_string();
        assert!(error.contains("over the transcript cap"), "got: {error}");
        let _ = fs::remove_dir_all(&dir);
    });
}

#[test]
fn exactly_at_cap_still_parses() {
    with_test_cap(|| {
        let dir = scratch_dir("edge");
        let path = dir.join("exact.jsonl");
        write_fixture(&path, TEST_CAP as usize, true);
        let session = parse_file(&path).expect("exactly-at-cap must parse");
        assert!(
            session.name.starts_with('a'),
            "Go-style sessions name from the first user message, got: {}",
            session.name
        );
        let _ = fs::remove_dir_all(&dir);
    });
}

#[test]
fn discovery_counts_the_skip_apart_from_parse_failures() {
    with_test_cap(|| {
        let dir = scratch_dir("disc");
        write_fixture(&dir.join("good.jsonl"), 256, true);
        write_fixture(&dir.join("big.jsonl"), (TEST_CAP + 1) as usize, true);
        let report = load_sessions_with_options(Some(&dir), &LoadOptions::default());
        assert_eq!(report.sessions.len(), 1, "the healthy session still loads");
        assert_eq!(
            report.oversized_skips, 1,
            "the over-cap transcript is a budget skip"
        );
        assert_eq!(
            report.parse_failures, 0,
            "a cap skip must never be counted as corruption"
        );
        let first = report
            .first_oversized_skip
            .as_deref()
            .expect("first oversized skip disclosed");
        assert!(first.contains("big.jsonl"), "got: {first}");
        assert_eq!(
            report.oversized_paths,
            vec![dir.join("big.jsonl")],
            "skipped paths are listed for the -o membership check"
        );
        let _ = fs::remove_dir_all(&dir);
    });
}

#[test]
fn doctor_counts_oversized_transcripts_as_budget_skips() {
    let home = scratch_dir("doc-home");
    let corpus = home.join("corpus");
    fs::create_dir_all(&corpus).expect("create corpus dir");
    // Sized OUTSIDE the env lock: the fixtures only need bytes on disk.
    write_fixture(&corpus.join("good.jsonl"), 256, true);
    write_fixture(&corpus.join("big.jsonl"), (TEST_CAP + 1) as usize, true);
    with_test_cap_and_home(&home, || {
        let report = build_doctor_report(Some(&corpus), false);
        let entry = report
            .directories
            .iter()
            .find(|dir| dir.files >= 2)
            .expect("corpus directory scanned by the doctor");
        assert_eq!(entry.oversized, 1, "doctor surfaces the cap skip count");
        assert_eq!(
            entry.failed, 0,
            "a budget skip is not a parse failure in the doctor either"
        );
        assert!(
            entry
                .oversized_samples
                .iter()
                .any(|sample| sample.contains("big.jsonl")),
            "doctor names a skipped sample, got {:?}",
            entry.oversized_samples
        );
    });
    let _ = fs::remove_dir_all(&home);
}

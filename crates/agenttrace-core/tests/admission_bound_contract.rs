//! rm-451 admission-bound contract, pinned red-first.
//!
//! `parse_file` used to `std::fs::read` every discovered transcript
//! whole, so a single oversized rollout blew first-scan memory (the
//! PoC that minted the row measured a 454 MB file costing 4,142 MB
//! RSS, 9.1x, with the walk's parallel parse workers amplifying it).
//! The admission bound refuses over-bound files on `fs::metadata`
//! alone — before a single byte is read — so the first test below is
//! deliberately built on a SPARSE file: its size is visible to `stat`
//! at zero disk cost, and the pre-fix parser could only discover it
//! by reading the whole buffer first (red capture: "unsupported
//! session format" after a full 256 MiB read).
//!
//! The walk contract (mirrored by the doctor row): an over-bound file
//! is an ADMISSION exclusion, not a parse failure — skipped whole,
//! counted on `LoadReport.admission_skips`, named by
//! `first_admission_skip`, never in the rm-912 `-o` refusal set. The
//! loader, the doctor scan and the explicit positional lanes quote the
//! same stated constant (`DEFAULT_MAX_SESSION_FILE_BYTES`), and
//! `--max-session-file-bytes BYTES` re-binds the whole chain (0
//! disables the gate).

use agenttrace_core::{
    build_doctor_report, load_sessions_with_options, parse_file, parse_file_with_bound,
    render_doctor_report, LoadOptions, DEFAULT_MAX_SESSION_FILE_BYTES,
};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

const PLANTED_SESSION: &str = concat!(
    "{\"role\":\"session_meta\",\"timestamp\":\"2026-10-01T09:00:00Z\",\"ModelUsed\":\"claude-sonnet-4-5\"}\n",
    "{\"role\":\"meta\",\"ModelUsed\":\"claude-sonnet-4-5\",\"Usage\":{\"input_tokens\":120,\"output_tokens\":30}}\n",
);

/// subagents.rs:231-236 precedent — pid + tag + monotonic stamp so
/// concurrent runs (and concurrent sibling test binaries) never share
/// a scratch root.
fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "at-rm451-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within
/// this test binary (doctor_honesty_contract.rs pattern) so the
/// loader's session-cache writes land in a sandbox, never in the
/// operator's real cache.
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

/// A discoverable corpus: two parseable sessions plus one sparse file
/// over the default bound (stat-visible size, zero disk cost — the
/// gate must fire on metadata alone, and any arm that fails to gate
/// would pay the full read).
fn planted_corpus(tag: &str, oversize_bytes: u64) -> PathBuf {
    let root = temp_root(tag);
    let project = root.join("projects").join("home-agent-rm451");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("a.jsonl"), PLANTED_SESSION).unwrap();
    fs::write(project.join("b.jsonl"), PLANTED_SESSION).unwrap();
    File::create(project.join("oversize.jsonl"))
        .unwrap()
        .set_len(oversize_bytes)
        .unwrap();
    root
}

#[test]
fn over_bound_session_file_is_refused_before_reading() {
    let dir = temp_root("sparse-default");
    let big = dir.join("big.jsonl");
    File::create(&big)
        .unwrap()
        .set_len(256 * 1024 * 1024 + 1)
        .unwrap();
    let err = parse_file(&big).unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("admission bound"),
        "over-bound file must be refused at the admission gate before it is read, got: {msg}"
    );
    // Loud skip vocabulary: size, bound, and the flag that lifts it —
    // the same refusal must be self-explaining wherever it surfaces.
    assert!(
        msg.contains("over the per-file admission bound"),
        "names the bound: {msg}"
    );
    assert!(
        msg.contains("--max-session-file-bytes"),
        "names the escape flag: {msg}"
    );
    assert!(
        msg.contains(big.to_string_lossy().as_ref()),
        "names the file: {msg}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn zero_bound_disables_the_gate_and_bound_comparison_is_strict() {
    // A small junk file exercises the flag semantics without any big
    // read: at bound 4096 the 4097-byte file is refused at admission;
    // at 0 the gate is off and the reader sees the file (and reports
    // a PARSE failure, not an admission refusal); at 4097 the file is
    // exactly AT the bound — admitted, because the comparison is
    // strictly greater.
    let dir = temp_root("zero-bound");
    let junk = dir.join("junk.jsonl");
    fs::write(&junk, vec![0u8; 4097]).unwrap();

    let refused = parse_file_with_bound(&junk, 4096).unwrap_err();
    let msg = format!("{refused:#}");
    assert!(
        msg.contains("admission bound"),
        "small bound refuses: {msg}"
    );

    let read = parse_file_with_bound(&junk, 0).unwrap_err();
    let msg0 = format!("{read:#}");
    assert!(
        !msg0.contains("admission bound"),
        "bound 0 disables the gate (the reader sees the file): {msg0}"
    );

    let at_bound = parse_file_with_bound(&junk, 4097).unwrap_err();
    let msg_at = format!("{at_bound:#}");
    assert!(
        !msg_at.contains("admission bound"),
        "size == bound passes admission (strict comparison): {msg_at}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn normal_files_parse_identically_under_default_raised_and_disabled_bounds() {
    // Parity pin: the gate changes nothing for in-bound files.
    let dir = temp_root("parity");
    let good = dir.join("good.jsonl");
    fs::write(&good, PLANTED_SESSION).unwrap();
    let baseline = parse_file(&good).unwrap();
    for bound in [DEFAULT_MAX_SESSION_FILE_BYTES, u64::MAX, 0] {
        let session = parse_file_with_bound(&good, bound).unwrap();
        assert_eq!(
            serde_json::to_string(&session).unwrap(),
            serde_json::to_string(&baseline).unwrap(),
            "in-bound file parses identically at bound {bound}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn discovery_walk_skips_over_bound_files_loudly_as_admission_not_failure() {
    let home = temp_root("home-walk");
    with_home(&home, || {
        let corpus = planted_corpus("walk", DEFAULT_MAX_SESSION_FILE_BYTES + 1);
        let report = load_sessions_with_options(
            Some(&corpus),
            &LoadOptions {
                max_session_file_bytes: None,
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            report.discovered, 3,
            "the walk still sees the over-bound file"
        );
        assert_eq!(report.sessions.len(), 2, "the two good sessions load");
        assert_eq!(
            report.admission_skips, 1,
            "over-bound file counted as an admission skip"
        );
        assert_eq!(
            report.parse_failures, 0,
            "admission skip is NOT a parse failure (pre-fix it was neither: it read the file)"
        );
        assert!(
            report.parse_failure_paths.is_empty(),
            "rm-912 -o refusal set stays clean"
        );
        // Review F1 (2026-10-13): the skip paths are exposed on the
        // report so the CLI's -o no-clobber guard can cover the
        // never-read class without polluting the parse-failure set.
        assert_eq!(
            report.admission_skip_paths.len(),
            1,
            "exactly the over-bound path is listed"
        );
        assert!(
            report.admission_skip_paths[0].ends_with("oversize.jsonl"),
            "the listed skip path is the over-bound file: {:?}",
            report.admission_skip_paths[0]
        );
        let first = report.first_admission_skip.as_deref().unwrap_or_default();
        assert!(
            first.contains("oversize.jsonl") && first.contains("admission bound"),
            "first_admission_skip names the file and the bound: {first}"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn max_session_file_bytes_flag_rebinds_the_walk_and_zero_disables_it() {
    let home = temp_root("home-rebind");
    with_home(&home, || {
        // Small corpus + small bound: the semantics are size-relative,
        // so the flag arm needs no big files at all.
        let corpus = temp_root("rebind");
        let project = corpus.join("projects").join("home-agent-rm451");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("a.jsonl"), PLANTED_SESSION).unwrap();
        fs::write(project.join("junk.jsonl"), vec![0u8; 8192]).unwrap();

        let tight = load_sessions_with_options(
            Some(&corpus),
            &LoadOptions {
                max_session_file_bytes: Some(4096),
                ..LoadOptions::default()
            },
        );
        assert_eq!(tight.admission_skips, 1, "tight bound skips the 8 KiB file");
        assert_eq!(tight.parse_failures, 0);

        let exact = load_sessions_with_options(
            Some(&corpus),
            &LoadOptions {
                max_session_file_bytes: Some(8192),
                ..LoadOptions::default()
            },
        );
        assert_eq!(exact.admission_skips, 0, "at-bound file is admitted");
        assert_eq!(
            exact.parse_failures, 1,
            "the junk file then fails parsing honestly"
        );

        let off = load_sessions_with_options(
            Some(&corpus),
            &LoadOptions {
                max_session_file_bytes: Some(0),
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            off.admission_skips, 0,
            "0 disables the gate in the walk too"
        );
        assert_eq!(off.parse_failures, 1);
        let _ = fs::remove_dir_all(&corpus);
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn doctor_renders_admission_skips_as_own_row_and_informational_key() {
    let home = temp_root("home-doctor");
    with_home(&home, || {
        let corpus = planted_corpus("doctor", DEFAULT_MAX_SESSION_FILE_BYTES + 1);
        let report = build_doctor_report(Some(&corpus), false);
        assert_eq!(
            report.admission_skips, 1,
            "doctor counts the over-bound file"
        );
        assert!(
            report
                .admission_skip_samples
                .iter()
                .any(|s| s.contains("oversize.jsonl")),
            "samples name the file: {:?}",
            report.admission_skip_samples
        );
        let dir = report
            .directories
            .iter()
            .find(|d| d.admission_skips == 1)
            .expect("the provider dir row carries the skip");
        assert_eq!(dir.parsed, 2, "the two good sessions parsed");
        assert_eq!(
            dir.failed, 0,
            "admission skip is not counted as failed (pre-fix: files - parsed inflated it)"
        );
        assert!(dir.failure_samples.is_empty());
        assert_eq!(
            report.dropped_lines.get("admission_oversize_skipped"),
            None,
            "admission skip is not parse loss"
        );
        assert_eq!(
            report
                .disclosures
                .get("admission_oversize_skipped")
                .copied()
                .unwrap_or(0),
            1,
            "informational key carries the count (0-count-clean on healthy corpora)"
        );
        let text = render_doctor_report(Some(&corpus), false, "text").unwrap();
        assert!(
            text.contains("Admission-bound skips: 1 file(s)"),
            "own render row: first lines\n{}",
            text.lines().take(40).collect::<Vec<_>>().join("\n")
        );
        assert!(
            text.contains("admission_oversize_skipped=1"),
            "informational tier render: {text}"
        );
        // 3 files, 0 valid cache hits, 1 admission skip: the scan paid
        // 2 parses, not 3 (pre-fix reparsed_this_scan counted the skip).
        assert_eq!(report.session_files, 3);
        assert_eq!(
            report.reparsed_this_scan, 2,
            "admission skips are not re-parses"
        );
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn healthy_corpus_reports_no_admission_keys_or_rows() {
    // 0-count-clean: the row, the key, and the fields must not appear
    // for an all-in-bound corpus — healthy reads are unchanged.
    let home = temp_root("home-clean");
    with_home(&home, || {
        let corpus = temp_root("clean");
        let project = corpus.join("projects").join("home-agent-rm451");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("a.jsonl"), PLANTED_SESSION).unwrap();
        let report = load_sessions_with_options(Some(&corpus), &LoadOptions::default());
        assert_eq!(report.admission_skips, 0);
        assert!(report.first_admission_skip.is_none());
        let doc = build_doctor_report(Some(&corpus), false);
        assert_eq!(doc.admission_skips, 0);
        assert!(doc.admission_skip_samples.is_empty());
        assert!(!doc.disclosures.contains_key("admission_oversize_skipped"));
        let text = render_doctor_report(Some(&corpus), false, "text").unwrap();
        assert!(
            !text.contains("Admission-bound skips"),
            "no row on healthy corpora"
        );
        let _ = fs::remove_dir_all(&corpus);
    });
    let _ = fs::remove_dir_all(&home);
}

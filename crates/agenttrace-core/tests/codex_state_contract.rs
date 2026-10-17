//! Contract tests for the codex state ingest lane (rm-1986, run 2b30a694
//! cycle 3): the `tokens_used` disagreement oracle, versioned-glob discovery,
//! WAL-tolerant strictly-read-only opens, and disclose-never-drop failures.
//!
//! Fixtures are BUILT at runtime with rusqlite (same crate as the main
//! dependency) — no host DB bytes are ever committed, per the stewardship
//! must_remain_separate contract.
//!
//! Red-first proof: before the lane existed, none of these symbols existed —
//! the contract reds at compile time against a pre-rm-1986 tree, and the
//! fabricated-disagreement fixture keeps the oracle itself honest (it must
//! fire on a real mismatch, stay quiet on an in-band pair).

use agenttrace_core::{doctor_report_text, CodexRolloutTotal, CodexStateScan, DoctorReport};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

fn md5_of(path: &Path) -> u128 {
    // A cheap-but-real content fingerprint: len + fnv-1a over the bytes.
    let bytes = fs::read(path).expect("read fixture");
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in &bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    ((bytes.len() as u128) << 64) | hash as u128
}

struct TempHome {
    root: PathBuf,
}

impl TempHome {
    fn new(tag: &str) -> TempHome {
        let root = std::env::temp_dir().join(format!(
            "at-codex-state-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp home");
        TempHome { root }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Build a `state_N.sqlite` with the codex `threads`/`thread_spawn_edges`
/// schema observed on the 2026-10-18 host corpus (codex 0.147.0).
fn build_state_db(
    path: &Path,
    threads: &[(i64, &str, i64, &str)], // (id, rollout_path, tokens_used, source)
    spawn_edges: &[(i64, i64)],
) {
    let conn = Connection::open(path).expect("create fixture db");
    conn.execute_batch(
        "CREATE TABLE threads (
            id INTEGER PRIMARY KEY,
            rollout_path TEXT,
            tokens_used INTEGER,
            source TEXT,
            title TEXT,
            archived INTEGER,
            model_provider TEXT,
            cwd TEXT
        );
        CREATE TABLE thread_spawn_edges (
            parent_thread_id INTEGER,
            child_thread_id INTEGER
        );",
    )
    .expect("fixture schema");
    for (id, rollout_path, tokens_used, source) in threads {
        conn.execute(
            "INSERT INTO threads (id, rollout_path, tokens_used, source, title, archived, model_provider, cwd)
             VALUES (?1, ?2, ?3, ?4, 'fixture', 0, 'openai', '/tmp')",
            rusqlite::params![id, rollout_path, tokens_used, source],
        )
        .expect("fixture thread row");
    }
    for (parent, child) in spawn_edges {
        conn.execute(
            "INSERT INTO thread_spawn_edges (parent_thread_id, child_thread_id) VALUES (?1, ?2)",
            rusqlite::params![parent, child],
        )
        .expect("fixture edge row");
    }
}

fn rollout(path: &str, derived: i64) -> CodexRolloutTotal {
    CodexRolloutTotal {
        path: PathBuf::from(path),
        derived_tokens: derived,
    }
}

#[test]
fn discovers_versioned_state_files_and_ignores_sidecars_and_foreign_dbs() {
    let home = TempHome::new("discover");
    build_state_db(&home.root.join("state_5.sqlite"), &[], &[]);
    build_state_db(&home.root.join("state_6.sqlite"), &[], &[]);
    build_state_db(&home.root.join("thread_history_1.sqlite"), &[], &[]);
    build_state_db(&home.root.join("unrelated.sqlite"), &[], &[]);
    fs::write(home.root.join("state_5.sqlite-wal"), b"sidecar").unwrap();
    fs::write(home.root.join("state_5.sqlite-shm"), b"sidecar").unwrap();
    fs::write(home.root.join("state.txt"), b"not a db").unwrap();

    let files = agenttrace_core::codex_state::discover_state_files(&home.root);
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        vec!["state_5.sqlite".to_string(), "state_6.sqlite".to_string()],
        "only versioned state_*.sqlite at the home root counts; sidecars, projections and foreign dbs are excluded"
    );
}

#[test]
fn empty_home_is_absent_and_zero_count_clean() {
    let home = TempHome::new("absent");
    let scan = agenttrace_core::codex_state::scan_codex_state(&home.root, &[]);
    assert!(scan.is_absent());
    assert_eq!(scan.files_found, 0);
    let json = serde_json::to_value(&scan).unwrap();
    assert_eq!(
        json.get("files_found"),
        Some(&serde_json::json!(0)),
        "absent scan still discloses the probe count"
    );
}

#[test]
fn oracle_flags_fabricated_disagreement_and_stays_quiet_in_band() {
    let home = TempHome::new("oracle");
    let disagreeing = home.root.join("rollout-a.jsonl");
    fs::write(&disagreeing, b"{}").unwrap();
    let agreeing = home.root.join("rollout-b.jsonl");
    fs::write(&agreeing, b"{}").unwrap();
    let subagent = home.root.join("rollout-c.jsonl");
    fs::write(&subagent, b"{}").unwrap();

    build_state_db(
        &home.root.join("state_5.sqlite"),
        &[
            (1, disagreeing.to_str().unwrap(), 5000, "primary"),
            (2, agreeing.to_str().unwrap(), 1010, "primary"), // derived 1000 -> in band
            (3, subagent.to_str().unwrap(), 400, "subagent:review"), // derived 400 -> agree
        ],
        &[(1, 3)],
    );

    let rollouts = vec![
        rollout(disagreeing.to_str().unwrap(), 1200), // 5000 vs 1200 -> DISAGREE
        rollout(agreeing.to_str().unwrap(), 1000),    // 1010 vs 1000 -> in band
        rollout(subagent.to_str().unwrap(), 400),     // exact
    ];
    let scan = agenttrace_core::codex_state::scan_codex_state(&home.root, &rollouts);

    assert_eq!(scan.files_found, 1);
    assert_eq!(scan.files_parsed, 1);
    assert_eq!(scan.files_failed, 0);
    assert_eq!(scan.threads_total, 3);
    assert_eq!(scan.threads_matched, 3);
    assert_eq!(
        scan.tokens_used_disagreements, 1,
        "exactly the fabricated mismatch fires"
    );
    assert_eq!(scan.subagent_threads, 1, "subagent:review source counted");
    assert_eq!(scan.spawn_edges_total, 1, "spawn edges counted");
    assert_eq!(scan.disagreement_samples.len(), 1);
    let sample = &scan.disagreement_samples[0];
    assert!(
        sample.contains("rollout-a.jsonl"),
        "sample names the rollout: {sample}"
    );
    assert!(
        sample.contains("sqlite tokens_used=5000") && sample.contains("derived=1200"),
        "sample discloses both authorities: {sample}"
    );
}

#[test]
fn matches_by_file_name_when_absolute_prefix_differs() {
    let home = TempHome::new("byname");
    // Codex recorded an absolute path under a different mount prefix.
    build_state_db(
        &home.root.join("state_5.sqlite"),
        &[(
            7,
            "/other-mount/home/a/.codex/sessions/x/rollout-same.jsonl",
            900,
            "primary",
        )],
        &[],
    );
    let rollouts = vec![rollout(
        "/current/home/a/.codex/sessions/x/rollout-same.jsonl",
        880, // in band vs 900
    )];
    let scan = agenttrace_core::codex_state::scan_codex_state(&home.root, &rollouts);
    assert_eq!(scan.threads_matched, 1, "file-name fallback matches");
    assert_eq!(scan.tokens_used_disagreements, 0);
}

#[test]
fn present_but_unreadable_files_are_disclosed_never_dropped() {
    let home = TempHome::new("unreadable");
    build_state_db(
        &home.root.join("state_5.sqlite"),
        &[(1, "/tmp/r.jsonl", 10, "primary")],
        &[],
    );
    fs::write(
        home.root.join("state_7.sqlite"),
        b"this is not a sqlite database at all",
    )
    .unwrap();

    let scan = agenttrace_state_scan(&home.root);
    assert_eq!(scan.files_found, 2);
    assert_eq!(scan.files_parsed, 1);
    assert_eq!(scan.files_failed, 1, "garbage file disclosed as failed");
    assert!(scan
        .failure_samples
        .iter()
        .any(|s| s.contains("state_7.sqlite")));
    assert_eq!(scan.threads_total, 1, "healthy file still ingested");
}

fn agenttrace_state_scan(root: &Path) -> CodexStateScan {
    agenttrace_core::codex_state::scan_codex_state(root, &[])
}

#[test]
fn read_only_scan_never_mutates_db_or_sidecars() {
    let home = TempHome::new("bytes");
    let db = home.root.join("state_5.sqlite");
    build_state_db(&db, &[(1, "/tmp/r.jsonl", 1234, "primary")], &[(1, 2)]);
    // Simulate hot sidecars: present, non-empty, must stay byte-identical.
    let wal = home.root.join("state_5.sqlite-wal");
    let shm = home.root.join("state_5.sqlite-shm");
    fs::write(&wal, vec![0u8; 4096]).unwrap();
    fs::write(&shm, vec![0u8; 4096]).unwrap();

    let before = (md5_of(&db), md5_of(&wal), md5_of(&shm));
    let scan = agenttrace_core::codex_state::scan_codex_state(&home.root, &[]);
    let after = (md5_of(&db), md5_of(&wal), md5_of(&shm));

    assert_eq!(
        before, after,
        "strictly read-only: db, -wal and -shm are byte-identical after the scan"
    );
    assert!(
        scan.files_parsed >= 1,
        "scan still succeeds with sidecars present"
    );
}

#[test]
fn doctor_report_text_renders_the_cross_check_section_and_samples() {
    let home = TempHome::new("render");
    let mismatched = home.root.join("rollout-z.jsonl");
    fs::write(&mismatched, b"{}").unwrap();
    build_state_db(
        &home.root.join("state_5.sqlite"),
        &[(1, mismatched.to_str().unwrap(), 9000, "primary")],
        &[],
    );
    let scan = agenttrace_core::codex_state::scan_codex_state(
        &home.root,
        &[rollout(mismatched.to_str().unwrap(), 1000)],
    );
    let report = DoctorReport {
        codex_state: scan,
        ..Default::default()
    };
    let text = doctor_report_text(&report);
    assert!(
        text.contains("Codex state cross-check (rm-1986)"),
        "section header renders"
    );
    assert!(
        text.contains("tokens_used DISAGREEMENTS: 1"),
        "disagreement counter renders: {}",
        text.lines()
            .find(|l| l.contains("DISAGREEMENTS"))
            .unwrap_or("")
    );
    assert!(text.contains("rollout-z.jsonl"), "sample renders");
    assert!(
        text.contains("tokens_input + tokens_output + tokens_reasoning"),
        "the compared formula is disclosed in the output"
    );
}

#[test]
fn absent_scan_renders_nothing() {
    let report = DoctorReport::default();
    let text = doctor_report_text(&report);
    assert!(
        !text.contains("Codex state cross-check"),
        "zero-count-clean: no codex on host -> no section"
    );
}

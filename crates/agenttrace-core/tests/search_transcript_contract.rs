//! rm-1415 contract (run 37101c74 cycle 4): `--search` is documented —
//! README.md and `--help` — as "Full-text search across discovered session
//! transcripts", but `search_session_evidence` scanned session metadata
//! only; body-only queries silently returned `count: 0` (live red:
//! /tmp/at-prioritize-ae20fc51/plain/s2.jsonl, query "zygomorphic" present
//! only in messages 2+3 → count:0 rc0).
//!
//! The contract below pins the restored behavior against in-repo fixtures:
//! body-only queries must HIT, bounded and disclosed (byte cap per file +
//! corpus read budget, every skip counted in the scan summary), while the
//! pre-existing lanes stay verbatim — metadata matches, name-derived
//! matching, and the rm-988 blank-query guard.

use agenttrace_core::parse_file;
use agenttrace_core::{
    search_sessions, search_sessions_detailed, TRANSCRIPT_SCAN_FILE_BYTE_CAP,
    TRANSCRIPT_SCAN_TOTAL_BYTE_BUDGET,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/search-transcript")
        .join(name)
}

#[test]
fn body_only_query_hits_transcript_content() {
    let session = parse_file(&fixture("body-only.jsonl")).expect("fixture parses");
    // The name derives from the FIRST user message; the query word appears
    // only in later messages, so no metadata field can carry this hit.
    let query = "zygomorphic";
    assert!(
        !session.name.to_lowercase().contains(query),
        "fixture hygiene: name {:?} must not contain the query, or the pin below is vacuous",
        session.name
    );
    let results = agenttrace_core::search_sessions(&[session], query, 20);
    assert_eq!(
        results.len(),
        1,
        "body-only transcript query must hit (rm-1415 red-first)"
    );
    assert!(
        results[0]
            .matches
            .iter()
            .any(|m| m.starts_with("transcript: ")),
        "evidence must cite the transcript content, got {:?}",
        results[0].matches
    );
    assert!(
        results[0].transcript_scan.starts_with("scanned ("),
        "the row must disclose its transcript scan, got {:?}",
        results[0].transcript_scan
    );
}

#[test]
fn metadata_lanes_stay_verbatim_and_detailed_reports_the_scan() {
    let session = parse_file(&fixture("body-only.jsonl")).expect("fixture parses");
    // Name-derived match (first user message) — the pre-1415 lane.
    let results = search_sessions(std::slice::from_ref(&session), "innocuous", 20);
    assert_eq!(results.len(), 1);
    assert!(results[0].matches.iter().any(|m| m.starts_with("name: ")));

    // Detailed lane: same hits plus the corpus-level disclosure.
    let report = search_sessions_detailed(
        &[session],
        "zygomorphic",
        20,
        TRANSCRIPT_SCAN_FILE_BYTE_CAP,
        TRANSCRIPT_SCAN_TOTAL_BYTE_BUDGET,
    );
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.scan.files_considered, 1);
    assert_eq!(report.scan.scanned_files, 1);
    assert_eq!(report.scan.scanned_bytes, 431);
    assert_eq!(report.scan.skipped_missing, 0);
    assert_eq!(report.scan.skipped_oversized, 0);
    let json = agenttrace_core::report_search_json_detailed(&report);
    // Parse the detailed lane: it is hand-rolled, and a missing comma
    // between `matches` and `transcript_scan` once shipped green under
    // `contains`-only pins (live probe caught it) — never again.
    let parsed: serde_json::Value = serde_json::from_str(&json)
        .unwrap_or_else(|err| panic!("detailed search JSON must parse: {err}\n{json}"));
    assert_eq!(parsed["count"], serde_json::json!(1));
    assert_eq!(
        parsed["results"][0]["transcript_scan"],
        serde_json::json!("scanned (431 bytes)")
    );
    assert_eq!(
        parsed["transcript_scan"]["skipped_oversized"],
        serde_json::json!(0)
    );
    assert_eq!(
        parsed["transcript_scan"]["file_byte_cap"],
        serde_json::json!(8388608)
    );
    let text = agenttrace_core::report_search_text_detailed(&report, "zygomorphic");
    assert!(text.contains("Transcript scan: 1 file(s) scanned (431 bytes)"));
    assert!(text.contains("bytes/file, 67108864 bytes total"));
    // Stable library lane stays byte-stable: no transcript_scan key.
    let stable = agenttrace_core::report_search_json(&report.results);
    assert!(!stable.contains("transcript_scan"));
}

#[test]
fn oversized_files_are_skipped_and_disclosed() {
    let session = parse_file(&fixture("body-only.jsonl")).expect("fixture parses");
    // Tiny cap: the 431-byte fixture is "oversized" — skipped, disclosed at
    // corpus level, and a body-only query legitimately returns zero rows.
    let report = search_sessions_detailed(&[session], "zygomorphic", 20, 16, 1 << 20);
    assert_eq!(report.results.len(), 0);
    assert_eq!(report.scan.files_considered, 1);
    assert_eq!(report.scan.skipped_oversized, 1);
    assert_eq!(report.scan.scanned_files, 0);
    let json = agenttrace_core::report_search_json_detailed(&report);
    assert!(json.contains("\"skipped_oversized\": 1"));
    let text = agenttrace_core::report_search_text_detailed(&report, "zygomorphic");
    assert!(text.contains("1 skipped: over byte cap"));
}

#[test]
fn budget_exhaustion_skips_remaining_files_and_discloses() {
    let first = parse_file(&fixture("body-only.jsonl")).expect("fixture parses");
    let second = parse_file(&fixture("body-only-2.jsonl")).expect("fixture parses");
    // Budget covers exactly the first file: the second is skipped as
    // budget-exhausted. Sessions are scanned in canonical (newest-session
    // first) order — both fixtures parse to 2026-10-13 timestamps; assert
    // against whichever file won so the pin holds under either order.
    let first_len = std::fs::metadata(fixture("body-only.jsonl")).unwrap().len();
    let second_len = std::fs::metadata(fixture("body-only-2.jsonl"))
        .unwrap()
        .len();
    let report = search_sessions_detailed(
        &[first, second],
        "zygomorphic",
        20,
        TRANSCRIPT_SCAN_FILE_BYTE_CAP,
        first_len.max(second_len),
    );
    assert_eq!(report.scan.files_considered, 2);
    assert_eq!(report.scan.scanned_files, 1);
    assert_eq!(report.scan.skipped_budget, 1);
    assert!(report.results.len() <= 1);
    let text = agenttrace_core::report_search_text_detailed(&report, "zygomorphic");
    assert!(text.contains("1 skipped: read budget exhausted"));
}

#[test]
fn missing_and_non_file_paths_disclose_not_panic() {
    let session = parse_file(&fixture("body-only.jsonl")).expect("fixture parses");
    let mut gone = session.clone();
    gone.path = "/nonexistent-rm-1415/no-such-session.jsonl".to_string();
    // Metadata query still matches (name); the transcript lane discloses.
    let report = search_sessions_detailed(&[gone], "innocuous", 20, 1 << 20, 1 << 20);
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].transcript_scan, "skipped: file missing");
    assert_eq!(report.scan.skipped_missing, 1);

    // demo:// corpus paths are non-file: skipped without touching the fs.
    let mut demo = session;
    demo.path = "demo://01-healthy-ship.jsonl".to_string();
    let report = search_sessions_detailed(&[demo], "innocuous", 20, 1 << 20, 1 << 20);
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].transcript_scan, "skipped: non-file path");
    assert_eq!(report.scan.skipped_non_file, 1);
}

#[test]
fn blank_queries_still_match_nothing_in_the_detailed_lane() {
    // rm-988 semantics extend to the detailed lane verbatim: blank queries
    // match nothing AND touch no filesystem path (zero files considered).
    let session = parse_file(&fixture("body-only.jsonl")).expect("fixture parses");
    for blank in ["", "   ", "\t"] {
        let report = search_sessions_detailed(
            std::slice::from_ref(&session),
            blank,
            20,
            TRANSCRIPT_SCAN_FILE_BYTE_CAP,
            TRANSCRIPT_SCAN_TOTAL_BYTE_BUDGET,
        );
        assert!(
            report.results.is_empty(),
            "blank {blank:?} must match nothing"
        );
        assert_eq!(report.scan.files_considered, 0, "blank must not scan");
    }
    assert_eq!(search_sessions(&[session], "   ", 20).len(), 0);
}

// ============================================================
// rm-992 / rm-1872: zstd-compressed Codex rollouts
// Codex compresses rollout files either in place (the `.jsonl` name
// is kept, content starts with the zstd magic) or under a
// `.jsonl.zst` name (ccusage's codex reader admits both names and
// retries the sibling on ENOENT, because codex flips the suffix when
// it compresses cold rollouts and back when it restores them).
// ============================================================

fn codex_compressed_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex-zstd/rollout.jsonl")
}

#[test]
fn compressed_transcript_is_decoded_and_matchable() {
    // Golden-parity twin of fixtures/codex-custom-tools/rollout.jsonl:
    // 2,833 zstd bytes on disk, 22,652 bytes decompressed, and the
    // needle `turn_context` appears exactly once in the decompressed
    // body. A raw-byte scan over the compressed file finds nothing
    // while charging the compressed size to the scan budget - that
    // silent miss is the falsified claim this test pins away.
    let session = parse_file(&codex_compressed_fixture())
        .expect("the decode lane already parses this fixture");
    let report = search_sessions_detailed(
        &[session],
        "turn_context",
        20,
        TRANSCRIPT_SCAN_FILE_BYTE_CAP,
        1 << 20,
    );
    assert!(
        report
            .results
            .iter()
            .any(|r| r.transcript_scan == "scanned (22652 bytes)"
                && r.matches.iter().any(|m| m.contains("turn_context"))),
        "scan must decode the zstd frame, charge decompressed bytes, and match: {:?}",
        report.results
    );
}

#[test]
fn refused_zstd_content_is_skipped_loudly() {
    // Content that claims the zstd magic but is not a decodable frame
    // must be skipped with an explicit disclosure - never scanned as
    // raw bytes with a `scanned:` verdict that hides the refusal.
    let garbage = b"\x28\xb5\x2f\xfd\x00\x00\x00not-a-zstd-frame";
    // pid-unique home: sibling delegate sessions share a TMPDIR
    // (workflow-issues/sibling-delegate-sessions-share-tmpdir.md),
    // and this test writes a fixed-content file under a fixed name.
    let dir = std::env::temp_dir().join(format!("agenttrace-zst-refused-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let garbage_path = dir.join("rollout-2026-09-26T20-29-31-00000000.jsonl");
    std::fs::write(&garbage_path, garbage).unwrap();
    let mut session = parse_file(&fixture("body-only.jsonl")).unwrap();
    session.path = garbage_path.to_string_lossy().to_string();
    // The query matches the session NAME so the row survives into
    // results - a vacuously-true `all()` over zero rows would pin
    // nothing about the refusal.
    session.name = "innocuous-session".to_string();
    let report = search_sessions_detailed(
        &[session],
        "innocuous",
        20,
        TRANSCRIPT_SCAN_FILE_BYTE_CAP,
        1 << 20,
    );
    assert_eq!(report.results.len(), 1, "the name match must keep the row");
    assert!(
        report
            .results
            .iter()
            .all(|r| r.transcript_scan.starts_with("skipped: zstd")),
        "a refused frame must disclose the refusal, got {:?}",
        report
            .results
            .iter()
            .map(|r| r.transcript_scan.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        report.scan.skipped_zstd, 1,
        "the refusal must be counted in the summary, not just per row"
    );
    let text = agenttrace_core::report_search_text_detailed(&report, "innocuous");
    assert!(
        text.contains("zstd decode refused"),
        "the text summary must name the refusal: {text}"
    );
    let full_json = agenttrace_core::report_search_json_detailed(&report);
    assert!(
        full_json.contains("\"skipped_zstd\": 1"),
        "the json summary must carry the refusal: {full_json}"
    );
}

#[test]
fn decompressed_body_over_the_per_file_cap_is_skipped_with_a_dedicated_verdict() {
    // rm-2349 (run e90edd6c cycle 1, assess 855e1234 F4): the plain-file
    // arm checks the on-disk length against the per-file byte cap BEFORE
    // reading (:551), but the zstd arm charged only the shared total
    // budget — a compressed body may be under the cap on disk yet expand
    // past it decompressed (a 2,833-byte frame holds 22,652 bytes), so a
    // single file could consume the ENTIRE corpus budget through one
    // decode. Parity: the per-file cap binds in DEcompressed bytes for
    // zstd files exactly as it binds in on-disk bytes for plain files,
    // skipped with a dedicated verdict + counter, and the budget
    // arithmetic stays charged in decompressed bytes (rm-1872) — the
    // skip happens BEFORE any budget charge.
    //
    // The compressed twin reuses fixtures/codex-zstd/rollout.jsonl (2,833
    // zstd bytes on disk, 22,652 decompressed) under a 4,000-byte per-file
    // cap: it passes the on-disk guard, decodes, and must be skipped
    // past the cap. The plain twin is an incompressible 22,652-byte body
    // skipped by the pre-existing on-disk arm. Both files sit in ONE
    // corpus so the summary proves the parity side by side.
    let cap = 4_000u64;
    let mut compressed = parse_file(&codex_compressed_fixture()).unwrap();
    compressed.name = "a-compressed-over-cap".to_string();
    compressed.path = codex_compressed_fixture().to_string_lossy().to_string();
    // Plain twin: "p\n" repeated keeps every read path honest; the body
    // byte-matches the compressed twin's decompressed size.
    let dir = std::env::temp_dir().join(format!("agenttrace-zst-cap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let plain_body = "p\n".repeat(11_326); // 22,652 bytes
    assert_eq!(plain_body.len(), 22_652);
    let plain_path = dir.join("plain-over-cap.jsonl");
    std::fs::write(&plain_path, plain_body.as_bytes()).unwrap();
    let mut plain = parse_file(&fixture("body-only.jsonl")).unwrap();
    plain.name = "b-plain-over-cap".to_string();
    plain.path = plain_path.to_string_lossy().to_string();
    let report = search_sessions_detailed(
        &[compressed, plain],
        "zygomorphic-nonmatching-needle",
        20,
        cap,
        1 << 20,
    );
    assert!(report.results.is_empty(), "the query matches nothing");
    assert_eq!(report.scan.files_considered, 2);
    assert_eq!(report.scan.scanned_files, 0, "neither file is scanned");
    assert_eq!(
        report.scan.scanned_bytes, 0,
        "no budget is charged for a skipped file"
    );
    assert_eq!(
        report.scan.skipped_oversized, 1,
        "the plain twin is skipped by the pre-existing on-disk arm"
    );
    assert_eq!(
        report.scan.skipped_decompressed_oversized, 1,
        "the compressed twin is skipped by the new decompressed-body arm"
    );
    assert_eq!(
        report.scan.skipped_budget, 0,
        "no budget exhaustion is involved"
    );
    assert_eq!(
        report.scan.skipped_zstd, 0,
        "the frame decodes fine; only its size refuses it"
    );
    let text = agenttrace_core::report_search_text_detailed(&report, "needle");
    assert!(
        text.contains("decompressed body over per-file cap"),
        "the text summary must name the new skip class: {text}"
    );
    let full_json = agenttrace_core::report_search_json_detailed(&report);
    assert!(
        full_json.contains("\"skipped_decompressed_oversized\": 1"),
        "the json summary must carry the new counter: {full_json}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scan_charge_saturates_when_a_file_reads_past_its_stat_length() {
    // rm-1872 seam (rm-1993 cession): the budget pre-check runs on the
    // stat length; the charge uses the ACTUAL read. /proc/self/status is
    // the deterministic shape of a growth race - stat length 0, read
    // returns ~1.3 KB. With a zero total budget the charge must
    // SATURATE at zero (never wrap the budget backward to ~u64::MAX),
    // so the next file is disclosed as budget-skipped instead of being
    // scanned for free on a wrapped budget.
    let proc = std::path::Path::new("/proc/self/status");
    let stat_len = std::fs::metadata(proc).map(|m| m.len()).unwrap_or(u64::MAX);
    let read_len = std::fs::read(proc).map(|b| b.len() as u64).unwrap_or(0);
    if stat_len != 0 || read_len == 0 {
        // Environment without procfs zero-length semantics: the seam pin
        // would prove nothing here.
        eprintln!("skipping: /proc/self/status stat={stat_len} read={read_len}");
        return;
    }
    let mut proc_session = parse_file(&fixture("body-only.jsonl")).unwrap();
    proc_session.name = "a-proc-shaped-file".to_string();
    proc_session.path = "/proc/self/status".to_string();
    let mut real_session = parse_file(&fixture("body-only.jsonl")).unwrap();
    real_session.name = "b-real-file".to_string();
    // canonical_sessions orders same-instant parses by name, so the
    // stat-0 proc file is scanned FIRST and the real file second.
    let report = search_sessions_detailed(
        &[proc_session, real_session],
        "zygobabbington-nonmatching-needle",
        20,
        TRANSCRIPT_SCAN_FILE_BYTE_CAP,
        0,
    );
    assert!(report.results.is_empty(), "the query matches nothing");
    assert_eq!(report.scan.files_considered, 2);
    assert_eq!(
        report.scan.scanned_files, 1,
        "the stat-0 proc file passes the length pre-checks and is read"
    );
    assert!(
        report.scan.scanned_bytes > 0,
        "the ACTUAL read length is charged, not the stat length"
    );
    assert_eq!(
        report.scan.skipped_budget, 1,
        "the real file must be budget-skipped: a wrapped budget would have scanned it instead (scanned_files would be 2)"
    );
}

#[test]
fn decompressed_budget_charge_saturates() {
    // Decompressed bodies are larger than their stat size: 2,833
    // compressed bytes admit through the stat guard, 22,652
    // decompressed bytes must exhaust a 5,000-byte total budget. The
    // charge has to saturate and disclose the skip - never wrap the
    // budget backward and never scan the frame anyway.
    let session = parse_file(&codex_compressed_fixture()).unwrap();
    let report = search_sessions_detailed(
        &[session],
        "turn_context",
        20,
        TRANSCRIPT_SCAN_FILE_BYTE_CAP,
        5_000,
    );
    let text = agenttrace_core::report_search_text_detailed(&report, "turn_context");
    assert!(
        text.contains("1 skipped"),
        "budget exhaustion must be disclosed, got: {}",
        text
    );
}

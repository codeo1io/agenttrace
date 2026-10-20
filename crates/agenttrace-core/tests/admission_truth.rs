// rm-451 + rm-1968 (run 737103b1d0ce cycle 4, batch "admission-truth"):
// integration-level pins for the plain-file admission bound and the
// parse-failure census. The unit tests inside parser.rs / discovery.rs
// pin the seams; these run the PUBLIC entry points a real caller uses —
// `parse_file_with_admission_cap` for the bound (no shared-process env
// surgery) and `load_sessions_with_options` for the census end to end.

use agenttrace_core::{load_sessions_with_options, parse_file_with_admission_cap, LoadOptions};
use std::fs;
use std::path::PathBuf;

fn temp_root(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temp root");
    root
}

/// 200 levels of single-line nesting — the assess F2 reproducer shape.
fn deep_nesting_line() -> String {
    let mut deep = String::from("{\"a\":");
    for _ in 0..200 {
        deep.push_str("{\"a\":");
    }
    deep.push('1');
    for _ in 0..201 {
        deep.push('}');
    }
    deep.push('\n');
    deep
}

#[test]
fn admission_bound_boundary_is_exact_and_refusal_is_derived() {
    // rm-451: exactly-at-cap is admitted (the bound is `> cap`, never
    // `>= cap`), one byte past is refused, and the refusal numeral
    // DERIVES from the effective cap (label discipline: "199 bytes",
    // never a stale hand-written constant), naming the override knob.
    let root = temp_root("at-admission-boundary");
    let path = root.join("junk.jsonl");
    fs::write(&path, vec![b'x'; 200]).expect("write fixture");

    let at_cap = parse_file_with_admission_cap(&path, 200);
    let err = at_cap.expect_err("junk content must fail to parse");
    assert!(
        !format!("{err:#}").contains("file admission bound"),
        "at-cap file must be admitted past the bound: {err:#}"
    );

    let past_cap = parse_file_with_admission_cap(&path, 199);
    let err = past_cap.expect_err("one byte past the cap must be refused");
    let text = format!("{err:#}");
    assert!(text.contains("file admission bound"), "{text}");
    assert!(text.contains("199 bytes"), "derived numeral: {text}");
    assert!(text.contains("200"), "measured length: {text}");
    assert!(
        text.contains("AGENTTRACE_FILE_ADMISSION_MAX_BYTES"),
        "{text}"
    );
}

#[test]
fn u64_max_override_means_no_bound_not_a_panic() {
    // rm-451 hardening: `take(cap + 1)` must saturate. Before the fix a
    // `u64::MAX` cap overflowed the `+ 1` in debug builds (panic) and
    // wrapped in release — an override meaning "no bound" must instead
    // simply admit (the junk file still fails as a format error, never
    // on the bound).
    let root = temp_root("at-admission-u64max");
    let path = root.join("junk.jsonl");
    fs::write(&path, b"not a session\n").expect("write fixture");
    let result = parse_file_with_admission_cap(&path, u64::MAX);
    let err = result.expect_err("junk content must fail to parse");
    assert!(
        !format!("{err:#}").contains("file admission bound"),
        "u64::MAX cap admits: {err:#}"
    );
}

#[test]
fn deep_nesting_through_the_file_lane_names_depth() {
    // rm-1968 clause 2 through parse_file (read + sniff + probe): a
    // serde recursion-limit failure must never surface as "unsupported
    // session format".
    let root = temp_root("at-admission-depth");
    let path = root.join("deep.jsonl");
    fs::write(&path, deep_nesting_line()).expect("write fixture");
    let err = parse_file_with_admission_cap(&path, u64::MAX).expect_err("deep must fail");
    let text = format!("{err:#}");
    assert!(
        text.contains("recursion limit") || text.contains("nesting depth"),
        "depth failure must name depth: {text}"
    );
    assert!(
        !text.contains("unsupported session format"),
        "depth failure must not be mislabeled: {text}"
    );
}

#[test]
fn census_splits_classes_end_to_end() {
    // rm-1968 clauses 1+3+4 end to end through the discovery walk:
    // per-class counts and first errors for a corrupt cline task dir
    // (the walk's ONLY directory lane — a plain directory named
    // *.jsonl is EXCLUDED by the walk, recursed into, never admitted),
    // a non-UTF-8 file, a deep-nesting file, and an over-admission
    // file (env override drives the walk's own cap), while the good
    // file still parses — the aggregate stays non-fatal (rm-835
    // composition: exhausted-set/ok()-swallow semantics unchanged).
    let root = temp_root("at-admission-census");
    // Clause 3, exclusion arm: an EMPTY *.jsonl-named directory is
    // recursed into, never admitted — it must not appear in the
    // census NOR in the discovered count.
    fs::create_dir_all(root.join("fake-dir.jsonl")).expect("excluded dir fixture");
    // Clause 3, disclosure arm: a cline task dir (marker file
    // present) whose transcript is junk fails to parse and lands in
    // its OWN class instead of the generic bucket.
    let corrupt_cline = root.join("cline-corrupt");
    fs::create_dir_all(&corrupt_cline).expect("cline dir fixture");
    fs::write(
        corrupt_cline.join("api_conversation_history.json"),
        "not json at all",
    )
    .expect("cline marker fixture");
    fs::write(
        root.join("good.jsonl"),
        concat!(
            r#"{"type":"user","timestamp":"2026-10-17T10:00:00Z","message":{"content":"hi"}}"#,
            "\n"
        ),
    )
    .expect("good fixture");
    fs::write(root.join("bad.jsonl"), b"\x80abc\n").expect("non-utf8 fixture");
    fs::write(root.join("deep.jsonl"), deep_nesting_line()).expect("deep fixture");
    fs::write(root.join("over.jsonl"), vec![b'x'; 8192]).expect("over fixture");

    let previous = std::env::var_os("AGENTTRACE_FILE_ADMISSION_MAX_BYTES");
    std::env::set_var("AGENTTRACE_FILE_ADMISSION_MAX_BYTES", "4096");
    let result = std::panic::catch_unwind(|| {
        load_sessions_with_options(Some(&root), &LoadOptions::default())
    });
    if let Some(previous) = previous {
        std::env::set_var("AGENTTRACE_FILE_ADMISSION_MAX_BYTES", previous);
    } else {
        std::env::remove_var("AGENTTRACE_FILE_ADMISSION_MAX_BYTES");
    }
    let report = result.expect("walk must not panic");

    assert_eq!(report.sessions.len(), 1, "good file still parses");
    // fake-dir.jsonl (excluded) never joins the discovered set: five
    // candidates admitted — good, cline-corrupt, bad, deep, over.
    assert_eq!(
        report.discovered, 5,
        "the empty *.jsonl directory is excluded from discovery: {:?}",
        report.parse_failure_classes
    );
    assert_eq!(report.parse_failures, 4, "four distinct failures: {:?}", {
        let mut m = std::collections::BTreeMap::new();
        m.extend(
            report
                .parse_failure_classes
                .iter()
                .map(|(k, v)| (k.clone(), *v)),
        );
        m
    });
    let mut classes: Vec<(String, usize)> = report
        .parse_failure_classes
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    classes.sort();
    assert_eq!(
        classes,
        vec![
            ("directory".to_string(), 1),
            ("nesting-depth".to_string(), 1),
            ("non-utf8".to_string(), 1),
            ("over-admission-bound".to_string(), 1),
        ],
        "each hostile fixture lands in its own class"
    );
    assert!(
        !report
            .parse_failure_classes
            .contains_key("unsupported-format"),
        "no hostile fixture may hide in the generic bucket: {:?}",
        report.parse_failure_classes
    );
    for class in [
        "directory",
        "nesting-depth",
        "non-utf8",
        "over-admission-bound",
    ] {
        assert!(
            report.parse_failure_first_by_class.contains_key(class),
            "first error retained for {class}"
        );
    }
    // The directory class carries the cline-lane truth, not a generic
    // format miss.
    let directory_first = &report.parse_failure_first_by_class["directory"];
    assert!(
        directory_first.contains("cline: no parseable events"),
        "directory first error names the cline lane: {directory_first}"
    );
    // The first-error strings stay honest (they carry the real failure
    // wording, including the derived admission numeral for the class
    // the override produced).
    let over_first = &report.parse_failure_first_by_class["over-admission-bound"];
    assert!(
        over_first.contains("file admission bound") && over_first.contains("4096 bytes"),
        "derived cap surfaces in the census: {over_first}"
    );
}

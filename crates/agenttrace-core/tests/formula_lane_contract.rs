use agenttrace_core::{
    compute_overview, parse_file, recommendations, render_doctor_report, render_waste_report,
    report_compare, report_overview_html, report_overview_markdown, report_overview_text,
    report_search_json, report_search_text, report_text, search_sessions, Session, ToolLatency,
};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Vendored byte-for-byte from the assess fdaf8b39 live PoC corpus
/// (delegate/fdaf8b39…-scratch/poc/corpus/=2+5|cmd.jsonl — the vendored
/// copy is renamed `=2+5_cmd.jsonl`: `|` is an un-creatable character on
/// NTFS and the raw PoC filename would break the windows-latest Test
/// lane at checkout; the fixture's poison lives in the message body, so
/// the rename costs nothing): a Claude Code
/// session whose FIRST user message is
/// `=HYPERLINK("http://evil.example","click-me") please review`.
/// The parser derives the session NAME from that message (lib.rs
/// first-user-request provenance), so every report lane renders the
/// formula payload as a cell.
const HOSTILE_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/formula-lane/=2+5_cmd.jsonl"
);
/// Same corpus shape as HOSTILE_FIXTURE, but the assistant's tool call is
/// named `+cmd|' /C calc'!A0` — live-proven by this run's rebuilt poison
/// corpus (assess 6ed54279's original PoC carried the hostile tool name in
/// a separate session; the vendored name-fixture has no tool calls, which
/// is exactly why the TOP TOOLS table row survived the first rm-985 sweep
/// untested).
const HOSTILE_TOOL_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/formula-lane/hostile-tool.jsonl"
);
/// rm-1395 poison leaf: the rm-985 name fixture (`=2+5_cmd.jsonl`)
/// carries NON-zero usage (input 120 / output 40), so it can never mint
/// the doctor's rm-408 zero-usage sample — the exact lane the assess
/// 77d65419 live PoC proved unguarded (`--doctor -d` over a zero-usage
/// corpus emitted `    =HYPERLINK("http://evil.example","click-me")
/// please review: 1 all-zero usage block(s) counted as measured`,
/// out/doctor-text.txt:21). Same corpus shape, but the assistant block
/// reports present-and-all-zero recognized classes, which is what trips
/// `zero_usage_events` and the `collect_zero_usage` sample mint.
const HOSTILE_ZERO_USAGE_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/formula-lane/hostile-zero-usage.jsonl"
);
/// Benign control for the doctor pin: identical zero-usage shape, plain
/// prose first-user-message — it must render byte-identical (no
/// paste-marker), proving the guard costs nothing on benign names.
const BENIGN_ZERO_USAGE_SESSION: &str = concat!(
    "{\"parentUuid\": null, \"isSidechain\": false, \"promptId\": \"p1\", \"type\": \"user\", ",
    "\"message\": {\"role\": \"user\", \"content\": \"benign code review request\"}, ",
    "\"uuid\": \"u1\", \"timestamp\": \"2026-10-12T10:00:00Z\", \"sessionId\": \"s2\", \"version\": \"1.0.0\"}\n",
    "{\"parentUuid\": \"u1\", \"isSidechain\": false, \"type\": \"assistant\", ",
    "\"message\": {\"role\": \"assistant\", \"model\": \"claude-sonnet-4\", ",
    "\"content\": [{\"type\": \"text\", \"text\": \"ok\"}], ",
    "\"usage\": {\"input_tokens\": 0, \"output_tokens\": 0}}, ",
    "\"uuid\": \"a1\", \"timestamp\": \"2026-10-12T10:00:02Z\", \"sessionId\": \"s2\", \"version\": \"1.0.0\"}\n",
);
const TOOL_PAYLOAD_HEAD: &str = "+cmd|";
const PAYLOAD_HEAD: &str = "=HYPERLINK(";
const PAYLOAD_FULL: &str = "=HYPERLINK(\"http://evil.example\",\"click-me\") please review";

fn hostile_sessions() -> Vec<Session> {
    let session = parse_file(Path::new(HOSTILE_FIXTURE))
        .unwrap_or_else(|error| panic!("vendored fixture must parse: {error}"));
    // Provenance pin: the lane inputs below are the transcript-derived
    // session name, not a hand-built stand-in.
    assert_eq!(session.name, PAYLOAD_FULL);
    vec![session]
}

/// Serializes HOME/XDG/AGENTTRACE_SESSION_CACHE_DIR pinning within this
/// test binary (doctor_honesty_contract.rs pattern — the doctor
/// PERSISTS fresh parses through the session cache, so the rm-1395 pin
/// must keep its fixtures out of the operator's real cache; the
/// catch-unwind keeps a failing assertion from poisoning the lock).
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

/// Both live assess PoC surfaces in one corpus: the vendored name fixture
/// (transcript-derived `=HYPERLINK…` session name) plus the hostile-tool
/// fixture whose `assistant.tool_use.name` is `+cmd|' /C calc'!A0` (the
/// name-fixture has no tool calls — see the HOSTILE_TOOL_FIXTURE note
/// above — so both sessions are needed for the value-guard pins).
fn search_lane_sessions() -> Vec<Session> {
    let mut sessions = hostile_sessions();
    let tool_session = parse_file(Path::new(HOSTILE_TOOL_FIXTURE))
        .unwrap_or_else(|error| panic!("hostile-tool fixture must parse: {error}"));
    assert_eq!(tool_session.name, PAYLOAD_FULL);
    sessions.push(tool_session);
    sessions
}

/// Every occurrence of the payload head in a human-readable lane must be
/// immediately preceded by the lane's paste-marker (`'` for text/markdown
/// lanes, `&#39;` for the HTML lane). Substring-absence cannot be the
/// assertion: the guarded form still CONTAINS the payload after the
/// marker — only the cell-start position proves neutralization. Used for
/// the lanes whose cells embed the session name directly (overview text/
/// html/markdown, compare).
fn assert_every_occurrence_preceded(output: &str, lane: &str, marker: &str) {
    for (index, _) in output.match_indices(PAYLOAD_HEAD) {
        assert!(
            output[..index].ends_with(marker),
            "{lane}: unguarded formula payload at byte {index}: {:?}",
            &output[index.saturating_sub(30)..(index + 40).min(output.len())]
        );
    }
}

/// The paste-relevant boundary for terminal text output is the LINE: no
/// trimmed line may BEGIN with the payload (a line that merely contains
/// it after prose — e.g. the search lane's `name: =HYPERLINK…` evidence
/// item behind its `- ` bullet — is a text cell in every spreadsheet,
/// because the cell-start character is the bullet/`n`, never the `=`).
/// Used for the search lane, whose evidence items embed the payload
/// mid-line by construction (`add_match` composes `field: snippet`).
fn assert_no_line_leads_with_payload(output: &str, lane: &str) {
    for line in output.lines() {
        let trimmed = line.trim_start();
        assert!(
            !trimmed.starts_with(PAYLOAD_HEAD),
            "{lane}: a pasted line would lead with the live formula: {trimmed:?}"
        );
    }
}

/// rm-985 (assess fdaf8b39 F1, CWE-1236): the spreadsheet-formula guard
/// family (CSV rm-540, markdown rm-897, sessions TSV 33eddd4) is now
/// routed through EVERY remaining human-readable render lane — the
/// overview text lane (text_cell chokepoint: incident timeline and
/// top-cost-driver rows shipped the raw `=HYPERLINK("http://evil.exa…`
/// session name, overview-text.txt:12/:20), the overview HTML lane
/// (html_escape chokepoint: `<td>=HYPERLINK(&#34;…` at
/// overview-html.txt:39/:43/:61), the search text lane and the compare
/// text lane. A session name that survives any of these lanes verbatim
/// re-arms as a live formula the moment the output is pasted into a
/// spreadsheet. Machine lanes (json) stay byte-lossless BY DESIGN and
/// are intentionally not routed through the guard.
#[test]
fn every_human_lane_neutralizes_the_hostile_formula_corpus() {
    let sessions = hostile_sessions();
    let overview = compute_overview(&sessions);

    let text = report_overview_text(&overview, &sessions);
    assert!(
        text.contains("'=HYPERLINK("),
        "text lane must carry the paste-marker: {text}"
    );
    assert_every_occurrence_preceded(&text, "overview text", "'");

    let html = report_overview_html(&overview, &sessions);
    assert!(
        html.contains("&#39;=HYPERLINK("),
        "html lane must carry the entity-escaped paste-marker: {html}"
    );
    assert_every_occurrence_preceded(&html, "overview html", "&#39;");

    // Control lane: markdown already routed (rm-897) — same corpus must
    // stay guarded there, proving the family is uniform, not special-cased.
    let markdown = report_overview_markdown(&overview, &sessions);
    assert!(markdown.contains("'=HYPERLINK("));
    assert_every_occurrence_preceded(&markdown, "overview markdown", "'");

    let results = search_sessions(&sessions, "please", 10);
    assert!(!results.is_empty(), "hostile name must match the query");
    let search_text = report_search_text(&results, "please");
    assert!(
        search_text.contains("\n'=HYPERLINK("),
        "search text lane must guard the result-name row: {search_text}"
    );
    // Evidence items embed the payload mid-line (`field: snippet`); the
    // paste boundary is the line, and no line may lead with the formula.
    // (rm-540 residual supersedes the weaker "mid-line is text" reading
    // this companion used to carry alone — see the value-guard test below,
    // which upgrades this lane to the strict marker-preceded form.)
    assert_no_line_leads_with_payload(&search_text, "search text");

    let compare = report_compare(&sessions, "claude-sonnet-4");
    assert!(
        compare.contains("'=HYPERLINK("),
        "compare text lane must carry the paste-marker: {compare}"
    );
    assert_every_occurrence_preceded(&compare, "compare text", "'");
}

/// rm-540 residual (assess cbf457f4 F1, run 7fd83ab2 cycle 2): the search
/// evidence rows join `label: value` BEFORE the render-time
/// `guarded_segment(item)` — and `guard_formula_cell` only neutralizes an
/// introducer at the FIRST non-whitespace character, so the whole-item
/// guard could never fire behind the label; every evidence row rendered
/// its payload raw. The VALUE must carry the `'` paste-marker itself:
/// text-to-columns and ": "-aware paste targets make the value a cell of
/// its own. Pinned on both live assess PoCs (`--search cmd` -> the
/// `- tool: '+cmd|…` row; `--search please` -> the `- name: '=HYPERLINK…`
/// row), with the lane upgraded to the strict marker-preceded form and
/// the JSON lane pinned lossless (the guard is a TEXT-render concern).
#[test]
fn search_evidence_rows_carry_the_value_paste_marker() {
    let sessions = search_lane_sessions();

    // Live assess PoC 1: the hostile tool name behind the `tool:` label.
    let tool_hits = search_sessions(&sessions, "cmd", 10);
    assert!(!tool_hits.is_empty(), "hostile tool name must match `cmd`");
    let tool_text = report_search_text(&tool_hits, "cmd");
    assert!(
        tool_text.contains("- tool: '+cmd|"),
        "tool evidence row must guard the VALUE, not the whole line: {tool_text}"
    );
    assert_every_occurrence_preceded(&tool_text, "search text", "'");

    // Live assess PoC 2: the hostile session name behind the `name:` label.
    let name_hits = search_sessions(&sessions, "please", 10);
    assert!(!name_hits.is_empty(), "hostile name must match `please`");
    let name_text = report_search_text(&name_hits, "please");
    assert!(
        name_text.contains("- name: '=HYPERLINK("),
        "name evidence row must guard the VALUE, not the whole line: {name_text}"
    );
    assert_every_occurrence_preceded(&name_text, "search text", "'");

    // The JSON lane is lossless by design: the raw payload rides the
    // machine-readable item untouched (rm-985 doc contract).
    let json = report_search_json(&tool_hits);
    assert!(
        json.contains("tool: +cmd|"),
        "JSON search output must stay lossless (raw payload, no marker): {json}"
    );
}

/// The guard must not damage benign numeric-looking transcript fields:
/// a `-42.50` or `+1e3` cell stays bare in the text and HTML lanes so
/// numeric columns still import as values (the numeric exemption the
/// CSV/markdown guards established, rm-540/rm-897).
#[test]
fn numeric_looking_fields_stay_bare_in_the_guarded_lanes() {
    let mut sessions = hostile_sessions();
    sessions[0].metrics.model_used = "-42.50".to_string();
    sessions[0].branch = "+1e3".to_string();
    let overview = compute_overview(&sessions);
    let text = report_overview_text(&overview, &sessions);
    let html = report_overview_html(&overview, &sessions);
    for output in [&text, &html] {
        assert!(
            !output.contains("'-42.50"),
            "numeric-exempt model cell must stay bare: {output}"
        );
        assert!(
            !output.contains("'+1e3"),
            "numeric-exempt branch cell must stay bare: {output}"
        );
    }
    // And the hostile name is still neutralized alongside them.
    assert!(text.contains("'=HYPERLINK("));
    assert!(html.contains("&#39;=HYPERLINK("));
}

/// rm-985 recommend lane (this run's delta, assess 6ed54279 F1 HIGH): the
/// `--recommend` evidence strings are a text lane the CLI dispatch
/// sanitizer only strips control bytes from, so the retry-loop /
/// tool-failures / context-pressure / slow-tool items shipped the raw
/// `session==HYPERLINK(…)` name (governance.rs:807/:830/:847/:894) and the
/// raw `tool=+cmd|…` name (:896). Every evidence value that embeds a
/// transcript-derived name now carries the `'` paste-marker at its cell
/// start, exactly like the other rm-985 text lanes.
#[test]
fn recommend_lane_neutralizes_the_hostile_formula_corpus() {
    let mut sessions = hostile_sessions();
    let session = &mut sessions[0];
    // Fire all four evidence sites that embed transcript-derived names:
    // retry-loop, tool-failures, context-pressure, slow-tool.
    session.diagnostics.loop_cost.total_loop_cost = 0.5;
    session.diagnostics.loop_cost.retry_events = 3;
    session.diagnostics.loop_cost.loop_groups = 1;
    session.metrics.tool_calls_fail = 4;
    session.metrics.tool_calls_total = 10;
    session.diagnostics.context_utilization.utilization_pct = 95.0;
    session.diagnostics.context_utilization.risk_level = "warning".to_string();
    session.diagnostics.tool_latencies.push(ToolLatency {
        tool_name: "+cmd|' /C calc'!A0".to_string(),
        count: 2,
        avg_sec: 1.0,
        p95_sec: 9.0,
        max_sec: 9.0,
        min_sec: 1.0,
        unmatched: 1,
        is_slow: true,
    });

    let items = recommendations(&sessions);
    let kinds: Vec<&str> = items.iter().map(|item| item.category.as_str()).collect();
    for kind in ["retry_loop", "tool_failure", "context", "latency"] {
        assert!(
            kinds.contains(&kind),
            "expected a {kind} recommendation, got {kinds:?} — the evidence-site pins below are vacuous without it"
        );
    }

    let mut session_evidence = 0;
    let mut tool_evidence = 0;
    for item in &items {
        for evidence in &item.evidence {
            if evidence.starts_with("session=") {
                session_evidence += 1;
                assert_eq!(
                    evidence,
                    "session='=HYPERLINK(\"http://evil.example\",\"click-me\") please review",
                    "session evidence must carry the paste-marker at the cell start: {evidence}"
                );
            }
            if evidence.starts_with("tool=") {
                tool_evidence += 1;
                assert_eq!(
                    evidence, "tool='+cmd|' /C calc'!A0 p95=9.0s unmatched=1",
                    "tool evidence must carry the paste-marker at the cell start: {evidence}"
                );
            }
            // Belt and braces: no evidence line anywhere may LEAD with a
            // formula introducer (the paste boundary for a text lane).
            let trimmed = evidence.trim_start();
            assert!(
                !trimmed.starts_with('=')
                    && !trimmed.starts_with('+')
                    && !trimmed.starts_with('@')
                    && !trimmed.starts_with("-cmd")
                    && !trimmed.contains("|unguarded"),
                "evidence line re-arms as a formula when pasted: {evidence}"
            );
        }
    }
    assert!(
        session_evidence >= 4,
        "all four session= evidence sites must have fired, saw {session_evidence}"
    );
    assert!(
        tool_evidence >= 1,
        "the tool= evidence site must have fired"
    );
}

/// rm-985 TOP TOOLS table row (this run's second delta): the per-session
/// text report's `🔧 TOP TOOLS` table LEADS each row with the tool name,
/// and tool names are transcript-derived — the assess live PoC rendered
/// `  +cmd|' /C calc'!A0` there. The row cell must carry the paste-marker
/// at the cell start like every other text lane. (The JSON trace dumps
/// in the same report quote their values, so they are paste-safe by
/// construction and stay byte-lossless.)
#[test]
fn top_tools_table_row_neutralizes_the_hostile_tool_name() {
    let session = parse_file(Path::new(HOSTILE_TOOL_FIXTURE))
        .unwrap_or_else(|error| panic!("hostile-tool fixture must parse: {error}"));
    let report = report_text(&session);
    let mut table_rows = 0;
    for line in report.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(TOOL_PAYLOAD_HEAD) {
            panic!("TOP TOOLS row leads with a live formula introducer: {line:?}");
        }
        if trimmed.starts_with("'+cmd|") {
            table_rows += 1;
            assert!(
                trimmed.contains("'+cmd|' /C calc'!A0"),
                "guarded row must preserve the full name after the marker: {line:?}"
            );
        }
    }
    assert!(
        table_rows >= 1,
        "the TOP TOOLS table row must have rendered (and must carry the marker): {report}"
    );
    // The benign name-fixture control: a plain tool name stays bare.
    let benign = hostile_sessions().remove(0);
    let benign_report = report_text(&benign);
    assert!(benign_report.contains("  write_file"));
}

/// rm-985 waste lane (review fix, review 1bbb59f2 finding F1 / attempt
/// d28abb78): the `--waste -f text` Tool Bloat table LEADS each row with
/// the transcript-derived tool name (`"    {:<25} …"`), and the row's four
/// leading spaces do NOT protect a pasted cell — the project's own rm-540
/// threat model says spreadsheet apps ignore leading whitespace when
/// deciding a cell is a formula. The review's live PoC rendered
/// `    +cmd|' /C calc'!A0          1x ~$0.0010 (100% of session)` on the
/// implement tree (the implement sweep's introducer grep was not
/// whitespace-tolerant, so it reported the lane clean). The row now
/// carries the same `'` paste-marker at its first non-whitespace
/// character as every other rm-985 text lane — L1's own rule applied
/// retroactively: one poison leaf per render family, and the waste
/// family finally has one. (The `top tool {:?}` action line quotes its
/// payload — safe by construction; the `--waste -f json` lane stays
/// byte-lossless by design, like every machine lane.)
#[test]
fn waste_tool_bloat_row_neutralizes_the_hostile_tool_name() {
    let session = parse_file(Path::new(HOSTILE_TOOL_FIXTURE))
        .unwrap_or_else(|error| panic!("hostile-tool fixture must parse: {error}"));
    let report = render_waste_report(&session);
    let mut bloat_rows = 0;
    for line in report.lines() {
        let trimmed = line.trim_start();
        assert!(
            !trimmed.starts_with(TOOL_PAYLOAD_HEAD),
            "Tool Bloat row leads with a live formula introducer: {line:?}"
        );
        if trimmed.starts_with("'+cmd|") {
            bloat_rows += 1;
            assert!(
                trimmed.contains("'+cmd|' /C calc'!A0"),
                "guarded row must preserve the full name after the marker: {line:?}"
            );
        }
    }
    assert!(
        bloat_rows >= 1,
        "the Tool Bloat row must have rendered (and must carry the marker): {report}"
    );
    // Benign control in the SAME table: a plain tool name renders bare —
    // the guard must not cost a paste-marker on non-introducer names.
    let mut benign = parse_file(Path::new(HOSTILE_TOOL_FIXTURE))
        .unwrap_or_else(|error| panic!("hostile-tool fixture must parse: {error}"));
    benign
        .metrics
        .tool_usage
        .insert("write_file".to_string(), 1);
    let benign_report = render_waste_report(&benign);
    assert!(
        benign_report.contains("write_file"),
        "benign tool row must render: {benign_report}"
    );
    assert!(
        !benign_report.contains("'write_file"),
        "benign tool name must stay bare (no paste-marker): {benign_report}"
    );
}

// ---------------------------------------------------------------------------
// Doctor text-report lanes (rm-897 residual; run a64735b75a46 cycle 3)
// ---------------------------------------------------------------------------
//
// Provenance pins:
// * formula-lane/=2+5_cmd.jsonl — the hostile-name session fixture above.
// * formula-lane/=2+5_zero_usage.jsonl — the same session with the assistant
//   usage block zeroed (every token class 0) so the session lands in the
//   zero-usage census (rm-408: present-but-zero counts as measured). Derived
//   at base 3e90993 by zeroing the usage object of =2+5_cmd.jsonl.
//
// The doctor TEXT report renders three corpus-controlled values at line
// start: zero-usage sample names, pre-rendered project-decode Resolved lines
// (their dir-derived prefix leads the line), and duplicate sqlite db paths.
// Paste-safety demands the same `'` marker the cell lanes get; the JSON
// surface stays byte-lossless BY DESIGN (asserted below as the contrast).

fn copy_fixture_to(dir: &std::path::Path, fixture: &str, as_name: &str) -> std::path::PathBuf {
    let src = Path::new(fixture);
    let dst = dir.join(as_name);
    std::fs::copy(src, &dst)
        .unwrap_or_else(|error| panic!("copy {fixture} into {}: {error}", dir.display()));
    dst
}

fn scratch_path(name: &str) -> std::path::PathBuf {
    // rm-1882 (rm-1065 discipline): per-call unique scratch paths. The
    // pid separates concurrent cargo-test processes on one host; the
    // monotonically increasing suffix separates tests (and repeat
    // calls) within one process, so no two invocations can ever race
    // remove_dir_all/create_dir_all on the same fixed path.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let (stem, ext) = match name.rfind('.') {
        Some(idx) => (&name[..idx], &name[idx..]),
        None => (name, ""),
    };
    std::env::temp_dir().join(format!("{stem}-{}-{seq}{ext}", std::process::id()))
}

#[test]
fn doctor_zero_usage_sample_lane_gets_formula_guard() {
    let tmp = scratch_path("at-doc-guard-zero");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("mkdir zero-usage corpus");
    copy_fixture_to(
        &tmp,
        "tests/fixtures/formula-lane/=2+5_zero_usage.jsonl",
        "session.jsonl",
    );
    let report = agenttrace_core::build_doctor_report(Some(&tmp), false);
    assert!(
        !report.zero_usage.samples.is_empty(),
        "fixture must land in the zero-usage census: {:?}",
        report.zero_usage.samples
    );
    let text = agenttrace_core::doctor_report_text(&report);
    // RED-FIRST: the sample line must carry the paste marker before the payload.
    assert_every_occurrence_preceded(&text, "=HYPERLINK(", "'");
    assert!(
        text.contains("'=HYPERLINK(\"http://evil.example\",\"click-me\")"),
        "guarded zero-usage sample must preserve the payload after the marker:\n{text}"
    );
}

// Defense-in-depth pin for the decode-sample render loop: today's mint
// sites bake fixed prefixes ("truncated:"/"opaque:"/"ambiguous:"/
// "unresolved:"), so live corpora cannot lead these lines with corpus
// bytes. The render loop still guards baked lines so a future arm change
// cannot silently reopen the lane (rm-897 residual discipline).
#[test]
fn doctor_project_decode_sample_render_stays_guarded() {
    let mut report = agenttrace_core::DoctorReport::default();
    report.project_decode.samples = vec![
        "opaque: benign-name kept as its own stable identity".to_string(),
        "=2+5 hostile baked line".to_string(),
    ];
    let text = agenttrace_core::doctor_report_text(&report);
    assert_every_occurrence_preceded(&text, "=2+5", "'");
    assert!(
        text.contains("'=2+5 hostile baked line"),
        "future regressive arm output must be neutralized at render:\n{text}"
    );
    assert!(
        text.contains("opaque: benign-name"),
        "benign fixed-prefix lines must stay bare (no marker):\n{text}"
    );
}

#[test]
fn doctor_duplicate_db_lane_gets_formula_guard_and_json_stays_raw() {
    // Synthetic duplicate entry: the render lane is the unit under guard
    // (dedup ingest needs a real sqlite corpus; JSON rawness is the design).
    let mut report = agenttrace_core::DoctorReport::default();
    report
        .sqlite_duplicate_dbs
        .push(agenttrace_core::SqliteDuplicateDb {
            path: "=2+5.db".to_string(),
            duplicate_sessions: 2,
        });
    let text = agenttrace_core::doctor_report_text(&report);
    assert!(
        text.contains("'=2+5.db"),
        "guarded duplicate-db line must carry the marker:\n{text}"
    );

    // Contrast: the same hostile session flows through the JSON surface with
    // NO marker — machine lanes stay byte-lossless BY DESIGN.
    let tmp = scratch_path("at-doc-guard-json");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("mkdir json corpus");
    copy_fixture_to(
        &tmp,
        "tests/fixtures/formula-lane/=2+5_zero_usage.jsonl",
        "session.jsonl",
    );
    let json = agenttrace_core::render_doctor_report(Some(&tmp), false, "json")
        .expect("render json doctor report");
    assert!(
        json.contains("=HYPERLINK(") && !json.contains("'=HYPERLINK("),
        "JSON surface must stay raw (payload present, marker absent):\n{json}"
    );
}

/// rm-1395 (run 966bd5f9 cycle 4, assess 77d65419 PoC replay): the
/// doctor's default text lane rendered its transcript-derived sample
/// families — the rm-408 zero-usage session names (`collect_zero_usage`
/// mints them from `session.name`, which derives from the transcript's
/// FIRST user message) and the rm-381 project-decode samples — as
/// `    {sample}` lines. Four leading spaces were the only prefix, and
/// a spreadsheet paste treats the first non-whitespace character as
/// the cell start, so a `=HYPERLINK(…)` first user message re-armed as
/// a live formula cell (live PoC: `--doctor -d` over the zero-usage
/// corpus emitted `    =HYPERLINK("http://evil.example","click-me")
/// please review: 1 all-zero usage block(s) counted as measured`).
/// rm-897's executed sweep re-swept 23 lanes whitespace-tolerant but
/// never enumerated the doctor leaf (its own rider warns coverage is
/// reachability-dependent — that corpus rendered no doctor leaf). Both
/// families now route through the same `guard_formula_cell` the CSV
/// (rm-540) / markdown (rm-897) / TSV lanes use, at ONE render
/// choke-point; the JSON lane stays byte-lossless by construction (the
/// guard is text-render-only). This pin drives the REAL doctor walk
/// (discovery + cache persist + render) over a corpus holding the
/// hostile zero-usage fixture beside a benign control.
#[test]
fn doctor_sample_lane_neutralizes_the_hostile_formula_corpus() {
    let home = std::env::temp_dir().join(format!("at-rm1395-home-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).expect("create temp home");
    with_home(&home, || {
        let corpus = std::env::temp_dir().join(format!("at-rm1395-corpus-{}", std::process::id()));
        let _ = fs::remove_dir_all(&corpus);
        fs::create_dir_all(&corpus).expect("create corpus dir");
        fs::copy(HOSTILE_ZERO_USAGE_FIXTURE, corpus.join("hostile.jsonl"))
            .expect("plant hostile zero-usage fixture");
        fs::write(corpus.join("benign.jsonl"), BENIGN_ZERO_USAGE_SESSION)
            .expect("plant benign control fixture");

        let text = render_doctor_report(Some(&corpus), false, "text").expect("render doctor text");

        // The poison must have actually minted a zero-usage sample, or
        // every assertion below is vacuous — the guarded line, whole.
        let guarded_line = concat!(
            "'=HYPERLINK(\"http://evil.example\",\"click-me\") please review: ",
            "1 all-zero usage block(s) counted as measured"
        );
        assert!(
            text.contains(&format!("    {guarded_line}\n")),
            "the hostile zero-usage sample must render guarded: {text}"
        );
        // Family uniformity: every payload occurrence carries the
        // paste-marker at its cell start, and no pasted line LEADS
        // with the live formula.
        assert_every_occurrence_preceded(&text, "doctor text", "'");
        assert_no_line_leads_with_payload(&text, "doctor text");
        // Acceptance arm: zero SAMPLE lines — the four-space-indented
        // cell surface both families render into — whose first
        // non-whitespace char is an introducer carry an unmarked
        // payload: any introducer-led sample must open with the `'`
        // marker instead (fullwidth introducers included, matching
        // guard_formula_cell). Prose bullets elsewhere in the report
        // (`- Ready: …`) are not paste cells and stay out of scope —
        // the rm-897 false-positive trade was made for cells, not for
        // the report's own vocabulary.
        for line in text.lines() {
            if !line.starts_with("    ") {
                continue;
            }
            let trimmed = line.trim_start();
            if let Some(first) = trimmed.chars().next() {
                if matches!(
                    first,
                    '=' | '+' | '@' | '-' | '\u{FF1D}' | '\u{FF0B}' | '\u{FF20}' | '\u{FF0D}'
                ) {
                    assert!(
                        trimmed.starts_with('\''),
                        "doctor text: introducer-led sample line lacks the paste-marker: {trimmed:?}"
                    );
                }
            }
        }

        // Benign control: the harmless name renders byte-identical —
        // the guard buys no paste-marker on non-introducer names.
        assert!(
            text.contains(concat!(
                "    benign code review request: ",
                "1 all-zero usage block(s) counted as measured\n"
            )),
            "benign zero-usage sample must render unmarked: {text}"
        );
        assert!(
            !text.contains("'benign"),
            "benign sample must not carry a paste-marker: {text}"
        );

        // The JSON lane stays byte-lossless by design: the guard is a
        // text-render arm, so the machine lane carries the raw name.
        let json = render_doctor_report(Some(&corpus), false, "json").expect("render doctor json");
        assert!(
            json.contains("=HYPERLINK("),
            "json lane must stay lossless and carry the raw sample: {json}"
        );
        assert!(
            !json.contains("'=HYPERLINK("),
            "json lane must not carry the text lane's paste-marker: {json}"
        );
    });
}

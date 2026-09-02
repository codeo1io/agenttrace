use agenttrace_core::{
    build_doctor_report, data_health, data_health_scoped, find_session_files,
    load_sessions_from_dir, load_sessions_with_options, load_sessions_with_progress, parse_file,
    render_waste_report, search_sessions, session_cache_path, session_capability, total_tokens,
    LoadOptions,
};
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::sync::{Mutex, OnceLock};

const SAMPLE_JSONL: &str = r#"{"role":"session_meta","timestamp":"2026-05-02T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"meta","ModelUsed":"claude-sonnet-4","Usage":{"input_tokens":1000,"output_tokens":500}}
{"role":"user","content":"Inspect billing export.","timestamp":"2026-05-02T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"assistant","content":"I will inspect the route.","timestamp":"2026-05-02T10:00:01Z","reasoning":"Find the route and keep the change small.","tool_calls":[{"id":"t1","name":"rg","args":"billing export"}],"ModelUsed":"claude-sonnet-4"}
{"role":"tool","content":"{\"success\":true}","tool_call_id":"t1","timestamp":"2026-05-02T10:00:02Z"}
"#;

fn generated_fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/generated")
        .join(name)
}

fn adversarial_sqlite_fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/generated/adversarial/sqlite")
        .join(name)
}

/// Copies an opencode database fixture into a temporary HOME so it loads
/// through the normal discovery path (databases are only discovered under
/// `$HOME/.local/share/opencode/`).
fn seed_home_with_opencode_db(home: &std::path::Path, db_path: &std::path::Path) {
    let target = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    fs::create_dir_all(target.parent().expect("db parent")).expect("create db parent");
    fs::copy(db_path, &target).expect("copy opencode fixture db");
}

#[test]
fn adversarial_sqlite_overflow_db_neither_panics_nor_wraps() {
    // P5-1 reproducer: two assistant messages with i64::MAX input tokens
    // made the per-session accumulator `agg.input_tokens += input`
    // overflow (debug exit 101) or wrap negative (release). Loading must
    // saturate: total pinned at the i64 ceiling, never negative, never a
    // panic. Regression guard for the cycle-2 H1 hardening.
    let root = temp_root("agenttrace-adversarial-sqlite-overflow");
    let home = root.join("home");
    seed_home_with_opencode_db(&home, &adversarial_sqlite_fixture("overflow.db"));

    with_home(&home, || {
        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1, "overflow fixture must yield one session");
        let metrics = &sessions[0].metrics;
        assert_eq!(
            metrics.tokens_input,
            i64::MAX,
            "saturated accumulation must pin input at the i64 ceiling"
        );
        assert_eq!(metrics.tokens_output, 2);
        assert!(metrics.tokens_input >= 0);
        assert!(metrics.tokens_output >= 0);
        assert!(metrics.cost_estimated.is_finite());
        assert!(metrics.cost_estimated >= 0.0);
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn adversarial_sqlite_u64_max_input_saturates_instead_of_wrapping() {
    // P5-2 reproducer: a `tokens.input` of u64::MAX wrapped through
    // `n as i64` and was reported as `"input": -1` by --latest -f json.
    // The value must saturate at i64::MAX instead.
    let root = temp_root("agenttrace-adversarial-sqlite-wrap");
    let home = root.join("home");
    seed_home_with_opencode_db(&home, &adversarial_sqlite_fixture("wrap.db"));

    with_home(&home, || {
        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1, "wrap fixture must yield one session");
        let metrics = &sessions[0].metrics;
        assert_eq!(
            metrics.tokens_input,
            i64::MAX,
            "u64::MAX input must saturate at i64::MAX, not wrap to -1"
        );
        assert_eq!(metrics.tokens_output, 1);
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn opencode_stored_session_totals_preferred_with_delta() {
    // CU-2 (candidate 8, totals scope): when the upstream session row
    // records authoritative totals (cost + the five token columns), those
    // displace message-derived aggregation, the provenance says so, and
    // the stored-versus-derived delta is exposed on the session for
    // data_health reporting.
    let root = temp_root("agenttrace-opencode-stored-totals");
    let home = root.join("home");
    let db_path = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
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
            tokens_cache_write integer
        );
        create table message (session_id text, data text);
        create table part (session_id text, data text);
        insert into session values (
            'ses_stored', 'Stored DB', 1764750000000, 1764750004000,
            0.5, 1000, 200, 40, 30, 20
        );
        insert into message values (
            'ses_stored',
            '{"id":"msg1","role":"assistant","modelID":"claude-sonnet-4","tokens":{"input":400,"output":150,"reasoning":10,"cache":{"read":5,"write":5}}}'
        );
        "#,
    )
    .expect("seed stored-totals opencode db");
    drop(db);

    with_home(&home, || {
        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1);
        let metrics = &sessions[0].metrics;
        assert_eq!(metrics.source_tool, "opencode_db");
        assert_eq!(metrics.tokens_input, 1000, "stored input must win");
        assert_eq!(
            metrics.tokens_output, 240,
            "stored output must include stored reasoning tokens"
        );
        assert_eq!(metrics.tokens_cache_r, 30);
        assert_eq!(metrics.tokens_cache_w, 20);
        assert_eq!(metrics.cost_estimated, 0.5, "stored cost must win");
        assert_eq!(
            metrics.provenance.tokens, "stored_session_totals",
            "provenance must disclose that totals came from the session row"
        );
        // derived = 400 + 150 + 10 + 5 + 5 = 570; stored = 1000+240+30+20 = 1290
        assert_eq!(
            metrics.stored_totals_delta, 720,
            "delta must expose how far derived aggregation drifted"
        );
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn opencode_placeholder_titles_gate_to_message_derived_names() {
    // Research candidate 34: OpenCode fills `title` with
    // `New session - <timestamp>` on every session it does not summarize
    // (227/227 placeholder titles in the live census at research pass 5),
    // and the reader used it verbatim. The placeholder must be treated as
    // absent so the first user message text names the session, with the
    // gate disclosed in naming provenance.
    let root = temp_root("agenttrace-opencode-placeholder-title");
    let home = root.join("home");
    let db_path = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    fs::create_dir_all(db_path.parent().expect("db parent")).expect("create db parent");
    let db = Connection::open(&db_path).expect("open opencode db");
    db.execute_batch(
        r#"
        create table session (id text primary key, title text, time_created integer, time_updated integer);
        create table message (id text, session_id text, data text);
        create table part (id text, session_id text, message_id text, time_created integer, time_updated integer, data text);
        insert into session values ('ses_placeholder', 'New session - 2026-08-10T01:13:33.266Z', 1764750000000, 1764750004000);
        insert into session values ('ses_real_title', 'Real provider title', 1764750100000, 1764750104000);
        insert into session values ('ses_empty_title', '', 1764750200000, 1764750204000);
        insert into message values ('msg1', 'ses_placeholder', '{"id":"msg1","role":"user"}');
        insert into message values ('msg2', 'ses_real_title', '{"id":"msg2","role":"user"}');
        insert into message values ('msg3', 'ses_empty_title', '{"id":"msg3","role":"assistant"}');
        insert into part values ('part1', 'ses_placeholder', 'msg1', 1, 1, '{"type":"text","text":"audit the billing export please"}');
        insert into part values ('part2', 'ses_real_title', 'msg2', 2, 2, '{"type":"text","text":"this text must not become the name"}');
        "#,
    )
    .expect("seed placeholder-title opencode db");
    drop(db);

    with_home(&home, || {
        let sessions = load_sessions_from_dir(None);
        assert_eq!(
            sessions.len(),
            3,
            "placeholder fixture must yield three sessions"
        );
        let by_name = |needle: &str| sessions.iter().find(|s| s.name.contains(needle));

        let placeholder = by_name("audit the billing export")
            .expect("placeholder title must yield to the message-derived name");
        assert_eq!(placeholder.name, "audit the billing export please");
        assert!(!placeholder.name.contains("New session"));
        assert_eq!(
            placeholder.metrics.provenance.naming, "provider:placeholder",
            "the gate must be disclosed in naming provenance"
        );

        let titled = by_name("Real provider title").expect("real titles survive");
        assert_eq!(titled.metrics.provenance.naming, "provider_title");

        let empty = sessions
            .iter()
            .find(|s| s.name == "ses_empty_title")
            .expect("empty title with no user text falls back to the session id");
        assert_eq!(empty.metrics.provenance.naming, "session_id");
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn opencode_missing_stored_columns_fall_back_to_derived() {
    // CU-2 companion: an older schema without the stored-total columns
    // keeps the derived path and the usual provenance, with zero delta.
    let root = temp_root("agenttrace-opencode-derived-fallback");
    let home = root.join("home");
    write_opencode_db(
        &home
            .join(".local")
            .join("share")
            .join("opencode")
            .join("opencode.db"),
    );

    with_home(&home, || {
        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1);
        let metrics = &sessions[0].metrics;
        assert_eq!(metrics.tokens_input, 42);
        assert_eq!(metrics.tokens_output, 22);
        assert_eq!(metrics.provenance.tokens, "reported_by_agent");
        assert_eq!(metrics.stored_totals_delta, 0);
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn opencode_text_valued_stored_columns_do_not_drop_the_session() {
    // SQLite columns are dynamically typed: a TEXT "999" in a stored
    // token column previously failed the row conversion and silently
    // dropped the whole session. It must parse; unparseable values fall
    // back to derived aggregation instead of dropping the row.
    let root = temp_root("agenttrace-opencode-text-stored");
    let home = root.join("home");
    let db_path = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    fs::create_dir_all(db_path.parent().expect("db parent")).expect("create db parent");
    let db = Connection::open(&db_path).expect("open opencode db");
    db.execute_batch(
        r#"
        create table session (
            id text primary key, title text, time_created, time_updated,
            cost, tokens_input, tokens_output, tokens_reasoning,
            tokens_cache_read, tokens_cache_write
        );
        create table message (session_id text, data text);
        create table part (session_id text, data text);
        insert into session values (
            'ses_text', 'Text DB', 1764750000000, 1764750004000,
            '0.25', '999', 'not-a-number', 40, 30, 20
        );
        insert into message values (
            'ses_text',
            '{"id":"msg1","role":"assistant","modelID":"claude-sonnet-4","tokens":{"input":400,"output":150,"cache":{"read":5,"write":5}}}'
        );
        "#,
    )
    .expect("seed text-valued opencode db");
    drop(db);

    with_home(&home, || {
        let sessions = load_sessions_from_dir(None);
        assert_eq!(
            sessions.len(),
            1,
            "session must survive text-valued columns"
        );
        let metrics = &sessions[0].metrics;
        assert_eq!(metrics.tokens_input, 999, "TEXT '999' must parse");
        assert_eq!(metrics.cost_estimated, 0.25, "TEXT '0.25' cost must parse");
        assert_eq!(
            metrics.provenance.tokens, "stored_session_totals",
            "stored columns that parse still win"
        );
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn opencode_unknown_time_session_stays_visible_in_range() {
    // CU-3 (N7): a session with time_created = 0 (unknown time) used to
    // vanish from every --range/--since view via both the SQL predicate
    // and the post-load filter. It must stay visible; its unknown-time
    // status is counted by data_health instead.
    let root = temp_root("agenttrace-opencode-unknown-time");
    let home = root.join("home");
    let db_path = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    fs::create_dir_all(db_path.parent().expect("db parent")).expect("create db parent");
    let db = Connection::open(&db_path).expect("open opencode db");
    db.execute_batch(
        r#"
        create table session (
            id text primary key, title text, time_created integer, time_updated integer
        );
        create table message (session_id text, data text);
        create table part (session_id text, data text);
        insert into session values ('ses_unknown', 'Unknown Time', 0, 0);
        insert into message values ('ses_unknown', '{"id":"m1","role":"user"}');
        "#,
    )
    .expect("seed unknown-time opencode db");
    drop(db);

    with_home(&home, || {
        let since = chrono::Utc::now() - chrono::Duration::days(7);
        let report = load_sessions_with_options(
            None,
            &LoadOptions {
                since: Some(since),
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            report.sessions.len(),
            1,
            "unknown-time session must stay visible under --range 7d"
        );
        let metrics = &report.sessions[0].metrics;
        assert_eq!(
            metrics.session_start, "",
            "start stays unknown, not fabricated"
        );
        let health = agenttrace_core::data_health(&report.sessions, report.discovered, 0);
        assert_eq!(
            health.unknown_time_sessions, 1,
            "unknown-time bucket is counted"
        );
        assert_eq!(health.stored_totals_sessions, 0);
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn hermes_negative_token_columns_are_clamped() {
    // CU-1: a corrupt hermes state.db with negative token columns must
    // not propagate negatives into reports (the adjacent event/tool
    // counts already clamp with .max(0)).
    let root = temp_root("agenttrace-hermes-negative-tokens");
    let home = root.join("home");
    fs::create_dir_all(home.join(".hermes")).expect("create hermes dir");
    let db = Connection::open(home.join(".hermes").join("state.db")).expect("open hermes db");
    db.execute_batch(
        r#"
        create table sessions (
            id text primary key, model text, started_at real, ended_at real,
            message_count integer, tool_call_count integer,
            input_tokens integer, output_tokens integer,
            cache_read_tokens integer, cache_write_tokens integer
        );
        create table messages (session_id text, role text);
        insert into sessions values ('neg', 'gpt-5.1', 1760000000, 1760000060, 1, 0, -500, -20, -10, -5);
        insert into messages values ('neg', 'user');
        "#,
    )
    .expect("seed negative hermes db");
    drop(db);

    with_home(&home, || {
        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1);
        let metrics = &sessions[0].metrics;
        assert_eq!(metrics.tokens_input, 0, "negative input must clamp to 0");
        assert_eq!(metrics.tokens_output, 0);
        assert_eq!(metrics.tokens_cache_r, 0);
        assert_eq!(metrics.tokens_cache_w, 0);
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn unicode_escape_hostile_lines_never_panic_from_format_detection() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/generated/adversarial/unicode-escape.jsonl");
    let parsed = parse_file(&fixture);
    assert!(
        parsed.is_err(),
        "hostile unicode escapes must leave the file unrecognized, got {parsed:?}"
    );
}

#[test]
fn generic_fallback_recovers_recoverable_lines_and_reports_the_rest() {
    // Pass-7 P7-1 committed reproducer: a lone-surrogate line and an
    // Event-typed usage line used to vanish from the generic JSONL
    // fallback with byte-identical health; the broken escape must be
    // counted as a loss instead of ignored.
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/generated/adversarial/generic-loss.jsonl");
    let session = parse_file(&fixture).expect("generic-loss fixture parses");
    assert_eq!(session.metrics.user_messages, 2);
    assert_eq!(session.metrics.tokens_input, 7);
    assert_eq!(
        session.metrics.line_skips,
        std::collections::BTreeMap::from([("unparseable_line".to_string(), 1)])
    );
    // The per-reason losses aggregate into DataHealth (pass-7 P7-1) so
    // every surface can disclose them.
    let health = data_health(&[session], 1, 0);
    assert_eq!(
        health.line_skips,
        std::collections::BTreeMap::from([("unparseable_line".to_string(), 1)])
    );
    assert_eq!(health.confidence, "low");
}

#[test]
fn unicode_escape_hostile_file_does_not_kill_directory_scans() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/generated/adversarial");
    let sessions = load_sessions_from_dir(Some(&dir));
    assert!(
        sessions.len() >= 3,
        "clean neighbors must survive a hostile unicode file, got {}",
        sessions.len()
    );
}

#[test]
fn generated_adversarial_corpus_stays_bounded_and_non_negative() {
    // The committed repro corpus from the adversarial assessment: two
    // claude_code sessions with 1e300 usage, an oh-my-pi usage carrying two
    // legal i64::MAX aliases, and a workbuddy entry with clamped-negative
    // input plus a cache-read subtraction. Parsing and aggregating it must
    // neither panic (debug overflow checks) nor produce negative totals
    // (release wrapping) — the H1 acceptance criterion.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/generated/adversarial");
    let sessions = load_sessions_from_dir(Some(&dir));
    assert!(
        sessions.len() >= 3,
        "expected the adversarial corpus to yield sessions, got {}",
        sessions.len()
    );
    let mut grand_total = 0i64;
    for session in &sessions {
        let tokens = total_tokens(session);
        assert!(
            tokens >= 0,
            "session {} reported negative tokens {tokens}",
            session.name
        );
        assert!(session.metrics.cost_estimated.is_finite());
        assert!(session.metrics.cost_estimated >= 0.0);
        grand_total = grand_total.saturating_add(tokens);
    }
    assert_eq!(
        grand_total,
        i64::MAX,
        "saturated sessions must pin the ceiling"
    );
}

#[test]
fn generated_capability_and_step_fixtures_cover_degradation() {
    let detailed = parse_file(&generated_fixture("detailed-tool-steps.jsonl")).unwrap();
    assert_eq!(session_capability(&detailed), "detailed");
    assert_eq!(detailed.diagnostics.steps.len(), 3);
    assert_eq!(detailed.diagnostics.steps[0].status, "ok");
    assert_eq!(detailed.diagnostics.steps[1].status, "error");
    assert_eq!(detailed.diagnostics.steps[2].status, "missing");
}

#[test]
fn generated_sql_builds_an_aggregate_only_session() {
    let root = temp_root("agenttrace-generated-aggregate");
    let home = root.join("home");
    let db_path = home.join(".hermes/state.db");
    fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    let db = Connection::open(&db_path).unwrap();
    db.execute_batch(&fs::read_to_string(generated_fixture("aggregate-session.sql")).unwrap())
        .unwrap();
    drop(db);
    with_home(&home, || {
        let sessions = agenttrace_core::load_sqlite_backed_sessions();
        let aggregate = sessions
            .iter()
            .find(|session| session.name == "aggregate")
            .unwrap();
        assert_eq!(session_capability(aggregate), "aggregate");
        assert!(aggregate.diagnostics.steps.is_empty());
        let limited = sessions
            .iter()
            .find(|session| session.name == "limited")
            .unwrap();
        assert_eq!(session_capability(limited), "limited");
        assert!(limited.diagnostics.steps.is_empty());
    });
    let _ = fs::remove_dir_all(root);
}

#[test]
fn generated_steps_never_serialize_content_or_arguments() {
    let session = parse_file(&generated_fixture("tool-step-redaction.jsonl")).unwrap();
    let json = serde_json::to_string(&session.diagnostics.steps).unwrap();
    assert!(!json.contains("SHOULD_NOT_LEAK"));
}

#[test]
fn generated_provider_fixtures_stay_parseable() {
    for (name, source) in [
        ("workbuddy.jsonl", "workbuddy"),
        ("antigravity.jsonl", "antigravity_cli"),
        ("copilot-session.jsonl", "copilot_cli"),
        ("kimi-wire.jsonl", "kimi_cli"),
        ("openclaw-wrapper.json", "openclaw"),
        ("qwen-stream.jsonl", "qwen_code"),
    ] {
        let parsed =
            parse_file(&generated_fixture(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(parsed.metrics.source_tool, source, "{name}");
    }

    let root = temp_root("agenttrace-generated-pi");
    let path = root.join(".pi/agent/sessions/pi-session.jsonl");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::copy(generated_fixture("pi-session.jsonl"), &path).unwrap();
    assert_eq!(parse_file(&path).unwrap().metrics.source_tool, "pi");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_discovers_and_loads_real_jsonl_files() {
    let root =
        std::env::temp_dir().join(format!("agenttrace-rust-discovery-{}", std::process::id()));
    let nested = root.join("nested");
    fs::create_dir_all(&nested).expect("create test dir");
    let session_path = nested.join("session.jsonl");
    let ignored_path = nested.join("sessions.json");
    fs::write(&session_path, SAMPLE_JSONL).expect("write session");
    fs::write(&ignored_path, SAMPLE_JSONL).expect("write ignored cache");

    let mut loaded_health = None;
    with_session_cache(&root.join("cache"), || {
        let files = find_session_files(Some(&root));
        assert_eq!(files, vec![session_path.clone()]);

        let sessions = load_sessions_from_dir(Some(&root));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "Inspect billing export.");
        assert_eq!(sessions[0].metrics.model_used, "claude-sonnet-4");
        assert_eq!(sessions[0].metrics.source_tool, "hermes_jsonl");
        assert_eq!(sessions[0].metrics.tool_calls_total, 1);
        loaded_health = Some(sessions[0].health);
    });

    let parsed = parse_file(&session_path).expect("parse single file");
    assert_eq!(parsed.health, loaded_health.expect("loaded session health"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_workbuddy_messages_tools_usage_and_millis() {
    let root = temp_root("agenttrace-rust-workbuddy");
    fs::create_dir_all(&root).expect("create workbuddy temp dir");
    let session_path = root.join("session.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"message","role":"user","content":[{"type":"input_text","text":"inspect"}],"timestamp":1783777800000,"sessionId":"s1","cwd":"/tmp/project","providerData":{"agent":"cli"}}
{"type":"reasoning","content":[],"rawContent":[{"type":"reasoning_text","text":"check first"}],"timestamp":1783777801000,"sessionId":"s1","cwd":"/tmp/project","providerData":{"model":"glm-5.2","agent":"cli"}}
{"type":"function_call","name":"Read","callId":"c1","arguments":"{\"path\":\"a.rs\"}","timestamp":1783777802000,"sessionId":"s1","cwd":"/tmp/project","message":{"usage":{"input_tokens":100,"output_tokens":20,"cache_read_input_tokens":60}},"providerData":{"model":"glm-5.2","agent":"cli"}}
{"type":"function_call_result","name":"Read","callId":"c1","status":"completed","output":{"type":"text","text":"ok"},"timestamp":1783777803000,"sessionId":"s1","cwd":"/tmp/project","providerData":{"model":"glm-5.2","agent":"cli"}}
{"type":"message","role":"assistant","content":[{"type":"output_text","text":"done"}],"timestamp":1783777804000,"sessionId":"s1","cwd":"/tmp/project","message":{"usage":{"input_tokens":120,"output_tokens":30,"cache_read_input_tokens":80}},"providerData":{"model":"glm-5.2","agent":"cli"}}
"#,
    )
    .expect("write workbuddy session");

    let parsed = parse_file(&session_path).expect("parse workbuddy session");
    assert_eq!(parsed.cwd, "/tmp/project");
    assert_eq!(parsed.metrics.source_tool, "workbuddy");
    assert_eq!(parsed.metrics.model_used, "glm-5.2");
    assert_eq!(parsed.metrics.tool_calls_total, 1);
    assert_eq!(parsed.metrics.tool_calls_ok, 1);
    assert_eq!(parsed.metrics.reasoning_blocks, 1);
    assert_eq!(parsed.metrics.tokens_input, 40);
    assert_eq!(parsed.metrics.tokens_output, 30);
    assert_eq!(parsed.metrics.tokens_cache_r, 80);
    assert_eq!(parsed.metrics.session_start, "2026-07-11T13:50:00Z");
    assert_eq!(parsed.metrics.session_end, "2026-07-11T13:50:04Z");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_antigravity_cli_transcript() {
    let root = temp_root("agenttrace-rust-antigravity");
    fs::create_dir_all(&root).expect("create antigravity temp dir");
    let path = root.join("transcript.jsonl");
    fs::write(
        &path,
        r#"{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","status":"DONE","created_at":"2026-05-19T19:33:40Z","content":"inspect"}
{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","status":"DONE","created_at":"2026-05-19T19:33:41Z","thinking":"check","tool_calls":[{"name":"view_file","args":{"AbsolutePath":"a.rs"}}]}
{"step_index":2,"source":"MODEL","type":"VIEW_FILE","status":"DONE","created_at":"2026-05-19T19:33:42Z","content":"ok"}
"#,
    )
    .expect("write antigravity transcript");

    let parsed = parse_file(&path).expect("parse antigravity transcript");
    assert_eq!(parsed.metrics.source_tool, "antigravity_cli");
    assert_eq!(parsed.metrics.tool_calls_total, 1);
    assert_eq!(parsed.metrics.tool_calls_ok, 1);
    assert_eq!(parsed.metrics.reasoning_blocks, 1);
    assert_eq!(parsed.metrics.session_start, "2026-05-19T19:33:40Z");
    assert_eq!(parsed.metrics.session_end, "2026-05-19T19:33:42Z");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_cursor_agent_transcript() {
    let root = temp_root("agenttrace-rust-cursor-transcript");
    fs::create_dir_all(&root).expect("create cursor temp dir");
    let path = root.join("session.jsonl");
    fs::write(
        &path,
        r#"{"role":"user","message":{"content":[{"type":"text","text":"inspect"}]}}
{"role":"assistant","message":{"content":[{"type":"text","text":"checking"},{"type":"tool_use","name":"Read","input":{"path":"a.rs"}}]}}
"#,
    )
    .expect("write cursor transcript");

    let parsed = parse_file(&path).expect("parse cursor transcript");
    assert_eq!(parsed.metrics.source_tool, "cursor");
    assert_eq!(parsed.metrics.tool_calls_total, 1);
    assert_eq!(parsed.metrics.user_messages, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_claude_flat_transcript() {
    let root = temp_root("agenttrace-rust-claude-transcript");
    fs::create_dir_all(&root).expect("create claude temp dir");
    let path = root.join("session.jsonl");
    fs::write(
        &path,
        r#"{"type":"user","timestamp":"2026-03-19T11:21:41Z","content":"inspect"}
{"type":"tool_use","timestamp":"2026-03-19T11:21:42Z","tool_name":"read","tool_input":{"path":"a.rs"}}
{"type":"tool_result","timestamp":"2026-03-19T11:21:43Z","tool_name":"read","tool_output":"ok"}
"#,
    )
    .expect("write claude transcript");

    let parsed = parse_file(&path).expect("parse claude transcript");
    assert_eq!(parsed.metrics.source_tool, "claude_code");
    assert_eq!(parsed.metrics.tool_calls_total, 1);
    assert_eq!(parsed.metrics.tool_calls_ok, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_copilot_session_state() {
    let root = temp_root("agenttrace-rust-copilot-session");
    fs::create_dir_all(&root).expect("create copilot temp dir");
    let path = root.join("events.jsonl");
    fs::write(
        &path,
        r#"{"type":"session.start","timestamp":"2026-05-07T10:00:00Z","data":{"context":{"cwd":"/tmp/copilot"}}}
{"type":"user.message","timestamp":"2026-05-07T10:00:01Z","data":{"content":"inspect"}}
{"type":"tool.execution_start","timestamp":"2026-05-07T10:00:02Z","data":{"toolName":"Read","toolCallId":"c1","arguments":{"path":"a.rs"}}}
{"type":"tool.execution_complete","timestamp":"2026-05-07T10:00:03Z","data":{"toolCallId":"c1","success":true}}
{"type":"session.shutdown","timestamp":"2026-05-07T10:00:04Z","data":{"modelMetrics":{"gpt-5.4":{"usage":{"inputTokens":100,"outputTokens":20,"cacheReadTokens":40}}}}}
"#,
    )
    .expect("write copilot session");

    let parsed = parse_file(&path).expect("parse copilot session");
    assert_eq!(parsed.cwd, "/tmp/copilot");
    assert_eq!(parsed.metrics.source_tool, "copilot_cli");
    assert_eq!(parsed.metrics.model_used, "gpt-5.4");
    assert_eq!(parsed.metrics.tool_calls_total, 1);
    assert_eq!(parsed.metrics.tool_calls_ok, 1);
    assert_eq!(parsed.metrics.tokens_input, 100);
    assert_eq!(parsed.metrics.tokens_cache_r, 40);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_kimi_wire_session() {
    let root = temp_root("agenttrace-rust-kimi-wire");
    fs::create_dir_all(&root).expect("create kimi temp dir");
    let path = root.join("wire.jsonl");
    fs::write(
        &path,
        r#"{"type":"metadata","protocol_version":"2"}
{"timestamp":1770000000.0,"message":{"type":"TurnBegin","payload":{"user_input":"inspect"}}}
{"timestamp":1770000001.0,"message":{"type":"ThinkPart","payload":{"text":"check"}}}
{"timestamp":1770000002.0,"message":{"type":"ToolCall","payload":{"id":"c1","name":"Read","arguments":{"path":"a.rs"}}}}
{"timestamp":1770000003.0,"message":{"type":"ToolResult","payload":{"id":"c1","result":"ok"}}}
{"timestamp":1770000004.0,"message":{"type":"StatusUpdate","payload":{"token_usage":{"inputTokens":100,"outputTokens":20,"cacheReadInputTokens":40}}}}
"#,
    )
    .expect("write kimi wire session");

    let parsed = parse_file(&path).expect("parse kimi wire session");
    assert_eq!(parsed.metrics.source_tool, "kimi_cli");
    assert_eq!(parsed.metrics.tool_calls_total, 1);
    assert_eq!(parsed.metrics.tool_calls_ok, 1);
    assert_eq!(parsed.metrics.reasoning_blocks, 1);
    assert_eq!(parsed.metrics.tokens_input, 100);
    assert_eq!(parsed.metrics.tokens_cache_r, 40);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_fallback_jsonl_matches_go_hermes_source_labeling() {
    let root = temp_root("agenttrace-rust-hermes-jsonl-source");
    fs::create_dir_all(&root).expect("create source-label temp dir");
    let session_path = root.join("session.jsonl");
    fs::write(
        &session_path,
        r#"{"role":"assistant","content":"I will inspect the file.","timestamp":"2026-05-02T10:40:00Z","tool_calls":[{"id":"bad-args-1","name":"read_file","args":"{\"path\":"}],"ModelUsed":"gpt-4.1","SourceTool":"generic"}
{"role":"tool","tool_call_id":"bad-args-1","content":"ok","timestamp":"2026-05-02T10:40:01Z","is_error":false,"SourceTool":"generic"}
"#,
    )
    .expect("write invalid args session");

    let parsed = parse_file(&session_path).expect("parse invalid args fixture");
    assert_eq!(parsed.metrics.source_tool, "hermes_jsonl");
    assert_eq!(parsed.tool_warnings.len(), 1);
    assert_eq!(parsed.tool_warnings[0].tool_name, "read_file");
    assert_eq!(parsed.tool_warnings[0].pattern, "invalid_args");

    let results = search_sessions(&[parsed], "malformed", 20);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_tool, "hermes_jsonl");
    assert!(results[0].matches.contains(
        &"tool warning: Tool 'read_file' had 1 call(s) with malformed arguments".to_string()
    ));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_generic_jsonl_accepts_go_style_event_field_names() {
    let root = temp_root("agenttrace-rust-go-style-jsonl");
    fs::create_dir_all(&root).expect("create go-style temp dir");
    let session_path = root.join("session.jsonl");
    fs::write(
        &session_path,
        r#"{"Role":"meta","Timestamp":"2026-05-02T10:00:00Z","ModelUsed":"claude-sonnet-4","Usage":{"input_tokens":100,"output_tokens":20}}
{"Role":"assistant","Content":"I will run the tool.","Timestamp":"2026-05-02T10:00:01Z","ToolCalls":[{"id":"t1","function":{"name":"run","arguments":"echo ok"},"type":"function"}],"ModelUsed":"claude-sonnet-4"}
{"Role":"tool","Content":"ok","ToolCallID":"t1","Timestamp":"2026-05-02T10:00:02Z","IsError":false}
"#,
    )
    .expect("write go-style jsonl session");

    let parsed = parse_file(&session_path).expect("parse go-style jsonl session");
    assert_eq!(parsed.metrics.source_tool, "hermes_jsonl");
    assert_eq!(parsed.metrics.model_used, "claude-sonnet-4");
    assert_eq!(parsed.metrics.tokens_input, 100);
    assert_eq!(parsed.metrics.tokens_output, 20);
    assert_eq!(parsed.metrics.tool_calls_total, 1);
    assert_eq!(parsed.metrics.tool_calls_ok, 1);
    assert_eq!(parsed.metrics.tool_usage.get("run"), Some(&1));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_single_codex_session_meta_json_object_falls_back_to_generic() {
    let root = temp_root("agenttrace-rust-single-codex-meta-generic");
    fs::create_dir_all(&root).expect("create codex meta temp dir");
    let session_path = root.join("meta-only.jsonl");
    fs::write(
        &session_path,
        r#"{"timestamp":"2026-05-03T10:00:00Z","type":"session_meta","payload":{"id":"s1","model_provider":"openai","source":"cli"}}"#,
    )
    .expect("write single meta session");

    let parsed = parse_file(&session_path).expect("parse single meta session");
    assert_eq!(parsed.metrics.source_tool, "generic");
    assert_eq!(parsed.metrics.events_total, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_generic_jsonl_accepts_legacy_type_events_without_role() {
    let root = temp_root("agenttrace-rust-legacy-generic-jsonl");
    fs::create_dir_all(&root).expect("create legacy generic temp dir");
    let session_path = root.join("legacy.jsonl");
    fs::write(
        &session_path,
        r#"{"kind":"session","sessionId":"s1","projectHash":"p","startTime":"2026-05-03T10:00:00Z","lastUpdated":"2026-05-03T10:00:01Z"}
{"type":"user","id":"u1","timestamp":"2026-05-03T10:00:00Z","content":["hello"]}
{"type":"assistant","id":"a1","timestamp":"2026-05-03T10:00:01Z","content":"hi"}
"#,
    )
    .expect("write legacy generic session");

    let parsed = parse_file(&session_path).expect("parse legacy generic session");
    assert_eq!(parsed.metrics.source_tool, "generic");
    assert_eq!(parsed.metrics.events_total, 2);
    assert_eq!(parsed.metrics.user_messages, 0);
    assert_eq!(parsed.metrics.assistant_turns, 0);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_messages_json_without_roles_falls_back_to_codex_cli() {
    let root = temp_root("agenttrace-rust-messages-without-roles");
    fs::create_dir_all(&root).expect("create messages temp dir");
    let session_path = root.join("messages.json");
    fs::write(
        &session_path,
        r#"{"kind":"session","sessionId":"s1","projectHash":"p","startTime":"2026-05-03T10:00:00Z","lastUpdated":"2026-05-03T10:00:01Z","messages":[{"type":"user","id":"u1","timestamp":"2026-05-03T10:00:00Z","content":"hello"},{"type":"assistant","id":"a1","timestamp":"2026-05-03T10:00:01Z","content":"hi"}]}"#,
    )
    .expect("write role-less messages session");

    let parsed = parse_file(&session_path).expect("parse role-less messages session");
    assert_eq!(parsed.metrics.source_tool, "codex_cli");
    assert_eq!(parsed.metrics.events_total, 2);
    assert_eq!(parsed.metrics.user_messages, 0);
    assert_eq!(parsed.metrics.assistant_turns, 0);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_writes_and_reuses_go_compatible_session_cache() {
    let root = temp_root("agenttrace-rust-session-cache");
    let home = root.join("home");
    let sessions_dir = home.join("sessions");
    let cache_dir = home.join("cache");
    fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    let session_path = sessions_dir.join("session.jsonl");
    fs::write(&session_path, SAMPLE_JSONL).expect("write session");

    with_home_and_cache(&home, &cache_dir, || {
        let sessions = load_sessions_from_dir(Some(&sessions_dir));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].metrics.source_tool, "hermes_jsonl");

        let cache_path = session_cache_path();
        assert_eq!(cache_path, cache_dir.join("sessions.json"));
        let raw = fs::read_to_string(&cache_path).expect("read written cache");
        let doc: Value = serde_json::from_str(&raw).expect("cache json");
        assert_eq!(
            doc.pointer("/schema_version").and_then(Value::as_i64),
            Some(17)
        );
        let entry = doc
            .pointer(&format!("/entries/{}", escape_json_pointer(&session_path)))
            .expect("cache entry");
        assert!(entry.get("mod_time").and_then(Value::as_i64).is_some());
        assert!(entry
            .pointer("/session/Metrics/Provenance/Tokens")
            .and_then(Value::as_str)
            .is_some());
        assert!(entry
            .pointer("/session/Metrics/Provenance/PricingSource")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty()));
        assert_eq!(
            entry.pointer("/session/Name").and_then(Value::as_str),
            Some("Inspect billing export.")
        );
        assert_eq!(
            entry
                .pointer("/session/Metrics/SourceTool")
                .and_then(Value::as_str),
            Some("hermes_jsonl")
        );
        assert_eq!(
            entry
                .pointer("/session/Metrics/ToolUsage/rg")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            entry
                .pointer("/session/Metrics/ToolArgUsage/billing export")
                .and_then(Value::as_u64),
            Some(1)
        );

        let doctor = build_doctor_report(Some(&sessions_dir), false);
        assert_eq!(doctor.cache_entries, 1);
        assert_eq!(doctor.cached_valid, 1);

        fs::write(&session_path, "not a session\n").expect("invalidate session cache entry");
        let after_stale = load_sessions_from_dir(Some(&sessions_dir));
        assert!(after_stale.is_empty());
        let doctor = build_doctor_report(Some(&sessions_dir), false);
        assert_eq!(doctor.cached_valid, 0);
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_reports_monotonic_load_progress_and_real_cache_hits() {
    let root = temp_root("agenttrace-rust-load-progress");
    let sessions_dir = root.join("sessions");
    let cache_dir = root.join("cache");
    fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    fs::write(sessions_dir.join("a.jsonl"), SAMPLE_JSONL).expect("write session a");
    fs::write(sessions_dir.join("b.jsonl"), SAMPLE_JSONL).expect("write session b");
    fs::write(sessions_dir.join("bad.jsonl"), "not a session\n").expect("write bad session");

    with_session_cache(&cache_dir, || {
        let mut first = Vec::new();
        let report =
            load_sessions_with_progress(Some(&sessions_dir), &LoadOptions::default(), |progress| {
                first.push(progress)
            });
        assert_eq!(report.discovered, 3);
        assert_eq!(report.parsed, 2);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.cache_hits, 0);
        assert_eq!(
            first.iter().map(|item| item.processed).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(first.last().map(|item| item.skipped), Some(1));

        let mut second = Vec::new();
        let report =
            load_sessions_with_progress(Some(&sessions_dir), &LoadOptions::default(), |progress| {
                second.push(progress)
            });
        assert_eq!(report.cache_hits, 2);
        assert_eq!(second.last().map(|item| item.cache_hits), Some(2));
        assert_eq!(
            second.iter().filter(|item| item.session.is_some()).count(),
            2
        );
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_skips_claude_workflow_definitions() {
    let root = temp_root("agenttrace-claude-workflows");
    let project = root.join(".claude/projects/demo/session");
    let workflows = project.join("workflows");
    fs::create_dir_all(&workflows).expect("create workflow directory");
    fs::write(project.join("session.jsonl"), SAMPLE_JSONL).expect("write session");
    fs::write(
        workflows.join("wf_demo.json"),
        r#"{"runId":"wf_demo","script":"x"}"#,
    )
    .expect("write workflow");

    let files = find_session_files(Some(&root.join(".claude/projects")));
    assert_eq!(files, vec![project.join("session.jsonl")]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_refreshes_cache_entries_from_old_schema_version() {
    let root = temp_root("agenttrace-rust-session-cache-old-schema");
    let home = root.join("home");
    let sessions_dir = home.join("sessions");
    let cache_dir = home.join("cache");
    fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    fs::create_dir_all(&cache_dir).expect("create cache dir");
    let session_path = sessions_dir.join("session.jsonl");
    fs::write(&session_path, SAMPLE_JSONL).expect("write session");
    let metadata = fs::metadata(&session_path).expect("session metadata");
    let cache_path = cache_dir.join("sessions.json");
    fs::write(
        &cache_path,
        format!(
            r#"{{"schema_version":3,"entries":{{"{}":{{"mod_time":{},"size":{},"session":{{"Name":"cached","Path":"{}","Metrics":{{"SourceTool":"stale","ModelUsed":"cached-model","SessionStart":"2026-05-02T09:00:00Z","ToolArgUsage":{{}}}},"Health":91,"ToolWarnings":[]}}}}}}}}"#,
            session_path.to_string_lossy(),
            file_mod_time_nanos_for_test(&metadata),
            metadata.len(),
            session_path.to_string_lossy()
        ),
    )
    .expect("write old schema cache");

    with_home_and_cache(&home, &cache_dir, || {
        let sessions = load_sessions_from_dir(Some(&sessions_dir));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "Inspect billing export.");
        assert_eq!(sessions[0].metrics.source_tool, "hermes_jsonl");

        let raw = fs::read_to_string(session_cache_path()).expect("read refreshed cache");
        let doc: Value = serde_json::from_str(&raw).expect("cache json");
        assert_eq!(
            doc.pointer("/schema_version").and_then(Value::as_i64),
            Some(17)
        );
        let entry = doc
            .pointer(&format!("/entries/{}", escape_json_pointer(&session_path)))
            .expect("cache entry");
        assert_eq!(
            entry
                .pointer("/session/Metrics/SourceTool")
                .and_then(Value::as_str),
            Some("hermes_jsonl")
        );
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_refreshes_cache_entries_missing_tool_arg_usage() {
    let root = temp_root("agenttrace-rust-session-cache-tool-args");
    let home = root.join("home");
    let sessions_dir = home.join("sessions");
    let cache_dir = home.join("cache");
    fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    fs::create_dir_all(&cache_dir).expect("create cache dir");
    let session_path = sessions_dir.join("session.jsonl");
    fs::write(&session_path, SAMPLE_JSONL).expect("write session");
    let metadata = fs::metadata(&session_path).expect("session metadata");
    let cache_path = cache_dir.join("sessions.json");
    fs::write(
        &cache_path,
        format!(
            r#"{{"entries":{{"{}":{{"mod_time":{},"size":{},"session":{{"Name":"cached","Path":"{}","Metrics":{{"SourceTool":"hermes_jsonl","ModelUsed":"cached-model","SessionStart":"2026-05-02T09:00:00Z","ToolUsage":{{"rg":1}},"FileUsage":{{"go test ./...":1}}}},"Health":91,"ToolWarnings":[]}}}}}}}}"#,
            session_path.to_string_lossy(),
            file_mod_time_nanos_for_test(&metadata),
            metadata.len(),
            session_path.to_string_lossy()
        ),
    )
    .expect("write old cache");

    with_home_and_cache(&home, &cache_dir, || {
        let sessions = load_sessions_from_dir(Some(&sessions_dir));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "Inspect billing export.");
        assert_eq!(
            sessions[0].metrics.tool_arg_usage.get("billing export"),
            Some(&1)
        );
        assert!(sessions[0].metrics.file_usage.is_empty());

        let raw = fs::read_to_string(session_cache_path()).expect("read refreshed cache");
        let doc: Value = serde_json::from_str(&raw).expect("cache json");
        let entry = doc
            .pointer(&format!("/entries/{}", escape_json_pointer(&session_path)))
            .expect("cache entry");
        assert_eq!(
            entry
                .pointer("/session/Metrics/ToolArgUsage/billing export")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert!(entry
            .pointer("/session/Metrics/FileUsage/go test ./...")
            .is_none());
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_refreshes_cache_entries_with_empty_source_tool() {
    let root = temp_root("agenttrace-rust-session-cache-empty-source");
    let home = root.join("home");
    let sessions_dir = home.join("sessions");
    let cache_dir = home.join("cache");
    fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    fs::create_dir_all(&cache_dir).expect("create cache dir");
    let session_path = sessions_dir.join("meta-only.jsonl");
    fs::write(
        &session_path,
        r#"{"timestamp":"2026-05-03T10:00:00Z","type":"session_meta","payload":{"id":"s1","model_provider":"openai","source":"cli"}}"#,
    )
    .expect("write single meta session");
    let metadata = fs::metadata(&session_path).expect("session metadata");
    let cache_path = cache_dir.join("sessions.json");
    fs::write(
        &cache_path,
        format!(
            r#"{{"entries":{{"{}":{{"mod_time":{},"size":{},"session":{{"Name":"cached","Path":"{}","Metrics":{{"SourceTool":"","ModelUsed":"cached-model","SessionStart":"2026-05-03T09:00:00Z","ToolArgUsage":{{}}}},"Health":91,"ToolWarnings":[]}}}}}}}}"#,
            session_path.to_string_lossy(),
            file_mod_time_nanos_for_test(&metadata),
            metadata.len(),
            session_path.to_string_lossy()
        ),
    )
    .expect("write old empty-source cache");

    with_home_and_cache(&home, &cache_dir, || {
        let sessions = load_sessions_from_dir(Some(&sessions_dir));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "meta-only");
        assert_eq!(sessions[0].metrics.source_tool, "generic");

        let raw = fs::read_to_string(session_cache_path()).expect("read refreshed cache");
        let doc: Value = serde_json::from_str(&raw).expect("cache json");
        let entry = doc
            .pointer(&format!("/entries/{}", escape_json_pointer(&session_path)))
            .expect("cache entry");
        assert_eq!(
            entry
                .pointer("/session/Metrics/SourceTool")
                .and_then(Value::as_str),
            Some("generic")
        );
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_writes_and_refreshes_go_compatible_directory_cache() {
    let root = temp_root("agenttrace-rust-dir-cache");
    let home = root.join("home");
    let sessions_dir = home.join("sessions");
    let nested = sessions_dir.join("nested");
    let cache_dir = home.join("cache");
    fs::create_dir_all(&nested).expect("create nested session dir");
    let first_path = nested.join("first.jsonl");
    fs::write(&first_path, SAMPLE_JSONL).expect("write first session");

    with_home_and_cache(&home, &cache_dir, || {
        let sessions = load_sessions_from_dir(Some(&sessions_dir));
        assert_eq!(sessions.len(), 1);

        let cache_path = session_cache_path();
        let raw = fs::read_to_string(&cache_path).expect("read written cache");
        let doc: Value = serde_json::from_str(&raw).expect("cache json");
        let root_dir = doc
            .pointer(&format!("/dirs/{}", escape_json_pointer(&sessions_dir)))
            .expect("root dir cache entry");
        let nested_dir = doc
            .pointer(&format!("/dirs/{}", escape_json_pointer(&nested)))
            .expect("nested dir cache entry");
        assert!(root_dir.get("mod_time").and_then(Value::as_i64).is_some());
        assert_eq!(
            root_dir.pointer("/dirs/0").and_then(Value::as_str),
            Some(nested.to_string_lossy().as_ref())
        );
        assert_eq!(
            nested_dir.pointer("/files/0").and_then(Value::as_str),
            Some(first_path.to_string_lossy().as_ref())
        );

        let doctor = build_doctor_report(Some(&sessions_dir), false);
        assert_eq!(doctor.cache_dirs, 2);

        let second_path = nested.join("second.jsonl");
        fs::write(&second_path, SAMPLE_JSONL).expect("write second session");
        bump_dir_mtime(&nested);
        bump_dir_mtime(&sessions_dir);
        let sessions = load_sessions_from_dir(Some(&sessions_dir));
        assert_eq!(sessions.len(), 2);
        assert!(sessions
            .iter()
            .any(|session| session.path == second_path.to_string_lossy()));

        fs::remove_file(&first_path).expect("remove stale cached session file");
        bump_dir_mtime(&nested);
        bump_dir_mtime(&sessions_dir);
        let sessions = load_sessions_from_dir(Some(&sessions_dir));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].path, second_path.to_string_lossy());
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_opencode_storage_session() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let session_path =
        repo_root.join("testdata/opencode/storage/session/project_alpha/ses_abc.json");

    let parsed = parse_file(&session_path).expect("parse OpenCode storage session");
    assert_eq!(parsed.metrics.source_tool, "opencode");
    assert_eq!(parsed.metrics.model_used, "claude-sonnet-4");
    assert_eq!(parsed.metrics.user_messages, 1);
    assert_eq!(parsed.metrics.assistant_turns, 2);
    assert_eq!(parsed.metrics.tool_calls_total, 1);
    assert_eq!(parsed.metrics.tool_calls_ok, 1);
    assert_eq!(parsed.metrics.tokens_input, 42);
    assert_eq!(parsed.metrics.tokens_output, 17);
    assert_eq!(parsed.metrics.tokens_cache_r, 3);
    assert_eq!(parsed.metrics.tokens_cache_w, 2);
}

#[test]
fn rust_discovers_only_opencode_storage_session_files_like_go() {
    let root = temp_root("agenttrace-rust-opencode-discovery");
    let home = root.join("home");
    let storage = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("storage");
    let session_dir = storage.join("session").join("project_alpha");
    let message_dir = storage.join("message").join("ses_abc");
    let part_dir = storage.join("part").join("msg_user");
    fs::create_dir_all(&session_dir).expect("create opencode session dir");
    fs::create_dir_all(&message_dir).expect("create opencode message dir");
    fs::create_dir_all(&part_dir).expect("create opencode part dir");
    let session_path = session_dir.join("ses_abc.json");
    let message_path = message_dir.join("msg_user.json");
    let part_path = part_dir.join("part_text.json");
    let raw = r#"{"id":"ses_abc","projectID":"project_alpha","time":{"created":1764750000000}}"#;
    fs::write(&session_path, raw).expect("write opencode session file");
    fs::write(&message_path, raw).expect("write opencode message file");
    fs::write(&part_path, raw).expect("write opencode part file");

    with_home(&home, || {
        let files = find_session_files(None);
        assert!(
            files.contains(&session_path),
            "missing session file: {files:?}"
        );
        assert!(
            !files.contains(&message_path) && !files.contains(&part_path),
            "message/part files should be skipped: {files:?}"
        );

        let files = find_session_files(Some(&storage));
        assert!(
            files.contains(&session_path),
            "missing session file: {files:?}"
        );
        assert!(
            !files.contains(&message_path) && !files.contains(&part_path),
            "message/part files should be skipped for custom storage: {files:?}"
        );
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_fixture_formats_used_by_compare_gate() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let cases = [
        (
            "testdata/codex-rollout-with-aider-text.jsonl",
            "codex_cli",
            "gpt-5.4",
            1,
            1,
            0,
            45,
        ),
        (
            "testdata/claude-code-preamble.jsonl",
            "claude_code",
            "claude-sonnet-4",
            1,
            1,
            1,
            175,
        ),
        (
            "testdata/copilot-attrs-map.jsonl",
            "copilot_cli",
            "gpt-4.1",
            0,
            2,
            1,
            160,
        ),
        (
            "testdata/kimi-tool-args.json",
            "kimi_cli",
            "kimi-k2.6",
            1,
            2,
            1,
            160,
        ),
    ];

    for (rel, source, model, users, turns, tools, tokens) in cases {
        let parsed = parse_file(&repo_root.join(rel)).unwrap_or_else(|err| panic!("{rel}: {err}"));
        assert_eq!(parsed.metrics.source_tool, source, "{rel}");
        assert_eq!(parsed.metrics.model_used, model, "{rel}");
        assert_eq!(parsed.metrics.user_messages, users, "{rel}");
        assert_eq!(parsed.metrics.assistant_turns, turns, "{rel}");
        assert_eq!(parsed.metrics.tool_calls_total, tools, "{rel}");
        assert_eq!(
            parsed.metrics.tokens_input
                + parsed.metrics.tokens_output
                + parsed.metrics.tokens_cache_r
                + parsed.metrics.tokens_cache_w,
            tokens,
            "{rel}"
        );
    }
}

#[test]
fn rust_codex_rollout_token_counts_use_turn_context_model() {
    let root = temp_root("agenttrace-rust-codex-token-counts");
    fs::create_dir_all(&root).expect("create codex temp dir");
    let session_path = root.join("rollout.jsonl");
    fs::write(
        &session_path,
        r#"{"timestamp":"2026-05-03T10:00:00Z","type":"session_meta","payload":{"model_provider":"openai"}}
{"timestamp":"2026-05-03T10:00:01Z","type":"turn_context","payload":{"model":"gpt-5.4"}}
{"timestamp":"2026-05-03T10:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":20,"total_tokens":1100},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":20,"total_tokens":1100}}}}
{"timestamp":"2026-05-03T10:00:03Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":20,"total_tokens":1100},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":20,"total_tokens":1100}}}}
{"timestamp":"2026-05-03T10:00:04Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1700,"cached_input_tokens":900,"output_tokens":160,"reasoning_output_tokens":30,"total_tokens":1860},"last_token_usage":{"input_tokens":700,"cached_input_tokens":500,"output_tokens":60,"reasoning_output_tokens":10,"total_tokens":760}}}}
{"timestamp":"2026-05-03T10:00:05Z","type":"response_item","payload":{"type":"function_call","call_id":"call_1","name":"shell","arguments":"{}"}}
{"timestamp":"2026-05-03T10:00:06Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call_1","output":"ok"}}
"#,
    )
    .expect("write codex rollout");

    let parsed = parse_file(&session_path).expect("parse codex rollout");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.model_used, "gpt-5.4");
    assert_eq!(metrics.source_tool, "codex_cli");
    assert_eq!(metrics.tokens_input, 800);
    assert_eq!(metrics.tokens_cache_r, 900);
    assert_eq!(metrics.tokens_output, 190);
    assert_eq!(metrics.tool_calls_total, 1);
    assert_eq!(metrics.tool_calls_ok, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_codex_rollout_prefers_cached_input_tokens_like_go() {
    let root = temp_root("agenttrace-rust-codex-cache-read-priority");
    fs::create_dir_all(&root).expect("create codex temp dir");
    let session_path = root.join("rollout.jsonl");
    fs::write(
        &session_path,
        r#"{"timestamp":"2026-05-03T10:00:00Z","type":"session_meta","payload":{"model":"gpt-5.5"}}
{"timestamp":"2026-05-03T10:00:01Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":1000,"cached_input_tokens":100,"cache_read_input_tokens":900,"output_tokens":10}}}}
{"timestamp":"2026-05-03T10:00:02Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"done"}]}}
"#,
    )
    .expect("write codex rollout");

    let parsed = parse_file(&session_path).expect("parse codex rollout");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "codex_cli");
    assert_eq!(metrics.tokens_input, 900);
    assert_eq!(metrics.tokens_cache_r, 100);
    assert_eq!(metrics.tokens_output, 10);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_codex_rollout_clamps_negative_uncached_input_like_go() {
    let root = temp_root("agenttrace-rust-codex-negative-input");
    fs::create_dir_all(&root).expect("create codex temp dir");
    let session_path = root.join("rollout.jsonl");
    fs::write(
        &session_path,
        r#"{"timestamp":"2026-05-03T10:00:00Z","type":"session_meta","payload":{"model":"gpt-5.5"}}
{"timestamp":"2026-05-03T10:00:01Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":100,"output_tokens":10}}}}
{"timestamp":"2026-05-03T10:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1100,"cached_input_tokens":500,"output_tokens":20}}}}
{"timestamp":"2026-05-03T10:00:03Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"done"}]}}
"#,
    )
    .expect("write codex rollout");

    let parsed = parse_file(&session_path).expect("parse codex rollout");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "codex_cli");
    assert_eq!(metrics.tokens_input, 900);
    assert_eq!(metrics.tokens_cache_r, 500);
    assert_eq!(metrics.tokens_output, 20);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_claude_code_jsonl_deduplicates_assistant_usage_snapshots() {
    let root = temp_root("agenttrace-rust-claude-usage-dedupe");
    fs::create_dir_all(&root).expect("create claude temp dir");
    let session_path = root.join("claude.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"assistant","timestamp":"2026-05-03T10:00:00Z","message":{"id":"msg_1","role":"assistant","model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":7,"cache_creation_input_tokens":3},"content":[{"type":"text","text":"hello"}]}}
{"type":"assistant","timestamp":"2026-05-03T10:00:01Z","message":{"id":"msg_1","role":"assistant","model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":7,"cache_creation_input_tokens":3},"content":[{"type":"tool_use","id":"tool_1","name":"Read","input":{}}]}}
"#,
    )
    .expect("write claude jsonl");

    let parsed = parse_file(&session_path).expect("parse claude jsonl");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "claude_code");
    assert_eq!(metrics.tokens_input, 100);
    assert_eq!(metrics.tokens_output, 10);
    assert_eq!(metrics.tokens_cache_r, 7);
    assert_eq!(metrics.tokens_cache_w, 3);
    assert_eq!(metrics.tool_calls_total, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_claude_code_jsonl_uses_body_cwd_like_go() {
    let root = temp_root("agenttrace-rust-claude-cwd");
    fs::create_dir_all(&root).expect("create claude temp dir");
    let session_path = root.join("claude.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"user","sessionId":"session-abc","cwd":"/real/worktree/alpha","timestamp":"2026-05-20T10:00:00Z","message":{"role":"user","content":"inspect cwd provenance"}}
{"type":"assistant","sessionId":"session-abc","timestamp":"2026-05-20T10:00:01Z","message":{"role":"assistant","model":"claude-sonnet-4","content":[{"type":"text","text":"ok"}]}}
"#,
    )
    .expect("write claude jsonl");

    let parsed = parse_file(&session_path).expect("parse claude jsonl");
    assert_eq!(parsed.cwd, "/real/worktree/alpha");
    assert_eq!(parsed.metrics.source_tool, "claude_code");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_claude_code_jsonl_keeps_first_non_unknown_model_like_go() {
    let root = temp_root("agenttrace-rust-claude-model-order");
    fs::create_dir_all(&root).expect("create claude temp dir");
    let session_path = root.join("claude.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"assistant","timestamp":"2026-05-03T10:00:00Z","message":{"id":"msg_1","role":"assistant","model":"glm-5.1","usage":{"input_tokens":100,"output_tokens":10},"content":[{"type":"text","text":"hello"}]}}
{"type":"assistant","timestamp":"2026-05-03T10:00:01Z","message":{"id":"msg_2","role":"assistant","model":"qwen3.7-max","usage":{"input_tokens":20,"output_tokens":5},"content":[{"type":"text","text":"again"}]}}
"#,
    )
    .expect("write claude jsonl");

    let parsed = parse_file(&session_path).expect("parse claude jsonl");
    assert_eq!(parsed.metrics.source_tool, "claude_code");
    assert_eq!(parsed.metrics.model_used, "glm-5.1");
    assert_eq!(parsed.metrics.tokens_input, 120);
    assert_eq!(parsed.metrics.tokens_output, 15);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_openclaw_anthropic_wrapper() {
    let root = temp_root("agenttrace-rust-openclaw");
    fs::create_dir_all(&root).expect("create openclaw temp dir");
    let session_path = root.join("openclaw.json");
    fs::write(
        &session_path,
        r#"{"provider":"openclaw","model":"claude-sonnet-4","usage":{"input_tokens":33,"output_tokens":12},"messages":[{"role":"user","content":"Inspect the parser.","timestamp":"2026-05-03T10:00:00Z"},{"role":"assistant","timestamp":"2026-05-03T10:00:01Z","content":[{"type":"thinking","thinking":"Check provider detection first."},{"type":"text","text":"I will inspect it."},{"type":"tool_use","id":"tc1","name":"read_file","input":{"path":"src/main.rs"}}]},{"role":"tool","tool_call_id":"tc1","content":"done","timestamp":"2026-05-03T10:00:02Z","is_error":false}]}"#,
    )
    .expect("write openclaw session");

    let parsed = parse_file(&session_path).expect("parse openclaw session");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "openclaw");
    assert_eq!(metrics.model_used, "claude-sonnet-4");
    assert_eq!(metrics.user_messages, 1);
    assert_eq!(metrics.assistant_turns, 3);
    assert_eq!(metrics.tool_calls_total, 1);
    assert_eq!(metrics.tool_calls_ok, 1);
    assert_eq!(metrics.tokens_input, 33);
    assert_eq!(metrics.tokens_output, 12);
    assert_eq!(metrics.reasoning_blocks, 1);
    assert_eq!(metrics.tool_usage.get("read_file"), Some(&1));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_hermes_json_session() {
    let root = temp_root("agenttrace-rust-hermes-json");
    fs::create_dir_all(&root).expect("create hermes json temp dir");
    let session_path = root.join("hermes.json");
    fs::write(
        &session_path,
        r#"{"session_id":"s1","platform":"darwin","model":"claude-sonnet-4","usage":{"input_tokens":100,"output_tokens":200},"messages":[{"role":"user","content":"hello","timestamp":"2026-01-01T00:00:00Z"},{"role":"assistant","content":"","timestamp":"2026-01-01T00:00:01Z","tool_calls":[{"id":"tc1","function":{"name":"read_file","arguments":{"path":"README.md"}}}]},{"role":"tool","content":"ok","tool_call_id":"tc1","timestamp":"2026-01-01T00:00:02Z","is_error":false}]}"#,
    )
    .expect("write hermes json session");

    let parsed = parse_file(&session_path).expect("parse hermes json session");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "hermes_json");
    assert_eq!(metrics.model_used, "claude-sonnet-4");
    assert_eq!(metrics.user_messages, 1);
    assert_eq!(metrics.assistant_turns, 1);
    assert_eq!(metrics.tool_calls_total, 1);
    assert_eq!(metrics.tool_calls_ok, 1);
    assert_eq!(metrics.tokens_input, 100);
    assert_eq!(metrics.tokens_output, 200);
    assert_eq!(metrics.session_start, "2026-01-01T00:00:00Z");
    assert_eq!(metrics.session_end, "2026-01-01T00:00:02Z");
    assert_eq!(metrics.tool_usage.get("read_file"), Some(&1));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_hermes_json_session_timestamps() {
    let root = temp_root("agenttrace-rust-hermes-json-session-ts");
    fs::create_dir_all(&root).expect("create hermes json temp dir");
    let session_path = root.join("hermes-session-times.json");
    fs::write(
        &session_path,
        r#"{"session_id":"s2","model":"claude-sonnet-4","session_start":"2026-01-02T00:00:00Z","last_updated":"2026-01-02T00:00:05Z","messages":[{"role":"user","content":"hello"},{"role":"assistant","content":"done"}]}"#,
    )
    .expect("write hermes json session");

    let parsed = parse_file(&session_path).expect("parse hermes json session times");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "hermes_json");
    assert_eq!(metrics.model_used, "claude-sonnet-4");
    assert_eq!(metrics.session_start, "2026-01-02T00:00:00Z");
    assert_eq!(metrics.session_end, "2026-01-02T00:00:05Z");
    assert_eq!(metrics.duration_sec, 5.0);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_aider_chat_history() {
    let root = temp_root("agenttrace-rust-aider-history");
    fs::create_dir_all(&root).expect("create aider temp dir");
    let session_path = root.join(".aider.chat.history.md");
    fs::write(
        &session_path,
        r#"# aider chat started at 2026-05-02 10:00:00

> aider --model gpt-5.4

#### Fix parser detection

I will keep embedded Aider text from stealing JSONL formats.

> Tokens: 1.2k sent, 300 cache write, 400 cache hit, 345 received

#### Continue with tests

Added focused parser tests.
"#,
    )
    .expect("write aider history");

    let files = find_session_files(Some(&root));
    assert_eq!(files, vec![session_path.clone()]);

    let parsed = parse_file(&session_path).expect("parse aider history");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "aider");
    assert_eq!(metrics.model_used, "gpt-5.4");
    assert_eq!(metrics.user_messages, 2);
    assert_eq!(metrics.assistant_turns, 2);
    assert_eq!(metrics.tokens_input, 1200);
    assert_eq!(metrics.tokens_output, 345);
    assert_eq!(metrics.tokens_cache_w, 300);
    assert_eq!(metrics.tokens_cache_r, 400);
    assert!(!metrics.session_start.is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_rejects_empty_aider_chat_history() {
    let root = temp_root("agenttrace-rust-aider-empty");
    fs::create_dir_all(&root).expect("create aider temp dir");
    let session_path = root.join(".aider.chat.history.md");
    fs::write(&session_path, "").expect("write empty aider history");

    let err = parse_file(&session_path).expect_err("empty aider history should fail");
    assert!(err
        .to_string()
        .contains("aider chat history: no parseable events"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_oh_my_pi_session_jsonl() {
    let root = temp_root("agenttrace-rust-oh-my-pi");
    fs::create_dir_all(&root).expect("create oh-my-pi temp dir");
    let session_path = root.join("session.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"session","version":3,"id":"1f9d2a6b9c0d1234","timestamp":"2026-02-16T10:20:30.000Z","cwd":"/work/pi"}
{"type":"message","id":"u1","parentId":null,"timestamp":"2026-02-16T10:21:00.000Z","message":{"role":"user","content":[{"type":"text","text":"Inspect the failing test"}],"timestamp":1771237260000}}
{"type":"message","id":"a1","parentId":"u1","timestamp":"2026-02-16T10:21:10.000Z","message":{"role":"assistant","provider":"anthropic","model":"claude-sonnet-4-5","content":[{"type":"thinking","thinking":"Need to inspect logs first."},{"type":"text","text":"I will inspect the failure."},{"type":"toolCall","id":"tc1","name":"read","arguments":{"path":"go.mod"}},{"type":"toolCall","id":"tc2","name":"read","arguments":{"path":"README.md"}}],"usage":{"input":100,"output":20,"cacheRead":7,"cacheWrite":3},"timestamp":1771237270000}}
{"type":"message","id":"t1","parentId":"a1","timestamp":"2026-02-16T10:21:12.000Z","message":{"role":"toolResult","toolCallId":"tc1","toolName":"read","content":[{"type":"text","text":"module github.com/luoyuctl/agenttrace"}],"isError":false,"timestamp":1771237272000}}
{"type":"message","id":"t2","parentId":"a1","timestamp":"2026-02-16T10:21:13.000Z","message":{"role":"toolResult","toolCallId":"tc2","toolName":"read","content":[{"type":"text","text":"lenient surrogate \ud83c result"}],"isError":false,"timestamp":1771237273000}}
"#,
    )
    .expect("write oh-my-pi session");

    let parsed = parse_file(&session_path).expect("parse oh-my-pi session");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "oh_my_pi");
    assert_eq!(metrics.model_used, "claude-sonnet-4-5");
    assert_eq!(metrics.user_messages, 1);
    assert_eq!(metrics.assistant_turns, 1);
    assert_eq!(metrics.tool_calls_total, 2);
    assert_eq!(metrics.tool_calls_ok, 2);
    assert_eq!(metrics.tokens_input, 100);
    assert_eq!(metrics.tokens_output, 20);
    assert_eq!(metrics.tokens_cache_r, 7);
    assert_eq!(metrics.tokens_cache_w, 3);
    assert_eq!(metrics.reasoning_blocks, 1);
    assert_eq!(metrics.tool_usage.get("read"), Some(&2));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_uses_pi_source_for_pi_session_path() {
    let root = temp_root("agenttrace-rust-pi-source");
    let session_dir = root.join(".pi").join("agent").join("sessions");
    fs::create_dir_all(&session_dir).expect("create pi session dir");
    let session_path = session_dir.join("session.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"session","version":3,"id":"pi-session","cwd":"/work/pi"}
{"type":"message","message":{"role":"user","content":"hello from pi"}}
"#,
    )
    .expect("write pi session");

    let parsed = parse_file(&session_path).expect("parse pi session");
    assert_eq!(parsed.metrics.source_tool, "pi");
    assert_eq!(parsed.metrics.user_messages, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_rejects_oh_my_pi_session_with_invalid_header() {
    let root = temp_root("agenttrace-rust-oh-my-pi-invalid");
    fs::create_dir_all(&root).expect("create oh-my-pi temp dir");
    let session_path = root.join("broken.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"session","version":3,"cwd":"/work/pi"}
{"type":"message","message":{"role":"user","content":"hello"}}
"#,
    )
    .expect("write invalid oh-my-pi session");

    let err = parse_file(&session_path).expect_err("invalid oh-my-pi header should fail");
    assert!(err.to_string().contains("oh_my_pi: invalid session header"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_discovers_pi_session_files() {
    let root = temp_root("agenttrace-rust-pi-discovery");
    let home = root.join("home");
    let session_dir = home.join(".pi").join("agent").join("sessions");
    let cache_dir = home.join("cache");
    fs::create_dir_all(&session_dir).expect("create pi session dir");
    let session_path = session_dir.join("session.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"session","version":3,"id":"pi-session","cwd":"/work/pi"}
{"type":"message","message":{"role":"user","content":"hello from pi"}}
"#,
    )
    .expect("write pi session");

    with_home_and_cache(&home, &cache_dir, || {
        let files = find_session_files(None);
        assert_eq!(files, vec![session_path.clone()]);

        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].metrics.source_tool, "pi");
        let raw = fs::read_to_string(session_cache_path()).expect("read cache");
        let doc: Value = serde_json::from_str(&raw).expect("cache json");
        assert!(doc
            .pointer(&format!("/dirs/{}", escape_json_pointer(&session_dir)))
            .is_some());
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_qwen_code_stream_jsonl() {
    let root = temp_root("agenttrace-rust-qwen-stream");
    fs::create_dir_all(&root).expect("create qwen temp dir");
    let session_path = root.join("qwen-stream.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"system","subtype":"session_start","uuid":"sys-1","session_id":"session-1","model":"qwen3-coder-plus","timestamp":"2026-05-03T10:00:00Z"}
{"type":"assistant","uuid":"assistant-1","session_id":"session-1","timestamp":"2026-05-03T10:00:02Z","message":{"id":"msg-1","type":"message","role":"assistant","model":"qwen3-coder-plus","content":[{"type":"reasoning","text":"Need to inspect package files."},{"type":"text","text":"I'll inspect the package."},{"type":"tool_use","id":"tool-1","name":"read_file","input":{"path":"package.json"}}],"usage":{"input_tokens":120,"output_tokens":45,"cache_read_input_tokens":10,"cache_creation_input_tokens":5}}}
{"type":"user","uuid":"user-1","session_id":"session-1","timestamp":"2026-05-03T10:00:03Z","message":{"role":"user","content":"Please continue after inspecting it."}}
{"type":"user","uuid":"tool-result-1","session_id":"session-1","timestamp":"2026-05-03T10:00:04Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tool-1","content":"package metadata","is_error":false}]}}
{"type":"result","subtype":"success","uuid":"result-1","session_id":"session-1","is_error":false,"duration_ms":1234,"result":"I'll inspect the package.","usage":{"input_tokens":120,"output_tokens":45}}
"#,
    )
    .expect("write qwen stream");

    let parsed = parse_file(&session_path).expect("parse qwen stream");
    let metrics = &parsed.metrics;
    assert_eq!(metrics.source_tool, "qwen_code");
    assert_eq!(metrics.model_used, "qwen3-coder-plus");
    assert_eq!(metrics.user_messages, 1);
    assert_eq!(metrics.assistant_turns, 1);
    assert_eq!(metrics.tool_calls_total, 1);
    assert_eq!(metrics.tool_calls_ok, 1);
    assert_eq!(metrics.tokens_input, 120);
    assert_eq!(metrics.tokens_output, 45);
    assert_eq!(metrics.tokens_cache_r, 10);
    assert_eq!(metrics.tokens_cache_w, 5);
    assert_eq!(metrics.reasoning_blocks, 1);
    assert_eq!(metrics.tool_usage.get("read_file"), Some(&1));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_qwen_code_json_output_array() {
    let root = temp_root("agenttrace-rust-qwen-array");
    fs::create_dir_all(&root).expect("create qwen temp dir");
    let session_path = root.join("qwen-output.json");
    fs::write(
        &session_path,
        r#"[{"type":"system","subtype":"session_start","uuid":"sys-1","session_id":"session-1","model":"qwen3-coder-plus"},{"type":"result","subtype":"success","uuid":"result-1","session_id":"session-1","is_error":false,"result":"The capital of France is Paris.","stats":{"models":{"qwen3-coder-plus":{"tokens":{"input":20,"output":7}}}}}]"#,
    )
    .expect("write qwen array");

    let parsed = parse_file(&session_path).expect("parse qwen array");
    assert_eq!(parsed.metrics.source_tool, "qwen_code");
    assert_eq!(parsed.metrics.assistant_turns, 1);
    assert_eq!(parsed.metrics.tokens_input, 20);
    assert_eq!(parsed.metrics.tokens_output, 7);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_parses_qwen_code_json_object_output() {
    let root = temp_root("agenttrace-rust-qwen-object");
    fs::create_dir_all(&root).expect("create qwen temp dir");
    let session_path = root.join("qwen-result.json");
    fs::write(
        &session_path,
        r#"{"response":"Done.","stats":{"models":{"qwen3-coder-plus":{"tokens":{"input":31,"output":9,"cacheRead":4}}}}}"#,
    )
    .expect("write qwen object");

    let parsed = parse_file(&session_path).expect("parse qwen object");
    assert_eq!(parsed.metrics.source_tool, "qwen_code");
    assert_eq!(parsed.metrics.assistant_turns, 1);
    assert_eq!(parsed.metrics.tokens_input, 31);
    assert_eq!(parsed.metrics.tokens_output, 9);
    assert_eq!(parsed.metrics.tokens_cache_r, 4);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_rejects_qwen_code_stream_without_messages() {
    let root = temp_root("agenttrace-rust-qwen-empty");
    fs::create_dir_all(&root).expect("create qwen temp dir");
    let session_path = root.join("empty-qwen.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"system","subtype":"session_start","uuid":"sys-1","session_id":"session-1","model":"qwen3-coder-plus"}
"#,
    )
    .expect("write empty qwen stream");

    let err = parse_file(&session_path).expect_err("empty qwen stream should fail");
    assert!(err.to_string().contains("qwen_code: no parseable events"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_discovers_qwen_project_chat_files() {
    let root = temp_root("agenttrace-rust-qwen-discovery");
    let home = root.join("home");
    let chat_dir = home
        .join(".qwen")
        .join("projects")
        .join("repo")
        .join("chats");
    let cache_dir = home.join("cache");
    fs::create_dir_all(&chat_dir).expect("create qwen chat dir");
    let session_path = chat_dir.join("chat.jsonl");
    fs::write(
        &session_path,
        r#"{"type":"result","subtype":"success","uuid":"result-1","session_id":"session-1","result":"Done.","usage":{"input_tokens":2,"output_tokens":1}}
"#,
    )
    .expect("write qwen chat file");

    with_home_and_cache(&home, &cache_dir, || {
        let files = find_session_files(None);
        assert_eq!(files, vec![session_path.clone()]);

        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].metrics.source_tool, "qwen_code");
        let raw = fs::read_to_string(session_cache_path()).expect("read cache");
        let doc: Value = serde_json::from_str(&raw).expect("cache json");
        assert!(doc
            .pointer(&format!("/dirs/{}", escape_json_pointer(&chat_dir)))
            .is_some());
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_renders_waste_report_for_testdata_latest_slice() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let cache_root = temp_root("agenttrace-rust-report-cache");
    with_session_cache(&cache_root.join("cache"), || {
        let sessions = load_sessions_from_dir(Some(&repo_root.join("testdata")));
        let latest = sessions
            .iter()
            .max_by(|a, b| {
                a.metrics
                    .session_start
                    .cmp(&b.metrics.session_start)
                    .then_with(|| a.name.cmp(&b.name))
            })
            .expect("latest session");

        let report = render_waste_report(latest);
        assert!(report.contains("AGENTTRACE v"));
        assert!(report.contains("Score: 22/100"));
        assert!(report.contains("minor waste - cache 0% hit"));
        assert!(report.contains("caching not enabled"));
    });
    let _ = fs::remove_dir_all(cache_root);
}

#[test]
fn default_discovery_uses_hermes_sqlite_when_present() {
    let root = temp_root("agenttrace-rust-sqlite-hermes");
    let home = root.join("home");
    let legacy_dir = home.join(".hermes").join("sessions");
    fs::create_dir_all(&legacy_dir).expect("create legacy dir");
    fs::write(legacy_dir.join("legacy.jsonl"), SAMPLE_JSONL).expect("write legacy session");
    write_hermes_state_db(&home.join(".hermes").join("state.db"));

    with_home(&home, || {
        let files = find_session_files(None);
        assert_eq!(files, vec![legacy_dir.join("legacy.jsonl")]);

        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1);
        let metrics = &sessions[0].metrics;
        assert_eq!(metrics.source_tool, "hermes_db");
        assert_eq!(metrics.model_used, "gpt-5.1");
        assert_eq!(metrics.user_messages, 1);
        assert_eq!(metrics.assistant_turns, 1);
        assert_eq!(metrics.tool_calls_total, 1);
        assert_eq!(metrics.tokens_input, 1000);
        assert_eq!(metrics.tokens_output, 200);
        assert_eq!(metrics.tokens_cache_r, 50);
        assert_eq!(metrics.tokens_cache_w, 25);
        assert_eq!(metrics.provenance.tokens, "reported_by_agent");
        assert_eq!(metrics.provenance.duration, "timestamp_span");
        assert_eq!(metrics.provenance.tool_results, "unavailable");
    });

    let _ = fs::remove_dir_all(root);
}

#[test]
fn default_discovery_uses_opencode_sqlite_when_present() {
    let root = temp_root("agenttrace-rust-sqlite-opencode");
    let home = root.join("home");
    let storage = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("storage");
    let session_dir = storage.join("session").join("project_alpha");
    fs::create_dir_all(&session_dir).expect("create storage session dir");
    let legacy_session = session_dir.join("ses_abc.json");
    fs::write(
        &legacy_session,
        r#"{"id":"ses_abc","projectID":"project_alpha"}"#,
    )
    .expect("write legacy opencode storage session");
    write_opencode_db(
        &home
            .join(".local")
            .join("share")
            .join("opencode")
            .join("opencode.db"),
    );

    with_home(&home, || {
        let files = find_session_files(None);
        assert_eq!(files, vec![legacy_session]);

        let sessions = load_sessions_from_dir(None);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "Parser DB");
        let metrics = &sessions[0].metrics;
        assert_eq!(metrics.source_tool, "opencode_db");
        assert_eq!(metrics.model_used, "claude-sonnet-4");
        assert_eq!(metrics.user_messages, 1);
        assert_eq!(metrics.assistant_turns, 1);
        assert_eq!(metrics.tool_calls_total, 1);
        assert_eq!(metrics.tool_calls_ok, 1);
        assert_eq!(metrics.tokens_input, 42);
        assert_eq!(metrics.tokens_output, 22);
        assert_eq!(metrics.tokens_cache_r, 3);
        assert_eq!(metrics.tokens_cache_w, 2);
        assert_eq!(metrics.provenance.tokens, "reported_by_agent");
        assert_eq!(metrics.provenance.duration, "timestamp_span");
        assert_eq!(metrics.provenance.tool_results, "unavailable");
    });

    let _ = fs::remove_dir_all(root);
}

fn temp_root(prefix: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

fn with_home(home: &std::path::Path, f: impl FnOnce()) {
    with_home_and_cache(home, &home.join("cache"), f);
}

fn with_session_cache(cache: &std::path::Path, f: impl FnOnce()) {
    // The guard is scoped so it is released before any caught panic is
    // resumed; resuming the unwind while still holding the lock used to
    // poison the mutex and cascade one flaky failure into every later
    // env-isolated test in the binary.
    let result = {
        let _guard = lock_env();
        let previous_session_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", cache);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_session_cache);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn with_home_and_cache(home: &std::path::Path, cache: &std::path::Path, f: impl FnOnce()) {
    // See with_session_cache: the guard must be dropped before the caught
    // panic is resumed, or the poisoned mutex fails every later test that
    // needs the env lock (observed as 10 simultaneous "failures" from a
    // single flaky assertion).
    let result = {
        let _guard = lock_env();
        let previous_home = std::env::var_os("HOME");
        let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
        let previous_session_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", cache);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore_env("HOME", previous_home);
        restore_env("XDG_CONFIG_HOME", previous_xdg_config);
        restore_env("XDG_CACHE_HOME", previous_xdg_cache);
        restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_session_cache);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn escape_json_pointer(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('~', "~0").replace('/', "~1")
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    if let Some(value) = previous {
        std::env::set_var(key, value);
    } else {
        std::env::remove_var(key);
    }
}

fn bump_dir_mtime(path: &std::path::Path) {
    // Directory mtimes advance only once per timestamp tick, and on
    // coarse-granularity filesystems (this /tmp is ext2/ext3) a quick
    // write+remove usually lands inside the same tick, leaving the mtime
    // unchanged and the directory cache stale. Keep bumping until the
    // mtime actually moves so cache-invalidation tests are deterministic.
    let before =
        file_mod_time_nanos_for_test(&fs::metadata(path).expect("dir metadata before bump"));
    let marker = path.join(format!(".agenttrace-mtime-{}", std::process::id()));
    for _ in 0..400 {
        fs::write(&marker, b"x").expect("write mtime marker");
        fs::remove_file(&marker).expect("remove mtime marker");
        let after =
            file_mod_time_nanos_for_test(&fs::metadata(path).expect("dir metadata after bump"));
        if after != before {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!(
        "directory mtime did not advance on this filesystem: {}",
        path.display()
    );
}

fn file_mod_time_nanos_for_test(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .expect("file modified time")
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("modified time after unix epoch")
        .as_nanos() as i64
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Acquire the shared env lock. Poisoning is tolerated on purpose: a
/// poisoned mutex must fail only the test that panicked, not every test
/// that still needs the lock (see with_home_and_cache).
fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    match env_lock().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_hermes_state_db(path: &std::path::Path) {
    fs::create_dir_all(path.parent().expect("db parent")).expect("create db parent");
    let db = Connection::open(path).expect("open hermes db");
    db.execute_batch(
        r#"
        create table sessions (
            id text primary key,
            model text,
            started_at real,
            ended_at real,
            message_count integer,
            tool_call_count integer,
            input_tokens integer,
            output_tokens integer,
            cache_read_tokens integer,
            cache_write_tokens integer
        );
        create table messages (session_id text, role text);
        insert into sessions values ('db-session', 'gpt-5.1', 1760000000, 1760000060, 2, 1, 1000, 200, 50, 25);
        insert into messages values ('db-session', 'user'), ('db-session', 'assistant');
        "#,
    )
    .expect("seed hermes db");
}

fn write_opencode_db(path: &std::path::Path) {
    fs::create_dir_all(path.parent().expect("db parent")).expect("create db parent");
    let db = Connection::open(path).expect("open opencode db");
    db.execute_batch(
        r#"
        create table session (
            id text primary key,
            title text,
            time_created integer,
            time_updated integer
        );
        create table message (session_id text, data text);
        create table part (session_id text, data text);
        insert into session values ('ses_abc', 'Parser DB', 1764750000000, 1764750004000);
        insert into message values ('ses_abc', '{"id":"msg_user","role":"user"}');
        insert into message values ('ses_abc', '{"id":"msg_assistant","role":"assistant","modelID":"claude-sonnet-4","tokens":{"input":42,"output":17,"reasoning":5,"cache":{"read":3,"write":2}}}');
        insert into part values ('ses_abc', '{"type":"tool","state":{"status":"completed"}}');
        "#,
    )
    .expect("seed opencode db");
}

#[test]
fn data_health_discovered_is_range_independent_and_splits_out_of_scope() {
    // Pass-8 F8-2: the CLI used to recompute `discovered` as
    // sessions.len() + skipped, so `--overview --range 1d` reported
    // discovered=71 while 1,400+ files existed and parse failures in
    // out-of-range files were invisible. `discovered` must come from
    // the loader (range-independent) and ranged runs must separate
    // parsed from out-of-scope sessions.
    let root = temp_root("agenttrace-range-health");
    fs::create_dir_all(&root).expect("create range-health dir");
    let recent = r#"{"role":"session_meta","timestamp":"2026-09-01T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"user","content":"recent work","timestamp":"2026-09-01T10:00:00Z"}
{"role":"assistant","content":"done","timestamp":"2026-09-01T10:00:01Z"}
"#;
    let old = r#"{"role":"session_meta","timestamp":"2020-01-02T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"user","content":"ancient work","timestamp":"2020-01-02T10:00:00Z"}
{"role":"assistant","content":"done","timestamp":"2020-01-02T10:00:01Z"}
"#;
    fs::write(root.join("recent.jsonl"), recent).expect("write recent session");
    fs::write(root.join("old.jsonl"), old).expect("write old session");
    with_session_cache(&root.join("cache"), || {
        let all = agenttrace_core::load_sessions_with_options(
            Some(&root),
            &LoadOptions {
                since: None,
                ..LoadOptions::default()
            },
        );
        let day = agenttrace_core::load_sessions_with_options(
            Some(&root),
            &LoadOptions {
                since: Some(chrono::Utc::now() - chrono::Duration::days(30)),
                ..LoadOptions::default()
            },
        );
        assert_eq!(all.discovered, 2);
        assert_eq!(
            day.discovered, all.discovered,
            "discovered must be range-independent"
        );
        let health_all =
            data_health_scoped(&all.sessions, all.discovered, all.skipped, all.cache_hits);
        let health_day =
            data_health_scoped(&day.sessions, day.discovered, day.skipped, day.cache_hits);
        assert_eq!(health_all.parsed, 2);
        assert_eq!(
            health_all.out_of_scope, 0,
            "no filter excludes anything for all-time"
        );
        assert_eq!(health_day.parsed, 1, "only the recent session is in scope");
        assert_eq!(
            health_day.out_of_scope, 1,
            "the out-of-range session must be counted, not erased from the denominator"
        );
        assert_eq!(health_day.skipped, 0, "out-of-range is not a parse failure");
    });
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cache_dead_paths_are_persisted_away_across_runs() {
    // Pass-8 F8-3 end to end: after the source file disappears, the
    // next cached load prunes the entry and the re-saved snapshot no
    // longer carries the dead path (the operator snapshot had 761 of
    // 1,487 entries dead and growing every day).
    let root = temp_root("agenttrace-cache-prune");
    let sessions = root.join("sessions");
    fs::create_dir_all(&sessions).expect("create sessions dir");
    let body = r#"{"role":"session_meta","timestamp":"2026-09-01T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"user","content":"work","timestamp":"2026-09-01T10:00:00Z"}
{"role":"assistant","content":"done","timestamp":"2026-09-01T10:00:01Z"}
"#;
    let keep = sessions.join("keep.jsonl");
    let drop_path = sessions.join("drop.jsonl");
    fs::write(&keep, body).expect("write keep");
    fs::write(&drop_path, body).expect("write drop");
    with_session_cache(&root.join("cache"), || {
        let first = load_sessions_with_options(Some(&sessions), &LoadOptions::default());
        assert_eq!(first.sessions.len(), 2, "both sessions parse");
        let snapshot = root.join("cache").join("sessions.json");
        assert!(snapshot.exists(), "cache snapshot exists after first run");
        let before = fs::read_to_string(&snapshot).expect("read snapshot");
        assert!(
            before.contains("drop.jsonl"),
            "second session is cached by path"
        );

        fs::remove_file(&drop_path).expect("delete source file");
        let second = load_sessions_with_options(Some(&sessions), &LoadOptions::default());
        assert_eq!(second.sessions.len(), 1, "only the surviving file parses");
        let after = fs::read_to_string(&snapshot).expect("re-read snapshot");
        assert!(
            !after.contains("drop.jsonl"),
            "dead entry is pruned from the persisted snapshot"
        );
        assert!(after.contains("keep.jsonl"), "live entry stays cached");
    });
    let _ = fs::remove_dir_all(root);
}

#[test]
fn non_finite_costs_lower_health_confidence_and_stay_visible() {
    // Pass-8 F8-5: a session whose estimated cost is not finite must
    // surface as `non_finite_costs` and drop confidence to low instead
    // of silently joining a total or panicking in the writer.
    let root = temp_root("agenttrace-nonfinite-health");
    fs::create_dir_all(&root).expect("create dir");
    fs::write(
        root.join("poisoned.jsonl"),
        r#"{"role":"session_meta","timestamp":"2026-09-01T10:00:00Z","ModelUsed":"claude-sonnet-4"}
{"role":"user","content":"work","timestamp":"2026-09-01T10:00:00Z"}
{"role":"assistant","content":"done","timestamp":"2026-09-01T10:00:01Z"}
"#,
    )
    .expect("write session");
    with_session_cache(&root.join("cache"), || {
        let report = load_sessions_with_options(Some(&root), &LoadOptions::default());
        let session = &report.sessions[0];
        let mut poisoned = session.clone();
        poisoned.metrics.cost_estimated = f64::INFINITY;
        let health = data_health_scoped(
            &[poisoned],
            report.discovered,
            report.skipped,
            report.cache_hits,
        );
        assert_eq!(health.non_finite_costs, 1, "non-finite cost is counted");
        assert_eq!(health.confidence, "low", "corrupted costs lower confidence");
        let clean = data_health_scoped(
            &report.sessions,
            report.discovered,
            report.skipped,
            report.cache_hits,
        );
        assert_eq!(clean.non_finite_costs, 0);
    });
    let _ = fs::remove_dir_all(root);
}

#[test]
fn zstd_rollouts_fail_with_a_named_error_not_generic_utf8() {
    // Pass-7 research / candidate-44 minimum (CU-16): Codex >=0.152
    // writes rollouts as zstd frames (magic 28 B5 2F FD). The parser
    // must name the format instead of the old misleading "not valid
    // UTF-8" — this is the smallest step toward Codex coverage.
    let root = temp_root("agenttrace-zstd-sniff");
    fs::create_dir_all(&root).expect("create dir");
    let rollout = root.join("rollout-2026-09-03.jsonl");
    fs::write(&rollout, [0x28u8, 0xB5, 0x2F, 0xFD, 0x00, 0x01]).expect("write zstd frame");
    let err = parse_file(&rollout)
        .expect_err("zstd frame must not parse as JSONL")
        .to_string();
    assert!(
        err.contains("zstd-compressed"),
        "error must name the format, got: {err}"
    );
    assert!(
        err.contains("zstd -d"),
        "error must offer the decompression recipe"
    );
    assert!(
        !err.contains("not valid UTF-8"),
        "the misleading UTF-8 error is gone"
    );
    let _ = fs::remove_dir_all(root);
}

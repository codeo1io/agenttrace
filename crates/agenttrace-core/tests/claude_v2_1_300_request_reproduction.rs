//! rm-1522 — Claude Code v2.1.300 request-reproduction records (run
//! 7fd83ab2 cycle 2, research f4bdc13e C2).
//!
//! Upstream CHANGELOG §2.1.300: "Changed local session transcripts to
//! also record each API request's system prompt, tools and parameters,
//! so a transcript you choose to share can reproduce a request exactly."
//! The v2.1.300 wire shape is unpublished and the host runs claude-code
//! 2.0.76 (no live capture exists), so the vendored fixture below is
//! hand-built FROM THE CHANGELOG CONTRACT — it pins the parser's
//! behavior (disclose-by-wire-type, never silent; known shapes
//! unaffected; parsing bounded under per-request duplicates), not any
//! guessed internals of the record layout.
//!
//! Before this contract the claude lane's `parse_claude_code_jsonl`
//! armed only `"user"`/`"assistant"` and silently `_ => {}`-skipped
//! EVERY other wire kind — including the new request-reproduction
//! records and the previously-silent known kinds (`summary`, `system`,
//! …). The qwen lane (`qwen_unmatched_type`) and the codex lane
//! (`codex_unmatched_type`/`codex_non_object_line`) already disclose;
//! this contract brings the claude lane to the same never-silent bar
//! (`claude_unmatched_type:{kind}`), riding the lane's existing
//! one-carrier meta event.

use std::fs;
use std::path::Path;

use agenttrace_core::{
    parse_file, render_doctor_report, report_search_text, search_sessions, Session,
};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-code-2-1-300/request-reproduction.jsonl"
);

/// The known-shape body shared by the vendored fixture and the inline
/// control corpora: one user turn, one assistant turn with a hostile
/// `tool_use` (paste-safety composition with the rm-540 residual guard)
/// and pinned usage.
const KNOWN_USER: &str = concat!(
    r#"{"type":"user","message":{"role":"user","content":"run the migration"},"uuid":"u1","parentUuid":null,"#,
    r#""sessionId":"rr-2-1-300","timestamp":"2026-10-15T10:00:00Z","cwd":"/work/hostile","#,
    r#""gitBranch":"feature/paste-lane","version":"2.1.300"}"#,
    "\n",
);
const KNOWN_ASSISTANT: &str = concat!(
    r#"{"type":"assistant","message":{"id":"msg_a1","role":"assistant","model":"claude-sonnet-4-5-20250929","#,
    r#""content":[{"type":"tool_use","id":"t1","name":"=HYPERLINK(\"http://evil.example\",\"click-me\") please review","#,
    r#""input":{"command":"+cmd|' /C calc'!A0"}}],"usage":{"input_tokens":1200,"output_tokens":200,"#,
    r#""cache_read_input_tokens":800,"cache_creation_input_tokens":100}},"uuid":"a1","parentUuid":"u1","#,
    r#""sessionId":"rr-2-1-300","timestamp":"2026-10-15T10:00:02Z","version":"2.1.300"}"#,
    "\n",
);

/// A request-reproduction record hand-built from the CHANGELOG
/// contract: system prompt + tools + parameters, repeated per request.
fn rr_record(n: usize) -> String {
    let system_prompt = "You are Claude Code, Anthropic's official CLI for Claude. ".repeat(24);
    let mut line = String::with_capacity(4096);
    line.push_str(r#"{"type":"request_reproduction","uuid":"rr"#);
    line.push_str(&n.to_string());
    line.push_str(
        r#"","parentUuid":"u1","sessionId":"rr-2-1-300","timestamp":"2026-10-15T10:00:0"#,
    );
    line.push_str(&n.to_string());
    line.push_str(r#"Z","cwd":"/work/hostile","gitBranch":"feature/paste-lane","version":"2.1.300","request":{"system":[{"type":"text","text":""#);
    line.push_str(&system_prompt);
    line.push_str(r#""}],"tools":[{"name":"Bash","description":"Executes a given bash command in a persistent shell session","#);
    line.push_str(r#""input_schema":{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}}],"#);
    line.push_str(r#""parameters":{"model":"claude-sonnet-4-5-20250929","max_tokens":8192,"temperature":1.0,"stream":true}}}"#);
    line.push('\n');
    line
}

fn vendored_session() -> Session {
    parse_file(Path::new(FIXTURE))
        .unwrap_or_else(|error| panic!("vendored v2.1.300 fixture must parse: {error:#}"))
}

fn parse_corpus(name: &str, body: &str) -> Session {
    let dir = std::env::temp_dir().join(format!("rm1522-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("tempdir");
    let file = dir.join(format!("{name}.jsonl"));
    fs::write(&file, body).expect("fixture write");
    let session =
        parse_file(&file).unwrap_or_else(|error| panic!("corpus {name} must parse: {error:#}"));
    let _ = fs::remove_dir_all(&dir);
    session
}

fn unmatched(session: &Session) -> i64 {
    session
        .metrics
        .disclosure_counters
        .get("claude_unmatched_type:request_reproduction")
        .copied()
        .unwrap_or(0) as i64
}

/// The vendored fixture parses on the claude lane with the known shapes
/// intact (tokens, tool call, hostile tool name) while ALL THREE
/// request-reproduction records are disclosed — never silently skipped.
#[test]
fn fixture_parses_and_discloses_every_request_reproduction_record() {
    let session = vendored_session();
    assert_eq!(session.metrics.tokens_input, 1200, "user usage unchanged");
    assert_eq!(session.metrics.tokens_output, 200);
    assert!(
        session
            .metrics
            .tool_usage
            .contains_key("=HYPERLINK(\"http://evil.example\",\"click-me\") please review"),
        "hostile tool name must be mined into tool_usage: {:?}",
        session.metrics.tool_usage.keys().collect::<Vec<_>>()
    );
    assert_eq!(unmatched(&session), 3, "all three RR records disclosed");
    // The disclosure rides the lane's ONLY meta carrier — one event
    // regardless of record count (events_total pinned against the
    // control corpus below).
    assert!(
        !session
            .metrics
            .disclosure_counters
            .keys()
            .any(|key| key.starts_with("claude_unmatched_type:")
                && key != "claude_unmatched_type:request_reproduction"),
        "user/assistant are armed kinds — no other unmatched keys: {:?}",
        session.metrics.disclosure_counters
    );
}

/// Zero regression for pre-existing record shapes: adding the
/// request-reproduction records changes NOTHING about the session
/// except the one disclosure carrier (+1 event) and its counters.
#[test]
fn known_shapes_parse_identically_with_and_without_request_reproduction_records() {
    let base = parse_corpus("control", &format!("{KNOWN_USER}{KNOWN_ASSISTANT}"));
    let with_rr = vendored_session();

    assert_eq!(
        base.metrics.events_total,
        with_rr.metrics.events_total - 1,
        "RR records add exactly the one-carrier disclosure event"
    );
    macro_rules! assert_same {
        ($a:expr, $b:expr, $name:literal) => {
            assert_eq!(
                $a, $b,
                concat!($name, " must be identical with RR records present")
            );
        };
    }
    assert_same!(
        base.metrics.tokens_input,
        with_rr.metrics.tokens_input,
        "tokens_input"
    );
    assert_same!(
        base.metrics.tokens_output,
        with_rr.metrics.tokens_output,
        "tokens_output"
    );
    assert_same!(
        base.metrics.tokens_cache_r,
        with_rr.metrics.tokens_cache_r,
        "tokens_cache_r"
    );
    assert_same!(
        base.metrics.tokens_cache_w,
        with_rr.metrics.tokens_cache_w,
        "tokens_cache_w"
    );
    assert_same!(
        base.metrics.tool_calls_total,
        with_rr.metrics.tool_calls_total,
        "tool_calls_total"
    );
    assert_same!(
        base.metrics.user_messages,
        with_rr.metrics.user_messages,
        "user_messages"
    );
    assert_same!(
        base.metrics.assistant_turns,
        with_rr.metrics.assistant_turns,
        "assistant_turns"
    );
    assert_same!(
        base.metrics.model_used,
        with_rr.metrics.model_used,
        "model_used"
    );
    assert_same!(base.cwd, with_rr.cwd, "cwd");
    assert_eq!(unmatched(&base), 0, "control corpus has no unmatched kinds");
}

/// Previously-silent KNOWN claude kinds (`summary`, `system`) disclose
/// under the same key family — the disclosure is by wire type, honest
/// about what the lane does not mine, and changes no events.
#[test]
fn unarmed_known_kinds_disclose_too() {
    let body = format!(
        "{KNOWN_USER}{}\n{KNOWN_ASSISTANT}{}\n",
        r#"{"type":"summary","summary":"migrated the billing table","leafUuid":"u1","sessionId":"rr-2-1-300"}"#,
        r#"{"type":"system","subtype":"stackoverflow_error","content":"retry","uuid":"s1","sessionId":"rr-2-1-300"}"#,
    );
    let session = parse_corpus("known-kinds", &body);
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("claude_unmatched_type:summary"),
        Some(&1),
        "summary disclosed: {:?}",
        session.metrics.disclosure_counters
    );
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("claude_unmatched_type:system"),
        Some(&1),
        "system disclosed: {:?}",
        session.metrics.disclosure_counters
    );
}

/// Bounded parsing: 2,000 per-request-duplicate request-reproduction
/// records, each repeating the system prompt — the parse stays bounded
/// (counter increments only, content never retained): the event count
/// is INDEPENDENT of the record count (+1 carrier, always) and the
/// known-shape totals are byte-identical.
#[test]
fn repeated_request_reproduction_corpus_stays_bounded() {
    const N: usize = 2_000;
    let base = parse_corpus("bounded-base", &format!("{KNOWN_USER}{KNOWN_ASSISTANT}"));
    let mut body = String::with_capacity(N * 4096);
    body.push_str(KNOWN_USER);
    body.push_str(KNOWN_ASSISTANT);
    for n in 0..N {
        body.push_str(&rr_record(n));
    }
    let session = parse_corpus("bounded-rr", &body);
    assert_eq!(unmatched(&session), N as i64, "every RR record counted");
    assert_eq!(
        session.metrics.events_total,
        base.metrics.events_total + 1,
        "event count independent of RR record count"
    );
    assert_eq!(session.metrics.tokens_input, base.metrics.tokens_input);
    assert_eq!(
        session.metrics.tool_calls_total,
        base.metrics.tool_calls_total
    );
}

/// Paste-safety composition (acceptance 5): the request-reproduction
/// era exposes LONGER hostile tool-parameter strings to the search
/// lanes — with the rm-540 residual guard the v2.1.300 fixture's
/// evidence rows carry the paste-marker on the VALUE.
#[test]
fn request_reproduction_era_search_evidence_rows_carry_the_paste_marker() {
    let sessions = vec![vendored_session()];
    let tool_hits = search_sessions(&sessions, "click", 10);
    assert!(
        !tool_hits.is_empty(),
        "hostile tool name must match `click`"
    );
    let tool_text = report_search_text(&tool_hits, "click");
    assert!(
        tool_text.contains("- tool: '=HYPERLINK("),
        "tool evidence row must guard the VALUE: {tool_text}"
    );
    let arg_hits = search_sessions(&sessions, "calc", 10);
    assert!(
        !arg_hits.is_empty(),
        "hostile tool argument must match `calc`"
    );
    let arg_text = report_search_text(&arg_hits, "calc");
    assert!(
        arg_text.contains("- tool argument: '+cmd|"),
        "tool argument evidence row must guard the VALUE: {arg_text}"
    );
}

/// The disclosure reaches `--doctor` (the fleet's honesty channel):
/// the counter folds into the doctor report for a directory holding
/// the vendored fixture.
#[test]
fn doctor_report_surfaces_the_unmatched_kind_counter() {
    let dir = std::env::temp_dir().join(format!("rm1522-doctor-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("tempdir");
    fs::copy(FIXTURE, dir.join("request-reproduction.jsonl")).expect("fixture copy");
    let text = render_doctor_report(Some(&dir), false, "text").expect("doctor text render");
    let _ = fs::remove_dir_all(&dir);
    assert!(
        text.contains("claude_unmatched_type:request_reproduction"),
        "doctor must surface the claude-lane unmatched-kind counter: {text}"
    );
}

/// rm-601 rider (run de92ccb4 cycle 7, "admission honesty on
/// explicit-dir lanes", research RC8 / upstream ccusage #1935): a
/// REQUESTLESS `resp_` assistant corpus — assistant rows keyed by
/// Response-API ids with no request pairing to anchor them — folds by
/// id, keeping the MAX per token class, never the sum. ccusage
/// double-counted exactly this shape (streaming re-emissions of the
/// same `resp_` id summed); the rm-601 fold keyed on ANY non-empty
/// message id is the immunity. Pinned here in the v2.1.300 family
/// because that era's journals are where requestless resp_ rows
/// first appear in the wild. Behavior-neutral: parser code is
/// untouched; this pin guards the fold against a future max→sum
/// regression at the requestless seam.
#[test]
fn requestless_resp_rows_fold_by_id_keep_max_never_sum() {
    // One message, id `resp_redacted01`, streamed twice with growing
    // totals (output 200 then 500, input repeating 1200) — requestless:
    // no user turn or request record precedes it.
    let resp_row = |id: &str, input: u64, output: u64, seq: u8| {
        format!(
            concat!(
                r#"{{"type":"assistant","message":{{"id":"{}","role":"assistant","model":"claude-sonnet-4-5-20250929","#,
                r#""content":[{{"type":"text","text":"block {}"}}],"#,
                r#""usage":{{"input_tokens":{},"output_tokens":{},"#,
                r#""cache_read_input_tokens":800,"cache_creation_input_tokens":100}}}},"#,
                r#""uuid":"a{}","parentUuid":null,"sessionId":"rr-2-1-300","#,
                r#""timestamp":"2026-10-15T10:00:0{}Z","version":"2.1.300"}}"#,
                "\n"
            ),
            id, seq, input, output, seq, seq
        )
    };
    // A second, DISTINCT resp_ id: any-non-empty-id keying admits it
    // as its own message (distinct sessions never collapse). The lone
    // user turn carries no usage and no request record — the assistant
    // rows stay REQUESTLESS (nothing pairs them), which is the ccusage
    // #1935 shape the rm-601 fold must survive.
    let body = format!(
        "{}{}{}{}",
        KNOWN_USER,
        resp_row("resp_redacted01", 1200, 200, 1),
        resp_row("resp_redacted01", 1200, 500, 2),
        resp_row("resp_redacted02", 300, 50, 3),
    );
    let session = parse_corpus("requestless-resp", &body);
    assert_eq!(
        session.metrics.tokens_input,
        1200 + 300,
        "keep-max per id: re-emission of resp_redacted01 must not re-add its 1200"
    );
    assert_eq!(
        session.metrics.tokens_output, 500 + 50,
        "keep-max per id: 200→500 climb keeps 500, never sums to 700 (the ccusage #1935 double-count)"
    );
    assert_eq!(
        session.metrics.tokens_cache_r,
        800 + 800,
        "cache reads fold the same way (max per emission, then the distinct id adds its own)"
    );
    assert_eq!(
        session.metrics.assistant_turns, 2,
        "ONE meta event per id: 2 distinct resp_ ids, never 3 emissions"
    );
    assert_eq!(
        session.metrics.events_total, 6,
        "decomposition: 1 user turn + 3 streamed text blocks (per emission,\n        by design — content renders each block) + 2 usage-meta events (one\n        per DISTINCT id — a sum-folded ledger would bank a 4th emission)"
    );
}

/// Provenance: the fixture and the inline corpora describe the SAME
/// known-shape lines (the byte-identical arm above compares across
/// them, so they must not drift).
#[test]
fn vendored_fixture_known_lines_match_the_inline_control_corpus() {
    let vendored = fs::read_to_string(Path::new(FIXTURE)).expect("fixture read");
    assert!(
        vendored.starts_with(KNOWN_USER),
        "fixture must open with the control user line"
    );
    assert!(
        vendored.contains(KNOWN_ASSISTANT.trim_end()),
        "fixture must contain the control assistant line"
    );
}

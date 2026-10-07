//! Hostile-journal disclosure contract (run 460d0633 cycle-1 batch).
//!
//! Pins the four adversarial findings of this run's assessment against the
//! fixed code:
//!
//! - rm-595 — counts cells / disclosure keys carry no control bytes in ANY
//!   report surface (PoC p6-ansi emitted a raw OSC-52 sequence to the
//!   terminal and verbatim into `-f markdown -o` files).
//! - rm-594 — journal-derived counter keys are length-capped at mint, so a
//!   megabyte `type` cannot bloat every report and the session cache (PoC
//!   p1-huge minted a 1,060,176-byte cache entry), with the cap shape
//!   deterministic across parses.
//! - rm-542 F3 — id-less custom tool calls never attribute failure, and a
//!   completed replay of a previously-failed id transitions the pair to
//!   success (PoC p3-emptyid: a healthy session reported 0 ok / 2 failed).
//! - rm-449 / rm-488 — dual-id and claude-evidence transcripts classify as
//!   claude/generic, never qwen_code (upstream #304 port + the whole-file
//!   guard), and unknown usage wire keys are disclosed instead of silently
//!   zeroing tokens (PoC p4-kimi).

use agenttrace_core::data_health;
use agenttrace_core::parse_raw_session;
use agenttrace_core::report_overview_html_with_context;
use agenttrace_core::report_overview_markdown_with_context;
use agenttrace_core::report_overview_text_with_context;
use agenttrace_core::TimeRange;

fn codex_meta() -> String {
    serde_json::json!({
        "timestamp": "2026-10-06T12:00:00Z",
        "type": "session_meta",
        "payload": {"cwd": "/tmp/probe", "model": "gpt-5.3-codex"}
    })
    .to_string()
}

fn codex_line(payload: serde_json::Value) -> String {
    serde_json::json!({
        "timestamp": "2026-10-06T12:00:01Z",
        "type": "response_item",
        "payload": payload
    })
    .to_string()
}

fn codex_call(status: &str, call_id: &str) -> String {
    codex_line(serde_json::json!({
        "type": "custom_tool_call",
        "status": status,
        "call_id": call_id,
        "name": "exec",
        "input": "echo probe"
    }))
}

fn codex_output(call_id: &str) -> String {
    codex_line(serde_json::json!({
        "type": "custom_tool_call_output",
        "call_id": call_id,
        "output": [{"type": "input_text", "text": "done"}]
    }))
}

fn hostile_ansi_corpus() -> String {
    [
        codex_meta(),
        // PoC p6-ansi line 1: OSC-52 (ESC ] 5 2 ; c ; … BEL) smuggled in an
        // unmatched response_item payload type.
        codex_line(serde_json::json!({
            "type": "ansi\u{1b}]52;c;cHduYWdlPWNhdA==\u{7}future"
        })),
        // PoC p6-ansi line 2: OSC-52 in an unmatched top-level type.
        serde_json::json!({
            "timestamp": "2026-10-06T12:00:02Z",
            "type": "toptype\u{1b}]52;c;cHduYWdlPWNhdA==\u{7}drrift"
        })
        .to_string(),
        // Benign unknown shape: key stays byte-identical (no cap, no
        // substitution) — the contract must not mangle clean journals.
        codex_line(serde_json::json!({"type": "future_widget"})),
    ]
    .join("\n")
}

fn assert_no_control_bytes(label: &str, text: &str) {
    // \n/\r/\t are layout control chars the report legitimately uses;
    // the contract targets the terminal-injection class (C0/C1/DEL).
    assert!(
        !text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t'),
        "{label} still carries control bytes"
    );
}

#[test]
fn codex_unmatched_keys_are_sanitized_at_mint() {
    // rm-595 mint layer: the control-byte sanitizer runs where the counter
    // key is born, so every consumer (text/markdown/html/JSON/cache) is
    // covered even before the render choke points apply.
    let raw = hostile_ansi_corpus();
    let session = parse_raw_session("codex", "rollout-2026-10-06.jsonl", &raw)
        .expect("hostile codex rollout parses");
    let mut hostile = 0;
    for key in session.metrics.line_skips.keys() {
        if key.contains("codex_unmatched") {
            assert_no_control_bytes("counter key", key);
            if key.contains('\u{fffd}') {
                hostile += 1;
            }
        }
    }
    assert_eq!(hostile, 2, "both unmatched hostile arms sanitized");
    // Benign value rides through byte-identical — no persisted-shape
    // change for clean corpora (rm-594: no schema bump).
    assert!(session
        .metrics
        .line_skips
        .contains_key("codex_unmatched_response_item:future_widget"));
}

#[test]
fn counts_cells_render_sanitized_in_all_formats() {
    // rm-595 render layer: every overview arm renders the counts cells
    // through the sanitizer choke point — no raw ESC/BEL to the terminal
    // or into markdown/html files (PoC p6-ansi `-f markdown -o` leak).
    let session = parse_raw_session("codex", "rollout-2026-10-06.jsonl", &hostile_ansi_corpus())
        .expect("hostile codex rollout parses");
    let sessions = vec![session];
    let overview = agenttrace_core::compute_overview(&sessions);
    let health = data_health(&sessions, sessions.len(), 0);
    let text =
        report_overview_text_with_context(&overview, &sessions, &health, TimeRange::All, false);
    assert_no_control_bytes("text overview", &text);
    assert!(
        text.contains('\u{fffd}'),
        "sanitized key visible in text arm"
    );
    let markdown =
        report_overview_markdown_with_context(&overview, &sessions, &health, TimeRange::All, false);
    assert_no_control_bytes("markdown overview", &markdown);
    assert!(markdown.contains('\u{fffd}'));
    let html =
        report_overview_html_with_context(&overview, &sessions, &health, TimeRange::All, false);
    assert_no_control_bytes("html overview", &html);
    assert!(html.contains('\u{fffd}'));
}

#[test]
fn codex_unmatched_keys_are_length_capped_and_deterministic() {
    // rm-594: PoC p1-huge — a 1,048,617-char type string used to mint a
    // megabyte counter key that reached every report surface and the
    // shared session cache. The mint cap keeps a bounded prefix+digest.
    let huge_x = "X".repeat(1_048_576);
    let huge_y = "Y".repeat(1_048_576);
    let raw = [
        codex_meta(),
        codex_line(serde_json::json!({"type": huge_x})),
        serde_json::json!({
            "timestamp": "2026-10-06T12:00:02Z",
            "type": huge_y
        })
        .to_string(),
    ]
    .join("\n");
    let first =
        parse_raw_session("codex", "rollout-huge.jsonl", &raw).expect("huge rollout parses");
    let second = parse_raw_session("codex", "rollout-huge.jsonl", &raw).expect("reparses");
    let keys: Vec<&String> = first
        .metrics
        .line_skips
        .keys()
        .filter(|key| key.contains("codex_unmatched"))
        .collect();
    assert_eq!(keys.len(), 2, "two distinct unmatched arms: {keys:?}");
    for key in &keys {
        assert!(
            key.chars().count() < 160,
            "counter key not capped: {} chars",
            key.chars().count()
        );
    }
    assert_ne!(keys[0], keys[1], "distinct long values stay distinct");
    assert!(
        keys.iter().all(|key| key.contains("…#")),
        "capped keys carry the visible-prefix + digest form: {keys:?}"
    );
    // Determinism: identical input mints identical keys (cache stability).
    assert_eq!(
        first.metrics.line_skips, second.metrics.line_skips,
        "cap digest must be deterministic across parses"
    );
}

#[test]
fn failed_custom_calls_transition_and_idless_calls_never_fail() {
    // rm-542 F3 (dated append, PoC p3-emptyid): two defects — (1) the
    // id-less call inserted "" into the failure set and every later
    // id-less success output inherited the error; (2) a completed replay
    // of a previously-failed id left the failure pinned forever.
    let raw = [
        codex_meta(),
        codex_call("failed", ""),
        codex_call("completed", ""),
        codex_output(""),
        codex_output(""),
        codex_call("failed", "dup"),
        codex_call("completed", "dup"),
        codex_output("dup"),
        // A genuinely-failed pair keeps its failure attribution — the fix
        // must not overcorrect into hiding real agent-reported errors.
        codex_call("failed", "call_bad"),
        codex_output("call_bad"),
    ]
    .join("\n");
    let session = parse_raw_session("codex", "rollout.jsonl", &raw).expect("rollout parses");
    assert_eq!(session.metrics.tool_results, 4);
    assert_eq!(session.metrics.tool_calls_total, 5);
    assert_eq!(
        session.metrics.tool_calls_fail, 1,
        "only the genuinely-failed pair"
    );
    assert_eq!(session.metrics.tool_calls_ok, 3);
}

#[test]
fn legacy_cache_overlong_keys_render_bounded() {
    // rm-594 render guard: a cache entry written by a pre-cap binary can
    // still carry a megabyte counter key; the render choke point caps it
    // so the report stays bounded even before the cache refreshes.
    let session = parse_raw_session(
        "codex",
        "rollout-2026-10-06.jsonl",
        &[
            codex_meta(),
            codex_line(serde_json::json!({"type": "future_widget"})),
        ]
        .join("\n"),
    )
    .expect("codex rollout parses");
    let sessions = vec![session];
    let overview = agenttrace_core::compute_overview(&sessions);
    let mut health = data_health(&sessions, sessions.len(), 0);
    health.line_skips.insert(
        "codex_unmatched_type:".to_string() + &"Z".repeat(1_048_576),
        1,
    );
    let text =
        report_overview_text_with_context(&overview, &sessions, &health, TimeRange::All, false);
    assert!(
        text.chars().count() < 20_000,
        "legacy over-long key bloated the report: {} chars",
        text.chars().count()
    );
    assert!(
        !text.contains(&"Z".repeat(200)),
        "over-long key rendered unbounded"
    );
}

#[test]
fn dual_key_jsonl_transcripts_are_not_qwen() {
    // rm-449 (upstream luoyuctl/agenttrace#304 / 008ca975): a Claude Code
    // transcript that grew a qwen-shaped snake_case id alongside its
    // native camelCase `sessionId` is claude's. Before the fix the
    // session_id arm returned true unconditionally and whole dual-id
    // journals misclassified as qwen_code.
    let raw = [
        serde_json::json!({
            "type": "user",
            "session_id": "5f9db3eb-7a4b-4c53-9a1d-2f4c8e2a1111",
            "sessionId": "5f9db3eb-7a4b-4c53-9a1d-2f4c8e2a1111",
            "message": {"role": "user", "content": [{"type": "text", "text": "hi"}]},
            "cwd": "/tmp"
        })
        .to_string(),
        serde_json::json!({
            "type": "user",
            "session_id": "5f9db3eb-7a4b-4c53-9a1d-2f4c8e2a1111",
            "message": {"role": "user", "content": [{"type": "text", "text": "again"}]},
            "cwd": "/tmp"
        })
        .to_string(),
    ]
    .join("\n");
    let session = parse_raw_session("t", "session.jsonl", &raw).expect("dual-key jsonl parses");
    assert_ne!(
        session.metrics.source_tool, "qwen_code",
        "dual-id transcript misclassified as qwen (rm-449)"
    );
}

#[test]
fn claude_evidence_whole_file_document_is_not_qwen() {
    // rm-488: the whole-file-JSON lane probed qwen shape without the
    // file-level claude-evidence guard the JSONL lane applies — a single
    // document carrying claude-only keys (parentUuid) misclassified.
    let raw = serde_json::json!({
        "type": "user",
        "session_id": "5f9db3eb-7a4b-4c53-9a1d-2f4c8e2a2222",
        "parentUuid": null,
        "message": {"role": "user", "content": [{"type": "text", "text": "hi"}]},
        "cwd": "/tmp"
    })
    .to_string();
    let session = parse_raw_session("t", "session.json", &raw).expect("whole-file json parses");
    assert_ne!(
        session.metrics.source_tool, "qwen_code",
        "claude-evidence document misclassified as qwen (rm-488)"
    );
}

#[test]
fn unknown_usage_wire_keys_are_disclosed_not_silent() {
    // rm-449 F4 (PoC p4-kimi + review bd6e4a50 F1): a usage entry this
    // lane cannot account is disclosed, never silently zeroed. The
    // fixture carries the three tiers at once: "input_tokens" is
    // CANONICAL but sits at message.usage, a scanned location this lane
    // never reads → usage_unconsumed_location (pre-fix: silently zeroed
    // — review counterexample f4-loc); "INPUT_TOKENS_X" is in no
    // vocabulary → usage_unknown_key (PoC p4-kimi's original arm); a
    // canonical TOP-LEVEL key would be consumed and stays silent
    // (counterexample f4-control, covered by
    // alias_usage_keys_disclose_instead_of_silent_zero's sibling and the
    // deterministic accountings below).
    let raw = [serde_json::json!({
        "type": "message",
        "role": "assistant",
        "timestamp": "2026-10-06T01:00:00Z",
        "message": {"usage": {"input_tokens": 5, "INPUT_TOKENS_X": 10}}
    })
    .to_string()]
    .join("\n");
    let session = parse_raw_session("t", "session.jsonl", &raw).expect("usage line parses");
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unknown_key:INPUT_TOKENS_X"),
        Some(&1),
        "unknown usage key must disclose: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:input_tokens"),
        Some(&1),
        "relocated canonical key must disclose, not silently zero: {:?}",
        session.metrics.line_skips
    );
    assert!(
        !session
            .metrics
            .line_skips
            .keys()
            .any(|key| key.contains("unknown:input_tokens")),
        "the canonical key is disclosed as relocated, not as unknown: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn alias_usage_keys_disclose_instead_of_silent_zero() {
    // review bd6e4a50 F1 counterexample f4-alias: top-level
    // prompt_tokens/completion_tokens ARE in the shared alias vocabulary
    // (the kimi lane maps them), but this lane's accounting reads only
    // the five canonical names — pre-fix these were "recognized" and
    // therefore silently zeroed with EMPTY line_skips (0/0 tokens, no
    // disclosure): the acceptance clause "never silently zeroed" was
    // live-false. The fix routes them to a distinct low-noise counter so
    // alias contamination is triageable apart from unknown garbage.
    // Fixture shape mirrors the review probe: a session_meta line pins
    // the file to the hermes/generic JSONL lane (a single-line file
    // would route to the gemini whole-JSON lane, which maps its own
    // aliases and never reaches this code).
    let raw = [
        serde_json::json!({
            "type": "message",
            "role": "session_meta",
            "timestamp": "2026-10-06T01:00:00Z"
        }),
        serde_json::json!({
            "type": "message",
            "role": "assistant",
            "timestamp": "2026-10-06T01:00:00Z",
            "usage": {"prompt_tokens": 100, "completion_tokens": 50}
        }),
    ]
    .iter()
    .map(|line| line.to_string())
    .collect::<Vec<_>>()
    .join("\n");
    let session = parse_raw_session("t", "session.jsonl", &raw).expect("alias line parses");
    assert_eq!(session.metrics.tokens_input, 0, "alias is not consumed");
    assert_eq!(session.metrics.tokens_output, 0, "alias is not consumed");
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_alias_unmapped:prompt_tokens"),
        Some(&1),
        "known-but-unmapped alias must disclose: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_alias_unmapped:completion_tokens"),
        Some(&1),
        "known-but-unmapped alias must disclose: {:?}",
        session.metrics.line_skips
    );
    assert!(
        !session
            .metrics
            .line_skips
            .keys()
            .any(|key| key.contains("unknown")),
        "aliases are disclosed under their own counter, not as unknown: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn relocated_usage_keys_disclose_instead_of_silent_zero() {
    // review bd6e4a50 F1 counterexamples f4-loc AND f4-control, plus the
    // role shape the location-only wording missed: canonical
    // {input_tokens, output_tokens} at message.usage AND at the top-level
    // `usage` of a NON-meta line used to be "recognized" → consumed=true
    // → NO disclosure, while the accumulator reads top-level usage only on
    // `session_meta`/`meta` events — tokens 0/0 with EMPTY line_skips.
    // The role+location-aware disclosure names every canonical key the
    // accounting cannot take, while the meta line's top-level canonical
    // usage in the SAME session is accounted normally (f4-control).
    let raw = [
        serde_json::json!({
            "type": "message",
            "role": "session_meta",
            "timestamp": "2026-10-06T01:00:00Z"
        }),
        serde_json::json!({
            "type": "message",
            "role": "assistant",
            "timestamp": "2026-10-06T01:00:01Z",
            "usage": {"input_tokens": 7, "output_tokens": 3},
            "message": {"usage": {"input_tokens": 100, "output_tokens": 50}}
        }),
        serde_json::json!({
            "type": "message",
            "role": "meta",
            "timestamp": "2026-10-06T01:00:02Z",
            "usage": {"input_tokens": 7, "output_tokens": 3}
        }),
    ]
    .iter()
    .map(|line| line.to_string())
    .collect::<Vec<_>>()
    .join("\n");
    let session = parse_raw_session("t", "session.jsonl", &raw).expect("relocated parses");
    assert_eq!(
        session.metrics.tokens_input, 7,
        "meta-role top-level usage IS read"
    );
    assert_eq!(
        session.metrics.tokens_output, 3,
        "meta-role top-level usage IS read"
    );
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:input_tokens"),
        Some(&2),
        "assistant top-level AND message.usage must each disclose: {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("usage_unconsumed_location:output_tokens"),
        Some(&2),
        "assistant top-level AND message.usage must each disclose: {:?}",
        session.metrics.line_skips
    );
    // rm-616 composition (reconciled at integration 2026-10-08, conflict
    // case 15bdfe5f): the landed rm-616 review fix F3 also fires
    // `usage_present_not_counted` beside meta usage for conversation
    // lines whose family's counted lane is the meta arm — this journal's
    // assistant line is exactly that shape, so its usage discloses on
    // its own aggregate channel in addition to the two relocated-key
    // counters. That supersedes this test's older len()==2 pin (the
    // disclose-not-silently-zero intent only widened); the meta line's
    // canonical usage still discloses nothing on its own behalf.
    assert_eq!(
        session.metrics.line_skips.get("usage_present_not_counted"),
        Some(&1),
        "the assistant line's out-of-lane usage discloses beside meta usage (rm-616 F3): {:?}",
        session.metrics.line_skips
    );
    assert_eq!(
        session.metrics.line_skips.len(),
        3,
        "exactly the two relocated-key counters plus rm-616's beside-meta disclosure: {:?}",
        session.metrics.line_skips
    );
}

#[test]
fn capped_digest_is_exactly_twelve_hex() {
    // review bd6e4a50 F2: the unmasked u64 FNV rendered as 12–16 hex
    // digits, contradicting every comment and doc that said "12 hex".
    // The cap helper now masks to the low 48 bits — EXACTLY 12 lowercase
    // hex chars after the '#' digest marker, deterministic across runs.
    let hostile_key = "ZZZ".repeat(100_000);
    let mut usage = serde_json::Map::new();
    usage.insert(hostile_key.clone(), serde_json::json!(10));
    let raw = [
        serde_json::json!({
            "type": "message",
            "role": "session_meta",
            "timestamp": "2026-10-06T01:00:00Z"
        }),
        serde_json::json!({
            "type": "message",
            "role": "assistant",
            "timestamp": "2026-10-06T01:00:00Z",
            "usage": serde_json::Value::Object(usage)
        }),
    ]
    .iter()
    .map(|line| line.to_string())
    .collect::<Vec<_>>()
    .join("\n");
    let first = parse_raw_session("t", "session.jsonl", &raw).expect("hostile line parses");
    let second = parse_raw_session("t", "session.jsonl", &raw).expect("reparse");
    let capped: Vec<&String> = first
        .metrics
        .line_skips
        .keys()
        .filter(|key| key.starts_with("usage_unknown_key:"))
        .collect();
    assert_eq!(capped.len(), 1, "exactly one disclosure: {capped:?}");
    let key = capped[0];
    let (prefix, digest) = key.rsplit_once('#').expect("capped keys carry '#'");
    assert_eq!(digest.len(), 12, "EXACTLY 12 hex digits: {digest}");
    assert!(
        digest
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "digest is lowercase hex: {digest}"
    );
    // CHARS, not bytes: the truncation marker `…` is 3 bytes in UTF-8,
    // and the cap contract is 48 visible chars (review bd6e4a50 F2
    // follow-up — the first draft asserted .len() and measured 69).
    assert_eq!(
        prefix.chars().count(),
        "usage_unknown_key:".len() + 48 + 1,
        "48 visible prefix chars + the marker: {prefix}"
    );
    assert_eq!(
        first.metrics.line_skips.keys().collect::<Vec<_>>(),
        second.metrics.line_skips.keys().collect::<Vec<_>>(),
        "digest is deterministic across parses"
    );
}

#[test]
fn unknown_usage_key_disclosure_is_bounded_and_sanitized() {
    // rm-449 F4 x rm-594 composition: the PoC's unknown key was 300,190
    // bytes of hostile text — the disclosure itself must stay bounded and
    // control-byte-free.
    let mut hostile_key = "Z".repeat(300_190);
    hostile_key.push_str("\u{1b}]52;c;x\u{7}");
    let mut usage = serde_json::Map::new();
    usage.insert(hostile_key.clone(), serde_json::json!(10));
    let raw = serde_json::json!({
        "type": "message",
        "role": "assistant",
        "timestamp": "2026-10-06T01:00:00Z",
        "message": {"usage": serde_json::Value::Object(usage)}
    })
    .to_string();
    let session = parse_raw_session("t", "session.jsonl", &raw).expect("hostile usage line parses");
    let minted: Vec<&String> = session
        .metrics
        .line_skips
        .keys()
        .filter(|key| key.starts_with("usage_unknown_key:"))
        .collect();
    assert_eq!(minted.len(), 1, "one hostile key, one disclosure");
    let key = minted[0];
    assert_no_control_bytes("usage disclosure key", key);
    assert!(
        key.chars().count() < 160,
        "disclosure key not capped: {key:?}"
    );
    assert!(key.contains("…#"), "digest form present: {key:?}");
}

//! pi-family journal format contract (rm-423).
//!
//! Upstream (`@earendil-works/pi-coding-agent`) shipped 1.0.0 ->
//! 1.0.1 -> 1.0.2 within four days (observed 2026-10-04) and went
//! PUBLIC that same day — rm-423's original PRIVATE/404 premise was
//! amended at the run-6403d975 integration per that run's research
//! pass-12 rider (releases+tags APIs live; a versioned session-format
//! spec ships at `packages/coding-agent/docs/session-format.md`).
//! Nothing in this repo watches that channel yet, so journal-format
//! drift still arrives unannounced here: these tests remain the
//! drift tripwire, pinning the wire shapes the parser sniffs and
//! extracts, now diffable against the published spec.
//! Every fixture is asserted at three levels — raw header keys, message
//! wire keys, and extracted metrics — and every assertion names the
//! offending key, so a drift failure points at the contract that moved
//! instead of failing as an opaque count mismatch.
//!
//! Fixture provenance:
//! - `pi-pi-v3/` — DERIVED + REDACTED from a real 1.0.2 journal parsed
//!   live on this host (2026-10-04); key sets and value types preserved,
//!   ids/cwd/text synthesized.
//! - `pi-oh-my-pi/`, `pi-senpi/` — synthesized per the parser's sniff
//!   arms (title preamble + versionless session header; `parentSession`
//!   header).
//!
//! Known-divergence pins (deliberately NOT fixed here; each belongs to
//! its own roadmap item):
//! - `model_change` wire key: RESOLVED — rm-438 (run 6403d975, landed
//!   at its integration) made the handler read `modelId` (with `model`
//!   honored as a legacy spelling), so the former dead-handler
//!   divergence pin was flipped by intent, not drift, as this header
//!   requires. The wire key is pinned positively below and the
//!   attribution/repricing behavior end to end in
//!   `pi_usage_tree_accounting.rs` (rm-436/437/438 batch).
//! - `branch_summary`/`compaction` entries are currently surfaced as
//!   assistant turns (the omp arm routes them through the message
//!   path). PINNED by
//!   `branch_summary_surfaces_as_assistant_turn_known_divergence`: if
//!   you change that routing, this suite must change WITH you, by
//!   intent, not by drift.

use agenttrace_core::parse_raw_session;
use serde_json::Value;

/// Journal `version` markers this contract knows how to interpret.
/// Observed live: v3 (1.0.2, 2026-10-04); v1/v2 are the recorded
/// historical shapes (v2 introduced id/parentId tree journals).
const KNOWN_PI_JOURNAL_VERSIONS: &[u64] = &[1, 2, 3];

const PI_V3: &str = "tests/fixtures/pi-pi-v3/journal.jsonl";
const OH_MY_PI: &str = "tests/fixtures/pi-oh-my-pi/journal.jsonl";
const SENPI: &str = "tests/fixtures/pi-senpi/journal.jsonl";

fn fixture(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

/// First JSONL line as an object — the journal header.
fn header_object(path: &str) -> serde_json::Map<String, Value> {
    let raw = fixture(path);
    let line = raw
        .lines()
        .next()
        .unwrap_or_else(|| panic!("{path}: fixture is empty"));
    let value: Value = serde_json::from_str(line)
        .unwrap_or_else(|error| panic!("{path}: header line is not JSON: {error}"));
    value
        .as_object()
        .cloned()
        .unwrap_or_else(|| panic!("{path}: header line is not an object"))
}

/// The version-marker gate: a journal whose header `version` is not in
/// the known set must fail HERE, naming the key, before any parse
/// silently degrades discovery/labeling/attribution.
fn assert_known_version(path: &str, header: &serde_json::Map<String, Value>) -> u64 {
    let version = header
        .get("version")
        .unwrap_or_else(|| panic!("{path}: contract drift — header key 'version' missing"))
        .as_u64()
        .unwrap_or_else(|| {
            panic!(
                "{path}: contract drift — header key 'version' is not an integer: {:?}",
                header.get("version")
            )
        });
    assert!(
        KNOWN_PI_JOURNAL_VERSIONS.contains(&version),
        "{path}: contract drift — header key 'version' = {version} is not a known journal version (known: {KNOWN_PI_JOURNAL_VERSIONS:?}); the pi upstream changed its journal format"
    );
    version
}

fn require_key(path: &str, header: &serde_json::Map<String, Value>, key: &str) {
    assert!(
        header.contains_key(key),
        "{path}: contract drift — header key '{key}' missing; header keys present: {:?}",
        header.keys().collect::<Vec<_>>()
    );
}

/// Recover a panic payload's text (format-string panics carry String).
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        String::from("<non-string panic payload>")
    }
}

#[test]
fn pi_v3_journal_header_contract() {
    let header = header_object(PI_V3);
    require_key(PI_V3, &header, "type");
    assert_eq!(
        header.get("type").and_then(Value::as_str),
        Some("session"),
        "{PI_V3}: contract drift — header key 'type' is no longer \"session\""
    );
    for key in ["id", "timestamp", "cwd"] {
        require_key(PI_V3, &header, key);
    }
    assert_eq!(assert_known_version(PI_V3, &header), 3);
}

#[test]
fn pi_v3_journal_message_usage_and_model_contract() {
    // Message wire shape + usage alias mapping, end to end.
    let session = parse_raw_session("pi-v3", PI_V3, &fixture(PI_V3))
        .unwrap_or_else(|error| panic!("{PI_V3}: real-shape v3 journal no longer parses: {error}"));
    assert_eq!(
        session.metrics.assistant_turns, 1,
        "{PI_V3}: contract drift — assistant message no longer counted"
    );
    assert_eq!(session.metrics.user_messages, 1);
    // usage { input, output, cacheRead, cacheWrite, totalTokens }
    assert_eq!(session.metrics.tokens_input, 500, "key 'input'");
    assert_eq!(session.metrics.tokens_output, 1000, "key 'output'");
    assert_eq!(session.metrics.tokens_cache_r, 25000, "key 'cacheRead'");
    assert_eq!(session.metrics.tokens_cache_w, 5000, "key 'cacheWrite'");
    // rm-438 (landed at the run 6403d975 integration): the handler is
    // LIVE — `modelId` is the wire key and `model` stays a legacy
    // spelling — so the session no longer reports (and prices at) the
    // pre-switch model. The usage block still attributes to its own
    // message-level `model` (pinned by the per-block cost below), and
    // the session row discloses the mix as `multiple` per the
    // parser-agnostic per-block rule instead of silently billing the
    // sonnet block at the post-switch model's rate.
    assert_eq!(
        session.metrics.model_used, "multiple",
        "{PI_V3}: contract drift — a switched session must disclose the model mix, not name the pre-switch model"
    );
    let sonnet = agenttrace_core::lookup_price("claude-sonnet-4-20250514");
    let expected_block_cost = 500.0 / 1e6 * sonnet.input
        + 1000.0 / 1e6 * sonnet.output
        + 25000.0 / 1e6 * sonnet.cr
        + 5000.0 / 1e6 * sonnet.cw;
    assert!(
        (session.metrics.cost_estimated - agenttrace_core::round4(expected_block_cost)).abs()
            < 1e-9,
        "{PI_V3}: message-level key 'model' no longer drives pricing attribution: got {} want {}",
        session.metrics.cost_estimated,
        expected_block_cost
    );
    assert!(
        session.metrics.cost_estimated > 0.0,
        "{PI_V3}: contract drift — usage no longer yields a priced cost"
    );
    // rm-438 (landed at the run 6403d975 integration): the handler is
    // LIVE — `modelId` is the wire key and `model` stays a legacy
    // spelling. Pin the switch end to end on this fixture's own
    // model_change: a model-less post-switch assistant message must
    // attribute to the switched model, not the pre-switch one (the
    // former dead-handler divergence pin is retired above).
    let model_change = fixture(PI_V3)
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("json"))
        .find(|value| value.get("type").and_then(Value::as_str) == Some("model_change"))
        .expect("model_change entry present");
    assert!(
        model_change.get("modelId").is_some(),
        "{PI_V3}: contract drift — model_change no longer carries key 'modelId'"
    );
    let switched_model = model_change
        .get("modelId")
        .and_then(Value::as_str)
        .expect("modelId is a string")
        .to_string();
    let switched = format!(
        "{parent}\n{{\"id\": \"00000000-0000-4000-8000-000000000007\", \"parentId\": \"00000000-0000-4000-8000-000000000006\", \"timestamp\": \"2026-10-04T07:16:48.300Z\", \"message\": {{\"role\": \"assistant\", \"content\": [{{\"type\": \"text\", \"text\": \"after the switch\"}}], \"timestamp\": \"2026-10-04T07:16:48.300Z\"}}, \"type\": \"message\"}}",
        parent = fixture(PI_V3)
    );
    let switched_session = parse_raw_session("pi-v3-switch", PI_V3, &switched)
        .unwrap_or_else(|error| panic!("{PI_V3}: post-switch extension no longer parses: {error}"));
    assert_ne!(
        switched_session.metrics.model_used, "claude-sonnet-4-20250514",
        "{PI_V3}: model_change 'modelId' no longer applies — the handler went dead again"
    );
    assert_eq!(
        switched_session.metrics.model_used, "multiple",
        "{PI_V3}: post-switch attribution must disclose the mix (switched {switched_model}, usage claude-sonnet-4-20250514)"
    );
    assert_eq!(
        switched_session.metrics.assistant_turns, 2,
        "{PI_V3}: the model-less post-switch assistant message must still count as a turn"
    );
}

#[test]
fn oh_my_pi_style_header_and_alias_contract() {
    // Title preamble + versionless session header (the sniff OR arm)
    // and the second usage alias family.
    let lines: Vec<Value> = fixture(OH_MY_PI)
        .lines()
        .map(|line| serde_json::from_str(line).expect("json"))
        .collect();
    assert!(
        lines[0].get("title").is_some(),
        "{OH_MY_PI}: contract drift — title preamble line no longer first"
    );
    let header = lines[1].as_object().expect("header object");
    require_key(OH_MY_PI, header, "type");
    require_key(OH_MY_PI, header, "id");
    assert!(
        !header.contains_key("version"),
        "{OH_MY_PI}: fixture drift — this variant pins the versionless arm"
    );
    let session = parse_raw_session("omp", OH_MY_PI, &fixture(OH_MY_PI)).unwrap_or_else(|error| {
        panic!("{OH_MY_PI}: versionless variant no longer parses: {error}")
    });
    // usage { input_tokens, output_tokens, cache_creation_input_tokens,
    // cache_read_input_tokens, cost }
    assert_eq!(session.metrics.tokens_input, 100, "key 'input_tokens'");
    assert_eq!(session.metrics.tokens_output, 50, "key 'output_tokens'");
    assert_eq!(
        session.metrics.tokens_cache_w, 40,
        "key 'cache_creation_input_tokens'"
    );
    assert_eq!(
        session.metrics.tokens_cache_r, 10,
        "key 'cache_read_input_tokens'"
    );
    assert_eq!(
        session.metrics.model_used, "gpt-5",
        "{OH_MY_PI}: contract drift — model attribution moved"
    );
}

#[test]
fn branch_summary_surfaces_as_assistant_turn_known_divergence() {
    // Known-divergence PIN (review fix 2026-10-05; finding 5): the
    // suite's header doc promises this suite changes WITH any
    // branch_summary/compaction routing change — that promise is now
    // enforced, not just documented. The fixture carries exactly one
    // real assistant message plus one `branch_summary` system entry
    // (parser.rs omp arm routes non-empty `summary` entries through
    // the message path as assistant events; rm-421 owns the reroute).
    let session = parse_raw_session("omp", OH_MY_PI, &fixture(OH_MY_PI)).unwrap_or_else(|error| {
        panic!("{OH_MY_PI}: versionless variant no longer parses: {error}")
    });
    assert_eq!(
        session.metrics.assistant_turns, 2,
        "{OH_MY_PI}: branch_summary/compaction entries no longer surface as \
         assistant turns — reroute deliberate? update this known-divergence \
         pin (and the suite's header doc) with the routing change (rm-421)"
    );
    assert_eq!(
        session.metrics.user_messages, 1,
        "{OH_MY_PI}: branch_summary entry must not surface as a user message"
    );
}

#[test]
fn senpi_omo_style_parent_session_contract() {
    // Header carrying `parentSession` (the fourth sniff-OR key) and a
    // minimal usage object { input, output } with no cache/cost keys.
    let header = header_object(SENPI);
    require_key(SENPI, &header, "parentSession");
    let session = parse_raw_session("senpi", SENPI, &fixture(SENPI))
        .unwrap_or_else(|error| panic!("{SENPI}: parentSession variant no longer parses: {error}"));
    assert_eq!(session.metrics.tokens_input, 10, "key 'input'");
    assert_eq!(session.metrics.tokens_output, 5, "key 'output'");
    assert_eq!(session.metrics.tokens_cache_r, 0);
    assert_eq!(session.metrics.tokens_cache_w, 0);
    assert_eq!(
        session.metrics.model_used, "claude-3-5-haiku-20241022",
        "{SENPI}: contract drift — model attribution moved"
    );
}

#[test]
fn version_marker_drift_is_named() {
    // The version-marker gate must fire on an unknown version BEFORE
    // parsing silently degrades: synthesize a v4 header and assert the
    // gate names the 'version' key.
    let header = header_object(PI_V3);
    let mut drifted = header.clone();
    drifted.insert("version".to_string(), Value::from(4_u64));
    let message = std::panic::catch_unwind(|| assert_known_version(PI_V3, &drifted))
        .expect_err("unknown version must trip the gate");
    let text = panic_message(&message);
    assert!(
        text.contains("version"),
        "drift failure must name the 'version' key, got: {text}"
    );
    assert!(
        text.contains("known"),
        "drift failure must list the known set"
    );
}

#[test]
fn missing_header_key_drift_is_named() {
    // Removing the header `id` must be caught by the key-presence
    // check naming 'id' — and the parser itself must reject the file
    // rather than misparse it.
    let header = header_object(PI_V3);
    let mut drifted = header.clone();
    drifted.remove("id");
    let message = std::panic::catch_unwind(|| require_key(PI_V3, &drifted, "id"))
        .expect_err("missing header id must trip the key gate");
    assert!(
        panic_message(&message).contains("'id'"),
        "drift failure must name the 'id' key"
    );
    let raw = {
        let original = fixture(PI_V3);
        let mut lines: Vec<String> = original.lines().map(str::to_string).collect();
        lines[0] = serde_json::to_string(&drifted).expect("json");
        lines.join("\n")
    };
    assert!(
        parse_raw_session("pi-v3-drift", PI_V3, &raw).is_err(),
        "header without 'id' must fail the parse, not silently misparse"
    );
}

#[test]
fn known_version_set_is_current() {
    // The observed 1.0.2 baseline (v3) must be in the known set — if
    // this fails, update KNOWN_PI_JOURNAL_VERSIONS when a new pi
    // release lands and extend the fixtures to match.
    assert!(KNOWN_PI_JOURNAL_VERSIONS.contains(&3));
}

/// Parses a bundled fixture journal into a Session for the writer-
/// surface contract tests below.
fn parse_fixture_session(name: &str, path: &str) -> agenttrace_core::Session {
    parse_raw_session(name, path, &fixture(path))
        .unwrap_or_else(|error| panic!("{name} must parse as pi journal: {error}"))
}

// rm-551 writer-surface drift contract. pi's dist session-manager.js
// writes exactly 13 entry types (verified 1.0.3 vs 1.0.4 tarballs,
// session-manager.js byte-identical; research C1). This contract pins
// the parser's response to EVERY one of them: each type must land in
// exactly one bucket — DISCLOSED (pi_entry_skipped:<type>) or HANDLED
// (no skip counter) — and every HANDLED type that can carry spend
// must actually land its spend in the totals. This is the regression
// gate for the handled-but-partial class the compaction arm embodied:
// the type counted as handled (pi_entry_skipped 0) while its usage
// block was silently discarded, leaving the largest single cost in a
// pi session invisible in every number AND every disclosure. If a
// future pi release adds/reshapes a writer type, one of the three
// assertions below fails until the arm (and this table) is updated
// with intent: the enumeration asserts the expected classification,
// the totals assert the spend landed, the skip-key sweep asserts no
// type silently falls between the buckets.
const WRITER_SURFACE_FIXTURE: &str = "tests/fixtures/pi-writer-surface/journal.jsonl";

#[test]
fn pi_writer_surface_contract_covers_all_thirteen_dist_types() {
    let session = parse_fixture_session("pi-writer-surface", WRITER_SURFACE_FIXTURE);
    let counters = &session.metrics.disclosure_counters;

    // The complete 13-type enumeration from pi's dist
    // session-manager.js writer surface, with the classification each
    // type must hold. true = the type is deliberately skipped and
    // named in a pi_entry_skipped:<type> counter; false = a parser
    // arm consumes it and NO skip counter may fire for it.
    const WRITER_SURFACE: [(&str, bool); 13] = [
        ("session", false), // header: id/cwd/version
        ("label", true),
        ("text", true),
        ("usage", false), // rm-436 standalone usage arm
        ("thinking_level_change", true),
        ("session_info", true),
        ("model_change", false), // rm-438 tracked-model switch
        ("message", false),
        // custom_message rides the message path (the omp arm routes it
        // through :1612) — handled, not skipped.
        ("custom_message", false),
        ("custom", true),
        ("context_edit", true),
        // rm-551: usage folded, disclosed per kind — the summary turn
        // and the usage meta event are BOTH accounted, and the kind is
        // distinguishable in the counters.
        ("compaction", false),
        ("branch_summary", false),
    ];

    for (typ, disclosed) in WRITER_SURFACE {
        let key = format!("pi_entry_skipped:{typ}");
        let hits = counters.get(&key).copied().unwrap_or(0);
        if disclosed {
            assert_eq!(hits, 1, "writer type {typ} must disclose its skip");
        } else {
            assert_eq!(hits, 0, "writer type {typ} is handled — a skip counter here means an arm regressed to the catch-all");
        }
    }

    // No pi_entry_skipped key may exist outside the 13-type table: a
    // stray key means a type reshaped on the wire (or a new type)
    // fell through the arms unclassified.
    for key in counters.keys() {
        if let Some(typ) = key.strip_prefix("pi_entry_skipped:") {
            assert!(
                WRITER_SURFACE.iter().any(|(t, d)| *d && *t == typ),
                "unexpected pi_entry_skipped:{typ} — the writer surface drifted beyond the 13-type contract; update the table and the arm with intent"
            );
        }
    }

    // The spend assertions: every HANDLED type that carries usage in
    // the fixture must land its tokens in the totals — message
    // (500/1000/25000/5000), usage entry (100/20), compaction
    // (38000/900, upstream-recorded $0.0123), branch_summary (700/80).
    let m = &session.metrics;
    assert_eq!(
        m.tokens_input,
        500 + 100 + 38000 + 700,
        "all four usage-bearing types must count input"
    );
    assert_eq!(
        m.tokens_output,
        1000 + 20 + 900 + 80,
        "all four usage-bearing types must count output"
    );
    assert_eq!(m.tokens_cache_r, 25000);
    assert_eq!(m.tokens_cache_w, 5000);
    assert_eq!(
        m.upstream_cost_usd, 0.0123,
        "the compaction call's upstream-recorded cost must ride along"
    );
    assert_eq!(
        m.assistant_turns, 3,
        "message + compaction summary + branch_summary summary"
    );
    assert_eq!(counters.get("pi_usage_entry:main").copied().unwrap_or(0), 1);
    assert_eq!(
        counters.get("pi_compaction_usage_counted:compaction").copied().unwrap_or(0),
        1,
        "the compaction usage fold must be disclosed per kind — its absence is the pre-rm-551 silent drop"
    );
    assert_eq!(
        counters
            .get("pi_compaction_usage_counted:branch_summary")
            .copied()
            .unwrap_or(0),
        1,
        "branch_summary usage folds through the same arm and stay separately countable"
    );
}

// rm-551 sentinel: the assess PoC shape pinned green at the parse
// level. One user/assistant exchange plus one compaction call whose
// inline usage block (38000 in / 900 out / $0.0123 upstream-recorded)
// was the whole-context call's real spend. Pre-fix the session
// reported $0.0023 (catalog of the exchange alone); spec-true is the
// catalog PLUS the compaction call's recorded cost — asserted to the
// cent in pi_usage_tree_accounting::compaction_usage_composes_with_
// catalog; here the parse-level facts are pinned: the tokens count,
// the recorded cost rides along, the fold is disclosed, and nothing
// skips.
#[test]
fn pi_compaction_usage_sentinel_counts_tokens_cost_and_disclosure() {
    let session = parse_fixture_session("pi-writer-surface", WRITER_SURFACE_FIXTURE);
    let counters = &session.metrics.disclosure_counters;
    assert_eq!(session.metrics.upstream_cost_usd, 0.0123);
    assert_eq!(session.metrics.assistant_turns, 3);
    assert_eq!(
        counters
            .get("pi_entry_skipped:compaction")
            .copied()
            .unwrap_or(0),
        0
    );
    assert_eq!(
        counters
            .get("pi_compaction_usage_counted:compaction")
            .copied()
            .unwrap_or(0),
        1
    );
}

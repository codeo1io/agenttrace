//! rm-493 — OpenTelemetry GenAI-conformant OTLP-JSON export (file-only).
//!
//! Emits an OTLP/HTTP-JSON `ExportTraceServiceRequest` document with one
//! INTERNAL span per session carrying `gen_ai.*` attributes per the GenAI
//! semantic conventions (open-telemetry/semantic-conventions docs/gen-ai,
//! attribute names verified on [`SEMCONV_SNAPSHOT_DATE`]). The render is a
//! file/stdout write only — there is no network transport in this module,
//! honoring PRIVACY.md's offline-first posture.
//!
//! Per-assistant-turn child spans are deliberately NOT emitted at this
//! revision: the loader folds events into [`Session`] aggregates and no
//! public parse API retains events (`parser::parse_file` returns
//! `Session` only). Exposing one means a parser.rs surface change that is
//! outside this batch's stewardship boundaries — tracked as a follow-up
//! rider on rm-493.
//!
//! Input-token basis (recorded decision, rm-599 ): the emitted
//! `gen_ai.usage.input_tokens` carries agenttrace's DELTA basis — the
//! uncached portion, with `gen_ai.usage.cache_read.input_tokens` and
//! `gen_ai.usage.cache_write.input_tokens` as separate subset counters —
//! not the cache-inclusive total the GenAI token-metrics notes prefer.
//! The aggregate prices `tokens_input` and `tokens_cache_r` as disjoint
//! buckets, and reconstructing an inclusive total (input + cache_read +
//! cache_write) would double-count on lanes whose sources are already
//! inclusive (OpenAI-compatible `prompt_tokens` includes `cached_tokens`).
//! The deviation is disclosed on every span as
//! `agenttrace.export.input_token_basis` so consumers never have to
//! guess which basis the numbers are on.

use crate::Session;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Date the GenAI semantic-convention attribute names used below were
/// verified against the `docs/gen-ai` tree. Golden tests pin this constant
/// so a future semconv drift forces a conscious refresh of the attribute
/// names rather than silent staleness.
pub const SEMCONV_SNAPSHOT_DATE: &str = "2026-10-05";

/// Recorded input-token basis of the emitted `gen_ai.usage.*` counters —
/// see the module docs for the rationale. Emitted on every span so
/// consumers never have to guess which basis the numbers are on.
const INPUT_TOKEN_BASIS: &str = "delta_excludes_cache";

const SPAN_KIND_INTERNAL: u32 = 1;
const SPAN_STATUS_OK: u32 = 1;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportTraceServiceRequest {
    resource_spans: Vec<ResourceSpans>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceSpans {
    resource: Resource,
    scope_spans: Vec<ScopeSpans>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Resource {
    attributes: Vec<KeyValue>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScopeSpans {
    scope: InstrumentationScope,
    spans: Vec<Span>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstrumentationScope {
    name: String,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    code: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Span {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    name: String,
    kind: u32,
    start_time_unix_nano: u64,
    end_time_unix_nano: u64,
    attributes: Vec<KeyValue>,
    status: Status,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyValue {
    key: String,
    value: AnyValue,
}

// Variant names intentionally mirror the OTLP JSON `AnyValue` wire field
// names (`stringValue`, `intValue`, `doubleValue`).
#[allow(clippy::enum_variant_names)]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum AnyValue {
    StringValue(String),
    IntValue(i64),
    DoubleValue(f64),
}

/// C0/C1/DEL control bytes in a stringValue become U+FFFD before the
/// value is emitted: JSON can carry them, but OTLP consumers (collectors
/// and their UIs) treat control bytes in attribute values as hostile
/// payload. Stated equivalence with the fleet's rm-539 sanitization
/// lane (same U+FFFD substitution for C0/C1/DEL); when that lane lands
/// this folds into its shared helper.
fn sanitize_otel_string(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            let code = c as u32;
            if code < 0x20 || code == 0x7f || (0x80..=0x9f).contains(&code) {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .collect()
}

/// rm-768: span names ride into OTLP-JSON documents that exporters may
/// surface raw (same exposure class as attribute values), so they get the
/// identical control-byte discipline plus a bound — an unbounded name is
/// an unbounded export record. 256 chars is far beyond any readable span
/// name and matches common backend display budgets.
const SPAN_NAME_MAX_CHARS: usize = 256;

fn sanitize_span_name(value: &str) -> String {
    let sanitized = sanitize_otel_string(value);
    if sanitized.chars().count() <= SPAN_NAME_MAX_CHARS {
        return sanitized;
    }
    match sanitized.char_indices().nth(SPAN_NAME_MAX_CHARS) {
        Some((idx, _)) => sanitized[..idx].to_string(),
        None => sanitized,
    }
}

fn attr_str(key: &str, value: impl Into<String>) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: AnyValue::StringValue(sanitize_otel_string(&value.into())),
    }
}

fn attr_int(key: &str, value: i64) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: AnyValue::IntValue(value),
    }
}

/// Finite `doubleValue` attribute. Non-finite values (NaN/±inf) cannot
/// ride an AnyValue: serde renders them as `null`, and an AnyValue whose
/// only value field is null is spec-invalid — conforming OTLP receivers
/// reject the whole request. The caller omits the attribute and
/// discloses the omission instead.
fn attr_f64_finite(key: &str, value: f64) -> Option<KeyValue> {
    value.is_finite().then(|| KeyValue {
        key: key.to_string(),
        value: AnyValue::DoubleValue(value),
    })
}

/// Best-effort `gen_ai.system` inference from the session log's path.
/// The loaded [`Session`] model does not carry the source family at this
/// revision, so we fall back to path markers for the common agent CLIs
/// and `"other"` when nothing matches.
fn infer_gen_ai_system(path: &str) -> &'static str {
    const MARKERS: &[(&str, &str)] = &[
        ("claude", "claude-code"),
        ("codex", "codex"),
        ("gemini", "gemini-cli"),
        ("qwen", "qwen-code"),
        ("kimi", "kimi"),
        ("opencode", "opencode"),
        ("cursor", "cursor"),
        ("cline", "cline"),
        ("aider", "aider"),
        ("goose", "goose"),
        ("crush", "crush"),
        ("copilot", "copilot"),
        // rm-599: workbuddy sessions are a first-class source tool —
        // without the marker they exported as "other".
        ("workbuddy", "workbuddy"),
    ];
    let path = path.to_lowercase();
    for (marker, system) in MARKERS {
        if path.contains(marker) {
            return system;
        }
    }
    "other"
}

fn rfc3339_to_unix_nanos(stamp: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(stamp)
        .ok()?
        .with_timezone(&Utc)
        .timestamp_nanos_opt()
        .and_then(|n| u64::try_from(n).ok())
}

fn session_bounds(session: &Session) -> (u64, u64) {
    let metrics = &session.metrics;
    let start = rfc3339_to_unix_nanos(&metrics.session_start)
        .or_else(|| {
            metrics
                .timestamps
                .first()
                .and_then(|t| t.timestamp_nanos_opt().and_then(|n| u64::try_from(n).ok()))
        })
        .unwrap_or(0);
    let end = rfc3339_to_unix_nanos(&metrics.session_end)
        .or_else(|| {
            metrics
                .timestamps
                .last()
                .and_then(|t| t.timestamp_nanos_opt().and_then(|n| u64::try_from(n).ok()))
        })
        .unwrap_or(start);
    (start, end.max(start))
}

/// Deterministic 32-hex-char trace id derived from the session identity.
fn trace_id_for(session: &Session) -> String {
    let mut hasher = DefaultHasher::new();
    "agenttrace-trace".hash(&mut hasher);
    session.path.hash(&mut hasher);
    session.name.hash(&mut hasher);
    let h = hasher.finish();
    format!("{h:016x}{h:016x}")
}

/// Deterministic 16-hex-char span id derived from the session identity
/// and export ordinal. The raw ordinal must never be used directly: the
/// OTel trace-data format requires a span id to contain at least one
/// non-zero byte, and ordinal 0 renders `0000000000000000` — an id that
/// conforming OTLP receivers (the OTel Collector among them) reject,
/// dropping the ENTIRE export request. Keying the hash on the session
/// identity also keeps ids distinct across sessions that share an
/// ordinal position.
fn span_id_for(session: &Session, ordinal: u64) -> String {
    let mut hasher = DefaultHasher::new();
    "agenttrace-span".hash(&mut hasher);
    session.path.hash(&mut hasher);
    session.name.hash(&mut hasher);
    ordinal.hash(&mut hasher);
    let seed = hasher.finish();
    if seed == 0 {
        // The all-zero id is the only invalid value; fold
        // deterministically to stay spec-valid.
        "0000000000000001".to_string()
    } else {
        format!("{seed:016x}")
    }
}

fn session_span(session: &Session, ordinal: u64) -> Span {
    let metrics = &session.metrics;
    let (start, end) = session_bounds(session);
    let mut attributes = vec![
        attr_str("gen_ai.system", infer_gen_ai_system(&session.path)),
        attr_str("gen_ai.request.model", metrics.model_used.clone()),
        // Delta basis — see the module docs and the basis disclosure
        // attribute below.
        attr_int("gen_ai.usage.input_tokens", metrics.tokens_input),
        attr_int("gen_ai.usage.output_tokens", metrics.tokens_output),
        // GenAI semconv snapshot 2026-10-05: cache_read/cache_write are
        // input-subset counters and reasoning is an output-subset
        // counter. The cache buckets always emit (honest zeros);
        // reasoning emits only when the session recorded any — a zero
        // cannot distinguish "none" from "source does not report".
        attr_int(
            "gen_ai.usage.cache_read.input_tokens",
            metrics.tokens_cache_r,
        ),
        attr_int(
            "gen_ai.usage.cache_write.input_tokens",
            metrics.tokens_cache_w,
        ),
        attr_int(
            "agenttrace.session.cache_read_tokens",
            metrics.tokens_cache_r,
        ),
        attr_int(
            "agenttrace.session.cache_write_tokens",
            metrics.tokens_cache_w,
        ),
        attr_int(
            "agenttrace.session.tool_calls",
            metrics.tool_calls_total as i64,
        ),
        attr_str(
            "agenttrace.session.pricing_source",
            crate::pricing::pricing_source_for(&metrics.model_used),
        ),
        attr_str(
            "agenttrace.session.source_tool",
            metrics.source_tool.clone(),
        ),
        attr_str("agenttrace.session.file", session.path.clone()),
        attr_str(
            "agenttrace.export.semconv_snapshot_date",
            SEMCONV_SNAPSHOT_DATE,
        ),
        attr_str("agenttrace.export.input_token_basis", INPUT_TOKEN_BASIS),
    ];
    if metrics.tokens_reasoning != 0 {
        attributes.push(attr_int(
            "gen_ai.usage.reasoning.output_tokens",
            metrics.tokens_reasoning,
        ));
    }
    // A non-finite cost (NaN/±inf from a hostile or future pricing
    // source) cannot ride a doubleValue (serde renders it as `null`,
    // which is a spec-invalid AnyValue) — omit the attribute and
    // disclose the omission as a string attribute.
    match attr_f64_finite("agenttrace.session.cost_usd", metrics.cost_estimated) {
        Some(attribute) => attributes.push(attribute),
        None => attributes.push(attr_str(
            "agenttrace.session.cost_usd_omitted",
            "non-finite",
        )),
    }
    Span {
        trace_id: trace_id_for(session),
        span_id: span_id_for(session, ordinal),
        parent_span_id: None,
        name: sanitize_span_name(&format!("session {}", session.name)),
        kind: SPAN_KIND_INTERNAL,
        start_time_unix_nano: start,
        end_time_unix_nano: end,
        attributes,
        status: Status {
            code: SPAN_STATUS_OK,
        },
    }
}

/// Render the sessions as a pretty-printed OTLP-JSON
/// `ExportTraceServiceRequest` document (export trace lane).
pub fn report_otel_export(sessions: &[Session]) -> String {
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0");
    let spans = sessions
        .iter()
        .enumerate()
        .map(|(idx, session)| session_span(session, idx as u64))
        .collect::<Vec<_>>();
    let request = ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Resource {
                attributes: vec![
                    attr_str("service.name", "agenttrace"),
                    attr_str("service.version", version),
                ],
            },
            scope_spans: vec![ScopeSpans {
                scope: InstrumentationScope {
                    name: "agenttrace".to_string(),
                    version: version.to_string(),
                },
                spans,
            }],
        }],
    };
    serde_json::to_string_pretty(&request).unwrap_or_else(|_| "{}".to_string())
}

/// True when the given path would be attributed to the given gen_ai
/// system by [`infer_gen_ai_system`] — exposed for tests.
pub fn gen_ai_system_for_path(path: &Path) -> &'static str {
    infer_gen_ai_system(&path.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semconv_snapshot_date_is_pinned() {
        assert_eq!(SEMCONV_SNAPSHOT_DATE, "2026-10-05");
    }

    #[test]
    fn rfc3339_conversion_round_trips() {
        let nanos = rfc3339_to_unix_nanos("2026-10-05T10:00:00Z");
        assert_eq!(nanos, Some(1_791_194_400_000_000_000));
        assert_eq!(rfc3339_to_unix_nanos(""), None);
    }

    #[test]
    fn non_finite_doubles_are_omitted_not_null() {
        // A null AnyValue is spec-invalid: NaN/±inf must never reach the
        // wire as doubleValue (serde would render `null`).
        assert!(attr_f64_finite("k", f64::NAN).is_none());
        assert!(attr_f64_finite("k", f64::INFINITY).is_none());
        assert!(attr_f64_finite("k", f64::NEG_INFINITY).is_none());
        let finite = attr_f64_finite("k", 1.5).expect("finite renders");
        match finite.value {
            AnyValue::DoubleValue(v) => assert_eq!(v, 1.5),
            _ => panic!("expected doubleValue"),
        }
    }

    #[test]
    fn control_bytes_are_replaced_in_string_values() {
        // 'm\x07' from the assess live probe must not ride a stringValue.
        let rendered =
            serde_json::to_string(&attr_str("k", "m\u{7}x\u{9b}y\u{7f}")).expect("renders");
        assert!(!rendered.contains("\\u0007"));
        assert!(!rendered.contains("\\u009b"));
        assert!(!rendered.contains("\\u007f"));
        assert!(rendered.contains('\u{FFFD}'));
    }

    #[test]
    fn span_name_sanitizer_matches_attribute_discipline_and_binds_length() {
        // rm-768: span names ride into OTLP-JSON documents that exporters
        // may surface raw, so the "session {name}" span gets the same
        // control-byte discipline as attribute values plus a 256-char cap.
        let hostile = sanitize_span_name(&format!(
            "pwn{}0;PWNED{} red {} text",
            '\u{1b}', '\u{7}', '['
        ));
        assert!(
            !hostile.contains('\u{1b}'),
            "ESC must not survive: {hostile:?}"
        );
        assert!(
            !hostile.contains('\u{7}'),
            "BEL must not survive: {hostile:?}"
        );
        assert!(hostile.contains('\u{FFFD}'), "neutralized marker expected");
        // Attribute discipline is stricter than the report-cell carve-outs:
        // every C0/C1 byte (including `\n`/`\t`) neutralizes — span names
        // inherit exactly that, so they can never smuggle layout bytes
        // into OTLP-JSON either.
        assert!(!sanitize_span_name("a\nb\tc").contains('\n'));
        assert!(!sanitize_span_name("a\nb\tc").contains('\t'));
        assert!(sanitize_span_name("plain name 42").contains("plain"));
        let long = sanitize_span_name(&"x".repeat(400));
        assert_eq!(
            long.chars().count(),
            SPAN_NAME_MAX_CHARS,
            "span name must be bounded at {SPAN_NAME_MAX_CHARS} chars"
        );
        assert!(long.chars().all(|c| c == 'x'));
    }
}

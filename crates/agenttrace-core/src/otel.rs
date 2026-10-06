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

fn attr_str(key: &str, value: impl Into<String>) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: AnyValue::StringValue(value.into()),
    }
}

fn attr_int(key: &str, value: i64) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: AnyValue::IntValue(value),
    }
}

fn attr_f64(key: &str, value: f64) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: AnyValue::DoubleValue(value),
    }
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
    // rm-605 review rider: clamp the astronomically unlikely all-zero
    // hash so no export can carry the OTLP/W3C-reserved all-zero trace
    // id either (span ids below clamp first; the row's acceptance
    // covers "trace id derivation" symmetrically).
    let h = if h == 0 { 1 } else { h };
    format!("{h:016x}{h:016x}")
}

/// Deterministic 16-hex-char span id derived from a seed.
/// Deterministic 16-hex span id. rm-605: zero-padding a raw ordinal
/// made the first span of every export carry the reserved all-zero
/// spanId (`0000000000000000`) — invalid per OTLP/W3C trace identity
/// semantics and dropped by collectors. The seed is hashed like the
/// trace id, and the astronomically unlikely all-zero hash is clamped
/// so no export can ever carry an invalid span id.
fn span_id_for(seed: u64) -> String {
    let mut hasher = DefaultHasher::new();
    "agenttrace-span".hash(&mut hasher);
    seed.hash(&mut hasher);
    let h = hasher.finish();
    let h = if h == 0 { 1 } else { h };
    format!("{h:016x}")
}

fn session_span(session: &Session, ordinal: u64) -> Span {
    let metrics = &session.metrics;
    let (start, end) = session_bounds(session);
    Span {
        trace_id: trace_id_for(session),
        span_id: span_id_for(ordinal),
        parent_span_id: None,
        name: format!("session {}", session.name),
        kind: SPAN_KIND_INTERNAL,
        start_time_unix_nano: start,
        end_time_unix_nano: end,
        attributes: vec![
            attr_str("gen_ai.system", infer_gen_ai_system(&session.path)),
            attr_str("gen_ai.request.model", metrics.model_used.clone()),
            attr_int("gen_ai.usage.input_tokens", metrics.tokens_input),
            attr_int("gen_ai.usage.output_tokens", metrics.tokens_output),
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
            attr_f64("agenttrace.session.cost_usd", metrics.cost_estimated),
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
        ],
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
}

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
use serde::Serialize;
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
/// this folds into its shared helper. rm-936: the fold landed — the Cc
/// test now composes with the shared Cf predicate
/// ([`crate::statusline::is_bidi_format_control`]) so span names and
/// attribute values cannot carry directional controls either.
fn sanitize_otel_string(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            let code = c as u32;
            if c.is_control()
                || crate::statusline::is_bidi_format_control(c)
                || (0x80..=0x9f).contains(&code)
            {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .collect()
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
    // rm-502 (integration of run 6aaf51aa): session bounds parse
    // through the shared lenient arm (lib.rs `parse_ts` — the single
    // source of timestamp truth) exactly like every other consumer of
    // `metrics.session_start`/`session_end`, so a naive-ISO stamp keeps
    // its real nanos here instead of silently dropping to the
    // `timestamps.first()` fallback the strict copy forced.
    crate::parse_ts(stamp)?
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

/// Render a hash-derived OTel id: the seed zero-padded to 16 hex
/// digits, repeated `repeat` times (span ids once; trace ids mirror the
/// seed across both 64-bit halves of the 128-bit width). The all-zero
/// id is the only invalid value in the OTel id format — a conforming
/// OTLP receiver rejects it and drops the ENTIRE export request — so a
/// zero seed folds deterministically to the smallest non-zero id
/// instead of rendering zeros.
fn non_zero_hex_id(seed: u64, repeat: usize) -> String {
    format!("{:016x}", if seed == 0 { 1 } else { seed }).repeat(repeat)
}

/// Deterministic 32-hex-char trace id derived from the session identity.
///
/// rm-599 (residual): the hash is the owned FNV-1a behind the history
/// lane's [`crate::history::stable_identity_hash`] — same rationale as
/// the rm-212 identity stamps: `DefaultHasher`'s SipHash output is
/// stable on today's toolchain but Rust does NOT guarantee it across
/// releases, and these ids are exported for cross-run correlation
/// (dedupe/joins in a collector), so a toolchain bump silently
/// re-keying every id defeats the export's purpose. Ids change exactly
/// once at this switch; they are computed at export time and no
/// on-disk artifact persists them, so there is nothing to migrate.
///
/// rm-651: the id gets the same all-zero guard span ids have had since
/// rm-599 — the seed mirrors itself across both halves, and a zero seed
/// folds (via [`non_zero_hex_id`]) instead of rendering the
/// spec-invalid all-zero traceId. Non-zero seeds render byte-identically
/// to the previous format (no wire-format change for existing exports).
fn trace_id_for(session: &Session) -> String {
    trace_id_from_identity(&session.path, &session.name)
}

/// The pure derivation behind [`trace_id_for`], exposed as a seam so
/// the golden vectors in the test module can pin exact ids without
/// depending on a fixture file's on-disk path.
fn trace_id_from_identity(path: &str, name: &str) -> String {
    let canonical = format!("agenttrace-trace|{path}|{name}");
    non_zero_hex_id(crate::history::stable_identity_hash(&canonical), 2)
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
    span_id_from_identity(&session.path, &session.name, ordinal)
}

fn span_id_from_identity(path: &str, name: &str, ordinal: u64) -> String {
    // rm-599 (residual): FNV-1a over a delimited canonical string —
    // see trace_id_from_identity for why the owned hash replaced
    // DefaultHasher. The delimiter guards against identity collisions
    // that space-free concatenation would admit ("a|bc" vs "ab|c").
    // rm-651 folds a zero seed through non_zero_hex_id — the same
    // shared rule the trace id arm adopted ("0000000000000001"),
    // byte-identical to the explicit fold this replaced.
    let canonical = format!("agenttrace-span|{path}|{name}|{ordinal}");
    non_zero_hex_id(crate::history::stable_identity_hash(&canonical), 1)
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
        name: format!("session {}", session.name),
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
    #[test]
    fn rm936_sanitize_otel_string_neutralizes_bidi_format_controls() {
        // rm-936: span names and attribute values must not carry raw
        // directional controls; is_control() alone passes every Cf.
        let out = sanitize_otel_string("span\u{202E}name \u{2066}\u{061C}");
        for c in ['\u{202E}', '\u{2066}', '\u{061C}'] {
            assert!(!out.contains(c), "raw {c:?} survived otel sanitization");
        }
        assert_eq!(out.matches('\u{FFFD}').count(), 3);
        // The pre-existing Cc/C1/DEL contract is unchanged.
        assert!(
            sanitize_otel_string("a\u{1B}[0mb\u{85}c\u{7F}d")
                .matches('\u{FFFD}')
                .count()
                == 3
        );
        assert_eq!(sanitize_otel_string("plain"), "plain");
    }
    use super::*;

    #[test]
    fn id_folds_never_render_the_all_zero_form() {
        // rm-651: trace ids get the same zero-fold span ids have had
        // since rm-599 — a zero hash seed must not produce the all-zero
        // id an OTLP receiver rejects (dropping the whole export).
        assert_eq!(non_zero_hex_id(0, 1), "0000000000000001");
        // The fold mirrors the 16-hex-digit half exactly like a real
        // seed does — both halves identical.
        assert_eq!(non_zero_hex_id(0, 2), "00000000000000010000000000000001");
        // Non-zero seeds render verbatim: the trace id stays the
        // doubled half it has always been (no wire-format change).
        let seed = 0xdead_beef_cafe_f00d_u64;
        assert_eq!(non_zero_hex_id(seed, 2), format!("{seed:016x}{seed:016x}"));
        assert_eq!(non_zero_hex_id(seed, 1), format!("{seed:016x}"));
        // Adversarial seeds: every width renders 16 hex digits per
        // repeat, hex-only, with at least one non-zero byte.
        for seed in [0u64, 1, 42, u64::MAX - 1, u64::MAX] {
            for repeat in [1usize, 2] {
                let id = non_zero_hex_id(seed, repeat);
                assert_eq!(id.len(), 16 * repeat);
                assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
                assert!(
                    id.bytes().any(|b| b != b'0'),
                    "all-zero id leaks through for seed {seed}"
                );
            }
        }
    }

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
    fn trace_and_span_ids_pin_fnv1a_golden_vectors() {
        // rm-599 (residual): the exported ids are the owned FNV-1a
        // (history::stable_identity_hash), not std's DefaultHasher —
        // cross-run correlation in a collector assumes the SAME
        // session re-keys to the SAME id on every toolchain. These
        // vectors were derived independently from the FNV-1a 64-bit
        // reference constants (offset basis 0xcbf29ce484222325, prime
        // 0x100000001b3), so a regression to DefaultHasher — or any
        // accidental change to the canonical string shape, including
        // the '|' delimiters — flips the pin.
        let path = "/tmp/at/sessions/claude.jsonl";
        let name = "claude-4";
        assert_eq!(
            trace_id_from_identity(path, name),
            "b17fe45ad4d93ba1b17fe45ad4d93ba1",
            "trace id golden (doubled 64-bit FNV-1a)"
        );
        assert_eq!(
            span_id_from_identity(path, name, 0),
            "ae053c09d3677036",
            "span id golden at ordinal 0"
        );
        assert_eq!(
            span_id_from_identity(path, name, 7),
            "ae053f09d367754f",
            "span id golden at ordinal 7 — ordinals must re-key the hash"
        );
        // The delimiter is load-bearing: identity-collision inputs
        // ("a|bc" vs "ab|c" concatenations) must not collapse.
        assert_ne!(
            span_id_from_identity("a", "bc|0", 0),
            span_id_from_identity("ab", "c", 0),
            "'|' delimiters must separate path from name from ordinal"
        );
        // Trace ids are 32 lowercase hex chars; span ids 16.
        let trace = trace_id_from_identity(path, name);
        assert_eq!(trace.len(), 32);
        assert!(trace
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        let span = span_id_from_identity(path, name, 42);
        assert_eq!(span.len(), 16);
        assert!(span
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}

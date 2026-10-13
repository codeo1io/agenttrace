# OTel export — emitted attribute reference

Generated reference for the OTLP/JSON export lane (`crates/agenttrace-core/src/otel.rs`, rm-493 lineage). Every attribute the export emits is listed here with its source and its standing against the GenAI semantic conventions.

**Semconv pin**: the attribute vocabulary is verified against `open-telemetry/semantic-conventions-genai` at commit **`07794021`** (committer date 2026-10-14; the repository publishes no releases or tags, so the commit is the only citable tree). The pin travels with every export as `agenttrace.export.semconv_snapshot_commit` + `agenttrace.export.semconv_snapshot_date` and is pinned by golden tests (`tests/otel_export.rs::emitted_attribute_vocabulary_is_pinned_rm546`) — a semconv drift must be a deliberate, test-forced refresh of both constants, the table below, and CHANGELOG.

## Span shape

One `INTERNAL` span per session, named `session <session-name>`; the tool travels as `gen_ai.system` (fork-retained name — see its table row). Per-assistant-turn child spans are a disclosed follow-up on rm-493 (the public parse API returns `Session` aggregates only).

## Session-span attributes

| Attribute | Type | Source | Standing at `07794021` |
|---|---|---|---|
| `gen_ai.system` | string | inferred from session path (`infer_gen_ai_system`) | **fork-retained** — not defined in the pinned registry @`07794021` (verified: registry's provider discriminator is `gen_ai.provider.name`, an enum; the only `gen_ai.system*` key there is `gen_ai.system_instructions`); migrating to `gen_ai.provider.name` is a named decision at the next pin refresh |
| `gen_ai.request.model` | string | `metrics.model_used` | semconv (`gen_ai.request.model`) |
| `gen_ai.usage.input_tokens` | int | `metrics.tokens_input` | semconv — **delta basis, see below** |
| `gen_ai.usage.output_tokens` | int | `metrics.tokens_output` | semconv |
| `gen_ai.usage.cache_read.input_tokens` | int | `metrics.tokens_cache_r` | semconv; always emits (honest zeros) |
| `gen_ai.usage.cache_write.input_tokens` | int | `metrics.tokens_cache_w` | semconv; always emits (honest zeros) |
| `gen_ai.usage.reasoning.output_tokens` | int | `metrics.tokens_reasoning` | semconv; emits only when recorded — zero cannot distinguish "none" from "not reported" |
| `agenttrace.session.tool_calls` | int | `metrics.tool_calls_total` | agenttrace custom |
| `agenttrace.session.pricing_source` | string | `pricing::pricing_source_for` | agenttrace custom |
| `agenttrace.session.source_tool` | string | `metrics.source_tool` | agenttrace custom |
| `agenttrace.session.file` | string | `session.path` | agenttrace custom |
| `agenttrace.session.cost_usd` | double | `metrics.cost_estimated` | agenttrace custom; **omitted when non-finite** — a null AnyValue is spec-invalid — with `agenttrace.session.cost_usd_omitted = "non-finite"` disclosing the omission |
| `agenttrace.session.cache_read_tokens` | int | `metrics.tokens_cache_r` | **legacy** alias of the semconv cache counter, retained one deprecation cycle |
| `agenttrace.session.cache_write_tokens` | int | `metrics.tokens_cache_w` | **legacy** alias, same policy |
| `agenttrace.export.semconv_snapshot_date` | string | constant | agenttrace custom |
| `agenttrace.export.semconv_snapshot_commit` | string | constant | agenttrace custom |
| `agenttrace.export.input_token_basis` | string | constant = `delta_excludes_cache` | agenttrace custom — basis disclosure |

## Resource attributes

Exactly `service.name = "agenttrace"` and `service.version = <crate version>`.

## Input-token basis (recorded divergence)

The semconv registry says the cache buckets **SHOULD be included** in `gen_ai.usage.input_tokens`. agenttrace deliberately emits the **uncached (delta) remainder** and discloses it per-span via `agenttrace.export.input_token_basis = "delta_excludes_cache"`: the pricing lane bills uncached input, cache reads, and cache writes at different rates, so the delta basis is the cost-relevant one. Upstream ccusage #1858 (2026-10-10) normalizes to the same uncached remainder (silently). A span's exported total (`input + cache_read + cache_write`) never exceeds the source-recorded input (wb7 clamp, pinned by test).

## Reconciliation vs upstream ccusage #1858 (rm-983, 2026-10-14)

- `inputTokens`-cache-subtraction → **fork-supersedes** (delta basis since inception + per-span disclosure).
- `reasoningTokens`-subset-of-output → **fork-supersedes** (pinned since rm-863).
- count-OTel-calls-missing-from-shutdowns → **declined this cycle** (copilot parser ingest-truth lane; follow-up candidate).
- cache-write pricing fix → out of scope (pricing seam; fork prices copilot off `totalNanoAiu` credits, rm-485).

## Non-goals at this pin

- `gen_ai.token.modality` / modality-scoped buckets (`gen_ai.usage.text|image|audio.*`) — not recorded by any source lane yet.
- GenAI token **histograms** (upstream metrics work) — the export is a trace-data file write only; no metrics signal is emitted.
- `gen_ai.client.inference.duration` (the 4f85037 `operation.duration`→`inference.duration` rename) — the fork emits no duration attribute; the rename is adopted at naming policy: any future duration attribute uses the new spelling.
- Network transport of any kind (PRIVACY.md offline-first).

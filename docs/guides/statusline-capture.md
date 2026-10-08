# Statusline capture

Two data sources agenttrace users repeatedly ask for never appear in session
transcripts:

- subscription limit pressure (the `5h` / `7d` usage windows and their reset
  times), and
- Claude Code's upstream prompt-cache analytics (hit ratio, misses, and miss
  causes such as `tools_changed`).

Both are only observable from the **statusline payload** Claude Code feeds the
`statusLine` command on every prompt. `agenttrace statusline` captures that
payload to a local journal so `--statusline-report`, the TUI Efficiency panel,
and `--doctor` can surface them.

## Setup

Register the command once in Claude Code's `settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "agenttrace statusline"
  }
}
```

The command must be on `PATH` for the host (the [install
script](../../README.md#install) puts it on `~/.local/bin` or `/usr/local/bin`).

## Host contract

Claude Code invokes the command with the payload as a single line of JSON on
stdin and expects **exactly one line on stdout**. `agenttrace statusline`:

- renders `name | ctx N% | 5h/7d N% until HH:MM | cache N% | $cost`,
- exits `0` for **any** input — valid payload, malformed JSON, or empty stdin,
- exits `0` for any **stdout condition** too — a closed pipe or full disk is
  reported to stderr at most, never a panic (the line is written with the
  write error ignored),
- sanitizes payload strings (session/model names) by replacing control
  characters (C0, DEL, C1) with `U+FFFD`, so a hostile session title can
  neither break the one-line contract nor inject escape sequences into the
  host terminal,
- writes diagnostics to stderr only (never stdout, never a non-zero exit), so
  the host UI cannot be broken by a capture bug,
- bounds stdin at 1 MiB so a pathological host cannot wedge the prompt.

On bad or empty input the rendered line is simply `agenttrace`.

## Journal and retention

Every invocation appends one JSON line — `{"captured_at": <unix seconds>,
"payload": <the host payload verbatim>}` — to:

- `$AGENTTRACE_SESSION_CACHE_DIR/statusline.jsonl` when that variable is set
  (the same directory as the session cache), otherwise
- `<user cache dir>/agenttrace/statusline.jsonl` (typically
  `~/.cache/agenttrace/statusline.jsonl`).

Retention is bounded at 10 MiB: past that, oldest **whole** lines are dropped
until the journal fits half the bound, through a temp-file rename so a crash
mid-compaction cannot truncate it. Torn or malformed lines are skipped on read.

## Schema (what agenttrace reads)

The journal is schema-faithful to the documented Claude Code statusline
payload. agenttrace reads these fields when present and ignores the rest:

| Field | Meaning |
| --- | --- |
| `session_id`, `session_name` | session identity for per-session grouping |
| `model.display_name` | rendered in the status line |
| `workspace.current_dir` | optional, for correlation |
| `context_window.used_percentage` | context pressure |
| `context_window.remaining_percentage` | context headroom (v2.1.251+) |
| `context_window.current_usage.{input_tokens,output_tokens,cache_creation_input_tokens,cache_read_input_tokens}` | last-API token counts; `null` before the first API call and right after `/compact` |
| `cost.total_cost_usd` | session cost to date |
| `rate_limits.five_hour.{used_percentage,resets_at}` | the 5-hour window |
| `rate_limits.seven_day.{used_percentage,resets_at}` | the 7-day window |
| `rate_limits.spend_limit.{used_percentage,resets_at,used_usd,limit_usd,period}` | gateway spend-limit window (Claude apps gateway, v2.1.251+; the USD/period trio is v2.1.284+ and arrives on a slower ~5-minute schedule, so each field is optional even when the object is present) |
| `prompt_cache.{hit_ratio,misses,last_miss_cause,miss_causes}` | cache analytics |
| `prompt_id` | prompt correlation key: matches the `prompt.id` attribute on OpenTelemetry events, when they exist; absent until the session's first user input |

Report semantics worth knowing:

- Captures are **deduplicated by exact payload**: identical consecutive
  payloads (e.g. a redrawn prompt) count once. Reports say so
  (`N captures (M after dedup)`).
- A **limit crossing** is evidenced only by observations on both sides of a
  `resets_at` boundary (usage observed before, and the first observation
  after). A window whose reset passed outside the journal, or a bare
  `resets_at` with no before-side, is not claimed as a crossing. The gateway
  `spend_limit` window resets on the same mechanism and joins the same scan.
- `hit_ratio` is a `0..1` ratio; rendered percentages multiply by 100.
- The report's context, spend and prompt arms render **only when the
  payload carries them** — a legacy pre-v2.1.251 journal produces no
  phantom arms, and unknown future fields are tolerated (ignored, never
  an error).
- `--statusline-report` discloses prompt-key presence per capture:
  captures **with** `prompt_id`, captures without it (the documented
  pre-first-input state), and unclassifiable payloads are counted
  separately. Correlating journal prompt ids against an OTLP-JSON export
  carrying `prompt.id` span attributes is the pure join in
  `agenttrace_core::correlated_prompt_ids`.

## Reviewing captures

```bash
agenttrace --statusline-report          # text
agenttrace --statusline-report -f json  # machine-readable insights
agenttrace --doctor                     # "Statusline capture:" health line
```

The TUI Efficiency panel shows the same insights under "Subscription limits"
when a journal exists.

## Honesty notes

- agenttrace **requires** the statusline command to be configured; nothing is
  captured otherwise, and `--doctor` says so.
- The fixtures in `crates/agenttrace-core/src/statusline.rs` tests follow the
  documented payload contract; they are not recordings of a real host.
